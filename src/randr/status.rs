use super::mode::{mode_by_id, refresh_millihertz};
use super::state::{OutputState, RandrState, DISABLED_CRTC};
use crate::{ModeSummary, OutputStatus};
pub(super) fn output_statuses(state: &RandrState) -> Vec<OutputStatus> {
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

#[cfg(test)]
mod tests {
    use super::super::state::CrtcState;
    use super::super::test_support::mode;
    use super::*;
    use x11rb::protocol::randr::Rotation;
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
}
