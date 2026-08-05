import type { DesktopAppState } from '$lib/app-state.svelte';
import { errorMessage } from '$lib/display';
import type { SettingsFormState } from './form-state.svelte';
import type { OpenRouterSettingsState } from './openrouter-state.svelte';
import type { RemoteSettingsState } from './remote-state.svelte';
import { settingsInputFromDraft, type SettingsDraft } from './settings-input';

export type SaveState = 'saved' | 'pending' | 'saving' | 'failed';

export class SettingsPersistenceState {
	saveState = $state<SaveState>('saved');
	error = $state('');
	#app: DesktopAppState;
	#form: SettingsFormState;
	#remote: RemoteSettingsState;
	#openRouter: OpenRouterSettingsState;
	#debounceTimer: ReturnType<typeof setTimeout> | null = null;
	#retryTimer: ReturnType<typeof setTimeout> | null = null;
	#retryCount = 0;
	#hasPendingChanges = false;
	#destroyed = false;

	constructor(
		app: DesktopAppState,
		form: SettingsFormState,
		remote: RemoteSettingsState,
		openRouter: OpenRouterSettingsState
	) {
		this.#app = app;
		this.#form = form;
		this.#remote = remote;
		this.#openRouter = openRouter;
	}

	setError(message: string) {
		this.error = message;
	}

	schedule() {
		if (this.#destroyed) return;
		this.#hasPendingChanges = true;
		this.#clearDebounce();
		this.#clearRetry(true);

		if (this.#saveBlocked()) {
			this.saveState = 'pending';
			return;
		}
		if (this.#form.pasteDelayError) {
			this.error = this.#form.pasteDelayError;
			this.saveState = 'failed';
			return;
		}
		if (this.#app.captureLocked) {
			this.saveState = 'pending';
			return;
		}

		this.saveState = 'pending';
		this.#debounceTimer = setTimeout(() => {
			this.#debounceTimer = null;
			void this.save();
		}, 650);
	}

	handleCaptureLock(locked: boolean) {
		if (!this.#hasPendingChanges) return;
		if (locked) {
			this.#clearDebounce();
			this.saveState = 'pending';
			return;
		}
		if (this.saveState === 'pending') this.schedule();
	}

	async save(flushing = false) {
		if ((this.#destroyed && !flushing) || this.saveState === 'saving') return;
		const settings = this.#app.settings;
		if (!settings || !this.#hasPendingChanges) return;
		this.#clearDebounce();

		if (this.#saveBlocked()) {
			this.saveState = 'pending';
			return;
		}
		if (this.#form.pasteDelayError) {
			this.error = this.#form.pasteDelayError;
			this.saveState = 'failed';
			return;
		}
		if (this.#app.captureLocked) {
			this.saveState = 'pending';
			return;
		}

		this.saveState = 'saving';
		this.error = '';
		const input = settingsInputFromDraft(settings, this.#draft());
		try {
			await this.#app.saveSettings(input);
			if (input.mode === 'remote' && this.#app.overview) {
				await this.#app.updateRuntime({
					...this.#app.overview.runtime,
					english_only: this.#form.englishOnly
				});
			}
			this.#remote.commitSecret();
			this.#openRouter.commitSecret();
			this.#retryCount = 0;
			this.#hasPendingChanges = false;
			this.saveState = 'saved';
		} catch (error) {
			this.error = errorMessage(error);
			if (!flushing && !this.#destroyed && this.#retryCount < 2) {
				this.#retryCount += 1;
				this.saveState = 'pending';
				this.#retryTimer = setTimeout(() => {
					this.#retryTimer = null;
					void this.save();
				}, 900 * this.#retryCount);
			} else {
				this.saveState = 'failed';
			}
		}
	}

	destroy() {
		const shouldFlush = this.saveState === 'pending';
		this.#clearDebounce();
		this.#clearRetry(false);
		if (shouldFlush) void this.save(true);
		this.#destroyed = true;
	}

	#saveBlocked() {
		return this.#remote.saveBlocked || this.#openRouter.saveBlocked;
	}

	#draft(): SettingsDraft {
		return {
			remoteEndpoint: this.#remote.endpoint,
			remoteToken: this.#remote.token,
			remoteTokenDirty: this.#remote.tokenDirty,
			clearRemoteToken: this.#remote.clearToken,
			openRouterModel: this.#openRouter.model,
			openRouterKey: this.#openRouter.key,
			openRouterKeyDirty: this.#openRouter.keyDirty,
			clearOpenRouterKey: this.#openRouter.clearKey,
			microphone: this.#form.microphone,
			sampleRate: this.#form.sampleRate,
			transcriptionMode: this.#form.transcriptionMode,
			streamingPcmFormat: this.#form.streamingPcmFormat,
			englishOnly: this.#form.englishOnly,
			copyFinal: this.#form.copyFinal,
			pasteMethod: this.#form.pasteMethod,
			pasteDelay: this.#form.pasteDelay,
			shortcut: this.#form.shortcut,
			shortcutMode: this.#form.shortcutMode,
			closeToTray: this.#form.closeToTray
		};
	}

	#clearDebounce() {
		if (!this.#debounceTimer) return;
		clearTimeout(this.#debounceTimer);
		this.#debounceTimer = null;
	}

	#clearRetry(resetCount: boolean) {
		if (this.#retryTimer) clearTimeout(this.#retryTimer);
		this.#retryTimer = null;
		if (resetCount) this.#retryCount = 0;
	}
}
