use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecutionTarget {
    Cpu {
        #[serde(default)]
        #[specta(type = Option<u32>)]
        threads: Option<usize>,
    },
    Gpu {
        device: i32,
        #[serde(default)]
        #[specta(type = Option<u32>)]
        host_threads: Option<usize>,
    },
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ExecutionUnitConfig {
    pub id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub required: bool,
    pub target: ExecutionTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct InferenceLimits {
    #[specta(type = u32)]
    pub max_queued_jobs: usize,
    #[specta(type = u32)]
    pub max_queued_audio_bytes: usize,
    #[specta(type = u32)]
    pub max_audio_bytes_per_job: usize,
    #[specta(type = u32)]
    pub max_outstanding_per_flow: usize,
    #[specta(type = u32)]
    #[serde(rename = "max_buffered_results_per_flow")]
    // Sequencer-only bound for API stream completions waiting on an earlier
    // sequence. Core job receivers are single-result channels are not counted.
    // The old wire name is retained for config compatibility.
    pub max_sequencer_buffered_results_per_flow: usize,
}

impl Default for InferenceLimits {
    fn default() -> Self {
        Self {
            max_queued_jobs: 32,
            max_queued_audio_bytes: 64 * 1024 * 1024,
            max_audio_bytes_per_job: 64 * 1024 * 1024,
            max_outstanding_per_flow: 8,
            max_sequencer_buffered_results_per_flow: 32,
        }
    }
}

fn default_preload_timeout_ms() -> u64 {
    120_000
}

fn default_max_draining_generations() -> usize {
    2
}

const MAX_PRELOAD_TIMEOUT_MS: u64 = 30 * 60 * 1_000;
const MAX_DRAINING_GENERATIONS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct InferencePoolConfig {
    pub units: Vec<ExecutionUnitConfig>,
    pub limits: InferenceLimits,
    #[specta(type = f64)]
    pub preload_timeout_ms: u64,
    #[specta(type = u32)]
    pub max_draining_generations: usize,
}

impl Default for InferencePoolConfig {
    fn default() -> Self {
        Self {
            units: Vec::new(),
            limits: InferenceLimits::default(),
            preload_timeout_ms: default_preload_timeout_ms(),
            max_draining_generations: default_max_draining_generations(),
        }
    }
}

impl InferencePoolConfig {
    pub fn validate(&self) -> Result<()> {
        let parallelism = std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(1);
        self.validate_with_parallelism(parallelism)
    }

    pub fn validate_with_parallelism(&self, available_parallelism: usize) -> Result<()> {
        if self.limits.max_queued_audio_bytes == 0
            || self.limits.max_audio_bytes_per_job == 0
            || self.limits.max_outstanding_per_flow == 0
            || self.limits.max_sequencer_buffered_results_per_flow == 0
        {
            anyhow::bail!(
                "inference pool byte, flow, and sequencer limits must be greater than zero"
            );
        }
        if self.preload_timeout_ms == 0 {
            anyhow::bail!("inference pool preload_timeout_ms must be greater than zero");
        }
        if self.preload_timeout_ms > MAX_PRELOAD_TIMEOUT_MS {
            anyhow::bail!(
                "inference pool preload_timeout_ms must not exceed {MAX_PRELOAD_TIMEOUT_MS}"
            );
        }
        if self.max_draining_generations == 0 {
            anyhow::bail!("inference pool max_draining_generations must be greater than zero");
        }
        if self.max_draining_generations > MAX_DRAINING_GENERATIONS {
            anyhow::bail!(
                "inference pool max_draining_generations must not exceed {MAX_DRAINING_GENERATIONS}"
            );
        }

        let enabled = self
            .units
            .iter()
            .filter(|unit| unit.enabled)
            .collect::<Vec<_>>();
        if enabled.is_empty() {
            anyhow::bail!("inference pool must contain at least one enabled execution unit");
        }

        let mut ids = HashSet::new();
        for unit in &self.units {
            if !valid_unit_id(&unit.id) {
                anyhow::bail!(
                    "invalid execution unit id {:?}; use 1-64 ASCII letters, digits, '.', '_' or '-'",
                    unit.id
                );
            }
            if !ids.insert(unit.id.as_str()) {
                anyhow::bail!("duplicate execution unit id {:?}", unit.id);
            }
        }

        let mut gpu_devices = HashSet::new();
        let mut requested_cpu_threads = 0usize;
        let available_parallelism = available_parallelism.max(1);

        for unit in enabled {
            match unit.target {
                ExecutionTarget::Gpu {
                    device,
                    host_threads,
                } => {
                    if device < 0 {
                        anyhow::bail!(
                            "execution unit {:?} must use an explicit non-negative GPU device",
                            unit.id
                        );
                    }
                    if !gpu_devices.insert(device) {
                        anyhow::bail!("GPU device {device} is assigned to more than one unit");
                    }
                    let threads = host_threads.unwrap_or(1);
                    if threads == 0 || threads > available_parallelism {
                        anyhow::bail!(
                            "execution unit {:?} requests {threads} GPU host threads, but available parallelism is {available_parallelism}",
                            unit.id
                        );
                    }
                    requested_cpu_threads = requested_cpu_threads.saturating_add(threads);
                }
                ExecutionTarget::Cpu { threads } => {
                    let threads = threads.unwrap_or(available_parallelism.min(4));
                    if threads == 0 || threads > available_parallelism {
                        anyhow::bail!(
                            "execution unit {:?} requests {threads} CPU threads, but available parallelism is {available_parallelism}",
                            unit.id
                        );
                    }
                    requested_cpu_threads = requested_cpu_threads.saturating_add(threads);
                }
            }
        }

        if requested_cpu_threads > available_parallelism {
            anyhow::bail!(
                "CPU execution units request {requested_cpu_threads} total threads, exceeding available parallelism {available_parallelism}"
            );
        }
        Ok(())
    }
}

fn valid_unit_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
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
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            copy_to_clipboard: true,
            paste_method: PasteMethod::None,
            paste_delay_ms: 120,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DesktopConfig {
    pub mode: ServiceMode,
    pub model_path: PathBuf,
    #[serde(default = "default_preload_on_startup")]
    pub preload_on_startup: bool,
    pub recording: RecordingConfig,
    pub output: OutputConfig,
    pub remote: RemoteConfig,
    #[serde(default)]
    pub openrouter: OpenRouterConfig,
    #[serde(default)]
    pub hotkey: HotkeyConfig,
    #[serde(default = "default_close_to_tray")]
    pub close_to_tray: bool,
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
            output: OutputConfig::default(),
            remote: RemoteConfig::default(),
            openrouter: OpenRouterConfig::default(),
            hotkey: HotkeyConfig::default(),
            close_to_tray: default_close_to_tray(),
            whisper_accelerator: WhisperAccelerator::Auto,
            whisper_gpu_device: default_whisper_gpu_device(),
            inference_pool: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    pub listen_addr: String,
    pub transcription: TranscriptionConfig,
    pub queue_capacity: usize,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:47813".to_string(),
            transcription: TranscriptionConfig::default(),
            queue_capacity: 4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigPaths {
    pub desktop_config: PathBuf,
    pub api_config: PathBuf,
    pub legacy_configs: Vec<PathBuf>,
}

impl ConfigPaths {
    pub fn new(desktop_config: PathBuf, api_config: PathBuf, legacy_configs: Vec<PathBuf>) -> Self {
        Self {
            desktop_config,
            api_config,
            legacy_configs,
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

impl DesktopConfig {
    fn project_dirs(app_name: &str) -> Result<ProjectDirs> {
        ProjectDirs::from("io", "fractaltess", app_name)
            .context("failed to resolve project directories")
    }

    fn config_paths() -> Result<ConfigPaths> {
        let dirs = Self::project_dirs("shadoword")?;
        let config_dir = dirs.config_dir();
        fs::create_dir_all(config_dir).context("failed to create config directory")?;

        let legacy_shadowword_dirs = Self::project_dirs("shadowword")?;

        Ok(ConfigPaths::new(
            config_dir.join("desktop.json"),
            config_dir.join("api.json"),
            vec![
                config_dir.join("config.json"),
                legacy_shadowword_dirs.config_dir().join("config.json"),
            ],
        ))
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_paths()?.desktop_config)
    }

    pub fn api_config_path() -> Result<PathBuf> {
        Ok(Self::config_paths()?.api_config)
    }

    pub fn models_dir() -> Result<PathBuf> {
        models_dir()
    }

    pub fn load() -> Result<Self> {
        Self::load_from_paths(&Self::config_paths()?)
    }

    pub fn load_from_paths(paths: &ConfigPaths) -> Result<Self> {
        if paths.desktop_config.exists() {
            let raw = fs::read_to_string(&paths.desktop_config).with_context(|| {
                format!(
                    "failed to read desktop config at {}",
                    paths.desktop_config.display()
                )
            })?;
            let config: Self =
                serde_json::from_str(&raw).context("failed to parse desktop config json")?;
            return Ok(config);
        }

        backup_legacy_configs(&paths.legacy_configs)?;
        Ok(Self::default())
    }

    pub fn save(&self) -> Result<()> {
        self.save_to_path(&Self::config_path()?)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        write_json_atomic(path, self, "desktop config")
    }
}

impl ApiConfig {
    pub fn config_path() -> Result<PathBuf> {
        DesktopConfig::api_config_path()
    }

    pub fn models_dir() -> Result<PathBuf> {
        models_dir()
    }

    pub fn load() -> Result<Self> {
        Self::load_from_paths(&DesktopConfig::config_paths()?)
    }

    pub fn load_from_path(path: &Path) -> Result<Self> {
        if path.exists() {
            let raw = fs::read_to_string(path)
                .with_context(|| format!("failed to read api config at {}", path.display()))?;
            let mut config: Self =
                serde_json::from_str(&raw).context("failed to parse api config json")?;
            config.apply_env_overrides();
            return Ok(config);
        }

        let mut config = Self::default();
        config.apply_env_overrides();
        Ok(config)
    }

    pub fn load_from_paths(paths: &ConfigPaths) -> Result<Self> {
        if paths.api_config.exists() {
            let raw = fs::read_to_string(&paths.api_config).with_context(|| {
                format!(
                    "failed to read api config at {}",
                    paths.api_config.display()
                )
            })?;
            let mut config: Self =
                serde_json::from_str(&raw).context("failed to parse api config json")?;
            config.apply_env_overrides();
            return Ok(config);
        }

        backup_legacy_configs(&paths.legacy_configs)?;
        let mut config = Self::default();
        config.apply_env_overrides();
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        self.save_to_path(&Self::config_path()?)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        write_json_atomic(path, self, "API config")
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(listen_addr) = env::var("SHADOWORD_LISTEN_ADDR") {
            let listen_addr = listen_addr.trim();
            if !listen_addr.is_empty() {
                self.listen_addr = listen_addr.to_string();
            }
        }
        if let Ok(queue_capacity) = env::var("SHADOWORD_QUEUE_CAPACITY") {
            let queue_capacity = queue_capacity.trim();
            if !queue_capacity.is_empty() {
                if let Ok(queue_capacity) = queue_capacity.parse() {
                    self.queue_capacity = queue_capacity;
                }
            }
        }
    }
}

impl From<&DesktopConfig> for TranscriptionConfig {
    fn from(config: &DesktopConfig) -> Self {
        Self {
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
}

impl From<ApiConfig> for TranscriptionConfig {
    fn from(config: ApiConfig) -> Self {
        config.transcription
    }
}

impl From<&ApiConfig> for TranscriptionConfig {
    fn from(config: &ApiConfig) -> Self {
        config.transcription.clone()
    }
}

pub fn models_dir() -> Result<PathBuf> {
    if let Ok(path) = env::var("SHADOWORD_MODELS_DIR") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            let override_dir = PathBuf::from(trimmed);
            fs::create_dir_all(&override_dir)
                .context("failed to create models directory override")?;
            return Ok(override_dir);
        }
    }

    let dirs = DesktopConfig::project_dirs("shadoword")?;
    let models_dir = dirs.data_dir().join("models");
    fs::create_dir_all(&models_dir).context("failed to create models directory")?;
    Ok(models_dir)
}

fn write_json_atomic(path: &Path, value: &impl Serialize, label: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let raw =
        serde_json::to_vec_pretty(value).with_context(|| format!("failed to serialize {label}"))?;
    let temporary = path.with_extension("json.tmp");
    if temporary.exists() {
        fs::remove_file(&temporary)
            .with_context(|| format!("failed to remove stale {}", temporary.display()))?;
    }
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options
        .open(&temporary)
        .with_context(|| format!("failed to create {}", temporary.display()))?;
    file.write_all(&raw)
        .with_context(|| format!("failed to write {}", temporary.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to sync {}", temporary.display()))?;
    fs::rename(&temporary, path)
        .with_context(|| format!("failed to replace {}", path.display()))?;
    Ok(())
}

fn backup_legacy_configs(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut backups = Vec::new();
    for path in paths {
        if !path.exists() {
            continue;
        }
        let backup = next_backup_path(path);
        fs::rename(path, &backup).with_context(|| {
            format!(
                "failed to rename legacy config {} to {}",
                path.display(),
                backup.display()
            )
        })?;
        backups.push(backup);
    }
    Ok(backups)
}

fn next_backup_path(path: &Path) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config.json");

    for suffix in 0.. {
        let backup_name = if suffix == 0 {
            format!("{file_name}.backup-{timestamp}")
        } else {
            format!("{file_name}.backup-{timestamp}-{suffix}")
        };
        let backup = path.with_file_name(backup_name);
        if !backup.exists() {
            return backup;
        }
    }

    unreachable!("unbounded backup suffix loop should always return")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_config_paths(test_name: &str) -> ConfigPaths {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let root = env::temp_dir().join(format!("shadoword-{test_name}-{suffix}"));
        fs::create_dir_all(&root).expect("create temp config dir");
        ConfigPaths::new(
            root.join("desktop.json"),
            root.join("api.json"),
            vec![
                root.join("config.json"),
                root.join("shadowword-config.json"),
            ],
        )
    }

    #[test]
    fn desktop_load_renames_legacy_combined_config_and_uses_defaults() {
        let paths = temp_config_paths("desktop-legacy-reset");
        fs::write(
            &paths.legacy_configs[0],
            r#"{"preload_on_startup":false,"remote":{"endpoint":"http://legacy"}}"#,
        )
        .expect("write legacy config");

        let config = DesktopConfig::load_from_paths(&paths).expect("load desktop config");

        assert!(config.preload_on_startup);
        assert_eq!(config.remote.endpoint, RemoteConfig::default().endpoint);
        assert!(!paths.legacy_configs[0].exists());
        let backup_count = fs::read_dir(paths.desktop_config.parent().unwrap())
            .expect("read config dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("config.json.backup-")
            })
            .count();
        assert_eq!(backup_count, 1);
    }

    #[test]
    fn legacy_recording_config_defaults_to_f32_streaming_pcm() {
        let recording: RecordingConfig =
            serde_json::from_str(r#"{"sample_rate":16000,"transcription_mode":"streaming"}"#)
                .expect("deserialize recording config without PCM format");

        assert_eq!(recording.streaming_pcm_format, StreamingPcmFormat::F32le);
    }

    #[test]
    fn legacy_desktop_config_defaults_openrouter_without_exposing_a_key() {
        let config: DesktopConfig = serde_json::from_str(r#"{"mode":"remote"}"#)
            .expect("deserialize desktop config without OpenRouter settings");

        assert_eq!(config.openrouter, OpenRouterConfig::default());
        assert!(!format!("{config:?}").contains("sk-or-test-secret"));
    }

    #[test]
    fn openrouter_debug_output_redacts_the_api_key() {
        let config = OpenRouterConfig {
            api_key: Some("sk-or-test-secret".to_string()),
            model: "openai/whisper-large-v3".to_string(),
        };
        let debug = format!("{config:?}");

        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("sk-or-test-secret"));
    }

    #[test]
    fn api_config_schema_excludes_desktop_owned_settings() {
        let paths = temp_config_paths("api-schema");
        let transcription = TranscriptionConfig {
            english_only: true,
            ..Default::default()
        };
        let config = ApiConfig {
            listen_addr: "0.0.0.0:47813".to_string(),
            transcription,
            queue_capacity: 4,
        };

        config
            .save_to_path(&paths.api_config)
            .expect("save api config");
        let raw = fs::read_to_string(&paths.api_config).expect("read api config");

        assert!(raw.contains("listen_addr"));
        assert!(raw.contains("transcription"));
        assert!(raw.contains("queue_capacity"));
        assert!(!raw.contains("remote"));
        assert!(!raw.contains("output"));
        assert!(!raw.contains("hotkey"));
        assert!(!raw.contains("input_device"));
        assert!(!raw.contains("transcription_mode"));
    }

    #[cfg(unix)]
    #[test]
    fn desktop_config_is_written_atomically_with_private_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let paths = temp_config_paths("desktop-permissions");
        DesktopConfig::default()
            .save_to_path(&paths.desktop_config)
            .expect("save desktop config");

        let mode = fs::metadata(&paths.desktop_config)
            .expect("desktop config metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        assert!(!paths.desktop_config.with_extension("json.tmp").exists());
    }

    fn unit(id: &str, target: ExecutionTarget) -> ExecutionUnitConfig {
        ExecutionUnitConfig {
            id: id.to_string(),
            enabled: true,
            required: true,
            target,
        }
    }

    #[test]
    fn explicit_pool_rejects_invalid_ids_duplicates_auto_gpu_and_empty_enabled_set() {
        let pool = |units| InferencePoolConfig {
            units,
            limits: InferenceLimits::default(),
            ..InferencePoolConfig::default()
        };

        assert!(pool(vec![unit(
            "bad id",
            ExecutionTarget::Gpu {
                device: 0,
                host_threads: None,
            },
        )])
        .validate_with_parallelism(8)
        .is_err());
        assert!(pool(vec![
            unit(
                "same",
                ExecutionTarget::Gpu {
                    device: 0,
                    host_threads: None
                }
            ),
            unit(
                "same",
                ExecutionTarget::Gpu {
                    device: 1,
                    host_threads: None
                }
            ),
        ])
        .validate_with_parallelism(8)
        .is_err());
        assert!(pool(vec![unit(
            "gpu-auto",
            ExecutionTarget::Gpu {
                device: -1,
                host_threads: None
            }
        )])
        .validate_with_parallelism(8)
        .is_err());
        assert!(pool(vec![
            unit(
                "gpu-0",
                ExecutionTarget::Gpu {
                    device: 0,
                    host_threads: None
                }
            ),
            unit(
                "gpu-copy",
                ExecutionTarget::Gpu {
                    device: 0,
                    host_threads: None
                }
            ),
        ])
        .validate_with_parallelism(8)
        .is_err());
        let mut disabled = unit("cpu", ExecutionTarget::Cpu { threads: Some(1) });
        disabled.enabled = false;
        assert!(pool(vec![disabled]).validate_with_parallelism(8).is_err());
    }

    #[test]
    fn cpu_thread_hints_cannot_oversubscribe_available_parallelism() {
        let pool = InferencePoolConfig {
            units: vec![
                unit("cpu-a", ExecutionTarget::Cpu { threads: Some(3) }),
                unit("cpu-b", ExecutionTarget::Cpu { threads: Some(2) }),
            ],
            limits: InferenceLimits::default(),
            ..InferencePoolConfig::default()
        };

        assert!(pool.validate_with_parallelism(4).is_err());
        assert!(pool.validate_with_parallelism(5).is_ok());
    }

    #[test]
    fn gpu_host_threads_participate_in_total_cpu_validation() {
        let pool = InferencePoolConfig {
            units: vec![
                unit("cpu", ExecutionTarget::Cpu { threads: Some(2) }),
                unit(
                    "gpu-a",
                    ExecutionTarget::Gpu {
                        device: 0,
                        host_threads: Some(2),
                    },
                ),
                unit(
                    "gpu-b",
                    ExecutionTarget::Gpu {
                        device: 1,
                        host_threads: None,
                    },
                ),
            ],
            limits: InferenceLimits::default(),
            ..InferencePoolConfig::default()
        };

        assert!(pool.validate_with_parallelism(4).is_err());
        assert!(pool.validate_with_parallelism(5).is_ok());

        let legacy_json = r#"{
            "units":[{"id":"gpu","enabled":true,"required":true,"target":{"kind":"gpu","device":0}}],
            "limits":{}
        }"#;
        let decoded: InferencePoolConfig =
            serde_json::from_str(legacy_json).expect("legacy GPU target without host hint");
        assert_eq!(
            decoded.units[0].target,
            ExecutionTarget::Gpu {
                device: 0,
                host_threads: None,
            }
        );
    }

    #[test]
    fn legacy_scalar_configuration_migrates_to_one_effective_unit() {
        let config: TranscriptionConfig = serde_json::from_str(
            r#"{"whisper_accelerator":"gpu","whisper_gpu_device":2,"english_only":true}"#,
        )
        .expect("deserialize legacy transcription config");

        assert!(config.inference_pool.is_none());
        let pool = config
            .effective_inference_pool()
            .expect("derive legacy pool");
        assert_eq!(pool.units.len(), 1);
        assert_eq!(pool.units[0].id, "legacy");
        assert_eq!(
            pool.units[0].target,
            ExecutionTarget::Gpu {
                device: 2,
                host_threads: None,
            }
        );
        assert!(pool.units[0].required);
    }
}
