<script lang="ts">
	import { ClipboardCheck, Cpu, Mic2, Radio } from '@lucide/svelte';
	import type { DesktopAppState } from '$lib/app-state.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { inferencePoolSummary } from '$lib/inference-pool';

	/**
	 * The right rail is the signal path: where audio enters, where it is inferred,
	 * where the text lands. Three stages down a single vertical line, because that
	 * is the shape of the claim the product makes — and the middle stage is the only
	 * one that ever changes machine, so it is the only one the accent marks.
	 */
	let { app }: { app: DesktopAppState } = $props();
	let mode = $derived(app.settings?.mode ?? 'remote');
	let modelName = $derived.by(() => {
		const modelPath = app.overview?.runtime.model_path;
		return (
			app.overview?.models.find((model) => modelPath?.endsWith(model.filename))?.name ??
			'No model selected'
		);
	});
	let ready = $derived(app.activity === 'ready' && Boolean(app.overview));
	let median = $derived(app.lastResult ? `${app.lastResult.elapsed_ms} ms` : '—');
	let speech = $derived(
		app.history.length > 0
			? `${Math.round(app.history.reduce((sum, item) => sum + Number.parseFloat(item.duration), 0))}s`
			: '00:00'
	);
	let segments = $derived(app.captureLocked ? app.segmentCount : (app.history[0]?.segments ?? 0));
	let poolSummary = $derived(inferencePoolSummary(app.overview?.status.inference_pool));
	let delivery = $derived.by(() => {
		switch (app.settings?.paste_method) {
			case 'direct':
				return 'Type directly';
			case 'ctrl_v':
				return 'Paste · Ctrl+V';
			case 'ctrl_shift_v':
				return 'Paste · Ctrl+Shift+V';
			case 'shift_insert':
				return 'Paste · Shift+Insert';
			default:
				return 'Transcript surface';
		}
	});
</script>

