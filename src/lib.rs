use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::fmt::{self, Display};
use std::fs;
use std::path::{Path, PathBuf};
use x11rb::connection::Connection;
use x11rb::protocol::randr::{
    Connection as OutputConnection, ConnectionExt as RandrConnectionExt, Crtc, Mode, ModeInfo,
    Output, Rotation, SetConfig,
};
use x11rb::protocol::xproto::{Timestamp, Window};
use x11rb::rust_connection::RustConnection;

const CURRENT_TIME: Timestamp = 0;
const DISABLED_MODE: Mode = 0;
const DISABLED_CRTC: Crtc = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    Changed,
    AlreadySatisfied,
    NoConfiguredConnectedOutput,
}

impl ExitStatus {
    pub const fn code(self) -> i32 {
        match self {
            Self::Changed => 0,
            Self::AlreadySatisfied => 10,
            Self::NoConfiguredConnectedOutput => 11,
        }
    }
}

impl Display for ExitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Changed => f.write_str("changed"),
            Self::AlreadySatisfied => f.write_str("already satisfied"),
            Self::NoConfiguredConnectedOutput => f.write_str("no configured connected output"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Usage,
    XorgUnavailable,
    Unavailable,
    RandrFailed,
}

#[derive(Debug)]
pub struct AttachError {
    kind: ErrorKind,
    message: String,
}

impl AttachError {
    pub fn usage(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Usage, message)
    }

    fn xorg(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::XorgUnavailable, message)
    }

    fn unavailable(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unavailable, message)
    }

    fn randr(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::RandrFailed, message)
    }

    fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub const fn exit_code(&self) -> i32 {
        match self.kind {
            ErrorKind::Usage => 64,
            ErrorKind::XorgUnavailable => 69,
            ErrorKind::Unavailable => 70,
            ErrorKind::RandrFailed => 71,
        }
    }
}

impl Display for AttachError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl std::error::Error for AttachError {}

