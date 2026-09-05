use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
mod inference_pool;
mod persistence;

pub(crate) use inference_pool::DEFAULT_GPU_HOST_THREADS;
pub use inference_pool::{
    ExecutionTarget, ExecutionUnitConfig, InferenceLimits, InferencePoolConfig,
};
pub use persistence::{data_dir, models_dir, write_json_atomic, ConfigPaths};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum ServiceMode {
    Local,
    Remote,
    OpenRouter,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum WhisperAccelerator {
    #[default]
    Auto,
    Cpu,
    Gpu,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum PasteMethod {
    #[default]
    None,
    Direct,
    CtrlV,
    CtrlShiftV,
    ShiftInsert,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptBoundary {
    #[default]
    None,
    Space,
    Newline,
    BlankLine,
}

impl TranscriptBoundary {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Space => " ",
            Self::Newline => "\n",
            Self::BlankLine => "\n\n",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionMode {
    #[default]
    Batch,
    Streaming,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum StreamingPcmFormat {
    S16le,
    #[default]
    F32le,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RecordingConfig {
    pub input_device: Option<String>,
    pub sample_rate: u32,
    pub transcription_mode: TranscriptionMode,
    pub streaming_pcm_format: StreamingPcmFormat,
    pub english_only: bool,
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            input_device: None,
            sample_rate: 16_000,
            transcription_mode: TranscriptionMode::Batch,
            streaming_pcm_format: StreamingPcmFormat::F32le,
            english_only: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    pub copy_to_clipboard: bool,
    pub paste_method: PasteMethod,
    pub paste_delay_ms: u64,
    pub prefix: TranscriptBoundary,
    pub suffix: TranscriptBoundary,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            copy_to_clipboard: true,
            paste_method: PasteMethod::None,
            paste_delay_ms: 120,
            prefix: TranscriptBoundary::None,
            suffix: TranscriptBoundary::Space,
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RemoteConfig {
    pub endpoint: String,
    pub api_token: Option<String>,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:47813".to_string(),
            api_token: None,
        }
    }
}

impl std::fmt::Debug for RemoteConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RemoteConfig")
            .field("endpoint", &self.endpoint)
            .field("api_token", &self.api_token.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

fn default_openrouter_model() -> String {
    "openai/whisper-large-v3".to_string()
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenRouterConfig {
    pub api_key: Option<String>,
    pub model: String,
}

impl Default for OpenRouterConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: default_openrouter_model(),
        }
    }
}

impl std::fmt::Debug for OpenRouterConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OpenRouterConfig")
            .field("api_key", &self.api_key.as_ref().map(|_| "[REDACTED]"))
            .field("model", &self.model)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum HotkeyMode {
    #[default]
    PushToTalk,
    Toggle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeyConfig {
    pub shortcut: String,
    pub mode: HotkeyMode,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            shortcut: "f2".to_string(),
            mode: HotkeyMode::PushToTalk,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TranscriptionConfig {
    pub model_path: PathBuf,
    #[serde(default = "default_preload_on_startup")]
    pub preload_on_startup: bool,
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
    #[serde(default)]
    pub english_only: bool,
    #[serde(default)]
    pub whisper_accelerator: WhisperAccelerator,
    #[serde(default = "default_whisper_gpu_device")]
    pub whisper_gpu_device: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference_pool: Option<InferencePoolConfig>,
    /// API-only compatibility override for the legacy daemon queue setting.
    /// It is deliberately not persisted as part of the shared transcription schema.
    #[serde(skip)]
    pub legacy_queue_capacity: Option<usize>,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::new(),
            preload_on_startup: default_preload_on_startup(),
            sample_rate: default_sample_rate(),
            english_only: false,
            whisper_accelerator: WhisperAccelerator::Auto,
            whisper_gpu_device: default_whisper_gpu_device(),
            inference_pool: None,
            legacy_queue_capacity: None,
        }
    }
}

impl TranscriptionConfig {
    pub fn backend_reload_required(&self, next: &Self) -> bool {
        self.model_path != next.model_path
            || self.whisper_accelerator != next.whisper_accelerator
            || self.whisper_gpu_device != next.whisper_gpu_device
            || self.inference_pool != next.inference_pool
            || self.legacy_queue_capacity != next.legacy_queue_capacity
    }

    pub fn effective_inference_pool(&self) -> Result<InferencePoolConfig> {
        if let Some(pool) = &self.inference_pool {
            pool.validate()?;
            return Ok(pool.clone());
        }

        let target = match self.whisper_accelerator {
            WhisperAccelerator::Cpu => ExecutionTarget::Cpu { threads: None },
            WhisperAccelerator::Auto | WhisperAccelerator::Gpu => ExecutionTarget::Gpu {
                device: self.whisper_gpu_device,
                host_threads: None,
            },
        };
        let mut pool = InferencePoolConfig {
            units: vec![ExecutionUnitConfig {
                id: "legacy".to_string(),
                enabled: true,
                required: true,
                target,
            }],
            limits: InferenceLimits::default(),
            ..InferencePoolConfig::default()
        };
        if let Some(queue_capacity) = self.legacy_queue_capacity {
            pool.limits.max_queued_jobs = queue_capacity;
        }
        Ok(pool)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModeRecordingPreferences {
    pub transcription_mode: TranscriptionMode,
    pub streaming_pcm_format: StreamingPcmFormat,
    pub english_only: bool,
}

impl From<&RecordingConfig> for ModeRecordingPreferences {
    fn from(recording: &RecordingConfig) -> Self {
        Self {
            transcription_mode: recording.transcription_mode,
            streaming_pcm_format: recording.streaming_pcm_format,
            english_only: recording.english_only,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DesktopConfig {
    pub mode: ServiceMode,
    pub model_path: PathBuf,
    #[serde(default = "default_preload_on_startup")]
    pub preload_on_startup: bool,
    pub recording: RecordingConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_recording: Option<ModeRecordingPreferences>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_recording: Option<ModeRecordingPreferences>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openrouter_recording: Option<ModeRecordingPreferences>,
    pub output: OutputConfig,
    pub remote: RemoteConfig,
    #[serde(default)]
    pub openrouter: OpenRouterConfig,
    #[serde(default)]
    pub hotkey: HotkeyConfig,
    #[serde(default = "default_close_to_tray")]
    pub close_to_tray: bool,
    #[serde(default = "default_show_window_title_bar")]
    pub show_window_title_bar: bool,
    #[serde(default)]
    pub whisper_accelerator: WhisperAccelerator,
    #[serde(default = "default_whisper_gpu_device")]
    pub whisper_gpu_device: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference_pool: Option<InferencePoolConfig>,
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            mode: ServiceMode::Local,
            model_path: PathBuf::new(),
            preload_on_startup: default_preload_on_startup(),
            recording: RecordingConfig::default(),
            local_recording: None,
            remote_recording: None,
            openrouter_recording: None,
            output: OutputConfig::default(),
            remote: RemoteConfig::default(),
            openrouter: OpenRouterConfig::default(),
            hotkey: HotkeyConfig::default(),
            close_to_tray: default_close_to_tray(),
            show_window_title_bar: default_show_window_title_bar(),
            whisper_accelerator: WhisperAccelerator::Auto,
            whisper_gpu_device: default_whisper_gpu_device(),
            inference_pool: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ApiTokenRole {
    Admin,
    User,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiTokenConfig {
    pub name: String,
    pub role: ApiTokenRole,
    pub token_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    pub listen_addr: String,
    pub transcription: TranscriptionConfig,
    pub queue_capacity: usize,
    pub tokens: Vec<ApiTokenConfig>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:47813".to_string(),
            transcription: TranscriptionConfig::default(),
            queue_capacity: 4,
            tokens: Vec::new(),
        }
    }
}

fn default_preload_on_startup() -> bool {
    true
}

fn default_whisper_gpu_device() -> i32 {
    -1
}

fn default_sample_rate() -> u32 {
    16_000
}

fn default_close_to_tray() -> bool {
    true
}

fn default_show_window_title_bar() -> bool {
    true
}
