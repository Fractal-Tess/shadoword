import type { DesktopAppState } from '$lib/app-state.svelte';
import { commands, type DesktopSettings, type OpenRouterKeyReport } from '$lib/bindings';
import { errorMessage } from '$lib/display';
import type { VerificationState } from './remote-state.svelte';

const OPENROUTER_KEY_PATTERN = /^sk-or-v1-[a-f\d]{64}$/i;
type ChangeHandler = () => void;

export class OpenRouterSettingsState {
	model = $state('');
	key = $state('');
	keyDirty = $state(false);
	clearKey = $state(false);
	showKey = $state(false);
	storedKey = $state('');
	replacingKey = $state(false);
	credentialMessage = $state('');
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

	get hasStoredKey() {
		return this.#app.settings?.openrouter_key_configured ?? false;
	}

	get saveBlocked() {
		return this.keyDirty && !this.clearKey && this.connectionState !== 'success';
	}

	get keyValue() {
		if (this.keyDirty || this.replacingKey) return this.key;
		if (this.showKey && this.storedKey) return this.storedKey;
		return this.hasStoredKey && !this.clearKey ? '••••••••••••••••' : '';
	}

	get storedKeyReadonly() {
		return this.hasStoredKey && !this.replacingKey && !this.clearKey;
	}

	setModel(value: string) {
		if (this.model === value) return;
		this.model = value;
		this.#onChange();
	}

	setKey(value: string) {
		this.#cancelValidation();
		this.credentialMessage = '';
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

	beginKeyReplacement() {
		this.#cancelValidation();
		this.storedKey = '';
		this.showKey = false;
		this.key = '';
		this.keyDirty = false;
		this.replacingKey = true;
		this.clearKey = false;
		this.connectionState = 'idle';
		this.credentialMessage = 'Enter a replacement key. It will save after verification.';
	}

	async toggleKeyVisibility() {
		if (this.keyDirty || this.replacingKey) {
			this.showKey = !this.showKey;
			return;
		}
		if (this.showKey) {
			this.showKey = false;
			this.storedKey = '';
			return;
		}
		try {
			this.storedKey = await commands.revealDesktopSecret('open_router_key');
			this.showKey = true;
			this.credentialMessage = 'Saved key revealed.';
		} catch {
			this.credentialMessage = 'Could not reveal the saved key.';
		}
	}

	async copyKey() {
		try {
			await commands.copyDesktopSecret('open_router_key');
			this.credentialMessage = 'Saved key copied.';
		} catch {
			this.credentialMessage = 'Could not copy the saved key.';
		}
	}

	hideKey() {
		this.showKey = false;
		this.storedKey = '';
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
		this.storedKey = '';
		this.keyDirty = false;
		this.replacingKey = false;
		this.clearKey = false;
		this.showKey = false;
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
		} catch (error) {
			if (!this.#isCurrentValidation(key, version)) return;
			this.keyReport = null;
			this.credentialMessage = errorMessage(error);
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
