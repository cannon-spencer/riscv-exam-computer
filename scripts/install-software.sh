#!/usr/bin/env bash
# Cross-compile seb-agent, scp it, and run it as a user systemd service.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/exam-env/seb-agent"
UNIT="$CRATE/seb-agent.service"
TARGET="riscv64gc-unknown-linux-gnu"
# seb-agent is a workspace member, so cargo writes the binary in the
# workspace target dir, not under the crate.
BIN="$ROOT/exam-env/target/$TARGET/release/seb-agent"
HOST=""
BOARD_PASS="${BOARD_PASS:-orangepi}"
SSH_OPTS=(-o StrictHostKeyChecking=accept-new -o ConnectTimeout=10 -o ServerAliveInterval=30)

usage() {
  echo "Usage: $(basename "$0") --host <user@ip>"
  echo "  --host   SSH target, e.g. orangepi@10.0.0.40"
  echo
  echo "BOARD_PASS defaults to orangepi (image default). Export to override."
  echo "Reads EXAM_AGENT_TOKEN and EXAM_PUBLIC_URL from the repo .env so the"
  echo "board posts to the same server the console is using."
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

stop_agent() {
  userctl "systemctl --user stop seb-agent.service" 2>/dev/null || true
}

copy_agent() {
  [[ -f "$BIN" ]] || die "missing $BIN"
  echo "copy $BIN -> $HOST:~/seb-agent"
  remote_copy "$BIN" "~/seb-agent"
  remote "chmod +x ~/seb-agent"

  echo "copy start-seb.sh -> $HOST:~/start-seb.sh"
  remote_copy "$CRATE/start-seb.sh" "~/start-seb.sh"
  remote "chmod +x ~/start-seb.sh"

  echo "copy stop-seb.sh -> $HOST:~/stop-seb.sh"
  remote_copy "$CRATE/stop-seb.sh" "~/stop-seb.sh"
  remote "chmod +x ~/stop-seb.sh"

  echo "install sudoers for start/stop-seb.sh"
  remote_copy "$CRATE/seb-start.sudoers" "/tmp/seb-start.sudoers"
  remote_sudo "visudo -cf /tmp/seb-start.sudoers"
  remote_sudo "install -m 440 /tmp/seb-start.sudoers /etc/sudoers.d/seb-start"
}

load_env() {
  local env_file="$ROOT/.env"
  [[ -f "$env_file" ]] || die "missing $env_file. Copy .env.example to .env (same file deploy-server.sh uses)"
  set -a
  # shellcheck disable=SC1090
  source "$env_file"
  set +a
  [[ -n "${EXAM_AGENT_TOKEN:-}" ]] || die "EXAM_AGENT_TOKEN is empty in .env"
}

write_board_env() {
  local server tmp
  server="${EXAM_SERVER:-${EXAM_PUBLIC_URL:-https://riscv-exam-computer.download}}"
  server="${server%/}"
  tmp="$(mktemp)"
  chmod 600 "$tmp"
  printf 'EXAM_AGENT_TOKEN=%s\nEXAM_SERVER=%s\n' "$EXAM_AGENT_TOKEN" "$server" > "$tmp"
  echo "copy agent env -> $HOST:~/.config/seb-agent.env"
  remote "mkdir -p ~/.config"
  remote_copy "$tmp" "~/.config/seb-agent.env"
  remote "chmod 600 ~/.config/seb-agent.env"
  rm -f "$tmp"
}

install_agent_service() {
  [[ -f "$UNIT" ]] || die "missing $UNIT"
  echo "install systemd user unit"
  remote "mkdir -p ~/.config/systemd/user"
  write_board_env
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

load_env
cross_compile
stop_agent
copy_agent
install_agent_service

echo
echo "agent:  systemctl --user status seb-agent"
echo "SEB:    ./scripts/install-seb.sh --host $HOST"
