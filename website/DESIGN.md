---
name: Shadoword Website
description: A blazing spectrum with one silent channel in it — lit abstract imagery, flat matte data plates.
colors:
  night: '#07090d'
  night-plate: '#0d1420'
  scarlet: '#e6202c'
  scarlet-lamp: '#f04e55'
  scarlet-deep: '#b80e17'
  ink: '#e8e6e1'
  ink-soft: '#7a8089'
  on-scarlet: '#ffffff'
  rule: '#252a33'
  rule-strong: '#5a6371'
  sign-magenta: '#ff2f7f'
  sign-cyan: '#26d4ec'
typography:
  display-hero:
    fontFamily: "'League Gothic', 'Oswald', ui-sans-serif, system-ui, sans-serif"
    fontSize: 'clamp(4.25rem, min(14vh, 8.75vw), 9rem)'
    fontWeight: 400
    lineHeight: 0.825
    letterSpacing: '-0.03em'
  display-section:
    fontFamily: "'League Gothic', 'Oswald', ui-sans-serif, system-ui, sans-serif"
    fontSize: 'clamp(2rem, 4.4vw, 3.375rem)'
    fontWeight: 400
    lineHeight: 0.94
    letterSpacing: '-0.02em'
  display-column:
    fontFamily: "'League Gothic', 'Oswald', ui-sans-serif, system-ui, sans-serif"
    fontSize: 'clamp(1.375rem, 2.1vw, 1.875rem)'
    fontWeight: 400
    lineHeight: 1
    letterSpacing: '-0.01em'
  display-label:
    fontFamily: "'League Gothic', 'Oswald', ui-sans-serif, system-ui, sans-serif"
    fontSize: '1.375rem'
    fontWeight: 400
    lineHeight: 1
    letterSpacing: '0.01em'
  display-plate:
    fontFamily: "'League Gothic', 'Oswald', ui-sans-serif, system-ui, sans-serif"
    fontSize: '3.375rem'
    fontWeight: 400
    lineHeight: 0.9
    letterSpacing: '0.02em'
  display-wordmark:
    fontFamily: "'League Gothic', 'Oswald', ui-sans-serif, system-ui, sans-serif"
    fontSize: '1.0625rem'
    fontWeight: 400
    lineHeight: 1
    letterSpacing: '0.34em'
  numeral-outline:
    fontFamily: "'League Gothic', 'Oswald', ui-sans-serif, system-ui, sans-serif"
    fontSize: '2.75rem'
    fontWeight: 400
    lineHeight: 1
  mono-legend:
    fontFamily: "'Azeret Mono Variable', ui-monospace, 'SFMono-Regular', monospace"
    fontSize: '0.875rem'
    fontWeight: 400
    lineHeight: 1.55
  mono-caption:
    fontFamily: "'Azeret Mono Variable', ui-monospace, 'SFMono-Regular', monospace"
    fontSize: '0.75rem'
    fontWeight: 400
    lineHeight: 1.5
  mono-micro:
    fontFamily: "'Azeret Mono Variable', ui-monospace, 'SFMono-Regular', monospace"
    fontSize: '0.71875rem'
    fontWeight: 400
    lineHeight: 1.4
    letterSpacing: '0.04em'
rounded:
  square: '0'
  plate: '10px'
  plate-inner: '9px'
spacing:
  tight: '0.5rem'
  cell-y: '0.75rem'
  gutter: '1.25rem'
  gap: '1.5rem'
  gutter-wide: '2rem'
  block: '2.5rem'
  band-y: '3.5rem'
  band-y-lg: '4rem'
components:
  cta-solid:
    backgroundColor: '{colors.scarlet}'
    textColor: '{colors.on-scarlet}'
    rounded: '{rounded.square}'
    padding: '1rem 1.75rem'
  cta-solid-hover:
    backgroundColor: '{colors.scarlet-deep}'
    textColor: '{colors.on-scarlet}'
  cta-outline:
    backgroundColor: 'transparent'
    textColor: '{colors.scarlet-lamp}'
    rounded: '{rounded.square}'
    padding: '0.5rem 0.9375rem'
  cta-outline-hover:
    backgroundColor: '{colors.scarlet}'
    textColor: '{colors.on-scarlet}'
  plate:
    backgroundColor: '{colors.night-plate}'
    textColor: '{colors.ink}'
    rounded: '{rounded.square}'
  plate-heading:
    textColor: '{colors.ink}'
    typography: '{typography.display-label}'
    padding: '0.875rem 1.25rem'
  plate-row:
    textColor: '{colors.ink}'
    typography: '{typography.mono-legend}'
    padding: '0.75rem 1.25rem'
  rivet-plate:
    backgroundColor: '{colors.night-plate}'
    textColor: '{colors.ink}'
    rounded: '{rounded.plate}'
    padding: '2.25rem 1.75rem 2.5rem'
  mode-box:
    backgroundColor: 'transparent'
    size: '0.875rem'
    rounded: '{rounded.square}'
  mode-box-selected:
    backgroundColor: '{colors.scarlet}'
    size: '0.875rem'
    rounded: '{rounded.square}'
