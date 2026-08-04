import type { ServiceMode } from '$lib/bindings';

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
	ServiceMode,
	{
		refreshOverview: 'refreshLocalOverview' | 'refreshRemoteOverview';
		updateRuntime: 'updateLocalRuntime' | 'updateRemoteRuntime';
		selectModel: 'selectLocalModel' | 'selectRemoteModel';
		startDownload: 'startLocalDownload' | 'startRemoteDownload';
		pollDownload: 'pollLocalDownload' | 'pollRemoteDownload';
	}
>;

export function commandNamesForMode(mode: ServiceMode) {
	return MODE_COMMAND_NAMES[mode];
}
