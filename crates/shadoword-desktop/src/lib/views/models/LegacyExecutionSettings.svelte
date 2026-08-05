<script lang="ts">
	import BrutalistSelect from '$lib/components/BrutalistSelect.svelte';
	import { isExplicitPool } from '$lib/inference-pool';
	import { getModelsContext } from './context';

	const context = getModelsContext();

	function setAccelerator(value: string) {
		if (value === 'auto' || value === 'gpu' || value === 'cpu') {
			void context.updateRuntime({ whisper_accelerator: value });
		}
	}
</script>

{#if !isExplicitPool(context.runtime)}
	<section class="execution-settings" aria-label="Legacy single-unit execution settings">
		<div>
			<label for="accelerator">Whisper accelerator</label>
			<span>Auto uses the best backend compiled into this runtime.</span>
		</div>
		<BrutalistSelect
			id="accelerator"
			value={context.accelerator}
			options={[
				{ value: 'auto', label: 'Automatic', detail: 'Best compiled backend' },
				{ value: 'gpu', label: 'GPU', detail: 'Hardware acceleration' },
				{ value: 'cpu', label: 'CPU', detail: 'Portable execution' }
			]}
			disabled={context.controlsLocked}
			onValueChange={setAccelerator}
		/>
		<div>
			<label for="gpu-device">GPU device</label>
			<span>Select a specific device or let Shadoword choose.</span>
		</div>
		<BrutalistSelect
			id="gpu-device"
			value={String(context.gpuDevice)}
			options={context.gpuDeviceOptions}
			disabled={context.controlsLocked || context.accelerator === 'cpu'}
			onValueChange={(value) => context.updateRuntime({ whisper_gpu_device: Number(value) })}
		/>
	</section>
{/if}

<style>
	.execution-settings {
		display: grid;
		grid-template-columns: minmax(11rem, 1fr) minmax(10rem, auto) minmax(11rem, 1fr) minmax(
				13rem,
				auto
			);
		align-items: center;
		gap: 0.8rem;
		border: 1px solid var(--line);
		padding: 0.8rem 1rem;
		background: var(--surface-1);
	}

	.execution-settings > div {
		display: grid;
		gap: 0.2rem;
	}

	.execution-settings label {
		color: var(--ink);
		font-size: 0.75rem;
		font-weight: 570;
	}

	.execution-settings span {
		color: var(--ink-muted);
		font-size: 0.6875rem;
	}

	@media (max-width: 920px) {
		.execution-settings {
			grid-template-columns: 1fr 1fr;
		}
	}
</style>