---

# Design System: Shadoword Website

## Overview

**Creative North Star: "Neo-Tokyo Neon Night — the Silent Channel"**

The world is **Neo-Tokyo Neon Night, abstracted from cityscape to spectrum**: an
abstract spectrogram burning in magenta and cyan, densest at the right and
dissolving to black at the left, with one perfectly black column standing at its
centre. Everything around that column is energised and broadcasting; the column
carries no signal at all, and it is the reader's machine. The argument of the
product — local execution stays silent on the network and remains the default —
is made as an image before a word is read.

**The imagery is not photographic and does not depict anything.** Its argument
is a whole energized field interrupted by one silent black column. The
authoritative statement of intent is the direction contract recorded as an HTML
comment at the top of `<body>` in `src/layouts/BaseLayout.astro`, which survives
into the production build. Read it before changing the visual language.

The world holds together on one split, and it is the single most important rule
in this file: **neon and glow live only inside the imagery.** Every region that
carries data is a flat, matte, near-black plate with crisp type on it and at most
a scarlet hairline. There is no bloom behind any text anywhere on the
page. That split is what lets a six-row spec table and a verbatim route list
exist inside a neon world instead of collapsing into the near-black-plus-glow rut
that every dark developer page occupies. It is also the product's argument
restated as a material: a privacy tool whose numbers glow is a privacy tool
asking to be taken on vibes.

The register is printed industrial signage, not UI chrome. Grounds are cold (hue
~215–220) so the page reads as the unlit ground a spectrum burns out of rather
than as a warm dark mode. Density is high and left-aligned: every heading, cell
and legend starts on a left margin, and cells are allowed to leave their right
half empty, which is
the panel-legend proportion the comp uses. Corners are square because printed
things are square, and radius is reserved to mean "this is a physical object" — a
rule that costs the page nothing and buys it two moments of real material. Motion
is atmosphere only: light, lateral drift and bin flicker, all of it inside the
imagery, none of it reporting a value. Nothing animates on scroll.

**Key Characteristics:**

- Lit abstract imagery against flat matte data plates; no glow behind type, ever.
- One accent (scarlet) rationed by meaning, split into a fill cut and an ink cut.
- Two self-hosted faces: a synthetically compressed condensed display grotesque
  against a monospace that carries every value, caption and legend.
- Square by default; radius, bevel, shadow and rivets belong to exactly two
  physical objects — one at the page's head, one at its foot.
- 1px lines throughout, and a scarlet hairline is the page's only structural
  device.
- Real controls and real text: zero client JavaScript ships, and nothing readable
  is baked into a raster.

## Colors

A cold near-black ground, one scarlet accent split by legal use, two inks, two
rule greys, and two neon hues quarantined inside the imagery.

### Primary

- **Signal Scarlet** (`#e6202c`): the identity red and the only interface accent.
  It is 4.37:1 on the page ground, so it is legal as a **fill, hairline or rule**
  and illegal as **type**. It appears as the 1px hairline bounding every major
  band, the selected mode's detent square and 2px marker, the 5px square tick
  beside `03 Commit`, the primary CTA's slab, the secondary CTA's border, the
  hero's animated lance, and the wordmark's mark.
- **Scarlet Lamp** (`#f04e55`): the _ink_ cut of the same colour — 5.62:1 on night
  and 5.21:1 on a plate. Every scarlet word on the page is this value: the
  `Default` flag in the model catalog, mutating HTTP methods (`PUT`/`POST`),
  received stream frames (`recv`), the F2 glyph on the shortcut plate, the
  secondary CTA's label.
- **Scarlet Deep** (`#b80e17`): the hover/pressed state of a scarlet fill. Fill
  only; never type.
- **On Scarlet** (`#ffffff`): pure white, and the **only** ink permitted on a
  scarlet fill. The off-white ink gives 3.65:1 there and fails; white gives
  4.56:1 and passes. The CTA is the most important element on the page, so it does
  not get to be the one thing that fails.

### Secondary

- **Sign Magenta** (`#ff2f7f`) and **Sign Cyan** (`#26d4ec`): the raster's own two
  burning hues, sampled off the delivered hero plate. They are real parts of this
  world and they exist **only inside the imagery** — as `mix-blend-mode: screen`
  radial overlays registered to bright bin clusters already present in the
  raster. They are declared in a bare `:root` block rather than in `@theme`
  precisely so Tailwind cannot generate a `text-` or `bg-` utility from them.

### Neutral

