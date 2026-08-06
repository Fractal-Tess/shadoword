<script lang="ts">
	import CaptureStage from './CaptureStage.svelte';
	import TranscriptSurface from './TranscriptSurface.svelte';
	import TranscribeHeader from './TranscribeHeader.svelte';
	import { endpointLabel, setTranscribeContext } from './context';
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';

	const shell = useDesktopShell();
	const app = shell.app;
	let copied = $state(false);
	let mode = $derived(app.settings?.mode ?? 'remote');
	let transcriptionMode = $derived(
		mode === 'open_router' ? 'batch' : (app.settings?.transcription_mode ?? 'batch')
	);
	let captureBlocked = $derived(
		app.activity === 'booting' ||
			app.activity === 'busy' ||
			!app.settings ||
			(mode === 'open_router'
				? !app.openRouterReady
				: app.activity === 'offline' || !app.overview) ||
			app.captureState === 'error'
	);
	let modelName = $derived.by(() => {
		if (mode === 'open_router') return app.settings?.openrouter_model ?? 'Unselected';
		const path = app.overview?.runtime.model_path;
		return (
			app.overview?.models.find((model) => path?.endsWith(model.filename))?.name ?? 'Unselected'
		);
	});
	let endpointHost = $derived(
		mode === 'open_router' ? 'openrouter.ai' : endpointLabel(app.settings?.remote_endpoint)
	);
	let surfaceTitle = $derived(
		app.recording
			? 'Listening now'
			: app.processing
				? 'Finishing your transcript'
				: app.captureState === 'error'
					? 'Capture needs attention'
					: captureBlocked
						? 'The signal path needs attention'
						: 'Ready when you are'
	);

	setTranscribeContext({
		get app() {
			return app;
		},
		get mode() {
			return mode;
		},
		get transcriptionMode() {
			return transcriptionMode;
		},
		get captureBlocked() {
			return captureBlocked;
		},
		get modelName() {
			return modelName;
		},
		get endpointHost() {
			return endpointHost;
		},
		get surfaceTitle() {
			return surfaceTitle;
		},
		get copied() {
			return copied;
		},
		setCopied: (value) => (copied = value),
		get onOpenSettings() {
			return () => shell.navigate('settings');
		}
	});
</script>

<svelte:head>
	<title>Transcribe · Shadoword</title>
</svelte:head>

<div
	class="grid h-full min-h-0 flex-1 grid-rows-[auto_minmax(18rem,1fr)_minmax(9.5rem,0.56fr)] gap-[0.85rem] [@media(max-height:720px)]:grid-rows-[auto_minmax(13rem,1fr)_minmax(7.5rem,0.5fr)] [@media(max-height:720px)]:gap-[0.65rem]"
>
	<TranscribeHeader />
	<CaptureStage />
	<TranscriptSurface />
</div>
