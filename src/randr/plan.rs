use super::apply::select_on_target;
use super::mode::find_output;
use super::state::{RandrState, SelectedMode, DISABLED_CRTC};
use crate::{CommandResult, DisplayConfig, ExitStatus, OnRequest, Result};
pub(super) fn plan_enforce(state: &RandrState, config: &DisplayConfig) -> Result<CommandResult> {
    let mut changed = false;
    let mut found_connected_enabled = false;
    let mut messages = Vec::new();

    for configured in &config.outputs {
        let output = find_output(state, &configured.name)?;
        if configured.enabled {
            if !output.connected {
                messages.push(format!("{} skipped disconnected", configured.name));
                continue;
            }
            found_connected_enabled = true;
            let request = configured.on_request()?;
            let target = select_on_target(state, &request)?;
            if target.already_satisfied {
                messages.push(format!("{} already satisfied", configured.name));
            } else {
                changed = true;
                messages.push(format_on_plan(&request, target.mode));
            }
        } else if output.crtc == DISABLED_CRTC {
            messages.push(format!("{} already disabled", configured.name));
        } else {
            changed = true;
            messages.push(format!("{} disable", configured.name));
        }
    }

    let status = if changed {
        ExitStatus::Changed
    } else if found_connected_enabled {
        ExitStatus::AlreadySatisfied
    } else {
        ExitStatus::NoConfiguredConnectedOutput
    };
    Ok(CommandResult::with_messages(status, messages))
}

fn format_on_plan(request: &OnRequest, mode: SelectedMode) -> String {
    format!(
        "{} set {}x{}+{}+{} rotate {}",
        request.output,
        mode.width,
        mode.height,
        request.x,
        request.y,
        request.rotation.as_str()
    )
}

#[cfg(test)]
mod tests {
    use super::super::state::{CrtcState, OutputState, DISABLED_MODE};
    use super::super::test_support::mode;
    use super::*;
    use x11rb::protocol::randr::Rotation;
    #[test]
    fn plan_enforce_reports_changed_enabled_output() {
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
                crtc: DISABLED_CRTC,
                possible_crtcs: vec![7],
                modes: vec![11],
                preferred_count: 1,
            }],
            crtcs: vec![CrtcState {
                id: 7,
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                mode: DISABLED_MODE,
                rotation: Rotation::ROTATE0,
                outputs: Vec::new(),
            }],
            modes: vec![mode(11, 1920, 1080, 148_500_000)],
        };
        let config = DisplayConfig {
            schema_version: None,
            outputs: vec![crate::ConfiguredOutput {
                name: "HDMI-1".to_string(),
                enabled: true,
                width: Some(1920),
                height: Some(1080),
                rate: None,
                x: 0,
                y: 0,
                rotation: crate::RotationRequest::Left,
            }],
        };

        let result = plan_enforce(&state, &config).unwrap();

        assert_eq!(result.status(), ExitStatus::Changed);
        assert_eq!(result.messages(), &["HDMI-1 set 1920x1080+0+0 rotate left"]);
    }

    #[test]
    fn plan_enforce_reports_already_satisfied_output() {
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
                modes: vec![11],
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
            modes: vec![mode(11, 1920, 1080, 148_500_000)],
        };
        let config = DisplayConfig {
            schema_version: None,
            outputs: vec![crate::ConfiguredOutput {
                name: "HDMI-1".to_string(),
                enabled: true,
                width: Some(1920),
                height: Some(1080),
                rate: None,
                x: 0,
                y: 0,
                rotation: crate::RotationRequest::Normal,
            }],
        };

        let result = plan_enforce(&state, &config).unwrap();

        assert_eq!(result.status(), ExitStatus::AlreadySatisfied);
        assert_eq!(result.messages(), &["HDMI-1 already satisfied"]);
    }

    #[test]
    fn plan_enforce_reports_disabled_output_change() {
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
                connected: true,
                crtc: 7,
                possible_crtcs: vec![7],
                modes: vec![11],
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
            modes: vec![mode(11, 1920, 1080, 148_500_000)],
        };
        let config = DisplayConfig {
            schema_version: None,
            outputs: vec![crate::ConfiguredOutput {
                name: "DP-1".to_string(),
                enabled: false,
                width: None,
                height: None,
                rate: None,
                x: 0,
                y: 0,
                rotation: crate::RotationRequest::Normal,
            }],
        };

        let result = plan_enforce(&state, &config).unwrap();

        assert_eq!(result.status(), ExitStatus::Changed);
        assert_eq!(result.messages(), &["DP-1 disable"]);
    }
}
