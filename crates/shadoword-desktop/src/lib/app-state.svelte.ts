import type {
	ConnectionInput,
	DesktopSettings,
	DesktopSettingsInput,
	DownloadJobStatus,
	InferencePoolConfig,
	InputDeviceInfo,
	OpenRouterKeyReport,
	OpenRouterModelInfo,
	OverviewDto,
	RuntimeConfigDto,
	ServiceMode,
	TranscriptionMode,
	TranscriptionResult
} from '$lib/bindings';
import type { TranscriptSegments } from '$lib/desktop-events';
import type { PoolFieldErrors } from '$lib/inference-pool';
import type { HistoryRecord } from '$lib/types';
import { CaptureOperations } from '$lib/state/capture-operations';
import type {
	ActivityState,
	CaptureState,
	DesktopStateContext,
	OpenRouterModelsState,
	PoolApplyState,
	PoolValidationState
} from '$lib/state/contracts';
import { DownloadOperations } from '$lib/state/download-operations';
import { LifecycleOperations } from '$lib/state/lifecycle-operations';
import { PoolOperations } from '$lib/state/pool-operations';
import { ProviderOperations } from '$lib/state/provider-operations';
import { RuntimeOperations } from '$lib/state/runtime-operations';

export type {
	ActivityState,
	CaptureState,
	PoolApplyState,
	PoolValidationState
} from '$lib/state/contracts';
export { settingsInput } from '$lib/state/settings-input';

export class DesktopAppState implements DesktopStateContext {
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
	openRouterModels = $state.raw<OpenRouterModelInfo[]>([]);
	openRouterModelsState = $state<OpenRouterModelsState>('idle');
	openRouterModelsError = $state<string | null>(null);
	openRouterKeyReport = $state.raw<OpenRouterKeyReport | null>(null);
	openRouterKeyMessage = $state<string | null>(null);
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

	private captureOperations: CaptureOperations;
	private downloadOperations: DownloadOperations;
	private lifecycleOperations: LifecycleOperations;
	private poolOperations: PoolOperations;
	private providerOperations: ProviderOperations;
	private runtimeOperations: RuntimeOperations;

	constructor(demo: boolean) {
		this.demo = demo;
		this.captureOperations = new CaptureOperations(this);
		this.downloadOperations = new DownloadOperations(this);
		this.providerOperations = new ProviderOperations(this);
		this.runtimeOperations = new RuntimeOperations(this, () =>
			this.downloadOperations.resetModeScopedState()
		);
		this.poolOperations = new PoolOperations(this);
		this.lifecycleOperations = new LifecycleOperations(
			this,
			this.captureOperations,
			this.downloadOperations
		);
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
		return this.lifecycleOperations.initialize();
	}

	dispose() {
		this.lifecycleOperations.dispose();
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

	refreshOverview() {
		return this.providerOperations.refreshOverview();
	}

	testConnection(input: ConnectionInput) {
		return this.providerOperations.testConnection(input);
	}

	refreshOpenRouterModels() {
		return this.providerOperations.refreshOpenRouterModels();
	}

	testOpenRouterKey(key: string | null, useSavedKey: boolean) {
		return this.providerOperations.testOpenRouterKey(key, useSavedKey);
	}

	saveSettings(input: DesktopSettingsInput) {
		return this.runtimeOperations.saveSettings(input);
	}

	setMode(mode: ServiceMode) {
		return this.runtimeOperations.setMode(mode);
	}

	refreshInputDevices() {
		return this.providerOperations.refreshInputDevices();
	}

	updateRuntime(runtime: RuntimeConfigDto) {
		return this.runtimeOperations.updateRuntime(runtime);
	}

	clearPoolDraftFeedback() {
		this.poolOperations.clearDraftFeedback();
	}

	validateInferencePoolDraft(pool: InferencePoolConfig) {
		return this.poolOperations.validateDraft(pool);
	}

	applyInferencePoolDraft(pool: InferencePoolConfig | null) {
		return this.poolOperations.applyDraft(pool);
	}

	preloadLocalModel() {
		return this.runtimeOperations.preloadLocalModel();
	}

	selectModel(modelId: string) {
		return this.runtimeOperations.selectModel(modelId);
	}

	startDownload(modelId: string) {
		return this.downloadOperations.start(modelId);
	}

	stopWatchingDownload(modelId: string) {
		this.downloadOperations.stopWatching(modelId);
	}

	startRecording() {
		return this.captureOperations.start();
	}

	stopRecording() {
		return this.captureOperations.stop();
	}

	cancelRecording() {
		return this.captureOperations.cancel();
	}

	clearHistory() {
		this.history = [];
	}
}
