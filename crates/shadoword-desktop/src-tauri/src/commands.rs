use crate::contracts::{
    ConnectionInput, ConnectionReport, DesktopBootstrap, DesktopError, DesktopEvent,
    DesktopSecretKind, DesktopSettings, DesktopSettingsInput, OpenRouterConnectionInput,
    OpenRouterKeyReport, OpenRouterModelInfo, RecordingPhase, RecordingState, RecordingStatus,
    SecretUpdate, TranscriptionResult, DESKTOP_EVENT_NAME,
};
use crate::hotkeys::{validate_shortcut, HotkeyBackend, HotkeyEventState};
use crate::openrouter::OpenRouterClient;
use crate::recording::{
    emit_error, spawn_streaming_worker, StreamCommand, StreamingWorker, TranscriptionTarget,
};
use crate::remote::RemoteClient;
use anyhow::{anyhow, Context, Result};
#[cfg(feature = "local-runtime")]
use shadoword_core::remote_contracts::{
    DaemonStatusDto, DownloadJobState, ModelInfoDto, ModelStorageDto,
};
use shadoword_core::remote_contracts::{DownloadJobStatus, OverviewDto, RuntimeConfigDto};
#[cfg(feature = "local-runtime")]
use shadoword_core::{
    default_whisper_model, download_whisper_model_with_progress, list_whisper_gpu_devices,
    list_whisper_models, resolve_whisper_model, unknown_model_error, InferencePoolConfig,
    InferenceRuntime, TranscriptionService, WhisperModelFactory,
};
use shadoword_core::{
    DesktopConfig, HotkeyMode, MicrophoneRecorder, ModeRecordingPreferences, RecordingSession,
    ServiceMode, StreamingPcmFormat, TranscriptionConfig, TranscriptionMode,
};
use std::collections::HashMap;
use std::path::PathBuf;
#[cfg(feature = "local-runtime")]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
#[cfg(feature = "local-runtime")]
use std::time::Duration;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};

type CommandResult<T> = std::result::Result<T, DesktopError>;

enum ActiveKind {
    Batch,
    Streaming(StreamingWorker),
}

struct ActiveRecording {
    session: RecordingSession,
    started_at: Instant,
    sample_rate: u32,
    config: DesktopConfig,
    kind: ActiveKind,
}

#[derive(Default)]
struct RecordingController {
    state: RecordingState,
    active: Option<ActiveRecording>,
    hotkey_down: bool,
}

pub struct DesktopState {
    config: Mutex<DesktopConfig>,
    recording: Mutex<RecordingController>,
    mutation: tokio::sync::Mutex<()>,
    remote: RemoteClient,
    openrouter: OpenRouterClient,
    #[cfg(feature = "local-runtime")]
    local: Arc<InferenceRuntime>,
    #[cfg(feature = "local-runtime")]
    local_startup_error: Mutex<Option<String>>,
    local_downloads: Arc<Mutex<HashMap<String, DownloadJobStatus>>>,
    #[cfg(feature = "local-runtime")]
    next_download_id: AtomicU64,
    hotkey: Mutex<Option<HotkeyBackend>>,
    hotkey_error: Mutex<Option<String>>,
}

impl DesktopState {
    pub fn load() -> Result<Self> {
        let config = normalize_config_for_build(
            DesktopConfig::load().context("failed to load desktop config")?,
        );
        #[cfg(feature = "local-runtime")]
        let (local, local_startup_error) = initialize_local_runtime(&config)?;
        Ok(Self {
            config: Mutex::new(config),
            recording: Mutex::new(RecordingController::default()),
            mutation: tokio::sync::Mutex::new(()),
            remote: RemoteClient::new()?,
            openrouter: OpenRouterClient::new()?,
            #[cfg(feature = "local-runtime")]
            local,
            #[cfg(feature = "local-runtime")]
            local_startup_error: Mutex::new(local_startup_error),
            local_downloads: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "local-runtime")]
            next_download_id: AtomicU64::new(1),
            hotkey: Mutex::new(None),
            hotkey_error: Mutex::new(None),
        })
    }

    fn config(&self) -> CommandResult<DesktopConfig> {
        self.config
            .lock()
            .map_err(|_| internal_error("desktop config lock poisoned"))
            .map(|config| config.clone())
    }

    fn recording_state(&self) -> CommandResult<RecordingState> {
        self.recording
            .lock()
            .map_err(|_| internal_error("recording state lock poisoned"))
            .map(|recording| recording.state.clone())
    }

    fn ensure_idle(&self, operation: &str) -> CommandResult<()> {
        let state = self.recording_state()?;
        if state.phase == RecordingPhase::Idle {
            Ok(())
        } else {
            Err(DesktopError::new(
                "recording_busy",
                format!(
                    "cannot {operation} while recording state is {:?}",
                    state.phase
                ),
            )
            .with_action("Stop or cancel the active recording first."))
        }
    }

    #[cfg(feature = "local-runtime")]
    fn ensure_local_runtime_mutable(&self, operation: &str) -> CommandResult<()> {
        self.ensure_idle(operation)?;
        let status = self.local.status();
        if status.queued_jobs == 0
            && status.running_jobs == 0
            && status.draining_generations.is_empty()
        {
            return Ok(());
        }
        Err(DesktopError::new(
            "local_runtime_busy",
            format!("cannot {operation} while local inference work is active or draining"),
        )
        .with_action("Wait for all local inference units and draining generations to become idle."))
    }
}

#[cfg(feature = "local-runtime")]
fn initialize_local_runtime(
    config: &DesktopConfig,
) -> Result<(Arc<InferenceRuntime>, Option<String>)> {
    let mut desired = local_transcription_config(config);
    if config.mode != ServiceMode::Local {
        desired.preload_on_startup = false;
    }
    match InferenceRuntime::new_with_factory(desired.clone(), Arc::new(WhisperModelFactory)) {
        Ok(runtime) => Ok((Arc::new(runtime), None)),
        Err(error) if desired.preload_on_startup => {
            let message = error.to_string();
            desired.preload_on_startup = false;
            let runtime =
                InferenceRuntime::new_with_factory(desired, Arc::new(WhisperModelFactory))
                    .context(
                        "failed to initialize lazy local runtime after eager preload failed",
                    )?;
            Ok((Arc::new(runtime), Some(message)))
        }
        Err(error) => Err(error),
    }
}

#[cfg(feature = "local-runtime")]
fn normalize_config_for_build(mut config: DesktopConfig) -> DesktopConfig {
    normalize_mode_scoped_recording(&mut config);
    config
}

#[cfg(not(feature = "local-runtime"))]
fn normalize_config_for_build(mut config: DesktopConfig) -> DesktopConfig {
    if config.mode == ServiceMode::Local {
        config.mode = ServiceMode::Remote;
    }
    normalize_mode_scoped_recording(&mut config);
    config
}

