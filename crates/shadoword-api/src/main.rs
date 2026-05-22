use axum::body::Bytes;
use axum::extract::State;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::{Json, Router};
use shadoword_core::{
    DeviceListResponse, LocalService, ModelDownloadStatus, ServiceHealth, ServiceStatus,
    ShadowwordConfig, TranscriptRequest, TranscriptResponse, TranscriptionService,
    download_whisper_model, list_whisper_models, parse_requested_models, resolve_download_dir,
    resolve_whisper_model, unknown_model_error,
};
use serde::Serialize;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    service: Arc<LocalService>,
}

#[derive(Debug, Clone, Serialize)]
struct DaemonDocs {
    default_download_dir: String,
    endpoints: Vec<DocEndpoint>,
    env: Vec<DocEnvVar>,
    whisper_models: Vec<DocModel>,
}

#[derive(Debug, Clone, Serialize)]
struct DocEndpoint {
    method: &'static str,
    path: &'static str,
    description: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct DocEnvVar {
    name: &'static str,
    description: &'static str,
    example: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct DocModel {
    id: &'static str,
    filename: &'static str,
    url: &'static str,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = ShadowwordConfig::load()?;
    let downloads = download_requested_models(&config)?;
    let addr: SocketAddr = config.daemon.listen_addr.parse()?;
    let state = AppState {
        service: Arc::new(LocalService::new(config)),
    };

    if !downloads.is_empty() {
        tracing::info!(downloaded = downloads.len(), "daemon model download step complete");
    }

    if state.service.config().preload_on_startup {
        tracing::info!("preload_on_startup enabled, loading model before serving requests");
        state.service.preload()?;
        tracing::info!("daemon model preload complete");
    }

    let app = Router::new()
        .route("/", get(docs))
        .route("/docs", get(docs))
        .route("/health", get(health))
        .route("/v1/status", get(status))
        .route("/v1/config", get(get_config).put(update_config))
        .route("/v1/devices", get(list_devices))
        .route("/v1/transcribe", post(transcribe))
        .route("/v1/transcribe-wav", post(transcribe_wav))
        .layer(DefaultBodyLimit::max(256 * 1024 * 1024))
        .with_state(state);

    tracing::info!("shadoword-api listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn docs() -> Json<DaemonDocs> {
    let default_download_dir = ShadowwordConfig::models_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "<unavailable>".to_string());
    let whisper_models = list_whisper_models()
        .iter()
        .map(|model| DocModel {
            id: model.id,
            filename: model.filename,
            url: model.url,
        })
        .collect();

    Json(DaemonDocs {
        default_download_dir,
        endpoints: vec![
            DocEndpoint {
                method: "GET",
                path: "/",
                description: "Return daemon docs and supported Whisper model ids.",
            },
            DocEndpoint {
                method: "GET",
                path: "/docs",
                description: "Return daemon docs and supported Whisper model ids.",
            },
            DocEndpoint {
                method: "GET",
                path: "/health",
                description: "Return a simple health response.",
            },
            DocEndpoint {
                method: "GET",
                path: "/v1/status",
                description: "Return current daemon status and loaded model state.",
            },
            DocEndpoint {
                method: "GET",
                path: "/v1/config",
                description: "Return the active daemon config.",
            },
            DocEndpoint {
                method: "PUT",
                path: "/v1/config",
                description: "Replace the active daemon config.",
            },
            DocEndpoint {
                method: "GET",
                path: "/v1/devices",
                description: "List local input devices visible to the daemon host.",
            },
            DocEndpoint {
                method: "POST",
                path: "/v1/transcribe",
                description: "Transcribe a base64-encoded WAV payload.",
            },
            DocEndpoint {
                method: "POST",
                path: "/v1/transcribe-wav",
                description: "Transcribe raw WAV bytes from the request body.",
            },
        ],
        env: vec![
            DocEnvVar {
                name: "SHADOWORD_LISTEN_ADDR",
                description: "Override the daemon bind address.",
                example: "0.0.0.0:47813",
            },
            DocEnvVar {
                name: "SHADOWORD_DOWNLOAD_MODELS",
                description: "Comma-separated Whisper model ids to download at startup.",
                example: "tiny,base,small,medium,turbo,large-v3",
            },
            DocEnvVar {
                name: "SHADOWORD_DOWNLOAD_DIR",
                description: "Target directory for startup model downloads. If omitted, the daemon uses the default data models directory shown above.",
                example: "/data/shadoword/models",
            },
        ],
        whisper_models,
    })
}

async fn health() -> Json<ServiceHealth> {
    Json(ServiceHealth { ok: true })
}

async fn status(State(state): State<AppState>) -> Result<Json<ServiceStatus>, (axum::http::StatusCode, String)> {
    state
        .service
        .status()
        .map(Json)
        .map_err(internal_error)
}

async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<ShadowwordConfig>, (axum::http::StatusCode, String)> {
    Ok(Json(state.service.config()))
}

async fn update_config(
    State(state): State<AppState>,
    Json(config): Json<ShadowwordConfig>,
) -> Result<Json<ShadowwordConfig>, (axum::http::StatusCode, String)> {
    state
        .service
        .update_config(config)
        .map_err(internal_error)?;
    Ok(Json(state.service.config()))
}

async fn list_devices(
    State(state): State<AppState>,
) -> Result<Json<DeviceListResponse>, (axum::http::StatusCode, String)> {
    state
        .service
        .list_input_devices()
        .map(Json)
        .map_err(internal_error)
}

async fn transcribe(
    State(state): State<AppState>,
    Json(request): Json<TranscriptRequest>,
) -> Result<Json<TranscriptResponse>, (axum::http::StatusCode, String)> {
    state
        .service
        .transcribe_wav_base64(request)
        .map(Json)
        .map_err(internal_error)
}

async fn transcribe_wav(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<TranscriptResponse>, (axum::http::StatusCode, String)> {
    state
        .service
        .transcribe_wav_bytes(&body)
        .map(Json)
        .map_err(internal_error)
}

fn internal_error(error: anyhow::Error) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        error.to_string(),
    )
}

fn download_requested_models(config: &ShadowwordConfig) -> anyhow::Result<Vec<ModelDownloadStatus>> {
    let requested = env::var("SHADOWORD_DOWNLOAD_MODELS")
        .ok()
        .map(|value| parse_requested_models(&value))
        .unwrap_or_default();

    if requested.is_empty() {
        return Ok(Vec::new());
    }

    let target_dir = resolve_download_dir(
        env::var("SHADOWORD_DOWNLOAD_DIR").ok(),
        ShadowwordConfig::models_dir()?,
    );

    let mut results = Vec::with_capacity(requested.len());
    for key in requested {
        let spec = resolve_whisper_model(&key).ok_or_else(|| unknown_model_error(&key))?;
        let status = download_whisper_model(spec, &target_dir)?;
        tracing::info!(
            model = spec.id,
            filename = spec.filename,
            path = %status.path,
            skipped = status.skipped,
            "startup whisper model download"
        );
        results.push(status);
    }

    if config.model_path.as_os_str().is_empty() {
        tracing::info!(
            dir = %target_dir.display(),
            "startup model downloads completed, but config.model_path is still empty"
        );
    }

    Ok(results)
}
