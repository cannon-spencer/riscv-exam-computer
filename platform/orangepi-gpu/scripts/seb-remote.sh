#!/bin/bash
# usage: sudo ~/seb-remote.sh '<seb:// or sebs:// url>'
URL="$1"
[ -n "$URL" ] || { echo "usage: sudo $0 <url>"; exit 1; }
[ "$(id -u)" = 0 ] || { echo "run with sudo"; exit 1; }
systemctl stop seb-kiosk 2>/dev/null
pkill -f safe-exam-browser 2>/dev/null; pkill -x cage 2>/dev/null; sleep 1
systemctl reset-failed seb-kiosk 2>/dev/null
chvt 7
systemd-run --unit=seb-kiosk --collect \
  -p User=orangepi -p PAMName=login \
  -p TTYPath=/dev/tty7 -p TTYReset=yes -p TTYVHangup=yes \
  -p StandardInput=tty -p StandardOutput=journal -p StandardError=journal \
  /home/orangepi/seb-launch.sh "$URL"
echo "SEB starting on tty7"
