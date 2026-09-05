import {
	commands,
	type DesktopSettings,
	type DesktopSettingsInput,
	type RuntimeConfigDto,
	type ServiceMode
} from '$lib/bindings';
import { commandNamesForMode } from '$lib/native-routing';
import type { DesktopStateContext } from './contracts';
import {
	demoOverviewAfterRuntime,
	demoOverviewForSettings,
	withDemoModelLoaded
} from './demo-operations';
import { demoOverview } from './demo-fixtures';
import { setAppError } from './errors';
import { settingsInputForMode } from '$lib/desktop-settings';

export class RuntimeOperations {
	private settingsQueue = Promise.resolve();

	constructor(
		private app: DesktopStateContext,
		private resetDownloads: () => void,
		private invalidateOverviewRefresh: () => void
	) {}

	saveSettings(input: DesktopSettingsInput) {
		return this.enqueueSettings(() =>
			this.persistSettings({ ...input, mode: this.app.settings?.mode ?? input.mode })
		);
	}

	setMode(mode: ServiceMode) {
		return this.enqueueSettings(async () => {
			const settings = this.app.settings;
			if (!settings || settings.mode === mode || this.app.captureLocked) return;
			await this.persistSettings(
				settingsInputForMode(settings, { action: 'keep' }, mode, { action: 'keep' })
			);
		});
	}

	private async persistSettings(input: DesktopSettingsInput) {
		if (this.app.captureLocked)
			throw new Error('Stop the active recording before saving settings.');
		this.app.activity = 'busy';
		this.app.error = null;
		const previousSettings = this.app.settings;
		const previousMode = previousSettings?.mode;
		const refreshRequired = this.settingsRequireOverviewRefresh(previousSettings, input);
		if (this.app.demo) {
			const { remote_token: remoteToken, openrouter_key: openRouterKey, ...settings } = input;
			const savedSettings: DesktopSettings = {
				...settings,
				remote_token_configured:
					remoteToken.action === 'set' ||
					(remoteToken.action === 'keep' && (this.app.settings?.remote_token_configured ?? false)),
				openrouter_key_configured:
					openRouterKey.action === 'set' ||
					(openRouterKey.action === 'keep' &&
						(this.app.settings?.openrouter_key_configured ?? false))
			};
			this.app.settings = savedSettings;
			this.app.overview = demoOverviewForSettings(savedSettings, this.app.overview ?? demoOverview);
			if (previousMode !== savedSettings.mode) this.resetModeScopedState();
			if (refreshRequired) await this.app.refreshOverview();
			else this.app.activity = 'ready';
			this.app.statusMessage = 'Simulated settings saved';
			return;
		}
		try {
			this.app.settings = await commands.saveDesktopSettings(input);
			this.app.hotkeyError = null;
			if (previousMode !== this.app.settings.mode) this.resetModeScopedState();
			if (refreshRequired) await this.app.refreshOverview();
			else this.app.activity = 'ready';
		} catch (error) {
			this.app.activity = this.app.overview ? 'ready' : 'offline';
			setAppError(this.app, error, 'Could not save desktop settings');
			throw error;
		}
	}

	async updateRuntime(runtime: RuntimeConfigDto) {
		const mode = this.app.settings?.mode;
		if (!mode) throw new Error('Select a local or remote runtime before applying changes.');
		if (this.app.captureLocked)
			throw new Error('Finish the active recording before applying runtime changes.');
		this.app.activity = 'busy';
		this.app.error = null;
		try {
			if (this.app.demo) {
				this.app.overview = demoOverviewAfterRuntime(this.app.overview ?? demoOverview, runtime);
			} else {
				const route = commandNamesForMode(mode);
				this.app.overview = await commands[route.updateRuntime](runtime);
			}
			if (this.app.overview) this.syncLocalSettingsFromRuntime(mode, this.app.overview.runtime);
			if (
				!this.app.demo &&
				(this.app.overview?.status.inference_pool?.draining_generations?.length ?? 0) > 0
			) {
				await this.app.refreshOverview();
			}
			this.app.activity = 'ready';
			this.app.statusMessage = `${mode === 'local' ? 'Local' : 'Shadoword API'} runtime updated`;
		} catch (error) {
			this.app.activity = this.app.overview ? 'ready' : 'offline';
			setAppError(this.app, error, `Could not update the ${mode} runtime`);
			throw error;
		}
	}

	async preloadLocalModel() {
		if (this.app.settings?.mode !== 'local' || this.app.captureLocked) return;
		this.app.activity = 'busy';
		this.app.error = null;
		this.app.statusMessage = 'Loading the selected local model…';
		try {
			this.app.overview = this.app.demo
				? withDemoModelLoaded(this.app.overview ?? demoOverview)
				: await commands.preloadLocalModel();
			this.app.activity = 'ready';
			this.app.statusMessage = 'Local model loaded';
		} catch (error) {
			this.app.activity = this.app.overview ? 'ready' : 'offline';
			setAppError(this.app, error, 'Could not load the selected local model');
		}
	}

