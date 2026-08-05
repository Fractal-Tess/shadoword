use crate::config::models_dir;
use crate::config::{ExecutionTarget, ExecutionUnitConfig, InferenceLimits, TranscriptionConfig};
use crate::contracts::{
    compiled_whisper_backends, DrainingGenerationStatus, ExecutionUnitState, ExecutionUnitStatus,
    InferencePoolStatus, ServiceStatus, TranscriptResponse, TranscriptionService,
};
use crate::model_download::default_whisper_model;
use anyhow::{anyhow, Context, Result};
use rubato::{FftFixedIn, Resampler};
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

impl ExecutionPool {
    pub fn prepare(
        config: TranscriptionConfig,
        generation: u64,
        factory: Arc<dyn ExecutionModelFactory>,
    ) -> Result<Self> {
        let (pool, prepared) = Self::prepare_candidate(config, generation, factory)?;
        if let Err(error) = prepared {
            pool.begin_drain();
            return Err(error);
        }
        Ok(pool)
    }

    fn prepare_candidate(
        config: TranscriptionConfig,
        generation: u64,
        factory: Arc<dyn ExecutionModelFactory>,
    ) -> Result<(Self, Result<()>)> {
        validate_sample_rate(config.sample_rate)?;
        let pool_config = config.effective_inference_pool()?;
        let enabled_units = pool_config
            .units
            .iter()
            .filter(|unit| unit.enabled)
            .cloned()
            .collect::<Vec<_>>();
        if enabled_units.is_empty() {
            return Err(anyhow!("inference pool has no enabled units"));
        }

        let eager = config.preload_on_startup;
        let snapshot = Arc::new(GenerationSnapshot { generation, config });
        let units = enabled_units
            .iter()
            .map(|unit| {
                (
                    unit.id.clone(),
                    UnitAccounting {
                        config: unit.clone(),
                        state: if eager {
                            ExecutionUnitState::Loading
                        } else {
                            ExecutionUnitState::Unloaded
                        },
                        load_timed_out: false,
                        worker_exited: false,
                        last_error: eager.then(|| "preloading".to_string()),
                        completed: 0,
                        failed: 0,
                    },
                )
            })
            .collect();
        let state = Arc::new(SharedState {
            accounting: Mutex::new(PoolAccounting {
                generation,
                accepting: true,
                limits: pool_config.limits,
                queues: HashMap::new(),
                ready_flows: VecDeque::new(),
                flow_outstanding: HashMap::new(),
                queued_jobs: 0,
                queued_audio_bytes: 0,
                running_jobs: 0,
                running_audio_bytes: 0,
                next_job_id: 1,
                units,
                completed: 0,
                failed: 0,
                last_error: None,
            }),
            wake_workers: Condvar::new(),
        });

        let (preload_tx, preload_rx) = mpsc::channel();
        let mut handles = Vec::with_capacity(enabled_units.len());
        let mut pending = HashSet::new();
        let mut required_failure = None;
        for unit in &enabled_units {
            match spawn_worker(
                unit.clone(),
                Arc::clone(&snapshot),
                Arc::clone(&state),
                Arc::clone(&factory),
                preload_tx.clone(),
                eager,
            ) {
                Ok(handle) => {
                    handles.push(handle);
                    if eager {
                        pending.insert(unit.id.clone());
                    }
                }
                Err(error) => {
                    let message = error.to_string();
                    mark_unit_unhealthy(&state, &unit.id, message.clone());
                    mark_worker_exited(&state, &unit.id);
                    if unit.required && required_failure.is_none() {
                        required_failure = Some(anyhow!(
                            "required execution unit {:?} failed to start: {message}",
                            unit.id
                        ));
                    }
                }
            }
        }
        drop(preload_tx);

        let pool = Self {
            snapshot,
            state,
            worker_handles: Mutex::new(handles),
        };

        if eager {
            let deadline = Instant::now() + Duration::from_millis(pool_config.preload_timeout_ms);
            while !pending.is_empty() && required_failure.is_none() {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    break;
                }
                match preload_rx.recv_timeout(remaining) {
                    Ok((unit_id, result)) => {
                        if !pending.remove(&unit_id) {
                            continue;
                        }
                        let unit = enabled_units
                            .iter()
                            .find(|unit| unit.id == unit_id)
                            .expect("worker reported a configured unit");
                        if let Err(message) = result {
                            if unit.required {
                                required_failure = Some(anyhow!(
                                    "required execution unit {:?} failed to preload: {message}",
                                    unit.id
                                ));
                            }
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => break,
                    Err(RecvTimeoutError::Disconnected) => {
                        required_failure = Some(anyhow!(
                            "execution workers exited before reporting preload status"
                        ));
                    }
                }
            }
            if required_failure.is_none() && !pending.is_empty() {
                let timeout = Duration::from_millis(pool_config.preload_timeout_ms);
                for unit_id in pending {
                    let unit = enabled_units
                        .iter()
                        .find(|unit| unit.id == unit_id)
                        .expect("pending unit must be configured");
                    mark_load_timed_out(&pool.state, &unit.id, timeout);
                    if unit.required && required_failure.is_none() {
                        required_failure = Some(anyhow!(
                            "required execution unit {:?} timed out after {} ms while preloading",
                            unit.id,
                            timeout.as_millis()
                        ));
                    }
                }
            }
        }

        if required_failure.is_some() {
            pool.begin_drain();
        }
        Ok((pool, required_failure.map_or(Ok(()), Err)))
    }

    pub fn generation(&self) -> u64 {
        self.snapshot.generation
    }

    pub fn config(&self) -> TranscriptionConfig {
        self.snapshot.config.clone()
    }

    pub fn submit(
        &self,
        request: InferenceRequest,
    ) -> std::result::Result<InferenceJob, InferenceError> {
        let audio_bytes = estimated_audio_bytes(&request.input, self.snapshot.config.sample_rate)?;
        let compatible_units = request
            .compatible_unit_ids
            .map(|units| units.into_iter().collect::<HashSet<_>>());
        let (sender, receiver) = mpsc::sync_channel(1);
        let lifecycle = Arc::new(Mutex::new(JobLifecycle::Queued));

        let id = {
            let mut state = self
                .state
                .accounting
                .lock()
                .expect("pool accounting lock poisoned");
            if !state.accepting {
                return Err(InferenceError::AdmissionClosed);
            }
            if audio_bytes > state.limits.max_audio_bytes_per_job {
                return Err(InferenceError::AudioTooLarge);
            }
            let has_compatible_unit = state.units.iter().any(|(id, unit)| {
                unit.state != ExecutionUnitState::Unhealthy
                    && !unit.load_timed_out
                    && compatible_units
                        .as_ref()
                        .is_none_or(|units| units.contains(id))
            });
            if !has_compatible_unit {
                return Err(InferenceError::NoCompatibleUnit);
            }
            let available_workers = state
                .units
                .values()
                .filter(|unit| unit.state != ExecutionUnitState::Unhealthy && !unit.load_timed_out)
                .count();
            let admitted_slots = available_workers.saturating_add(state.limits.max_queued_jobs);
            if state.queued_jobs.saturating_add(state.running_jobs) >= admitted_slots {
                return Err(InferenceError::QueueFull);
            }
            let queued_audio_bytes = state
                .queued_audio_bytes
                .checked_add(audio_bytes)
                .ok_or(InferenceError::AudioQueueFull)?;
            if queued_audio_bytes > state.limits.max_queued_audio_bytes {
                return Err(InferenceError::AudioQueueFull);
            }

            let id = state.next_job_id;
            state.next_job_id += 1;
            let flow_key = match &request.identity {
                InferenceIdentity::Batch => format!("batch-{id}"),
                InferenceIdentity::Stream { flow_id, .. } => flow_id.clone(),
            };
            if state.flow_outstanding.get(&flow_key).copied().unwrap_or(0)
                >= state.limits.max_outstanding_per_flow
            {
                return Err(InferenceError::FlowLimit);
            }

            let new_flow = !state.queues.contains_key(&flow_key);
            state
                .queues
                .entry(flow_key.clone())
                .or_default()
                .push_back(QueuedJob {
                    id,
                    flow_key: flow_key.clone(),
                    identity: request.identity,
                    input: request.input,
                    audio_bytes,
                    compatible_units,
                    snapshot: Arc::clone(&self.snapshot),
                    lifecycle: Arc::clone(&lifecycle),
                    sender,
                });
            if new_flow {
                state.ready_flows.push_back(flow_key.clone());
            }
            *state.flow_outstanding.entry(flow_key).or_default() += 1;
            state.queued_jobs += 1;
            state.queued_audio_bytes += audio_bytes;
            id
        };
        self.state.wake_workers.notify_all();

        Ok(InferenceJob {
            id,
            lifecycle,
            state: Arc::downgrade(&self.state),
            receiver: Mutex::new(receiver),
        })
    }

    pub fn submit_batch(
        &self,
        input: AudioInput,
    ) -> std::result::Result<InferenceJob, InferenceError> {
        self.submit(InferenceRequest::batch(input))
    }

    pub fn submit_stream(
        &self,
        flow_id: impl Into<String>,
        sequence: u64,
        input: AudioInput,
    ) -> std::result::Result<InferenceJob, InferenceError> {
        self.submit(InferenceRequest::stream(flow_id, sequence, input))
    }

    pub fn status(&self) -> InferencePoolStatus {
        let state = self
            .state
            .accounting
            .lock()
            .expect("pool accounting lock poisoned");
        let mut units = state
            .units
            .iter()
            .map(|(id, unit)| ExecutionUnitStatus {
                id: id.clone(),
                required: unit.config.required,
                target: unit.config.target.clone(),
                state: unit.state,
                last_error: unit.last_error.clone(),
                completed: unit.completed,
                failed: unit.failed,
            })
            .collect::<Vec<_>>();
        units.sort_by(|left, right| left.id.cmp(&right.id));
        let ready_units = units
            .iter()
            .filter(|unit| unit.state == ExecutionUnitState::Ready)
            .count();
        let busy_units = units
            .iter()
            .filter(|unit| unit.state == ExecutionUnitState::Busy)
            .count();
        let unhealthy_units = units
            .iter()
            .filter(|unit| unit.state == ExecutionUnitState::Unhealthy)
            .count();
        InferencePoolStatus {
            generation: state.generation,
            units,
            accepting: state.accepting,
            draining_generations: Vec::new(),
            ready_units,
            busy_units,
            unhealthy_units,
            queued_jobs: state.queued_jobs,
            queued_audio_bytes: state.queued_audio_bytes,
            running_jobs: state.running_jobs,
            running_audio_bytes: state.running_audio_bytes,
            completed: state.completed,
            failed: state.failed,
            last_error: state.last_error.clone(),
        }
    }

    pub fn begin_drain(&self) {
        let mut state = self
            .state
            .accounting
            .lock()
            .expect("pool accounting lock poisoned");
        state.accepting = false;
        drop(state);
        self.state.wake_workers.notify_all();
    }

    fn draining_status(&self) -> DrainingGenerationStatus {
        let state = self
            .state
            .accounting
            .lock()
            .expect("pool accounting lock poisoned");
        DrainingGenerationStatus {
            generation: state.generation,
            queued_jobs: state.queued_jobs,
            queued_audio_bytes: state.queued_audio_bytes,
            running_jobs: state.running_jobs,
            running_audio_bytes: state.running_audio_bytes,
            workers_remaining: self.workers_remaining(),
            loading_units: state
                .units
                .values()
                .filter(|unit| unit.state == ExecutionUnitState::Loading && !unit.worker_exited)
                .count(),
        }
    }

    fn workers_remaining(&self) -> usize {
        self.worker_handles
            .lock()
            .expect("worker handles lock poisoned")
            .iter()
            .filter(|handle| !handle.is_finished())
            .count()
    }

    fn reap_finished_workers(&self) {
        let mut handles = self
            .worker_handles
            .lock()
            .expect("worker handles lock poisoned");
        let mut index = 0;
        while index < handles.len() {
            if handles[index].is_finished() {
                let handle = handles.swap_remove(index);
                let _ = handle.join();
            } else {
                index += 1;
            }
        }
    }

    fn is_drained(&self) -> bool {
        self.reap_finished_workers();
        let state = self
            .state
            .accounting
            .lock()
            .expect("pool accounting lock poisoned");
        state.queued_jobs == 0
            && state.running_jobs == 0
            && self
                .worker_handles
                .lock()
                .expect("worker handles lock poisoned")
                .is_empty()
    }

    fn has_timed_out_load_running(&self) -> bool {
        self.state
            .accounting
            .lock()
            .expect("pool accounting lock poisoned")
            .units
            .values()
            .any(|unit| unit.load_timed_out && !unit.worker_exited)
    }
}

impl Drop for ExecutionPool {
    fn drop(&mut self) {
        self.begin_drain();
    }
}

pub struct InferenceRuntime {
    active: RwLock<Arc<ExecutionPool>>,
    retired: Mutex<Vec<Arc<ExecutionPool>>>,
    factory: Arc<dyn ExecutionModelFactory>,
    reload_lock: Mutex<()>,
    shutting_down: AtomicBool,
}

impl InferenceRuntime {
    pub fn new(config: TranscriptionConfig) -> Result<Self> {
        Self::new_with_factory(config, Arc::new(WhisperModelFactory))
    }

