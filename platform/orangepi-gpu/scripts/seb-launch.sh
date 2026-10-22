#!/bin/bash
URL="${1:-}"
if [ -z "$URL" ]; then
  echo "usage: seb-launch.sh <url>" >&2
  exit 1
fi
SEB_BIN="${SEB_BIN:-/usr/local/bin/safe-exam-browser}"
if [ ! -x "$SEB_BIN" ]; then
  echo "SEB binary is not executable: $SEB_BIN" >&2
  exit 1
fi
export CAGE_MODE=1280x720
export SEB_IN_CAGE=1
export MESA_LOADER_DRIVER_OVERRIDE=pvr
export GDK_GL=gles
export GDK_BACKEND=wayland
export QT_QPA_PLATFORM=wayland
unset WEBKIT_DISABLE_COMPOSITING_MODE
exec cage -- "$SEB_BIN" --anti-cheat "$URL"
