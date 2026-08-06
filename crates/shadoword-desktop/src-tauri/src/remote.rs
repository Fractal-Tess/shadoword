use anyhow::{anyhow, Context, Result};
use reqwest::{Client, Method, Response, Url};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use shadoword_core::remote_contracts::{
    DaemonStatusDto, DownloadJobStatus, HealthDto, OverviewDto, RuntimeConfigDto,
    StartDownloadRequest,
};
use shadoword_core::TranscriptResponse;
use std::time::Duration;

const REMOTE_TIMEOUT: Duration = Duration::from_secs(10);
const TRANSCRIBE_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    error: String,
    message: String,
}

#[derive(Debug)]
pub(crate) struct RemoteApiError {
    status: reqwest::StatusCode,
    code: String,
    message: String,
}

impl RemoteApiError {
    pub(crate) fn code(&self) -> &str {
        &self.code
    }
}

impl std::fmt::Display for RemoteApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Shadoword API returned {} ({}): {}",
            self.status, self.code, self.message
        )
    }
}

impl std::error::Error for RemoteApiError {}

#[derive(Clone)]
pub struct RemoteClient {
    client: Client,
}

impl RemoteClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .user_agent(concat!("shadoword-desktop/", env!("CARGO_PKG_VERSION")))
                .build()
                .context("failed to build remote HTTP client")?,
        })
    }

    pub fn validate_endpoint(endpoint: &str) -> Result<String> {
        let endpoint = endpoint.trim().trim_end_matches('/');
        let parsed = Url::parse(endpoint).context("API endpoint must be an absolute URL")?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(anyhow!("API endpoint must use http or https"));
        }
        if parsed.host_str().is_none() {
            return Err(anyhow!("API endpoint must include a host"));
        }
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(anyhow!("API endpoint must not contain credentials"));
        }
        Ok(endpoint.to_string())
    }

    pub async fn health(&self, endpoint: &str, token: Option<&str>) -> Result<HealthDto> {
        self.get(endpoint, token, &["health"]).await
    }

    pub async fn status(&self, endpoint: &str, token: Option<&str>) -> Result<DaemonStatusDto> {
        self.get(endpoint, token, &["v1", "status"]).await
    }

    pub async fn overview(&self, endpoint: &str, token: Option<&str>) -> Result<OverviewDto> {
        self.get(endpoint, token, &["v1", "overview"]).await
    }

    pub async fn runtime_config(
        &self,
        endpoint: &str,
        token: Option<&str>,
    ) -> Result<RuntimeConfigDto> {
        self.get(endpoint, token, &["v1", "config"]).await
    }

    pub async fn update_runtime(
        &self,
        endpoint: &str,
        token: Option<&str>,
        runtime: &RuntimeConfigDto,
    ) -> Result<RuntimeConfigDto> {
        let response = self
            .request(endpoint, token, Method::PUT, &["v1", "config"])?
            .json(runtime)
            .timeout(TRANSCRIBE_TIMEOUT)
            .send()
            .await
            .context("failed to reach the remote API")?;
        decode(response).await
    }

    pub async fn select_model(
        &self,
        endpoint: &str,
        token: Option<&str>,
        model_id: &str,
    ) -> Result<RuntimeConfigDto> {
        let response = self
            .request(
                endpoint,
                token,
                Method::POST,
                &["v1", "models", model_id, "select"],
            )?
            .timeout(TRANSCRIBE_TIMEOUT)
            .send()
            .await
            .context("failed to reach the remote API")?;
        decode(response).await
    }

    pub async fn delete_model(
        &self,
        endpoint: &str,
        token: Option<&str>,
        model_id: &str,
    ) -> Result<()> {
        let response = self
            .request(endpoint, token, Method::DELETE, &["v1", "models", model_id])?
            .timeout(REMOTE_TIMEOUT)
            .send()
            .await
            .context("failed to reach the remote API")?;
        decode_empty(response).await
    }

    pub async fn start_download(
        &self,
        endpoint: &str,
        token: Option<&str>,
        model_id: String,
    ) -> Result<DownloadJobStatus> {
        let response = self
            .request(endpoint, token, Method::POST, &["v1", "downloads"])?
            .json(&StartDownloadRequest { model_id })
            .timeout(REMOTE_TIMEOUT)
            .send()
            .await
            .context("failed to reach the remote API")?;
        decode(response).await
    }

    pub async fn download_status(
        &self,
        endpoint: &str,
        token: Option<&str>,
        job_id: &str,
    ) -> Result<DownloadJobStatus> {
        self.get(endpoint, token, &["v1", "downloads", job_id])
            .await
    }

    pub async fn transcribe_wav(
        &self,
        endpoint: &str,
        token: Option<&str>,
        wav: Vec<u8>,
    ) -> Result<TranscriptResponse> {
        let response = self
            .request(endpoint, token, Method::POST, &["v1", "transcribe-wav"])?
            .header("content-type", "audio/wav")
            .body(wav)
            .timeout(TRANSCRIBE_TIMEOUT)
            .send()
            .await
            .context("failed to reach the remote API")?;
        decode(response).await
    }

    async fn get<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        token: Option<&str>,
        segments: &[&str],
    ) -> Result<T> {
        let response = self
            .request(endpoint, token, Method::GET, segments)?
            .timeout(REMOTE_TIMEOUT)
            .send()
            .await
            .context("failed to reach the remote API")?;
        decode(response).await
    }

    fn request(
        &self,
        endpoint: &str,
        token: Option<&str>,
        method: Method,
        segments: &[&str],
    ) -> Result<reqwest::RequestBuilder> {
        let normalized = Self::validate_endpoint(endpoint)?;
        let mut url = Url::parse(&normalized).context("invalid API endpoint")?;
        {
            let mut path = url
                .path_segments_mut()
                .map_err(|_| anyhow!("API endpoint cannot be used as a base URL"))?;
            path.pop_if_empty();
            for segment in segments {
                path.push(segment);
            }
        }
        let request = self.client.request(method, url);
        Ok(match token.map(str::trim) {
            Some(token) if !token.is_empty() => request.bearer_auth(token),
            _ => request,
        })
    }
}

async fn decode_empty(response: Response) -> Result<()> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }

    let body = response.text().await.unwrap_or_default();
    if let Some(error) = structured_api_error(status, &body) {
        return Err(error);
    }
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(anyhow::Error::new(RemoteApiError {
            status,
            code: "model_deletion_unsupported".to_string(),
            message: "this daemon does not expose remote model deletion".to_string(),
        }));
    }
    Err(anyhow!("Shadoword API returned {status}"))
}

async fn decode<T: DeserializeOwned>(response: Response) -> Result<T> {
    let status = response.status();
    if status.is_success() {
        return response
            .json()
            .await
            .context("failed to decode the Shadoword API response");
    }

    decode_error(response).await
}

async fn decode_error<T>(response: Response) -> Result<T> {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if let Some(error) = structured_api_error(status, &body) {
        return Err(error);
    }
    Err(anyhow!("Shadoword API returned {status}"))
}

fn structured_api_error(status: reqwest::StatusCode, body: &str) -> Option<anyhow::Error> {
    let error = serde_json::from_str::<ApiErrorBody>(body).ok()?;
    Some(anyhow::Error::new(RemoteApiError {
        status,
        code: error.error,
        message: error.message,
    }))
}
