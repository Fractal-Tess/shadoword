use crate::error::ApiError;
use anyhow::{anyhow, Context, Result};
use axum::extract::{Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};
use shadoword_core::{ApiTokenConfig, ApiTokenRole};
use std::net::SocketAddr;
use std::sync::Arc;

const TOKEN_BYTES: usize = 32;
const TOKEN_HASH_PREFIX: &str = "sha256:";

#[derive(Clone, Debug)]
struct StoredToken {
    role: ApiTokenRole,
    digest: [u8; 32],
}

#[derive(Clone, Debug, Default)]
pub struct AuthConfig {
    tokens: Arc<[StoredToken]>,
}

impl AuthConfig {
    pub fn new(configured: &[ApiTokenConfig]) -> Result<Self> {
        let mut tokens = Vec::with_capacity(configured.len());
        for token in configured {
            tokens.push(StoredToken {
                role: token.role,
                digest: parse_token_hash(&token.token_hash).with_context(|| {
                    format!("invalid stored hash for API token {:?}", token.name)
                })?,
            });
        }
        Ok(Self {
            tokens: Arc::from(tokens),
        })
    }

    pub fn is_configured(&self) -> bool {
        !self.tokens.is_empty()
    }

    fn authenticate(&self, candidate: &str) -> Option<ApiTokenRole> {
        let candidate = token_digest(candidate);
        let mut admin = false;
        let mut user = false;
        for token in self.tokens.iter() {
            let matched = constant_time_eq(&token.digest, &candidate);
            admin |= matched && token.role == ApiTokenRole::Admin;
            user |= matched && token.role == ApiTokenRole::User;
        }
        if admin {
            Some(ApiTokenRole::Admin)
        } else if user {
            Some(ApiTokenRole::User)
        } else {
            None
        }
    }
}

pub fn generate_token(role: ApiTokenRole, name: &str) -> Result<(String, ApiTokenConfig)> {
    let name = name.trim();
    if name.is_empty() {
        return Err(anyhow!("token name cannot be empty"));
    }
    if name.len() > 64 || name.chars().any(char::is_control) {
        return Err(anyhow!(
            "token name must be at most 64 characters and contain no control characters"
        ));
    }

    let mut random = [0_u8; TOKEN_BYTES];
    getrandom::fill(&mut random).context("failed to obtain secure randomness for API token")?;
    let role_name = match role {
        ApiTokenRole::Admin => "admin",
        ApiTokenRole::User => "user",
    };
    let value = format!("swd_{role_name}_{}", URL_SAFE_NO_PAD.encode(random));
    let token_hash = format!(
        "{TOKEN_HASH_PREFIX}{}",
        URL_SAFE_NO_PAD.encode(token_digest(&value))
    );
    Ok((
        value,
        ApiTokenConfig {
            name: name.to_string(),
            role,
            token_hash,
        },
    ))
}

pub fn enforce_bind_auth(addr: &SocketAddr, auth: &AuthConfig) -> Result<()> {
    if addr.ip().is_loopback() || auth.is_configured() {
        Ok(())
    } else {
        Err(anyhow!(
            "an API admin or user token is required for non-loopback binds"
        ))
    }
}

pub async fn require_admin(
    State(auth): State<AuthConfig>,
    request: Request,
    next: Next,
) -> Response {
    match request_role(&auth, &request) {
        Some(ApiTokenRole::Admin) => next.run(request).await,
        Some(ApiTokenRole::User) => ApiError::forbidden().into_response(),
        None => ApiError::unauthorized().into_response(),
    }
}

pub async fn require_transcription(
    State(auth): State<AuthConfig>,
    request: Request,
    next: Next,
) -> Response {
    if request_role(&auth, &request).is_some() {
        next.run(request).await
    } else {
        ApiError::unauthorized().into_response()
    }
}

fn request_role(auth: &AuthConfig, request: &Request) -> Option<ApiTokenRole> {
    request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .and_then(|token| auth.authenticate(token))
}

fn token_digest(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

fn parse_token_hash(value: &str) -> Result<[u8; 32]> {
    let encoded = value
        .strip_prefix(TOKEN_HASH_PREFIX)
        .ok_or_else(|| anyhow!("token hash must use sha256"))?;
    let decoded = URL_SAFE_NO_PAD
        .decode(encoded)
        .context("token hash is not valid base64url")?;
    decoded
        .try_into()
        .map_err(|_| anyhow!("token hash must contain 32 bytes"))
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
