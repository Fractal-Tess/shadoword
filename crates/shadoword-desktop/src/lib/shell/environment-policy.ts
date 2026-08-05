import type { ServiceMode } from '$lib/bindings';
import type { PageId } from '$lib/types';

export type EnvironmentCapabilities = {
	managesWhisperRuntime: boolean;
	supportsStreaming: boolean;
	supportsPcmSelection: boolean;
};

const CAPABILITIES: Record<ServiceMode, EnvironmentCapabilities> = {
	local: {
		managesWhisperRuntime: true,
		supportsStreaming: true,
		supportsPcmSelection: false
	},
	remote: {
		managesWhisperRuntime: true,
		supportsStreaming: true,
		supportsPcmSelection: true
	},
	open_router: {
		managesWhisperRuntime: false,
		supportsStreaming: false,
		supportsPcmSelection: false
	}
};

export function capabilitiesFor(mode: ServiceMode | null | undefined) {
	return mode ? CAPABILITIES[mode] : null;
}

export function isPageAvailable(page: PageId, mode: ServiceMode | null | undefined) {
	return page !== 'models' || capabilitiesFor(mode)?.managesWhisperRuntime !== false;
}

export function fallbackPage(page: PageId, mode: ServiceMode | null | undefined): PageId {
	return isPageAvailable(page, mode) ? page : 'settings';
}

export function effectiveTranscriptionMode(mode: ServiceMode, configured: 'batch' | 'streaming') {
	return capabilitiesFor(mode)?.supportsStreaming ? configured : 'batch';
}

export function effectiveStreamingPcmFormat(mode: ServiceMode, configured: 's16le' | 'f32le') {
	return capabilitiesFor(mode)?.supportsPcmSelection ? configured : 'f32le';
}
