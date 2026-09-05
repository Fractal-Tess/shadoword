<script lang="ts">
	import { RefreshCw } from '@lucide/svelte';
	import { SurfaceHeader } from '$lib/components/ui/surface-header';
	import { Button } from '$lib/components/ui/button';
	import { getModelsContext } from './state.svelte';

	const context = getModelsContext();
</script>

<SurfaceHeader
	title="Inference, without guesswork."
	description={context.mode === 'remote'
		? 'Manage verified model files on the connected Shadoword API.'
		: context.mode === 'local'
			? 'Manage verified model files in the native local Whisper runtime.'
			: 'OpenRouter owns its hosted transcription models; local Whisper runtime controls do not apply.'}
>
	{#snippet actions()}
		<Button
			variant="outline"
			size="sm"
			onclick={() => context.app.refreshOverview()}
			disabled={context.app.activity === 'busy'}
		>
			<RefreshCw size={14} />{context.app.activity === 'busy'
				? 'Refreshing…'
				: context.mode === 'open_router'
					? 'Refresh provider'
					: 'Refresh state'}
		</Button>
	{/snippet}
</SurfaceHeader>
