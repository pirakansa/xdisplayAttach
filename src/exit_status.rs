use std::fmt::{self, Display};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    Changed,
    AlreadySatisfied,
    NoConfiguredConnectedOutput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    status: ExitStatus,
    warnings: Vec<String>,
    print_status: bool,
}

impl CommandResult {
    pub fn new(status: ExitStatus) -> Self {
        Self {
            status,
            warnings: Vec::new(),
            print_status: true,
        }
    }

    pub fn with_warning(status: ExitStatus, warning: impl Into<String>) -> Self {
        Self {
            status,
            warnings: vec![warning.into()],
            print_status: true,
        }
    }

    pub fn status(&self) -> ExitStatus {
        self.status
    }

    pub fn should_print_status(&self) -> bool {
        self.print_status
    }

    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    pub(crate) fn without_status_line(status: ExitStatus) -> Self {
        Self {
            status,
            warnings: Vec::new(),
            print_status: false,
        }
    }

    pub(crate) fn extend_warnings(&mut self, warnings: Vec<String>) {
        self.warnings.extend(warnings);
    }
}

impl ExitStatus {
    pub const fn code(self) -> i32 {
        match self {
            Self::Changed => 0,
            Self::AlreadySatisfied => 10,
            Self::NoConfiguredConnectedOutput => 11,
        }
    }
}

impl Display for ExitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Changed => f.write_str("changed"),
            Self::AlreadySatisfied => f.write_str("already satisfied"),
            Self::NoConfiguredConnectedOutput => f.write_str("no configured connected output"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_stable_success_exit_codes() {
        assert_eq!(ExitStatus::Changed.code(), 0);
        assert_eq!(ExitStatus::AlreadySatisfied.code(), 10);
        assert_eq!(ExitStatus::NoConfiguredConnectedOutput.code(), 11);
    }

    #[test]
    fn command_result_carries_warnings_without_changing_status() {
        let result = CommandResult::with_warning(ExitStatus::Changed, "touch remapping failed");

        assert_eq!(result.status(), ExitStatus::Changed);
        assert_eq!(result.warnings(), &["touch remapping failed"]);
        assert!(result.should_print_status());
    }

    #[test]
    fn command_result_can_suppress_status_line() {
        let result = CommandResult::without_status_line(ExitStatus::AlreadySatisfied);

        assert_eq!(result.status(), ExitStatus::AlreadySatisfied);
        assert!(!result.should_print_status());
    }
}
