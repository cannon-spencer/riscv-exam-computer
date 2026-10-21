#!/bin/bash
export GDK_BACKEND=x11
export QT_QPA_PLATFORM=xcb
export WEBKIT_DISABLE_COMPOSITING_MODE=1
exec ~/seb-linux/build/bin/safe-exam-browser --url https://elearning.ufl.edu/