	async selectModel(modelId: string) {
		const mode = this.app.settings?.mode;
		if (!mode || this.app.captureLocked) return;
		const modelName =
			this.app.overview?.models.find((model) => model.id === modelId)?.name ?? modelId;
		this.invalidateOverviewRefresh();
		this.app.activity = 'busy';
		this.app.error = null;
		try {
			if (this.app.demo) {
				const overview = this.app.overview ?? demoOverview;
				const model = overview.models.find((item) => item.id === modelId);
				if (model) {
					this.app.overview = {
						...overview,
						runtime: { ...overview.runtime, model_path: `/models/${model.filename}` },
						status: { ...overview.status, model_path: `/models/${model.filename}` }
					};
				}
			} else {
				const route = commandNamesForMode(mode);
				this.app.overview = await commands[route.selectModel](modelId);
				try {
					this.app.overview = await commands[route.refreshOverview]();
				} catch {
					// The select response is already authoritative; reconciliation is best-effort.
				}
			}
			if (this.app.overview) this.syncLocalSettingsFromRuntime(mode, this.app.overview.runtime);
			if (
				!this.app.demo &&
				(this.app.overview?.status.inference_pool?.draining_generations?.length ?? 0) > 0
			) {
				await this.app.refreshOverview();
			}
			this.app.activity = 'ready';
			this.app.statusMessage = `${mode === 'local' ? 'Local' : 'Shadoword API'} model selected`;
			this.app.notify(
				`${modelName} reloaded`,
				`${mode === 'local' ? 'Local Shadoword' : 'Shadoword API'} is now using this model.`
			);
		} catch (error) {
			this.app.activity = this.app.overview ? 'ready' : 'offline';
			setAppError(this.app, error, `Could not select the ${mode} model`);
		}
	}

	async deleteModel(modelId: string) {
		const mode = this.app.settings?.mode;
		if (!mode || mode === 'open_router' || this.app.poolMutationLocked) return;
		this.app.activity = 'busy';
		this.app.error = null;
		try {
			if (this.app.demo) {
				const overview = this.app.overview ?? demoOverview;
				this.app.overview = {
					...overview,
					models: overview.models.map((model) =>
						model.id === modelId ? { ...model, installed: false } : model
					)
				};
			} else {
				const route = commandNamesForMode(mode);
				this.app.overview = await commands[route.deleteModel](modelId);
			}
			this.app.activity = 'ready';
			this.app.statusMessage = `${mode === 'local' ? 'Local' : 'Shadoword API'} model deleted`;
		} catch (error) {
			this.app.activity = this.app.overview ? 'ready' : 'offline';
			setAppError(this.app, error, `Could not delete the ${mode} model`);
		}
	}

	private settingsRequireOverviewRefresh(
		previous: DesktopSettings | null,
		input: DesktopSettingsInput
	) {
		if (!previous || previous.mode !== input.mode) return true;
		if (input.mode === 'remote') {
			return (
				previous.remote_endpoint.trim() !== input.remote_endpoint.trim() ||
				input.remote_token.action !== 'keep'
			);
		}
		if (input.mode === 'open_router') return input.openrouter_key.action !== 'keep';
		if (input.mode === 'local') {
			return (
				previous.model_path !== input.model_path ||
				previous.preload_on_startup !== input.preload_on_startup ||
				previous.whisper_accelerator !== input.whisper_accelerator ||
				previous.whisper_gpu_device !== input.whisper_gpu_device ||
				previous.sample_rate !== input.sample_rate ||
				previous.english_only !== input.english_only
			);
		}
		return false;
	}

	private enqueueSettings<T>(operation: () => Promise<T>) {
		const next = this.settingsQueue.then(operation, operation);
		this.settingsQueue = next.then(
			() => undefined,
			() => undefined
		);
		return next;
	}

	private resetModeScopedState() {
		this.resetDownloads();
		this.app.overview = null;
		this.app.openRouterKeyReport = null;
		this.app.openRouterCredentialState = this.app.settings?.openrouter_key_configured
			? 'checking'
			: 'missing';
		this.app.poolApplyState = 'idle';
		this.app.poolFieldErrors = {};
		this.app.poolFeedback = null;
	}

	private syncLocalSettingsFromRuntime(mode: ServiceMode, runtime: RuntimeConfigDto) {
		if (mode !== 'local' || !this.app.settings) return;
		this.app.settings = {
			...this.app.settings,
			model_path: runtime.model_path,
			preload_on_startup: runtime.preload_on_startup,
			whisper_accelerator: runtime.whisper_accelerator,
			whisper_gpu_device: runtime.whisper_gpu_device,
			english_only: runtime.english_only
		};
	}
}
