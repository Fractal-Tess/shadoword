import { describe, expect, test } from 'bun:test';
import type { TranscriptionResult } from './bindings';
import {
	historyRecordFromCompletion,
	mergeTranscriptSegment,
	transcriptFromSegments,
	transcriptionFingerprint
} from './desktop-events';

const result = (text: string, elapsedMs: number): TranscriptionResult => ({
	text,
	elapsed_ms: elapsedMs,
	engine: 'whisper.cpp',
	audio_duration_ms: 900,
	sample_rate: 48_000,
	cost_usd: null
});

describe('desktop event routing helpers', () => {
	test('orders streamed segments and replaces duplicate indexes without duplicate text', () => {
		let segments = mergeTranscriptSegment({}, 1, result('world', 20));
		segments = mergeTranscriptSegment(segments, 0, result('hello', 10));
		const unchanged = mergeTranscriptSegment(segments, 0, result('hello', 10));

		expect(unchanged).toBe(segments);
		expect(transcriptFromSegments(segments)).toBe('hello\nworld');
	});

	test('labels completed history with the native session target', () => {
		expect(
			historyRecordFromCompletion('1', '09:42', 'local', result('hello', 10), 2)
		).toMatchObject({
			engine: 'Local · whisper.cpp',
			segments: 2,
			text: 'hello'
		});
		expect(historyRecordFromCompletion('2', '09:43', 'remote', result('hello', 10), 1).engine).toBe(
			'Shadoword API · whisper.cpp'
		);
		expect(
			historyRecordFromCompletion('3', '09:44', 'open_router', result('hello', 10), 1).engine
		).toBe('OpenRouter · whisper.cpp');
	});

	test('keeps OpenRouter request cost in session history', () => {
		const completed = { ...result('priced words', 18), cost_usd: 0.000125 };
		expect(
			historyRecordFromCompletion('priced', '09:45', 'open_router', completed, 1)
		).toMatchObject({ costUsd: 0.000125, engine: 'OpenRouter · whisper.cpp' });
	});

	test('completion fingerprints distinguish target and segment count', () => {
		const completed = result('same words', 10);
		expect(transcriptionFingerprint('local', completed, 1)).not.toBe(
			transcriptionFingerprint('remote', completed, 1)
		);
		expect(transcriptionFingerprint('remote', completed, 1)).not.toBe(
			transcriptionFingerprint('remote', completed, 2)
		);
	});
});
