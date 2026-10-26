# Instructor console design

The console is used in a classroom, often on a laptop at the front of the
room, sometimes on a phone while walking between rows. It has one job: show
which exam boards are ready and let the instructor start or stop them with
confidence. The design is called **Night**. It is flat and dark only:
square-cornered charcoal panels with 1px rules on a near-black page, a
dense layout, white for primary actions, and color spent only on status.
Think instrument panel, not a marketing site: no rounded corners, no
blur, no glow, no hover lift.

Principles, in priority order:

1. **State is never ambiguous.** Every board status has a color, a shape,
   and a word. Remove any one of them and the status is still readable.
2. **Destructive and fleet-wide actions are explicit.** The UI always says
   how many boards an action targets and which boards it skipped and why.
3. **Quiet by default, loud when it matters.** Structure comes from 1px
   rules and spacing. Color is spent on status and on the security
   warning only.
4. **Works offline.** No CDN fonts, scripts, or images. Everything is served
   by the exam server.

## Tokens

All tokens are CSS custom properties on `:root` in `server/style/main.css`.
Components use tokens, never raw values. There is one theme (dark); the
old light/dark toggle is no longer shown.

### Color

| Token              | Value                 | Use                                   |
| ------------------ | --------------------- | ------------------------------------- |
| `--paper`          | `#121214`             | Page background                       |
| `--paper-glow`     | `#17171C`             | Soft radial light at the top of page  |
| `--surface`        | `#18181B`             | Cards, panels, tiles                  |
| `--surface-hover`  | `#202024`             | Hovered cards, secondary btns         |
| `--surface-sunk`   | `#151518`             | Empty states, quiet panels            |
| `--surface-solid`  | `#1C1C20`             | Opaque fallback (select menus)        |
| `--glass`          | `#1C1C20`             | Floating bars (solid, no blur)        |
| `--ink`            | `#F5F5F7`             | Primary text                          |
| `--ink-muted`      | `#A1A1A6`             | Secondary text                        |
| `--ink-faint`      | `#6E6E73`             | Labels, hints, placeholders           |
| `--rule`           | `#2A2A2E`             | 1px borders and dividers              |
| `--rule-strong`    | `#3A3A40`             | Inputs, offline cards (dashed)        |
| `--accent`         | `#F5F5F7`             | Primary button fill                   |
| `--accent-ink`     | `#121214`             | Text on accent                        |
| `--accent-wash`    | `#232327`             | Current nav item, selected card       |
| `--focus`          | `#F5F5F7`             | 2px focus ring (3px offset)           |

### Status colors (semantic)

Each status has a foreground (`--st-*`) and a wash (`--st-*-wash`, the
foreground at 14-16% alpha). **Status is always shown as icon + word +
color.** Offline cards are also transparent with an outline, so a
grayscale screenshot still reads correctly.

| Status     | Word         | Icon (shape)                    | Color     |
| ---------- | ------------ | ------------------------------- | --------- |
| `running`  | Running      | Solid square with a slow pulse  | `#7FD08A` |
| `starting` | Starting…    | Square with a spinning edge     | `#E0A458` |
| `stopping` | Stopping…    | Small square inside a square    | `#E0A458` |
| `idle`     | Ready        | Hollow square                   | `#ECECEF` |
| `error`    | Error        | Triangle with exclamation mark  | `#E5484D` |
| `offline`  | Offline      | Dashed square                   | `#6E6E76` |

Starting and stopping share a hue on purpose, since both mean "in
transition, wait". Their shapes and words differ.

The security warning (no agent token) uses `--warn` (`#E5C07B`) for its
icon and heading on a `--warn-wash` background.

### Type

- UI text uses the system font (`-apple-system`, SF Pro, Segoe UI
  Variable, ...) with vendored **IBM Plex Sans** as the fallback, so it
  stays offline.
- **IBM Plex Mono** 400, 500 (vendored) for hostnames, IDs, JSON, and
  timestamps. Numbers always use `font-variant-numeric: tabular-nums`.

