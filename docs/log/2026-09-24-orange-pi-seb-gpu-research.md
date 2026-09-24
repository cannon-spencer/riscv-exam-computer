# SEB on Orange Pi RV (GPU): Research/ troubleshooting the slow SEB canvas rendering 

2026-09-24

## GPU

- The GPU is a PowerVR BXE-4-32 (StarFive JH7110). `dmesg` shows BVNC 36.50.54.182, firmware and shader images loaded, and `Initialized pvr 1.17.6210866`.
- The kernel driver is `pvrsrvkm`, built into the kernel (`lsmod` shows nothing, and `card0` is bound to it).
- The display controller is separate: `card1`, driver `starfive`, HDMI at 1920x1080.
- Userspace PowerVR libraries in `/usr/lib` are the same version, 1.17.6210866.
- GNOME uses the GPU. `gnome-shell` has `libGLESv2_PVR_MESA`, `libsrv_um` and `/dev/dri/renderD128` mapped (`/proc/<pid>/maps`), and the `pvrsrvkm` interrupt counter read 25,822 (`/proc/interrupts`).
- On the stock image, `pvr_dri.so`, `swrast_dri.so` and `kms_swrast_dri.so` are the same 15.8 MB Mesa file (hard links). Mesa is 22.3.5-1.1, and `dpkg -V` reports no modified files.
- PowerVR kernel and userspace driver versions must match, and they do.

## Board

- Orange Pi RV
- Mesa is mixed: core packages are 22.3.1-1 (downgraded 9/22), others are 22.3.5-1.1. The original 22.3.5-1.1 can't be downloaded.
- (attempted a downgrade for potential improved compatibility)
- Mesa's software renderer (llvmpipe) is present.
- WebKitGTK is 2.38.2.
- `seb-linux` builds, and both binary copies are identical.

## Tested

- SEB under Cage: polkit works and the taskbar appears, but the content pane is blank. The power icon quits SEB, then Cage hangs.
- MiniBrowser under Cage: about 10+ seconds per keystroke, and the pointer is invisible (cursor format errors).
- Cage log: GLES2 init failed, and `Could not commit output` repeats.
- GNOME is now slow, with `gnome-shell` at about 100% of one core.
- Not the cause: memory (~1 GB free, no swap), storage (no I/O wait or SD errors), temperature (45°C idle), resolution (1080p).

## GPU support status

- Mesa's open-source PowerVR driver is Vulkan only and lists BXE-4-32 as unsupported and not under active development.
- A community bring-up on the VisionFive 2 (same GPU) has experimental Vulkan working (`domibel/visionfive2_gpu_bringup`). It needs kernel 7.2 plus two out-of-tree branches, loading the driver twice, and the `PVR_I_WANT_A_BROKEN_VULKAN_DRIVER` flag. Most glmark2 scenes under Weston hit a reset or hang, glxgears under XWayland didn't work, and unloading the driver can hang the board.
- A community VisionFive 2 Debian image (kernel 6.12) states that GPU hardware acceleration is not supported.
- A VisionFive 2 user on Imagination's forum reports X11 color and mouse-stutter problems, and OpenGL ES scrambling the screen with the vendor driver.
- Two Imagination forum threads (GNOME on RISC-V falling back to softpipe with the vendor driver) got the reply that it needs internal support, and the poster said Imagination support wasn't helpful. Both are about a GE8300 GPU, not this one specifically

## Research

- WebKitGTK 2.46+ renders with Skia. Igalia says multi-threaded CPU rendering is usually better on embedded SoCs with weak GPUs. 2.38 predates this.
- Debian trixie has a newer WebKitGTK (2.52) for riscv64. It needs glibc 2.38 or newer.
- Debian doesn't package QtWebEngine for riscv64.
