<script lang="ts">
	import { AlertTriangle, RefreshCw, ShieldCheck } from '@lucide/svelte';
	import { getSettingsContext } from './_state/context.svelte';
	import { cn } from '$lib/utils';

	const settings = getSettingsContext();
	let saveState = $derived(settings.persistence.saveState);
	let label = $derived(
		saveState === 'failed'
			? 'Changes not saved'
			: saveState === 'pending' && settings.persistence.blockedReason
				? 'Verification required before save'
				: saveState === 'pending'
					? 'Waiting to save…'
					: saveState === 'saving'
						? 'Saving changes…'
						: 'All changes saved'
	);
</script>

<div class="grid justify-items-end gap-[0.35rem] max-[720px]:justify-items-start">
	<span
		class={cn(
			'inline-flex items-center gap-[0.45rem] text-[0.6875rem] text-ink-muted',
			saveState === 'failed' && 'text-scarlet-lamp'
		)}
		aria-live="polite"
	>
		{#if saveState === 'failed'}
			<AlertTriangle size={14} />
		{:else if saveState === 'saved'}
			<ShieldCheck size={14} />
		{:else}
			<span
				class={cn(
					'inline-flex',
					saveState === 'saving' && 'animate-spin motion-reduce:animate-none'
				)}><RefreshCw size={14} /></span
			>
		{/if}
		{label}
	</span>
	{#if settings.app.captureLocked}
		<span
			class="inline-flex items-center gap-[0.35rem] text-[0.6875rem] text-ink-muted"
			role="status"
		>
			<RefreshCw size={13} />
			Settings locked during {settings.app.processing ? 'finalization' : 'recording'}
		</span>
	{/if}
</div>
