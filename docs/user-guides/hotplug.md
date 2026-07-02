# Hotplug Usage

`xdisplay-attach enforce --config FILE --watch` keeps an Xorg/RandR display
policy applied over time. It runs as a foreground process, applies the policy
once at startup, subscribes to RandR change notifications, and reapplies the
policy after relevant output or screen changes.

Do not run X11/RandR work directly from a udev rule. The watcher is an X client
and must run with the user-session environment needed to reach Xorg, such as
`DISPLAY` and `XAUTHORITY`.

## Display Configuration

Use the same configuration file that you would pass to one-shot `enforce`:

```json
{
  "schema_version": 1,
  "outputs": [
    {
      "name": "HDMI-1",
      "enabled": true,
      "width": 1920,
      "height": 1080,
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

`enforce` skips enabled outputs that are disconnected. If every configured
enabled output is disconnected and no disabled output changes state, the command
reports `no configured connected output` and watch mode keeps running.

Before starting the watcher, preview the actions:

```bash
xdisplay-attach enforce --config displays.json --dry-run
```

## Watch Command

Run the watcher in the Xorg session environment:

```bash
xdisplay-attach enforce --config displays.json --watch
```

Optional timing controls:

```bash
xdisplay-attach enforce --config displays.json --watch --debounce-ms 500 --retry 3 --retry-delay-ms 1000
```

- `--debounce-ms`: wait after the last relevant RandR event before enforcing.
- `--retry`: additional attempts after an event-time operational failure.
- `--retry-delay-ms`: wait between retry attempts.

`--dry-run` and `--watch` cannot be combined.

## Service

Use systemd to keep the watcher process running. The service should not be a
udev-triggered oneshot unit.

System service example:

```ini
[Unit]
Description=Watch and enforce X display attachment policy
After=graphical-session.target

[Service]
Type=simple
User=kiosk
Environment=DISPLAY=:0
Environment=XAUTHORITY=/home/kiosk/.Xauthority
ExecStart=/usr/local/bin/xdisplay-attach enforce --config /etc/xdisplay-attach/displays.json --watch
Restart=on-failure
RestartSec=2
```

User service example:

```ini
[Unit]
Description=Watch and enforce X display attachment policy
After=graphical-session.target

[Service]
Type=simple
Environment=DISPLAY=:0
Environment=XAUTHORITY=/home/kiosk/.Xauthority
ExecStart=/usr/local/bin/xdisplay-attach enforce --config /home/kiosk/.config/xdisplay-attach/displays.json --watch
Restart=on-failure
RestartSec=2
```

Choose the system or user unit form based on how the kiosk Xorg session is
launched. In both cases, `DISPLAY` and `XAUTHORITY` must match the target
session.

## Hotplug Boundary

When a monitor is connected, the kernel DRM/KMS driver and Xorg are responsible
for detecting the connector, EDID, and modes. `xdisplay-attach` does not open
DRM devices or initialize KMS connectors directly. It observes the RandR state
that Xorg exposes and converges that state to the configured policy.

If a monitor is connected but the watcher does not react, compare:

```bash
udevadm monitor --kernel --udev --subsystem-match=drm --property
xdisplay-attach status
```

Also check Xorg logs for connector, EDID, or modesetting updates.

## Verification

Manual verification:

```bash
xdisplay-attach enforce --config displays.json --watch
```

Connect or disconnect a monitor and confirm that the watcher logs a RandR event
followed by an enforcement result.

Service verification:

```bash
systemctl status xdisplay-attach-watch.service
journalctl -u xdisplay-attach-watch.service
```

For user units:

```bash
systemctl --user status xdisplay-attach-watch.service
journalctl --user -u xdisplay-attach-watch.service
```

## Downstream Layout

`xdisplay-attach` only manages display pipeline state. If another tool places
windows after display changes, run that tool separately after the display policy
has converged.
