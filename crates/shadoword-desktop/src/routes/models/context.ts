import type { DesktopAppState } from '$lib/app-state.svelte';
import type {
	DownloadJobStatus,
	ModelInfoDto,
	RuntimeConfigDto,
	ServiceMode,
	WhisperGpuDeviceInfo
} from '$lib/bindings';
import { createContext } from 'svelte';

type RuntimeChanges = Partial<Pick<RuntimeConfigDto, 'preload_on_startup'>>;

export type ModelsContext = {
	readonly app: DesktopAppState;
	readonly mode: ServiceMode;
	readonly runtime: RuntimeConfigDto | null;
	readonly models: ModelInfoDto[];
	readonly selectedId: string | null;
	readonly controlsLocked: boolean;
	readonly preload: boolean;
	readonly gpuDevices: WhisperGpuDeviceInfo[];
	readonly customPath: string;
	readonly failedDownload: DownloadJobStatus | null;
	readonly updateRuntime: (changes: RuntimeChanges) => Promise<void>;
	readonly setCustomPath: (value: string) => void;
	readonly useCustomPath: () => Promise<void>;
};

export const [getModelsContext, setModelsContext] = createContext<ModelsContext>();
