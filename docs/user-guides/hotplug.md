# Hotplug Usage

`xdisplay-attach auto --config FILE` is intended to run as a short-lived systemd
service triggered by DRM/udev events.

Do not run X11/RandR work directly from a udev rule. The service must provide
the user-session environment needed to reach Xorg, such as `DISPLAY` and
`XAUTHORITY`.

For services that run `auto`, include successful no-op statuses:

```ini
SuccessExitStatus=10 11
```

Expected downstream order:

```text
xdisplay-attach auto --config displays.json
xdisplay-ruler enforce --layout layout.json --once
```

`xdisplay-attach` leaves active outputs in their final requested modes before
the downstream layout tool observes outputs and places windows.
