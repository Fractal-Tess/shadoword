<script lang="ts">
	import { StatusIndicator } from '$lib/components/ui/status-indicator';
	import { SurfaceHeader } from '$lib/components/ui/surface-header';
	import { getTranscribeContext } from './state.svelte';

	const context = getTranscribeContext();
</script>

<SurfaceHeader
	title={context.surfaceTitle}
	description="Private local speech to text, your self-hosted API, or direct OpenRouter transcription."
>
	{#snippet actions()}
		<StatusIndicator
			state={context.app.recording || context.app.processing
				? 'loading'
				: context.captureBlocked
					? 'offline'
					: context.mode === 'open_router' || context.app.overview?.status.model_loaded
						? 'ready'
						: 'warning'}
			label={context.app.recording
				? 'Recording'
				: context.app.processing
					? 'Finalizing'
					: context.captureBlocked
						? 'Action required'
						: context.mode === 'open_router'
							? 'Provider ready'
							: context.app.overview?.status.model_loaded
								? 'Model ready'
								: 'Loads on demand'}
		/>
	{/snippet}
</SurfaceHeader>