<aside class="context-strip" aria-label="Current signal path">
	<div class="context-heading" aria-live="polite">
		<span class="mono-label">Signal path</span>
		<strong class="display-panel"
			>{app.recording
				? 'Capturing'
				: app.processing
					? 'Finalizing'
					: app.captureState === 'error'
						? 'Capture error'
						: ready
							? 'Ready'
							: 'Action required'}</strong
		>
		<StatusPill
			state={app.activity === 'busy' || app.captureLocked
				? 'loading'
				: ready
					? app.overview?.status.model_loaded
						? 'ready'
						: 'warning'
					: 'offline'}
			label={app.activity === 'busy'
				? 'Updating runtime'
				: app.recording
					? 'Recording'
					: app.processing
						? 'Finalizing'
						: ready
							? app.overview?.status.model_loaded
								? `${mode === 'local' ? 'Local' : 'Remote'} · ${poolSummary}`
								: 'Model loads on demand'
							: 'Runtime unavailable'}
		/>
	</div>

	{#if app.recording}
		<!-- The one place in this rail that lights up, and only while audio is
		     actually moving: the same lateral drift the site's spectrum carries,
		     because a spectrogram advances in time. Off-white rather than scarlet —
		     scarlet is already spent on the record slab in the same viewport. -->
		<div class="wire" aria-hidden="true"></div>
	{/if}

	<div class="path" class:live={app.recording}>
		<section>
			<div class="path-icon"><Mic2 size={15} strokeWidth={1.7} /></div>
			<div>
				<span class="mono-label">Input</span>
				<strong class="mono-caption">{app.settings?.input_device ?? 'System default'}</strong>
				<small class="mono-micro"
					>{app.recordingSampleRate || app.settings?.sample_rate || 0} Hz · mono</small
				>
			</div>
		</section>
		<section>
			<div class="path-icon marked">
				{#if mode === 'remote'}
					<Radio size={15} strokeWidth={1.7} />
				{:else}
					<Cpu size={15} strokeWidth={1.7} />
				{/if}
			</div>
			<div>
				<span class="mono-label">Inference</span>
				<strong class="mono-caption">{mode === 'remote' ? 'Remote API' : 'This machine'}</strong>
				<small class="mono-micro">{modelName} · {poolSummary}</small>
			</div>
		</section>
		<section>
			<div class="path-icon"><ClipboardCheck size={15} strokeWidth={1.7} /></div>
			<div>
				<span class="mono-label">Delivery</span>
				<strong class="mono-caption">{delivery}</strong>
				<small class="mono-micro"
					>{app.settings?.copy_to_clipboard ? 'Clipboard enabled' : 'Clipboard disabled'}</small
				>
			</div>
		</section>
	</div>

	<div class="session-readout">
		<span class="mono-label">Session</span>
		<dl>
			<div>
				<dt class="mono-micro">Segments</dt>
				<dd class="mono-caption">{segments}</dd>
			</div>
			<div>
				<dt class="mono-micro">Speech</dt>
				<dd class="mono-caption">{speech}</dd>
			</div>
			<div>
				<dt class="mono-micro">Last latency</dt>
				<dd class="mono-caption">{median}</dd>
			</div>
		</dl>
	</div>

	<p class="host-note mono-micro">
		{app.demo
			? 'Development demo · simulated runtime.'
			: `${mode === 'local' ? 'Local inference' : 'Remote networking'} and microphone capture run in the native Rust host.`}
	</p>
</aside>

<style>
	.context-strip {
		position: relative;
		display: flex;
		min-width: 0;
		flex-direction: column;
		border-left: 1px solid var(--line);
		background: var(--surface-1);
	}

	.context-heading {
		display: grid;
		gap: 0.5rem;
		border-bottom: 1px solid var(--line);
		padding: 1.25rem 1.15rem;
	}

	.context-heading strong {
		color: var(--ink);
		margin-right: calc((var(--squeeze-label) - 1) * 100%);
	}

	/* ---- The path ------------------------------------------------------- */

	.path {
		position: relative;
		display: grid;
		padding: 1.15rem 1.15rem 1.35rem;
	}

	/* One continuous line down the icon column joining the three stages, drawn on
	   the container so it cannot stop short of the last one. This replaces three
	   arrow glyphs: an arrow between each pair said "then", and the product's claim
	   is that this is one path, not three hops. */
	.path::before {
		content: '';
		position: absolute;
		top: 2.6rem;
		bottom: 3.4rem;
		left: calc(1.15rem + 0.9rem);
		width: 1px;
		background: var(--line-strong);
	}

	.path section {
		position: relative;
		display: grid;
		grid-template-columns: 1.8rem 1fr;
		align-items: start;
		gap: 0.85rem;
		padding: 0.55rem 0;
	}

	.path-icon {
		display: grid;
		width: 1.8rem;
		height: 1.8rem;
		place-items: center;
		border: 1px solid var(--line);
		background: var(--surface-0);
		color: var(--ink-muted);
	}

	/* The inference stage is the only one that can move to another machine, so it
	   is the only one marked. Scarlet hairline plus scarlet-lamp glyph: ink and
	   rule, never a fill — a filled square here would read as "running now", which
	   is the record slab's job. */
	.path-icon.marked {
		border-color: var(--scarlet);
		color: var(--scarlet-lamp);
	}

	.path section > div:last-child {
		display: grid;
		gap: 0.22rem;
		min-width: 0;
		padding-top: 0.1rem;
	}

	.path section strong {
		overflow: hidden;
		color: var(--ink);
		font-weight: 400;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.path section small {
		overflow: hidden;
		color: var(--ink-muted);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* While recording, the joining line becomes a moving dash. Nothing else in the
	   rail changes: the readout is the same readout, and one animated 1px rule is
	   enough to say the path is carrying something. */
	.path.live::before {
		background-image: repeating-linear-gradient(to bottom, var(--ink) 0 4px, transparent 4px 11px);
		background-color: var(--line);
		animation: path-travel 1.1s linear infinite;
	}

	.wire {
		position: absolute;
		inset: 0 0 auto;
		height: 1px;
		background-image: repeating-linear-gradient(
			90deg,
			transparent 0 9px,
			rgb(232 230 225 / 0.5) 9px 10px,
			transparent 10px 22px
		);
		animation: wire-drift 0.75s linear infinite;
	}

	/* ---- Session readout ------------------------------------------------ */

	.session-readout {
		margin: auto 1.15rem 0;
		border-top: 1px solid var(--line);
		padding-top: 1rem;
	}

	dl {
		display: grid;
		gap: 0.4rem;
		margin: 0.8rem 0 0;
	}

	dl div {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 1rem;
	}

	dt {
		color: var(--ink-muted);
	}

	dd {
		margin: 0;
		color: var(--ink);
		font-variant-numeric: tabular-nums;
	}

	.host-note {
		margin: 1.15rem;
		color: var(--ink-muted);
	}

	@keyframes path-travel {
		to {
			background-position-y: 11px;
		}
	}

	@keyframes wire-drift {
		to {
			background-position-x: 22px;
		}
	}

	@media (max-width: 1180px) {
		.context-strip {
			display: none;
		}
	}
</style>
