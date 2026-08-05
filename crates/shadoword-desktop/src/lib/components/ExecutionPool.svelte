<script lang="ts">
	import {
		AlertTriangle,
		ArrowDown,
		ArrowUp,
		Cpu,
		Gauge,
		MemoryStick,
		Plus,
		ServerCog,
		Trash2,
		Zap
	} from '@lucide/svelte';
	import type { DesktopAppState } from '$lib/app-state.svelte';
	import BrutalistSelect from '$lib/components/BrutalistSelect.svelte';
	import type {
		ExecutionTarget,
		ExecutionUnitConfig,
		ExecutionUnitState,
		InferencePoolConfig,
		RuntimeConfigDto,
		WhisperGpuDeviceInfo
	} from '$lib/bindings';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Switch } from '$lib/components/ui/switch';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { errorMessage, formatBytes } from '$lib/display';
	import {
		isExplicitPool,
		legacyRuntimeToExplicitPool,
		nextUnitId,
		normalizeInferencePool
	} from '$lib/inference-pool';
	import type { RuntimeState } from '$lib/types';
	import { untrack } from 'svelte';

	let {
		app,
		runtime,
		gpuDevices
	}: {
		app: DesktopAppState;
		runtime: RuntimeConfigDto;
		gpuDevices: WhisperGpuDeviceInfo[];
	} = $props();

	const initial = untrack(() => ({ runtime, gpuDevices }));
	let explicit = $state(isExplicitPool(initial.runtime));
	let draft = $state.raw<InferencePoolConfig>(
		initial.runtime.inference_pool
			? normalizeInferencePool(initial.runtime.inference_pool)
			: legacyRuntimeToExplicitPool(initial.runtime, initial.gpuDevices)
	);
	let localActionError = $state('');
	let rowSequence = 0;
	let rowKeys = $state(unitsFor(initial.runtime, initial.gpuDevices).map(() => nextRowKey()));
	let poolStatus = $derived(app.overview?.status.inference_pool ?? null);
	let units = $derived(draft.units ?? []);
	let locked = $derived(app.poolMutationLocked);
	let draining = $derived(poolStatus?.draining_generations ?? []);
	let assignedGpuDevices = $derived(
		new Set(
			units
				.filter((unit) => unit.enabled !== false && unit.target.kind === 'gpu')
				.map((unit) => (unit.target.kind === 'gpu' ? unit.target.device : -1))
		)
	);
	let availableGpu = $derived(gpuDevices.find((device) => !assignedGpuDevices.has(device.id)));

	function markChanged() {
		localActionError = '';
		app.clearPoolDraftFeedback();
	}

	function unitsFor(value: RuntimeConfigDto, devices: WhisperGpuDeviceInfo[]) {
		return value.inference_pool?.units ?? legacyRuntimeToExplicitPool(value, devices).units ?? [];
	}

	function nextRowKey() {
		rowSequence += 1;
		return `pool-row-${rowSequence}`;
	}

	function setExplicit(next: boolean) {
		if (explicit === next) return;
		explicit = next;
		if (next && !isExplicitPool(runtime)) {
			draft = legacyRuntimeToExplicitPool(runtime, gpuDevices);
			rowKeys = (draft.units ?? []).map(() => nextRowKey());
		}
		markChanged();
	}

	function replaceUnit(index: number, unit: ExecutionUnitConfig) {
		const next = [...units];
		next[index] = unit;
		draft = { ...draft, units: next };
		markChanged();
	}

	function setUnitTarget(index: number, kind: ExecutionTarget['kind']) {
		const unit = units[index];
		if (!unit) return;
		const target: ExecutionTarget =
			kind === 'cpu'
				? { kind: 'cpu', threads: 4 }
				: { kind: 'gpu', device: availableGpu?.id ?? gpuDevices[0]?.id ?? -1, host_threads: 1 };
		replaceUnit(index, { ...unit, target });
	}

	function setGpuDevice(index: number, device: number) {
		const unit = units[index];
		if (!unit || unit.target.kind !== 'gpu') return;
		replaceUnit(index, {
			...unit,
			target: { kind: 'gpu', device, host_threads: unit.target.host_threads }
		});
	}

	function setGpuHostThreads(index: number, hostThreads: number) {
		const unit = units[index];
		if (!unit || unit.target.kind !== 'gpu') return;
		replaceUnit(index, {
			...unit,
			target: { kind: 'gpu', device: unit.target.device, host_threads: hostThreads }
		});
	}

	function addCpu() {
		draft = {
			...draft,
			units: [
				...units,
				{
					id: nextUnitId(units, 'cpu'),
					enabled: true,
					required: false,
					target: { kind: 'cpu', threads: 4 }
				}
			]
		};
		rowKeys = [...rowKeys, nextRowKey()];
		markChanged();
	}

	function addGpu() {
		if (!availableGpu) return;
		draft = {
			...draft,
			units: [
				...units,
				{
					id: nextUnitId(units, 'gpu'),
					enabled: true,
					required: true,
					target: { kind: 'gpu', device: availableGpu.id, host_threads: 1 }
				}
			]
		};
		rowKeys = [...rowKeys, nextRowKey()];
		markChanged();
	}

	function removeUnit(index: number) {
		draft = { ...draft, units: units.filter((_, unitIndex) => unitIndex !== index) };
		rowKeys = rowKeys.filter((_, unitIndex) => unitIndex !== index);
		markChanged();
	}

	function moveUnit(index: number, offset: -1 | 1) {
		const destination = index + offset;
		if (destination < 0 || destination >= units.length) return;
		const next = [...units];
		[next[index], next[destination]] = [next[destination], next[index]];
		const nextKeys = [...rowKeys];
		[nextKeys[index], nextKeys[destination]] = [nextKeys[destination], nextKeys[index]];
		draft = { ...draft, units: next };
		rowKeys = nextKeys;
		markChanged();
	}

	function setLimit(key: keyof NonNullable<InferencePoolConfig['limits']>, value: number) {
		draft = { ...draft, limits: { ...draft.limits, [key]: Math.max(0, Math.trunc(value)) } };
		markChanged();
	}

	function setByteLimit(
		key: 'max_queued_audio_bytes' | 'max_audio_bytes_per_job',
		mebibytes: number
	) {
		setLimit(key, Math.max(1, Math.trunc(mebibytes)) * 1024 ** 2);
	}

	async function validate() {
		localActionError = '';
		try {
			await app.validateInferencePoolDraft(draft);
		} catch (error) {
			localActionError = errorMessage(error);
		}
	}

	async function applyPool() {
		localActionError = '';
		try {
			await app.applyInferencePoolDraft(explicit ? draft : null);
		} catch (error) {
			localActionError = errorMessage(error);
		}
	}

	function gpuName(deviceId: number) {
		return gpuDevices.find((device) => device.id === deviceId);
	}

	function gpuOptions(currentDevice: number) {
		return gpuDevices.map((device) => ({
			value: String(device.id),
			label: `GPU ${device.id} · ${device.name}`,
			detail: formatBytes(device.total_vram),
			disabled: device.id !== currentDevice && assignedGpuDevices.has(device.id)
		}));
	}

	function targetLabel(target: ExecutionTarget) {
		if (target.kind === 'cpu') return `CPU · ${target.threads ?? 'auto'} threads`;
		const device = gpuName(target.device);
		return `GPU ${target.device}${device ? ` · ${device.name}` : ''} · ${target.host_threads ?? 'auto'} host threads`;
	}

	function statusState(state: ExecutionUnitState): RuntimeState {
		if (state === 'unhealthy') return 'offline';
		if (state === 'busy' || state === 'loading') return 'loading';
		if (state === 'unloaded') return 'warning';
		return 'ready';
	}

	function fieldError(index: number, field: 'id' | 'device') {
		return app.poolFieldErrors[`units.${index}.${field}`];
	}
