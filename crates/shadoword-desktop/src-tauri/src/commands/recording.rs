use super::*;

#[tauri::command]
#[specta::specta]
pub async fn start_recording(
    app: AppHandle,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<RecordingStatus> {
    start_recording_inner(&app, &state).await
}

#[tauri::command]
#[specta::specta]
pub fn cancel_recording(
    app: AppHandle,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<()> {
    cancel_recording_inner(&app, &state)
}

#[tauri::command]
#[specta::specta]
pub async fn stop_and_transcribe(
    app: AppHandle,
    state: tauri::State<'_, DesktopState>,
) -> CommandResult<TranscriptionResult> {
    finish_recording_inner(&app, &state).await
}

async fn start_recording_inner(
    app: &AppHandle,
    state: &DesktopState,
) -> CommandResult<RecordingStatus> {
    let _mutation = state.mutation.lock().await;
    let config = state.config()?;
    if config.mode == ServiceMode::OpenRouter {
        validate_openrouter_config(&config)?;
    }
    // Keep the monitor and recording state locked together until recording owns the device.
    // The polling command uses the same order, preventing two CPAL input streams from racing.
    let sample_rate = {
        let mut monitor = state
            .microphone_level_monitor
            .lock()
            .map_err(|_| internal_error("microphone level monitor lock poisoned"))?;
        let mut recording = state
            .recording
            .lock()
            .map_err(|_| internal_error("recording state lock poisoned"))?;
        if recording.active.is_some()
            || !matches!(
                recording.state.phase,
                RecordingPhase::Idle | RecordingPhase::Finalizing
            )
        {
            return Err(busy_recording_error());
        }
        drop(monitor.active.take());

        let session = MicrophoneRecorder::start(config.recording.input_device.as_deref()).map_err(
            |error| {
                DesktopError::new("microphone_start_failed", error.to_string())
                    .with_action("Check the selected input device and microphone permissions.")
            },
        )?;
        let started_at = Instant::now();
        let sample_rate = session.snapshot_source().sample_rate();
        let effective_transcription_mode = config.recording.transcription_mode;
        let target = if effective_transcription_mode == TranscriptionMode::Streaming {
            Some(transcription_target(state, &config)?)
        } else {
            None
        };
        recording.state = RecordingState {
            phase: RecordingPhase::Recording,
            service_mode: Some(config.mode),
            transcription_mode: Some(effective_transcription_mode),
            sample_rate: Some(sample_rate),
            segment_count: 0,
        };
        let kind = if let Some(target) = target {
            let source = session.snapshot_source();
            ActiveKind::Streaming(spawn_streaming_worker(
                app.clone(),
                config.clone(),
                target,
                source,
                started_at,
            ))
        } else {
            ActiveKind::Batch
        };
        recording.active = Some(ActiveRecording {
            session,
            started_at,
            sample_rate,
            config,
            kind,
        });
        sample_rate
    };
    crate::tray::set_icon_for_phase(app, RecordingPhase::Recording);
    let _ = app.emit(
        DESKTOP_EVENT_NAME,
        DesktopEvent::RecordingStarted { sample_rate },
    );
    Ok(RecordingStatus {
        recording: true,
        sample_rate,
    })
}

async fn finish_recording_inner(
    app: &AppHandle,
    state: &DesktopState,
) -> CommandResult<TranscriptionResult> {
    let active = {
        let mut recording = state
            .recording
            .lock()
            .map_err(|_| internal_error("recording state lock poisoned"))?;
        if recording.state.phase != RecordingPhase::Recording {
            return Err(DesktopError::new(
                "no_active_recording",
                "no recording is active",
            ));
        }
        let Some(active) = recording.active.take() else {
            recording.state = RecordingState::default();
            crate::tray::set_icon_for_phase(app, RecordingPhase::Idle);
            return Err(internal_error("recording state had no active session"));
        };
        recording.state.phase = RecordingPhase::Finalizing;
        crate::tray::set_icon_for_phase(app, RecordingPhase::Finalizing);
        active
    };
    let _ = app.emit(
        DESKTOP_EVENT_NAME,
        DesktopEvent::RecordingStopped { processing: true },
    );
    let ActiveRecording {
        session,
        started_at,
        sample_rate,
        config,
        kind,
    } = active;
    let result = match kind {
        ActiveKind::Batch => {
            let completion = finish_batch(
                state,
                ActiveRecording {
                    session,
                    started_at,
                    sample_rate,
                    config,
                    kind: ActiveKind::Batch,
                },
            )
            .await;
            reset_completed_finalization(state);
            match completion {
                Ok((result, output)) => {
                    let _ = app.emit(
                        DESKTOP_EVENT_NAME,
                        DesktopEvent::TranscriptionComplete {
                            result: result.clone(),
                            segments: 1,
                        },
                    );
                    spawn_batch_output(app.clone(), output, result.text.clone());
                    Ok(result)
                }
                Err(error) => Err(error),
            }
        }
        ActiveKind::Streaming(worker) => {
            session.stop_without_snapshot();
            let result = if worker.command_tx.send(StreamCommand::Finish).is_err() {
                Err(DesktopError::new(
                    "stream_worker_stopped",
                    "streaming worker stopped before finalization",
                ))
            } else {
                match worker.handle.await {
                    Ok(result) => result,
                    Err(error) => Err(join_error(error)),
                }
            };
            reset_completed_finalization(state);
            result
        }
    };
    if let Err(error) = &result {
        emit_error(
            app,
            "transcription",
            &error.code,
            error.message.clone(),
            error.action.clone(),
        );
    }
    result
}

async fn finish_batch(
    state: &DesktopState,
    active: ActiveRecording,
) -> CommandResult<(TranscriptionResult, OutputConfig)> {
    let duration = active.started_at.elapsed();
    let audio = active
        .session
        .stop()
        .map_err(|error| DesktopError::new("microphone_stop_failed", error.to_string()))?;
    if audio.samples.is_empty() {
        return Err(DesktopError::new(
            "empty_recording",
            "the microphone recording did not contain any samples",
        ));
    }
    let mut cost_usd = None;
    let response = match active.config.mode {
        ServiceMode::Local => {
            #[cfg(feature = "local-runtime")]
            {
                let runtime = Arc::clone(&state.local);
                let job = runtime.submit_batch(audio).map_err(local_error)?;
                tokio::task::spawn_blocking(move || {
                    match job.wait_timeout(Duration::from_secs(5 * 60)) {
                        Ok(Some(completion)) => Ok(completion.response),
                        Ok(None) => {
                            job.cancel();
                            Err(anyhow!("local batch inference timed out"))
                        }
                        Err(error) => Err(anyhow::Error::new(error)),
                    }
                })
                .await
                .map_err(join_error)?
                .map_err(local_error)?
            }
            #[cfg(not(feature = "local-runtime"))]
            return unavailable_local();
        }
        ServiceMode::Remote => {
            let wav = shadoword_core::wav::encode_wav(&audio).map_err(local_error)?;
            state
                .remote
                .transcribe_wav(
                    &active.config.remote.endpoint,
                    active.config.remote.api_token.as_deref(),
                    wav,
                )
                .await
                .map_err(remote_error)?
        }
        ServiceMode::OpenRouter => {
            let wav = shadoword_core::wav::encode_wav(&audio).map_err(local_error)?;
            let api_key = active
                .config
                .openrouter
                .api_key
                .as_deref()
                .ok_or_else(openrouter_key_required)?;
            let openrouter_response = state
                .openrouter
                .transcribe_wav(
                    api_key,
                    &active.config.openrouter.model,
                    wav,
                    active.config.recording.english_only,
                )
                .await
                .map_err(openrouter_error)?;
            cost_usd = openrouter_response.cost_usd;
            shadoword_core::TranscriptResponse {
                text: openrouter_response.text,
                elapsed_ms: openrouter_response.elapsed_ms,
                engine: format!("OpenRouter · {}", active.config.openrouter.model),
            }
        }
    };
    let result = TranscriptionResult {
        text: response.text,
        elapsed_ms: u64::try_from(response.elapsed_ms).unwrap_or(u64::MAX),
        engine: response.engine,
        audio_duration_ms: u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
        sample_rate: active.sample_rate,
        cost_usd,
    };
    Ok((result, active.config.output))
}

fn spawn_batch_output(app: AppHandle, output: OutputConfig, text: String) {
    tauri::async_runtime::spawn(async move {
        match tokio::task::spawn_blocking(move || crate::output::apply_output(&output, &text)).await
        {
            Ok(Ok(())) => {}
            Ok(Err(error)) => emit_error(
                &app,
                "output",
                "output_delivery_failed",
                error.to_string(),
                Some("Check clipboard access and the selected paste method.".to_string()),
            ),
            Err(error) => emit_error(
                &app,
                "output",
                "output_task_failed",
                error.to_string(),
                None,
            ),
        }
    });
}

pub(super) fn cancel_recording_inner(app: &AppHandle, state: &DesktopState) -> CommandResult<()> {
    let active = {
        let mut recording = state
            .recording
            .lock()
            .map_err(|_| internal_error("recording state lock poisoned"))?;
        if recording.state.phase == RecordingPhase::Finalizing {
            return Err(DesktopError::new(
                "transcription_finalizing",
                "the stopped recording is already being finalized",
            )
            .with_action("Wait for the transcription result or error event."));
        }
        if recording.state.phase == RecordingPhase::Idle {
            crate::tray::set_icon_for_phase(app, RecordingPhase::Idle);
            return Ok(());
        }
        recording.state.phase = RecordingPhase::Finalizing;
        crate::tray::set_icon_for_phase(app, RecordingPhase::Finalizing);
        recording.active.take()
    };
    if let Some(active) = active {
        active.session.stop_without_snapshot();
        if let ActiveKind::Streaming(worker) = active.kind {
            worker.cancelled.store(true, Ordering::Release);
            let _ = worker.command_tx.send(StreamCommand::Cancel);
        }
        let _ = app.emit(DESKTOP_EVENT_NAME, DesktopEvent::RecordingCancelled);
    }
    reset_recording_state(state);
    crate::tray::set_icon_for_phase(app, RecordingPhase::Idle);
    Ok(())
}

pub fn stream_worker_failed(app: &AppHandle, error: &DesktopError) {
    let state = app.state::<DesktopState>();
    let active = {
        let Ok(mut recording) = state.recording.lock() else {
            crate::tray::set_icon_for_phase(app, RecordingPhase::Idle);
            return;
        };
        if recording.state.phase != RecordingPhase::Recording {
            return;
        }
        recording.state.phase = RecordingPhase::Finalizing;
        crate::tray::set_icon_for_phase(app, RecordingPhase::Finalizing);
        recording.active.take()
    };
    if let Some(active) = active {
        active.session.stop_without_snapshot();
    }
    reset_recording_state(&state);
    crate::tray::set_icon_for_phase(app, RecordingPhase::Idle);
    emit_error(
        app,
        "streaming",
        &error.code,
        error.message.clone(),
        error.action.clone(),
    );
    let _ = app.emit(
        DESKTOP_EVENT_NAME,
        DesktopEvent::RecordingStopped { processing: false },
    );
}

fn reset_recording_state(state: &DesktopState) {
    if let Ok(mut recording) = state.recording.lock() {
        recording.state = RecordingState::default();
        recording.active = None;
    }
}

fn reset_completed_finalization(state: &DesktopState) {
    if let Ok(mut recording) = state.recording.lock() {
        if recording.state.phase == RecordingPhase::Finalizing && recording.active.is_none() {
            recording.state = RecordingState::default();
        }
    }
}

pub fn increment_stream_segment(app: &AppHandle) {
    let state = app.state::<DesktopState>();
    if let Ok(mut recording) = state.recording.lock() {
        recording.state.segment_count = recording.state.segment_count.saturating_add(1);
    };
}

pub fn handle_hotkey_event(app: &AppHandle, event: HotkeyEventState) {
    let state = app.state::<DesktopState>();
    let action = {
        let config = match state.config() {
            Ok(config) => config,
            Err(_) => return,
        };
        let mut recording = match state.recording.lock() {
            Ok(recording) => recording,
            Err(_) => return,
        };
        match event {
            HotkeyEventState::Released => {
                recording.hotkey_down = false;
                (config.hotkey.mode == HotkeyMode::PushToTalk
                    && recording.state.phase == RecordingPhase::Recording)
                    .then_some(false)
            }
            HotkeyEventState::Pressed if recording.hotkey_down => None,
            HotkeyEventState::Pressed => {
                recording.hotkey_down = true;
                match (config.hotkey.mode, recording.state.phase) {
                    (_, RecordingPhase::Idle | RecordingPhase::Finalizing) => Some(true),
                    (HotkeyMode::Toggle, RecordingPhase::Recording) => Some(false),
                    _ => None,
                }
            }
        }
    };
    match action {
        Some(true) => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app.state::<DesktopState>();
                if let Err(error) = start_recording_inner(&app, &state).await {
                    emit_error(&app, "hotkey", &error.code, error.message, error.action);
                }
            });
        }
        Some(false) => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app.state::<DesktopState>();
                if let Err(error) = finish_recording_inner(&app, &state).await {
                    emit_error(&app, "hotkey", &error.code, error.message, error.action);
                }
            });
        }
        None => {}
    }
}

fn transcription_target(
    state: &DesktopState,
    config: &DesktopConfig,
) -> CommandResult<TranscriptionTarget> {
    match config.mode {
        ServiceMode::Remote => Ok(TranscriptionTarget::Remote {
            endpoint: config.remote.endpoint.clone(),
            token: config.remote.api_token.clone(),
        }),
        ServiceMode::Local => {
            #[cfg(feature = "local-runtime")]
            return Ok(TranscriptionTarget::Local(Arc::clone(&state.local)));
            #[cfg(not(feature = "local-runtime"))]
            {
                let _ = state;
                unavailable_local()
            }
        }
        ServiceMode::OpenRouter => Ok(TranscriptionTarget::OpenRouter(OpenRouterStreamTarget {
            client: state.openrouter.clone(),
            api_key: config
                .openrouter
                .api_key
                .clone()
                .ok_or_else(openrouter_key_required)?,
            model: config.openrouter.model.clone(),
            english_only: config.recording.english_only,
        })),
    }
}
