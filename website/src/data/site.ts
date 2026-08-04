/**
 * Every fact in this file is traceable to the workspace source it names below,
 * and each is recorded in ../../../PRODUCT.md at the repository root. Nothing
 * here may be invented. Figures the project has not measured are marked
 * `PLACEHOLDER` and listed in README.md under "Before publishing".
 */

export const repoUrl = 'https://github.com/Fractal-Tess/shadoword'

export const hook = 'Speech to text that never leaves your machine.'

export const subLegend =
  'Linux first. Local Whisper by default. You choose the model and the silicon.'

/**
 * The three selector positions. These are the product's real runtime modes.
 *
 * `isDefault` marks Local, and it is a *product fact* rather than the strip's
 * selection: the desktop client boots into local inference. The strip renders it
 * as a flag that never moves, because the detent the reader can move must not be
 * the only thing on the band saying which mode is the real one — the detent copy
 * is present tense, so a reader who clicks `Off` would otherwise leave the page
 * asserting "the microphone is released" as its live state.
 */
export const modes = [
  {
    id: 'off',
    label: 'Off',
    isDefault: false,
    detent: 'No capture. The microphone is released and no model is resident.',
  },
  {
    id: 'local',
    label: 'Local',
    isDefault: true,
    detent:
      'Whisper runs on this machine. Audio reaches the model over a function call, never a socket.',
  },
  {
    id: 'remote',
    label: 'Remote',
    isDefault: false,
    detent: 'Audio is sent to a daemon you host, over an authenticated connection you configured.',
  },
] as const

/**
 * The default global shortcut and its default mode, from the desktop client's
 * settings defaults (`hotkey_shortcut: 'f2'`, `hotkey_mode: 'push_to_talk'` in
 * crates/shadoword-desktop/src/lib/app-state.svelte.ts). Both are configurable,
 * so the page says "default".
 */
export const pushToTalk = {
  key: 'F2',
  above: 'Push to talk',
  below: 'Hold to capture',
  note: 'Default shortcut. Rebindable, or switch to toggle.',
}

/**
 * The whole catalog, verbatim from `WHISPER_MODELS` in
 * crates/shadoword-core/src/model_download.rs. Every field is transcribed from
 * that array — the `id` is the value `--download-model` takes, the size is
 * `size_bytes` converted to a single base (2^30 for GiB, 2^20 for MiB, so the
 * column is internally comparable), and the note is the spec's own `description`.
 *
 * There are no English-only variants. Earlier drafts of this file listed
 * "Medium English" and "Small English" as "English only", which the workspace
 * has never shipped: `medium` is described there as a *multilingual* model and
 * `small` as "Balanced model for lower-memory systems". Two invented rows out of
 * four, on the one band whose entire job is proof. Nothing goes in this array
 * that is not in that Rust file.
 */
export const modelCatalog = [
  {
    id: 'turbo',
    name: 'Large v3 Turbo',
    size: '1.51 GiB',
    note: 'Recommended balance of speed and accuracy.',
    recommended: true,
  },
  {
    id: 'large-v3',
    name: 'Large v3',
    size: '2.88 GiB',
    note: 'Largest and most accurate catalog model.',
    recommended: false,
  },
  {
    id: 'medium',
    name: 'Medium',
    size: '1.43 GiB',
    note: 'High-accuracy multilingual model.',
    recommended: false,
  },
  {
    id: 'small',
    name: 'Small',
    size: '465 MiB',
    note: 'Balanced model for lower-memory systems.',
    recommended: false,
  },
  {
    id: 'base',
    name: 'Base',
    size: '141 MiB',
    note: 'Fast model with improved accuracy over Tiny.',
    recommended: false,
  },
  {
    id: 'tiny',
    name: 'Tiny',
    size: '74 MiB',
    note: 'Smallest and fastest catalog model.',
    recommended: false,
  },
]

/** Build-time accelerator features, from AGENTS.md. */
export const accelerators = [
  { id: 'cpu', label: 'CPU', feature: 'default', note: 'Plain cargo build. No feature flag.' },
  { id: 'vulkan', label: 'Vulkan', feature: 'whisper-vulkan', note: 'Opt-in cargo feature.' },
  { id: 'cuda', label: 'CUDA', feature: 'whisper-cuda', note: 'Opt-in cargo feature.' },
]

/**
 * Latency is a measured claim, so it is not written here until it is measured.
 * Fill `ms` from a real run of the bench corpus and delete `placeholder`.
 * Harness: crates/shadoword-model-whisper/tests/whisper_integration.rs
 * Corpus:  bench_corpus/clip_{10,15,20,30}s.wav
 */
