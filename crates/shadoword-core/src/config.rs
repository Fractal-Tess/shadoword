use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::env;
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
pub enum WhisperAccelerator {
    Auto,
    Cpu,
    Gpu,
}

impl Default for WhisperAccelerator {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PasteMethod {
    None,
    Direct,
    CtrlV,
    CtrlShiftV,
    ShiftInsert,
}

impl Default for PasteMethod {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TypingTool {
    Auto,
    Wtype,
    Kwtype,
    Dotool,
    Ydotool,
    Xdotool,
}

impl Default for TypingTool {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordingConfig {
    pub input_device: Option<String>,
    pub sample_rate: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    pub copy_to_clipboard: bool,
    pub type_into_active_window: bool,
    pub paste_method: PasteMethod,
    pub typing_tool: TypingTool,
    pub paste_delay_ms: u64,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            copy_to_clipboard: true,
            type_into_active_window: false,
            paste_method: PasteMethod::None,
            typing_tool: TypingTool::Auto,
            paste_delay_ms: 120,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteConfig {
    pub endpoint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub listen_addr: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeyConfig {
    pub shortcut: String,
    pub push_to_talk: bool,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            shortcut: "ctrl+space".to_string(),
            push_to_talk: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShadowwordConfig {
    pub mode: ServiceMode,
    pub model_path: PathBuf,
    #[serde(default)]
    pub preload_on_startup: bool,
    pub recording: RecordingConfig,
    pub output: OutputConfig,
    pub remote: RemoteConfig,
    pub daemon: DaemonConfig,
    #[serde(default)]
    pub hotkey: HotkeyConfig,
    #[serde(default)]
    pub whisper_accelerator: WhisperAccelerator,
}

impl Default for ShadowwordConfig {
    fn default() -> Self {
        Self {
            mode: ServiceMode::Local,
            model_path: PathBuf::new(),
            preload_on_startup: false,
            recording: RecordingConfig {
                input_device: None,
                sample_rate: 16_000,
            },
            output: OutputConfig::default(),
            remote: RemoteConfig {
                endpoint: "http://127.0.0.1:47813".to_string(),
            },
            daemon: DaemonConfig {
                listen_addr: "127.0.0.1:47813".to_string(),
            },
            hotkey: HotkeyConfig::default(),
            whisper_accelerator: WhisperAccelerator::Auto,
        }
    }
}

impl ShadowwordConfig {
    fn project_dirs(app_name: &str) -> Result<ProjectDirs> {
        ProjectDirs::from("io", "fractaltess", app_name)
            .context("failed to resolve project directories")
    }

    fn shadoword_config_path() -> Result<PathBuf> {
        let dirs = Self::project_dirs("shadoword")?;
        let config_dir = dirs.config_dir();
        fs::create_dir_all(config_dir).context("failed to create config directory")?;
        Ok(config_dir.join("config.json"))
    }

    fn legacy_config_path() -> Result<PathBuf> {
        let dirs = Self::project_dirs("shadowword")?;
        Ok(dirs.config_dir().join("config.json"))
    }

    pub fn config_path() -> Result<PathBuf> {
        Self::shadoword_config_path()
    }

    pub fn models_dir() -> Result<PathBuf> {
        let shadoword_dirs = Self::project_dirs("shadoword")?;
        let shadoword_models_dir = shadoword_dirs.data_dir().join("models");
        let legacy_dirs = Self::project_dirs("shadowword")?;
        let legacy_models_dir = legacy_dirs.data_dir().join("models");

        if legacy_models_dir.exists() && !shadoword_models_dir.exists() {
            return Ok(legacy_models_dir);
        }

        fs::create_dir_all(&shadoword_models_dir).context("failed to create models directory")?;
        Ok(shadoword_models_dir)
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        let legacy_path = Self::legacy_config_path()?;

        let load_path = if path.exists() {
            path
        } else if legacy_path.exists() {
            legacy_path
        } else {
            let mut config = Self::default();
            config.apply_env_overrides();
            return Ok(config);
        };

        let raw = fs::read_to_string(&load_path)
            .with_context(|| format!("failed to read config at {}", load_path.display()))?;
        let mut config: Self =
            serde_json::from_str(&raw).context("failed to parse config json")?;
        config.apply_env_overrides();
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let raw = serde_json::to_string_pretty(self).context("failed to serialize config")?;
        fs::write(&path, raw).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(listen_addr) = env::var("SHADOWORD_LISTEN_ADDR") {
            let listen_addr = listen_addr.trim();
            if !listen_addr.is_empty() {
                self.daemon.listen_addr = listen_addr.to_string();
            }
        }
    }
}
