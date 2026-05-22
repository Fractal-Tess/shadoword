# Shadoword

Rust workspace for an offline speech-to-text app with:

- a native `egui` desktop client
- an optional HTTP daemon

The active codebase is Rust-native and Whisper-focused.

## Workspace

- `crates/shadoword-core` - shared audio/config/transcription service logic
- `crates/shadoword-model-whisper` - Whisper model implementation
- `crates/shadoword-shared` - shared trait/types contracts
- `crates/shadoword-egui` - native desktop client
- `crates/shadoword-api` - HTTP daemon

## Backend selection

No whisper backend is enabled by default. Choose one explicitly:

- `whisper-vulkan`
- `whisper-cuda`

Examples:

```bash
cargo run -p shadoword-egui --features whisper-vulkan
cargo run -p shadoword-egui --features whisper-cuda
```

## Development

### Nix

```bash
nix develop
cargo run -p shadoword-egui --features whisper-vulkan
```

CUDA shell:

```bash
nix develop .#cuda
cargo run -p shadoword-egui --features whisper-cuda
```

### Plain Cargo

```bash
cargo build
cargo run -p shadoword-egui --features whisper-vulkan
cargo run -p shadoword-api --features whisper-vulkan
```

## Docker (daemon)

```bash
./docker/export-rootfs.sh
docker build -t shadoword-backend .
```

Run with NVIDIA GPU access:

```bash
docker run --rm -p 47813:47813 \
  --device nvidia.com/gpu=all \
  -v $PWD/docker/config:/config \
  -v $HOME/.local/share/shadoword/models:/data/shadoword/models:ro \
  shadoword-backend
```

Daemon endpoints:

```text
GET /
GET /docs
```
