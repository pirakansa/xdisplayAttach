# xdisplay-ruler Migration Handoff

Status: Draft

Audience: worker responsible for this `xdisplay-ruler` repository.

This document is self-contained. It describes the work for `xdisplay-ruler`
without requiring access to the future `xdisplay-attach` repository.

## Objective

Keep `xdisplay-ruler` focused on observing X11 output/window state and enforcing
kiosk window layouts on already active outputs.

Display pipeline control should be removed from `xdisplay-ruler` after a
separate tool provides replacement coverage. In this context, display pipeline
control means output activation, output deactivation, CRTC assignment, output
mode selection, output position, output rotation, and RandR root screen sizing.

## Current Repository Context

`xdisplay-ruler` currently includes both:

- window and layout behavior: snapshots, watch, window raise/lower,
  move/resize, fullscreen placement, and layout enforcement;
- display pipeline behavior: output mode listing and output mode switching.

The migration goal is to keep the first group and remove or deprecate the
second group once replacement behavior exists in another tool.

## Responsibility to Preserve

Preserve these `xdisplay-ruler` responsibilities:

- Observe current X11/RandR output state.
- Observe root-level X11 window state.
- Print snapshots.
- Watch display and window changes.
- Raise and lower selected windows.
- Move and resize selected windows.
- Place selected windows on already active output geometry.
- Enforce kiosk layouts against already active outputs.
- Report missing or disconnected outputs clearly.

## Responsibility to Remove or Avoid

Do not add new display pipeline responsibilities to `xdisplay-ruler`.

After replacement coverage exists in a separate display pipeline tool, remove or
deprecate these responsibilities from `xdisplay-ruler`:

- selecting output modes;
- switching output modes;
- activating inactive outputs;
- disabling outputs;
- assigning CRTCs;
- changing output rotation;
- resizing the RandR root screen as a display pipeline operation;
- recovering HDMI hotplug by creating the output pipeline.

`enforce` must not implicitly enable an output or change an output mode.

## Required Migration Stages

### Stage 1: Preserve Behavior While Boundary Is Draft

Do not break existing users during planning.

- Keep existing commands working.
- Keep existing tests passing.
- Do not remove `mode` or `modes` until replacement coverage exists or the user
  explicitly approves a breaking change.
- Do not update user-facing docs to claim behavior has changed before code
  changes land.

### Stage 2: Mark Pipeline Commands as Transitional

When replacement coverage exists elsewhere, add deprecation messaging for
pipeline commands.

Expected deprecated commands:

```text
xdisplay-ruler modes
xdisplay-ruler mode
```

Deprecation behavior should:

- print a clear warning to standard error;
- keep the command behavior working during the deprecation window;
- name the replacement tool and command shape when known;
- include tests for warning behavior and exit status.

### Stage 3: Remove Pipeline Commands

After the deprecation window or explicit approval:

- remove `mode` command parsing and execution;
- remove `modes` command parsing and execution unless retained as a read-only
  observation command by explicit decision;
- remove output mode switching implementation from this repository;
- remove docs that describe `xdisplay-ruler` as a mode control tool;
- keep observation models only where they support snapshots, watch, placement,
  and layout enforcement.

## Layout Enforcement Contract

`enforce` should operate only on outputs already visible to Xorg/RandR.

Required behavior:

- If a layout references a missing output, report the missing output clearly.
- If a layout references a disconnected or inactive output, report the problem
  clearly.
- Do not try to create, attach, or enable the output.
- Do not change output mode, rotation, or root screen size.
- Place windows using the output geometry that is already active.

This keeps hotplug recovery ordered as:

```text
display pipeline tool sets active outputs
  -> xdisplay-ruler enforces window layout
```

## Documentation Work

When behavior changes are implemented:

- Update user guides to describe `xdisplay-ruler` as operating on already active
  outputs.
- Update specifications so they list only implemented `xdisplay-ruler`
  behavior.
- Remove or rewrite output mode user guidance if `mode` and `modes` are removed.
- In PR descriptions, state whether there are "No documentation changes" or
  list the updated docs.

Do not update the top-level README as if planned behavior is already
implemented.

## Acceptance Criteria

- Existing non-pipeline behavior remains covered by tests.
- `enforce` does not enable outputs or change output modes.
- Missing outputs remain clear user-facing errors or warnings according to the
  existing enforce mode.
- Deprecation warnings for `mode` and `modes`, if added, are tested.
- After final removal, CLI help, specifications, user guides, and tests no
  longer advertise removed pipeline commands.
- `vorbere run check`, `vorbere run test`, and `vorbere run build` pass before
  handoff completion.

## Non-Goals

- Do not implement the new display pipeline tool in this repository as part of
  this migration handoff.
- Do not add udev/systemd hotplug recovery implementation to `xdisplay-ruler`.
- Do not add direct DRM/KMS control to `xdisplay-ruler`.
- Do not make `enforce` select output modes.
- Do not remove existing commands before replacement coverage or explicit
  approval.

## Risks and Constraints

- Removing `mode` and `modes` is a breaking CLI change unless a deprecation
  window or compatibility shim is provided.
- Some output mode listing code may look read-only, but it still belongs to the
  display pipeline area if it is documented as mode-management workflow.
- Touch remapping currently tied to mode changes may need a migration decision
  if mode changes leave this repository.
- This repository's documentation policy requires all documentation to be in
  English.

## Open Questions

- Should `xdisplay-ruler modes` remain as a read-only diagnostic command, or
  should all mode-oriented output move out of this repository?
- What deprecation window is acceptable for existing `xdisplay-ruler mode`
  users?
- Should touch input remapping move out with display pipeline mode changes?
