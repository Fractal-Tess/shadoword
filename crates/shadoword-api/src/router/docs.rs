use super::*;

#[derive(Debug, Serialize)]
pub(super) struct DaemonDocs {
    endpoints: Vec<DocEndpoint>,
    env: Vec<DocEnvVar>,
    limits: DocsLimits,
    whisper_models: Vec<DocModel>,
}

#[derive(Debug, Serialize)]
struct DocEndpoint {
    method: &'static str,
    path: &'static str,
    description: &'static str,
}

#[derive(Debug, Serialize)]
struct DocEnvVar {
    name: &'static str,
    description: &'static str,
    example: &'static str,
}

#[derive(Debug, Serialize)]
struct DocModel {
    id: &'static str,
    filename: &'static str,
    description: &'static str,
    size_bytes: u64,
    recommended: bool,
}

#[derive(Debug, Serialize)]
struct DocsLimits {
    raw_wav_bytes: usize,
    default_decoded_audio_bytes_per_job: usize,
    opus_packet_bytes: usize,
    raw_pcm_max_packet_bytes: usize,
    raw_pcm_max_packet_milliseconds: usize,
    pcm_f32le_wire_bytes_per_sample: usize,
    pcm_s16le_wire_bytes_per_sample: usize,
    decoded_pcm_bytes_per_sample: usize,
    stream_segment_seconds: usize,
    stream_max_segments: usize,
    stream_idle_seconds: u64,
}
pub(super) async fn docs() -> Json<DaemonDocs> {
    Json(DaemonDocs {
    endpoints: vec![
        DocEndpoint {
            method: "GET",
            path: "/health",
            description: "Public health check.",
        },
        DocEndpoint {
            method: "GET",
            path: "/v1/version",
            description: "Public daemon version, for clients checking which endpoints they can expect.",
        },
        DocEndpoint {
            method: "GET",
            path: "/v1/tokens",
            description: "Admin-only list of token names and roles. Token hashes are never returned.",
        },
        DocEndpoint {
            method: "POST",
            path: "/v1/tokens",
            description: "Admin-only token issue. Returns the secret exactly once; it is stored only as a SHA-256 hash and takes effect immediately.",
        },
        DocEndpoint {
            method: "DELETE",
            path: "/v1/tokens/{name}",
            description: "Admin-only token revoke, effective immediately. The last remaining admin token cannot be revoked.",
        },
        DocEndpoint {
            method: "GET",
            path: "/v1/status",
            description: "Admin-only daemon and inference-pool status, including generation, units, queued/running work, bytes, and counters.",
        },
        DocEndpoint {
            method: "GET",
            path: "/v1/overview",
            description: "Admin-only combined runtime, status, and model catalog snapshot.",
        },
        DocEndpoint {
            method: "GET",
            path: "/v1/config",
            description: "Admin-only runtime config with effective inference_pool and generation. Units preload eagerly only when preload_on_startup is true; otherwise they load on first dispatch.",
        },
        DocEndpoint {
            method: "PUT",
            path: "/v1/config",
            description: "Admin-only atomic runtime update. Accepts inference_pool and an optional generation revision; eager candidates prepare before commit and lazy candidates activate unloaded.",
        },
        DocEndpoint {
            method: "POST",
            path: "/v1/transcribe-wav",
            description: "Raw WAV batch transcription for admin or user tokens.",
        },
        DocEndpoint {
            method: "GET",
            path: "/v1/stream",
            description: "WebSocket transcription for admin or user tokens. Protocol v1 remains lockstep; v2 negotiates indexed concurrent Opus segments. For v3, Start includes protocol_version=3, audio_format=pcm_f32le or pcm_s16le, sample_rate, and channels=1; binary messages contain little-endian mono PCM and the server performs VAD, segmentation, bounded scheduling, and ordered partial delivery.",
        },
        DocEndpoint {
            method: "GET",
            path: "/v1/models",
            description: "Admin-only catalog and installation status.",
        },
        DocEndpoint {
            method: "DELETE",
            path: "/v1/models/{id}",
            description: "Admin-only deletion of an installed, inactive catalog model.",
        },
        DocEndpoint {
            method: "POST",
            path: "/v1/models/{id}/select",
            description: "Admin-only selection of an installed catalog model.",
        },
        DocEndpoint {
            method: "POST",
            path: "/v1/downloads",
            description: "Admin-only async catalog model download job.",
        },
        DocEndpoint {
            method: "GET",
            path: "/v1/downloads/{id}",
            description: "Admin-only model download job status.",
        },
    ],
    env: vec![
        DocEnvVar {
            name: "SHADOWORD_API_CONFIG",
            description: "API config path.",
            example: "/config/shadoword/api.json",
        },
        DocEnvVar {
            name: "SHADOWORD_LISTEN_ADDR",
            description: "API bind address. Non-loopback binds require bearer auth.",
            example: "0.0.0.0:47813",
        },
        DocEnvVar {
            name: "SHADOWORD_MODEL_PATH",
            description: "Selected Whisper model file.",
            example: "/data/shadoword/models/ggml-large-v3-turbo.bin",
        },
        DocEnvVar {
            name: "SHADOWORD_DOWNLOAD_MODELS",
            description: "Comma-separated catalog model ids to download at startup.",
            example: "turbo",
        },
        DocEnvVar {
            name: "SHADOWORD_QUEUE_CAPACITY",
            description: "Legacy queue limit. When inference_pool is absent, maps to pool max_queued_jobs (zero uses one worker hand-off slot).",
            example: "4",
        },
        DocEnvVar {
            name: "SHADOWORD_REQUEST_RECORDING_DIR",
            description: "Optional directory for WAV copies of accepted transcription requests and JSON response metadata.",
            example: "/var/lib/shadoword/requests",
        },
    ],
    limits: DocsLimits {
        raw_wav_bytes: MAX_RAW_WAV_BYTES,
        default_decoded_audio_bytes_per_job: shadoword_core::InferenceLimits::default()
            .max_audio_bytes_per_job,
        opus_packet_bytes: stream::MAX_OPUS_PACKET_BYTES,
        raw_pcm_max_packet_bytes: stream::MAX_OPUS_PACKET_BYTES,
        raw_pcm_max_packet_milliseconds: stream::MAX_PCM_PACKET_MILLISECONDS,
        pcm_f32le_wire_bytes_per_sample: stream::PCM_F32LE_WIRE_BYTES_PER_SAMPLE,
        pcm_s16le_wire_bytes_per_sample: stream::PCM_S16LE_WIRE_BYTES_PER_SAMPLE,
        decoded_pcm_bytes_per_sample: stream::DECODED_PCM_BYTES_PER_SAMPLE,
        stream_segment_seconds: stream::MAX_SEGMENT_SECONDS,
        stream_max_segments: stream::MAX_STREAM_SEGMENTS,
        stream_idle_seconds: stream::STREAM_IDLE_TIMEOUT.as_secs(),
    },
    whisper_models: list_whisper_models()
        .iter()
        .map(|model| DocModel {
            id: model.id,
            filename: model.filename,
            description: model.description,
            size_bytes: model.size_bytes,
            recommended: model.recommended,
        })
        .collect(),
})
}
