#!/usr/bin/env bash
# Build exam-env/server, run it on 127.0.0.1:8000, and attach the Cloudflare tunnel.
# Reads ../.env. Ctrl-C stops the tunnel and the container.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/exam-env"
IMAGE="ghcr.io/cannon-spencer/exam-server:latest"
PORT=8000
ENV_FILE="$ROOT/.env"
CRED_FILE=""
CFG_FILE=""

die() { echo "error: $*" >&2; exit 1; }

load_env() {
  [[ -f "$ENV_FILE" ]] || die "missing $ENV_FILE. Copy .env.example to .env and fill it in"
  set -a
  # shellcheck disable=SC1090
  source "$ENV_FILE"
  set +a
  : "${EXAM_PUBLIC_URL:=https://riscv-exam-computer.download}"
  EXAM_PUBLIC_URL="${EXAM_PUBLIC_URL%/}"
  [[ -n "${EXAM_ADMIN_PASSWORD:-}" ]] || die "EXAM_ADMIN_PASSWORD is empty in .env"
  [[ -n "${EXAM_AGENT_TOKEN:-}" ]] || die "EXAM_AGENT_TOKEN is empty in .env"
  [[ "$EXAM_PUBLIC_URL" == https://* ]] || die "EXAM_PUBLIC_URL must be an https URL"
}

public_host() {
  local host="${EXAM_PUBLIC_URL#*://}"
  printf '%s\n' "${host%%/*}"
}

cleanup() {
  trap - EXIT INT TERM
  docker stop exam-server >/dev/null 2>&1 || true
  [[ -n "$CRED_FILE" ]] && rm -f "$CRED_FILE"
  [[ -n "$CFG_FILE" ]] && rm -f "$CFG_FILE"
}

wait_ready() {
  local i
  for i in $(seq 1 30); do
    if curl -fsS --max-time 1 "http://127.0.0.1:${PORT}/healthz" >/dev/null; then
      return 0
    fi
    sleep 1
  done
  echo "server did not answer /healthz. recent logs:" >&2
  docker logs --tail 60 exam-server >&2 || true
  return 1
}

start_tunnel() {
  local host id
  host="$(public_host)"
  [[ -n "${CLOUDFLARE_TUNNEL_CREDENTIALS:-}" ]] || die "CLOUDFLARE_TUNNEL_CREDENTIALS is empty in .env"
  CRED_FILE="$(mktemp)"
  CFG_FILE="$(mktemp)"
  chmod 600 "$CRED_FILE" "$CFG_FILE"
  printf '%s\n' "$CLOUDFLARE_TUNNEL_CREDENTIALS" > "$CRED_FILE"
  id="$(tr -d '\n' < "$CRED_FILE" | sed -n 's/.*"TunnelID"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')"
  [[ -n "$id" ]] || die "CLOUDFLARE_TUNNEL_CREDENTIALS is not the tunnel JSON (expected TunnelID)"
  cat > "$CFG_FILE" <<EOF
tunnel: ${id}
credentials-file: ${CRED_FILE}
ingress:
  - hostname: ${host}
    service: http://127.0.0.1:${PORT}
  - service: http_status:404
EOF
  echo "starting tunnel ${id} for https://${host} (Ctrl-C to stop)"
  cloudflared tunnel --config "$CFG_FILE" --no-autoupdate run "$id"
}

load_env

docker info >/dev/null 2>&1 || die "docker is not running"
command -v cloudflared >/dev/null || die "cloudflared not on PATH"
command -v curl >/dev/null || die "curl not on PATH"

trap cleanup EXIT INT TERM

echo "building $IMAGE"
DOCKER_BUILDKIT=1 docker build -f "$CRATE/server/Dockerfile" -t "$IMAGE" "$CRATE"

mkdir -p "$CRATE/data"
docker rm -f exam-server >/dev/null 2>&1 || true
docker run -d --name exam-server --restart=no \
  -p "127.0.0.1:${PORT}:8000" \
  -e EXAM_ADMIN_PASSWORD \
  -e EXAM_AGENT_TOKEN \
  -e EXAM_PUBLIC_URL \
  -e EXAM_DATA_DIR=/data \
  -e EXAM_COOKIE_SECURE=always \
  -v "$CRATE/data:/data" \
  "$IMAGE"

echo "waiting for http://127.0.0.1:${PORT}/healthz"
wait_ready
echo "console: ${EXAM_PUBLIC_URL}"
start_tunnel
