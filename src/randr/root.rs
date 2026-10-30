use super::{clamp_mm, scaled_mm};
use crate::Result;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AttachError;
    use std::cell::{Cell, RefCell};

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
