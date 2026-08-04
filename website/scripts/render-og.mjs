/**
 * Renders the share card by shooting the built `/og-card` page.
 *
 * The card is composed rather than cropped now, but it is still *shot* rather
 * than drawn in code, and that is the whole point: the only way the image can
 * disagree with the site is if the stylesheet, the traced mark or `site.ts`
 * changed — in which case the next run picks the change up. A hand-drawn canvas
 * card would need every token restated, and would drift the first time one of
 * them moved.
 *
 * It shoots `dist/`, not the dev server, because the dev server serves unbundled
 * CSS and a different font-loading order. The page under test has to be the page
 * that ships.
 *
 * Usage:  bun run og        (builds, then renders)
 * Chrome: set CHROME to override the binary.
 */

import { spawn } from 'node:child_process'
import { mkdtemp, rm, writeFile, stat } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const dist = join(root, 'dist')
const out = join(root, 'public', 'images', 'shadoword-og.jpg')

/** The card's frame. Must match `.card` in src/pages/og-card.astro. */
const WIDTH = 1200
const HEIGHT = 630
/** 82 keeps the raster's thousands of fine bright strokes intact at ~150 KB.
 *  This is the case PNG is worst at — the same frame is several times larger
 *  lossless, at no visible gain. */
const QUALITY = 82

const CHROME = process.env.CHROME ?? 'google-chrome-stable'

function fail(message) {
  console.error(`render-og: ${message}`)
  process.exit(1)
}

await stat(join(dist, 'og-card', 'index.html')).catch(() =>
  stat(join(dist, 'og-card.html')),
).catch(() => fail('dist/og-card not found. Run `bun run build` first, or use `bun run og`.'))

/* ---- Serve dist ------------------------------------------------------------
   Bun's own static server rather than `astro preview`, so the script owns the
   port and cannot collide with a preview the developer already has running. */
const server = Bun.serve({
  port: 0,
  async fetch(request) {
    const path = decodeURIComponent(new URL(request.url).pathname)
    for (const candidate of [path, `${path}.html`, join(path, 'index.html')]) {
      const file = Bun.file(join(dist, candidate))
      if (await file.exists()) return new Response(file)
    }
    return new Response('not found', { status: 404 })
  },
})
const base = `http://127.0.0.1:${server.port}`

/* ---- Chrome ---------------------------------------------------------------- */
const profile = await mkdtemp(join(tmpdir(), 'shadoword-og-'))
const debugPort = 9222 + Math.floor(Math.random() * 500)

const chrome = spawn(
  CHROME,
  [
    '--headless=new',
    `--remote-debugging-port=${debugPort}`,
    `--user-data-dir=${profile}`,
    // dpr is set over CDP as well, but a mismatched host scale factor still
    // changes text rasterisation, so it is pinned here too.
    '--force-device-scale-factor=1',
    '--hide-scrollbars',
    '--disable-gpu',
    '--no-first-run',
    '--no-default-browser-check',
    'about:blank',
  ],
  { stdio: 'ignore' },
)
chrome.on('error', (error) => fail(`could not start ${CHROME}: ${error.message}`))

async function pageSocketUrl() {
  const deadline = Date.now() + 20_000
  while (Date.now() < deadline) {
    try {
      const targets = await fetch(`http://127.0.0.1:${debugPort}/json/list`).then((r) => r.json())
      const page = targets.find((t) => t.type === 'page' && t.webSocketDebuggerUrl)
      if (page) return page.webSocketDebuggerUrl
    } catch {
      /* not listening yet */
    }
    await Bun.sleep(120)
  }
  fail('Chrome never exposed a page target on the debugging port.')
}

const socket = new WebSocket(await pageSocketUrl())
await new Promise((resolve, reject) => {
  socket.onopen = resolve
  socket.onerror = () => reject(new Error('devtools socket failed'))
})

let nextId = 0
const pending = new Map()
const events = new Map()

socket.onmessage = ({ data }) => {
  const message = JSON.parse(data)
  if (message.id !== undefined) {
    const entry = pending.get(message.id)
    pending.delete(message.id)
    if (!entry) return
    if (message.error) entry.reject(new Error(message.error.message))
    else entry.resolve(message.result)
    return
  }
  events.get(message.method)?.forEach((resolve) => resolve(message.params))
  events.delete(message.method)
}

const send = (method, params = {}) =>
  new Promise((resolve, reject) => {
    const id = ++nextId
    pending.set(id, { resolve, reject })
    socket.send(JSON.stringify({ id, method, params }))
  })

const once = (method) =>
  new Promise((resolve) => {
    if (!events.has(method)) events.set(method, [])
    events.get(method).push(resolve)
  })

/* ---- Shoot ----------------------------------------------------------------- */
await send('Page.enable')
await send('Emulation.setDeviceMetricsOverride', {
  width: WIDTH,
  height: HEIGHT,
  deviceScaleFactor: 1,
  mobile: false,
})

const loaded = once('Page.loadEventFired')
await send('Page.navigate', { url: `${base}/og-card` })
await loaded

// The card's LCP is a 200 KB WebP behind `object-fit: cover`, and the headline
// is a synthetically compressed display face. `load` fires before either is
// guaranteed painted, so both are awaited explicitly rather than slept on.
const ready = await send('Runtime.evaluate', {
  expression: `(async () => {
    await document.fonts.ready
    const images = [...document.images]
    await Promise.all(images.map((i) => (i.complete ? i.decode() : new Promise((r) => { i.onload = i.onerror = r }))))
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)))
    const card = document.querySelector('.card').getBoundingClientRect()
    return JSON.stringify({ w: card.width, h: card.height, fonts: document.fonts.size })
  })()`,
  awaitPromise: true,
  returnByValue: true,
})
const frame = JSON.parse(ready.result.value)
if (frame.w !== WIDTH || frame.h !== HEIGHT) {
  fail(`the card measured ${frame.w}x${frame.h}, not ${WIDTH}x${HEIGHT}.`)
}

const shot = await send('Page.captureScreenshot', {
  format: 'jpeg',
  quality: QUALITY,
  captureBeyondViewport: false,
})

await writeFile(out, Buffer.from(shot.data, 'base64'))
const { size } = await stat(out)

socket.close()
chrome.kill()
server.stop(true)
await rm(profile, { recursive: true, force: true })

console.log(`render-og: ${out.replace(`${root}/`, '')} — ${WIDTH}x${HEIGHT}, ${(size / 1024).toFixed(0)} KB`)
console.log('render-og: this writes public/, so run the build again before deploying.')
