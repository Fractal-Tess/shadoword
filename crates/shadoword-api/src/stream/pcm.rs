use super::*;

pub(super) fn validate_start(start: &StartMessage) -> Result<(), String> {
    let sample_rate_valid = match start.audio_format {
        StreamAudioFormat::Opus => {
            matches!(start.sample_rate, 8_000 | 12_000 | 16_000 | 24_000 | 48_000)
        }
        StreamAudioFormat::PcmF32le | StreamAudioFormat::PcmS16le => {
            (8_000..=384_000).contains(&start.sample_rate)
        }
    };
    if !sample_rate_valid {
        return Err("Start sample_rate is unsupported for the selected audio_format".to_string());
    }
    if start.channels != 1 {
        return Err("Start channels must be 1 (mono)".to_string());
    }
    match (start.protocol_version, start.audio_format) {
        (PROTOCOL_V1 | PROTOCOL_V2, StreamAudioFormat::Opus)
        | (PROTOCOL_V3, StreamAudioFormat::PcmF32le | StreamAudioFormat::PcmS16le) => Ok(()),
        (PROTOCOL_V1 | PROTOCOL_V2, StreamAudioFormat::PcmF32le | StreamAudioFormat::PcmS16le) => {
            Err("raw PCM streaming requires protocol_version 3".to_string())
        }
        (PROTOCOL_V3, StreamAudioFormat::Opus) => {
            Err("protocol_version 3 requires audio_format pcm_f32le or pcm_s16le".to_string())
        }
        _ => Err("Start protocol_version must be 1, 2, or 3".to_string()),
    }
}

pub(super) fn pcm_audio_input(
    source_rate: u32,
    target_rate: u32,
    max_audio_bytes: usize,
    wire_format: PcmWireFormat,
) -> StreamAudioInput {
    StreamAudioInput::Pcm {
        wire_format,
        segmenter: VadSegmenter::new(source_rate, VadSegmenterConfig::default()),
        samples_since_segment: 0,
        max_segment_samples: max_vad_segment_samples(
            max_audio_bytes,
            source_rate,
            target_rate,
            wire_format,
        ),
    }
}

pub(super) fn max_pcm_packet_bytes(sample_rate: u32, wire_format: PcmWireFormat) -> usize {
    (sample_rate as usize)
        .saturating_mul(MAX_PCM_PACKET_MILLISECONDS)
        .saturating_div(1_000)
        .saturating_mul(wire_format.bytes_per_sample())
        .min(MAX_OPUS_PACKET_BYTES)
}

pub(super) fn max_pcm_packet_samples(sample_rate: u32, wire_format: PcmWireFormat) -> usize {
    max_pcm_packet_bytes(sample_rate, wire_format) / wire_format.bytes_per_sample()
}

pub(super) fn max_vad_segment_samples(
    max_audio_bytes: usize,
    source_rate: u32,
    target_rate: u32,
    wire_format: PcmWireFormat,
) -> usize {
    let duration_limit = (source_rate as usize).saturating_mul(MAX_VAD_SEGMENT_SECONDS);
    duration_limit
        .min(max_input_samples_for_inference(
            max_audio_bytes,
            source_rate,
            target_rate,
        ))
        .saturating_sub(max_pcm_packet_samples(source_rate, wire_format))
}

pub(super) fn estimated_inference_audio_bytes(
    samples: usize,
    source_rate: u32,
    target_rate: u32,
) -> Option<usize> {
    let raw_bytes = samples.checked_mul(DECODED_PCM_BYTES_PER_SAMPLE)?;
    if source_rate == target_rate {
        return Some(raw_bytes);
    }
    if source_rate == 0 {
        return None;
    }
    let resampled_samples = (samples as u128)
        .checked_mul(u128::from(target_rate))?
        .div_ceil(u128::from(source_rate));
    let resampled_bytes = usize::try_from(resampled_samples)
        .ok()?
        .checked_mul(DECODED_PCM_BYTES_PER_SAMPLE)?;
    raw_bytes.checked_add(resampled_bytes)
}

pub(super) fn max_input_samples_for_inference(
    max_audio_bytes: usize,
    source_rate: u32,
    target_rate: u32,
) -> usize {
    let mut lower = 0_usize;
    let mut upper = (max_audio_bytes / DECODED_PCM_BYTES_PER_SAMPLE).saturating_add(1);
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let fits = estimated_inference_audio_bytes(middle, source_rate, target_rate)
            .is_some_and(|bytes| bytes <= max_audio_bytes);
        if fits {
            lower = middle + 1;
        } else {
            upper = middle;
        }
    }
    lower.saturating_sub(1)
}

pub(super) fn decode_pcm(wire_format: PcmWireFormat, packet: &[u8]) -> Result<Vec<f32>, ApiError> {
    match wire_format {
        PcmWireFormat::F32Le => decode_pcm_f32le(packet),
        PcmWireFormat::S16Le => decode_pcm_s16le(packet),
    }
}

pub(super) fn decode_pcm_f32le(packet: &[u8]) -> Result<Vec<f32>, ApiError> {
    if !packet.len().is_multiple_of(PCM_F32LE_WIRE_BYTES_PER_SAMPLE) {
        return Err(ApiError::bad_request(
            "pcm_f32le packet length must be divisible by 4",
        ));
    }
    packet
        .chunks_exact(PCM_F32LE_WIRE_BYTES_PER_SAMPLE)
        .map(|bytes| {
            let sample = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            if !sample.is_finite() {
                return Err(ApiError::bad_request("pcm_f32le samples must be finite"));
            }
            if !(-1.0..=1.0).contains(&sample) {
                return Err(ApiError::bad_request(
                    "pcm_f32le samples must be in the range [-1.0, 1.0]",
                ));
            }
            Ok(sample)
        })
        .collect()
}

pub(super) fn decode_pcm_s16le(packet: &[u8]) -> Result<Vec<f32>, ApiError> {
    if !packet.len().is_multiple_of(PCM_S16LE_WIRE_BYTES_PER_SAMPLE) {
        return Err(ApiError::bad_request(
            "pcm_s16le packet length must be divisible by 2",
        ));
    }
    Ok(packet
        .chunks_exact(PCM_S16LE_WIRE_BYTES_PER_SAMPLE)
        .map(|bytes| f32::from(i16::from_le_bytes([bytes[0], bytes[1]])) / 32_768.0)
        .collect())
}

pub(super) fn downmix_to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels == 1 {
        return samples.to_vec();
    }

    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

pub(super) fn default_sample_rate() -> u32 {
    STREAM_SAMPLE_RATE
}

pub(super) fn default_channels() -> usize {
    1
}

pub(super) fn default_protocol_version() -> u8 {
    PROTOCOL_V1
}