- **Rain Black** (`#07090d`): the page ground, and the document `theme-color`.
  Cold, not neutral-dark.
- **Plate Night** (`#0d1420`): any region carrying data — tables, route lists,
  instruction plates, notices, both physical plates. One step up from the page,
  bounded by a 1px grey rule. Never gradient, never glassy.
- **Ink** (`#e8e6e1`): primary off-white for headlines, values, commands, table
  cells and the hazard chevrons.
- **Ink Soft** (`#7a8089`): secondary ink for captions, legends, detent copy,
  column heads and micro labels. 4.64:1 on a plate, 5.01:1 on the ground.
- **Rule** (`#252a33`): decorative hairline — inset cell dividers, column
  dividers, footnote rules, plate heading underlines. 1.38:1 on purpose, matching
  the comp's near-invisible lines, and legal only where the boundary is already
  carried by layout and semantic HTML.
- **Rule Strong** (`#5a6371`): 3.28:1, for any rule that is the _only_ thing
  conveying a boundary — the unselected detent square's border, the physical
  plate's edge, the rivet rim, and the 1px stroke on the outlined numerals.

### Named Rules

**The Lit-Imagery Rule.** Neon, bloom, gradient light and saturated hue live
inside the imagery and nowhere else. Any region that carries data — a table, a
route list, a notice, a control, a command — is a flat matte plate with crisp
type and at most a scarlet hairline. There is no glow behind any text on this
page. Breaking this anywhere collapses the direction.

**The Pigment-and-Ink Rule.** Scarlet is two tokens because it is two jobs.
`scarlet` is pigment: fills, hairlines, rules, 2px markers, square ticks.
`scarlet-lamp` is ink: any scarlet word on a dark ground. Shipping one value for
both is how an accent colour quietly makes its own labels unreadable. White is
the only ink legal on a scarlet fill.

**The Rationed-Accent Rule.** Scarlet does scanning work, never decoration, and
it is rationed by _meaning_ rather than by quantity. Its authorised jobs are: the
band hairlines, the selected mode, mutating routes and received frames, the tick
that marks the fulcrum of the signal-path argument, and the primary action. If a
new scarlet mark cannot name which of those jobs it is doing, it is decoration
and the answer is off-white. Six repeated scarlet band labels were removed for
exactly this reason: scarlet on every band is scarlet meaning nothing on any of
them.

**The Sign-Hue Quarantine Rule.** `--sign-magenta` and `--sign-cyan` stay out of
`@theme`. They may only be used in screen-blend overlays registered to bright
bins already present in the raster. They may never reach a word, a control, a
border or a plate.

**The Token-Not-Opacity Rule.** Secondary ink is `--color-ink-soft`. An `/50` or
`/60` of the primary off-white lands near 4:1, which is how running prose quietly
stops being readable. There is no `text-ink/60` anywhere on this page and there
should not be one — and that is now verifiable against the built stylesheet,
because `global.css` scopes source detection to `src/` (`source(none)` plus
`@source '..'`). Under Tailwind's default whole-project scan this sentence was
itself a class-name candidate, so the rule minted the utility it bans.

**The Two-Greys Rule.** `rule` is decorative and may be near-invisible;
`rule-strong` is for any line that is the sole carrier of a boundary or a state,
and it clears WCAG 1.4.11 at 3.28:1. Picking the decorative grey for a functional
line is the failure this split exists to prevent.

**The Ring-Is-Not-The-Accent Rule.** The focus ring is off-white
(`outline: 2px solid var(--color-ink)`, `outline-offset: 2px`) with page black
flooded into the offset gap by `box-shadow: 0 0 0 2px var(--color-night)`. It is
deliberately not scarlet: scarlet is the primary CTA's own fill and the nav CTA's
own border, so a scarlet ring on either read as the slab quietly growing 2px
rather than as focus. Off-white is the one value nothing on this page is filled
with, and the black spacer guarantees the ring has a dark edge beneath it on any
ground — plate, scarlet slab, or a hot bin in the imagery.

## Typography

**Display Font:** League Gothic 400 (with Oswald, then `ui-sans-serif`)
**Body / Data Font:** Azeret Mono Variable (with `ui-monospace`, `SFMono-Regular`)

Both faces are self-hosted through `@fontsource`: a privacy product's site does
not phone a font CDN. Two faces and no third.

**Character:** A condensed grotesque set as printed signage against a monospace
that carries every value the page states. The display face is used for UI labels
too, not only headlines; the mono is for data, captions, legends and commands.
Display type is uppercase throughout; mono is uppercase for labels and legends
and sentence case for prose and footnotes.

