<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { getPoolDraftContext } from './context.svelte';

	const state = getPoolDraftContext();
</script>

<div class="flex min-h-[3.45rem] items-center justify-end gap-4 px-[0.85rem] py-[0.65rem]">
	<div class="mr-auto grid text-[0.6875rem] text-ink-dim" aria-live="polite">
		{#if state.app.poolFeedback}<span>{state.app.poolFeedback}</span>{/if}
		{#if state.localActionError && state.localActionError !== state.app.poolFeedback}<small
				class="text-scarlet-lamp">{state.localActionError}</small
			>{/if}
	</div>
	<Button size="sm" disabled={state.locked} onclick={() => state.applyPool()}
		>{state.app.poolApplyState === 'applying' ? 'Applying…' : 'Apply & reload'}</Button
	>
</div>
