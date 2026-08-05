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

impl StreamSession {
    fn new(start: StartMessage, state: &AppState) -> Result<Self, ApiError> {
        validate_start(&start).map_err(ApiError::bad_request)?;
        let transcription_config = state.runtime.transcription_config();
        let target_sample_rate = transcription_config.sample_rate;
        let limits = transcription_config
            .effective_inference_pool()
            .map_err(ApiError::internal)?
            .limits;
        let audio_input = match start.audio_format {
            StreamAudioFormat::Opus => StreamAudioInput::Opus {
                decoder: Decoder::new(start.sample_rate, Channels::Mono)
                    .map_err(|_| ApiError::bad_request("unsupported opus stream parameters"))?,
            },
            StreamAudioFormat::PcmF32le => pcm_audio_input(
                start.sample_rate,
                target_sample_rate,
                limits.max_audio_bytes_per_job,
                PcmWireFormat::F32Le,
            ),
            StreamAudioFormat::PcmS16le => pcm_audio_input(
                start.sample_rate,
                target_sample_rate,
                limits.max_audio_bytes_per_job,
                PcmWireFormat::S16Le,
            ),
        };
        let credit = if start.protocol_version == PROTOCOL_V1 {
            1
        } else {
            limits
                .max_outstanding_per_flow
                .min(
                    limits
                        .max_sequencer_buffered_results_per_flow
                        .saturating_add(1),
                )
                .min(MAX_STREAM_SEGMENTS)
        };
        let flow_number = NEXT_FLOW_ID.fetch_add(1, Ordering::Relaxed);
        let pending_capacity = credit.saturating_add(1).min(MAX_STREAM_SEGMENTS);
        Ok(Self {
            audio_input,
            channels: start.channels,
            sample_rate: start.sample_rate,
            protocol_version: start.protocol_version,
            flow_id: format!("stream-{}-{flow_number}", std::process::id()),
            credit,
            samples: Vec::new(),
            next_sequence: 0,
            ordered: OrderedCompletion::new(0, limits.max_sequencer_buffered_results_per_flow),
            outstanding: HashMap::new(),
            pending_segments: VecDeque::new(),
            pending_audio_bytes: 0,
            max_pending_audio_bytes: limits.max_queued_audio_bytes.min(
                limits
                    .max_audio_bytes_per_job
                    .saturating_mul(pending_capacity),
            ),
            max_pending_segments: pending_capacity,
            target_sample_rate,
            segments: Vec::new(),
            transcript_bytes: 0,
            finishing: false,
        })
    }

    fn accept_packet(&mut self, packet: &[u8]) -> Result<Vec<VadSegment>, ApiError> {
        if packet.len() > MAX_OPUS_PACKET_BYTES {
            return Err(ApiError::payload_too_large(
                "streaming audio packet too large",
            ));
        }

        match &mut self.audio_input {
            StreamAudioInput::Opus { decoder } => {
                let max_frame_samples =
                    self.sample_rate as usize * MAX_OPUS_FRAME_MILLISECONDS / 1_000;
                let mut pcm = vec![0.0; max_frame_samples * self.channels];
                let samples_per_channel = decoder
                    .decode_float(packet, &mut pcm, false)
                    .map_err(|_| ApiError::bad_request("invalid opus packet"))?;
                let written = samples_per_channel * self.channels;
                let mono = downmix_to_mono(&pcm[..written], self.channels);
                let max_segment_samples = self.sample_rate as usize * MAX_SEGMENT_SECONDS;
                if self.samples.len().saturating_add(mono.len()) > max_segment_samples {
                    return Err(ApiError::payload_too_large("stream segment too large"));
                }
                self.samples.extend(mono);
                Ok(Vec::new())
            }
            StreamAudioInput::Pcm {
                wire_format,
                segmenter,
                samples_since_segment,
                max_segment_samples,
            } => {
                if packet.len() > max_pcm_packet_bytes(self.sample_rate, *wire_format) {
                    return Err(ApiError::payload_too_large(format!(
                        "{} packet exceeds the packet duration or websocket byte limit",
                        wire_format.name()
                    )));
                }
                let samples = decode_pcm(*wire_format, packet)?;
                if samples.len() > *max_segment_samples {
                    return Err(ApiError::payload_too_large(format!(
                        "{} packet exceeds the per-job audio limit",
                        wire_format.name()
                    )));
                }
                let mut segments = Vec::new();
                if samples_since_segment.saturating_add(samples.len()) > *max_segment_samples {
                    if let Some(segment) = segmenter.force_finish() {
                        segments.push(segment);
                    }
                    *samples_since_segment = 0;
                }
                *samples_since_segment = samples_since_segment.saturating_add(samples.len());
                let detected = segmenter.push_samples(&samples);
                if !detected.is_empty() {
                    *samples_since_segment = 0;
                }
                segments.extend(detected);
                if *samples_since_segment >= *max_segment_samples {
                    if let Some(segment) = segmenter.force_finish() {
                        segments.push(segment);
                    }
                    *samples_since_segment = 0;
                }
                Ok(segments)
            }
        }
    }

