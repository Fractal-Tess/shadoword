import type { DesktopAppState } from '$lib/app-state.svelte';
import type { DesktopSettings } from '$lib/bindings';
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
		this.token = value;
		this.tokenDirty = value.trim() !== '';
		this.clearToken = false;
		this.connectionState = 'idle';
		this.#onChange();
	}

	toggleTokenVisibility() {
		this.showToken = !this.showToken;
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
		this.tokenDirty = false;
		this.clearToken = false;
		this.#savedEndpoint = this.endpoint;
		this.#storedTokenConfigured = this.#app.settings?.remote_token_configured ?? false;
	}
}