    pub fn new_with_factory(
        config: TranscriptionConfig,
        factory: Arc<dyn ExecutionModelFactory>,
    ) -> Result<Self> {
        let (pool, prepared) = ExecutionPool::prepare_candidate(config, 1, Arc::clone(&factory))?;
        prepared?;
        let pool = Arc::new(pool);
        Ok(Self {
            active: RwLock::new(pool),
            retired: Mutex::new(Vec::new()),
            factory,
            reload_lock: Mutex::new(()),
            shutting_down: AtomicBool::new(false),
        })
    }

    pub fn submit(
        &self,
        request: InferenceRequest,
    ) -> std::result::Result<InferenceJob, InferenceError> {
        self.active
            .read()
            .expect("active inference pool lock poisoned")
            .submit(request)
    }

    pub fn submit_batch(
        &self,
        input: AudioInput,
    ) -> std::result::Result<InferenceJob, InferenceError> {
        self.submit(InferenceRequest::batch(input))
    }

    pub fn submit_stream(
        &self,
        flow_id: impl Into<String>,
        sequence: u64,
        input: AudioInput,
    ) -> std::result::Result<InferenceJob, InferenceError> {
        self.submit(InferenceRequest::stream(flow_id, sequence, input))
    }

    pub fn reload(&self, config: TranscriptionConfig) -> Result<u64> {
        self.reload_transactional(None, config, || Ok(()))
    }

