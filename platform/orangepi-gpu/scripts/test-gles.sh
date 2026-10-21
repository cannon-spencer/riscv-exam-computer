#!/bin/bash
LOG=/tmp/minibrowser.log
pkill -f MiniBrowser 2>/dev/null; sleep 1

echo "Launching MiniBrowser on Wayland..."
GDK_GL=gles WEBKIT_FORCE_COMPOSITING_MODE=1 GDK_BACKEND=wayland \
  /usr/lib/riscv64-linux-gnu/webkit2gtk-4.1/MiniBrowser https://example.com >"$LOG" 2>&1 &

echo "Waiting 20s for page load..."
sleep 20

pid=$(pgrep -nf webkit2gtk-4.1/WebKitWebProcess)
if [ -z "$pid" ]; then
  echo "FAIL: WebKitWebProcess not running"
  echo "--- log ---"; cat "$LOG"; exit 1
fi

echo "--- GL libraries loaded by WebKitWebProcess ($pid) ---"
LIBS=$(grep -oE '[^ ]*(pvr|GLESv2|EGL|libGL\.)[^ ]*' /proc/$pid/maps | sort -u)
echo "$LIBS"

echo "--- EGL errors in log ---"
grep -i -E 'egl|could not create' "$LOG" || echo "(none)"

echo "--- RESULT ---"
if echo "$LIBS" | grep -q pvr_dri && ! grep -qi 'could not create' "$LOG"; then
  echo "PASS: WebKit is rendering on the PowerVR GPU"
else
  echo "FAIL: GPU not in use (paste full output)"
fi
