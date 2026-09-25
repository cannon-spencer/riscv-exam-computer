#!/usr/bin/env bash
# Download the release SEB binary if needed, then copy it onto a board.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CACHE="$ROOT/cache/safe-exam-browser"
SEB_URL="${SEB_URL:-https://github.com/cannon-spencer/seb-linux/releases/download/riscv64/safe-exam-browser}"
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

echo "copy $CACHE -> $HOST:~/safe-exam-browser"
sshpass -p "$BOARD_PASS" scp "${SSH_OPTS[@]}" "$CACHE" "$HOST:~/safe-exam-browser"
remote "chmod +x ~/safe-exam-browser"

echo "apt SEB runtime"
remote_sudo "apt-get update"
remote_sudo "apt-get install -y libwebkit2gtk-4.1-0 libgtk-3-0 \
  libqt6core6 libqt6gui6 libqt6widgets6 libqt6network6 libqt6xml6 \
  libqt6svg6 qt6-qpa-plugins libxcb-cursor0 libssl3"

echo "SEB:  ~/safe-exam-browser"
