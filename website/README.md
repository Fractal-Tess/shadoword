# Shadoword public website

A standalone Astro site for Shadoword's public-facing page. One route, one
argument: a developer arrives, believes the offline claim because the page proves
it with the workspace's own routes, flags and model sizes, and leaves for the
repository.

## Stack

- **Astro** for static generation. No client-side JavaScript ships: the only
  `<script>` in the built page is an inert `application/ld+json` block, nothing
  executes, and every interactive thing on the page is a real form control.
- **Tailwind v4** via `@tailwindcss/vite`. The token layer that decides the world
  lives in `src/styles/global.css`.
- **Bun** for dependencies and scripts.
- **Nginx** for production serving.

`@astrojs/react` is installed and configured but no island exists yet, so the
build emits an unreferenced ~142 kB client chunk. It is kept deliberately, for
react-bits components; if that plan is dropped, remove the integration and the
four React dependencies together.

Fonts are self-hosted through `@fontsource`. This is deliberate: a privacy
product's site does not phone a font CDN. Two faces and no third — League Gothic
carries display, Azeret Mono carries every value, caption and legend.

## Quick start (dev)

```bash
cd website
bun install
bun run dev
```

## Production build

```bash
cd website
bun run build
bun run preview
```

## Container (Astro + Nginx)

Follows Astro's static/NGINX recipe — <https://docs.astro.build/en/recipes/docker/> —
with the build stage on Bun, since this repo keeps no npm or pnpm lockfile.

```bash
docker build -t shadoword-website ./website
docker run --rm -p 8080:80 shadoword-website
# http://localhost:8080
```

Three things worth knowing before you change the Dockerfile:

- **Nginx listens on 80, not the recipe's 8080.** 8080 is the rootless
  convention and `nginx:alpine` runs as root, so it buys nothing here — and it
  cost a live 502: Dokploy generates its Traefik service against the image's
  port 80, so a container listening only on 8080 was healthy and unreachable at
  the same time. Change this only together with the proxy that fronts it.

- The build stage pins `oven/bun:1.3.10`. The floating `oven/bun:1.3` tag
  currently resolves to 1.3.14, which fails the build outright: it cannot extract
  the astro tarball.
- `bun.lock` must stay out of `.dockerignore`: the build installs with
  `--frozen-lockfile` and cannot do that without it.

`nginx/nginx.conf` is Astro's config plus three additions, each marked `ADDED` in
the file: `gzip_vary`, a permanent cache for the content-hashed `_astro/` assets,
and an expiring one for `public/` files whose names do not change.

`astro check`, `bun run format`, and `bun run lint` are configured in
`package.json`.

## Design

The world is a blazing spectrum with one silent channel in it. The hero is an
abstract spectrogram — not a photograph of anything — and one column at its
centre is perfectly black while everything around it burns. That column is the
product's claim rendered as an image: everywhere signal, and one channel that
never lights up. The scarlet lance runs down its length and stops on the row
where the spectrum closes over it.

The page's argument is carried by a split: **the imagery is lit, and everything
that carries data is a flat matte near-black plate with crisp type on it.** A
privacy tool whose numbers glow is a privacy tool asking to be taken on vibes.

The world was previously a neon night city with one unlit tower in it, and the
translation is deliberate rather than a change of subject: the tower and the void
column are the same argument, and the second one does not have to pretend to be a
photograph of a place that does not exist.

The direction contract is recorded as an HTML comment at the top of `<body>` in
`src/layouts/BaseLayout.astro` and survives into the production build. Read it
before changing the visual language. `DESIGN.md` holds the design system, and
`.impeccable/surfaces/src-pages-index-astro.md` holds the surface brief with the
measured fidelity inventory and every place the build overruled the comp.

### Rules the page holds itself to

- **No invented product facts.** Every model size, endpoint, command, flag and
  default in `src/data/site.ts` is traceable to the workspace source named in that
  file, and the whole set is recorded in `../PRODUCT.md` at the repository root.
  The shortcut is the real `f2` push-to-talk default from the desktop client's
  settings, not a plausible-looking one, and the page says "default" because it
  is one. This rule has already been broken once and caught: an earlier
  `modelCatalog` listed "Medium English" and "Small English", which the workspace
  has never shipped — two invented rows out of four, on the one band whose entire
  job is proof. Nothing goes in that array that is not in
  `crates/shadoword-core/src/model_download.rs`.
