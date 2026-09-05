use super::*;

pub(super) async fn run_remote_streaming(
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
