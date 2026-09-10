# SEB on Orange Pi RV 

## Progress report 9/10/26
Successfully got SEB running on the orange pi rv (debian)

- **Bug 1 (`webkitgtk_view.cpp`):** the browser's backup rendering engine (WebKitGTK) was created but never shown on screen and never kept running 
- **Bug 2 (`main.cpp`):** SEB relaunches itself with admin rights (`pkexec`) for lockdown mode, but only forwarded a fixed list of settings to that relaunch
added the two we needed (`GDK_BACKEND`, `WEBKIT_DISABLE_COMPOSITING_MODE`) to that list.
- Result: SEB now boots, asks for admin permission, and renders a real webpage fullscreen on the board.

## How to boot the board
1. Power on with HDMI (monitor) + UART connected.
2. Manual U-Boot sequence:
   ```
   mmc dev 1
   load mmc 1:1 0x40200000 extlinux/extlinux.conf
   sysboot mmc 1:1 any 0x40200000
   ```
3. Board boots straight into a desktop (may require login)/`startx` needed.
4. Open a terminal **inside that desktop session**

## Exact command that loaded Canvas
```
GDK_BACKEND=x11 WEBKIT_DISABLE_COMPOSITING_MODE=1 \
  /home/orangepi/seb-linux/build/bin/safe-exam-browser --url https://elearning.ufl.edu/
```
- Click **Yes** on the "Administrator Permission Required" dialog.
- Enter the board's password at the prompt.
- Wait through the SEB splash screen — Canvas login page loads fullscreen.

## Known issue
- Rendering is slow, that's `WEBKIT_DISABLE_COMPOSITING_MODE=1` forcing CPU-only rendering 
(I will tackle the GPU driver issue later)
