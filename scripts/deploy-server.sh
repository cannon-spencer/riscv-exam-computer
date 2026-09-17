#!/usr/bin/env bash
# Build exam-env/server here and run the image on the Oracle VM over SSH.
# On the VM: sudo dnf install -y podman podman-docker

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/exam-env/server"
ENV_FILE="$ROOT/.env"

HOST_ARG=""
IMAGE="ghcr.io/cannon-spencer/exam-server:latest"
PORT=8000
SSH_OPTS=(-o StrictHostKeyChecking=accept-new -o ConnectTimeout=15)

usage() {
  echo "Usage: $(basename "$0") [--host user@ip]"
  echo "  --host   SSH target, e.g. opc@129.213.203.27 (or SERVER_HOST)"
}

die() { echo "error: $*" >&2; exit 1; }

remote() {
  ssh "${SSH_OPTS[@]}" "$HOST" "$@"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h | --help) usage; exit 0 ;;
    --host) HOST_ARG="${2:?}"; shift 2 ;;
    *) usage >&2; die "unknown argument: $1" ;;
  esac
done

if [[ -f "$ENV_FILE" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "$ENV_FILE"
  set +a
fi

HOST="${HOST_ARG:-${SERVER_HOST:-}}"
[[ -n "$HOST" ]] || { usage >&2; die "--host or SERVER_HOST is required"; }
docker info >/dev/null 2>&1 || die "docker daemon not running; try: colima start"

arch="$(remote uname -m)"
case "$arch" in
  x86_64) platform=linux/amd64 ;;
  aarch64 | arm64) platform=linux/arm64 ;;
  *) die "unsupported VM arch: $arch" ;;
esac

echo "building $IMAGE ($platform)"
DOCKER_BUILDKIT=1 docker build --platform "$platform" -t "$IMAGE" "$CRATE"

echo "copying image to $HOST"
docker save "$IMAGE" | gzip -c | remote "gzip -dc | sudo docker load"

remote "sudo docker rm -f exam-server" >/dev/null 2>&1 || true
remote "sudo docker run -d --name exam-server --restart=always --network host $IMAGE"

PUBLIC_IP="${HOST#*@}"
echo "listening at http://${PUBLIC_IP}:${PORT}/heartbeat"
