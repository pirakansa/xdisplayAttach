# xdisplay-attach Implementation Handoff

Status: Draft

Audience: worker responsible for the future `xdisplay-attach` repository.

This document is self-contained. It describes the work for `xdisplay-attach`
without requiring access to the `xdisplay-ruler` repository.

## Objective

Create `xdisplay-attach`, a command-line tool that manages Xorg/RandR display
pipeline state. It should replace the display pipeline responsibilities that are
currently mixed into a separate kiosk window layout tool.

The tool should be usable for Raspberry Pi HDMI hotplug recovery and for
non-Raspberry Pi Linux/Xorg systems where DRM/udev hotplug events are visible.

## Responsibility

`xdisplay-attach` owns these concerns:

- Detect Xorg/RandR outputs.
- Enable a connected output.
- Disable an active output.
- Assign a CRTC to a connected output.
- Select the final desired output mode.
- Set output position and rotation.
- Resize the RandR root screen when required.
- Recover display pipeline state after DRM/udev hotplug events.
- Optionally remap touch input when output geometry or rotation changes.

`xdisplay-attach` must not manage application windows. Window placement and
kiosk layout enforcement belong to a separate tool.

## Non-Goals

- Do not manage, raise, lower, move, or resize application windows.
- Do not implement kiosk layout rules.
- Do not require the external `xrandr` command for the main workflow.
- Do not run long-lived X11 work directly from udev rules.
- Do not add Wayland support as part of the first implementation.
- Do not use direct DRM/KMS control while Xorg owns the DRM master.

## Required CLI

Implement these initial commands:

```text
xdisplay-attach status
xdisplay-attach on --output NAME --preferred
xdisplay-attach on --output NAME --width N --height N [--rate HZ]
xdisplay-attach off --output NAME
xdisplay-attach auto --config FILE
```

The tool may add more options later, but these commands define the first
handoff scope.

## Activation Behavior

`on` and `auto` must activate outputs by setting the final desired mode in the
first successful mode set. Avoid a temporary preferred-mode activation followed
by a second resize, because that can cause extra display flicker.

Activation flow:

1. Connect to the Xorg server through the normal `DISPLAY` environment.
2. Verify that the Xorg server exposes the RandR extension.
3. Read RandR screen resources.
4. Find the selected connected output.
5. Select the final mode:
   - use the explicit `--width` / `--height` / optional `--rate` request when
     provided;
   - otherwise use the output's preferred mode;
   - otherwise use a deterministic fallback mode, such as the first reported
     mode.
6. Choose a CRTC:
   - reuse the output's current CRTC when it already has one;
   - otherwise choose an unused CRTC allowed by the output's possible CRTC mask.
7. Compute the output position and rotation.
   - The first implementation may use `0,0` and normal rotation unless the
     config file specifies otherwise.
8. Expand the RandR root screen before activation if the chosen output bounds
   would exceed the current root screen.
9. Call RandR `SetCrtcConfig` with the chosen CRTC, mode, output list, position,
   and rotation.
10. Flush the X11 connection.
11. Return a clear status indicating whether the display pipeline changed.

## Deactivation Behavior

`off --output NAME` must disable the selected active output.

Deactivation flow:

1. Find the selected output.
2. If the output has no active CRTC, report "already off" and exit
   successfully.
3. Call RandR `SetCrtcConfig` for the output's CRTC with mode `0` and no
   outputs.
4. Recompute the root screen size from the remaining active outputs when this
   can be done safely.
5. Flush the X11 connection.

## Hotplug Automation

`auto --config FILE` is the command intended for systemd services triggered by
DRM/udev events.

The command must:

- read a persistent display configuration;
- inspect the current RandR state;
- activate connected configured outputs using their final configured modes;
- optionally disable outputs configured as off;
- avoid changing already-correct outputs;
- exit quickly and deterministically.

udev rules should start a systemd service. They should not directly run
long-lived X11 or RandR commands. The service should provide the user session
environment required to reach Xorg, such as `DISPLAY` and `XAUTHORITY` when
needed.

## Exit Status Contract

Define and document exit statuses for at least these outcomes:

- success, changed;
- success, already satisfied;
- success, no configured connected output was available;
- usage error;
- Xorg/RandR unavailable;
- requested output or mode unavailable;
- RandR operation failed.

The exact numeric values may be chosen by the implementation, but they must be
stable once documented.

## Operational Contract with xdisplay-ruler

After a successful activation, `xdisplay-attach` should leave Xorg/RandR in a
state where another tool can observe active outputs and place windows on them.

Expected downstream sequence:

```text
xdisplay-attach auto --config displays.json
xdisplay-ruler enforce --layout layout.json --once
```

`xdisplay-attach` must not place windows. The downstream window layout tool
must not need to change output modes to avoid flicker.

## Acceptance Criteria

- Given a connected but inactive output, `xdisplay-attach on --output NAME
  --preferred` activates it using one RandR mode set.
- Given a connected but inactive output and an explicit available mode,
  `xdisplay-attach on --output NAME --width N --height N --rate HZ` activates
  it directly in that mode.
- Given an active output, `xdisplay-attach off --output NAME` disables it.
- Given an already-correct configured display state, `xdisplay-attach auto`
  exits successfully without changing RandR state.
- Given a DRM/udev-triggered service environment with access to Xorg,
  `xdisplay-attach auto` can recover the display pipeline before window layout
  enforcement runs.
- The implementation does not shell out to `xrandr` for normal operation.

## Risks and Constraints

- If Xorg/RandR reports zero outputs, an Xorg/RandR-based attach tool cannot
  recover the display pipeline by itself. The system may need boot firmware,
  kernel, Xorg, EDID, or DRM/KMS configuration changes first.
- Direct DRM/KMS control can conflict with Xorg when Xorg owns the DRM master.
  Prefer X11/RandR for Xorg sessions.
- CRTC assignment and mode selection must be implemented together. RandR does
  not provide a useful normal activation state where an output is attached to a
  CRTC without a mode.
- Extra mode sets can produce visible flicker. The final desired mode should be
  selected before activation.

## Open Questions

- What exact configuration schema should `auto --config FILE` use?
- Should touch input remapping be included in the first release or added after
  output activation is stable?
- Which Linux/Xorg environments are required for initial validation besides
  Raspberry Pi?
