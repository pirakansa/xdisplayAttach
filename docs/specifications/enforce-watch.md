# Enforce Watch Plan

Status: Initial implementation available.

This plan defines a persistent enforcement mode for keeping an Xorg/RandR
display policy applied over time. It replaces the earlier service-triggered
oneshot concept: systemd may still manage process lifetime, but monitor state
changes are detected by `xdisplay-attach` itself.

## Objective

Add a long-running watch mode that observes RandR display changes and reruns
the existing `enforce` convergence behavior whenever the current display state
may have drifted from the configured policy.

The target workflow is:

1. A user installs the binary and a display policy JSON file.
2. A systemd service starts `xdisplay-attach enforce --config FILE --watch`
   inside the graphical Xorg session environment.
3. The process applies the configured policy once at startup.
4. The process subscribes to RandR change notifications and remains running.
5. When outputs, CRTCs, screen resources, or output properties change, the
   process waits for a short debounce interval and applies the policy again.

## Scope

This plan covers kiosk-style Xorg sessions first. The watcher is an Xorg/RandR
client, not a udev handler and not a Wayland integration.

In scope:

- A new persistent `enforce --watch` mode.
- Initial enforcement when the watcher starts.
- RandR event subscription for display topology and configuration changes.
- Debounced re-enforcement after relevant RandR events.
- Service documentation for running the watcher under systemd.
- Clear handling of transient and permanent enforcement failures.
- Tests for argument parsing, watch-loop decision logic, and retry/debounce
  behavior where those can be isolated from a live X server.

Out of scope:

- Running `xdisplay-attach` directly from udev.
- Using udev as the primary change-detection mechanism.
- Opening DRM devices directly or initializing KMS connectors without Xorg.
- Adding a separate daemon binary.
- Managing downstream window placement.
- Desktop-environment-specific autostart integrations as the primary path.
- Claiming Wayland support.

## Requirements

Functional requirements:

- `xdisplay-attach enforce --config FILE --watch` must run until interrupted or
  until it encounters an unrecoverable startup error.
- Watch mode must apply the configured policy immediately after startup before
  waiting for events.
- Watch mode must subscribe to RandR notifications on the root window for the
  selected screen.
- Watch mode must react to events that can affect output connection, CRTC
  assignment, screen resources, output properties, mode availability, or screen
  size.
- Multiple events from one hotplug burst must be coalesced before enforcement.
- If enforcement reports `0`, `10`, or `11`, watch mode must keep running.
- If enforcement reports an operational RandR failure during an event burst, the
  watcher should retry according to configured retry settings before logging a
  warning and continuing to wait for the next event.
- If the X connection is lost, watch mode must exit with the existing
  Xorg/RandR unavailable status.
- The process must handle `SIGINT` and `SIGTERM` as graceful shutdown requests.

Non-functional requirements:

- The watcher must be safe for repeated or noisy hotplug events.
- The watcher must not busy-loop when display state is unstable.
- The initial implementation should avoid new async runtime dependencies unless
  blocking X11 event handling proves insufficient.
- Service examples must provide `DISPLAY` and `XAUTHORITY` explicitly.
- The documented examples must use fictitious users and paths.

## Command-Line Interface

Implemented command:

```text
xdisplay-attach enforce --config FILE [--dry-run]
xdisplay-attach enforce --config FILE --watch [--debounce-ms N] [--retry COUNT] [--retry-delay-ms N]
```

Rules:

- `--watch` makes `enforce` persistent.
- `--dry-run` and `--watch` are mutually exclusive. Dry-run should remain a
  single planning operation.
- `--debounce-ms N` controls how long the watcher waits after the last relevant
  event before enforcing. Default: `500`.
- `--retry COUNT` controls how many additional enforcement attempts are made
  after a recoverable event-time failure. Default: `3`.
- `--retry-delay-ms N` controls the delay between event-time retries. Proposed
  default: `1000`.
- Timing values reject zero. `--retry 0` is allowed and means no additional
  attempts.

## Watch Behavior

Startup sequence:

1. Parse CLI options and load the config file.
2. Connect to Xorg and verify RandR support.
3. Select RandR input notifications on the root window.
4. Run one enforcement pass.
5. Enter the event loop.

Event-loop sequence:

1. Poll for available X events and sleep briefly when none are pending.
2. Ignore unrelated events.
3. When a relevant RandR event arrives, start or reset the debounce timer.
4. After the debounce interval expires with no newer relevant event, run
   enforcement.
5. Log the resulting status or warning.
6. Continue waiting for events.

The implementation should isolate pure decision logic from X11 I/O so tests can
exercise event filtering, debounce scheduling, retry decisions, and result
classification without a live display server.

## RandR Event Sources

The watcher should use RandR event selection on the root window. Candidate event
masks include:

- `SCREEN_CHANGE_NOTIFY`
- `CRTC_CHANGE_NOTIFY`
- `OUTPUT_CHANGE_NOTIFY`
- `OUTPUT_PROPERTY_NOTIFY`
- `RESOURCE_CHANGE_NOTIFY`

The implementation uses the corresponding `x11rb` RandR notify masks. The
intent is to observe changes that can invalidate the current policy, including
monitor connect/disconnect, CRTC reassignment, mode/resource changes, and output
property updates.

The watcher does not rely on udev for hotplug detection. udev may still be
useful for diagnostics, but it is not part of the primary enforcement loop.

## Hotplug Responsibility Boundary

When a second monitor is connected, `xdisplay-attach` does not initialize the
monitor's DRM/KMS connector directly. The expected responsibility chain is:

```text
physical monitor hotplug
kernel DRM/KMS driver updates connector and EDID state
Xorg modesetting or GPU driver observes the kernel DRM change
Xorg updates RandR outputs, modes, CRTCs, and screen resources
Xorg sends RandR events to subscribed clients
xdisplay-attach enforce --watch receives the RandR event
xdisplay-attach debounces and reapplies the configured policy
```

The watcher is therefore a policy convergence client for the RandR state exposed
by Xorg. It must not try to compensate for missing kernel or Xorg hotplug
support by opening DRM devices and driving KMS itself.

If a monitor is physically connected but no relevant RandR event is emitted,
that is treated as an environment or driver integration issue to diagnose
outside `xdisplay-attach`. Useful diagnostics include comparing:

- Whether kernel and udev observe a DRM `change` event.
- Whether Xorg logs report a connector, EDID, or modesetting update.
- Whether `xdisplay-attach status` shows the new output after the hotplug.

A future fallback periodic rescan can be considered if field deployments show
that Xorg occasionally updates RandR state without delivering an event. That
fallback should still read RandR state through Xorg; it should not become a
direct DRM/KMS initialization path.

## Enforcement Result Handling

Watch mode should classify enforcement outcomes as follows:

| Code | Watch behavior | Meaning |
| ---: | --- | --- |
| 0 | Continue | RandR state changed |
| 10 | Continue | Requested state was already satisfied |
| 11 | Continue | No configured connected output was available |
| 64 | Exit | Usage or configuration error |
| 69 | Exit if X connection is unavailable; retry only for event-time transient checks if the connection remains valid | Xorg/RandR unavailable |
| 70 | Retry during event burst, then warn and continue | Requested output, mode, or CRTC unavailable |
| 71 | Retry during event burst, then warn and continue | RandR operation failed |

Startup is stricter than event-time enforcement for configuration and X
connection setup. If the initial config cannot be loaded or the initial
X/RandR connection cannot be established, the command exits immediately with the
existing error status. If enforcement reports `70` or `71`, including during
the initial pass, the watcher uses the retry policy, logs retry exhaustion, and
continues waiting for future RandR events.

## Logging

The first implementation should log human-readable lines to standard error for:

- Watch startup and selected debounce/retry settings.
- Initial enforcement result.
- Relevant RandR event receipt, using concise event names where available.
- Enforcement result after debounce.
- Retry attempts and final retry exhaustion.
- Graceful shutdown.

The existing command result status line format should remain unchanged for
non-watch mode.

## Systemd Service Model

Systemd should manage the long-running watcher process. It should not be used as
the hotplug trigger mechanism for this design.

Example system service:

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

Example user service:

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

Notes:

- `Type=simple` is appropriate because the process remains in the foreground.
- `Restart=on-failure` lets systemd restart the watcher if the X connection is
  lost or the process crashes.
- The service must still provide the target Xorg session environment.
- udev rules are not required for normal watch-mode operation.

## Verification Flow

Manual verification:

1. Start an Xorg session with a known `DISPLAY` and `XAUTHORITY`.
2. Run:

   ```bash
   xdisplay-attach enforce --config displays.json --watch
   ```

3. Connect and disconnect a monitor.
4. Confirm that the watcher logs a RandR event, waits for the debounce interval,
   and applies the configured policy.
5. Confirm that repeated hotplug bursts produce one enforcement pass after the
   burst, not one pass per individual event.
6. If a second monitor is physically connected but no enforcement occurs,
   compare kernel/udev hotplug events, Xorg logs, and `xdisplay-attach status`
   before treating the watcher as the failing component.

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

Diagnostic-only udev check:

```bash
udevadm monitor --kernel --udev --subsystem-match=drm --property
```

This can help confirm hardware hotplug behavior, but the watcher must not
depend on this signal.

## Implementation Status

- Implemented: CLI options `--watch`, `--debounce-ms`, `--retry`, and
  `--retry-delay-ms`.
- Implemented: rejection of incompatible `--watch --dry-run`.
- Implemented: RandR event subscription for screen, CRTC, output, output
  property, and resource changes.
- Implemented: debounce and retry logic around existing `enforce` operation.
- Implemented: graceful shutdown request handling for `SIGINT` and `SIGTERM`.
- Implemented: CLI parsing tests and retry-classification tests.
- Implemented: user-guide and CLI-spec documentation for watch mode.

## Acceptance Criteria

- `xdisplay-attach enforce --config FILE --watch` applies the policy once on
  startup and then remains running.
- Connecting or disconnecting a monitor causes a relevant RandR event to be
  observed and the policy to be reapplied after debounce.
- With two monitors connected, the watcher relies on Xorg to expose the second
  monitor through RandR and then converges that RandR state to the configured
  policy.
- A burst of related RandR events results in coalesced enforcement.
- Exit statuses `0`, `10`, and `11` do not stop the watcher.
- Recoverable event-time failures are retried without a busy loop.
- Losing the X connection exits with the existing unavailable status so systemd
  can restart the process.
- `--dry-run --watch` is rejected.
- Documentation does not instruct users to run `xdisplay-attach` from udev.

## Open Questions

- Should the initial enforcement fail fast on status `70` or `71`, or should it
  warn and continue waiting for a later RandR event? The initial implementation
  chooses warn-and-continue after retry exhaustion.
- What debounce default is best for real hardware: `250 ms`, `500 ms`, or
  `1000 ms`?
- Should retry options be exposed in the first implementation or kept as
  internal defaults until field testing?
- Should config changes on disk also trigger reloading, or should watch mode
  watch only RandR state in the first version?
- Should downstream window-layout tools be invoked by a separate service, or
  should this project eventually expose a post-enforce hook? The initial answer
  should remain separate services unless there is a strong requirement.
