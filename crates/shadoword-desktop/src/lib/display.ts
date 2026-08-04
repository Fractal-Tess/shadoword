import type { DownloadJobStatus } from './bindings';

export function errorMessage(error: unknown) {
	if (error instanceof Error) return error.message;
	if (typeof error === 'string') return error;
	if (isErrorPayload(error)) {
		return `${error.message}${typeof error.action === 'string' ? ` ${error.action}` : ''}`;
	}
	return 'Unknown error';
}

function isErrorPayload(error: unknown): error is { message: string; action?: unknown } {
	return (
		typeof error === 'object' &&
		error !== null &&
		'message' in error &&
		typeof error.message === 'string'
	);
}

export function formatDuration(milliseconds: number) {
	return `${(milliseconds / 1000).toFixed(1)}s`;
}

export function formatBytes(bytes: number) {
	if (!Number.isFinite(bytes) || bytes < 0) return 'Unknown size';
	if (bytes === 0) return '0 B';
	const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB'] as const;
	const unitIndex = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
	const value = bytes / 1024 ** unitIndex;
	const precision = unitIndex === 0 ? 0 : value >= 100 ? 0 : value >= 10 ? 1 : 2;
	return `${value.toFixed(precision)} ${units[unitIndex]}`;
}

export function downloadPercent(download: DownloadJobStatus | undefined) {
	if (!download || download.total <= 0) return 0;
	return Math.min(100, Math.round((download.downloaded / download.total) * 100));
}
