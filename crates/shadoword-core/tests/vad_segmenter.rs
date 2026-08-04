use std::path::Path;

use shadoword_core::{wav, VadSegmenter, VadSegmenterConfig};

#[test]
fn vad_segments_bench_corpus_clip_when_available() {
    let corpus_clip = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("bench_corpus")
        .join("clip_10s.wav");

    if !corpus_clip.exists() {
        eprintln!(
            "skipping vad corpus test: missing {}",
            corpus_clip.display()
        );
        return;
    }

    let bytes = std::fs::read(&corpus_clip).expect("failed to read corpus wav");
    let audio = wav::decode_wav(&bytes).expect("failed to decode corpus wav");
    let mut segmenter = VadSegmenter::new(audio.sample_rate, VadSegmenterConfig::default());

    let mut segments = segmenter.push_samples(&audio.samples);
    if let Some(segment) = segmenter.finish() {
        segments.push(segment);
    }

    eprintln!(
        "vad corpus segments: {} for {} samples @ {}Hz",
        segments.len(),
        audio.samples.len(),
        audio.sample_rate
    );
    assert!(
        !segments.is_empty(),
        "vad should find at least one speech segment"
    );
    assert!(
        segments
            .iter()
            .all(|segment| !segment.audio.samples.is_empty()),
        "vad emitted an empty segment"
    );
}