The display face is **synthetically compressed, and the compression is a recorded
token rather than an ad-hoc transform.** The approved comp's headline runs at
0.266–0.278 advance per cap height, agreeing across all three lines — 59% of
Anton's natural width, which no obtainable face reaches natively. League Gothic
at `-0.03em` tracking with `scaleX(0.703)` reproduces it at IoU 0.740 and 51.1%
ink against the comp's 52%, the closest of five candidates rendered at matched
cap height. The compression eases off as size drops, the way a real family's text
cut is wider than its display cut: `--squeeze-display: 0.703`,
`--squeeze-heading: 0.78`, `--squeeze-label: 0.88`. Nothing below ~18px is
compressed at all.

Display sizes are specified as a **cap height divided by `--cap-em: 0.75`**,
League Gothic's cap/em ratio, because the comp was measured in cap heights and
font-size is not a visible quantity.

### Hierarchy

- **Display / hero** (400, `clamp(4.25rem, min(14vh, 8.75vw), 9rem)`, 0.825,
  `-0.03em`, uppercase, `scaleX(0.703)`): the page's one `h1`. Cap height is 10.5%
  of viewport height in the comp; the width term (8.75vw) is the same measurement
  taken off the comp's width, because a headline sized only from height overflows
  a narrow viewport and shrinks to nothing on a short wide one. Leading is 0.825
  unitless — line top-to-top is 1.10 × cap — so the three lines read as one
  stacked block.
- **Headline / section** (400, `clamp(2rem, 4.4vw, 3.375rem)`, 0.94, `-0.02em`,
  uppercase, `scaleX(0.78)`): every band's `h2`.
- **Title / column** (400, `clamp(1.375rem, 2.1vw, 1.875rem)`, 1, `-0.01em`,
  uppercase, `scaleX(0.88)`): the four signal-path step labels.
- **Label** (400, 1.375rem, 1, `0.01em`, uppercase, `scaleX(0.88)`): plate
  headings, mode labels, and the heading of any notice inside a plate. This is the
  workhorse UI label, and it is display, not mono.
- **Wordmark** (400, 1.0625rem, 1, `0.34em`, uppercase, **not compressed**): the
  lockup only. The letters are tracked wide apart, which is the opposite operation
  to compression; the two cannot be applied to the same word.
- **Body / legend** (mono, 0.875rem, 1.55): the hero sub-legend, table values,
  route paths, band lead-ins. Measures are capped in `ch` at a sentence
  boundary — `40ch` for the hero legend, `52ch` for a band lead-in, `80ch` for a
  notice.
- **Caption** (mono, 0.75rem, 1.5): detent copy, step bodies (`30ch`), notes,
  commands inside `<pre>`, footnotes.
- **Micro** (mono, 0.71875rem, 1.4, `0.04em`, uppercase): top-bar legends, table
  column heads, HTTP methods and frame directions, the shortcut plate's over- and
  under-labels.

The two remaining display steps belong to single components and are documented
there: the F2 glyph (`display-plate`, 3.375rem, uncompressed — a two-character
legend has no width problem to solve) and the outlined numeral
(`numeral-outline`, 2.75rem, stroke-only).

### Named Rules

**The Cap-Height Rule.** Every display size in this world is derived as a cap
height divided by `--cap-em` (0.75). If a new display size is needed, measure the
cap you want and divide; do not pick a font-size.

**The Recorded-Squeeze Rule.** Horizontal compression is one of three tokens, it
is applied with `transform-origin: left` so ink stays registered to the left
margin, and it stops entirely below ~18px. A squeezed 11px legend is just a
damaged 11px legend.

**The Authored-Break Rule.** Compressed display lines never wrap. Each headline
line is its own `nowrap` block, so a reflow cannot change the ink width of every
line at once. The breaks are authored, exactly as they are in the comp.

**The Mono-Carries-The-Facts Rule.** Every value the product asserts — a size, a
flag, a route, a frame, a command — is set in the monospace, and a value the
reader may copy is `whitespace-nowrap` so it cannot break mid-token. The display
face carries claims and labels; it never carries a number.

## Layout

One route, one column of full-bleed bands, in a fixed order: `TopBar`, `Hero`,
`ModeStrip`, `SignalPath`, `Choices`, `Daemon`, `Operation`, `TypePlate`. Each
band is full-bleed and bounded by a scarlet hairline (`.band-top`, or
`.band-bottom` on the top bar); inside it, content is centred in a
`max-width: 1536px` container.

**Gutters and rhythm.** Horizontal gutters are 1.25rem, 2rem from `sm` up. Band
padding is 3.5rem vertical, 4rem from `lg` up. Heading to content is 2.5rem;
plate to plate is 1.5rem. Inside a plate, rows are 1.25rem horizontal and
0.75rem vertical (0.625rem for list rows and column heads), and a plate heading
is 1.25rem × 0.875rem. The top bar is a fixed 3rem tall and deliberately not
sticky: the page's one action is duplicated at full size in the hero directly
beneath it, and a bar that followed the reader down would sit on the imagery for
the whole scroll.

