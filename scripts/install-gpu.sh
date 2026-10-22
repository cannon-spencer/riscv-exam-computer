#!/usr/bin/env bash
# Download the Orange Pi GPU stack release if needed, install it on a board,
# and install the cage launchers. Run scripts/install-seb.sh first.
# Does not compile WebKit.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CACHE_DIR="$ROOT/cache"
TARBALL="$CACHE_DIR/seb-gpu-orangepi-rv.tar.gz"
SUM="$CACHE_DIR/seb-gpu-orangepi-rv.tar.gz.sha256"
URL="${GPU_URL:-https://github.com/cannon-spencer/riscv-exam-computer/releases/download/gpu-stack-v1/seb-gpu-orangepi-rv.tar.gz}"
SUM_URL="${GPU_SUM_URL:-https://github.com/cannon-spencer/riscv-exam-computer/releases/download/gpu-stack-v1/seb-gpu-orangepi-rv.tar.gz.sha256}"
HOST=""
BOARD_PASS="${BOARD_PASS:-orangepi}"
SSH_OPTS=(-o StrictHostKeyChecking=accept-new -o ConnectTimeout=10 -o ServerAliveInterval=30)

usage() {
  echo "Usage: $(basename "$0") --host <user@ip>"
  echo "  --host   SSH target, e.g. orangepi@10.0.0.40"
  echo
  echo "Run scripts/install-seb.sh --host <user@ip> first."
  echo "BOARD_PASS defaults to orangepi (image default). Export to override."
}

die() { echo "error: $*" >&2; exit 1; }

remote() {
  command -v sshpass >/dev/null || die "install sshpass: brew install hudochenkov/sshpass/sshpass"
  sshpass -p "$BOARD_PASS" ssh "${SSH_OPTS[@]}" "$HOST" "$@"
}

remote_sudo() {
  remote "printf '%s\n' $(printf %q "$BOARD_PASS") | sudo -S -p '' $*"
}

remote_copy() {
  command -v sshpass >/dev/null || die "install sshpass: brew install hudochenkov/sshpass/sshpass"
  sshpass -p "$BOARD_PASS" scp "${SSH_OPTS[@]}" "$1" "$HOST:$2"
}

verify_sum() {
  if command -v sha256sum >/dev/null; then
    (cd "$CACHE_DIR" && sha256sum -c "$(basename "$SUM")")
  else
    (cd "$CACHE_DIR" && shasum -a 256 -c "$(basename "$SUM")")
  fi
}

fetch_release() {
  mkdir -p "$CACHE_DIR"
  if [[ -f "$TARBALL" && -f "$SUM" ]] && verify_sum; then
    echo "using $TARBALL"
    return
  fi
  echo "downloading $URL"
  curl -fL --retry 3 -o "$TARBALL" "$URL"
  curl -fL --retry 3 -o "$SUM" "$SUM_URL"
  verify_sum || die "checksum failed for $TARBALL"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h | --help) usage; exit 0 ;;
    --host) HOST="${2:?}"; shift 2 ;;
    *) usage >&2; die "unknown argument: $1" ;;
  esac
done

[[ -n "$HOST" ]] || { usage >&2; die "--host is required"; }
SEB_USER="${SEB_USER:-${HOST%@*}}"

fetch_release

echo "copy $TARBALL -> $HOST:/tmp/"
remote_copy "$TARBALL" "/tmp/seb-gpu-orangepi-rv.tar.gz"
remote "rm -rf /tmp/seb-gpu-orangepi-rv && tar xzf /tmp/seb-gpu-orangepi-rv.tar.gz -C /tmp"
remote_sudo "bash /tmp/seb-gpu-orangepi-rv/install.sh"
remote "rm -rf /tmp/seb-gpu-orangepi-rv /tmp/seb-gpu-orangepi-rv.tar.gz"

echo "install launchers"
remote_copy "$ROOT/platform/orangepi-gpu/scripts/seb-launch.sh" "/tmp/seb-launch.sh"
remote_copy "$ROOT/platform/orangepi-gpu/scripts/seb-remote.sh" "/tmp/seb-remote.sh"
remote_sudo "install -m 755 /tmp/seb-launch.sh /usr/local/bin/seb-launch.sh"
remote_sudo "install -m 755 /tmp/seb-remote.sh /usr/local/bin/seb-remote.sh"
remote "rm -f /tmp/seb-launch.sh /tmp/seb-remote.sh"

echo "install sudoers"
rule="$(mktemp)"
trap 'rm -f "$rule"' EXIT
echo "$SEB_USER ALL=(root) NOPASSWD: /usr/local/bin/seb-remote.sh" > "$rule"
remote_copy "$rule" "/tmp/seb-remote.sudoers"
remote_sudo "visudo -cf /tmp/seb-remote.sudoers"
remote_sudo "install -m 440 /tmp/seb-remote.sudoers /etc/sudoers.d/seb-remote"
remote_sudo "rm -f /etc/sudoers.d/seb-kiosk /tmp/seb-remote.sudoers"
remote_sudo "visudo -c"

remote_sudo "if [ -x /home/$SEB_USER/safe-exam-browser ]; then install -m 755 /home/$SEB_USER/safe-exam-browser /usr/local/bin/safe-exam-browser; else echo 'no ~/safe-exam-browser yet'; fi"

echo
echo "GPU stack installed."
echo "SEB must be seb-linux main at f6decb6. Then:"
echo "  sudo seb-remote.sh 'seb://<host>:8000/<file>.seb'"
