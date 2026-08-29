#!/usr/bin/env bash
# Stage 1: write an OS image to SD / eMMC.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="${IMAGE:-debian}" # debian | reptilian

usage() {
  cat <<EOF
Usage: $(basename "$0") [--image debian|reptilian]
EOF
}

resolve_image() {
  case "$IMAGE" in
    debian)
      echo "TODO: resolve Orange Pi Debian image path"
      ;;
    reptilian)
      echo "TODO: resolve image under $ROOT/platform/reptilian-riscv"
      ;;
    *)
      usage >&2
      echo "error: unknown image: $IMAGE" >&2
      exit 1
      ;;
  esac
}

# Confirm the device, then write.
write_image() {
  echo "TODO: write image to a caller-supplied block device"
}

main() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -h | --help)
        usage
        exit 0
        ;;
      --image)
        IMAGE="${2:?}"
        shift 2
        ;;
      *)
        usage >&2
        echo "error: unknown argument: $1" >&2
        exit 1
        ;;
    esac
  done

  resolve_image
  write_image

  echo "TODO: implementation missing"
  exit 0
}

main "$@"
