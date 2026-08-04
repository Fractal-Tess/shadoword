use std::path::Path;
use std::sync::Mutex;

use shadoword_shared::{
    AudioInput, LoadProgress, LoadState, Model, ModelAffinity, ModelConfig, ModelError,
    SharedResult, Transcription, TranscriptionOptions,
};
pub use shadoword_shared::{WhisperGpuDeviceInfo, WhisperGpuKind};
use transcribe_rs::accel;
use transcribe_rs::whisper_cpp::{WhisperEngine, WhisperInferenceParams, WhisperLoadParams};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WhisperAcceleration {
    #[default]
    Auto,
    Cpu,
    Gpu,
}

pub fn list_whisper_gpu_devices() -> Vec<WhisperGpuDeviceInfo> {
    transcribe_rs::whisper_cpp::gpu::list_gpu_devices()
        .into_iter()
        .map(|device| WhisperGpuDeviceInfo {
            id: device.id,
            name: device.name,
            kind: match device.kind {
                transcribe_rs::whisper_cpp::gpu::GpuKind::Dedicated => WhisperGpuKind::Dedicated,
                transcribe_rs::whisper_cpp::gpu::GpuKind::Integrated => WhisperGpuKind::Integrated,
            },
            total_vram: device.total_vram,
            free_vram: device.free_vram,
        })
        .collect()
}

pub fn apply_whisper_gpu_device(device: i32) {
    accel::set_whisper_gpu_device(device);
}

pub fn apply_whisper_acceleration(acceleration: WhisperAcceleration) {
    let whisper = match acceleration {
        WhisperAcceleration::Auto => accel::WhisperAccelerator::Auto,
        WhisperAcceleration::Cpu => accel::WhisperAccelerator::CpuOnly,
        WhisperAcceleration::Gpu => accel::WhisperAccelerator::Gpu,
    };
    accel::set_whisper_accelerator(whisper);
}

pub struct WhisperModel {
    engine: Option<Mutex<WhisperEngine>>,
    progress: Option<LoadProgress>,
    model_path: Option<String>,
    cpu_threads: Option<usize>,
}

impl WhisperModel {
    pub fn new() -> Self {
        Self {
            engine: None,
            progress: Some(LoadProgress {
                state: LoadState::Unloaded,
                fraction: 0.0,
                detail: None,
            }),
            model_path: None,
            cpu_threads: None,
        }
    }

    pub fn with_engine_mut<R>(
        &self,
        f: impl FnOnce(&mut WhisperEngine) -> SharedResult<R>,
    ) -> SharedResult<R> {
        let engine = self.engine.as_ref().ok_or_else(|| ModelError {
            message: "transcription engine not loaded".to_string(),
        })?;
        let mut guard = engine.lock().map_err(|_| ModelError {
            message: "whisper engine lock poisoned".to_string(),
        })?;
        f(&mut guard)
    }

    pub fn transcribe(&self, input: &AudioInput) -> SharedResult<Transcription> {
        <Self as Model>::transcribe(self, input, &TranscriptionOptions::default())
    }
}

impl Default for WhisperModel {
    fn default() -> Self {
        Self::new()
    }
}

impl Model for WhisperModel {
    fn name(&self) -> &'static str {
        "whisper"
    }

    fn load(&mut self, cfg: &ModelConfig) -> SharedResult<()> {
        self.progress = Some(LoadProgress {
            state: LoadState::Loading,
            fraction: 0.0,
            detail: Some(format!("loading {}", cfg.model_path)),
        });

        let model_path = Path::new(&cfg.model_path);
        if !model_path.exists() {
            self.progress = Some(LoadProgress {
                state: LoadState::Failed,
                fraction: 0.0,
                detail: Some(format!("model path does not exist: {}", cfg.model_path)),
            });
            return Err(ModelError {
                message: format!("model path does not exist: {}", cfg.model_path),
            });
        }

        let load_params = match cfg.affinity.as_ref() {
            None => None,
            Some(ModelAffinity::Auto { gpu_device }) => WhisperLoadParams {
                use_gpu: true,
                gpu_device: *gpu_device,
                ..Default::default()
            }
            .into(),
            Some(ModelAffinity::Cpu { .. }) => WhisperLoadParams {
                use_gpu: false,
                ..Default::default()
            }
            .into(),
            Some(ModelAffinity::Gpu { device, .. }) => WhisperLoadParams {
                use_gpu: true,
                gpu_device: *device,
                ..Default::default()
            }
            .into(),
        };
        let cpu_threads = match cfg.affinity.as_ref() {
            Some(ModelAffinity::Cpu { threads }) => *threads,
            Some(ModelAffinity::Gpu { threads, .. }) => *threads,
            _ => None,
        };

        let loaded = match load_params {
            Some(load_params) => WhisperEngine::load_with_params(model_path, load_params),
            None => WhisperEngine::load(model_path),
        }
        .map_err(|error| {
            self.progress = Some(LoadProgress {
                state: LoadState::Failed,
                fraction: 0.0,
                detail: Some(error.to_string()),
            });
            ModelError {
                message: format!(
                    "failed to load whisper model from {}: {error}",
                    cfg.model_path
                ),
            }
        })?;

        self.engine = Some(Mutex::new(loaded));
        self.cpu_threads = cpu_threads;
        self.model_path = Some(cfg.model_path.clone());
        self.progress = Some(LoadProgress {
            state: LoadState::Loaded,
            fraction: 1.0,
            detail: self.model_path.clone(),
        });
        Ok(())
    }

    fn unload(&mut self) -> SharedResult<()> {
        self.engine = None;
        self.cpu_threads = None;
        self.progress = Some(LoadProgress {
            state: LoadState::Unloaded,
            fraction: 0.0,
            detail: None,
        });
        Ok(())
    }

    fn is_loaded(&self) -> bool {
        self.engine.is_some()
    }

    fn load_progress(&self) -> Option<LoadProgress> {
        self.progress.clone()
    }

    fn transcribe(
        &self,
        input: &AudioInput,
        options: &TranscriptionOptions,
    ) -> SharedResult<Transcription> {
        let result = self.with_engine_mut(|engine| {
            engine
                .transcribe_with(
                    &input.samples,
                    &WhisperInferenceParams {
                        language: options.language.clone(),
                        translate: options.translate_to_english,
                        n_threads: self
                            .cpu_threads
                            .and_then(|threads| i32::try_from(threads).ok())
                            .unwrap_or(0),
                        ..Default::default()
                    },
                )
                .map_err(|error| ModelError {
                    message: format!("whisper transcription failed: {error}"),
                })
        })?;

        Ok(Transcription { text: result.text })
    }
}
