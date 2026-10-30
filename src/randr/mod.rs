mod apply;
mod auto;
mod mode;
mod plan;
mod root;
mod snapshot;
mod state;
mod status;
#[cfg(test)]
mod test_support;
mod touch;
mod watch;

use self::auto::apply_auto;
use self::plan::plan_enforce;
use self::status::output_statuses;
use crate::{AttachError, CommandResult, DisplayConfig, OnRequest, OutputStatus, Result};
use x11rb::protocol::randr::ConnectionExt as RandrConnectionExt;
use x11rb::rust_connection::RustConnection;

pub(crate) struct X11Randr {
    conn: RustConnection,
    screen_num: usize,
}

impl X11Randr {
    pub(crate) fn connect() -> Result<Self> {
        let (conn, screen_num) =
            x11rb::connect(None).map_err(|error| AttachError::xorg(error.to_string()))?;
        conn.randr_query_version(1, 5)
            .map_err(|error| AttachError::xorg(format!("RandR is unavailable: {error}")))?
            .reply()
            .map_err(|error| AttachError::xorg(format!("RandR is unavailable: {error}")))?;
        Ok(Self { conn, screen_num })
    }

    pub(crate) fn status(&self) -> Result<Vec<OutputStatus>> {
        let state = self.load_state()?;
        Ok(output_statuses(&state))
    }

    pub(crate) fn turn_on(&self, request: &OnRequest) -> Result<CommandResult> {
        apply::apply_on(self, request)
    }

    pub(crate) fn turn_off(&self, output_name: &str) -> Result<CommandResult> {
        let state = self.load_state()?;
        self.apply_off(&state, output_name)
    }

    pub(crate) fn auto(&self, config: &DisplayConfig) -> Result<CommandResult> {
        apply_auto(config, self)
    }

    pub(crate) fn enforce(&self, config: &DisplayConfig, dry_run: bool) -> Result<CommandResult> {
        if dry_run {
            let state = self.load_state()?;
            plan_enforce(&state, config)
        } else {
            self.auto(config)
        }
    }
}
