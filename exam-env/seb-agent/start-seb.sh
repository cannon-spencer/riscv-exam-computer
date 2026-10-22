#!/bin/sh
# Runs as root via sudo. Starts SEB on ~/exam.seb, which seb-agent wrote from
# the start job. Stays in the foreground while SEB runs: seb-agent reports
# "running" for as long as this script is alive.
EXAM=/home/orangepi/exam.seb

# GPU stack (scripts/install-gpu.sh): cage kiosk on tty7 as a systemd unit.
if [ -x /usr/local/bin/seb-remote.sh ]; then
  /usr/local/bin/seb-remote.sh "$EXAM" || exit 1
  sleep 2
  while systemctl is-active --quiet seb-kiosk; do
    sleep 2
  done
  exit 0
fi

# CPU fallback: X + openbox on vt3.
exec /usr/bin/xinit /bin/sh -c '
  openbox &
  exec /usr/bin/env \
    XDG_RUNTIME_DIR=/run/user/0 \
    WEBKIT_DISABLE_COMPOSITING_MODE=1 \
    GDK_BACKEND=x11 \
    QT_QPA_PLATFORM=xcb \
    /home/orangepi/safe-exam-browser --windowed /home/orangepi/exam.seb
' -- :1 vt3
