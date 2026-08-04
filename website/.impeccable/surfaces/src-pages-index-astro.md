---
version: 1
slug: 'src-pages-index-astro'
primary_target: 'src/pages/index.astro'
related_targets: ['src/layouts/BaseLayout.astro', 'src/styles/global.css']
---

# website/ — public landing page

## Scope and mode

**Persuade.** One page at `src/pages/index.astro`. Success is the visitor
understanding that inference happens on their own machine, and going to the
repository. Build-from-source is the only distribution, so the repo _is_ the
conversion.

Audience: a privacy-minded Linux developer who assumes the offline claim is
marketing until shown otherwise. Proof available: the real mode set, the real
push-to-talk key, the real model catalog, the real endpoints and stream frames.
Constraint: no measured latency exists, so the page states that absence on a
part rather than filling it in.

## Chosen direction

**Neo-Tokyo Neon Night.** Seed `322d3899`; the roll assigned RED/BLACK
Separation at ASSIGNED INDEX 3 and the user took this dealt challenger instead,
which the playbook permits freely. Steer that produced it: the previous world's
simulated waveform and fake signal monitor "do nothing", and the page should
lean on abstract imagery and real motion instead.

**Approved comp:** `.impeccable/mocks/comp-a-sealed-tower.webp` (sidecar carries
`"approved": true`). Rejected: `comp-b-split-ward.webp`, `comp-c-signage-stack.webp`.

**Memorable moment:** the whole city is lit and wired; the single monolithic
tower at the vanishing point is unlit, sealed, with one scarlet lance of light
above it. That tower is your machine. The mechanism is legible as an image
before a word is read.

**How this world fuses despite the glow.** Neon lives only in the photographic
imagery. Every region carrying data is a flat, matte, near-black plate with
crisp type and at most a scarlet hairline. There is no bloom behind any text.
This is the rule that makes a spec table possible in a neon world, and breaking
it anywhere collapses the direction into the near-black-plus-neon rut.

## Design system read from the approved comp

**Corner language is split by material, and this is load-bearing.** Painted or
printed things are square: 0 radius on buttons, bands, strips, table cells,
dividers. Physical objects are rounded: the F2 plate carries ~10px radius, a
lighter inner bevel line, and eight domed rivets (four per long edge). Radius
means "this is an object", so a rounded button or a chamfered table would be
wrong in both directions.

**Line weights.** 1px throughout. Scarlet `#E6202C` hairlines bound major
bands top and bottom. Column and cell dividers are 1px grey `#252A33`, inset
from the band's edges rather than spanning it. The selected mode's underline is
the only 2px rule on the page.

**Elevation.** Flat everywhere except the F2 plate, which has a soft outer
shadow and an inner bevel. No card shadows, no glass, no gradient panels.

**Two button variants, both square.** Solid scarlet with off-white mono label
for the primary; 1px scarlet outline on transparent with scarlet mono label for
the top bar's secondary.

**Signature device.** Large outlined display numerals (`01`–`04`) set beside
solid display labels on a shared baseline. Outline is stroke-only in grey.

**Type ramp.** Two faces. A strongly condensed display grotesque carries the
hero headline, section headings ~54px, column labels ~30px, mode labels ~22px,
outlined numerals ~44px, the F2 glyph ~54px. A monospace carries everything
else: sub-legend ~14px, captions ~12px, top-bar micro ~11.5px, scarlet section
labels ~11px at 0.18em tracking. Azeret Mono already in the project is retained.
The display face is used for UI labels too, not only headlines; mono is for
data, captions and legends.

**Display face: League Gothic — verified against the comp, not assumed.** The
comp's headline was measured by isolating each line's ink box and column-profiling
to separate type from the signage behind it. All three lines agree at **0.266–0.278
advance per cap height**, so the comp's face is coherent rather than a rendering
accident. That is 59% of Anton's width — no obtainable face reaches it natively.
It is also _moderate_ weight, not the ~900 first assumed: its ink coverage is
~52% of the headline box, where Anton compressed into the same box sits at 59%
and Anybody at wdth 50 / wght 900 at 78%.

Five candidates were rendered at matched cap height and compared. League Gothic
at `letter-spacing: -0.03em` with `scaleX(0.703)` reproduces the comp at **IoU
0.740, RMSE 0.363, ink 51.1% against the comp's 52%**. Anton needed a far harsher
`scaleX(0.517)` and still only reached IoU 0.623, with word gaps visibly wider
than the comp. Six Caps (42% ink) and Pathway Gothic One (34%) are too light;
Anybody is far too heavy.

