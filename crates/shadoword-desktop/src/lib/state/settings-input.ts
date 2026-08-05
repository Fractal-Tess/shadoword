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

export function settingsInput(
	settings: DesktopSettings,
	remoteToken: SecretUpdate,
	mode: ServiceMode = settings.mode,
	openRouterKey: SecretUpdate = { action: 'keep' }
): DesktopSettingsInput {
	return {
		mode,
		model_path: settings.model_path,
		preload_on_startup: settings.preload_on_startup,
		whisper_accelerator: settings.whisper_accelerator,
		whisper_gpu_device: settings.whisper_gpu_device,
		remote_endpoint: settings.remote_endpoint,
		remote_token: remoteToken,
		openrouter_model: settings.openrouter_model,
		openrouter_key: openRouterKey,
		input_device: settings.input_device,
		sample_rate: settings.sample_rate,
		transcription_mode: effectiveTranscriptionMode(mode, settings.transcription_mode),
		streaming_pcm_format: effectiveStreamingPcmFormat(mode, settings.streaming_pcm_format),
		english_only: settings.english_only,
		copy_to_clipboard: settings.copy_to_clipboard,
		paste_method: settings.paste_method,
		paste_delay_ms: settings.paste_delay_ms,
		hotkey_shortcut: settings.hotkey_shortcut,
		hotkey_mode: settings.hotkey_mode,
		close_to_tray: settings.close_to_tray
	};
}