    /// Prepares a candidate while the active generation remains unchanged, runs
    /// the persistence callback, and only then commits the prepared generation.
    /// The reload lock makes the final generation-checked swap infallible.
    pub fn reload_transactional<F>(
        &self,
        expected_generation: Option<u64>,
        config: TranscriptionConfig,
        persist: F,
    ) -> Result<u64>
    where
        F: FnOnce() -> Result<()>,
    {
        let _reload = self
            .reload_lock
            .lock()
            .expect("inference reload lock poisoned");
        if self.shutting_down.load(Ordering::Acquire) {
            return Err(anyhow!("inference runtime is shutting down"));
        }
        self.reap_retired();
        let active_generation = self
            .active
            .read()
            .expect("active inference pool lock poisoned")
            .generation();
        if let Some(expected) = expected_generation {
            if expected != active_generation {
                return Err(anyhow!(
                    "stale runtime generation: expected {expected}, active generation is {active_generation}"
                ));
            }
        }
        let active_has_timed_out_load = self
            .active
            .read()
            .expect("active inference pool lock poisoned")
            .has_timed_out_load_running();
        let retired_has_timed_out_load = self
            .retired
            .lock()
            .expect("retired inference pools lock poisoned")
            .iter()
            .any(|pool| pool.has_timed_out_load_running());
        if active_has_timed_out_load || retired_has_timed_out_load {
            return Err(anyhow!(
                "inference reload is blocked by a timed-out model load that is still running"
            ));
        }
        let max_draining = self
            .active
            .read()
            .expect("active inference pool lock poisoned")
            .config()
            .effective_inference_pool()?
            .max_draining_generations;
        if self
            .retired
            .lock()
            .expect("retired inference pools lock poisoned")
            .len()
            >= max_draining
        {
            return Err(anyhow!(
                "inference reload is blocked by {max_draining} draining generation(s)"
            ));
        }
        let generation = active_generation + 1;
        let (candidate, prepared) =
            ExecutionPool::prepare_candidate(config, generation, Arc::clone(&self.factory))?;
        let candidate = Arc::new(candidate);
        if let Err(error) = prepared {
            candidate.begin_drain();
            self.retired
                .lock()
                .expect("retired inference pools lock poisoned")
                .push(candidate);
            return Err(error);
        }
        if let Err(error) = persist() {
            candidate.begin_drain();
            self.retired
                .lock()
                .expect("retired inference pools lock poisoned")
                .push(candidate);
            return Err(error);
        }
        let old = {
            let mut active = self
                .active
                .write()
                .expect("active inference pool lock poisoned");
            std::mem::replace(&mut *active, candidate)
        };
        old.begin_drain();
        self.retired
            .lock()
            .expect("retired inference pools lock poisoned")
            .push(old);
        Ok(generation)
    }