</script>

<section class="pool-observatory" aria-labelledby="pool-title">
	<header class="pool-heading">
		<div>
			<h2 id="pool-title" class="display-legend">Execution pool</h2>
			<p>Observe the active generation, then stage a replacement without interrupting its state.</p>
		</div>
		<div class="generation-readout">
			<span>Generation</span>
			<strong>{poolStatus?.generation ?? runtime.generation ?? '—'}</strong>
		</div>
	</header>

	<div class="pool-telemetry" aria-live="polite">
		<div>
			<Gauge size={15} aria-hidden="true" />
			<span>Capacity</span>
			<strong
				>{poolStatus?.ready_units ?? 0} ready · {poolStatus?.busy_units ?? 0} busy · {poolStatus?.unhealthy_units ??
					0} unhealthy · {poolStatus?.accepting ? 'accepting' : 'paused'}</strong
			>
		</div>
		<div>
			<ServerCog size={15} aria-hidden="true" />
			<span>Work</span>
			<strong
				>{poolStatus?.queued_jobs ?? 0} queued · {poolStatus?.running_jobs ?? 0} running</strong
			>
		</div>
		<div>
			<MemoryStick size={15} aria-hidden="true" />
			<span>Audio memory</span>
			<strong
				>{formatBytes(poolStatus?.queued_audio_bytes ?? 0)} queued · {formatBytes(
					poolStatus?.running_audio_bytes ?? 0
				)} running</strong
			>
		</div>
		<div>
			<Zap size={15} aria-hidden="true" />
			<span>Lifetime</span>
			<strong>{poolStatus?.completed ?? 0} complete · {poolStatus?.failed ?? 0} failed</strong>
		</div>
	</div>

	{#if poolStatus?.units.length}
		<div class="active-units" aria-label="Active execution units">
			{#each poolStatus.units as unit (unit.id)}
				<div class="active-unit">
					<div class="unit-identity">
						{#if unit.target.kind === 'gpu'}<Zap size={16} />{:else}<Cpu size={16} />{/if}
						<div>
							<strong>{unit.id}</strong>
							<span>{targetLabel(unit.target)}</span>
						</div>
					</div>
					<StatusPill state={statusState(unit.state)} label={unit.state} compact />
					<div class="unit-counters">
						<span>{unit.required ? 'Required' : 'Optional'}</span>
						<span>{unit.completed ?? 0} complete</span>
						<span>{unit.failed ?? 0} failed</span>
					</div>
					{#if unit.last_error}<p role="status">{unit.last_error}</p>{/if}
				</div>
			{/each}
		</div>
	{:else}
		<div class="empty-monitor">No execution-unit telemetry is available from this runtime.</div>
	{/if}

	{#if draining.length > 0}
		<div class="draining-notice" role="status">
			<AlertTriangle size={16} />
			<div>
				<strong
					>{draining.length} prior generation{draining.length === 1 ? '' : 's'} draining</strong
				>
				<span
					>Pool mutation is locked until its queued and running work finishes, preventing too many
					model copies from overlapping.</span
				>
				{#each draining as generation (generation.generation)}
					<small
						>Generation {generation.generation ?? '—'} · {generation.running_jobs} running · {formatBytes(
							generation.running_audio_bytes
						)} · {generation.workers_remaining} workers remaining</small
					>
				{/each}
			</div>
		</div>
	{/if}

	<div class="pool-editor" class:locked>
		<div class="editor-heading">
			<div>
				<h3 class="display-legend">Stage execution topology</h3>
				<p>
					Model, language, and preload policy stay shared so every worker produces consistent
					output.
				</p>
			</div>
			<div class="mode-switch" aria-label="Inference topology mode">
				<button
					type="button"
					class:active={!explicit}
					aria-pressed={!explicit}
					disabled={locked}
					onclick={() => setExplicit(false)}>Legacy single</button
				>
				<button
					type="button"
					class:active={explicit}
					aria-pressed={explicit}
					disabled={locked}
					onclick={() => setExplicit(true)}>Explicit pool</button
				>
			</div>
		</div>

		{#if explicit}
			<div class="memory-warning">
				<AlertTriangle size={16} />
				<p>
					<strong>Every enabled unit loads an independent copy of the selected model.</strong>
					Plan RAM or VRAM for each worker; sharing a model file does not share its loaded memory.
				</p>
			</div>

			<div class="draft-units">
				{#each units as unit, index (rowKeys[index])}
					<fieldset class="draft-unit" disabled={locked}>
						<legend>Unit {index + 1}</legend>
						<div class="unit-controls">
							<label>
								<span>Stable ID</span>
								<Input
									value={unit.id}
									aria-label={`Execution unit ${index + 1} stable ID`}
									aria-invalid={Boolean(fieldError(index, 'id'))}
									oninput={(event) =>
										replaceUnit(index, { ...unit, id: event.currentTarget.value })}
								/>
								{#if fieldError(index, 'id')}<small class="field-error"
										>{fieldError(index, 'id')}</small
									>{/if}
							</label>
							<label>
								<span>Target</span>
								<BrutalistSelect
									value={unit.target.kind}
									options={[
										{ value: 'cpu', label: 'CPU', detail: 'Host threads' },
										{
											value: 'gpu',
											label: 'GPU',
											detail: 'Dedicated accelerator',
											disabled: gpuDevices.length === 0
										}
									]}
									ariaLabel={`Execution unit ${index + 1} target`}
									onValueChange={(value) => setUnitTarget(index, value === 'gpu' ? 'gpu' : 'cpu')}
								/>
							</label>
							{#if unit.target.kind === 'cpu'}
								<label>
									<span>Threads</span>
									<Input
										type="number"
										min="1"
										max="256"
										value={unit.target.threads ?? 4}
										aria-label={`CPU threads for ${unit.id || `unit ${index + 1}`}`}
										oninput={(event) =>
											replaceUnit(index, {
												...unit,
												target: { kind: 'cpu', threads: Number(event.currentTarget.value) }
											})}
									/>
								</label>
							{:else}
								<label class="gpu-select">
									<span>GPU device</span>
									<BrutalistSelect
										value={String(unit.target.device)}
										options={gpuOptions(unit.target.device)}
										ariaLabel={`GPU device for ${unit.id || `unit ${index + 1}`}`}
										ariaInvalid={Boolean(fieldError(index, 'device'))}
										ariaDescribedBy={fieldError(index, 'device')
											? `execution-unit-${index}-device-error`
											: undefined}
										onValueChange={(value) => setGpuDevice(index, Number(value))}
									/>
									{#if fieldError(index, 'device')}<small
											id={`execution-unit-${index}-device-error`}
											class="field-error">{fieldError(index, 'device')}</small
										>{:else if gpuName(unit.target.device)}
										<small
											>{formatBytes(gpuName(unit.target.device)?.free_vram ?? 0)} free VRAM</small
										>
									{/if}
								</label>
								<label>
									<span>Host threads</span>
									<Input
										type="number"
										min="1"
										max="256"
										value={unit.target.host_threads ?? 1}
										aria-label={`GPU host threads for ${unit.id || `unit ${index + 1}`}`}
										oninput={(event) => setGpuHostThreads(index, Number(event.currentTarget.value))}
									/>
								</label>
							{/if}
						</div>
						<div class="unit-flags">
							<label class="flag-control">
								<Switch
									checked={unit.enabled ?? true}
									onclick={() => replaceUnit(index, { ...unit, enabled: !(unit.enabled ?? true) })}
									aria-label={`Enable ${unit.id || `unit ${index + 1}`}`}
								/>
								<span>Enabled</span>
							</label>
							<label class="flag-control">
								<Switch
									checked={unit.required ?? true}
									onclick={() =>
										replaceUnit(index, { ...unit, required: !(unit.required ?? true) })}
									aria-label={`Require ${unit.id || `unit ${index + 1}`} during reload`}
								/>
								<span>Required to reload</span>
							</label>
							<div class="unit-order">
								<Button
									variant="ghost"
									size="icon-xs"
									disabled={index === 0}
									aria-label={`Move ${unit.id || `unit ${index + 1}`} up`}
									onclick={() => moveUnit(index, -1)}><ArrowUp size={13} /></Button
								>
								<Button
									variant="ghost"
									size="icon-xs"
									disabled={index === units.length - 1}
									aria-label={`Move ${unit.id || `unit ${index + 1}`} down`}
									onclick={() => moveUnit(index, 1)}><ArrowDown size={13} /></Button
								>
								<Button
									variant="ghost"
									size="icon-xs"
									aria-label={`Delete ${unit.id || `unit ${index + 1}`}`}
									onclick={() => removeUnit(index)}><Trash2 size={13} /></Button
								>
							</div>
						</div>
					</fieldset>
				{/each}
			</div>

			<div class="add-unit-row">
				<Button variant="outline" size="sm" disabled={locked} onclick={addCpu}
					><Plus size={13} />Add CPU unit</Button
				>
				<Button variant="outline" size="sm" disabled={locked || !availableGpu} onclick={addGpu}
					><Plus size={13} />{availableGpu
						? `Add GPU ${availableGpu.id}`
						: 'All GPUs assigned'}</Button
				>
				<span>{gpuDevices.length} detected GPU{gpuDevices.length === 1 ? '' : 's'}</span>
			</div>

			<details class="advanced-limits">
				<summary>Advanced admission and reload limits</summary>
				<div class="limit-grid">
					<label>
						<span>Queued jobs</span>
						<Input
							type="number"
							disabled={locked}
							min="0"
							max="10000"
							value={draft.limits?.max_queued_jobs ?? 32}
							oninput={(event) => setLimit('max_queued_jobs', Number(event.currentTarget.value))}
						/>
						<small>Zero permits direct worker hand-off only.</small>
					</label>
					<label>
						<span>Queue memory · MiB</span>
						<Input
							type="number"
							disabled={locked}
							min="1"
							max="1048576"
							value={Math.round((draft.limits?.max_queued_audio_bytes ?? 67_108_864) / 1024 ** 2)}
							oninput={(event) =>
								setByteLimit('max_queued_audio_bytes', Number(event.currentTarget.value))}
						/>
						<small>{formatBytes(draft.limits?.max_queued_audio_bytes ?? 0)} decoded audio</small>
					</label>
					<label>
						<span>Per-job memory · MiB</span>
						<Input
							type="number"
							disabled={locked}
							min="1"
							max="1048576"
							value={Math.round((draft.limits?.max_audio_bytes_per_job ?? 67_108_864) / 1024 ** 2)}
							oninput={(event) =>
								setByteLimit('max_audio_bytes_per_job', Number(event.currentTarget.value))}
						/>
						<small>{formatBytes(draft.limits?.max_audio_bytes_per_job ?? 0)} maximum</small>
					</label>
					<label>
						<span>Outstanding per flow</span>
						<Input
							type="number"
							disabled={locked}
							min="1"
							max="10000"
							value={draft.limits?.max_outstanding_per_flow ?? 8}
							oninput={(event) =>
								setLimit('max_outstanding_per_flow', Number(event.currentTarget.value))}
						/>
					</label>
					<label>
						<span>Buffered results per flow</span>
						<Input
							type="number"
							disabled={locked}
							min="1"
							max="10000"
							value={draft.limits?.max_buffered_results_per_flow ?? 32}
							oninput={(event) =>
								setLimit('max_buffered_results_per_flow', Number(event.currentTarget.value))}
						/>
						<small>Bounds out-of-order streaming completions.</small>
					</label>
					<label>
						<span>Preload timeout · seconds</span>
						<Input
							type="number"
							disabled={locked}
							min="1"
							max="1800"
							aria-invalid={Boolean(app.poolFieldErrors.preload_timeout_ms)}
							value={Math.round((draft.preload_timeout_ms ?? 120_000) / 1000)}
							oninput={(event) => {
								draft = {
									...draft,
									preload_timeout_ms: Math.max(1, Number(event.currentTarget.value)) * 1000
								};
								markChanged();
							}}
						/>
						{#if app.poolFieldErrors.preload_timeout_ms}<small class="field-error"
								>{app.poolFieldErrors.preload_timeout_ms}</small
							>{/if}
					</label>
					<label>
						<span>Maximum draining generations</span>
						<Input
							type="number"
							disabled={locked}
							min="1"
							max="8"
							aria-invalid={Boolean(app.poolFieldErrors.max_draining_generations)}
							value={draft.max_draining_generations ?? 2}
							oninput={(event) => {
								draft = { ...draft, max_draining_generations: Number(event.currentTarget.value) };
								markChanged();
							}}
						/>
						{#if app.poolFieldErrors.max_draining_generations}<small class="field-error"
								>{app.poolFieldErrors.max_draining_generations}</small
							>{/if}
						<small>Hard cap: 8 overlapping retiring generations.</small>
					</label>
				</div>
			</details>
		{:else}
			<div class="legacy-copy">
				<Cpu size={17} />
				<div>
					<strong>One worker follows the legacy accelerator controls below.</strong>
					<span
						>Auto remains automatic, including device −1. Converting to a pool chooses a detected
						GPU explicitly or creates a CPU unit, never an invalid automatic GPU target.</span
					>
				</div>
			</div>
		{/if}

		<div class="editor-actions">
			<div class="feedback" aria-live="polite">
				{#if app.poolFeedback}<span>{app.poolFeedback}</span>{/if}
				{#if localActionError && localActionError !== app.poolFeedback}<small
						>{localActionError}</small
					>{/if}
			</div>
			{#if explicit}
				<Button
					variant="outline"
					size="sm"
					disabled={locked || app.poolValidationState === 'validating'}
					onclick={validate}
					>{app.poolValidationState === 'validating' ? 'Validating…' : 'Validate pool'}</Button
				>
			{/if}
			<Button
				size="sm"
				disabled={locked || (explicit && app.poolValidationState !== 'valid')}
				onclick={applyPool}
				>{app.poolApplyState === 'applying' ? 'Applying…' : 'Apply & reload'}</Button
			>
		</div>
	</div>
</section>

<style>
	.pool-observatory {
		border-top: 1px solid var(--line);
		padding-top: 1.35rem;
	}

	.pool-heading,
	.editor-heading,
	.editor-actions,
	.add-unit-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
	}

	.pool-heading h2,
	.editor-heading h3 {
		margin: 0;
		color: var(--ink);
	}

	.pool-heading p,
	.editor-heading p {
		max-width: 68ch;
		margin: 0.25rem 0 0;
		color: var(--ink-muted);
		font-size: 0.72rem;
		line-height: 1.45;
	}

	.generation-readout {
		display: flex;
		align-items: baseline;
		gap: 0.6rem;
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.6875rem;
	}

	/* The generation number is a reading, and readings are off-white with tabular
	   figures. It was the old accent hue, which made the one number on this panel that
	   never needs acting on the loudest thing on it. */
	.generation-readout strong {
		color: var(--ink);
		font-size: 1rem;
		font-weight: 400;
		font-variant-numeric: tabular-nums;
	}

	.pool-telemetry {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		margin-top: 0.9rem;
		border-block: 1px solid var(--line);
		background: var(--surface-1);
	}

	.pool-telemetry > div {
		display: grid;
		grid-template-columns: auto 1fr;
		align-items: center;
		gap: 0.2rem 0.55rem;
		min-width: 0;
		padding: 0.75rem;
		color: var(--ink-muted);
	}

	.pool-telemetry > div + div {
		border-left: 1px solid var(--line);
	}

	.pool-telemetry :global(svg) {
		grid-row: 1 / 3;
		color: var(--ink-muted);
	}

	.pool-telemetry span {
		font-size: 0.65rem;
		font-weight: 680;
		letter-spacing: 0.07em;
		text-transform: uppercase;
	}

	.pool-telemetry strong {
		color: var(--ink-dim);
		font-family: var(--font-mono);
		font-size: 0.65rem;
		font-weight: 520;
		line-height: 1.35;
	}

	.active-units,
	.draft-units {
		border-bottom: 1px solid var(--line);
	}

	.active-unit {
		display: grid;
		grid-template-columns: minmax(12rem, 1fr) auto minmax(14rem, auto);
		align-items: center;
		gap: 0.8rem;
		padding: 0.72rem 0.8rem;
	}

	.active-unit + .active-unit,
	.draft-unit + .draft-unit {
		border-top: 1px solid var(--line);
	}

	.unit-identity {
		display: flex;
		align-items: center;
		gap: 0.7rem;
		min-width: 0;
		color: var(--ink-muted);
	}

	.unit-identity > div {
		display: grid;
		min-width: 0;
	}

	.unit-identity strong {
		color: var(--ink);
		font-size: 0.75rem;
	}

	.unit-identity span,
	.unit-counters,
	.active-unit p,
	.draining-notice small {
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.65rem;
	}

	.unit-identity span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.unit-counters {
		display: flex;
		justify-content: end;
		gap: 0.75rem;
	}

	.active-unit p {
		grid-column: 1 / -1;
		margin: -0.25rem 0 0 1.7rem;
		color: var(--scarlet-lamp);
	}

	.empty-monitor {
		border-bottom: 1px solid var(--line);
		padding: 1rem;
		color: var(--ink-muted);
		font-size: 0.72rem;
	}

	.draining-notice,
	.memory-warning,
	.legacy-copy {
		display: flex;
		align-items: flex-start;
		gap: 0.7rem;
		padding: 0.75rem 0.85rem;
	}

	/* Warnings are exactly what a rationed accent is rationed *for*, so both notices in
	   this panel get a scarlet edge — and, like the global error notice, a scarlet edge
	   is all they get. The old amber tint mixed at 94% transparent was invisible on a
	   night ground anyway, so it was a warning colour that warned nobody. The leading
	   2px rule is the part you actually see. */
	.draining-notice {
		border-bottom: 1px solid var(--line);
		border-left: 2px solid var(--scarlet);
		color: var(--scarlet-lamp);
	}

	.draining-notice > div,
	.legacy-copy > div {
		display: grid;
		gap: 0.15rem;
	}

	.draining-notice strong,
	.memory-warning strong,
	.legacy-copy strong {
		color: var(--ink);
		font-size: 0.72rem;
	}

	.draining-notice span,
	.memory-warning p,
	.legacy-copy span {
		margin: 0;
		color: var(--ink-dim);
		font-size: 0.6875rem;
		line-height: 1.45;
	}

	.pool-editor {
		margin-top: 1rem;
		border: 1px solid var(--line);
		background: var(--surface-1);
	}

	.editor-heading {
		padding: 0.85rem 1rem;
		border-bottom: 1px solid var(--line);
	}

	/* Third instance of the one segmented control, identical to the capture stage's
	   target switch and Settings' segmented control: butted cells, shared hairline,
	   scarlet ink and an underscore rule on the selected one. */
	.mode-switch {
		display: inline-flex;
		gap: 1px;
		background: var(--line);
	}

	.mode-switch button {
		border: 0;
		padding: 0.4rem 0.7rem;
		background: var(--surface-1);
		color: var(--ink-muted);
		font: inherit;
		font-size: 0.6875rem;
		cursor: pointer;
		transition:
			background-color 120ms linear,
			color 120ms linear;
	}

	.mode-switch button:hover:not(:disabled) {
		background: var(--surface-2);
		color: var(--ink);
	}

	.mode-switch button.active {
		background: var(--surface-2);
		color: var(--scarlet-lamp);
		box-shadow: inset 0 -2px 0 var(--scarlet);
	}

	.memory-warning {
		border-bottom: 1px solid var(--line);
		border-left: 2px solid var(--scarlet);
		color: var(--scarlet-lamp);
	}

	.memory-warning strong {
		display: block;
		margin-bottom: 0.08rem;
	}

	.draft-unit {
		min-width: 0;
		margin: 0;
		border: 0;
		padding: 0.75rem 0.85rem;
	}

	.draft-unit legend {
		float: left;
		width: 3.4rem;
		padding-top: 1.5rem;
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.65rem;
	}

	.unit-controls {
		display: grid;
		grid-template-columns: minmax(8rem, 0.8fr) minmax(5rem, 0.45fr) minmax(7rem, 1.6fr) minmax(
				5.5rem,
				0.45fr
			);
		gap: 0.55rem;
	}

	.unit-controls label,
	.limit-grid label {
		display: grid;
		align-content: start;
		gap: 0.25rem;
		min-width: 0;
	}

	.unit-controls label > span,
	.limit-grid label > span {
		color: var(--ink-muted);
		font-size: 0.65rem;
		font-weight: 620;
	}

	.gpu-select small,
	.limit-grid small,
	.field-error {
		color: var(--ink-muted);
		font-size: 0.62rem;
		line-height: 1.3;
	}

	.field-error,
	.gpu-select .field-error {
		color: var(--scarlet-lamp);
	}

	.unit-flags {
		display: flex;
		align-items: center;
		gap: 1rem;
		margin: 0.55rem 0 0 3.4rem;
		color: var(--ink-dim);
		font-size: 0.6875rem;
	}

	.unit-flags label {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
	}

	.unit-order {
		display: flex;
		margin-left: auto;
	}

	.add-unit-row {
		justify-content: flex-start;
		padding: 0.7rem 0.85rem;
		border-bottom: 1px solid var(--line);
	}

	.add-unit-row span {
		margin-left: auto;
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.65rem;
	}

	.advanced-limits {
		border-bottom: 1px solid var(--line);
	}

	.advanced-limits summary {
		padding: 0.72rem 0.85rem;
		color: var(--ink-dim);
		font-size: 0.72rem;
		font-weight: 570;
		cursor: pointer;
	}

	.limit-grid {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: 0.7rem;
		padding: 0 0.85rem 0.85rem;
	}

	.legacy-copy {
		min-height: 5rem;
		align-items: center;
		color: var(--ink-muted);
	}

	.editor-actions {
		justify-content: flex-end;
		min-height: 3.45rem;
		padding: 0.65rem 0.85rem;
	}

	.feedback {
		display: grid;
		margin-right: auto;
		color: var(--ink-dim);
		font-size: 0.6875rem;
	}

	.feedback small {
		color: var(--scarlet-lamp);
	}

	.locked {
		opacity: 0.64;
	}

	@media (max-width: 1050px) {
		.pool-telemetry,
		.limit-grid {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}

		.pool-telemetry > div:nth-child(3) {
			border-left: 0;
			border-top: 1px solid var(--line);
		}

		.pool-telemetry > div:nth-child(4) {
			border-top: 1px solid var(--line);
		}

		.unit-controls {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}

	@media (max-width: 760px) {
		.pool-heading,
		.editor-heading {
			align-items: flex-start;
		}

		.active-unit {
			grid-template-columns: minmax(10rem, 1fr) auto;
		}

		.unit-counters {
			grid-column: 1 / -1;
			justify-content: flex-start;
			padding-left: 1.7rem;
		}

		.unit-controls {
			grid-template-columns: repeat(2, minmax(6rem, 1fr));
		}
	}
</style>
