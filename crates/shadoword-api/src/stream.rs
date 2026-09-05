use crate::error::ApiError;
use crate::request_recording::PendingRecording;
use crate::router::AppState;
use axum::extract::ws::{Message, WebSocket};
use futures_util::StreamExt;
use opus::{Channels, Decoder};
use serde::{Deserialize, Serialize};
use shadoword_core::{
    AudioInput, InferenceCompletion, InferenceError, InferenceJob, OrderedCompletion,
    TranscriptResponse, VadSegment, VadSegmenter, VadSegmenterConfig,
};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
mod pcm;
mod session;

use pcm::*;

pub const MAX_OPUS_PACKET_BYTES: usize = 64 * 1024;
pub const PCM_F32LE_WIRE_BYTES_PER_SAMPLE: usize = std::mem::size_of::<f32>();
pub const PCM_S16LE_WIRE_BYTES_PER_SAMPLE: usize = std::mem::size_of::<i16>();
pub const DECODED_PCM_BYTES_PER_SAMPLE: usize = std::mem::size_of::<f32>();
pub const MAX_SEGMENT_SECONDS: usize = 120;
pub const MAX_STREAM_SEGMENTS: usize = 256;
pub const STREAM_SAMPLE_RATE: u32 = 48_000;
pub const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const INFERENCE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const MAX_TRANSCRIPT_BYTES: usize = 4 * 1024 * 1024;
const MAX_OPUS_FRAME_MILLISECONDS: usize = 120;
pub const MAX_PCM_PACKET_MILLISECONDS: usize = 250;
const MAX_VAD_SEGMENT_SECONDS: usize = 100;
const PROTOCOL_V1: u8 = 1;
const PROTOCOL_V2: u8 = 2;
const PROTOCOL_V3: u8 = 3;

