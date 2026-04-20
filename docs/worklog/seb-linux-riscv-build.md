# `seb-linux` on riscv64: WebKitGTK build line

**EEL4907 · Hardened RISC-V Exam Computer**

**Accomplishment:** We **drove the native build** on Orange Pi: **`qmake6`** correctly selected the **WebKitGTK** backend (no usable Qt WebEngine/Chromium **riscv64** packages in practice), pulled in **Qt6 + GTK + WebKit** dev deps on **sid**, and got a **real compile** far enough to hit **actionable code issues** (Qt `signals` vs GLib naming, spell-check API typing)—with **upstream issues** filed/tracked.

**When:** **Apr 1 – Apr 18, 2026** (iterations gated by **board availability**).

**Honest status:** Build **not green** at milestone; blockers split between **shallow source fixes** and **hardware handoff** (documented in report/tickets). This log is the **engineering record** of that line, not a success narrative.

Corroboration: **BAC-10** and related tickets, upstream tracker, failed build logs.
