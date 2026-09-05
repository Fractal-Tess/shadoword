use super::*;

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
pub(super) async fn run_local_streaming(
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
