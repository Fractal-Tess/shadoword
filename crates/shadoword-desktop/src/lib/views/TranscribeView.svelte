<script lang="ts">
	import { Check, Cloud, Copy, Mic2, Radio, Square, Trash2 } from '@lucide/svelte';
	import type { DesktopAppState } from '$lib/app-state.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Textarea } from '$lib/components/ui/textarea';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import SurfaceHeader from '$lib/components/SurfaceHeader.svelte';

	/**
	 * The primary surface, and the one place in the app where the world is imagery
	 * rather than material.
	 *
	 * The stage is a spectrum with one silent channel in it — the site's whole
	 * argument, but here it is *live and true* rather than a picture of a claim. The
	 * void column is registered to the transcription target: in Local it runs the
	 * full height of the stage and carries the scarlet lance, because nothing on
	 * this machine's audio path goes anywhere. In Remote the spectrum closes over
	 * it, because it does. That is not decoration keyed to a toggle; it is the
	 * single fact this product sells, drawn as the state it actually is.
	 *
	 * The spectrum is generated rather than loaded. A baked raster cannot answer to
	 * mode or to whether audio is moving, and `background-size: cover` on a
	 * 1536x617 file inside a ~2:1 panel crops horizontally, which slides the void
	 * column off its measured 49.64% the moment the window resizes. Layered
	 * gradients hold the registration exactly, cost no bytes, and can drift.
	 */
	let { app, onOpenSettings = () => {} }: { app: DesktopAppState; onOpenSettings?: () => void } =
		$props();
	let copied = $state(false);
	let mode = $derived(app.settings?.mode ?? 'remote');
	let transcriptionMode = $derived(
		mode === 'open_router' ? 'batch' : (app.settings?.transcription_mode ?? 'batch')
	);
	let captureBlocked = $derived(
		app.activity === 'booting' ||
			app.activity === 'busy' ||
			!app.settings ||
			(mode === 'open_router'
				? !app.settings.openrouter_key_configured
				: app.activity === 'offline' || !app.overview) ||
			app.captureState === 'error'
	);
	let modelName = $derived.by(() => {
		if (mode === 'open_router') return app.settings?.openrouter_model ?? 'Unselected';
		const path = app.overview?.runtime.model_path;
		return (
			app.overview?.models.find((model) => path?.endsWith(model.filename))?.name ?? 'Unselected'
		);
	});
	let endpointHost = $derived(
		mode === 'open_router' ? 'openrouter.ai' : endpointLabel(app.settings?.remote_endpoint)
	);
	let surfaceTitle = $derived(
		app.recording
			? 'Listening now'
			: app.processing
				? 'Finishing your transcript'
				: app.captureState === 'error'
					? 'Capture needs attention'
					: captureBlocked
						? 'The signal path needs attention'
						: 'Ready when you are'
	);

	const setRecording = async () => {
		copied = false;
		if (app.recording) await app.stopRecording();
		else await app.startRecording();
	};

	const copyTranscript = async () => {
		if (app.transcript) {
			await navigator.clipboard?.writeText(app.transcript);
			copied = true;
		}
	};

	function modeLabel(value: typeof mode) {
		if (value === 'local') return 'Local';
		if (value === 'open_router') return 'OpenRouter';
		return 'Remote';
	}

	function endpointLabel(endpoint: string | undefined) {
		if (!endpoint) return 'Not configured';
		try {
			return new URL(endpoint).host;
		} catch {
			return endpoint;
		}
	}
</script>

