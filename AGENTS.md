# AGENTS.md

## Development Commands

Plain Cargo defaults to the CPU Whisper backend:

```bash
cargo build
cargo run -p shadoword-api
cd crates/shadoword-desktop && bun run tauri dev
```

With Nix and Vulkan:

```bash
nix develop
cd crates/shadoword-desktop && bun run tauri dev -- --features whisper-vulkan
cargo run -p shadoword-api --features whisper-vulkan
```

CUDA:

```bash
nix develop .#cuda
cd crates/shadoword-desktop && bun run tauri dev -- --features whisper-cuda
cargo run -p shadoword-api --features whisper-cuda
```

## Required Checks

```bash
cargo check --workspace --all-targets
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cd crates/shadoword-desktop
bun run generate:bindings
bun run check
bun run lint
bun run build
```

## Architecture Overview

Shadoword is a Linux-first Rust workspace for offline speech-to-text with a Tauri 2 desktop client and an optional HTTP API daemon.

### Active Workspace

- `crates/shadoword-shared`
  - canonical model contracts and shared domain types
- `crates/shadoword-model-whisper`
  - Whisper backend implementation
- `crates/shadoword-core`
  - audio capture, WAV/VAD helpers, configuration, model downloads, and transcription service orchestration
- `crates/shadoword-desktop`
  - Tauri 2 + SvelteKit + shadcn-svelte desktop client with local/remote batch and streaming transcription
- `crates/shadoword-api`
  - authenticated batch and streaming transcription service

## Project Direction

- Keep local and remote desktop operation at feature parity.
- Use Bun exclusively for frontend dependency management and scripts; do not add npm or pnpm lockfiles.
- Keep backend-specific inference behind the shared model contract.
- Keep desktop-only and daemon-only configuration separate.
- Publish model and hardware benchmark claims only from explicitly documented manual measurements.
