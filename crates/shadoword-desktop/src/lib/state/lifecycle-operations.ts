import { commands, events, type DesktopEvent } from '$lib/bindings';
import { demoHistory } from '$lib/demo-data';
import type { DesktopStateContext } from './contracts';
import type { CaptureOperations } from './capture-operations';
import { demoOverview, demoSettings } from './demo-fixtures';
import type { DownloadOperations } from './download-operations';
import { failOverview } from './errors';

export class LifecycleOperations {
	private eventRevision = 0;
	private listenerPromise: Promise<() => void> | null = null;
	private unlisten: (() => void) | null = null;
	private listenerClosed = false;
	private initialization: Promise<void> | null = null;
	private disposed = false;

	constructor(
		private app: DesktopStateContext,
		private capture: CaptureOperations,
		private downloads: DownloadOperations
	) {}

	initialize() {
		this.initialization ??= this.initializeOnce();
		return this.initialization;
	}

	dispose() {
		this.disposed = true;
		this.downloads.dispose();
		if (this.unlisten) this.closeListener(this.unlisten);
		else if (this.listenerPromise) {
			void this.listenerPromise.then((unlisten) => this.closeListener(unlisten));
		}
	}

	private async initializeOnce() {
		this.disposed = false;
		this.listenerClosed = false;
		this.app.activity = 'booting';
		this.app.error = null;
		if (this.app.demo) {
			this.app.settings = { ...demoSettings };
			this.app.inputDevices = [
				{ name: 'Studio Microphone', is_default: true },
				{ name: 'Webcam Microphone', is_default: false }
			];
			this.app.overview = demoOverview;
			this.app.history = demoHistory.map((record) => ({ ...record }));
			this.app.activity = 'ready';
			this.app.statusMessage = 'Simulated desktop ready';
			return;
		}

		try {
			await this.subscribeToDesktopEvents();
			const revisionBeforeLoad = this.eventRevision;
			const bootstrap = await commands.loadDesktopState();
			if (this.disposed) return;
			// Assigned directly rather than through `setHistory`: this is the disk
			// contents arriving, and routing it through the setter would write them
			// straight back out on every launch. Failure is swallowed because an
			// unreadable history is not a reason to refuse to start recording.
			this.app.history = await commands.loadHistory().catch(() => []);
			if (this.disposed) return;
			this.app.settings = bootstrap.settings;
			this.app.inputDevices = bootstrap.input_devices;
			this.app.inputDevicesError = bootstrap.input_devices_error;
			this.app.hotkeyError = bootstrap.hotkey_error;
			const localStartupError = bootstrap.local_startup_error;
			this.app.overview =
				bootstrap.settings.mode === 'local' ? bootstrap.local_overview : this.app.overview;
			if (revisionBeforeLoad === this.eventRevision) {
				this.capture.hydrateRecording(bootstrap.recording);
			}
			await this.app.refreshOverview();
			if (localStartupError) {
				this.app.error = `Local model: ${localStartupError.message}${localStartupError.action ? ` ${localStartupError.action}` : ''}`;
				this.app.statusMessage = 'Local model preload failed';
			}
		} catch (error) {
			if (!this.disposed) {
				failOverview(this.app, error, 'Could not initialize the native desktop host');
			}
		}
	}

	private async subscribeToDesktopEvents() {
		if (this.listenerPromise) return this.listenerPromise;
		this.listenerPromise = events.shadowordDesktopEvent.listen((event) => {
			if (!this.disposed) this.handleDesktopEvent(event.payload);
		});
		const unlisten = await this.listenerPromise;
		if (this.disposed) this.closeListener(unlisten);
		else this.unlisten = unlisten;
		return unlisten;
	}

	private closeListener(unlisten: () => void) {
		if (this.listenerClosed) return;
		this.listenerClosed = true;
		unlisten();
		this.unlisten = null;
	}

	private handleDesktopEvent(event: DesktopEvent) {
		this.eventRevision += 1;
		switch (event.type) {
			case 'status':
				this.app.statusMessage = event.message;
				if (event.message.toLowerCase().includes('ready')) void this.app.refreshOverview();
				break;
			case 'recording_started':
				this.capture.beginRecording(event.sample_rate);
				void this.capture.syncActiveRecordingMetadata();
				break;
			case 'recording_stopped':
				if (event.processing) {
					this.app.captureState = 'finalizing';
					this.app.statusMessage = 'Finalizing the recording…';
				} else if (this.app.captureState !== 'error') {
					this.app.captureState = 'idle';
					this.app.statusMessage = 'Recording stopped';
				}
				break;
			case 'recording_cancelled':
				this.capture.cancelActiveSession();
				break;
			case 'transcript_segment':
				this.capture.acceptTranscriptSegment(event.segment_index, event.result);
				break;
			case 'transcription_complete':
				this.capture.completeTranscription(event.result, event.segments);
				break;
			case 'error':
				this.capture.handleNativeError(event);
				break;
		}
	}
}
