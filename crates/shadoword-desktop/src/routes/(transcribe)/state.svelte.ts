import type { ServiceMode, TranscriptionMode } from '$lib/bindings';
import type { DesktopShellState } from '$lib/shell/desktop-shell.svelte';
import { createContext } from 'svelte';

export class TranscribeState {
	readonly shell: DesktopShellState;
	copied = $state(false);

	constructor(shell: DesktopShellState) {
		this.shell = shell;
	}

	get app() {
		return this.shell.app;
	}

	get mode(): ServiceMode {
		return this.app.settings?.mode ?? 'remote';
	}

	get transcriptionMode(): TranscriptionMode {
		return this.mode === 'open_router'
			? 'batch'
			: (this.app.settings?.transcription_mode ?? 'batch');
	}

	get captureBlocked() {
		return (
			this.app.activity === 'booting' ||
			this.app.activity === 'busy' ||
			!this.app.settings ||
			(this.mode === 'open_router'
				? !this.app.openRouterReady
				: this.app.activity === 'offline' || !this.app.overview) ||
			this.app.captureState === 'error'
		);
	}

	get modelName() {
		if (this.mode === 'open_router') return this.app.settings?.openrouter_model ?? 'Unselected';
		const path = this.app.overview?.runtime.model_path;
		return (
			this.app.overview?.models.find((model) => path?.endsWith(model.filename))?.name ??
			'Unselected'
		);
	}

	get endpointHost() {
		return this.mode === 'open_router'
			? 'openrouter.ai'
			: endpointLabel(this.app.settings?.remote_endpoint);
	}

	get surfaceTitle() {
		if (this.app.recording) return 'Listening now';
		if (this.app.processing) return 'Finishing your transcript';
		if (this.app.captureState === 'error') return 'Capture needs attention';
		if (this.captureBlocked) return 'The signal path needs attention';
		return 'Ready when you are';
	}

	setCopied(value: boolean) {
		this.copied = value;
	}

	onOpenSettings = () => this.shell.navigate('settings');
}

export const [getTranscribeContext, setTranscribeContext] = createContext<TranscribeState>();

export function modeLabel(mode: ServiceMode) {
	if (mode === 'local') return 'Local';
	if (mode === 'open_router') return 'OpenRouter';
	return 'Shadoword API';
}

function endpointLabel(endpoint: string | undefined) {
	if (!endpoint) return 'Not configured';
	try {
		return new URL(endpoint).host;
	} catch {
		return endpoint;
	}
}
