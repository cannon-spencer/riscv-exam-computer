# riscv-exam-computer

A plug-and-play secure exam computer built around a RISC-V SBC. The device boots into a locked-down Linux environment and launches a Safe Exam Browser (SEB)-compatible kiosk so students can take exams via Canvas without invasive proctoring.

This repo is the **project “core”**: the exam/kiosk environment, policies, and glue needed to turn a RISC-V board into a deployable exam device.

## Repo layout

- `platform/reptilian-riscv/`: board bring-up + boot chain + kernel/userspace sources
- `exam-env/`: SEB wrapper / kiosk compositor configuration
- `docs/`: design docs (LaTeX sources in `docs/latex/`)

## Clone

This repo uses a git submodule for the platform/kernel stack. Clone with submodules:

```bash
git clone --recurse-submodules https://github.com/cannon-spencer/riscv-exam-computer.git
```
OR
```bash
git submodule update --init --recursive
```
