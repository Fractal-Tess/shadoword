import type { DesktopAppState } from '$lib/app-state.svelte';
import { commands, type DesktopSettings } from '$lib/bindings';
import { errorMessage } from '$lib/display';

export type VerificationState = 'idle' | 'testing' | 'success' | 'failed';
type ChangeHandler = () => void;
type ErrorHandler = (message: string) => void;

export class RemoteSettingsState {
	endpoint = $state('');
	token = $state('');
	tokenDirty = $state(false);
	clearToken = $state(false);
	showToken = $state(false);
	storedToken = $state('');
	credentialMessage = $state('');
	connectionState = $state<VerificationState>('idle');
	#app: DesktopAppState;
	#savedEndpoint: string;
	#storedTokenConfigured: boolean;
	#onChange: ChangeHandler = () => {};
	#onError: ErrorHandler = () => {};

	constructor(app: DesktopAppState, settings: DesktopSettings) {
		this.#app = app;
		this.endpoint = settings.remote_endpoint;
		this.#savedEndpoint = settings.remote_endpoint;
		this.#storedTokenConfigured = settings.remote_token_configured;
	}

	setHandlers(onChange: ChangeHandler, onError: ErrorHandler) {
		this.#onChange = onChange;
		this.#onError = onError;
	}

	get hasStoredToken() {
		return this.#app.settings?.remote_token_configured ?? this.#storedTokenConfigured;
	}

	get tokenValue() {
		if (this.tokenDirty) return this.token;
		if (this.showToken && this.storedToken) return this.storedToken;
		return this.hasStoredToken && !this.clearToken ? '••••••••••••••••' : '';
	}

	get storedTokenReadonly() {
		return this.hasStoredToken && !this.tokenDirty && !this.clearToken;
	}

	get verificationRequired() {
		return (
			(this.tokenDirty && !this.clearToken) ||
			(this.endpoint.trim() !== this.#savedEndpoint.trim() &&
				this.#storedTokenConfigured &&
				!this.clearToken)
		);
	}

	get saveBlocked() {
		return this.verificationRequired && this.connectionState !== 'success';
	}

	setEndpoint(value: string) {
		if (this.endpoint === value) return;
		this.endpoint = value;
		this.connectionState = 'idle';
		this.#onChange();
	}

	setToken(value: string) {
		this.credentialMessage = '';
		this.token = value;
		this.tokenDirty = true;
		this.clearToken = false;
		this.connectionState = 'idle';
		this.#onChange();
	}

	beginTokenReplacement() {
		this.storedToken = '';
		this.showToken = false;
		this.token = '';
		this.tokenDirty = true;
		this.clearToken = false;
		this.connectionState = 'idle';
		this.credentialMessage = 'Enter the replacement token, then test the connection.';
	}

	async toggleTokenVisibility() {
		if (this.tokenDirty) {
			this.showToken = !this.showToken;
			return;
		}
		if (this.showToken) {
			this.showToken = false;
			this.storedToken = '';
			return;
		}
		try {
			this.storedToken = await commands.revealDesktopSecret('remote_token');
			this.showToken = true;
			this.#onError('');
			this.credentialMessage = 'Saved token revealed.';
		} catch (error) {
			this.#onError(errorMessage(error));
		}
	}

	async copyToken() {
		try {
			await commands.copyDesktopSecret('remote_token');
			this.#onError('');
			this.credentialMessage = 'Saved token copied.';
		} catch (error) {
			this.#onError(errorMessage(error));
		}
	}

	hideToken() {
		this.showToken = false;
		this.storedToken = '';
	}

	toggleClearToken() {
		this.clearToken = !this.clearToken;
		this.tokenDirty = false;
		this.token = '';
		this.connectionState = 'idle';
		this.#onChange();
	}

	async testConnection() {
		const testedEndpoint = this.endpoint;
		const testedToken = this.token;
		const testedTokenDirty = this.tokenDirty;
		const testedClearToken = this.clearToken;
		const inputIsCurrent = () =>
			this.endpoint === testedEndpoint &&
			this.token === testedToken &&
			this.tokenDirty === testedTokenDirty &&
			this.clearToken === testedClearToken;

		this.connectionState = 'testing';
		this.#onError('');
		try {
			await this.#app.testConnection({
				endpoint: testedEndpoint,
				token: testedTokenDirty && !testedClearToken ? testedToken : null,
				use_saved_token: !testedTokenDirty && !testedClearToken
			});
			if (!inputIsCurrent()) return;
			this.connectionState = 'success';
			this.#onChange();
		} catch (error) {
			if (!inputIsCurrent()) return;
			this.connectionState = 'failed';
			this.#onError(errorMessage(error));
			this.#onChange();
		}
	}

	commitSecret() {
		this.token = '';
		this.storedToken = '';
		this.tokenDirty = false;
		this.clearToken = false;
		this.showToken = false;
		this.#savedEndpoint = this.endpoint;
		this.#storedTokenConfigured = this.#app.settings?.remote_token_configured ?? false;
	}
}
