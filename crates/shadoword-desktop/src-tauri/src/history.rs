use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use shadoword_core::{data_dir, write_json_atomic, ServiceMode};
use specta::Type;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

/// The list is trimmed to this many entries on every write. A transcript history
/// is append-only from the operator's side and nothing pages it, so an uncapped
/// file grows until a single startup read stalls the window.
const HISTORY_LIMIT: usize = 500;

/// Stored raw rather than pre-formatted. The window used to keep history in
/// memory and could therefore afford a bare `14:32` timestamp — everything on
/// screen was from the session you were looking at. Once entries outlive the
/// process that stops being true, so the record carries an absolute instant and
/// unrounded durations, and the frontend formats them at render.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct HistoryEntry {
    pub id: String,
    /// RFC 3339, UTC.
    pub recorded_at: String,
    pub mode: ServiceMode,
    pub engine: String,
    #[specta(type = u32)]
    pub elapsed_ms: u64,
    #[specta(type = u32)]
    pub audio_duration_ms: u64,
    pub text: String,
    pub segments: u32,
    pub cost_usd: Option<f64>,
}

pub struct HistoryStore {
    path: PathBuf,
    entries: Mutex<Vec<HistoryEntry>>,
}

impl HistoryStore {
    /// A history that cannot be read is reported as empty rather than as a
    /// failure: a truncated or hand-edited file should cost the operator their
    /// old transcripts, not their ability to launch the app and record new ones.
    /// The first successful write replaces the unreadable file.
    pub fn load() -> Result<Self> {
        let path = data_dir()?.join("history.json");
        let entries = match fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str::<Vec<HistoryEntry>>(&raw).unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        Ok(Self {
            path,
            entries: Mutex::new(trim(entries)),
        })
    }

    pub fn entries(&self) -> Result<Vec<HistoryEntry>> {
        Ok(self
            .entries
            .lock()
            .map_err(|_| anyhow::anyhow!("history lock poisoned"))?
            .clone())
    }

    /// Whole-list replacement rather than append/delete/clear, because the window
    /// already owns the ordering rules — newest first, dedup by session id, undo
    /// restores at its original index — and splitting those across the boundary
    /// would mean maintaining them twice.
    pub fn replace(&self, entries: Vec<HistoryEntry>) -> Result<Vec<HistoryEntry>> {
        let trimmed = trim(entries);
        write_json_atomic(&self.path, &trimmed, "transcript history")
            .context("failed to write transcript history")?;
        let mut guard = self
            .entries
            .lock()
            .map_err(|_| anyhow::anyhow!("history lock poisoned"))?;
        *guard = trimmed.clone();
        Ok(trimmed)
    }
}

fn trim(mut entries: Vec<HistoryEntry>) -> Vec<HistoryEntry> {
    entries.truncate(HISTORY_LIMIT);
    entries
}
