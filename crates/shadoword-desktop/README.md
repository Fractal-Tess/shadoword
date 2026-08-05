# Shadoword desktop

Tauri 2 + SvelteKit desktop client for Shadoword. It supports local Whisper inference and authenticated remote operation through the Shadoword API.

## Runtime boundary

The webview does not contact the Shadoword daemon or inference backend directly. Tauri commands own local Whisper, remote HTTP/WebSocket requests, bearer authentication, `desktop.json`, microphone capture, global shortcuts, tray behavior, and transcript delivery. This keeps credentials and native capabilities out of browser code.

Normal startup loads native state. The old design fixtures are only enabled explicitly with `?demo=1` during frontend design work; the UI labels that mode as simulated.

Implemented operations:

- local model status, preload/reload, accelerator/GPU selection, verified downloads, custom paths, and batch or VAD-segmented streaming inference;
- authenticated remote status, runtime configuration, catalog selection, model download polling, batch WAV transcription, and 48 kHz Opus WebSocket streaming;
- native microphone enumeration and capture shared by UI controls and push-to-talk/toggle global shortcuts;
- tray show/hide/quit, close-to-tray, clipboard copy, direct typing, and configurable paste shortcuts;
- private endpoint/token and desktop preference persistence plus in-memory session history.

The Shadoword API has no server-side download cancellation, custom-model, or model-delete endpoint. “Stop watching” stops client polling without claiming to cancel the server job; custom paths remain local-only.

## Generated contracts

`tauri-specta` is pinned to an exact release and generates both DTOs and typed `invoke` wrappers in `src/lib/bindings.ts`. The Axum wire DTOs live in `shadoword-core::remote_contracts`, so the daemon and native desktop client share the same Rust definitions. This is more direct than OpenAPI here because the intended caller is the Rust Tauri host, not the webview.

Regenerate and format the checked-in artifact with Cargo and Bun only:

```bash
bun run generate:bindings
```

The export test fails on unsupported contract shapes. Large Rust counters are annotated as JavaScript `number` because API sizes and counters remain below JavaScript's exact-integer limit; JSON transport itself is unchanged.

## Tooling

The frontend uses Bun exclusively:

```bash
bun install
bun run generate:bindings
bun run format
bun run check
bun run lint
bun run test
bun run build
bun run tauri dev
```

Run Tauri through `nix develop` on NixOS so WebKitGTK and libsoup are available.
