use crate::contracts::{DesktopError, DesktopEvent, TranscriptionResult, DESKTOP_EVENT_NAME};
use crate::openrouter::{OpenRouterClient, OpenRouterTranscription};
use crate::remote_stream::{RemoteEvent, RemoteProtocol, RemoteStream};
#[cfg(feature = "local-runtime")]
use shadoword_core::OrderedCompletion;
use shadoword_core::{
    DesktopConfig, RecordingSnapshotSource, VadSegment, VadSegmenter, VadSegmenterConfig,
};
#[cfg(feature = "local-runtime")]
use shadoword_core::{InferenceCompletion, InferenceError, InferenceJob, InferenceRuntime};
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "local-runtime")]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
#[cfg(feature = "local-runtime")]
mod local;
mod openrouter;
mod remote;

#[cfg(feature = "local-runtime")]
use local::run_local_streaming;
use openrouter::run_openrouter_streaming;
use remote::run_remote_streaming;

const STREAM_POLL_INTERVAL: Duration = Duration::from_millis(150);
const STREAM_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(10);
#[cfg(feature = "local-runtime")]
const STREAM_INFERENCE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const MAX_VAD_SEGMENT_SECONDS: usize = 100;
const MAX_STREAM_SEGMENTS: u64 = 256;

#[cfg(feature = "local-runtime")]
static NEXT_LOCAL_FLOW_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub enum TranscriptionTarget {
    #[cfg(feature = "local-runtime")]
    Local(Arc<InferenceRuntime>),
    Remote {
        endpoint: String,
        token: Option<String>,
    },
    OpenRouter(OpenRouterStreamTarget),
}

#[derive(Clone)]
pub struct OpenRouterStreamTarget {
    pub(crate) client: OpenRouterClient,
    pub(crate) api_key: String,
    pub(crate) model: String,
    pub(crate) english_only: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum StreamCommand {
    Finish,
    Cancel,
}

pub struct StreamingWorker {
    pub command_tx: mpsc::UnboundedSender<StreamCommand>,
    pub cancelled: Arc<AtomicBool>,
    pub handle: JoinHandle<Result<TranscriptionResult, DesktopError>>,
}

pub fn spawn_streaming_worker(
    app: AppHandle,
    config: DesktopConfig,
    target: TranscriptionTarget,
    source: RecordingSnapshotSource,
    started_at: Instant,
) -> StreamingWorker {
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker_cancelled = Arc::clone(&cancelled);
    let failure_app = app.clone();
    let handle = tokio::spawn(async move {
        let result = match target {
            #[cfg(feature = "local-runtime")]
            TranscriptionTarget::Local(runtime) => {
                run_local_streaming(
                    app,
                    config,
                    runtime,
                    source,
                    started_at,
                    command_rx,
                    worker_cancelled,
                )
                .await
            }
            TranscriptionTarget::Remote { endpoint, token } => {
                run_remote_streaming(
                    app,
                    config,
                    RemoteConnection { endpoint, token },
                    source,
                    started_at,
                    command_rx,
                    worker_cancelled,
                )
                .await
            }
            TranscriptionTarget::OpenRouter(target) => {
                run_openrouter_streaming(
                    app,
                    config,
                    target,
                    source,
                    started_at,
                    command_rx,
                    worker_cancelled,
                )
                .await
            }
        };
        if let Err(error) = &result {
            if error.code != "recording_cancelled" {
                crate::commands::recording::stream_worker_failed(&failure_app, error);
            }
        }
        result
    });
    StreamingWorker {
        command_tx,
        cancelled,
        handle,
    }
}

struct RemoteConnection {
    endpoint: String,
    token: Option<String>,
}

fn capture_segments(
    source: &RecordingSnapshotSource,
    segmenter: &mut VadSegmenter,
    samples_since_segment: &mut usize,
    backlog: &mut VecDeque<VadSegment>,
    max_backlog: usize,
) -> Result<(), DesktopError> {
    let audio = source
        .drain_available()
        .map_err(|error| stream_error("audio_capture", error))?;
    *samples_since_segment = samples_since_segment.saturating_add(audio.samples.len());
    let segments = segmenter.push_samples(&audio.samples);
    if !segments.is_empty() {
        *samples_since_segment = 0;
    }
    enqueue_segments(backlog, segments, max_backlog)?;
    let max_samples = source.sample_rate() as usize * MAX_VAD_SEGMENT_SECONDS;
    if *samples_since_segment >= max_samples {
        if let Some(segment) = segmenter.force_finish() {
            enqueue_segments(backlog, [segment], max_backlog)?;
        }
        *samples_since_segment = 0;
    }
    Ok(())
}

fn enqueue_final_segments(
    source: &RecordingSnapshotSource,
    segmenter: &mut VadSegmenter,
    backlog: &mut VecDeque<VadSegment>,
    max_backlog: usize,
) -> Result<(), DesktopError> {
    let audio = source
        .drain_available()
        .map_err(|error| stream_error("audio_capture", error))?;
    enqueue_segments(backlog, segmenter.push_samples(&audio.samples), max_backlog)?;
    if let Some(segment) = segmenter.force_finish() {
        enqueue_segments(backlog, [segment], max_backlog)?;
    }
    Ok(())
}

fn enqueue_segments(
    backlog: &mut VecDeque<VadSegment>,
    segments: impl IntoIterator<Item = VadSegment>,
    max_backlog: usize,
) -> Result<(), DesktopError> {
    for segment in segments {
        if backlog.len() >= max_backlog.max(1) {
            return Err(stream_error(
                "stream_backpressure",
                "streaming audio backlog exceeded its pending segment limit",
            ));
        }
        backlog.push_back(segment);
    }
    Ok(())
}

async fn deliver_ready_segments(
    app: &AppHandle,
    config: &DesktopConfig,
    results: &mut Vec<TranscriptionResult>,
    ready: Vec<(usize, TranscriptionResult)>,
    cancelled: &AtomicBool,
) {
    for (segment_index, result) in ready {
        if cancelled.load(Ordering::Acquire) {
            break;
        }
        let output = config.output.clone();
        let text = result.text.clone();
        if !text.trim().is_empty() {
            match tokio::task::spawn_blocking(move || {
                crate::output::apply_streaming_segment_output(&output, &text)
            })
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    emit_error(app, "output", "output_delivery", error.to_string(), None)
                }
                Err(error) => emit_error(app, "output", "output_task", error.to_string(), None),
            }
        }
        if cancelled.load(Ordering::Acquire) {
            break;
        }
        results.push(result.clone());
        crate::commands::recording::increment_stream_segment(app);
        let _ = app.emit(
            DESKTOP_EVENT_NAME,
            DesktopEvent::TranscriptSegment {
                result,
                segment_index,
            },
        );
    }
}

