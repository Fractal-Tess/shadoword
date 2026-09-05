<script lang="ts">
	import { Database, HardDrive, MemoryStick } from '@lucide/svelte';
	import { StatusIndicator } from '$lib/components/ui/status-indicator';
	import { Switch } from '$lib/components/ui/switch';
	import { formatBytes } from '$lib/display';
	import { getModelsContext } from './state.svelte';

	const context = getModelsContext();
	let selectedModel = $derived(
		context.models.find((model) => model.id === context.selectedId) ?? null
	);
	let installedBytes = $derived(
		context.app.overview?.model_storage?.total_bytes ??
			context.models
				.filter((model) => model.installed)
				.reduce((total, model) => total + model.size_bytes, 0)
	);
	let installedCount = $derived(
		context.app.overview?.model_storage?.installed_model_count ??
			context.models.filter((model) => model.installed).length
	);
	let usedDeviceMemory = $derived(
		context.gpuDevices.reduce(
			(total, device) => total + Math.max(0, device.total_vram - device.free_vram),
			0
		)
	);
	let modelPath = $derived(context.runtime?.model_path || 'No model selected');
	let storagePath = $derived.by(() => {
		if (context.app.overview?.model_storage?.directory) {
			return context.app.overview.model_storage.directory;
		}
		if (!context.runtime?.model_path) return 'No model directory available';
		const pieces = context.runtime.model_path.split('/');
		pieces.pop();
		return pieces.join('/') || '/';
	});
</script>

<section class="border border-line bg-plate" aria-labelledby="runtime-title">
	<div
		class="grid grid-cols-[2.6rem_minmax(0,1fr)_auto] items-center gap-[0.8rem] border-b border-line p-4"
	>
		<div class="grid size-[2.6rem] place-items-center border border-scarlet text-scarlet-lamp">
			<Database size={19} />
		</div>
		<div>
			<h2
				id="runtime-title"
				class="m-0 font-display text-[1.125rem] leading-none font-normal tracking-[0.035em] text-ink uppercase"
			>
				{context.mode === 'remote' ? 'Shadoword API model library' : 'Local model library'}
			</h2>
			<p class="mt-1 mb-0 text-[0.6875rem] leading-[1.4] text-ink-muted">
				Verified Whisper files available to this execution target.
			</p>
		</div>
		<StatusIndicator
			state={context.app.activity === 'busy'
				? 'loading'
				: context.app.overview?.status.model_loaded
					? 'ready'
					: 'warning'}
			label={context.app.activity === 'busy'
				? 'Updating'
				: context.app.overview?.status.model_loaded
					? 'Model ready'
					: 'Model unloaded'}
		/>
	</div>

	<div class="grid grid-cols-3 border-b border-line max-[720px]:grid-cols-1">
		<div
			class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-2 gap-y-[0.3rem] px-4 py-[0.85rem]"
		>
			<HardDrive class="row-span-2 text-ink-muted" size={16} />
			<span class="text-[0.6rem] tracking-[0.06em] text-ink-muted uppercase"> Selected model </span>
			<strong class="text-[0.72rem] text-ink">{selectedModel?.name ?? 'Custom model'}</strong>
		</div>
		<div
			class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-2 gap-y-[0.3rem] border-l border-line px-4 py-[0.85rem] max-[720px]:border-t max-[720px]:border-l-0"
		>
			<Database class="row-span-2 text-ink-muted" size={16} />
			<span class="text-[0.6rem] tracking-[0.06em] text-ink-muted uppercase">
				Catalog footprint
			</span>
			<strong class="text-[0.72rem] text-ink">
				{formatBytes(installedBytes)} · {installedCount} installed
			</strong>
		</div>
		<div
			class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-2 gap-y-[0.3rem] border-l border-line px-4 py-[0.85rem] max-[720px]:border-t max-[720px]:border-l-0"
		>
			<MemoryStick class="row-span-2 text-ink-muted" size={16} />
			<span class="text-[0.6rem] tracking-[0.06em] text-ink-muted uppercase">
				Device memory in use
			</span>
			<strong class="text-[0.72rem] text-ink">
				{context.gpuDevices.length > 0 ? formatBytes(usedDeviceMemory) : 'CPU runtime'}
			</strong>
		</div>
	</div>

	<div class="grid gap-3 border-b border-line px-4 py-[0.9rem]">
		<div class="grid min-w-0 gap-[0.35rem]">
			<span class="text-[0.6rem] tracking-[0.06em] text-ink-muted uppercase">Model storage</span>
			<code
				class="border border-line bg-night px-[0.65rem] py-[0.55rem] text-[0.62rem] leading-[1.4] [overflow-wrap:anywhere] text-ink-dim"
				>{storagePath}</code
			>
		</div>
		<div class="grid min-w-0 gap-[0.35rem]">
			<span class="text-[0.6rem] tracking-[0.06em] text-ink-muted uppercase">
				Active model file
			</span>
			<code
				class="border border-line bg-night px-[0.65rem] py-[0.55rem] text-[0.62rem] leading-[1.4] [overflow-wrap:anywhere] text-ink-dim"
				>{modelPath}</code
			>
		</div>
	</div>

	<div class="flex items-center justify-between gap-4 px-4 py-[0.9rem]">
		<div class="grid">
			<strong class="text-[0.72rem] text-ink">Preload selected model</strong>
			<span class="mt-1 text-[0.6875rem] leading-[1.4] text-ink-muted">
				Keep weights ready after startup instead of loading on first capture.
			</span>
		</div>
		<Switch
			checked={context.preload}
			disabled={context.controlsLocked}
			onclick={() => context.updateRuntime({ preload_on_startup: !context.preload })}
			aria-label="Preload selected model"
		/>
	</div>
</section>
