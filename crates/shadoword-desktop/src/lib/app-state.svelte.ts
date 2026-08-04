import {
	commands,
	events,
	type ConnectionInput,
	type DesktopEvent,
	type DesktopSettings,
	type DesktopSettingsInput,
	type DownloadJobStatus,
	type InferencePoolConfig,
	type InputDeviceInfo,
	type OverviewDto,
	type RecordingState,
	type RuntimeConfigDto,
	type SecretUpdate,
	type ServiceMode,
	type TranscriptionMode,
	type TranscriptionResult
} from '$lib/bindings';
import {
	historyRecordFromCompletion,
	mergeTranscriptSegment,
	transcriptFromSegments,
	transcriptionFingerprint,
	type TranscriptSegments
} from '$lib/desktop-events';
import { demoHistory, demoModels } from '$lib/demo-data';
import { errorMessage } from '$lib/display';
import {
	isExplicitPool,
	isStaleRuntimeError,
	runtimeWithInferencePool,
	validateInferencePoolCandidate,
	type PoolFieldErrors
} from '$lib/inference-pool';
import { commandNamesForMode } from '$lib/native-routing';
import type { HistoryRecord } from '$lib/types';
import { SvelteMap, SvelteSet } from 'svelte/reactivity';

export type ActivityState = 'booting' | 'ready' | 'busy' | 'offline';
export type CaptureState = 'idle' | 'recording' | 'finalizing' | 'error';
export type PoolValidationState = 'idle' | 'validating' | 'valid' | 'invalid';
export type PoolApplyState = 'idle' | 'applying' | 'applied' | 'failed' | 'stale';

