use crate::config::{models_dir, TranscriptionConfig, WhisperAccelerator};
use crate::contracts::{
    compiled_whisper_backends, ServiceStatus, TranscriptResponse, TranscriptionService,
};
use crate::model_download::default_whisper_model;
use crate::wav;
use anyhow::{anyhow, Context, Result};
use rubato::{FftFixedIn, Resampler};
use shadoword_model_whisper::{list_whisper_gpu_devices, WhisperModel};
use shadoword_shared::AudioInput;
use shadoword_shared::{Model, ModelAffinity, ModelConfig, TranscriptionOptions};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

#[derive(Debug, Clone, Default)]
struct ProfileTimings {
    request_bytes: usize,
    input_samples: usize,
    output_samples: usize,
    sample_rate: u32,
    decoded_wav_ms: u128,
    ensure_loaded_ms: u128,
    resample_ms: u128,
    inference_ms: u128,
    total_ms: u128,
    cold_load: bool,
}

pub struct LocalService {
    config: Arc<RwLock<TranscriptionConfig>>,
    model: Arc<Mutex<Box<dyn Model>>>,
    config_generation: Arc<AtomicU64>,
    loaded_generation: Arc<AtomicU64>,
}

struct BackendRegistry;

impl BackendRegistry {
    fn create(backend_id: &str) -> Result<Box<dyn Model>> {
        match backend_id {
            "whisper" => Ok(Box::new(WhisperModel::new())),
            other => Err(anyhow!("unsupported transcription backend: {other}")),
        }
    }
}

impl LocalService {
    const BACKEND_ID: &'static str = "whisper";

    fn resolve_model_path(config: &TranscriptionConfig) -> Result<std::path::PathBuf> {
        if !config.model_path.as_os_str().is_empty() {
            return Ok(config.model_path.clone());
        }

        Ok(models_dir()?.join(default_whisper_model().filename))
    }

