# Shadoword

<p align="center">
  <img src="shadoword-logo.svg" alt="Shadoword waveform mark" width="220">
</p>

Private speech-to-text for Linux. Hold a shortcut, speak, and send the transcript to the active application.

Shadoword supports three execution modes:

- **Local** — offline Whisper inference on your machine
- **Shadoword API** — self-hosted transcription from another machine
- **OpenRouter** — optional direct, batch-only transcription

The desktop is built with Tauri 2 and SvelteKit. Audio capture, inference, credentials, hotkeys, tray behavior, and text delivery remain in Rust.

## Install

Download the Linux desktop archive from [GitHub Releases](https://github.com/Fractal-Tess/shadoword/releases):

```text
shadoword-desktop-x86_64-linux.tar.gz
shadoword-desktop-cuda-x86_64-linux.tar.gz
shadoword-desktop-vulkan-x86_64-linux.tar.gz
```

All desktop archives install the `shadoword` command. Choose the CUDA or Vulkan archive for accelerated local inference. Each release also includes `SHA256SUMS` and matching CPU, CUDA, and Vulkan API daemon archives.

Nix source packages are also available:

```bash
nix build github:Fractal-Tess/shadoword#shadoword-desktop-source
nix build github:Fractal-Tess/shadoword#shadoword-desktop-cuda-source
nix build github:Fractal-Tess/shadoword#shadoword-desktop-vulkan-source
nix build github:Fractal-Tess/shadoword#shadoword-api-source
```

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

## Shadoword API

Generate named bearer tokens before exposing the daemon outside localhost:

```bash
shadoword-api token generate admin "desktop administrator"
shadoword-api token generate user "transcription client"
shadoword-api token list
shadoword-api token revoke "transcription client"
```

Token secrets are printed once. Only SHA-256 hashes are stored in the API configuration.

- **Admin tokens** can manage models, downloads, configuration, and transcription.
- **User tokens** can only submit batch or streaming transcription requests.
- `GET /health` is public.

Start the daemon and download the default Turbo model when needed:

```bash
shadoword-api --download-model turbo
```

Main endpoints:

```text
GET  /health
POST /v1/transcribe-wav
GET  /v1/stream
GET  /v1/overview
GET  /v1/models
```

Use `shadoword-api --help` for configuration, model, and request-recording options.

## Docker

CPU, CUDA, and Vulkan daemon images are published to both `ghcr.io/fractal-tess/shadoword-backend` and `vgfractal/shadoword-backend` on Docker Hub.

| Backend | Rolling tag | Versioned tag |
| --- | --- | --- |
| CPU | `latest` or `cpu` | `<version>` or `<version>-cpu` |
| CUDA | `cuda` | `<version>-cuda` |
| Vulkan | `vulkan` | `<version>-vulkan` |

Mount persistent configuration and model directories at `/config` and `/data`. Start from [`docker/config/shadoword/api.json.example`](docker/config/shadoword/api.json.example), using `"cpu"` for the CPU image and `"gpu"` for CUDA or Vulkan.

CUDA requires the NVIDIA Container Toolkit and `--gpus all`. Vulkan on AMD or Intel requires the render device, for example `--device /dev/dri`, plus access to the device's host group.

The container images are reproducible Flake outputs built directly from the Nix package closures without a mutable distribution base image or `apt-get`. To build and load a variant locally:

```bash
image=$(nix build .#shadoword-container-cpu --no-link --print-out-paths) # or -cuda / -vulkan
docker load < "$image"
```

Release automation reuses the exact CPU, CUDA, and Vulkan API derivations for the binary archives and their corresponding layered container images.

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

See [`CHANGELOG.md`](CHANGELOG.md) for release history and [`crates/shadoword-desktop/README.md`](crates/shadoword-desktop/README.md) for desktop-specific development notes.
