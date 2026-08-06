use anyhow::{anyhow, Context};
use clap::{Parser, Subcommand, ValueEnum};
use shadoword_api::auth::{enforce_bind_auth, generate_token, AuthConfig};
use shadoword_api::downloads::DownloadJobs;
use shadoword_api::request_recording::RequestRecorder;
use shadoword_api::router::{
    build_router, resolved_model_path, runtime_transcription_config, AppState, RouterConfig,
};
use shadoword_core::{
    default_whisper_model, download_whisper_model, parse_requested_models, resolve_download_dir,
    resolve_whisper_model, unknown_model_error, ApiConfig, ApiTokenRole, InferenceRuntime,
    ModelDownloadStatus, WhisperModelFactory,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(version, about = "Shadoword HTTP/WebSocket transcription API daemon")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(long = "config", env = "SHADOWORD_API_CONFIG", global = true)]
    config_path: Option<PathBuf>,

    #[arg(long = "listen", env = "SHADOWORD_LISTEN_ADDR")]
    listen_addr: Option<String>,

    #[arg(long = "model", env = "SHADOWORD_MODEL_PATH")]
    model_path: Option<PathBuf>,

    #[arg(long = "preload", env = "SHADOWORD_PRELOAD")]
    preload: Option<bool>,

    #[arg(long = "no-preload", conflicts_with = "preload")]
    no_preload: bool,

    #[arg(
        long = "download-model",
        env = "SHADOWORD_DOWNLOAD_MODELS",
        value_delimiter = ','
    )]
    download_models: Vec<String>,

    #[arg(long = "download-dir", env = "SHADOWORD_DOWNLOAD_DIR")]
    download_dir: Option<PathBuf>,

    #[arg(long = "queue-capacity", env = "SHADOWORD_QUEUE_CAPACITY")]
    queue_capacity: Option<usize>,

    /// Archive every accepted transcription request as WAV plus response metadata.
    #[arg(
        long = "request-recording-dir",
        env = "SHADOWORD_REQUEST_RECORDING_DIR"
    )]
    request_recording_dir: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Manage named API bearer tokens stored as SHA-256 hashes.
    Token {
        #[command(subcommand)]
        command: TokenCommand,
    },
}

