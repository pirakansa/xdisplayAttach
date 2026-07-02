mod auto;
mod mode;
mod state;
mod touch;

use self::auto::apply_auto;
use self::mode::{
    choose_crtc, find_output, mode_by_id, output_already_satisfied, refresh_millihertz, select_mode,
};
use self::state::{
    CrtcState, OutputState, RandrState, SelectedMode, CURRENT_TIME, DISABLED_CRTC, DISABLED_MODE,
};
use crate::{
    AttachError, CommandResult, DisplayConfig, ExitStatus, ModeSummary, OnRequest, OutputStatus,
    Result,
};
use x11rb::connection::Connection;
use x11rb::protocol::randr::{
    Connection as OutputConnection, ConnectionExt as RandrConnectionExt, Crtc, Rotation, SetConfig,
};
use x11rb::rust_connection::RustConnection;

pub(crate) struct X11Randr {
    conn: RustConnection,
    screen_num: usize,
}

#[derive(Debug, Clone, Copy)]
struct OnTarget {
    output_id: x11rb::protocol::randr::Output,
    crtc: Crtc,
    mode: SelectedMode,
    already_satisfied: bool,
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
        Ok(output_statuses(&state))
    }

    pub(crate) fn turn_on(&self, request: &OnRequest) -> Result<CommandResult> {
        let state = self.load_state()?;
        self.apply_on(&state, request)
    }

    pub(crate) fn turn_off(&self, output_name: &str) -> Result<CommandResult> {
        let state = self.load_state()?;
        self.apply_off(&state, output_name)
    }

    pub(crate) fn auto(&self, config: &DisplayConfig) -> Result<CommandResult> {
        apply_auto(config, self)
    }

    fn apply_on(&self, state: &RandrState, request: &OnRequest) -> Result<CommandResult> {
        let mut target = select_on_target(state, request)?;
        let active_state;
        let state = if target.already_satisfied {
            return Ok(CommandResult::new(ExitStatus::AlreadySatisfied));
        } else if self.expand_root_if_needed(state, target.mode, request)? {
            active_state = self.load_state()?;
            target = select_on_target(&active_state, request)?;
            if target.already_satisfied {
                return Ok(CommandResult::new(ExitStatus::AlreadySatisfied));
            }
            &active_state
        } else {
            state
        };

        self.apply_selected_on(state, target, request)
    }

    fn apply_selected_on(
        &self,
        state: &RandrState,
        target: OnTarget,
        request: &OnRequest,
    ) -> Result<CommandResult> {
        let outputs = [target.output_id];
        let reply = self
            .conn
            .randr_set_crtc_config(
                target.crtc,
                CURRENT_TIME,
                state.config_timestamp,
                request.x,
                request.y,
                target.mode.id,
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
        let mut result = CommandResult::new(ExitStatus::Changed);
        if let Err(error) = self.remap_touch_devices_to_output(&request.output, request) {
            result.extend_warnings(vec![format!(
                "output mode changed, but touch remapping failed: {error}"
            )]);
        }
        Ok(result)
    }

    fn apply_off(&self, state: &RandrState, output_name: &str) -> Result<CommandResult> {
        let output = find_output(state, output_name)?;
        if output.crtc == DISABLED_CRTC {
            return Ok(CommandResult::new(ExitStatus::AlreadySatisfied));
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
        Ok(CommandResult::new(ExitStatus::Changed))
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
    ) -> Result<bool> {
        let (width, height) = request.rotation.dimensions(mode.width, mode.height);
        let right = checked_extent(request.x, width)?;
        let bottom = checked_extent(request.y, height)?;
        let new_width = right.max(state.root_width);
        let new_height = bottom.max(state.root_height);
        if new_width == state.root_width && new_height == state.root_height {
            return Ok(false);
        }
        self.set_screen_size(state, new_width, new_height)?;
        Ok(true)
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

fn output_statuses(state: &RandrState) -> Vec<OutputStatus> {
    state
        .outputs
        .iter()
        .map(|output| {
            let crtc = state.crtcs.iter().find(|crtc| crtc.id == output.crtc);
            OutputStatus {
                name: output.name.clone(),
                connected: output.connected,
                active: output.crtc != DISABLED_CRTC,
                current_mode: crtc.and_then(|crtc| mode_summary(state, output, crtc.mode)),
                available_modes: output
                    .modes
                    .iter()
                    .copied()
                    .filter_map(|mode_id| mode_summary(state, output, mode_id))
                    .collect(),
                x: crtc.map(|crtc| crtc.x),
                y: crtc.map(|crtc| crtc.y),
            }
        })
        .collect()
}

fn mode_summary(
    state: &RandrState,
    output: &OutputState,
    mode_id: x11rb::protocol::randr::Mode,
) -> Option<ModeSummary> {
    let mode = mode_by_id(&state.modes, mode_id)?;
    Some(ModeSummary {
        width: mode.width,
        height: mode.height,
        mode_id: mode.id,
        refresh_millihertz: refresh_millihertz(mode),
        preferred: output
            .modes
            .get(0..output.preferred_count)
            .is_some_and(|preferred| preferred.contains(&mode_id)),
    })
}

fn select_on_target(state: &RandrState, request: &OnRequest) -> Result<OnTarget> {
    let output = find_output(state, &request.output)?;
    if !output.connected {
        return Err(AttachError::unavailable(format!(
            "output '{}' is not connected",
            request.output
        )));
    }

    let mode = select_mode(state, output, request.mode)?;
    let crtc = choose_crtc(output, &state.crtcs)?;
    Ok(OnTarget {
        output_id: output.id,
        crtc,
        mode,
        already_satisfied: output_already_satisfied(state, output, crtc, mode, request),
    })
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
    use x11rb::protocol::randr::{Mode, ModeFlag, ModeInfo};

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

    #[test]
    fn output_statuses_include_available_modes() {
        let state = RandrState {
            root: 1,
            config_timestamp: 1,
            root_width: 1920,
            root_height: 1080,
            root_mm_width: 300,
            root_mm_height: 200,
            outputs: vec![OutputState {
                id: 2,
                name: "HDMI-1".to_string(),
                connected: true,
                crtc: 7,
                possible_crtcs: vec![7],
                modes: vec![11, 12],
                preferred_count: 1,
            }],
            crtcs: vec![CrtcState {
                id: 7,
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                mode: 11,
                rotation: Rotation::ROTATE0,
                outputs: vec![2],
            }],
            modes: vec![
                mode(11, 1920, 1080, 148_500_000),
                mode(12, 1280, 720, 74_250_000),
            ],
        };

        let statuses = output_statuses(&state);

        assert_eq!(statuses.len(), 1);
        assert_eq!(
            statuses[0].current_mode,
            Some(ModeSummary {
                width: 1920,
                height: 1080,
                mode_id: 11,
                refresh_millihertz: Some(60_000),
                preferred: true,
            })
        );
        assert_eq!(
            statuses[0].available_modes,
            vec![
                ModeSummary {
                    width: 1920,
                    height: 1080,
                    mode_id: 11,
                    refresh_millihertz: Some(60_000),
                    preferred: true,
                },
                ModeSummary {
                    width: 1280,
                    height: 720,
                    mode_id: 12,
                    refresh_millihertz: Some(30_000),
                    preferred: false,
                },
            ]
        );
    }

    #[test]
    fn output_statuses_ignore_unresolved_available_mode_ids() {
        let state = RandrState {
            root: 1,
            config_timestamp: 1,
            root_width: 1920,
            root_height: 1080,
            root_mm_width: 300,
            root_mm_height: 200,
            outputs: vec![OutputState {
                id: 2,
                name: "DP-1".to_string(),
                connected: false,
                crtc: DISABLED_CRTC,
                possible_crtcs: Vec::new(),
                modes: vec![99],
                preferred_count: 1,
            }],
            crtcs: Vec::new(),
            modes: Vec::new(),
        };

        let statuses = output_statuses(&state);

        assert_eq!(statuses[0].available_modes, Vec::new());
    }

    #[test]
    fn checked_extent_rejects_negative_positions() {
        let error = checked_extent(-1, 100).unwrap_err();
        assert_eq!(error.kind(), crate::ErrorKind::Unavailable);
    }

    #[test]
    fn checked_extent_rejects_overflow() {
        let error = checked_extent(i16::MAX, u16::MAX).unwrap_err();
        assert_eq!(error.kind(), crate::ErrorKind::Unavailable);
    }

    #[test]
    fn scales_physical_size_with_pixel_size() {
        assert_eq!(scaled_mm(300, 1920, 3840), 600);
        assert_eq!(scaled_mm(0, 1920, 3840), 3840);
        assert_eq!(scaled_mm(300, 0, 3840), 3840);
    }

    #[test]
    fn set_config_status_maps_non_success_to_randr_error() {
        let error = check_set_config(SetConfig::INVALID_CONFIG_TIME).unwrap_err();
        assert_eq!(error.kind(), crate::ErrorKind::RandrFailed);
    }
}
