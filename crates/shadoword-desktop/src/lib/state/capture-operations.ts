import {
	commands,
	type DesktopEvent,
	type RecordingState,
	type TranscriptionResult
} from '$lib/bindings';
import {
	historyRecordFromCompletion,
	mergeTranscriptSegment,
	transcriptFromSegments,
	transcriptionFingerprint
} from '$lib/desktop-events';
import type { DesktopStateContext } from './contracts';
import { demoTranscription } from './demo-operations';
import { sentenceCase, setAppError } from './errors';

export class CaptureOperations {
	private activeSessionId: string | null = null;
	private completedSessionId: string | null = null;
	private sessionSequence = 0;
	private lastCompletion: { fingerprint: string; at: number } | null = null;

	constructor(private app: DesktopStateContext) {}

	async start() {
		if (this.app.captureLocked) return;
		this.app.clearError();
		this.app.statusMessage = 'Starting microphone capture…';
		if (this.app.demo) {
			this.beginRecording(48_000);
			void this.syncActiveRecordingMetadata();
			return;
		}
		try {
			const status = await commands.startRecording();
			if (status.recording && this.app.captureState !== 'recording') {
				this.beginRecording(status.sample_rate);
				void this.syncActiveRecordingMetadata();
			}
		} catch (error) {
			this.failCapture(error, 'Could not start microphone capture');
		}
	}

	async stop() {
		if (this.app.captureState !== 'recording') return;
		const sessionId = this.activeSessionId;
		this.app.captureState = 'finalizing';
		this.app.statusMessage = 'Finalizing the recording…';
		try {
			if (this.app.demo) {
				const result = demoTranscription(this.app.recordingTranscriptionMode ?? 'batch');
				if (this.app.recordingTranscriptionMode === 'streaming') {
					this.acceptTranscriptSegment(0, {
						...result,
						text: 'Keep the API on the workstation.'
					});
				}
				this.completeTranscription(
					result,
					this.app.recordingTranscriptionMode === 'streaming' ? 1 : 1
				);
				return;
			}
			const result = await commands.stopAndTranscribe();
			const segments =
				this.app.recordingTranscriptionMode === 'streaming' ? this.app.segmentCount : 1;
			this.completeTranscription(result, segments, sessionId);
		} catch (error) {
			if (this.completedSessionId !== sessionId) this.failCapture(error, 'Transcription failed');
		}
	}

	async cancel() {
		if (this.app.captureState !== 'recording') return;
		try {
			if (this.app.demo) this.cancelActiveSession();
			else {
				await commands.cancelRecording();
				if (this.app.captureState === 'recording') this.cancelActiveSession();
			}
		} catch (error) {
			this.failCapture(error, 'Could not cancel the recording');
		}
	}

	hydrateRecording(recording: RecordingState) {
		if (recording.phase === 'idle') return;
		this.activeSessionId = this.nextSessionId();
		this.app.recordingMode = recording.service_mode ?? this.app.settings?.mode ?? null;
		this.app.recordingTranscriptionMode =
			recording.transcription_mode ?? this.app.settings?.transcription_mode ?? null;
		this.app.recordingSampleRate = recording.sample_rate ?? 0;
		this.app.captureState = recording.phase;
		this.app.statusMessage =
			recording.phase === 'recording' ? 'Recording in progress' : 'Finalizing';
	}

	async syncActiveRecordingMetadata() {
		const sessionId = this.activeSessionId;
		if (this.app.demo || !sessionId) return;
		try {
			const recording = await commands.getRecordingState();
			if (this.activeSessionId !== sessionId || recording.phase === 'idle') return;
			this.app.recordingMode = recording.service_mode ?? this.app.recordingMode;
			this.app.recordingTranscriptionMode =
				recording.transcription_mode ?? this.app.recordingTranscriptionMode;
			this.app.recordingSampleRate = recording.sample_rate ?? this.app.recordingSampleRate;
		} catch {
			// Event state remains authoritative when a metadata refresh races with shutdown.
		}
	}

