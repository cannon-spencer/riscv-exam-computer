# SEB GPU acceleration on Orange Pi RV

Gets SEB rendering on the PowerVR BXE-4-32 GPU, inside a cage kiosk at 1280x720.

## Only works on
- Orange Pi RV (StarFive JH7110), vendor Debian image
- debian-ports snapshot 20221225T084846Z, kernel 5.15.0-starfive2
- IMG GPU userspace `img-gpu-powervr-bin-1.17.6210866` (starfive-tech/soft_3rdpart) installed in /usr/lib
- `MESA_LOADER_DRIVER_OVERRIDE=pvr` set in /etc/environment (vendor default)
- Do NOT run `apt upgrade` on this snapshot.

## Path A: prebuilt (minutes)
1. Download `seb-gpu-orangepi-rv.tar.gz` + `.sha256` from this repo's Releases.
2. `sha256sum -c seb-gpu-orangepi-rv.tar.gz.sha256`
3. `tar xzf seb-gpu-orangepi-rv.tar.gz && cd seb-gpu-orangepi-rv && ./install.sh`
4. Apply SEB patch + rebuild SEB (below), then `system/setup-system.sh`, then copy `scripts/` to `~`.

## Path B: build from source (~12 h)
1. `system/setup-system.sh` (swap + sudo rules)
2. `build/build-webkit-gles.sh` (run in tmux, ~11.5 h), then `build/install-webkit-gles.sh`
3. `build/build-wlroots-cage.sh`
4. Apply SEB patch + rebuild SEB (below), copy `scripts/` to `~`.

## SEB
Check `patches/seb-linux-base.txt` for the commit. Then:
`cd ~/seb-linux && git apply <repo>/platform/orangepi-gpu/patches/seb-linux-gpu.patch && ./scripts/build.sh CONFIG+=force_webkitgtk`

## Run
- Local / UART / daemon: `sudo ~/seb-remote.sh 'seb://<host>:8000/<file>.seb'`
- Stop: `sudo systemctl stop seb-kiosk; sudo chvt 2`
- Check GPU: `~/seb-status.sh` (looks for `pvr_dri.so` in SEB's WebKit process)

## Required env (set by seb-launch.sh)
`GDK_GL=gles MESA_LOADER_DRIVER_OVERRIDE=pvr GDK_BACKEND=wayland QT_QPA_PLATFORM=wayland SEB_IN_CAGE=1 CAGE_MODE=1280x720`

## Patches
- wlroots: skip `GL_EXT_unpack_subimage` check on GLES 3+ (driver omits the extension; GLES 3.2 has it built in)
- cage: `CAGE_MODE=WxH` picks the display mode (stock cage always uses the monitor's preferred mode)
- seb-linux: `SEB_IN_CAGE` skips the VT switch + linuxfb; pkexec forwards `GDK_GL`, `MESA_LOADER_DRIVER_OVERRIDE`, `QT_XCB_GL_INTEGRATION`
