use crate::contracts::{DesktopError, DesktopEvent, TranscriptionResult, DESKTOP_EVENT_NAME};
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
        };
        if let Err(error) = &result {
            if error.code != "recording_cancelled" {
                crate::commands::stream_worker_failed(&failure_app, error);
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

#[cfg(feature = "local-runtime")]
struct LocalJobCompletion {
    sequence: u64,
    audio_duration_ms: u64,
    sample_rate: u32,
    result: Result<InferenceCompletion, LocalJobError>,
}

#[cfg(feature = "local-runtime")]
enum LocalJobError {
    Inference(InferenceError),
    Timeout,
}

#[cfg(feature = "local-runtime")]
impl std::fmt::Display for LocalJobError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inference(error) => error.fmt(formatter),
            Self::Timeout => formatter.write_str("local streaming inference timed out"),
        }
    }
}

#[cfg(feature = "local-runtime")]
async fn run_local_streaming(
    app: AppHandle,
    config: DesktopConfig,
    runtime: Arc<InferenceRuntime>,
    source: RecordingSnapshotSource,
    started_at: Instant,
    mut command_rx: mpsc::UnboundedReceiver<StreamCommand>,
    cancelled: Arc<AtomicBool>,
) -> Result<TranscriptionResult, DesktopError> {
    let limits = runtime
        .transcription_config()
        .effective_inference_pool()
        .map_err(|error| stream_error("local_stream", error))?
        .limits;
    let credit = limits
        .max_outstanding_per_flow
        .min(
            limits
                .max_sequencer_buffered_results_per_flow
                .saturating_add(1),
        )
        .max(1);
    let flow_number = NEXT_LOCAL_FLOW_ID.fetch_add(1, Ordering::Relaxed);
    let flow_id = format!("desktop-{}-{flow_number}", std::process::id());
    let (completion_tx, mut completion_rx) = mpsc::channel(credit);
    let mut outstanding = LocalOutstandingJobs::default();
    let mut backlog = VecDeque::new();
    let mut barrier = OrderedSegmentBarrier::new(limits.max_sequencer_buffered_results_per_flow);
    let mut results = Vec::new();
    let mut segmenter = VadSegmenter::new(source.sample_rate(), VadSegmenterConfig::default());
    let mut interval = tokio::time::interval(STREAM_POLL_INTERVAL);
    let mut samples_since_segment = 0_usize;
    let mut next_sequence = 0_u64;
    let mut finishing = false;

    loop {
        while barrier.uncommitted_len(outstanding.len()) < credit {
            let Some(segment) = backlog.pop_front() else {
                break;
            };
            if next_sequence >= MAX_STREAM_SEGMENTS {
                cancel_local_jobs(&outstanding);
                return Err(stream_error(
                    "local_stream",
                    "stream contains too many segments",
                ));
            }
            let sequence = next_sequence;
            next_sequence += 1;
            let audio_duration_ms = segment_duration_ms(&segment);
            let sample_rate = segment.audio.sample_rate;
            let job = match runtime.submit_stream(flow_id.clone(), sequence, segment.audio) {
                Ok(job) => Arc::new(job),
                Err(error) => {
                    let _ = barrier.fail(sequence, error.to_string());
                    cancel_local_jobs(&outstanding);
                    return Err(stream_error("local_stream", error));
                }
            };
            outstanding.insert(sequence, Arc::clone(&job));
            spawn_local_completion(
                sequence,
                audio_duration_ms,
                sample_rate,
                job,
                completion_tx.clone(),
            );
        }

        if finishing && backlog.is_empty() && outstanding.is_empty() {
            break;
        }

        tokio::select! {
            biased;
            command = command_rx.recv(), if !finishing => match command {
                Some(StreamCommand::Finish) => {
                    enqueue_final_segments(
                        &source,
                        &mut segmenter,
                        &mut backlog,
                        credit,
                    )?;
                    finishing = true;
                }
                Some(StreamCommand::Cancel) | None => {
                    cancelled.store(true, Ordering::Release);
                    cancel_local_jobs(&outstanding);
                    return Err(cancelled_error());
                }
            },
            completion = completion_rx.recv(), if !outstanding.is_empty() => {
                let completion = completion.ok_or_else(|| {
                    stream_error("local_stream", "local completion channel closed")
                })?;
                if outstanding.remove(&completion.sequence).is_none() {
                    continue;
                }
                match completion.result {
                    Ok(completed) => {
                        let response = completed.response;
                        let result = TranscriptionResult {
                            text: response.text,
                            elapsed_ms: u64::try_from(response.elapsed_ms).unwrap_or(u64::MAX),
                            engine: response.engine,
                            audio_duration_ms: completion.audio_duration_ms,
                            sample_rate: completion.sample_rate,
                            cost_usd: None,
                        };
                        let ready = barrier
                            .complete(completion.sequence, result)
                            .map_err(|error| stream_error("local_stream", error))?;
                        deliver_ready_segments(
                            &app,
                            &config,
                            &mut results,
                            ready,
                            &cancelled,
                        )
                        .await;
                    }
                    Err(error) => {
                        if barrier.fail(completion.sequence, error.to_string()).is_ok() {
                            cancel_local_jobs(&outstanding);
                            return Err(stream_error("local_stream", error));
                        }
                    }
                }
            },
            _ = interval.tick(), if !finishing => {
                capture_segments(
                    &source,
                    &mut segmenter,
                    &mut samples_since_segment,
                    &mut backlog,
                    credit,
                )?;
            }
        }
    }

    finish_stream(&app, &config, &source, started_at, results, None).await
}

