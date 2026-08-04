# Changelog

All notable changes to Shadoword are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and Shadoword uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/Fractal-Tess/shadoword/compare/v0.10.1...HEAD
[0.10.1]: https://github.com/Fractal-Tess/shadoword/compare/v0.10.0...v0.10.1
[0.10.0]: https://github.com/Fractal-Tess/shadoword/compare/v0.9.2...v0.10.0
[0.9.2]: https://github.com/Fractal-Tess/shadoword/compare/v0.9.1...v0.9.2
[0.9.1]: https://github.com/Fractal-Tess/shadoword/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/Fractal-Tess/shadoword/compare/v0.8.2...v0.9.0
