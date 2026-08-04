import { describe, expect, test } from 'bun:test';
import type { RuntimeConfigDto, WhisperGpuDeviceInfo } from './bindings';
import {
	legacyRuntimeToExplicitPool,
	inferencePoolSummary,
	isStaleRuntimeError,
	normalizeInferencePool,
	runtimeWithInferencePool,
	validateInferencePoolCandidate
} from './inference-pool';

const runtime: RuntimeConfigDto = {
	model_path: '/models/turbo.bin',
	whisper_accelerator: 'auto',
	whisper_gpu_device: -1,
	english_only: true,
	preload_on_startup: true,
	generation: 12
};

const devices: WhisperGpuDeviceInfo[] = [
	{ id: 3, name: 'GPU three', kind: 'dedicated', total_vram: 16, free_vram: 8 }
];

describe('inference pool helpers', () => {
	test('converts legacy auto to a valid explicit detected target', () => {
		expect(legacyRuntimeToExplicitPool(runtime, devices).units?.[0]).toEqual({
			id: 'gpu-3',
			enabled: true,
			required: true,
			target: { kind: 'gpu', device: 3, host_threads: 1 }
		});
		expect(legacyRuntimeToExplicitPool(runtime, []).units?.[0]?.target).toEqual({
			kind: 'cpu',
			threads: 4
		});
	});

	test('normalizes candidate defaults and stable IDs without mutating the source', () => {
		const candidate = {
			units: [{ id: ' gpu-main ', target: { kind: 'gpu' as const, device: 3 } }]
		};
		const normalized = normalizeInferencePool(candidate);
		expect(candidate.units[0].id).toBe(' gpu-main ');
		expect(normalized.units?.[0]).toEqual({
			id: 'gpu-main',
			enabled: true,
			required: true,
			target: { kind: 'gpu', device: 3, host_threads: null }
		});
		expect(normalized.limits?.max_queued_jobs).toBe(32);
	});

	test('reports duplicate IDs and duplicate enabled GPU assignments by field', () => {
		const result = validateInferencePoolCandidate({
			units: [
				{ id: 'same', target: { kind: 'gpu', device: 3 } },
				{ id: 'same', target: { kind: 'gpu', device: 3 } }
			]
		});
		expect(result.fieldErrors['units.0.id']).toContain('row 2');
		expect(result.fieldErrors['units.1.id']).toContain('row 1');
		expect(result.fieldErrors['units.0.device']).toContain('row 2');
		expect(result.fieldErrors['units.1.device']).toContain('row 1');
	});

	test('preserves every runtime field and expected generation', () => {
		const pool = legacyRuntimeToExplicitPool(runtime, devices);
		expect(runtimeWithInferencePool(runtime, pool)).toEqual({
			...runtime,
			inference_pool: pool,
			inference_pool_explicit: true
		});
		expect(runtimeWithInferencePool(runtime, null).inference_pool_explicit).toBe(false);
	});

	test('recognizes the native stale-generation code for local and remote updates', () => {
		expect(isStaleRuntimeError({ code: 'stale_runtime_generation' })).toBe(true);
		expect(isStaleRuntimeError({ code: 'remote_request_failed' })).toBe(false);
	});

	test('summarizes operational pool state without color-only meaning', () => {
		expect(
			inferencePoolSummary({
				generation: 4,
				units: [],
				accepting: true,
				ready_units: 2,
				busy_units: 1,
				unhealthy_units: 1,
				queued_jobs: 0,
				queued_audio_bytes: 0,
				running_jobs: 0,
				running_audio_bytes: 0,
				completed: 0,
				failed: 0
			})
		).toBe('2 ready · 1 busy · 1 unhealthy');
	});
});