export const latencyBench = {
  placeholder: true,
  hardware: 'PLACEHOLDER — state the CPU or GPU the run used',
  model: 'Large v3 Turbo (turbo)',
  rows: [
    { clip: '10 s', ms: null },
    { clip: '15 s', ms: null },
    { clip: '20 s', ms: null },
    { clip: '30 s', ms: null },
  ],
}

/** The signal path, in the product's own terminology. */
export const signalPath = [
  {
    n: '01',
    label: 'Capture',
    body: 'Microphone input is recorded and voice activity is detected, so silence never reaches a model.',
  },
  {
    n: '02',
    label: 'Segment',
    body: 'Audio accumulates into a segment. Duration, queue length, and idle time are all bounded.',
  },
  {
    n: '03',
    label: 'Commit',
    body: 'A commit is what triggers inference. Until one arrives the segment simply sits there.',
  },
  {
    n: '04',
    label: 'Deliver',
    body: 'The transcript arrives as text at your cursor, or as a payload to the process that asked for it.',
  },
]

/** Real endpoints, from the daemon router. */
export const endpoints = [
  { method: 'GET', path: '/health', note: 'Public.' },
  { method: 'GET', path: '/v1/status', note: '' },
  { method: 'GET', path: '/v1/overview', note: '' },
  { method: 'GET', path: '/v1/config', note: '' },
  { method: 'PUT', path: '/v1/config', note: 'Restricted DTO.' },
  { method: 'POST', path: '/v1/transcribe-wav', note: 'Raw WAV body, 64 MiB cap.' },
  { method: 'GET', path: '/v1/stream', note: 'WebSocket. Opus packets.' },
  { method: 'GET', path: '/v1/models', note: '' },
  { method: 'POST', path: '/v1/models/{id}/select', note: '' },
  { method: 'POST', path: '/v1/downloads', note: 'Explicit catalog jobs only.' },
  { method: 'GET', path: '/v1/downloads/{id}', note: '' },
]

/** Real commands, from AGENTS.md and the daemon README. */
export const instructionPlates = [
  {
    legend: 'Desktop — build and run',
    lines: ['cd crates/shadoword-desktop', 'bun install', 'bun run tauri dev'],
  },
  {
    legend: 'Desktop — with Vulkan',
    lines: ['nix develop', 'bun run tauri dev -- --features whisper-vulkan'],
  },
  {
    legend: 'Daemon — run locally',
    lines: [
      'cargo run -p shadoword-api',
      '# add --features whisper-cuda inside nix develop .#cuda',
    ],
  },
  {
    legend: 'Daemon — install a model first',
    lines: ['cargo run -p shadoword-api -- \\', '  --download-model turbo'],
  },
]

/**
 * The streaming protocol, frame for frame from `ServerMessage` and `ClientText`
 * in `crates/shadoword-api/src/stream.rs`. This is the specificity a competitor
 * cannot copy-paste.
 *
 * This is protocol *v2*, because v2 is what actually runs: the desktop client
 * opens with `protocol_version: 2` and only falls back to omitting the field
 * (`remote_stream.rs:277`). An earlier version of this array showed the v1
 * lockstep flow — `Start` with no version, no `Started`, no `Accepted` — which
 * was true of a protocol nothing negotiates by choice, under a plate promising
 * "the socket as shipped". The band's whole job is proof, so it shows the
 * handshake the shipped client performs.
 */
export const streamProtocol = [
  {
    dir: 'tx',
    frame: '{"type":"Start","sample_rate":48000,"channels":1,"protocol_version":2}',
    note: '',
  },
  { dir: 'rx', frame: 'Started', note: 'flow_id, credit' },
  { dir: 'tx', frame: '<binary>', note: 'raw Opus packets' },
  { dir: 'tx', frame: 'CommitSegment', note: 'segment_index required' },
  { dir: 'rx', frame: 'Accepted', note: 'outstanding, remaining_credit' },
  { dir: 'rx', frame: 'Partial', note: 'one per committed segment' },
  { dir: 'tx', frame: 'Finish', note: '' },
  { dir: 'rx', frame: 'Done', note: 'exactly once' },
]

/** Honest pre-launch status. Do not soften these. */
export const typePlate = [
  { k: 'Status', v: 'Pre-launch. Build from source.' },
  { k: 'Platform', v: 'Linux. Wayland and X11.' },
  { k: 'Desktop client', v: 'Tauri 2 port in progress toward full parity.' },
  { k: 'Packages', v: 'None published yet.' },
  { k: 'License', v: 'See LICENSE in the repository.' },
]
