import type {
	DesktopSettings,
	HotkeyMode,
	PasteMethod,
	SecretUpdate,
	StreamingPcmFormat,
	TranscriptBoundary,
	TranscriptionMode
} from '$lib/bindings';
import { buildDesktopSettingsInput } from '$lib/desktop-settings';

export type SettingsDraft = {
	readonly remoteEndpoint: string;
	readonly remoteToken: string;
	readonly remoteTokenDirty: boolean;
	readonly clearRemoteToken: boolean;
	readonly openRouterModel: string;
	readonly openRouterKey: string;
	readonly openRouterKeyDirty: boolean;
	readonly clearOpenRouterKey: boolean;
	readonly microphone: string;
	readonly transcriptionMode: TranscriptionMode;
	readonly streamingPcmFormat: StreamingPcmFormat;
	readonly englishOnly: boolean;
	readonly copyFinal: boolean;
	readonly pasteMethod: PasteMethod;
	readonly pasteDelay: string;
	readonly outputPrefix: TranscriptBoundary;
	readonly outputSuffix: TranscriptBoundary;
	readonly shortcut: string;
	readonly shortcutMode: HotkeyMode;
	readonly closeToTray: boolean;
	readonly showWindowTitleBar: boolean;
};

export function settingsInputFromDraft(settings: DesktopSettings, draft: SettingsDraft) {
	return buildDesktopSettingsInput(settings, {
		remote_endpoint: draft.remoteEndpoint,
		remote_token: secretUpdate(draft.remoteToken, draft.remoteTokenDirty, draft.clearRemoteToken),
		openrouter_model: draft.openRouterModel,
		openrouter_key: secretUpdate(
			draft.openRouterKey,
			draft.openRouterKeyDirty,
			draft.clearOpenRouterKey
		),
		input_device: draft.microphone || null,
		transcription_mode: draft.transcriptionMode,
		streaming_pcm_format: draft.streamingPcmFormat,
		english_only: draft.englishOnly,
		copy_to_clipboard: draft.copyFinal,
		paste_method: draft.pasteMethod,
		paste_delay_ms: Number(draft.pasteDelay),
		output_prefix: draft.outputPrefix,
		output_suffix: draft.outputSuffix,
		hotkey_shortcut: draft.shortcut,
		hotkey_mode: draft.shortcutMode,
		close_to_tray: draft.closeToTray,
		show_window_title_bar: draft.showWindowTitleBar
	});
}

function secretUpdate(value: string, dirty: boolean, clear: boolean): SecretUpdate {
	if (clear) return { action: 'clear' };
	if (dirty) return { action: 'set', value };
	return { action: 'keep' };
}
