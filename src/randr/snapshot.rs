use super::root;
use super::state::{CrtcState, OutputState, RandrState};
use super::X11Randr;
use crate::{AttachError, Result};
use x11rb::connection::Connection;
use x11rb::protocol::randr::{Connection as OutputConnection, ConnectionExt as RandrConnectionExt};
use x11rb::protocol::xproto::ConnectionExt as XprotoConnectionExt;

impl X11Randr {
    pub(super) fn load_state(&self) -> Result<RandrState> {
        let setup = self.conn.setup();
        let screen = setup
            .roots
            .get(self.screen_num)
            .ok_or_else(|| AttachError::xorg("default X screen is unavailable"))?;
        let root = screen.root;
        let dimensions = root::load_dimensions(screen, || {
            let geometry = self
                .conn
                .get_geometry(root)
                .map_err(|error| AttachError::randr(error.to_string()))?
                .reply()
                .map_err(|error| AttachError::randr(error.to_string()))?;
            Ok((geometry.width, geometry.height))
        })?;
        let resources = self
            .conn
            .randr_get_screen_resources(root)
            .map_err(|error| AttachError::randr(error.to_string()))?
            .reply()
            .map_err(|error| AttachError::randr(error.to_string()))?;

        let outputs = resources
            .outputs
            .iter()
            .map(|output_id| {
                let info = self
                    .conn
                    .randr_get_output_info(*output_id, resources.config_timestamp)
                    .map_err(|error| AttachError::randr(error.to_string()))?
                    .reply()
                    .map_err(|error| AttachError::randr(error.to_string()))?;
                Ok(OutputState {
                    id: *output_id,
                    name: String::from_utf8_lossy(&info.name).into_owned(),
                    connected: info.connection == OutputConnection::CONNECTED,
                    crtc: info.crtc,
                    possible_crtcs: info.crtcs,
                    modes: info.modes,
                    preferred_count: usize::from(info.num_preferred),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let crtcs = resources
            .crtcs
            .iter()
            .map(|crtc_id| {
                let info = self
                    .conn
                    .randr_get_crtc_info(*crtc_id, resources.config_timestamp)
                    .map_err(|error| AttachError::randr(error.to_string()))?
                    .reply()
                    .map_err(|error| AttachError::randr(error.to_string()))?;
                Ok(CrtcState {
                    id: *crtc_id,
                    x: info.x,
                    y: info.y,
                    width: info.width,
                    height: info.height,
                    mode: info.mode,
                    rotation: info.rotation,
                    outputs: info.outputs,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(RandrState {
            root,
            config_timestamp: resources.config_timestamp,
            root_width: dimensions.width,
            root_height: dimensions.height,
            root_mm_width: dimensions.mm_width,
            root_mm_height: dimensions.mm_height,
            outputs,
            crtcs,
            modes: resources.modes,
        })
    }
}