#[derive(Debug, Subcommand)]
enum TokenCommand {
    /// Generate a token and print its secret value once.
    Generate { role: TokenRole, name: String },
    /// List token names and roles without exposing token hashes.
    List,
    /// Revoke a named token.
    Revoke { name: String },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TokenRole {
    Admin,
    User,
}

impl From<TokenRole> for ApiTokenRole {
    fn from(role: TokenRole) -> Self {
        match role {
            TokenRole::Admin => Self::Admin,
            TokenRole::User => Self::User,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let config_path = cli.config_path.clone().unwrap_or(ApiConfig::config_path()?);
    let mut config = ApiConfig::load_from_path(&config_path)?;
    if let Some(Command::Token { command }) = &cli.command {
        manage_tokens(&config_path, &mut config, command)?;
        return Ok(());
    }
    apply_cli_overrides(&mut config, &cli);

    let addr: SocketAddr = config
        .listen_addr
        .parse()
        .with_context(|| format!("invalid listen address '{}'", config.listen_addr))?;
    let auth = AuthConfig::new(&config.tokens)?;
    enforce_bind_auth(&addr, &auth)?;

    let download_dir = resolve_download_dir(
        cli.download_dir
            .as_ref()
            .map(|path| path.display().to_string()),
        ApiConfig::models_dir()?,
    );
    let downloads = startup_downloads(&cli.download_models, download_dir.clone()).await?;
    if !downloads.is_empty() {
        tracing::info!(
            downloaded = downloads.len(),
            "api startup model download step complete"
        );
    }
    if config.transcription.model_path.as_os_str().is_empty() {
        config.transcription.model_path = download_dir.join(default_whisper_model().filename);
    }

    let model_path = resolved_model_path(&config.transcription);
    if config.transcription.preload_on_startup && !model_path.exists() {
        return Err(anyhow!(
            "inference pool model file is missing: {}; run with --download-model {} or set --model to an existing file",
            model_path.display(),
            default_whisper_model().id
        ));
    }
    let runtime_config = runtime_transcription_config(&config);
    let runtime = tokio::task::spawn_blocking(move || {
        InferenceRuntime::new_with_factory(runtime_config, Arc::new(WhisperModelFactory))
    })
    .await
    .context("inference pool startup task failed")?
    .context("failed to prepare the inference pool; check required unit targets, model path, compiled backend, and device availability")?;
    let runtime = Arc::new(runtime);
    let pool = runtime.status();
    tracing::info!(
        generation = pool.generation,
        ready_units = pool.ready_units,
        unhealthy_units = pool.unhealthy_units,
        preload_on_startup = config.transcription.preload_on_startup,
        "api inference pool startup complete"
    );

    let request_recorder = RequestRecorder::new(cli.request_recording_dir.clone())?;
    if let Some(directory) = request_recorder.directory() {
        tracing::info!(
            path = %directory.display(),
            "API audio request recording enabled"
        );
    }

    let state = AppState {
        runtime,
        request_recorder,
        downloads: DownloadJobs::default(),
        config_path,
        download_dir,
        listen_addr: addr,
        queue_capacity: config.queue_capacity,
        tokens: Arc::from(config.tokens.clone()),
        config_update_lock: Arc::new(tokio::sync::Mutex::new(())),
    };
    let app = build_router(state, RouterConfig { auth });

    tracing::info!("shadoword-api listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn manage_tokens(
    config_path: &std::path::Path,
    config: &mut ApiConfig,
    command: &TokenCommand,
) -> anyhow::Result<()> {
    match command {
        TokenCommand::Generate { role, name } => {
            if config.tokens.iter().any(|token| token.name == name.trim()) {
                return Err(anyhow!(
                    "an API token named {:?} already exists; revoke it before generating a replacement",
                    name.trim()
                ));
            }
            let (value, token) = generate_token((*role).into(), name)?;
            let role = token_role_name(token.role);
            let name = token.name.clone();
            config.tokens.push(token);
            config.save_to_path(config_path)?;
            println!("{value}");
            eprintln!(
                "Generated {role} token {name:?}. Its hash was saved to {}. Restart shadoword-api to load it.",
                config_path.display()
            );
        }
        TokenCommand::List => {
            for token in &config.tokens {
                println!("{}\t{}", token_role_name(token.role), token.name);
            }
        }
        TokenCommand::Revoke { name } => {
            let name = name.trim();
            let previous_len = config.tokens.len();
            config.tokens.retain(|token| token.name != name);
            if config.tokens.len() == previous_len {
                return Err(anyhow!("no API token named {name:?} exists"));
            }
            config.save_to_path(config_path)?;
            eprintln!(
                "Revoked token {name:?} in {}. Restart shadoword-api to apply the change.",
                config_path.display()
            );
        }
    }
    Ok(())
}

fn token_role_name(role: ApiTokenRole) -> &'static str {
    match role {
        ApiTokenRole::Admin => "admin",
        ApiTokenRole::User => "user",
    }
}

fn apply_cli_overrides(config: &mut ApiConfig, cli: &Cli) {
    if let Some(listen_addr) = &cli.listen_addr {
        config.listen_addr = listen_addr.clone();
    }
    if let Some(model_path) = &cli.model_path {
        config.transcription.model_path = model_path.clone();
    }
    if let Some(preload) = cli.preload {
        config.transcription.preload_on_startup = preload;
    }
    if cli.no_preload {
        config.transcription.preload_on_startup = false;
    }
    if let Some(queue_capacity) = cli.queue_capacity {
        config.queue_capacity = queue_capacity;
    }
}

async fn startup_downloads(
    requested: &[String],
    target_dir: PathBuf,
) -> anyhow::Result<Vec<ModelDownloadStatus>> {
    let requested = requested
        .iter()
        .flat_map(|value| parse_requested_models(value))
        .collect::<Vec<_>>();
    if requested.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = Vec::with_capacity(requested.len());
    for key in requested {
        let target_dir = target_dir.clone();
        let spec = resolve_whisper_model(&key).ok_or_else(|| unknown_model_error(&key))?;
        let status = tokio::task::spawn_blocking(move || download_whisper_model(spec, &target_dir))
            .await
            .context("startup download task failed")??;
        tracing::info!(
            model = spec.id,
            filename = spec.filename,
            path = %status.path.display(),
            skipped = status.skipped,
            "startup whisper model download"
        );
        results.push(status);
    }

    Ok(results)
}
