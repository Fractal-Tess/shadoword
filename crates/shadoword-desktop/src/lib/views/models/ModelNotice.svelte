<script lang="ts">
	import { AlertTriangle, RefreshCw } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import { getModelsContext } from './context';

	const context = getModelsContext();

	function retryDownload() {
		const failedDownload = context.failedDownload;
		if (failedDownload) void context.app.startDownload(failedDownload.model_id);
	}
</script>

{#if context.failedDownload || context.app.captureLocked}
	<div
		class="model-notice"
		class:error={Boolean(context.failedDownload)}
		role={context.failedDownload ? 'alert' : 'status'}
	>
		{#if context.failedDownload}<AlertTriangle size={17} />{:else}<RefreshCw size={17} />{/if}
		<div>
			<strong
				>{context.failedDownload
					? 'Model download failed'
					: context.app.processing
						? 'Controls locked during finalization'
						: 'Controls locked during capture'}</strong
			>
			<span
				>{context.failedDownload?.error ??
					'Finish the current recording before changing the active runtime.'}</span
			>
		</div>
		{#if context.failedDownload}
			<Button variant="outline" size="sm" onclick={retryDownload}>Retry</Button>
		{/if}
	</div>
{/if}

<style>
	.model-notice {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: center;
		gap: 0.8rem;
		border: 1px solid var(--line-strong);
		padding: 0.8rem 1rem;
		background: var(--surface-1);
		color: var(--ink-dim);
	}

	.model-notice.error {
		border-color: var(--scarlet);
		border-left-width: 2px;
		color: var(--scarlet-lamp);
	}

	.model-notice > div {
		display: grid;
		gap: 0.2rem;
	}

	.model-notice strong {
		color: var(--ink);
		font-size: 0.75rem;
	}

	.model-notice span {
		color: var(--ink-dim);
		font-size: 0.6875rem;
	}
</style>
