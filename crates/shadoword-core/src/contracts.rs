use crate::config::{ExecutionTarget, WhisperAccelerator};
use crate::wav;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use shadoword_shared::{AudioInput, WhisperGpuDeviceInfo};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TranscriptResponse {
    pub text: String,
    #[specta(type = u64)]
    pub elapsed_ms: u128,
    pub engine: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum WhisperBackendCapability {
    Cpu,
    Vulkan,
    Cuda,
}

impl WhisperBackendCapability {
    pub fn label(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Vulkan => "Vulkan",
            Self::Cuda => "CUDA",
        }
    }
}

pub fn compiled_whisper_backends() -> Vec<WhisperBackendCapability> {
    let backends: &[WhisperBackendCapability] = &[
        #[cfg(feature = "local-whisper")]
        WhisperBackendCapability::Cpu,
        #[cfg(feature = "whisper-vulkan")]
        WhisperBackendCapability::Vulkan,
        #[cfg(feature = "whisper-cuda")]
        WhisperBackendCapability::Cuda,
    ];
    backends.to_vec()
}

fn default_gpu_device() -> i32 {
    -1
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ServiceStatus {
    pub model_loaded: bool,
    pub engine: String,
    pub model_path: String,
    // Legacy compatibility summary; mixed pools should inspect unit targets.
    pub whisper_accelerator: WhisperAccelerator,
    // Legacy compatibility summary; mixed pools should inspect unit targets.
    #[serde(default = "default_gpu_device")]
    pub whisper_gpu_device: i32,
    #[serde(default)]
    pub compiled_whisper_backends: Vec<WhisperBackendCapability>,
    #[serde(default)]
    pub available_gpu_devices: Vec<WhisperGpuDeviceInfo>,
    pub sample_rate: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference_pool: Option<InferencePoolStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionUnitState {
    Unloaded,
    Loading,
    Ready,
    Busy,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ExecutionUnitStatus {
    pub id: String,
    pub required: bool,
    pub target: ExecutionTarget,
    pub state: ExecutionUnitState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[specta(type = f64)]
    pub completed: u64,
    #[specta(type = f64)]
    pub failed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DrainingGenerationStatus {
    #[specta(type = f64)]
    pub generation: u64,
    #[specta(type = u32)]
    pub queued_jobs: usize,
    #[specta(type = u32)]
    pub queued_audio_bytes: usize,
    #[specta(type = u32)]
    pub running_jobs: usize,
    #[specta(type = u32)]
    pub running_audio_bytes: usize,
    #[specta(type = u32)]
    pub workers_remaining: usize,
    #[serde(default)]
    #[specta(type = u32)]
    pub loading_units: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct InferencePoolStatus {
    #[specta(type = f64)]
    pub generation: u64,
    pub units: Vec<ExecutionUnitStatus>,
    pub accepting: bool,
    #[serde(default)]
    pub draining_generations: Vec<DrainingGenerationStatus>,
    #[specta(type = u32)]
    pub ready_units: usize,
    #[specta(type = u32)]
    pub busy_units: usize,
    #[specta(type = u32)]
    pub unhealthy_units: usize,
    #[specta(type = u32)]
    pub queued_jobs: usize,
    #[specta(type = u32)]
    pub queued_audio_bytes: usize,
    #[specta(type = u32)]
    pub running_jobs: usize,
    #[specta(type = u32)]
    pub running_audio_bytes: usize,
    #[specta(type = f64)]
    pub completed: u64,
    #[specta(type = f64)]
    pub failed: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

pub trait TranscriptionService: Send + Sync {
    fn status(&self) -> Result<ServiceStatus>;
    fn transcribe_audio(&self, input: AudioInput) -> Result<TranscriptResponse>;

    fn transcribe_wav_bytes(&self, bytes: &[u8]) -> Result<TranscriptResponse> {
        let input = wav::decode_wav(bytes)?;
        self.transcribe_audio(input)
    }
}
