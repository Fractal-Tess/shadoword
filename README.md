# Shadoword

Rust workspace for an offline speech-to-text desktop app built with `egui`, plus an optional remote daemon.

## Workspace

- `crates/shadoword-core`: shared audio, config, WAV, and transcription service code
- `crates/shadoword-desktop`: native `egui` desktop client
- `crates/shadoword-daemon`: HTTP daemon for remote transcription mode

## Development

### Nix

```bash
direnv reload
cargo run -p shadoword-desktop
```

CUDA shell:

```bash
nix develop .#cuda
cargo run -p shadoword-desktop --features cuda
```

### Plain Cargo

```bash
cargo build
cargo run -p shadoword-desktop
cargo run -p shadoword-daemon
```

## Docker

First container target is the working Whisper GPU daemon path: Vulkan.

Build:

```bash
docker build -t shadoword-backend --target runtime-vulkan .
```

Run on NixOS with NVIDIA Vulkan passthrough:

```bash
docker run --rm -p 47813:47813 \
  --device /dev/dri \
  -v /run/opengl-driver:/run/opengl-driver:ro \
  -e VK_ICD_FILENAMES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.x86_64.json \
  -e VK_DRIVER_FILES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.x86_64.json \
  -e VK_LAYER_PATH=/run/opengl-driver/share/vulkan/implicit_layer.d:/run/opengl-driver/share/vulkan/explicit_layer.d \
  -v $PWD/docker/config:/config \
  -v $PWD/docker/data:/data \
  shadoword-backend
```

The daemon will read config from `/config/shadoword/config.json` and models from `/data/shadoword/models`.
Start by copying `docker/config/config.json.example` to `docker/config/shadoword/config.json`.

## Current Direction

This repo is the Rust-native workspace only: desktop client, shared core, and daemon API.
