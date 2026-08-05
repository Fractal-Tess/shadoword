import type { DesktopAppState } from '$lib/app-state.svelte';
import type { DesktopSettings, OpenRouterKeyReport } from '$lib/bindings';
import type { VerificationState } from './remote-state.svelte';

const OPENROUTER_KEY_PATTERN = /^sk-or-v1-[a-f\d]{64}$/i;
type ChangeHandler = () => void;

export class OpenRouterSettingsState {
	model = $state('');
	key = $state('');
	keyDirty = $state(false);
	clearKey = $state(false);
	showKey = $state(false);
	connectionState = $state<VerificationState>('idle');
	keyReport = $state.raw<OpenRouterKeyReport | null>(null);
	#app: DesktopAppState;
	#onChange: ChangeHandler = () => {};
	#validationTimer: ReturnType<typeof setTimeout> | null = null;
	#validationVersion = 0;

	constructor(app: DesktopAppState, settings: DesktopSettings) {
		this.#app = app;
		this.model = settings.openrouter_model;
	}

	setChangeHandler(handler: ChangeHandler) {
		this.#onChange = handler;
	}

	get saveBlocked() {
		return this.keyDirty && !this.clearKey && this.connectionState !== 'success';
	}

	setModel(value: string) {
		if (this.model === value) return;
		this.model = value;
		this.#onChange();
	}

	setKey(value: string) {
		this.#cancelValidation();
		this.key = value;
		this.keyDirty = true;
		this.clearKey = false;
		this.connectionState = 'idle';
		this.keyReport = null;
		this.#app.openRouterKeyReport = null;

		const key = value.trim();
		if (key === '') {
			this.keyDirty = false;
			this.#onChange();
			return;
		}
		if (OPENROUTER_KEY_PATTERN.test(key)) {
			this.connectionState = 'testing';
			const version = this.#validationVersion;
			this.#validationTimer = setTimeout(() => void this.#validateKey(key, version), 250);
		}
		this.#onChange();
	}

	toggleKeyVisibility() {
		this.showKey = !this.showKey;
	}

	toggleClearKey() {
		this.#cancelValidation();
		this.clearKey = !this.clearKey;
		this.keyDirty = false;
		this.key = '';
		this.connectionState = 'idle';
		this.keyReport = null;
		this.#app.openRouterKeyReport = null;
		this.#onChange();
	}

	commitSecret() {
		this.key = '';
		this.keyDirty = false;
		this.clearKey = false;
	}

	destroy() {
		this.#cancelValidation();
	}

	async #validateKey(key: string, version: number) {
		this.#validationTimer = null;
		try {
			await this.#app.testOpenRouterKey(key, false);
			if (!this.#isCurrentValidation(key, version)) return;
			this.keyReport = this.#app.openRouterKeyReport;
			this.connectionState = 'success';
			this.#onChange();
		} catch {
			if (!this.#isCurrentValidation(key, version)) return;
			this.keyReport = null;
			this.connectionState = 'failed';
			this.#onChange();
		}
	}

	#isCurrentValidation(key: string, version: number) {
		return version === this.#validationVersion && this.key.trim() === key;
	}

	#cancelValidation() {
		this.#validationVersion += 1;
		if (!this.#validationTimer) return;
		clearTimeout(this.#validationTimer);
		this.#validationTimer = null;
	}
}