**Breakpoints** are Tailwind's defaults — `sm` 640, `md` 768, `lg` 1024, `xl`
1280 — and the column counts they trigger are set by content measure, not by
symmetry: instruction plates go two-across at `md` and never four, because at
four the longest real command is clipped by its own plate even at 1536px. The
signal path splits two-column and four-across together at `xl`, not `lg`, because
a 27rem heading column at `lg` leaves each step 118px.

**The hero band is a floor, not a height.** `min-height: clamp(26rem, min(100vw /
2.489, 66svh), 46rem)`, rising to a 30rem floor at `lg`. The aspect term is the
hero raster's own 2.489:1, so at a desktop viewport `cover` trims almost nothing
and the raster's void column stays under the CSS lance. Below `lg` the
composition **changes rather than shrinks**: the headline block and the shortcut
plate join the flow at the foot of the frame over a graded scrim, and the
spectrum and its one black column read above them. At `lg` and up the block
leaves the flow
and is placed on the raster at `left: 2.86%`, vertically centred rather than
pinned — the block does not scale with the band (headline in vw, everything below
it in rem), so a top pin only survives above ~1500px.

**Alignment.** Everything starts on a left margin. Cells may leave their right
half empty; that emptiness is the panel-legend proportion, not waste. The
exceptions are the footer's right-hand column, right-aligned at `lg`, and the
model catalog's size column, right-aligned in tabular figures because it is the
one column read against itself rather than left to right.

### Named Rules

**The Hairline-Is-The-Structure Rule.** A 1px scarlet rule between bands is the
page's only structural device. There are no section cards, no container borders
around bands, and no background alternation between them. A new band gets
`.band-top` and nothing else.

**The Inset-Divider Rule.** Column and cell dividers are inset from the band's
edges rather than spanning it, and the outer edges of a group carry none. This is
what keeps a table reading as a printed panel instead of a spreadsheet. Where two
internal columns hold unequal row counts, the divider is painted on the container
rather than on the items, so it cannot stop a row short and leave a rule hanging
in mid-plate.

**The Divider-Follows-The-Stacking-Axis Rule.** A left-border divider draws
nothing when its cells stack. Every divided group re-points its rule across the
top when it stacks, and any marker that sits under a cell three-across moves to
the cell's left edge when stacked — an underline there would land on the divider
it shares with the next cell.

**The Floor-Not-Height Rule.** Bands that carry absolutely positioned composition
use `min-height` on a flex column, never a fixed height. With a fixed height and
`justify-content: flex-end`, content that outgrows the band is pushed off its
_top_ inside `overflow: hidden`, which decapitates the headline at 390px.

## Elevation & Depth

The page is flat. Depth comes from tonal layering (`night` → `night-plate`), 1px
rules, and the raster's own right-to-left density falloff — not from shadows.
There are no card shadows, no glass, no gradient panels and no hover lift on any
plate,
because plates are printed matter and printed matter does not float.

The single exception is the physical riveted plate, which carries the page's only
shadow and its only bevel. There are exactly two instances of it.

### Shadow Vocabulary

- **Plate lift** (`box-shadow: 0 18px 40px -12px rgb(0 0 0 / 0.9), inset 0 1px 0
rgb(255 255 255 / 0.07)`): a soft diffuse drop plus a 1px top-edge highlight.
  Used only by `.rivet-plate`. The drop says the object stands off the surface;
  the inset says it is bevelled and lit from above.
- **Rivet modelling** (two stacked radial gradients per fastener): a highlight
  offset up-left of centre ramping through `rule-strong` to a dark rim, plus a
  seat shadow cast down-right. Soft stops, not hard: at ~5px diameter a
  hard-stopped radial gradient has too few pixels for its own circumference to
  survive and aliases into a flat grey square. Offsets and gradient sizes are in
  px, not %, because each background tile is a fraction of an unknown plate width
  and a percentage offset would slide the highlight off the head as the viewport
  changes.

### Named Rules

**The Flat-Data Rule.** No shadow, gradient, blur or glow may appear on a region
that carries data. Elevation on this page is a material claim, and data is
printed, not lifted.

**The One-Light-Source Rule.** Light comes from up-left, once, for the whole
object: the plate's bevel is a top-edge inset highlight, the rivet highlight sits
up-left of centre, and the seat shadow falls down-right. Any new modelled detail
takes the same light.

## Shapes

Square by default and 1px throughout. **Radius means "this is a physical
object"**, so it is spent deliberately and almost never: anchors, buttons, inputs
and selects are reset to `border-radius: 0` in the base layer, and every plate,
table, notice, strip and divider carries 0.

- **Line weight** is 1px everywhere. The one 2px rule on the page is the selected
  mode's marker, and it is present but transparent when unselected so selecting a
  mode cannot shift the row.
