# SEB GPU stack on Orange Pi RV

Gets SEB rendering on the PowerVR BXE-4-32, inside a cage kiosk at 1280x720.

This does not make exams fast. The JH7110 main thread is still the limit. What this folder does is put compositing on the GPU instead of the CPU.

## Only works on

- Orange Pi RV (StarFive JH7110), vendor Debian image
- debian-ports snapshot `20221225T084846Z`, kernel `5.15.0-starfive2`
- IMG GPU userspace `img-gpu-powervr-bin-1.17.6210866` (starfive-tech/soft_3rdpart) installed in `/usr/lib`
- `MESA_LOADER_DRIVER_OVERRIDE=pvr` set in `/etc/environment` (vendor default)
- Do not run `apt upgrade` on this snapshot.

## Install the release and run SEB

1. Download `seb-gpu-orangepi-rv.tar.gz` and the `.sha256` from the `gpu-stack-v1` release.
2. `sha256sum -c seb-gpu-orangepi-rv.tar.gz.sha256`
3. `tar xzf seb-gpu-orangepi-rv.tar.gz && cd seb-gpu-orangepi-rv && ./install.sh`
4. Build SEB from [seb-linux#1](https://github.com/cannon-spencer/seb-linux/pull/1) (`17d0f2b`, branch `orangepi-cage-env`) with `CONFIG+=force_webkitgtk`.
5. `sudo install -m 755 build/bin/safe-exam-browser /usr/local/bin/safe-exam-browser`
6. From this folder, `system/setup-system.sh`. That installs `seb-launch.sh` and `seb-remote.sh` to `/usr/local/bin` and allows `orangepi` to run `seb-remote.sh` as root. The 8 GB swap it creates is only for a from-source WebKit build.
7. `sudo seb-remote.sh 'seb://<host>:8000/<file>.seb'`

Stop: `sudo systemctl stop seb-kiosk; sudo chvt 2`

`seb-launch.sh` sets `CAGE_MODE=1280x720`, `SEB_IN_CAGE=1`, `MESA_LOADER_DRIVER_OVERRIDE=pvr`, `GDK_GL=gles`, `GDK_BACKEND=wayland`, and `QT_QPA_PLATFORM=wayland`. Override the binary with `SEB_BIN` and the SSH user with `SEB_USER`.

## Rebuild the stack

Run these only to cut a new release tarball. A lab board installs the release above.

1. `system/setup-system.sh` for the swap file.
2. `build/build-webkit-gles.sh` (about 11.5 h, run it in tmux), then `build/install-webkit-gles.sh`.
3. `build/build-wlroots-cage.sh`.
4. `bundle/make-bundle.sh`.

## Patches

- wlroots 0.15.1: skip the `GL_EXT_unpack_subimage` check on GLES 3+. The driver omits the extension string. GLES 3 has the entry points.
- cage 0.1.4: `CAGE_MODE=WxH` picks the display mode. An unknown mode falls back to the monitor's preferred mode.

The SEB side of this (`SEB_IN_CAGE`, and forwarding `GDK_GL` / `MESA_LOADER_DRIVER_OVERRIDE` through pkexec) lives in seb-linux, not in a patch here.
