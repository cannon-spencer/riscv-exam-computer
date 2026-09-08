# riscv-exam-computer

A plug-and-play secure exam computer on a RISC-V SBC. It boots a locked-down Linux environment and a Safe Exam Browser kiosk for Canvas exams.

This repo is the exam/kiosk glue. Board OS images are built in [cannon-spencer/reptilian-riscv](https://github.com/cannon-spencer/reptilian-riscv).

## Usage

SD card in this machine (`diskutil list`; never `disk0`):

```bash
./scripts/flash-os.sh --device /dev/rdiskN
```

Board booted and on Wi‑Fi:

```bash
./scripts/install-software.sh --host orangepi@10.0.0.xx \
  --control-url http://10.0.0.yy:8000
```

## How the OS is built

`reptilian-riscv` holds the Orange Pi kernel, U-Boot, and image builder. That repo rarely changes.

On push to `main`, its CI runs `scripts/ci-build.sh` on Ubuntu 22.04 and publishes both files on the `orangepi-rv` GitHub Release:

- `visionfive2_fw_payload.img`
- `os.img.xz`

This repo’s flash script downloads:

```
https://github.com/cannon-spencer/reptilian-riscv/releases/download/orangepi-rv/os.img.xz
```

## Layout

- `platform/reptilian-riscv/` — board sources (submodule)
- `platform/seb-linux/` — Safe Exam Browser (submodule)
- `exam-env/` — board agent, server API, admin UI
- `scripts/flash-os.sh` — write the SD image
- `scripts/install-software.sh` — cross-compile `seb-agent` and scp to a booted board
- `docs/` — course LaTeX

## Clone

```bash
git clone --recurse-submodules https://github.com/cannon-spencer/riscv-exam-computer.git
```
