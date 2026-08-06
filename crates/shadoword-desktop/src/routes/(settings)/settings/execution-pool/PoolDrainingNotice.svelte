<script lang="ts">
	import { AlertTriangle } from '@lucide/svelte';
	import { formatBytes } from '$lib/display';
	import { getPoolDraftContext } from './context.svelte';

	const state = getPoolDraftContext();
</script>

{#if state.draining.length > 0}
	<div
		class="flex items-start gap-[0.7rem] border-b border-l-2 border-line border-l-scarlet px-[0.85rem] py-3 text-scarlet-lamp"
		role="status"
	>
		<AlertTriangle size={16} />
		<div class="grid gap-[0.15rem]">
			<strong class="text-[0.72rem] text-ink"
				>{state.draining.length} prior generation{state.draining.length === 1 ? '' : 's'} draining</strong
			>
			<span class="m-0 text-[0.6875rem] leading-[1.45] text-ink-dim"
				>Pool mutation remains locked while queued work finishes and retired workers release their
				model copies.</span
			>
			{#each state.draining as generation (generation.generation)}
				<small class="font-mono text-[0.65rem] text-ink-muted"
					>Generation {generation.generation ?? '—'} · {generation.running_jobs} running · {formatBytes(
						generation.running_audio_bytes
					)} · {generation.workers_remaining} workers remaining</small
				>
			{/each}
		</div>
	</div>
{/if}
