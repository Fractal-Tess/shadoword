use super::*;

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