So the display face is compressed deliberately, and the compression is a recorded
token rather than an ad-hoc transform. Compress hardest at display sizes and ease
off toward the small UI labels, the way a real family's display cut is narrower
than its text cut. Do not apply 0.703 to an 11px label.

**Measured headline geometry** (comp is 1536x960): cap height **10.5% of viewport
height**; font-size = cap ÷ 0.75, League Gothic's cap/em ratio. Line top-to-top is
**1.10 × cap** ⇒ `line-height: 0.825em`. Left edge at **2.86% of width**; the block
is **31.5% of width** at its widest line and spans **15.2%–50.5% of height**.

## Ingredient inventory and implementation medium

| Region                            | Medium                            | Note                                                                                                                                                                                                                                                                                                                                                                                 |
| --------------------------------- | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Abstract spectrogram, hero        | **generated raster**              | Not photographic and not a depiction: a field of frequency bins, densest at the right, dissolving to black at the left, over a solid low-frequency rail. Regenerate at asset resolution; never upscale the comp.                                                                                                                                                                     |
| The void column                   | part of the same raster           | It must sit at the centre of the same raster; separating it invites misregistration.                                                                                                                                                                                                                                                                                                 |
| Scarlet lance in the void column  | **CSS/SVG + motion**              | Drawn, not baked, because it animates and must align to the raster's void column.                                                                                                                                                                                                                                                                                                    |
| Taillight streaks                 | **baked into the raster**         | Reversed from "canvas or CSS" during production. A motion-blurred taillight on wet asphalt is a photographic material — it carries its own reflection, falloff and grain, and a CSS streak reads as a coloured rectangle beside the real ones. The page's motion budget is spent instead on the scarlet lance, sign flicker and rain, which are all things CSS renders convincingly. |
| Sign flicker in the imagery       | **CSS over raster**               | Bounded, few elements, reduced-motion off.                                                                                                                                                                                                                                                                                                                                           |
| Wordmark + scarlet mark           | **SVG**                           | Existing `shadoword-mark-trim.png` is a raster luminance mask; redraw as SVG in scarlet.                                                                                                                                                                                                                                                                                             |
| F2 hazard plate                   | **semantic HTML/CSS**             | Rounded plate, bevel, 8 rivets, chevron strip. Real control-adjacent content, must stay live text.                                                                                                                                                                                                                                                                                   |
| Hazard chevron strip              | **CSS repeating-linear-gradient** | Flat shape system, no depth.                                                                                                                                                                                                                                                                                                                                                         |
| **Primary CTA "Read the source"** | **semantic HTML/CSS**             | Solid scarlet square slab, mono label. The page's most important element; stays a real link, never baked into the raster.                                                                                                                                                                                                                                                            |
| Mode strip (Off/Local/Remote)     | **semantic HTML/CSS**             | Real interactive control with real detent copy. Never rasterized.                                                                                                                                                                                                                                                                                                                    |
| Signal path 01–04                 | **semantic HTML/CSS**             | Outlined numerals via `-webkit-text-stroke`.                                                                                                                                                                                                                                                                                                                                         |
| Spec tables, endpoints, protocol  | **semantic HTML**                 | Flat matte plates, must clear 4.5:1.                                                                                                                                                                                                                                                                                                                                                 |
| Unmeasured-latency block          | **semantic HTML**                 | States the absence; names corpus and harness.                                                                                                                                                                                                                                                                                                                                        |

**Quantity commitments.** The hero raster is dense: hundreds of fine vertical
frequency bins burning in magenta and cyan, densest at the right and dissolving
to black at the left, over a solid rail of low-frequency light along the bottom
~25% of the hero. A version rebuilt at a tenth of that density passes every
checklist and is not the design.
The hero occupies ~68% of the first viewport; the headline block spans **31.5%**
of its width (measured off the comp, correcting the earlier ~40% estimate).

**The delivered hero raster is `public/images/hero-spectrum.webp`** — 1536x617,
WebP q78, 197 KB, composed at the band's 2.489:1. It replaces the cityscape plate
`hero-canyon.webp`, now deleted: the world was translated from a neon night city
photograph with one unlit tower in it to an abstract spectrogram with one
perfectly black column in it. Same argument, same palette, same rules, no
photography. The superseded plate's provenance, crop parameters and winning
prompt stay recorded in `.impeccable/refs/hero-plate-full.md`, because the
v1→v2→v3 lineage is still why legibility is stated in frame coordinates rather
than as an instruction to be tasteful.

