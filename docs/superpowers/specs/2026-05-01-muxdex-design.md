# muxdex Design

Date: 2026-05-01
Status: Approved for planning

## Summary

`muxdex` is a macOS-friendly terminal dashboard for monitoring tmux sessions. It runs as a standalone TUI inside an existing terminal application such as iTerm2, Terminal.app, Ghostty, or Alacritty. It automatically discovers tmux sessions, shows one tile per session, updates tiles as sessions change, fades tiles when sessions die while preserving the last snapshot, and restores the same tile if a session later reappears with the same name.

The released artifact should be a single self-contained binary. At runtime, the only required external dependency is `tmux`.

## Goals

- Provide a live, read-only dashboard for all active tmux sessions on the machine.
- Work well on macOS, especially inside iTerm2, without requiring any GUI app.
- Avoid interfering with the sessions being monitored.
- Be reliable under frequent tmux session creation and removal.
- Use terminal space effectively with a fluid tiled layout.
- Preserve enough rendering fidelity to be useful for coding-agent sessions, logs, servers, watchers, and tests.

## Non-Goals

- Mirroring a tmux session's full internal pane layout.
- Supporting full interactive control of watched sessions.
- Optimizing for sessions with multiple panes or complex internal window graphs.
- Implementing arbitrary tile rearrangement in v1.
- Providing tile-local scrollback browsing in v1.

## Product Behavior

### Launch and Runtime Model

`muxdex` launches as a normal terminal TUI process in the user's current terminal window. It does not require the user to already be inside tmux, and it does not create a dedicated dashboard tmux session.

### Session Discovery

On startup and during runtime, `muxdex` automatically discovers all active tmux sessions visible from the current environment. No manual registration or configuration is required for sessions to appear.

### Tile Model

Each tmux session maps to exactly one dashboard tile.

For v1, a tile renders only the session's currently active pane. If the session changes active window or active pane, the tile follows that active pane on the next refresh. `muxdex` intentionally does not attempt to expand a session into multiple dashboard tiles.

### Dead Sessions

When a tmux session disappears, its tile remains visible in a faded "dead" state and preserves the last successfully captured snapshot. Dead tiles remain visible until explicitly dismissed.

If a tmux session later reappears with the same name, `muxdex` reactivates the existing tile instead of creating a duplicate tile. The dead styling is removed and live updates resume.

### Tile Actions

Tiles are read-only except for a close action.

For a live session:
- the close action prompts for confirmation
- confirming kills the tmux session
- the tile is then hidden

For a dead session:
- the close action hides the tile immediately

### Empty State

If no tmux sessions are present, `muxdex` shows a clear empty state rather than a blank screen. The empty state should indicate that no tmux sessions were detected and that tiles appear automatically when sessions start.

## UX and Interaction

### Layout

The dashboard uses a responsive tiled grid layout. The layout reflows as terminal size and visible tile count change. The v1 layout algorithm should prioritize predictability and reliability over novelty.

### Tile Styling

Each tile includes:
- a visible border
- a stable per-session color assignment
- a session name label
- a live or dead status indication
- a close affordance rendered in the tile chrome

Dead tiles should appear visually distinct using dimming or faded styling while preserving readable content.

### Input Model

V1 is keyboard-first. Optional mouse support may be added later, but the first implementation should not depend on mouse reporting or terminal-specific mouse behavior.

The UI should support:
- moving focus between tiles
- opening the close confirmation for the focused tile
- confirming or canceling destructive actions
- hiding a dead tile
- quitting the application

The tile chrome may visually show an `X`, but the primary interaction path is keyboard-driven.

## Architecture

`muxdex` should be implemented as a single-process Rust application with four subsystems.

### 1. tmux Probe

A thin adapter shells out to `tmux` to gather:
- the current session list
- each session's active pane identity
- pane metadata such as dimensions and status
- visible pane content snapshots, including styling when tmux can provide it

This layer is read-only except when the user explicitly confirms killing a session.

### 2. Session Store

An in-memory store keyed by session name tracks:
- current lifecycle state: live or dead
- last-seen timestamp
- last good snapshot
- hidden flag
- stable color assignment
- close confirmation state
- stale or error markers for capture failures

This store is the authority for duplicate prevention, fade-out behavior, and resurrection of a session tile when a session name returns.

### 3. Layout Engine

A layout engine computes tile rectangles from the current terminal size and the set of visible records. V1 should use a straightforward responsive grid.

### 4. Renderer and Input Loop

A TUI event loop:
- polls tmux on a fixed cadence
- updates the session store
- redraws when state changes or terminal size changes
- handles keyboard input for navigation and actions

## Data Flow

On each refresh cycle:

1. Query tmux for the current session set.
2. Resolve the active pane for each session.
3. Capture a fresh snapshot for each active pane.
4. Reconcile the results with the session store.
5. Mark missing sessions as dead while keeping their last snapshot.
6. Reactivate matching dead records when a same-named session returns.
7. Render visible records into the dashboard grid.

## Rendering Fidelity

The dashboard is read-only, which materially reduces complexity. `muxdex` does not need to act as a general-purpose PTY host or interactive terminal emulator for watched processes. Tmux already maintains the visible pane state; `muxdex` only needs to query and display that state.

V1 should aim for high practical fidelity by using tmux pane snapshots and preserving styling where available. The goal is faithful monitoring of real coding workflows, not literal embedding of a second live client for the underlying programs.

If later testing shows that tmux snapshots are insufficient for common workflows, the architecture should permit introducing a dedicated emulator core in a future version. That is explicitly out of scope for v1.

## Performance

The UI should feel live without attempting unnecessary frame rates. The recommended model is a change-driven redraw loop with a fallback poll interval of approximately 150-250 ms.

This implies:
- no continuous repaint loop when nothing changes
- redraw on terminal resize
- redraw on store changes
- redraw on confirmation-state changes

## Reliability and Error Handling

### tmux Availability

If `tmux` is not installed or cannot be reached, `muxdex` should show a clear full-screen error state rather than crash or spam raw errors.

### Partial Capture Failures

If one session cannot be captured during a refresh:
- preserve the last good snapshot for that tile
- mark the tile as stale or errored
- continue updating the rest of the dashboard

### Small Terminal Sizes

If the terminal becomes too small to render the full grid comfortably, the layout should degrade predictably. The UI must remain coherent rather than corrupting the screen or crashing.

### Session Kill Behavior

Killing a session is only allowed after explicit user confirmation. If the kill command fails, the tile should remain visible and surface a non-fatal error state.

## V1 Scope

Included in v1:
- automatic discovery of tmux sessions
- one tile per session
- rendering the session's active pane only
- live updates as sessions change
- dead-session fade with frozen last snapshot
- session resurrection by name
- confirmation-before-kill
- hide dead tiles
- responsive grid layout
- stable colored borders
- duplicate prevention
- clear zero-session and error states

Excluded from v1:
- arbitrary drag-and-drop or freeform tile rearrangement
- support for a session's full internal multi-pane layout
- scrollback inspection inside a tile
- per-session configuration
- mandatory mouse-first interaction
- embedded emulator-core reconstruction beyond tmux snapshots

## Implementation Guidance

The tmux integration surface should stay as small as possible. Reliability depends on treating tmux as a read-only data source and keeping dashboard-specific behavior in `muxdex`'s own state model, not in tmux.

The first implementation should prefer simple rules and stable behavior over clever layout or interaction features. Features that create state complexity without clear monitoring value should be deferred.
