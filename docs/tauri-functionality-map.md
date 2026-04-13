# Tauri Functionality Map

This document captures the feature surface of the removed Tauri/React app so the `egui` desktop client can absorb it deliberately instead of losing behavior by accident.

## Scope

Code audited for this map:

- `src-tauri/src/lib.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/commands/*.rs`
- `src-tauri/src/shortcut/*.rs`
- `src-tauri/src/tray.rs`
- `src/App.tsx`
- `src/hooks/useSettings.ts`
- `src/stores/settingsStore.ts`

## Core Product Flows

### 1. App startup and lifecycle

- Single-instance desktop app with remote-control relaunch flags.
- Optional hidden startup.
- Optional tray-less mode.
- Settings window can be reopened from tray.
- Main window is forced visible when permissions need attention.
- Debug logging can be elevated via CLI and runtime settings.

### 2. Onboarding

- First-run onboarding checks whether any models are already available.
- Returning users skip model selection if they already have a downloaded model.
- macOS onboarding checks accessibility and microphone permissions.
- Windows onboarding checks desktop microphone privacy status.
- Post-onboarding initialization performs delayed `Enigo` init and shortcut registration.

### 3. Recording and transcription

- Global shortcut driven recording.
- Push-to-talk mode.
- Always-on microphone mode.
- Manual cancel of active recording/transcription.
- Recording pipeline feeds model inference and then post-processing/output.
- Translation-to-English toggle.
- Selected language control.
- Extra recording buffer setting.
- Lazy stream close toggle.

### 4. Output delivery

- Copy transcript to clipboard.
- Type transcript into the active window.
- Multiple paste methods:
  - `ctrl_v`
  - `direct`
  - `none`
  - `shift_insert`
  - `ctrl_shift_v`
  - `external_script`
- Typing tool selection.
- External script path for paste/output integration.
- Clipboard handling mode.
- Auto-submit toggle plus configurable submit key.
- Append trailing space toggle.
- Paste delay setting.
- Paste error event surfaced to the UI.

### 5. Model management

- Enumerate models.
- Inspect one model.
- Download model.
- Cancel download.
- Delete model.
- Select active model.
- Keep active model persisted in settings.
- Manual unload.
- Configurable unload timeout.
- Emit model state/download failure events back to UI.

### 6. History

- Paginated history listing.
- Save/unsave entries.
- Delete entries.
- Resolve recording file paths.
- Retry transcription from saved audio.
- History retention limit.
- Recording retention policy cleanup.

### 7. Audio devices and feedback

- Enumerate input devices.
- Enumerate output devices.
- Select active microphone.
- Select clamshell microphone.
- Select output device.
- Toggle audio feedback.
- Audio feedback volume.
- Sound theme selection.
- Detect custom start/stop sounds.
- Play test sounds.

### 8. Shortcuts and input backends

- Editable shortcut bindings with reset/suspend/resume.
- Cancel shortcut.
- Shortcut validation.
- Two keyboard backends:
  - Tauri/global shortcut path
  - Handy Keys path
- Delayed initialization after permissions.

### 9. Tray and window chrome

- Tray icon state machine:
  - idle
  - recording
  - transcribing
- Theme-aware tray icons.
- Tray menu actions:
  - open settings
  - check updates
  - copy last transcript
  - choose active model
  - unload model
  - cancel active work
  - quit
- Tray visibility toggle.
- Optional overlay position:
  - none
  - top
  - bottom

### 10. Updates, launch, and portability

- Auto-start toggle.
- Update-check toggle.
- Manual check-for-updates trigger from tray/UI.
- Portable-mode detection.
- Open app-data folder.
- Open logs folder.
- Open recordings folder.

### 11. Post-processing / LLM features

- Enable/disable post-processing.
- Provider selection.
- Provider base URL.
- Provider API key.
- Provider model selection.
- Fetch remote model lists.
- Prompt CRUD:
  - add
  - update
  - delete
  - choose active prompt
- Structured output capability flag per provider.
- Apple Intelligence availability check on supported macOS targets.

### 12. Settings surface

Persisted `AppSettings` fields found in `src-tauri/src/settings.rs`:

- bindings
- push_to_talk
- audio_feedback
- audio_feedback_volume
- sound_theme
- start_hidden
- autostart_enabled
- update_checks_enabled
- selected_model
- always_on_microphone
- selected_microphone
- clamshell_microphone
- selected_output_device
- translate_to_english
- selected_language
- overlay_position
- debug_mode
- log_level
- custom_words
- model_unload_timeout
- word_correction_threshold
- history_limit
- recording_retention_period
- paste_method
- clipboard_handling
- auto_submit
- auto_submit_key
- post_process_enabled
- post_process_provider_id
- post_process_providers
- post_process_api_keys
- post_process_models
- post_process_prompts
- post_process_selected_prompt_id
- mute_while_recording
- append_trailing_space
- app_language
- experimental_enabled
- lazy_stream_close
- keyboard_implementation
- show_tray_icon
- paste_delay_ms
- typing_tool
- external_script_path
- custom_filler_words
- whisper_accelerator
- ort_accelerator
- whisper_gpu_device
- extra_recording_buffer_ms

## Present `egui` Desktop Coverage

The current `crates/shadowword-desktop` app already covers a narrower but useful subset:

- local vs remote mode
- input device selection
- sample-rate selection
- copy/type output toggles
- engine selection
- ORT accelerator selection
- ONNX quantization selection
- Whisper accelerator selection
- model path selection
- daemon endpoint configuration
- local config persistence
- remote status pull
- remote config pull/push
- local recording start/stop
- local or remote transcription
- clipboard/type output application

## Migration Gaps

The biggest Tauri features not yet represented in `egui` are:

- onboarding and permission UX
- model catalog/download management
- history browser and retry flow
- tray integration
- global shortcut editor
- overlay window
- post-processing provider/prompt management
- update/autostart controls
- expanded settings parity
- localization/i18n

## Migration Order Recommendation

1. Keep the Rust audio/transcription/core paths stable.
2. Rebuild settings management in the `egui` client around `ShadowwordConfig` or a richer Rust-native settings type.
3. Add model management next, because onboarding and steady-state use both depend on it.
4. Add tray + shortcuts after recording/transcription UX is stable.
5. Reintroduce history and post-processing after the main local workflow is complete.
