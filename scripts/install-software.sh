#!/usr/bin/env bash
# Cross-compile seb-agent, scp it, and run it as a user systemd service.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/exam-env/seb-agent"
UNIT="$CRATE/seb-agent.service"
TARGET="riscv64gc-unknown-linux-gnu"
HOST=""
BOARD_PASS="${BOARD_PASS:-orangepi}"
SSH_OPTS=(-o StrictHostKeyChecking=accept-new -o ConnectTimeout=10 -o ServerAliveInterval=30)

usage() {
  echo "Usage: $(basename "$0") --host <user@ip>"
  echo "  --host   SSH target, e.g. orangepi@10.0.0.40"
  echo
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

userctl() {
  remote "export XDG_RUNTIME_DIR=/run/user/\$(id -u)
    $*"
}

cross_compile() {
  command -v rustup >/dev/null || die "install rust"
  command -v cargo >/dev/null || die "cargo not on PATH"
  command -v zig >/dev/null || die "install zig"
  cargo zigbuild --help >/dev/null 2>&1 \
    || die "install cargo-zigbuild: cargo install cargo-zigbuild"

  rustup target add "$TARGET"
  echo "building seb-agent ($TARGET)"
  cargo zigbuild --release --target "$TARGET" --manifest-path "$CRATE/Cargo.toml"
}

BIN="$CRATE/target/$TARGET/release/seb-agent"

stop_agent() {
  userctl "systemctl --user stop seb-agent.service" 2>/dev/null || true
}

copy_agent() {
  [[ -f "$BIN" ]] || die "missing $BIN"
  echo "copy $BIN -> $HOST:~/seb-agent"
  remote_copy "$BIN" "~/seb-agent"
  remote "chmod +x ~/seb-agent"
}

install_agent_service() {
  [[ -f "$UNIT" ]] || die "missing $UNIT"
  echo "install systemd user unit"
  remote "mkdir -p ~/.config/systemd/user"
  remote_copy "$UNIT" "~/.config/systemd/user/seb-agent.service"
  remote_sudo "loginctl enable-linger \$USER"
  remote_sudo "systemctl start user@\$(id -u).service"
  userctl "systemctl --user daemon-reload
    systemctl --user enable --now seb-agent.service
    systemctl --user --no-pager --full status seb-agent.service || true"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h | --help) usage; exit 0 ;;
    --host) HOST="${2:?}"; shift 2 ;;
    *) usage >&2; die "unknown argument: $1" ;;
  esac
done

[[ -n "$HOST" ]] || { usage >&2; die "--host is required"; }

cross_compile
stop_agent
copy_agent
install_agent_service

echo
echo "agent:  systemctl --user status seb-agent"
echo "SEB:    ./scripts/install-seb.sh --host $HOST"
