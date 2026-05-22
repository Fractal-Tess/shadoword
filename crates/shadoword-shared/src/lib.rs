use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioInput {
    pub samples: Vec<f32>,
    pub sample_rate_hz: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transcription {
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub model_path: String,
    pub language: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LoadState {
    Unloaded,
    Loading,
    Loaded,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoadProgress {
    pub state: LoadState,
    pub fraction: f32,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelError {
    pub message: String,
}

pub type SharedResult<T> = Result<T, ModelError>;

pub trait Model: Send + Sync {
    fn name(&self) -> &'static str;
    fn load(&mut self, cfg: &ModelConfig) -> SharedResult<()>;
    fn unload(&mut self) -> SharedResult<()>;
    fn is_loaded(&self) -> bool;
    fn load_progress(&self) -> Option<LoadProgress>;
    fn transcribe(&self, input: &AudioInput) -> SharedResult<Transcription>;
}