    pub fn generation(&self) -> u64 {
        self.active
            .read()
            .expect("active inference pool lock poisoned")
            .generation()
    }

    pub fn transcription_config(&self) -> TranscriptionConfig {
        self.active
            .read()
            .expect("active inference pool lock poisoned")
            .config()
    }

    pub fn status(&self) -> InferencePoolStatus {
        self.reap_retired();
        let mut status = self
            .active
            .read()
            .expect("active inference pool lock poisoned")
            .status();
        let retired = self
            .retired
            .lock()
            .expect("retired inference pools lock poisoned");
        for pool in retired.iter() {
            let draining = pool.draining_status();
            status.queued_jobs = status.queued_jobs.saturating_add(draining.queued_jobs);
            status.queued_audio_bytes = status
                .queued_audio_bytes
                .saturating_add(draining.queued_audio_bytes);
            status.running_jobs = status.running_jobs.saturating_add(draining.running_jobs);
            status.running_audio_bytes = status
                .running_audio_bytes
                .saturating_add(draining.running_audio_bytes);
            status.draining_generations.push(draining);
        }
        status
    }

    pub fn begin_shutdown(&self) {
        self.shutting_down.store(true, Ordering::Release);
        self.active
            .read()
            .expect("active inference pool lock poisoned")
            .begin_drain();
        for pool in self
            .retired
            .lock()
            .expect("retired inference pools lock poisoned")
            .iter()
        {
            pool.begin_drain();
        }
    }

