# Display Pipeline Separation Plan

Status: Draft

## Purpose

This directory contains the planning documents for separating display pipeline
control from kiosk window layout control.

The intended final state is two independent tools in two independent
repositories:

- `xdisplay-attach`: display pipeline control for Xorg/RandR outputs.
- `xdisplay-ruler`: output/window observation and kiosk window layout
  enforcement.

Each handoff document is written so it can be given to a worker who does not
have access to the other repository. Do not require an implementation worker to
read another repository's plan to understand their own scope.

## Handoff Documents

- [xdisplay-attach handoff](xdisplay-attach-handoff.md): give this to the
  worker responsible for creating or maintaining the `xdisplay-attach`
  repository.
- [xdisplay-ruler handoff](xdisplay-ruler-handoff.md): give this to the worker
  responsible for changing this `xdisplay-ruler` repository.

## Shared Boundary

`xdisplay-attach` owns display pipeline state:

- output activation and deactivation
- CRTC assignment
- mode selection
- output position and rotation
- RandR root screen sizing
- hotplug recovery before window layout

`xdisplay-ruler` owns window and layout state:

- X11/RandR output and root-level window observation
- snapshots and watch reports
- window raise, lower, move, resize, and fullscreen placement
- kiosk layout enforcement on already active outputs

Hotplug recovery should eventually run in this order:

```text
DRM/udev change event
  -> xdisplay-attach auto --config displays.json
  -> xdisplay-ruler enforce --layout layout.json --once
```

`xdisplay-attach` must set the final desired output mode before
`xdisplay-ruler` runs. `xdisplay-ruler` must not enable outputs or change modes
as part of layout enforcement.

## Key Design Decision

CRTC assignment and mode selection belong together. RandR display activation is
performed with `SetCrtcConfig`, which requires choosing the CRTC, output list,
mode, position, and rotation in one operation. A separate "attach CRTC but do
not choose a mode" phase is not a useful target state for normal Xorg/RandR
operation.

## Current Repository Note

This repository is currently the `xdisplay-ruler` repository. The
`xdisplay-attach` handoff is kept here only as planning material so it can be
copied to the future `xdisplay-attach` repository or assigned to a separate
worker.
