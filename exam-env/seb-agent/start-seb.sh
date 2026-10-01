#!/bin/sh
# Runs as root via sudo.
exec /usr/bin/xinit /bin/sh -c '
  openbox &
  exec /usr/bin/env \
    XDG_RUNTIME_DIR=/run/user/0 \
    WEBKIT_DISABLE_COMPOSITING_MODE=1 \
    GDK_BACKEND=x11 \
    QT_QPA_PLATFORM=xcb \
    /home/orangepi/safe-exam-browser --windowed /home/orangepi/demo-exam.json
' -- :1 vt3
