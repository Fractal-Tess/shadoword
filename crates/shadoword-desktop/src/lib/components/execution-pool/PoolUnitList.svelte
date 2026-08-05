<script lang="ts">
	import { AlertTriangle, Plus } from '@lucide/svelte';
	import PoolUnitRow from '$lib/components/execution-pool/PoolUnitRow.svelte';
	import { Button } from '$lib/components/ui/button';
	import { getPoolDraftContext } from '$lib/execution-pool/context.svelte';

	const state = getPoolDraftContext();
</script>

<div class="memory-warning">
	<AlertTriangle size={16} />
	<p>
		<strong>Every enabled unit loads an independent copy of the selected model.</strong>
		Plan RAM or VRAM for each worker; sharing a model file does not share its loaded memory.
	</p>
</div>

<div class="draft-units">
	{#each state.units as unit, index (state.rowKeys[index])}
		<PoolUnitRow {unit} {index} />
	{/each}
</div>

<div class="add-unit-row">
	<Button variant="outline" size="sm" disabled={state.locked} onclick={() => state.addCpu()}
		><Plus size={13} />Add CPU unit</Button
	>
	<Button
		variant="outline"
		size="sm"
		disabled={state.locked || !state.availableGpu}
		onclick={() => state.addGpu()}
		><Plus size={13} />{state.availableGpu
			? `Add GPU ${state.availableGpu.id}`
			: 'All GPUs assigned'}</Button
	>
	<span>{state.gpuDevices.length} detected GPU{state.gpuDevices.length === 1 ? '' : 's'}</span>
</div>

<style>
	.memory-warning {
		display: flex;
		align-items: flex-start;
		gap: 0.7rem;
		padding: 0.75rem 0.85rem;
		border-bottom: 1px solid var(--line);
		border-left: 2px solid var(--scarlet);
		color: var(--scarlet-lamp);
	}

	.memory-warning strong {
		display: block;
		margin-bottom: 0.08rem;
		color: var(--ink);
		font-size: 0.72rem;
	}

	.memory-warning p {
		margin: 0;
		color: var(--ink-dim);
		font-size: 0.6875rem;
		line-height: 1.45;
	}

	.draft-units {
		border-bottom: 1px solid var(--line);
	}

	.add-unit-row {
		display: flex;
		align-items: center;
		justify-content: flex-start;
		gap: 1rem;
		padding: 0.7rem 0.85rem;
		border-bottom: 1px solid var(--line);
	}

	.add-unit-row span {
		margin-left: auto;
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.65rem;
	}
</style>
