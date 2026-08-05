<script lang="ts">
	import { ArrowDown, ArrowUp, Trash2 } from '@lucide/svelte';
	import type { ExecutionUnitConfig } from '$lib/bindings';
	import BrutalistSelect from '$lib/components/BrutalistSelect.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Switch } from '$lib/components/ui/switch';
	import { formatBytes } from '$lib/display';
	import { getPoolDraftContext } from '$lib/execution-pool/context.svelte';

	let { unit, index }: { unit: ExecutionUnitConfig; index: number } = $props();
	const state = getPoolDraftContext();
</script>

<fieldset class="draft-unit" class:bordered={index > 0} disabled={state.locked}>
	<legend>Unit {index + 1}</legend>
	<div class="unit-controls">
		<label>
			<span>Stable ID</span>
			<Input
				value={unit.id}
				aria-label={`Execution unit ${index + 1} stable ID`}
				aria-invalid={Boolean(state.fieldError(index, 'id'))}
				oninput={(event) => state.replaceUnit(index, { ...unit, id: event.currentTarget.value })}
			/>
			{#if state.fieldError(index, 'id')}<small class="field-error"
					>{state.fieldError(index, 'id')}</small
				>{/if}
		</label>
		<label>
			<span>Target</span>
			<BrutalistSelect
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
			<label>
				<span>Threads</span>
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
			<label class="gpu-select">
				<span>GPU device</span>
				<BrutalistSelect
					value={String(unit.target.device)}
					options={state.gpuOptions(unit.target.device)}
					ariaLabel={`GPU device for ${unit.id || `unit ${index + 1}`}`}
					ariaInvalid={Boolean(state.fieldError(index, 'device'))}
					ariaDescribedBy={state.fieldError(index, 'device')
						? `execution-unit-${index}-device-error`
						: undefined}
					onValueChange={(value) => state.setGpuDevice(index, Number(value))}
				/>
				{#if state.fieldError(index, 'device')}<small
						id={`execution-unit-${index}-device-error`}
						class="field-error">{state.fieldError(index, 'device')}</small
					>{:else if state.gpuName(unit.target.device)}
					<small>{formatBytes(state.gpuName(unit.target.device)?.free_vram ?? 0)} free VRAM</small>
				{/if}
			</label>
			<label>
				<span>Host threads</span>
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
	<div class="unit-flags">
		<label class="flag-control">
			<Switch
				checked={unit.enabled ?? true}
				onclick={() => state.replaceUnit(index, { ...unit, enabled: !(unit.enabled ?? true) })}
				aria-label={`Enable ${unit.id || `unit ${index + 1}`}`}
			/>
			<span>Enabled</span>
		</label>
		<label class="flag-control">
			<Switch
				checked={unit.required ?? true}
				onclick={() => state.replaceUnit(index, { ...unit, required: !(unit.required ?? true) })}
				aria-label={`Require ${unit.id || `unit ${index + 1}`} during reload`}
			/>
			<span>Required to reload</span>
		</label>
		<div class="unit-order">
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

<style>
	.draft-unit {
		min-width: 0;
		margin: 0;
		border: 0;
		padding: 0.75rem 0.85rem;
	}

	.draft-unit.bordered {
		border-top: 1px solid var(--line);
	}

	.draft-unit legend {
		float: left;
		width: 3.4rem;
		padding-top: 1.5rem;
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.65rem;
	}

	.unit-controls {
		display: grid;
		grid-template-columns: minmax(8rem, 0.8fr) minmax(5rem, 0.45fr) minmax(7rem, 1.6fr) minmax(
				5.5rem,
				0.45fr
			);
		gap: 0.55rem;
	}

	.unit-controls label {
		display: grid;
		align-content: start;
		gap: 0.25rem;
		min-width: 0;
	}

	.unit-controls label > span {
		color: var(--ink-muted);
		font-size: 0.65rem;
		font-weight: 620;
	}

	.gpu-select small,
	.field-error {
		color: var(--ink-muted);
		font-size: 0.62rem;
		line-height: 1.3;
	}

	.field-error,
	.gpu-select .field-error {
		color: var(--scarlet-lamp);
	}

	.unit-flags {
		display: flex;
		align-items: center;
		gap: 1rem;
		margin: 0.55rem 0 0 3.4rem;
		color: var(--ink-dim);
		font-size: 0.6875rem;
	}

	.unit-flags label {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
	}

	.unit-order {
		display: flex;
		margin-left: auto;
	}

	@media (max-width: 1050px) {
		.unit-controls {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}

	@media (max-width: 760px) {
		.unit-controls {
			grid-template-columns: repeat(2, minmax(6rem, 1fr));
		}
	}
</style>
