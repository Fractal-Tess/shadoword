# AGENTS.md

## Commands

CPU is the default Whisper backend:

```bash
cargo build
cargo run -p shadoword-api
cd crates/shadoword-desktop && bun run tauri dev
```

Vulkan:

```bash
nix develop
cargo run -p shadoword-api --features whisper-vulkan
cd crates/shadoword-desktop && bun run tauri dev -- --features whisper-vulkan
```

CUDA:

```bash
nix develop .#cuda
cargo run -p shadoword-api --features whisper-cuda
cd crates/shadoword-desktop && bun run tauri dev -- --features whisper-cuda
```

## Required checks

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

Do not run test suites unless the user explicitly requests them.

## Architecture

- `shadoword-shared` — canonical model contracts and domain types
- `shadoword-model-whisper` — Whisper backend
- `shadoword-core` — audio, configuration, models, VAD, and transcription orchestration
- `shadoword-desktop` — Tauri 2 and SvelteKit desktop
- `shadoword-api` — authenticated HTTP and WebSocket daemon

## Project rules

- Keep Local, Shadoword API, and OpenRouter desktop operation at feature parity where the provider supports it.
- Keep capture, inference, credentials, networking, hotkeys, tray behavior, and output delivery in Rust.
- Keep backend-specific inference behind the shared model contract.
- Rust 1.94 is the minimum; keep Cargo, `rust-toolchain.toml`, CI, and the Nix development environment aligned.
- Keep desktop and daemon configuration separate.
- Let the desktop application own its mutable settings JSON; do not generate it through NixOS.
- Use named, hashed API tokens. Do not restore legacy token environment variables or token-file support.
- Use Bun exclusively for frontend dependencies and scripts.
- Keep route-specific Svelte code beside its route and reserve `$lib` for shared UI, global state, generated bindings, and cross-route utilities.
- Use Tailwind utilities and the shared semantic UI components instead of new one-off component styles.
- Publish performance claims only from documented manual measurements.

## Releases

- Keep workspace, desktop package, Tauri, changelog, and tag versions aligned.
- Preserve the optimized Cargo release profile: fat LTO, one codegen unit, optimization level 3, abort-on-panic, and stripped symbols.
- Push the release tag and let GitHub Actions build the release archives.
- Do not build release application binaries on the local system.
- Update NixOS only after CI succeeds and the prebuilt release manifest has been published.
