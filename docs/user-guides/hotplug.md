# Hotplug Usage

`xdisplay-attach enforce --config FILE` is intended to run as a short-lived
systemd service triggered by startup or DRM/udev events.

Do not run X11/RandR work directly from a udev rule. The service must provide
the user-session environment needed to reach Xorg, such as `DISPLAY` and
`XAUTHORITY`.

## Display Configuration

Use the same configuration file that you would pass to `enforce` manually:

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
exits with status `11`.

For a portrait startup policy, keep `width` and `height` set to the unrotated
RandR mode dimensions and set `rotation` separately:

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
      "rotation": "left"
    }
  ]
}
```

Before enabling a service, preview the actions:

```bash
xdisplay-attach enforce --config displays.json --dry-run
```

## Service

Create a user or system service that runs the command once and exits. The exact
environment values depend on how the kiosk session starts Xorg.

```ini
[Unit]
Description=Apply X display attachment policy
After=graphical-session.target

[Service]
Type=oneshot
Environment=DISPLAY=:0
Environment=XAUTHORITY=/home/kiosk/.Xauthority
ExecStart=/usr/local/bin/xdisplay-attach enforce --config /etc/xdisplay-attach/displays.json
SuccessExitStatus=10 11
```

Status `10` means the requested state was already satisfied. Status `11` means
no configured connected output was available.

## Udev Trigger

Use a udev rule only to ask systemd to start the service. Do not put
`xdisplay-attach` directly in `RUN+=`.

```udev
SUBSYSTEM=="drm", ACTION=="change", TAG+="systemd", ENV{SYSTEMD_WANTS}+="xdisplay-attach.service"
```

If you run the display command as a user service, have the system service or
session manager bridge the DRM event to that user service while preserving the
user-session X11 environment.

## Downstream Layout

Expected downstream order:

```text
xdisplay-attach enforce --config displays.json
xdisplay-ruler enforce --layout layout.json --once
```

`xdisplay-attach` leaves active outputs in their final requested modes before
the downstream layout tool observes outputs and places windows.
