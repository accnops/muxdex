# muxdex

Read-only terminal dashboard for monitoring tmux sessions.

`muxdex` runs as a normal TUI in your terminal and shows one tile per tmux session. It follows each session's active pane, keeps the last snapshot when a session dies, and revives the same tile if a session with the same name comes back.

## Install

### macOS from GitHub Releases

Install the latest release into `~/.local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/accnops/muxdex/main/install.sh | sh
```

Install into a different directory:

```bash
curl -fsSL https://raw.githubusercontent.com/accnops/muxdex/main/install.sh | BIN_DIR=/usr/local/bin sh
```

### Manual download

Download the archive that matches your Mac from the [latest release](https://github.com/accnops/muxdex/releases/latest):

- `muxdex-aarch64-apple-darwin.tar.gz` for Apple Silicon
- `muxdex-x86_64-apple-darwin.tar.gz` for Intel

Extract the archive and place `muxdex` somewhere on your `PATH`.

### Build from source

```bash
cargo run
```

## Requirements

- macOS
- `tmux`

If no tmux server is running, `muxdex` shows an empty state and starts tracking sessions automatically as they appear.

## Usage

```bash
muxdex
```

Keybindings:

- `tab` / `shift-tab`: move focus
- `x`: close the focused tile
- `enter`: confirm kill in the modal
- `esc`: cancel the modal
- `q`: quit

## Agent workflow

Suggested system-prompt guidance for coding agents:

> Prefer `tmux` for permanent processes and for long-running work, roughly anything expected to run longer than a minute, especially when the user may want to inspect logs or reattach later. Do not use `tmux` for short-lived commands where the extra session adds little value. For suitable tasks, use a clearly named `tmux` session, reuse relevant existing sessions.

## Development

```bash
cargo fmt
cargo test
cargo build
```

## Release process

Push a version tag like `v0.1.0`:

```bash
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions will:

1. build `muxdex` for Apple Silicon and Intel macOS
2. package each binary as a `.tar.gz`
3. create or update the GitHub Release for that tag
4. attach release artifacts and a checksum file
