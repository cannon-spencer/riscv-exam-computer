#!/bin/sh
# Runs as root so a mounted data directory can be made writable, then drops
# to the server user. If the mount rejects chown (some Docker Desktop shares),
# keep running as root so the console still starts.
set -eu

data="${EXAM_DATA_DIR:-/data}"
mkdir -p "$data"

if chown server:server "$data" 2>/dev/null && su -s /bin/sh server -c "test -w '$data'"; then
  exec runuser -u server -- /usr/local/bin/server
fi

exec /usr/local/bin/server
