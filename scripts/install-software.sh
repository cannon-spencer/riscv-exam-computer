#!/usr/bin/env bash
# Cross-compile seb-agent for the Orange Pi (riscv64 Linux) and scp it over.
# SEB install is still TODO.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/exam-env/seb-agent"
TARGET="riscv64gc-unknown-linux-gnu"
HOST=""
CONTROL_URL_ARG=""
BOARD_PASS="${BOARD_PASS:-orangepi}"
SSH_OPTS=(-o StrictHostKeyChecking=accept-new -o ConnectTimeout=10)

usage() {
  echo "Usage: $(basename "$0") --host <user@ip> [--control-url <url>]"
  echo "  --host          SSH target, e.g. orangepi@10.0.0.40"
  echo "  --control-url   baked into ~/seb-agent.env (or export CONTROL_URL)"
  echo
  echo "BOARD_PASS defaults to orangepi (image default). Export to override."
}

die() { echo "error: $*" >&2; exit 1; }

remote() {
  command -v sshpass >/dev/null || die "install sshpass: brew install hudochenkov/sshpass/sshpass"
  sshpass -p "$BOARD_PASS" ssh "${SSH_OPTS[@]}" "$HOST" "$@"
}

remote_copy() {
  command -v sshpass >/dev/null || die "install sshpass: brew install hudochenkov/sshpass/sshpass"
  sshpass -p "$BOARD_PASS" scp "${SSH_OPTS[@]}" "$1" "$HOST:$2"
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

copy_agent() {
  [[ -f "$BIN" ]] || die "missing $BIN"
  echo "copy $BIN -> $HOST:~/seb-agent"
  remote_copy "$BIN" "~/seb-agent"
  remote "chmod +x ~/seb-agent"
}

write_control_url() {
  remote "printf '%s\n' 'CONTROL_URL=$CONTROL_URL' > ~/seb-agent.env"
  echo "wrote $HOST:~/seb-agent.env"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h | --help) usage; exit 0 ;;
    --host) HOST="${2:?}"; shift 2 ;;
    --control-url) CONTROL_URL_ARG="${2:?}"; shift 2 ;;
    *) usage >&2; die "unknown argument: $1" ;;
  esac
done

if [[ -n "$CONTROL_URL_ARG" ]]; then
  CONTROL_URL="$CONTROL_URL_ARG"
else
  : "${CONTROL_URL:?set CONTROL_URL or pass --control-url}"
fi

[[ -n "$HOST" ]] || { usage >&2; die "--host is required"; }

cross_compile
copy_agent
write_control_url

echo
echo "on the board:"
echo "  set -a && source ~/seb-agent.env && set +a && ~/seb-agent"
