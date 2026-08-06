import type { DesktopAppState } from '$lib/app-state.svelte';
import type { ServiceMode, TranscriptionMode } from '$lib/bindings';
import { createContext } from 'svelte';

export type TranscribeContext = {
	readonly app: DesktopAppState;
	readonly mode: ServiceMode;
	readonly transcriptionMode: TranscriptionMode;
	readonly captureBlocked: boolean;
	readonly modelName: string;
	readonly endpointHost: string;
	readonly surfaceTitle: string;
	readonly copied: boolean;
	readonly setCopied: (value: boolean) => void;
	readonly onOpenSettings: () => void;
};

export const [getTranscribeContext, setTranscribeContext] = createContext<TranscribeContext>();

export function modeLabel(mode: ServiceMode) {
	if (mode === 'local') return 'Local';
	if (mode === 'open_router') return 'OpenRouter';
	return 'Shadoword API';
}

export function endpointLabel(endpoint: string | undefined) {
	if (!endpoint) return 'Not configured';
	try {
		return new URL(endpoint).host;
	} catch {
		return endpoint;
	}
}