    fn compiled_backend_summary() -> &'static str {
        if cfg!(all(feature = "whisper-vulkan", feature = "whisper-cuda")) {
            "whisper-vulkan,whisper-cuda"
        } else if cfg!(feature = "whisper-vulkan") {
            "whisper-vulkan"
        } else if cfg!(feature = "whisper-cuda") {
            "whisper-cuda"
        } else {
            "cpu-only"
        }
    }

    fn log_backend_request(config: &TranscriptionConfig, phase: &str) {
        tracing::info!(
            target: "shadowword.backend",
            phase,
            engine = Self::BACKEND_ID,
            model_path = %config.model_path.display(),
            whisper_accelerator = ?config.whisper_accelerator,
            whisper_gpu_device = config.whisper_gpu_device,
            compiled_backends = Self::compiled_backend_summary(),
            "backend configuration"
        );
    }

    pub fn new(config: TranscriptionConfig) -> Self {
        Self::new_with_model(
            config,
            BackendRegistry::create(Self::BACKEND_ID).expect("whisper backend registered"),
        )
    }

    pub fn new_with_model(config: TranscriptionConfig, model: Box<dyn Model>) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            model: Arc::new(Mutex::new(model)),
            config_generation: Arc::new(AtomicU64::new(1)),
            loaded_generation: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn transcription_config(&self) -> TranscriptionConfig {
        self.config.read().expect("config lock poisoned").clone()
    }

    pub fn update_config(&self, next: TranscriptionConfig) -> Result<()> {
        let reload_required = {
            let mut current = self.config.write().expect("config lock poisoned");
            let reload_required = current.backend_reload_required(&next);
            *current = next;
            reload_required
        };

        if reload_required {
            self.config_generation.fetch_add(1, Ordering::AcqRel);
        }
        Ok(())
    }

    fn model_affinity(config: &TranscriptionConfig) -> ModelAffinity {
        match config.whisper_accelerator {
            WhisperAccelerator::Auto => ModelAffinity::Auto {
                gpu_device: config.whisper_gpu_device,
            },
            WhisperAccelerator::Cpu => ModelAffinity::Cpu { threads: None },
            WhisperAccelerator::Gpu => ModelAffinity::Gpu {
                device: config.whisper_gpu_device,
                threads: None,
            },
        }
    }

    fn profiling_enabled() -> bool {
        std::env::var_os("SHADOWWORD_PROFILE").is_some()
    }

    fn log_profile(&self, config: &TranscriptionConfig, timings: &ProfileTimings) {
        if !Self::profiling_enabled() {
            return;
        }

        tracing::info!(
            target: "shadowword.profile",
            engine = "whisper",
            whisper = ?config.whisper_accelerator,
            whisper_gpu_device = config.whisper_gpu_device,
            sample_rate = timings.sample_rate,
            request_bytes = timings.request_bytes,
            input_samples = timings.input_samples,
            output_samples = timings.output_samples,
            cold_load = timings.cold_load,
            decode_wav_ms = timings.decoded_wav_ms,
            ensure_loaded_ms = timings.ensure_loaded_ms,
            resample_ms = timings.resample_ms,
            inference_ms = timings.inference_ms,
            total_ms = timings.total_ms,
            "transcription profile"
        );
    }

    pub fn preload(&self) -> Result<()> {
        self.ensure_loaded()?;
        Ok(())
    }

    pub fn is_loaded(&self) -> bool {
        let loaded = self.loaded_generation.load(Ordering::Acquire);
        loaded != 0 && loaded == self.config_generation.load(Ordering::Acquire)
    }

    fn ensure_loaded(&self) -> Result<bool> {
        let mut model_guard = self.model.lock().expect("model lock poisoned");
        let generation = self.config_generation.load(Ordering::Acquire);
        let config = self.transcription_config();
        let model_path = Self::resolve_model_path(&config)?;
        Self::log_backend_request(&config, "ensure_loaded");
        if self.loaded_generation.load(Ordering::Acquire) != generation && model_guard.is_loaded() {
            model_guard
                .unload()
                .map_err(|error| anyhow!(error.message))?;
            self.loaded_generation.store(0, Ordering::Release);
        }
        if model_guard.is_loaded() {
            tracing::info!(
                target: "shadowword.backend",
                engine = model_guard.name(),
                model_path = %model_path.display(),
                compiled_backends = Self::compiled_backend_summary(),
                "model already loaded"
            );
            return Ok(false);
        }

        if !model_path.exists() {
            return Err(anyhow!(
                "model is not installed: {}. Download it explicitly before transcription",
                model_path.display()
            ));
        }

        model_guard
            .load(&ModelConfig {
                id: Self::BACKEND_ID.to_string(),
                model_path: model_path.display().to_string(),
                affinity: Some(Self::model_affinity(&config)),
            })
            .map_err(|error| anyhow!(error.message))?;
        self.loaded_generation.store(generation, Ordering::Release);
        tracing::info!(
            target: "shadowword.backend",
            engine = model_guard.name(),
            model_path = %model_path.display(),
            compiled_backends = Self::compiled_backend_summary(),
            "model load complete"
        );
        Ok(true)
    }

    fn resample_if_needed(&self, input: AudioInput) -> Result<Vec<f32>> {
        let target_rate = self.transcription_config().sample_rate as usize;
        if input.sample_rate as usize == target_rate {
            return Ok(input.samples);
        }

        let chunk_size = 1024;
        let mut resampler =
            FftFixedIn::<f32>::new(input.sample_rate as usize, target_rate, chunk_size, 1, 1)
                .context("failed to initialize resampler")?;

        let expected_samples = ((input.samples.len() as u128 * target_rate as u128)
            .div_ceil(input.sample_rate as u128)) as usize;
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

    fn transcribe_audio_internal(
        &self,
        input: AudioInput,
        timings: &mut ProfileTimings,
    ) -> Result<TranscriptResponse> {
        timings.sample_rate = input.sample_rate;
        timings.input_samples = input.samples.len();
        let total_start = Instant::now();

        let ensure_loaded_start = Instant::now();
        timings.cold_load = self.ensure_loaded()?;
        timings.ensure_loaded_ms = ensure_loaded_start.elapsed().as_millis();

        let resample_start = Instant::now();
        let audio = self.resample_if_needed(input)?;
        timings.resample_ms = resample_start.elapsed().as_millis();
        timings.output_samples = audio.len();

        let config = self.transcription_config();
        Self::log_backend_request(&config, "transcribe");
        let inference_start = Instant::now();

        let options = TranscriptionOptions {
            language: config.english_only.then(|| "en".to_string()),
            translate_to_english: false,
        };
        let model_input = AudioInput {
            samples: audio,
            sample_rate: config.sample_rate,
        };
        let text = self
            .model
            .lock()
            .expect("model lock poisoned")
            .transcribe(&model_input, &options)
            .map_err(|error| anyhow!(error.message))?
            .text;

        timings.inference_ms = inference_start.elapsed().as_millis();
        timings.total_ms = total_start.elapsed().as_millis();
        tracing::info!(
            target: "shadowword.backend",
            engine = "whisper",
            elapsed_ms = timings.total_ms,
            inference_ms = timings.inference_ms,
            cold_load = timings.cold_load,
            compiled_backends = Self::compiled_backend_summary(),
            "transcription complete"
        );

        Ok(TranscriptResponse {
            text,
            elapsed_ms: timings.total_ms,
            engine: Self::BACKEND_ID.to_string(),
        })
    }
}

