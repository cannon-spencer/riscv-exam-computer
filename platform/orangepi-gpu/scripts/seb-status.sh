#!/bin/bash
pid=$(pgrep -nf WebKitWebProcess)
echo "WebProcess pid=$pid"
if [ -n "$pid" ] && sudo grep -q pvr_dri /proc/$pid/maps; then
  echo "GPU: ON ✅"
else
  echo "GPU: OFF ❌"
fi
echo "=== Current display mode ==="
sudo sh -c 'grep -h "mode:" /sys/kernel/debug/dri/*/state 2>/dev/null' | sort -u
echo "=== cage mode setting ==="
grep CAGE_MODE ~/seb-kiosk.sh || echo "CAGE_MODE not set (patch not applied yet)"
