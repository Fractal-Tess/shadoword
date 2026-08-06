<script lang="ts">
	import CustomModels from './CustomModels.svelte';
	import ModelCatalog from './ModelCatalog.svelte';
	import ModelNotice from './ModelNotice.svelte';
	import ModelsHeader from './ModelsHeader.svelte';
	import RuntimeBand from './RuntimeBand.svelte';
	import UnsupportedProvider from './UnsupportedProvider.svelte';
	import { setModelsContext } from './context';
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';

	const app = useDesktopShell().app;
	let mode = $derived(app.settings?.mode ?? 'remote');
	let runtime = $derived(app.overview?.runtime ?? null);
	let models = $derived(app.overview?.models ?? []);
	let selectedId = $derived.by(() => {
		const path = runtime?.model_path;
		return models.find((model) => path?.endsWith(model.filename))?.id ?? null;
	});
	let controlsLocked = $derived(app.poolMutationLocked || !runtime);
	let preload = $derived(runtime?.preload_on_startup ?? false);
	let gpuDevices = $derived(app.overview?.status.available_gpu_devices ?? []);
	let customPath = $derived(runtime?.model_path ?? app.settings?.model_path ?? '');
	let failedDownload = $derived(
		Object.values(app.downloads).find((download) => download.state === 'failed') ?? null
	);

	const updateRuntime = async (changes: { preload_on_startup?: boolean }) => {
		if (!runtime || mode === 'open_router') return;
		try {
			await app.updateRuntime({ ...runtime, ...changes });
		} catch {
			// App state exposes the native error and preserves the active runtime.
		}
	};

	const useCustomPath = async () => {
		if (!runtime || mode !== 'local') return;
		try {
			await app.updateRuntime({ ...runtime, model_path: customPath.trim() });
		} catch {
			// The global runtime alert provides retry context.
		}
	};

	setModelsContext({
		get app() {
			return app;
		},
		get mode() {
			return mode;
		},
		get runtime() {
			return runtime;
		},
		get models() {
			return models;
		},
		get selectedId() {
			return selectedId;
		},
		get controlsLocked() {
			return controlsLocked;
		},
		get preload() {
			return preload;
		},
		get gpuDevices() {
			return gpuDevices;
		},
		get customPath() {
			return customPath;
		},
		get failedDownload() {
			return failedDownload;
		},
		updateRuntime,
		setCustomPath: (value) => (customPath = value),
		useCustomPath
	});
</script>

<svelte:head>
	<title>Models · Shadoword</title>
</svelte:head>

<div class="grid gap-4">
	<ModelsHeader />
	{#if mode === 'open_router'}
		<UnsupportedProvider />
	{:else}
		<RuntimeBand />
		<ModelNotice />
		<ModelCatalog />
		<CustomModels />
	{/if}
</div>
