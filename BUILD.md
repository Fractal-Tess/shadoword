# Build

## Prerequisites

- Rust stable
- On Linux/NixOS: audio, Wayland/X11, Vulkan/OpenGL runtime libraries available through the flake shell

## Common Commands

```bash
cargo build
cargo build -p shadoword-egui --features whisper-vulkan
cargo build -p shadoword-api --features whisper-vulkan

cargo run -p shadoword-egui --features whisper-vulkan
cargo run -p shadoword-api --features whisper-vulkan
```

No whisper backend is enabled by default. Pick one explicitly:

```bash
# Vulkan
cargo run -p shadoword-egui --features whisper-vulkan

# CUDA
cargo run -p shadoword-egui --features whisper-cuda
```

## Nix

```bash
nix develop
cargo run -p shadoword-egui --features whisper-vulkan
```

CUDA:

```bash
nix develop .#cuda
cargo run -p shadoword-egui --features whisper-cuda
```

## Notes

- The desktop client now uses `eframe`/`egui` with the `wgpu` renderer.
- The old Tauri/Vite build is no longer part of the active workspace.
