---
version: 1
slug: "website-src-pages-index-astro"
primary_target: "website/src/pages/index.astro"
related_targets: ["website/src/layouts/BaseLayout.astro"]
---

Scope: the public marketing website's landing page. Visitor mode: Persuade.

Audience and job: a privacy-minded Linux developer, mid-task, evaluating whether
to spend an evening building this. They are skeptical of cloud dictation tools
and have seen many transcription projects this month.

Action: go to the source repository. This page sells the idea; the repo converts.
No signup, no waitlist, no download binaries (none are published yet).

Proof this page is allowed to use, all four requested by the user:
- the desktop client at work (real captures, user-supplied)
- real measured latency from bench_corpus runs, hardware and model stated
- real copyable commands and real daemon request/response shapes
- network silence dramatized rather than asserted

Constraints: pre-launch. No users, testimonials, logos, star counts, or adoption
claims exist and none may be invented. Distribution, licensing, pricing, and
macOS/Windows support are undecided and must not be implied. The Tauri desktop
client is mid-port, so unfinished capability may not be shown as shipped.

Chosen direction: Bench Instrument (seed e0dabeb8) — the page is the front panel
of a functional-era instrument, on a light warm-grey ground that deliberately
refuses both the near-black dev-tool page and the cream-editorial opposite.

Memorable moment: the OFF / LOCAL / REMOTE selector. Operating it changes the
page's state — in LOCAL the NET OUT meter is dead and its jack field capped; in
REMOTE the field lights and the meter lives. The visitor learns the product's
architecture by operating it rather than reading about it.

Unresolved: the real app captures and the final latency figures are placeholders
on the user's replacement list until supplied. React Bits Pro is licensed and
available but its key was not reachable this session; the panel components are
authored directly, which is the correct call for a committed form regardless.
