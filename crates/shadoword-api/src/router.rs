use crate::auth::{require_auth, AuthConfig};
use crate::downloads::DownloadJobs;
use crate::error::{ApiError, ApiResult};
use crate::request_recording::RequestRecorder;
use crate::stream;
use axum::body::Bytes;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::middleware::from_fn_with_state;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use shadoword_core::remote_contracts::{
    DaemonStatusDto, DownloadJobStatus, HealthDto, ModelInfoDto, OverviewDto, RuntimeConfigDto,
    StartDownloadRequest,
};
use shadoword_core::{
    default_whisper_model, list_whisper_gpu_devices, list_whisper_models, resolve_whisper_model,
    unknown_model_error, wav, ApiConfig, InferenceJob, InferenceRuntime, TranscriptResponse,
    TranscriptionConfig, TranscriptionService,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

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
    pub config_update_lock: Arc<tokio::sync::Mutex<()>>,
}

#[derive(Clone)]
pub struct RouterConfig {
    pub auth: AuthConfig,
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

#[derive(Debug, Serialize)]
struct DaemonDocs {
    endpoints: Vec<DocEndpoint>,
    env: Vec<DocEnvVar>,
    limits: DocsLimits,
    whisper_models: Vec<DocModel>,
}

#[derive(Debug, Serialize)]
struct DocEndpoint {
    method: &'static str,
    path: &'static str,
    description: &'static str,
}

#[derive(Debug, Serialize)]
struct DocEnvVar {
    name: &'static str,
    description: &'static str,
    example: &'static str,
}

#[derive(Debug, Serialize)]
struct DocModel {
    id: &'static str,
    filename: &'static str,
    description: &'static str,
    size_bytes: u64,
    recommended: bool,
}

#[derive(Debug, Serialize)]
struct DocsLimits {
    raw_wav_bytes: usize,
    default_decoded_audio_bytes_per_job: usize,
    opus_packet_bytes: usize,
    raw_pcm_max_packet_bytes: usize,
    raw_pcm_max_packet_milliseconds: usize,
    pcm_f32le_wire_bytes_per_sample: usize,
    pcm_s16le_wire_bytes_per_sample: usize,
    decoded_pcm_bytes_per_sample: usize,
    stream_segment_seconds: usize,
    stream_max_segments: usize,
    stream_idle_seconds: u64,
}

pub fn build_router(state: AppState, config: RouterConfig) -> Router {
    let protected = Router::new()
        .route("/", get(docs))
        .route("/docs", get(docs))
        .route("/v1/status", get(status))
        .route("/v1/overview", get(overview))
        .route("/v1/config", get(get_config).put(update_config))
        .route("/v1/transcribe-wav", post(transcribe_wav))
        .route("/v1/stream", get(stream_socket))
        .route("/v1/models", get(list_models))
        .route("/v1/models/{id}/select", post(select_model))
        .route("/v1/downloads", post(start_download))
        .route("/v1/downloads/{id}", get(download_status))
        .layer(DefaultBodyLimit::max(MAX_RAW_WAV_BYTES))
        .with_state(state.clone());

    let protected = if config.auth.is_configured() {
        protected.layer(from_fn_with_state(config.auth, require_auth))
    } else {
        protected
    };

    Router::new()
        .route("/health", get(health))
        .merge(protected)
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

async fn docs() -> Json<DaemonDocs> {
    Json(DaemonDocs {
        endpoints: vec![
            DocEndpoint {
                method: "GET",
                path: "/health",
                description: "Public health check.",
            },
            DocEndpoint {
                method: "GET",
                path: "/v1/status",
                description: "Authenticated daemon and inference-pool status, including generation, units, queued/running work, bytes, and counters.",
            },
            DocEndpoint {
                method: "GET",
                path: "/v1/overview",
                description: "Authenticated combined runtime, status, and model catalog snapshot.",
            },
            DocEndpoint {
                method: "GET",
                path: "/v1/config",
                description: "Authenticated runtime config with effective inference_pool and generation. Units preload eagerly only when preload_on_startup is true; otherwise they load on first dispatch.",
            },
            DocEndpoint {
                method: "PUT",
                path: "/v1/config",
                description: "Authenticated atomic runtime update. Accepts inference_pool and an optional generation revision; eager candidates prepare before commit and lazy candidates activate unloaded.",
            },
            DocEndpoint {
                method: "POST",
                path: "/v1/transcribe-wav",
                description: "Authenticated raw WAV batch transcription.",
            },
            DocEndpoint {
                method: "GET",
                path: "/v1/stream",
                description: "Authenticated WebSocket stream. Protocol v1 remains lockstep; v2 negotiates indexed concurrent Opus segments. For v3, Start includes protocol_version=3, audio_format=pcm_f32le or pcm_s16le, sample_rate, and channels=1; binary messages contain little-endian mono PCM and the server performs VAD, segmentation, bounded scheduling, and ordered partial delivery.",
            },
            DocEndpoint {
                method: "GET",
                path: "/v1/models",
                description: "Authenticated catalog and installation status.",
            },
            DocEndpoint {
                method: "POST",
                path: "/v1/models/{id}/select",
                description: "Authenticated selection of an installed catalog model.",
            },
            DocEndpoint {
                method: "POST",
                path: "/v1/downloads",
                description: "Authenticated async catalog model download job.",
            },
            DocEndpoint {
                method: "GET",
                path: "/v1/downloads/{id}",
                description: "Authenticated model download job status.",
            },
        ],
        env: vec![
            DocEnvVar {
                name: "SHADOWORD_API_CONFIG",
                description: "API config path.",
                example: "/config/shadoword/api.json",
            },
            DocEnvVar {
                name: "SHADOWORD_LISTEN_ADDR",
                description: "API bind address. Non-loopback binds require bearer auth.",
                example: "0.0.0.0:47813",
            },
            DocEnvVar {
                name: "SHADOWORD_API_TOKEN",
                description: "Bearer token value. Never returned by the API.",
                example: "<secret>",
            },
            DocEnvVar {
                name: "SHADOWORD_API_TOKEN_FILE",
                description: "Path to a mode-0600 bearer token file.",
                example: "/run/secrets/shadoword-api-token",
            },
            DocEnvVar {
                name: "SHADOWORD_MODEL_PATH",
                description: "Selected Whisper model file.",
                example: "/data/shadoword/models/ggml-large-v3-turbo.bin",
            },
            DocEnvVar {
                name: "SHADOWORD_DOWNLOAD_MODELS",
                description: "Comma-separated catalog model ids to download at startup.",
                example: "turbo",
            },
            DocEnvVar {
                name: "SHADOWORD_QUEUE_CAPACITY",
                description: "Legacy queue limit. When inference_pool is absent, maps to pool max_queued_jobs (zero uses one worker hand-off slot).",
                example: "4",
            },
            DocEnvVar {
                name: "SHADOWORD_REQUEST_RECORDING_DIR",
                description: "Optional directory for WAV copies of accepted transcription requests and JSON response metadata.",
                example: "/var/lib/shadoword/requests",
            },
        ],
        limits: DocsLimits {
            raw_wav_bytes: MAX_RAW_WAV_BYTES,
            default_decoded_audio_bytes_per_job: shadoword_core::InferenceLimits::default()
                .max_audio_bytes_per_job,
            opus_packet_bytes: stream::MAX_OPUS_PACKET_BYTES,
            raw_pcm_max_packet_bytes: stream::MAX_OPUS_PACKET_BYTES,
            raw_pcm_max_packet_milliseconds: stream::MAX_PCM_PACKET_MILLISECONDS,
            pcm_f32le_wire_bytes_per_sample: stream::PCM_F32LE_WIRE_BYTES_PER_SAMPLE,
            pcm_s16le_wire_bytes_per_sample: stream::PCM_S16LE_WIRE_BYTES_PER_SAMPLE,
            decoded_pcm_bytes_per_sample: stream::DECODED_PCM_BYTES_PER_SAMPLE,
            stream_segment_seconds: stream::MAX_SEGMENT_SECONDS,
            stream_max_segments: stream::MAX_STREAM_SEGMENTS,
            stream_idle_seconds: stream::STREAM_IDLE_TIMEOUT.as_secs(),
        },
        whisper_models: list_whisper_models()
            .iter()
            .map(|model| DocModel {
                id: model.id,
                filename: model.filename,
                description: model.description,
                size_bytes: model.size_bytes,
                recommended: model.recommended,
            })
            .collect(),
    })
}

async fn health() -> Json<HealthDto> {
    Json(HealthDto { ok: true })
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
    Ok(Json(OverviewDto {
        status: DaemonStatusDto {
            service: status,
            in_flight_requests: pool.queued_jobs + pool.running_jobs,
            queue_capacity: state.queue_capacity,
        },
        runtime,
        models: model_info(&state),
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

    tokio::task::spawn_blocking(move || {
        runtime.reload_transactional(Some(generation), next, move || {
            ApiConfig {
                listen_addr: listen_addr.to_string(),
                transcription: saved,
                queue_capacity,
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

    tokio::task::spawn_blocking(move || {
        runtime.reload_transactional(Some(generation), next, move || {
            ApiConfig {
                listen_addr: listen_addr.to_string(),
                transcription: saved,
                queue_capacity,
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
