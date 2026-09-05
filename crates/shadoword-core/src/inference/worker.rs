use super::*;

pub(super) fn spawn_worker(
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
            threads: Some(host_threads.unwrap_or(DEFAULT_GPU_HOST_THREADS)),
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

pub(super) fn decrement_flow(flows: &mut HashMap<String, usize>, flow_key: &str) {
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