fn normalize_mode_scoped_recording(config: &mut DesktopConfig) {
    let current = ModeRecordingPreferences::from(&config.recording);
    config
        .local_recording
        .get_or_insert_with(|| current.clone());
    config
        .remote_recording
        .get_or_insert_with(|| current.clone());
    config
        .openrouter_recording
        .get_or_insert_with(|| current.clone());

    let preferences = match config.mode {
        ServiceMode::Local => config.local_recording.as_ref(),
        ServiceMode::Remote => config.remote_recording.as_ref(),
        ServiceMode::OpenRouter => config.openrouter_recording.as_ref(),
    }
    .expect("mode recording preferences initialized above");
    config.recording.transcription_mode = preferences.transcription_mode;
    config.recording.streaming_pcm_format = preferences.streaming_pcm_format;
    config.recording.english_only = preferences.english_only;
    config.recording.sample_rate = 16_000;

    match config.mode {
        ServiceMode::Local => config.recording.streaming_pcm_format = StreamingPcmFormat::F32le,
        ServiceMode::OpenRouter => {
            config.recording.transcription_mode = TranscriptionMode::Batch;
            config.recording.streaming_pcm_format = StreamingPcmFormat::F32le;
        }
        ServiceMode::Remote => {}
    }
}

fn store_mode_recording(config: &mut DesktopConfig, mode: ServiceMode) {
    let preferences = Some(ModeRecordingPreferences::from(&config.recording));
    match mode {
        ServiceMode::Local => config.local_recording = preferences,
        ServiceMode::Remote => config.remote_recording = preferences,
        ServiceMode::OpenRouter => config.openrouter_recording = preferences,
    }
}

impl DesktopSettings {
    fn from_config(config: &DesktopConfig) -> Self {
        Self {
            mode: config.mode,
            model_path: config.model_path.to_string_lossy().into_owned(),
            preload_on_startup: config.preload_on_startup,
            whisper_accelerator: config.whisper_accelerator,
            whisper_gpu_device: config.whisper_gpu_device,
            remote_endpoint: config.remote.endpoint.clone(),
            remote_token_configured: config
                .remote
                .api_token
                .as_deref()
                .is_some_and(|token| !token.trim().is_empty()),
            openrouter_model: config.openrouter.model.clone(),
            openrouter_key_configured: config
                .openrouter
                .api_key
                .as_deref()
                .is_some_and(|key| !key.trim().is_empty()),
            input_device: config.recording.input_device.clone(),
            sample_rate: config.recording.sample_rate,
            transcription_mode: config.recording.transcription_mode,
            streaming_pcm_format: config.recording.streaming_pcm_format,
            english_only: config.recording.english_only,
            copy_to_clipboard: config.output.copy_to_clipboard,
            paste_method: config.output.paste_method,
            paste_delay_ms: config.output.paste_delay_ms,
            output_prefix: config.output.prefix,
            output_suffix: config.output.suffix,
            hotkey_shortcut: config.hotkey.shortcut.clone(),
            hotkey_mode: config.hotkey.mode,
            close_to_tray: config.close_to_tray,
            show_window_title_bar: config.show_window_title_bar,
        }
    }
}

pub fn setup_native(app: &AppHandle) {
    let state = app.state::<DesktopState>();
    let config = match state.config() {
        Ok(config) => config,
        Err(error) => {
            emit_error(app, "startup", &error.code, error.message, error.action);
            return;
        }
    };
    match HotkeyBackend::new(app.clone()).and_then(|backend| {
        backend.register(&config.hotkey.shortcut)?;
        Ok(backend)
    }) {
        Ok(backend) => {
            if let Ok(mut hotkey) = state.hotkey.lock() {
                *hotkey = Some(backend);
            }
        }
        Err(error) => {
            tracing::warn!(%error, "global shortcut unavailable");
            if let Ok(mut hotkey_error) = state.hotkey_error.lock() {
                *hotkey_error = Some(error.clone());
            }
            emit_error(
                app,
                "hotkey",
                "hotkey_unavailable",
                error,
                Some("Choose another shortcut or check desktop portal permissions.".to_string()),
            );
        }
    }

    #[cfg(feature = "local-runtime")]
    if config.mode == ServiceMode::Local && config.preload_on_startup {
        let startup_error = state
            .local_startup_error
            .lock()
            .ok()
            .and_then(|error| error.clone());
        if let Some(error) = startup_error {
            emit_error(
                app,
                "local_model",
                "model_preload_failed",
                error,
                Some("Download or select an installed model, then preload again.".to_string()),
            );
        } else {
            let _ = app.emit(
                DESKTOP_EVENT_NAME,
                DesktopEvent::Status {
                    message: "Local model ready".to_string(),
                },
            );
        }
    }
}

pub fn shutdown(app: &AppHandle) {
    let state = app.state::<DesktopState>();
    let _ = cancel_recording_inner(app, &state);
    crate::tray::set_icon_for_phase(app, RecordingPhase::Idle);
    #[cfg(feature = "local-runtime")]
    state.local.begin_shutdown();
    if let Ok(mut hotkey) = state.hotkey.lock() {
        *hotkey = None;
    };
}

pub fn close_to_tray(state: &DesktopState) -> bool {
    state
        .config()
        .map(|config| config.close_to_tray)
        .unwrap_or(false)
}

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