- **Painted marks are square.** The mode strip's detent box is a 0.875rem square
  with a 1px `rule-strong` border that fills with scarlet pigment when selected;
  the tick beside `03 Commit` is a 5px square of pigment riding the label's
  baseline rather than centred on it, so it reads as a printed stamp beside the
  word and not as a bullet belonging to it. There are no round dots or bullets in
  this world.
- **Physical objects are rounded**: 10px radius, a 1px `rule-strong` edge, the
  bevel, the drop shadow, and domed rivets along both long edges. The chevron
  strip that caps the hero plate takes 9px so it sits inside the 10px edge.
- **The hazard chevron strip** is a `repeating-linear-gradient(-45deg, ink 0 8px,
night 8px 16px)` at 6px tall. Off-white, not scarlet — it is a "read this one
  differently" mark, and two strips of the accent spent on decoration costs the
  reader the ability to find the places where scarlet means something. It stays
  horizontal at every width: rotated to a vertical 6px edge the chevrons stop
  touching and read as a dashed line, which is a different sign entirely.
- **Rivet pitch is a custom property** (`--rivet-pitch`, default 25% = four per
  edge). A wide plate overrides it; the footer plate uses 12.5% for eight per
  edge. Four screws holding a 900px panel is not a fixing, it is a decoration that
  has forgotten what it is for, and the pitch is what makes both plates read as
  the same fastener at two sizes.

### Named Rules

**The Radius-Means-Object Rule.** Radius, bevel, shadow and rivets are one
indivisible set and they belong only to elements claiming to be physical things.
A rounded button or a chamfered table is wrong in both directions.

**The Two-Plates Rule.** There are exactly two physical objects on this page: the
shortcut plate at the page's head, which says what the machine does, and the
status plate at its foot, which says what the machine is. That is where a real
machine carries its plates. A third would make the material a texture instead of
a material.

## Components

### Buttons

- **Shape:** hard square (`border-radius: 0`), inline-flex, uppercase mono label.
- **Primary** (`.cta-solid`): a solid scarlet slab with a white mono label —
  0.875rem, weight 500, `0.08em` tracking — at 1rem × 1.75rem padding. It is the
  loudest thing on the page that is not the imagery, because the page's only job
  is to send a developer to the source. It appears twice: under the hero headline
  and at the foot of the page.
- **Secondary** (`.cta-outline`): a 1px scarlet border on transparent with a
  scarlet-lamp label at 0.71875rem and `0.1em` tracking, 0.5rem × 0.9375rem
  padding. The border may be pigment; the label may not.
- **Hover / active:** the primary darkens to `scarlet-deep` and, on `:active`,
  translates 1px down; the secondary inverts to a scarlet fill with white ink.
  Both transition at 140ms linear — a press, not an ease.
- **Focus:** the page-wide off-white ring with its page-black spacer. Never a
  scarlet ring.

### Cards / Containers

`.plate` is the only container in the system, and every band's data lives in one.

- **Corner style:** square (0).
- **Background:** `night-plate`, flat. No gradient, no image, no blur.
- **Border:** 1px `rule`.
- **Shadow:** none. See Elevation & Depth.
- **Internal structure:** a `display-label` heading on a 1px `rule` bottom border
  at 1.25rem × 0.875rem, then rows at 1.25rem × 0.75rem separated by 1px `rule`
  top borders. A plate that would stretch to match a taller neighbour takes
  `self-start` instead: an empty bordered box reads as a table that failed to
  load.

### Inputs / Fields

The page ships one real control: the mode strip, a three-position radio group
(`Off` / `Local` / `Remote`) built from visually hidden native radios, because the
three modes are mutually exclusive and that is exactly what a radio group is.
Arrow keys work and the whole cell is the click target.

- **Style:** a left-aligned cell with a 0.875rem square detent box (1px
  `rule-strong`) beside a `display-label` in `ink-soft`, and a caption detent line
  capped at `34ch`. Cells are separated by an inset 1px `rule` that follows the
  stacking axis.
- **Selected:** the box fills with scarlet pigment, the label brightens to `ink`,
  and a 2px scarlet marker appears — 45% of the cell's foot starting on the
  label's own left margin at `sm` and up, running down the cell's left edge when
  stacked. The label itself never turns scarlet; a scarlet word would be the
  accent reaching type.
- **Hover:** the detent box's border brightens to `ink`.
- **Focus:** because the input is visually hidden, the ring is drawn on the label
  group it names (off-white, 2px, 4px offset). Without that, keyboard users get no
  ring at all.
- **Transitions:** 120ms linear on background, border and colour.

### Navigation

