import { describe, expect, test } from 'bun:test';
import { commandNamesForMode } from './native-routing';

describe('mode-specific native command routing', () => {
	test('routes every local operation to its local command', () => {
		expect(commandNamesForMode('local')).toEqual({
			refreshOverview: 'refreshLocalOverview',
			updateRuntime: 'updateLocalRuntime',
			selectModel: 'selectLocalModel',
			startDownload: 'startLocalDownload',
			pollDownload: 'pollLocalDownload'
		});
	});

	test('routes every remote operation to its remote command', () => {
		expect(commandNamesForMode('remote')).toEqual({
			refreshOverview: 'refreshRemoteOverview',
			updateRuntime: 'updateRemoteRuntime',
			selectModel: 'selectRemoteModel',
			startDownload: 'startRemoteDownload',
			pollDownload: 'pollRemoteDownload'
		});
	});
});
