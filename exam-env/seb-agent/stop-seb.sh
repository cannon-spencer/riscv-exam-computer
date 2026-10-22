#!/bin/sh
# Runs as root. start-seb.sh execs xinit under sudo; killing sudo leaves
# X/SEB on :1. Tear the session down, then put a text getty back on screen.
# GPU stack: SEB runs in cage under the seb-kiosk unit (seb-remote.sh).
if systemctl is-active --quiet seb-kiosk 2>/dev/null; then
  systemctl stop seb-kiosk || true
  chvt 2 2>/dev/null || true
  exit 0
fi

pkill -f /home/orangepi/safe-exam-browser || true
pkill -f '/usr/bin/xinit /bin/sh' || true
if [ -e /tmp/.X11-unix/X1 ]; then
  fuser -k /tmp/.X11-unix/X1 2>/dev/null || true
fi

sleep 1

# X/KMS unbinds the framebuffer console.
for bind in /sys/class/vtconsole/vtcon*/bind; do
  [ -w "$bind" ] && echo 1 > "$bind" 2>/dev/null || true
done

kbd_mode -a < /dev/tty1 2>/dev/null || true
chvt 1 2>/dev/null || true
deallocvt 3 2>/dev/null || true
