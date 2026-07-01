use serde::Deserialize;
use std::path::PathBuf;
use x11rb::protocol::randr::{Mode, Rotation};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Status,
    On(OnRequest),
    Off { output: String },
    Auto { config: PathBuf },
    Help,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OnRequest {
    pub output: String,
    pub mode: ModeRequest,
    pub x: i16,
    pub y: i16,
    pub rotation: RotationRequest,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModeRequest {
    Preferred,
    Explicit {
        width: u16,
        height: u16,
        rate: Option<f64>,
    },
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RotationRequest {
    #[default]
    Normal,
    Left,
    Inverted,
    Right,
}

impl RotationRequest {
    pub(crate) fn to_randr(self) -> Rotation {
        match self {
            Self::Normal => Rotation::ROTATE0,
            Self::Left => Rotation::ROTATE90,
            Self::Inverted => Rotation::ROTATE180,
            Self::Right => Rotation::ROTATE270,
        }
    }

    pub(crate) fn dimensions(self, width: u16, height: u16) -> (u16, u16) {
        match self {
            Self::Normal | Self::Inverted => (width, height),
            Self::Left | Self::Right => (height, width),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutputStatus {
    pub name: String,
    pub connected: bool,
    pub active: bool,
    pub current_mode: Option<ModeSummary>,
    pub x: Option<i16>,
    pub y: Option<i16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeSummary {
    pub width: u16,
    pub height: u16,
    pub mode_id: Mode,
}
