use crate::config::{
    models_dir, ExecutionTarget, ExecutionUnitConfig, InferenceLimits, TranscriptionConfig,
    DEFAULT_GPU_HOST_THREADS,
};
use crate::contracts::{
    compiled_whisper_backends, DrainingGenerationStatus, ExecutionUnitState, ExecutionUnitStatus,
    InferencePoolStatus, ServiceStatus, TranscriptResponse, TranscriptionService,
};
use crate::model_download::default_whisper_model;
use anyhow::{anyhow, Context, Result};
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};
use serde::{Deserialize, Serialize};
use shadoword_model_whisper::{list_whisper_gpu_devices, WhisperModel};
use shadoword_shared::{
    AudioInput, Model, ModelAffinity, ModelConfig, ModelError, SharedResult, TranscriptionOptions,
};
use specta::Type;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender};
use std::sync::{Arc, Condvar, Mutex, OnceLock, RwLock, Weak};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
mod pool;
mod runtime;
mod worker;

use worker::{decrement_flow, spawn_worker};

pub trait ExecutionModelFactory: Send + Sync + 'static {
    fn create(&self, unit: &ExecutionUnitConfig) -> SharedResult<Box<dyn Model>>;

    fn load_scope(&self) -> Option<usize> {
        None
    }
}

#[derive(Debug, Default)]
pub struct WhisperModelFactory;

impl ExecutionModelFactory for WhisperModelFactory {
    fn create(&self, _unit: &ExecutionUnitConfig) -> SharedResult<Box<dyn Model>> {
        Ok(Box::new(WhisperModel::new()))
    }