Verified against the load-bearing requirements: the void column is the only
region of the raster that emits nothing and it carries no baked lance; its
contiguously black columns sit at **x 738–787 (centre 49.64%)** and it runs
contiguously black from the top edge down to **row 299 of 617 — 48.5% of the
band** — where the spectrum closes over it. Headline-zone mean luminance, sampled
at three points: **0.1%, 1.4%, 6.2%**, which is why off-white type sits directly
on the raster there with no scrim.

**The plate must be composed at the band's aspect, 2.489:1.** v1 was generated
at 1.99:1 and cover-cropping drifted its vanishing point out from under the
CSS lance. **And legibility has to be stated in frame coordinates, not as an
instruction to be tasteful:** v2 left the left wall unlit _and_ swept bright
taillight streaks through the sub-legend's line, at 5.3% above threshold. The v3
prompt names the protected rectangle — left 45%, 40%–90% of frame height, nothing
above a quarter of peak brightness, "an empty black rectangle there is a failure"
— and confines bright streaks to the right half. **Regenerating the plate obliges
re-measuring 49.64% and 48.5%**, since the lance is registered to them in CSS.

**Compositional commitments.** Top bar ~4% height with wordmark left and two
mono legends plus an outlined CTA right, on a scarlet hairline. Hero ~68%,
symmetrical one-point perspective, headline over the left third, primary CTA
below it, F2 plate lower right. Mode strip ~7%, three equal cells, flat, scarlet
hairline above. Signal path second fold, left heading block plus four
rule-divided columns.

## Where the build overruled the comp, and why

Three times, and each time the comp lost to something the comp was not drawn
against. Recorded so they are not "fixed" back later.

1. **The wordmark.** The comp invented an angular S. The product already has a
   waveform-S icon, and a landing page does not get to issue the product a second
   identity. The real mark is inlined as SVG in scarlet.
2. **The sub-legend's measure.** The comp sets it on one line, which is 638px and
   runs 158px past the edge of the plate's dark zone — measured at 2.17% of pixels
   above 40% luminance under "and the silicon" against 0% under the rest of the
   line. It is `max-w-[40ch]`, chosen so the wrap lands on the sentence boundary:
   two claims, two lines.
3. **The build-from-source caveat.** The comp sets it beside the button. Its right
   edge is a fixed ~547px from the content's left edge, so the narrower the
   viewport the further right that pixel maps into the raster — at 1280 it reaches
   raster x 681 at 97% of band height, the brightest wet asphalt in the frame.
   Stacked above the button it never leaves the unlit left wall at any width, and
   the caveat now precedes the click, which is the better order anyway.

**The comp's 15.6% top pin is also gone**, at every width. The block does not
scale with the band — the headline is sized in vw but the legend, caveat and
button are in rem — so it runs 83% of band height at 1536 and 85% at 1440, and a
15.6% top puts its foot 4px through the bottom edge at 1440 and 23px at 1280. The
pin survives only above ~1500px. The block is vertically centred instead, which
costs 43px of upward shift at 1536 and is stable everywhere.

## Motion: what it is allowed to be

The steer that produced this world objected to a simulated waveform and a fake
signal monitor because they "do nothing". So the first viewport's entire motion
budget is light: a 6s opacity breath on the scarlet lance, three `mix-blend-mode:
screen` radial lamps registered to real sign clusters in the raster (60.6%/4.6%,
74.1%/30.5%, 61.1%/30.5%, measured at 40%/25%/22% mean luminance), and two rain
layers at different scales and speeds. None of it makes a claim, reports a value
or carries data.

Lamp flicker uses `steps(1, end)` with deliberately uneven keyframes — hold,
stumble, fast double blink, hold — at 7s, 5.4s and 11s so the three periods never
align into a visible pulse. Measured lift over the raster: lamp B **+21%**, lamp C
**+11%**, lamp A **+2.6%** (that region is already at 0.46 luminance, where screen
blending saturates). The rain adds **+0.4%** to the ground mean and its peak
contribution is bounded at +0.055, so it cannot threaten any text contrast.

