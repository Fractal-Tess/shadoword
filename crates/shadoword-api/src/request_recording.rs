use anyhow::{Context, Result};
use serde::Serialize;
use shadoword_core::{AudioInput, TranscriptResponse};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Default)]
pub struct RequestRecorder {
    directory: Option<Arc<PathBuf>>,
    sequence: Arc<AtomicU64>,
}

pub struct PendingRecording {
    audio_path: PathBuf,
    metadata_path: PathBuf,
    source: String,
    created_unix_ms: u128,
    audio_bytes: usize,
}

#[derive(Serialize)]
struct RecordingMetadata<'a> {
    source: &'a str,
    created_unix_ms: u128,
    audio_file: String,
    audio_bytes: usize,
    response: Option<&'a TranscriptResponse>,
    error: Option<&'a str>,
}

impl RequestRecorder {
    pub fn new(directory: Option<PathBuf>) -> Result<Self> {
        if let Some(directory) = &directory {
            fs::create_dir_all(directory).with_context(|| {
                format!(
                    "failed to create API request recording directory {}",
                    directory.display()
                )
            })?;
        }

        Ok(Self {
            directory: directory.map(Arc::new),
            sequence: Arc::new(AtomicU64::new(0)),
        })
    }

    pub fn directory(&self) -> Option<&Path> {
        self.directory.as_deref().map(PathBuf::as_path)
    }

    pub fn record_wav(&self, source: &str, wav: &[u8]) -> Result<Option<PendingRecording>> {
        let Some(directory) = &self.directory else {
            return Ok(None);
        };

        let created_unix_ms = unix_time_ms();
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        let stem = format!(
            "{created_unix_ms}-{}-{sequence:06}-{}",
            std::process::id(),
            safe_label(source)
        );
        let audio_path = directory.join(format!("{stem}.wav"));
        write_atomic(&audio_path, wav).with_context(|| {
            format!(
                "failed to archive API audio request at {}",
                audio_path.display()
            )
        })?;

        tracing::info!(
            source,
            bytes = wav.len(),
            path = %audio_path.display(),
            "archived API audio request"
        );

        Ok(Some(PendingRecording {
            metadata_path: directory.join(format!("{stem}.json")),
            audio_path,
            source: source.to_string(),
            created_unix_ms,
            audio_bytes: wav.len(),
        }))
    }

    pub fn record_audio(
        &self,
        source: &str,
        audio: &AudioInput,
    ) -> Result<Option<PendingRecording>> {
        if self.directory.is_none() {
            return Ok(None);
        }
        let wav = shadoword_core::wav::encode_wav(audio)?;
        self.record_wav(source, &wav)
    }

    pub fn record_success(
        &self,
        recording: Option<PendingRecording>,
        response: &TranscriptResponse,
    ) -> Result<()> {
        self.write_metadata(recording, Some(response), None)
    }

    pub fn record_error(&self, recording: Option<PendingRecording>, error: &str) -> Result<()> {
        self.write_metadata(recording, None, Some(error))
    }

    fn write_metadata(
        &self,
        recording: Option<PendingRecording>,
        response: Option<&TranscriptResponse>,
        error: Option<&str>,
    ) -> Result<()> {
        let Some(recording) = recording else {
            return Ok(());
        };
        let audio_file = recording
            .audio_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        let metadata = RecordingMetadata {
            source: &recording.source,
            created_unix_ms: recording.created_unix_ms,
            audio_file,
            audio_bytes: recording.audio_bytes,
            response,
            error,
        };
        let json = serde_json::to_vec_pretty(&metadata)
            .context("failed to serialize API request recording metadata")?;
        write_atomic(&recording.metadata_path, &json).with_context(|| {
            format!(
                "failed to write API request metadata at {}",
                recording.metadata_path.display()
            )
        })?;
        Ok(())
    }
}

fn safe_label(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn unix_time_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    let temporary = path.with_extension(format!(
        "{}.part",
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("tmp")
    ));
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary)
        .with_context(|| format!("failed to create {}", temporary.display()))?;
    file.write_all(contents)
        .with_context(|| format!("failed to write {}", temporary.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to sync {}", temporary.display()))?;
    fs::rename(&temporary, path)
        .with_context(|| format!("failed to move recording into {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "shadoword-request-recording-{}-{}",
            std::process::id(),
            unix_time_ms()
        ))
    }

    #[test]
    fn archives_audio_and_response_metadata() {
        let directory = temp_dir();
        let recorder =
            RequestRecorder::new(Some(directory.clone())).expect("create request recorder");
        let response = TranscriptResponse {
            text: "hello".to_string(),
            elapsed_ms: 42,
            engine: "test".to_string(),
        };

        let pending = recorder
            .record_wav("batch", b"RIFFtest")
            .expect("record request");
        recorder
            .record_success(pending, &response)
            .expect("record response");

        let files = fs::read_dir(&directory)
            .expect("read recording directory")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        assert_eq!(
            files
                .iter()
                .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("wav"))
                .count(),
            1
        );
        let metadata_path = files
            .iter()
            .find(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
            .expect("metadata file");
        let metadata = fs::read_to_string(metadata_path).expect("read metadata");
        assert!(metadata.contains("hello"));
        assert!(metadata.contains("batch"));

        fs::remove_dir_all(directory).expect("remove recording directory");
    }
}
