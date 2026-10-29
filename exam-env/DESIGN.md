# Instructor console design

The console is used in a classroom, often on a laptop at the front of the
room, sometimes on a phone while walking between rows. It has one job: show
which exam boards are ready and let the instructor start or stop them with
confidence.

The visual system follows **IBM's Carbon Design System**. The upstream
reference is vendored at
[`design/ibm-carbon-reference.md`](design/ibm-carbon-reference.md) (from
[awesome-design-md](https://github.com/VoltAgent/awesome-design-md)). This
file is the adaptation for the console and wins wherever the two disagree.
In short: white and Gray 10 layers, charcoal type, square corners, no
shadows, IBM Plex Sans with light-weight (300) display numbers and titles,
IBM Blue as the single accent, and Carbon's own patterns for the pieces:
side nav, tags, content switcher, inline notifications, batch-action bar,
structured list, progress bar.

Principles, in priority order:

1. **State is never ambiguous.** Every board status has a color, a shape,
   and a word. Remove any one of them and the status is still readable.
2. **Destructive and fleet-wide actions are explicit.** The UI always says
   how many boards an action targets and which boards it skipped and why.
3. **Engineered, not decorated.** Structure comes from layers and 1px
   rules. Blue marks what is interactive.
4. **Works offline.** No CDN fonts, scripts, or images. IBM Plex Sans and
   Mono are vendored (OFL).

## Themes

**White** is the default. **Gray 100** applies when the system prefers dark
or the instructor picks Dark in the sidebar toggle.

## Tokens

All tokens are CSS custom properties on `:root` in `server/style/main.css`,
named after Carbon's own tokens.

| Token              | White     | Gray 100  | Use                                   |
| ------------------ | --------- | --------- | ------------------------------------- |
| `--background`     | `#FFFFFF` | `#161616` | Page; fields and code inside layers   |
| `--layer`          | `#F4F4F4` | `#262626` | Sidebar, tiles, cards, panels         |
| `--layer-hover`    | `#E8E8E8` | `#333333` | Hovered layer items                   |
| `--layer-selected` | `#E0E0E0` | `#393939` | Current nav item, selected card/tile  |
| `--border-subtle`  | `#E0E0E0` | `#393939` | Dividers                              |
| `--border-strong`  | `#8D8D8D` | `#6F6F6F` | Field bottom rule, switcher outline   |
| `--text-primary`   | `#161616` | `#F4F4F4` | Primary text                          |
| `--text-secondary` | `#525252` | `#C6C6C6` | Secondary text, labels                |
| `--text-helper`    | `#6F6F6F` | `#A8A8A8` | Helper text, meta                     |
| `--interactive`    | `#0F62FE` | `#0F62FE` | Primary button, nav indicator, bar    |
| `--link`           | `#0F62FE` | `#78A9FF` | Links, ghost buttons                  |
| `--focus`          | `#0F62FE` | `#FFFFFF` | 2px inset focus outline               |

### Status: Carbon tags

Badges are Carbon tags (24px pills, 12px label). Each status sets a text
color `--st-*` and a tag pair `--tag-*` / `--tag-*-ink`.

| Status     | Word      | Icon (shape)                   | Tag (White)         | Tag (Gray 100)      |
| ---------- | --------- | ------------------------------ | ------------------- | ------------------- |
| `running`  | Running   | Solid disc with a slow pulse   | green 20 / green 70 | green 80 / green 20 |
| `starting` | Starting… | Quarter-arc spinner            | yellow 10 / 70      | yellow 80 / 20      |
| `stopping` | Stopping… | Square inside a ring           | yellow 10 / 70      | yellow 80 / 20      |
| `idle`     | Ready     | Hollow ring                    | blue 20 / blue 70   | blue 80 / blue 20   |
| `error`    | Error     | Triangle with exclamation mark | red 20 / red 70     | red 80 / red 20     |
| `offline`  | Offline   | Dashed ring                    | gray 20 / gray 80   | gray 80 / gray 20   |

Error cards also carry a 3px red left rule; offline cards drop to the page
color with a 1px outline, so status survives a grayscale screenshot.

### Type

IBM Plex Sans 300 / 400 / 600; IBM Plex Mono 400 / 500 for hostnames,
timestamps, JSON and the add-site chips.

| Token      | Size / line height | Weight | Tracking | Use                          |
| ---------- | ------------------ | ------ | -------- | ---------------------------- |
| `--t-label`| 12 / 16            | 400    | 0.32px   | Labels, helper text, tags    |
| `--t-body` | 14 / 20            | 400    | 0.16px   | Body, controls (base)        |
| `--t-h2`   | 16 / 22            | 600    | 0        | Card titles                  |
| `--t-h3`   | 20 / 28            | 400    | 0        | Panel titles, board stats    |
| `--t-h5`   | 32 / 40            | 300    | 0        | Sign-in, empty states        |
| `--t-disp` | 42 / 50            | 300    | 0        | Page titles, summary counts  |

The light 300 weight at display size is the Carbon/IBM signature; never
bold large type.

### Shape and depth

- **Square corners everywhere** except tags (pills, as in Carbon).
- **No shadows** on anything in the page flow. Tiles and cards sit 2px
  apart on the white page so the gaps read as rules. The only shadow is
  under the floating batch-action bar.
- Motion: 70ms Carbon "productive" easing on background color only.

## Components (Carbon equivalents)

| Console piece   | Carbon pattern           | Notes                                                        |
| --------------- | ------------------------ | ------------------------------------------------------------ |
| Sidebar         | UI shell side nav        | Gray 10 rail; current item gets `--layer-selected` and a 3px blue left bar. |
| Summary tiles   | Clickable tiles          | Light 42px count; pressed tile gets a 3px status-colored bottom bar. |
| Board card      | Selectable tile          | Selected = 1px charcoal outline on `--layer-selected`.       |
| Selection bar   | Data table batch actions | Full IBM Blue 48px bar with white text; the only blue surface. |
| Buttons         | Button kinds             | Square: primary (blue), secondary (charcoal), danger tertiary (red outline, solid when armed), ghost (blue text). Label left-aligned. |
| Fields          | Text input / select      | `--background` fill, 1px bottom rule, 2px inset focus outline. |
| Filters, theme  | Content switcher         | Outlined segments; selected segment is inverted charcoal.    |
| Token banner    | Inline notification      | Warning: yellow 3px left bar. Result panel: info, blue bar.  |
| Activity        | Structured list          | 1px row rules, hover fill; actor shown as tags.              |
| Memory          | Progress bar             | 8px square track, blue fill (yellow/red when high/full).     |
| JSON preview    | Code snippet             | Plex Mono 12/16 on `--background`.                           |
| Exam downloads  | Ghost buttons            | "Download .seb" and "Download JSON"; "With password…" opens a full-width form row; Delete sits at the far right. |

## Brand marks

- `Logo`: college wordmark at the top of the sidebar and sign-in card;
  reversed white-text file in Gray 100.
- `ProductMark`: small RISC-V Exam Computer mark (max 152px) in the sidebar
  footer and the foot of the sign-in card; in Gray 100 it uses a file
  with white "RISC" lettering instead of the navy one.

## Layout

- Desktop: 256px side nav + content column (max 1584px, 32px gutters).
- < 960px: side nav becomes a top bar with tabs (2px blue underline on the
  current tab); RISC-V mark and theme switcher are hidden; the batch-action
  bar docks full width to the bottom edge.
- Board grid `repeat(auto-fill, minmax(288px, 1fr))` with 2px gaps.

## Rules for future UI work

1. Use tokens. If a value is missing, add a token here first.
2. Never encode meaning in color alone. Pair it with an icon shape and a word.
3. Every action that changes boards reports what happened per board,
   including skipped boards and why.
4. Text from boards or instructors is untrusted. Render it as text.
5. Keep it offline: no external fonts, scripts, images, or analytics.
6. Check 360px width, keyboard-only use, reduced motion, and both themes.
7. Copy is plain and specific: "Start exam on 4 boards", not "Submit".
8. Mono for anything compared character by character.
9. Carbon rules that apply here: one accent (IBM Blue), square corners,
   no drop shadows in the page flow, light weight for large type, positive
   tracking on body and labels.
