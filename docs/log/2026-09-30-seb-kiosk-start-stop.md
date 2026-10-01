# SEB kiosk session + instructor start/stop

## 9/30/2026

Board is a lightweight image with no desktop. SEB is an `xinit` session on vt3 plus a tiny WM, ideally less RAM and CPU than a desktop. Pointer motion is fine. Clicking a Canvas control takes on the order of 10+ seconds and the page paints in triangular tiles. A full desktop would not fix that: WebKit 2.38 is CPU-painting, and this PowerVR has no desktop-GL path for it.

Builds on the tunnel in [2026-09-23](2026-09-23-cloudflare-tunnel.md) and the release ELF in [2026-09-25](2026-09-25-seb-release-install.md).

### What works

On the board, Canvas comes up with:

```bash
sudo xinit /bin/sh -c '
  openbox &
  exec /usr/bin/env \
    XDG_RUNTIME_DIR=/run/user/0 \
    WEBKIT_DISABLE_COMPOSITING_MODE=1 \
    GDK_BACKEND=x11 \
    QT_QPA_PLATFORM=xcb \
    '"$HOME"'/safe-exam-browser --windowed '"$HOME"'/demo-exam.json
' -- :1 vt3
```

`--windowed` plus `demo-exam.json` skips the admin-password prompt.

Instructor side, with `deploy-server.sh` up:

```bash
curl -sS https://riscv-exam-computer.download/machines
curl -sS -X POST https://riscv-exam-computer.download/start
curl -sS -X POST https://riscv-exam-computer.download/stop
```

`/start` returned `{"ok":true,"queued":["orangepirv"]}`. Agent journal: `started /home/orangepi/start-seb.sh`, then `state=running`. `/stop` tears down X/SEB and puts a text getty back on vt1.

### New board packages

On top of `install-seb.sh` (Qt6 + `libwebkit2gtk-4.1-0`):

```bash
sudo apt-get install -y openbox xorg xinit
```

Need `~/demo-exam.json` (start URL Canvas) and the release `~/safe-exam-browser`.

### Agent / server

Heartbeat is still `POST https://riscv-exam-computer.download/heartbeat`. The server now keeps an in-memory inventory and a one-job queue:


| Route             | Role                                                                      |
| ----------------- | ------------------------------------------------------------------------- |
| `POST /heartbeat` | record host + state; if a job is pending, return `{"ok":true,"job":"start |
| `GET /machines`   | hosts seen in the last 15s                                                |
| `POST /start`     | queue `start` on every online board                                       |
| `POST /stop`      | queue `stop` on every online board                                        |


`seb-agent` execs `sudo -n ~/start-seb.sh` / `~/stop-seb.sh` (not the ELF). `install-software.sh` copies those scripts and `/etc/sudoers.d/seb-start` (`NOPASSWD` for those two paths only). The user unit no longer sets `DISPLAY=:0`.

### Caveats

Canvas on this chip is not exam-usable yet. Mouse is responsive; clicks are not. A full desktop does not help the page.

`demo-exam.json` is a local demo config. Later the server will push a `.seb` and this file goes away.