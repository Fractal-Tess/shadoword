import type { DesktopAppState } from '$lib/app-state.svelte';
import { inferencePoolSummary } from '$lib/inference-pool';
import type { PageId } from '$lib/types';
import { createContext } from 'svelte';
import { SettingsFormState } from './form-state.svelte';
import { OpenRouterSettingsState } from './openrouter-state.svelte';
import { SettingsPersistenceState } from './persistence-state.svelte';
import { RemoteSettingsState } from './remote-state.svelte';
import { RemoteTokenSettingsState } from './token-state.svelte';

export class SettingsContextState {
	readonly app: DesktopAppState;
	readonly form: SettingsFormState;
	readonly remote: RemoteSettingsState;
	readonly remoteTokens: RemoteTokenSettingsState;
	readonly openRouter: OpenRouterSettingsState;
	readonly persistence: SettingsPersistenceState;
	#navigate: (page: PageId) => void;

	constructor(app: DesktopAppState, navigate: (page: PageId) => void) {
		const settings = app.settings;
		if (!settings) throw new Error('Desktop settings must be loaded before opening settings.');
		this.app = app;
		this.#navigate = navigate;
		this.form = new SettingsFormState(settings, app.overview?.runtime.english_only);
		this.remote = new RemoteSettingsState(app, settings);
		this.remoteTokens = new RemoteTokenSettingsState(app);
		this.openRouter = new OpenRouterSettingsState(app, settings);
		this.persistence = new SettingsPersistenceState(app, this.form, this.remote, this.openRouter);
		const schedule = (immediate = false) => this.persistence.schedule(immediate);
		this.form.setChangeHandler(schedule);
		this.remote.setHandlers(schedule, (message) => this.persistence.setActionError(message));
		this.openRouter.setChangeHandler(schedule);
	}

	get mode() {
		return this.app.settings?.mode ?? 'remote';
	}

	get locked() {
		return this.app.captureLocked;
	}

	get activeRuntime() {
		return this.app.overview?.runtime ?? null;
	}

	get poolSummary() {
		return inferencePoolSummary(this.app.overview?.status.inference_pool);
	}

	get localModelName() {
		const path = this.app.overview?.runtime.model_path;
		return (
			this.app.overview?.models.find((model) => path?.endsWith(model.filename))?.name ??
			'No model selected'
		);
	}

	get microphoneOptions() {
		return [
			{ value: '', label: 'System default', detail: 'Follow the desktop audio default' },
			...this.app.inputDevices.map((device) => ({
				value: device.name,
				label: device.name,
				detail: device.is_default ? 'Default input' : 'Available input'
			}))
		];
	}

	navigate(page: PageId) {
		this.#navigate(page);
	}

	hideRevealedSecrets() {
		this.remote.hideToken();
		this.openRouter.hideKey();
	}

	destroy() {
		this.openRouter.destroy();
		this.persistence.destroy();
	}
}

export const [getSettingsContext, setSettingsContext] = createContext<SettingsContextState>();
