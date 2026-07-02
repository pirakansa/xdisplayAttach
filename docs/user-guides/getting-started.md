# Getting Started

`xdisplay-attach` controls Xorg/RandR display pipeline state. Use it to inspect
outputs, enable connected outputs, disable active outputs, and apply a small
JSON display configuration.

## Install

Install the binary from the repository checkout:

```bash
cargo install --path .
```

For development without installing, run commands through Cargo:

```bash
cargo run -- status
```

The X11 backend requires a reachable Xorg server through the user-session
environment, including `DISPLAY` and, when needed, `XAUTHORITY`.

## Inspect Outputs

Print the current RandR output state:

```bash
xdisplay-attach status
```

Output rows include the RandR output name accepted by `--output`, connection
state, activity state, and current geometry when the output is active:

```text
HDMI-1 connected active 1920x1080+0+0
DP-1 connected inactive
```

## Activate an Output

Enable a connected output with its preferred mode:

```bash
xdisplay-attach on --output HDMI-1 --preferred
```

Select an explicit existing RandR mode when you need a specific size or refresh
rate:

```bash
xdisplay-attach on --output HDMI-1 --width 1920 --height 1080 --rate 60
```

Rotate an already active output while keeping its current mode:

```bash
xdisplay-attach on --output HDMI-1 --rotate left
```

For left or right rotation, `--width` and `--height` are still the unrotated
RandR mode dimensions.

## Disable an Output

Disable an active output:

```bash
xdisplay-attach off --output DP-1
```

After disabling an output, `xdisplay-attach` shrinks the RandR root screen to
the remaining active output bounds when those bounds can be represented safely.

## Apply a Configuration

Create a display configuration:

```json
{
  "outputs": [
    {
      "name": "HDMI-1",
      "enabled": true,
      "width": 1920,
      "height": 1080,
      "rate": 60.0,
      "x": 0,
      "y": 0,
      "rotation": "normal"
    },
    {
      "name": "DP-1",
      "enabled": false
    }
  ]
}
```

Apply it:

```bash
xdisplay-attach auto --config displays.json
```

`auto` skips enabled outputs that are disconnected, activates connected enabled
outputs, disables outputs configured with `"enabled": false`, and leaves outputs
unchanged when the requested state is already satisfied.

Successful commands print one of these status lines unless the command is
`status` or help:

```text
changed
already satisfied
no configured connected output
```

Use exit status `10` for an already satisfied request and `11` when no
configured enabled output was connected. For the exact command contract, see the
[CLI specification](../specifications/cli.md).
