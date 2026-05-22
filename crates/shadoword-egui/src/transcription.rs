use shadoword_core::StreamEvent;
use std::sync::mpsc;
use std::time::SystemTime;

use crate::{output, days_to_date, HistoryEntry, ShadowwordApp};

impl ShadowwordApp {
    pub(crate) fn poll_stream_events(&mut self) {
        let Some(rx) = &self.stream_event_rx else {
            return;
        };

        match rx.try_recv() {
            Ok(StreamEvent::Partial(text)) => {
                self.transcript.push_str(&text);
                self.transcript.push(' ');
                self.status_line = "Transcribing...".to_string();
            }
            Ok(StreamEvent::Final(response)) => {
                self.transcript = response.text.clone();
                self.status_line = format!(
                    "Transcribed in {}ms with {}",
                    response.elapsed_ms, response.engine
                );
                let _ = output::apply_output(&self.config, &response.text);

                let timestamp = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| {
                        let secs = d.as_secs();
                        let hours = (secs / 3600) % 24;
                        let mins = (secs / 60) % 60;
                        let days = secs / 86400;
                        let (y, m, d) = days_to_date(days);
                        format!("{y}-{m:02}-{d:02} {hours:02}:{mins:02}")
                    })
                    .unwrap_or_else(|_| "unknown".to_string());

                self.history.insert(
                    0,
                    HistoryEntry {
                        text: response.text,
                        engine: response.engine,
                        elapsed_ms: response.elapsed_ms,
                        timestamp,
                    },
                );
                self.stream_event_rx = None;
            }
            Ok(StreamEvent::Error(error)) => {
                self.status_line = format!("Transcription error: {error}");
                self.stream_event_rx = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.stream_event_rx = None;
            }
        }
    }

    pub(crate) fn poll_background_work(&mut self) {
        let Some(rx) = &self.response_rx else {
            return;
        };

        match rx.try_recv() {
            Ok(Ok(response)) => {
                self.transcript = response.text.clone();
                self.status_line = format!(
                    "Transcribed in {}ms with {}",
                    response.elapsed_ms, response.engine
                );
                let _ = output::apply_output(&self.config, &response.text);

                let timestamp = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| {
                        let secs = d.as_secs();
                        let hours = (secs / 3600) % 24;
                        let mins = (secs / 60) % 60;
                        let days = secs / 86400;
                        let (y, m, d) = days_to_date(days);
                        format!("{y}-{m:02}-{d:02} {hours:02}:{mins:02}")
                    })
                    .unwrap_or_else(|_| "unknown".to_string());

                self.history.insert(
                    0,
                    HistoryEntry {
                        text: response.text,
                        engine: response.engine,
                        elapsed_ms: response.elapsed_ms,
                        timestamp,
                    },
                );

                self.response_rx = None;
            }
            Ok(Err(error)) => {
                self.status_line = format!("Transcription failed: {error}");
                self.response_rx = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.status_line = "Background worker disconnected".to_string();
                self.response_rx = None;
            }
        }
    }
}
