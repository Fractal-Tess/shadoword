<script lang="ts">
	import { Input } from '$lib/components/ui/input';
	import { formatBytes } from '$lib/display';
	import { getPoolDraftContext } from '$lib/execution-pool/context.svelte';

	const state = getPoolDraftContext();
</script>

<details class="advanced-limits">
	<summary>Advanced admission and reload limits</summary>
	<div class="limit-grid">
		<label>
			<span>Queued jobs</span>
			<Input
				type="number"
				disabled={state.locked}
				min="0"
				max="10000"
				value={state.draft.limits?.max_queued_jobs ?? 32}
				oninput={(event) => state.setLimit('max_queued_jobs', Number(event.currentTarget.value))}
			/>
			<small>Zero permits direct worker hand-off only.</small>
		</label>
		<label>
			<span>Queue memory · MiB</span>
			<Input
				type="number"
				disabled={state.locked}
				min="1"
				max="1048576"
				value={Math.round((state.draft.limits?.max_queued_audio_bytes ?? 67_108_864) / 1024 ** 2)}
				oninput={(event) =>
					state.setByteLimit('max_queued_audio_bytes', Number(event.currentTarget.value))}
			/>
			<small>{formatBytes(state.draft.limits?.max_queued_audio_bytes ?? 0)} decoded audio</small>
		</label>
		<label>
			<span>Per-job memory · MiB</span>
			<Input
				type="number"
				disabled={state.locked}
				min="1"
				max="1048576"
				value={Math.round((state.draft.limits?.max_audio_bytes_per_job ?? 67_108_864) / 1024 ** 2)}
				oninput={(event) =>
					state.setByteLimit('max_audio_bytes_per_job', Number(event.currentTarget.value))}
			/>
			<small>{formatBytes(state.draft.limits?.max_audio_bytes_per_job ?? 0)} maximum</small>
		</label>
		<label>
			<span>Outstanding per flow</span>
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
		<label>
			<span>Buffered results per flow</span>
			<Input
				type="number"
				disabled={state.locked}
				min="1"
				max="10000"
				value={state.draft.limits?.max_buffered_results_per_flow ?? 32}
				oninput={(event) =>
					state.setLimit('max_buffered_results_per_flow', Number(event.currentTarget.value))}
			/>
			<small>Bounds out-of-order streaming completions.</small>
		</label>
		<label>
			<span>Preload timeout · seconds</span>
			<Input
				type="number"
				disabled={state.locked}
				min="1"
				max="1800"
				aria-invalid={Boolean(state.app.poolFieldErrors.preload_timeout_ms)}
				value={Math.round((state.draft.preload_timeout_ms ?? 120_000) / 1000)}
				oninput={(event) => state.setPreloadTimeout(Number(event.currentTarget.value))}
			/>
			{#if state.app.poolFieldErrors.preload_timeout_ms}<small class="field-error"
					>{state.app.poolFieldErrors.preload_timeout_ms}</small
				>{/if}
		</label>
		<label>
			<span>Maximum draining generations</span>
			<Input
				type="number"
				disabled={state.locked}
				min="1"
				max="8"
				aria-invalid={Boolean(state.app.poolFieldErrors.max_draining_generations)}
				value={state.draft.max_draining_generations ?? 2}
				oninput={(event) => state.setMaxDrainingGenerations(Number(event.currentTarget.value))}
			/>
			{#if state.app.poolFieldErrors.max_draining_generations}<small class="field-error"
					>{state.app.poolFieldErrors.max_draining_generations}</small
				>{/if}
			<small>Hard cap: 8 overlapping retiring generations.</small>
		</label>
	</div>
</details>

<style>
	.advanced-limits {
		border-bottom: 1px solid var(--line);
	}

	.advanced-limits summary {
		padding: 0.72rem 0.85rem;
		color: var(--ink-dim);
		font-size: 0.72rem;
		font-weight: 570;
		cursor: pointer;
	}

	.limit-grid {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: 0.7rem;
		padding: 0 0.85rem 0.85rem;
	}

	.limit-grid label {
		display: grid;
		align-content: start;
		gap: 0.25rem;
		min-width: 0;
	}

	.limit-grid label > span {
		color: var(--ink-muted);
		font-size: 0.65rem;
		font-weight: 620;
	}

	.limit-grid small,
	.field-error {
		color: var(--ink-muted);
		font-size: 0.62rem;
		line-height: 1.3;
	}

	.field-error {
		color: var(--scarlet-lamp);
	}

	@media (max-width: 1050px) {
		.limit-grid {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
</style>
