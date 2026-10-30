use x11rb::protocol::randr::{Mode, ModeFlag, ModeInfo};
pub(super) fn mode(id: Mode, width: u16, height: u16, dot_clock: u32) -> ModeInfo {
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
