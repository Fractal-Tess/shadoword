use std::path::Path;
use std::sync::Mutex;

use shadoword_shared::{
    AudioInput, LoadProgress, LoadState, Model, ModelConfig, ModelError, SharedResult,
    Transcription,
};
use transcribe_rs::whisper_cpp::{WhisperEngine, WhisperInferenceParams};

pub struct WhisperModel {
    engine: Option<Mutex<WhisperEngine>>,
    progress: Option<LoadProgress>,
    model_path: Option<String>,
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

        let loaded = WhisperEngine::load(model_path).map_err(|error| {
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

    fn transcribe(&self, input: &AudioInput) -> SharedResult<Transcription> {
        let result = self.with_engine_mut(|engine| {
            engine
                .transcribe_with(
                    &input.samples,
                    &WhisperInferenceParams {
                        language: None,
                        translate: false,
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
