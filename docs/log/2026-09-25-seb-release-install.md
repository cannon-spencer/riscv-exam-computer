# SEB binary: build once on a board, release, install like the agent

## 9/25/2026

Same idea as `flash-os.sh` / the reptilian `orangepi-rv` release. We do **not** compile SEB on every lab machine. One Orange Pi builds a riscv64 ELF, we put it on a GitHub Release, `install-seb.sh` downloads it and scps it.

### One-time build (on the Pi)

```bash
git clone https://github.com/cannon-spencer/seb-linux.git
cd seb-linux
./scripts/build-on-device.sh
```

Writes `~/safe-exam-browser`. Test from a **desktop** terminal (SSH has no display):

```bash
~/safe-exam-browser
```



### Upload to the release

First time (no `riscv64` release yet):

```bash
scp orangepi@<ip>:~/safe-exam-browser .
gh release create riscv64 ./safe-exam-browser \
  -R cannon-spencer/seb-linux \
  --title "riscv64 WebKitGTK" \
  --notes "On-device Orange Pi build"
```

Replace it after a rebuild:

```bash
scp orangepi@<ip>:~/safe-exam-browser .
gh release upload riscv64 ./safe-exam-browser --clobber \
  -R cannon-spencer/seb-linux
```

If `cache/safe-exam-browser` already exists, delete it so `install-seb.sh` fetches the new file.

### Install on a board

Same shape as `install-software.sh`:

```bash
./scripts/install-seb.sh --host orangepi@<ip>
```

Downloads the release ELF (or reuses the cache), scps `~/safe-exam-browser`, apt-installs the Qt/WebKit runtime. No `make` on the target.

### What the binary sets itself

We bake Orange Pi / UF defaults into the ELF so you do not need `~/seb.sh` exports:

- `GDK_BACKEND=x11` and `QT_QPA_PLATFORM=xcb` — Qt and WebKitGTK both use X11
- `WEBKIT_DISABLE_COMPOSITING_MODE=1` — no GPU compositing (blank page on this PowerVR)
- start URL `https://elearning.ufl.edu/` (still overridable with `--url` or a `.seb`)

Those env vars are only set if the session did not already set them. Exam lockdown can still switch Qt to `linuxfb`. They get forwarded through `pkexec` when SEB re-execs as root.

### Caveats

The release ELF has to be the **on-device** build, not an x86 CI artifact. Run SEB on the desktop, not over SSH. The default URL is Canvas; a real exam config can still replace it later.