use crate::error::ApiError;
use anyhow::{anyhow, Context, Result};
use axum::extract::{Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};
use shadoword_core::remote_contracts::ApiTokenSummaryDto;
use shadoword_core::{ApiTokenConfig, ApiTokenRole};
use std::path::Path;
use std::sync::{Arc, RwLock};

const TOKEN_BYTES: usize = 32;
const TOKEN_HASH_PREFIX: &str = "sha256:";
const MIN_INIT_TOKEN_LEN: usize = 16;

#[derive(Clone, Debug)]
struct StoredToken {
    name: String,
    role: ApiTokenRole,
    /// Kept alongside the parsed digest so the token list can be written back to
    /// `api.json` from memory. Re-encoding the digest would work, but round-tripping
    /// the operator's own file contents means a token they wrote by hand keeps its
    /// exact spelling.
    token_hash: String,
    digest: [u8; 32],
}

/// The daemon's live token set. Cheap clones share one store, so a token created
/// or revoked over HTTP takes effect on the very next request rather than at the
/// next restart — which matters because the caller doing the revoking is usually
/// revoking a token that is being abused right now.
#[derive(Clone, Debug, Default)]
pub struct AuthConfig {
    tokens: Arc<RwLock<Arc<[StoredToken]>>>,
}

impl AuthConfig {
    pub fn new(configured: &[ApiTokenConfig]) -> Result<Self> {
        Ok(Self {
            tokens: Arc::new(RwLock::new(parse_tokens(configured)?)),
        })
    }

    pub fn is_configured(&self) -> bool {
        !self.read().is_empty()
    }

    /// The token list in the shape `api.json` stores it, for handlers that have to
    /// rewrite the whole config file.
    pub fn snapshot(&self) -> Vec<ApiTokenConfig> {
        self.read()
            .iter()
            .map(|token| ApiTokenConfig {
                name: token.name.clone(),
                role: token.role,
                token_hash: token.token_hash.clone(),
            })
            .collect()
    }

    pub fn summaries(&self) -> Vec<ApiTokenSummaryDto> {
        self.read()
            .iter()
            .map(|token| ApiTokenSummaryDto {
                name: token.name.clone(),
                role: token.role,
            })
            .collect()
    }

    /// Swaps the live token set. Callers validate and persist first: this is the
    /// last step, so a rejected or unwritable change never becomes visible.
    pub fn replace(&self, configured: &[ApiTokenConfig]) -> Result<()> {
        let parsed = parse_tokens(configured)?;
        *self.tokens.write().expect("auth token lock poisoned") = parsed;
        Ok(())
    }

    fn read(&self) -> Arc<[StoredToken]> {
        Arc::clone(&self.tokens.read().expect("auth token lock poisoned"))
    }

    fn authenticate(&self, candidate: &str) -> Option<ApiTokenRole> {
        let candidate = token_digest(candidate);
        let mut admin = false;
        let mut user = false;
        for token in self.read().iter() {
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

fn parse_tokens(configured: &[ApiTokenConfig]) -> Result<Arc<[StoredToken]>> {
    let mut tokens = Vec::with_capacity(configured.len());
    for token in configured {
        tokens.push(StoredToken {
            name: token.name.clone(),
            role: token.role,
            token_hash: token.token_hash.clone(),
            digest: parse_token_hash(&token.token_hash)
                .with_context(|| format!("invalid stored hash for API token {:?}", token.name))?,
        });
    }
    Ok(Arc::from(tokens))
}

pub fn generate_token(role: ApiTokenRole, name: &str) -> Result<(String, ApiTokenConfig)> {
    let name = validate_token_name(name)?;
    let mut random = [0_u8; TOKEN_BYTES];
    getrandom::fill(&mut random).context("failed to obtain secure randomness for API token")?;
    let role_name = match role {
        ApiTokenRole::Admin => "admin",
        ApiTokenRole::User => "user",
    };
    let value = format!("swd_{role_name}_{}", URL_SAFE_NO_PAD.encode(random));
    let token = ApiTokenConfig {
        name,
        role,
        token_hash: hash_secret(&value),
    };
    Ok((value, token))
}

/// Records a secret the operator chose, for the bootstrap path where the value
/// has to be known before the daemon first runs. `generate_token` guarantees its
/// own entropy; here the only thing that can be checked is length.
pub fn adopt_token(role: ApiTokenRole, name: &str, secret: &str) -> Result<ApiTokenConfig> {
    let name = validate_token_name(name)?;
    let secret = secret.trim();
    if secret.len() < MIN_INIT_TOKEN_LEN {
        return Err(anyhow!(
            "the supplied API token must be at least {MIN_INIT_TOKEN_LEN} characters"
        ));
    }
    Ok(ApiTokenConfig {
        name,
        role,
        token_hash: hash_secret(secret),
    })
}

fn validate_token_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(anyhow!("token name cannot be empty"));
    }
    if name.len() > 64 || name.chars().any(char::is_control) {
        return Err(anyhow!(
            "token name must be at most 64 characters and contain no control characters"
        ));
    }
    Ok(name.to_string())
}

fn hash_secret(secret: &str) -> String {
    format!(
        "{TOKEN_HASH_PREFIX}{}",
        URL_SAFE_NO_PAD.encode(token_digest(secret))
    )
}

/// A daemon with no tokens can answer nothing, so it refuses to start instead of
/// listening and rejecting every caller. Failing here also keeps the "is this
/// daemon open?" question from existing at all: there is no such state to reason
/// about, on loopback or anywhere else.
pub fn enforce_token_requirement(auth: &AuthConfig, config_path: &Path) -> Result<()> {
    if auth.is_configured() {
        return Ok(());
    }
    Err(anyhow!(
        "no API tokens are configured in {}; issue one with `shadoword-api token generate admin <name>` \
         or set SHADOWORD_INIT_TOKEN_FILE to a file holding the first admin token",
        config_path.display()
    ))
}

/// Both guards are always mounted and decide per request, so a token created or
/// revoked over HTTP takes effect immediately rather than at the next restart.
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