**Reduced motion is a rendering, not a freeze.** The global rule neutralises every
duration, which would leave the rain as a static diagonal hatch across the
photograph and the lamps stuck at their first keyframe. So the rain is removed
outright, the lamps are pinned to a steady 0.2, and the lance simply stops
breathing at full strength since it is composition rather than effect.

Grain is not applied over live display text. Not yet recorded elsewhere.

## Responsive: what reflows and what a narrow frame costs

- **Below lg the composition changes rather than shrinks.** The band is a
  `min-height` floor on a bottom-anchored flex column, not a fixed height: at
  390px the content is 40px taller than the 26rem floor, and with a fixed height
  plus `justify-content: flex-end` that surplus is pushed off the _top_ of an
  `overflow-hidden` band, which decapitates the headline.
- **The F2 plate joins the flow below lg.** It cannot stay absolute there — the
  flowed content already fills the band, so bottom-right lands on the caveat and
  top-right lands on the headline. Flowing it also finally puts the product's one
  concrete affordance on a phone, where `hidden sm:block` had dropped it.
- **The lg floor is 30rem.** The aspect term only overtakes it at ~1195px; below
  that the band would be 417px tall holding a 410px block, with no canyon left
  above or below the type.
- **Stacked dividers follow the stacking axis.** The shared `divide-inset` draws
  a left border only, so the three modes ran together as one block of prose at
  390px, and the four signal-path steps had three of them inset behind a hanging
  rule — reading as nesting rather than as a series. Both rule across the top
  when stacked. The selected mode's marker likewise runs down the cell's left
  edge when stacked, because an underline there sits on the divider it shares
  with the next cell.
- **The signal path splits two-column at xl, not lg.** At lg the heading takes a
  27rem column, leaving each of four steps 118px — narrow enough that "A commit
  is what triggers inference" sets three words to a line.
- **What a phone costs.** At 390 the tower is behind the headline and cannot be
  revealed without a 780px hero. The mobile frame therefore carries the night
  city and the type; the tower argument lands at sm and above.

## Copy that must not carry forward from the comp

The comp invented sub-copy and two items are false. `REMOTE — DISABLED BY
DEFAULT` is untrue: remote is a real mode pointing at a daemon the user hosts.
`03 COMMIT — YOU APPROVE` is untrue: a commit is a protocol event, not user
approval. The build uses the real `src/data/site.ts` strings, which are longer
than the comp's two-line mono blocks, so those blocks run to three lines. That
is translation, not recomposition.

## Band-level decisions settled during the build

- **Daemon band: two stacked full-width plates, not a two-column grid.** Eleven
  routes against six frames is a 190px height difference, which in two columns is
  either a ragged bottom edge or a stretched plate with a large hollow in it. The
  eleven routes run in two internal columns instead, which is also how a route
  table is read: down, then down again. The column divider is painted on the
  container rather than on the right column's items, because that column holds
  five against the left's six and a per-item border stops a row short, leaving the
  rule hanging in mid-plate.
- **Instruction plates: two columns, never four.** At four, the longest real
  command — `bun run tauri dev -- --features whisper-vulkan` — is clipped by its
  own plate at 1536px wide, and a build instruction the reader cannot finish
  reading is the worst thing that band could ship.
- **The hazard chevron strip stays horizontal at every width.** Rotated to a
  vertical 6px edge, the -45° chevrons stop touching and read as a dashed line,
  which is a different sign entirely.
- **Accent colour does scanning work, not decoration.** Mutating routes
  (`PUT`/`POST`) and received frames (`recv`) are set in scarlet _ink_, so a
  reader scanning for what the daemon will let a caller change finds it without
  reading a legend.
- **The photograph's two sign hues are named but kept out of `@theme`.**
  `--sign-magenta` and `--sign-cyan` live in `:root`, because a value in the theme
  block becomes a `text-*` and `bg-*` utility and the discipline of this world is
  that neon never reaches a word or a control.

## Corrections made after the finish review

An independent review of the finished build produced an ordered list of material
defects. All of them were real. Recorded here because several were failures of a
kind that will recur if the reasoning is not written down.

- **The model catalog contained two fabricated rows.** "Medium English" and
  "Small English", described as "English only", have never existed in
  `crates/shadoword-core/src/model_download.rs` — `medium` is a _multilingual_
  model there and `small` is "Balanced model for lower-memory systems". Turbo's
  size was also wrong (1.62 GiB against a real 1.51) and the sizes mixed decimal
  and binary bases, so the one column readers compare against itself was not
  internally comparable. Two invented rows out of four, on the one band whose
  entire job is proof, on a privacy product's landing page. All six real entries
  are now transcribed verbatim, converted to a single base, and the table gained
  a `Download flag` column so the `id` is presented as what it is: the argument
  `--download-model` takes, printed as a real command two bands below.
