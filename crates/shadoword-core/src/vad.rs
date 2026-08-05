use shadoword_shared::AudioInput;

#[derive(Debug, Clone, Copy)]
pub struct VadSegmenterConfig {
    pub frame_ms: u32,
    pub silence_ms: u32,
    pub pre_speech_ms: u32,
    pub min_speech_ms: u32,
    pub rms_threshold: f32,
}

impl Default for VadSegmenterConfig {
    fn default() -> Self {
        Self {
            frame_ms: 30,
            silence_ms: 650,
            pre_speech_ms: 250,
            // Short words commonly contain only 200–500 ms of frames above the
            // energy threshold. A 600 ms minimum silently discarded words such
            // as "hello" whenever a pause caused an automatic segment commit.
            min_speech_ms: 150,
            rms_threshold: 0.012,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VadSegment {
    pub audio: AudioInput,
    pub speech_ms: u32,
    pub trailing_silence_ms: u32,
}

pub struct VadSegmenter {
    config: VadSegmenterConfig,
    sample_rate: u32,
    frame_samples: usize,
    silence_frames: usize,
    pre_speech_frames: usize,
    min_speech_samples: usize,
    pending: Vec<f32>,
    pre_speech: Vec<f32>,
    in_speech: bool,
    silence_samples: usize,
    voiced_samples: usize,
    buffered: Vec<f32>,
}

impl VadSegmenter {
    pub fn new(sample_rate: u32, config: VadSegmenterConfig) -> Self {
        let frame_samples = samples_for_ms(sample_rate, config.frame_ms).max(1);
        Self {
            config,
            sample_rate,
            frame_samples,
            silence_frames: frames_for_ms(config.silence_ms, config.frame_ms),
            pre_speech_frames: frames_for_ms(config.pre_speech_ms, config.frame_ms),
            min_speech_samples: samples_for_ms(sample_rate, config.min_speech_ms),
            pending: Vec::new(),
            pre_speech: Vec::new(),
            in_speech: false,
            silence_samples: 0,
            voiced_samples: 0,
            buffered: Vec::new(),
        }
    }

    pub fn push_samples(&mut self, samples: &[f32]) -> Vec<VadSegment> {
        self.pending.extend_from_slice(samples);
        let mut segments = Vec::new();

        while self.pending.len() >= self.frame_samples {
            let frame = self.pending[..self.frame_samples].to_vec();
            self.pending.drain(..self.frame_samples);
            if let Some(segment) = self.push_frame(&frame) {
                segments.push(segment);
            }
        }

        segments
    }

    pub fn finish(&mut self) -> Option<VadSegment> {
        if !self.pending.is_empty() {
            let frame = std::mem::take(&mut self.pending);
            if let Some(segment) = self.push_frame(&frame) {
                return Some(segment);
            }
        }

        if self.voiced_samples >= self.min_speech_samples {
            return Some(self.drain_segment());
        }

        self.reset_segment();
        None
    }

    /// Flush any remaining audio as a segment, regardless of minimum speech
    /// duration. Use this when the user explicitly ends a recording — the intent
    /// is clear, so even short utterances should be transcribed.
    pub fn force_finish(&mut self) -> Option<VadSegment> {
        if !self.pending.is_empty() {
            let frame = std::mem::take(&mut self.pending);
            if let Some(segment) = self.push_frame(&frame) {
                return Some(segment);
            }
        }

        if self.voiced_samples > 0 {
            return Some(self.drain_segment());
        }

        self.reset_segment();
        None
    }

    fn push_frame(&mut self, frame: &[f32]) -> Option<VadSegment> {
        let speech = rms(frame) >= self.config.rms_threshold;

        if speech {
            if !self.in_speech {
                self.in_speech = true;
                self.buffered.extend_from_slice(&self.pre_speech);
                self.pre_speech.clear();
                tracing::info!(
                    target: "shadoword.streaming",
                    sample_rate = self.sample_rate,
                    "vad speech start"
                );
            }
            self.silence_samples = 0;
            self.voiced_samples += frame.len();
            self.buffered.extend_from_slice(frame);
        } else if self.in_speech {
            self.silence_samples += frame.len();
            self.buffered.extend_from_slice(frame);
        } else {
            self.pre_speech.extend_from_slice(frame);
            let max_pre_speech = self.pre_speech_frames * self.frame_samples;
            if self.pre_speech.len() > max_pre_speech {
                let trim = self.pre_speech.len() - max_pre_speech;
                self.pre_speech.drain(..trim);
            }
        }

        let silence_limit = self.silence_frames * self.frame_samples;
        if self.in_speech && self.silence_samples >= silence_limit {
            if self.voiced_samples >= self.min_speech_samples {
                return Some(self.drain_segment());
            }
            tracing::info!(
                target: "shadoword.streaming",
                voiced_ms = ms_for_samples(self.sample_rate, self.voiced_samples),
                samples = self.buffered.len(),
                "vad candidate discarded below minimum speech duration"
            );
            self.reset_segment();
        }

        None
    }

    fn drain_segment(&mut self) -> VadSegment {
        let samples = std::mem::take(&mut self.buffered);
        let trailing_silence_ms = ms_for_samples(self.sample_rate, self.silence_samples);
        let speech_ms = ms_for_samples(self.sample_rate, self.voiced_samples);
        tracing::info!(
            target: "shadoword.streaming",
            speech_ms,
            trailing_silence_ms,
            samples = samples.len(),
            "vad segment ready"
        );
        self.reset_segment();
        VadSegment {
            audio: AudioInput {
                samples,
                sample_rate: self.sample_rate,
            },
            speech_ms,
            trailing_silence_ms,
        }
    }

    fn reset_segment(&mut self) {
        self.in_speech = false;
        self.silence_samples = 0;
        self.voiced_samples = 0;
        self.buffered.clear();
        self.pre_speech.clear();
    }
}

fn samples_for_ms(sample_rate: u32, ms: u32) -> usize {
    ((sample_rate as u64 * ms as u64) / 1000) as usize
}

fn frames_for_ms(ms: u32, frame_ms: u32) -> usize {
    ms.div_ceil(frame_ms).max(1) as usize
}

fn ms_for_samples(sample_rate: u32, samples: usize) -> u32 {
    ((samples as u64 * 1000) / sample_rate as u64) as u32
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let energy = samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32;
    energy.sqrt()
}
