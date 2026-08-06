<script lang="ts">
	import type { DesktopAppState } from '$lib/app-state.svelte';
	import type { RuntimeConfigDto, WhisperGpuDeviceInfo } from '$lib/bindings';
	import PoolActiveUnits from './PoolActiveUnits.svelte';
	import PoolDrainingNotice from './PoolDrainingNotice.svelte';
	import PoolEditor from './PoolEditor.svelte';
	import PoolTelemetry from './PoolTelemetry.svelte';
	import { PoolDraftState, setPoolDraftContext } from './context.svelte';
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

<section class="border-t border-line pt-[1.35rem]" aria-labelledby="pool-title">
	<header class="flex items-center justify-between gap-4 max-[760px]:items-start">
		<div>
			<h2
				id="pool-title"
				class="m-0 font-display text-[1.125rem] leading-none font-normal tracking-[0.035em] text-ink uppercase"
			>
				Execution pool
			</h2>
			<p class="mt-1 mb-0 max-w-[68ch] text-[0.72rem] leading-[1.45] text-ink-muted">
				Configure bounded workers without interrupting active transcription.
			</p>
		</div>
		<div class="flex items-baseline gap-[0.6rem] font-mono text-[0.6875rem] text-ink-muted">
			<span>Generation</span>
			<strong class="text-base font-normal text-ink tabular-nums">
				{state.poolStatus?.generation ?? runtime.generation ?? '—'}
			</strong>
		</div>
	</header>

	<PoolTelemetry />
	<PoolActiveUnits />
	<PoolDrainingNotice />
	<PoolEditor />
</section>
