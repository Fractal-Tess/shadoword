use serde::{Deserialize, Serialize};
use shadoword_core::remote_contracts::{OverviewDto, RuntimeConfigDto};
use shadoword_core::{
    HotkeyMode, InputDeviceInfo, PasteMethod, ServiceMode, StreamingPcmFormat, TranscriptionMode,
    WhisperAccelerator,
};
use specta::Type;

#[derive(Debug, Clone, Serialize, Type)]
pub struct DesktopSettings {
    pub mode: ServiceMode,
    pub local_runtime_available: bool,
    pub model_path: String,
    pub preload_on_startup: bool,
    pub whisper_accelerator: WhisperAccelerator,
    pub whisper_gpu_device: i32,
    pub remote_endpoint: String,
    pub remote_token_configured: bool,
    pub openrouter_model: String,
    pub openrouter_key_configured: bool,
    pub input_device: Option<String>,
    pub sample_rate: u32,
    pub transcription_mode: TranscriptionMode,
    pub streaming_pcm_format: StreamingPcmFormat,
    pub english_only: bool,
    pub copy_to_clipboard: bool,
    pub paste_method: PasteMethod,
    #[specta(type = u32)]
    pub paste_delay_ms: u64,
    pub hotkey_shortcut: String,
    pub hotkey_mode: HotkeyMode,
    pub close_to_tray: bool,
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct DesktopSettingsInput {
    pub mode: ServiceMode,
    pub model_path: String,
    pub preload_on_startup: bool,
    pub whisper_accelerator: WhisperAccelerator,
    pub whisper_gpu_device: i32,
    pub remote_endpoint: String,
    pub remote_token: SecretUpdate,
    pub openrouter_model: String,
    pub openrouter_key: SecretUpdate,
    pub input_device: Option<String>,
    pub sample_rate: u32,
    pub transcription_mode: TranscriptionMode,
    pub streaming_pcm_format: StreamingPcmFormat,
    pub english_only: bool,
    pub copy_to_clipboard: bool,
    pub paste_method: PasteMethod,
    #[specta(type = u32)]
    pub paste_delay_ms: u64,
    pub hotkey_shortcut: String,
    pub hotkey_mode: HotkeyMode,
    pub close_to_tray: bool,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum SecretUpdate {
    Keep,
    Set { value: String },
    Clear,
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct ConnectionInput {
    pub endpoint: String,
    pub token: Option<String>,
    pub use_saved_token: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct DesktopBootstrap {
    pub settings: DesktopSettings,
    pub input_devices: Vec<InputDeviceInfo>,
    pub input_devices_error: Option<String>,
    pub recording: RecordingState,
    pub local_overview: Option<OverviewDto>,
    pub local_startup_error: Option<DesktopError>,
    pub hotkey_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ConnectionReport {
    pub health_ok: bool,
    pub status_model_loaded: bool,
    pub overview: OverviewDto,
    pub runtime_config: RuntimeConfigDto,
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct OpenRouterConnectionInput {
    pub key: Option<String>,
    pub use_saved_key: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct OpenRouterKeyReport {
    pub label: Option<String>,
    pub is_free_tier: bool,
    pub limit: Option<f64>,
    pub limit_remaining: Option<f64>,
    pub usage: f64,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct OpenRouterModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct RecordingStatus {
    pub recording: bool,
    pub sample_rate: u32,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum RecordingPhase {
    Idle,
    Recording,
    Finalizing,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct RecordingState {
    pub phase: RecordingPhase,
    pub service_mode: Option<ServiceMode>,
    pub transcription_mode: Option<TranscriptionMode>,
    pub sample_rate: Option<u32>,
    #[specta(type = u32)]
    pub segment_count: usize,
}

impl Default for RecordingState {
    fn default() -> Self {
        Self {
            phase: RecordingPhase::Idle,
            service_mode: None,
            transcription_mode: None,
            sample_rate: None,
            segment_count: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct DesktopError {
    pub code: String,
    pub message: String,
    pub action: Option<String>,
}

impl DesktopError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            action: None,
        }
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }
}

impl std::fmt::Display for DesktopError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for DesktopError {}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TranscriptionResult {
    pub text: String,
    #[specta(type = u32)]
    pub elapsed_ms: u64,
    pub engine: String,
    #[specta(type = u32)]
    pub audio_duration_ms: u64,
    pub sample_rate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
#[tauri_specta(event_name = "shadoword://desktop-event")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DesktopEvent {
    Status {
        message: String,
    },
    RecordingStarted {
        sample_rate: u32,
    },
    RecordingStopped {
        processing: bool,
    },
    RecordingCancelled,
    TranscriptSegment {
        result: TranscriptionResult,
        #[specta(type = u32)]
        segment_index: usize,
    },
    TranscriptionComplete {
        result: TranscriptionResult,
        #[specta(type = u32)]
        segments: usize,
    },
    Error {
        code: String,
        context: String,
        message: String,
        action: Option<String>,
    },
}

pub const DESKTOP_EVENT_NAME: &str = "shadoword://desktop-event";
