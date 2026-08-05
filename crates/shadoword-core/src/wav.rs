use anyhow::{Context, Result};
use shadoword_shared::AudioInput;

pub fn encode_wav(input: &AudioInput) -> Result<Vec<u8>> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: input.sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer =
        hound::WavWriter::new(&mut cursor, spec).context("failed to start wav writer")?;
    for sample in &input.samples {
        writer
            .write_sample(*sample)
            .context("failed to write wav sample")?;
    }
    writer.finalize().context("failed to finalize wav data")?;
    Ok(cursor.into_inner())
}

pub fn decode_wav(bytes: &[u8]) -> Result<AudioInput> {
    let mut reader =
        hound::WavReader::new(std::io::Cursor::new(bytes)).context("failed to open wav bytes")?;
    let spec = reader.spec();
    let interleaved = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("failed reading float wav samples")?,
        hound::SampleFormat::Int => {
            let scale = ((1_i64 << (spec.bits_per_sample.saturating_sub(1) as u32)) - 1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| sample.map(|value| value as f32 / scale))
                .collect::<std::result::Result<Vec<_>, _>>()
                .context("failed reading integer wav samples")?
        }
    };

    let channels = usize::from(spec.channels);
    if channels == 0 {
        anyhow::bail!("WAV has no channels");
    }
    let samples = if channels == 1 {
        interleaved
    } else {
        interleaved
            .chunks_exact(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect()
    };

    Ok(AudioInput {
        samples,
        sample_rate: spec.sample_rate,
    })
}