- **No unmeasured figures.** `latencyBench.placeholder` is `true`, so the page
  ships an explicit "Not yet measured" plate naming the corpus and the harness
  instead of a number nobody ran.
- **Neon never touches a word or a control.** Magenta `#FF2F7F` and cyan
  `#26D4EC` are real parts of this world and they exist only inside the imagery —
  the hero raster and the three flaring-bin glows over it. They are declared in a
  bare `:root` block rather than in `@theme` precisely so Tailwind cannot generate
  a `text-` or `bg-` utility from them.
- **Scarlet is split into pigment and ink, and it only does scanning work.**
  `--color-scarlet` (`#E6202C`) is 4.37:1 on the page ground, so it is legal as a
  fill, hairline or rule and illegal as type; `--color-scarlet-lamp` is the ink
  cut, at 5.62:1 on the page ground and 5.21:1 on a plate. And it is rationed by _meaning_: the band hairlines,
  the selected mode, the mutating routes, the one section kicker, the tick on
  `03 Commit`, and the primary CTA. Not the chevron tape, which is off-white, and
  not a kicker on every band — six bands of scarlet label is scarlet meaning
  nothing on any of them.
- **White is the only ink legal on a scarlet fill.** The off-white gives 3.65:1
  there and fails; `#FFFFFF` gives 4.56:1. The CTA is the most important element
  on the page, so it does not get to be the one thing that fails.
- **Secondary ink is a token, never an opacity.** `--color-ink-soft` is 4.64:1 on
  a plate and 5.01:1 on the page ground. An `/50` of the primary off-white lands
  near 4:1, which is how running prose quietly stops being readable. No such
  utility exists in the built stylesheet — and it is checkable, because
  `global.css` scopes Tailwind's source detection to `src/` with
  `source(none)` + `@source '..'`. Under the default whole-project scan Tailwind
  read this very paragraph, found the class name it forbids, and generated it.
  Markup is the only thing that decides what CSS exists.
- **The focus ring is not scarlet.** Scarlet is the primary CTA's own fill and
  the nav CTA's own border, so a scarlet ring on either read as the slab growing
  2px rather than as focus. It is off-white with a page-black spacer flooded into
  the offset gap, so it has a dark edge beneath it on any ground — plate, scarlet
  slab, or a hot bin in the hero raster.
- **Radius means "this is a physical object".** Painted and printed things are
  square: every plate, table, notice and control carries 0. Radius plus bevel plus
  shadow plus rivets belongs to exactly two elements — the shortcut plate at the
  page's head, which says what the machine does, and the status plate at its foot,
  which says what it is. That is where a real machine carries its plates, and a
  third would make the material a texture.
- **Motion is atmosphere, and it never asks to be watched.** Two drift layers and
  three flaring bins, all inside the imagery, all on deliberately uneven periods
  so they never align. The drift travels sideways because a spectrogram advances
  in time — the previous world's falling rain belonged to a wet street and nothing
  in this plate falls. Nothing on this page animates on scroll, and
  `prefers-reduced-motion` gets a still rendering rather than a frozen frame.
- **Type is compressed by a recorded token, not an ad-hoc transform.** No
  obtainable face reaches the comp's 0.266 advance-per-cap-height natively;
  League Gothic at `-0.03em` scaled to `--squeeze-display: 0.703` reproduces it at
  IoU 0.740. Compression eases off as size drops and stops entirely below ~18px,
  because a squeezed 11px legend is just a damaged 11px legend.
- **The world has to survive the fold.** Above the mode strip the page is a lit
  spectrum; below it, it was six consecutive flat matte bands, which is how a
  distinctive world becomes an ordinary spec sheet by the second scroll. The
  daemon band reprises the raster's low-frequency rail — the same file the hero
  already loaded, bottom-pinned, so it costs no additional bytes — and the
  footer's status block is the page's second physical plate. The void column is
  never reprised: it is the offline claim, and the daemon band is the one about
  sending audio to another machine.