pub type Result<T> = std::result::Result<T, AttachError>;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Status,
    On(OnRequest),
    Off { output: String },
    Auto { config: PathBuf },
    Help,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OnRequest {
    pub output: String,
    pub mode: ModeRequest,
    pub x: i16,
    pub y: i16,
    pub rotation: RotationRequest,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModeRequest {
    Preferred,
    Explicit {
        width: u16,
        height: u16,
        rate: Option<f64>,
    },
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RotationRequest {
    #[default]
    Normal,
    Left,
    Inverted,
    Right,
}

impl RotationRequest {
    fn to_randr(self) -> Rotation {
        match self {
            Self::Normal => Rotation::ROTATE0,
            Self::Left => Rotation::ROTATE90,
            Self::Inverted => Rotation::ROTATE180,
            Self::Right => Rotation::ROTATE270,
        }
    }

    fn dimensions(self, width: u16, height: u16) -> (u16, u16) {
        match self {
            Self::Normal | Self::Inverted => (width, height),
            Self::Left | Self::Right => (height, width),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DisplayConfig {
    pub outputs: Vec<ConfiguredOutput>,
}

#[derive(Debug, Deserialize)]
pub struct ConfiguredOutput {
    pub name: String,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub rate: Option<f64>,
    #[serde(default)]
    pub x: i16,
    #[serde(default)]
    pub y: i16,
    #[serde(default)]
    pub rotation: RotationRequest,
}

fn enabled_by_default() -> bool {
    true
}

impl ConfiguredOutput {
    fn on_request(&self) -> Result<OnRequest> {
        let mode = match (self.width, self.height, self.rate) {
            (Some(width), Some(height), rate) => ModeRequest::Explicit {
                width,
                height,
                rate,
            },
            (None, None, None) => ModeRequest::Preferred,
            _ => {
                return Err(AttachError::usage(format!(
                    "output '{}' must set both width and height, or neither",
                    self.name
                )))
            }
        };

        Ok(OnRequest {
            output: self.name.clone(),
            mode,
            x: self.x,
            y: self.y,
            rotation: self.rotation,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutputStatus {
    pub name: String,
    pub connected: bool,
    pub active: bool,
    pub current_mode: Option<ModeSummary>,
    pub x: Option<i16>,
    pub y: Option<i16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeSummary {
    pub width: u16,
    pub height: u16,
    pub mode_id: Mode,
}

#[derive(Debug, Clone)]
struct OutputState {
    id: Output,
    name: String,
    connected: bool,
    crtc: Crtc,
    possible_crtcs: Vec<Crtc>,
    modes: Vec<Mode>,
    preferred_count: usize,
}

#[derive(Debug, Clone)]
struct CrtcState {
    id: Crtc,
    x: i16,
    y: i16,
    width: u16,
    height: u16,
    mode: Mode,
    rotation: Rotation,
    outputs: Vec<Output>,
}

#[derive(Debug, Clone)]
struct RandrState {
    root: Window,
    config_timestamp: Timestamp,
    root_width: u16,
    root_height: u16,
    root_mm_width: u16,
    root_mm_height: u16,
    outputs: Vec<OutputState>,
    crtcs: Vec<CrtcState>,
    modes: Vec<ModeInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SelectedMode {
    id: Mode,
    width: u16,
    height: u16,
}

pub fn parse_args<I, S>(args: I) -> Result<Command>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);
    let Some(command) = args.next() else {
        return Err(AttachError::usage(usage()));
    };

    match command.as_str() {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "status" => reject_extra(args, Command::Status),
        "on" => parse_on(args),
        "off" => parse_off(args),
        "auto" => parse_auto(args),
        _ => Err(AttachError::usage(format!(
            "unknown command '{command}'\n\n{}",
            usage()
        ))),
    }
}

pub fn usage() -> String {
    "Usage:
  xdisplay-attach status
  xdisplay-attach on --output NAME --preferred
  xdisplay-attach on --output NAME --width N --height N [--rate HZ]
  xdisplay-attach off --output NAME
  xdisplay-attach auto --config FILE"
        .to_string()
}

pub fn run_cli() -> Result<ExitStatus> {
    let command = parse_args(env::args().skip(1))?;
    match command {
        Command::Help => {
            println!("{}", usage());
            Ok(ExitStatus::AlreadySatisfied)
        }
        Command::Status => {
            let statuses = X11Randr::connect()?.status()?;
            for status in statuses {
                print_status(&status);
            }
            Ok(ExitStatus::AlreadySatisfied)
        }
        Command::On(request) => X11Randr::connect()?.turn_on(&request),
        Command::Off { output } => X11Randr::connect()?.turn_off(&output),
        Command::Auto { config } => {
            let config = read_config(&config)?;
            X11Randr::connect()?.auto(&config)
        }
    }
}

pub fn read_config(path: &Path) -> Result<DisplayConfig> {
    let content = fs::read_to_string(path).map_err(|error| {
        AttachError::usage(format!("failed to read '{}': {error}", path.display()))
    })?;
    serde_json::from_str(&content).map_err(|error| {
        AttachError::usage(format!("failed to parse '{}': {error}", path.display()))
    })
}

fn parse_on(args: impl Iterator<Item = String>) -> Result<Command> {
    let mut output = None;
    let mut preferred = false;
    let mut width = None;
    let mut height = None;
    let mut rate = None;
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => output = Some(next_value(&mut args, "--output")?),
            "--preferred" => preferred = true,
            "--width" => width = Some(parse_u16(next_value(&mut args, "--width")?, "--width")?),
            "--height" => height = Some(parse_u16(next_value(&mut args, "--height")?, "--height")?),
            "--rate" => rate = Some(parse_f64(next_value(&mut args, "--rate")?, "--rate")?),
            _ => return Err(AttachError::usage(format!("unknown on option '{arg}'"))),
        }
    }

    let output = output.ok_or_else(|| AttachError::usage("on requires --output"))?;
    let mode = match (preferred, width, height, rate) {
        (true, None, None, None) => ModeRequest::Preferred,
        (false, Some(width), Some(height), rate) => ModeRequest::Explicit {
            width,
            height,
            rate,
        },
        (true, Some(_), _, _) | (true, _, Some(_), _) | (true, _, _, Some(_)) => Err(
            AttachError::usage("--preferred cannot be combined with --width, --height, or --rate"),
        )?,
        (false, _, _, Some(_)) => Err(AttachError::usage("--rate requires --width and --height"))?,
        _ => Err(AttachError::usage(
            "on requires either --preferred or --width N --height N",
        ))?,
    };

    Ok(Command::On(OnRequest {
        output,
        mode,
        x: 0,
        y: 0,
        rotation: RotationRequest::Normal,
    }))
}

fn parse_off(args: impl Iterator<Item = String>) -> Result<Command> {
    let mut output = None;
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => output = Some(next_value(&mut args, "--output")?),
            _ => return Err(AttachError::usage(format!("unknown off option '{arg}'"))),
        }
    }
    Ok(Command::Off {
        output: output.ok_or_else(|| AttachError::usage("off requires --output"))?,
    })
}

fn parse_auto(args: impl Iterator<Item = String>) -> Result<Command> {
    let mut config = None;
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => config = Some(PathBuf::from(next_value(&mut args, "--config")?)),
            _ => return Err(AttachError::usage(format!("unknown auto option '{arg}'"))),
        }
    }
    Ok(Command::Auto {
        config: config.ok_or_else(|| AttachError::usage("auto requires --config"))?,
    })
}

fn reject_extra(args: impl Iterator<Item = String>, command: Command) -> Result<Command> {
    let extras: Vec<String> = args.collect();
    if extras.is_empty() {
        Ok(command)
    } else {
        Err(AttachError::usage(format!(
            "unexpected argument '{}'",
            extras[0]
        )))
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String> {
    args.next()
        .ok_or_else(|| AttachError::usage(format!("{option} requires a value")))
}

fn parse_u16(value: String, option: &str) -> Result<u16> {
    value
        .parse()
        .map_err(|_| AttachError::usage(format!("{option} must be an integer")))
}

fn parse_f64(value: String, option: &str) -> Result<f64> {
    value
        .parse()
        .map_err(|_| AttachError::usage(format!("{option} must be a number")))
}

fn print_status(status: &OutputStatus) {
    let connection = if status.connected {
        "connected"
    } else {
        "disconnected"
    };
    let activity = if status.active { "active" } else { "inactive" };
    match (status.current_mode, status.x, status.y) {
        (Some(mode), Some(x), Some(y)) => {
            println!(
                "{} {connection} {activity} {}x{}+{x}+{y}",
                status.name, mode.width, mode.height
            );
        }
        _ => println!("{} {connection} {activity}", status.name),
    }
}

struct X11Randr {
    conn: RustConnection,
    screen_num: usize,
}

impl X11Randr {
    fn connect() -> Result<Self> {
        let (conn, screen_num) =
            x11rb::connect(None).map_err(|error| AttachError::xorg(error.to_string()))?;
        conn.randr_query_version(1, 5)
            .map_err(|error| AttachError::xorg(format!("RandR is unavailable: {error}")))?
            .reply()
            .map_err(|error| AttachError::xorg(format!("RandR is unavailable: {error}")))?;
        Ok(Self { conn, screen_num })
    }

    fn status(&self) -> Result<Vec<OutputStatus>> {
        let state = self.load_state()?;
        Ok(state
            .outputs
            .iter()
            .map(|output| {
                let crtc = state.crtcs.iter().find(|crtc| crtc.id == output.crtc);
                OutputStatus {
                    name: output.name.clone(),
                    connected: output.connected,
                    active: output.crtc != DISABLED_CRTC,
                    current_mode: crtc
                        .and_then(|crtc| mode_by_id(&state.modes, crtc.mode))
                        .map(|mode| ModeSummary {
                            width: mode.width,
                            height: mode.height,
                            mode_id: mode.id,
                        }),
                    x: crtc.map(|crtc| crtc.x),
                    y: crtc.map(|crtc| crtc.y),
                }
            })
            .collect())
    }

    fn turn_on(&self, request: &OnRequest) -> Result<ExitStatus> {
        let state = self.load_state()?;
        self.apply_on(&state, request)
    }

    fn turn_off(&self, output_name: &str) -> Result<ExitStatus> {
        let state = self.load_state()?;
        self.apply_off(&state, output_name)
    }

    fn auto(&self, config: &DisplayConfig) -> Result<ExitStatus> {
        let mut changed = false;
        let mut found_connected_enabled = false;

        for configured in &config.outputs {
            let state = self.load_state()?;
            let output = state
                .outputs
                .iter()
                .find(|output| output.name == configured.name)
                .ok_or_else(|| {
                    AttachError::unavailable(format!("output '{}' is unavailable", configured.name))
                })?;

            if configured.enabled {
                if !output.connected {
                    continue;
                }
                found_connected_enabled = true;
                let status = self.apply_on(&state, &configured.on_request()?)?;
                changed |= status == ExitStatus::Changed;
            } else {
                let status = self.apply_off(&state, &configured.name)?;
                changed |= status == ExitStatus::Changed;
            }
        }

        if changed {
            Ok(ExitStatus::Changed)
        } else if found_connected_enabled {
            Ok(ExitStatus::AlreadySatisfied)
        } else {
            Ok(ExitStatus::NoConfiguredConnectedOutput)
        }
    }

    fn apply_on(&self, state: &RandrState, request: &OnRequest) -> Result<ExitStatus> {
        let output = find_output(state, &request.output)?;
        if !output.connected {
            return Err(AttachError::unavailable(format!(
                "output '{}' is not connected",
                request.output
            )));
        }

        let mode = select_mode(&state.modes, output, request.mode)?;
        let crtc = choose_crtc(output, &state.crtcs)?;
        if output_already_satisfied(state, output, crtc, mode, request) {
            return Ok(ExitStatus::AlreadySatisfied);
        }

        self.expand_root_if_needed(state, mode, request)?;
        let outputs = [output.id];
        let reply = self
            .conn
            .randr_set_crtc_config(
                crtc,
                CURRENT_TIME,
                state.config_timestamp,
                request.x,
                request.y,
                mode.id,
                request.rotation.to_randr(),
                &outputs,
            )
            .map_err(|error| AttachError::randr(error.to_string()))?
            .reply()
            .map_err(|error| AttachError::randr(error.to_string()))?;
        check_set_config(reply.status)?;
        self.conn
            .flush()
            .map_err(|error| AttachError::randr(error.to_string()))?;
        Ok(ExitStatus::Changed)
    }

    fn apply_off(&self, state: &RandrState, output_name: &str) -> Result<ExitStatus> {
        let output = find_output(state, output_name)?;
        if output.crtc == DISABLED_CRTC {
            return Ok(ExitStatus::AlreadySatisfied);
        }

        let reply = self
            .conn
            .randr_set_crtc_config(
                output.crtc,
                CURRENT_TIME,
                state.config_timestamp,
                0,
                0,
                DISABLED_MODE,
                Rotation::ROTATE0,
                &[],
            )
            .map_err(|error| AttachError::randr(error.to_string()))?
            .reply()
            .map_err(|error| AttachError::randr(error.to_string()))?;
        check_set_config(reply.status)?;

        self.shrink_root_after_disable(state, output.crtc)?;
        self.conn
            .flush()
            .map_err(|error| AttachError::randr(error.to_string()))?;
        Ok(ExitStatus::Changed)
    }

    fn load_state(&self) -> Result<RandrState> {
        let setup = self.conn.setup();
        let screen = setup
            .roots
            .get(self.screen_num)
            .ok_or_else(|| AttachError::xorg("default X screen is unavailable"))?;
        let root = screen.root;
        let resources = self
            .conn
            .randr_get_screen_resources(root)
            .map_err(|error| AttachError::randr(error.to_string()))?
            .reply()
            .map_err(|error| AttachError::randr(error.to_string()))?;

        let outputs = resources
            .outputs
            .iter()
            .map(|output_id| {
                let info = self
                    .conn
                    .randr_get_output_info(*output_id, resources.config_timestamp)
                    .map_err(|error| AttachError::randr(error.to_string()))?
                    .reply()
                    .map_err(|error| AttachError::randr(error.to_string()))?;
                Ok(OutputState {
                    id: *output_id,
                    name: String::from_utf8_lossy(&info.name).into_owned(),
                    connected: info.connection == OutputConnection::CONNECTED,
                    crtc: info.crtc,
                    possible_crtcs: info.crtcs,
                    modes: info.modes,
                    preferred_count: usize::from(info.num_preferred),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let crtcs = resources
            .crtcs
            .iter()
            .map(|crtc_id| {
                let info = self
                    .conn
                    .randr_get_crtc_info(*crtc_id, resources.config_timestamp)
                    .map_err(|error| AttachError::randr(error.to_string()))?
                    .reply()
                    .map_err(|error| AttachError::randr(error.to_string()))?;
                Ok(CrtcState {
                    id: *crtc_id,
                    x: info.x,
                    y: info.y,
                    width: info.width,
                    height: info.height,
                    mode: info.mode,
                    rotation: info.rotation,
                    outputs: info.outputs,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(RandrState {
            root,
            config_timestamp: resources.config_timestamp,
            root_width: screen.width_in_pixels,
            root_height: screen.height_in_pixels,
            root_mm_width: screen.width_in_millimeters,
            root_mm_height: screen.height_in_millimeters,
            outputs,
            crtcs,
            modes: resources.modes,
        })
    }

    fn expand_root_if_needed(
        &self,
        state: &RandrState,
        mode: SelectedMode,
        request: &OnRequest,
    ) -> Result<()> {
        let (width, height) = request.rotation.dimensions(mode.width, mode.height);
        let right = checked_extent(request.x, width)?;
        let bottom = checked_extent(request.y, height)?;
        let new_width = right.max(state.root_width);
        let new_height = bottom.max(state.root_height);
        if new_width == state.root_width && new_height == state.root_height {
            return Ok(());
        }
        self.set_screen_size(state, new_width, new_height)
    }

    fn shrink_root_after_disable(&self, state: &RandrState, disabled_crtc: Crtc) -> Result<()> {
        let mut width: u16 = 1;
        let mut height: u16 = 1;
        for crtc in &state.crtcs {
            if crtc.id == disabled_crtc || crtc.mode == DISABLED_MODE {
                continue;
            }
            let right = checked_extent(crtc.x, crtc.width)?;
            let bottom = checked_extent(crtc.y, crtc.height)?;
            width = width.max(right);
            height = height.max(bottom);
        }

        if width < state.root_width || height < state.root_height {
            self.set_screen_size(state, width, height)?;
        }
        Ok(())
    }

    fn set_screen_size(&self, state: &RandrState, width: u16, height: u16) -> Result<()> {
        let mm_width = scaled_mm(state.root_mm_width, state.root_width, width);
        let mm_height = scaled_mm(state.root_mm_height, state.root_height, height);
        self.conn
            .randr_set_screen_size(state.root, width, height, mm_width, mm_height)
            .map_err(|error| AttachError::randr(error.to_string()))?
            .check()
            .map_err(|error| AttachError::randr(error.to_string()))
    }
}

fn find_output<'a>(state: &'a RandrState, name: &str) -> Result<&'a OutputState> {
    state
        .outputs
        .iter()
        .find(|output| output.name == name)
        .ok_or_else(|| AttachError::unavailable(format!("output '{name}' is unavailable")))
}

fn select_mode(
    modes: &[ModeInfo],
    output: &OutputState,
    request: ModeRequest,
) -> Result<SelectedMode> {
    let mode_id = match request {
        ModeRequest::Preferred => output
            .modes
            .get(0..output.preferred_count)
            .and_then(|preferred| preferred.first())
            .or_else(|| output.modes.first())
            .copied(),
        ModeRequest::Explicit {
            width,
            height,
            rate,
        } => output.modes.iter().copied().find(|mode_id| {
            modes
                .iter()
                .find(|mode| mode.id == *mode_id)
                .is_some_and(|mode| mode_matches(mode, width, height, rate))
        }),
    }
    .ok_or_else(|| AttachError::unavailable(format!("no matching mode for '{}'", output.name)))?;

    let mode = mode_by_id(modes, mode_id).ok_or_else(|| {
        AttachError::unavailable(format!(
            "mode id {mode_id} for output '{}' was not reported in screen resources",
            output.name
        ))
    })?;

    Ok(SelectedMode {
        id: mode.id,
        width: mode.width,
        height: mode.height,
    })
}

fn mode_matches(mode: &ModeInfo, width: u16, height: u16, rate: Option<f64>) -> bool {
    mode.width == width
        && mode.height == height
        && rate.is_none_or(|requested| {
            refresh_rate(mode).is_some_and(|actual| (actual - requested).abs() < 0.5)
        })
}

fn refresh_rate(mode: &ModeInfo) -> Option<f64> {
    let total = u32::from(mode.htotal).checked_mul(u32::from(mode.vtotal))?;
    if total == 0 {
        return None;
    }
    Some(f64::from(mode.dot_clock) / f64::from(total))
}

fn mode_by_id(modes: &[ModeInfo], mode_id: Mode) -> Option<&ModeInfo> {
    modes.iter().find(|mode| mode.id == mode_id)
}

fn choose_crtc(output: &OutputState, crtcs: &[CrtcState]) -> Result<Crtc> {
    if output.crtc != DISABLED_CRTC {
        return Ok(output.crtc);
    }

    let used_crtcs: HashSet<Crtc> = crtcs
        .iter()
        .filter(|crtc| crtc.mode != DISABLED_MODE || !crtc.outputs.is_empty())
        .map(|crtc| crtc.id)
        .collect();

    output
        .possible_crtcs
        .iter()
        .copied()
        .find(|crtc| !used_crtcs.contains(crtc))
        .ok_or_else(|| AttachError::unavailable(format!("no unused CRTC for '{}'", output.name)))
}

fn output_already_satisfied(
    state: &RandrState,
    output: &OutputState,
    crtc_id: Crtc,
    mode: SelectedMode,
    request: &OnRequest,
) -> bool {
    let Some(crtc) = state.crtcs.iter().find(|crtc| crtc.id == crtc_id) else {
        return false;
    };
    crtc.mode == mode.id
        && crtc.x == request.x
        && crtc.y == request.y
        && crtc.rotation == request.rotation.to_randr()
        && crtc.outputs == [output.id]
}

fn check_set_config(status: SetConfig) -> Result<()> {
    if status == SetConfig::SUCCESS {
        Ok(())
    } else {
        Err(AttachError::randr(format!(
            "RandR SetCrtcConfig failed with status {status:?}"
        )))
    }
}

fn checked_extent(offset: i16, size: u16) -> Result<u16> {
    if offset < 0 {
        return Err(AttachError::unavailable(
            "negative output positions cannot be represented by the RandR root size",
        ));
    }
    let extent = u32::from(offset as u16) + u32::from(size);
    u16::try_from(extent).map_err(|_| AttachError::unavailable("output bounds exceed u16 range"))
}

fn scaled_mm(current_mm: u16, current_pixels: u16, new_pixels: u16) -> u32 {
    if current_mm == 0 || current_pixels == 0 {
        return u32::from(new_pixels);
    }
    (u32::from(current_mm) * u32::from(new_pixels) / u32::from(current_pixels)).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use x11rb::protocol::randr::ModeFlag;

    fn mode(id: Mode, width: u16, height: u16, dot_clock: u32) -> ModeInfo {
        ModeInfo {
            id,
            width,
            height,
            dot_clock,
            hsync_start: 0,
            hsync_end: 0,
            htotal: 2200,
            hskew: 0,
            vsync_start: 0,
            vsync_end: 0,
            vtotal: 1125,
            name_len: 0,
            mode_flags: ModeFlag::from(0_u32),
        }
    }

    fn output(modes: Vec<Mode>, preferred_count: usize) -> OutputState {
        OutputState {
            id: 1,
            name: "HDMI-1".to_string(),
            connected: true,
            crtc: DISABLED_CRTC,
            possible_crtcs: vec![7],
            modes,
            preferred_count,
        }
    }

    #[test]
    fn parses_required_on_preferred_command() {
        let command = parse_args(["on", "--output", "HDMI-1", "--preferred"]).unwrap();
        assert_eq!(
            command,
            Command::On(OnRequest {
                output: "HDMI-1".to_string(),
                mode: ModeRequest::Preferred,
                x: 0,
                y: 0,
                rotation: RotationRequest::Normal
            })
        );
    }

    #[test]
    fn rejects_preferred_with_explicit_mode() {
        let error = parse_args([
            "on",
            "--output",
            "HDMI-1",
            "--preferred",
            "--width",
            "1920",
            "--height",
            "1080",
        ])
        .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Usage);
    }

    #[test]
    fn selects_preferred_mode_without_intermediate_fallback() {
        let modes = vec![
            mode(11, 1920, 1080, 148_500_000),
            mode(12, 1280, 720, 74_250_000),
        ];
        let selected =
            select_mode(&modes, &output(vec![11, 12], 1), ModeRequest::Preferred).unwrap();
        assert_eq!(
            selected,
            SelectedMode {
                id: 11,
                width: 1920,
                height: 1080
            }
        );
    }

    #[test]
    fn selects_explicit_mode_with_refresh_rate() {
        let modes = vec![
            mode(11, 1920, 1080, 148_500_000),
            mode(12, 1920, 1080, 74_250_000),
        ];
        let selected = select_mode(
            &modes,
            &output(vec![11, 12], 1),
            ModeRequest::Explicit {
                width: 1920,
                height: 1080,
                rate: Some(30.0),
            },
        )
        .unwrap();
        assert_eq!(selected.id, 12);
    }

    #[test]
    fn chooses_unused_allowed_crtc() {
        let selected = choose_crtc(
            &output(vec![11], 1),
            &[
                CrtcState {
                    id: 7,
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 0,
                    mode: DISABLED_MODE,
                    rotation: Rotation::ROTATE0,
                    outputs: vec![],
                },
                CrtcState {
                    id: 8,
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                    mode: 11,
                    rotation: Rotation::ROTATE0,
                    outputs: vec![2],
                },
            ],
        )
        .unwrap();
        assert_eq!(selected, 7);
    }

    #[test]
    fn parses_auto_config() {
        let config: DisplayConfig = serde_json::from_str(
            r#"{
                "outputs": [
                    {"name": "HDMI-1", "width": 1920, "height": 1080, "rate": 60.0},
                    {"name": "DP-1", "enabled": false}
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(config.outputs.len(), 2);
        assert!(config.outputs[0].enabled);
        assert!(!config.outputs[1].enabled);
    }
}
