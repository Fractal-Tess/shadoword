import type { ServiceMode, TranscriptionResult } from '$lib/bindings';
import type { HistoryRecord } from '$lib/types';
import { formatDuration } from '$lib/display';

export type TranscriptSegments = Readonly<Record<number, TranscriptionResult>>;

export function mergeTranscriptSegment(
	segments: TranscriptSegments,
	segmentIndex: number,
	result: TranscriptionResult
): TranscriptSegments {
	const current = segments[segmentIndex];
	if (current && transcriptionResultsEqual(current, result)) return segments;
	return { ...segments, [segmentIndex]: result };
}

export function transcriptFromSegments(segments: TranscriptSegments) {
	return Object.entries(segments)
		.sort(([left], [right]) => Number(left) - Number(right))
		.map(([, result]) => result.text.trim())
		.filter(Boolean)
		.join('\n');
}

export function historyRecordFromCompletion(
	id: string,
	timestamp: string,
	mode: ServiceMode,
	result: TranscriptionResult,
	segments: number
): HistoryRecord {
	return {
		id,
		timestamp,
		engine: `${serviceModeLabel(mode)} · ${result.engine}`,
		duration: formatDuration(result.audio_duration_ms),
		latency: `${result.elapsed_ms}ms`,
		text: result.text,
		segments
	};
}

export function transcriptionFingerprint(
	mode: ServiceMode,
	result: TranscriptionResult,
	segments: number
) {
	return [
		mode,
		result.text,
		result.elapsed_ms,
		result.engine,
		result.audio_duration_ms,
		result.sample_rate,
		segments
	].join('\u001f');
}

function serviceModeLabel(mode: ServiceMode) {
	if (mode === 'local') return 'Local';
	if (mode === 'open_router') return 'OpenRouter';
	return 'Remote';
}

function transcriptionResultsEqual(left: TranscriptionResult, right: TranscriptionResult) {
	return (
		left.text === right.text &&
		left.elapsed_ms === right.elapsed_ms &&
		left.engine === right.engine &&
		left.audio_duration_ms === right.audio_duration_ms &&
		left.sample_rate === right.sample_rate
	);
}