#[cfg(feature = "local-runtime")]
fn spawn_local_completion(
    sequence: u64,
    audio_duration_ms: u64,
    sample_rate: u32,
    job: Arc<InferenceJob>,
    completion_tx: mpsc::Sender<LocalJobCompletion>,
) {
    tokio::spawn(async move {
        let waited_job = Arc::clone(&job);
        let result = tokio::task::spawn_blocking(move || {
            match waited_job.wait_timeout(STREAM_INFERENCE_TIMEOUT) {
                Ok(Some(completion)) => Ok(completion),
                Ok(None) => {
                    job.cancel();
                    Err(LocalJobError::Timeout)
                }
                Err(error) => Err(LocalJobError::Inference(error)),
            }
        })
        .await
        .unwrap_or_else(|error| {
            Err(LocalJobError::Inference(InferenceError::WorkerFailed(
                error.to_string(),
            )))
        });
        let _ = completion_tx
            .send(LocalJobCompletion {
                sequence,
                audio_duration_ms,
                sample_rate,
                result,
            })
            .await;
    });
}

#[cfg(feature = "local-runtime")]
#[derive(Default)]
struct LocalOutstandingJobs(HashMap<u64, Arc<InferenceJob>>);

#[cfg(feature = "local-runtime")]
impl std::ops::Deref for LocalOutstandingJobs {
    type Target = HashMap<u64, Arc<InferenceJob>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "local-runtime")]
impl std::ops::DerefMut for LocalOutstandingJobs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "local-runtime")]
impl Drop for LocalOutstandingJobs {
    fn drop(&mut self) {
        cancel_local_jobs(self);
    }
}

#[cfg(feature = "local-runtime")]
fn cancel_local_jobs(outstanding: &LocalOutstandingJobs) {
    for job in outstanding.0.values() {
        job.cancel();
    }
}

