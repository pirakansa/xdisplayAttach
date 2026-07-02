use super::mode::find_output;
use super::X11Randr;
use crate::{CommandResult, DisplayConfig, ExitStatus, OnRequest, Result};

pub(super) trait AutoBackend {
    fn output_connected(&self, output_name: &str) -> Result<bool>;
    fn apply_on_request(&self, request: &OnRequest) -> Result<CommandResult>;
    fn apply_off_request(&self, output_name: &str) -> Result<CommandResult>;
}

impl AutoBackend for X11Randr {
    fn output_connected(&self, output_name: &str) -> Result<bool> {
        let state = self.load_state()?;
        let output = find_output(&state, output_name)?;
        Ok(output.connected)
    }

    fn apply_on_request(&self, request: &OnRequest) -> Result<CommandResult> {
        self.turn_on(request)
    }

    fn apply_off_request(&self, output_name: &str) -> Result<CommandResult> {
        self.turn_off(output_name)
    }
}

pub(super) fn apply_auto(
    config: &DisplayConfig,
    backend: &impl AutoBackend,
) -> Result<CommandResult> {
    let mut changed = false;
    let mut found_connected_enabled = false;
    let mut warnings = Vec::new();

    for configured in &config.outputs {
        if configured.enabled {
            if !backend.output_connected(&configured.name)? {
                continue;
            }
            found_connected_enabled = true;
            let result = backend.apply_on_request(&configured.on_request()?)?;
            changed |= result.status() == ExitStatus::Changed;
            warnings.extend(result.warnings().iter().cloned());
        } else {
            let result = backend.apply_off_request(&configured.name)?;
            changed |= result.status() == ExitStatus::Changed;
            warnings.extend(result.warnings().iter().cloned());
        }
    }

    let status = if changed {
        ExitStatus::Changed
    } else if found_connected_enabled {
        ExitStatus::AlreadySatisfied
    } else {
        ExitStatus::NoConfiguredConnectedOutput
    };
    let mut result = CommandResult::new(status);
    result.extend_warnings(warnings);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AttachError, ConfiguredOutput, ModeRequest, RotationRequest};
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[derive(Debug, Default)]
    struct FakeBackend {
        connected: HashMap<String, bool>,
        on_results: HashMap<String, ExitStatus>,
        off_results: HashMap<String, ExitStatus>,
        calls: RefCell<Vec<String>>,
    }

    impl FakeBackend {
        fn with_connected(mut self, name: &str, connected: bool) -> Self {
            self.connected.insert(name.to_string(), connected);
            self
        }

        fn with_on_result(mut self, name: &str, status: ExitStatus) -> Self {
            self.on_results.insert(name.to_string(), status);
            self
        }

        fn with_off_result(mut self, name: &str, status: ExitStatus) -> Self {
            self.off_results.insert(name.to_string(), status);
            self
        }

        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl AutoBackend for FakeBackend {
        fn output_connected(&self, output_name: &str) -> Result<bool> {
            self.calls
                .borrow_mut()
                .push(format!("connected:{output_name}"));
            self.connected.get(output_name).copied().ok_or_else(|| {
                AttachError::unavailable(format!("output '{output_name}' is unavailable"))
            })
        }

        fn apply_on_request(&self, request: &OnRequest) -> Result<CommandResult> {
            self.calls
                .borrow_mut()
                .push(format!("on:{}:{:?}", request.output, request.mode));
            Ok(CommandResult::new(
                self.on_results
                    .get(&request.output)
                    .copied()
                    .unwrap_or(ExitStatus::AlreadySatisfied),
            ))
        }

        fn apply_off_request(&self, output_name: &str) -> Result<CommandResult> {
            self.calls.borrow_mut().push(format!("off:{output_name}"));
            Ok(CommandResult::new(
                self.off_results
                    .get(output_name)
                    .copied()
                    .unwrap_or(ExitStatus::AlreadySatisfied),
            ))
        }
    }

    fn config(outputs: Vec<ConfiguredOutput>) -> DisplayConfig {
        DisplayConfig {
            schema_version: None,
            outputs,
        }
    }

    fn enabled_output(name: &str) -> ConfiguredOutput {
        ConfiguredOutput {
            name: name.to_string(),
            enabled: true,
            width: None,
            height: None,
            rate: None,
            x: 0,
            y: 0,
            rotation: RotationRequest::Normal,
        }
    }

    fn disabled_output(name: &str) -> ConfiguredOutput {
        ConfiguredOutput {
            enabled: false,
            ..enabled_output(name)
        }
    }

    #[test]
    fn auto_skips_disconnected_enabled_outputs() {
        let backend = FakeBackend::default().with_connected("HDMI-1", false);
        let result = apply_auto(&config(vec![enabled_output("HDMI-1")]), &backend).unwrap();

        assert_eq!(result.status(), ExitStatus::NoConfiguredConnectedOutput);
        assert_eq!(backend.calls(), vec!["connected:HDMI-1"]);
    }

    #[test]
    fn auto_reports_already_satisfied_for_connected_noop_output() {
        let backend = FakeBackend::default().with_connected("HDMI-1", true);
        let result = apply_auto(&config(vec![enabled_output("HDMI-1")]), &backend).unwrap();

        assert_eq!(result.status(), ExitStatus::AlreadySatisfied);
        assert_eq!(
            backend.calls(),
            vec![
                "connected:HDMI-1".to_string(),
                "on:HDMI-1:Preferred".to_string()
            ]
        );
    }

    #[test]
    fn auto_reports_changed_when_any_enabled_output_changes() {
        let backend = FakeBackend::default()
            .with_connected("HDMI-1", true)
            .with_on_result("HDMI-1", ExitStatus::Changed);
        let result = apply_auto(&config(vec![enabled_output("HDMI-1")]), &backend).unwrap();

        assert_eq!(result.status(), ExitStatus::Changed);
    }

    #[test]
    fn auto_applies_disabled_outputs_without_connectivity_check() {
        let backend = FakeBackend::default().with_off_result("DP-1", ExitStatus::Changed);
        let result = apply_auto(&config(vec![disabled_output("DP-1")]), &backend).unwrap();

        assert_eq!(result.status(), ExitStatus::Changed);
        assert_eq!(backend.calls(), vec!["off:DP-1"]);
    }

    #[test]
    fn auto_converts_explicit_enabled_config_before_applying() {
        let backend = FakeBackend::default().with_connected("HDMI-1", true);
        let result = apply_auto(
            &config(vec![ConfiguredOutput {
                name: "HDMI-1".to_string(),
                enabled: true,
                width: Some(1280),
                height: Some(720),
                rate: Some(60.0),
                x: 0,
                y: 0,
                rotation: RotationRequest::Normal,
            }]),
            &backend,
        )
        .unwrap();

        assert_eq!(result.status(), ExitStatus::AlreadySatisfied);
        assert_eq!(
            backend.calls(),
            vec![
                "connected:HDMI-1".to_string(),
                format!(
                    "on:HDMI-1:{:?}",
                    ModeRequest::Explicit {
                        width: 1280,
                        height: 720,
                        rate: Some(60.0)
                    }
                )
            ]
        );
    }

    #[test]
    fn auto_propagates_unavailable_output_errors() {
        let backend = FakeBackend::default();
        let error = apply_auto(&config(vec![enabled_output("HDMI-1")]), &backend).unwrap_err();

        assert_eq!(error.kind(), crate::ErrorKind::Unavailable);
    }
}
