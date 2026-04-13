use crate::audio::{AudioInput, InputDeviceInfo, MicrophoneRecorder};
use crate::config::{ServiceMode, ShadowwordConfig, WhisperAccelerator};
use crate::wav;
use anyhow::{anyhow, Context, Result};
use base64::Engine;
use rubato::{FftFixedIn, Resampler};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;
use transcribe_rs::accel;
use transcribe_rs::whisper_cpp::{WhisperEngine, WhisperInferenceParams};
use transcribe_rs::TranscribeOptions;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptRequest {
    pub wav_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptResponse {
    pub text: String,
    pub elapsed_ms: u128,
    pub engine: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub model_loaded: bool,
    pub mode: ServiceMode,
    pub engine: String,
    pub model_path: String,
    pub whisper_accelerator: WhisperAccelerator,
    pub input_device: Option<String>,
    pub sample_rate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceListResponse {
    pub inputs: Vec<InputDeviceInfo>,
}

#[derive(Debug, Clone, Default)]
struct ProfileTimings {
    request_bytes: usize,
    input_samples: usize,
    output_samples: usize,
    sample_rate: u32,
    decoded_base64_ms: u128,
    decoded_wav_ms: u128,
    ensure_loaded_ms: u128,
    resample_ms: u128,
    inference_ms: u128,
    total_ms: u128,
    cold_load: bool,
}

pub trait TranscriptionService: Send + Sync {
    fn status(&self) -> Result<ServiceStatus>;
    fn transcribe_audio(&self, input: AudioInput) -> Result<TranscriptResponse>;

    fn transcribe_wav_bytes(&self, bytes: &[u8]) -> Result<TranscriptResponse> {
        let input = wav::decode_wav(bytes)?;
        self.transcribe_audio(input)
    }

    fn transcribe_wav_base64(&self, request: TranscriptRequest) -> Result<TranscriptResponse> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(request.wav_base64)
            .context("failed to decode base64 audio payload")?;
        let input = wav::decode_wav(&bytes)?;
        self.transcribe_audio(input)
    }
}

pub struct LocalService {
    config: Arc<RwLock<ShadowwordConfig>>,
    engine: Arc<Mutex<Option<WhisperEngine>>>,
    model_loaded: Arc<AtomicBool>,
}

impl LocalService {
    fn compiled_backend_summary() -> &'static str {
        if cfg!(feature = "whisper-vulkan") {
            "whisper-vulkan"
        } else {
            "cpu-only"
        }
    }

    fn log_backend_request(config: &ShadowwordConfig, phase: &str) {
        tracing::info!(
            target: "shadowword.backend",
            phase,
            engine = "whisper",
            model_path = %config.model_path.display(),
            whisper_accelerator = ?config.whisper_accelerator,
            compiled_backends = Self::compiled_backend_summary(),
            "backend configuration"
        );
    }

    pub fn new(config: ShadowwordConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            engine: Arc::new(Mutex::new(None)),
            model_loaded: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn config(&self) -> ShadowwordConfig {
        self.config.read().expect("config lock poisoned").clone()
    }

    pub fn update_config(&self, config: ShadowwordConfig) -> Result<()> {
        config.save()?;
        *self.config.write().expect("config lock poisoned") = config;
        *self.engine.lock().expect("engine lock poisoned") = None;
        self.model_loaded.store(false, Ordering::Relaxed);
        Ok(())
    }

    pub fn list_input_devices(&self) -> Result<DeviceListResponse> {
        Ok(DeviceListResponse {
            inputs: MicrophoneRecorder::list_input_devices()?,
        })
    }

    fn apply_accelerator(config: &ShadowwordConfig) {
        let whisper = match config.whisper_accelerator {
            WhisperAccelerator::Auto => accel::WhisperAccelerator::Auto,
            WhisperAccelerator::Cpu => accel::WhisperAccelerator::CpuOnly,
            WhisperAccelerator::Gpu => accel::WhisperAccelerator::Gpu,
        };
        accel::set_whisper_accelerator(whisper);
    }

    fn profiling_enabled() -> bool {
        std::env::var_os("SHADOWWORD_PROFILE").is_some()
    }

    fn log_profile(&self, config: &ShadowwordConfig, timings: &ProfileTimings) {
        if !Self::profiling_enabled() {
            return;
        }

        tracing::info!(
            target: "shadowword.profile",
            engine = "whisper",
            whisper = ?config.whisper_accelerator,
            sample_rate = timings.sample_rate,
            request_bytes = timings.request_bytes,
            input_samples = timings.input_samples,
            output_samples = timings.output_samples,
            cold_load = timings.cold_load,
            decode_base64_ms = timings.decoded_base64_ms,
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
        self.model_loaded.load(Ordering::Relaxed)
    }

    fn ensure_loaded(&self) -> Result<bool> {
        let config = self.config();
        Self::log_backend_request(&config, "ensure_loaded");
        Self::apply_accelerator(&config);

        let mut engine_guard = self.engine.lock().expect("engine lock poisoned");
        if engine_guard.is_some() {
            tracing::info!(
                target: "shadowword.backend",
                engine = "whisper",
                model_path = %config.model_path.display(),
                compiled_backends = Self::compiled_backend_summary(),
                "model already loaded"
            );
            return Ok(false);
        }

        if !config.model_path.exists() {
            return Err(anyhow!(
                "model path does not exist: {}",
                config.model_path.display()
            ));
        }

        let loaded = WhisperEngine::load(Path::new(&config.model_path)).with_context(|| {
            format!(
                "failed to load whisper model from {}",
                config.model_path.display()
            )
        })?;

        *engine_guard = Some(loaded);
        self.model_loaded.store(true, Ordering::Relaxed);
        tracing::info!(
            target: "shadowword.backend",
            engine = "whisper",
            model_path = %config.model_path.display(),
            compiled_backends = Self::compiled_backend_summary(),
            "model load complete"
        );
        Ok(true)
    }

    fn resample_if_needed(&self, input: AudioInput) -> Result<Vec<f32>> {
        let target_rate = self.config().recording.sample_rate as usize;
        if input.sample_rate as usize == target_rate {
            return Ok(input.samples);
        }

        let chunk_size = 1024;
        let mut resampler = FftFixedIn::<f32>::new(
            input.sample_rate as usize,
            target_rate,
            chunk_size,
            1,
            1,
        )
        .context("failed to initialize resampler")?;

        let mut output = Vec::new();
        for chunk in input.samples.chunks(chunk_size) {
            let mut owned = chunk.to_vec();
            if owned.len() < chunk_size {
                owned.resize(chunk_size, 0.0);
            }
            let processed = resampler
                .process(&[owned], None)
                .context("failed to resample audio")?;
            output.extend_from_slice(&processed[0]);
        }
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

        let config = self.config();
        Self::log_backend_request(&config, "transcribe");
        let inference_start = Instant::now();

        let text = {
            let mut engine_guard = self.engine.lock().expect("engine lock poisoned");
            let engine = engine_guard
                .as_mut()
                .context("transcription engine not loaded")?;

            engine
                .transcribe_with(
                    &audio,
                    &WhisperInferenceParams {
                        language: None,
                        translate: false,
                        ..Default::default()
                    },
                )
                .context("whisper transcription failed")?
                .text
        };

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
            engine: "whisper".to_string(),
        })
    }
}

