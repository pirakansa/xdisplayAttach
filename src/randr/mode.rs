use super::state::{
    CrtcState, OutputState, RandrState, SelectedMode, DISABLED_CRTC, DISABLED_MODE,
};
use crate::{AttachError, ModeRequest, OnRequest, Result};
use std::collections::HashSet;
use x11rb::protocol::randr::{Crtc, Mode, ModeInfo};

pub(super) fn find_output<'a>(state: &'a RandrState, name: &str) -> Result<&'a OutputState> {
    state
        .outputs
        .iter()
        .find(|output| output.name == name)
        .ok_or_else(|| AttachError::unavailable(format!("output '{name}' is unavailable")))
}

pub(super) fn select_mode(
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

pub(super) fn mode_by_id(modes: &[ModeInfo], mode_id: Mode) -> Option<&ModeInfo> {
    modes.iter().find(|mode| mode.id == mode_id)
}

pub(super) fn choose_crtc(output: &OutputState, crtcs: &[CrtcState]) -> Result<Crtc> {
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

pub(super) fn output_already_satisfied(
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

#[cfg(test)]
mod tests {
    use super::*;
    use x11rb::protocol::randr::{ModeFlag, Rotation};

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
    fn falls_back_to_first_mode_when_no_preferred_mode_is_marked() {
        let modes = vec![
            mode(11, 1920, 1080, 148_500_000),
            mode(12, 1280, 720, 74_250_000),
        ];
        let selected =
            select_mode(&modes, &output(vec![11, 12], 0), ModeRequest::Preferred).unwrap();
        assert_eq!(selected.id, 11);
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
    fn rejects_unavailable_explicit_mode() {
        let modes = vec![mode(11, 1920, 1080, 148_500_000)];
        let error = select_mode(
            &modes,
            &output(vec![11], 1),
            ModeRequest::Explicit {
                width: 1024,
                height: 768,
                rate: None,
            },
        )
        .unwrap_err();
        assert_eq!(error.kind(), crate::ErrorKind::Unavailable);
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
    fn rejects_when_all_allowed_crtcs_are_used() {
        let error = choose_crtc(
            &output(vec![11], 1),
            &[CrtcState {
                id: 7,
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                mode: 11,
                rotation: Rotation::ROTATE0,
                outputs: vec![2],
            }],
        )
        .unwrap_err();
        assert_eq!(error.kind(), crate::ErrorKind::Unavailable);
    }

    #[test]
    fn detects_already_satisfied_output_including_rotation() {
        let output = OutputState {
            id: 1,
            name: "HDMI-1".to_string(),
            connected: true,
            crtc: 7,
            possible_crtcs: vec![7],
            modes: vec![11],
            preferred_count: 1,
        };
        let state = RandrState {
            root: 1,
            config_timestamp: 1,
            root_width: 1920,
            root_height: 1080,
            root_mm_width: 300,
            root_mm_height: 200,
            outputs: vec![output.clone()],
            crtcs: vec![CrtcState {
                id: 7,
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                mode: 11,
                rotation: Rotation::ROTATE0,
                outputs: vec![1],
            }],
            modes: vec![mode(11, 1920, 1080, 148_500_000)],
        };
        let request = OnRequest {
            output: "HDMI-1".to_string(),
            mode: ModeRequest::Preferred,
            x: 0,
            y: 0,
            rotation: crate::RotationRequest::Normal,
        };

        assert!(output_already_satisfied(
            &state,
            &output,
            7,
            SelectedMode {
                id: 11,
                width: 1920,
                height: 1080
            },
            &request
        ));

        let rotated_request = OnRequest {
            rotation: crate::RotationRequest::Left,
            ..request
        };
        assert!(!output_already_satisfied(
            &state,
            &output,
            7,
            SelectedMode {
                id: 11,
                width: 1920,
                height: 1080
            },
            &rotated_request
        ));
    }
}
