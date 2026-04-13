use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceMode {
    Local,
    Remote,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    Parakeet,
    Whisper,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OrtxAccelerator {
    Auto,
    Cpu,
    Cuda,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WhisperAccelerator {
    Auto,
    Cpu,
    Gpu,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OnnxQuantization {
    Fp32,
    Fp16,
    Int8,
    Int4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingConfig {
    pub input_device: Option<String>,
    pub sample_rate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub copy_to_clipboard: bool,
    pub type_into_active_window: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub listen_addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowwordConfig {
    pub mode: ServiceMode,
    pub engine: EngineKind,
    pub model_path: PathBuf,
    pub recording: RecordingConfig,
    pub output: OutputConfig,
    pub remote: RemoteConfig,
    pub daemon: DaemonConfig,
    pub onnx_quantization: OnnxQuantization,
    pub ort_accelerator: OrtxAccelerator,
    pub whisper_accelerator: WhisperAccelerator,
}

impl Default for ShadowwordConfig {
    fn default() -> Self {
        Self {
            mode: ServiceMode::Local,
            engine: EngineKind::Parakeet,
            model_path: PathBuf::new(),
            recording: RecordingConfig {
                input_device: None,
                sample_rate: 16_000,
            },
            output: OutputConfig {
                copy_to_clipboard: true,
                type_into_active_window: false,
            },
            remote: RemoteConfig {
                endpoint: "http://127.0.0.1:47813".to_string(),
            },
            daemon: DaemonConfig {
                listen_addr: "127.0.0.1:47813".to_string(),
            },
            onnx_quantization: OnnxQuantization::Fp32,
            ort_accelerator: OrtxAccelerator::Auto,
            whisper_accelerator: WhisperAccelerator::Auto,
        }
    }
}

impl ShadowwordConfig {
    pub fn config_path() -> Result<PathBuf> {
        let dirs = ProjectDirs::from("io", "fractaltess", "shadowword")
            .context("failed to resolve project directories")?;
        let config_dir = dirs.config_dir();
        fs::create_dir_all(config_dir).context("failed to create config directory")?;
        Ok(config_dir.join("config.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config at {}", path.display()))?;
        let config = serde_json::from_str(&raw).context("failed to parse config json")?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let raw = serde_json::to_string_pretty(self).context("failed to serialize config")?;
        fs::write(&path, raw).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }
}
