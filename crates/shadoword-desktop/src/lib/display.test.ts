import { describe, expect, test } from 'bun:test';
import { downloadPercent, errorMessage, formatBytes, formatDuration } from './display';

describe('desktop display helpers', () => {
	test('formats native transcription measurements', () => {
		expect(formatDuration(4250)).toBe('4.3s');
		expect(formatBytes(0)).toBe('0 B');
		expect(formatBytes(1536)).toBe('1.50 KiB');
		expect(formatBytes(64 * 1024 ** 2)).toBe('64.0 MiB');
		expect(formatBytes(1.5 * 1024 ** 3)).toBe('1.50 GiB');
	});

	test('keeps download progress bounded and handles missing totals', () => {
		expect(downloadPercent(undefined)).toBe(0);
		expect(
			downloadPercent({
				id: '1',
				model_id: 'turbo',
				state: 'running',
				downloaded: 120,
				total: 100,
				path: null,
				skipped: false,
				verified: false,
				error: null
			})
		).toBe(100);
	});

	test('normalizes thrown IPC errors without unsafe casts', () => {
		expect(errorMessage(new Error('offline'))).toBe('offline');
		expect(errorMessage('unauthorized')).toBe('unauthorized');
		expect(errorMessage({ message: 'offline', action: 'Check the endpoint.' })).toBe(
			'offline Check the endpoint.'
		);
		expect(errorMessage({ code: 'opaque' })).toBe('Unknown error');
	});
});
