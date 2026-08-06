<script lang="ts">
	import { AlertTriangle, RefreshCw } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import { cn } from '$lib/utils';
	import { getModelsContext } from './context';

	const context = getModelsContext();

	function retryDownload() {
		const failedDownload = context.failedDownload;
		if (failedDownload) void context.app.startDownload(failedDownload.model_id);
	}
</script>

{#if context.failedDownload || context.app.captureLocked}
	<div
		class={cn(
			'grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-[0.8rem] border border-line-strong bg-plate px-4 py-[0.8rem] text-ink-dim',
			context.failedDownload && 'border-l-2 border-scarlet text-scarlet-lamp'
		)}
		role={context.failedDownload ? 'alert' : 'status'}
	>
		{#if context.failedDownload}<AlertTriangle size={17} />{:else}<RefreshCw size={17} />{/if}
		<div class="grid gap-[0.2rem]">
			<strong class="text-xs text-ink"
				>{context.failedDownload
					? 'Model download failed'
					: context.app.processing
						? 'Controls locked during finalization'
						: 'Controls locked during capture'}</strong
			>
			<span class="text-[0.6875rem] text-ink-dim"
				>{context.failedDownload?.error ??
					'Finish the current recording before changing the active runtime.'}</span
			>
		</div>
		{#if context.failedDownload}
			<Button variant="outline" size="sm" onclick={retryDownload}>Retry</Button>
		{/if}
	</div>
{/if}
