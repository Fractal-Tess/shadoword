<script lang="ts">
	import { AlertTriangle } from '@lucide/svelte';
	import ModelRow from './ModelRow.svelte';
	import { getModelsContext } from './context';

	const context = getModelsContext();
</script>

<section class="catalog" aria-labelledby="catalog-title">
	<header>
		<div>
			<span>Verified catalog</span>
			<h2 id="catalog-title" class="display-legend">Whisper models</h2>
		</div>
		<p>Downloads are checksum-verified before use.</p>
	</header>

	{#if context.models.length > 0}
		<div class="model-list">
			{#each context.models as model (model.id)}
				<ModelRow {model} />
			{/each}
		</div>
	{:else}
		<div class="custom-row">
			<AlertTriangle size={17} />
			<div>
				<strong>No model catalog available</strong>
				<span>Refresh the {context.mode === 'remote' ? 'Shadoword API' : 'local runtime'}.</span>
			</div>
		</div>
	{/if}
</section>

<style>
	.catalog {
		border-top: 1px solid var(--line);
		padding-top: 1.5rem;
	}

	.catalog > header {
		display: flex;
		align-items: end;
		justify-content: space-between;
		gap: 1rem;
		padding-bottom: 0.9rem;
	}

	.catalog header span {
		color: var(--ink-muted);
		font-size: 0.6875rem;
		font-weight: 680;
		letter-spacing: 0.09em;
		text-transform: uppercase;
	}

	.catalog h2 {
		margin: 0.2rem 0 0;
		color: var(--ink);
	}

	.catalog > header p {
		margin: 0;
		color: var(--ink-muted);
		font-size: 0.6875rem;
	}

	.model-list {
		border: 1px solid var(--line);
		background: var(--surface-1);
	}

	.model-list :global(.model-row + .model-row) {
		border-top: 1px solid var(--line);
	}

	.custom-row {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto auto;
		align-items: center;
		gap: 0.75rem;
		border: 1px solid var(--line);
		padding: 0.85rem 1rem;
		background: var(--surface-1);
		color: var(--ink-muted);
	}

	.custom-row > div {
		display: grid;
		gap: 0.25rem;
	}

	.custom-row strong {
		color: var(--ink);
		font-size: 0.75rem;
	}

	.custom-row span {
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.6875rem;
	}
</style>
