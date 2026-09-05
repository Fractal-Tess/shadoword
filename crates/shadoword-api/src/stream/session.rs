use super::*;

impl StreamSession {
    pub(super) fn new(start: StartMessage, state: &AppState) -> Result<Self, ApiError> {
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

    pub(super) fn accept_packet(&mut self, packet: &[u8]) -> Result<Vec<VadSegment>, ApiError> {
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

    pub(super) fn force_finish_vad(&mut self) -> Option<VadSegment> {
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

    pub(super) fn enqueue_vad_segment(&mut self, segment: VadSegment) -> Result<(), ApiError> {
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

    pub(super) fn pop_pending_vad_segment(&mut self) -> Option<VadSegment> {
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

    pub(super) async fn commit_vad_segment(
        &mut self,
        state: &AppState,
        completion_tx: &mpsc::Sender<JobCompletion>,
        segment: VadSegment,
    ) -> Result<Vec<(usize, TranscriptResponse)>, ApiError> {
        self.submit_audio(state, completion_tx, segment.audio, true)
            .await
    }

    pub(super) async fn commit_segment(
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

    pub(super) fn ensure_commit_capacity(&self, allow_finishing: bool) -> Result<(), ApiError> {
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

    pub(super) async fn submit_audio(
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

    pub(super) fn complete(
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

    pub(super) fn window_len(&self) -> usize {
        self.next_sequence
            .saturating_sub(self.ordered.next_sequence()) as usize
    }

    pub(super) fn acceptance(&self, metadata: Option<SegmentMetadata>) -> SegmentAcceptance {
        let outstanding = self.window_len();
        SegmentAcceptance {
            segment_index: self.next_sequence.saturating_sub(1),
            outstanding,
            remaining_credit: self.credit.saturating_sub(outstanding),
            metadata,
        }
    }

    pub(super) fn cancel_outstanding(&mut self) {
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
