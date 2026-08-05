<script lang="ts">
	import type { DesktopAppState } from '$lib/app-state.svelte';
	import type { RuntimeConfigDto, WhisperGpuDeviceInfo } from '$lib/bindings';
	import PoolActiveUnits from '$lib/components/execution-pool/PoolActiveUnits.svelte';
	import PoolDrainingNotice from '$lib/components/execution-pool/PoolDrainingNotice.svelte';
	import PoolEditor from '$lib/components/execution-pool/PoolEditor.svelte';
	import PoolTelemetry from '$lib/components/execution-pool/PoolTelemetry.svelte';
	import { PoolDraftState, setPoolDraftContext } from '$lib/execution-pool/context.svelte';
	import { untrack } from 'svelte';

	let {
		app,
		runtime,
		gpuDevices
	}: {
		app: DesktopAppState;
		runtime: RuntimeConfigDto;
		gpuDevices: WhisperGpuDeviceInfo[];
	} = $props();

	const state = untrack(
		() =>
			new PoolDraftState({
				get app() {
					return app;
				},
				get runtime() {
					return runtime;
				},
				get gpuDevices() {
					return gpuDevices;
				}
			})
	);
	setPoolDraftContext(state);
</script>

<section class="pool-observatory" aria-labelledby="pool-title">
	<header class="pool-heading">
		<div>
			<h2 id="pool-title" class="display-legend">Execution pool</h2>
			<p>Observe the active generation, then stage a replacement without interrupting its state.</p>
		</div>
		<div class="generation-readout">
			<span>Generation</span>
			<strong>{state.poolStatus?.generation ?? runtime.generation ?? '—'}</strong>
		</div>
	</header>

	<PoolTelemetry />
	<PoolActiveUnits />
	<PoolDrainingNotice />
	<PoolEditor />
</section>

<style>
	.pool-observatory {
		border-top: 1px solid var(--line);
		padding-top: 1.35rem;
	}

	.pool-heading {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
	}

	.pool-heading h2 {
		margin: 0;
		color: var(--ink);
	}

	.pool-heading p {
		max-width: 68ch;
		margin: 0.25rem 0 0;
		color: var(--ink-muted);
		font-size: 0.72rem;
		line-height: 1.45;
	}

	.generation-readout {
		display: flex;
		align-items: baseline;
		gap: 0.6rem;
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.6875rem;
	}

	/* The generation number is a reading, and readings are off-white with tabular
	   figures. It was the old accent hue, which made the one number on this panel that
	   never needs acting on the loudest thing on it. */
	.generation-readout strong {
		color: var(--ink);
		font-size: 1rem;
		font-weight: 400;
		font-variant-numeric: tabular-nums;
	}

	@media (max-width: 760px) {
		.pool-heading {
			align-items: flex-start;
		}
	}
</style>
