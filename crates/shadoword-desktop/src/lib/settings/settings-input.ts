import type {
	DesktopSettings,
	DesktopSettingsInput,
	HotkeyMode,
	PasteMethod,
	SecretUpdate,
	StreamingPcmFormat,
	TranscriptionMode
} from '$lib/bindings';

export interface SettingsDraft {
	readonly remoteEndpoint: string;
	readonly remoteToken: string;
	readonly remoteTokenDirty: boolean;
	readonly clearRemoteToken: boolean;
	readonly openRouterModel: string;
	readonly openRouterKey: string;
	readonly openRouterKeyDirty: boolean;
	readonly clearOpenRouterKey: boolean;
	readonly microphone: string;
	readonly sampleRate: string;
	readonly transcriptionMode: TranscriptionMode;
	readonly streamingPcmFormat: StreamingPcmFormat;
	readonly englishOnly: boolean;
	readonly copyFinal: boolean;
	readonly pasteMethod: PasteMethod;
	readonly pasteDelay: string;
	readonly shortcut: string;
	readonly shortcutMode: HotkeyMode;
	readonly closeToTray: boolean;
}

export function settingsInputFromDraft(
	settings: DesktopSettings,
	draft: SettingsDraft
): DesktopSettingsInput {
	const mode = settings.mode;
	return {
		mode,
		model_path: settings.model_path,
		preload_on_startup: settings.preload_on_startup,
		whisper_accelerator: settings.whisper_accelerator,
		whisper_gpu_device: settings.whisper_gpu_device,
		remote_endpoint: draft.remoteEndpoint,
		remote_token: secretUpdate(draft.remoteToken, draft.remoteTokenDirty, draft.clearRemoteToken),
		openrouter_model: draft.openRouterModel,
		openrouter_key: secretUpdate(
			draft.openRouterKey,
			draft.openRouterKeyDirty,
			draft.clearOpenRouterKey
		),
		input_device: draft.microphone || null,
		sample_rate: Number(draft.sampleRate),
		transcription_mode: mode === 'open_router' ? 'batch' : draft.transcriptionMode,
		streaming_pcm_format: mode === 'remote' ? draft.streamingPcmFormat : 'f32le',
		english_only: draft.englishOnly,
		copy_to_clipboard: draft.copyFinal,
		paste_method: draft.pasteMethod,
		paste_delay_ms: Number(draft.pasteDelay),
		hotkey_shortcut: draft.shortcut,
		hotkey_mode: draft.shortcutMode,
		close_to_tray: draft.closeToTray
	};
}

function secretUpdate(value: string, dirty: boolean, clear: boolean): SecretUpdate {
	if (clear) return { action: 'clear' };
	if (dirty) return { action: 'set', value };
	return { action: 'keep' };
}