async fn finish_stream(
    app: &AppHandle,
    config: &DesktopConfig,
    source: &RecordingSnapshotSource,
    started_at: Instant,
    results: Vec<TranscriptionResult>,
    authoritative_text: Option<String>,
) -> Result<TranscriptionResult, DesktopError> {
    let text = authoritative_text.unwrap_or_else(|| join_transcript(&results));
    let elapsed_ms = results.iter().fold(0_u64, |total, result| {
        total.saturating_add(result.elapsed_ms)
    });
    let engine = results
        .last()
        .map_or_else(|| "whisper".to_string(), |result| result.engine.clone());
    let cost_usd = results
        .iter()
        .map(|result| result.cost_usd)
        .try_fold(0.0_f64, |total, cost| {
            let total = total + cost?;
            total.is_finite().then_some(total)
        })
        .filter(|_| !results.is_empty());
    let result = TranscriptionResult {
        text,
        elapsed_ms,
        engine,
        audio_duration_ms: millis(started_at.elapsed()),
        sample_rate: source.sample_rate(),
        cost_usd,
    };
    let output = config.output.clone();
    let final_text = result.text.clone();
    match tokio::task::spawn_blocking(move || {
        crate::output::apply_final_clipboard(&output, &final_text)
    })
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(error)) => emit_error(app, "output", "output_delivery", error.to_string(), None),
        Err(error) => emit_error(app, "output", "output_task", error.to_string(), None),
    }
    let _ = app.emit(
        DESKTOP_EVENT_NAME,
        DesktopEvent::TranscriptionComplete {
            result: result.clone(),
            segments: results.len(),
        },
    );
    Ok(result)
}

fn segment_duration_ms(segment: &VadSegment) -> u64 {
    u64::from(segment.speech_ms + segment.trailing_silence_ms)
}

fn join_transcript(results: &[TranscriptionResult]) -> String {
    results
        .iter()
        .map(|result| result.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn cancelled_error() -> DesktopError {
    DesktopError::new("recording_cancelled", "recording was cancelled")
}

fn stream_error(context: &'static str, error: impl std::fmt::Display) -> DesktopError {
    DesktopError::new(context, error.to_string()).with_action(
        "Stop the recording, verify the selected model or remote endpoint, then try again.",
    )
}

pub fn emit_error(
    app: &AppHandle,
    context: &str,
    code: &str,
    message: String,
    action: Option<String>,
) {
    let _ = app.emit(
        DESKTOP_EVENT_NAME,
        DesktopEvent::Error {
            code: code.to_string(),
            context: context.to_string(),
            message,
            action,
        },
    );
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}
