use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;
use x11rb::protocol::randr::{Mode, Rotation};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Status,
    On(OnRequest),
    Off {
        output: String,
    },
    Auto {
        config: PathBuf,
    },
    Enforce {
        config: PathBuf,
        dry_run: bool,
        watch: Option<WatchOptions>,
    },
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchOptions {
    pub debounce: Duration,
    pub retry_count: u16,
    pub retry_delay: Duration,
}

impl Default for WatchOptions {
    fn default() -> Self {
        Self {
            debounce: Duration::from_millis(500),
            retry_count: 3,
            retry_delay: Duration::from_millis(1000),
        }
    }
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
    Current,
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

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Left => "left",
            Self::Inverted => "inverted",
            Self::Right => "right",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutputStatus {
    pub name: String,
    pub connected: bool,
    pub active: bool,
    pub current_mode: Option<ModeSummary>,
    pub available_modes: Vec<ModeSummary>,
    pub x: Option<i16>,
    pub y: Option<i16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeSummary {
    pub width: u16,
    pub height: u16,
    pub mode_id: Mode,
    pub refresh_millihertz: Option<u32>,
    pub preferred: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_dimensions_swap_for_quarter_turns() {
        assert_eq!(RotationRequest::Normal.dimensions(1920, 1080), (1920, 1080));
        assert_eq!(
            RotationRequest::Inverted.dimensions(1920, 1080),
            (1920, 1080)
        );
        assert_eq!(RotationRequest::Left.dimensions(1920, 1080), (1080, 1920));
        assert_eq!(RotationRequest::Right.dimensions(1920, 1080), (1080, 1920));
    }
}
