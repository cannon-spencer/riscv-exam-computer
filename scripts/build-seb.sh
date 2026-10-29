#!/usr/bin/env bash
# Build seb-linux on a board from platform/seb-linux and copy the results
# into cache/, where install-seb.sh picks them up. Optionally upload them to
# the riscv64 release.
#
# Nothing is cloned on the board: the submodule's committed tree is streamed
# over SSH. The board still needs internet for the apt build dependencies.
# The build runs detached on the board, so this script can be stopped and
# rerun; it reattaches to a running build of the same commit.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/platform/seb-linux"
CACHE_DIR="$ROOT/cache"
REMOTE_DIR="seb-linux-build"
RELEASE_REPO="${RELEASE_REPO:-cannon-spencer/seb-linux}"
HOST=""
UPLOAD=0
BOARD_PASS="${BOARD_PASS:-orangepi}"
SSH_OPTS=(-o StrictHostKeyChecking=accept-new -o ConnectTimeout=10 -o ServerAliveInterval=30)

usage() {
  echo "Usage: $(basename "$0") --host <user@ip> [--upload]"
  echo "  --host    SSH target, e.g. orangepi@10.0.0.40 (an Orange Pi RV on the project image)"
  echo "  --upload  also upload both files to the riscv64 release of $RELEASE_REPO (needs gh)"
  echo
  echo "Builds whatever commit platform/seb-linux has checked out."
}

die() { echo "error: $*" >&2; exit 1; }

remote() {
  command -v sshpass >/dev/null || die "install sshpass"
  sshpass -p "$BOARD_PASS" ssh "${SSH_OPTS[@]}" "$HOST" "$@"
}

remote_sudo() {
  remote "printf '%s\n' $(printf %q "$BOARD_PASS") | sudo -S -p '' $*"
}

fetch() {
  sshpass -p "$BOARD_PASS" scp "${SSH_OPTS[@]}" "$HOST:$1" "$2"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h | --help) usage; exit 0 ;;
    --host) HOST="${2:?}"; shift 2 ;;
    --upload) UPLOAD=1; shift ;;
    *) usage >&2; die "unknown argument: $1" ;;
  esac
done

[[ -n "$HOST" ]] || { usage >&2; die "--host is required"; }
[[ -f "$SRC/seb-linux-qt.pro" ]] || die "platform/seb-linux is empty; run: git submodule update --init platform/seb-linux"
[[ -f "$SRC/src/browser/engines/webkitgtk/extension/seb_web_extension.c" ]] \
  || die "platform/seb-linux has no WebKitGTK extension; check out a seb-linux commit that has it"
if [[ -n "$(git -C "$SRC" status --porcelain)" ]]; then
  echo "warning: platform/seb-linux has uncommitted changes; only committed files are built"
fi

REV="$(git -C "$SRC" rev-parse HEAD)"
# Expanded by the shell on the board, not here.
# shellcheck disable=SC2088
R="~/$REMOTE_DIR"

remote_rev="$(remote "cat $R/.rev 2>/dev/null" || true)"
remote_done="$(remote "cat $R/build.done 2>/dev/null" || true)"
remote_running=0
remote "pgrep -f 'scripts/[b]uild.sh CONFIG' >/dev/null" && remote_running=1

if [[ "$remote_rev" == "$REV" && "$remote_done" == "0" ]]; then
  echo "board already has a finished build of ${REV:0:12}"
elif [[ "$remote_rev" == "$REV" && "$remote_running" == 1 ]]; then
  echo "board is already building ${REV:0:12}; waiting for it"
else
  echo "send platform/seb-linux ${REV:0:12} -> $HOST:$R"
  git -C "$SRC" archive --format=tar HEAD \
    | remote "rm -rf $R && mkdir -p $R && tar -x -C $R && echo $REV > $R/.rev"

  echo "apt build dependencies"
  # Boards with the GPU stack (install-gpu.sh) have their own WebKit and its
  # headers, and hold the Debian packages; don't ask apt to replace them.
  webkit_dev="libwebkit2gtk-4.1-dev"
  if remote "find /usr/lib /usr/local/lib -name webkit2gtk-web-extension-4.1.pc -print -quit 2>/dev/null | grep -q ."; then
    echo "using the board's installed WebKitGTK headers"
    webkit_dev=""
  fi
  # One sudo shell, like install-seb.sh. update can fail the snapshot
  # signature check; the image already has indexes.
  deps="apt-get -q update || true
DEBIAN_FRONTEND=noninteractive apt-get -q install -y --no-install-recommends \\
  pkg-config build-essential ca-certificates file qmake6 qt6-base-dev qt6-tools-dev-tools \\
  libqt6svg6-dev libgtk-3-dev libssl-dev libtiff-dev libxcb-cursor0 zlib1g-dev $webkit_dev"
  remote_sudo "bash -c $(printf %q "$deps")"

  echo "start build on the board (an hour or two)"
  remote "cd $R && nohup bash -c './scripts/build.sh CONFIG+=force_webkitgtk > build.log 2>&1; echo \$? > build.done' </dev/null >/dev/null 2>&1 &"
fi

# Wait for the detached build. Losing the connection for a while is fine.
while ! remote "test -f $R/build.done" 2>/dev/null; do
  if ! remote "pgrep -f 'scripts/[b]uild.sh CONFIG' >/dev/null" 2>/dev/null; then
    remote "test -f $R/build.done" 2>/dev/null && break
    if remote true 2>/dev/null; then
      remote "tail -n 30 $R/build.log" || true
      die "build stopped without finishing (board rebooted?); rerun this script"
    fi
  fi
  line="$(remote "tail -n 1 $R/build.log 2>/dev/null" 2>/dev/null || true)"
  echo "  $(date +%H:%M) ${line:0:110}"
  sleep 60
done

if [[ "$(remote "cat $R/build.done")" != "0" ]]; then
  remote "tail -n 40 $R/build.log" || true
  die "build failed; full log on the board: $R/build.log"
fi
remote "file $R/build/bin/safe-exam-browser | grep -qiE 'riscv64|RISC-V'" || die "binary is not riscv64"

mkdir -p "$CACHE_DIR"
echo "copy safe-exam-browser and libseb-web-extension.so -> cache/"
fetch "$R/build/bin/safe-exam-browser" "$CACHE_DIR/safe-exam-browser"
fetch "$R/build/bin/webkit-extensions/libseb-web-extension.so" "$CACHE_DIR/libseb-web-extension.so"
chmod +x "$CACHE_DIR/safe-exam-browser"

if [[ "$UPLOAD" == 1 ]]; then
  command -v gh >/dev/null || die "install gh to upload"
  echo "upload to the riscv64 release of $RELEASE_REPO"
  gh release upload riscv64 "$CACHE_DIR/safe-exam-browser" "$CACHE_DIR/libseb-web-extension.so" \
    --clobber -R "$RELEASE_REPO"
fi

echo "built seb-linux ${REV:0:12}:"
echo "  $CACHE_DIR/safe-exam-browser"
echo "  $CACHE_DIR/libseb-web-extension.so"
echo "install on a board: ./scripts/install-seb.sh --host <user@ip>"
