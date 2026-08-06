<script lang="ts">
	import { Check, Clock3, Copy, Trash2 } from '@lucide/svelte';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import type { HistoryRecord } from '$lib/types';

	let {
		item,
		copied = false,
		onCopy,
		onDelete
	}: {
		item: HistoryRecord;
		copied?: boolean;
		onCopy: () => void;
		onDelete: () => void;
	} = $props();

	const formatCost = (cost: number) => {
		if (cost === 0) return '$0.00';
		if (cost < 0.000001) return '<$0.000001';
		if (cost < 0.0001) return `$${cost.toFixed(6)}`;
		if (cost < 0.01) return `$${cost.toFixed(5)}`;
		return `$${cost.toFixed(4)}`;
	};
</script>

<article class="group grid grid-cols-[1.5rem_minmax(0,1fr)] gap-[0.8rem]">
	<div
		class="relative flex justify-center after:absolute after:top-5 after:bottom-0 after:w-px after:bg-line after:content-[''] group-last:after:hidden"
		aria-hidden="true"
	>
		<span class="z-1 mt-[1.15rem] size-2 border-2 border-night bg-ink"></span>
	</div>
	<div
		class="mb-[0.7rem] border border-line bg-plate transition-[border-color,background-color] duration-150 ease-in-out hover:border-line-strong hover:bg-raised"
	>
		<header
			class="flex items-center justify-between gap-4 border-b border-line py-[0.55rem] pr-[0.65rem] pl-[0.9rem]"
		>
			<div class="flex items-center gap-[0.4rem] font-mono text-[0.6875rem] text-ink-muted">
				<Clock3 size={13} />{item.timestamp}
			</div>
			<div class="flex items-center gap-[0.15rem]">
				<Button variant="ghost" size="icon-sm" onclick={onCopy} aria-label="Copy transcript">
					{#if copied}<Check size={14} />{:else}<Copy size={14} />{/if}
				</Button>
				<Button
					variant="ghost"
					size="icon-sm"
					onclick={onDelete}
					aria-label="Delete transcript"
					data-history-delete={item.id}
				>
					<Trash2 size={14} />
				</Button>
			</div>
		</header>
		<p class="m-0 max-w-[72ch] px-4 pt-4 pb-[0.85rem] text-sm leading-[1.6] text-ink">
			{item.text}
		</p>
		<footer class="flex flex-wrap items-center gap-[0.7rem] px-4 pb-4">
			<Badge variant="outline">{item.engine}</Badge>
			<span class="font-mono text-[0.6875rem] text-ink-muted">{item.duration} audio</span>
			<span class="font-mono text-[0.6875rem] text-ink-muted">
				{item.segments}
				{item.segments === 1 ? 'segment' : 'segments'}
			</span>
			<span class="font-mono text-[0.6875rem] text-ink-muted">{item.latency} inference</span>
			{#if item.costUsd != null}
				<span class="border-l border-scarlet pl-[0.7rem] font-mono text-[0.6875rem] text-ink">
					{formatCost(item.costUsd)} request cost
				</span>
			{/if}
		</footer>
	</div>
</article>