#[tauri::command]
#[specta::specta]
pub async fn test_remote_connection(
    input: ConnectionInput,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<ConnectionReport> {
    let saved_token = state.config()?.remote.api_token;
    let token = if input.use_saved_token {
        saved_token
    } else {
        input.token
    };
    let endpoint = RemoteClient::validate_endpoint(&input.endpoint).map_err(config_error)?;
    let token = token.as_deref();
    let health = state
        .remote
        .health(&endpoint, token)
        .await
        .map_err(remote_error)?;
    let status = state
        .remote
        .status(&endpoint, token)
        .await
        .map_err(remote_error)?;
    let overview = state
        .remote
        .overview(&endpoint, token)
        .await
        .map_err(remote_error)?;
    let runtime_config = state
        .remote
        .runtime_config(&endpoint, token)
        .await
        .map_err(remote_error)?;
    Ok(ConnectionReport {
        health_ok: health.ok,
        status_model_loaded: status.service.model_loaded,
        overview,
        runtime_config,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn list_openrouter_models(
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<Vec<OpenRouterModelInfo>> {
    state
        .openrouter
        .list_transcription_models()
        .await
        .map(|models| {
            models
                .into_iter()
                .map(|model| OpenRouterModelInfo {
                    id: model.id,
                    name: model.name,
                    description: model.description,
                })
                .collect()
        })
        .map_err(openrouter_error)
}

#[tauri::command]
#[specta::specta]
pub async fn test_openrouter_key(
    input: OpenRouterConnectionInput,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<OpenRouterKeyReport> {
    let saved_key = state.config()?.openrouter.api_key;
    let key = if input.use_saved_key {
        saved_key.as_deref().ok_or_else(openrouter_key_required)?
    } else {
        input.key.as_deref().ok_or_else(openrouter_key_required)?
    };
    let report = state
        .openrouter
        .test_api_key(key)
        .await
        .map_err(openrouter_error)?;
    Ok(OpenRouterKeyReport {
        label: report.label,
        is_free_tier: report.is_free_tier,
        limit: report.limit,
        limit_remaining: report.limit_remaining,
        usage: report.usage,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn refresh_remote_overview(
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<OverviewDto> {
    let config = state.config()?;
    ensure_mode(&config, ServiceMode::Remote)?;
    state
        .remote
        .overview(&config.remote.endpoint, config.remote.api_token.as_deref())
        .await
        .map_err(remote_error)
}

#[tauri::command]
#[specta::specta]
pub async fn update_remote_runtime(
    runtime: RuntimeConfigDto,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<OverviewDto> {
    let _mutation = state.mutation.lock().await;
    state.ensure_idle("change remote runtime settings")?;
    let config = state.config()?;
    ensure_mode(&config, ServiceMode::Remote)?;
    state
        .remote
        .update_runtime(
            &config.remote.endpoint,
            config.remote.api_token.as_deref(),
            &runtime,
        )
        .await
        .map_err(remote_error)?;
    refresh_remote(&state.remote, &config).await
}

#[tauri::command]
#[specta::specta]
pub async fn select_remote_model(
    model_id: String,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<OverviewDto> {
    let _mutation = state.mutation.lock().await;
    state.ensure_idle("select a remote model")?;
    let config = state.config()?;
    ensure_mode(&config, ServiceMode::Remote)?;
    state
        .remote
        .select_model(
            &config.remote.endpoint,
            config.remote.api_token.as_deref(),
            &model_id,
        )
        .await
        .map_err(remote_error)?;
    refresh_remote(&state.remote, &config).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_remote_model(
    model_id: String,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<OverviewDto> {
    let _mutation = state.mutation.lock().await;
    state.ensure_idle("delete a remote model")?;
    let config = state.config()?;
    ensure_mode(&config, ServiceMode::Remote)?;
    state
        .remote
        .delete_model(
            &config.remote.endpoint,
            config.remote.api_token.as_deref(),
            &model_id,
        )
        .await
        .map_err(remote_error)?;
    refresh_remote(&state.remote, &config).await
}

#[tauri::command]
#[specta::specta]
pub async fn start_remote_download(
    model_id: String,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<DownloadJobStatus> {
    let config = state.config()?;
    ensure_mode(&config, ServiceMode::Remote)?;
    state
        .remote
        .start_download(
            &config.remote.endpoint,
            config.remote.api_token.as_deref(),
            model_id,
        )
        .await
        .map_err(remote_error)
}

#[tauri::command]
#[specta::specta]
pub async fn poll_remote_download(
    job_id: String,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<DownloadJobStatus> {
    let config = state.config()?;
    ensure_mode(&config, ServiceMode::Remote)?;
    state
        .remote
        .download_status(
            &config.remote.endpoint,
            config.remote.api_token.as_deref(),
            &job_id,
        )
        .await
        .map_err(remote_error)
}

#[tauri::command]
#[specta::specta]
pub fn refresh_local_overview(state: tauri::State<'_, DesktopState>) -> CommandResult<OverviewDto> {
    let config = state.config()?;
    ensure_mode(&config, ServiceMode::Local)?;
    local_overview(&state)
}

#[tauri::command]
#[specta::specta]
pub async fn preload_local_model(
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<OverviewDto> {
    let _mutation = state.mutation.lock().await;
    #[cfg(feature = "local-runtime")]
    state.ensure_local_runtime_mutable("preload the local model")?;
    #[cfg(not(feature = "local-runtime"))]
    state.ensure_idle("preload the local model")?;
    let config = state.config()?;
    ensure_mode(&config, ServiceMode::Local)?;
    #[cfg(feature = "local-runtime")]
    {
        let runtime = Arc::clone(&state.local);
        let expected_generation = runtime.generation();
        let mut candidate = local_transcription_config(&config);
        candidate.preload_on_startup = true;
        tokio::task::spawn_blocking(move || {
            reload_local_candidate(&runtime, Some(expected_generation), candidate)
        })
        .await
        .map_err(join_error)?
        .map_err(local_error)?;
        if let Ok(mut startup_error) = state.local_startup_error.lock() {
            *startup_error = None;
        }
        local_overview(&state)
    }
    #[cfg(not(feature = "local-runtime"))]
    unavailable_local()
}

#[tauri::command]
#[specta::specta]
pub async fn update_local_runtime(
    runtime: RuntimeConfigDto,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<OverviewDto> {
    let _mutation = state.mutation.lock().await;
    #[cfg(feature = "local-runtime")]
    state.ensure_local_runtime_mutable("change local runtime settings")?;
    #[cfg(not(feature = "local-runtime"))]
    state.ensure_idle("change local runtime settings")?;
    let current = state.config()?;
    ensure_mode(&current, ServiceMode::Local)?;
    validate_runtime(&runtime)?;
    let next = apply_local_runtime_config(&current, runtime.clone());
    let saved = next.clone();
    persist_local_service(&state, &current, &next, runtime.generation, move || {
        saved.save()
    })
    .await?;
    *state
        .config
        .lock()
        .map_err(|_| internal_error("desktop config lock poisoned"))? = next;
    local_overview(&state)
}

fn apply_local_runtime_config(current: &DesktopConfig, runtime: RuntimeConfigDto) -> DesktopConfig {
    let mut next = current.clone();
    next.model_path = PathBuf::from(runtime.model_path.trim());
    next.whisper_accelerator = runtime.whisper_accelerator;
    next.whisper_gpu_device = runtime.whisper_gpu_device;
    next.recording.english_only = runtime.english_only;
    next.preload_on_startup = runtime.preload_on_startup;
    let unchanged_legacy_pool = runtime.inference_pool_explicit.is_none()
        && current.inference_pool.is_none()
        && runtime.inference_pool.as_ref().is_some_and(|pool| {
            local_transcription_config(current)
                .effective_inference_pool()
                .ok()
                .as_ref()
                == Some(pool)
        });
    match runtime.inference_pool_explicit {
        Some(false) => next.inference_pool = None,
        Some(true) => {
            if let Some(pool) = runtime.inference_pool {
                next.inference_pool = Some(pool);
            }
        }
        None => {
            if let Some(pool) = runtime.inference_pool.filter(|_| !unchanged_legacy_pool) {
                next.inference_pool = Some(pool);
            }
        }
    }
    next
}

#[tauri::command]
#[specta::specta]
pub async fn select_local_model(
    model_id: String,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<OverviewDto> {
    let _mutation = state.mutation.lock().await;
    #[cfg(feature = "local-runtime")]
    state.ensure_local_runtime_mutable("select a local model")?;
    #[cfg(not(feature = "local-runtime"))]
    state.ensure_idle("select a local model")?;
    let current = state.config()?;
    ensure_mode(&current, ServiceMode::Local)?;
    #[cfg(feature = "local-runtime")]
    {
        let model = resolve_whisper_model(&model_id)
            .ok_or_else(|| config_error(unknown_model_error(&model_id)))?;
        let path = DesktopConfig::models_dir()
            .map_err(config_error)?
            .join(model.filename);
        if !path.is_file() {
            return Err(DesktopError::new(
                "model_not_installed",
                format!("local model '{}' is not installed", model.id),
            )
            .with_action("Download the model before selecting it."));
        }
        let mut next = current.clone();
        next.model_path = path;
        let saved = next.clone();
        persist_local_service(
            &state,
            &current,
            &next,
            Some(state.local.generation()),
            move || saved.save(),
        )
        .await?;
        *state
            .config
            .lock()
            .map_err(|_| internal_error("desktop config lock poisoned"))? = next;
        local_overview(&state)
    }
    #[cfg(not(feature = "local-runtime"))]
    {
        let _ = model_id;
        unavailable_local()
    }
}

#[tauri::command]
#[specta::specta]
pub async fn delete_local_model(
    model_id: String,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<OverviewDto> {
    let _mutation = state.mutation.lock().await;
    #[cfg(feature = "local-runtime")]
    state.ensure_local_runtime_mutable("delete a local model")?;
    #[cfg(not(feature = "local-runtime"))]
    state.ensure_idle("delete a local model")?;
    let config = state.config()?;
    ensure_mode(&config, ServiceMode::Local)?;
    #[cfg(feature = "local-runtime")]
    {
        let model = resolve_whisper_model(&model_id)
            .ok_or_else(|| config_error(unknown_model_error(&model_id)))?;
        let directory = DesktopConfig::models_dir().map_err(config_error)?;
        let path = directory.join(model.filename);
        let active_path = if config.model_path.as_os_str().is_empty() {
            directory.join(default_whisper_model().filename)
        } else {
            config.model_path.clone()
        };
        if active_path == path {
            return Err(DesktopError::new(
                "model_in_use",
                "select another model before deleting the active model",
            ));
        }
        let download_active = state
            .local_downloads
            .lock()
            .map_err(|_| internal_error("local download lock poisoned"))?
            .values()
            .any(|job| {
                job.model_id == model.id
                    && matches!(
                        job.state,
                        DownloadJobState::Queued | DownloadJobState::Running
                    )
            });
        if download_active {
            return Err(DesktopError::new(
                "model_download_active",
                "stop waiting for the active download before deleting this model",
            ));
        }
        if !path.is_file() {
            return Err(DesktopError::new(
                "model_not_installed",
                format!("local model '{}' is not installed", model.id),
            ));
        }
        tokio::fs::remove_file(&path).await.map_err(local_error)?;
        local_overview(&state)
    }
    #[cfg(not(feature = "local-runtime"))]
    {
        let _ = model_id;
        unavailable_local()
    }
}

#[tauri::command]
#[specta::specta]
pub fn validate_local_inference_pool(
    pool: shadoword_core::InferencePoolConfig,
) -> CommandResult<shadoword_core::InferencePoolConfig> {
    pool.validate().map_err(config_error)?;
    #[cfg(feature = "local-runtime")]
    validate_pool_devices(&pool)?;
    #[cfg(not(feature = "local-runtime"))]
    return unavailable_local();
    #[cfg(feature = "local-runtime")]
    Ok(pool)
}

#[tauri::command]
#[specta::specta]
pub fn start_local_download(
    model_id: String,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<DownloadJobStatus> {
    #[cfg(feature = "local-runtime")]
    state.ensure_local_runtime_mutable("start a local model download")?;
    #[cfg(not(feature = "local-runtime"))]
    state.ensure_idle("start a local model download")?;
    let config = state.config()?;
    ensure_mode(&config, ServiceMode::Local)?;
    #[cfg(feature = "local-runtime")]
    {
        let model = resolve_whisper_model(&model_id)
            .ok_or_else(|| config_error(unknown_model_error(&model_id)))?;
        let job_id = format!(
            "local-{}",
            state.next_download_id.fetch_add(1, Ordering::Relaxed)
        );
        let initial = DownloadJobStatus {
            id: job_id.clone(),
            model_id: model.id.to_string(),
            state: DownloadJobState::Queued,
            downloaded: 0,
            total: model.size_bytes,
            path: None,
            skipped: false,
            verified: false,
            error: None,
        };
        state
            .local_downloads
            .lock()
            .map_err(|_| internal_error("local download lock poisoned"))?
            .insert(job_id.clone(), initial.clone());
        let jobs = Arc::clone(&state.local_downloads);
        let directory = DesktopConfig::models_dir().map_err(config_error)?;
        std::thread::Builder::new()
            .name(format!("shadoword-download-{job_id}"))
            .spawn(move || {
                update_download(&jobs, &job_id, |job| job.state = DownloadJobState::Running);
                let result = download_whisper_model_with_progress(model, &directory, |progress| {
                    update_download(&jobs, &job_id, |job| {
                        job.state = DownloadJobState::Running;
                        job.downloaded = progress.downloaded;
                        job.total = progress.total;
                    });
                });
                match result {
                    Ok(status) => update_download(&jobs, &job_id, |job| {
                        job.state = DownloadJobState::Succeeded;
                        job.downloaded = job.total;
                        job.path = Some(status.path.to_string_lossy().into_owned());
                        job.skipped = status.skipped;
                        job.verified = status.verified;
                    }),
                    Err(error) => update_download(&jobs, &job_id, |job| {
                        job.state = DownloadJobState::Failed;
                        job.error = Some(error.to_string());
                    }),
                }
            })
            .map_err(|error| DesktopError::new("download_start_failed", error.to_string()))?;
        Ok(initial)
    }
    #[cfg(not(feature = "local-runtime"))]
    {
        let _ = model_id;
        unavailable_local()
    }
}

#[tauri::command]
#[specta::specta]
pub fn poll_local_download(
    job_id: String,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<DownloadJobStatus> {
    state
        .local_downloads
        .lock()
        .map_err(|_| internal_error("local download lock poisoned"))?
        .get(&job_id)
        .cloned()
        .ok_or_else(|| DesktopError::new("download_not_found", format!("unknown job '{job_id}'")))
}

#[tauri::command]
#[specta::specta]
pub async fn start_recording(
    app: AppHandle,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<RecordingStatus> {
    start_recording_inner(&app, &state).await
}

#[tauri::command]
#[specta::specta]
pub fn cancel_recording(
    app: AppHandle,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<()> {
    cancel_recording_inner(&app, &state)
}

#[tauri::command]
#[specta::specta]
pub async fn stop_and_transcribe(
    app: AppHandle,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<TranscriptionResult> {
    finish_recording_inner(&app, &state).await
}

async fn start_recording_inner(
    app: &AppHandle,
    state: &DesktopState,
) -> CommandResult<RecordingStatus> {
    let _mutation = state.mutation.lock().await;
    let config = state.config()?;
    if config.mode == ServiceMode::OpenRouter {
        validate_openrouter_config(&config)?;
    }
    let session = {
        let recording = state
            .recording
            .lock()
            .map_err(|_| internal_error("recording state lock poisoned"))?;
        if recording.state.phase != RecordingPhase::Idle {
            return Err(busy_recording_error());
        }
        drop(recording);
        MicrophoneRecorder::start(config.recording.input_device.as_deref()).map_err(|error| {
            DesktopError::new("microphone_start_failed", error.to_string())
                .with_action("Check the selected input device and microphone permissions.")
        })?
    };
    let started_at = Instant::now();
    let sample_rate = session.snapshot_source().sample_rate();
    let effective_transcription_mode = if config.mode == ServiceMode::OpenRouter {
        TranscriptionMode::Batch
    } else {
        config.recording.transcription_mode
    };
    let target = if effective_transcription_mode == TranscriptionMode::Streaming {
        Some(transcription_target(state, &config)?)
    } else {
        None
    };
    let mut recording = state
        .recording
        .lock()
        .map_err(|_| internal_error("recording state lock poisoned"))?;
    if recording.state.phase != RecordingPhase::Idle {
        session.stop_without_snapshot();
        return Err(busy_recording_error());
    }
    recording.state = RecordingState {
        phase: RecordingPhase::Recording,
        service_mode: Some(config.mode),
        transcription_mode: Some(effective_transcription_mode),
        sample_rate: Some(sample_rate),
        segment_count: 0,
    };
    let kind = if let Some(target) = target {
        let source = session.snapshot_source();
        ActiveKind::Streaming(spawn_streaming_worker(
            app.clone(),
            config.clone(),
            target,
            source,
            started_at,
        ))
    } else {
        ActiveKind::Batch
    };
    recording.active = Some(ActiveRecording {
        session,
        started_at,
        sample_rate,
        config,
        kind,
    });
    crate::tray::set_icon_for_phase(app, RecordingPhase::Recording);
    drop(recording);
    let _ = app.emit(
        DESKTOP_EVENT_NAME,
        DesktopEvent::RecordingStarted { sample_rate },
    );
    Ok(RecordingStatus {
        recording: true,
        sample_rate,
    })
}

async fn finish_recording_inner(
    app: &AppHandle,
    state: &DesktopState,
) -> CommandResult<TranscriptionResult> {
    let active = {
        let mut recording = state
            .recording
            .lock()
            .map_err(|_| internal_error("recording state lock poisoned"))?;
        if recording.state.phase != RecordingPhase::Recording {
            return Err(DesktopError::new(
                "no_active_recording",
                "no recording is active",
            ));
        }
        let Some(active) = recording.active.take() else {
            recording.state = RecordingState::default();
            crate::tray::set_icon_for_phase(app, RecordingPhase::Idle);
            return Err(internal_error("recording state had no active session"));
        };
        recording.state.phase = RecordingPhase::Finalizing;
        crate::tray::set_icon_for_phase(app, RecordingPhase::Finalizing);
        active
    };
    let _ = app.emit(
        DESKTOP_EVENT_NAME,
        DesktopEvent::RecordingStopped { processing: true },
    );
    let ActiveRecording {
        session,
        started_at,
        sample_rate,
        config,
        kind,
    } = active;
    let (result, event_emitted) = match kind {
        ActiveKind::Batch => (
            finish_batch(
                app,
                state,
                ActiveRecording {
                    session,
                    started_at,
                    sample_rate,
                    config,
                    kind: ActiveKind::Batch,
                },
            )
            .await,
            false,
        ),
        ActiveKind::Streaming(worker) => {
            session.stop_without_snapshot();
            let result = if worker.command_tx.send(StreamCommand::Finish).is_err() {
                Err(DesktopError::new(
                    "stream_worker_stopped",
                    "streaming worker stopped before finalization",
                ))
            } else {
                match worker.handle.await {
                    Ok(result) => result,
                    Err(error) => Err(join_error(error)),
                }
            };
            (result, true)
        }
    };
    reset_recording_state(state);
    if !event_emitted {
        if let Ok(result) = &result {
            let _ = app.emit(
                DESKTOP_EVENT_NAME,
                DesktopEvent::TranscriptionComplete {
                    result: result.clone(),
                    segments: 1,
                },
            );
        }
    }
    if let Err(error) = &result {
        emit_error(
            app,
            "transcription",
            &error.code,
            error.message.clone(),
            error.action.clone(),
        );
    }
    result
}

async fn finish_batch(
    app: &AppHandle,
    state: &DesktopState,
    active: ActiveRecording,
) -> CommandResult<TranscriptionResult> {
    let duration = active.started_at.elapsed();
    let audio = active
        .session
        .stop()
        .map_err(|error| DesktopError::new("microphone_stop_failed", error.to_string()))?;
    if audio.samples.is_empty() {
        return Err(DesktopError::new(
            "empty_recording",
            "the microphone recording did not contain any samples",
        ));
    }
    let mut cost_usd = None;
    let response = match active.config.mode {
        ServiceMode::Local => {
            #[cfg(feature = "local-runtime")]
            {
                let runtime = Arc::clone(&state.local);
                let job = runtime.submit_batch(audio).map_err(local_error)?;
                tokio::task::spawn_blocking(move || {
                    match job.wait_timeout(Duration::from_secs(5 * 60)) {
                        Ok(Some(completion)) => Ok(completion.response),
                        Ok(None) => {
                            job.cancel();
                            Err(anyhow!("local batch inference timed out"))
                        }
                        Err(error) => Err(anyhow::Error::new(error)),
                    }
                })
                .await
                .map_err(join_error)?
                .map_err(local_error)?
            }
            #[cfg(not(feature = "local-runtime"))]
            return unavailable_local();
        }
        ServiceMode::Remote => {
            let wav = shadoword_core::wav::encode_wav(&audio).map_err(local_error)?;
            state
                .remote
                .transcribe_wav(
                    &active.config.remote.endpoint,
                    active.config.remote.api_token.as_deref(),
                    wav,
                )
                .await
                .map_err(remote_error)?
        }
        ServiceMode::OpenRouter => {
            let wav = shadoword_core::wav::encode_wav(&audio).map_err(local_error)?;
            let api_key = active
                .config
                .openrouter
                .api_key
                .as_deref()
                .ok_or_else(openrouter_key_required)?;
            let openrouter_response = state
                .openrouter
                .transcribe_wav(
                    api_key,
                    &active.config.openrouter.model,
                    wav,
                    active.config.recording.english_only,
                )
                .await
                .map_err(openrouter_error)?;
            cost_usd = openrouter_response.cost_usd;
            shadoword_core::TranscriptResponse {
                text: openrouter_response.text,
                elapsed_ms: openrouter_response.elapsed_ms,
                engine: format!("OpenRouter · {}", active.config.openrouter.model),
            }
        }
    };
    let result = TranscriptionResult {
        text: response.text,
        elapsed_ms: u64::try_from(response.elapsed_ms).unwrap_or(u64::MAX),
        engine: response.engine,
        audio_duration_ms: u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
        sample_rate: active.sample_rate,
        cost_usd,
    };
    let output = active.config.output;
    let text = result.text.clone();
    match tokio::task::spawn_blocking(move || crate::output::apply_output(&output, &text)).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => emit_error(
            app,
            "output",
            "output_delivery_failed",
            error.to_string(),
            Some("Check clipboard access and the selected paste method.".to_string()),
        ),
        Err(error) => emit_error(app, "output", "output_task_failed", error.to_string(), None),
    }
    Ok(result)
}

fn cancel_recording_inner(app: &AppHandle, state: &DesktopState) -> CommandResult<()> {
    let active = {
        let mut recording = state
            .recording
            .lock()
            .map_err(|_| internal_error("recording state lock poisoned"))?;
        if recording.state.phase == RecordingPhase::Finalizing {
            return Err(DesktopError::new(
                "transcription_finalizing",
                "the stopped recording is already being finalized",
            )
            .with_action("Wait for the transcription result or error event."));
        }
        if recording.state.phase == RecordingPhase::Idle {
            crate::tray::set_icon_for_phase(app, RecordingPhase::Idle);
            return Ok(());
        }
        recording.state = RecordingState::default();
        crate::tray::set_icon_for_phase(app, RecordingPhase::Idle);
        recording.active.take()
    };
    if let Some(active) = active {
        active.session.stop_without_snapshot();
        if let ActiveKind::Streaming(worker) = active.kind {
            worker.cancelled.store(true, Ordering::Release);
            let _ = worker.command_tx.send(StreamCommand::Cancel);
        }
        let _ = app.emit(DESKTOP_EVENT_NAME, DesktopEvent::RecordingCancelled);
    }
    Ok(())
}

pub fn stream_worker_failed(app: &AppHandle, error: &DesktopError) {
    let state = app.state::<DesktopState>();
    let active = {
        let Ok(mut recording) = state.recording.lock() else {
            crate::tray::set_icon_for_phase(app, RecordingPhase::Idle);
            return;
        };
        if recording.state.phase != RecordingPhase::Recording {
            return;
        }
        recording.state = RecordingState::default();
        crate::tray::set_icon_for_phase(app, RecordingPhase::Idle);
        recording.active.take()
    };
    if let Some(active) = active {
        active.session.stop_without_snapshot();
    }
    emit_error(
        app,
        "streaming",
        &error.code,
        error.message.clone(),
        error.action.clone(),
    );
    let _ = app.emit(
        DESKTOP_EVENT_NAME,
        DesktopEvent::RecordingStopped { processing: false },
    );
}

fn reset_recording_state(state: &DesktopState) {
    if let Ok(mut recording) = state.recording.lock() {
        recording.state = RecordingState::default();
        recording.active = None;
    }
}

pub fn increment_stream_segment(app: &AppHandle) {
    let state = app.state::<DesktopState>();
    if let Ok(mut recording) = state.recording.lock() {
        recording.state.segment_count = recording.state.segment_count.saturating_add(1);
    };
}

pub fn handle_hotkey_event(app: &AppHandle, event: HotkeyEventState) {
    let state = app.state::<DesktopState>();
    let action = {
        let config = match state.config() {
            Ok(config) => config,
            Err(_) => return,
        };
        let mut recording = match state.recording.lock() {
            Ok(recording) => recording,
            Err(_) => return,
        };
        match event {
            HotkeyEventState::Released => {
                recording.hotkey_down = false;
                (config.hotkey.mode == HotkeyMode::PushToTalk
                    && recording.state.phase == RecordingPhase::Recording)
                    .then_some(false)
            }
            HotkeyEventState::Pressed if recording.hotkey_down => None,
            HotkeyEventState::Pressed => {
                recording.hotkey_down = true;
                match (config.hotkey.mode, recording.state.phase) {
                    (_, RecordingPhase::Idle) => Some(true),
                    (HotkeyMode::Toggle, RecordingPhase::Recording) => Some(false),
                    _ => None,
                }
            }
        }
    };
    match action {
        Some(true) => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app.state::<DesktopState>();
                if let Err(error) = start_recording_inner(&app, &state).await {
                    emit_error(&app, "hotkey", &error.code, error.message, error.action);
                }
            });
        }
        Some(false) => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app.state::<DesktopState>();
                if let Err(error) = finish_recording_inner(&app, &state).await {
                    emit_error(&app, "hotkey", &error.code, error.message, error.action);
                }
            });
        }
        None => {}
    }
}

fn transcription_target(
    state: &DesktopState,
    config: &DesktopConfig,
) -> CommandResult<TranscriptionTarget> {
    match config.mode {
        ServiceMode::Remote => Ok(TranscriptionTarget::Remote {
            endpoint: config.remote.endpoint.clone(),
            token: config.remote.api_token.clone(),
        }),
        ServiceMode::Local => {
            #[cfg(feature = "local-runtime")]
            return Ok(TranscriptionTarget::Local(Arc::clone(&state.local)));
            #[cfg(not(feature = "local-runtime"))]
            {
                let _ = state;
                unavailable_local()
            }
        }
        ServiceMode::OpenRouter => Err(DesktopError::new(
            "openrouter_streaming_unsupported",
            "OpenRouter transcription runs after capture completes",
        )
        .with_action("Use batch capture for OpenRouter transcription.")),
    }
}

fn apply_settings(config: &mut DesktopConfig, input: DesktopSettingsInput) -> Result<()> {
    validate_shortcut(&input.hotkey_shortcut).map_err(|error| anyhow!(error))?;
    if input.sample_rate == 0 {
        return Err(anyhow!("sample rate must be greater than zero"));
    }
    if input.whisper_gpu_device < -1 {
        return Err(anyhow!(
            "GPU device must be -1 (automatic) or a non-negative device id"
        ));
    }
    let previous_mode = config.mode;
    config.model_path = PathBuf::from(input.model_path.trim());
    config.preload_on_startup = input.preload_on_startup;
    config.whisper_accelerator = input.whisper_accelerator;
    config.whisper_gpu_device = input.whisper_gpu_device;
    config.remote.endpoint = RemoteClient::validate_endpoint(&input.remote_endpoint)?;
    match input.remote_token {
        SecretUpdate::Keep => {}
        SecretUpdate::Set { value } => {
            let value = value.trim();
            if value.is_empty() {
                return Err(anyhow!("bearer token cannot be empty; use Clear instead"));
            }
            config.remote.api_token = Some(value.to_string());
        }
        SecretUpdate::Clear => config.remote.api_token = None,
    }
    let openrouter_model = input.openrouter_model.trim();
    crate::openrouter::validate_model(openrouter_model)?;
    config.openrouter.model = openrouter_model.to_string();
    match input.openrouter_key {
        SecretUpdate::Keep => {}
        SecretUpdate::Set { value } => {
            let value = value.trim();
            crate::openrouter::validate_api_key(value)?;
            config.openrouter.api_key = Some(value.to_string());
        }
        SecretUpdate::Clear => config.openrouter.api_key = None,
    }
    config.recording.input_device = input
        .input_device
        .map(|device| device.trim().to_string())
        .filter(|device| !device.is_empty());
    config.recording.transcription_mode = input.transcription_mode;
    config.recording.streaming_pcm_format = input.streaming_pcm_format;
    config.recording.english_only = input.english_only;
    store_mode_recording(config, previous_mode);
    config.mode = input.mode;
    normalize_mode_scoped_recording(config);
    config.output.copy_to_clipboard = input.copy_to_clipboard;
    config.output.paste_method = input.paste_method;
    config.output.paste_delay_ms = input.paste_delay_ms;
    config.output.prefix = input.output_prefix;
    config.output.suffix = input.output_suffix;
    config.hotkey.shortcut = input.hotkey_shortcut.trim().to_ascii_lowercase();
    config.hotkey.mode = input.hotkey_mode;
    config.close_to_tray = input.close_to_tray;
    config.show_window_title_bar = input.show_window_title_bar;
    Ok(())
}

fn ensure_mode(config: &DesktopConfig, expected: ServiceMode) -> CommandResult<()> {
    if config.mode == expected {
        Ok(())
    } else {
        Err(DesktopError::new(
            "wrong_service_mode",
            format!("this command requires {:?} mode", expected),
        )
        .with_action("Save the matching service mode in Settings first."))
    }
}

fn register_hotkey(state: &DesktopState, shortcut: &str) -> CommandResult<()> {
    let hotkey = state
        .hotkey
        .lock()
        .map_err(|_| internal_error("hotkey lock poisoned"))?;
    if let Some(hotkey) = hotkey.as_ref() {
        hotkey.register(shortcut).map_err(|error| {
            DesktopError::new("hotkey_registration_failed", error)
                .with_action("Choose a different global shortcut.")
        })?;
    }
    Ok(())
}

async fn refresh_remote(
    remote: &RemoteClient,
    config: &DesktopConfig,
) -> CommandResult<OverviewDto> {
    remote
        .overview(&config.remote.endpoint, config.remote.api_token.as_deref())
        .await
        .map_err(remote_error)
}

fn local_transcription_config(config: &DesktopConfig) -> TranscriptionConfig {
    TranscriptionConfig {
        model_path: config.model_path.clone(),
        preload_on_startup: config.preload_on_startup,
        sample_rate: config.recording.sample_rate,
        english_only: config.recording.english_only,
        whisper_accelerator: config.whisper_accelerator,
        whisper_gpu_device: config.whisper_gpu_device,
        inference_pool: config.inference_pool.clone(),
        legacy_queue_capacity: None,
    }
}

async fn persist_local_service<F>(
    state: &DesktopState,
    current: &DesktopConfig,
    next: &DesktopConfig,
    expected_generation: Option<u64>,
    persist: F,
) -> CommandResult<bool>
where
    F: FnOnce() -> Result<()> + Send + 'static,
{
    #[cfg(feature = "local-runtime")]
    {
        if let Some(expected) = expected_generation {
            let generation = state.local.generation();
            if expected != generation {
                return Err(stale_generation_error(expected, generation));
            }
        }
        let old_runtime = local_transcription_config(current);
        let next_runtime = local_transcription_config(next);
        if next.mode != ServiceMode::Local
            || (current.mode == ServiceMode::Local && old_runtime == next_runtime)
        {
            tokio::task::spawn_blocking(persist)
                .await
                .map_err(join_error)?
                .map_err(config_error)?;
            return Ok(false);
        }
        state.ensure_local_runtime_mutable("reload the local inference runtime")?;
        let runtime = Arc::clone(&state.local);
        tokio::task::spawn_blocking(move || {
            runtime.reload_transactional(expected_generation, next_runtime, persist)
        })
        .await
        .map_err(join_error)?
        .map_err(|error| {
            if error.to_string().starts_with("stale runtime generation") {
                DesktopError::new("stale_runtime_generation", error.to_string())
                    .with_action("Refresh the local runtime overview and retry the change.")
            } else {
                local_error(error)
            }
        })?;
        if let Ok(mut startup_error) = state.local_startup_error.lock() {
            *startup_error = None;
        }
        Ok(true)
    }
    #[cfg(not(feature = "local-runtime"))]
    {
        let _ = (state, current, next, expected_generation);
        tokio::task::spawn_blocking(persist)
            .await
            .map_err(join_error)?
            .map_err(config_error)?;
        Ok(false)
    }
}

#[cfg(feature = "local-runtime")]
fn stale_generation_error(expected: u64, generation: u64) -> DesktopError {
    DesktopError::new(
        "stale_runtime_generation",
        format!("stale runtime generation: expected {expected}, active generation is {generation}"),
    )
    .with_action("Refresh the local runtime overview and retry the change.")
}

#[cfg(feature = "local-runtime")]
fn reload_local_candidate(
    runtime: &InferenceRuntime,
    expected_generation: Option<u64>,
    candidate: TranscriptionConfig,
) -> Result<u64> {
    let generation = runtime.generation();
    if let Some(expected) = expected_generation {
        if expected != generation {
            anyhow::bail!(
                "stale runtime generation: expected {expected}, active generation is {generation}"
            );
        }
    }
    candidate.effective_inference_pool()?;
    runtime.reload(candidate)
}

fn validate_runtime(runtime: &RuntimeConfigDto) -> CommandResult<()> {
    if runtime.whisper_gpu_device < -1 {
        return Err(config_error(anyhow!(
            "GPU device must be -1 (automatic) or a non-negative device id"
        )));
    }
    if runtime.inference_pool_explicit == Some(true) && runtime.inference_pool.is_none() {
        return Err(config_error(anyhow!(
            "inference_pool is required when inference_pool_explicit is true"
        )));
    }
    #[cfg(feature = "local-runtime")]
    if runtime.whisper_gpu_device >= 0
        && !list_whisper_gpu_devices()
            .iter()
            .any(|device| device.id == runtime.whisper_gpu_device)
    {
        return Err(config_error(anyhow!(
            "GPU device {} is not available",
            runtime.whisper_gpu_device
        )));
    }
    if let Some(pool) = &runtime.inference_pool {
        pool.validate().map_err(config_error)?;
        #[cfg(feature = "local-runtime")]
        validate_pool_devices(pool)?;
    }
    Ok(())
}

#[cfg(feature = "local-runtime")]
fn validate_pool_devices(pool: &InferencePoolConfig) -> CommandResult<()> {
    let devices = list_whisper_gpu_devices();
    for unit in pool.units.iter().filter(|unit| unit.enabled) {
        if let shadoword_core::ExecutionTarget::Gpu { device, .. } = unit.target {
            if !devices.iter().any(|candidate| candidate.id == device) {
                return Err(config_error(anyhow!(
                    "execution unit {:?} requests unavailable GPU device {device}",
                    unit.id
                )));
            }
        }
    }
    Ok(())
}

fn local_overview(state: &DesktopState) -> CommandResult<OverviewDto> {
    #[cfg(feature = "local-runtime")]
    {
        let config = state.config()?;
        let directory = DesktopConfig::models_dir().map_err(config_error)?;
        let resolved_path = if config.model_path.as_os_str().is_empty() {
            directory.join(default_whisper_model().filename)
        } else {
            config.model_path.clone()
        };
        let mut service =
            TranscriptionService::status(state.local.as_ref()).map_err(local_error)?;
        let pool_status = state.local.status();
        service.model_path = resolved_path.to_string_lossy().into_owned();
        let models: Vec<ModelInfoDto> = list_whisper_models()
            .iter()
            .map(|model| ModelInfoDto {
                id: model.id.to_string(),
                name: model.name.to_string(),
                filename: model.filename.to_string(),
                description: model.description.to_string(),
                size_bytes: model.size_bytes,
                recommended: model.recommended,
                installed: directory.join(model.filename).is_file(),
            })
            .collect();
        let effective_pool = local_transcription_config(&config)
            .effective_inference_pool()
            .map_err(config_error)?;
        let in_flight = pool_status
            .queued_jobs
            .saturating_add(pool_status.running_jobs);
        let generation = Some(pool_status.generation);
        Ok(OverviewDto {
            status: DaemonStatusDto {
                service,
                in_flight_requests: in_flight,
                queue_capacity: effective_pool.limits.max_queued_jobs,
            },
            runtime: RuntimeConfigDto {
                model_path: resolved_path.to_string_lossy().into_owned(),
                whisper_accelerator: config.whisper_accelerator,
                whisper_gpu_device: config.whisper_gpu_device,
                english_only: config.recording.english_only,
                preload_on_startup: config.preload_on_startup,
                inference_pool: Some(effective_pool),
                inference_pool_explicit: Some(config.inference_pool.is_some()),
                generation,
            },
            model_storage: Some(model_storage(&directory, &models)),
            models,
        })
    }
    #[cfg(not(feature = "local-runtime"))]
    {
        let _ = state;
        unavailable_local()
    }
}

#[cfg(feature = "local-runtime")]
fn model_storage(directory: &std::path::Path, models: &[ModelInfoDto]) -> ModelStorageDto {
    let total_bytes = std::fs::read_dir(directory)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.metadata().ok())
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
        .sum();
    ModelStorageDto {
        directory: directory.to_string_lossy().into_owned(),
        total_bytes,
        installed_model_count: models.iter().filter(|model| model.installed).count(),
    }
}

#[cfg(feature = "local-runtime")]
fn update_download(
    jobs: &Mutex<HashMap<String, DownloadJobStatus>>,
    id: &str,
    update: impl FnOnce(&mut DownloadJobStatus),
) {
    if let Ok(mut jobs) = jobs.lock() {
        if let Some(job) = jobs.get_mut(id) {
            update(job);
        }
    }
}

fn busy_recording_error() -> DesktopError {
    DesktopError::new("recording_busy", "a recording is already active")
        .with_action("Stop or cancel it before starting another recording.")
}

fn internal_error(message: impl Into<String>) -> DesktopError {
    DesktopError::new("internal_state_error", message)
}

fn config_error(error: impl std::fmt::Display) -> DesktopError {
    DesktopError::new("invalid_configuration", error.to_string())
        .with_action("Correct the highlighted setting and try again.")
}

fn validate_openrouter_config(config: &DesktopConfig) -> CommandResult<()> {
    config
        .openrouter
        .api_key
        .as_deref()
        .ok_or_else(openrouter_key_required)
        .and_then(|key| {
            crate::openrouter::validate_api_key(key)
                .map(|_| ())
                .map_err(config_error)
        })?;
    crate::openrouter::validate_model(&config.openrouter.model).map_err(config_error)?;
    Ok(())
}

fn configured_secret(config: &DesktopConfig, kind: DesktopSecretKind) -> CommandResult<String> {
    let value = match kind {
        DesktopSecretKind::RemoteToken => config.remote.api_token.as_deref(),
        DesktopSecretKind::OpenRouterKey => config.openrouter.api_key.as_deref(),
    };
    value
        .filter(|secret| !secret.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            DesktopError::new(
                "credential_not_configured",
                "no saved credential is configured",
            )
            .with_action("Enter and verify a credential before revealing or copying it.")
        })
}

fn openrouter_key_required() -> DesktopError {
    DesktopError::new(
        "openrouter_key_required",
        "an OpenRouter API key is required",
    )
    .with_action("Enter an OpenRouter API key in Settings and save the configuration.")
}

fn openrouter_error(error: impl std::fmt::Display) -> DesktopError {
    DesktopError::new("openrouter_transcription_failed", error.to_string()).with_action(
        "Check the OpenRouter API key, transcription model, account credits, and network connection.",
    )
}

fn remote_error(error: anyhow::Error) -> DesktopError {
    if let Some(remote) = error.downcast_ref::<crate::remote::RemoteApiError>() {
        if remote.code() == "stale_generation" {
            return DesktopError::new("stale_runtime_generation", error.to_string())
                .with_action("Refresh the remote runtime overview and retry the change.");
        }
        if remote.code() == "model_deletion_unsupported" {
            return DesktopError::new("remote_model_deletion_unsupported", error.to_string())
                .with_action(
                    "Update and restart the Shadoword API daemon before deleting remote models.",
                );
        }
        if remote.code() == "forbidden" {
            return DesktopError::new("remote_permission_denied", error.to_string()).with_action(
                "Use an admin token for desktop management; user tokens can only transcribe audio.",
            );
        }
    }
    DesktopError::new("remote_request_failed", error.to_string())
        .with_action("Check the endpoint, bearer token, daemon status, and network connection.")
}

fn local_error(error: impl std::fmt::Display) -> DesktopError {
    DesktopError::new("local_runtime_failed", error.to_string())
        .with_action("Download or select a valid model and verify accelerator settings.")
}

fn join_error(error: impl std::fmt::Display) -> DesktopError {
    DesktopError::new("native_task_failed", error.to_string())
}

#[cfg(not(feature = "local-runtime"))]
fn unavailable_local<T>() -> CommandResult<T> {
    Err(DesktopError::new(
        "local_runtime_not_compiled",
        "this desktop build does not include the local runtime feature",
    )
    .with_action("Rebuild Shadoword Desktop with the local-runtime feature."))
}
