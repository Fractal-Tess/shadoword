use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, Stream, StreamConfig};
use serde::{Deserialize, Serialize};
pub use shadoword_shared::AudioInput;
use specta::Type;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct InputDeviceInfo {
    pub name: String,
    pub is_default: bool,
}

pub struct RecordingSession {
    stream: Stream,
    samples: Arc<Mutex<Vec<f32>>>,
    available: Arc<Notify>,
    sample_rate: u32,
}

pub struct MicrophoneLevelMonitor {
    _stream: Stream,
    peak: Arc<AtomicU32>,
}

impl MicrophoneLevelMonitor {
    /// Returns the highest normalized sample magnitude since the previous read.
    pub fn peak(&self) -> f32 {
        f32::from_bits(self.peak.swap(0, Ordering::Relaxed))
    }
}

#[derive(Clone)]
pub struct RecordingSnapshotSource {
    samples: Arc<Mutex<Vec<f32>>>,
    available: Arc<Notify>,
    sample_rate: u32,
}

impl RecordingSnapshotSource {
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub async fn wait_for_samples(&self) {
        let has_samples = self
            .samples
            .lock()
            .map(|samples| !samples.is_empty())
            .unwrap_or(true);
        if !has_samples {
            self.available.notified().await;
        }
    }

    pub fn drain_available(&self) -> Result<AudioInput> {
        let mut samples = self
            .samples
            .lock()
            .map_err(|_| anyhow!("recording buffer poisoned"))?;
        let drained = std::mem::take(&mut *samples);
        Ok(AudioInput {
            samples: drained,
            sample_rate: self.sample_rate,
        })
    }
}

impl RecordingSession {
    pub fn snapshot_source(&self) -> RecordingSnapshotSource {
        RecordingSnapshotSource {
            samples: Arc::clone(&self.samples),
            available: Arc::clone(&self.available),
            sample_rate: self.sample_rate,
        }
    }

    pub fn stop(self) -> Result<AudioInput> {
        drop(self.stream);
        let mut samples = self
            .samples
            .lock()
            .map_err(|_| anyhow!("recording buffer poisoned"))?;
        Ok(AudioInput {
            samples: std::mem::take(&mut *samples),
            sample_rate: self.sample_rate,
        })
    }

    pub fn stop_without_snapshot(self) {
        drop(self.stream);
    }
}

#[derive(Default)]
pub struct MicrophoneRecorder;

impl MicrophoneRecorder {
    pub fn list_input_devices() -> Result<Vec<InputDeviceInfo>> {
        let host = cpal::default_host();
        let default_name = host.default_input_device().map(|device| device.to_string());

        let mut devices = host
            .input_devices()
            .context("failed to list input devices")?
            .map(|device| {
                let name = device.to_string();
                InputDeviceInfo {
                    is_default: default_name.as_deref() == Some(name.as_str()),
                    name,
                }
            })
            .collect::<Vec<_>>();

        devices.sort_by(|left, right| left.name.cmp(&right.name));
        devices.dedup_by(|left, right| left.name == right.name);
        Ok(devices)
    }

    pub fn start(input_device_name: Option<&str>) -> Result<RecordingSession> {
        let host = cpal::default_host();

        if let Some(name) = input_device_name {
            let device = host
                .input_devices()
                .context("failed to list input devices")?
                .find(|device| device.to_string() == name)
                .with_context(|| format!("input device '{}' not found", name))?;
            return Self::start_device(&device);
        }

        let default_device = host.default_input_device();
        let default_name = default_device.as_ref().map(ToString::to_string);
        if let Some(device) = default_device {
            match Self::start_device(&device) {
                Ok(session) => return Ok(session),
                Err(default_error) => {
                    for fallback in host
                        .input_devices()
                        .context("failed to list fallback input devices")?
                    {
                        if default_name.as_deref() == Some(fallback.to_string().as_str()) {
                            continue;
                        }
                        if let Ok(session) = Self::start_device(&fallback) {
                            log::warn!(
                                "default audio input was unavailable; using '{}' instead",
                                fallback
                            );
                            return Ok(session);
                        }
                    }
                    return Err(default_error).context("default audio input is unavailable");
                }
            }
        }

        let fallback = host
            .input_devices()
            .context("failed to list fallback input devices")?
            .next()
            .context("failed to find an input device")?;
        log::warn!(
            "no default audio input is configured; using '{}' instead",
            fallback
        );
        Self::start_device(&fallback)
    }

    pub fn start_level_monitor(input_device_name: Option<&str>) -> Result<MicrophoneLevelMonitor> {
        let host = cpal::default_host();

        if let Some(name) = input_device_name {
            let device = host
                .input_devices()
                .context("failed to list input devices")?
                .find(|device| device.to_string() == name)
                .with_context(|| format!("input device '{}' not found", name))?;
            return Self::start_level_monitor_device(&device);
        }

        let default_device = host.default_input_device();
        let default_name = default_device.as_ref().map(ToString::to_string);
        if let Some(device) = default_device {
            match Self::start_level_monitor_device(&device) {
                Ok(monitor) => return Ok(monitor),
                Err(default_error) => {
                    for fallback in host
                        .input_devices()
                        .context("failed to list fallback input devices")?
                    {
                        if default_name.as_deref() == Some(fallback.to_string().as_str()) {
                            continue;
                        }
                        if let Ok(monitor) = Self::start_level_monitor_device(&fallback) {
                            log::warn!(
                                "default audio input was unavailable for level monitoring; using '{}' instead",
                                fallback
                            );
                            return Ok(monitor);
                        }
                    }
                    return Err(default_error)
                        .context("default audio input is unavailable for level monitoring");
                }
            }
        }

        let fallback = host
            .input_devices()
            .context("failed to list fallback input devices")?
            .next()
            .context("failed to find an input device")?;
        log::warn!(
            "no default audio input is configured for level monitoring; using '{}' instead",
            fallback
        );
        Self::start_level_monitor_device(&fallback)
    }

