# Orange Pi RV: Debian sid (riscv64) lab environment

**EEL4907 · Hardened RISC-V Exam Computer**

**Accomplishment:** A **repeatable riscv64 lab**: SD images that boot **Debian sid**, **UART + SSH** for work, working **Ethernet** (including **USB–LAN** and host sharing when the bench required it), **`apt`** against main mirrors, and **GUI smoke** (HDMI and/or **VNC**) so we could **compile and debug** browser-class stacks on silicon.

**When:** **Mar 18 – Apr 18, 2026**.

**Highlights**

- Imaging / reflashing until boots were **stable** (bad power-offs and SD wear were real).
- **Networking** debug separated **bench cabling / dongles** from OS issues.
- Package baseline showed the **riscv64** graph was **viable** for Qt/WebKit build deps even though **`apt install chromium`** is not a practical shortcut on this arch.

This environment is what **on-device `seb-linux` builds** depended on—not a finished exam appliance, but a **usable lab**.

Corroboration: **Jira**, **lab notes**, **Git** where configs or notes landed.
