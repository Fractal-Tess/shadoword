import type { DesktopAppState } from '$lib/app-state.svelte';
import { commands, type DesktopSettings, type OpenRouterKeyReport } from '$lib/bindings';
import { errorMessage } from '$lib/display';
import type { VerificationState } from './remote-state.svelte';

type ChangeHandler = () => void;

export class OpenRouterSettingsState {
	model = $state('');
	key = $state('');
	keyDirty = $state(false);
	clearKey = $state(false);
	showKey = $state(false);
	storedKey = $state('');
	credentialMessage = $state('');
	connectionState = $state<VerificationState>('idle');
	keyReport = $state.raw<OpenRouterKeyReport | null>(null);
	#app: DesktopAppState;
	#onChange: ChangeHandler = () => {};
	#validationVersion = 0;

	constructor(app: DesktopAppState, settings: DesktopSettings) {
		this.#app = app;
		this.model = settings.openrouter_model;
	}

	setChangeHandler(handler: ChangeHandler) {
		this.#onChange = handler;
	}

	get hasStoredKey() {
		return this.#app.settings?.openrouter_key_configured ?? false;
	}

	get saveBlocked() {
		return false;
	}

	get keyValue() {
		return this.keyDirty || this.clearKey ? this.key : this.storedKey;
	}

	get canTestKey() {
		return !this.clearKey && (this.key.trim() !== '' || this.hasStoredKey);
	}

	toggleKeyVisibility() {
		this.showKey = !this.showKey;
	}

	async loadSavedKey() {
		if (!this.hasStoredKey || this.keyDirty || this.storedKey) return;
		try {
			this.storedKey = await commands.revealDesktopSecret('open_router_key');
		} catch {
			this.credentialMessage = 'Could not load the saved key.';
		}
	}

	setModel(value: string) {
		if (this.model === value) return;
		this.model = value;
		this.#onChange();
	}

	setKey(value: string) {
		this.#validationVersion += 1;
		this.credentialMessage = '';
		this.key = value;
		this.connectionState = 'idle';
		this.keyReport = null;
		this.#app.openRouterKeyReport = null;

		if (value.trim() === '') {
			this.clearKey = this.hasStoredKey;
			this.keyDirty = this.clearKey;
			this.#onChange();
			return;
		}

		this.keyDirty = true;
		this.clearKey = false;
		this.#onChange();
	}

	async testKey() {
		const key = this.key.trim();
		const useSavedKey = key === '' && this.hasStoredKey;
		if (!key && !useSavedKey) {
			this.credentialMessage = 'Enter an OpenRouter API key before testing.';
			return;
		}

		this.#validationVersion += 1;
		const version = this.#validationVersion;
		this.credentialMessage = '';
		this.connectionState = 'testing';
		this.keyReport = null;
		this.#app.openRouterKeyReport = null;
		try {
			await this.#app.testOpenRouterKey(useSavedKey ? null : key, useSavedKey);
			if (!this.#isCurrentValidation(useSavedKey ? null : key, version)) return;
			this.keyReport = this.#app.openRouterKeyReport;
			this.connectionState = 'success';
			this.#onChange();
		} catch (error) {
			if (!this.#isCurrentValidation(useSavedKey ? null : key, version)) return;
			this.credentialMessage = errorMessage(error);
			this.connectionState = 'failed';
			this.#onChange();
		}
	}

	commitSecret() {
		if (this.clearKey) this.storedKey = '';
		else if (this.keyDirty) this.storedKey = this.key.trim();
		this.key = '';
		this.keyDirty = false;
		this.clearKey = false;
	}

	hideKey() {
		this.showKey = false;
	}

	destroy() {
		this.#validationVersion += 1;
	}

	#isCurrentValidation(key: string | null, version: number) {
		return (
			version === this.#validationVersion &&
			(key === null
				? !this.keyDirty && !this.clearKey
				: this.key.trim() === key || (!this.keyDirty && this.storedKey === key))
		);
	}
}
