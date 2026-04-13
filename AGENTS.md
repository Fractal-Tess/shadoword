# AGENTS.md

## Development Commands

```bash
cargo build
cargo run -p shadowword-desktop
cargo run -p shadowword-daemon
```

With Nix:

```bash
nix develop
cargo run -p shadowword-desktop
```

CUDA:

```bash
nix develop .#cuda
cargo run -p shadowword-desktop --features cuda
```

## Architecture Overview

Shadow Word is now a Rust workspace with a native `egui` desktop client and an optional remote daemon.

### Workspace Layout

- `crates/shadowword-core`
  - shared audio capture
  - WAV helpers
  - config loading/saving
  - local transcription service logic
- `crates/shadowword-desktop`
  - `eframe` / `egui` desktop UI
  - local microphone capture
  - local or remote transcription flow
  - clipboard / active-window output
- `crates/shadowword-daemon`
  - HTTP daemon for remote transcription

## Current Migration Context

- The old Tauri/React app has been removed from the active repo.
- The feature inventory for the removed app is documented in `docs/tauri-functionality-map.md`.
- New work should extend the Rust-native `egui` desktop app rather than reintroducing web/Tauri UI code.
