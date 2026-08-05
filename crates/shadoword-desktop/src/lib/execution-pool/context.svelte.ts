import type { DesktopAppState } from '$lib/app-state.svelte';
import type {
	ExecutionTarget,
	ExecutionUnitConfig,
	ExecutionUnitState,
	InferencePoolConfig,
	RuntimeConfigDto,
	WhisperGpuDeviceInfo
} from '$lib/bindings';
import { errorMessage, formatBytes } from '$lib/display';
import {
	isExplicitPool,
	legacyRuntimeToExplicitPool,
	nextUnitId,
	normalizeInferencePool
} from '$lib/inference-pool';
import type { RuntimeState } from '$lib/types';
import { createContext } from 'svelte';
import { SvelteSet } from 'svelte/reactivity';

interface PoolDraftSources {
	readonly app: DesktopAppState;
	readonly runtime: RuntimeConfigDto;
	readonly gpuDevices: WhisperGpuDeviceInfo[];
}

export class PoolDraftState {
	explicit = $state(false);
	draft = $state.raw<InferencePoolConfig>({});
	localActionError = $state('');
	rowKeys = $state.raw<string[]>([]);

	#rowSequence = 0;
	#sources: PoolDraftSources;

	constructor(sources: PoolDraftSources) {
		this.#sources = sources;
		this.explicit = isExplicitPool(this.runtime);
		this.draft = this.runtime.inference_pool
			? normalizeInferencePool(this.runtime.inference_pool)
			: legacyRuntimeToExplicitPool(this.runtime, this.gpuDevices);
		this.rowKeys = this.#unitsFor(this.runtime, this.gpuDevices).map(() => this.#nextRowKey());
	}

	get app() {
		return this.#sources.app;
	}

	get runtime() {
		return this.#sources.runtime;
	}

	get gpuDevices() {
		return this.#sources.gpuDevices;
	}

	get poolStatus() {
		return this.app.overview?.status.inference_pool ?? null;
	}

	get units() {
		return this.draft.units ?? [];
	}

	get locked() {
		return this.app.poolMutationLocked;
	}

	get draining() {
		return this.poolStatus?.draining_generations ?? [];
	}

	get assignedGpuDevices() {
		return new SvelteSet(
			this.units
				.filter((unit) => unit.enabled !== false && unit.target.kind === 'gpu')
				.map((unit) => (unit.target.kind === 'gpu' ? unit.target.device : -1))
		);
	}

	get availableGpu() {
		return this.gpuDevices.find((device) => !this.assignedGpuDevices.has(device.id));
	}

	markChanged() {
		this.localActionError = '';
		this.app.clearPoolDraftFeedback();
	}

