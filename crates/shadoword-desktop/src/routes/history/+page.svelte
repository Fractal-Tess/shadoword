<script lang="ts">
	import { Clock3, Trash2 } from '@lucide/svelte';
	import { SurfaceHeader } from '$lib/components/ui/surface-header';
	import { Button } from '$lib/components/ui/button';
	import HistoryEntry from './HistoryEntry.svelte';
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';
	import type { HistoryRecord } from '$lib/types';
	import { tick } from 'svelte';

	const app = useDesktopShell().app;
	let history = $derived(app.history);
	let copiedId = $state<string | null>(null);
	let lastDeleted = $state<{ record: HistoryRecord; index: number } | null>(null);
	let announcement = $state('');
	let undoButton = $state<HTMLButtonElement | null>(null);

	const copyItem = async (id: string, text: string) => {
		await navigator.clipboard?.writeText(text);
		copiedId = id;
	};

	const deleteItem = async (id: string) => {
		const index = history.findIndex((item) => item.id === id);
		if (index < 0) return;
		lastDeleted = { record: history[index], index };
		app.setHistory(history.filter((item) => item.id !== id));
		announcement = 'Transcript deleted. Undo is available.';
		await tick();
		undoButton?.focus();
	};

	const undoDelete = async () => {
		const deleted = lastDeleted;
		if (!deleted) return;
		const restored = [...history];
		restored.splice(deleted.index, 0, deleted.record);
		app.setHistory(restored);
		lastDeleted = null;
		announcement = 'Transcript restored.';
		await tick();
		document
			.querySelector<HTMLButtonElement>(`[data-history-delete="${CSS.escape(deleted.record.id)}"]`)
			?.focus();
	};
</script>

<svelte:head>
	<title>History · Shadoword</title>
</svelte:head>

<div class="grid gap-4">
	<SurfaceHeader
		title="Everything you have said."
		description="Transcripts are kept on this machine across restarts. Review, copy, or remove them."
	>
		{#snippet actions()}
			<span class="font-mono text-[0.625rem] text-ink-muted">
				{history.length}
				{history.length === 1 ? 'transcript' : 'transcripts'}
			</span>
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

	{#if history.length > 0}
		<section aria-label="Transcript history">
			{#each history as item (item.id)}
				<HistoryEntry
					{item}
					copied={copiedId === item.id}
					onCopy={() => copyItem(item.id, item.text)}
					onDelete={() => deleteItem(item.id)}
				/>
			{/each}
		</section>
	{:else}
		<section
			class="grid min-h-88 place-items-center content-center border border-dashed border-line-strong bg-plate text-center"
		>
			<div class="grid size-13 place-items-center border border-line text-ink-muted">
				<Clock3 size={22} strokeWidth={1.5} />
			</div>
			<h2
				class="mt-4 mb-[0.35rem] font-display text-lg leading-none font-normal tracking-[0.035em] text-ink uppercase"
			>
				No transcripts yet
			</h2>
			<p class="mt-0 mb-4 text-xs text-ink-dim">
				Start a recording or use
				{app.settings?.hotkey_shortcut.toUpperCase() ?? 'the global shortcut'} to create the first entry.
			</p>
		</section>
	{/if}

	{#if lastDeleted}
		<div
			class="sticky bottom-4 ml-auto flex w-[min(24rem,100%)] items-center justify-between gap-4 border border-line-strong bg-raised py-[0.65rem] pr-3 pl-[0.9rem] text-[0.6875rem] text-ink-dim shadow-[0_0.65rem_1.5rem_rgb(0_0_0/0.34)]"
		>
			<span>Transcript deleted.</span>
			<Button bind:ref={undoButton} variant="outline" size="sm" onclick={undoDelete}>Undo</Button>
		</div>
	{/if}
	<div class="sr-only" role="status" aria-live="polite">{announcement}</div>
</div>
