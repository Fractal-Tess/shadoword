<script lang="ts">
	import { Clock3, Search, Trash2 } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import SurfaceHeader from '$lib/components/SurfaceHeader.svelte';
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';
	import type { HistoryRecord } from '$lib/types';
	import HistoryEntry from './history/HistoryEntry.svelte';

	const app = useDesktopShell().app;
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
				<HistoryEntry
					{item}
					copied={copiedId === item.id}
					onCopy={() => copyItem(item.id, item.text)}
					onDelete={() => deleteItem(item.id)}
				/>
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
		margin-left: auto;
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
