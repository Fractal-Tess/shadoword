# Changelog

All notable changes to Shadoword are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and Shadoword uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.11.0] - 2026-08-06

### Added

- Named admin and transcription-only user API tokens generated through `shadoword-api token generate`, with one-time secret output and hashed persistence.
- An accessible API-port tooltip beside the desktop endpoint field.
- Configurable whitespace before and after delivered transcripts, with a trailing space by default so consecutive recordings remain separated.
- Secure native credential reveal and copy controls that keep bearer tokens and OpenRouter keys masked in the frontend.
- Authenticated model deletion with active-model and in-progress-download protection.

### Changed

- **Breaking:** Shadoword API authentication now uses the API-owned named-token registry. The legacy `SHADOWORD_API_TOKEN`, `SHADOWORD_API_TOKEN_FILE`, and `--token-file` configuration paths were removed.
- Migrated the desktop to native SvelteKit route pages with route-owned settings state, shared application-lifetime operations, semantic UI components, and Sonner notifications.
- Made explicit worker pools the only execution topology and moved CPU/GPU worker targeting into each pool unit.
- Separated model asset management from execution topology and added runtime, storage, preload, memory, download, selection, and deletion reconciliation.
- Save discrete desktop settings immediately while retaining a short debounce for text entry and avoiding unrelated runtime refreshes.
- Build release binaries with fat LTO, one codegen unit, abort-on-panic semantics, and stripped symbols.

### Fixed

- Automatically refresh draining inference generations so completed worker teardown no longer leaves desktop pool mutations locked by stale status.
- Stop NixOS from regenerating the mutable desktop settings JSON and overwriting preferences saved in the application.
- Report outdated Shadoword API daemons and transcription-only token permission failures with actionable desktop errors.
- Keep mode-specific transcription, PCM format, and English-only preferences stable across target switches and restarts.

## [0.10.6] - 2026-08-05

### Added

- A global Local, Shadoword API, and OpenRouter selector above the command-rail navigation, with keyboard operation and mode-aware page routing.

### Changed

- Settings now show only controls that apply to the selected execution environment; OpenRouter is explicitly batch-only and remote PCM precision appears only for live Shadoword API streaming.
- Local transcription now canonicalizes streaming audio to 32-bit float in both the desktop UI and native configuration boundary.
- Large desktop views, state management, settings, execution-pool controls, and global styles were split into focused Svelte 5 context-backed modules under 300 lines each.
- Removed the project's automated test suites and test jobs while retaining formatting, static analysis, lint, compile, and production-build validation.
- Renamed the installed desktop command from `shadoword-desktop` to `shadoword`.

### Fixed

- Forced packaged WebKitGTK launches onto XWayland to avoid Wayland protocol errors under NVIDIA/Hyprland, including direct CLI launches outside the systemd user service.

## [0.10.5] - 2026-08-05

### Changed

- Collapsed the remote-only and CPU desktop packages into one unified desktop build that always includes Local CPU, Shadoword API, and OpenRouter transcription.
- Kept the historical `shadoword-desktop-client` Nix outputs as aliases to the unified desktop package so existing configurations gain local inference without migration.
- Replaced the two desktop release archives with `shadoword-desktop-x86_64-linux.tar.gz`.

### Fixed

- Restored the Local execution option for installations that previously selected the remote-only desktop-client package.

## [0.10.4] - 2026-08-05

### Added

- Dedicated Execution, Capture, Transcription, Output, and Application pages with large Local, Shadoword API, and OpenRouter target cards.
- Launcher-visible Linux desktop metadata and icons in source and prebuilt Nix desktop packages.
- Trustworthy OpenRouter request-cost reporting in session History when the provider returns an explicit usage cost.

### Changed

- Desktop settings now save automatically with debouncing, capture locks, serialized secret updates, validation, and bounded retries.
- OpenRouter keys are validated automatically at their complete format and persisted only after verification.
- Native dropdowns and execution-pool checkboxes now use high-contrast neon-brutalist selects and explicit ON/OFF switches.
- App and website copy now describe Shadoword as local and offline by default while naming the optional Shadoword API and OpenRouter paths.

## [0.10.3] - 2026-08-04

### Added

- Direct OpenRouter speech-to-text as a desktop transcription target, with native API-key storage and validation, dynamic transcription-model discovery, bounded WAV uploads, and batch-only capture semantics.
- A full CPU desktop release archive and Nix package that combine the Tauri UI with the embedded local Whisper runtime while retaining remote and OpenRouter modes.

### Changed

- The tray icon now switches from neutral gray to scarlet while microphone recording is active and returns to gray during finalization, cancellation, failure, and idle states.
- Linux releases now cover API-only, remote/OpenRouter desktop-only, and combined desktop/runtime installations.

