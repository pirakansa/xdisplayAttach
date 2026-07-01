use x11rb::protocol::randr::{Crtc, Mode, ModeInfo, Output, Rotation};
use x11rb::protocol::xproto::{Timestamp, Window};

pub(super) const CURRENT_TIME: Timestamp = 0;
pub(super) const DISABLED_CRTC: Crtc = 0;
pub(super) const DISABLED_MODE: Mode = 0;

#[derive(Debug, Clone)]
pub(super) struct OutputState {
    pub id: Output,
    pub name: String,
    pub connected: bool,
    pub crtc: Crtc,
    pub possible_crtcs: Vec<Crtc>,
    pub modes: Vec<Mode>,
    pub preferred_count: usize,
}

#[derive(Debug, Clone)]
pub(super) struct CrtcState {
    pub id: Crtc,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub mode: Mode,
    pub rotation: Rotation,
    pub outputs: Vec<Output>,
}

#[derive(Debug, Clone)]
pub(super) struct RandrState {
    pub root: Window,
    pub config_timestamp: Timestamp,
    pub root_width: u16,
    pub root_height: u16,
    pub root_mm_width: u16,
    pub root_mm_height: u16,
    pub outputs: Vec<OutputState>,
    pub crtcs: Vec<CrtcState>,
    pub modes: Vec<ModeInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SelectedMode {
    pub id: Mode,
    pub width: u16,
    pub height: u16,
}
