import type {
	DownloadJobStatus,
	ModelInfoDto,
	RuntimeConfigDto,
	ServiceMode,
	WhisperGpuDeviceInfo
} from '$lib/bindings';
import type { DesktopAppState } from '$lib/app-state.svelte';
import { createContext } from 'svelte';

type RuntimeChanges = Partial<Pick<RuntimeConfigDto, 'preload_on_startup'>>;

export class ModelsState {
	readonly app: DesktopAppState;
	#customPath = $state<string | null>(null);

	constructor(app: DesktopAppState) {
		this.app = app;
	}

	get mode(): ServiceMode {
		return this.app.settings?.mode ?? 'remote';
	}

	get runtime(): RuntimeConfigDto | null {
		return this.app.overview?.runtime ?? null;
	}

	get models(): ModelInfoDto[] {
		return this.app.overview?.models ?? [];
	}

	get selectedId() {
		const path = this.runtime?.model_path;
		return this.models.find((model) => path?.endsWith(model.filename))?.id ?? null;
	}

	get controlsLocked() {
		return this.app.poolMutationLocked || !this.runtime;
	}

	get preload() {
		return this.runtime?.preload_on_startup ?? false;
	}

	get gpuDevices(): WhisperGpuDeviceInfo[] {
		return this.app.overview?.status.available_gpu_devices ?? [];
	}

	get customPath() {
		return this.#customPath ?? this.runtime?.model_path ?? this.app.settings?.model_path ?? '';
	}

	get failedDownload(): DownloadJobStatus | null {
		return (
			Object.values(this.app.downloads).find((download) => download.state === 'failed') ?? null
		);
	}

	setCustomPath(value: string) {
		this.#customPath = value;
	}

	async updateRuntime(changes: RuntimeChanges) {
		if (!this.runtime || this.mode === 'open_router') return;
		try {
			await this.app.updateRuntime({ ...this.runtime, ...changes });
		} catch {
			// App state exposes the native error and preserves the active runtime.
		}
	}

	useCustomPath = async () => {
		if (!this.runtime || this.mode !== 'local') return;
		try {
			await this.app.updateRuntime({ ...this.runtime, model_path: this.customPath.trim() });
			this.#customPath = null;
		} catch {
			// The global runtime alert provides retry context.
		}
	};
}

export const [getModelsContext, setModelsContext] = createContext<ModelsState>();
