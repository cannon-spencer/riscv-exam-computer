#!/usr/bin/env bash
# Download the CI Orange Pi RV image if needed, then write it to an SD card.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CACHE="$ROOT/cache/os.img"
URL="https://github.com/cannon-spencer/reptilian-riscv/releases/download/orangepi-rv/os.img.xz"
DEVICE=""
IMAGE=""

usage() {
  echo "Usage: $(basename "$0") --device <disk> [--image <path>]"
  echo "macOS: --device /dev/rdisk4    Linux: --device /dev/sdX"
}

die() { echo "error: $*" >&2; exit 1; }

find_image() {
  if [[ -n "$IMAGE" ]]; then
    [[ -f "$IMAGE" ]] || die "no file $IMAGE"
    return
  fi
  IMAGE="$CACHE"
  [[ -f "$IMAGE" ]] && return

  mkdir -p "$(dirname "$CACHE")"
  echo "downloading $URL"
  curl -fL --retry 3 -o "$CACHE.xz" "$URL"
  command -v xz >/dev/null || die "install xz (brew install xz)"
  xz -dkc "$CACHE.xz" >"$CACHE"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h | --help) usage; exit 0 ;;
    --device) DEVICE="${2:?}"; shift 2 ;;
    --image) IMAGE="${2:?}"; shift 2 ;;
    *) usage >&2; die "unknown argument: $1" ;;
  esac
done

[[ -n "$DEVICE" ]] || { usage >&2; die "--device is required"; }
[[ "$DEVICE" != /dev/disk0 && "$DEVICE" != /dev/rdisk0 && "$DEVICE" != /dev/sda ]] \
  || die "refusing to write $DEVICE"

find_image

echo "About to overwrite $DEVICE with $IMAGE"
if [[ "$(uname -s)" == "Darwin" ]]; then diskutil list; else lsblk; fi
printf "Type the device path again: "
read -r confirm
[[ "$confirm" == "$DEVICE" ]] || die "confirmation did not match"

if [[ "$(uname -s)" == "Darwin" ]]; then
  whole="${DEVICE/\/dev\/rdisk/\/dev\/disk}"
  diskutil unmountDisk "$whole" || true
  sudo dd if="$IMAGE" of="$DEVICE" bs=4m status=progress
else
  sudo umount "${DEVICE}"* 2>/dev/null || true
  sudo dd if="$IMAGE" of="$DEVICE" bs=4M status=progress conv=fsync
fi
sync
echo "wrote $IMAGE -> $DEVICE"
