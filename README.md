# Shadow Word

Rust workspace for an offline speech-to-text desktop app built with `egui`, plus an optional remote daemon.

## Workspace

- `crates/shadowword-core`: shared audio, config, WAV, and transcription service code
- `crates/shadowword-desktop`: native `egui` desktop client
- `crates/shadowword-daemon`: HTTP daemon for remote transcription mode
- `docs/tauri-functionality-map.md`: feature inventory from the removed Tauri/React app

## Development

### Nix

```bash
direnv reload
cargo run -p shadowword-desktop
```

CUDA shell:

```bash
nix develop .#cuda
cargo run -p shadowword-desktop --features cuda
```

### Plain Cargo

```bash
cargo build
cargo run -p shadowword-desktop
cargo run -p shadowword-daemon
```

## Current Direction

The web/Tauri application has been retired from the active repo. The remaining work is to move missing product functionality into the Rust-native `egui` client in stages instead of maintaining two separate desktop stacks.
