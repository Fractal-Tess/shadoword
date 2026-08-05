use std::fmt;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use reqwest::header::{HeaderValue, AUTHORIZATION};
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const TRANSCRIPTIONS_ENDPOINT: &str = "https://openrouter.ai/api/v1/audio/transcriptions";
const MODELS_ENDPOINT: &str = "https://openrouter.ai/api/v1/models?output_modalities=transcription";
const KEY_ENDPOINT: &str = "https://openrouter.ai/api/v1/auth/key";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_MODEL_LENGTH: usize = 200;
const MAX_WAV_BYTES: usize = 25 * 1024 * 1024;

#[derive(Clone)]
pub struct OpenRouterClient {
    client: Client,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpenRouterTranscription {
    pub text: String,
    pub usage: Option<Value>,
    pub cost_usd: Option<f64>,
    pub elapsed_ms: u128,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpenRouterModel {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpenRouterKeyInfo {
    pub label: Option<String>,
    pub is_free_tier: bool,
    pub limit: Option<f64>,
    pub limit_remaining: Option<f64>,
    pub usage: f64,
}

#[derive(Debug)]
pub enum OpenRouterError {
    InvalidApiKey,
    InvalidModel {
        reason: &'static str,
    },
    AudioTooLarge,
    ClientBuild(reqwest::Error),
    Request(reqwest::Error),
    InvalidResponse(reqwest::Error),
    Api {
        status: StatusCode,
        code: Option<String>,
        message: String,
    },
}

impl fmt::Display for OpenRouterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidApiKey => write!(formatter, "OpenRouter API key is invalid"),
            Self::InvalidModel { reason } => {
                write!(formatter, "OpenRouter model is invalid: {reason}")
            }
            Self::AudioTooLarge => write!(
                formatter,
                "recording exceeds the 25 MiB OpenRouter upload limit"
            ),
            Self::ClientBuild(_) => write!(formatter, "failed to build the OpenRouter HTTP client"),
            Self::Request(error) if error.is_timeout() => {
                write!(formatter, "OpenRouter transcription request timed out")
            }
            Self::Request(_) => write!(formatter, "failed to reach OpenRouter"),
            Self::InvalidResponse(_) => {
                write!(
                    formatter,
                    "OpenRouter returned an invalid transcription response"
                )
            }
            Self::Api {
                status,
                code,
                message,
            } => {
                write!(formatter, "OpenRouter returned {status}")?;
                if let Some(code) = code {
                    write!(formatter, " ({code})")?;
                }
                write!(formatter, ": {message}")
            }
        }
    }
}

impl std::error::Error for OpenRouterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ClientBuild(error) | Self::Request(error) | Self::InvalidResponse(error) => {
                Some(error)
            }
            Self::InvalidApiKey
            | Self::InvalidModel { .. }
            | Self::AudioTooLarge
            | Self::Api { .. } => None,
        }
    }
}

impl OpenRouterClient {
    pub fn new() -> Result<Self, OpenRouterError> {
        let client = Client::builder()
            .user_agent(concat!("shadoword-desktop/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(OpenRouterError::ClientBuild)?;

        Ok(Self { client })
    }

    pub async fn list_transcription_models(&self) -> Result<Vec<OpenRouterModel>, OpenRouterError> {
        let response = self
            .client
            .get(MODELS_ENDPOINT)
            .send()
            .await
            .map_err(OpenRouterError::Request)?;
        let response = decode_success::<ModelsEnvelope>(response).await?;
        Ok(transcription_models(response))
    }

    pub async fn test_api_key(&self, api_key: &str) -> Result<OpenRouterKeyInfo, OpenRouterError> {
        let authorization = authorization_header(api_key)?;
        let response = self
            .client
            .get(KEY_ENDPOINT)
            .header(AUTHORIZATION, authorization)
            .send()
            .await
            .map_err(OpenRouterError::Request)?;
        let response = decode_success::<KeyEnvelope>(response).await?;
        Ok(OpenRouterKeyInfo {
            label: response.data.label,
            is_free_tier: response.data.is_free_tier,
            limit: response.data.limit,
            limit_remaining: response.data.limit_remaining,
            usage: response.data.usage,
        })
    }

    pub async fn transcribe_wav(
        &self,
        api_key: &str,
        model: &str,
        wav: Vec<u8>,
        english_only: bool,
    ) -> Result<OpenRouterTranscription, OpenRouterError> {
        validate_api_key(api_key)?;
        validate_model(model)?;
        validate_audio_size(wav.len())?;

        let authorization = authorization_header(api_key)?;

        let request = TranscriptionRequest {
            model,
            input_audio: InputAudio {
                data: BASE64_STANDARD.encode(wav),
                format: "wav",
            },
            language: english_only.then_some("en"),
        };
        let started_at = Instant::now();
        let response = self
            .client
            .post(TRANSCRIPTIONS_ENDPOINT)
            .header(AUTHORIZATION, authorization)
            .json(&request)
            .send()
            .await
            .map_err(OpenRouterError::Request)?;
        let response = decode_response(response).await?;
        let cost_usd = response.usage.as_ref().and_then(usage_cost);

        Ok(OpenRouterTranscription {
            text: response.text,
            usage: response.usage,
            cost_usd,
            elapsed_ms: started_at.elapsed().as_millis(),
        })
    }
}

#[derive(Serialize)]
struct TranscriptionRequest<'a> {
    model: &'a str,
    input_audio: InputAudio,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<&'static str>,
}

#[derive(Serialize)]
struct InputAudio {
    data: String,
    format: &'static str,
}

#[derive(Deserialize)]
struct ModelsEnvelope {
    data: Vec<ModelResponse>,
}

#[derive(Deserialize)]
struct ModelResponse {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    architecture: ModelArchitecture,
}

#[derive(Deserialize)]
struct ModelArchitecture {
    #[serde(default)]
    output_modalities: Vec<String>,
}

#[derive(Deserialize)]
struct KeyEnvelope {
    data: KeyResponse,
}

#[derive(Deserialize)]
struct KeyResponse {
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    is_free_tier: bool,
    #[serde(default)]
    limit: Option<f64>,
    #[serde(default)]
    limit_remaining: Option<f64>,
    #[serde(default)]
    usage: f64,
}

#[derive(Deserialize)]
struct TranscriptionResponse {
    text: String,
    #[serde(default)]
    usage: Option<Value>,
}

#[derive(Deserialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Deserialize)]
struct ErrorBody {
    message: String,
    #[serde(default)]
    code: Option<Value>,
}

pub(crate) fn validate_api_key(api_key: &str) -> Result<(), OpenRouterError> {
    if api_key.is_empty()
        || api_key
            .bytes()
            .any(|byte| !is_valid_bearer_token_byte(byte))
    {
        return Err(OpenRouterError::InvalidApiKey);
    }
    Ok(())
}

fn is_valid_bearer_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'+' | b'/' | b'=')
}

