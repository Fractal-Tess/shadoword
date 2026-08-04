import type { ServiceMode } from '$lib/bindings';

type WhisperServiceMode = Exclude<ServiceMode, 'open_router'>;

export const MODE_COMMAND_NAMES = {
	local: {
		refreshOverview: 'refreshLocalOverview',
		updateRuntime: 'updateLocalRuntime',
		selectModel: 'selectLocalModel',
		startDownload: 'startLocalDownload',
		pollDownload: 'pollLocalDownload'
	},
	remote: {
		refreshOverview: 'refreshRemoteOverview',
		updateRuntime: 'updateRemoteRuntime',
		selectModel: 'selectRemoteModel',
		startDownload: 'startRemoteDownload',
		pollDownload: 'pollRemoteDownload'
	}
} as const satisfies Record<
	WhisperServiceMode,
	{
		refreshOverview: 'refreshLocalOverview' | 'refreshRemoteOverview';
		updateRuntime: 'updateLocalRuntime' | 'updateRemoteRuntime';
		selectModel: 'selectLocalModel' | 'selectRemoteModel';
		startDownload: 'startLocalDownload' | 'startRemoteDownload';
		pollDownload: 'pollLocalDownload' | 'pollRemoteDownload';
	}
>;

export function commandNamesForMode(mode: ServiceMode) {
	if (mode === 'open_router') {
		throw new Error('OpenRouter does not expose Whisper runtime management commands.');
	}
	return MODE_COMMAND_NAMES[mode];
}