    pub fn wait_for_drain(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            self.reap_retired();
            let active_drained = self
                .active
                .read()
                .expect("active inference pool lock poisoned")
                .is_drained();
            let retired_empty = self
                .retired
                .lock()
                .expect("retired inference pools lock poisoned")
                .is_empty();
            if active_drained && retired_empty {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    pub fn shutdown(&self, timeout: Duration) -> bool {
        self.begin_shutdown();
        self.wait_for_drain(timeout)
    }

    fn reap_retired(&self) {
        self.retired
            .lock()
            .expect("retired inference pools lock poisoned")
            .retain(|pool| !pool.is_drained());
    }
}

impl Drop for InferenceRuntime {
    fn drop(&mut self) {
        self.begin_shutdown();
    }
}

impl TranscriptionService for InferenceRuntime {
    fn status(&self) -> Result<ServiceStatus> {
        let config = self.transcription_config();
        let pool = self.status();
        Ok(ServiceStatus {
            model_loaded: pool.units.iter().any(|unit| {
                matches!(
                    unit.state,
                    ExecutionUnitState::Ready | ExecutionUnitState::Busy
                )
            }),
            engine: "whisper".to_string(),
            model_path: config.model_path.display().to_string(),
            whisper_accelerator: config.whisper_accelerator,
            whisper_gpu_device: config.whisper_gpu_device,
            compiled_whisper_backends: compiled_whisper_backends(),
            available_gpu_devices: list_whisper_gpu_devices(),
            sample_rate: config.sample_rate,
            inference_pool: Some(pool),
        })
    }

    fn transcribe_audio(&self, input: AudioInput) -> Result<TranscriptResponse> {
        self.submit_batch(input)
            .map_err(anyhow::Error::new)?
            .wait()
            .map(|completion| completion.response)
            .map_err(anyhow::Error::new)
    }
}

fn spawn_worker(
    unit: ExecutionUnitConfig,
    snapshot: Arc<GenerationSnapshot>,
    state: Arc<SharedState>,
    factory: Arc<dyn ExecutionModelFactory>,
    preload_tx: Sender<(String, std::result::Result<(), String>)>,
    eager: bool,
) -> Result<JoinHandle<()>> {
    let thread_name = format!("inference-{}-g{}", unit.id, snapshot.generation);
    thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            let exit_state = Arc::clone(&state);
            let exit_unit_id = unit.id.clone();
            let _exit = WorkerExitGuard {
                state: exit_state,
                unit_id: exit_unit_id,
            };
            worker_entry(unit, snapshot, state, factory, preload_tx, eager);
        })
        .context("failed to spawn inference worker")
}

struct WorkerExitGuard {
    state: Arc<SharedState>,
    unit_id: String,
}

impl Drop for WorkerExitGuard {
    fn drop(&mut self) {
        mark_worker_exited(&self.state, &self.unit_id);
    }
}

fn worker_entry(
    unit: ExecutionUnitConfig,
    snapshot: Arc<GenerationSnapshot>,
    state: Arc<SharedState>,
    factory: Arc<dyn ExecutionModelFactory>,
    preload_tx: Sender<(String, std::result::Result<(), String>)>,
    eager: bool,
) {
    if eager {
        let loaded = catch_unwind(AssertUnwindSafe(|| {
            load_worker_model(&unit, &snapshot, factory)
        }))
        .map_err(panic_message)
        .and_then(|result| result.map_err(|error| error.message));
        match loaded {
            Ok(model) => {
                let usable = mark_eager_load_ready(&state, &unit.id);
                let _ = preload_tx.send((unit.id.clone(), Ok(())));
                if usable {
                    worker_loop(&unit.id, model, state);
                }
            }
            Err(message) => {
                mark_unit_unhealthy(&state, &unit.id, message.clone());
                let _ = preload_tx.send((unit.id.clone(), Err(message)));
            }
        }
        return;
    }

    let Some(first_job) = take_job(&unit.id, &state, false) else {
        return;
    };
    let loaded = catch_unwind(AssertUnwindSafe(|| {
        load_worker_model(&unit, &snapshot, factory)
    }))
    .map_err(panic_message)
    .and_then(|result| result.map_err(|error| error.message));
    let model = match loaded {
        Ok(model) => model,
        Err(message) => {
            mark_unit_unhealthy(&state, &unit.id, message.clone());
            finish_job(
                &unit.id,
                &state,
                first_job,
                Err(InferenceError::WorkerFailed(message.clone())),
            );
            return;
        }
    };
    execute_owned_job(&unit.id, model.as_ref(), &state, first_job);
    worker_loop(&unit.id, model, state);
}

fn load_worker_model(
    unit: &ExecutionUnitConfig,
    snapshot: &GenerationSnapshot,
    factory: Arc<dyn ExecutionModelFactory>,
) -> SharedResult<Box<dyn Model>> {
    let model_path = resolve_model_path(&snapshot.config).map_err(|error| ModelError {
        message: error.to_string(),
    })?;
    let affinity = model_affinity(unit, &snapshot.config);
    let load_scope = factory
        .load_scope()
        .unwrap_or(Arc::as_ptr(&factory) as *const () as usize);
    let _load_lease = TargetLoadLease::acquire(load_scope, &unit.id, &affinity)?;
    let mut model = factory.create(unit)?;
    model.load(&ModelConfig {
        id: "whisper".to_string(),
        model_path: model_path.display().to_string(),
        affinity: Some(affinity),
    })?;
    Ok(model)
}

fn model_affinity(unit: &ExecutionUnitConfig, config: &TranscriptionConfig) -> ModelAffinity {
    match unit.target {
        ExecutionTarget::Cpu { threads } => ModelAffinity::Cpu { threads },
        ExecutionTarget::Gpu { device, .. } if device < 0 => ModelAffinity::Auto {
            gpu_device: config.whisper_gpu_device,
        },
        ExecutionTarget::Gpu {
            device,
            host_threads,
        } => ModelAffinity::Gpu {
            device,
            threads: Some(host_threads.unwrap_or(1)),
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LoadTarget {
    Cpu(String),
    Gpu(i32),
    Auto,
}

impl LoadTarget {
    fn from_affinity(unit_id: &str, affinity: &ModelAffinity) -> Self {
        match affinity {
            ModelAffinity::Cpu { .. } => Self::Cpu(unit_id.to_string()),
            ModelAffinity::Gpu { device, .. } => Self::Gpu(*device),
            ModelAffinity::Auto { .. } => Self::Auto,
        }
    }

    fn conflicts_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Cpu(left), Self::Cpu(right)) => left == right,
            (Self::Gpu(left), Self::Gpu(right)) => left == right,
            (Self::Auto, Self::Auto | Self::Gpu(_)) | (Self::Gpu(_), Self::Auto) => true,
            _ => false,
        }
    }
}

fn loading_targets() -> &'static Mutex<Vec<(usize, LoadTarget)>> {
    static TARGETS: OnceLock<Mutex<Vec<(usize, LoadTarget)>>> = OnceLock::new();
    TARGETS.get_or_init(|| Mutex::new(Vec::new()))
}

