use super::mode::{choose_crtc, find_output, output_already_satisfied, select_mode};
use super::root::{self, scaled_mm};
use super::state::{RandrState, SelectedMode, CURRENT_TIME, DISABLED_CRTC, DISABLED_MODE};
use super::X11Randr;
use crate::{AttachError, CommandResult, ExitStatus, OnRequest, Result};
use x11rb::connection::Connection;
use x11rb::protocol::randr::{ConnectionExt as RandrConnectionExt, Crtc, Rotation, SetConfig};

#[derive(Debug, Clone, Copy)]
pub(super) struct OnTarget {
    pub(super) output_id: x11rb::protocol::randr::Output,
    pub(super) crtc: Crtc,
    pub(super) mode: SelectedMode,
    pub(super) already_satisfied: bool,
}

/// The operations needed by the output application sequence, independent of X11 transport.
pub(super) trait ApplyBackend {
    fn load_state(&self) -> Result<RandrState>;
    fn set_screen_size(&self, state: &RandrState, width: u16, height: u16) -> Result<()>;
    fn apply_selected_on(
        &self,
        state: &RandrState,
        target: OnTarget,
        request: &OnRequest,
    ) -> Result<CommandResult>;
}

impl ApplyBackend for X11Randr {
    fn load_state(&self) -> Result<RandrState> {
        X11Randr::load_state(self)
    }

    fn set_screen_size(&self, state: &RandrState, width: u16, height: u16) -> Result<()> {
        X11Randr::set_screen_size(self, state, width, height)
    }

    fn apply_selected_on(
        &self,
        state: &RandrState,
        target: OnTarget,
        request: &OnRequest,
    ) -> Result<CommandResult> {
        X11Randr::apply_selected_on(self, state, target, request)
    }
}

pub(super) fn apply_on(backend: &impl ApplyBackend, request: &OnRequest) -> Result<CommandResult> {
    let initial = backend.load_state()?;
    let mut target = select_on_target(&initial, request)?;
    if target.already_satisfied {
        return Ok(CommandResult::new(ExitStatus::AlreadySatisfied));
    }

    let required = root::required_size(target.mode, request)?;
    let expanded = root::expand_if_needed(
        (initial.root_width, initial.root_height),
        required,
        |width, height| backend.set_screen_size(&initial, width, height),
    )?;
    let refreshed;
    let state = if expanded {
        refreshed = backend.load_state()?;
        target = select_on_target(&refreshed, request)?;
        if target.already_satisfied {
            return Ok(CommandResult::new(ExitStatus::AlreadySatisfied));
        }
        &refreshed
    } else {
        &initial
    };
    backend.apply_selected_on(state, target, request)
}

impl X11Randr {
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

