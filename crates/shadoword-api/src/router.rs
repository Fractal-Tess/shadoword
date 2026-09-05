use crate::auth::{generate_token, require_admin, require_transcription, AuthConfig};
use crate::downloads::DownloadJobs;
use crate::error::{ApiError, ApiResult};
use crate::request_recording::RequestRecorder;
use crate::stream;
use axum::body::Bytes;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::response::Response;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Serialize;
use shadoword_core::remote_contracts::{
    ApiTokenSummaryDto, CreateApiTokenRequest, CreatedApiTokenDto, DaemonStatusDto,
    DownloadJobStatus, HealthDto, ModelInfoDto, ModelStorageDto, OverviewDto, RuntimeConfigDto,
    StartDownloadRequest, VersionDto,
};
use shadoword_core::{
    default_whisper_model, list_whisper_gpu_devices, list_whisper_models, resolve_whisper_model,
    unknown_model_error, wav, ApiConfig, ApiTokenConfig, ApiTokenRole, InferenceJob,
    InferenceRuntime, TranscriptResponse, TranscriptionConfig, TranscriptionService,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

mod docs;

use docs::docs;
const INFERENCE_TIMEOUT: Duration = Duration::from_secs(5 * 60);

pub const MAX_RAW_WAV_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone)]
pub struct AppState {
    pub runtime: Arc<InferenceRuntime>,
    pub request_recorder: RequestRecorder,
    pub downloads: DownloadJobs,
    pub config_path: PathBuf,
    pub download_dir: PathBuf,
    pub listen_addr: SocketAddr,
    pub queue_capacity: usize,
    /// Also the token store: handlers read it to rewrite `api.json` and mutate it
    /// to add or revoke tokens, so there is no second copy to keep in step.
    pub auth: AuthConfig,
    pub config_update_lock: Arc<tokio::sync::Mutex<()>>,
}

struct CancelOnDrop {
    job: Arc<InferenceJob>,
    armed: bool,
}

