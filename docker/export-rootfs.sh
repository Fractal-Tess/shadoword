#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

nix build .#shadoword-daemon --no-link
out_path="$(nix path-info .#shadoword-daemon)"
rootfs_dir="$repo_root/docker/rootfs"

if [[ -e "$rootfs_dir" ]]; then
  chmod -R u+w "$rootfs_dir" 2>/dev/null || true
  rm -rf "$rootfs_dir"
fi
mkdir -p "$rootfs_dir/usr/local/bin"

while IFS= read -r path; do
  cp -a --parents "$path" "$rootfs_dir"
done < <(nix path-info .#shadoword-daemon -r)

ln -sf "$out_path/bin/shadoword-daemon" "$rootfs_dir/usr/local/bin/shadoword-daemon"

echo "Exported daemon closure to $rootfs_dir"
echo "Daemon path: $out_path"
