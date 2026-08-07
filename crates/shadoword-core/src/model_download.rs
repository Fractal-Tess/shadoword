use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static NEXT_PART_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy)]
pub struct WhisperModelSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub filename: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub size_bytes: u64,
    pub recommended: bool,
}

#[derive(Debug, Clone)]
pub struct ModelDownloadStatus {
    pub path: PathBuf,
    pub skipped: bool,
    pub verified: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ModelDownloadProgress {
    pub downloaded: u64,
    pub total: u64,
}

const WHISPER_MODELS: &[WhisperModelSpec] = &[
    WhisperModelSpec {
        id: "tiny",
        name: "Whisper Tiny",
        description: "Smallest and fastest catalog model",
        filename: "ggml-tiny.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
        sha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
        size_bytes: 77_691_713,
        recommended: false,
    },
    WhisperModelSpec {
        id: "base",
        name: "Whisper Base",
        description: "Fast model with improved accuracy over Tiny",
        filename: "ggml-base.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
        sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
        size_bytes: 147_964_211,
        recommended: false,
    },
    WhisperModelSpec {
        id: "small",
        name: "Whisper Small",
        description: "Balanced model for lower-memory systems",
        filename: "ggml-small.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
        size_bytes: 487_601_967,
        recommended: false,
    },
    WhisperModelSpec {
        id: "medium",
        name: "Whisper Medium",
        description: "High-accuracy multilingual model",
        filename: "ggml-medium.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
        sha256: "6c14d5adee5f86394037b4e4e8b59f1673b6cee10e3cf0b11bbdbee79c156208",
        size_bytes: 1_533_763_059,
        recommended: false,
    },
    WhisperModelSpec {
        id: "turbo",
        name: "Whisper Large v3 Turbo",
        description: "Recommended balance of speed and accuracy",
        filename: "ggml-large-v3-turbo.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin",
        sha256: "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69",
        size_bytes: 1_621_004_003,
        recommended: true,
    },
    WhisperModelSpec {
        id: "large-v3",
        name: "Whisper Large v3",
        description: "Largest and most accurate catalog model",
        filename: "ggml-large-v3.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
        sha256: "64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2",
        size_bytes: 3_094_623_691,
        recommended: false,
    },
];

pub fn list_whisper_models() -> &'static [WhisperModelSpec] {
    WHISPER_MODELS
}

pub fn default_whisper_model() -> &'static WhisperModelSpec {
    WHISPER_MODELS
        .iter()
        .find(|spec| spec.recommended)
        .expect("canonical Whisper catalog must include a recommended model")
}

pub fn resolve_whisper_model(id: &str) -> Option<&'static WhisperModelSpec> {
    WHISPER_MODELS
        .iter()
        .find(|spec| spec.id.eq_ignore_ascii_case(id.trim()))
}

pub fn parse_requested_models(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub fn resolve_download_dir(override_dir: Option<String>, default_dir: PathBuf) -> PathBuf {
    override_dir
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or(default_dir)
}

pub fn unknown_model_error(id: &str) -> anyhow::Error {
    let known = WHISPER_MODELS
        .iter()
        .map(|model| model.id)
        .collect::<Vec<_>>()
        .join(", ");
    anyhow!("unknown Whisper model '{id}'; supported ids: {known}")
}

pub fn download_whisper_model(
    spec: &WhisperModelSpec,
    target_dir: &Path,
) -> Result<ModelDownloadStatus> {
    download_whisper_model_with_progress(spec, target_dir, |_| {})
}

pub fn download_whisper_model_with_progress(
    spec: &WhisperModelSpec,
    target_dir: &Path,
    mut on_progress: impl FnMut(ModelDownloadProgress),
) -> Result<ModelDownloadStatus> {
    fs::create_dir_all(target_dir).with_context(|| {
        format!(
            "failed to create target model directory {}",
            target_dir.display()
        )
    })?;

    let path = target_dir.join(spec.filename);
    if path.exists() {
        verify_model_file(&path, spec.sha256)?;
        return Ok(ModelDownloadStatus {
            path,
            skipped: true,
            verified: true,
        });
    }

    let part_id = NEXT_PART_ID.fetch_add(1, Ordering::Relaxed);
    let part_path = path.with_extension(format!("bin.{}.{part_id}.part", std::process::id()));

    let client = Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60 * 60))
        .build()
        .context("failed to create model download client")?;
    let mut response = client
        .get(spec.url)
        .send()
        .with_context(|| format!("failed to start model download from {}", spec.url))?
        .error_for_status()
        .with_context(|| format!("model download returned error status from {}", spec.url))?;

    let response_total = response.content_length().unwrap_or(spec.size_bytes);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&part_path)
        .with_context(|| format!("failed to create {}", part_path.display()))?;
    let mut hasher = Sha256::new();
    let mut downloaded = 0_u64;
    let mut buffer = vec![0_u8; 256 * 1024];

    let download_result = (|| -> Result<()> {
        loop {
            let read = response
                .read(&mut buffer)
                .context("model download read error")?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read])
                .context("failed to write downloaded model bytes")?;
            hasher.update(&buffer[..read]);
            downloaded += read as u64;
            on_progress(ModelDownloadProgress {
                downloaded,
                total: response_total,
            });
        }
        file.flush().context("failed to flush downloaded model")?;
        file.sync_all().context("failed to sync downloaded model")?;

        let actual = hex::encode(hasher.finalize());
        if !actual.eq_ignore_ascii_case(spec.sha256) {
            return Err(anyhow!(
                "SHA-256 mismatch for {}: expected {}, got {}",
                spec.filename,
                spec.sha256,
                actual
            ));
        }
        Ok(())
    })();

    if let Err(error) = download_result {
        let _ = fs::remove_file(&part_path);
        return Err(error);
    }

    fs::rename(&part_path, &path).with_context(|| {
        format!(
            "failed to move verified model into place {}",
            path.display()
        )
    })?;

    Ok(ModelDownloadStatus {
        path,
        skipped: false,
        verified: true,
    })
}

pub fn verify_model_file(path: &Path, expected_sha256: &str) -> Result<()> {
    let mut file =
        fs::File::open(path).with_context(|| format!("failed to open model {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 256 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("failed to read model {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    let actual = hex::encode(hasher.finalize());
    if actual.eq_ignore_ascii_case(expected_sha256) {
        Ok(())
    } else {
        Err(anyhow!(
            "SHA-256 mismatch for {}: expected {}, got {}",
            path.display(),
            expected_sha256,
            actual
        ))
    }
}