- **The rain ran the wrong way by 90°.** `repeating-linear-gradient(Ndeg, …)`
  orients the gradient _axis_, and the bands run perpendicular to it, so `12deg`
  produced five or six pale near-_horizontal_ scratches lying across the sky and
  the tower. Near-vertical rain needs a near-horizontal axis: `100deg` and
  `97deg` now give streaks about 10° off plumb, leaning the way the reflections
  in the plate already lean.
- **The F2 plate's rivets rendered as flat grey squares.** The cause is worth
  keeping: the first pass used _hard_ colour stops (`0 1.6px, 1.6px 2.4px`), and
  at a ~5px diameter a hard-stopped radial gradient has too few pixels for its
  own circumference to survive, so it aliases into a block. Two layers with soft
  ramps — a highlight offset up-left of centre through the rim colour to a dark
  edge, plus a seat shadow cast down-right — read as round and as lit. Offsets
  and gradient sizes are in px, not %, because each background tile is a fraction
  of an unknown plate width and a percentage offset would slide the highlight off
  the head as the viewport changes.
- **The chevron tape was scarlet where the comp and the world's material list
  both say off-white.** Two strips of the accent on decoration, against the
  build's own recorded rule that scarlet does scanning work only — and it cost
  the reader the ability to find the places where scarlet means something.
- **`pushToTalk.note` was never rendered.** `site.ts` states that the page says
  "default" because the shortcut is rebindable and the hold is switchable to a
  toggle; the plate asserted F2 flatly. The note now sits under a hairline inside
  the plate, as a footnote to the key rather than a fourth peer label.
- **The focus ring was `--color-scarlet` — the same value as the primary CTA's
  own fill and the nav CTA's own border.** Focusing the most important control on
  the page read as the slab quietly growing 2px. It is now off-white with page
  black flooded into the 2px offset gap by a `box-shadow`, so the ring always has
  a dark edge beneath it: light-on-dark over a plate, dark-on-light over a
  scarlet slab or a lit sign.
- **The mode cells were centred at `sm` and up, and the comment claimed the comp
  centred them.** It does not: it left-aligns box, label and detent and leaves
  the cell's right half empty. They were also the only centred block on a page
  whose every other heading, table cell and legend starts on a left margin. The
  selected mode's 2px rule now starts on the same left margin as the label above
  it instead of being centred under it.
- **Six scarlet kickers had turned a comp device into a page template.** One is
  authorised. `Signal path` kept it, because it is the only one naming something
  no heading names — the four numbered steps beside it. `Your machine, your
terms` and `Remote, if you want it` merely restated the headings under them.
  `No packages yet` was a _fact_ dressed as a label, already stated under the
  hero CTA and in the footer's Packages row. `Not yet measured` titles a notice
  inside a plate, so it became an off-white `display-label` like every other
  plate heading. `Type plate` was the worst: design-internal jargon that was also
  the footer's only `<h2>` and therefore its accessible name, so a screen reader
  announced a region called "type plate". It is now `Project status`.
- **The missing scarlet tick on `03 Commit`**, named in the comp's own sidecar
  prompt, is a 5px square of pigment — square because painted marks in this world
  are square, and riding the baseline rather than centred on it so it reads as a
  stamp beside the word and not a bullet belonging to it. It marks the fulcrum of
  the band's argument: Commit is the only one of the four steps that is a
  decision rather than a consequence, and the heading is true because of it.
- **The `Accelerator` plate stretched to match the six-row catalog beside it**,
  leaving ~150px of empty bordered box. An empty box reads as a table that failed
  to load, so it is `lg:self-start` and ends after CUDA.
- **`large-v3` broke after its hyphen at 390px.** A download argument that wraps
  mid-token is one a reader can copy wrongly, so the value — not the cell's prose
  — is `whitespace-nowrap`.

## The world below the fold

The finish review's largest finding: the world existed only in the first ~630px
of a ~3560px page. Below the mode strip there was no imagery, no reprised
reflection, no plate-edge or rivet language and no second depth event — six
identical bands of kicker, condensed heading and flat matte table. That is how a
distinctive world becomes an ordinary spec sheet by the second scroll, and it is
the direct cause of a page reading as stock even when every band is correct.

