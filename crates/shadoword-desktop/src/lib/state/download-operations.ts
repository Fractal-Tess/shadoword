import { commands, type DownloadJobStatus, type ServiceMode } from '$lib/bindings';
import { commandNamesForMode } from '$lib/native-routing';
import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import type { DesktopStateContext } from './contracts';
import { advanceDemoDownload, demoDownload } from './demo-operations';
import { delay, sentenceCase, setAppError } from './errors';

export class DownloadOperations {
	private polling = new SvelteSet<string>();
	private downloadModes = new SvelteMap<string, ServiceMode>();

	constructor(private app: DesktopStateContext) {}

	async start(modelId: string) {
		const mode = this.app.settings?.mode;
		if (!mode || this.app.captureLocked) return;
		this.app.error = null;
		try {
			const route = commandNamesForMode(mode);
			const status = this.app.demo
				? demoDownload(modelId)
				: await commands[route.startDownload](modelId);
			this.setDownload(modelId, status, true);
			this.downloadModes.set(modelId, mode);
			this.polling.add(modelId);
			void this.poll(modelId, status.id, mode);
		} catch (error) {
			setAppError(this.app, error, `Could not start the ${mode} model download`);
		}
	}

	stopWatching(modelId: string) {
		this.polling.delete(modelId);
		this.downloadModes.delete(modelId);
		this.app.downloadWatching = { ...this.app.downloadWatching, [modelId]: false };
	}

	dispose() {
		this.polling.clear();
		this.app.downloadWatching = {};
	}

	resetModeScopedState() {
		this.polling.clear();
		this.downloadModes.clear();
		this.app.downloads = {};
		this.app.downloadWatching = {};
	}

	private async poll(modelId: string, jobId: string, mode: ServiceMode) {
		let transientFailures = 0;
		while (this.polling.has(modelId) && this.downloadModes.get(modelId) === mode) {
			await delay(this.app.demo ? 350 : 1000);
			if (!this.polling.has(modelId) || this.downloadModes.get(modelId) !== mode) return;
			try {
				const previous = this.app.downloads[modelId];
				const route = commandNamesForMode(mode);
				const status = this.app.demo
					? advanceDemoDownload(previous)
					: await commands[route.pollDownload](jobId);
				transientFailures = 0;
				this.setDownload(modelId, status, true);
				if (status.state === 'succeeded' || status.state === 'failed') {
					this.stopWatching(modelId);
					if (this.app.demo && status.state === 'succeeded' && this.app.overview) {
						this.app.overview = {
							...this.app.overview,
							models: this.app.overview.models.map((model) =>
								model.id === modelId ? { ...model, installed: true } : model
							)
						};
					}
					if (this.app.settings?.mode === mode) await this.app.refreshOverview();
					return;
				}
			} catch (error) {
				if (transientFailures < 3) {
					transientFailures += 1;
					continue;
				}
				this.stopWatching(modelId);
				setAppError(
					this.app,
					error,
					`${sentenceCase(mode)} download polling stopped after three retries`
				);
			}
		}
	}

	private setDownload(modelId: string, status: DownloadJobStatus, watching: boolean) {
		this.app.downloads = { ...this.app.downloads, [modelId]: status };
		this.app.downloadWatching = { ...this.app.downloadWatching, [modelId]: watching };
	}
}
