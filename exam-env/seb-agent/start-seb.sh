#!/bin/sh
# Runs as root via sudo. Starts SEB on the exam file seb-agent wrote from the
# start job. Stays in the foreground while SEB runs: seb-agent reports
# "running" for as long as this script is alive.
USER_NAME=orangepi
USER_UID=$(id -u "$USER_NAME")
RUNTIME=/run/user/$USER_UID
EXAM=/home/$USER_NAME/exam.seb
SEB=/home/$USER_NAME/safe-exam-browser

# GPU stack (scripts/install-gpu.sh): cage kiosk on tty7 as a systemd unit.
if [ -x /usr/local/bin/seb-remote.sh ]; then
  /usr/local/bin/seb-remote.sh "$EXAM" || exit 1
  sleep 2
  while systemctl is-active --quiet seb-kiosk; do
    sleep 2
  done
  exit 0
fi

# GNOME desktop image: open SEB in the logged-in orangepi session.
# Prefer XWayland. With the WebKitGTK engine the exam page is its own
# fullscreen keep-above window, and native Wayland ignores keep-above, so
# SEB's Qt window ends up covering the page. Mutter honors it for X clients.
XAUTH=$(ls "$RUNTIME"/.mutter-Xwaylandauth.* 2>/dev/null | head -n 1)
if [ -S "$RUNTIME/wayland-0" ] && [ -S /tmp/.X11-unix/X0 ] && [ -n "$XAUTH" ]; then
  exec runuser -u "$USER_NAME" -- env \
    XDG_RUNTIME_DIR="$RUNTIME" \
    DISPLAY=:0 \
    XAUTHORITY="$XAUTH" \
    DBUS_SESSION_BUS_ADDRESS="unix:path=$RUNTIME/bus" \
    GDK_BACKEND=x11 \
    QT_QPA_PLATFORM=xcb \
    "$SEB" --windowed "$EXAM"
fi

if [ -S "$RUNTIME/wayland-0" ]; then
  exec runuser -u "$USER_NAME" -- env \
    XDG_RUNTIME_DIR="$RUNTIME" \
    WAYLAND_DISPLAY=wayland-0 \
    DBUS_SESSION_BUS_ADDRESS="unix:path=$RUNTIME/bus" \
    GDK_BACKEND=wayland \
    QT_QPA_PLATFORM=wayland \
    "$SEB" --windowed "$EXAM"
fi

if [ -S /tmp/.X11-unix/X0 ]; then
  exec runuser -u "$USER_NAME" -- env \
    XDG_RUNTIME_DIR="$RUNTIME" \
    DISPLAY=:0 \
    DBUS_SESSION_BUS_ADDRESS="unix:path=$RUNTIME/bus" \
    GDK_BACKEND=x11 \
    QT_QPA_PLATFORM=xcb \
    "$SEB" --windowed "$EXAM"
fi

echo "no graphical session for $USER_NAME" >&2
exit 1
