#!/usr/bin/env bash
set -euo pipefail

if (( $# < 3 )); then
  echo "usage: $0 <image-archive> <loaded-image> <target-tag>..." >&2
  exit 2
fi

image_archive="$1"
loaded_image="$2"
shift 2

if [[ ! -f "$image_archive" ]]; then
  echo "container image archive not found: $image_archive" >&2
  exit 1
fi

docker image load --input "$image_archive"
docker image inspect "$loaded_image" >/dev/null

for target_tag in "$@"; do
  [[ -n "$target_tag" ]] || continue
  docker image tag "$loaded_image" "$target_tag"
  docker image push "$target_tag"
done