## [0.10.2] - 2026-08-04

### Changed

- Simplified the desktop shell to the command rail and work surface, removing the custom top bar and redundant signal-path sidebar.
- Constrained the capture and transcript surfaces to the native viewport so the primary transcription workflow no longer requires scrolling.

## [0.10.1] - 2026-08-04

### Changed

- Increased the desktop typography, control, brand-mark, status, and navigation-icon scales for comfortable reading on high-resolution Linux displays.
- Increased the default desktop window to 1440 × 900 with a practical 900 × 640 minimum.

### Fixed

- Replaced native window decorations with draggable Shadoword chrome and visible minimize, maximize, and close controls.
- Restored visual prominence to navigation, signal-path, target, and recording icons that previously rendered too small.

## [0.10.0] - 2026-08-04

### Added

- WebSocket protocol v3 for immediate mono `pcm_f32le` or `pcm_s16le` streaming with daemon-side VAD, bounded automatic segment scheduling, and ordered partial responses.
- Desktop streaming PCM precision setting for controlled 16-bit integer versus 32-bit float transcription comparisons.

### Changed

- Remote desktop streaming now sends microphone samples as capture callbacks produce them instead of waiting for client-side VAD segments.

### Fixed

- Replaced the slow pure-Rust Opus compatibility decoder with native libopus, removing audio-duration-proportional ingestion delays for protocol v1/v2 clients.
- Preserved Rust 1.88 builds with local compatibility patches for the generated-binding dependency stack.

## [0.9.2] - 2026-08-03

### Fixed

- Built the release desktop client with Tauri's production custom protocol so it loads bundled assets instead of trying the development server at `localhost:5173`.

## [0.9.1] - 2026-08-03

### Added

- Precompiled Vulkan API daemon archives for tagged Linux x86_64 releases.

### Fixed

- Regenerated and verified desktop TypeScript bindings before frontend linting in CI.

## [0.9.0] - 2026-08-03

### Added

- Tauri 2, SvelteKit 5, and shadcn-svelte desktop application with local and authenticated remote daemon modes.
- Batch and VAD-segmented streaming transcription with ordered partial delivery.
- Configurable CPU, GPU, and mixed inference pools with one model instance per execution unit.
- Bounded flow-aware scheduling, queue and audio-memory limits, per-flow backpressure, cancellation accounting, and worker health reporting.
- Concurrent WebSocket protocol v2 with indexed segments and explicit credits while retaining protocol v1 compatibility.
- Atomic runtime generations for safe model, device, and execution-pool reloads.
- Desktop execution-pool editor and live utilization monitor.
- Secure native token storage boundary, global hotkeys, tray behavior, history, clipboard output, and active-window transcript delivery.
- API request WAV and response metadata archiving for opt-in debugging.
- GitHub release automation for CPU daemon, CUDA daemon, and remote desktop client binaries.

### Changed

- Replaced the previous egui desktop client with the Tauri desktop application.
- Reduced minimum accepted VAD speech duration to 150 ms so short paused words are preserved.
- Moved shared API and desktop contracts into the core crate with generated TypeScript bindings.
- Reworked daemon and desktop inference around the shared multi-unit runtime.

### Fixed

- Preserved short streaming utterances that were previously discarded.
- Guaranteed A/B/C segment delivery order even when later segments finish inference first.
- Prevented stale runtime generations and failed persistence from partially applying configuration.
- Prevented duplicate model loads after eager-preload timeouts.
- Added bounded remote connect, handshake, read, write, and finalization waits.

[Unreleased]: https://github.com/Fractal-Tess/shadoword/compare/v0.11.0...HEAD
[0.11.0]: https://github.com/Fractal-Tess/shadoword/compare/v0.10.6...v0.11.0
[0.10.6]: https://github.com/Fractal-Tess/shadoword/compare/v0.10.5...v0.10.6
[0.10.5]: https://github.com/Fractal-Tess/shadoword/compare/v0.10.4...v0.10.5
[0.10.4]: https://github.com/Fractal-Tess/shadoword/compare/v0.10.3...v0.10.4
[0.10.3]: https://github.com/Fractal-Tess/shadoword/compare/v0.10.2...v0.10.3
[0.10.2]: https://github.com/Fractal-Tess/shadoword/compare/v0.10.1...v0.10.2
[0.10.1]: https://github.com/Fractal-Tess/shadoword/compare/v0.10.0...v0.10.1
[0.10.0]: https://github.com/Fractal-Tess/shadoword/compare/v0.9.2...v0.10.0
[0.9.2]: https://github.com/Fractal-Tess/shadoword/compare/v0.9.1...v0.9.2
[0.9.1]: https://github.com/Fractal-Tess/shadoword/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/Fractal-Tess/shadoword/compare/v0.8.2...v0.9.0
