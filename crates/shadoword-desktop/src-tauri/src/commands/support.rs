use super::*;

pub(super) fn apply_settings(
    config: &mut DesktopConfig,
    input: DesktopSettingsInput,
) -> Result<()> {
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

pub(super) fn ensure_mode(config: &DesktopConfig, expected: ServiceMode) -> CommandResult<()> {
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

pub(super) fn register_hotkey(state: &DesktopState, shortcut: &str) -> CommandResult<()> {
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

pub(super) async fn refresh_remote(
    remote: &RemoteClient,
    config: &DesktopConfig,
) -> CommandResult<OverviewDto> {
    remote
        .overview(&config.remote.endpoint, config.remote.api_token.as_deref())
        .await
        .map_err(remote_error)
}

pub(super) fn local_transcription_config(config: &DesktopConfig) -> TranscriptionConfig {
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

pub(super) async fn persist_local_service<F>(
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
        let entering_local_without_model = current.mode != ServiceMode::Local
            && next.mode == ServiceMode::Local
            && !next.model_path.is_file();
        if next.mode != ServiceMode::Local
            || entering_local_without_model
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
pub(super) fn stale_generation_error(expected: u64, generation: u64) -> DesktopError {
    DesktopError::new(
        "stale_runtime_generation",
        format!("stale runtime generation: expected {expected}, active generation is {generation}"),
    )
    .with_action("Refresh the local runtime overview and retry the change.")
}

#[cfg(feature = "local-runtime")]
pub(super) fn reload_local_candidate(
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

pub(super) fn validate_runtime(runtime: &RuntimeConfigDto) -> CommandResult<()> {
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
pub(super) fn validate_pool_devices(pool: &InferencePoolConfig) -> CommandResult<()> {
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

pub(super) fn local_overview(state: &DesktopState) -> CommandResult<OverviewDto> {
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
pub(super) fn model_storage(
    directory: &std::path::Path,
    models: &[ModelInfoDto],
) -> ModelStorageDto {
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
pub(super) fn update_download(
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

pub(super) fn busy_recording_error() -> DesktopError {
    DesktopError::new("recording_busy", "a recording is already active")
        .with_action("Stop or cancel it before starting another recording.")
}

pub(super) fn internal_error(message: impl Into<String>) -> DesktopError {
    DesktopError::new("internal_state_error", message)
}

pub(super) fn internal_error_from(error: impl std::fmt::Display) -> DesktopError {
    internal_error(error.to_string())
}

pub(super) fn config_error(error: impl std::fmt::Display) -> DesktopError {
    DesktopError::new("invalid_configuration", error.to_string())
        .with_action("Correct the highlighted setting and try again.")
}

pub(super) fn validate_openrouter_config(config: &DesktopConfig) -> CommandResult<()> {
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

pub(super) fn configured_secret(
    config: &DesktopConfig,
    kind: DesktopSecretKind,
) -> CommandResult<String> {
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

pub(super) fn openrouter_key_required() -> DesktopError {
    DesktopError::new(
        "openrouter_key_required",
        "an OpenRouter API key is required",
    )
    .with_action("Enter an OpenRouter API key in Settings and save the configuration.")
}

pub(crate) fn openrouter_error(error: impl std::fmt::Display) -> DesktopError {
    DesktopError::new("openrouter_transcription_failed", error.to_string()).with_action(
        "Check the OpenRouter API key, transcription model, account credits, and network connection.",
    )
}

pub(super) fn remote_error(error: anyhow::Error) -> DesktopError {
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
        if remote.code() == "token_management_unsupported" {
            return DesktopError::new("remote_token_management_unsupported", error.to_string())
            .with_action(
                "Update and restart the Shadoword API daemon, or manage tokens with the shadoword-api token command.",
            );
        }
        if remote.code() == "token_conflict" {
            return DesktopError::new("remote_token_conflict", error.to_string()).with_action(
                "Choose a different token name, or issue a replacement before revoking this one.",
            );
        }
        if remote.code() == "forbidden" {
            return DesktopError::new("remote_permission_denied", error.to_string()).with_action(
                "Use an admin token for desktop management; user tokens can only transcribe audio.",
            );
        }
        // Every daemon requires a token now, including on loopback, so this is the
        // expected failure for a configuration that predates that rule.
        if remote.code() == "unauthorized" {
            return DesktopError::new("remote_unauthorized", error.to_string()).with_action(
            "Set an API token for this daemon. Issue one with `shadoword-api token generate admin <name>` on the machine running it.",
        );
        }
    }
    DesktopError::new("remote_request_failed", error.to_string())
        .with_action("Check the endpoint, bearer token, daemon status, and network connection.")
}

pub(super) fn local_error(error: impl std::fmt::Display) -> DesktopError {
    DesktopError::new("local_runtime_failed", error.to_string())
        .with_action("Download or select a valid model and verify accelerator settings.")
}

pub(super) fn join_error(error: impl std::fmt::Display) -> DesktopError {
    DesktopError::new("native_task_failed", error.to_string())
}

#[cfg(not(feature = "local-runtime"))]
pub(super) fn unavailable_local<T>() -> CommandResult<T> {
    Err(DesktopError::new(
        "local_runtime_not_compiled",
        "this desktop build does not include the local runtime feature",
    )
    .with_action("Rebuild Shadoword Desktop with the local-runtime feature."))
}
