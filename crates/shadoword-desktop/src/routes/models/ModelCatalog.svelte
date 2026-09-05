<script lang="ts">
	import { AlertTriangle } from '@lucide/svelte';
	import ModelRow from './ModelRow.svelte';
	import { getModelsContext } from './state.svelte';

	const context = getModelsContext();
</script>

<section class="border-t border-line pt-6" aria-labelledby="catalog-title">
	<header class="flex items-end justify-between gap-4 pb-[0.9rem]">
		<h2
			id="catalog-title"
			class="m-0 font-display text-[1.125rem] leading-none font-normal tracking-[0.035em] text-ink uppercase"
		>
			Whisper models
		</h2>
		<p class="m-0 text-[0.6875rem] text-ink-muted">Downloads are checksum-verified before use.</p>
	</header>

	{#if context.models.length > 0}
		<div class="border border-line bg-plate">
			{#each context.models as model (model.id)}
				<ModelRow {model} />
			{/each}
		</div>
	{:else}
		<div
			class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-3 border border-line bg-plate px-4 py-[0.85rem] text-ink-muted"
		>
			<AlertTriangle size={17} />
			<div class="grid gap-1">
				<strong class="text-xs text-ink">No model catalog available</strong>
				<span class="font-mono text-[0.6875rem] text-ink-muted">
					Refresh the {context.mode === 'remote' ? 'Shadoword API' : 'local runtime'}.
				</span>
			</div>
		</div>
	{/if}
</section>
