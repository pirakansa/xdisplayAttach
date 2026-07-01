use std::fmt::{self, Display};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    Changed,
    AlreadySatisfied,
    NoConfiguredConnectedOutput,
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