static NEXT_FLOW_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Default, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StreamAudioFormat {
    #[default]
    Opus,
    PcmF32le,
    PcmS16le,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PcmWireFormat {
    F32Le,
    S16Le,
}

impl PcmWireFormat {
    fn bytes_per_sample(self) -> usize {
        match self {
            Self::F32Le => PCM_F32LE_WIRE_BYTES_PER_SAMPLE,
            Self::S16Le => PCM_S16LE_WIRE_BYTES_PER_SAMPLE,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::F32Le => "pcm_f32le",
            Self::S16Le => "pcm_s16le",
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct StartMessage {
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
    #[serde(default = "default_channels")]
    pub channels: usize,
    #[serde(default = "default_protocol_version")]
    pub protocol_version: u8,
    #[serde(default)]
    pub audio_format: StreamAudioFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientText {
    Start(StartMessage),
    CommitSegment { segment_index: Option<u64> },
    Finish,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ServerMessage<'a> {
    Started {
        protocol_version: u8,
        flow_id: &'a str,
        credit: usize,
    },
    Accepted {
        segment_index: u64,
        outstanding: usize,
        remaining_credit: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        audio_duration_ms: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        sample_rate: Option<u32>,
    },
    Partial {
        segment_index: usize,
        text: &'a str,
        elapsed_ms: u128,
        engine: &'a str,
    },
    Done {
        text: &'a str,
        segments: &'a [String],
    },
    Error {
        code: &'a str,
        message: &'a str,
    },
}

struct JobCompletion {
    sequence: u64,
    result: Result<InferenceCompletion, InferenceError>,
}

enum StreamAudioInput {
    Opus {
        decoder: Decoder,
    },
    Pcm {
        wire_format: PcmWireFormat,
        segmenter: VadSegmenter,
        samples_since_segment: usize,
        max_segment_samples: usize,
    },
}

struct StreamSession {
    audio_input: StreamAudioInput,
    channels: usize,
    sample_rate: u32,
    protocol_version: u8,
    flow_id: String,
    credit: usize,
    samples: Vec<f32>,
    next_sequence: u64,
    ordered: OrderedCompletion<TranscriptResponse>,
    outstanding: HashMap<u64, Arc<InferenceJob>>,
    pending_segments: VecDeque<VadSegment>,
    pending_audio_bytes: usize,
    max_pending_audio_bytes: usize,
    max_pending_segments: usize,
    target_sample_rate: u32,
    segments: Vec<String>,
    transcript_bytes: usize,
    finishing: bool,
}

pub fn parse_client_text(text: &str) -> Result<ClientText, String> {
    let trimmed = text.trim();
    if trimmed.eq_ignore_ascii_case("CommitSegment") {
        return Ok(ClientText::CommitSegment {
            segment_index: None,
        });
    }
    if trimmed.eq_ignore_ascii_case("Finish") {
        return Ok(ClientText::Finish);
    }

    let value: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|_| "expected Start JSON, CommitSegment, or Finish".to_string())?;
    let message_type = value
        .get("type")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "text JSON must include a type field".to_string())?;

    match message_type {
        "Start" | "start" => {
            let start: StartMessage =
                serde_json::from_value(value).map_err(|_| "invalid Start message".to_string())?;
            validate_start(&start)?;
            Ok(ClientText::Start(start))
        }
        "CommitSegment" | "commit_segment" => {
            let segment_index = value
                .get("segment_index")
                .map(|index| {
                    index
                        .as_u64()
                        .ok_or_else(|| "segment_index must be a non-negative integer".to_string())
                })
                .transpose()?;
            Ok(ClientText::CommitSegment { segment_index })
        }
        "Finish" | "finish" => Ok(ClientText::Finish),
        _ => Err("unknown text message type".to_string()),
    }
}

pub async fn handle_stream(mut socket: WebSocket, state: AppState) {
    let (completion_tx, mut completion_rx) = mpsc::channel(MAX_STREAM_SEGMENTS);
    let mut session: Option<StreamSession> = None;

    loop {
        let waiting_for_inference = session
            .as_ref()
            .is_some_and(|session| !session.outstanding.is_empty());
        let wait_duration = if waiting_for_inference {
            INFERENCE_TIMEOUT
        } else {
            STREAM_IDLE_TIMEOUT
        };
        let event = tokio::select! {
            message = tokio::time::timeout(wait_duration, socket.next()) => StreamEvent::Socket(message),
            completion = completion_rx.recv(), if session.is_some() => StreamEvent::Completion(completion),
        };

        let result = match event {
            StreamEvent::Socket(Ok(Some(Ok(Message::Text(text))))) => {
                handle_text(
                    &mut socket,
                    &state,
                    &completion_tx,
                    &mut session,
                    text.as_str(),
                )
                .await
            }
            StreamEvent::Socket(Ok(Some(Ok(Message::Binary(packet))))) => {
                handle_packet(&mut socket, &state, &completion_tx, &mut session, &packet).await
            }
            StreamEvent::Socket(Ok(Some(Ok(Message::Close(_)))))
            | StreamEvent::Socket(Ok(None)) => break,
            StreamEvent::Socket(Ok(Some(Ok(Message::Ping(_) | Message::Pong(_))))) => Ok(false),
            StreamEvent::Socket(Ok(Some(Err(error)))) => {
                tracing::debug!(error = %error, "websocket receive failed");
                break;
            }
            StreamEvent::Socket(Err(_)) if waiting_for_inference => Err(ApiError::timeout()),
            StreamEvent::Socket(Err(_)) => Err(ApiError::idle_timeout()),
            StreamEvent::Completion(Some(completion)) => {
                handle_completion(
                    &mut socket,
                    &state,
                    &completion_tx,
                    session.as_mut(),
                    completion,
                )
                .await
            }
            StreamEvent::Completion(None) => Err(ApiError::internal(anyhow::anyhow!(
                "stream completion channel closed"
            ))),
        };

        match result {
            Ok(true) => break,
            Ok(false) => {}
            Err(error) => {
                if let Some(session) = session.as_mut() {
                    session.cancel_outstanding();
                }
                let _ = send_error(&mut socket, error.code(), error.message()).await;
                break;
            }
        }
    }
}

enum StreamEvent {
    Socket(Result<Option<Result<Message, axum::Error>>, tokio::time::error::Elapsed>),
    Completion(Option<JobCompletion>),
}

async fn handle_text(
    socket: &mut WebSocket,
    state: &AppState,
    completion_tx: &mpsc::Sender<JobCompletion>,
    session: &mut Option<StreamSession>,
    text: &str,
) -> Result<bool, ApiError> {
    match parse_client_text(text).map_err(ApiError::bad_request)? {
        ClientText::Start(start) => {
            if session.is_some() {
                return Err(ApiError::bad_request("stream already started"));
            }
            let started = StreamSession::new(start, state)?;
            if started.protocol_version >= PROTOCOL_V2 {
                send_json(
                    socket,
                    &ServerMessage::Started {
                        protocol_version: started.protocol_version,
                        flow_id: &started.flow_id,
                        credit: started.credit,
                    },
                )
                .await?;
            }
            *session = Some(started);
            Ok(false)
        }
        ClientText::CommitSegment { segment_index } => {
            let session = session
                .as_mut()
                .ok_or_else(|| ApiError::bad_request("stream must start before commit"))?;
            if session.protocol_version == PROTOCOL_V3 {
                return Err(ApiError::bad_request(
                    "protocol v3 commits segments automatically using server VAD",
                ));
            }
            let ready = session
                .commit_segment(state, completion_tx, segment_index, false)
                .await?;
            if session.protocol_version == PROTOCOL_V2 {
                send_accepted(socket, session.acceptance(None)).await?;
            }
            emit_ready(socket, session, ready).await?;
            Ok(false)
        }
        ClientText::Finish => {
            let session = session
                .as_mut()
                .ok_or_else(|| ApiError::bad_request("stream must start before finish"))?;
            if session.finishing {
                return Err(ApiError::bad_request("stream is already finishing"));
            }
            if session.protocol_version == PROTOCOL_V3 {
                if let Some(segment) = session.force_finish_vad() {
                    session.enqueue_vad_segment(segment)?;
                }
                submit_pending_vad(socket, state, completion_tx, session).await?;
            } else if !session.samples.is_empty() {
                let ready = session
                    .commit_segment(state, completion_tx, None, true)
                    .await?;
                if session.protocol_version == PROTOCOL_V2 {
                    send_accepted(socket, session.acceptance(None)).await?;
                }
                emit_ready(socket, session, ready).await?;
            }
            session.finishing = true;
            maybe_finish(socket, session).await
        }
    }
}

async fn handle_packet(
    socket: &mut WebSocket,
    state: &AppState,
    completion_tx: &mpsc::Sender<JobCompletion>,
    session: &mut Option<StreamSession>,
    packet: &[u8],
) -> Result<bool, ApiError> {
    let session = session
        .as_mut()
        .ok_or_else(|| ApiError::bad_request("stream must start before binary packets"))?;
    if session.finishing {
        return Err(ApiError::bad_request("stream is already finishing"));
    }

    for segment in session.accept_packet(packet)? {
        session.enqueue_vad_segment(segment)?;
    }
    submit_pending_vad(socket, state, completion_tx, session).await?;
    Ok(false)
}

#[derive(Clone, Copy)]
struct SegmentAcceptance {
    segment_index: u64,
    outstanding: usize,
    remaining_credit: usize,
    metadata: Option<SegmentMetadata>,
}

#[derive(Clone, Copy)]
struct SegmentMetadata {
    audio_duration_ms: u64,
    sample_rate: u32,
}

impl From<&VadSegment> for SegmentMetadata {
    fn from(segment: &VadSegment) -> Self {
        Self {
            audio_duration_ms: u64::from(
                segment
                    .speech_ms
                    .saturating_add(segment.trailing_silence_ms),
            ),
            sample_rate: segment.audio.sample_rate,
        }
    }
}

async fn submit_pending_vad(
    socket: &mut WebSocket,
    state: &AppState,
    completion_tx: &mpsc::Sender<JobCompletion>,
    session: &mut StreamSession,
) -> Result<(), ApiError> {
    while session.window_len() < session.credit {
        let Some(segment) = session.pop_pending_vad_segment() else {
            break;
        };
        let metadata = SegmentMetadata::from(&segment);
        let ready = session
            .commit_vad_segment(state, completion_tx, segment)
            .await?;
        send_accepted(socket, session.acceptance(Some(metadata))).await?;
        emit_ready(socket, session, ready).await?;
    }
    Ok(())
}

async fn send_accepted(
    socket: &mut WebSocket,
    acceptance: SegmentAcceptance,
) -> Result<(), ApiError> {
    send_json(
        socket,
        &ServerMessage::Accepted {
            segment_index: acceptance.segment_index,
            outstanding: acceptance.outstanding,
            remaining_credit: acceptance.remaining_credit,
            audio_duration_ms: acceptance
                .metadata
                .map(|metadata| metadata.audio_duration_ms),
            sample_rate: acceptance.metadata.map(|metadata| metadata.sample_rate),
        },
    )
    .await
}

async fn handle_completion(
    socket: &mut WebSocket,
    state: &AppState,
    completion_tx: &mpsc::Sender<JobCompletion>,
    session: Option<&mut StreamSession>,
    completion: JobCompletion,
) -> Result<bool, ApiError> {
    let session = session.ok_or_else(|| ApiError::bad_request("stream is not active"))?;
    if session.outstanding.remove(&completion.sequence).is_none() {
        return Ok(false);
    }
    let response = match completion.result {
        Ok(response) => response,
        Err(error) => {
            if !terminate_on_completion_failure(session, completion.sequence, &error) {
                return Ok(false);
            }
            return Err(ApiError::from_inference(error));
        }
    };
    let ready = session.complete(completion.sequence, response.response)?;
    emit_ready(socket, session, ready).await?;
    if session.protocol_version == PROTOCOL_V3 {
        submit_pending_vad(socket, state, completion_tx, session).await?;
    }
    maybe_finish(socket, session).await
}

fn terminate_on_completion_failure(
    session: &mut StreamSession,
    sequence: u64,
    error: &InferenceError,
) -> bool {
    if session.ordered.fail(sequence, error.to_string()).is_err() {
        return false;
    }
    session.cancel_outstanding();
    true
}

async fn emit_ready(
    socket: &mut WebSocket,
    session: &mut StreamSession,
    ready: Vec<(usize, TranscriptResponse)>,
) -> Result<(), ApiError> {
    for (segment_index, response) in ready {
        send_json(
            socket,
            &ServerMessage::Partial {
                segment_index,
                text: &response.text,
                elapsed_ms: response.elapsed_ms,
                engine: &response.engine,
            },
        )
        .await?;
        session.segments.push(response.text);
    }
    Ok(())
}

async fn maybe_finish(
    socket: &mut WebSocket,
    session: &mut StreamSession,
) -> Result<bool, ApiError> {
    if !session.finishing
        || !session.outstanding.is_empty()
        || !session.pending_segments.is_empty()
        || session.ordered.next_sequence() != session.next_sequence
    {
        return Ok(false);
    }
    let text = session
        .segments
        .iter()
        .filter(|segment| !segment.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    send_json(
        socket,
        &ServerMessage::Done {
            text: &text,
            segments: &session.segments,
        },
    )
    .await?;
    Ok(true)
}

async fn send_error(socket: &mut WebSocket, code: &str, message: &str) -> Result<(), ApiError> {
    send_json(socket, &ServerMessage::Error { code, message }).await
}

async fn send_json(socket: &mut WebSocket, message: &ServerMessage<'_>) -> Result<(), ApiError> {
    let text = serde_json::to_string(message)
        .map_err(|_| ApiError::internal(anyhow::anyhow!("failed to encode websocket message")))?;
    socket
        .send(Message::Text(text.into()))
        .await
        .map_err(|_| ApiError::bad_request("websocket send failed"))
}

fn spawn_completion_wait(
    sequence: u64,
    job: Arc<InferenceJob>,
    recording: Option<PendingRecording>,
    recorder: crate::request_recording::RequestRecorder,
    completion_tx: mpsc::Sender<JobCompletion>,
) {
    tokio::spawn(async move {
        let result = tokio::task::spawn_blocking(move || {
            let result = job.wait();
            let metadata_result = match &result {
                Ok(completion) => recorder.record_success(recording, &completion.response),
                Err(error) => recorder.record_error(recording, &error.to_string()),
            };
            if let Err(error) = metadata_result {
                tracing::warn!(error = %error, "failed to archive streaming response metadata");
            }
            result
        })
        .await
        .unwrap_or_else(|error| Err(InferenceError::WorkerFailed(error.to_string())));
        let _ = completion_tx.send(JobCompletion { sequence, result }).await;
    });
}

async fn record_submit_error(
    state: &AppState,
    recording: Option<PendingRecording>,
    message: String,
) -> Result<(), ApiError> {
    let recorder = state.request_recorder.clone();
    tokio::task::spawn_blocking(move || {
        if let Err(error) = recorder.record_error(recording, &message) {
            tracing::warn!(error = %error, "failed to archive streaming response metadata");
        }
    })
    .await
    .map_err(ApiError::from_join)
}
