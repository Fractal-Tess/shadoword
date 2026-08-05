import { commands, type ConnectionInput, type ServiceMode } from '$lib/bindings';
import { errorMessage } from '$lib/display';
import { commandNamesForMode } from '$lib/native-routing';
import type { DesktopStateContext } from './contracts';
import { demoOpenRouterModels } from './demo-operations';
import { demoOverview } from './demo-fixtures';
import { failOverview, setAppError } from './errors';

export class ProviderOperations {
	#refreshEpoch = 0;

	constructor(private app: DesktopStateContext) {}

	async refreshOverview() {
		const epoch = ++this.#refreshEpoch;
		if (this.app.demo) {
			this.app.overview ??= { ...demoOverview };
			this.app.activity = 'ready';
			this.app.statusMessage = 'Simulated runtime ready';
			return;
		}
		const mode = this.app.settings?.mode;
		if (!mode) return;
		this.app.activity = 'busy';
		this.app.error = null;
		this.app.errorRetry = null;
		try {
			if (mode === 'open_router') {
				this.app.overview = null;
				await this.refreshOpenRouterModels();
				if (!this.#isCurrentRefresh(epoch, mode)) return;
				this.app.activity = this.app.settings?.openrouter_key_configured ? 'ready' : 'offline';
				this.app.statusMessage = this.app.settings?.openrouter_key_configured
					? 'OpenRouter transcription ready'
					: 'OpenRouter API key required';
				return;
			}
			const route = commandNamesForMode(mode);
			const overview = await commands[route.refreshOverview]();
			if (!this.#isCurrentRefresh(epoch, mode)) return;
			this.app.overview = overview;
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
		this.app.openRouterKeyMessage = null;
		this.app.openRouterKeyReport = null;
		if (this.app.demo) {
			this.app.openRouterKeyMessage = 'Simulated OpenRouter key verified';
			return;
		}
		try {
			const report = await commands.testOpenrouterKey({ key, use_saved_key: useSavedKey });
			this.app.openRouterKeyReport = report;
			this.app.openRouterKeyMessage =
				report.limit_remaining == null
					? 'OpenRouter key verified'
					: `OpenRouter key verified · ${report.limit_remaining.toFixed(4)} credits remaining`;
		} catch (error) {
			this.app.openRouterKeyMessage = `OpenRouter key test failed: ${errorMessage(error)}`;
			throw error;
		}
	}

	#isCurrentRefresh(epoch: number, mode: ServiceMode) {
		return epoch === this.#refreshEpoch && this.app.settings?.mode === mode;
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
