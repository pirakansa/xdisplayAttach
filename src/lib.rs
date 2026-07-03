mod cli;
mod config;
mod error;
mod exit_status;
mod model;
mod randr;
mod signal;

pub use cli::{parse_args, run_cli, usage};
pub use config::{read_config, ConfiguredOutput, DisplayConfig};
pub use error::{AttachError, ErrorKind, Result};
pub use exit_status::{CommandResult, ExitStatus};
pub use model::{
    Command, ModeRequest, ModeSummary, OnRequest, OutputStatus, RotationRequest, WatchOptions,
};
