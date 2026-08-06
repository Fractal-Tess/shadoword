import { commands, type ConnectionInput, type OverviewDto, type ServiceMode } from '$lib/bindings';
import { errorMessage } from '$lib/display';
import { commandNamesForMode } from '$lib/native-routing';
import type { DesktopStateContext } from './contracts';
import { demoOpenRouterModels } from './demo-operations';
import { demoOverview } from './demo-fixtures';
import { failOverview, setAppError } from './errors';

export class ProviderOperations {
	#refreshEpoch = 0;
	#drainRefreshTimer: ReturnType<typeof globalThis.setTimeout> | null = null;

	constructor(private app: DesktopStateContext) {}

	invalidateRefresh() {
		this.#refreshEpoch += 1;
		if (this.#drainRefreshTimer !== null) {
			globalThis.clearTimeout(this.#drainRefreshTimer);
			this.#drainRefreshTimer = null;
		}
	}

	async refreshOverview() {
		const epoch = ++this.#refreshEpoch;
		const mode = this.app.settings?.mode;
		if (!mode) return;

		this.app.activity = 'busy';
		this.app.error = null;
		this.app.errorRetry = null;

		if (mode === 'open_router') {
			this.app.overview = null;
			await this.refreshOpenRouterModels();
			if (!this.#isCurrentRefresh(epoch, mode)) return;
			if (!this.app.settings?.openrouter_key_configured) {
				this.app.openRouterCredentialState = 'missing';
				this.app.activity = 'offline';
				this.app.statusMessage = 'OpenRouter API key required';
				return;
			}

			try {
				await this.testOpenRouterKey(null, true);
				if (!this.#isCurrentRefresh(epoch, mode)) return;
				this.app.activity = 'ready';
				this.app.statusMessage = 'OpenRouter key verified · transcription ready';
			} catch (error) {
				if (!this.#isCurrentRefresh(epoch, mode)) return;
				this.app.activity = 'offline';
				this.app.statusMessage = `OpenRouter key rejected · ${errorMessage(error)}`;
			}
			return;
		}

		if (this.app.demo) {
			this.app.overview ??= { ...demoOverview };
			this.app.activity = 'ready';
			this.app.statusMessage = 'Simulated runtime ready';
			return;
		}

		try {
			const route = commandNamesForMode(mode);
			const overview = await commands[route.refreshOverview]();
			if (!this.#isCurrentRefresh(epoch, mode)) return;
			this.app.overview = overview;
			this.#scheduleDrainRefresh(epoch, mode, overview);
			this.app.activity = 'ready';
			this.app.statusMessage = `${mode === 'local' ? 'Local' : 'Shadoword API'} runtime connected`;
		} catch (error) {
			if (this.#isCurrentRefresh(epoch, mode)) {
				failOverview(this.app, error, `Could not refresh the ${mode} runtime`);
			}
		}
	}

	async testConnection(input: ConnectionInput) {
		this.app.connectionMessage = null;
		this.app.error = null;
		this.app.activity = 'busy';
		if (this.app.demo) {
			this.app.overview = demoOverview;
			this.app.connectionMessage = 'Simulated connection verified · health, status, and runtime';
			this.app.activity = 'ready';
			return;
		}
		try {
			const report = await commands.testRemoteConnection(input);
			this.app.overview = report.overview;
			this.app.connectionMessage = `Connected · health ${report.health_ok ? 'ok' : 'failed'} · ${report.status_model_loaded ? 'model ready' : 'model unloaded'}`;
			this.app.activity = 'ready';
		} catch (error) {
			this.app.activity = this.app.overview ? 'ready' : 'offline';
			setAppError(this.app, error, 'Connection test failed');
			throw error;
		}
	}

	async refreshOpenRouterModels() {
		this.app.openRouterModelsState = 'loading';
		this.app.openRouterModelsError = null;
		try {
			this.app.openRouterModels = this.app.demo
				? demoOpenRouterModels()
				: await commands.listOpenrouterModels();
			this.app.openRouterModelsState = 'ready';
		} catch (error) {
			this.app.openRouterModelsState = 'failed';
			this.app.openRouterModelsError = errorMessage(error);
		}
	}

	async testOpenRouterKey(key: string | null, useSavedKey: boolean) {
		this.app.openRouterKeyReport = null;
		if (useSavedKey) this.app.openRouterCredentialState = 'checking';
		if (this.app.demo) {
			if (useSavedKey) this.app.openRouterCredentialState = 'verified';
			return;
		}
		try {
			this.app.openRouterKeyReport = await commands.testOpenrouterKey({
				key,
				use_saved_key: useSavedKey
			});
			if (useSavedKey) this.app.openRouterCredentialState = 'verified';
		} catch (error) {
			if (useSavedKey) this.app.openRouterCredentialState = 'invalid';
			throw error;
		}
	}

	#isCurrentRefresh(epoch: number, mode: ServiceMode) {
		return epoch === this.#refreshEpoch && this.app.settings?.mode === mode;
	}

	#scheduleDrainRefresh(epoch: number, mode: ServiceMode, overview: OverviewDto) {
		if (this.#drainRefreshTimer !== null) {
			globalThis.clearTimeout(this.#drainRefreshTimer);
			this.#drainRefreshTimer = null;
		}
		if (
			mode === 'open_router' ||
			(overview.status.inference_pool?.draining_generations?.length ?? 0) === 0
		)
			return;
		this.#drainRefreshTimer = globalThis.setTimeout(
			() => void this.#refreshDrainingPool(epoch, mode),
			1000
		);
	}

	async #refreshDrainingPool(epoch: number, mode: ServiceMode) {
		this.#drainRefreshTimer = null;
		if (mode === 'open_router' || !this.#isCurrentRefresh(epoch, mode)) return;
		try {
			const route = commandNamesForMode(mode);
			const overview = await commands[route.refreshOverview]();
			if (!this.#isCurrentRefresh(epoch, mode)) return;
			this.app.overview = overview;
			this.#scheduleDrainRefresh(epoch, mode, overview);
		} catch {
			if (this.#isCurrentRefresh(epoch, mode) && this.app.overview) {
				this.#scheduleDrainRefresh(epoch, mode, this.app.overview);
			}
		}
	}

	async refreshInputDevices() {
		this.app.inputDevicesError = null;
		if (this.app.demo) return;
		try {
			this.app.inputDevices = await commands.listInputDevices();
		} catch (error) {
			this.app.inputDevicesError = errorMessage(error);
		}
	}
}