struct TargetLoadLease {
    scope: usize,
    target: LoadTarget,
}

impl TargetLoadLease {
    fn acquire(scope: usize, unit_id: &str, affinity: &ModelAffinity) -> SharedResult<Self> {
        let target = LoadTarget::from_affinity(unit_id, affinity);
        let mut loading = loading_targets().lock().map_err(|_| ModelError {
            message: "model load target registry is unavailable".to_string(),
        })?;
        if loading.iter().any(|(candidate_scope, candidate)| {
            *candidate_scope == scope && candidate.conflicts_with(&target)
        }) {
            return Err(ModelError {
                message: format!(
                    "model load for execution target {target:?} is already in progress; wait for the quarantined loader to finish"
                ),
            });
        }
        loading.push((scope, target.clone()));
        Ok(Self { scope, target })
    }
}

impl Drop for TargetLoadLease {
    fn drop(&mut self) {
        if let Ok(mut loading) = loading_targets().lock() {
            if let Some(index) = loading
                .iter()
                .position(|(scope, target)| *scope == self.scope && target == &self.target)
            {
                loading.swap_remove(index);
            }
        }
    }
}

fn resolve_model_path(config: &TranscriptionConfig) -> Result<std::path::PathBuf> {
    if !config.model_path.as_os_str().is_empty() {
        return Ok(config.model_path.clone());
    }
    Ok(models_dir()?.join(default_whisper_model().filename))
}