fn validate_audio_size(bytes: usize) -> Result<(), OpenRouterError> {
    if bytes > MAX_WAV_BYTES {
        Err(OpenRouterError::AudioTooLarge)
    } else {
        Ok(())
    }
}

pub(crate) fn validate_model(model: &str) -> Result<(), OpenRouterError> {
    if model.is_empty() {
        return Err(OpenRouterError::InvalidModel {
            reason: "model must not be empty",
        });
    }
    if model.len() > MAX_MODEL_LENGTH {
        return Err(OpenRouterError::InvalidModel {
            reason: "model is too long",
        });
    }
    if !model.bytes().all(|byte| {
        byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'.' | b'_' | b'/' | b':' | b'@' | b'+')
    }) {
        return Err(OpenRouterError::InvalidModel {
            reason: "model contains an unsafe character",
        });
    }
    Ok(())
}

fn usage_cost(usage: &Value) -> Option<f64> {
    usage
        .get("cost")
        .or_else(|| usage.get("total_cost"))
        .and_then(json_number)
}

fn json_number(value: &Value) -> Option<f64> {
    let number = value
        .as_f64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))?;
    number
        .is_finite()
        .then_some(number)
        .filter(|number| *number >= 0.0)
}

fn transcription_models(response: ModelsEnvelope) -> Vec<OpenRouterModel> {
    let mut models = response
        .data
        .into_iter()
        .filter(|model| {
            model
                .architecture
                .output_modalities
                .iter()
                .any(|modality| modality == "transcription")
        })
        .map(|model| OpenRouterModel {
            id: model.id,
            name: model.name,
            description: model.description,
        })
        .collect::<Vec<_>>();
    models.sort_by(|left, right| left.name.cmp(&right.name));
    models
}

fn authorization_header(api_key: &str) -> Result<HeaderValue, OpenRouterError> {
    validate_api_key(api_key)?;
    let mut authorization = HeaderValue::from_str(&format!("Bearer {api_key}"))
        .map_err(|_| OpenRouterError::InvalidApiKey)?;
    authorization.set_sensitive(true);
    Ok(authorization)
}

async fn decode_response(response: Response) -> Result<TranscriptionResponse, OpenRouterError> {
    decode_success(response).await
}

async fn decode_success<T: for<'de> Deserialize<'de>>(
    response: Response,
) -> Result<T, OpenRouterError> {
    let status = response.status();
    if status.is_success() {
        return response
            .json()
            .await
            .map_err(OpenRouterError::InvalidResponse);
    }

    let error = response.json::<ErrorEnvelope>().await.ok();
    let (code, message) = error.map_or_else(
        || (None, "request failed".to_owned()),
        |error| {
            (
                error.error.code.and_then(format_error_code),
                error.error.message,
            )
        },
    );

    Err(OpenRouterError::Api {
        status,
        code,
        message,
    })
}

fn format_error_code(code: Value) -> Option<String> {
    match code {
        Value::String(code) => Some(code),
        Value::Number(code) => Some(code.to_string()),
        Value::Bool(code) => Some(code.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}
