#!/bin/bash
URL="$1"
[ -n "$URL" ] || { echo "usage: seb-launch.sh <url>"; exit 1; }
export CAGE_MODE=1280x720 SEB_IN_CAGE=1 MESA_LOADER_DRIVER_OVERRIDE=pvr GDK_GL=gles GDK_BACKEND=wayland QT_QPA_PLATFORM=wayland
exec cage -- sudo -E /home/orangepi/seb-linux/build/bin/safe-exam-browser --anti-cheat "$URL"