    pub(super) fn apply_off(&self, state: &RandrState, output_name: &str) -> Result<CommandResult> {
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

    fn shrink_root_after_disable(&self, state: &RandrState, disabled_crtc: Crtc) -> Result<()> {
        let (width, height) = root::remaining_size(&state.crtcs, disabled_crtc)?;

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
pub(super) fn select_on_target(state: &RandrState, request: &OnRequest) -> Result<OnTarget> {
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

#[cfg(test)]
mod tests {
    use super::super::state::{CrtcState, OutputState};
    use super::super::test_support::mode;
    use super::*;
    use crate::{ModeRequest, RotationRequest};
    use std::cell::RefCell;
    use std::collections::VecDeque;

    #[derive(Debug, PartialEq)]
    enum Operation {
        Load,
        Resize(u16, u16),
        Apply {
            timestamp: u32,
            crtc: Crtc,
            mode: u32,
        },
    }

    struct MockBackend {
        snapshots: RefCell<VecDeque<RandrState>>,
        operations: RefCell<Vec<Operation>>,
        resize_fails: bool,
    }

    impl MockBackend {
        fn new(snapshots: Vec<RandrState>) -> Self {
            Self {
                snapshots: RefCell::new(snapshots.into()),
                operations: RefCell::new(Vec::new()),
                resize_fails: false,
            }
        }
    }

    impl ApplyBackend for MockBackend {
        fn load_state(&self) -> Result<RandrState> {
            self.operations.borrow_mut().push(Operation::Load);
            Ok(self
                .snapshots
                .borrow_mut()
                .pop_front()
                .expect("unexpected state reload"))
        }

        fn set_screen_size(&self, state: &RandrState, width: u16, height: u16) -> Result<()> {
            self.operations
                .borrow_mut()
                .push(Operation::Resize(width, height));
            if self.resize_fails
                || state.crtcs.iter().any(|crtc| {
                    crtc.mode != DISABLED_MODE
                        && (i32::from(crtc.x) + i32::from(crtc.width) > i32::from(width)
                            || i32::from(crtc.y) + i32::from(crtc.height) > i32::from(height))
                })
            {
                return Err(AttachError::randr("SetScreenSize: BadMatch"));
            }
            Ok(())
        }

        fn apply_selected_on(
            &self,
            state: &RandrState,
            target: OnTarget,
            _: &OnRequest,
        ) -> Result<CommandResult> {
            self.operations.borrow_mut().push(Operation::Apply {
                timestamp: state.config_timestamp,
                crtc: target.crtc,
                mode: target.mode.id,
            });
            Ok(CommandResult::new(ExitStatus::Changed))
        }
    }

    fn snapshot(root: (u16, u16), active_mode: u32) -> RandrState {
        RandrState {
            root: 1,
            config_timestamp: 1,
            root_width: root.0,
            root_height: root.1,
            root_mm_width: 480,
            root_mm_height: 270,
            outputs: vec![OutputState {
                id: 2,
                name: "DP-1".into(),
                connected: true,
                crtc: 7,
                possible_crtcs: vec![7],
                modes: vec![11, 22],
                preferred_count: 1,
            }],
            crtcs: vec![CrtcState {
                id: 7,
                x: 0,
                y: 0,
                width: if active_mode == 22 { 3840 } else { 1920 },
                height: if active_mode == 22 { 2160 } else { 1080 },
                mode: active_mode,
                rotation: Rotation::ROTATE0,
                outputs: vec![2],
            }],
            modes: vec![
                mode(11, 1920, 1080, 148_500_000),
                mode(22, 3840, 2160, 594_000_000),
            ],
        }
    }

    fn request(x: i16) -> OnRequest {
        OnRequest {
            output: "DP-1".into(),
            mode: ModeRequest::Preferred,
            x,
            y: 0,
            rotation: RotationRequest::Normal,
        }
    }

    #[test]
    fn applying_smaller_mode_to_enlarged_root_does_not_resize() {
        let backend = MockBackend::new(vec![snapshot((3840, 2160), 22)]);
        assert_eq!(
            apply_on(&backend, &request(1920)).unwrap().status(),
            ExitStatus::Changed
        );
        assert_eq!(
            *backend.operations.borrow(),
            vec![
                Operation::Load,
                Operation::Apply {
                    timestamp: 1,
                    crtc: 7,
                    mode: 11
                }
            ]
        );
    }

    #[test]
    fn expansion_reloads_state_and_reselects_crtc_and_mode() {
        let mut refreshed = snapshot((3840, 2160), 22);
        refreshed.config_timestamp = 2;
        refreshed.outputs[0].crtc = 8;
        refreshed.outputs[0].modes = vec![33];
        refreshed.crtcs[0].id = 8;
        refreshed.modes.push(mode(33, 1920, 1080, 148_500_000));
        let backend = MockBackend::new(vec![snapshot((1920, 1080), 11), refreshed]);
        apply_on(&backend, &request(1920)).unwrap();
        assert_eq!(
            *backend.operations.borrow(),
            vec![
                Operation::Load,
                Operation::Resize(3840, 1080),
                Operation::Load,
                Operation::Apply {
                    timestamp: 2,
                    crtc: 8,
                    mode: 33
                }
            ]
        );
    }

    #[test]
    fn already_satisfied_output_is_not_written() {
        let backend = MockBackend::new(vec![snapshot((1920, 1080), 11)]);
        assert_eq!(
            apply_on(&backend, &request(0)).unwrap().status(),
            ExitStatus::AlreadySatisfied
        );
        assert_eq!(*backend.operations.borrow(), vec![Operation::Load]);
    }

    #[test]
    fn output_satisfied_after_expansion_is_not_written() {
        let mut refreshed = snapshot((3840, 1080), 11);
        refreshed.crtcs[0].x = 1920;
        let backend = MockBackend::new(vec![snapshot((1920, 1080), 11), refreshed]);
        assert_eq!(
            apply_on(&backend, &request(1920)).unwrap().status(),
            ExitStatus::AlreadySatisfied
        );
        assert_eq!(
            *backend.operations.borrow(),
            vec![
                Operation::Load,
                Operation::Resize(3840, 1080),
                Operation::Load
            ]
        );
    }

    #[test]
    fn resize_failure_stops_before_reload_or_crtc_update() {
        let mut backend = MockBackend::new(vec![snapshot((1920, 1080), 11)]);
        backend.resize_fails = true;
        assert_eq!(
            apply_on(&backend, &request(1920)).unwrap_err().kind(),
            crate::ErrorKind::RandrFailed
        );
        assert_eq!(
            *backend.operations.borrow(),
            vec![Operation::Load, Operation::Resize(3840, 1080)]
        );
    }

    #[test]
    fn set_config_status_maps_non_success_to_randr_error() {
        let error = check_set_config(SetConfig::INVALID_CONFIG_TIME).unwrap_err();
        assert_eq!(error.kind(), crate::ErrorKind::RandrFailed);
    }
}
