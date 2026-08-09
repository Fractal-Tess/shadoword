use anyhow::{anyhow, Context, Result};
use reqwest::{Client, Method, Response, Url};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use shadoword_core::remote_contracts::{
    ApiTokenSummaryDto, CreateApiTokenRequest, CreatedApiTokenDto, DaemonStatusDto,
    DownloadJobStatus, HealthDto, OverviewDto, RuntimeConfigDto, StartDownloadRequest, VersionDto,
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
    fn unsupported(code: &str, message: &str) -> Self {
        Self {
            status: reqwest::StatusCode::NOT_FOUND,
            code: code.to_string(),
            message: message.to_string(),
        }
    }

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

    /// `None` when the daemon predates the version route, which is itself the
    /// answer: it is older than the first release that could report a version.
    pub async fn version(&self, endpoint: &str, token: Option<&str>) -> Result<Option<VersionDto>> {
        let response = self
            .request(endpoint, token, Method::GET, &["v1", "version"])?
            .timeout(REMOTE_TIMEOUT)
            .send()
            .await
            .context("failed to reach the remote API")?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        decode(response).await.map(Some)
    }

    pub async fn list_tokens(
        &self,
        endpoint: &str,
        token: Option<&str>,
    ) -> Result<Vec<ApiTokenSummaryDto>> {
        let response = self
            .request(endpoint, token, Method::GET, &["v1", "tokens"])?
            .timeout(REMOTE_TIMEOUT)
            .send()
            .await
            .context("failed to reach the remote API")?;
        decode_supported(response, token_management_unsupported()).await
    }

    pub async fn create_token(
        &self,
        endpoint: &str,
        token: Option<&str>,
        request: &CreateApiTokenRequest,
    ) -> Result<CreatedApiTokenDto> {
        let response = self
            .request(endpoint, token, Method::POST, &["v1", "tokens"])?
            .json(request)
            .timeout(REMOTE_TIMEOUT)
            .send()
            .await
            .context("failed to reach the remote API")?;
        decode_supported(response, token_management_unsupported()).await
    }

    pub async fn revoke_token(
        &self,
        endpoint: &str,
        token: Option<&str>,
        name: &str,
    ) -> Result<()> {
        let response = self
            .request(endpoint, token, Method::DELETE, &["v1", "tokens", name])?
            .timeout(REMOTE_TIMEOUT)
            .send()
            .await
            .context("failed to reach the remote API")?;
        decode_empty(response, token_management_unsupported()).await
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
        decode_empty(
            response,
            RemoteApiError::unsupported(
                "model_deletion_unsupported",
                "this daemon does not expose remote model deletion",
            ),
        )
        .await
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

/// `missing` names what an unstructured 404 means for this route. The daemon
/// answers its own failures with a JSON error body, so a bare 404 is the router
/// saying the route does not exist — that is, the daemon predates the feature.
async fn decode_empty(response: Response, missing: RemoteApiError) -> Result<()> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }

    let body = response.text().await.unwrap_or_default();
    if let Some(error) = structured_api_error(status, &body) {
        return Err(error);
    }
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(anyhow::Error::new(missing));
    }
    Err(anyhow!("Shadoword API returned {status}"))
}

/// `decode` for routes that may simply not exist on an older daemon, so that the
/// caller can tell "you are talking to a daemon without this feature" apart from
/// "the daemon rejected what you asked for".
async fn decode_supported<T: DeserializeOwned>(
    response: Response,
    missing: RemoteApiError,
) -> Result<T> {
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        let body = response.text().await.unwrap_or_default();
        return Err(structured_api_error(reqwest::StatusCode::NOT_FOUND, &body)
            .unwrap_or_else(|| anyhow::Error::new(missing)));
    }
    decode(response).await
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

fn token_management_unsupported() -> RemoteApiError {
    RemoteApiError::unsupported(
        "token_management_unsupported",
        "this daemon does not expose remote token management",
    )
}

fn structured_api_error(status: reqwest::StatusCode, body: &str) -> Option<anyhow::Error> {
    let error = serde_json::from_str::<ApiErrorBody>(body).ok()?;
    Some(anyhow::Error::new(RemoteApiError {
        status,
        code: error.error,
        message: error.message,
    }))
}
