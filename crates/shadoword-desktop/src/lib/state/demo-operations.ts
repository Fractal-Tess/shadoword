import type {
	DesktopSettings,
	DownloadJobStatus,
	OverviewDto,
	RuntimeConfigDto,
	TranscriptionMode,
	TranscriptionResult
} from '$lib/bindings';
import { isExplicitPool } from '$lib/inference-pool';
import { demoOverview } from './demo-fixtures';

export function demoOverviewForSettings(settings: DesktopSettings, overview: OverviewDto) {
	return {
		...overview,
		runtime: {
			...overview.runtime,
			model_path: settings.model_path,
			preload_on_startup: settings.preload_on_startup,
			whisper_accelerator: settings.whisper_accelerator,
			whisper_gpu_device: settings.whisper_gpu_device,
			english_only: settings.english_only
		}
	} satisfies OverviewDto;
}

export function demoOverviewAfterRuntime(overview: OverviewDto, runtime: RuntimeConfigDto) {
	const generation = (overview.runtime.generation ?? 0) + 1;
	const nextRuntime = { ...runtime, generation };
	if (!isExplicitPool(runtime) || !runtime.inference_pool) {
		return {
			...overview,
			runtime: nextRuntime,
			status: { ...overview.status, inference_pool: null }
		} satisfies OverviewDto;
	}
	const units = (runtime.inference_pool.units ?? []).filter((unit) => unit.enabled ?? true);
	return {
		...overview,
		runtime: nextRuntime,
		status: {
			...overview.status,
			model_loaded: units.length > 0,
			inference_pool: {
				generation,
				accepting: true,
				ready_units: units.length,
				busy_units: 0,
				unhealthy_units: 0,
				queued_jobs: 0,
				queued_audio_bytes: 0,
				running_jobs: 0,
				running_audio_bytes: 0,
				completed: 0,
				failed: 0,
				units: units.map((unit) => ({
					id: unit.id,
					required: unit.required ?? true,
					target: unit.target,
					state: 'ready',
					completed: 0,
					failed: 0
				})),
				draining_generations: []
			}
		}
	} satisfies OverviewDto;
}

export function withDemoModelLoaded(overview: OverviewDto) {
	return { ...overview, status: { ...overview.status, model_loaded: true } } satisfies OverviewDto;
}

export function demoDownload(modelId: string): DownloadJobStatus {
	const total = demoOverview.models.find((model) => model.id === modelId)?.size_bytes ?? 1;
	return {
		id: `demo-${modelId}`,
		model_id: modelId,
		state: 'running',
		downloaded: 0,
		total,
		path: null,
		skipped: false,
		verified: false,
		error: null
	};
}

export function advanceDemoDownload(status: DownloadJobStatus | undefined): DownloadJobStatus {
	const current = status ?? demoDownload('turbo');
	const downloaded = Math.min(current.total, current.downloaded + current.total * 0.35);
	return {
		...current,
		downloaded,
		state: downloaded >= current.total ? 'succeeded' : 'running',
		verified: downloaded >= current.total,
		path: downloaded >= current.total ? `/models/ggml-${current.model_id}.bin` : null
	};
}

export function demoOpenRouterModels() {
	return [
		{
			id: 'openai/whisper-large-v3',
			name: 'Whisper Large v3',
			description: 'High-accuracy multilingual speech recognition.'
		},
		{
			id: 'mistralai/voxtral-small-24b-2507',
			name: 'Voxtral Small 24B',
			description: 'Multilingual transcription through OpenRouter.'
		},
		{
			id: 'google/gemini-2.5-flash',
			name: 'Gemini 2.5 Flash',
			description: 'Fast multimodal transcription.'
		}
	];
}

export function demoTranscription(mode: TranscriptionMode): TranscriptionResult {
	return {
		text:
			mode === 'streaming'
				? 'Keep the API on the workstation. Let this desktop remain a lightweight capture client.'
				: 'Keep the API on the workstation and let this desktop remain a lightweight capture client.',
		elapsed_ms: 612,
		engine: 'whisper.cpp · CUDA',
		audio_duration_ms: 4200,
		sample_rate: 48000,
		cost_usd: null
	};
}
