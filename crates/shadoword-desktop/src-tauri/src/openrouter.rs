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
    #[cfg(test)]
    endpoint: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpenRouterTranscription {
    pub text: String,
    pub usage: Option<Value>,
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

        Ok(Self {
            client,
            #[cfg(test)]
            endpoint: TRANSCRIPTIONS_ENDPOINT.to_owned(),
        })
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
            .post(self.endpoint())
            .header(AUTHORIZATION, authorization)
            .json(&request)
            .send()
            .await
            .map_err(OpenRouterError::Request)?;
        let response = decode_response(response).await?;

        Ok(OpenRouterTranscription {
            text: response.text,
            usage: response.usage,
            elapsed_ms: started_at.elapsed().as_millis(),
        })
    }

    #[cfg(not(test))]
    fn endpoint(&self) -> &str {
        TRANSCRIPTIONS_ENDPOINT
    }

    #[cfg(test)]
    fn endpoint(&self) -> &str {
        &self.endpoint
    }

    #[cfg(test)]
    fn with_test_endpoint(endpoint: String) -> Result<Self, OpenRouterError> {
        let mut client = Self::new()?;
        client.endpoint = endpoint;
        Ok(client)
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

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc::{self, Receiver};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use serde_json::json;

    use super::*;

    struct Fixture {
        endpoint: String,
        request: Receiver<String>,
        server: JoinHandle<()>,
    }

    fn spawn_fixture(status: &str, response_body: &str) -> Fixture {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture server");
        let address = listener.local_addr().expect("read fixture address");
        let (request_sender, request) = mpsc::channel();
        let status = status.to_owned();
        let response_body = response_body.to_owned();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept fixture request");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("set fixture timeout");

            let mut received = Vec::new();
            let mut buffer = [0_u8; 4096];
            let header_end = loop {
                let count = stream.read(&mut buffer).expect("read fixture request");
                assert_ne!(count, 0, "request ended before headers were complete");
                received.extend_from_slice(&buffer[..count]);
                if let Some(position) = received.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    break position + 4;
                }
            };

            let headers = String::from_utf8_lossy(&received[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().expect("parse content length"))
                })
                .expect("request content length");

            while received.len() < header_end + content_length {
                let count = stream.read(&mut buffer).expect("read fixture body");
                assert_ne!(count, 0, "request ended before body was complete");
                received.extend_from_slice(&buffer[..count]);
            }

            request_sender
                .send(String::from_utf8(received).expect("request is UTF-8"))
                .expect("send captured request");

            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
                response_body.len()
            )
            .expect("write fixture response");
        });

        Fixture {
            endpoint: format!("http://{address}/audio/transcriptions"),
            request,
            server,
        }
    }

    #[test]
    fn validate_api_key_rejects_empty_value() {
        let result = validate_api_key("");

        assert!(matches!(result, Err(OpenRouterError::InvalidApiKey)));
    }

    #[test]
    fn validate_model_rejects_unsafe_character() {
        let result = validate_model("openai/whisper 1");

        assert!(matches!(result, Err(OpenRouterError::InvalidModel { .. })));
    }

    #[test]
    fn validate_model_rejects_overlong_value() {
        let model = "a".repeat(MAX_MODEL_LENGTH + 1);
        let result = validate_model(&model);

        assert!(matches!(
            result,
            Err(OpenRouterError::InvalidModel {
                reason: "model is too long"
            })
        ));
    }

    #[test]
    fn validate_audio_size_rejects_payload_over_limit() {
        assert!(matches!(
            validate_audio_size(MAX_WAV_BYTES + 1),
            Err(OpenRouterError::AudioTooLarge)
        ));
    }

    #[test]
    fn model_discovery_keeps_only_transcription_models_and_sorts_names() {
        let response: ModelsEnvelope = serde_json::from_value(json!({
            "data": [
                {
                    "id": "z/transcribe",
                    "name": "Zulu STT",
                    "description": "speech",
                    "architecture": { "output_modalities": ["transcription"] }
                },
                {
                    "id": "a/chat",
                    "name": "Chat",
                    "description": "text",
                    "architecture": { "output_modalities": ["text"] }
                },
                {
                    "id": "a/transcribe",
                    "name": "Alpha STT",
                    "description": "speech",
                    "architecture": { "output_modalities": ["transcription"] }
                }
            ]
        }))
        .expect("decode model fixture");

        let models = transcription_models(response);

        assert_eq!(
            models.into_iter().map(|model| model.id).collect::<Vec<_>>(),
            ["a/transcribe", "z/transcribe"]
        );
    }

    #[test]
    fn request_omits_language_when_english_is_not_requested() {
        let request = TranscriptionRequest {
            model: "openai/whisper-1",
            input_audio: InputAudio {
                data: "UklGRg==".to_owned(),
                format: "wav",
            },
            language: None,
        };

        let value = serde_json::to_value(request).expect("serialize request");

        assert_eq!(
            value,
            json!({
                "model": "openai/whisper-1",
                "input_audio": { "data": "UklGRg==", "format": "wav" }
            })
        );
    }

    #[tokio::test]
    async fn transcribe_wav_sends_authenticated_json_and_decodes_usage() {
        let fixture = spawn_fixture(
            "200 OK",
            r#"{"text":"hello","usage":{"audio_seconds":1.25}}"#,
        );
        let client = OpenRouterClient::with_test_endpoint(fixture.endpoint).expect("build client");

        let transcription = client
            .transcribe_wav("test-key", "openai/whisper-1", b"RIFF".to_vec(), true)
            .await
            .expect("transcribe fixture audio");
        let request = fixture
            .request
            .recv_timeout(Duration::from_secs(5))
            .expect("receive fixture request");
        fixture.server.join().expect("join fixture server");
        let (headers, body) = request.split_once("\r\n\r\n").expect("split request");
        let body: Value = serde_json::from_str(body).expect("decode request body");

        assert_eq!(
            (
                transcription.text,
                transcription.usage,
                headers.contains("authorization: Bearer test-key"),
                body
            ),
            (
                "hello".to_owned(),
                Some(json!({ "audio_seconds": 1.25 })),
                true,
                json!({
                    "model": "openai/whisper-1",
                    "input_audio": { "data": "UklGRg==", "format": "wav" },
                    "language": "en"
                })
            )
        );
    }

    #[tokio::test]
    async fn transcribe_wav_parses_structured_api_error() {
        let fixture = spawn_fixture(
            "429 Too Many Requests",
            r#"{"error":{"code":429,"message":"rate limit exceeded","metadata":{"ignored":true}}}"#,
        );
        let client = OpenRouterClient::with_test_endpoint(fixture.endpoint).expect("build client");

        let error = client
            .transcribe_wav("test-key", "openai/whisper-1", b"RIFF".to_vec(), false)
            .await
            .expect_err("fixture should return an API error");
        fixture
            .request
            .recv_timeout(Duration::from_secs(5))
            .expect("receive fixture request");
        fixture.server.join().expect("join fixture server");

        assert!(matches!(
            error,
            OpenRouterError::Api {
                status: StatusCode::TOO_MANY_REQUESTS,
                code: Some(code),
                message,
            } if code == "429" && message == "rate limit exceeded"
        ));
    }

    #[tokio::test]
    async fn transcribe_wav_does_not_expose_unstructured_error_body() {
        let fixture = spawn_fixture("500 Internal Server Error", r#"{"request":"sensitive"}"#);
        let client = OpenRouterClient::with_test_endpoint(fixture.endpoint).expect("build client");

        let error = client
            .transcribe_wav(
                "test-key",
                "openai/whisper-1",
                b"sensitive audio".to_vec(),
                false,
            )
            .await
            .expect_err("fixture should return an API error");
        fixture
            .request
            .recv_timeout(Duration::from_secs(5))
            .expect("receive fixture request");
        fixture.server.join().expect("join fixture server");

        assert_eq!(
            error.to_string(),
            "OpenRouter returned 500 Internal Server Error: request failed"
        );
    }
}
