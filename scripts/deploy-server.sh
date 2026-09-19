#!/usr/bin/env bash
# Build exam-env/server and run it on localhost:8000.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/exam-env/server"
IMAGE="ghcr.io/cannon-spencer/exam-server:latest"
PORT=8000

die() { echo "error: $*" >&2; exit 1; }

docker info >/dev/null 2>&1 || die "docker daemon not running; try: colima start"

echo "building $IMAGE"
DOCKER_BUILDKIT=1 docker build -t "$IMAGE" "$CRATE"

docker rm -f exam-server >/dev/null 2>&1 || true
docker run -d --name exam-server --restart=unless-stopped -p "${PORT}:8000" "$IMAGE"

echo "listening on http://127.0.0.1:${PORT}/heartbeat"