fn worker_loop(unit_id: &str, model: Box<dyn Model>, state: Arc<SharedState>) {
    loop {
        let Some(job) = take_job(unit_id, &state, true) else {
            return;
        };
        if !execute_owned_job(unit_id, model.as_ref(), &state, job) {
            return;
        }
    }
}

fn execute_owned_job(
    unit_id: &str,
    model: &dyn Model,
    state: &SharedState,
    mut job: QueuedJob,
) -> bool {
    let started = Instant::now();
    let input = AudioInput {
        samples: std::mem::take(&mut job.input.samples),
        sample_rate: job.input.sample_rate,
    };
    let result = catch_unwind(AssertUnwindSafe(|| {
        execute_job(model, input, &job, unit_id, started)
    }));
    match result {
        Ok(result) => {
            finish_job(unit_id, state, job, result);
            true
        }
        Err(panic) => {
            let message = panic_message(panic);
            mark_unit_unhealthy(state, unit_id, message.clone());
            finish_job(
                unit_id,
                state,
                job,
                Err(InferenceError::WorkerFailed(message.clone())),
            );
            false
        }
    }
}

fn take_job(unit_id: &str, state: &SharedState, loaded: bool) -> Option<QueuedJob> {
    let mut accounting = state
        .accounting
        .lock()
        .expect("pool accounting lock poisoned");
    loop {
        if accounting
            .units
            .get(unit_id)
            .is_none_or(|unit| unit.state == ExecutionUnitState::Unhealthy || unit.load_timed_out)
        {
            return None;
        }
        let flow_count = accounting.ready_flows.len();
        for _ in 0..flow_count {
            let flow_key = accounting
                .ready_flows
                .pop_front()
                .expect("flow count came from ready queue");
            let compatible = accounting
                .queues
                .get(&flow_key)
                .and_then(VecDeque::front)
                .is_some_and(|job| job.compatible_with(unit_id));
            if !compatible {
                accounting.ready_flows.push_back(flow_key);
                continue;
            }

            let job = accounting
                .queues
                .get_mut(&flow_key)
                .and_then(VecDeque::pop_front)
                .expect("ready flow must contain a job");
            let still_ready = accounting
                .queues
                .get(&flow_key)
                .is_some_and(|queue| !queue.is_empty());
            if still_ready {
                accounting.ready_flows.push_back(flow_key);
            } else {
                accounting.queues.remove(&flow_key);
            }
            accounting.queued_jobs -= 1;
            accounting.queued_audio_bytes -= job.audio_bytes;
            accounting.running_jobs += 1;
            accounting.running_audio_bytes += job.audio_bytes;
            *job.lifecycle.lock().expect("job lifecycle lock poisoned") = JobLifecycle::Running;
            if let Some(unit) = accounting.units.get_mut(unit_id) {
                unit.state = if loaded {
                    ExecutionUnitState::Busy
                } else {
                    ExecutionUnitState::Loading
                };
            }
            return Some(job);
        }

        if !accounting.accepting && accounting.queued_jobs == 0 {
            return None;
        }
        accounting = state
            .wake_workers
            .wait(accounting)
            .expect("pool accounting lock poisoned while waiting");
    }
}

