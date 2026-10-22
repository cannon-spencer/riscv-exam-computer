#!/bin/bash
# usage: sudo seb-remote.sh '<seb:// or sebs:// url>'
URL="${1:-}"
if [ -z "$URL" ]; then
  echo "usage: sudo $0 <url>" >&2
  exit 1
fi
if [ "$(id -u)" != 0 ]; then
  echo "run with sudo" >&2
  exit 1
fi
SEB_USER="${SEB_USER:-orangepi}"
LAUNCH="${SEB_LAUNCH:-/usr/local/bin/seb-launch.sh}"
if [ ! -x "$LAUNCH" ]; then
  echo "launcher is not executable: $LAUNCH" >&2
  exit 1
fi
systemctl stop seb-kiosk 2>/dev/null || true
pkill -f safe-exam-browser 2>/dev/null || true
pkill -x cage 2>/dev/null || true
sleep 1
systemctl reset-failed seb-kiosk 2>/dev/null || true
chvt 7
systemd-run --unit=seb-kiosk --collect \
  -p "User=$SEB_USER" -p PAMName=login \
  -p TTYPath=/dev/tty7 -p TTYReset=yes -p TTYVHangup=yes \
  -p StandardInput=tty -p StandardOutput=journal -p StandardError=journal \
  "$LAUNCH" "$URL"
echo "SEB starting on tty7"
