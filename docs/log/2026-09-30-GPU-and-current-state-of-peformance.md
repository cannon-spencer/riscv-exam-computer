# GPU and Current State of Performance

## TL;DR
GPU works, but performance is still bad. Need to reassess the board and Canvas possibility.

## Summary
- **GPU problem: solved.** SEB's browser engine now renders through the PowerVR GPU, verified directly on the running process.
- **Performance problem: not solved.** SEB is still slow, because the bottleneck is one CPU core running mostly Canvas's JavaScript, not graphics.
- **Long-run test:** two ~12-hour sessions with no crashes, but the board ran very hot (~75°C) and had display glitches. It needs cooling for long use cases.
- Everything below is on my board only (an isolated test environment). Nothing is pushed to the repo, as this is a drastically altered environment.

---

## 1. Starting point
| Item | Value |
|---|---|
| Board | Orange Pi RV: StarFive JH7110, 4 cores @ 1.5 GHz, 8 GB RAM |
| GPU | PowerVR BXE-4-32, **OpenGL ES only** (no desktop OpenGL) |
| OS | Vendor Debian, frozen to a 2022-12-25 debian-ports snapshot, kernel 5.15, GNOME on Wayland |
| GPU driver | Vendor kernel module + IMG userspace 1.17 (GNOME already ran on the GPU: glmark2-es2-wayland score 735) |
| SEB before | X11 mode with WebKit compositing disabled = everything drawn on the CPU |

**Why SEB couldn't use the GPU:** Debian's WebKitGTK 2.38.2 is built for desktop OpenGL and failed with "Could not create EGL context". The other routes were dead too:
- X11 GPU path: Xwayland GPU acceleration (glamor) crashes on this driver, so the vendor disables it.
- Zink (OpenGL over Vulkan): the IMG Vulkan driver lacks features Zink needs, so it aborts.

---

