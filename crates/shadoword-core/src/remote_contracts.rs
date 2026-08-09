use crate::{ApiTokenRole, InferencePoolConfig, ServiceStatus, WhisperAccelerator};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
pub struct HealthDto {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
pub struct VersionDto {
    /// The daemon's own crate version, so a client can tell whether an endpoint
    /// it wants to call exists before it tries and reads a 404 as a network fault.
    pub version: String,
}

/// What an admin is allowed to see about a token that already exists. The stored
/// hash never leaves the daemon: it is not a secret a client can use, but it is
/// an offline-guessable one, and nothing here needs it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
pub struct ApiTokenSummaryDto {
    pub name: String,
    pub role: ApiTokenRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
pub struct CreateApiTokenRequest {
    pub name: String,
    pub role: ApiTokenRole,
}

/// The only time the secret is ever transmitted. The daemon keeps a hash, so a
/// caller that loses this value has to revoke the token and issue another one.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
pub struct CreatedApiTokenDto {
    pub name: String,
    pub role: ApiTokenRole,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DaemonStatusDto {
    #[serde(flatten)]
    pub service: ServiceStatus,
    #[specta(type = u32)]
    pub in_flight_requests: usize,
    #[specta(type = u32)]
    pub queue_capacity: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
pub struct RuntimeConfigDto {
    pub model_path: String,
    pub whisper_accelerator: WhisperAccelerator,
    pub whisper_gpu_device: i32,
    pub english_only: bool,
    pub preload_on_startup: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference_pool: Option<InferencePoolConfig>,
    /// `None` preserves legacy client behavior, while `Some(false)` explicitly
    /// clears a persisted pool and selects the derived single-unit config.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference_pool_explicit: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[specta(type = Option<f64>)]
    pub generation: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
pub struct ModelInfoDto {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub description: String,
    #[specta(type = u32)]
    pub size_bytes: u64,
    pub recommended: bool,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
pub struct ModelStorageDto {
    pub directory: String,
    #[specta(type = f64)]
    pub total_bytes: u64,
    #[specta(type = u32)]
    pub installed_model_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OverviewDto {
    pub status: DaemonStatusDto,
    pub runtime: RuntimeConfigDto,
    pub models: Vec<ModelInfoDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_storage: Option<ModelStorageDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum DownloadJobState {
    Queued,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DownloadJobStatus {
    pub id: String,
    pub model_id: String,
    pub state: DownloadJobState,
    #[specta(type = u32)]
    pub downloaded: u64,
    #[specta(type = u32)]
    pub total: u64,
    pub path: Option<String>,
    pub skipped: bool,
    pub verified: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
pub struct StartDownloadRequest {
    pub model_id: String,
}
