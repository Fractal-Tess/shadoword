import type {
	ExecutionUnitConfig,
	InferenceLimits,
	InferencePoolConfig,
	InferencePoolStatus,
	RuntimeConfigDto,
	WhisperGpuDeviceInfo
} from '$lib/bindings';

export const DEFAULT_INFERENCE_POOL: {
	limits: Required<InferenceLimits>;
	preload_timeout_ms: number;
	max_draining_generations: number;
} = {
	limits: {
		max_queued_jobs: 32,
		max_queued_audio_bytes: 64 * 1024 ** 2,
		max_audio_bytes_per_job: 64 * 1024 ** 2,
		max_outstanding_per_flow: 8,
		max_buffered_results_per_flow: 32
	},
	preload_timeout_ms: 120_000,
	max_draining_generations: 2
};

export type PoolFieldErrors = Record<string, string>;

export type PoolValidation = {
	pool: InferencePoolConfig;
	fieldErrors: PoolFieldErrors;
	globalError: string | null;
};

export function legacyRuntimeToExplicitPool(
	runtime: RuntimeConfigDto,
	devices: readonly WhisperGpuDeviceInfo[]
): InferencePoolConfig {
	const explicitDevice =
		runtime.whisper_gpu_device >= 0
			? devices.find((device) => device.id === runtime.whisper_gpu_device)?.id
			: devices[0]?.id;
	const useGpu = runtime.whisper_accelerator !== 'cpu' && explicitDevice !== undefined;
	const unit: ExecutionUnitConfig = useGpu
		? {
				id: `gpu-${explicitDevice}`,
				enabled: true,
				required: true,
				target: { kind: 'gpu', device: explicitDevice, host_threads: 1 }
			}
		: {
				id: 'cpu-0',
				enabled: true,
				required: true,
				target: { kind: 'cpu', threads: 4 }
			};

	return normalizeInferencePool({ units: [unit] });
}

export function normalizeInferencePool(pool: InferencePoolConfig): InferencePoolConfig {
	return {
		units: (pool.units ?? []).map((unit) => ({
			id: unit.id.trim(),
			enabled: unit.enabled ?? true,
			required: unit.required ?? true,
			target:
				unit.target.kind === 'cpu'
					? { kind: 'cpu', threads: optionalPositiveInteger(unit.target.threads) }
					: {
							kind: 'gpu',
							device: integerOr(unit.target.device, -1),
							host_threads: optionalPositiveInteger(unit.target.host_threads)
						}
		})),
		limits: {
			max_queued_jobs: nonNegativeInteger(
				pool.limits?.max_queued_jobs,
				DEFAULT_INFERENCE_POOL.limits.max_queued_jobs
			),
			max_queued_audio_bytes: positiveInteger(
				pool.limits?.max_queued_audio_bytes,
				DEFAULT_INFERENCE_POOL.limits.max_queued_audio_bytes
			),
			max_audio_bytes_per_job: positiveInteger(
				pool.limits?.max_audio_bytes_per_job,
				DEFAULT_INFERENCE_POOL.limits.max_audio_bytes_per_job
			),
			max_outstanding_per_flow: positiveInteger(
				pool.limits?.max_outstanding_per_flow,
				DEFAULT_INFERENCE_POOL.limits.max_outstanding_per_flow
			),
			max_buffered_results_per_flow: positiveInteger(
				pool.limits?.max_buffered_results_per_flow,
				DEFAULT_INFERENCE_POOL.limits.max_buffered_results_per_flow
			)
		},
		preload_timeout_ms: positiveInteger(
			pool.preload_timeout_ms,
			DEFAULT_INFERENCE_POOL.preload_timeout_ms
		),
		max_draining_generations: positiveInteger(
			pool.max_draining_generations,
			DEFAULT_INFERENCE_POOL.max_draining_generations
		)
	};
}