impl TranscriptionService for LocalService {
    fn status(&self) -> Result<ServiceStatus> {
        let config = self.config();
        let loaded = self.model_loaded.load(Ordering::Relaxed);
        Ok(ServiceStatus {
            model_loaded: loaded,
            mode: config.mode,
            engine: "whisper".to_string(),
            model_path: config.model_path.display().to_string(),
            whisper_accelerator: config.whisper_accelerator,
            input_device: config.recording.input_device.clone(),
            sample_rate: config.recording.sample_rate,
        })
    }

    fn transcribe_wav_base64(&self, request: TranscriptRequest) -> Result<TranscriptResponse> {
        let mut timings = ProfileTimings {
            request_bytes: request.wav_base64.len(),
            ..Default::default()
        };

        let decode_base64_start = Instant::now();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(request.wav_base64)
            .context("failed to decode base64 audio payload")?;
        timings.decoded_base64_ms = decode_base64_start.elapsed().as_millis();

        let decode_wav_start = Instant::now();
        let input = wav::decode_wav(&bytes)?;
        timings.decoded_wav_ms = decode_wav_start.elapsed().as_millis();

        let response = self.transcribe_audio_internal(input, &mut timings)?;
        let config = self.config();
        self.log_profile(&config, &timings);
        Ok(response)
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
        let config = self.config();
        self.log_profile(&config, &timings);
        Ok(response)
    }

    fn transcribe_audio(&self, input: AudioInput) -> Result<TranscriptResponse> {
        let mut timings = ProfileTimings::default();
        let response = self.transcribe_audio_internal(input, &mut timings)?;
        let config = self.config();
        self.log_profile(&config, &timings);
        Ok(response)
    }
}

impl Clone for LocalService {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            engine: Arc::clone(&self.engine),
            model_loaded: Arc::clone(&self.model_loaded),
        }
    }
}

#[allow(dead_code)]
fn _keep_transcribe_options_linked() -> TranscribeOptions {
    TranscribeOptions::default()
}
