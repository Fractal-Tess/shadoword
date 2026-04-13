# Build

## Prerequisites

- Rust stable
- On Linux/NixOS: audio, Wayland/X11, Vulkan/OpenGL runtime libraries available through the flake shell

## Common Commands

```bash
cargo build
cargo build -p shadowword-desktop
cargo build -p shadowword-daemon

cargo run -p shadowword-desktop
cargo run -p shadowword-daemon
```

## Nix

```bash
nix develop
cargo run -p shadowword-desktop
```

CUDA:

```bash
nix develop .#cuda
cargo run -p shadowword-desktop --features cuda
```

## Notes

- The desktop client now uses `eframe`/`egui` with the `wgpu` renderer.
- The old Tauri/Vite build is no longer part of the active workspace.