	setExplicit(next: boolean) {
		if (this.explicit === next) return;
		this.explicit = next;
		if (next && !isExplicitPool(this.runtime)) {
			this.draft = legacyRuntimeToExplicitPool(this.runtime, this.gpuDevices);
			this.rowKeys = (this.draft.units ?? []).map(() => this.#nextRowKey());
		}
		this.markChanged();
	}

	replaceUnit(index: number, unit: ExecutionUnitConfig) {
		const next = [...this.units];
		next[index] = unit;
		this.draft = { ...this.draft, units: next };
		this.markChanged();
	}

	setUnitTarget(index: number, kind: ExecutionTarget['kind']) {
		const unit = this.units[index];
		if (!unit) return;
		const target: ExecutionTarget =
			kind === 'cpu'
				? { kind: 'cpu', threads: 4 }
				: {
						kind: 'gpu',
						device: this.availableGpu?.id ?? this.gpuDevices[0]?.id ?? -1,
						host_threads: 1
					};
		this.replaceUnit(index, { ...unit, target });
	}

	setGpuDevice(index: number, device: number) {
		const unit = this.units[index];
		if (!unit || unit.target.kind !== 'gpu') return;
		this.replaceUnit(index, {
			...unit,
			target: { kind: 'gpu', device, host_threads: unit.target.host_threads }
		});
	}

	setGpuHostThreads(index: number, hostThreads: number) {
		const unit = this.units[index];
		if (!unit || unit.target.kind !== 'gpu') return;
		this.replaceUnit(index, {
			...unit,
			target: { kind: 'gpu', device: unit.target.device, host_threads: hostThreads }
		});
	}

	addCpu() {
		this.draft = {
			...this.draft,
			units: [
				...this.units,
				{
					id: nextUnitId(this.units, 'cpu'),
					enabled: true,
					required: false,
					target: { kind: 'cpu', threads: 4 }
				}
			]
		};
		this.rowKeys = [...this.rowKeys, this.#nextRowKey()];
		this.markChanged();
	}

	addGpu() {
		if (!this.availableGpu) return;
		this.draft = {
			...this.draft,
			units: [
				...this.units,
				{
					id: nextUnitId(this.units, 'gpu'),
					enabled: true,
					required: true,
					target: { kind: 'gpu', device: this.availableGpu.id, host_threads: 1 }
				}
			]
		};
		this.rowKeys = [...this.rowKeys, this.#nextRowKey()];
		this.markChanged();
	}

	removeUnit(index: number) {
		this.draft = {
			...this.draft,
			units: this.units.filter((_, unitIndex) => unitIndex !== index)
		};
		this.rowKeys = this.rowKeys.filter((_, unitIndex) => unitIndex !== index);
		this.markChanged();
	}

	moveUnit(index: number, offset: -1 | 1) {
		const destination = index + offset;
		if (destination < 0 || destination >= this.units.length) return;
		const next = [...this.units];
		[next[index], next[destination]] = [next[destination], next[index]];
		const nextKeys = [...this.rowKeys];
		[nextKeys[index], nextKeys[destination]] = [nextKeys[destination], nextKeys[index]];
		this.draft = { ...this.draft, units: next };
		this.rowKeys = nextKeys;
		this.markChanged();
	}

	setLimit(key: keyof NonNullable<InferencePoolConfig['limits']>, value: number) {
		this.draft = {
			...this.draft,
			limits: { ...this.draft.limits, [key]: Math.max(0, Math.trunc(value)) }
		};
		this.markChanged();
	}

	setByteLimit(key: 'max_queued_audio_bytes' | 'max_audio_bytes_per_job', mebibytes: number) {
		this.setLimit(key, Math.max(1, Math.trunc(mebibytes)) * 1024 ** 2);
	}

	setPreloadTimeout(seconds: number) {
		this.draft = {
			...this.draft,
			preload_timeout_ms: Math.max(1, seconds) * 1000
		};
		this.markChanged();
	}

	setMaxDrainingGenerations(value: number) {
		this.draft = { ...this.draft, max_draining_generations: value };
		this.markChanged();
	}

	async validate() {
		this.localActionError = '';
		try {
			await this.app.validateInferencePoolDraft(this.draft);
		} catch (error) {
			this.localActionError = errorMessage(error);
		}
	}

	async applyPool() {
		this.localActionError = '';
		try {
			await this.app.applyInferencePoolDraft(this.explicit ? this.draft : null);
		} catch (error) {
			this.localActionError = errorMessage(error);
		}
	}

	gpuName(deviceId: number) {
		return this.gpuDevices.find((device) => device.id === deviceId);
	}

	gpuOptions(currentDevice: number) {
		return this.gpuDevices.map((device) => ({
			value: String(device.id),
			label: `GPU ${device.id} · ${device.name}`,
			detail: formatBytes(device.total_vram),
			disabled: device.id !== currentDevice && this.assignedGpuDevices.has(device.id)
		}));
	}

	targetLabel(target: ExecutionTarget) {
		if (target.kind === 'cpu') return `CPU · ${target.threads ?? 'auto'} threads`;
		const device = this.gpuName(target.device);
		return `GPU ${target.device}${device ? ` · ${device.name}` : ''} · ${target.host_threads ?? 'auto'} host threads`;
	}

	statusState(state: ExecutionUnitState): RuntimeState {
		if (state === 'unhealthy') return 'offline';
		if (state === 'busy' || state === 'loading') return 'loading';
		if (state === 'unloaded') return 'warning';
		return 'ready';
	}

	fieldError(index: number, field: 'id' | 'device') {
		return this.app.poolFieldErrors[`units.${index}.${field}`];
	}

	#unitsFor(value: RuntimeConfigDto, devices: WhisperGpuDeviceInfo[]) {
		return value.inference_pool?.units ?? legacyRuntimeToExplicitPool(value, devices).units ?? [];
	}

	#nextRowKey() {
		this.#rowSequence += 1;
		return `pool-row-${this.#rowSequence}`;
	}
}

export const [getPoolDraftContext, setPoolDraftContext] = createContext<PoolDraftState>();
