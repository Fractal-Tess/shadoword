use crate::error::ApiError;
use anyhow::{anyhow, Context, Result};
use axum::extract::{Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct AuthConfig {
    token: Option<Arc<str>>,
}

impl AuthConfig {
    pub fn new(token: Option<String>) -> Self {
        Self {
            token: token.map(Arc::from),
        }
    }

    pub fn is_configured(&self) -> bool {
        self.token.is_some()
    }

    fn accepts(&self, candidate: &str) -> bool {
        self.token
            .as_deref()
            .is_some_and(|expected| constant_time_eq(expected.as_bytes(), candidate.as_bytes()))
    }
}

pub fn load_token(token_file: Option<&Path>) -> Result<Option<String>> {
    if let Ok(token) = std::env::var("SHADOWORD_API_TOKEN") {
        let token = token.trim();
        if !token.is_empty() {
            return Ok(Some(token.to_string()));
        }
    }

    let Some(token_file) = token_file else {
        return Ok(None);
    };

    require_private_file_mode(token_file)?;
    let token = std::fs::read_to_string(token_file)
        .with_context(|| format!("failed to read token file {}", token_file.display()))?;
    let token = token.trim();
    if token.is_empty() {
        return Err(anyhow!("token file is empty"));
    }
    Ok(Some(token.to_string()))
}

pub fn enforce_bind_auth(addr: &SocketAddr, auth: &AuthConfig) -> Result<()> {
    if addr.ip().is_loopback() || auth.is_configured() {
        Ok(())
    } else {
        Err(anyhow!(
            "SHADOWORD_API_TOKEN or a mode-0600 token file is required for non-loopback binds"
        ))
    }
}

pub async fn require_auth(
    State(auth): State<AuthConfig>,
    request: Request,
    next: Next,
) -> Response {
    if !auth.is_configured() {
        return next.run(request).await;
    }

    let authorized = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|token| auth.accepts(token));

    if authorized {
        next.run(request).await
    } else {
        ApiError::unauthorized().into_response()
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let max_len = left.len().max(right.len());
    let mut diff = left.len() ^ right.len();
    for index in 0..max_len {
        let left = left.get(index).copied().unwrap_or(0);
        let right = right.get(index).copied().unwrap_or(0);
        diff |= usize::from(left ^ right);
    }
    diff == 0
}

#[cfg(unix)]
fn require_private_file_mode(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)
        .with_context(|| format!("failed to inspect token file {}", path.display()))?;
    let mode = metadata.permissions().mode() & 0o777;
    if mode == 0o600 {
        Ok(())
    } else {
        Err(anyhow!("token file must be mode 0600"))
    }
}

#[cfg(not(unix))]
fn require_private_file_mode(path: &Path) -> Result<()> {
    if path.is_file() {
        Ok(())
    } else {
        Err(anyhow!("token file does not exist"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn auth_accepts_only_exact_bearer_token() {
        let auth = AuthConfig::new(Some("secret-token".to_string()));

        assert!(auth.accepts("secret-token"));
        assert!(!auth.accepts("secret"));
        assert!(!auth.accepts("secret-token-extra"));
    }

    #[test]
    fn non_loopback_bind_requires_authentication() {
        let public: SocketAddr = "0.0.0.0:47813".parse().expect("public address");
        let loopback: SocketAddr = "127.0.0.1:47813".parse().expect("loopback address");

        assert!(enforce_bind_auth(&public, &AuthConfig::default()).is_err());
        assert!(enforce_bind_auth(&loopback, &AuthConfig::default()).is_ok());
        assert!(enforce_bind_auth(&public, &AuthConfig::new(Some("secret".to_string()))).is_ok());
    }

    #[test]
    #[cfg(unix)]
    fn token_file_must_be_mode_0600() {
        let path = std::env::temp_dir().join(format!(
            "shadoword-token-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock before epoch")
                .as_nanos()
        ));
        std::fs::write(&path, "token\n").expect("write token file");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
            .expect("set public mode");
        assert!(require_private_file_mode(&path).is_err());

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .expect("set private mode");
        assert!(require_private_file_mode(&path).is_ok());
        let _ = std::fs::remove_file(path);
    }
}
