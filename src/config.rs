use crate::{AttachError, ModeRequest, OnRequest, Result, RotationRequest};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct DisplayConfig {
    pub outputs: Vec<ConfiguredOutput>,
}

#[derive(Debug, Deserialize)]
pub struct ConfiguredOutput {
    pub name: String,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub rate: Option<f64>,
    #[serde(default)]
    pub x: i16,
    #[serde(default)]
    pub y: i16,
    #[serde(default)]
    pub rotation: RotationRequest,
}

pub fn read_config(path: &Path) -> Result<DisplayConfig> {
    let content = fs::read_to_string(path).map_err(|error| {
        AttachError::usage(format!("failed to read '{}': {error}", path.display()))
    })?;
    serde_json::from_str(&content).map_err(|error| {
        AttachError::usage(format!("failed to parse '{}': {error}", path.display()))
    })
}

fn enabled_by_default() -> bool {
    true
}

impl ConfiguredOutput {
    pub(crate) fn on_request(&self) -> Result<OnRequest> {
        let mode = match (self.width, self.height, self.rate) {
            (Some(width), Some(height), rate) => ModeRequest::Explicit {
                width,
                height,
                rate,
            },
            (None, None, None) => ModeRequest::Preferred,
            _ => {
                return Err(AttachError::usage(format!(
                    "output '{}' must set both width and height, or neither",
                    self.name
                )))
            }
        };

        Ok(OnRequest {
            output: self.name.clone(),
            mode,
            x: self.x,
            y: self.y,
            rotation: self.rotation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_auto_config() {
        let config: DisplayConfig = serde_json::from_str(
            r#"{
                "outputs": [
                    {"name": "HDMI-1", "width": 1920, "height": 1080, "rate": 60.0},
                    {"name": "DP-1", "enabled": false}
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(config.outputs.len(), 2);
        assert!(config.outputs[0].enabled);
        assert!(!config.outputs[1].enabled);
    }

    #[test]
    fn defaults_enabled_position_and_rotation() {
        let config: DisplayConfig = serde_json::from_str(
            r#"{
                "outputs": [
                    {"name": "HDMI-1"}
                ]
            }"#,
        )
        .unwrap();

        let request = config.outputs[0].on_request().unwrap();
        assert!(config.outputs[0].enabled);
        assert_eq!(request.mode, ModeRequest::Preferred);
        assert_eq!(request.x, 0);
        assert_eq!(request.y, 0);
        assert_eq!(request.rotation, RotationRequest::Normal);
    }

    #[test]
    fn converts_explicit_config_to_on_request() {
        let config: DisplayConfig = serde_json::from_str(
            r#"{
                "outputs": [
                    {
                        "name": "HDMI-1",
                        "width": 1280,
                        "height": 720,
                        "rate": 60.0,
                        "x": 10,
                        "y": 20,
                        "rotation": "left"
                    }
                ]
            }"#,
        )
        .unwrap();

        let request = config.outputs[0].on_request().unwrap();
        assert_eq!(request.output, "HDMI-1");
        assert_eq!(
            request.mode,
            ModeRequest::Explicit {
                width: 1280,
                height: 720,
                rate: Some(60.0)
            }
        );
        assert_eq!(request.x, 10);
        assert_eq!(request.y, 20);
        assert_eq!(request.rotation, RotationRequest::Left);
    }

    #[test]
    fn rejects_partial_dimensions() {
        let config: DisplayConfig = serde_json::from_str(
            r#"{
                "outputs": [
                    {"name": "HDMI-1", "width": 1920}
                ]
            }"#,
        )
        .unwrap();

        let error = config.outputs[0].on_request().unwrap_err();
        assert_eq!(error.kind(), crate::ErrorKind::Usage);
    }
}