    fn force_finish_vad(&mut self) -> Option<VadSegment> {
        match &mut self.audio_input {
            StreamAudioInput::Pcm {
                segmenter,
                samples_since_segment,
                ..
            } => {
                *samples_since_segment = 0;
                segmenter.force_finish()
            }
            StreamAudioInput::Opus { .. } => None,
        }
    }

    fn enqueue_vad_segment(&mut self, segment: VadSegment) -> Result<(), ApiError> {
        let queued_segments = self
            .next_sequence
            .saturating_add(self.pending_segments.len() as u64);
        if queued_segments as usize >= MAX_STREAM_SEGMENTS {
            return Err(ApiError::payload_too_large(
                "stream contains too many segments",
            ));
        }
        let audio_bytes = estimated_inference_audio_bytes(
            segment.audio.samples.len(),
            segment.audio.sample_rate,
            self.target_sample_rate,
        )
        .ok_or_else(|| ApiError::payload_too_large("stream audio is too large"))?;
        let pending_audio_bytes = self
            .pending_audio_bytes
            .checked_add(audio_bytes)
            .ok_or_else(|| ApiError::payload_too_large("stream audio is too large"))?;
        if self.pending_segments.len() >= self.max_pending_segments
            || pending_audio_bytes > self.max_pending_audio_bytes
        {
            return Err(ApiError::from_inference(InferenceError::FlowLimit));
        }
        self.pending_audio_bytes = pending_audio_bytes;
        self.pending_segments.push_back(segment);
        Ok(())
    }

    fn pop_pending_vad_segment(&mut self) -> Option<VadSegment> {
        let segment = self.pending_segments.pop_front()?;
        let audio_bytes = estimated_inference_audio_bytes(
            segment.audio.samples.len(),
            segment.audio.sample_rate,
            self.target_sample_rate,
        )
        .unwrap_or(usize::MAX);
        self.pending_audio_bytes = self.pending_audio_bytes.saturating_sub(audio_bytes);
        Some(segment)
    }

    async fn commit_vad_segment(
        &mut self,
        state: &AppState,
        completion_tx: &mpsc::Sender<JobCompletion>,
        segment: VadSegment,
    ) -> Result<Vec<(usize, TranscriptResponse)>, ApiError> {
        self.submit_audio(state, completion_tx, segment.audio, true)
            .await
    }

    async fn commit_segment(
        &mut self,
        state: &AppState,
        completion_tx: &mpsc::Sender<JobCompletion>,
        requested_index: Option<u64>,
        implicit_finish: bool,
    ) -> Result<Vec<(usize, TranscriptResponse)>, ApiError> {
        if self.protocol_version == PROTOCOL_V1 && requested_index.is_some() {
            return Err(ApiError::bad_request(
                "segment_index is only supported by stream protocol v2",
            ));
        }
        if self.protocol_version == PROTOCOL_V2 && !implicit_finish {
            let requested_index = requested_index.ok_or_else(|| {
                ApiError::bad_request("protocol v2 CommitSegment requires segment_index")
            })?;
            if requested_index != self.next_sequence {
                return Err(ApiError::bad_request(format!(
                    "expected segment_index {}, received {requested_index}",
                    self.next_sequence
                )));
            }
        }

        let samples = std::mem::take(&mut self.samples);
        if samples.is_empty() {
            self.ensure_commit_capacity(false)?;
            let sequence = self.next_sequence;
            self.next_sequence += 1;
            return self.complete(
                sequence,
                TranscriptResponse {
                    text: String::new(),
                    elapsed_ms: 0,
                    engine: "whisper".to_string(),
                },
            );
        }

        self.submit_audio(
            state,
            completion_tx,
            AudioInput {
                samples,
                sample_rate: self.sample_rate,
            },
            false,
        )
        .await
    }

    fn ensure_commit_capacity(&self, allow_finishing: bool) -> Result<(), ApiError> {
        if self.finishing && !allow_finishing {
            return Err(ApiError::bad_request("stream is already finishing"));
        }
        if self.next_sequence as usize >= MAX_STREAM_SEGMENTS {
            return Err(ApiError::payload_too_large(
                "stream contains too many segments",
            ));
        }
        if self.window_len() >= self.credit {
            return Err(ApiError::from_inference(InferenceError::FlowLimit));
        }
        Ok(())
    }

