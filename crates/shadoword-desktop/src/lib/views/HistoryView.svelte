<script lang="ts">
	import { Check, Clock3, Copy, Search, Trash2 } from '@lucide/svelte';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import SurfaceHeader from '$lib/components/SurfaceHeader.svelte';
	import type { DesktopAppState } from '$lib/app-state.svelte';
	import type { HistoryRecord } from '$lib/types';

	let { app }: { app: DesktopAppState } = $props();
	let history = $derived(app.history);
	let query = $state('');
	let copiedId = $state<string | null>(null);
	let lastDeleted = $state<{ record: HistoryRecord; index: number } | null>(null);

	let filteredHistory = $derived(
		history.filter((item) => item.text.toLowerCase().includes(query.trim().toLowerCase()))
	);

	const copyItem = async (id: string, text: string) => {
		await navigator.clipboard?.writeText(text);
		copiedId = id;
	};

	const deleteItem = (id: string) => {
		const index = history.findIndex((item) => item.id === id);
		if (index < 0) return;
		lastDeleted = { record: history[index], index };
		app.history = history.filter((item) => item.id !== id);
	};

	const formatCost = (cost: number) => {
		if (cost === 0) return '$0.00';
		if (cost < 0.000001) return '<$0.000001';
		if (cost < 0.0001) return `$${cost.toFixed(6)}`;
		if (cost < 0.01) return `$${cost.toFixed(5)}`;
		return `$${cost.toFixed(4)}`;
	};

	const undoDelete = () => {
		if (lastDeleted) {
			const restored = [...history];
			restored.splice(lastDeleted.index, 0, lastDeleted.record);
			app.history = restored;
		}
		lastDeleted = null;
	};
</script>

<div class="history-view">
	<SurfaceHeader
		kicker="History"
		title="Words from this session."
		description="Review, copy, or remove transcripts captured while Shadoword is running."
	>
		{#snippet actions()}
			<Button
				variant="ghost"
				size="sm"
				onclick={() => {
					lastDeleted = null;
					app.clearHistory();
				}}
				disabled={history.length === 0}
			>
				<Trash2 size={14} />Clear all
			</Button>
		{/snippet}
	</SurfaceHeader>

	<div class="history-toolbar">
		<div class="search-field">
			<Search size={15} aria-hidden="true" />
			<Input
				bind:value={query}
				placeholder="Search session transcripts"
				aria-label="Search transcripts"
			/>
		</div>
		<span
			>{filteredHistory.length} {filteredHistory.length === 1 ? 'transcript' : 'transcripts'}</span
		>
	</div>

	{#if filteredHistory.length > 0}
		<section class="timeline" aria-label="Transcript history">
			{#each filteredHistory as item (item.id)}
				<article>
					<div class="timeline-marker" aria-hidden="true"><span></span></div>
					<div class="entry">
						<header>
							<div class="entry-time"><Clock3 size={13} />{item.timestamp}</div>
							<div class="entry-actions">
								<Button
									variant="ghost"
									size="icon-sm"
									onclick={() => copyItem(item.id, item.text)}
									aria-label="Copy transcript"
								>
									{#if copiedId === item.id}<Check size={14} />{:else}<Copy size={14} />{/if}
								</Button>
								<Button
									variant="ghost"
									size="icon-sm"
									onclick={() => deleteItem(item.id)}
									aria-label="Delete transcript"
								>
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
			{/each}
		</section>
	{:else}
		<section class="empty-state">
			<div class="empty-mark"><Clock3 size={22} strokeWidth={1.5} /></div>
			<h2 class="display-legend">{query ? 'No matching transcript' : 'No transcripts yet'}</h2>
			<p>
				{query
					? 'Try a different phrase or clear the search.'
					: `Start a recording or use ${app.settings?.hotkey_shortcut.toUpperCase() ?? 'the global shortcut'} to create the first entry.`}
			</p>
			{#if query}<Button variant="outline" size="sm" onclick={() => (query = '')}
					>Clear search</Button
				>{/if}
		</section>
	{/if}

	{#if lastDeleted}
		<div class="undo-notice" role="status">
			<span>Transcript deleted from this session.</span>
			<Button variant="outline" size="sm" onclick={undoDelete}>Undo</Button>
		</div>
	{/if}
</div>

<style>
	.history-view {
		display: grid;
		gap: 1rem;
	}

	.history-toolbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		border-top: 1px solid var(--line);
		border-bottom: 1px solid var(--line);
		padding: 0.75rem 0;
	}

	.search-field {
		position: relative;
		width: min(22rem, 100%);
	}

	.search-field > :global(svg) {
		position: absolute;
		top: 50%;
		left: 0.7rem;
		z-index: 1;
		transform: translateY(-50%);
		color: var(--ink-muted);
	}

	.search-field :global(input) {
		padding-left: 2.1rem;
	}

	.history-toolbar > span {
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.6875rem;
	}

	.timeline {
		padding-top: 0.35rem;
	}

	.timeline article {
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

	.timeline article:last-child .timeline-marker::after {
		display: none;
	}

	/* A square node on the timeline, in off-white. Every entry in this list is a
	   completed transcript, so no entry is more urgent than any other and none of
	   them earns the accent — this is the same solid ink square the status pill's
	   `ready` state uses, and it means the same thing. The 2px ground-coloured border
	   is what punches the node out of the line running behind it, so the halo the old
	   marker carried is redundant. */
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

	.entry header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		border-bottom: 1px solid var(--line);
		padding: 0.55rem 0.65rem 0.55rem 0.9rem;
	}

	.entry-time,
	.entry-actions,
	.entry footer {
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

	.entry > p {
		max-width: 72ch;
		margin: 0;
		padding: 1rem 1rem 0.85rem;
		color: var(--ink);
		font-size: 0.875rem;
		line-height: 1.6;
	}

	.entry footer {
		gap: 0.7rem;
		flex-wrap: wrap;
		padding: 0 1rem 1rem;
	}

	.entry footer > span {
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.6875rem;
	}

	.entry footer > .request-cost {
		border-left: 1px solid var(--scarlet);
		padding-left: 0.7rem;
		color: var(--ink);
	}

	.empty-state {
		display: grid;
		min-height: 22rem;
		place-items: center;
		align-content: center;
		border: 1px dashed var(--line-strong);
		background: var(--surface-1);
		text-align: center;
	}

	.empty-mark {
		display: grid;
		width: 3.25rem;
		height: 3.25rem;
		place-items: center;
		border: 1px solid var(--line);
		color: var(--ink-muted);
	}

	.empty-state h2 {
		margin: 1rem 0 0.35rem;
		color: var(--ink);
	}

	.empty-state p {
		margin: 0 0 1rem;
		color: var(--ink-dim);
		font-size: 0.75rem;
	}

	.undo-notice {
		position: sticky;
		bottom: 1rem;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		width: min(24rem, 100%);
		margin: 0 0 0 auto;
		border: 1px solid var(--line-strong);
		padding: 0.65rem 0.75rem 0.65rem 0.9rem;
		background: var(--surface-2);
		color: var(--ink-dim);
		font-size: 0.6875rem;
		box-shadow: 0 0.65rem 1.5rem rgba(0, 0, 0, 0.34);
	}

	@media (max-width: 720px) {
		.history-toolbar > span {
			display: none;
		}
	}
</style>
