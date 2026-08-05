<script lang="ts">
	import { AlertTriangle, RefreshCw, ShieldCheck } from '@lucide/svelte';
	import { getSettingsContext } from '$lib/settings/context.svelte';

	const settings = getSettingsContext();
	let saveState = $derived(settings.persistence.saveState);
	let label = $derived(
		saveState === 'failed'
			? 'Changes not saved'
			: saveState === 'pending'
				? 'Waiting to save…'
				: saveState === 'saving'
					? 'Saving changes…'
					: 'All changes saved'
	);
</script>

<span class:failed={saveState === 'failed'} class="saved-state" aria-live="polite">
	{#if saveState === 'failed'}
		<AlertTriangle size={14} />
	{:else if saveState === 'saved'}
		<ShieldCheck size={14} />
	{:else}
		<span class:spin={saveState === 'saving'}><RefreshCw size={14} /></span>
	{/if}
	{label}
</span>