    async fn submit_audio(
        &mut self,
        state: &AppState,
        completion_tx: &mpsc::Sender<JobCompletion>,
        input: AudioInput,
        allow_finishing: bool,
    ) -> Result<Vec<(usize, TranscriptResponse)>, ApiError> {
        self.ensure_commit_capacity(allow_finishing)?;
        let sequence = self.next_sequence;
        let recorder = state.request_recorder.clone();
        let source = format!("{}-segment-{sequence:03}", self.flow_id);
        let (input, recording) = tokio::task::spawn_blocking(move || {
            let recording = match recorder.record_audio(&source, &input) {
                Ok(recording) => recording,
                Err(error) => {
                    tracing::warn!(error = %error, "failed to archive streaming audio request");
                    None
                }
            };
            (input, recording)
        })
        .await
        .map_err(ApiError::from_join)?;

        let job = match state
            .runtime
            .submit_stream(self.flow_id.clone(), sequence, input)
        {
            Ok(job) => Arc::new(job),
            Err(error) => {
                record_submit_error(state, recording, error.to_string()).await?;
                return Err(ApiError::from_inference(error));
            }
        };
        self.next_sequence += 1;
        self.outstanding.insert(sequence, Arc::clone(&job));
        spawn_completion_wait(
            sequence,
            job,
            recording,
            state.request_recorder.clone(),
            completion_tx.clone(),
        );
        Ok(Vec::new())
    }

    fn complete(
        &mut self,
        sequence: u64,
        response: TranscriptResponse,
    ) -> Result<Vec<(usize, TranscriptResponse)>, ApiError> {
        self.transcript_bytes = self
            .transcript_bytes
            .checked_add(response.text.len())
            .ok_or_else(|| ApiError::payload_too_large("stream transcript is too large"))?;
        if self.transcript_bytes > MAX_TRANSCRIPT_BYTES {
            if self
                .ordered
                .fail(sequence, "stream transcript is too large")
                .is_ok()
            {
                self.cancel_outstanding();
            }
            return Err(ApiError::payload_too_large(
                "stream transcript is too large",
            ));
        }
        let first = self.ordered.next_sequence();
        let ready = self
            .ordered
            .complete(sequence, response)
            .map_err(|error| ApiError::bad_request(error.to_string()))?;
        Ok(ready
            .into_iter()
            .enumerate()
            .map(|(offset, response)| (first as usize + offset, response))
            .collect())
    }

    fn window_len(&self) -> usize {
        self.next_sequence
            .saturating_sub(self.ordered.next_sequence()) as usize
    }

    fn acceptance(&self, metadata: Option<SegmentMetadata>) -> SegmentAcceptance {
        let outstanding = self.window_len();
        SegmentAcceptance {
            segment_index: self.next_sequence.saturating_sub(1),
            outstanding,
            remaining_credit: self.credit.saturating_sub(outstanding),
            metadata,
        }
    }

    fn cancel_outstanding(&mut self) {
        for job in self.outstanding.values() {
            job.cancel();
        }
        self.outstanding.clear();
    }
}

