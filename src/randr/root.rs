use super::state::{CrtcState, SelectedMode, DISABLED_MODE};
use crate::OnRequest;
use crate::{AttachError, Result};
use x11rb::protocol::randr::Crtc;
use x11rb::protocol::xproto::Screen;

pub(super) struct Dimensions {
    pub width: u16,
    pub height: u16,
    pub mm_width: u16,
    pub mm_height: u16,
}

pub(super) fn load_dimensions(
    screen: &Screen,
    get_geometry: impl FnOnce() -> Result<(u16, u16)>,
) -> Result<Dimensions> {
    // Setup dimensions are fixed at connection time; only use them for density.
    let (width, height) = get_geometry()?;
    Ok(Dimensions {
        width,
        height,
        mm_width: clamp_mm(scaled_mm(
            screen.width_in_millimeters,
            screen.width_in_pixels,
            width,
        )),
        mm_height: clamp_mm(scaled_mm(
            screen.height_in_millimeters,
            screen.height_in_pixels,
            height,
        )),
    })
}

pub(super) fn expand_if_needed(
    current: (u16, u16),
    required: (u16, u16),
    set_screen_size: impl FnOnce(u16, u16) -> Result<()>,
) -> Result<bool> {
    let size = (current.0.max(required.0), current.1.max(required.1));
    if size == current {
        return Ok(false);
    }
    set_screen_size(size.0, size.1)?;
    Ok(true)
}

pub(super) fn required_size(mode: SelectedMode, request: &OnRequest) -> Result<(u16, u16)> {
    let (width, height) = request.rotation.dimensions(mode.width, mode.height);
    Ok((
        checked_extent(request.x, width)?,
        checked_extent(request.y, height)?,
    ))
}

pub(super) fn remaining_size(crtcs: &[CrtcState], disabled_crtc: Crtc) -> Result<(u16, u16)> {
    let mut width = 1;
    let mut height = 1;
    for crtc in crtcs {
        if crtc.id == disabled_crtc || crtc.mode == DISABLED_MODE {
            continue;
        }
        width = width.max(checked_extent(crtc.x, crtc.width)?);
        height = height.max(checked_extent(crtc.y, crtc.height)?);
    }
    Ok((width, height))
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

fn clamp_mm(mm: u32) -> u16 {
    mm.clamp(1, u32::from(u16::MAX)) as u16
}

pub(super) fn scaled_mm(current_mm: u16, current_pixels: u16, new_pixels: u16) -> u32 {
    if current_mm == 0 || current_pixels == 0 {
        return u32::from(new_pixels);
    }
    (u32::from(current_mm) * u32::from(new_pixels) / u32::from(current_pixels)).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AttachError;
    use crate::{ModeRequest, RotationRequest};
    use std::cell::{Cell, RefCell};
    use x11rb::protocol::randr::Rotation;

    #[test]
    fn required_size_includes_rotation_and_position() {
        let request = OnRequest {
            output: "DP-1".into(),
            mode: ModeRequest::Preferred,
            x: 1920,
            y: 100,
            rotation: RotationRequest::Left,
        };
        let mode = SelectedMode {
            id: 11,
            width: 1920,
            height: 1080,
        };
        assert_eq!(required_size(mode, &request).unwrap(), (3000, 2020));
    }

    #[test]
    fn remaining_size_excludes_disabled_and_inactive_crtcs() {
        let active = CrtcState {
            id: 7,
            x: 100,
            y: 200,
            width: 1920,
            height: 1080,
            mode: 11,
            rotation: Rotation::ROTATE0,
            outputs: vec![2],
        };
        let disabled = CrtcState {
            id: 8,
            width: 3840,
            height: 2160,
            ..active.clone()
        };
        let inactive = CrtcState {
            id: 9,
            mode: DISABLED_MODE,
            width: 5000,
            ..active.clone()
        };
        assert_eq!(
            remaining_size(&[active, disabled, inactive], 8).unwrap(),
            (2020, 1280)
        );
    }

    #[test]
    fn no_remaining_crtcs_uses_minimum_root_size() {
        assert_eq!(remaining_size(&[], 7).unwrap(), (1, 1));
    }

    struct MockRoot {
        geometry: Cell<(u16, u16)>,
        requests: RefCell<Vec<(u16, u16)>>,
    }

    impl MockRoot {
        fn set_screen_size(&self, width: u16, height: u16) -> Result<()> {
            self.requests.borrow_mut().push((width, height));
            let active_crtc = self.geometry.get();
            if width < active_crtc.0 || height < active_crtc.1 {
                return Err(AttachError::randr("SetScreenSize: BadMatch"));
            }
            self.geometry.set((width, height));
            Ok(())
        }
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
        assert_eq!(scaled_mm(600, 3840, 1920), 300);
        assert_eq!(scaled_mm(1, u16::MAX, 1), 1);
        assert_eq!(scaled_mm(0, 1920, 3840), 3840);
        assert_eq!(scaled_mm(300, 0, 3840), 3840);
    }

    #[test]
    fn physical_size_fits_protocol_range() {
        assert_eq!(clamp_mm(0), 1);
        assert_eq!(clamp_mm(600), 600);
        assert_eq!(clamp_mm(scaled_mm(u16::MAX, 1, u16::MAX)), u16::MAX);
    }

    fn setup_screen() -> Screen {
        Screen {
            width_in_pixels: 1920,
            height_in_pixels: 1080,
            width_in_millimeters: 480,
            height_in_millimeters: 270,
            ..Screen::default()
        }
    }

    #[test]
    fn externally_enlarged_root_is_refreshed_before_applying_smaller_mode() {
        let screen = setup_screen();
        let server = MockRoot {
            geometry: Cell::new((1920, 1080)),
            requests: RefCell::new(Vec::new()),
        };
        let load = || load_dimensions(&screen, || Ok(server.geometry.get()));
        let initial = load().unwrap();
        assert_eq!((initial.width, initial.height), (1920, 1080));

        // Another client changes the root and active CRTC on the same connection.
        server.geometry.set((3840, 2160));
        let current = load().unwrap();
        assert_eq!((current.width, current.height), (3840, 2160));
        assert_eq!((current.mm_width, current.mm_height), (960, 540));
        let changed = expand_if_needed((current.width, current.height), (1920, 1080), |w, h| {
            server.set_screen_size(w, h)
        })
        .unwrap();
        assert!(!changed);
        assert!(server.requests.borrow().is_empty());

        // A 1080p output at x=1920 requires a right edge of 3840.
        assert!(
            !expand_if_needed((current.width, current.height), (3840, 1080), |w, h| server
                .set_screen_size(w, h),)
            .unwrap()
        );
        assert!(server.requests.borrow().is_empty());

        // The old snapshot would request a root too short for the active CRTC.
        assert!(
            expand_if_needed((initial.width, initial.height), (3840, 1080), |w, h| server
                .set_screen_size(w, h),)
            .is_err()
        );
        assert_eq!(*server.requests.borrow(), vec![(3840, 1080)]);
    }

    #[test]
    fn expansion_preserves_current_height() {
        let server = MockRoot {
            geometry: Cell::new((3840, 2160)),
            requests: RefCell::new(Vec::new()),
        };
        assert!(expand_if_needed((3840, 2160), (5760, 1080), |w, h| {
            server.set_screen_size(w, h)
        })
        .unwrap());
        assert_eq!(*server.requests.borrow(), vec![(5760, 2160)]);
    }

    #[test]
    fn geometry_failure_is_propagated() {
        assert!(load_dimensions(&setup_screen(), || {
            Err(AttachError::randr("GetGeometry failed"))
        })
        .is_err());
    }
}
