<script lang="ts">
	import { ArrowDown, ArrowUp, Trash2 } from '@lucide/svelte';
	import type { ExecutionUnitConfig } from '$lib/bindings';
	import { Select } from '$lib/components/ui/select';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Switch } from '$lib/components/ui/switch';
	import { formatBytes } from '$lib/display';
	import { DEFAULT_GPU_HOST_THREADS } from '$lib/inference-pool';
	import { cn } from '$lib/utils';
	import { getPoolDraftContext } from './context.svelte';

	let { unit, index }: { unit: ExecutionUnitConfig; index: number } = $props();
	const state = getPoolDraftContext();
	const controlClass = 'grid min-w-0 content-start gap-1';
	const labelClass = 'text-[0.65rem] font-[620] text-ink-muted';
	const hintClass = 'text-[0.62rem] leading-[1.3] text-ink-muted';
</script>

<fieldset
	class={cn('m-0 min-w-0 border-0 px-[0.85rem] py-3', index > 0 && 'border-t border-line')}
	disabled={state.locked}
>
	<legend class="float-left w-[3.4rem] pt-6 font-mono text-[0.65rem] text-ink-muted">
		Unit {index + 1}
	</legend>
	<div
		class="grid grid-cols-[minmax(8rem,0.8fr)_minmax(12rem,1.6fr)_minmax(8rem,0.65fr)] gap-[0.55rem] max-[1050px]:grid-cols-2 max-[760px]:grid-cols-1"
	>
		<div class={controlClass}>
			<span class={labelClass}>Stable ID</span>
			<div
				class="bg-void/35 flex h-8 items-center border border-line px-3 font-mono text-[0.72rem] text-ink-dim"
				aria-label={`Execution unit ${index + 1} stable ID`}
			>
				{unit.id}
			</div>
			{#if state.fieldError(index, 'id')}
				<small class={cn(hintClass, 'text-scarlet-lamp')}>{state.fieldError(index, 'id')}</small>
			{:else}
				<small class={hintClass}>Fixed for this worker and kept across reloads.</small>
			{/if}
		</div>
		<label class={controlClass}>
			<span class={labelClass}>Execution device</span>
			<Select
				value={state.executionDeviceValue(unit)}
				options={state.executionDeviceOptions(unit)}
				ariaLabel={`Execution device for ${unit.id || `unit ${index + 1}`}`}
				ariaInvalid={Boolean(state.fieldError(index, 'device'))}
				ariaDescribedBy={state.fieldError(index, 'device')
					? `execution-unit-${index}-device-error`
					: undefined}
				onValueChange={(value) => state.setExecutionDevice(index, value)}
			/>
			{#if state.fieldError(index, 'device')}
				<small
					id={`execution-unit-${index}-device-error`}
					class={cn(hintClass, 'text-scarlet-lamp')}>{state.fieldError(index, 'device')}</small
				>
			{:else if unit.target.kind === 'gpu' && state.gpuName(unit.target.device)}
				<small class={hintClass}>
					{formatBytes(state.gpuName(unit.target.device)?.free_vram ?? 0)} free VRAM
				</small>
			{:else}
				<small class={hintClass}>Choose the processor that owns this worker.</small>
			{/if}
		</label>
		{#if unit.target.kind === 'cpu'}
			<label class={controlClass}>
				<span class={labelClass}>Worker threads</span>
				<Input
					type="number"
					min="1"
					max="256"
					value={unit.target.threads ?? 4}
					aria-label={`CPU threads for ${unit.id || `unit ${index + 1}`}`}
					oninput={(event) =>
						state.replaceUnit(index, {
							...unit,
							target: { kind: 'cpu', threads: Number(event.currentTarget.value) }
						})}
				/>
				<small class={hintClass}>CPU threads used by this worker for inference.</small>
			</label>
		{:else}
			<label class={controlClass}>
				<span class={labelClass}>CPU helper threads</span>
				<Input
					type="number"
					min="1"
					max="256"
					value={unit.target.host_threads ?? DEFAULT_GPU_HOST_THREADS}
					aria-label={`CPU helper threads for ${unit.id || `unit ${index + 1}`}`}
					oninput={(event) => state.setGpuHostThreads(index, Number(event.currentTarget.value))}
				/>
				<small class={hintClass}>
					CPU threads used for audio preprocessing and operations not handled by the GPU. Four is
					the upstream default; additional threads may not improve performance.
				</small>
			</label>
		{/if}
	</div>
	<div class="mt-[0.55rem] ml-[3.4rem] flex items-center gap-4 text-[0.6875rem] text-ink-dim">
		<label class="inline-flex items-center gap-[0.35rem]">
			<Switch
				checked={unit.enabled ?? true}
				onclick={() => state.replaceUnit(index, { ...unit, enabled: !(unit.enabled ?? true) })}
				aria-label={`Enable ${unit.id || `unit ${index + 1}`}`}
			/>
			<span>Enabled</span>
		</label>
		<label class="inline-flex items-center gap-[0.35rem]">
			<Switch
				checked={unit.required ?? true}
				onclick={() => state.replaceUnit(index, { ...unit, required: !(unit.required ?? true) })}
				aria-label={`Require ${unit.id || `unit ${index + 1}`} during reload`}
			/>
			<span>Required to reload</span>
		</label>
		<div class="ml-auto flex">
			<Button
				variant="ghost"
				size="icon-xs"
				disabled={index === 0}
				aria-label={`Move ${unit.id || `unit ${index + 1}`} up`}
				onclick={() => state.moveUnit(index, -1)}><ArrowUp data-icon="inline-start" /></Button
			>
			<Button
				variant="ghost"
				size="icon-xs"
				disabled={index === state.units.length - 1}
				aria-label={`Move ${unit.id || `unit ${index + 1}`} down`}
				onclick={() => state.moveUnit(index, 1)}><ArrowDown data-icon="inline-start" /></Button
			>
			<Button
				variant="ghost"
				size="icon-xs"
				aria-label={`Delete ${unit.id || `unit ${index + 1}`}`}
				onclick={() => state.removeUnit(index)}><Trash2 data-icon="inline-start" /></Button
			>
		</div>
	</div>
</fieldset>
