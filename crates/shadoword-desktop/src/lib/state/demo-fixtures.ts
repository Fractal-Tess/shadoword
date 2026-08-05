import type { DesktopSettings, OverviewDto } from '$lib/bindings';
import { demoModels } from '$lib/demo-data';

export const demoSettings: DesktopSettings = {
	mode: 'remote',
	model_path: '/var/lib/shadoword/models/ggml-turbo.bin',
	preload_on_startup: true,
	whisper_accelerator: 'gpu',
	whisper_gpu_device: 0,
	remote_endpoint: 'http://127.0.0.1:47813',
	remote_token_configured: true,
	openrouter_model: 'openai/whisper-large-v3',
	openrouter_key_configured: false,
	input_device: null,
	sample_rate: 16000,
	transcription_mode: 'batch',
	streaming_pcm_format: 'f32le',
	english_only: true,
	copy_to_clipboard: true,
	paste_method: 'direct',
	paste_delay_ms: 120,
	hotkey_shortcut: 'f2',
	hotkey_mode: 'push_to_talk',
	close_to_tray: true
};

export const demoOverview: OverviewDto = {
	status: {
		model_loaded: true,
		engine: 'whisper.cpp · CUDA',
		model_path: '/var/lib/shadoword/models/ggml-turbo.bin',
		whisper_accelerator: 'gpu',
		whisper_gpu_device: 0,
		compiled_whisper_backends: ['cpu', 'cuda'],
		available_gpu_devices: [
			{
				id: 0,
				name: 'NVIDIA GeForce RTX 3090',
				kind: 'dedicated',
				total_vram: 25_769_803_776,
				free_vram: 10_522_869_760
			},
			{
				id: 1,
				name: 'NVIDIA RTX A5000',
				kind: 'dedicated',
				total_vram: 25_769_803_776,
				free_vram: 7_784_628_224
			}
		],
		sample_rate: 16000,
		in_flight_requests: 0,
		queue_capacity: 32,
		inference_pool: {
			generation: 7,
			accepting: true,
			ready_units: 1,
			busy_units: 1,
			unhealthy_units: 1,
			queued_jobs: 3,
			queued_audio_bytes: 18_874_368,
			running_jobs: 1,
			running_audio_bytes: 6_291_456,
			completed: 1842,
			failed: 7,
			last_error: 'Optional CPU worker stopped after a backend initialization error.',
			units: [
				{
					id: 'gpu-main',
					required: true,
					target: { kind: 'gpu', device: 0, host_threads: 1 },
					state: 'ready',
					completed: 1124,
					failed: 2
				},
				{
					id: 'gpu-batch',
					required: true,
					target: { kind: 'gpu', device: 1, host_threads: 1 },
					state: 'busy',
					completed: 718,
					failed: 3
				},
				{
					id: 'cpu-spare',
					required: false,
					target: { kind: 'cpu', threads: 4 },
					state: 'unhealthy',
					last_error: 'CPU backend unavailable in the demo fixture.',
					completed: 0,
					failed: 2
				}
			],
			draining_generations: [
				{
					generation: 6,
					queued_jobs: 0,
					queued_audio_bytes: 0,
					running_jobs: 1,
					running_audio_bytes: 3_145_728,
					workers_remaining: 1
				}
			]
		}
	},
	runtime: {
		model_path: '/var/lib/shadoword/models/ggml-turbo.bin',
		whisper_accelerator: 'gpu',
		whisper_gpu_device: 0,
		english_only: true,
		preload_on_startup: true,
		inference_pool_explicit: true,
		generation: 7,
		inference_pool: {
			units: [
				{
					id: 'gpu-main',
					enabled: true,
					required: true,
					target: { kind: 'gpu', device: 0, host_threads: 1 }
				},
				{
					id: 'gpu-batch',
					enabled: true,
					required: true,
					target: { kind: 'gpu', device: 1, host_threads: 1 }
				},
				{
					id: 'cpu-spare',
					enabled: true,
					required: false,
					target: { kind: 'cpu', threads: 4 }
				}
			],
			limits: {
				max_queued_jobs: 32,
				max_queued_audio_bytes: 67_108_864,
				max_audio_bytes_per_job: 67_108_864,
				max_outstanding_per_flow: 8,
				max_buffered_results_per_flow: 32
			},
			preload_timeout_ms: 120_000,
			max_draining_generations: 2
		}
	},
	models: demoModels.map((model) => ({
		id: model.id,
		name: model.name,
		filename: `ggml-${model.id}.bin`,
		description: model.description,
		size_bytes: parseDemoSize(model.size),
		recommended: model.recommended ?? false,
		installed: model.installed
	}))
};

function parseDemoSize(size: string) {
	const value = Number.parseFloat(size);
	return Math.round(value * (size.includes('GiB') ? 1024 ** 3 : 1024 ** 2));
}