impl TranscriptionService for LocalService {
    fn status(&self) -> Result<ServiceStatus> {
        let config = self.transcription_config();
        Ok(ServiceStatus {
            model_loaded: self.is_loaded(),
            engine: Self::BACKEND_ID.to_string(),
            model_path: config.model_path.display().to_string(),
            whisper_accelerator: config.whisper_accelerator,
            whisper_gpu_device: config.whisper_gpu_device,
            compiled_whisper_backends: compiled_whisper_backends(),
            available_gpu_devices: list_whisper_gpu_devices(),
            sample_rate: config.sample_rate,
            inference_pool: None,
        })
    }

    fn transcribe_wav_bytes(&self, bytes: &[u8]) -> Result<TranscriptResponse> {
        let mut timings = ProfileTimings {
            request_bytes: bytes.len(),
            ..Default::default()
        };

        let decode_wav_start = Instant::now();
        let input = wav::decode_wav(bytes)?;
        timings.decoded_wav_ms = decode_wav_start.elapsed().as_millis();

        let response = self.transcribe_audio_internal(input, &mut timings)?;
        let config = self.transcription_config();
        self.log_profile(&config, &timings);
        Ok(response)
    }

    fn transcribe_audio(&self, input: AudioInput) -> Result<TranscriptResponse> {
        let mut timings = ProfileTimings::default();
        let response = self.transcribe_audio_internal(input, &mut timings)?;
        let config = self.transcription_config();
        self.log_profile(&config, &timings);
        Ok(response)
    }
}

impl Clone for LocalService {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            model: Arc::clone(&self.model),
            config_generation: Arc::clone(&self.config_generation),
            loaded_generation: Arc::clone(&self.loaded_generation),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shadoword_shared::{LoadProgress, ModelError, SharedResult, Transcription};

    struct NoopModel;

    impl Model for NoopModel {
        fn name(&self) -> &'static str {
            "noop"
        }

        fn load(&mut self, _config: &ModelConfig) -> SharedResult<()> {
            Ok(())
        }

        fn unload(&mut self) -> SharedResult<()> {
            Ok(())
        }

        fn is_loaded(&self) -> bool {
            false
        }

        fn load_progress(&self) -> Option<LoadProgress> {
            None
        }

        fn transcribe(
            &self,
            _input: &AudioInput,
            _options: &TranscriptionOptions,
        ) -> Result<Transcription, ModelError> {
            Ok(Transcription {
                text: String::new(),
            })
        }
    }

    #[test]
    fn resampling_does_not_retain_a_padded_tail() {
        let service =
            LocalService::new_with_model(TranscriptionConfig::default(), Box::new(NoopModel));
        let input = AudioInput {
            samples: vec![0.25; 4_801],
            sample_rate: 48_000,
        };

        let output = service.resample_if_needed(input).expect("resample audio");

        assert_eq!(output.len(), 1_601);
    }
}