## 2. What we did
1. **Rebuilt WebKitGTK 2.38.2 with OpenGL ES** (`ENABLE_GLES2=ON`, same version as Debian's, installed over it with a rollback backup). Build time ~11.5 hrs on the board.
2. **Fixed the GPU environment:**
   - `GDK_GL=gles`: without it, GTK asks for desktop GL and falls back to slow CPU copies.
   - `MESA_LOADER_DRIVER_OVERRIDE=pvr`: root processes otherwise load the display driver (`starfive`) instead of the GPU driver.
3. **Found why SEB still failed:** in lockdown mode, SEB switches to its own virtual console and draws straight to the framebuffer. That leaves the GNOME compositor behind, so WebKit's GPU path breaks (EGL init fails, Wayland "Error 71").
4. **Moved SEB into cage** (a kiosk Wayland compositor) running on that console, so WebKit has a compositor again.
5. **Patched wlroots** (cage's renderer). It rejected the driver for missing `GL_EXT_unpack_subimage`, which GLES 3.2 already includes. Patch: skip that check on GLES 3+.
6. **Patched SEB** with a flag to skip its console switch when cage already owns the screen.
7. **Patched cage** to force 1280x720 (it always picked the monitor's 2K mode).
8. **Sudo rule** so SEB runs as root inside cage without pkexec (pkexec was stripping the GPU env vars).

George's WebKitGTK view file is used as-is.

---

## 3. How we know the GPU is working
| Test | Method | Result |
|---|---|---|
| WebKit on the GPU (desktop) | MiniBrowser on Wayland, then inspected the web process's loaded libraries (`/proc/<pid>/maps`) | `pvr_dri.so`, `libpvr_dri_support.so`, `libGLESv2_PVR_MESA.so` loaded, no EGL errors: **PASS** |
| WebKit as root | Same test, run as root with the env vars | **PASS** (before the env vars: wrong driver loaded, CPU fallback) |
| WebKit inside cage | MiniBrowser in cage on its own console, same library check | PowerVR libs loaded: **PASS** |
| **SEB inside cage** | SEB's own WebKit process, same library check | `pvr_dri.so` + IMG GLES libs loaded: **PASS** |
| Display mode | Kernel DRM debug state, read while SEB ran in cage | 1280x720 @ 60 Hz confirmed (cage sets its own mode; GNOME's 50 Hz setting applies only to the desktop) |

**Why this is valid evidence:** a process only loads `pvr_dri.so` and IMG's GLES library when it creates a GPU context. Stock WebKit couldn't create one at all ("Could not create EGL context"). In our own tests, the same library check came back empty whenever the setup was wrong (for example, running as root without the env vars), and showed the PowerVR libraries once it was right. So the result comes from the rebuild and setup, not coincidence.

---

## 4. How we know the CPU is the bottleneck
| Measurement | Method | Result |
|---|---|---|
| CPU load while using Canvas | `top` | SEB's WebKit process at **100% of one core**; whole system ~44% (other 3 cores mostly idle) |
| CPU clock | cpufreq governor / frequency | Already `performance`, **1.5 GHz max**, so there's no free speed left |
| JavaScript JIT | WebKit build config | JIT, DFG and FTL enabled, so JavaScript isn't stuck in a slow interpreter build |
| Where the main thread spends time | gdb sampling, 33 pauses (perf counters aren't available in this kernel) | **~42% JavaScript engine**, ~39% unlabeled (SEB/Qt), ~12% WebKit layout/paint, ~6% idle |
| Real-world load time | Stopwatch, post Duo login (is this your device page) to Robert's Canvas page loaded | **Over 2 minutes** (single run timed, all runs very slow) |

**Reading it:**
- One thread maxes out one core, so the limit is single-core CPU speed.
- JavaScript is the biggest identifiable cost, mostly Canvas loading and running its large scripts.
- The GPU only does compositing and scrolling. In WebKit 2.38, JavaScript, layout and painting stay on the CPU, so the GPU can't take over the main cost.

**Limits of this data (to be upfront):**
- 33 samples is small, so treat the percentages as rough shares, not exact numbers.
- Page-load time is a single after-GPU run; we don't have a before baseline to compare against. By natural use, it's too slow for any test-taking use.

---

## 5. Result
| Goal | Status |
|---|---|
| WebKit renders on the PowerVR GPU | Done, verified |
| SEB runs on the GPU with lockdown | Done, verified (cage kiosk) |
| SEB fast enough for Canvas | **No.** CPU-bound |

**The GPU problem is solved, but solving it didn't fix performance.** The real limits are:
- **The board:** the JH7110's cores are slow, and the work Canvas needs is mostly single-threaded.
- **Canvas:** a heavy JavaScript web app built for desktop browsers. The dashboard is the heaviest page, and it also loads analytics/tracking scripts an exam doesn't need.

---

## 6. Board stability and thermals
**Long-run test.** The board ran two separate ~12-hour sessions with no crashes. Temperatures weren't logged continuously, but readings reached about 75°C, and the board was too hot to touch. Several times the display flashed and tore, and needed an HDMI replug or a short cooldown to recover. The cause (heat vs. HDMI) isn't confirmed. Conditions: open desk, ~73°F room, no heatsink or fan.

**Takeaway:** it runs for long sessions, but display glitches mid-exam are a risk. Add a heatsink/fan and retest before relying on it for back-to-back exams.

---

## 7. Options
**Keep tuning (small gains, do now)**
- Launch SEB straight to the quiz URL instead of the dashboard. Real exams open the quiz directly anyway.
- Domain allowlist (only Canvas, UF login, Duo): less JavaScript, tighter lockdown.
- Boot straight into the cage kiosk with no GNOME desktop running.

**Alternative hardware (the only direct fix for the bottleneck)**
- A board with faster single-core CPU performance. Not researched yet.
- Most of the cage/SEB work should carry over.

**Canvas replacement (unlikely)**
- Canvas is the university's choice, so replacing it isn't realistic. We can only reduce what it loads.

---

## 8. Open items
- Discuss how to move forward and set expectations with Robert/stakeholders.
- A failed UF login redirects to ufl.edu instead of Canvas, and the user gets stuck there. SEB's lockdown blocks normal navigation, so there's no way back to Canvas.