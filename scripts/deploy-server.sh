#!/usr/bin/env bash
# Build exam-env/server, run it on localhost:8000, and attach the Cloudflare tunnel.
# Requires Docker already running. Ctrl-C stops the tunnel and the container.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/exam-env/server"
IMAGE="ghcr.io/cannon-spencer/exam-server:latest"
PORT=8000
TUNNEL="${TUNNEL:-exam-server}"

die() { echo "error: $*" >&2; exit 1; }

cleanup() {
  trap - EXIT INT TERM
  docker stop exam-server >/dev/null 2>&1 || true
}

trap cleanup EXIT INT TERM

docker info >/dev/null 2>&1 || die "docker is not running"
command -v cloudflared >/dev/null || die "cloudflared not on PATH"

echo "building $IMAGE"
DOCKER_BUILDKIT=1 docker build -t "$IMAGE" "$CRATE"

docker rm -f exam-server >/dev/null 2>&1 || true
docker run -d --name exam-server --restart=no -p "${PORT}:8000" "$IMAGE"

echo "listening on http://127.0.0.1:${PORT}/heartbeat"
echo "starting tunnel $TUNNEL (Ctrl-C to stop)"
cloudflared tunnel run "$TUNNEL"