export function validateInferencePoolCandidate(pool: InferencePoolConfig): PoolValidation {
	const normalized = normalizeInferencePool(pool);
	const units = normalized.units ?? [];
	const fieldErrors: PoolFieldErrors = {};
	const ids = new Map<string, number>();
	const enabledGpuDevices = new Map<number, number>();

	units.forEach((unit, index) => {
		const idKey = `units.${index}.id`;
		if (!/^[A-Za-z0-9._-]{1,64}$/.test(unit.id)) {
			fieldErrors[idKey] = 'Use 1–64 letters, numbers, dots, underscores, or hyphens.';
		}
		const previousId = ids.get(unit.id);
		if (previousId !== undefined) {
			fieldErrors[idKey] = `Unit ID duplicates row ${previousId + 1}.`;
			fieldErrors[`units.${previousId}.id`] = `Unit ID duplicates row ${index + 1}.`;
		} else if (unit.id) {
			ids.set(unit.id, index);
		}

		if (unit.enabled && unit.target.kind === 'gpu') {
			if (unit.target.device < 0 || !Number.isInteger(unit.target.device)) {
				fieldErrors[`units.${index}.device`] = 'Choose a detected, explicit GPU.';
			}
			const previousDevice = enabledGpuDevices.get(unit.target.device);
			if (previousDevice !== undefined) {
				fieldErrors[`units.${index}.device`] =
					`GPU is already assigned to row ${previousDevice + 1}.`;
				fieldErrors[`units.${previousDevice}.device`] =
					`GPU is already assigned to row ${index + 1}.`;
			} else {
				enabledGpuDevices.set(unit.target.device, index);
			}
		}
	});

	if (!units.some((unit) => unit.enabled)) {
		return {
			pool: normalized,
			fieldErrors,
			globalError: 'Enable at least one execution unit before validation.'
		};
	}
	if ((normalized.preload_timeout_ms ?? 0) > 30 * 60 * 1000) {
		fieldErrors.preload_timeout_ms = 'Preload timeout cannot exceed 30 minutes.';
	}
	if ((normalized.max_draining_generations ?? 0) > 8) {
		fieldErrors.max_draining_generations = 'Draining generations cannot exceed 8.';
	}

	return {
		pool: normalized,
		fieldErrors,
		globalError:
			Object.keys(fieldErrors).length > 0
				? 'Resolve the marked pool fields and validate again.'
				: null
	};
}

export function runtimeWithInferencePool(
	runtime: RuntimeConfigDto,
	pool: InferencePoolConfig | null
): RuntimeConfigDto {
	return {
		...runtime,
		inference_pool: pool,
		inference_pool_explicit: pool !== null,
		generation: runtime.generation ?? null
	};
}

export function isExplicitPool(runtime: RuntimeConfigDto | null | undefined) {
	return runtime?.inference_pool_explicit ?? runtime?.inference_pool != null;
}

export function isStaleRuntimeError(error: unknown) {
	if (typeof error !== 'object' || error === null || !('code' in error)) return false;
	return error.code === 'stale_runtime_generation';
}

export function inferencePoolSummary(status: InferencePoolStatus | null | undefined) {
	if (!status) return 'Single unit';
	const parts = [
		status.ready_units > 0 ? `${status.ready_units} ready` : null,
		status.busy_units > 0 ? `${status.busy_units} busy` : null,
		status.unhealthy_units > 0 ? `${status.unhealthy_units} unhealthy` : null
	].filter((part): part is string => part !== null);
	return parts.length > 0 ? parts.join(' · ') : 'No units ready';
}

export function nextUnitId(units: readonly ExecutionUnitConfig[], prefix: 'cpu' | 'gpu') {
	const ids = new Set(units.map((unit) => unit.id));
	let suffix = 0;
	while (ids.has(`${prefix}-${suffix}`)) suffix += 1;
	return `${prefix}-${suffix}`;
}

function integerOr(value: number | undefined | null, fallback: number) {
	return typeof value === 'number' && Number.isFinite(value) ? Math.trunc(value) : fallback;
}

function optionalPositiveInteger(value: number | undefined | null) {
	return typeof value === 'number' && Number.isFinite(value) && value > 0
		? Math.trunc(value)
		: null;
}

function positiveInteger(value: number | undefined | null, fallback: number) {
	const integer = integerOr(value, fallback);
	return integer > 0 ? integer : fallback;
}

function nonNegativeInteger(value: number | undefined | null, fallback: number) {
	const integer = integerOr(value, fallback);
	return integer >= 0 ? integer : fallback;
}
