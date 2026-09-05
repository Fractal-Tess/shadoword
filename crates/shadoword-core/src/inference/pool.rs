use super::*;

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

    pub(super) fn prepare_candidate(
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

    pub(super) fn draining_status(&self) -> DrainingGenerationStatus {
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

    pub(super) fn is_drained(&self) -> bool {
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

    pub(super) fn has_timed_out_load_running(&self) -> bool {
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
