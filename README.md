# riscv-exam-computer

A plug-and-play secure exam computer on a RISC-V SBC. It boots a locked-down Linux environment and a Safe Exam Browser kiosk for Canvas exams.

This repo is the exam/kiosk glue. Board OS images are built in [cannon-spencer/reptilian-riscv](https://github.com/cannon-spencer/reptilian-riscv).

## Flash a new Orange Pi RV

1. Plug in an SD card. On macOS: `diskutil list` — use `/dev/rdiskN`, never `disk0`.
2. Flash (downloads the CI image if you do not have one cached):

```bash
./scripts/flash-os.sh --device /dev/rdisk4
```

3. Put the card in the board and power on. First boot resizes the rootfs, then logs in as `orangepi`.

A new board already has SPI firmware. You do not flash U-Boot for a first boot.

## How the OS is built

`reptilian-riscv` holds the Orange Pi kernel, U-Boot, and image builder. That repo rarely changes.

On push to `main`, its CI runs `scripts/ci-build.sh` on Ubuntu 22.04 and writes `build/`:

- `build/visionfive2_fw_payload.img` — committed (small)
- `build/os.img.xz` — GitHub Release `orangepi-rv` (too large for git)

This repo’s flash script downloads:

```
https://github.com/cannon-spencer/reptilian-riscv/releases/download/orangepi-rv/os.img.xz
```

## Layout

- `platform/reptilian-riscv/` — board sources (submodule)
- `platform/seb-linux/` — Safe Exam Browser (submodule)
- `exam-env/` — board agent, server API, admin UI
- `scripts/flash-os.sh` — write the SD image
- `scripts/install-software.sh` — SEB/agent (not implemented)
- `docs/` — course LaTeX

## Clone

```bash
git clone --recurse-submodules https://github.com/cannon-spencer/riscv-exam-computer.git
```