const demoSettings: DesktopSettings = {
	mode: 'remote',
	model_path: '/var/lib/shadoword/models/ggml-turbo.bin',
	preload_on_startup: true,
	whisper_accelerator: 'gpu',
	whisper_gpu_device: 0,
	remote_endpoint: 'http://127.0.0.1:47813',
	remote_token_configured: true,
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

const demoOverview: OverviewDto = {
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

export class DesktopAppState {
	readonly demo: boolean;
	activity = $state<ActivityState>('booting');
	settings = $state.raw<DesktopSettings | null>(null);
	inputDevices = $state.raw<InputDeviceInfo[]>([]);
	overview = $state.raw<OverviewDto | null>(null);
	history = $state.raw<HistoryRecord[]>([]);
	error = $state<string | null>(null);
	errorRetry = $state<'overview' | null>(null);
	inputDevicesError = $state<string | null>(null);
	connectionMessage = $state<string | null>(null);
	hotkeyError = $state<string | null>(null);
	statusMessage = $state('Starting native desktop…');
	captureState = $state<CaptureState>('idle');
	recordingSampleRate = $state(0);
	recordingMode = $state<ServiceMode | null>(null);
	recordingTranscriptionMode = $state<TranscriptionMode | null>(null);
	transcript = $state('');
	lastResult = $state.raw<TranscriptionResult | null>(null);
	segmentResults = $state.raw<TranscriptSegments>({});
	downloads = $state.raw<Record<string, DownloadJobStatus>>({});
	downloadWatching = $state.raw<Record<string, boolean>>({});
	poolValidationState = $state<PoolValidationState>('idle');
	poolApplyState = $state<PoolApplyState>('idle');
	poolFieldErrors = $state.raw<PoolFieldErrors>({});
	poolFeedback = $state<string | null>(null);
	validatedPool = $state.raw<InferencePoolConfig | null>(null);

	private polling = new SvelteSet<string>();
	private downloadModes = new SvelteMap<string, ServiceMode>();
	private activeSessionId: string | null = null;
	private completedSessionId: string | null = null;
	private sessionSequence = 0;
	private lastCompletion: { fingerprint: string; at: number } | null = null;
	private eventRevision = 0;
	private listenerPromise: Promise<() => void> | null = null;
	private unlisten: (() => void) | null = null;
	private listenerClosed = false;
	private initialization: Promise<void> | null = null;
	private disposed = false;

	constructor(demo: boolean) {
		this.demo = demo;
	}

	get recording() {
		return this.captureState === 'recording';
	}

	get processing() {
		return this.captureState === 'finalizing';
	}

	get captureLocked() {
		return this.captureState === 'recording' || this.captureState === 'finalizing';
	}

	get drainingPool() {
		return (this.overview?.status.inference_pool?.draining_generations?.length ?? 0) > 0;
	}

	get poolMutationLocked() {
		return (
			this.captureLocked ||
			this.activity === 'busy' ||
			this.poolValidationState === 'validating' ||
			this.poolApplyState === 'applying' ||
			this.drainingPool
		);
	}

	get segmentCount() {
		return Object.keys(this.segmentResults).length;
	}

	initialize() {
		this.initialization ??= this.initializeOnce();
		return this.initialization;
	}

	dispose() {
		this.disposed = true;
		this.polling.clear();
		this.downloadWatching = {};
		if (this.unlisten) this.closeListener(this.unlisten);
		else if (this.listenerPromise)
			void this.listenerPromise.then((unlisten) => this.closeListener(unlisten));
	}

	clearError() {
		this.error = null;
		this.errorRetry = null;
		if (this.captureState === 'error') this.captureState = 'idle';
	}

	async retryError() {
		if (this.errorRetry === 'overview') await this.refreshOverview();
		else this.clearError();
	}

	async refreshOverview() {
		if (this.demo) {
			this.overview ??= { ...demoOverview };
			this.activity = 'ready';
			this.statusMessage = 'Simulated runtime ready';
			return;
		}
		const mode = this.settings?.mode;
		if (!mode) return;
		this.activity = 'busy';
		this.error = null;
		this.errorRetry = null;
		try {
			const route = commandNamesForMode(mode);
			this.overview = await commands[route.refreshOverview]();
			this.activity = 'ready';
			this.statusMessage = `${mode === 'local' ? 'Local' : 'Remote'} runtime connected`;
		} catch (error) {
			this.failOverview(error, `Could not refresh the ${mode} runtime`);
		}
	}

	async testConnection(input: ConnectionInput) {
		this.connectionMessage = null;
		this.error = null;
		this.activity = 'busy';
		if (this.demo) {
			this.overview = demoOverview;
			this.connectionMessage = 'Simulated connection verified · health, status, and runtime';
			this.activity = 'ready';
			return;
		}
		try {
			const report = await commands.testRemoteConnection(input);
			this.overview = report.overview;
			this.connectionMessage = `Connected · health ${report.health_ok ? 'ok' : 'failed'} · ${report.status_model_loaded ? 'model ready' : 'model unloaded'}`;
			this.activity = 'ready';
		} catch (error) {
			this.activity = this.overview ? 'ready' : 'offline';
			this.setError(error, 'Connection test failed');
			throw error;
		}
	}

	async saveSettings(input: DesktopSettingsInput) {
		if (this.captureLocked) throw new Error('Stop the active recording before saving settings.');
		this.activity = 'busy';
		this.error = null;
		const previousMode = this.settings?.mode;
		if (this.demo) {
			const { remote_token: remoteToken, ...settings } = input;
			this.settings = {
				...settings,
				remote_token_configured:
					remoteToken.action === 'set' ||
					(remoteToken.action === 'keep' && (this.settings?.remote_token_configured ?? false))
			};
			this.overview = demoOverviewForSettings(this.settings, this.overview ?? demoOverview);
			this.activity = 'ready';
			this.statusMessage = 'Simulated settings saved';
			return;
		}
		try {
			this.settings = await commands.saveDesktopSettings(input);
			this.hotkeyError = null;
			if (previousMode !== this.settings.mode) this.resetModeScopedState();
			await this.refreshOverview();
		} catch (error) {
			this.activity = this.overview ? 'ready' : 'offline';
			this.setError(error, 'Could not save desktop settings');
			throw error;
		}
	}

	async setMode(mode: ServiceMode) {
		if (!this.settings || this.settings.mode === mode || this.captureLocked) return;
		await this.saveSettings(settingsInput(this.settings, { action: 'keep' }, mode));
	}

	async refreshInputDevices() {
		this.inputDevicesError = null;
		if (this.demo) return;
		try {
			this.inputDevices = await commands.listInputDevices();
		} catch (error) {
			this.inputDevicesError = errorMessage(error);
		}
	}

	async updateRuntime(runtime: RuntimeConfigDto) {
		const mode = this.settings?.mode;
		if (!mode) throw new Error('Select a local or remote runtime before applying changes.');
		if (this.captureLocked)
			throw new Error('Finish the active recording before applying runtime changes.');
		this.activity = 'busy';
		this.error = null;
		try {
			if (this.demo) {
				this.overview = demoOverviewAfterRuntime(this.overview ?? demoOverview, runtime);
			} else {
				const route = commandNamesForMode(mode);
				this.overview = await commands[route.updateRuntime](runtime);
			}
			if (this.overview) this.syncLocalSettingsFromRuntime(mode, this.overview.runtime);
			this.activity = 'ready';
			this.statusMessage = `${mode === 'local' ? 'Local' : 'Remote'} runtime updated`;
		} catch (error) {
			this.activity = this.overview ? 'ready' : 'offline';
			this.setError(error, `Could not update the ${mode} runtime`);
			throw error;
		}
	}

	clearPoolDraftFeedback() {
		this.poolValidationState = 'idle';
		this.poolApplyState = 'idle';
		this.poolFieldErrors = {};
		this.poolFeedback = null;
		this.validatedPool = null;
	}

	async validateInferencePoolDraft(pool: InferencePoolConfig) {
		if (this.captureLocked) throw new Error('Finish the active recording before validation.');
		const local = validateInferencePoolCandidate(pool);
		this.poolFieldErrors = local.fieldErrors;
		this.validatedPool = null;
		if (local.globalError) {
			this.poolValidationState = 'invalid';
			this.poolFeedback = local.globalError;
			throw new Error(local.globalError);
		}

		this.poolValidationState = 'validating';
		this.poolApplyState = 'idle';
		this.poolFeedback = 'Checking execution targets and resource limits…';
		try {
			const effective =
				this.demo || this.settings?.mode === 'remote'
					? local.pool
					: await commands.validateLocalInferencePool(local.pool);
			this.validatedPool = effective;
			this.poolValidationState = 'valid';
			this.poolFeedback =
				this.settings?.mode === 'local'
					? 'Pool is valid for the detected local hardware.'
					: 'Pool shape is valid. The remote host will verify hardware during apply.';
			return effective;
		} catch (error) {
			this.poolValidationState = 'invalid';
			this.poolFeedback = errorMessage(error);
			throw error;
		}
	}

	async applyInferencePoolDraft(pool: InferencePoolConfig | null) {
		if (this.poolMutationLocked)
			throw new Error(
				this.drainingPool
					? 'Wait for the draining generation to finish before reloading the pool.'
					: 'The runtime is busy. Wait for the current operation to finish.'
			);
		const runtime = this.overview?.runtime;
		if (!runtime) throw new Error('Refresh the runtime before applying an inference pool.');

		this.poolApplyState = 'applying';
		this.poolFeedback = 'Validating, loading units, and preparing the next generation…';
		try {
			const effective = pool === null ? null : await this.validateInferencePoolDraft(pool);
			this.poolApplyState = 'applying';
			await this.updateRuntime(runtimeWithInferencePool(runtime, effective));
			this.poolApplyState = 'applied';
			this.poolValidationState = 'valid';
			this.poolFeedback = `Generation ${this.overview?.runtime.generation ?? 'updated'} is active.`;
			return this.overview;
		} catch (error) {
			if (isStaleRuntimeError(error)) {
				this.poolApplyState = 'stale';
				await this.refreshOverview();
				this.poolFeedback =
					'The runtime changed elsewhere. Active state was refreshed; review the draft and retry.';
			} else {
				this.poolApplyState = 'failed';
				this.poolFeedback = `The active pool was kept unchanged. ${errorMessage(error)}`;
			}
			throw error;
		}
	}

	async preloadLocalModel() {
		if (this.settings?.mode !== 'local' || this.captureLocked) return;
		this.activity = 'busy';
		this.error = null;
		this.statusMessage = 'Loading the selected local model…';
		try {
			this.overview = this.demo
				? withDemoModelLoaded(this.overview ?? demoOverview)
				: await commands.preloadLocalModel();
			this.activity = 'ready';
			this.statusMessage = 'Local model loaded';
		} catch (error) {
			this.activity = this.overview ? 'ready' : 'offline';
			this.setError(error, 'Could not load the selected local model');
		}
	}

	async selectModel(modelId: string) {
		const mode = this.settings?.mode;
		if (!mode || this.captureLocked) return;
		this.activity = 'busy';
		this.error = null;
		try {
			if (this.demo) {
				const overview = this.overview ?? demoOverview;
				const model = overview.models.find((item) => item.id === modelId);
				if (model) {
					this.overview = {
						...overview,
						runtime: { ...overview.runtime, model_path: `/models/${model.filename}` },
						status: { ...overview.status, model_path: `/models/${model.filename}` }
					};
				}
			} else {
				const route = commandNamesForMode(mode);
				this.overview = await commands[route.selectModel](modelId);
			}
			if (this.overview) this.syncLocalSettingsFromRuntime(mode, this.overview.runtime);
			this.activity = 'ready';
			this.statusMessage = `${mode === 'local' ? 'Local' : 'Remote'} model selected`;
		} catch (error) {
			this.activity = this.overview ? 'ready' : 'offline';
			this.setError(error, `Could not select the ${mode} model`);
		}
	}

	async startDownload(modelId: string) {
		const mode = this.settings?.mode;
		if (!mode || this.captureLocked) return;
		this.error = null;
		try {
			const route = commandNamesForMode(mode);
			const status = this.demo
				? demoDownload(modelId)
				: await commands[route.startDownload](modelId);
			this.setDownload(modelId, status, true);
			this.downloadModes.set(modelId, mode);
			this.polling.add(modelId);
			void this.pollDownload(modelId, status.id, mode);
		} catch (error) {
			this.setError(error, `Could not start the ${mode} model download`);
		}
	}

	stopWatchingDownload(modelId: string) {
		this.polling.delete(modelId);
		this.downloadModes.delete(modelId);
		this.downloadWatching = { ...this.downloadWatching, [modelId]: false };
	}

	async startRecording() {
		if (this.captureLocked) return;
		this.clearError();
		this.statusMessage = 'Starting microphone capture…';
		if (this.demo) {
			this.handleDesktopEvent({ type: 'recording_started', sample_rate: 48_000 });
			return;
		}
		try {
			const status = await commands.startRecording();
			if (status.recording && this.captureState !== 'recording') {
				this.beginRecording(status.sample_rate);
				void this.syncRecordingMetadata(this.activeSessionId);
			}
		} catch (error) {
			this.failCapture(error, 'Could not start microphone capture');
		}
	}

	async stopRecording() {
		if (this.captureState !== 'recording') return;
		const sessionId = this.activeSessionId;
		this.captureState = 'finalizing';
		this.statusMessage = 'Finalizing the recording…';
		try {
			if (this.demo) {
				const result = demoTranscription(this.recordingTranscriptionMode ?? 'batch');
				if (this.recordingTranscriptionMode === 'streaming') {
					this.handleDesktopEvent({
						type: 'transcript_segment',
						result: { ...result, text: 'Keep the API on the workstation.' },
						segment_index: 0
					});
				}
				this.handleDesktopEvent({
					type: 'transcription_complete',
					result,
					segments: this.recordingTranscriptionMode === 'streaming' ? 1 : 1
				});
				return;
			}
			const result = await commands.stopAndTranscribe();
			const segments = this.recordingTranscriptionMode === 'streaming' ? this.segmentCount : 1;
			this.completeTranscription(result, segments, sessionId);
		} catch (error) {
			if (this.completedSessionId !== sessionId) {
				this.failCapture(error, 'Transcription failed');
			}
		}
	}

	async cancelRecording() {
		if (this.captureState !== 'recording') return;
		try {
			if (this.demo) this.handleDesktopEvent({ type: 'recording_cancelled' });
			else {
				await commands.cancelRecording();
				if (this.captureState === 'recording') this.cancelActiveSession();
			}
		} catch (error) {
			this.failCapture(error, 'Could not cancel the recording');
		}
	}

	clearHistory() {
		this.history = [];
	}

	private async initializeOnce() {
		this.disposed = false;
		this.listenerClosed = false;
		this.activity = 'booting';
		this.error = null;
		if (this.demo) {
			this.settings = { ...demoSettings };
			this.inputDevices = [
				{ name: 'Studio Microphone', is_default: true },
				{ name: 'Webcam Microphone', is_default: false }
			];
			this.overview = demoOverview;
			this.history = demoHistory.map((record) => ({ ...record }));
			this.activity = 'ready';
			this.statusMessage = 'Simulated desktop ready';
			return;
		}

		try {
			await this.subscribeToDesktopEvents();
			const revisionBeforeLoad = this.eventRevision;
			const bootstrap = await commands.loadDesktopState();
			if (this.disposed) return;
			this.settings = bootstrap.settings;
			this.inputDevices = bootstrap.input_devices;
			this.inputDevicesError = bootstrap.input_devices_error;
			this.hotkeyError = bootstrap.hotkey_error;
			const localStartupError = bootstrap.local_startup_error;
			this.overview =
				bootstrap.settings.mode === 'local' ? bootstrap.local_overview : this.overview;
			if (revisionBeforeLoad === this.eventRevision) this.hydrateRecording(bootstrap.recording);
			await this.refreshOverview();
			if (localStartupError) {
				this.error = `Local model: ${localStartupError.message}${localStartupError.action ? ` ${localStartupError.action}` : ''}`;
				this.statusMessage = 'Local model preload failed';
			}
		} catch (error) {
			if (!this.disposed) this.failOverview(error, 'Could not initialize the native desktop host');
		}
	}

	private async subscribeToDesktopEvents() {
		if (this.listenerPromise) return this.listenerPromise;
		this.listenerPromise = events.shadowordDesktopEvent.listen((event) => {
			if (!this.disposed) this.handleDesktopEvent(event.payload);
		});
		const unlisten = await this.listenerPromise;
		if (this.disposed) this.closeListener(unlisten);
		else this.unlisten = unlisten;
		return unlisten;
	}

	private closeListener(unlisten: () => void) {
		if (this.listenerClosed) return;
		this.listenerClosed = true;
		unlisten();
		this.unlisten = null;
	}

	private handleDesktopEvent(event: DesktopEvent) {
		this.eventRevision += 1;
		switch (event.type) {
			case 'status':
				this.statusMessage = event.message;
				if (event.message.toLowerCase().includes('ready')) void this.refreshOverview();
				break;
			case 'recording_started':
				this.beginRecording(event.sample_rate);
				void this.syncRecordingMetadata(this.activeSessionId);
				break;
			case 'recording_stopped':
				if (event.processing) {
					this.captureState = 'finalizing';
					this.statusMessage = 'Finalizing the recording…';
				} else if (this.captureState !== 'error') {
					this.captureState = 'idle';
					this.statusMessage = 'Recording stopped';
				}
				break;
			case 'recording_cancelled':
				this.cancelActiveSession();
				break;
			case 'transcript_segment':
				this.acceptTranscriptSegment(event.segment_index, event.result);
				break;
			case 'transcription_complete':
				this.completeTranscription(event.result, event.segments);
				break;
			case 'error':
				this.handleNativeError(event);
				break;
		}
	}

	private hydrateRecording(recording: RecordingState) {
		if (recording.phase === 'idle') return;
		this.activeSessionId = this.nextSessionId();
		this.recordingMode = recording.service_mode ?? this.settings?.mode ?? null;
		this.recordingTranscriptionMode =
			recording.transcription_mode ?? this.settings?.transcription_mode ?? null;
		this.recordingSampleRate = recording.sample_rate ?? 0;
		this.captureState = recording.phase;
		this.statusMessage = recording.phase === 'recording' ? 'Recording in progress' : 'Finalizing';
	}

	private async syncRecordingMetadata(sessionId: string | null) {
		if (this.demo || !sessionId) return;
		try {
			const recording = await commands.getRecordingState();
			if (this.activeSessionId !== sessionId || recording.phase === 'idle') return;
			this.recordingMode = recording.service_mode ?? this.recordingMode;
			this.recordingTranscriptionMode =
				recording.transcription_mode ?? this.recordingTranscriptionMode;
			this.recordingSampleRate = recording.sample_rate ?? this.recordingSampleRate;
		} catch {
			// Event state remains authoritative when a metadata refresh races with shutdown.
		}
	}

	private beginRecording(sampleRate: number) {
		if (this.captureState === 'error') this.clearError();
		if (this.captureState !== 'recording' || !this.activeSessionId) {
			this.activeSessionId = this.nextSessionId();
			this.completedSessionId = null;
			this.recordingMode = this.settings?.mode ?? null;
			this.recordingTranscriptionMode = this.settings?.transcription_mode ?? null;
			this.segmentResults = {};
			this.transcript = '';
			this.lastResult = null;
		}
		this.captureState = 'recording';
		this.recordingSampleRate = sampleRate;
		this.statusMessage = `${this.recordingTranscriptionMode === 'streaming' ? 'Streaming' : 'Batch'} recording in progress`;
	}

	private acceptTranscriptSegment(segmentIndex: number, result: TranscriptionResult) {
		if (!this.activeSessionId) this.beginRecording(result.sample_rate);
		this.segmentResults = mergeTranscriptSegment(this.segmentResults, segmentIndex, result);
		this.transcript = transcriptFromSegments(this.segmentResults);
		this.lastResult = result;
		this.statusMessage = `Received transcript segment ${segmentIndex + 1}`;
	}

	private completeTranscription(
		result: TranscriptionResult,
		segments: number,
		expectedSessionId?: string | null
	) {
		const sessionId = expectedSessionId ?? this.activeSessionId ?? this.nextSessionId();
		if (this.completedSessionId === sessionId) return;
		const mode = this.recordingMode ?? this.settings?.mode ?? 'remote';
		const fingerprint = transcriptionFingerprint(mode, result, segments);
		const now = Date.now();
		if (
			!this.activeSessionId &&
			this.lastCompletion?.fingerprint === fingerprint &&
			now - this.lastCompletion.at < 2_000
		) {
			return;
		}

		this.completedSessionId = sessionId;
		this.lastCompletion = { fingerprint, at: now };
		this.lastResult = result;
		this.transcript = result.text;
		if (result.text.trim()) {
			const record = historyRecordFromCompletion(
				sessionId,
				new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit' }).format(now),
				mode,
				result,
				segments
			);
			if (!this.history.some((item) => item.id === record.id))
				this.history = [record, ...this.history];
		}
		this.captureState = 'idle';
		this.recordingSampleRate = result.sample_rate;
		this.statusMessage = `Transcription complete · ${segments} ${segments === 1 ? 'segment' : 'segments'}`;
		this.activity = 'ready';
		this.activeSessionId = null;
		this.recordingMode = null;
		this.recordingTranscriptionMode = null;
	}

	private cancelActiveSession() {
		this.captureState = 'idle';
		this.statusMessage = 'Recording cancelled';
		this.activeSessionId = null;
		this.recordingMode = null;
		this.recordingTranscriptionMode = null;
		this.segmentResults = {};
	}

	private handleNativeError(event: Extract<DesktopEvent, { type: 'error' }>) {
		const context = sentenceCase(event.context);
		this.error = `${context}: ${event.message}${event.action ? ` ${event.action}` : ''}`;
		this.errorRetry = null;
		this.statusMessage = `${context} error`;
		if (event.context === 'hotkey') this.hotkeyError = event.message;
		if (
			event.context === 'streaming' ||
			event.context === 'transcription' ||
			(event.context === 'hotkey' && !event.code.includes('unavailable'))
		) {
			this.captureState = 'error';
			this.activeSessionId = null;
		}
	}

	private async pollDownload(modelId: string, jobId: string, mode: ServiceMode) {
		let transientFailures = 0;
		while (this.polling.has(modelId) && this.downloadModes.get(modelId) === mode) {
			await delay(this.demo ? 350 : 1000);
			if (!this.polling.has(modelId) || this.downloadModes.get(modelId) !== mode) return;
			try {
				const previous = this.downloads[modelId];
				const route = commandNamesForMode(mode);
				const status = this.demo
					? advanceDemoDownload(previous)
					: await commands[route.pollDownload](jobId);
				transientFailures = 0;
				this.setDownload(modelId, status, true);
				if (status.state === 'succeeded' || status.state === 'failed') {
					this.stopWatchingDownload(modelId);
					if (this.demo && status.state === 'succeeded' && this.overview) {
						this.overview = {
							...this.overview,
							models: this.overview.models.map((model) =>
								model.id === modelId ? { ...model, installed: true } : model
							)
						};
					}
					if (this.settings?.mode === mode) await this.refreshOverview();
					return;
				}
			} catch (error) {
				if (transientFailures < 3) {
					transientFailures += 1;
					continue;
				}
				this.stopWatchingDownload(modelId);
				this.setError(error, `${sentenceCase(mode)} download polling stopped after three retries`);
			}
		}
	}

	private setDownload(modelId: string, status: DownloadJobStatus, watching: boolean) {
		this.downloads = { ...this.downloads, [modelId]: status };
		this.downloadWatching = { ...this.downloadWatching, [modelId]: watching };
	}

	private resetModeScopedState() {
		this.polling.clear();
		this.downloadModes.clear();
		this.downloads = {};
		this.downloadWatching = {};
		this.overview = null;
	}

	private syncLocalSettingsFromRuntime(mode: ServiceMode, runtime: RuntimeConfigDto) {
		if (mode !== 'local' || !this.settings) return;
		this.settings = {
			...this.settings,
			model_path: runtime.model_path,
			preload_on_startup: runtime.preload_on_startup,
			whisper_accelerator: runtime.whisper_accelerator,
			whisper_gpu_device: runtime.whisper_gpu_device,
			english_only: runtime.english_only
		};
	}

	private nextSessionId() {
		this.sessionSequence += 1;
		return `session-${Date.now()}-${this.sessionSequence}`;
	}

	private failOverview(error: unknown, context: string) {
		this.activity = 'offline';
		this.setError(error, context);
		this.errorRetry = 'overview';
		this.statusMessage = 'Runtime unavailable';
	}

	private failCapture(error: unknown, context: string) {
		this.captureState = 'error';
		this.activeSessionId = null;
		this.setError(error, context);
		this.statusMessage = 'Capture error';
	}

	private setError(error: unknown, context: string) {
		this.error = `${context}: ${errorMessage(error)}`;
		this.errorRetry = null;
	}
}

export function settingsInput(
	settings: DesktopSettings,
	remoteToken: SecretUpdate,
	mode = settings.mode
): DesktopSettingsInput {
	return {
		mode,
		model_path: settings.model_path,
		preload_on_startup: settings.preload_on_startup,
		whisper_accelerator: settings.whisper_accelerator,
		whisper_gpu_device: settings.whisper_gpu_device,
		remote_endpoint: settings.remote_endpoint,
		remote_token: remoteToken,
		input_device: settings.input_device,
		sample_rate: settings.sample_rate,
		transcription_mode: settings.transcription_mode,
		streaming_pcm_format: settings.streaming_pcm_format,
		english_only: settings.english_only,
		copy_to_clipboard: settings.copy_to_clipboard,
		paste_method: settings.paste_method,
		paste_delay_ms: settings.paste_delay_ms,
		hotkey_shortcut: settings.hotkey_shortcut,
		hotkey_mode: settings.hotkey_mode,
		close_to_tray: settings.close_to_tray
	};
}

function parseDemoSize(size: string) {
	const value = Number.parseFloat(size);
	return Math.round(value * (size.includes('GiB') ? 1024 ** 3 : 1024 ** 2));
}

function demoOverviewForSettings(settings: DesktopSettings, overview: OverviewDto): OverviewDto {
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
	};
}

function demoOverviewAfterRuntime(overview: OverviewDto, runtime: RuntimeConfigDto): OverviewDto {
	const generation = (overview.runtime.generation ?? 0) + 1;
	const nextRuntime = { ...runtime, generation };
	if (!isExplicitPool(runtime) || !runtime.inference_pool) {
		return {
			...overview,
			runtime: nextRuntime,
			status: { ...overview.status, inference_pool: null }
		};
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
	};
}

function withDemoModelLoaded(overview: OverviewDto): OverviewDto {
	return { ...overview, status: { ...overview.status, model_loaded: true } };
}

function demoDownload(modelId: string): DownloadJobStatus {
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

function advanceDemoDownload(status: DownloadJobStatus | undefined): DownloadJobStatus {
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

function demoTranscription(mode: TranscriptionMode): TranscriptionResult {
	return {
		text:
			mode === 'streaming'
				? 'Keep the API on the workstation. Let this desktop remain a lightweight capture client.'
				: 'Keep the API on the workstation and let this desktop remain a lightweight capture client.',
		elapsed_ms: 612,
		engine: 'whisper.cpp · CUDA',
		audio_duration_ms: 4200,
		sample_rate: 48000
	};
}

function sentenceCase(value: string) {
	return value.replaceAll('_', ' ').replace(/^./, (character) => character.toUpperCase());
}

function delay(milliseconds: number) {
	return new Promise<void>((resolve) => globalThis.setTimeout(resolve, milliseconds));
}
