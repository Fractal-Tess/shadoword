<script lang="ts">
	import { RivetedPlate } from '$lib/components/ui/riveted-plate';
	import { StatusIndicator } from '$lib/components/ui/status-indicator';
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';
	import type { RuntimeState } from '$lib/types';

	const shell = useDesktopShell();
	const app = shell.app;
	let mode = $derived(shell.mode);
	let poolStatus = $derived(app.overview?.status.inference_pool ?? null);
	let modelName = $derived.by(() => {
		if (mode === 'open_router') return app.settings?.openrouter_model ?? 'No model';
		const path = app.overview?.runtime.model_path;
		return app.overview?.models.find((model) => path?.endsWith(model.filename))?.name ?? 'No model';
	});
	let runtimeState = $derived<RuntimeState>(
		app.activity === 'busy'
			? 'loading'
			: mode === 'open_router'
				? app.openRouterReady
					? 'ready'
					: 'offline'
				: (poolStatus?.unhealthy_units ?? 0) > 0
					? 'warning'
					: app.overview
						? 'ready'
						: 'offline'
	);
	let runtimeLabel = $derived(
		app.recording
			? app.recordingTranscriptionMode === 'streaming'
				? 'Streaming'
				: 'Recording'
			: app.processing
				? 'Finalizing'
				: app.activity === 'busy'
					? 'Updating'
					: runtimeState === 'ready'
						? 'Ready'
						: runtimeState === 'warning'
							? 'Degraded'
							: 'Offline'
	);
</script>

<RivetedPlate
	class="mx-[0.85rem] mt-auto mb-[0.85rem] px-[0.95rem] pt-[0.95rem] pb-4 max-[999px]:mx-2 max-[999px]:mt-auto max-[999px]:mb-[0.85rem] max-[999px]:px-1 max-[999px]:py-[0.65rem]"
>
	<div class="mt-[0.2rem] mb-[0.8rem] flex items-center max-[999px]:m-0 max-[999px]:justify-center">
		<StatusIndicator state={runtimeState} label={runtimeLabel} compact active={app.recording} />
	</div>
	<strong
		class="block max-w-full font-display text-[1.375rem] leading-none font-normal tracking-[0.01em] [overflow-wrap:anywhere] text-ink uppercase max-[999px]:hidden"
	>
		{modelName}
	</strong>
</RivetedPlate>
