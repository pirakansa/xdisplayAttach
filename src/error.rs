use std::fmt::{self, Display};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Usage,
    XorgUnavailable,
    Unavailable,
    RandrFailed,
}

#[derive(Debug)]
pub struct AttachError {
    kind: ErrorKind,
    message: String,
}

impl AttachError {
    pub fn usage(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Usage, message)
    }

    pub(crate) fn xorg(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::XorgUnavailable, message)
    }

    pub(crate) fn unavailable(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unavailable, message)
    }

    pub(crate) fn randr(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::RandrFailed, message)
    }

    fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub const fn exit_code(&self) -> i32 {
        match self.kind {
            ErrorKind::Usage => 64,
            ErrorKind::XorgUnavailable => 69,
            ErrorKind::Unavailable => 70,
            ErrorKind::RandrFailed => 71,
        }
    }
}

impl Display for AttachError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl std::error::Error for AttachError {}

pub type Result<T> = std::result::Result<T, AttachError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_stable_error_exit_codes() {
        assert_eq!(AttachError::usage("bad input").exit_code(), 64);
        assert_eq!(AttachError::xorg("missing display").exit_code(), 69);
        assert_eq!(AttachError::unavailable("missing mode").exit_code(), 70);
        assert_eq!(AttachError::randr("failed").exit_code(), 71);
    }
}
