use anyhow::{Context, Result};
use shadoword_core::{
    AudioInput, DeviceListResponse, ServiceStatus, ShadowwordConfig, TranscriptResponse,
};

// ── Remote helpers ──────────────────────────────────────────────────────────

#[allow(dead_code)]
pub(crate) fn remote_transcribe(
    config: &ShadowwordConfig,
    audio: AudioInput,
) -> Result<TranscriptResponse> {
    let client = reqwest::blocking::Client::new();
    let wav = shadoword_core::wav::encode_wav(&audio)?;
    let response = client
        .post(format!("{}/v1/transcribe-wav", config.remote.endpoint))
        .header("content-type", "audio/wav")
        .body(wav)
        .send()
        .context("failed to reach remote daemon")?
        .error_for_status()
        .context("daemon returned an error status")?
        .json::<TranscriptResponse>()
        .context("failed to decode daemon response")?;
    Ok(response)
}

pub(crate) fn remote_status(config: &ShadowwordConfig) -> Result<ServiceStatus> {
    reqwest::blocking::get(format!("{}/v1/status", config.remote.endpoint))
        .context("failed to reach remote daemon")?
        .error_for_status()
        .context("daemon returned an error status")?
        .json::<ServiceStatus>()
        .context("failed to decode daemon status")
}

pub(crate) fn remote_get_config(config: &ShadowwordConfig) -> Result<ShadowwordConfig> {
    reqwest::blocking::get(format!("{}/v1/config", config.remote.endpoint))
        .context("failed to reach remote daemon")?
        .error_for_status()
        .context("daemon returned an error status")?
        .json::<ShadowwordConfig>()
        .context("failed to decode daemon config")
}

pub(crate) fn remote_update_config(
    current: &ShadowwordConfig,
    next: &ShadowwordConfig,
) -> Result<ShadowwordConfig> {
    let client = reqwest::blocking::Client::new();
    client
        .put(format!("{}/v1/config", current.remote.endpoint))
        .json(next)
        .send()
        .context("failed to reach remote daemon")?
        .error_for_status()
        .context("daemon returned an error status")?
        .json::<ShadowwordConfig>()
        .context("failed to decode updated daemon config")
}

#[allow(dead_code)]
pub(crate) fn remote_devices(config: &ShadowwordConfig) -> Result<DeviceListResponse> {
    reqwest::blocking::get(format!("{}/v1/devices", config.remote.endpoint))
        .context("failed to reach remote daemon")?
        .error_for_status()
        .context("daemon returned an error status")?
        .json::<DeviceListResponse>()
        .context("failed to decode daemon device list")
}
