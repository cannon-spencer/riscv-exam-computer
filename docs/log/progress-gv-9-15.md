# progress gv 9/15

- Confirmed Canvas already worked end to end from last session
- Researched GPU driver status for JH7110 BXE-4-32. Mainline drm/imagination driver does not support this GPU yet
- Researched StarFive official Debian image as a possible driver source, but held off since GPU cause was not confirmed
- Decided to diagnose the actual issue first instead of assuming it was the GPU
- Checked /dev/dri, dmesg, lsmod on the board. Found PowerVR kernel driver already loads fine at boot
- Checked dri folder, found pvr_dri.so already present alongside swrast_dri.so
- Ran SEB with debug logging directly on the board screen, not SSH, since rendering needs an actual display
- Ran SEB without WEBKIT_DISABLE_COMPOSITING_MODE=1. It did not crash this time, but was still slow
- Checked debug log, found libGL was loading swrast_dri.so, software rendering, not pvr
- Checked env vars, found MESA_LOADER_DRIVER_OVERRIDE=pvr was set locally but not reaching the render process
- Checked main.cpp, found the pkexec env var whitelist was missing MESA_LOADER_DRIVER_OVERRIDE
- Patched main.cpp with sed to add MESA_LOADER_DRIVER_OVERRIDE to both pkexec whitelist call sites
- Rebuilt seb-linux with qmake6 and make
- Ran again with MESA_LOADER_DRIVER_OVERRIDE=pvr set. Still felt slow
- Currently checking new debug log to confirm whether pvr driver actually loaded this time. Not resolved yet
