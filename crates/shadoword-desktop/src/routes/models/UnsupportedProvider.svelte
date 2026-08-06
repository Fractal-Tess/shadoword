<script lang="ts">
	import { Cloud } from '@lucide/svelte';
	import { Badge } from '$lib/components/ui/badge';
	import { getModelsContext } from './context';

	const context = getModelsContext();
</script>

<section
	class="grid grid-cols-[auto_minmax(0,1fr)_auto] items-start gap-[0.9rem] border border-l-2 border-line border-l-scarlet bg-plate p-4 text-ink-muted max-[720px]:grid-cols-[auto_minmax(0,1fr)]"
	aria-labelledby="unsupported-models-title"
>
	<div
		class="grid size-[2.6rem] place-items-center border border-scarlet text-scarlet-lamp"
		aria-hidden="true"
	>
		<Cloud size={20} />
	</div>
	<div>
		<span
			class="font-mono text-xs leading-[1.3] font-medium tracking-[0.14em] text-ink-muted uppercase"
		>
			Hosted provider
		</span>
		<h2
			id="unsupported-models-title"
			class="mt-1 mb-0 font-display text-[1.125rem] leading-none font-normal tracking-[0.035em] text-ink uppercase"
		>
			OpenRouter model management
		</h2>
		<p class="mt-[0.45rem] mb-0 max-w-[70ch] text-xs leading-[1.55] text-ink-dim">
			OpenRouter does not expose a Shadoword Whisper runtime, verified model downloads, custom GGML
			paths, or accelerator controls. Choose the hosted transcription model in Execution settings
			instead.
		</p>
		{#if context.app.settings?.openrouter_model}
			<small class="mt-[0.7rem] block font-mono text-[0.6875rem] text-ink-muted">
				Current model · {context.app.settings.openrouter_model}
			</small>
		{/if}
	</div>
	<Badge class="max-[720px]:col-start-2 max-[720px]:justify-self-start" variant="outline">
		Unsupported here
	</Badge>
</section>
