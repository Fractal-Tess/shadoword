import type { DesktopAppState } from '$lib/app-state.svelte';
import type {
	DownloadJobStatus,
	ModelInfoDto,
	RuntimeConfigDto,
	ServiceMode,
	WhisperAccelerator,
	WhisperGpuDeviceInfo
} from '$lib/bindings';
import { createContext } from 'svelte';

type RuntimeChanges = Partial<
	Pick<RuntimeConfigDto, 'preload_on_startup' | 'whisper_accelerator' | 'whisper_gpu_device'>
>;

export interface ModelsContext {
	readonly app: DesktopAppState;
	readonly mode: ServiceMode;
	readonly runtime: RuntimeConfigDto | null;
	readonly models: ModelInfoDto[];
	readonly selectedId: string | null;
	readonly controlsLocked: boolean;
	readonly preload: boolean;
	readonly accelerator: WhisperAccelerator;
	readonly gpuDevice: number;
	readonly gpuDevices: WhisperGpuDeviceInfo[];
	readonly gpuDeviceOptions: Array<{ value: string; label: string; detail: string }>;
	readonly customPath: string;
	readonly failedDownload: DownloadJobStatus | null;
	readonly updateRuntime: (changes: RuntimeChanges) => Promise<void>;
	readonly setCustomPath: (value: string) => void;
	readonly useCustomPath: () => Promise<void>;
}

export const [getModelsContext, setModelsContext] = createContext<ModelsContext>();