async fn run_remote_streaming(
    app: AppHandle,
    config: DesktopConfig,
    connection: RemoteConnection,
    source: RecordingSnapshotSource,
    started_at: Instant,
    mut command_rx: mpsc::UnboundedReceiver<StreamCommand>,
    cancelled: Arc<AtomicBool>,
) -> Result<TranscriptionResult, DesktopError> {
    let connecting = RemoteStream::connect_with_pcm_format(
        &connection.endpoint,
        connection.token.as_deref(),
        source.sample_rate(),
        config.recording.streaming_pcm_format,
    );
    tokio::pin!(connecting);
    let mut finishing = false;
    let mut remote = loop {
        tokio::select! {
            biased;
            command = command_rx.recv(), if !finishing => match command {
                Some(StreamCommand::Finish) => finishing = true,
                Some(StreamCommand::Cancel) | None => {
                    cancelled.store(true, Ordering::Release);
                    return Err(cancelled_error());
                }
            },
            connected = &mut connecting => {
                break connected.map_err(|error| stream_error("stream_connect", error))?;
            }
        }
    };
    let credit = remote.credit().max(1);
    let protocol = remote.protocol();
    let _flow_id = remote.flow_id();
    if protocol == RemoteProtocol::V3 {
        return run_remote_pcm_streaming(
            app,
            config,
            source,
            started_at,
            command_rx,
            cancelled,
            NegotiatedPcmStream { remote, finishing },
        )
        .await;
    }
    let mut outstanding = HashMap::<u64, RemotePending>::new();
    let mut backlog = VecDeque::new();
    let mut results = Vec::new();
    let mut segmenter = VadSegmenter::new(source.sample_rate(), VadSegmenterConfig::default());
    let mut interval = tokio::time::interval(STREAM_POLL_INTERVAL);
    let mut samples_since_segment = 0_usize;
    let mut next_sequence = 0_u64;
    let mut last_keepalive = tokio::time::Instant::now();
    let mut finish_sent = false;

    if finishing {
        enqueue_final_segments(&source, &mut segmenter, &mut backlog, credit)?;
    }

    loop {
        while outstanding.len() < credit {
            let Some(segment) = backlog.pop_front() else {
                break;
            };
            if next_sequence >= MAX_STREAM_SEGMENTS {
                remote.close().await;
                return Err(stream_error(
                    "remote_stream",
                    "stream contains too many segments",
                ));
            }
            let sequence = next_sequence;
            next_sequence += 1;
            let pending = RemotePending {
                audio_duration_ms: segment_duration_ms(&segment),
                sample_rate: segment.audio.sample_rate,
                accepted: protocol == RemoteProtocol::V1,
            };
            if let Err(error) = remote.send_segment(sequence, segment.audio).await {
                remote.close().await;
                return Err(stream_error("remote_stream", error));
            }
            outstanding.insert(sequence, pending);
        }

        if finishing && backlog.is_empty() && !finish_sent {
            remote
                .finish_request()
                .await
                .map_err(|error| stream_error("stream_finish", error))?;
            finish_sent = true;
        }

        tokio::select! {
            biased;
            command = command_rx.recv(), if !finishing => match command {
                Some(StreamCommand::Finish) => {
                    enqueue_final_segments(
                        &source,
                        &mut segmenter,
                        &mut backlog,
                        credit,
                    )?;
                    finishing = true;
                }
                Some(StreamCommand::Cancel) | None => {
                    cancelled.store(true, Ordering::Release);
                    remote.close().await;
                    return Err(cancelled_error());
                }
            },
            event = remote.next_event(), if !outstanding.is_empty() || finish_sent => {
                let event = match event {
                    Ok(event) => event,
                    Err(error) => {
                        remote.close().await;
                        return Err(stream_error("remote_stream", error));
                    }
                };
                match event {
                    RemoteEvent::Accepted {
                        segment_index,
                        outstanding: server_outstanding,
                        remaining_credit,
                        ..
                    } => {
                        if protocol != RemoteProtocol::V2
                            || server_outstanding > credit
                            || remaining_credit > credit
                        {
                            remote.close().await;
                            return Err(stream_error(
                                "remote_stream",
                                "remote server returned invalid stream credit accounting",
                            ));
                        }
                        let pending = outstanding.get_mut(&segment_index).ok_or_else(|| {
                            stream_error(
                                "remote_stream",
                                format!("remote server accepted unknown segment {segment_index}"),
                            )
                        })?;
                        if pending.accepted {
                            remote.close().await;
                            return Err(stream_error(
                                "remote_stream",
                                format!("remote server accepted segment {segment_index} twice"),
                            ));
                        }
                        pending.accepted = true;
                    }
                    RemoteEvent::Partial(partial) => {
                        let expected = u64::try_from(results.len()).unwrap_or(u64::MAX);
                        if partial.segment_index != expected {
                            remote.close().await;
                            return Err(stream_error(
                                "remote_stream",
                                format!(
                                    "expected ordered partial {expected}, received {}",
                                    partial.segment_index
                                ),
                            ));
                        }
                        let pending = outstanding.remove(&partial.segment_index).ok_or_else(|| {
                            stream_error(
                                "remote_stream",
                                format!("remote server completed unknown segment {expected}"),
                            )
                        })?;
                        if !pending.accepted {
                            remote.close().await;
                            return Err(stream_error(
                                "remote_stream",
                                format!("remote server completed unaccepted segment {expected}"),
                            ));
                        }
                        let result = TranscriptionResult {
                            text: partial.text,
                            elapsed_ms: partial.elapsed_ms,
                            engine: partial.engine,
                            audio_duration_ms: pending.audio_duration_ms,
                            sample_rate: pending.sample_rate,
                            cost_usd: None,
                        };
                        deliver_ready_segments(
                            &app,
                            &config,
                            &mut results,
                            vec![(expected as usize, result)],
                            &cancelled,
                        )
                        .await;
                    }
                    RemoteEvent::Done(done) => {
                        if !finish_sent || !outstanding.is_empty() {
                            remote.close().await;
                            return Err(stream_error(
                                "remote_stream",
                                "remote stream finished before all ordered partials arrived",
                            ));
                        }
                        let result = finish_stream(
                            &app,
                            &config,
                            &source,
                            started_at,
                            results,
                            Some(done.text),
                        )
                        .await;
                        remote.close().await;
                        return result;
                    }
                }
            },
            _ = interval.tick(), if !finishing => {
                capture_segments(
                    &source,
                    &mut segmenter,
                    &mut samples_since_segment,
                    &mut backlog,
                    credit,
                )?;
                if last_keepalive.elapsed() >= STREAM_KEEPALIVE_INTERVAL {
                    remote
                        .keep_alive()
                        .await
                        .map_err(|error| stream_error("remote_stream", error))?;
                    last_keepalive = tokio::time::Instant::now();
                }
            }
        }
    }
}

