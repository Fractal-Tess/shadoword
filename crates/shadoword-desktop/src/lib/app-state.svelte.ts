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
import { toast } from 'svelte-sonner';
import { CaptureOperations } from '$lib/state/capture-operations';
import type {
	ActivityState,
	CaptureState,
	DesktopStateContext,
	NotificationVariant,
	OpenRouterCredentialState,
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
	openRouterCredentialState = $state<OpenRouterCredentialState>('missing');
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
		this.runtimeOperations = new RuntimeOperations(
			this,
			() => this.downloadOperations.resetModeScopedState(),
			() => this.providerOperations.invalidateRefresh()
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

	get openRouterReady() {
		return (
			(this.settings?.openrouter_key_configured ?? false) &&
			this.openRouterCredentialState === 'verified'
		);
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
		toast.dismiss();
		this.providerOperations.invalidateRefresh();
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

	notify(title: string, detail: string, variant: NotificationVariant = 'success') {
		const options = { description: detail, duration: 4200, important: variant === 'error' };
		if (variant === 'error') toast.error(title, options);
		else if (variant === 'progress') toast.loading(title, options);
		else toast.success(title, options);
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

	applyInferencePoolDraft(pool: InferencePoolConfig) {
		return this.poolOperations.applyDraft(pool);
	}

	preloadLocalModel() {
		return this.runtimeOperations.preloadLocalModel();
	}

	selectModel(modelId: string) {
		return this.runtimeOperations.selectModel(modelId);
	}

	deleteModel(modelId: string) {
		return this.runtimeOperations.deleteModel(modelId);
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
