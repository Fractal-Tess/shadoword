use super::*;

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

/// Durable state that is *not* configuration — anything the user produced rather
/// than chose. It goes to the data dir instead of the config dir so that wiping a
/// broken config never takes a transcript history with it.
pub fn data_dir() -> Result<PathBuf> {
    let dirs = DesktopConfig::project_dirs("shadoword")?;
    let data_dir = dirs.data_dir().to_path_buf();
    fs::create_dir_all(&data_dir).context("failed to create data directory")?;
    Ok(data_dir)
}

pub fn write_json_atomic(path: &Path, value: &impl Serialize, label: &str) -> Result<()> {
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
