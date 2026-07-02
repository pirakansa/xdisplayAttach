# CLI Specification

`xdisplay-attach` controls Xorg/RandR display pipeline state only. It does not
manage, move, resize, raise, lower, or place application windows.

## Commands

```text
xdisplay-attach status
xdisplay-attach on --output NAME --preferred [--rotate DIR]
xdisplay-attach on --output NAME --width N --height N [--rate HZ] [--rotate DIR]
xdisplay-attach on --output NAME --rotate DIR
xdisplay-attach off --output NAME
xdisplay-attach auto --config FILE
xdisplay-attach enforce --config FILE [--dry-run]
xdisplay-attach enforce --config FILE --watch [--debounce-ms N] [--retry COUNT] [--retry-delay-ms N]
```

`status` observes RandR outputs and prints each output name, connection state,
activity state, and current geometry when active. It then prints the output's
available RandR modes, in RandR-reported order, indented below the output row.
Mode rows include dimensions, refresh rate when RandR timing can represent it,
and `current` or `preferred` markers when applicable:

```text
HDMI-1 connected active 1920x1080+0+0
  1920x1080 60.000Hz current preferred
  1280x720 59.940Hz
DP-1 connected inactive
  1920x1080 60.000Hz preferred
```

`on` activates a connected output with a single `SetCrtcConfig` call for the
final selected mode. With `--preferred`, it uses the first preferred mode
reported by RandR, then falls back to the first reported mode if no preferred
mode is marked. With `--width` and `--height`, it selects an available mode with
matching dimensions and, when supplied, matching refresh rate. `width` and
`height` are the unrotated RandR mode dimensions; for a rotated portrait output,
configure the native mode dimensions and set `rotation` separately. Refresh
rate matching uses RandR mode timing and applies RandR `INTERLACE` and
`DOUBLE_SCAN` mode flag adjustments. With `--rotate` and no mode request, `on`
reuses the output's current active mode and changes only the requested rotation.

`off` disables the selected output's active CRTC with `SetCrtcConfig` mode `0`
and no outputs. It then shrinks the root screen to the remaining active output
bounds when those bounds can be represented safely.

`auto` reads a JSON configuration, activates connected enabled outputs, disables
outputs configured as off, and avoids changing outputs that already match the
requested mode, position, rotation, and CRTC output list.

`enforce` reads the same JSON configuration as `auto` and applies the same
state convergence behavior. It is intended for startup and hotplug workflows
where a display policy should be restored whenever the command runs. With
`--watch`, it applies the policy once, subscribes to RandR change notifications,
and keeps running. When a relevant RandR event is observed, the watcher waits
for the debounce interval and applies the policy again. The default debounce is
`500` milliseconds, the default retry count is `3`, and the default retry delay
is `1000` milliseconds. `--dry-run` and `--watch` cannot be combined.

With `--dry-run`, `enforce` loads current RandR state, validates configured
connected outputs, prints the planned per-output actions, and does not call
RandR configuration methods. Dry-run output lines use these forms:

```text
HDMI-1 set 1920x1080+0+0 rotate left
HDMI-1 already satisfied
HDMI-1 skipped disconnected
DP-1 disable
DP-1 already disabled
```

After the dry-run action lines, the command prints the same final status line
as a normal successful command.

After a successful output activation, `xdisplay-attach` remaps enabled XInput
touch devices by updating their `Coordinate Transformation Matrix` to the
selected output geometry and rotation. If RandR accepts the display change but
touch remapping fails, the command still succeeds and prints a warning to
standard error.

## Auto Configuration

```json
{
  "schema_version": 1,
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

- `schema_version`: optional configuration schema version. Omitted files are
  treated as version `1`. When present, the only supported value is `1`.
- `name`: RandR output name.
- `enabled`: whether the output should be active. Defaults to `true`.
- `width` and `height`: explicit unrotated RandR mode dimensions. Both must be
  present or both must be omitted.
- `rate`: optional refresh rate in Hz for explicit mode selection.
- `x` and `y`: output position. Defaults to `0`.
- `rotation`: one of `normal`, `left`, `inverted`, or `right`. Defaults to
  `normal`. For `left` and `right`, `width` and `height` still refer to the
  unrotated RandR mode dimensions.

If `width` and `height` are omitted for an enabled output, the preferred mode is
used.

If an enabled connected output cannot satisfy its requested mode, position,
rotation, or CRTC assignment, the command fails with exit status `70`. For
example, an explicit `width` and `height` pair that is not present in the
output's RandR modes is reported as an unavailable requested mode. Commands do
not roll back changes already applied to earlier outputs in the same
configuration.

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
