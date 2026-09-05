use super::*;

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
    let version = state
        .remote
        .version(&endpoint, token)
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
        daemon_version: version.map(|version| version.version),
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
pub async fn list_remote_tokens(
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<Vec<ApiTokenSummaryDto>> {
    let config = state.config()?;
    ensure_mode(&config, ServiceMode::Remote)?;
    state
        .remote
        .list_tokens(&config.remote.endpoint, config.remote.api_token.as_deref())
        .await
        .map_err(remote_error)
}

/// The secret comes back here and nowhere else — the daemon keeps only a hash —
/// so the caller has to surface it to the operator before the value is dropped.
#[tauri::command]
#[specta::specta]
pub async fn create_remote_token(
    request: CreateApiTokenRequest,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<CreatedApiTokenDto> {
    let config = state.config()?;
    ensure_mode(&config, ServiceMode::Remote)?;
    state
        .remote
        .create_token(
            &config.remote.endpoint,
            config.remote.api_token.as_deref(),
            &request,
        )
        .await
        .map_err(remote_error)
}

#[tauri::command]
#[specta::specta]
pub async fn revoke_remote_token(
    name: String,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<Vec<ApiTokenSummaryDto>> {
    let config = state.config()?;
    ensure_mode(&config, ServiceMode::Remote)?;
    let token = config.remote.api_token.as_deref();
    state
        .remote
        .revoke_token(&config.remote.endpoint, token, &name)
        .await
        .map_err(remote_error)?;
    state
        .remote
        .list_tokens(&config.remote.endpoint, token)
        .await
        .map_err(remote_error)
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
