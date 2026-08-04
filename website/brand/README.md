# Brand source rasters

Not served. These are the large source images the site was drawn from; they live
here rather than in `public/` because nothing on the page references them and
`public/` ships verbatim into `dist/`. Between them they were 3.0 MB of a 3.7 MB
build, on a page whose whole argument is restraint.

- `shadoword-hero.png` — unused.
- `shadoword-mark-square.png` — was the `og:image`. Replaced by
  `public/images/shadoword-og.png`, a real 1200×630 crop of the front panel,
  which is what `summary_large_image` actually wants.

The mark the page _does_ use is `public/images/shadoword-mark-trim.png`: a 13 KB
luminance mask cropped to the mark's ink box, tinted at render time with
`mask-image`, so one file serves every ground.
