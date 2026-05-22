use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use shadoword_model_whisper::WhisperModel;
use shadoword_shared::{AudioInput, Model, ModelConfig};
use transcribe_rs::accel;

struct Clip {
    name: &'static str,
    wav: &'static str,
    reference: &'static str,
}

struct ClipRun {
    name: String,
    audio_secs: f64,
    infer_secs: f64,
    realtime_x: f64,
    wer: f64,
}

fn clips() -> [Clip; 4] {
    [
        Clip {
            name: "clip_10s",
            wav: "clip_10s.wav",
            reference: "clip_10s_ref.txt",
        },
        Clip {
            name: "clip_15s",
            wav: "clip_15s.wav",
            reference: "clip_15s_ref.txt",
        },
        Clip {
            name: "clip_20s",
            wav: "clip_20s.wav",
            reference: "clip_20s_ref.txt",
        },
        Clip {
            name: "clip_30s",
            wav: "clip_30s.wav",
            reference: "clip_30s_ref.txt",
        },
    ]
}

fn corpus_dir() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .to_path_buf();

    let bench_corpus = root.join("bench_corpus");
    if bench_corpus.exists() {
        return bench_corpus;
    }

    root.join("test_corpus")
}

fn default_model_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home)
        .join(".local")
        .join("share")
        .join("shadowword")
        .join("models")
        .join("ggml-large-v3-turbo.bin")
}

fn model_path() -> PathBuf {
    if let Ok(path) = std::env::var("SHADOWORD_WHISPER_MODEL_PATH") {
        return PathBuf::from(path);
    }
    default_model_path()
}

fn report_path() -> PathBuf {
    if let Ok(path) = std::env::var("SHADOWORD_WHISPER_REPORT_PATH") {
        return PathBuf::from(path);
    }

    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bench-results")
        .join(format!("whisper-integration-report-{}.md", backend_name()))
}

fn backend_name() -> &'static str {
    if cfg!(feature = "whisper-cuda") {
        "cuda"
    } else if cfg!(feature = "whisper-vulkan") {
        "vulkan"
    } else {
        "cpu"
    }
}

fn whisper_gpu_device_index() -> i32 {
    std::env::var("SHADOWORD_WHISPER_GPU_DEVICE")
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0)
}

fn detect_gpu_name(device_index: i32) -> Option<String> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name",
            "--format=csv,noheader,nounits",
            "-i",
            &device_index.to_string(),
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn backend_hardware_label() -> String {
    let backend = backend_name();
    if backend == "cpu" {
        return "CPU".to_string();
    }

    let device_index = whisper_gpu_device_index();
    if let Some(gpu_name) = detect_gpu_name(device_index) {
        format!("{} (device {}, backend={})", gpu_name, device_index, backend)
    } else {
        format!("GPU device {} (backend={})", device_index, backend)
    }
}

fn write_markdown_report(
    path: &Path,
    model_path: &Path,
    hardware: &str,
    runs: &[ClipRun],
    avg_wer: f64,
    total_audio_secs: f64,
    total_infer_secs: f64,
    realtime_x: f64,
) {
    let parent = path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .unwrap_or_else(|err| panic!("failed to create report dir {}: {err}", parent.display()));

    let mut markdown = String::new();
    markdown.push_str("# Whisper Integration Test Report\n\n");
    markdown.push_str(&format!("- Backend: `{}`\n", backend_name()));
    markdown.push_str(&format!("- Hardware: `{}`\n", hardware));
    markdown.push_str(&format!("- Model: `{}`\n", model_path.display()));
    markdown.push_str(&format!("- Clips processed: `{}`\n", runs.len()));
    markdown.push_str(&format!("- Total audio seconds: `{:.2}`\n", total_audio_secs));
    markdown.push_str(&format!("- Total inference seconds: `{:.2}`\n", total_infer_secs));
    markdown.push_str(&format!("- Aggregate realtime factor: `{:.2}x`\n", realtime_x));
    markdown.push_str(&format!("- Average WER: `{:.2}%`\n\n", avg_wer * 100.0));

    markdown.push_str("## Per-clip results\n\n");
    markdown.push_str("| Clip | Audio (s) | Inference (s) | Realtime (x) | WER (%) |\n");
    markdown.push_str("|---|---:|---:|---:|---:|\n");
    for run in runs {
        markdown.push_str(&format!(
            "| {} | {:.2} | {:.2} | {:.2} | {:.2}% |\n",
            run.name,
            run.audio_secs,
            run.infer_secs,
            run.realtime_x,
            run.wer * 100.0
        ));
    }

    fs::write(path, markdown)
        .unwrap_or_else(|err| panic!("failed to write report {}: {err}", path.display()));
}

fn load_wav(path: &Path) -> (Vec<f32>, u32) {
    let reader = hound::WavReader::open(path)
        .unwrap_or_else(|err| panic!("failed to open wav {}: {err}", path.display()));
    let spec = reader.spec();
    assert_eq!(spec.channels, 1, "only mono wav supported");
    assert_eq!(spec.bits_per_sample, 16, "only 16-bit wav supported");
    let sample_rate = spec.sample_rate;
    let samples: Vec<f32> = reader
        .into_samples::<i16>()
        .map(|s| s.unwrap_or(0) as f32 / 32768.0)
        .collect();
    (samples, sample_rate)
}

fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn word_error_rate(hypothesis: &str, reference: &str) -> f64 {
    let hyp_words: Vec<&str> = hypothesis.split_whitespace().collect();
    let ref_words: Vec<&str> = reference.split_whitespace().collect();
    if ref_words.is_empty() {
        return if hyp_words.is_empty() { 0.0 } else { 1.0 };
    }
    let mut prev: Vec<usize> = (0..=ref_words.len()).collect();
    let mut curr = vec![0usize; ref_words.len() + 1];
    for i in 1..=hyp_words.len() {
        curr[0] = i;
        for j in 1..=ref_words.len() {
            let cost = if hyp_words[i - 1] == ref_words[j - 1] { 0 } else { 1 };
            curr[j] = std::cmp::min(
                std::cmp::min(curr[j - 1] + 1, prev[j] + 1),
                prev[j - 1] + cost,
            );
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[ref_words.len()] as f64 / ref_words.len() as f64
}

#[test]
fn whisper_transcribes_corpus_with_quality_and_speed_metrics() {
    assert!(
        cfg!(any(feature = "whisper-vulkan", feature = "whisper-cuda")),
        "either whisper-vulkan or whisper-cuda feature must be enabled for this integration test"
    );

    let corpus = corpus_dir();
    if !corpus.exists() {
        eprintln!("skipping: missing corpus dir {}", corpus.display());
        return;
    }

    let model_path = model_path();
    if !model_path.exists() {
        eprintln!("skipping: missing model path {}", model_path.display());
        return;
    }

    let gpu_device = whisper_gpu_device_index();
    accel::set_whisper_gpu_device(gpu_device);

    let mut model = WhisperModel::new();
    model
        .load(&ModelConfig {
            id: "whisper".to_string(),
            model_path: model_path.display().to_string(),
            language: None,
        })
        .unwrap_or_else(|err| panic!("failed to load model: {}", err.message));

    let warmup = clips();
    let warmup_path = corpus.join(warmup[0].wav);
    if warmup_path.exists() {
        let (warmup_samples, warmup_rate) = load_wav(&warmup_path);
        let _ = model
            .transcribe(&AudioInput {
                samples: warmup_samples,
                sample_rate_hz: warmup_rate,
            })
            .unwrap_or_else(|err| panic!("warmup transcribe failed: {}", err.message));
    }

    let mut total_audio_secs = 0.0f64;
    let mut total_infer_secs = 0.0f64;
    let mut total_wer = 0.0f64;
    let mut processed = 0usize;
    let mut runs: Vec<ClipRun> = Vec::new();

    for clip in clips() {
        let wav_path = corpus.join(clip.wav);
        let ref_path = corpus.join(clip.reference);
        if !wav_path.exists() || !ref_path.exists() {
            eprintln!("skipping clip {} due to missing files", clip.name);
            continue;
        }

        let (samples, sample_rate) = load_wav(&wav_path);
        let audio_secs = samples.len() as f64 / sample_rate as f64;
        let start = Instant::now();
        let output = model
            .transcribe(&AudioInput {
                samples,
                sample_rate_hz: sample_rate,
            })
            .unwrap_or_else(|err| panic!("transcribe failed for {}: {}", clip.name, err.message));
        let infer_secs = start.elapsed().as_secs_f64();

        let reference = fs::read_to_string(&ref_path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", ref_path.display()));
        let ref_norm = normalize(&reference);
        let hyp_norm = normalize(&output.text);
        let wer = word_error_rate(&hyp_norm, &ref_norm);

        println!(
            "clip={} audio_secs={:.2} infer_secs={:.2} rt_mult={:.2} wer={:.3}",
            clip.name,
            audio_secs,
            infer_secs,
            if infer_secs > 0.0 {
                audio_secs / infer_secs
            } else {
                0.0
            },
            wer
        );

        runs.push(ClipRun {
            name: clip.name.to_string(),
            audio_secs,
            infer_secs,
            realtime_x: if infer_secs > 0.0 {
                audio_secs / infer_secs
            } else {
                0.0
            },
            wer,
        });

        assert!(!hyp_norm.is_empty(), "empty transcription for {}", clip.name);
        assert!(wer <= 0.90, "WER too high for {}: {:.3}", clip.name, wer);

        total_audio_secs += audio_secs;
        total_infer_secs += infer_secs;
        total_wer += wer;
        processed += 1;
    }

    assert!(processed > 0, "no corpus clips were processed");
    let avg_wer = total_wer / processed as f64;
    let rt_mult = if total_infer_secs > 0.0 {
        total_audio_secs / total_infer_secs
    } else {
        0.0
    };

    println!(
        "summary clips={} avg_wer={:.3} audio_secs={:.2} infer_secs={:.2} realtime_x={:.2}",
        processed, avg_wer, total_audio_secs, total_infer_secs, rt_mult
    );

    let report = report_path();
    let hardware = backend_hardware_label();
    write_markdown_report(
        &report,
        &model_path,
        &hardware,
        &runs,
        avg_wer,
        total_audio_secs,
        total_infer_secs,
        rt_mult,
    );
    println!("report_path={}", report.display());

    assert!(avg_wer <= 0.70, "average WER too high: {:.3}", avg_wer);
    assert!(total_infer_secs > 0.0, "inference timing did not advance");
}
