# riscv-exam-computer

A plug-and-play secure exam computer built around a RISC-V SBC. The device boots into a locked-down Linux environment and launches a Safe Exam Browser (SEB)-compatible kiosk so students can take exams via Canvas without invasive proctoring.

This repo is the **project “core”**: the exam/kiosk environment, policies, and glue needed to turn a RISC-V board into a deployable exam device.

## Repo layout

- `platform/reptilian-riscv/`: board bring-up + boot chain + kernel/userspace sources
- `exam-env/`: SEB wrapper / kiosk compositor configuration
- `docs/`: design docs (LaTeX sources in `docs/latex/`)

## Architecture (pre-alpha)

```mermaid
%%{init: {'themeVariables': {'fontSize': '17px', 'fontFamily': 'system-ui, sans-serif'}}}%%
flowchart LR
  UI["Kiosk / browser"]
  WM["Compositor / session"]
  OS["Linux + boot stack"]
  NET["HTTPS / Canvas / IdP"]
  UI <--> WM
  WM <--> OS
  OS <--> NET
```

**What each box is**

- **Kiosk / browser** — full-screen exam UI (SEB-class client or Linux wrapper target).
- **Compositor / session** — Wayland kiosk, single-app focus, process policy.
- **Linux + boot stack** — distro image, kernel, bootloader; platform sources live under `platform/reptilian-riscv/`.
- **HTTPS / Canvas / IdP** — outbound TLS to LMS and campus identity (e.g. Gatorlink, Duo).

Exam and SEB configuration are intended on verified or read-only storage. Secure boot and stronger isolation (e.g. PMP) are design targets, not fully implemented in this milestone.

## Known bugs / limitations

- End-to-end exam flow (Canvas + kiosk lockdown) is not yet validated on target hardware.
- Secure boot, partition layout, and hardware-backed key storage are not complete relative to the design draft.
- Add concrete, reproducible issues here as they are found; the pre-alpha report should stay in sync.

## Clone

This repo uses a git submodule for the platform/kernel stack. Clone with submodules:

```bash
git clone --recurse-submodules https://github.com/cannon-spencer/riscv-exam-computer.git
```
OR
```bash
git submodule update --init --recursive
```
