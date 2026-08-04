# hero-plate-full.png — superseded cityscape plate source

**This plate no longer ships.** The world was translated from a neon night city
photograph with one unlit tower in it to an abstract spectrogram with one
perfectly black column in it, and the band that ships is now
`public/images/hero-spectrum.webp` (1536x617, WebP q78, 197 KB), which is not
derived from this file. The crop it used to feed, `hero-canyon.webp`, is deleted.
Kept because the lineage below is still why legibility is stated in frame
coordinates rather than as an instruction to be tasteful.

The numbers the CSS lance registers to are now measured off the spectrogram, not
off this plate: void column x 738-787, centre **49.64%**; contiguously black rows
0-299 of 617, so **48.5%** of the band. The tower measurements below are history.

Generated 2026-08-03 with codex `image_gen` at 1536x1024. The band that shipped as
`public/images/hero-canyon.webp` was the crop `1536x617+0+340` of this file, encoded
WebP q86 (206 KB, RMSE 0.0169 against the PNG crop).

Measured off this plate, when it was the shipping source:

- tower columns x 736-796, so centre x = 766/1536 = **49.9%**
- tower top edge at y = 471, which is **21.2%** of the cropped band
- tower is 3.9% of the frame width

Third generation. v1 composed its content at 1.99:1 when the band is 2.49:1, so no
crop held both the lance's headroom and the road reflections. v2 fixed the framing
but left the left wall almost unlit and swept bright taillight streaks straight
through the sub-legend's line — 5.3% of the third headline line's ground was above
40% luminance, and the legend crossed a 73% streak. v3 states the protected region
in frame coordinates (left 45%, between 40% and 90% of frame height) and confines
bright streaks to the right half, which drops that figure to 0.09%.

## Prompt

```
A photographic-quality night cityscape used as a full-bleed WEBSITE HERO BACKGROUND PLATE. Landscape. This is a clean background image only: it contains NO text, NO lettering, NO logos, NO buttons, NO user-interface elements of any kind, and no watermark. Do not add a headline. Do not add captions.

FRAMING — CRITICAL. This plate will be cropped down to a WIDE CINEMATIC LETTERBOX BAND, taking the horizontal slice between 33% and 93% of the frame height. Everything that matters must sit inside that slice, and these exact vertical placements are the most important instruction in this prompt:

- The street's vanishing point sits at the horizontal centre and at 60% of the frame height.
- The dark tower's TOP EDGE sits at 46% of the frame height, and the tower is 5% of the frame width — a clearly readable vertical slab, not a distant sliver.
- The rain-wet foreground asphalt begins at 70% of the frame height and continues to the bottom edge.
- The top 30% of the frame and the bottom 7% will both be cropped away, so nothing important may sit in either.

COMPOSITION — a tight, symmetrical one-point perspective looking straight down a NARROW rain-wet city canyon at night. The canyon is genuinely narrow: at the bottom edge of the frame the open street occupies only the middle third of the width, and the two walls rise steeply on either side, each filling roughly a third of the frame's width. This is a slot between buildings, not an open plaza.

At the vanishing point stands ONE monolithic dark tower: a plain black slab, utterly unlit, no signage on it, no lit windows, sharply silhouetted against the faint cold haze of the sky behind it. It is unmistakably the only dark, sealed structure in the entire image, and it must read as deliberately separate from everything around it. Do not put any light source on this tower and do not put a beam, glow, or shaft of light above it.

BOTH canyon walls are packed densely with lit signage from the frame's left edge to its right edge: hundreds of tall vertical sign panels, stacked boxes, hanging banners, and glowing window grids, in saturated magenta-pink and electric cyan with a few scattered warm reds. Cables, ducts, air-conditioning units, fire escapes and utility clutter fill the gaps. Neither wall may be a bare or empty facade — the left wall carries just as many lit sign panels as the right wall.

IMPORTANT LEGIBILITY REQUIREMENT, and it overrides the density instruction wherever the two conflict. Large white type will later be placed over the left side of the crop, so:

- In the LEFT 45% of the image, between 40% and 90% of the frame height, there must be NO bright sign, NO bright reflection, NO bright taillight streak, and NO high-contrast detail of any kind. Nothing in that region may exceed roughly a quarter of the image's peak brightness.
- That region must nevertheless still be full of city, not empty: dim distant signage, faint lit window grids, dark wet reflections and silhouetted clutter, all rendered low and quiet — as though that wall is further from the camera and its lights are weaker. An empty black rectangle there is a failure.
- The RIGHT half of the image carries the dense, bright, saturated signage and the strongest wet reflections.

Include a few soft red taillight streaks, motion-blurred, low in the road. Bright taillight streaks are allowed only in the right half of the frame. Any streak that reaches into the left 45% must be dim, thin and low, well below the brightness of the signage.

Overall grade: deep rain-black, cold and moody, slightly desaturated in the shadows, fine photographic film grain throughout, no lens flare, no bloom blowout, no HDR glow halos. Not a painting or illustration — read as a photograph.
```