impl CancelOnDrop {
    fn new(job: Arc<InferenceJob>) -> Self {
        Self { job, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if self.armed {
            self.job.cancel();
        }
    }
}

pub fn build_router(state: AppState) -> Router {
    let auth = state.auth.clone();
    let admin = Router::new()
        .route("/", get(docs))
        .route("/docs", get(docs))
        .route("/v1/status", get(status))
        .route("/v1/overview", get(overview))
        .route("/v1/config", get(get_config).put(update_config))
        .route("/v1/models", get(list_models))
        .route("/v1/models/{id}", delete(delete_model))
        .route("/v1/models/{id}/select", post(select_model))
        .route("/v1/downloads", post(start_download))
        .route("/v1/downloads/{id}", get(download_status))
        .route("/v1/tokens", get(list_tokens).post(create_token))
        .route("/v1/tokens/{name}", delete(revoke_token))
        .layer(from_fn_with_state(auth.clone(), require_admin))
        .with_state(state.clone());
    let transcription = Router::new()
        .route("/v1/transcribe-wav", post(transcribe_wav))
        .route("/v1/stream", get(stream_socket))
        .layer(DefaultBodyLimit::max(MAX_RAW_WAV_BYTES))
        .layer(from_fn_with_state(auth, require_transcription))
        .with_state(state.clone());

    Router::new()
        // Unauthenticated on purpose: a client comparing its own build against the
        // daemon's needs an answer before it knows whether its token is any good,
        // so that a stale token and a stale daemon are distinguishable failures.
        .route("/health", get(health))
        .route("/v1/version", get(version))
        .merge(admin)
        .merge(transcription)
        .with_state(state)
}

pub fn runtime_config_from_transcription(
    config: &TranscriptionConfig,
    generation: u64,
) -> RuntimeConfigDto {
    RuntimeConfigDto {
        model_path: resolved_model_path(config).to_string_lossy().into_owned(),
        whisper_accelerator: config.whisper_accelerator,
        whisper_gpu_device: config.whisper_gpu_device,
        english_only: config.english_only,
        preload_on_startup: config.preload_on_startup,
        inference_pool: config.effective_inference_pool().ok(),
        inference_pool_explicit: Some(config.inference_pool.is_some()),
        generation: Some(generation),
    }
}

pub fn apply_runtime_config(
    current: &TranscriptionConfig,
    dto: RuntimeConfigDto,
) -> TranscriptionConfig {
    let mut next = current.clone();
    let unchanged_legacy_pool = dto.inference_pool_explicit.is_none()
        && current.inference_pool.is_none()
        && dto
            .inference_pool
            .as_ref()
            .is_some_and(|pool| current.effective_inference_pool().ok().as_ref() == Some(pool));
    next.model_path = PathBuf::from(dto.model_path);
    next.whisper_accelerator = dto.whisper_accelerator;
    next.whisper_gpu_device = dto.whisper_gpu_device;
    next.english_only = dto.english_only;
    next.preload_on_startup = dto.preload_on_startup;
    match dto.inference_pool_explicit {
        Some(false) => next.inference_pool = None,
        Some(true) => {
            if let Some(pool) = dto.inference_pool {
                next.inference_pool = Some(pool);
                next.legacy_queue_capacity = None;
            }
        }
        None => {
            if let Some(pool) = dto.inference_pool.filter(|_| !unchanged_legacy_pool) {
                next.inference_pool = Some(pool);
                next.legacy_queue_capacity = None;
            }
        }
    }
    next
}

pub fn runtime_transcription_config(config: &ApiConfig) -> TranscriptionConfig {
    let mut transcription = config.transcription.clone();
    if transcription.inference_pool.is_none() {
        transcription.legacy_queue_capacity = Some(config.queue_capacity);
    }
    transcription
}

pub fn resolved_model_path(config: &TranscriptionConfig) -> PathBuf {
    if !config.model_path.as_os_str().is_empty() {
        return config.model_path.clone();
    }
    shadoword_core::ApiConfig::models_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(default_whisper_model().filename)
}

async fn health() -> Json<HealthDto> {
    Json(HealthDto { ok: true })
}

async fn version() -> Json<VersionDto> {
    Json(VersionDto {
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

async fn list_tokens(State(state): State<AppState>) -> Json<Vec<ApiTokenSummaryDto>> {
    Json(state.auth.summaries())
}

async fn create_token(
    State(state): State<AppState>,
    Json(request): Json<CreateApiTokenRequest>,
) -> ApiResult<Json<CreatedApiTokenDto>> {
    let _update = state.config_update_lock.lock().await;
    let (secret, token) = generate_token(request.role, &request.name)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let mut tokens = state.auth.snapshot();
    if tokens.iter().any(|existing| existing.name == token.name) {
        return Err(ApiError::token_conflict(format!(
            "an API token named {:?} already exists; revoke it before issuing a replacement",
            token.name
        )));
    }
    tokens.push(token.clone());
    commit_tokens(&state, tokens)?;
    Ok(Json(CreatedApiTokenDto {
        name: token.name,
        role: token.role,
        token: secret,
    }))
}

async fn revoke_token(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    let _update = state.config_update_lock.lock().await;
    let name = name.trim();
    let mut tokens = state.auth.snapshot();
    let Some(index) = tokens.iter().position(|token| token.name == name) else {
        return Err(ApiError::not_found(format!(
            "no API token named {name:?} exists"
        )));
    };
    // Without this the daemon can be talked out of its own admin access: revoking
    // the last admin token leaves nobody who can issue another one, and the daemon
    // would refuse to start again once the token list emptied out.
    let admins = tokens
        .iter()
        .filter(|token| token.role == ApiTokenRole::Admin)
        .count();
    if tokens[index].role == ApiTokenRole::Admin && admins == 1 {
        return Err(ApiError::token_conflict(
            "issue another admin token before revoking the last one",
        ));
    }
    tokens.remove(index);
    commit_tokens(&state, tokens)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Writes the token list to disk and only then swaps it into the live auth store,
/// so a failed write leaves the daemon enforcing exactly what a restart would.
/// Callers must already hold `config_update_lock`, because this rewrites the same
/// file that runtime and model changes do.
fn commit_tokens(state: &AppState, tokens: Vec<ApiTokenConfig>) -> ApiResult<()> {
    ApiConfig {
        listen_addr: state.listen_addr.to_string(),
        transcription: state.runtime.transcription_config(),
        queue_capacity: state.queue_capacity,
        tokens: tokens.clone(),
    }
    .save_to_path(&state.config_path)
    .map_err(ApiError::internal)?;
    state.auth.replace(&tokens).map_err(ApiError::internal)
}

async fn status(State(state): State<AppState>) -> ApiResult<Json<DaemonStatusDto>> {
    let service =
        TranscriptionService::status(state.runtime.as_ref()).map_err(ApiError::internal)?;
    let pool = state.runtime.status();
    Ok(Json(DaemonStatusDto {
        service,
        in_flight_requests: pool.queued_jobs + pool.running_jobs,
        queue_capacity: state.queue_capacity,
    }))
}

async fn overview(State(state): State<AppState>) -> ApiResult<Json<OverviewDto>> {
    let status =
        TranscriptionService::status(state.runtime.as_ref()).map_err(ApiError::internal)?;
    let pool = state.runtime.status();
    let runtime = runtime_config_from_transcription(
        &state.runtime.transcription_config(),
        state.runtime.generation(),
    );
    let models = model_info(&state);
    let model_storage = Some(model_storage(&state.download_dir, &models));
    Ok(Json(OverviewDto {
        status: DaemonStatusDto {
            service: status,
            in_flight_requests: pool.queued_jobs + pool.running_jobs,
            queue_capacity: state.queue_capacity,
        },
        runtime,
        models,
        model_storage,
    }))
}

async fn get_config(State(state): State<AppState>) -> Json<RuntimeConfigDto> {
    Json(runtime_config_from_transcription(
        &state.runtime.transcription_config(),
        state.runtime.generation(),
    ))
}

async fn update_config(
    State(state): State<AppState>,
    Json(dto): Json<RuntimeConfigDto>,
) -> ApiResult<Json<RuntimeConfigDto>> {
    let _update = state.config_update_lock.lock().await;
    let generation = state.runtime.generation();
    if let Some(expected) = dto.generation {
        if expected != generation {
            return Err(ApiError::conflict(format!(
                "runtime generation changed from {expected} to {generation}; fetch /v1/config and retry"
            )));
        }
    }
    if dto.whisper_gpu_device < -1 {
        return Err(ApiError::bad_request(
            "whisper_gpu_device must be -1 (auto) or a non-negative device index",
        ));
    }
    if dto.whisper_gpu_device >= 0
        && !list_whisper_gpu_devices()
            .iter()
            .any(|device| device.id == dto.whisper_gpu_device)
    {
        return Err(ApiError::bad_request(format!(
            "GPU device {} is not available",
            dto.whisper_gpu_device
        )));
    }
    if dto.inference_pool_explicit == Some(true) && dto.inference_pool.is_none() {
        return Err(ApiError::bad_request(
            "inference_pool is required when inference_pool_explicit is true",
        ));
    }
    let current = state.runtime.transcription_config();
    let next = apply_runtime_config(&current, dto);
    let mut next = next;
    if next.inference_pool.is_none() {
        next.legacy_queue_capacity = Some(state.queue_capacity);
    }
    next.effective_inference_pool()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let runtime = state.runtime.clone();
    let saved = next.clone();
    let config_path = state.config_path.clone();
    let listen_addr = state.listen_addr;
    let queue_capacity = state.queue_capacity;
    let tokens = state.auth.snapshot();

    tokio::task::spawn_blocking(move || {
        runtime.reload_transactional(Some(generation), next, move || {
            ApiConfig {
                listen_addr: listen_addr.to_string(),
                transcription: saved,
                queue_capacity,
                tokens,
            }
            .save_to_path(&config_path)
        })
    })
    .await
    .map_err(ApiError::from_join)?
    .map_err(ApiError::internal)?;

    Ok(Json(runtime_config_from_transcription(
        &state.runtime.transcription_config(),
        state.runtime.generation(),
    )))
}

async fn transcribe_wav(
    State(state): State<AppState>,
    body: Bytes,
) -> ApiResult<Json<TranscriptResponse>> {
    if body.len() > MAX_RAW_WAV_BYTES {
        return Err(ApiError::payload_too_large(
            "raw WAV request body too large",
        ));
    }
    let recorder = state.request_recorder.clone();
    let (input, recording) = tokio::task::spawn_blocking(move || {
        let recording = match recorder.record_wav("batch", &body) {
            Ok(recording) => recording,
            Err(error) => {
                tracing::warn!(error = %error, "failed to archive API audio request");
                None
            }
        };
        match wav::decode_wav(&body) {
            Ok(input) => Ok((input, recording)),
            Err(error) => {
                if let Err(record_error) = recorder.record_error(recording, &error.to_string()) {
                    tracing::warn!(error = %record_error, "failed to archive API response metadata");
                }
                Err(ApiError::bad_request(format!("invalid WAV input: {error}")))
            }
        }
    })
    .await
    .map_err(ApiError::from_join)??;

    let job = match state.runtime.submit_batch(input) {
        Ok(job) => Arc::new(job),
        Err(error) => {
            let message = error.to_string();
            let recorder = state.request_recorder.clone();
            tokio::task::spawn_blocking(move || {
                if let Err(record_error) = recorder.record_error(recording, &message) {
                    tracing::warn!(error = %record_error, "failed to archive API response metadata");
                }
            })
            .await
            .map_err(ApiError::from_join)?;
            return Err(ApiError::from_inference(error));
        }
    };
    let mut cancellation = CancelOnDrop::new(Arc::clone(&job));
    let recorder = state.request_recorder.clone();
    let wait = tokio::task::spawn_blocking(move || {
        let result = job.wait();
        let metadata_result = match &result {
            Ok(completion) => recorder.record_success(recording, &completion.response),
            Err(error) => recorder.record_error(recording, &error.to_string()),
        };
        if let Err(error) = metadata_result {
            tracing::warn!(error = %error, "failed to archive API response metadata");
        }
        result
    });
    let completion = match tokio::time::timeout(INFERENCE_TIMEOUT, wait).await {
        Ok(joined) => joined
            .map_err(ApiError::from_join)?
            .map_err(ApiError::from_inference)?,
        Err(_) => return Err(ApiError::timeout()),
    };
    cancellation.disarm();
    Ok(Json(completion.response))
}

async fn stream_socket(State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    ws.max_message_size(stream::MAX_OPUS_PACKET_BYTES)
        .max_frame_size(stream::MAX_OPUS_PACKET_BYTES)
        .on_upgrade(move |socket| stream::handle_stream(socket, state))
}

async fn list_models(State(state): State<AppState>) -> Json<Vec<ModelInfoDto>> {
    Json(model_info(&state))
}

async fn delete_model(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let _update = state.config_update_lock.lock().await;
    let model = resolve_whisper_model(&id)
        .ok_or_else(|| ApiError::bad_request(unknown_model_error(&id).to_string()))?;
    let path = state.download_dir.join(model.filename);
    if state.runtime.transcription_config().model_path == path {
        return Err(ApiError::conflict(
            "select another model before deleting the active model",
        ));
    }
    if state.downloads.is_active(model.id) {
        return Err(ApiError::conflict(
            "wait for the active model download before deleting it",
        ));
    }
    if !path.is_file() {
        return Err(ApiError::not_found(format!(
            "model '{}' is not installed",
            model.id
        )));
    }
    tokio::fs::remove_file(&path)
        .await
        .map_err(|error| ApiError::internal(error.into()))?;
    Ok(StatusCode::NO_CONTENT)
}

fn model_storage(directory: &std::path::Path, models: &[ModelInfoDto]) -> ModelStorageDto {
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

fn model_info(state: &AppState) -> Vec<ModelInfoDto> {
    list_whisper_models()
        .iter()
        .map(|model| ModelInfoDto {
            id: model.id.to_string(),
            name: model.name.to_string(),
            filename: model.filename.to_string(),
            description: model.description.to_string(),
            size_bytes: model.size_bytes,
            recommended: model.recommended,
            installed: state.download_dir.join(model.filename).is_file(),
        })
        .collect()
}

async fn select_model(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<RuntimeConfigDto>> {
    let _update = state.config_update_lock.lock().await;
    let model = resolve_whisper_model(&id)
        .ok_or_else(|| ApiError::bad_request(unknown_model_error(&id).to_string()))?;
    let model_path = state.download_dir.join(model.filename);
    if !model_path.is_file() {
        return Err(ApiError::not_found(format!(
            "model '{}' is not installed",
            model.id
        )));
    }

    let mut next = state.runtime.transcription_config();
    next.model_path = model_path;
    let runtime = state.runtime.clone();
    let generation = state.runtime.generation();
    let saved = next.clone();
    let config_path = state.config_path.clone();
    let listen_addr = state.listen_addr;
    let queue_capacity = state.queue_capacity;
    let tokens = state.auth.snapshot();

    tokio::task::spawn_blocking(move || {
        runtime.reload_transactional(Some(generation), next, move || {
            ApiConfig {
                listen_addr: listen_addr.to_string(),
                transcription: saved,
                queue_capacity,
                tokens,
            }
            .save_to_path(&config_path)
        })
    })
    .await
    .map_err(ApiError::from_join)?
    .map_err(ApiError::internal)?;

    Ok(Json(runtime_config_from_transcription(
        &state.runtime.transcription_config(),
        state.runtime.generation(),
    )))
}

async fn start_download(
    State(state): State<AppState>,
    Json(request): Json<StartDownloadRequest>,
) -> ApiResult<Json<DownloadJobStatus>> {
    let _update = state.config_update_lock.lock().await;
    state
        .downloads
        .start(request.model_id, state.download_dir)
        .map(Json)
}

async fn download_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<DownloadJobStatus>> {
    state.downloads.get(&id).map(Json)
}
