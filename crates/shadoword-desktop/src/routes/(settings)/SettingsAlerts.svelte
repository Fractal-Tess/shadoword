<script lang="ts">
	import { AlertTriangle, X } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import { getSettingsContext } from './_state/context.svelte';

	const settings = getSettingsContext();
</script>

{#if settings.persistence.saveState === 'failed'}
	<div
		class="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-[0.8rem] border border-l-2 border-scarlet bg-plate px-4 py-[0.8rem] text-scarlet-lamp"
		role="alert"
	>
		<AlertTriangle size={17} />
		<div class="grid min-w-0 gap-[0.2rem]">
			<strong class="text-xs text-ink">Shadoword could not save desktop.json</strong>
			<span class="text-[0.6875rem] break-words text-ink-dim">
				{settings.persistence.error}
			</span>
		</div>
		<Button
			variant="outline"
			size="sm"
			onclick={() => void settings.persistence.save()}
			disabled={settings.locked}>Retry save</Button
		>
	</div>
{/if}

{#if settings.persistence.actionError}
	<div
		class="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-[0.8rem] border border-scarlet bg-plate px-4 py-[0.8rem] text-scarlet-lamp"
		role="alert"
	>
		<AlertTriangle size={17} />
		<div class="grid min-w-0 gap-[0.2rem]">
			<strong class="text-xs text-ink">Credential action failed</strong>
			<span class="text-[0.6875rem] break-words text-ink-dim">
				{settings.persistence.actionError}
			</span>
		</div>
		<Button
			variant="ghost"
			size="icon-sm"
			onclick={() => settings.persistence.setActionError('')}
			aria-label="Dismiss credential error"
		>
			<X size={14} />
		</Button>
	</div>
{/if}
