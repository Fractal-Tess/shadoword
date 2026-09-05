use super::*;

struct OpenRouterStreamState {
    segmenter: VadSegmenter,
    backlog: VecDeque<VadSegment>,
    samples_since_segment: usize,
    next_sequence: u64,
    finishing: bool,
    max_backlog: usize,
}

impl OpenRouterStreamState {
    fn new(sample_rate: u32) -> Self {
        Self {
            segmenter: VadSegmenter::new(sample_rate, VadSegmenterConfig::default()),
            backlog: VecDeque::new(),
            samples_since_segment: 0,
            next_sequence: 0,
            finishing: false,
            max_backlog: usize::try_from(MAX_STREAM_SEGMENTS)
                .expect("stream segment limit must fit usize"),
        }
    }

    fn capture(&mut self, source: &RecordingSnapshotSource) -> Result<(), DesktopError> {
        capture_segments(
            source,
            &mut self.segmenter,
            &mut self.samples_since_segment,
            &mut self.backlog,
            self.max_backlog,
        )
    }

    fn finish(&mut self, source: &RecordingSnapshotSource) -> Result<(), DesktopError> {
        enqueue_final_segments(
            source,
            &mut self.segmenter,
            &mut self.backlog,
            self.max_backlog,
        )
    }
}

pub(super) async fn run_openrouter_streaming(
    app: AppHandle,
    config: DesktopConfig,
    target: OpenRouterStreamTarget,
    source: RecordingSnapshotSource,
    started_at: Instant,
    mut command_rx: mpsc::UnboundedReceiver<StreamCommand>,
    cancelled: Arc<AtomicBool>,
) -> Result<TranscriptionResult, DesktopError> {
    let mut stream = OpenRouterStreamState::new(source.sample_rate());
    let mut results = Vec::new();
    let mut interval = tokio::time::interval(STREAM_POLL_INTERVAL);

    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err(cancelled_error());
        }

        if let Some(segment) = stream.backlog.pop_front() {
            if stream.next_sequence >= MAX_STREAM_SEGMENTS {
                return Err(stream_error(
                    "openrouter_stream",
                    "stream contains too many segments",
                ));
            }
            let sequence = stream.next_sequence;
            stream.next_sequence += 1;
            let audio_duration_ms = segment_duration_ms(&segment);
            let sample_rate = segment.audio.sample_rate;
            // Keep OpenRouter uploads and output delivery in capture order.
            let response = transcribe_openrouter_segment(
                &target,
                segment,
                &source,
                &mut stream,
                &mut interval,
                &mut command_rx,
                &cancelled,
            )
            .await?;
            if cancelled.load(Ordering::Acquire) {
                return Err(cancelled_error());
            }
            let result = TranscriptionResult {
                text: response.text,
                elapsed_ms: u64::try_from(response.elapsed_ms).unwrap_or(u64::MAX),
                engine: format!("OpenRouter · {}", target.model),
                audio_duration_ms,
                sample_rate,
                cost_usd: response.cost_usd,
            };
            deliver_ready_segments(
                &app,
                &config,
                &mut results,
                vec![(sequence as usize, result)],
                &cancelled,
            )
            .await;
            if cancelled.load(Ordering::Acquire) {
                return Err(cancelled_error());
            }
            continue;
        }

        if stream.finishing {
            break;
        }

        tokio::select! {
            biased;
            command = command_rx.recv() => match command {
                Some(StreamCommand::Finish) => {
                    stream.finish(&source)?;
                    stream.finishing = true;
                }
                Some(StreamCommand::Cancel) | None => {
                    cancelled.store(true, Ordering::Release);
                    return Err(cancelled_error());
                }
            },
            _ = interval.tick() => stream.capture(&source)?,
        }
    }

    finish_stream(&app, &config, &source, started_at, results, None).await
}

async fn transcribe_openrouter_segment(
    target: &OpenRouterStreamTarget,
    segment: VadSegment,
    source: &RecordingSnapshotSource,
    stream: &mut OpenRouterStreamState,
    interval: &mut tokio::time::Interval,
    command_rx: &mut mpsc::UnboundedReceiver<StreamCommand>,
    cancelled: &AtomicBool,
) -> Result<OpenRouterTranscription, DesktopError> {
    let wav = shadoword_core::wav::encode_wav(&segment.audio)
        .map_err(|error| stream_error("openrouter_stream", error))?;
    let request =
        target
            .client
            .transcribe_wav(&target.api_key, &target.model, wav, target.english_only);
    tokio::pin!(request);

    loop {
        tokio::select! {
            biased;
            command = command_rx.recv(), if !stream.finishing => match command {
                Some(StreamCommand::Finish) => {
                    stream.finish(source)?;
                    stream.finishing = true;
                }
                Some(StreamCommand::Cancel) | None => {
                    cancelled.store(true, Ordering::Release);
                    return Err(cancelled_error());
                }
            },
            response = &mut request => {
                return response.map_err(crate::commands::support::openrouter_error);
            }
            _ = interval.tick(), if !stream.finishing => stream.capture(source)?,
        }
    }
}
