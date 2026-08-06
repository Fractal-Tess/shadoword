import type {
	DesktopSettings,
	DesktopSettingsInput,
	SecretUpdate,
	ServiceMode
} from '$lib/bindings';
import {
	effectiveStreamingPcmFormat,
	effectiveTranscriptionMode
} from '$lib/shell/environment-policy';

export function settingsInputForMode(
	settings: DesktopSettings,
	remoteToken: SecretUpdate,
	mode: ServiceMode = settings.mode,
	openRouterKey: SecretUpdate = { action: 'keep' }
) {
	return buildDesktopSettingsInput(settings, {
		mode,
		remote_token: remoteToken,
		openrouter_key: openRouterKey
	});
}

export function buildDesktopSettingsInput(
	settings: DesktopSettings,
	updates: Partial<DesktopSettingsInput> = {}
) {
	const mode = updates.mode ?? settings.mode;
	const transcriptionMode = updates.transcription_mode ?? settings.transcription_mode;
	const streamingPcmFormat = updates.streaming_pcm_format ?? settings.streaming_pcm_format;

	return {
		model_path: settings.model_path,
		preload_on_startup: settings.preload_on_startup,
		whisper_accelerator: settings.whisper_accelerator,
		whisper_gpu_device: settings.whisper_gpu_device,
		remote_endpoint: settings.remote_endpoint,
		remote_token: { action: 'keep' },
		openrouter_model: settings.openrouter_model,
		openrouter_key: { action: 'keep' },
		input_device: settings.input_device,
		sample_rate: settings.sample_rate,
		english_only: settings.english_only,
		copy_to_clipboard: settings.copy_to_clipboard,
		paste_method: settings.paste_method,
		paste_delay_ms: settings.paste_delay_ms,
		output_prefix: settings.output_prefix,
		output_suffix: settings.output_suffix,
		hotkey_shortcut: settings.hotkey_shortcut,
		hotkey_mode: settings.hotkey_mode,
		close_to_tray: settings.close_to_tray,
		show_window_title_bar: settings.show_window_title_bar,
		...updates,
		mode,
		transcription_mode: effectiveTranscriptionMode(mode, transcriptionMode),
		streaming_pcm_format: effectiveStreamingPcmFormat(mode, streamingPcmFormat)
	} satisfies DesktopSettingsInput;
}
