# PRODUCT.md

Durable product truth for Shadoword. Design work reads this before it invents
anything. Every claim here is traceable to the workspace; where a fact does not
exist yet it is recorded as absent rather than filled in.

## What it is

Shadoword is Linux-first speech to text for developers. It turns held-key
speech into text at the cursor, running Whisper inference on the user's own
machine by default, with explicit Shadoword API and OpenRouter alternatives.

It ships in two shapes over one core:

- **Desktop client** (`crates/shadoword-desktop`) — Tauri 2 + SvelteKit +
  shadcn-svelte. The primary product. Mid-port toward full native parity.
- **API daemon** (`crates/shadoword-api`) — an optional authenticated HTTP and
  WebSocket service the user self-hosts, so a strong workstation can serve a
  thin laptop.

`shadoword-core` owns capture, WAV/VAD, config, model downloads, and
transcription orchestration. `shadoword-shared` owns the model contract;
`shadoword-model-whisper` implements it. Backends stay behind that contract.

## Who it is for

A privacy-minded Linux developer, desktop first. They are skeptical of cloud
dictation, comfortable with `cargo` and a Nix shell, and willing to build from
source if the trade is control. They have seen many transcription projects and
assume the offline claim is marketing until shown otherwise.

Secondary: the same person self-hosting the daemon for their own machines. Not a
team, not a tenant — one person, several computers.

## Positioning

Four claims, in priority order. All four are load-bearing; none may be softened.

1. **Local and offline by default.** In local mode audio reaches the model over
   a function call and no socket is opened. The user can explicitly select a
   self-hosted Shadoword API or direct OpenRouter batch transcription instead.
2. **Linux first.** Wayland and X11. Not "Linux also supported."
3. **Hardware control is explicit.** The user picks the model and the
   accelerator. Nothing is chosen silently on their behalf.
4. **Same stack, two shapes.** Desktop and daemon share one core, so behaviour
   does not diverge between them.

## Product truth a design may state

- **Execution targets:** Local (default), Shadoword API, OpenRouter.
- **Push-to-talk default:** `f2` (from `shadoword-core` config).
- **Model catalog:** Large v3 Turbo (1.62 GiB, default, multilingual), Large v3
  (2.88 GiB), Medium English (1.42 GiB), Small English (466 MiB).
- **Accelerators:** CPU is the plain `cargo build` default. Vulkan and CUDA are
  opt-in cargo features (`whisper-vulkan`, `whisper-cuda`), both inside
  `nix develop`.
- **Inference trigger:** a commit. `CommitSegment` on the stream, or a completed
  batch body. Nothing is transcribed speculatively or in the background.
- **Streaming protocol:** WebSocket at `/v1/stream`; `Start` frame, raw Opus
  packets, `CommitSegment` → `Partial`, `Finish` → `Done` exactly once.
- **Batch:** `POST /v1/transcribe-wav`, raw WAV body, 64 MiB cap.
- **Distribution:** GitHub Releases publishes Linux API, desktop-client, and
  embedded CPU desktop archives; Nix source packages remain available.
- **Toolchain:** Bun only for frontend dependencies and scripts. No npm or pnpm
  lockfiles.

## Facts that do not exist yet

Design must not supply these. Where a surface needs one, it states the absence.

- **Measured latency.** No figure has been run on stated hardware. The bench
  harness is `crates/shadoword-model-whisper/tests/whisper_integration.rs`
  against `bench_corpus/clip_{10,15,20,30}s.wav`, and it is opt-in, not part of
  the normal test run.
- **macOS and Windows support.** Undecided.
- **Licensing and pricing.** See `LICENSE` in the repository; no commercial
  model is decided.
- **Users, testimonials, logos, adoption or star counts.** None exist.

## Voice

Flat, specific, unhurried. The voice of a manual, not a launch post. Name the
mechanism instead of praising it: "inference fires on a commit" rather than
"blazing fast." Never claim a number nobody measured. Never use the word
"seamless."

## Surfaces

- `website/` — the public landing page. Persuade mode; the action is going to
  the source repository at `github.com/Fractal-Tess/shadoword`. Its own design
  system is recorded in `website/DESIGN.md`.
- `crates/shadoword-desktop` — the application UI. Operate mode. Carries its own
  separate direction; not governed by the website's visual world.