	beginRecording(sampleRate: number) {
		if (this.app.captureState === 'error') this.app.clearError();
		if (this.app.captureState !== 'recording' || !this.activeSessionId) {
			this.activeSessionId = this.nextSessionId();
			this.completedSessionId = null;
			this.app.recordingMode = this.app.settings?.mode ?? null;
			this.app.recordingTranscriptionMode =
				this.app.settings?.mode === 'open_router'
					? 'batch'
					: (this.app.settings?.transcription_mode ?? null);
			this.app.segmentResults = {};
			this.app.transcript = '';
			this.app.lastResult = null;
		}
		this.app.captureState = 'recording';
		this.app.recordingSampleRate = sampleRate;
		this.app.statusMessage = `${this.app.recordingTranscriptionMode === 'streaming' ? 'Streaming' : 'Batch'} recording in progress`;
	}

	acceptTranscriptSegment(segmentIndex: number, result: TranscriptionResult) {
		if (!this.activeSessionId) this.beginRecording(result.sample_rate);
		this.app.segmentResults = mergeTranscriptSegment(this.app.segmentResults, segmentIndex, result);
		this.app.transcript = transcriptFromSegments(this.app.segmentResults);
		this.app.lastResult = result;
		this.app.statusMessage = `Received transcript segment ${segmentIndex + 1}`;
	}

	completeTranscription(
		result: TranscriptionResult,
		segments: number,
		expectedSessionId?: string | null
	) {
		const sessionId = expectedSessionId ?? this.activeSessionId ?? this.nextSessionId();
		if (this.completedSessionId === sessionId) return;
		const mode = this.app.recordingMode ?? this.app.settings?.mode ?? 'remote';
		const fingerprint = transcriptionFingerprint(mode, result, segments);
		const now = Date.now();
		if (
			!this.activeSessionId &&
			this.lastCompletion?.fingerprint === fingerprint &&
			now - this.lastCompletion.at < 2_000
		) {
			return;
		}

		this.completedSessionId = sessionId;
		this.lastCompletion = { fingerprint, at: now };
		this.app.lastResult = result;
		this.app.transcript = result.text;
		if (result.text.trim()) {
			const record = historyRecordFromCompletion(
				sessionId,
				new Date(now).toISOString(),
				mode,
				result,
				segments
			);
			if (!this.app.history.some((item) => item.id === record.id)) {
				this.app.setHistory([record, ...this.app.history]);
			}
		}
		this.app.captureState = 'idle';
		this.app.recordingSampleRate = result.sample_rate;
		this.app.statusMessage = `Transcription complete · ${segments} ${segments === 1 ? 'segment' : 'segments'}`;
		this.app.activity = 'ready';
		this.activeSessionId = null;
		this.app.recordingMode = null;
		this.app.recordingTranscriptionMode = null;
	}

	cancelActiveSession() {
		this.app.captureState = 'idle';
		this.app.statusMessage = 'Recording cancelled';
		this.activeSessionId = null;
		this.app.recordingMode = null;
		this.app.recordingTranscriptionMode = null;
		this.app.segmentResults = {};
	}

	handleNativeError(event: Extract<DesktopEvent, { type: 'error' }>) {
		const context = sentenceCase(event.context);
		this.app.error = `${context}: ${event.message}${event.action ? ` ${event.action}` : ''}`;
		this.app.errorRetry = null;
		this.app.statusMessage = `${context} error`;
		if (event.context === 'hotkey') this.app.hotkeyError = event.message;
		if (
			event.context === 'streaming' ||
			event.context === 'transcription' ||
			(event.context === 'hotkey' && !event.code.includes('unavailable'))
		) {
			this.app.captureState = 'error';
			this.activeSessionId = null;
		}
	}

	private nextSessionId() {
		this.sessionSequence += 1;
		return `session-${Date.now()}-${this.sessionSequence}`;
	}

	private failCapture(error: unknown, context: string) {
		this.app.captureState = 'error';
		this.activeSessionId = null;
		setAppError(this.app, error, context);
		this.app.statusMessage = 'Capture error';
	}
}
