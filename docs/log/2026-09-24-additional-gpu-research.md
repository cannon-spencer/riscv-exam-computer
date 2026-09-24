# SEB on Orange Pi RV - notes

## Status
SEB runs on the vendor Debian image (GNOME/Wayland, kernel 5.15) but is slow. Set up a fresh 64GB microSD with `setup.sh` (deps, SEB build with George's view + pkexec patches, launcher) and `cpu-opt.sh`. WebKit rebuild is compiling on that card now, will test when done.

## Why it's slow
- WebKitWebProcess is CPU bound. Network (~75 Mbps) and RAM (~500MB used) are fine.
- The GPU (PowerVR BXE-4-32) works, GNOME uses it (glmark2-es2-wayland: 735). Driver only does OpenGL ES though.
- Debian's WebKit 2.38 asks for desktop OpenGL, gets refused, falls back to CPU.
- X11 apps get no GPU. Vendor sets `XWAYLAND_NO_GLAMOR=1` because glamor crashes Xwayland on real apps (tested, it does). SEB runs under X11 right now so it's fully software.

## Tried, didn't work
- llvmpipe: not in the vendor Mesa
- Zink: Vulkan driver missing features Zink needs
- SEB on Wayland with stock WebKit: EGL fails
- Xfce instead of GNOME: white screen

## CPU tweaks
- `WEBKIT_DISABLE_COMPOSITING_MODE=1` (web process ~100% -> 30-70%)
- performance governor at boot
- 720p
- tracker, packagekit, cups, bluetooth etc. off

## Fix in progress
Rebuild WebKitGTK 2.38.2 with `-DUSE_OPENGL_ES=ON` and run SEB on native Wayland so page rendering uses the GPU. George's view already opens WebKit in its own window, so SEB itself shouldn't need changes. Build takes ~1-2 days on the board.