<script lang="ts">
	import { RefreshCw } from '@lucide/svelte';
	import SurfaceHeader from '$lib/components/SurfaceHeader.svelte';
	import { Button } from '$lib/components/ui/button';
	import { getModelsContext } from './context';

	const context = getModelsContext();
</script>

<SurfaceHeader
	title="Inference, without guesswork."
	description={context.mode === 'remote'
		? 'Manage the model and accelerator on the connected Shadoword API.'
		: context.mode === 'local'
			? 'Manage models and acceleration in the native local Whisper runtime.'
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
