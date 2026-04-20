# SEB on Arch (x86_64): lockdown reference demo

**EEL4907 · Hardened RISC-V Exam Computer**

**Accomplishment:** Before depending on riscv64 graphics stacks, we showed **SEB-class lockdown on real Linux**: reliable fullscreen launch, exits blocked under stress, and policy failures that **fail closed**—with **screenshots/slides/recordings** suitable for demos.

**When:** **Mar 5 – Apr 14, 2026** (multiple lab sessions). Optional **VM probe** (Apr 12–14): inconclusive; **not** claimed in formal results.

**What we ran**

- **Arch Linux (x86_64)** — official SEB install and exam-style fullscreen.
- **Escape / WM stress** — keyboard shortcuts and normal desktop exits while SEB ran (**blocked** as expected).
- **Process / compatibility policy** — blacklisted apps present → SEB **refused start** or stopped when policy was violated.

**Why it mattered:** Gave a **known-good behavioral baseline** independent of Orange Pi schedule; anything flaky on riscv64 could be contrasted with this reference.

Corroboration: **Git**, **Jira (BAC)**, demo artifacts in slides/repo where committed.