| Token        | Size / line height | Use                                  |
| ------------ | ------------------ | ------------------------------------ |
| `--t-xs`     | 12 / 16            | Meta, captions, stat labels          |
| `--t-sm`     | 13 / 18            | Secondary text                       |
| `--t-base`   | 15 / 22            | Body, controls, stat values          |
| `--t-md`     | 17 / 24            | Card titles                          |
| `--t-lg`     | 22 / 28            | Section titles                       |
| `--t-xl`     | 34 / 40            | Page titles (700)                    |
| `--t-num`    | 34 / 40            | Summary counts                       |

Headings use `letter-spacing: -0.02em`. Labels are sentence case in
`--ink-faint`, not uppercase.

### Space, radius, elevation, motion

- Spacing scale (4px base): `--s1` 4, `--s2` 8, `--s3` 12, `--s4` 16,
  `--s5` 24, `--s6` 40, `--s7` 64.
- Radius: all `--r-*` tokens are 0. Nothing is rounded, including
  buttons, badges, inputs, and status shapes.
- Elevation: surfaces are flat fills with 1px `--rule` borders. Hover
  changes the fill only; nothing lifts or scales. The selection bar and
  login card float with `--shadow-float` on an opaque `--glass` fill.
- Motion: `.2-.3s` with `--ease`. All transitions and status
  animations stop under `prefers-reduced-motion`.
- Hit targets are at least 40×40px; 44px on touch (`pointer: coarse`).

## Layout

- Desktop (≥ 960px): 240px borderless sidebar (wordmark, nav, sign out)
  + content column with max width 1320px and `--s6` gutters.
- Tablet / phone (< 960px): sidebar becomes a sticky top bar with
  the wordmark and a horizontal nav row; gutters drop to 16px. No
  horizontal page scroll at 360px width.
- Board grid: `repeat(auto-fill, minmax(280px, 1fr))`; one column on
  phones.

## Components

| Component        | Notes                                                                          |
| ---------------- | ------------------------------------------------------------------------------ |
| `Shell`          | Sidebar/topbar, nav with `aria-current="page"`, sign-out form.                  |
| `Banner`         | Page-wide notice. Variant `warn` for the missing agent token.                   |
| `StatusBadge`    | Icon + word, tinted wash. `role="status"` is not used (it would be too chatty). |
| `SummaryStrip`   | One tile per status with count; tiles act as filters (`aria-pressed`).         |
| `BoardCard`      | Checkbox, display name, mono hostname, badge, 3 key stats, last seen, link.    |
| `SelectionBar`   | Sticky bottom bar when ≥ 1 board selected: count, exam picker, Start, Stop.    |
| `ResultPanel`    | After an action: boards queued, boards skipped with the reason for each.       |
| `Button`         | Square buttons: `primary` (white), `secondary` (gray fill), `danger` (red wash), `ghost`. |
| `Field`          | Label above, hint below, error text in `--st-error` with an icon.              |
| `Stat`           | Caps label + mono value; optional `Meter` (memory) or `Sparkline` (temp).      |
| `Timeline`       | Activity rows: time (mono), actor chip (Instructor / Board / System), text.    |
| `JsonPreview`    | Mono `<pre>` on `--surface-sunk`, scrolls horizontally inside itself.          |
| `EmptyState`     | Line illustration (inline SVG, `currentColor`), one sentence, one action.      |
| `ConfirmButton`  | Two-step destructive button: first click arms it ("Click again to forget").    |

## Rules for future UI work

1. Use tokens. If a value is missing, add a token here first.
2. Never encode meaning in color alone. Pair it with an icon shape and a word.
3. Every action that changes boards reports what happened per board,
   including skipped boards and why.
4. Text from boards or instructors (hostnames, names, URLs) is untrusted.
   Render it as text. Never use `inner_html` or build HTML strings.
5. Keep it offline: no external fonts, scripts, images, or analytics.
6. Check a 360px phone width, keyboard-only use (visible focus,
   logical tab order), and `prefers-reduced-motion`.
7. Copy is plain and specific: "Start exam on 4 boards", not "Submit".
   Use sentence case. No exclamation marks.
8. Mono font for anything a person might compare character by character:
   hostnames, IPs, hashes, versions, timestamps, JSON.
