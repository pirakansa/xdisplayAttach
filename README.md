# xdisplay-attach

`xdisplay-attach` manages Xorg/RandR display pipeline state. It detects outputs,
enables connected outputs, disables active outputs, selects modes, assigns
CRTCs, sets output geometry, and resizes the RandR root screen when needed.

Window placement and kiosk layout enforcement are intentionally out of scope.
Run a separate layout tool after `xdisplay-attach` has made the desired outputs
active.

## Quick Start

```sh
cargo build
xdisplay-attach status
xdisplay-attach on --output HDMI-1 --preferred
xdisplay-attach auto --config displays.json
```

See [CLI specification](docs/specifications/cli.md) for commands, JSON schema,
and exit statuses. See [hotplug usage](docs/user-guides/hotplug.md) for the
recommended DRM/udev-triggered service flow.
