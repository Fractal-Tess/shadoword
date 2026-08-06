<script lang="ts">
	import { AlertTriangle, Plus } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import PoolUnitRow from './PoolUnitRow.svelte';
	import { getPoolDraftContext } from './context.svelte';

	const state = getPoolDraftContext();
</script>

<div
	class="flex items-start gap-[0.7rem] border-b border-l-2 border-line border-l-scarlet px-[0.85rem] py-3 text-scarlet-lamp"
>
	<AlertTriangle size={16} />
	<p class="m-0 text-[0.6875rem] leading-[1.45] text-ink-dim">
		<strong class="mb-[0.08rem] block text-[0.72rem] text-ink">
			Every enabled unit loads an independent copy of the selected model.
		</strong>
		Plan RAM or VRAM for each worker; sharing a model file does not share its loaded memory.
	</p>
</div>

<div class="border-b border-line">
	{#each state.units as unit, index (state.rowKeys[index])}
		<PoolUnitRow {unit} {index} />
	{/each}
</div>

<div class="flex items-center justify-start gap-4 border-b border-line px-[0.85rem] py-[0.7rem]">
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
	<span class="ml-auto font-mono text-[0.65rem] text-ink-muted">
		{state.gpuDevices.length} detected GPU{state.gpuDevices.length === 1 ? '' : 's'}
	</span>
</div>
