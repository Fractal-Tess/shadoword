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

<article>
	<div class="timeline-marker" aria-hidden="true"><span></span></div>
	<div class="entry">
		<header>
			<div class="entry-time"><Clock3 size={13} />{item.timestamp}</div>
			<div class="entry-actions">
				<Button variant="ghost" size="icon-sm" onclick={onCopy} aria-label="Copy transcript">
					{#if copied}<Check size={14} />{:else}<Copy size={14} />{/if}
				</Button>
				<Button variant="ghost" size="icon-sm" onclick={onDelete} aria-label="Delete transcript">
					<Trash2 size={14} />
				</Button>
			</div>
		</header>
		<p>{item.text}</p>
		<footer>
			<Badge variant="outline">{item.engine}</Badge>
			<span>{item.duration} audio</span>
			<span>{item.segments} {item.segments === 1 ? 'segment' : 'segments'}</span>
			<span>{item.latency} inference</span>
			{#if item.costUsd != null}
				<span class="request-cost">{formatCost(item.costUsd)} request cost</span>
			{/if}
		</footer>
	</div>
</article>

<style>
	article {
		display: grid;
		grid-template-columns: 1.5rem minmax(0, 1fr);
		gap: 0.8rem;
	}

	.timeline-marker {
		position: relative;
		display: flex;
		justify-content: center;
	}

	.timeline-marker::after {
		position: absolute;
		top: 1.25rem;
		bottom: 0;
		width: 1px;
		background: var(--line);
		content: '';
	}

	article:last-child .timeline-marker::after {
		display: none;
	}

	.timeline-marker span {
		z-index: 1;
		width: 0.5rem;
		height: 0.5rem;
		margin-top: 1.15rem;
		border: 2px solid var(--surface-0);
		background: var(--ink);
	}

	.entry {
		margin-bottom: 0.7rem;
		border: 1px solid var(--line);
		background: var(--surface-1);
		transition:
			border-color 140ms ease,
			background-color 140ms ease;
	}

	.entry:hover {
		border-color: var(--line-strong);
		background: var(--surface-2);
	}

	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		border-bottom: 1px solid var(--line);
		padding: 0.55rem 0.65rem 0.55rem 0.9rem;
	}

	.entry-time,
	.entry-actions,
	footer {
		display: flex;
		align-items: center;
	}

	.entry-time {
		gap: 0.4rem;
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.6875rem;
	}

	.entry-actions {
		gap: 0.15rem;
	}

	p {
		max-width: 72ch;
		margin: 0;
		padding: 1rem 1rem 0.85rem;
		color: var(--ink);
		font-size: 0.875rem;
		line-height: 1.6;
	}

	footer {
		gap: 0.7rem;
		flex-wrap: wrap;
		padding: 0 1rem 1rem;
	}

	footer > span {
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.6875rem;
	}

	footer > .request-cost {
		border-left: 1px solid var(--scarlet);
		padding-left: 0.7rem;
		color: var(--ink);
	}
</style>
