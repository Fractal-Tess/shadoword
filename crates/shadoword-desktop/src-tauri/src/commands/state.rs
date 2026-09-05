use super::*;

#[tauri::command]
#[specta::specta]
pub fn load_desktop_state(
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<DesktopBootstrap> {
    let config = state.config()?;
    let (input_devices, input_devices_error) = match MicrophoneRecorder::list_input_devices() {
        Ok(devices) => (devices, None),
        Err(error) => (Vec::new(), Some(error.to_string())),
    };
    let local_overview = if config.mode == ServiceMode::Local {
        local_overview(&state).ok()
    } else {
        None
    };
    let hotkey_error = state
        .hotkey_error
        .lock()
        .map_err(|_| internal_error("hotkey error lock poisoned"))?
        .clone();
    #[cfg(feature = "local-runtime")]
    let local_startup_error = state
        .local_startup_error
        .lock()
        .map_err(|_| internal_error("local startup error lock poisoned"))?
        .clone()
        .map(|message| {
            DesktopError::new("model_preload_failed", message)
                .with_action("Download or select an installed model, then preload again.")
        });
    #[cfg(not(feature = "local-runtime"))]
    let local_startup_error = None;
    Ok(DesktopBootstrap {
        settings: DesktopSettings::from_config(&config),
        input_devices,
        input_devices_error,
        recording: state.recording_state()?,
        local_overview,
        local_startup_error,
        hotkey_error,
    })
}

#[tauri::command]
#[specta::specta]
pub fn get_recording_state(state: tauri::State<'_, DesktopState>) -> CommandResult<RecordingState> {
    state.recording_state()
}

#[tauri::command]
#[specta::specta]
pub fn list_input_devices() -> CommandResult<Vec<shadoword_core::InputDeviceInfo>> {
    MicrophoneRecorder::list_input_devices().map_err(|error| {
        DesktopError::new("input_devices_unavailable", error.to_string())
            .with_action("Check microphone permissions and audio service availability.")
    })
}

#[tauri::command]
#[specta::specta]
pub fn poll_microphone_level(
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<MicrophoneLevel> {
    state.poll_microphone_level()
}

#[tauri::command]
#[specta::specta]
pub fn stop_microphone_level_monitor(state: tauri::State<'_, DesktopState>) -> CommandResult<()> {
    state.stop_microphone_level_monitor()
}

#[tauri::command]
#[specta::specta]
pub fn load_history(state: tauri::State<'_, DesktopState>) -> CommandResult<Vec<HistoryEntry>> {
    state.history.entries().map_err(internal_error_from)
}

#[tauri::command]
#[specta::specta]
pub fn save_history(
    entries: Vec<HistoryEntry>,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<Vec<HistoryEntry>> {
    state.history.replace(entries).map_err(internal_error_from)
}

#[tauri::command]
#[specta::specta]
pub async fn save_desktop_settings(
    input: DesktopSettingsInput,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<DesktopSettings> {
    let _mutation = state.mutation.lock().await;
    state.ensure_idle("change desktop settings")?;
    let current = state.config()?;
    let mut next = current.clone();
    apply_settings(&mut next, input).map_err(config_error)?;
    #[cfg(not(feature = "local-runtime"))]
    if next.mode == ServiceMode::Local {
        return unavailable_local();
    }
    #[cfg(feature = "local-runtime")]
    if current.mode == ServiceMode::Local || next.mode == ServiceMode::Local {
        state.ensure_local_runtime_mutable("change desktop settings")?;
    }
    let shortcut_changed = current.hotkey.shortcut != next.hotkey.shortcut;
    if shortcut_changed {
        register_hotkey(&state, &next.hotkey.shortcut)?;
    }
    let saved = next.clone();
    if let Err(error) =
        persist_local_service(&state, &current, &next, None, move || saved.save()).await
    {
        if shortcut_changed {
            let _ = register_hotkey(&state, &current.hotkey.shortcut);
        }
        return Err(error);
    }
    *state
        .config
        .lock()
        .map_err(|_| internal_error("desktop config lock poisoned"))? = next.clone();
    if current.recording.input_device != next.recording.input_device {
        state.stop_microphone_level_monitor()?;
    }
    Ok(DesktopSettings::from_config(&next))
}

#[tauri::command]
#[specta::specta]
pub fn reveal_desktop_secret(
    kind: DesktopSecretKind,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<String> {
    configured_secret(&state.config()?, kind)
}

#[tauri::command]
#[specta::specta]
pub fn copy_desktop_secret(
    kind: DesktopSecretKind,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<()> {
    let secret = configured_secret(&state.config()?, kind)?;
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.set_text(secret))
        .map_err(|error| {
            DesktopError::new(
                "clipboard_unavailable",
                "could not copy the saved credential",
            )
            .with_action(error.to_string())
        })
}