<div class="transcribe-view">
	<SurfaceHeader
		kicker="Transcribe"
		title={surfaceTitle}
		description="Private local speech to text, your self-hosted API, or direct OpenRouter transcription."
	>
		{#snippet actions()}
			<StatusPill
				state={app.recording || app.processing
					? 'loading'
					: captureBlocked
						? 'offline'
						: mode === 'open_router' || app.overview?.status.model_loaded
							? 'ready'
							: 'warning'}
				label={app.recording
					? 'Recording'
					: app.processing
						? 'Finalizing'
						: captureBlocked
							? 'Action required'
							: mode === 'open_router'
								? 'Provider ready'
								: app.overview?.status.model_loaded
									? 'Model ready'
									: 'Loads on demand'}
			/>
		{/snippet}
	</SurfaceHeader>

	<section
		class:recording={app.recording}
		class:local={mode === 'local'}
		class:unavailable={captureBlocked}
		class="capture-stage"
		aria-labelledby="capture-title"
	>
		<!--
			The spectrum. Densest at the right and dissolving to black at the left,
			which is what leaves the controls sitting on flat night — this world's
			oldest rule is that nothing readable lands on a lit bin.
		-->
		<div class="spectrum" aria-hidden="true">
			<div class="bins"></div>
			<div class="bloom"></div>
			<div class="void"><span class="lance"></span></div>
		</div>

		<div class="stage-body">
			<fieldset class="target-switch">
				<legend class="mono-label">Transcription target</legend>
				<div>
					{#if app.settings?.local_runtime_available}
						<button
							class:active={mode === 'local'}
							type="button"
							disabled={app.captureLocked || app.activity === 'busy' || !app.settings}
							onclick={() => app.setMode('local')}
							aria-pressed={mode === 'local'}
						>
							<Mic2 size={16} strokeWidth={1.9} aria-hidden="true" />
							<span class="mono-micro">This machine</span>
						</button>
					{/if}
					<button
						class:active={mode === 'remote'}
						type="button"
						disabled={app.captureLocked || app.activity === 'busy' || !app.settings}
						onclick={() => app.setMode('remote')}
						aria-pressed={mode === 'remote'}
					>
						<Radio size={16} strokeWidth={1.9} aria-hidden="true" />
						<span class="mono-micro">Remote API</span>
					</button>
					<button
						class:active={mode === 'open_router'}
						type="button"
						disabled={app.captureLocked || app.activity === 'busy' || !app.settings}
						onclick={() => app.setMode('open_router')}
						aria-pressed={mode === 'open_router'}
					>
						<Cloud size={16} strokeWidth={1.9} aria-hidden="true" />
						<span class="mono-micro">OpenRouter</span>
					</button>
				</div>
			</fieldset>

			<div class="capture-core">
				<p id="capture-title" class="mono-label">
					{app.recording ? `${modeLabel(mode)} ${transcriptionMode}` : 'Capture'}
				</p>
				<!--
					A square scarlet slab, not a round mic bubble. Round is the generic
					voice-app affordance and this world's radius rule says round means
					"physical object" — the one riveted plate in this app is at the foot of
					the rail. A solid scarlet square with a white glyph is exactly the
					site's primary CTA, which is the correct inheritance: on both surfaces
					it is the one thing the accent is spent on.
				-->
				<button
					type="button"
					class="record-slab"
					class:active={app.recording}
					onclick={setRecording}
					aria-label={app.recording ? 'Stop recording' : 'Start recording'}
					aria-pressed={app.recording}
					disabled={!app.recording && (captureBlocked || app.processing)}
				>
					<span class="pulse" aria-hidden="true"></span>
					{#if app.recording}
						<Square size={27} fill="currentColor" strokeWidth={0} aria-hidden="true" />
					{:else}
						<Mic2 size={32} strokeWidth={1.8} aria-hidden="true" />
					{/if}
				</button>
				<strong class="display-panel">
					{app.recording
						? 'Stop to finish'
						: app.processing
							? 'Transcribing'
							: captureBlocked
								? 'Unavailable'
								: 'Start recording'}
				</strong>
				<span class="mono-micro"
					>{app.recording
						? `${app.recordingSampleRate} Hz · mono · ${app.segmentCount} segments`
						: `${transcriptionMode === 'streaming' ? 'Pause-separated streaming' : 'Batch capture'} · native microphone`}</span
				>
				{#if app.recording}
					<Button variant="ghost" size="sm" onclick={() => app.cancelRecording()}>Cancel</Button>
				{/if}
				<span class="live-announcement" aria-live="polite">{surfaceTitle}</span>
			</div>

			<!--
				The claim, stated in words beside the column that draws it. Only in Local:
				in Remote the sentence would be false, and the column has already closed.
			-->
			<p class="void-legend mono-micro" aria-hidden={mode !== 'local'}>
				{#if mode === 'local'}One channel never lights up. Audio stays on this machine.{/if}
			</p>
		</div>

		{#if captureBlocked || app.processing}
			<div
				class="state-callout"
				class:error={captureBlocked}
				role={captureBlocked ? 'alert' : 'status'}
			>
				<strong class="mono-caption">
					{app.processing
						? `Finalizing ${transcriptionMode} transcription`
						: app.captureState === 'error'
							? 'The last capture failed'
							: `${modeLabel(mode)} transcription unavailable`}
				</strong>
				<span class="mono-micro">
					{app.processing
						? transcriptionMode === 'streaming'
							? 'Committing the final pause-separated segment and assembling the transcript.'
							: `The captured audio is being transcribed by ${modeLabel(mode)}.`
						: (app.error ??
							(mode === 'remote'
								? 'Check the endpoint and bearer token in Settings, then retry.'
								: mode === 'open_router'
									? 'Enter an OpenRouter API key in Settings, choose a transcription model, and save.'
									: 'Select or download a local model, then refresh the runtime.'))}
				</span>
				{#if captureBlocked}
					<Button
						variant="outline"
						size="sm"
						onclick={() => {
							if (mode === 'open_router' && !app.settings?.openrouter_key_configured) {
								onOpenSettings();
							} else if (app.captureState === 'error') {
								app.clearError();
							} else {
								void app.refreshOverview();
							}
						}}
						>{mode === 'open_router' && !app.settings?.openrouter_key_configured
							? 'Configure key'
							: app.captureState === 'error'
								? 'Dismiss error'
								: 'Try again'}</Button
					>
				{/if}
			</div>
		{/if}

		<!-- Flat, opaque, and full width: the readout is data, so it is a matte strip
		     laid across the bottom of the imagery rather than a card floating on it. -->
		<div class="stage-readout" aria-live="polite">
			<div>
				<span class="mono-label">Target</span>
				<strong class="mono-caption">{mode === 'local' ? 'This machine' : endpointHost}</strong>
			</div>
			<div>
				<span class="mono-label">Model</span>
				<strong class="mono-caption">{modelName}</strong>
			</div>
			<div>
				<span class="mono-label">Delivery</span>
				<strong class="mono-caption"
					>{app.settings?.paste_method === 'direct'
						? 'Type directly'
						: app.settings?.copy_to_clipboard
							? 'Clipboard + surface'
							: 'Transcript surface'}</strong
				>
			</div>
		</div>
	</section>

	<section class="transcript-surface plate" aria-labelledby="transcript-title">
		<header>
			<h2 id="transcript-title" class="display-legend">Working text</h2>
			<div class="transcript-actions">
				<Button
					variant="ghost"
					size="sm"
					onclick={() => (app.transcript = '')}
					disabled={!app.transcript}
				>
					<Trash2 size={13} />
					Clear
				</Button>
				<Button variant="outline" size="sm" onclick={copyTranscript} disabled={!app.transcript}>
					{#if copied}<Check size={13} />{:else}<Copy size={13} />{/if}
					{copied ? 'Copied' : 'Copy'}
				</Button>
			</div>
		</header>
		<Textarea
			bind:value={app.transcript}
			aria-label="Transcript text"
			placeholder="Your transcript will appear here…"
			class="transcript-editor"
		/>
		<footer class="mono-micro">
			<span>{modeLabel(mode)} · {modelName}</span>
			<span>{app.history.length} session results</span>
			<span
				>{app.lastResult ? `${app.lastResult.elapsed_ms} ms inference` : 'No inference yet'}</span
			>
		</footer>
	</section>
</div>

<style>
	.transcribe-view {
		display: grid;
		grid-template-rows: auto minmax(18rem, 1fr) minmax(9.5rem, 0.56fr);
		gap: 0.85rem;
		height: 100%;
		min-height: 0;
	}

	.capture-stage {
		position: relative;
		display: grid;
		grid-template-rows: 1fr auto;
		min-height: 0;
		overflow: hidden;
		border: 1px solid var(--line);
		background: var(--surface-0);
		isolation: isolate;
	}

	/* ---- The spectrum ---------------------------------------------------- */

	.spectrum {
		position: absolute;
		inset: 0;
		z-index: -1;
		/* Dissolves to black across the left third, so everything readable in the stage
		   sits over flat night. The ramp has to be *finished* by the time it reaches
		   49.64%, not still climbing through it: the void column is a black notch, and
		   a black notch cut into a field that is only 55% lit is not a notch. So the
		   mask reaches full strength at 46% — just left of the column — and the column
		   lands in fully lit bins with fully lit bins on the far side of it. That
		   registration is the entire reason this panel exists. */
		mask-image: linear-gradient(
			to right,
			transparent 0%,
			rgb(0 0 0 / 0.04) 20%,
			rgb(0 0 0 / 0.42) 34%,
			rgb(0 0 0 / 1) 46%
		);
		/* Operate mode's tax on the imagery, and it is the difference between a
		   readout and a screensaver. At full strength the field read as a rainbow wash
		   filling the panel — the colour became the subject and the void column stopped
		   being findable inside it. Held down here, the stage reads as an instrument
		   with a signal on it, which is what the operator is looking at. */
		opacity: 0.66;
	}

	/* Three bar layers on deliberately coprime periods — 7px, 11px and 23px — so
	   the field never resolves into a visible repeat. A spectrogram is literally a
	   field of vertical bins, so vertical bars are not a stand-in for the imagery;
	   they are what the imagery is. */
	.bins {
		position: absolute;
		inset: 0 auto 0 0;
		/* Wider than the box it sits in, so the drift below has somewhere to travel
		   from without the right edge walking into view. */
		width: 160%;
		background-image:
			repeating-linear-gradient(
				90deg,
				transparent 0 5px,
				rgb(255 255 255 / 0.2) 5px 6px,
				transparent 6px 7px
			),
			repeating-linear-gradient(
				90deg,
				transparent 0 9px,
				rgb(255 255 255 / 0.1) 9px 10px,
				transparent 10px 11px
			),
			repeating-linear-gradient(
				90deg,
				transparent 0 21px,
				rgb(255 255 255 / 0.3) 21px 23px,
				transparent 23px 23px
			);
		/* The bins are brightest along a low-frequency rail at the foot and thin out
		   upward, which is what a real spectrogram of speech looks like and what keeps
		   the top of the stage quiet enough to put a heading on. The falloff is steeper
		   than a linear ramp on purpose: the upper two thirds of this panel is where
		   the target switch and the record slab live, and bins that survive up there
		   are bins competing with the controls. */
		mask-image: linear-gradient(
			to top,
			rgb(0 0 0 / 1) 0%,
			rgb(0 0 0 / 0.5) 24%,
			rgb(0 0 0 / 0.14) 56%,
			rgb(0 0 0 / 0) 86%
		);
	}

	/* Magenta and cyan, and they exist here and nowhere else. They are declared as
	   literals inside the imagery rather than as tokens precisely so no utility and
	   no other component can reach them: in this world neon never touches a word or
	   a control. `screen` so they light the bins already under them instead of
	   painting over them.

	   Three *flares*, not three washes. The first pass used ellipses 30% of the panel
	   wide, which is not what colour does in a spectrogram: energy arrives in
	   particular bins, so the neon has to be narrow and vertical — a lit column, not
	   a cloud. 4-6% wide and 55-70% tall, rooted at the low-frequency rail and dying
	   before the top edge. That reads as three loud partials in an otherwise cold
	   field, which is both truer and quieter than the wash it replaces. */
	.bloom {
		position: absolute;
		inset: 0;
		mix-blend-mode: screen;
		background-image:
			radial-gradient(5% 62% at 71% 96%, rgb(255 47 127 / 0.62) 0%, rgb(255 47 127 / 0) 100%),
			radial-gradient(4% 48% at 88% 96%, rgb(38 212 236 / 0.55) 0%, rgb(38 212 236 / 0) 100%),
			radial-gradient(6% 34% at 57% 100%, rgb(255 47 127 / 0.4) 0%, rgb(255 47 127 / 0) 100%),
			radial-gradient(3% 26% at 81% 100%, rgb(38 212 236 / 0.34) 0%, rgb(38 212 236 / 0) 100%);
	}

	/* ---- The void column ------------------------------------------------- */

	/* 49.64% and 3.26% wide are the site's measured registration — its raster's
	   contiguously black columns are x738-787 of 1536 — so the same mark lands in
	   the same place on both surfaces. */
	.void {
		position: absolute;
		top: 0;
		bottom: 0;
		left: 49.64%;
		width: 3.26%;
		background: var(--surface-0);
		/* Remote is the closed state: the column fills in from the top, leaving the
		   spectrum unbroken. Height rather than opacity, so the closing reads as the
		   spectrum growing over the channel rather than as a rectangle fading. */
		transform-origin: bottom;
		transition: transform 420ms cubic-bezier(0.16, 1, 0.3, 1);
		transform: scaleY(0);
	}

	.capture-stage.local .void {
		transform: scaleY(1);
	}

	/* The lance runs down the inside of the column and stops short of the foot,
	   where the spectrum's low-frequency rail closes over it — the same termination
	   the site's hero draws. Pigment is legal: a 1px rule is not type. */
	.lance {
		position: absolute;
		top: 0;
		left: 50%;
		width: 1px;
		height: 74%;
		background: var(--scarlet);
	}

	/* While audio is moving, the field drifts sideways. A spectrogram advances in
	   time; nothing in this plate falls. Two speeds on uneven periods so they never
	   align, and the whole thing is atmosphere — it never asks to be watched. */
	.capture-stage.recording .bins {
		animation: bin-drift 2.6s linear infinite;
	}

	.capture-stage.recording .bloom {
		animation: bloom-breathe 4.3s ease-in-out infinite;
	}

	.capture-stage.unavailable .spectrum {
		opacity: 0.4;
	}

	/* ---- Stage body ------------------------------------------------------ */

	.stage-body {
		position: relative;
		display: grid;
		align-content: start;
		gap: 1.6rem;
		padding: 1.15rem 1.25rem 1.5rem;
	}

	.target-switch {
		min-width: 0;
		margin: 0;
		border: 0;
		padding: 0;
	}

	.target-switch legend {
		margin-bottom: 0.5rem;
		padding: 0;
	}

	.target-switch > div {
		display: inline-flex;
		gap: 1px;
		background: var(--line);
	}

	.target-switch button {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		height: 2.25rem;
		border: 0;
		padding: 0 0.75rem;
		background: var(--surface-1);
		color: var(--ink-muted);
		cursor: pointer;
		transition:
			background-color 120ms linear,
			color 120ms linear;
	}

	.target-switch button:hover:not(:disabled) {
		background: var(--surface-2);
		color: var(--ink);
	}

	.target-switch button:disabled {
		cursor: not-allowed;
		opacity: 0.45;
	}

	/* The selected target is scarlet *ink over a raised ground* rather than a
	   scarlet fill. Fill is reserved for the record slab, and two scarlet slabs in
	   one stage is the accent telling the operator that two different things are
	   happening at once. */
	.target-switch button.active {
		background: var(--surface-2);
		color: var(--scarlet-lamp);
		box-shadow: inset 0 -2px 0 var(--scarlet);
	}

	/* ---- The record slab ------------------------------------------------- */

	.capture-core {
		display: grid;
		justify-items: start;
		max-width: 20rem;
	}

	.capture-core > p {
		margin: 0 0 0.85rem;
	}

	.record-slab {
		position: relative;
		display: grid;
		width: 5rem;
		height: 5rem;
		place-items: center;
		border: 0;
		background: var(--scarlet);
		color: var(--on-scarlet);
		cursor: pointer;
		transition:
			background-color 140ms linear,
			transform 140ms linear;
	}

	.record-slab:hover:not(:disabled) {
		background: var(--scarlet-deep);
	}

	.record-slab:active:not(:disabled) {
		transform: translateY(1px);
	}

	.record-slab:disabled {
		cursor: not-allowed;
		background: var(--surface-2);
		color: var(--ink-muted);
		box-shadow: inset 0 0 0 1px var(--line-strong);
	}

	/* Recording inverts the slab to ink and moves the scarlet outward into the
	   pulse. The loud fill invites; once it is running, the accent belongs to the
	   thing that is running, and the control's job is now "stop". */
	.record-slab.active {
		background: var(--ink);
		color: var(--surface-0);
	}

	.record-slab.active:hover {
		background: var(--on-scarlet);
	}

	.pulse {
		position: absolute;
		inset: 0;
		border: 1px solid transparent;
	}

	.record-slab.active .pulse {
		border-color: var(--scarlet);
		animation: slab-pulse 1.8s cubic-bezier(0.16, 1, 0.3, 1) infinite;
	}

	.capture-core strong {
		margin: 1.1rem 0 0;
		color: var(--ink);
		margin-right: calc((var(--squeeze-label) - 1) * 100%);
	}

	.capture-core > span {
		margin-top: 0.35rem;
		color: var(--ink-muted);
	}

	.capture-core :global(button) {
		margin-top: 0.7rem;
	}

	.live-announcement {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}

	/* The legend sits left of the column, on flat night, and its right edge stops
	   before 49.64% so no word ever overlaps the mark it is describing. */
	.void-legend {
		max-width: 15rem;
		margin: 0;
		color: var(--ink-dim);
		border-left: 1px solid var(--scarlet);
		padding-left: 0.7rem;
		min-height: 1.4rem;
	}

	.capture-stage:not(.local) .void-legend {
		border-left-color: transparent;
	}

	/* ---- Callout and readout --------------------------------------------- */

	.state-callout {
		position: absolute;
		top: 1.15rem;
		right: 1.25rem;
		display: grid;
		justify-items: start;
		gap: 0.4rem;
		max-width: 17rem;
		border: 1px solid var(--line-strong);
		padding: 0.75rem 0.85rem;
		background: var(--surface-1);
	}

	.state-callout.error {
		border-color: var(--scarlet);
		border-left-width: 2px;
	}

	.state-callout strong {
		color: var(--ink);
		font-weight: 400;
	}

	.state-callout span {
		color: var(--ink-muted);
	}

	.state-callout :global(button) {
		margin-top: 0.2rem;
	}

	.stage-readout {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 1px;
		border-top: 1px solid var(--line);
		background: var(--line);
	}

	.stage-readout > div {
		display: grid;
		gap: 0.28rem;
		min-width: 0;
		padding: 0.7rem 0.85rem;
		background: var(--surface-1);
	}

	.stage-readout strong {
		overflow: hidden;
		color: var(--ink);
		font-weight: 400;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* ---- Transcript ------------------------------------------------------ */

	.transcript-surface {
		display: grid;
		grid-template-rows: auto minmax(0, 1fr) auto;
		min-height: 0;
		overflow: hidden;
	}

	.transcript-surface header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		border-bottom: 1px solid var(--line);
		padding: 0.7rem 0.9rem;
	}

	.transcript-surface h2 {
		color: var(--ink);
	}

	.transcript-actions {
		display: flex;
		gap: 0.35rem;
	}

	.transcript-surface :global(.transcript-editor) {
		min-height: 0;
		resize: none;
		border: 0;
		border-radius: 0;
		padding: 1.05rem 0.9rem;
		background: var(--surface-0);
		color: var(--ink);
		font-family: var(--font-mono);
		font-size: 0.875rem;
		line-height: 1.7;
		box-shadow: none;
	}

	/* The empty state has to read as empty. At `--ink-dim` the placeholder was close
	   enough to real transcript text to be mistaken for one at a glance, which is the
	   worst thing an empty state can do on the surface whose whole output is text. */
	.transcript-surface :global(.transcript-editor)::placeholder {
		color: var(--ink-muted);
	}

	.transcript-surface footer {
		display: flex;
		gap: 1.15rem;
		border-top: 1px solid var(--line);
		padding: 0.6rem 0.9rem;
		color: var(--ink-muted);
	}

	@keyframes slab-pulse {
		0% {
			transform: scale(1);
			opacity: 0.95;
		}
		100% {
			transform: scale(1.36);
			opacity: 0;
		}
	}

	@keyframes bin-drift {
		from {
			transform: translate3d(0, 0, 0);
		}
		to {
			transform: translate3d(-23px, 0, 0);
		}
	}

	@keyframes bloom-breathe {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.62;
		}
	}

	@media (max-width: 860px) {
		/* The stage stops being a composition and becomes a stack: the spectrum
		   retreats to a band at the foot, above the readout, because at this width the
		   controls would sit on the lit half no matter how the mask is tuned. The void
		   column survives — it is the argument, not an ornament. */
		.capture-stage {
			min-height: 0;
		}

		.spectrum {
			inset: auto 0 3.4rem;
			height: 6rem;
			mask-image: linear-gradient(
				to right,
				transparent 0%,
				rgb(0 0 0 / 0.2) 26%,
				rgb(0 0 0 / 1) 62%
			);
		}

		.state-callout {
			position: static;
			max-width: none;
			margin: 0 1.25rem;
		}

		.stage-body {
			gap: 1.2rem;
			padding-bottom: 7.5rem;
		}

		.void-legend {
			max-width: none;
		}

		.stage-readout {
			grid-template-columns: 1fr;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.record-slab.active .pulse,
		.capture-stage.recording .bins,
		.capture-stage.recording .bloom {
			animation: none;
		}

		.void {
			transition: none;
		}
	}
</style>
