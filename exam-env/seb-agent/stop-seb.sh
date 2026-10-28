#!/bin/sh
# Runs as root. Stops the SEB process start-seb.sh launched.
# GPU stack: SEB runs in cage under the seb-kiosk unit (seb-remote.sh).
if systemctl is-active --quiet seb-kiosk 2>/dev/null; then
  systemctl stop seb-kiosk || true
  exit 0
fi

pkill -f /home/orangepi/safe-exam-browser || true
