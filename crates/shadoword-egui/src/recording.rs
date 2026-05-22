use shadoword_core::{MicrophoneRecorder, StreamEvent};
use std::sync::mpsc;

use crate::ShadowwordApp;

impl ShadowwordApp {
    pub(crate) fn start_recording(&mut self) {
        // Don't start while a stream transcription session is still active
        if self.stream_event_rx.is_some() {
            return;
        }

        let (chunk_tx, chunk_rx) = mpsc::channel::<Vec<f32>>();
        let (event_tx, event_rx) = mpsc::channel::<StreamEvent>();
        let chunk_samples = 480;
        let sample_rate = self.config.recording.sample_rate;

        let session = match MicrophoneRecorder::start_streaming(
            self.config.recording.input_device.as_deref(),
            chunk_tx,
            chunk_samples,
            sample_rate,
        ) {
            Ok(session) => session,
            Err(error) => {
                self.status_line = format!("Failed to start recording: {error}");
                return;
            }
        };

        if let Err(error) = self
            .local_service
            .start_stream_transcription(chunk_rx, event_tx, sample_rate)
        {
            self.status_line = format!("Failed to start stream transcription: {error}");
            drop(session);
            return;
        }

        self.status_line = "Recording from local microphone...".to_string();
        self.active_recording = Some(session);
        self.stream_event_rx = Some(event_rx);
    }

    pub(crate) fn stop_recording(&mut self) {
        let Some(session) = self.active_recording.take() else {
            return;
        };

        match session.stop() {
            Ok(_audio) => {
                self.status_line = "Finalizing transcription...".to_string();
            }
            Err(error) => {
                self.status_line = format!("Failed to finish recording: {error}");
            }
        }
    }
}
