import type { ServiceMode, TranscriptionResult } from '$lib/bindings';
import type { HistoryRecord } from '$lib/types';

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
	recordedAt: string,
	mode: ServiceMode,
	result: TranscriptionResult,
	segments: number
): HistoryRecord {
	return {
		id,
		recorded_at: recordedAt,
		mode,
		engine: result.engine,
		elapsed_ms: result.elapsed_ms,
		audio_duration_ms: result.audio_duration_ms,
		text: result.text,
		segments,
		cost_usd: result.cost_usd
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
		result.cost_usd,
		segments
	].join('\u001f');
}

function transcriptionResultsEqual(left: TranscriptionResult, right: TranscriptionResult) {
	return (
		left.text === right.text &&
		left.elapsed_ms === right.elapsed_ms &&
		left.engine === right.engine &&
		left.audio_duration_ms === right.audio_duration_ms &&
		left.sample_rate === right.sample_rate &&
		left.cost_usd === right.cost_usd
	);
}
