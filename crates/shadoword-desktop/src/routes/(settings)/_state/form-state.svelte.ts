import type {
	DesktopSettings,
	HotkeyMode,
	PasteMethod,
	StreamingPcmFormat,
	TranscriptBoundary,
	TranscriptionMode
} from '$lib/bindings';

type ChangeHandler = (immediate?: boolean) => void;

export class SettingsFormState {
	microphone = $state('');
	shortcutMode = $state<HotkeyMode>('push_to_talk');
	shortcut = $state('F2');
	shortcutCapturing = $state(false);
	shortcutError = $state('');
	transcriptionMode = $state<TranscriptionMode>('batch');
	streamingPcmFormat = $state<StreamingPcmFormat>('f32le');
	englishOnly = $state(false);
	copyFinal = $state(true);
	pasteMethod = $state<PasteMethod>('none');
	pasteDelay = $state('120');
	outputPrefix = $state<TranscriptBoundary>('none');
	outputSuffix = $state<TranscriptBoundary>('space');
	closeToTray = $state(true);
	showWindowTitleBar = $state(true);
	#onChange: ChangeHandler = () => {};

	constructor(settings: DesktopSettings, runtimeEnglishOnly?: boolean) {
		this.microphone = settings.input_device ?? '';
		this.shortcutMode = settings.hotkey_mode;
		this.shortcut = settings.hotkey_shortcut.toUpperCase();
		this.transcriptionMode = settings.transcription_mode;
		this.streamingPcmFormat = settings.streaming_pcm_format;
		this.englishOnly = runtimeEnglishOnly ?? settings.english_only;
		this.copyFinal = settings.copy_to_clipboard;
		this.pasteMethod = settings.paste_method;
		this.pasteDelay = String(settings.paste_delay_ms);
		this.outputPrefix = settings.output_prefix;
		this.outputSuffix = settings.output_suffix;
		this.closeToTray = settings.close_to_tray;
		this.showWindowTitleBar = settings.show_window_title_bar;
	}

	setChangeHandler(handler: ChangeHandler) {
		this.#onChange = handler;
	}

	get pasteDelayError() {
		const delay = Number(this.pasteDelay);
		return Number.isInteger(delay) && delay >= 0 && delay <= 1000
			? ''
			: 'Paste delay must be a whole number from 0 to 1000 milliseconds.';
	}

	setMicrophone(value: string) {
		if (this.microphone === value) return;
		this.microphone = value;
		this.#onChange(true);
	}

	setShortcutMode(value: string) {
		if (value !== 'push_to_talk' && value !== 'toggle') return;
		if (this.shortcutMode === value) return;
		this.shortcutMode = value;
		this.#onChange(true);
	}

	setTranscriptionMode(value: TranscriptionMode) {
		if (this.transcriptionMode === value) return;
		this.transcriptionMode = value;
		this.#onChange(true);
	}

	setStreamingPcmFormat(value: string) {
		if (value !== 's16le' && value !== 'f32le') return;
		if (this.streamingPcmFormat === value) return;
		this.streamingPcmFormat = value;
		this.#onChange(true);
	}

	setEnglishOnly(value: boolean) {
		if (this.englishOnly === value) return;
		this.englishOnly = value;
		this.#onChange(true);
	}

	setCopyFinal(value: boolean) {
		if (this.copyFinal === value) return;
		this.copyFinal = value;
		this.#onChange(true);
	}

	setPasteMethod(value: string) {
		if (!isPasteMethod(value) || this.pasteMethod === value) return;
		this.pasteMethod = value;
		this.#onChange(true);
	}

	setPasteDelay(value: string) {
		if (this.pasteDelay === value) return;
		this.pasteDelay = value;
		this.#onChange();
	}

	setOutputPrefix(value: string) {
		if (!isTranscriptBoundary(value) || this.outputPrefix === value) return;
		this.outputPrefix = value;
		this.#onChange(true);
	}

	setOutputSuffix(value: string) {
		if (!isTranscriptBoundary(value) || this.outputSuffix === value) return;
		this.outputSuffix = value;
		this.#onChange(true);
	}

	setCloseToTray(value: boolean) {
		if (this.closeToTray === value) return;
		this.closeToTray = value;
		this.#onChange(true);
	}

	setShowWindowTitleBar(value: boolean) {
		if (this.showWindowTitleBar === value) return;
		this.showWindowTitleBar = value;
		this.#onChange(true);
	}

	toggleShortcutCapture() {
		this.shortcutCapturing = !this.shortcutCapturing;
		this.shortcutError = '';
	}

	captureShortcut(event: KeyboardEvent) {
		if (!this.shortcutCapturing || event.repeat || event.isComposing) return;
		event.preventDefault();
		event.stopPropagation();
		if (event.key === 'Escape') {
			this.shortcutCapturing = false;
			this.shortcutError = '';
			return;
		}
		if (['Control', 'Alt', 'Shift', 'Meta'].includes(event.key)) return;

		const key = shortcutKey(event.key);
		if (!key) {
			this.shortcutError = `Unsupported shortcut key: ${event.key}`;
			return;
		}
		const modifiers = [
			event.ctrlKey ? 'Ctrl' : null,
			event.altKey ? 'Alt' : null,
			event.shiftKey ? 'Shift' : null,
			event.metaKey ? 'Super' : null
		].filter((modifier): modifier is string => modifier !== null);
		if (key.length === 1 && modifiers.length === 0) {
			this.shortcutError = 'Text keys need Ctrl, Alt, Shift, or Super.';
			return;
		}

		this.shortcut = [...modifiers, key].join('+');
		this.shortcutError = '';
		this.shortcutCapturing = false;
		this.#onChange(true);
	}
}

function isPasteMethod(value: string): value is PasteMethod {
	return ['none', 'direct', 'ctrl_v', 'ctrl_shift_v', 'shift_insert'].includes(value);
}

function isTranscriptBoundary(value: string): value is TranscriptBoundary {
	return ['none', 'space', 'newline', 'blank_line'].includes(value);
}

function shortcutKey(key: string) {
	if (key === ' ') return 'Space';
	if (key.startsWith('Arrow')) return key.slice('Arrow'.length);
	if (/^F(?:[1-9]|1[0-9]|2[0-4])$/i.test(key)) return key.toUpperCase();
	if (key.length === 1) return key.toUpperCase();
	const supported = new Set([
		'Tab',
		'Enter',
		'Backspace',
		'Insert',
		'Delete',
		'Home',
		'End',
		'PageUp',
		'PageDown',
		'CapsLock',
		'PrintScreen',
		'ScrollLock',
		'Pause'
	]);
	return supported.has(key) ? key : null;
}
