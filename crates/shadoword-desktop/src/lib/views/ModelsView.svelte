<script lang="ts">
	import {
		AlertTriangle,
		Check,
		CloudDownload,
		Cpu,
		HardDrive,
		RefreshCw,
		Server,
		X
	} from '@lucide/svelte';
	import type { DesktopAppState } from '$lib/app-state.svelte';
	import type { WhisperAccelerator } from '$lib/bindings';
	import ExecutionPool from '$lib/components/ExecutionPool.svelte';
	import { downloadPercent, formatBytes } from '$lib/display';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Progress } from '$lib/components/ui/progress';
	import { Switch } from '$lib/components/ui/switch';
	import SurfaceHeader from '$lib/components/SurfaceHeader.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { isExplicitPool } from '$lib/inference-pool';

	let { app }: { app: DesktopAppState } = $props();
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
	let customPath = $derived(runtime?.model_path ?? app.settings?.model_path ?? '');
	let failedDownload = $derived(
		Object.values(app.downloads).find((download) => download.state === 'failed') ?? null
	);

	const updateRuntime = async (changes: {
		preload_on_startup?: boolean;
		whisper_accelerator?: WhisperAccelerator;
		whisper_gpu_device?: number;
	}) => {
		if (!runtime) return;
		try {
			await app.updateRuntime({ ...runtime, ...changes });
		} catch {
			// The app state exposes the native error and preserves the active runtime.
		}
	};

	const useCustomPath = async () => {
		if (!runtime) return;
		try {
			await app.updateRuntime({ ...runtime, model_path: customPath.trim() });
		} catch {
			// The global runtime alert provides retry context.
		}
	};
</script>

