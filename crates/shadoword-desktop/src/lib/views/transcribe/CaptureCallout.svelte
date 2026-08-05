<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { getTranscribeContext, modeLabel } from './context';

	const context = getTranscribeContext();

	function handleAction() {
		if (context.mode === 'open_router' && !context.app.settings?.openrouter_key_configured) {
			context.onOpenSettings();
		} else if (context.app.captureState === 'error') {
			context.app.clearError();
		} else {
			void context.app.refreshOverview();
		}
	}
</script>

<div
	class="state-callout"
	class:error={context.captureBlocked}
	role={context.captureBlocked ? 'alert' : 'status'}
>
	<strong class="mono-caption">
		{context.app.processing
			? `Finalizing ${context.transcriptionMode} transcription`
			: context.app.captureState === 'error'
				? 'The last capture failed'
				: `${modeLabel(context.mode)} transcription unavailable`}
	</strong>
	<span class="mono-micro">
		{context.app.processing
			? context.transcriptionMode === 'streaming'
				? 'Committing the final pause-separated segment and assembling the transcript.'
				: `The captured audio is being transcribed by ${modeLabel(context.mode)}.`
			: (context.app.error ??
				(context.mode === 'remote'
					? 'Check the endpoint and bearer token in Settings, then retry.'
					: context.mode === 'open_router'
						? 'Enter an OpenRouter API key in Execution and choose a transcription model.'
						: 'Select or download a local model, then refresh the runtime.'))}
	</span>
	{#if context.captureBlocked}
		<Button variant="outline" size="sm" onclick={handleAction}
			>{context.mode === 'open_router' && !context.app.settings?.openrouter_key_configured
				? 'Configure key'
				: context.app.captureState === 'error'
					? 'Dismiss error'
					: 'Try again'}</Button
		>
	{/if}
</div>

<style>
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

	@media (max-width: 860px) {
		.state-callout {
			position: static;
			max-width: none;
			margin: 0 1.25rem;
		}
	}
</style>
