#!/usr/bin/env bash
# Download the release SEB binary if needed, then copy it onto a board.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
AGENT="$ROOT/exam-env/seb-agent"
CACHE="$ROOT/cache/safe-exam-browser"
SEB_URL="${SEB_URL:-https://github.com/cannon-spencer/seb-linux/releases/download/riscv64/safe-exam-browser}"
# Temporary: the riscv64 release has no libseb-web-extension.so, so this 404s.
# WebKitGTK extension: URL filter + SEB key headers. SEB won't start without it.
# EXT_CACHE="$ROOT/cache/libseb-web-extension.so"
# EXT_URL="${EXT_URL:-https://github.com/cannon-spencer/seb-linux/releases/download/riscv64/libseb-web-extension.so}"
HOST=""
BOARD_PASS="${BOARD_PASS:-orangepi}"
SSH_OPTS=(-o StrictHostKeyChecking=accept-new -o ConnectTimeout=10 -o ServerAliveInterval=30)

usage() {
  echo "Usage: $(basename "$0") --host <user@ip>"
}

die() { echo "error: $*" >&2; exit 1; }

remote() {
  command -v sshpass >/dev/null || die "install sshpass"
  sshpass -p "$BOARD_PASS" ssh "${SSH_OPTS[@]}" "$HOST" "$@"
}

remote_sudo() {
  remote "printf '%s\n' $(printf %q "$BOARD_PASS") | sudo -S -p '' $*"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h | --help) usage; exit 0 ;;
    --host) HOST="${2:?}"; shift 2 ;;
    *) usage >&2; die "unknown argument: $1" ;;
  esac
done

[[ -n "$HOST" ]] || { usage >&2; die "--host is required"; }

if [[ ! -f "$CACHE" ]]; then
  mkdir -p "$(dirname "$CACHE")"
  echo "downloading $SEB_URL"
  curl -fL --retry 3 -o "$CACHE.tmp" "$SEB_URL" || die "no release yet ($SEB_URL)"
  mv "$CACHE.tmp" "$CACHE"
  chmod +x "$CACHE"
else
  echo "using $CACHE"
fi

# if [[ ! -f "$EXT_CACHE" ]]; then
#   echo "downloading $EXT_URL"
#   curl -fL --retry 3 -o "$EXT_CACHE.tmp" "$EXT_URL" || die "no extension in the release yet ($EXT_URL)"
#   mv "$EXT_CACHE.tmp" "$EXT_CACHE"
# else
#   echo "using $EXT_CACHE"
# fi

echo "copy $CACHE -> $HOST:~/safe-exam-browser"
sshpass -p "$BOARD_PASS" scp "${SSH_OPTS[@]}" "$CACHE" "$HOST:~/safe-exam-browser"
remote "chmod +x ~/safe-exam-browser"

echo "copy start-seb.sh and stop-seb.sh"
sshpass -p "$BOARD_PASS" scp "${SSH_OPTS[@]}" \
  "$AGENT/start-seb.sh" "$AGENT/stop-seb.sh" "$HOST:~/"
remote "chmod +x ~/start-seb.sh ~/stop-seb.sh"

# System location, so it is found wherever the binary ends up
# (~/safe-exam-browser, or /usr/local/bin after scripts/install-gpu.sh).
# echo "copy $EXT_CACHE -> $HOST:/usr/lib/seb-linux/webkit-extensions/"
# sshpass -p "$BOARD_PASS" scp "${SSH_OPTS[@]}" "$EXT_CACHE" "$HOST:/tmp/libseb-web-extension.so"
# remote_sudo "install -D -m 644 /tmp/libseb-web-extension.so /usr/lib/seb-linux/webkit-extensions/libseb-web-extension.so"
# remote "rm -f /tmp/libseb-web-extension.so"

echo "apt SEB runtime"
# One sudo shell. A second SSH connection is what skipped this install last time.
# update can fail the snapshot signature check; the image already has indexes.
install='apt-get update || true
DEBIAN_FRONTEND=noninteractive apt-get install -y \
  libwebkit2gtk-4.1-0 libgtk-3-0 \
  libqt6core6 libqt6gui6 libqt6widgets6 libqt6network6 libqt6xml6 \
  libqt6svg6 qt6-qpa-plugins qt6-wayland libxcb-cursor0 libssl3'
remote_sudo "bash -c $(printf %q "$install")"

echo "SEB:  ~/safe-exam-browser"
