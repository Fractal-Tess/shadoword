<script lang="ts">
	import { Input } from '$lib/components/ui/input';
	import { formatBytes } from '$lib/display';
	import { cn } from '$lib/utils';
	import { getPoolDraftContext } from './context.svelte';

	const state = getPoolDraftContext();
	const fieldClass = 'grid min-w-0 content-start gap-1';
	const labelClass = 'text-[0.65rem] font-[620] text-ink-muted';
	const hintClass = 'text-[0.62rem] leading-[1.3] text-ink-muted';
</script>

<details class="border-b border-line">
	<summary class="cursor-pointer px-[0.85rem] py-[0.72rem] text-[0.72rem] font-[570] text-ink-dim">
		Advanced admission and reload limits
	</summary>
	<div class="grid grid-cols-4 gap-[0.7rem] px-[0.85rem] pb-[0.85rem] max-[1050px]:grid-cols-2">
		<label class={fieldClass}>
			<span class={labelClass}>Queued jobs</span>
			<Input
				type="number"
				disabled={state.locked}
				min="0"
				max="10000"
				value={state.draft.limits?.max_queued_jobs ?? 32}
				oninput={(event) => state.setLimit('max_queued_jobs', Number(event.currentTarget.value))}
			/>
			<small class={hintClass}>Zero permits direct worker hand-off only.</small>
		</label>
		<label class={fieldClass}>
			<span class={labelClass}>Queue memory · MiB</span>
			<Input
				type="number"
				disabled={state.locked}
				min="1"
				max="1048576"
				value={Math.round((state.draft.limits?.max_queued_audio_bytes ?? 67_108_864) / 1024 ** 2)}
				oninput={(event) =>
					state.setByteLimit('max_queued_audio_bytes', Number(event.currentTarget.value))}
			/>
			<small class={hintClass}>
				{formatBytes(state.draft.limits?.max_queued_audio_bytes ?? 0)} decoded audio
			</small>
		</label>
		<label class={fieldClass}>
			<span class={labelClass}>Per-job memory · MiB</span>
			<Input
				type="number"
				disabled={state.locked}
				min="1"
				max="1048576"
				value={Math.round((state.draft.limits?.max_audio_bytes_per_job ?? 67_108_864) / 1024 ** 2)}
				oninput={(event) =>
					state.setByteLimit('max_audio_bytes_per_job', Number(event.currentTarget.value))}
			/>
			<small class={hintClass}>
				{formatBytes(state.draft.limits?.max_audio_bytes_per_job ?? 0)} maximum
			</small>
		</label>
		<label class={fieldClass}>
			<span class={labelClass}>Outstanding per flow</span>
			<Input
				type="number"
				disabled={state.locked}
				min="1"
				max="10000"
				value={state.draft.limits?.max_outstanding_per_flow ?? 8}
				oninput={(event) =>
					state.setLimit('max_outstanding_per_flow', Number(event.currentTarget.value))}
			/>
		</label>
		<label class={fieldClass}>
			<span class={labelClass}>Buffered results per flow</span>
			<Input
				type="number"
				disabled={state.locked}
				min="1"
				max="10000"
				value={state.draft.limits?.max_buffered_results_per_flow ?? 32}
				oninput={(event) =>
					state.setLimit('max_buffered_results_per_flow', Number(event.currentTarget.value))}
			/>
			<small class={hintClass}>Bounds out-of-order streaming completions.</small>
		</label>
		<label class={fieldClass}>
			<span class={labelClass}>Preload timeout · seconds</span>
			<Input
				type="number"
				disabled={state.locked}
				min="1"
				max="1800"
				aria-invalid={Boolean(state.app.poolFieldErrors.preload_timeout_ms)}
				value={Math.round((state.draft.preload_timeout_ms ?? 120_000) / 1000)}
				oninput={(event) => state.setPreloadTimeout(Number(event.currentTarget.value))}
			/>
			{#if state.app.poolFieldErrors.preload_timeout_ms}
				<small class={cn(hintClass, 'text-scarlet-lamp')}>
					{state.app.poolFieldErrors.preload_timeout_ms}
				</small>
			{/if}
		</label>
		<label class={fieldClass}>
			<span class={labelClass}>Maximum draining generations</span>
			<Input
				type="number"
				disabled={state.locked}
				min="1"
				max="8"
				aria-invalid={Boolean(state.app.poolFieldErrors.max_draining_generations)}
				value={state.draft.max_draining_generations ?? 2}
				oninput={(event) => state.setMaxDrainingGenerations(Number(event.currentTarget.value))}
			/>
			{#if state.app.poolFieldErrors.max_draining_generations}
				<small class={cn(hintClass, 'text-scarlet-lamp')}>
					{state.app.poolFieldErrors.max_draining_generations}
				</small>
			{/if}
			<small class={hintClass}>Hard cap: 8 overlapping retiring generations.</small>
		</label>
	</div>
</details>