struct NegotiatedPcmStream {
    remote: RemoteStream,
    finishing: bool,
}

async fn run_remote_pcm_streaming(
    app: AppHandle,
    config: DesktopConfig,
    source: RecordingSnapshotSource,
    started_at: Instant,
    mut command_rx: mpsc::UnboundedReceiver<StreamCommand>,
    cancelled: Arc<AtomicBool>,
    stream: NegotiatedPcmStream,
) -> Result<TranscriptionResult, DesktopError> {
    let NegotiatedPcmStream {
        mut remote,
        mut finishing,
    } = stream;
    let credit = remote.credit().max(1);
    let mut outstanding = HashMap::<u64, RemotePending>::new();
    let mut results = Vec::new();
    let mut next_accepted = 0_u64;
    let mut keepalive = tokio::time::interval(STREAM_KEEPALIVE_INTERVAL);
    keepalive.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    keepalive.tick().await;
    let mut finish_sent = false;
    send_available_pcm(&source, &mut remote).await?;

    loop {
        if finishing && !finish_sent {
            send_available_pcm(&source, &mut remote).await?;
            remote
                .finish_request()
                .await
                .map_err(|error| stream_error("stream_finish", error))?;
            finish_sent = true;
        }

        tokio::select! {
            biased;
            command = command_rx.recv(), if !finishing => match command {
                Some(StreamCommand::Finish) => {
                    send_available_pcm(&source, &mut remote).await?;
                    finishing = true;
                }
                Some(StreamCommand::Cancel) | None => {
                    cancelled.store(true, Ordering::Release);
                    remote.close().await;
                    return Err(cancelled_error());
                }
            },
            event = remote.next_event() => {
                let event = match event {
                    Ok(event) => event,
                    Err(error) => {
                        remote.close().await;
                        return Err(stream_error("remote_stream", error));
                    }
                };
                match event {
                    RemoteEvent::Accepted {
                        segment_index,
                        outstanding: server_outstanding,
                        remaining_credit,
                        audio_duration_ms,
                        sample_rate,
                    } => {
                        let expected_outstanding = outstanding.len().saturating_add(1);
                        if segment_index != next_accepted
                            || segment_index >= MAX_STREAM_SEGMENTS
                            || outstanding.len() >= credit
                            || server_outstanding != expected_outstanding
                            || remaining_credit != credit.saturating_sub(server_outstanding)
                        {
                            remote.close().await;
                            return Err(stream_error(
                                "remote_stream",
                                format!(
                                    "remote server returned invalid PCM segment acceptance {segment_index}"
                                ),
                            ));
                        }
                        let pending = RemotePending {
                            audio_duration_ms: audio_duration_ms.ok_or_else(|| {
                                stream_error(
                                    "remote_stream",
                                    "remote PCM acceptance omitted audio_duration_ms",
                                )
                            })?,
                            sample_rate: sample_rate.ok_or_else(|| {
                                stream_error(
                                    "remote_stream",
                                    "remote PCM acceptance omitted sample_rate",
                                )
                            })?,
                            accepted: true,
                        };
                        if outstanding.insert(segment_index, pending).is_some() {
                            remote.close().await;
                            return Err(stream_error(
                                "remote_stream",
                                format!("remote server accepted segment {segment_index} twice"),
                            ));
                        }
                        next_accepted += 1;
                    }
                    RemoteEvent::Partial(partial) => {
                        let expected = u64::try_from(results.len()).unwrap_or(u64::MAX);
                        if partial.segment_index != expected {
                            remote.close().await;
                            return Err(stream_error(
                                "remote_stream",
                                format!(
                                    "expected ordered partial {expected}, received {}",
                                    partial.segment_index
                                ),
                            ));
                        }
                        let pending = outstanding.remove(&partial.segment_index).ok_or_else(|| {
                            stream_error(
                                "remote_stream",
                                format!("remote server completed unknown segment {expected}"),
                            )
                        })?;
                        let result = TranscriptionResult {
                            text: partial.text,
                            elapsed_ms: partial.elapsed_ms,
                            engine: partial.engine,
                            audio_duration_ms: pending.audio_duration_ms,
                            sample_rate: pending.sample_rate,
                            cost_usd: None,
                        };
                        deliver_ready_segments(
                            &app,
                            &config,
                            &mut results,
                            vec![(expected as usize, result)],
                            &cancelled,
                        )
                        .await;
                    }
                    RemoteEvent::Done(done) => {
                        if !finish_sent || !outstanding.is_empty() {
                            remote.close().await;
                            return Err(stream_error(
                                "remote_stream",
                                "remote stream finished before all ordered partials arrived",
                            ));
                        }
                        let result = finish_stream(
                            &app,
                            &config,
                            &source,
                            started_at,
                            results,
                            Some(done.text),
                        )
                        .await;
                        remote.close().await;
                        return result;
                    }
                }
            },
            _ = source.wait_for_samples(), if !finishing => {
                send_available_pcm(&source, &mut remote).await?;
            }
            _ = keepalive.tick(), if !finishing => {
                remote
                    .keep_alive()
                    .await
                    .map_err(|error| stream_error("stream_keepalive", error))?;
            }
        }
    }
}