## Before publishing

These are the only knowingly unfinished items.

- [ ] **Real latency figures.** Run the bench corpus
      (`bench_corpus/clip_{10,15,20,30}s.wav`) through
      `crates/shadoword-model-whisper/tests/whisper_integration.rs`, fill `ms` and
      `hardware` in `latencyBench`, and delete `placeholder`. The band already has
      the plate the figures belong in.
- [ ] **A vector mark.** `public/images/shadoword-mark-trim.png` is a raster
      luminance mask cropped to the mark's ink box; the page tints it with
      `mask-image`. An SVG would remove the raster dependency entirely.
- [ ] **Decide on React.** See the note under Stack.

`public/` ships verbatim, so nothing large belongs there that the page does not
reference. `dist/` is ~752 KB, most of it the hero raster and the share card. The
unreferenced brand rasters live in `brand/`, which is not served.

## The share card

`og:image` is composed at exactly 1200×630 by `src/pages/og-card.astro` and shot
from the built page by `bun run og`:

```bash
cd website
bun run og      # astro build, then render public/images/shadoword-og.jpg
bun run build   # again, so dist/ picks the new file up
```

It used to be a 1200×532 crop of a browser viewport pointed at the real hero.
That had one genuine virtue — a shot cannot drift from the site — and the
composed card keeps it, because the card page imports the same stylesheet, the
same traced `Wordmark` and the same `site.ts` strings the index does, and is
still shot rather than drawn. Change a token and the next run shows it. What the
crop cost was everything it decided by accident: the wordmark squashed into a
48px bar at the top edge, the F2 plate sliced off at the bottom, and the scarlet
CTA rendering as a button in an image nobody can press.

Things the card holds to:

- **The void column is the composition, not a background.** `cover` on the
  1200×558 image band scales the 1536×617 plate by 0.9044 and trims 94.65px from
  each side, which puts the column's measured centre at 49.57% of the card and
  leaves its black run at 48.5% of the height. Those are the lance's coordinates.
  Regenerate the hero plate, or change that band's size, and both are recomputed
  from the hero's own measurements — not nudged by eye.
- **No type size on the card is new.** At a 1200×630 viewport `display-hero`
  computes to 88.2px on its own, so the headline is the ramp's value for this
  frame rather than a number chosen for it. The wordmark is _placed_ larger with
  `transform: scale(1.7)`, not restyled larger, because `display-wordmark`'s
  0.34em tracking is calibrated to its own size.
- **The foot strip is opaque for a reason.** The raster carries a solid rail of
  low-frequency light along its bottom edge — the one region of the frame where
  type on the image is unreadable at any weight. The index solves the same
  problem in the same place with the mode strip.
- **The route is `noindex, nofollow`.** It ships in `dist/` because the renderer
  shoots the built page rather than a dev server, which is what makes the card
  identical to what deploys.

`scripts/render-og.mjs` drives `google-chrome-stable` over CDP against a Bun
static server on `dist/`; set `CHROME` to use a different binary. It waits on
`document.fonts.ready` and an explicit `decode()` of the raster, then asserts the
frame measured 1200×630 before shooting, so a layout regression fails the render
instead of silently shipping a cropped card.

## Files

- `src/pages/index.astro` — band order and the shape of the argument
- `src/layouts/BaseLayout.astro` — HTML envelope, meta, and the direction contract
- `src/styles/global.css` — the token layer that decides the world
- `src/data/site.ts` — every fact the page states, and where it came from
- `src/components/Hero.astro` — first viewport; owns the raster, the void column's
  measured registration, and the motion
- `src/components/ModeStrip.astro` — the three runtime modes, as a real radio group
- `src/components/Daemon.astro` — the daemon's routes and frames, verbatim
- `src/components/TypePlate.astro` — current release and platform status, on the second plate
- `src/pages/404.astro` — the only other route; exists because `error_page 404`
  serves it, and it is built from the index's own parts
- `nginx/nginx.conf` — Astro's recipe config, plus gzip_vary and two cache tiers
- `Dockerfile` — Bun build + Nginx runtime stages
