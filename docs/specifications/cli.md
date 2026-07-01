# CLI Specification

`xdisplay-attach` controls Xorg/RandR display pipeline state only. It does not
manage, move, resize, raise, lower, or place application windows.

## Commands

```text
xdisplay-attach status
xdisplay-attach on --output NAME --preferred
xdisplay-attach on --output NAME --width N --height N [--rate HZ]
xdisplay-attach off --output NAME
xdisplay-attach auto --config FILE
```

`status` observes RandR outputs and prints each output name, connection state,
activity state, and current geometry when active.

`on` activates a connected output with a single `SetCrtcConfig` call for the
final selected mode. With `--preferred`, it uses the first preferred mode
reported by RandR, then falls back to the first reported mode if no preferred
mode is marked. With `--width` and `--height`, it selects an available mode with
matching dimensions and, when supplied, matching refresh rate.

`off` disables the selected output's active CRTC with `SetCrtcConfig` mode `0`
and no outputs. It then shrinks the root screen to the remaining active output
bounds when those bounds can be represented safely.

`auto` reads a JSON configuration, activates connected enabled outputs, disables
outputs configured as off, and avoids changing outputs that already match the
requested mode, position, rotation, and CRTC output list.

## Auto Configuration

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

Fields:

- `name`: RandR output name.
- `enabled`: whether the output should be active. Defaults to `true`.
- `width` and `height`: explicit mode dimensions. Both must be present or both
  must be omitted.
- `rate`: optional refresh rate in Hz for explicit mode selection.
- `x` and `y`: output position. Defaults to `0`.
- `rotation`: one of `normal`, `left`, `inverted`, or `right`. Defaults to
  `normal`.

If `width` and `height` are omitted for an enabled output, the preferred mode is
used.

## Exit Status

The status values are stable API:

| Code | Meaning |
| ---: | --- |
| 0 | success, RandR state changed |
| 10 | success, requested state was already satisfied |
| 11 | success, no configured connected output was available |
| 64 | usage or configuration error |
| 69 | Xorg/RandR unavailable |
| 70 | requested output, mode, or CRTC unavailable |
| 71 | RandR operation failed |
