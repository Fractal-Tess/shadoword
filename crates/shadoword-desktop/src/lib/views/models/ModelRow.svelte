<script lang="ts">
	import { Check, CloudDownload, HardDrive, X } from '@lucide/svelte';
	import type { ModelInfoDto } from '$lib/bindings';
	import { formatBytes, downloadPercent } from '$lib/display';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Progress } from '$lib/components/ui/progress';
	import { getModelsContext } from './context';

	let { model }: { model: ModelInfoDto } = $props();
	const context = getModelsContext();
	let download = $derived(context.app.downloads[model.id]);
</script>

<article
	class="model-row"
	class:selected={context.selectedId === model.id}
	class:locked={context.controlsLocked}
>
	<div class="model-symbol" aria-hidden="true">
		{#if model.installed}<HardDrive size={18} />{:else}<CloudDownload size={18} />{/if}
	</div>
	<div class="model-copy">
		<div class="model-name">
			<h3 class="display-legend">{model.name}</h3>
			{#if model.recommended}<Badge variant="outline">Recommended</Badge>{/if}
			{#if context.selectedId === model.id}
				<Badge class="selected-badge"><Check size={11} />Selected</Badge>
			{/if}
		</div>
		<p>{model.description}</p>
		<div class="model-meta">
			<span>{formatBytes(model.size_bytes)}</span>
			<span>
				{model.installed
					? 'Installed'
					: context.mode === 'remote'
						? 'Not on API'
						: 'Not on this machine'}
			</span>
		</div>
		{#if download && download.state !== 'failed'}
			<div class="download-progress">
				<Progress value={downloadPercent(download)} />
				<span>{downloadPercent(download)}% · {download.state}</span>
			</div>
		{/if}
	</div>
	<div class="model-actions">
		{#if context.app.downloadWatching[model.id]}
			<Button
				variant="outline"
				size="sm"
				onclick={() => context.app.stopWatchingDownload(model.id)}
			>
				<X size={13} />Stop watching
			</Button>
		{:else if model.installed}
			<Button
				variant={context.selectedId === model.id ? 'ghost' : 'outline'}
				size="sm"
				disabled={context.selectedId === model.id || context.controlsLocked}
				onclick={() => context.app.selectModel(model.id)}
			>
				{context.selectedId === model.id
					? 'In use'
					: context.mode === 'remote'
						? 'Select on API'
						: 'Select locally'}
			</Button>
		{:else}
			<Button
				size="sm"
				disabled={context.controlsLocked}
				onclick={() => context.app.startDownload(model.id)}
			>
				<CloudDownload size={14} />
				{context.mode === 'remote' ? 'Download to API' : 'Download'}
			</Button>
		{/if}
	</div>
</article>

<style>
	.model-row {
		display: grid;
		grid-template-columns: 2.5rem minmax(0, 1fr) auto;
		align-items: center;
		gap: 0.9rem;
		min-height: 6.4rem;
		padding: 1rem;
		transition: background-color 140ms ease;
	}

	.model-row:hover {
		background: var(--surface-2);
	}

	.model-row.selected {
		background: var(--surface-2);
		box-shadow: inset 2px 0 0 var(--scarlet);
	}

	.model-symbol {
		display: grid;
		width: 2.35rem;
		height: 2.35rem;
		place-items: center;
		border: 1px solid var(--line);
		color: var(--ink-muted);
	}

	.selected .model-symbol {
		border-color: var(--scarlet);
		color: var(--scarlet-lamp);
	}

	.model-copy {
		min-width: 0;
	}

	.model-name,
	.model-meta,
	.download-progress {
		display: flex;
		align-items: center;
		gap: 0.45rem;
	}

	.model-name h3 {
		margin: 0;
		color: var(--ink);
	}

	.model-copy > p {
		margin: 0.35rem 0;
		color: var(--ink-dim);
		font-size: 0.72rem;
		line-height: 1.45;
	}

	.model-meta span,
	.download-progress span {
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.6875rem;
	}

	.model-meta span + span::before {
		content: '·';
		margin-right: 0.45rem;
	}

	.download-progress {
		max-width: 14rem;
		margin-top: 0.65rem;
	}

	.download-progress :global([data-slot='progress']) {
		height: 0.25rem;
	}

	.model-actions {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		padding-left: 1rem;
	}

	.locked {
		opacity: 0.58;
	}

	:global(.selected-badge) {
		border-color: var(--scarlet);
		background: transparent;
		color: var(--scarlet-lamp);
	}

	@media (max-width: 720px) {
		.model-row {
			grid-template-columns: 2.5rem minmax(0, 1fr);
		}

		.model-actions {
			grid-column: 2;
			padding: 0;
		}
	}
</style>
