#[cfg(feature = "desktop-audio")]
mod audio;
mod config;
pub mod contracts;
#[cfg(feature = "local-whisper")]
mod inference;
#[cfg(feature = "local-whisper")]
mod model_download;
pub mod remote_contracts;
mod sequencer;
#[cfg(feature = "local-whisper")]
mod service;
mod vad;
pub mod wav;

#[cfg(feature = "desktop-audio")]
pub use audio::{InputDeviceInfo, MicrophoneRecorder, RecordingSession, RecordingSnapshotSource};
pub use config::{
    ApiConfig, ApiTokenConfig, ApiTokenRole, ConfigPaths, DesktopConfig, ExecutionTarget,
    ExecutionUnitConfig, HotkeyMode, InferenceLimits, InferencePoolConfig,
    ModeRecordingPreferences, OutputConfig, PasteMethod, RecordingConfig, RemoteConfig,
    ServiceMode, StreamingPcmFormat, TranscriptBoundary, TranscriptionConfig, TranscriptionMode,
    WhisperAccelerator,
};
pub use contracts::{
    compiled_whisper_backends, DrainingGenerationStatus, ExecutionUnitState, ExecutionUnitStatus,
    InferencePoolStatus, ServiceStatus, TranscriptResponse, TranscriptionService,
    WhisperBackendCapability,
};
#[cfg(feature = "local-whisper")]
pub use inference::{
    ExecutionModelFactory, ExecutionPool, InferenceCompletion, InferenceError, InferenceIdentity,
    InferenceJob, InferenceRequest, InferenceRuntime, WhisperModelFactory,
};
#[cfg(feature = "local-whisper")]
pub use model_download::{
    default_whisper_model, download_whisper_model, download_whisper_model_with_progress,
    list_whisper_models, parse_requested_models, resolve_download_dir, resolve_whisper_model,
    unknown_model_error, ModelDownloadProgress, ModelDownloadStatus, WhisperModelSpec,
};
pub use sequencer::{OrderedCompletion, SequencerError};
#[cfg(feature = "local-whisper")]
pub use service::LocalService;
#[cfg(feature = "local-whisper")]
pub use shadoword_model_whisper::list_whisper_gpu_devices;
pub use shadoword_shared::{AudioInput, ModelAffinity, WhisperGpuDeviceInfo, WhisperGpuKind};
pub use vad::{VadSegment, VadSegmenter, VadSegmenterConfig};