async fn send_available_pcm(
    source: &RecordingSnapshotSource,
    remote: &mut RemoteStream,
) -> Result<(), DesktopError> {
    let audio = source
        .drain_available()
        .map_err(|error| stream_error("audio_capture", error))?;
    if !audio.samples.is_empty() {
        remote
            .send_samples(&audio.samples)
            .await
            .map_err(|error| stream_error("remote_stream", error))?;
    }
    Ok(())
}

struct RemotePending {
    audio_duration_ms: u64,
    sample_rate: u32,
    accepted: bool,
}

struct RemoteConnection {
    endpoint: String,
    token: Option<String>,
}

#[cfg(feature = "local-runtime")]
struct OrderedSegmentBarrier {
    ordered: OrderedCompletion<TranscriptionResult>,
}

#[cfg(feature = "local-runtime")]
impl OrderedSegmentBarrier {
    fn new(max_buffered: usize) -> Self {
        Self {
            ordered: OrderedCompletion::new(0, max_buffered.max(1)),
        }
    }

    fn complete(
        &mut self,
        sequence: u64,
        result: TranscriptionResult,
    ) -> Result<Vec<(usize, TranscriptionResult)>, shadoword_core::SequencerError> {
        let first = self.ordered.next_sequence();
        self.ordered.complete(sequence, result).map(|ready| {
            ready
                .into_iter()
                .enumerate()
                .map(|(offset, result)| (first as usize + offset, result))
                .collect()
        })
    }

    fn uncommitted_len(&self, outstanding: usize) -> usize {
        outstanding.saturating_add(self.ordered.buffered_len())
    }

    fn fail(
        &mut self,
        sequence: u64,
        message: impl Into<String>,
    ) -> Result<(), shadoword_core::SequencerError> {
        self.ordered.fail(sequence, message)
    }
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
                "streaming audio backlog exceeded the negotiated flow credit",
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
        crate::commands::increment_stream_segment(app);
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
    let result = TranscriptionResult {
        text,
        elapsed_ms,
        engine,
        audio_duration_ms: millis(started_at.elapsed()),
        sample_rate: source.sample_rate(),
        cost_usd: None,
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
