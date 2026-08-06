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
	actionError = $state('');
	#app: DesktopAppState;
	#form: SettingsFormState;
	#remote: RemoteSettingsState;
	#openRouter: OpenRouterSettingsState;
	#debounceTimer: ReturnType<typeof setTimeout> | null = null;
	#retryTimer: ReturnType<typeof setTimeout> | null = null;
	#retryCount = 0;
	#revision = 0;
	#savedRevision = 0;
	#destroyed = false;
	#activeSave: Promise<void> | null = null;

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

	setActionError(message: string) {
		this.actionError = message;
	}

	get blockedReason() {
		if (this.#remote.saveBlocked) {
			return 'Test the Shadoword API endpoint and token before saving these changes.';
		}
		if (this.#openRouter.saveBlocked) {
			return 'Enter a complete OpenRouter key and wait for verification before saving it.';
		}
		return '';
	}

	schedule(immediate = false) {
		if (this.#destroyed) return;
		this.#revision += 1;
		this.#clearDebounce();
		this.#clearRetry(true);

		if (this.blockedReason) {
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
		if (immediate) {
			void this.save();
			return;
		}
		this.#debounceTimer = setTimeout(() => {
			this.#debounceTimer = null;
			void this.save();
		}, 250);
	}

	handleCaptureLock(locked: boolean) {
		if (!this.#hasPendingChanges()) return;
		if (locked) {
			this.#clearDebounce();
			this.saveState = 'pending';
			return;
		}
		if (this.saveState === 'pending') this.schedule();
	}

	save(flushing = false) {
		if (this.#activeSave) return this.#activeSave;
		const save = this.#drainSaves(flushing);
		this.#activeSave = save.finally(() => {
			this.#activeSave = null;
		});
		return this.#activeSave;
	}

	async flush() {
		this.#clearDebounce();
		await this.save(true);
		if (this.#hasPendingChanges() && this.saveState === 'pending') {
			this.#clearRetry(false);
			await this.save(true);
		}
		if (this.saveState !== 'saved') {
			throw new Error(this.blockedReason || this.error || 'Pending settings could not be saved.');
		}
	}

	async #drainSaves(flushing: boolean) {
		while (this.#hasPendingChanges()) {
			const revision = this.#revision;
			if (!(await this.#performSave(flushing, revision))) return;
		}
	}

	async #performSave(flushing: boolean, revision: number) {
		if (this.#destroyed && !flushing) return false;
		const settings = this.#app.settings;
		if (!settings || !this.#hasPendingChanges()) return false;
		this.#clearDebounce();

		if (this.blockedReason) {
			this.saveState = 'pending';
			return false;
		}
		if (this.#form.pasteDelayError) {
			this.error = this.#form.pasteDelayError;
			this.saveState = 'failed';
			return false;
		}
		if (this.#app.captureLocked) {
			this.saveState = 'pending';
			return false;
		}

		this.saveState = 'saving';
		this.error = '';
		const draft = this.#draft();
		const input = settingsInputFromDraft(settings, draft);
		const remoteEnglishChanged =
			input.mode === 'remote' && this.#app.overview?.runtime.english_only !== draft.englishOnly;
		try {
			await this.#app.saveSettings(input);
			if (remoteEnglishChanged && this.#app.overview) {
				await this.#app.updateRuntime({
					...this.#app.overview.runtime,
					english_only: draft.englishOnly
				});
			}
			this.#savedRevision = revision;
			this.#retryCount = 0;
			if (revision !== this.#revision) {
				this.saveState = 'pending';
				return true;
			}
			this.#remote.commitSecret();
			this.#openRouter.commitSecret();
			this.saveState = 'saved';
			return true;
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
			return false;
		}
	}

	destroy() {
		const shouldFlush = this.saveState === 'pending' && this.#hasPendingChanges();
		this.#clearDebounce();
		this.#clearRetry(false);
		if (shouldFlush) void this.save(true);
		this.#destroyed = true;
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
			transcriptionMode: this.#form.transcriptionMode,
			streamingPcmFormat: this.#form.streamingPcmFormat,
			englishOnly: this.#form.englishOnly,
			copyFinal: this.#form.copyFinal,
			pasteMethod: this.#form.pasteMethod,
			pasteDelay: this.#form.pasteDelay,
			outputPrefix: this.#form.outputPrefix,
			outputSuffix: this.#form.outputSuffix,
			shortcut: this.#form.shortcut,
			shortcutMode: this.#form.shortcutMode,
			closeToTray: this.#form.closeToTray,
			showWindowTitleBar: this.#form.showWindowTitleBar
		};
	}

	#hasPendingChanges() {
		return this.#savedRevision < this.#revision;
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
