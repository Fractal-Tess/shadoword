import type { DownloadJobStatus, ServiceMode } from './bindings';

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

export function serviceModeLabel(mode: ServiceMode) {
	if (mode === 'local') return 'Local';
	if (mode === 'open_router') return 'OpenRouter';
	return 'Shadoword API';
}

/** History entries now survive restarts, so a bare clock reading is ambiguous the
 *  moment the app is reopened the next day. Today stays a clock reading because
 *  that is the common case and the date would be noise; anything older is dated. */
export function formatRecordedAt(recordedAt: string) {
	const recorded = new Date(recordedAt);
	if (Number.isNaN(recorded.getTime())) return recordedAt;
	const time = recorded.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
	const today = new Date();
	if (recorded.toDateString() === today.toDateString()) return time;
	const yesterday = new Date(today);
	yesterday.setDate(today.getDate() - 1);
	if (recorded.toDateString() === yesterday.toDateString()) return `Yesterday, ${time}`;
	const date = recorded.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
	return `${date}, ${time}`;
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