Two devices answer it, deliberately only two.

- **The daemon band reprises the wet street.** It re-enters _there_ because that
  band is about sending audio somewhere else, and the street is the distance the
  remote mode crosses. It is the same file the hero already loaded, bottom-pinned
  with `object-position: 50% 100%` so what shows is the raster's last rows — the
  wet road with the signs in it, never the tower, which belongs to the hero alone
  and would read as the page repeating itself. A second photographic moment for
  zero additional bytes. Fixed at `10rem` at every width: a reprise that grows
  with the window competes with the thing it is recalling.

  The fusion rule still governs it. A `mask-image` fade plus one flat scrim means
  the image is gone before the route tables, and the only text over photography
  is the heading's cap, sitting in the last quarter of the fade — the claim
  coming up out of the street, with nothing to read on top of a reflection. The
  band's lead-in paragraph stays secondary ink because `lg:items-end`
  bottom-aligns it against a heading whose own top is already past the fade, so
  it can only ever sit on flat night.

- **The footer's status block is the page's second physical plate.** This
  refines, rather than breaks, the "one rounded object" rule: physical objects are
  rounded, bevelled, shadowed and riveted, and there are exactly two — the
  shortcut plate at the page's head, which says what the machine does, and the
  status plate at its foot, which says what it is. That is where a real machine
  carries its plates, and it is the literal thing a type plate _is_: the rating
  label bolted to the back of a machine, stating what it is and refusing to
  flatter it. A third would make the material a texture. The pitch is a custom
  property (`--rivet-pitch`) rather than a hardcoded `25%`, because four screws
  holding a ~900px panel is not a fixing; the footer plate takes eight per edge
  so both read as the same fastener at two sizes.

## Unresolved

- No measured latency figure exists; the page states the absence.
- No real capture of the desktop client exists.
- `site` in `astro.config.mjs` is still the assumed `https://shadoword.dev`.
- **`@astrojs/react` is kept deliberately, not by neglect.** The page ships zero
  `<script>` tags and there is no island, so the build emits an unreferenced
  ~142 kB `client.*.js`; it costs a visitor nothing and the deploy a little. It
  stays because react-bits components are still a live possibility. If that is
  dropped, remove the integration and the four React dependencies together.
- react-bits pro registry is unavailable: `REACTBITS_LICENSE_KEY` is rendered to
  `~/.secrets.fish` but nothing sources that file, so it never reaches the
  environment.
- `brandkit-brainstorm.md` at the website root predates this world and has not
  been reviewed against it.

## Resolved since the first pass

- `og:image` is regenerated: `public/images/shadoword-og.jpg`, a 1200×532
  screenshot of the shipped hero rather than a hand-composed card, so it cannot
  drift from the page it advertises. Its bottom edge is row 531 of the shot — the
  scarlet band hairline under the hero — because on a page whose only structural
  device is that hairline, ending the card on one is composing it. Reaching the
  nominal 1.91:1 would mean slicing 100px into the mode strip, which is 11px type
  no feed renders legibly. JPEG because it is a photograph: the same crop is
  845 KB as PNG against 199 KB. `og:image:alt` now describes the shipped hero.
- `website/README.md` is rewritten for this world.
- `scripts/grain.mjs` is deleted. It generated the discarded housing grain tile,
  which no longer exists in `public/`.

- No measured latency figure exists; the page states the absence.
- No real capture of the desktop client exists.
- `site` in `astro.config.mjs` is still the assumed `https://shadoword.dev`.
- **`og:image` and `og:image:alt` in `BaseLayout.astro` still describe the
  discarded panel/grille world** — "a perforated steel face… signal monitor
  reading 'capped — no socket to open'" — and `public/images/shadoword-og.png` is
  still the old crop. Both need regenerating from the shipped hero.
- **`website/README.md` still documents the Bench Instrument world.**
- **`@astrojs/react` is still in `astro.config.mjs`** and the build emits an
  unreferenced 142.42 kB `client.*.js` chunk. The page ships zero `<script>`
  tags and there is no React island; the integration should probably go.
- react-bits pro registry is unavailable: `REACTBITS_LICENSE_KEY` is rendered to
  `~/.secrets.fish` but nothing sources that file, so it never reaches the
  environment.
