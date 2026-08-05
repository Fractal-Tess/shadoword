<script lang="ts">
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';
	import CaptureStage from './transcribe/CaptureStage.svelte';
	import TranscriptSurface from './transcribe/TranscriptSurface.svelte';
	import TranscribeHeader from './transcribe/TranscribeHeader.svelte';
	import { endpointLabel, setTranscribeContext } from './transcribe/context';

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
				? !app.settings.openrouter_key_configured
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

<div class="transcribe-view">
	<TranscribeHeader />
	<CaptureStage />
	<TranscriptSurface />
</div>

<style>
	.transcribe-view {
		display: grid;
		grid-template-rows: auto minmax(18rem, 1fr) minmax(9.5rem, 0.56fr);
		gap: 0.85rem;
		height: 100%;
		min-height: 0;
	}

	@media (max-height: 720px) {
		.transcribe-view {
			grid-template-rows: auto minmax(13rem, 1fr) minmax(7.5rem, 0.5fr);
			gap: 0.65rem;
		}
	}
</style>
