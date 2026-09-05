use super::*;

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
