# Instructor console design

The console is used in a classroom, often on a laptop at the front of the
room, sometimes on a phone while walking between rows. It has one job: show
which exam boards are ready and let the instructor start or stop them with
confidence. The design is called **Proctor's desk**. It should feel like a
calm, precise instrument panel on warm paper, not like a SaaS dashboard.

Principles, in priority order:

1. **State is never ambiguous.** Every board status has a color, a shape,
   and a word. Remove any one of them and the status is still readable.
2. **Destructive and fleet-wide actions are explicit.** The UI always says
   how many boards an action targets and which boards it skipped and why.
3. **Quiet by default, loud when it matters.** Neutral surfaces. Color is
   spent on status and on the security warning, not on decoration.
4. **Works offline.** No CDN fonts, scripts, or images. Everything is served
   by the exam server.

## Tokens

All tokens are CSS custom properties on `:root` in `server/style/main.css`.
Components use tokens, never raw values.

### Color: light (default)

| Token              | Value     | Use                                      |
| ------------------ | --------- | ---------------------------------------- |
| `--paper`          | `#F4F1EA` | Page background (warm paper)             |
| `--surface`        | `#FBFAF6` | Cards, panels                            |
| `--surface-sunk`   | `#ECE8DE` | Wells, code blocks, inputs               |
| `--ink`            | `#1A1D21` | Primary text                             |
| `--ink-muted`      | `#5B606A` | Secondary text, labels                   |
| `--ink-faint`      | `#8C9099` | Placeholders, disabled                   |
| `--rule`           | `#DCD6C8` | Borders and dividers                     |
| `--rule-strong`    | `#BDB5A3` | Focused/hovered borders                  |
| `--accent`         | `#24308F` | Primary actions, links, selection        |
| `--accent-ink`     | `#FFFFFF` | Text on accent                           |
| `--accent-wash`    | `#E3E5F5` | Selected card background                 |
| `--focus`          | `#24308F` | 2px focus ring (with 2px offset)         |

### Color: dark

| Token              | Value     |
| ------------------ | --------- |
| `--paper`          | `#101317` |
| `--surface`        | `#171B21` |
| `--surface-sunk`   | `#0C0F12` |
| `--ink`            | `#E7E9EC` |
| `--ink-muted`      | `#A1A8B3` |
| `--ink-faint`      | `#6C737E` |
| `--rule`           | `#2A3038` |
| `--rule-strong`    | `#3C4450` |
| `--accent`         | `#A9B4FF` |
| `--accent-ink`     | `#0E1240` |
| `--accent-wash`    | `#1D2245` |
| `--focus`          | `#A9B4FF` |

The theme follows `prefers-color-scheme`. The toggle in the sidebar
overrides it with `data-theme="light|dark"` on `<html>`, remembered in a
non-HttpOnly `theme` cookie so the server renders the right theme and the
page never flashes.

### Status colors (semantic)

Each status has a foreground (`--st-*`) and a wash (`--st-*-wash`) token,
with light and dark values. **Status is always shown as icon + word + color.**
Offline cards are also desaturated and use a dashed border, so a grayscale
printout still reads correctly.

| Status     | Word         | Icon (shape)                    | Light fg  | Dark fg   |
| ---------- | ------------ | ------------------------------- | --------- | --------- |
| `running`  | Running      | Solid disc with a slow pulse    | `#17663F` | `#5FD39A` |
| `starting` | Starting…    | Quarter-arc spinner             | `#8A5300` | `#F2B45A` |
| `stopping` | Stopping…    | Square inside a ring            | `#8A5300` | `#F2B45A` |
| `idle`     | Ready        | Hollow ring                     | `#3E4A5C` | `#B4BDCA` |
| `error`    | Error        | Triangle with exclamation mark  | `#B0261A` | `#FF8A7A` |
| `offline`  | Offline      | Dashed ring                     | `#7C818A` | `#7C838E` |

Starting and stopping share a hue on purpose, since both mean "in
transition, wait". Their shapes and words differ. Animations stop under
`prefers-reduced-motion`.

The security warning (no agent token) uses `--warn` / `--warn-wash`
(light `#7A4B00` on `#FFF0CC`, dark `#FFD27A` on `#3A2A06`) with a
striped left edge and a warning icon.

### Type

Fonts are vendored in `server/public/fonts/` (SIL OFL, license included):

- **IBM Plex Sans** 400, 500, 600 for UI text.
- **IBM Plex Mono** 400, 500 for hostnames, numbers, IDs, JSON, and
  timestamps. Numbers always use `font-variant-numeric: tabular-nums`.

| Token        | Size / line height | Use                                  |
| ------------ | ------------------ | ------------------------------------ |
| `--t-xs`     | 12 / 16            | Meta, captions, table headers (caps) |
| `--t-sm`     | 13 / 18            | Secondary text, stats                |
| `--t-base`   | 14 / 20            | Body, controls                       |
| `--t-md`     | 16 / 24            | Card titles                          |
| `--t-lg`     | 20 / 28            | Section titles                       |
| `--t-xl`     | 28 / 34            | Page titles                          |
| `--t-num`    | 32 / 36 (mono)     | Summary counts                       |

Labels above stats are `--t-xs`, uppercase, `letter-spacing: .06em`,
`--ink-muted`. Page titles are 600 weight; everything else 400 or 500.

### Space, radius, elevation

- Spacing scale (4px base): `--s1` 4, `--s2` 8, `--s3` 12, `--s4` 16,
  `--s5` 24, `--s6` 32, `--s7` 48.
- Radius: `--r-sm` 4 (inputs, chips), `--r-md` 8 (cards, buttons),
  `--r-lg` 12 (panels, dialogs), `--r-pill` 999.
- Elevation: cards are flat with a 1px `--rule` border. Only the selection
  bar and toasts float (`--shadow-float`). No other shadows.
- Hit targets are at least 40×40px; 44px on touch (`pointer: coarse`).

## Layout

- Desktop (≥ 960px): 232px sidebar (wordmark, nav, theme toggle, sign
  out) + content column with max width 1280px and `--s6` gutters.
- Tablet / phone (< 960px): sidebar becomes a top bar with the wordmark
  and a horizontal nav row; gutters drop to 16px. No horizontal page
  scroll at 360px width.
- Board grid: `repeat(auto-fill, minmax(260px, 1fr))`; one column on
  phones.

## Components

| Component        | Notes                                                                          |
| ---------------- | ------------------------------------------------------------------------------ |
| `Shell`          | Sidebar/topbar, nav with `aria-current="page"`, theme toggle, sign-out form.    |
| `Banner`         | Page-wide notice. Variant `warn` for the missing agent token.                   |
| `StatusBadge`    | Icon + word, tinted wash. `role="status"` is not used (it would be too chatty). |
| `SummaryStrip`   | One tile per status with count; tiles act as filters (`aria-pressed`).         |
| `BoardCard`      | Checkbox, display name, mono hostname, badge, 3 key stats, last seen, link.    |
| `SelectionBar`   | Sticky bottom bar when ≥ 1 board selected: count, exam picker, Start, Stop.    |
| `ResultPanel`    | After an action: boards queued, boards skipped with the reason for each.       |
| `Button`         | `primary` (accent), `secondary` (outline), `danger` (error fg outline), `ghost`. |
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
6. Check both themes, a 360px phone width, keyboard-only use (visible focus,
   logical tab order), and `prefers-reduced-motion`.
7. Copy is plain and specific: "Start exam on 4 boards", not "Submit".
   Use sentence case. No exclamation marks.
8. Mono font for anything a person might compare character by character:
   hostnames, IPs, hashes, versions, timestamps, JSON.