impl Drop for StreamSession {
    fn drop(&mut self) {
        self.cancel_outstanding();
    }
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

fn validate_start(start: &StartMessage) -> Result<(), String> {
    let sample_rate_valid = match start.audio_format {
        StreamAudioFormat::Opus => {
            matches!(start.sample_rate, 8_000 | 12_000 | 16_000 | 24_000 | 48_000)
        }
        StreamAudioFormat::PcmF32le | StreamAudioFormat::PcmS16le => {
            (8_000..=384_000).contains(&start.sample_rate)
        }
    };
    if !sample_rate_valid {
        return Err("Start sample_rate is unsupported for the selected audio_format".to_string());
    }
    if start.channels != 1 {
        return Err("Start channels must be 1 (mono)".to_string());
    }
    match (start.protocol_version, start.audio_format) {
        (PROTOCOL_V1 | PROTOCOL_V2, StreamAudioFormat::Opus)
        | (PROTOCOL_V3, StreamAudioFormat::PcmF32le | StreamAudioFormat::PcmS16le) => Ok(()),
        (PROTOCOL_V1 | PROTOCOL_V2, StreamAudioFormat::PcmF32le | StreamAudioFormat::PcmS16le) => {
            Err("raw PCM streaming requires protocol_version 3".to_string())
        }
        (PROTOCOL_V3, StreamAudioFormat::Opus) => {
            Err("protocol_version 3 requires audio_format pcm_f32le or pcm_s16le".to_string())
        }
        _ => Err("Start protocol_version must be 1, 2, or 3".to_string()),
    }
}

fn pcm_audio_input(
    source_rate: u32,
    target_rate: u32,
    max_audio_bytes: usize,
    wire_format: PcmWireFormat,
) -> StreamAudioInput {
    StreamAudioInput::Pcm {
        wire_format,
        segmenter: VadSegmenter::new(source_rate, VadSegmenterConfig::default()),
        samples_since_segment: 0,
        max_segment_samples: max_vad_segment_samples(
            max_audio_bytes,
            source_rate,
            target_rate,
            wire_format,
        ),
    }
}

fn max_pcm_packet_bytes(sample_rate: u32, wire_format: PcmWireFormat) -> usize {
    (sample_rate as usize)
        .saturating_mul(MAX_PCM_PACKET_MILLISECONDS)
        .saturating_div(1_000)
        .saturating_mul(wire_format.bytes_per_sample())
        .min(MAX_OPUS_PACKET_BYTES)
}

fn max_pcm_packet_samples(sample_rate: u32, wire_format: PcmWireFormat) -> usize {
    max_pcm_packet_bytes(sample_rate, wire_format) / wire_format.bytes_per_sample()
}

fn max_vad_segment_samples(
    max_audio_bytes: usize,
    source_rate: u32,
    target_rate: u32,
    wire_format: PcmWireFormat,
) -> usize {
    let duration_limit = (source_rate as usize).saturating_mul(MAX_VAD_SEGMENT_SECONDS);
    duration_limit
        .min(max_input_samples_for_inference(
            max_audio_bytes,
            source_rate,
            target_rate,
        ))
        .saturating_sub(max_pcm_packet_samples(source_rate, wire_format))
}

fn estimated_inference_audio_bytes(
    samples: usize,
    source_rate: u32,
    target_rate: u32,
) -> Option<usize> {
    let raw_bytes = samples.checked_mul(DECODED_PCM_BYTES_PER_SAMPLE)?;
    if source_rate == target_rate {
        return Some(raw_bytes);
    }
    if source_rate == 0 {
        return None;
    }
    let resampled_samples = (samples as u128)
        .checked_mul(u128::from(target_rate))?
        .div_ceil(u128::from(source_rate));
    let resampled_bytes = usize::try_from(resampled_samples)
        .ok()?
        .checked_mul(DECODED_PCM_BYTES_PER_SAMPLE)?;
    raw_bytes.checked_add(resampled_bytes)
}

fn max_input_samples_for_inference(
    max_audio_bytes: usize,
    source_rate: u32,
    target_rate: u32,
) -> usize {
    let mut lower = 0_usize;
    let mut upper = (max_audio_bytes / DECODED_PCM_BYTES_PER_SAMPLE).saturating_add(1);
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let fits = estimated_inference_audio_bytes(middle, source_rate, target_rate)
            .is_some_and(|bytes| bytes <= max_audio_bytes);
        if fits {
            lower = middle + 1;
        } else {
            upper = middle;
        }
    }
    lower.saturating_sub(1)
}

fn decode_pcm(wire_format: PcmWireFormat, packet: &[u8]) -> Result<Vec<f32>, ApiError> {
    match wire_format {
        PcmWireFormat::F32Le => decode_pcm_f32le(packet),
        PcmWireFormat::S16Le => decode_pcm_s16le(packet),
    }
}

fn decode_pcm_f32le(packet: &[u8]) -> Result<Vec<f32>, ApiError> {
    if !packet.len().is_multiple_of(PCM_F32LE_WIRE_BYTES_PER_SAMPLE) {
        return Err(ApiError::bad_request(
            "pcm_f32le packet length must be divisible by 4",
        ));
    }
    packet
        .chunks_exact(PCM_F32LE_WIRE_BYTES_PER_SAMPLE)
        .map(|bytes| {
            let sample = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            if !sample.is_finite() {
                return Err(ApiError::bad_request("pcm_f32le samples must be finite"));
            }
            if !(-1.0..=1.0).contains(&sample) {
                return Err(ApiError::bad_request(
                    "pcm_f32le samples must be in the range [-1.0, 1.0]",
                ));
            }
            Ok(sample)
        })
        .collect()
}

fn decode_pcm_s16le(packet: &[u8]) -> Result<Vec<f32>, ApiError> {
    if !packet.len().is_multiple_of(PCM_S16LE_WIRE_BYTES_PER_SAMPLE) {
        return Err(ApiError::bad_request(
            "pcm_s16le packet length must be divisible by 2",
        ));
    }
    Ok(packet
        .chunks_exact(PCM_S16LE_WIRE_BYTES_PER_SAMPLE)
        .map(|bytes| f32::from(i16::from_le_bytes([bytes[0], bytes[1]])) / 32_768.0)
        .collect())
}

fn downmix_to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels == 1 {
        return samples.to_vec();
    }

    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

fn default_sample_rate() -> u32 {
    STREAM_SAMPLE_RATE
}

fn default_channels() -> usize {
    1
}

fn default_protocol_version() -> u8 {
    PROTOCOL_V1
}