    fn load_scope(&self) -> Option<usize> {
        Some(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InferenceIdentity {
    Batch,
    Stream {
        flow_id: String,
        #[specta(type = f64)]
        sequence: u64,
    },
}

#[derive(Debug, Clone)]
pub struct InferenceRequest {
    pub input: AudioInput,
    pub identity: InferenceIdentity,
    pub compatible_unit_ids: Option<Vec<String>>,
}

impl InferenceRequest {
    pub fn batch(input: AudioInput) -> Self {
        Self {
            input,
            identity: InferenceIdentity::Batch,
            compatible_unit_ids: None,
        }
    }

    pub fn stream(flow_id: impl Into<String>, sequence: u64, input: AudioInput) -> Self {
        Self {
            input,
            identity: InferenceIdentity::Stream {
                flow_id: flow_id.into(),
                sequence,
            },
            compatible_unit_ids: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct InferenceCompletion {
    #[specta(type = f64)]
    pub generation: u64,
    pub unit_id: String,
    pub identity: InferenceIdentity,
    pub response: TranscriptResponse,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferenceError {
    AdmissionClosed,
    QueueFull,
    AudioQueueFull,
    AudioTooLarge,
    InvalidSampleRate(u32),
    FlowLimit,
    NoCompatibleUnit,
    Cancelled,
    WorkerFailed(String),
    ResponseDisconnected,
}

impl fmt::Display for InferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AdmissionClosed => formatter.write_str("inference pool is draining"),
            Self::QueueFull => formatter.write_str("inference job queue is full"),
            Self::AudioQueueFull => formatter.write_str("inference audio queue is full"),
            Self::AudioTooLarge => {
                formatter.write_str("inference job audio exceeds the per-job limit")
            }
            Self::InvalidSampleRate(sample_rate) => write!(
                formatter,
                "audio sample rate {sample_rate} Hz is outside the supported range"
            ),
            Self::FlowLimit => formatter.write_str("inference flow has too many outstanding jobs"),
            Self::NoCompatibleUnit => {
                formatter.write_str("no healthy compatible execution unit is available")
            }
            Self::Cancelled => formatter.write_str("inference job was cancelled"),
            Self::WorkerFailed(message) => write!(formatter, "inference worker failed: {message}"),
            Self::ResponseDisconnected => {
                formatter.write_str("inference response channel disconnected")
            }
        }
    }
}

impl std::error::Error for InferenceError {}

pub struct InferenceJob {
    id: u64,
    lifecycle: Arc<Mutex<JobLifecycle>>,
    state: Weak<SharedState>,
    receiver: Mutex<Receiver<std::result::Result<InferenceCompletion, InferenceError>>>,
}

impl InferenceJob {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn cancel(&self) -> bool {
        let Some(state) = self.state.upgrade() else {
            return false;
        };
        state.cancel(self.id, &self.lifecycle)
    }

    pub fn wait(&self) -> std::result::Result<InferenceCompletion, InferenceError> {
        self.receiver
            .lock()
            .expect("inference response lock poisoned")
            .recv()
            .map_err(|_| InferenceError::ResponseDisconnected)?
    }

    pub fn wait_timeout(
        &self,
        timeout: Duration,
    ) -> std::result::Result<Option<InferenceCompletion>, InferenceError> {
        match self
            .receiver
            .lock()
            .expect("inference response lock poisoned")
            .recv_timeout(timeout)
        {
            Ok(result) => result.map(Some),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err(InferenceError::ResponseDisconnected),
        }
    }
}

#[derive(Clone)]
struct GenerationSnapshot {
    generation: u64,
    config: TranscriptionConfig,
}

struct QueuedJob {
    id: u64,
    flow_key: String,
    identity: InferenceIdentity,
    input: AudioInput,
    audio_bytes: usize,
    compatible_units: Option<HashSet<String>>,
    snapshot: Arc<GenerationSnapshot>,
    lifecycle: Arc<Mutex<JobLifecycle>>,
    sender: SyncSender<std::result::Result<InferenceCompletion, InferenceError>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JobLifecycle {
    Queued,
    Running,
    Cancelled,
    Completed,
}

impl QueuedJob {
    fn compatible_with(&self, unit_id: &str) -> bool {
        self.compatible_units
            .as_ref()
            .is_none_or(|units| units.contains(unit_id))
    }
}

struct UnitAccounting {
    config: ExecutionUnitConfig,
    state: ExecutionUnitState,
    load_timed_out: bool,
    worker_exited: bool,
    last_error: Option<String>,
    completed: u64,
    failed: u64,
}

struct PoolAccounting {
    generation: u64,
    accepting: bool,
    limits: InferenceLimits,
    queues: HashMap<String, VecDeque<QueuedJob>>,
    ready_flows: VecDeque<String>,
    flow_outstanding: HashMap<String, usize>,
    queued_jobs: usize,
    queued_audio_bytes: usize,
    running_jobs: usize,
    running_audio_bytes: usize,
    next_job_id: u64,
    units: HashMap<String, UnitAccounting>,
    completed: u64,
    failed: u64,
    last_error: Option<String>,
}

struct SharedState {
    accounting: Mutex<PoolAccounting>,
    wake_workers: Condvar,
}

impl SharedState {
    fn cancel(&self, id: u64, lifecycle: &Mutex<JobLifecycle>) -> bool {
        let (won, removed) = {
            let mut state = self
                .accounting
                .lock()
                .expect("pool accounting lock poisoned");
            let mut lifecycle = lifecycle.lock().expect("job lifecycle lock poisoned");
            match *lifecycle {
                JobLifecycle::Queued => {
                    let mut removed = None;
                    let flow_keys = state.queues.keys().cloned().collect::<Vec<_>>();
                    for flow_key in flow_keys {
                        let position = state
                            .queues
                            .get(&flow_key)
                            .and_then(|queue| queue.iter().position(|job| job.id == id));
                        let Some(position) = position else {
                            continue;
                        };
                        let job = state
                            .queues
                            .get_mut(&flow_key)
                            .and_then(|queue| queue.remove(position))
                            .expect("queued job position must exist");
                        let queue_empty =
                            state.queues.get(&flow_key).is_none_or(VecDeque::is_empty);
                        if queue_empty {
                            state.queues.remove(&flow_key);
                            state.ready_flows.retain(|ready| ready != &flow_key);
                        }
                        state.queued_jobs -= 1;
                        state.queued_audio_bytes -= job.audio_bytes;
                        decrement_flow(&mut state.flow_outstanding, &job.flow_key);
                        removed = Some(job);
                        break;
                    }
                    let removed = removed.expect("queued lifecycle must have a queued job");
                    *lifecycle = JobLifecycle::Cancelled;
                    (true, Some(removed))
                }
                JobLifecycle::Running => {
                    *lifecycle = JobLifecycle::Cancelled;
                    (true, None)
                }
                JobLifecycle::Cancelled | JobLifecycle::Completed => (false, None),
            }
        };

        if let Some(job) = removed {
            let _ = job.sender.send(Err(InferenceError::Cancelled));
            self.wake_workers.notify_all();
        }
        won
    }
}

pub struct ExecutionPool {
    snapshot: Arc<GenerationSnapshot>,
    state: Arc<SharedState>,
    worker_handles: Mutex<Vec<JoinHandle<()>>>,
}

pub struct InferenceRuntime {
    active: RwLock<Arc<ExecutionPool>>,
    retired: Mutex<Vec<Arc<ExecutionPool>>>,
    factory: Arc<dyn ExecutionModelFactory>,
    reload_lock: Mutex<()>,
    shutting_down: AtomicBool,
}

fn mark_load_timed_out(state: &SharedState, unit_id: &str, timeout: Duration) {
    let mut accounting = state
        .accounting
        .lock()
        .expect("pool accounting lock poisoned");
    let message = format!("preload timed out after {} ms", timeout.as_millis());
    if let Some(unit) = accounting.units.get_mut(unit_id) {
        unit.load_timed_out = true;
        unit.state = ExecutionUnitState::Loading;
        unit.last_error = Some(message.clone());
    }
    accounting.last_error = Some(message);
    drop(accounting);
    state.wake_workers.notify_all();
}

fn mark_worker_exited(state: &SharedState, unit_id: &str) {
    let mut accounting = state
        .accounting
        .lock()
        .expect("pool accounting lock poisoned");
    if let Some(unit) = accounting.units.get_mut(unit_id) {
        unit.worker_exited = true;
        if unit.load_timed_out {
            unit.state = ExecutionUnitState::Unhealthy;
        }
    }
    drop(accounting);
    state.wake_workers.notify_all();
}

fn mark_unit_unhealthy(state: &SharedState, unit_id: &str, message: String) {
    let stranded = {
        let mut accounting = state
            .accounting
            .lock()
            .expect("pool accounting lock poisoned");
        if let Some(unit) = accounting.units.get_mut(unit_id) {
            unit.state = ExecutionUnitState::Unhealthy;
            unit.last_error = Some(message.clone());
        }
        accounting.last_error = Some(message);

        let healthy_units = accounting
            .units
            .iter()
            .filter(|(_, unit)| unit.state != ExecutionUnitState::Unhealthy && !unit.load_timed_out)
            .map(|(id, _)| id.clone())
            .collect::<HashSet<_>>();
        let mut stranded = Vec::new();
        let flow_keys = accounting.queues.keys().cloned().collect::<Vec<_>>();
        for flow_key in flow_keys {
            let jobs = accounting
                .queues
                .remove(&flow_key)
                .expect("flow key came from queue map");
            let mut retained = VecDeque::new();
            for job in jobs {
                let compatible = job
                    .compatible_units
                    .as_ref()
                    .is_none_or(|units| units.iter().any(|id| healthy_units.contains(id)));
                if compatible {
                    retained.push_back(job);
                } else {
                    accounting.queued_jobs -= 1;
                    accounting.queued_audio_bytes -= job.audio_bytes;
                    decrement_flow(&mut accounting.flow_outstanding, &job.flow_key);
                    accounting.failed += 1;
                    *job.lifecycle.lock().expect("job lifecycle lock poisoned") =
                        JobLifecycle::Completed;
                    stranded.push(job);
                }
            }
            if !retained.is_empty() {
                accounting.queues.insert(flow_key, retained);
            }
        }
        let remaining_flows = accounting.queues.keys().cloned().collect::<HashSet<_>>();
        accounting
            .ready_flows
            .retain(|flow| remaining_flows.contains(flow));
        stranded
    };
    for job in stranded {
        let _ = job.sender.send(Err(InferenceError::NoCompatibleUnit));
    }
    state.wake_workers.notify_all();
}

fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    panic
        .downcast_ref::<&str>()
        .map(|message| (*message).to_string())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "worker panicked".to_string())
}

const MIN_AUDIO_SAMPLE_RATE: u32 = 8_000;
const MAX_AUDIO_SAMPLE_RATE: u32 = 384_000;

fn validate_sample_rate(sample_rate: u32) -> Result<()> {
    if !(MIN_AUDIO_SAMPLE_RATE..=MAX_AUDIO_SAMPLE_RATE).contains(&sample_rate) {
        anyhow::bail!(
            "audio sample rate {sample_rate} Hz is outside the supported range {MIN_AUDIO_SAMPLE_RATE}..={MAX_AUDIO_SAMPLE_RATE} Hz"
        );
    }
    Ok(())
}

fn predicted_sample_count(
    samples: usize,
    source_rate: u32,
    target_rate: u32,
) -> std::result::Result<usize, InferenceError> {
    if !(MIN_AUDIO_SAMPLE_RATE..=MAX_AUDIO_SAMPLE_RATE).contains(&source_rate)
        || !(MIN_AUDIO_SAMPLE_RATE..=MAX_AUDIO_SAMPLE_RATE).contains(&target_rate)
    {
        return Err(InferenceError::InvalidSampleRate(source_rate));
    }
    let predicted = (samples as u128)
        .checked_mul(u128::from(target_rate))
        .ok_or(InferenceError::AudioTooLarge)?
        .div_ceil(u128::from(source_rate));
    usize::try_from(predicted).map_err(|_| InferenceError::AudioTooLarge)
}

fn estimated_audio_bytes(
    input: &AudioInput,
    target_rate: u32,
) -> std::result::Result<usize, InferenceError> {
    let raw = input
        .samples
        .len()
        .checked_mul(std::mem::size_of::<f32>())
        .ok_or(InferenceError::AudioTooLarge)?;
    if input.sample_rate == target_rate {
        validate_sample_rate(input.sample_rate)
            .map_err(|_| InferenceError::InvalidSampleRate(input.sample_rate))?;
        return Ok(raw);
    }
    let predicted = predicted_sample_count(input.samples.len(), input.sample_rate, target_rate)?
        .checked_mul(std::mem::size_of::<f32>())
        .ok_or(InferenceError::AudioTooLarge)?;
    raw.checked_add(predicted)
        .ok_or(InferenceError::AudioTooLarge)
}

fn resample(input: AudioInput, target_rate: u32) -> Result<Vec<f32>> {
    validate_sample_rate(input.sample_rate)?;
    validate_sample_rate(target_rate)?;
    if input.sample_rate == target_rate {
        return Ok(input.samples);
    }
    if input.samples.is_empty() {
        return Ok(Vec::new());
    }
    let mut resampler = Fft::<f32>::new(
        input.sample_rate as usize,
        target_rate as usize,
        1024,
        1,
        FixedSync::Both,
    )
    .context("failed to initialize resampler")?;
    let input_buffer = InterleavedSlice::new(&input.samples, 1, input.samples.len())
        .context("failed to adapt audio buffer")?;
    let output = resampler
        .process_all(&input_buffer, input.samples.len(), None)
        .context("failed to resample audio")?;
    Ok(output.take_data())
}
