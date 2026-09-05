use crate::contracts::{
    ConnectionInput, ConnectionReport, DesktopBootstrap, DesktopError, DesktopEvent,
    DesktopSecretKind, DesktopSettings, DesktopSettingsInput, MicrophoneLevel,
    OpenRouterConnectionInput, OpenRouterKeyReport, OpenRouterModelInfo, RecordingPhase,
    RecordingState, RecordingStatus, SecretUpdate, TranscriptionResult, DESKTOP_EVENT_NAME,
};
use crate::history::{HistoryEntry, HistoryStore};
use crate::hotkeys::{validate_shortcut, HotkeyBackend, HotkeyEventState};
use crate::openrouter::OpenRouterClient;
use crate::recording::{
    emit_error, spawn_streaming_worker, OpenRouterStreamTarget, StreamCommand, StreamingWorker,
    TranscriptionTarget,
};
use crate::remote::RemoteClient;
use anyhow::{anyhow, Context, Result};
use shadoword_core::remote_contracts::{
    ApiTokenSummaryDto, CreateApiTokenRequest, CreatedApiTokenDto, DownloadJobStatus, OverviewDto,
    RuntimeConfigDto,
};
#[cfg(feature = "local-runtime")]
use shadoword_core::remote_contracts::{
    DaemonStatusDto, DownloadJobState, ModelInfoDto, ModelStorageDto,
};
#[cfg(feature = "local-runtime")]
use shadoword_core::{
    default_whisper_model, download_whisper_model_with_progress, list_whisper_gpu_devices,
    list_whisper_models, resolve_whisper_model, unknown_model_error, InferencePoolConfig,
    InferenceRuntime, TranscriptionService, WhisperModelFactory,
};
use shadoword_core::{
    DesktopConfig, HotkeyMode, MicrophoneLevelMonitor, MicrophoneRecorder,
    ModeRecordingPreferences, OutputConfig, RecordingSession, ServiceMode, StreamingPcmFormat,
    TranscriptionConfig, TranscriptionMode,
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
pub(crate) mod local;
pub(crate) mod recording;
pub(crate) mod remote;
pub(crate) mod state;
pub(crate) mod support;

use support::*;

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

struct ActiveMicrophoneLevelMonitor {
    input_device_name: Option<String>,
    monitor: MicrophoneLevelMonitor,
}

#[derive(Default)]
struct MicrophoneLevelMonitorController {
    active: Option<ActiveMicrophoneLevelMonitor>,
}

pub struct DesktopState {
    config: Mutex<DesktopConfig>,
    recording: Mutex<RecordingController>,
    microphone_level_monitor: Mutex<MicrophoneLevelMonitorController>,
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
    history: HistoryStore,
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
            microphone_level_monitor: Mutex::new(MicrophoneLevelMonitorController::default()),
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
            history: HistoryStore::load().context("failed to load transcript history")?,
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

    fn poll_microphone_level(&self) -> CommandResult<MicrophoneLevel> {
        let input_device_name = self.config()?.recording.input_device;
        let mut controller = self
            .microphone_level_monitor
            .lock()
            .map_err(|_| internal_error("microphone level monitor lock poisoned"))?;

        if self.recording_state()?.phase != RecordingPhase::Idle {
            drop(controller.active.take());
            return Ok(MicrophoneLevel {
                peak: 0.0,
                monitoring: false,
            });
        }

        if controller
            .active
            .as_ref()
            .is_some_and(|active| active.input_device_name != input_device_name)
        {
            drop(controller.active.take());
        }

        if controller.active.is_none() {
            let monitor = MicrophoneRecorder::start_level_monitor(input_device_name.as_deref())
                .map_err(|error| {
                    DesktopError::new("microphone_monitor_start_failed", error.to_string())
                        .with_action("Check the selected input device and microphone permissions.")
                })?;
            controller.active = Some(ActiveMicrophoneLevelMonitor {
                input_device_name,
                monitor,
            });
        }

        Ok(MicrophoneLevel {
            peak: controller
                .active
                .as_ref()
                .expect("microphone level monitor initialized above")
                .monitor
                .peak(),
            monitoring: true,
        })
    }

    fn stop_microphone_level_monitor(&self) -> CommandResult<()> {
        let active = self
            .microphone_level_monitor
            .lock()
            .map_err(|_| internal_error("microphone level monitor lock poisoned"))?
            .active
            .take();
        drop(active);
        Ok(())
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
        ServiceMode::Local | ServiceMode::OpenRouter => {
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
    let _ = recording::cancel_recording_inner(app, &state);
    let _ = state.stop_microphone_level_monitor();
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
