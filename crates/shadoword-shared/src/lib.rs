use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct AudioInput {
    pub samples: Vec<f32>,
    #[serde(alias = "sample_rate_hz")]
    pub sample_rate: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum WhisperGpuKind {
    Dedicated,
    Integrated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct WhisperGpuDeviceInfo {
    pub id: i32,
    pub name: String,
    pub kind: WhisperGpuKind,
    #[specta(type = u32)]
    pub total_vram: usize,
    #[specta(type = u32)]
    pub free_vram: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transcription {
    pub text: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranscriptionOptions {
    pub language: Option<String>,
    pub translate_to_english: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub model_path: String,
    #[serde(default)]
    pub affinity: Option<ModelAffinity>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModelAffinity {
    Auto {
        gpu_device: i32,
    },
    Cpu {
        #[serde(default)]
        #[specta(type = Option<u32>)]
        threads: Option<usize>,
    },
    Gpu {
        device: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[specta(type = Option<u32>)]
        threads: Option<usize>,
    },
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
    fn transcribe(
        &self,
        input: &AudioInput,
        options: &TranscriptionOptions,
    ) -> SharedResult<Transcription>;
}