A 3rem top bar, not sticky, bounded below by the scarlet hairline. The wordmark
sits left; two uppercase micro legends (`Linux first`, `Offline by default`) drop
out below `md` and `lg` respectively; the secondary CTA sits right. The wordmark
lockup is the product's real waveform-S mark inlined as SVG at 18px tall, filled
with `currentColor` in scarlet pigment and brightening to scarlet lamp on hover,
beside the wordmark at `0.34em` tracking. A landing page does not get to issue the
product a second identity, so the mark is the app's own.

### Signature Component: the physical plate

`.rivet-plate` is the world's one material: 10px radius, a 1px `rule-strong`
edge, an inset top-edge bevel, a soft outer drop, and domed rivets drawn as
repeating radial gradients on two pseudo-elements rather than one DOM node per
fastener. It appears exactly twice — the shortcut plate in the hero (15.5rem wide
in flow, 16.5% of the band when absolutely placed at `lg`, carrying a micro
over-label, the F2 glyph in `display-plate` scarlet lamp, a micro under-label, and
a rebindability footnote under a 1px hairline) and the status plate in the footer
(up to ~900px, a `display-label` heading over a definition list). The hero
instance also carries the hazard chevron strip on its top edge, which pushes that
instance's upper rivet row down locally rather than in the component. Padding is
authored on each instance so the copy clears the fasteners: ~2rem above the first
line and below the last.

### Signature Component: the outlined numeral

A large outlined display numeral beside a solid display label on a shared
baseline, four across, with inset 1px dividers between them. The numeral is
2.75rem, `color: transparent`,
`-webkit-text-stroke: 1px var(--color-rule-strong)` — the boundary grey rather
than the decorative one, so the outline survives at 1px — and compressed with
`--squeeze-label`. It is `aria-hidden`, because it is ordinal decoration on an
already-ordered list.

### Atmosphere: the raster and its motion