fn execute_job(
    model: &dyn Model,
    input: AudioInput,
    job: &QueuedJob,
    unit_id: &str,
    started: Instant,
) -> std::result::Result<InferenceCompletion, InferenceError> {
    let config = &job.snapshot.config;
    let samples = resample(input, config.sample_rate)
        .map_err(|error| InferenceError::WorkerFailed(error.to_string()))?;
    let input = AudioInput {
        samples,
        sample_rate: config.sample_rate,
    };
    let transcription = model
        .transcribe(
            &input,
            &TranscriptionOptions {
                language: config.english_only.then(|| "en".to_string()),
                translate_to_english: false,
            },
        )
        .map_err(|error| InferenceError::WorkerFailed(error.message))?;
    Ok(InferenceCompletion {
        generation: job.snapshot.generation,
        unit_id: unit_id.to_string(),
        identity: job.identity.clone(),
        response: TranscriptResponse {
            text: transcription.text,
            elapsed_ms: started.elapsed().as_millis(),
            engine: model.name().to_string(),
        },
    })
}

fn finish_job(
    unit_id: &str,
    state: &SharedState,
    job: QueuedJob,
    result: std::result::Result<InferenceCompletion, InferenceError>,
) {
    let cancelled = {
        let mut accounting = state
            .accounting
            .lock()
            .expect("pool accounting lock poisoned");
        let mut lifecycle = job.lifecycle.lock().expect("job lifecycle lock poisoned");
        let cancelled = match *lifecycle {
            JobLifecycle::Cancelled => true,
            JobLifecycle::Running => {
                *lifecycle = JobLifecycle::Completed;
                false
            }
            JobLifecycle::Queued | JobLifecycle::Completed => {
                unreachable!("running job must be running or cancelled at completion")
            }
        };
        accounting.running_jobs -= 1;
        accounting.running_audio_bytes -= job.audio_bytes;
        decrement_flow(&mut accounting.flow_outstanding, &job.flow_key);
        if let Some(unit) = accounting.units.get_mut(unit_id) {
            if unit.state != ExecutionUnitState::Unhealthy {
                unit.state = ExecutionUnitState::Ready;
            }
            if !cancelled {
                if let Err(error) = &result {
                    unit.failed += 1;
                    unit.last_error = Some(error.to_string());
                } else {
                    unit.completed += 1;
                }
            }
        }
        if !cancelled {
            if let Err(error) = &result {
                accounting.failed += 1;
                accounting.last_error = Some(error.to_string());
            } else {
                accounting.completed += 1;
            }
        }
        cancelled
    };
    let response = if cancelled {
        Err(InferenceError::Cancelled)
    } else {
        result
    };
    let _ = job.sender.send(response);
    state.wake_workers.notify_all();
}

fn decrement_flow(flows: &mut HashMap<String, usize>, flow_key: &str) {
    if let Some(outstanding) = flows.get_mut(flow_key) {
        *outstanding -= 1;
        if *outstanding == 0 {
            flows.remove(flow_key);
        }
    }
}

fn mark_eager_load_ready(state: &SharedState, unit_id: &str) -> bool {
    let mut accounting = state
        .accounting
        .lock()
        .expect("pool accounting lock poisoned");
    let accepting = accounting.accepting;
    if let Some(unit) = accounting.units.get_mut(unit_id) {
        if unit.load_timed_out || !accepting {
            return false;
        }
        unit.state = ExecutionUnitState::Ready;
        unit.last_error = None;
        return true;
    }
    false
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
    let chunk_size = 1024;
    let mut resampler = FftFixedIn::<f32>::new(
        input.sample_rate as usize,
        target_rate as usize,
        chunk_size,
        1,
        1,
    )
    .context("failed to initialize resampler")?;
    let expected_samples =
        predicted_sample_count(input.samples.len(), input.sample_rate, target_rate)
            .map_err(anyhow::Error::new)?;
    let mut output = Vec::with_capacity(expected_samples);
    let mut chunks = input.samples.chunks_exact(chunk_size);
    for chunk in &mut chunks {
        let processed = resampler
            .process(&[chunk], None)
            .context("failed to resample audio")?;
        output.extend_from_slice(&processed[0]);
    }
    let remainder = chunks.remainder();
    if !remainder.is_empty() {
        let processed = resampler
            .process_partial(Some(&[remainder]), None)
            .context("failed to resample final audio frames")?;
        output.extend_from_slice(&processed[0]);
    }
    while output.len() < expected_samples {
        let processed = resampler
            .process_partial::<&[f32]>(None, None)
            .context("failed to flush delayed resampler frames")?;
        if processed[0].is_empty() {
            break;
        }
        output.extend_from_slice(&processed[0]);
    }
    output.truncate(expected_samples);
    Ok(output)
}
