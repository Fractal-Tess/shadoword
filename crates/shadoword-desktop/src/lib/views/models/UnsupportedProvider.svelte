<script lang="ts">
	import { Cloud } from '@lucide/svelte';
	import { Badge } from '$lib/components/ui/badge';
	import { getModelsContext } from './context';

	const context = getModelsContext();
</script>

<section class="unsupported-provider" aria-labelledby="unsupported-models-title">
	<div class="provider-icon" aria-hidden="true"><Cloud size={20} /></div>
	<div>
		<span class="mono-label">Hosted provider</span>
		<h2 id="unsupported-models-title" class="display-legend">OpenRouter model management</h2>
		<p>
			OpenRouter does not expose a Shadoword Whisper runtime, verified model downloads, custom GGML
			paths, or accelerator controls. Choose the hosted transcription model in Execution settings
			instead.
		</p>
		{#if context.app.settings?.openrouter_model}
			<small>Current model · {context.app.settings.openrouter_model}</small>
		{/if}
	</div>
	<Badge variant="outline">Unsupported here</Badge>
</section>

<style>
	.unsupported-provider {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: start;
		gap: 0.9rem;
		border: 1px solid var(--line);
		border-left: 2px solid var(--scarlet);
		padding: 1rem;
		background: var(--surface-1);
		color: var(--ink-muted);
	}

	.provider-icon {
		display: grid;
		width: 2.6rem;
		height: 2.6rem;
		place-items: center;
		border: 1px solid var(--scarlet);
		color: var(--scarlet-lamp);
	}

	h2 {
		margin: 0.25rem 0 0;
		color: var(--ink);
	}

	p {
		max-width: 70ch;
		margin: 0.45rem 0 0;
		color: var(--ink-dim);
		font-size: 0.75rem;
		line-height: 1.55;
	}

	small {
		display: block;
		margin-top: 0.7rem;
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.6875rem;
	}

	@media (max-width: 720px) {
		.unsupported-provider {
			grid-template-columns: auto minmax(0, 1fr);
		}

		.unsupported-provider :global([data-slot='badge']) {
			grid-column: 2;
			justify-self: start;
		}
	}
</style>
