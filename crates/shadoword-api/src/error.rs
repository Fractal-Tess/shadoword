use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use shadoword_core::InferenceError;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Clone)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    retry_after: Option<HeaderValue>,
}

#[derive(Debug, Serialize)]
struct ErrorBody<'a> {
    error: &'a str,
    message: &'a str,
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "bad_request",
            message: message.into(),
            retry_after: None,
        }
    }

    pub fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "unauthorized",
            message: "missing or invalid bearer token".to_string(),
            retry_after: None,
        }
    }

    pub fn forbidden() -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: "forbidden",
            message: "this bearer token does not have permission for that operation".to_string(),
            retry_after: None,
        }
    }

    pub fn busy() -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "busy",
            message: "server busy".to_string(),
            retry_after: Some(HeaderValue::from_static("5")),
        }
    }

    pub fn idle_timeout() -> Self {
        Self {
            status: StatusCode::REQUEST_TIMEOUT,
            code: "idle_timeout",
            message: "stream idle timeout".to_string(),
            retry_after: None,
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "unavailable",
            message: message.into(),
            retry_after: Some(HeaderValue::from_static("5")),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code: "stale_generation",
            message: message.into(),
            retry_after: None,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: message.into(),
            retry_after: None,
        }
    }

    pub fn payload_too_large(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            code: "payload_too_large",
            message: message.into(),
            retry_after: None,
        }
    }

    pub fn timeout() -> Self {
        Self {
            status: StatusCode::GATEWAY_TIMEOUT,
            code: "timeout",
            message: "transcription timed out".to_string(),
            retry_after: None,
        }
    }

    pub fn internal(error: anyhow::Error) -> Self {
        tracing::error!(error = %error, "api request failed");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: "internal server error".to_string(),
            retry_after: None,
        }
    }

    pub fn from_join(error: tokio::task::JoinError) -> Self {
        tracing::error!(error = %error, "blocking task failed");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: "internal server error".to_string(),
            retry_after: None,
        }
    }

    pub fn from_inference(error: InferenceError) -> Self {
        match error {
            InferenceError::QueueFull | InferenceError::AudioQueueFull => Self::busy(),
            InferenceError::FlowLimit => Self {
                status: StatusCode::TOO_MANY_REQUESTS,
                code: "flow_limit",
                message: error.to_string(),
                retry_after: Some(HeaderValue::from_static("5")),
            },
            InferenceError::AdmissionClosed | InferenceError::NoCompatibleUnit => {
                Self::unavailable(error.to_string())
            }
            InferenceError::AudioTooLarge => {
                Self::payload_too_large("decoded audio exceeds the inference job limit")
            }
            InferenceError::InvalidSampleRate(_) => Self::bad_request(error.to_string()),
            InferenceError::Cancelled => Self {
                status: StatusCode::REQUEST_TIMEOUT,
                code: "cancelled",
                message: error.to_string(),
                retry_after: None,
            },
            InferenceError::WorkerFailed(_) => Self {
                status: StatusCode::BAD_GATEWAY,
                code: "inference_failed",
                message: error.to_string(),
                retry_after: None,
            },
            InferenceError::ResponseDisconnected => Self::internal(anyhow::Error::new(error)),
        }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let retry_after = self.retry_after.clone();
        let mut response = (
            self.status,
            Json(ErrorBody {
                error: self.code,
                message: &self.message,
            }),
        )
            .into_response();
        if let Some(retry_after) = retry_after {
            response
                .headers_mut()
                .insert(axum::http::header::RETRY_AFTER, retry_after);
        }
        response
    }
}
