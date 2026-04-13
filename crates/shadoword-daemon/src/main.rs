use axum::body::Bytes;
use axum::extract::State;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::{Json, Router};
use shadoword_core::{
    DeviceListResponse, LocalService, ServiceHealth, ServiceStatus, ShadowwordConfig,
    TranscriptRequest, TranscriptResponse, TranscriptionService,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    service: Arc<LocalService>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = ShadowwordConfig::load()?;
    let addr: SocketAddr = config.daemon.listen_addr.parse()?;
    let state = AppState {
        service: Arc::new(LocalService::new(config)),
    };

    if state.service.config().preload_on_startup {
        tracing::info!("preload_on_startup enabled, loading model before serving requests");
        state.service.preload()?;
        tracing::info!("daemon model preload complete");
    }

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/status", get(status))
        .route("/v1/config", get(get_config).put(update_config))
        .route("/v1/devices", get(list_devices))
        .route("/v1/transcribe", post(transcribe))
        .route("/v1/transcribe-wav", post(transcribe_wav))
        .layer(DefaultBodyLimit::max(256 * 1024 * 1024))
        .with_state(state);

    tracing::info!("shadoword-daemon listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
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
