<script lang="ts">
	import { AlertTriangle } from '@lucide/svelte';
	import { formatBytes } from '$lib/display';
	import { getPoolDraftContext } from '$lib/execution-pool/context.svelte';

	const state = getPoolDraftContext();
</script>

{#if state.draining.length > 0}
	<div class="draining-notice" role="status">
		<AlertTriangle size={16} />
		<div>
			<strong
				>{state.draining.length} prior generation{state.draining.length === 1 ? '' : 's'} draining</strong
			>
			<span
				>Pool mutation is locked until its queued and running work finishes, preventing too many
				model copies from overlapping.</span
			>
			{#each state.draining as generation (generation.generation)}
				<small
					>Generation {generation.generation ?? '—'} · {generation.running_jobs} running · {formatBytes(
						generation.running_audio_bytes
					)} · {generation.workers_remaining} workers remaining</small
				>
			{/each}
		</div>
	</div>
{/if}

<style>
	.draining-notice {
		display: flex;
		align-items: flex-start;
		gap: 0.7rem;
		padding: 0.75rem 0.85rem;
		border-bottom: 1px solid var(--line);
		border-left: 2px solid var(--scarlet);
		color: var(--scarlet-lamp);
	}

	.draining-notice > div {
		display: grid;
		gap: 0.15rem;
	}

	.draining-notice strong {
		color: var(--ink);
		font-size: 0.72rem;
	}

	.draining-notice span {
		margin: 0;
		color: var(--ink-dim);
		font-size: 0.6875rem;
		line-height: 1.45;
	}

	.draining-notice small {
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.65rem;
	}
</style>
