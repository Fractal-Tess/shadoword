<script lang="ts">
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';
	import type { WhisperAccelerator } from '$lib/bindings';
	import { formatBytes } from '$lib/display';
	import CustomModels from './models/CustomModels.svelte';
	import LegacyExecutionSettings from './models/LegacyExecutionSettings.svelte';
	import ModelCatalog from './models/ModelCatalog.svelte';
	import ModelNotice from './models/ModelNotice.svelte';
	import ModelsExecutionPool from './models/ModelsExecutionPool.svelte';
	import ModelsHeader from './models/ModelsHeader.svelte';
	import RuntimeBand from './models/RuntimeBand.svelte';
	import UnsupportedProvider from './models/UnsupportedProvider.svelte';
	import { setModelsContext } from './models/context';

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
	let accelerator = $derived(runtime?.whisper_accelerator ?? 'auto');
	let gpuDevice = $derived(runtime?.whisper_gpu_device ?? -1);
	let gpuDevices = $derived(app.overview?.status.available_gpu_devices ?? []);
	let gpuDeviceOptions = $derived([
		{ value: '-1', label: 'Automatic', detail: 'Best available device' },
		...gpuDevices.map((device) => ({
			value: String(device.id),
			label: `GPU ${device.id} · ${device.name}`,
			detail: formatBytes(device.total_vram)
		}))
	]);
	let customPath = $derived(runtime?.model_path ?? app.settings?.model_path ?? '');
	let failedDownload = $derived(
		Object.values(app.downloads).find((download) => download.state === 'failed') ?? null
	);

	const updateRuntime = async (changes: {
		preload_on_startup?: boolean;
		whisper_accelerator?: WhisperAccelerator;
		whisper_gpu_device?: number;
	}) => {
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
		get accelerator() {
			return accelerator;
		},
		get gpuDevice() {
			return gpuDevice;
		},
		get gpuDevices() {
			return gpuDevices;
		},
		get gpuDeviceOptions() {
			return gpuDeviceOptions;
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

<div class="models-view">
	<ModelsHeader />
	{#if mode === 'open_router'}
		<UnsupportedProvider />
	{:else}
		<RuntimeBand />
		<ModelsExecutionPool />
		<LegacyExecutionSettings />
		<ModelNotice />
		<ModelCatalog />
		<CustomModels />
	{/if}
</div>

<style>
	.models-view {
		display: grid;
		gap: 1rem;
	}
</style>