    fn start_device(device: &cpal::Device) -> Result<RecordingSession> {
        let supported = device
            .default_input_config()
            .context("failed to query default input config")?;

        let sample_rate = supported.sample_rate();
        let channels = usize::from(supported.channels());
        let config: StreamConfig = supported.into();
        let samples = Arc::new(Mutex::new(Vec::new()));
        let available = Arc::new(Notify::new());
        let writer = Arc::clone(&samples);
        let writer_available = Arc::clone(&available);

        let err_fn = |err| log::error!("audio input stream error: {err}");

        let stream = match supported.sample_format() {
            SampleFormat::I8 => {
                build_stream::<i8>(device, &config, channels, writer, writer_available, err_fn)?
            }
            SampleFormat::I16 => {
                build_stream::<i16>(device, &config, channels, writer, writer_available, err_fn)?
            }
            SampleFormat::I32 => {
                build_stream::<i32>(device, &config, channels, writer, writer_available, err_fn)?
            }
            SampleFormat::I64 => {
                build_stream::<i64>(device, &config, channels, writer, writer_available, err_fn)?
            }
            SampleFormat::U8 => {
                build_stream::<u8>(device, &config, channels, writer, writer_available, err_fn)?
            }
            SampleFormat::U16 => {
                build_stream::<u16>(device, &config, channels, writer, writer_available, err_fn)?
            }
            SampleFormat::U32 => {
                build_stream::<u32>(device, &config, channels, writer, writer_available, err_fn)?
            }
            SampleFormat::U64 => {
                build_stream::<u64>(device, &config, channels, writer, writer_available, err_fn)?
            }
            SampleFormat::F32 => {
                build_stream::<f32>(device, &config, channels, writer, writer_available, err_fn)?
            }
            SampleFormat::F64 => {
                build_stream::<f64>(device, &config, channels, writer, writer_available, err_fn)?
            }
            other => return Err(anyhow!("unsupported input sample format: {:?}", other)),
        };

        stream.play().context("failed to start input stream")?;

        Ok(RecordingSession {
            stream,
            samples,
            available,
            sample_rate,
        })
    }

    fn start_level_monitor_device(device: &cpal::Device) -> Result<MicrophoneLevelMonitor> {
        let supported = device
            .default_input_config()
            .context("failed to query default input config")?;
        let config: StreamConfig = supported.into();
        let peak = Arc::new(AtomicU32::new(0));
        let writer = Arc::clone(&peak);
        let err_fn = |err| log::error!("microphone level monitor stream error: {err}");

        let stream = match supported.sample_format() {
            SampleFormat::I8 => build_level_stream::<i8>(device, &config, writer, err_fn)?,
            SampleFormat::I16 => build_level_stream::<i16>(device, &config, writer, err_fn)?,
            SampleFormat::I32 => build_level_stream::<i32>(device, &config, writer, err_fn)?,
            SampleFormat::I64 => build_level_stream::<i64>(device, &config, writer, err_fn)?,
            SampleFormat::U8 => build_level_stream::<u8>(device, &config, writer, err_fn)?,
            SampleFormat::U16 => build_level_stream::<u16>(device, &config, writer, err_fn)?,
            SampleFormat::U32 => build_level_stream::<u32>(device, &config, writer, err_fn)?,
            SampleFormat::U64 => build_level_stream::<u64>(device, &config, writer, err_fn)?,
            SampleFormat::F32 => build_level_stream::<f32>(device, &config, writer, err_fn)?,
            SampleFormat::F64 => build_level_stream::<f64>(device, &config, writer, err_fn)?,
            other => return Err(anyhow!("unsupported input sample format: {:?}", other)),
        };

        stream
            .play()
            .context("failed to start microphone level monitor stream")?;

        Ok(MicrophoneLevelMonitor {
            _stream: stream,
            peak,
        })
    }
}

fn build_level_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    peak: Arc<AtomicU32>,
    err_fn: impl FnMut(cpal::Error) + Send + 'static,
) -> Result<Stream>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    let stream = device.build_input_stream(
        *config,
        move |data: &[T], _| {
            let peak_value = data.iter().fold(0.0f32, |peak_value, sample| {
                peak_value.max(f32::from_sample(*sample).abs())
            });
            let peak_bits = peak_value.min(1.0).to_bits();
            let mut current = peak.load(Ordering::Relaxed);
            while peak_bits > current {
                match peak.compare_exchange_weak(
                    current,
                    peak_bits,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(observed) => current = observed,
                }
            }
        },
        err_fn,
        None,
    )?;
    Ok(stream)
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    channels: usize,
    samples: Arc<Mutex<Vec<f32>>>,
    available: Arc<Notify>,
    err_fn: impl FnMut(cpal::Error) + Send + 'static,
) -> Result<Stream>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    let stream = device.build_input_stream(
        *config,
        move |data: &[T], _| {
            let wrote_samples = if let Ok(mut buffer) = samples.lock() {
                for frame in data.chunks(channels.max(1)) {
                    let mut mixed = 0.0f32;
                    for sample in frame {
                        mixed += f32::from_sample(*sample);
                    }
                    buffer.push(mixed / frame.len() as f32);
                }
                !data.is_empty()
            } else {
                false
            };
            if wrote_samples {
                available.notify_one();
            }
        },
        err_fn,
        None,
    )?;
    Ok(stream)
}
