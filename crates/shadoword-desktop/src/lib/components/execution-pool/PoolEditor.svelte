<script lang="ts">
	import { Cpu } from '@lucide/svelte';
	import PoolActions from '$lib/components/execution-pool/PoolActions.svelte';
	import PoolLimits from '$lib/components/execution-pool/PoolLimits.svelte';
	import PoolUnitList from '$lib/components/execution-pool/PoolUnitList.svelte';
	import { getPoolDraftContext } from '$lib/execution-pool/context.svelte';

	const state = getPoolDraftContext();
</script>

<div class="pool-editor" class:locked={state.locked}>
	<div class="editor-heading">
		<div>
			<h3 class="display-legend">Stage execution topology</h3>
			<p>
				Model, language, and preload policy stay shared so every worker produces consistent output.
			</p>
		</div>
		<div class="mode-switch" aria-label="Inference topology mode">
			<button
				type="button"
				class:active={!state.explicit}
				aria-pressed={!state.explicit}
				disabled={state.locked}
				onclick={() => state.setExplicit(false)}>Legacy single</button
			>
			<button
				type="button"
				class:active={state.explicit}
				aria-pressed={state.explicit}
				disabled={state.locked}
				onclick={() => state.setExplicit(true)}>Explicit pool</button
			>
		</div>
	</div>

	{#if state.explicit}
		<PoolUnitList />
		<PoolLimits />
	{:else}
		<div class="legacy-copy">
			<Cpu size={17} />
			<div>
				<strong>One worker follows the legacy accelerator controls below.</strong>
				<span
					>Auto remains automatic, including device −1. Converting to a pool chooses a detected GPU
					explicitly or creates a CPU unit, never an invalid automatic GPU target.</span
				>
			</div>
		</div>
	{/if}

	<PoolActions />
</div>

<style>
	.pool-editor {
		margin-top: 1rem;
		border: 1px solid var(--line);
		background: var(--surface-1);
	}

	.editor-heading {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.85rem 1rem;
		border-bottom: 1px solid var(--line);
	}

	.editor-heading h3 {
		margin: 0;
		color: var(--ink);
	}

	.editor-heading p {
		max-width: 68ch;
		margin: 0.25rem 0 0;
		color: var(--ink-muted);
		font-size: 0.72rem;
		line-height: 1.45;
	}

	.mode-switch {
		display: inline-flex;
		gap: 1px;
		background: var(--line);
	}

	.mode-switch button {
		border: 0;
		padding: 0.4rem 0.7rem;
		background: var(--surface-1);
		color: var(--ink-muted);
		font: inherit;
		font-size: 0.6875rem;
		cursor: pointer;
		transition:
			background-color 120ms linear,
			color 120ms linear;
	}

	.mode-switch button:hover:not(:disabled) {
		background: var(--surface-2);
		color: var(--ink);
	}

	.mode-switch button.active {
		background: var(--surface-2);
		color: var(--scarlet-lamp);
		box-shadow: inset 0 -2px 0 var(--scarlet);
	}

	.legacy-copy {
		display: flex;
		min-height: 5rem;
		align-items: center;
		gap: 0.7rem;
		padding: 0.75rem 0.85rem;
		color: var(--ink-muted);
	}

	.legacy-copy > div {
		display: grid;
		gap: 0.15rem;
	}

	.legacy-copy strong {
		color: var(--ink);
		font-size: 0.72rem;
	}

	.legacy-copy span {
		margin: 0;
		color: var(--ink-dim);
		font-size: 0.6875rem;
		line-height: 1.45;
	}

	.locked {
		opacity: 0.64;
	}

	@media (max-width: 760px) {
		.editor-heading {
			align-items: flex-start;
		}
	}
</style>
