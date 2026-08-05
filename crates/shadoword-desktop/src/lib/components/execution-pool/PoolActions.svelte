<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { getPoolDraftContext } from '$lib/execution-pool/context.svelte';

	const state = getPoolDraftContext();
</script>

<div class="editor-actions">
	<div class="feedback" aria-live="polite">
		{#if state.app.poolFeedback}<span>{state.app.poolFeedback}</span>{/if}
		{#if state.localActionError && state.localActionError !== state.app.poolFeedback}<small
				>{state.localActionError}</small
			>{/if}
	</div>
	{#if state.explicit}
		<Button
			variant="outline"
			size="sm"
			disabled={state.locked || state.app.poolValidationState === 'validating'}
			onclick={() => state.validate()}
			>{state.app.poolValidationState === 'validating' ? 'Validating…' : 'Validate pool'}</Button
		>
	{/if}
	<Button
		size="sm"
		disabled={state.locked || (state.explicit && state.app.poolValidationState !== 'valid')}
		onclick={() => state.applyPool()}
		>{state.app.poolApplyState === 'applying' ? 'Applying…' : 'Apply & reload'}</Button
	>
</div>

<style>
	.editor-actions {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 1rem;
		min-height: 3.45rem;
		padding: 0.65rem 0.85rem;
	}

	.feedback {
		display: grid;
		margin-right: auto;
		color: var(--ink-dim);
		font-size: 0.6875rem;
	}

	.feedback small {
		color: var(--scarlet-lamp);
	}
</style>
