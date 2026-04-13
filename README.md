# Shadoword

Rust workspace for an offline speech-to-text desktop app built with `egui`, plus an optional remote daemon.
The active codebase is Whisper-only.

## Workspace

- `crates/shadoword-core`: shared audio, config, WAV, and transcription service code
- `crates/shadoword-desktop`: native `egui` desktop client
- `crates/shadoword-daemon`: HTTP daemon for remote transcription mode

## Development

### Nix

```bash
nix develop
cargo run -p shadoword-desktop --features whisper-vulkan
```

### Plain Cargo

```bash
cargo build
cargo run -p shadoword-desktop --features whisper-vulkan
cargo run -p shadoword-daemon --features whisper-vulkan
```

## Docker

First container target is the working Whisper GPU daemon path: Vulkan.

This image is intentionally split into:
- a stable Debian runtime layer with only a few runtime packages
- a generated `docker/rootfs` layer exported from the local Nix build

That keeps the OS package layer cached when application code changes. Rebuild flow is:

1. rebuild the Nix daemon closure
2. export it into `docker/rootfs`
3. rebuild the container image

```bash
./docker/export-rootfs.sh
docker build -t shadoword-backend .
```

Run with NVIDIA GPU access:

```bash
docker run --rm -p 47813:47813 \
  --device nvidia.com/gpu=all \
  -v $PWD/docker/config:/config \
  -v $HOME/.local/share/shadowword/models:/data/shadoword/models:ro \
  shadoword-backend
```

The daemon will read config from `/config/shadoword/config.json` and models from `/data/shadoword/models`.
Start by copying `docker/config/config.json.example` to `docker/config/shadoword/config.json`.

This runtime contract is now self-contained from the application side:
- no host `/nix/store` mount
- no host `/run/opengl-driver` mount
- no manual `/dev/nvidia*` or `/dev/dri` mounts

The only host requirement is NVIDIA CDI / container-toolkit support so Docker can inject the GPU
devices and matching driver userspace into the container. On this NixOS machine, the working form
is `--device nvidia.com/gpu=all`.

## Current Direction

This repo is the Rust-native workspace only: desktop client, shared core, and daemon API.
