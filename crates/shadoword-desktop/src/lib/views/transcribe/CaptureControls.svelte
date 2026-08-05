<script lang="ts">
	import { Mic2, Square } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import { getTranscribeContext, modeLabel } from './context';

	const context = getTranscribeContext();

	const setRecording = async () => {
		context.setCopied(false);
		if (context.app.recording) await context.app.stopRecording();
		else await context.app.startRecording();
	};
</script>

<div class="stage-body">
	<div class="capture-core">
		<p id="capture-title" class="mono-label">
			{context.app.recording
				? `${modeLabel(context.mode)} ${context.transcriptionMode}`
				: 'Capture'}
		</p>
		<button
			type="button"
			class="record-slab"
			class:active={context.app.recording}
			onclick={setRecording}
			aria-label={context.app.recording ? 'Stop recording' : 'Start recording'}
			aria-pressed={context.app.recording}
			disabled={!context.app.recording && (context.captureBlocked || context.app.processing)}
		>
			<span class="pulse" aria-hidden="true"></span>
			{#if context.app.recording}
				<Square size={27} fill="currentColor" strokeWidth={0} aria-hidden="true" />
			{:else}
				<Mic2 size={32} strokeWidth={1.8} aria-hidden="true" />
			{/if}
		</button>
		<strong class="display-panel">
			{context.app.recording
				? 'Stop to finish'
				: context.app.processing
					? 'Transcribing'
					: context.captureBlocked
						? 'Unavailable'
						: 'Start recording'}
		</strong>
		<span class="mono-micro"
			>{context.app.recording
				? `${context.app.recordingSampleRate} Hz · mono · ${context.app.segmentCount} segments`
				: `${context.transcriptionMode === 'streaming' ? 'Pause-separated streaming' : 'Batch capture'} · native microphone`}</span
		>
		{#if context.app.recording}
			<Button variant="ghost" size="sm" onclick={() => context.app.cancelRecording()}>Cancel</Button
			>
		{/if}
		<span class="live-announcement" aria-live="polite">{context.surfaceTitle}</span>
	</div>

	<p
		class:local={context.mode === 'local'}
		class="void-legend mono-micro"
		aria-hidden={context.mode !== 'local'}
	>
		{#if context.mode === 'local'}One channel never lights up. Audio stays on this machine.{/if}
	</p>
</div>

<style>
	.stage-body {
		position: relative;
		display: grid;
		align-content: start;
		gap: 1.6rem;
		padding: 1.15rem 1.25rem 1.5rem;
	}

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
		margin-right: calc((var(--squeeze-label) - 1) * 100%);
		color: var(--ink);
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

	.void-legend {
		max-width: 15rem;
		min-height: 1.4rem;
		margin: 0;
		border-left: 1px solid transparent;
		padding-left: 0.7rem;
		color: var(--ink-dim);
	}

	.void-legend.local {
		border-left-color: var(--scarlet);
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

	@media (max-width: 860px) {
		.stage-body {
			gap: 1.2rem;
			padding-bottom: 7.5rem;
		}

		.void-legend {
			max-width: none;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.record-slab.active .pulse {
			animation: none;
		}
	}
</style>