<div class="models-view">
	<SurfaceHeader
		kicker="Models"
		title="Inference, without guesswork."
		description={mode === 'remote'
			? 'Manage the model and accelerator on the connected Shadoword API.'
			: 'Manage models and acceleration in the native local Whisper runtime.'}
	>
		{#snippet actions()}
			<Button
				variant="outline"
				size="sm"
				onclick={() => app.refreshOverview()}
				disabled={app.activity === 'busy'}
			>
				<RefreshCw size={14} />{app.activity === 'busy' ? 'Refreshing…' : 'Refresh state'}
			</Button>
		{/snippet}
	</SurfaceHeader>

	<section class="runtime-band" aria-labelledby="runtime-title">
		<div class="runtime-identity">
			<div class="runtime-icon">
				{#if mode === 'remote'}<Server size={19} />{:else}<Cpu size={19} />{/if}
			</div>
			<div>
				<span>Active runtime</span>
				<h2 id="runtime-title" class="display-legend">
					{mode === 'remote' ? 'Remote API' : 'Local machine'}
				</h2>
				<p>
					{mode === 'remote'
						? (app.settings?.remote_endpoint ?? 'No endpoint configured')
						: (runtime?.model_path ?? 'No local model selected')}
				</p>
			</div>
		</div>
		<div class="runtime-control">
			<div>
				<span>Preload model</span>
				<small>Keep the selected model ready after startup.</small>
			</div>
			<Switch
				checked={preload}
				disabled={controlsLocked}
				onclick={() => updateRuntime({ preload_on_startup: !preload })}
				aria-label="Preload selected model"
			/>
		</div>
		<div class="runtime-health">
			<StatusPill
				state={app.activity === 'busy'
					? 'loading'
					: app.overview?.status.model_loaded
						? 'ready'
						: 'warning'}
				label={app.activity === 'busy'
					? 'Applying runtime'
					: app.overview?.status.model_loaded
						? 'Model ready'
						: 'Model unloaded'}
			/>
			<span>{app.overview?.status.engine ?? 'API not connected'}</span>
		</div>
	</section>

	{#if runtime}
		{#key runtime.generation ?? `${mode}-${runtime.model_path}`}
			<ExecutionPool {app} {runtime} {gpuDevices} />
		{/key}
	{/if}

	{#if !isExplicitPool(runtime)}
		<section class="execution-settings" aria-label="Legacy single-unit execution settings">
			<div>
				<label for="accelerator">Whisper accelerator</label>
				<span>Auto uses the best backend compiled into this runtime.</span>
			</div>
			<select
				id="accelerator"
				value={accelerator}
				disabled={controlsLocked}
				onchange={(event) =>
					updateRuntime({
						whisper_accelerator: event.currentTarget.value as WhisperAccelerator
					})}
			>
				<option value="auto">Auto</option><option value="gpu">GPU</option><option value="cpu"
					>CPU</option
				>
			</select>
			<div>
				<label for="gpu-device">GPU device</label>
				<span>Select a specific device or let Shadoword choose.</span>
			</div>
			<select
				id="gpu-device"
				value={gpuDevice}
				disabled={controlsLocked || accelerator === 'cpu'}
				onchange={(event) =>
					updateRuntime({ whisper_gpu_device: Number(event.currentTarget.value) })}
			>
				<option value={-1}>Auto (best available)</option>
				{#each gpuDevices as device (device.id)}
					<option value={device.id}
						>GPU {device.id} · {device.name} · {formatBytes(device.total_vram)}</option
					>
				{/each}
			</select>
		</section>
	{/if}

	{#if failedDownload || app.captureLocked}
		<div
			class:error={Boolean(failedDownload)}
			class="model-notice"
			role={failedDownload ? 'alert' : 'status'}
		>
			{#if failedDownload}<AlertTriangle size={17} />{:else}<RefreshCw size={17} />{/if}
			<div>
				<strong
					>{failedDownload
						? 'Model download failed'
						: app.processing
							? 'Controls locked during finalization'
							: 'Controls locked during capture'}</strong
				>
				<span
					>{failedDownload?.error ??
						'Finish the current recording before changing the active runtime.'}</span
				>
			</div>
			{#if failedDownload}<Button
					variant="outline"
					size="sm"
					onclick={() => app.startDownload(failedDownload.model_id)}>Retry</Button
				>{/if}
		</div>
	{/if}

	<section class="catalog" aria-labelledby="catalog-title">
		<header>
			<div>
				<span>Verified catalog</span>
				<h2 id="catalog-title" class="display-legend">Whisper models</h2>
			</div>
			<p>Downloads are checksum-verified before use.</p>
		</header>

		{#if models.length > 0}
			<div class="model-list">
				{#each models as model (model.id)}
					<article class:selected={selectedId === model.id} class:locked={controlsLocked}>
						<div class="model-symbol" aria-hidden="true">
							{#if model.installed}<HardDrive size={18} />{:else}<CloudDownload size={18} />{/if}
						</div>
						<div class="model-copy">
							<div class="model-name">
								<h3 class="display-legend">{model.name}</h3>
								{#if model.recommended}<Badge variant="outline">Recommended</Badge>{/if}
								{#if selectedId === model.id}<Badge class="selected-badge"
										><Check size={11} />Selected</Badge
									>{/if}
							</div>
							<p>{model.description}</p>
							<div class="model-meta">
								<span>{formatBytes(model.size_bytes)}</span>
								<span
									>{model.installed
										? 'Installed'
										: mode === 'remote'
											? 'Not on API'
											: 'Not on this machine'}</span
								>
							</div>
							{#if app.downloads[model.id] && app.downloads[model.id].state !== 'failed'}
								<div class="download-progress">
									<Progress value={downloadPercent(app.downloads[model.id])} />
									<span
										>{downloadPercent(app.downloads[model.id])}% · {app.downloads[model.id]
											.state}</span
									>
								</div>
							{/if}
						</div>
						<div class="model-actions">
							{#if app.downloadWatching[model.id]}
								<Button
									variant="outline"
									size="sm"
									onclick={() => app.stopWatchingDownload(model.id)}
								>
									<X size={13} />Stop watching
								</Button>
							{:else if model.installed}
								<Button
									variant={selectedId === model.id ? 'ghost' : 'outline'}
									size="sm"
									disabled={selectedId === model.id || controlsLocked}
									onclick={() => app.selectModel(model.id)}
								>
									{selectedId === model.id
										? 'In use'
										: mode === 'remote'
											? 'Select on API'
											: 'Select locally'}
								</Button>
							{:else}
								<Button
									size="sm"
									disabled={controlsLocked}
									onclick={() => app.startDownload(model.id)}
								>
									<CloudDownload size={14} />
									{mode === 'remote' ? 'Download to API' : 'Download'}
								</Button>
							{/if}
						</div>
					</article>
				{/each}
			</div>
		{:else}
			<div class="custom-row">
				<AlertTriangle size={17} />
				<div>
					<strong>No model catalog available</strong><span
						>Refresh the {mode === 'remote' ? 'remote API' : 'local runtime'}.</span
					>
				</div>
			</div>
		{/if}
	</section>

	<section class="custom-models" aria-labelledby="custom-title">
		<header>
			<div>
				<span>Custom models</span>
				<h2 id="custom-title" class="display-legend">Unverified files</h2>
			</div>
			<p>Files outside the checksum-verified catalog.</p>
		</header>
		{#if mode === 'local'}
			<div class="custom-row custom-path-row">
				<HardDrive size={17} />
				<div>
					<strong>Local model path</strong><span
						>Use an existing Whisper GGML file outside the verified catalog.</span
					>
					<Input bind:value={customPath} aria-label="Custom local model path" />
				</div>
				<div class="custom-actions">
					<Button
						variant="outline"
						size="sm"
						disabled={controlsLocked || !customPath.trim() || customPath === runtime?.model_path}
						onclick={useCustomPath}
					>
						Use path
					</Button>
					<Button size="sm" disabled={controlsLocked} onclick={() => app.preloadLocalModel()}>
						Load / reload
					</Button>
				</div>
			</div>
		{:else}
			<div class="custom-row">
				<HardDrive size={17} />
				<div>
					<strong>Managed by the remote host</strong><span
						>The current API exposes catalog selection and verified downloads, not arbitrary paths.</span
					>
				</div>
				<Badge variant="outline">API-managed</Badge>
			</div>
		{/if}
	</section>
</div>

<style>
	.models-view {
		display: grid;
		gap: 1rem;
	}

	.execution-settings select {
		height: 2rem;
		border: 1px solid var(--line);
		padding: 0 1.8rem 0 0.6rem;
		background: var(--surface-1);
		color: var(--ink-dim);
		font-size: 0.6875rem;
	}

	.runtime-band {
		display: grid;
		grid-template-columns: minmax(15rem, 1.3fr) minmax(13rem, 1fr) minmax(12rem, 0.8fr);
		border: 1px solid var(--line);
		background: var(--surface-1);
	}

	.runtime-band > div {
		min-width: 0;
		padding: 1rem;
	}

	.runtime-band > div + div {
		border-left: 1px solid var(--line);
	}

	.runtime-identity {
		display: grid;
		grid-template-columns: 2.6rem 1fr;
		align-items: center;
		gap: 0.8rem;
	}

	.runtime-icon {
		display: grid;
		width: 2.6rem;
		height: 2.6rem;
		place-items: center;
		/* The same marking the signal path's inference stage carries, for the same
		   reason: this plate names which machine is doing the work. Hairline and lamp
		   ink, never a fill — a filled scarlet square is the record control. */
		border: 1px solid var(--scarlet);
		color: var(--scarlet-lamp);
	}

	.runtime-identity span,
	.runtime-control span,
	.catalog header span {
		color: var(--ink-muted);
		font-size: 0.6875rem;
		font-weight: 680;
		letter-spacing: 0.09em;
		text-transform: uppercase;
	}

	/* Panel headings are the display cut, sized by `.display-legend`. They were
	   monospace at weight 600, which is this world's one typographic mistake stated
	   twice: the mono face is a legend face and takes no bold, and emphasis here is
	   supposed to come from changing cut, not from thickening a monospace. Size and
	   weight are deliberately absent so the global class owns them and every panel
	   heading in the app stays one size. */
	.runtime-identity h2,
	.catalog h2 {
		margin: 0.2rem 0 0;
		color: var(--ink);
	}

	.runtime-identity p {
		margin: 0.3rem 0 0;
		overflow: hidden;
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.runtime-control {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
	}

	.runtime-control > div {
		display: grid;
		gap: 0.25rem;
	}

	.runtime-control small,
	.runtime-health > span {
		color: var(--ink-muted);
		font-size: 0.6875rem;
		line-height: 1.4;
	}

	.runtime-health {
		display: grid;
		align-content: center;
		gap: 0.45rem;
	}

	.execution-settings {
		display: grid;
		grid-template-columns: minmax(11rem, 1fr) minmax(10rem, auto) minmax(11rem, 1fr) minmax(
				13rem,
				auto
			);
		align-items: center;
		gap: 0.8rem;
		border: 1px solid var(--line);
		padding: 0.8rem 1rem;
		background: var(--surface-1);
	}

	.execution-settings > div {
		display: grid;
		gap: 0.2rem;
	}

	.execution-settings label {
		color: var(--ink);
		font-size: 0.75rem;
		font-weight: 570;
	}

	.execution-settings span {
		color: var(--ink-muted);
		font-size: 0.6875rem;
	}

	.execution-settings select:disabled {
		opacity: 0.45;
	}

	.model-notice {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: center;
		gap: 0.8rem;
		/* Information, not an alert, so it takes no accent at all. The old tinted-blue
		   card was the dashboard reflex of colouring every notice; in this world a
		   plate with a sole-boundary rule is enough to say "read this". */
		border: 1px solid var(--line-strong);
		padding: 0.8rem 1rem;
		background: var(--surface-1);
		color: var(--ink-dim);
	}

	.model-notice.error {
		border-color: var(--scarlet);
		border-left-width: 2px;
		color: var(--scarlet-lamp);
	}

	.model-notice > div {
		display: grid;
		gap: 0.2rem;
	}

	.model-notice strong {
		color: var(--ink);
		font-size: 0.75rem;
	}

	.model-notice span {
		color: var(--ink-dim);
		font-size: 0.6875rem;
	}

	.catalog {
		border-top: 1px solid var(--line);
		padding-top: 1.5rem;
	}

	.catalog > header {
		display: flex;
		align-items: end;
		justify-content: space-between;
		gap: 1rem;
		padding-bottom: 0.9rem;
	}

	.catalog > header p {
		margin: 0;
		color: var(--ink-muted);
		font-size: 0.6875rem;
	}

	.model-list {
		border: 1px solid var(--line);
		background: var(--surface-1);
	}

	.model-list article {
		display: grid;
		grid-template-columns: 2.5rem minmax(0, 1fr) auto;
		align-items: center;
		gap: 0.9rem;
		min-height: 6.4rem;
		padding: 1rem;
		transition: background-color 140ms ease;
	}

	.model-list article + article {
		border-top: 1px solid var(--line);
	}

	.model-list article:hover {
		background: var(--surface-2);
	}

	/* The selected model is marked the way the selected destination is marked in the
	   command rail: a scarlet bar on the row's own leading edge, over the raised
	   ground. Consistent, findable from the corner of the eye, and it survives the row
	   also being hovered — which a background change alone did not. */
	.model-list article.selected {
		background: var(--surface-2);
		box-shadow: inset 2px 0 0 var(--scarlet);
	}

	.model-symbol {
		display: grid;
		width: 2.35rem;
		height: 2.35rem;
		place-items: center;
		border: 1px solid var(--line);
		color: var(--ink-muted);
	}

	.selected .model-symbol {
		border-color: var(--scarlet);
		color: var(--scarlet-lamp);
	}

	.model-copy {
		min-width: 0;
	}

	.model-name,
	.model-meta,
	.download-progress {
		display: flex;
		align-items: center;
		gap: 0.45rem;
	}

	.model-name h3 {
		margin: 0;
		color: var(--ink);
	}

	.model-copy > p {
		margin: 0.35rem 0;
		color: var(--ink-dim);
		font-size: 0.72rem;
		line-height: 1.45;
	}

	.model-meta span,
	.download-progress span {
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.6875rem;
	}

	.model-meta span + span::before {
		content: '·';
		margin-right: 0.45rem;
	}

	.download-progress {
		max-width: 14rem;
		margin-top: 0.65rem;
	}

	.download-progress :global([data-slot='progress']) {
		height: 0.25rem;
	}

	.model-actions {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		padding-left: 1rem;
	}

	.locked {
		opacity: 0.58;
	}

	.custom-models {
		border-top: 1px solid var(--line);
		padding-top: 1.25rem;
	}

	.custom-models > header {
		display: flex;
		align-items: end;
		justify-content: space-between;
		gap: 1rem;
		padding-bottom: 0.75rem;
	}

	.custom-models h2 {
		margin: 0.2rem 0 0;
		color: var(--ink);
	}

	.custom-models header span {
		color: var(--ink-muted);
		font-size: 0.6875rem;
		font-weight: 680;
		letter-spacing: 0.09em;
		text-transform: uppercase;
	}

	.custom-models header p {
		margin: 0;
		color: var(--ink-muted);
		font-size: 0.6875rem;
	}

	.custom-row {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto auto;
		align-items: center;
		gap: 0.75rem;
		border: 1px solid var(--line);
		padding: 0.85rem 1rem;
		background: var(--surface-1);
		color: var(--ink-muted);
	}

	.custom-row > div {
		display: grid;
		gap: 0.25rem;
	}

	.custom-path-row > div:nth-child(2) {
		grid-template-columns: minmax(12rem, 1fr);
		gap: 0.55rem;
	}

	.custom-path-row :global(input) {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
	}

	.custom-row > .custom-actions {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}

	.custom-row strong {
		color: var(--ink);
		font-size: 0.75rem;
	}

	.custom-row span {
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.6875rem;
	}

	/* Was a green "success" chip. This world has no green, and "selected" is not a
	   success anyway — it is the marked state, which is scarlet ink inside a scarlet
	   hairline and no fill. */
	:global(.selected-badge) {
		border-color: var(--scarlet);
		background: transparent;
		color: var(--scarlet-lamp);
	}

	@media (max-width: 920px) {
		.runtime-band {
			grid-template-columns: 1fr 1fr;
		}

		.execution-settings {
			grid-template-columns: 1fr 1fr;
		}

		.runtime-health {
			display: none;
		}
	}

	@media (max-width: 720px) {
		.runtime-band {
			grid-template-columns: 1fr;
		}

		.runtime-band > div + div {
			border-top: 1px solid var(--line);
			border-left: 0;
		}

		.model-list article {
			grid-template-columns: 2.5rem minmax(0, 1fr);
		}

		.model-actions {
			grid-column: 2;
			padding: 0;
		}
	}
</style>
