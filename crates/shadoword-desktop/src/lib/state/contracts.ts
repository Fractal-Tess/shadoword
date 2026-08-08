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
import type { PoolFieldErrors } from '$lib/inference-pool';
import type { HistoryRecord } from '$lib/types';
import type { TranscriptSegments } from '$lib/desktop-events';

export type ActivityState = 'booting' | 'ready' | 'busy' | 'offline';
export type CaptureState = 'idle' | 'recording' | 'finalizing' | 'error';
export type PoolValidationState = 'idle' | 'validating' | 'valid' | 'invalid';
export type PoolApplyState = 'idle' | 'applying' | 'applied' | 'failed' | 'stale';
export type OpenRouterModelsState = 'idle' | 'loading' | 'ready' | 'failed';
export type OpenRouterCredentialState = 'missing' | 'checking' | 'verified' | 'invalid';
export type NotificationVariant = 'success' | 'error' | 'progress';

export type DesktopStateContext = {
	readonly demo: boolean;
	activity: ActivityState;
	settings: DesktopSettings | null;
	inputDevices: InputDeviceInfo[];
	overview: OverviewDto | null;
	history: HistoryRecord[];
	error: string | null;
	errorRetry: 'overview' | null;
	inputDevicesError: string | null;
	connectionMessage: string | null;
	openRouterModels: OpenRouterModelInfo[];
	openRouterModelsState: OpenRouterModelsState;
	openRouterModelsError: string | null;
	openRouterKeyReport: OpenRouterKeyReport | null;
	openRouterCredentialState: OpenRouterCredentialState;
	hotkeyError: string | null;
	statusMessage: string;
	captureState: CaptureState;
	recordingSampleRate: number;
	recordingMode: ServiceMode | null;
	recordingTranscriptionMode: TranscriptionMode | null;
	transcript: string;
	lastResult: TranscriptionResult | null;
	segmentResults: TranscriptSegments;
	downloads: Record<string, DownloadJobStatus>;
	downloadWatching: Record<string, boolean>;
	poolValidationState: PoolValidationState;
	poolApplyState: PoolApplyState;
	poolFieldErrors: PoolFieldErrors;
	poolFeedback: string | null;
	readonly recording: boolean;
	readonly processing: boolean;
	readonly captureLocked: boolean;
	readonly openRouterReady: boolean;
	readonly drainingPool: boolean;
	readonly poolMutationLocked: boolean;
	readonly segmentCount: number;

	initialize(): Promise<void>;
	dispose(): void;
	clearError(): void;
	setHistory(records: HistoryRecord[]): void;
	retryError(): Promise<void>;
	notify(title: string, detail: string, variant?: NotificationVariant): void;
	refreshOverview(): Promise<void>;
	testConnection(input: ConnectionInput): Promise<void>;
	refreshOpenRouterModels(): Promise<void>;
	testOpenRouterKey(key: string | null, useSavedKey: boolean): Promise<void>;
	saveSettings(input: DesktopSettingsInput): Promise<void>;
	setMode(mode: ServiceMode): Promise<void>;
	refreshInputDevices(): Promise<void>;
	updateRuntime(runtime: RuntimeConfigDto): Promise<void>;
	clearPoolDraftFeedback(): void;
	validateInferencePoolDraft(pool: InferencePoolConfig): Promise<InferencePoolConfig>;
	applyInferencePoolDraft(pool: InferencePoolConfig): Promise<OverviewDto | null>;
	preloadLocalModel(): Promise<void>;
	selectModel(modelId: string): Promise<void>;
	deleteModel(modelId: string): Promise<void>;
	startDownload(modelId: string): Promise<void>;
	stopWatchingDownload(modelId: string): void;
	startRecording(): Promise<void>;
	stopRecording(): Promise<void>;
	cancelRecording(): Promise<void>;
	clearHistory(): void;
};
