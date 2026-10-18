# Contributing to Shadoword

See [AGENTS.md](AGENTS.md) for project rules and release steps.

## Develop

CPU Whisper is the default backend:

```bash
cargo build
cargo run -p shadoword-api

cd crates/shadoword-desktop
bun install
bun run tauri dev
```

Vulkan:

```bash
nix develop
cd crates/shadoword-desktop
bun run tauri dev -- --features whisper-vulkan
```

CUDA:

```bash
nix develop .#cuda
cd crates/shadoword-desktop
bun run tauri dev -- --features whisper-cuda
```

## Checks

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

## Workspace

- `crates/shadoword-core` — audio, configuration, models, and transcription orchestration
- `crates/shadoword-model-whisper` — Whisper backend
- `crates/shadoword-shared` — shared model contracts
- `crates/shadoword-desktop` — Tauri and SvelteKit desktop
- `crates/shadoword-api` — HTTP and WebSocket daemon
- `website` — public website

See [`crates/shadoword-desktop/README.md`](crates/shadoword-desktop/README.md) for desktop-specific development notes.

## README images

The README cover and gallery are composed from HTML layouts in `assets/screenshots/source/` (`hero.html`, `models.html`, `vocabulary.html`, `execution.html`, sharing `style.css`). They load raw captures from `assets/screenshots/source/shots/`, which are not committed.

To remake them, run the desktop frontend in a browser (`cd crates/shadoword-desktop && bun run dev`) with `window.__TAURI_INTERNALS__.invoke` stubbed to return a local-mode `DesktopBootstrap`, history, and overview. Hide the in-app title bar (`show_window_title_bar: false`), since the layouts draw their own. Capture at a device scale of 2:

- `transcribe.png`: `/` at 1440 × 1000, while recording, with two streamed segments
- `history-small.png`: `/history` at 1120 × 800
- `models-list.png`: `/models` at 1440 × 1000, scrolled to the Whisper models list
- `transcription.png` and `settings.png`: `/transcription` and `/settings` at 1440 × 1000

Then render and convert each layout:

```bash
cd assets/screenshots/source
for f in hero models vocabulary execution; do
  google-chrome-stable --headless=new --hide-scrollbars --force-device-scale-factor=2 \
    --window-size=1600,1000 --screenshot=/tmp/$f.png "file://$PWD/$f.html"
  cwebp -q 82 /tmp/$f.png -o ../$f.webp
done
```
