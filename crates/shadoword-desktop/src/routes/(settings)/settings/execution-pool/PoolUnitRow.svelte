<script lang="ts">
	import { ArrowDown, ArrowUp, Trash2 } from '@lucide/svelte';
	import type { ExecutionUnitConfig } from '$lib/bindings';
	import { Select } from '$lib/components/ui/select';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Switch } from '$lib/components/ui/switch';
	import { formatBytes } from '$lib/display';
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
		class="grid grid-cols-[minmax(8rem,0.8fr)_minmax(5rem,0.45fr)_minmax(7rem,1.6fr)_minmax(5.5rem,0.45fr)] gap-[0.55rem] max-[1050px]:grid-cols-2 max-[760px]:grid-cols-[repeat(2,minmax(6rem,1fr))]"
	>
		<label class={controlClass}>
			<span class={labelClass}>Stable ID</span>
			<Input
				value={unit.id}
				aria-label={`Execution unit ${index + 1} stable ID`}
				aria-invalid={Boolean(state.fieldError(index, 'id'))}
				oninput={(event) => state.replaceUnit(index, { ...unit, id: event.currentTarget.value })}
			/>
			{#if state.fieldError(index, 'id')}
				<small class={cn(hintClass, 'text-scarlet-lamp')}>{state.fieldError(index, 'id')}</small>
			{/if}
		</label>
		<label class={controlClass}>
			<span class={labelClass}>Target</span>
			<Select
				value={unit.target.kind}
				options={[
					{ value: 'cpu', label: 'CPU', detail: 'Host threads' },
					{
						value: 'gpu',
						label: 'GPU',
						detail: 'Dedicated accelerator',
						disabled: state.gpuDevices.length === 0
					}
				]}
				ariaLabel={`Execution unit ${index + 1} target`}
				onValueChange={(value) => state.setUnitTarget(index, value === 'gpu' ? 'gpu' : 'cpu')}
			/>
		</label>
		{#if unit.target.kind === 'cpu'}
			<label class={controlClass}>
				<span class={labelClass}>Threads</span>
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
			</label>
		{:else}
			<label class={controlClass}>
				<span class={labelClass}>GPU device</span>
				<Select
					value={String(unit.target.device)}
					options={state.gpuOptions(unit.target.device)}
					ariaLabel={`GPU device for ${unit.id || `unit ${index + 1}`}`}
					ariaInvalid={Boolean(state.fieldError(index, 'device'))}
					ariaDescribedBy={state.fieldError(index, 'device')
						? `execution-unit-${index}-device-error`
						: undefined}
					onValueChange={(value) => state.setGpuDevice(index, Number(value))}
				/>
				{#if state.fieldError(index, 'device')}
					<small
						id={`execution-unit-${index}-device-error`}
						class={cn(hintClass, 'text-scarlet-lamp')}>{state.fieldError(index, 'device')}</small
					>
				{:else if state.gpuName(unit.target.device)}
					<small class={hintClass}>
						{formatBytes(state.gpuName(unit.target.device)?.free_vram ?? 0)} free VRAM
					</small>
				{/if}
			</label>
			<label class={controlClass}>
				<span class={labelClass}>Host threads</span>
				<Input
					type="number"
					min="1"
					max="256"
					value={unit.target.host_threads ?? 1}
					aria-label={`GPU host threads for ${unit.id || `unit ${index + 1}`}`}
					oninput={(event) => state.setGpuHostThreads(index, Number(event.currentTarget.value))}
				/>
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
				onclick={() => state.moveUnit(index, -1)}><ArrowUp size={13} /></Button
			>
			<Button
				variant="ghost"
				size="icon-xs"
				disabled={index === state.units.length - 1}
				aria-label={`Move ${unit.id || `unit ${index + 1}`} down`}
				onclick={() => state.moveUnit(index, 1)}><ArrowDown size={13} /></Button
			>
			<Button
				variant="ghost"
				size="icon-xs"
				aria-label={`Delete ${unit.id || `unit ${index + 1}`}`}
				onclick={() => state.removeUnit(index)}><Trash2 size={13} /></Button
			>
		</div>
	</div>
</fieldset>
