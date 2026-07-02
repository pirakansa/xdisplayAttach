# xdisplay-attach

`xdisplay-attach` manages Xorg/RandR display pipeline state. It detects outputs
and their available modes, enables connected outputs, disables active outputs,
assigns CRTCs, sets output geometry, and resizes the RandR root screen when
needed.

Window placement and kiosk layout enforcement are intentionally out of scope.
Run a separate layout tool after `xdisplay-attach` has made the desired outputs
active.

## Quick Start

```sh
cargo install --path .
xdisplay-attach status
xdisplay-attach on --output HDMI-1 --preferred
xdisplay-attach enforce --config displays.json
```

See [user guides](docs/user-guides/README.md) for practical workflows. See the
[CLI specification](docs/specifications/cli.md) for commands, JSON schema, and
exit statuses.
