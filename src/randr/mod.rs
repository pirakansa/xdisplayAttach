mod mode;
mod state;

use self::mode::{choose_crtc, find_output, mode_by_id, output_already_satisfied, select_mode};
use self::state::{
    CrtcState, OutputState, RandrState, SelectedMode, CURRENT_TIME, DISABLED_CRTC, DISABLED_MODE,
};
use crate::{AttachError, DisplayConfig, ExitStatus, ModeSummary, OnRequest, OutputStatus, Result};
use x11rb::connection::Connection;
use x11rb::protocol::randr::{
    Connection as OutputConnection, ConnectionExt as RandrConnectionExt, Crtc, Rotation, SetConfig,
};
use x11rb::rust_connection::RustConnection;

pub(crate) struct X11Randr {
    conn: RustConnection,
    screen_num: usize,
}

impl X11Randr {
    pub(crate) fn connect() -> Result<Self> {
        let (conn, screen_num) =
            x11rb::connect(None).map_err(|error| AttachError::xorg(error.to_string()))?;
        conn.randr_query_version(1, 5)
            .map_err(|error| AttachError::xorg(format!("RandR is unavailable: {error}")))?
            .reply()
            .map_err(|error| AttachError::xorg(format!("RandR is unavailable: {error}")))?;
        Ok(Self { conn, screen_num })
    }

    pub(crate) fn status(&self) -> Result<Vec<OutputStatus>> {
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

    pub(crate) fn turn_on(&self, request: &OnRequest) -> Result<ExitStatus> {
        let state = self.load_state()?;
        self.apply_on(&state, request)
    }

    pub(crate) fn turn_off(&self, output_name: &str) -> Result<ExitStatus> {
        let state = self.load_state()?;
        self.apply_off(&state, output_name)
    }

    pub(crate) fn auto(&self, config: &DisplayConfig) -> Result<ExitStatus> {
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