The hero raster (`public/images/hero-spectrum.webp`, 1536×617, WebP q78, 197 KB,
composed at the band's 2.489:1) is the LCP element: eager,
`fetchpriority="high"`, never lazy. It is an abstract spectrogram — densest at
the right, dissolving to black at the left, over a solid rail of low-frequency
light along the bottom — and everything lit on the page is inside it or blended
into it.

- **The void column** is the argument in one mark: contiguously black pixel
  columns x738–787 of 1536 (centre **49.64%**), contiguously black rows 0–299 of
  617, so the column runs **48.5%** of the plate's height before the spectrum
  closes over it. It is the one region of this raster that emits nothing, and it
  must stay that way.
- **The lance**: a 2px vertical gradient from transparent through scarlet to
  scarlet lamp, drawn in CSS at `left: 49.64%` and `height: 48.5%` — registered
  to those two measurements, so it runs down the inside of the void column and
  stops on the row where the silence does. It breathes opacity 0.55→1 over 6s and
  moves nothing else, because a lance that lengthened or swept would be a
  screensaver — and one that swept would also be pretending to scan something.
  **Regenerating the raster obliges re-measuring both numbers.** A 2px scarlet
  line is invisible over a lit spectrogram and unmissable inside a black column,
  which is why this is the one place the accent touches the imagery at all.
- **Off-white type sits on the raster unassisted at `lg` and up**, and that is a
  measurement rather than a hope: the headline zone samples 0.1%, 1.4% and 6.2%
  mean luminance at three points. No scrim there, because none is needed.
- **Three flaring bins**: soft radial gradients in `mix-blend-mode: screen` at
  69.8%/8% (magenta), 85.5%/30% (cyan) and 55%/26% (magenta), each placed over a
  local mean luminance of 47–82 out of 255, so a screen blend reads as a bin
  flaring rather than as a stain on empty black. They flicker on `steps(1, end)`
  with deliberately uneven keyframes — hold, stumble, fast double blink, hold —
  at 7s, 5.4s and 11s, so the three periods never align into a visible pulse. All
  three sit right of the void column and none touches it.
- **Two drift layers, lateral and never falling.** A spectrogram advances in time
  and nothing in this plate falls, so the atmosphere is vertical striation
  sliding sideways: `.drift-near` is a `repeating-linear-gradient` at 90deg with
  9px/10px/22px stops in `rgb(255 255 255 / 0.055)` at 0.75s, `.drift-far`
  15px/16px/40px in `rgb(255 255 255 / 0.03)` at 1.9s. Both run
  `inset: 0 -50%; width: 200%` and animate `translate3d(25%, 0, 0)` to
  `translate3d(-25%, 0, 0)`, so each travels exactly 50% of its own width and the
  loop has no seam. Leftward, because a spectrogram's newest column arrives at
  the right edge and its history slides away from it. **90deg exactly**: the
  angle orients the gradient _axis_ and the bands run perpendicular to it, so
  90deg gives bands precisely plumb. Frequency bins do not lean, and a raked
  striation over an upright spectrum reads as a printing error.
- **Scrims are flat ramps, never bloom.** Below `lg` the hero adds a neutral
  bottom-up gradient because the raster is cover-cropped hard toward its bright
  centre there. It is a graded ground under the type, not a glow behind the
  words.
- **Reduced motion is a rendering, not a freeze.** The global rule neutralises
  every duration, which would leave the drift as a static hatch of vertical lines
  ruled across the spectrum and the bins stuck on their first keyframe. So the
  drift is removed outright, the bins are pinned to a steady 0.2, and the lance
  stops breathing at full strength because it is composition rather than effect.
- **The reprise.** One band below the fold — the daemon band, because it is the
  band about sending audio somewhere else — re-enters the imagery as a 10rem
  strip of the same already-loaded file, bottom-pinned with
  `object-position: 50% 100%` so what shows is the raster's low-frequency rail: a
  signal on a wire rather than a field of it. **The void column is deliberately
  never reprised.** It is the offline claim, and this is the one band that is
  about sending audio to another machine; setting the mark twice would spend it,
  and it would make the claim on the wrong band. A `mask-image` fade plus one
  flat scrim means the image is gone before the route tables; the only text over
  the imagery is the heading's cap, sitting in the last quarter of the fade.
  Fixed at 10rem at every width: a reprise that grows with the window competes
  with the thing it is recalling.
- **The share card is a shot of the shipped hero**, not a composite:
  `public/images/shadoword-og.jpg`, 1200×532, mozjpeg q82, 145 KB. Its bottom
  edge _is_ the scarlet band hairline under the hero — row 531 — because on a
  page whose only structural device is that hairline, ending the card on one is
  composing it. Re-shoot it whenever the hero changes.

## Do's and Don'ts

### Do:

- **Do** give a new band `.band-top`, a `max-w-[1536px]` container and
  `px-5 sm:px-8 py-14 lg:py-16`, and put its data in `.plate`. That is the whole
  band recipe.
- **Do** keep every data-bearing region flat and matte, with crisp type and at
  most a scarlet hairline. If a new region needs light on it, the light belongs
  in the imagery.
- **Do** pick the scarlet cut by use: `scarlet` for a fill, hairline, rule or
  square mark; `scarlet-lamp` for a word; `on-scarlet` (white) for ink on a
  scarlet fill.
- **Do** name the job a new scarlet mark is doing. If it is not marking the band
  boundary, the selected mode, a mutating route or received frame, the fulcrum of
  an argument, or the primary action, set it off-white.
- **Do** use `--color-ink-soft` for secondary text, and `--color-rule-strong` for
  any line that is the only thing conveying a boundary or a state.
- **Do** derive a new display size as a cap height ÷ `--cap-em`, and take the
  matching `--squeeze-*` step for that size.
- **Do** author display line breaks as separate `nowrap` blocks.
- **Do** re-point a divider across the top when its cells stack, and move any
  cell marker to the left edge there.
- **Do** let content measure decide column counts — cap measures in `ch` at a
  sentence boundary, and drop a column rather than clip a command.
- **Do** keep interactive things real: native controls, live text, and no value
  baked into a raster.
- **Do** give `prefers-reduced-motion` a still _rendering_ — remove what only
  reads as motion, pin what reads as light.

### Don't:

- **Don't** put glow, bloom, blur, a lit gradient or a screen-blend behind type, a
  control or a table. The world's light lives in the imagery, and nowhere else.
- **Don't** let `--sign-magenta` or `--sign-cyan` out of the imagery, and don't
  move them into `@theme`, where Tailwind would generate `text-` and `bg-`
  utilities from them.
- **Don't** set type in `--color-scarlet`, and don't put the off-white ink on a
  scarlet fill (3.65:1).
- **Don't** express secondary text as an opacity of the primary ink.
- **Don't** make the focus ring scarlet, or drop the page-black spacer that gives
  it an edge over a scarlet slab or a lit sign.
- **Don't** add a scarlet label above a heading. Six of them turned a comp device
  into a page template and were removed; the one surviving instance in the signal
  path is a carried exception, not a slot for a new band to fill. A band's heading
  is its label.
- **Don't** round a corner, add a shadow, or add a bevel to anything printed —
  and don't add a third riveted plate.
- **Don't** use a round dot or a bullet as a mark; painted marks in this world are
  square.
- **Don't** add a second 2px rule; that weight belongs to the selected mode alone.
- **Don't** span a divider across a band's full width, or hang one on the outer
  edge of a group.
- **Don't** compress type below ~18px, and don't apply tracking-out and
  compression to the same word.
- **Don't** give a band a fixed height when it carries absolutely positioned
  composition, and don't let a plate stretch to match a taller neighbour.
- **Don't** animate on scroll, and don't let motion report a value, carry data or
  make a claim. Atmosphere only.
- **Don't** rasterize text, a command, a route or a control into an image.
