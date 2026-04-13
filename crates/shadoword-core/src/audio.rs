use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, Stream, StreamConfig};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct AudioInput {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputDeviceInfo {
    pub name: String,
    pub is_default: bool,
}

pub struct RecordingSession {
    stream: Stream,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
}

impl RecordingSession {
    pub fn stop(self) -> Result<AudioInput> {
        drop(self.stream);
        let samples = self
            .samples
            .lock()
            .map_err(|_| anyhow!("recording buffer poisoned"))?
            .clone();
        Ok(AudioInput {
            samples,
            sample_rate: self.sample_rate,
        })
    }
}

#[derive(Default)]
pub struct MicrophoneRecorder;

impl MicrophoneRecorder {
    pub fn list_input_devices() -> Result<Vec<InputDeviceInfo>> {
        let host = cpal::default_host();
        let default_name = host
            .default_input_device()
            .and_then(|device| device.name().ok());

        let mut devices = host
            .input_devices()
            .context("failed to list input devices")?
            .filter_map(|device| {
                let name = device.name().ok()?;
                Some(InputDeviceInfo {
                    is_default: default_name.as_deref() == Some(name.as_str()),
                    name,
                })
            })
            .collect::<Vec<_>>();

        devices.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(devices)
    }

    pub fn start(input_device_name: Option<&str>) -> Result<RecordingSession> {
        let host = cpal::default_host();

        let device = if let Some(name) = input_device_name {
            host.input_devices()
                .context("failed to list input devices")?
                .find(|device| device.name().map(|current| current == name).unwrap_or(false))
                .with_context(|| format!("input device '{}' not found", name))?
        } else {
            host.default_input_device()
                .context("failed to find default input device")?
        };

        let supported = device
            .default_input_config()
            .context("failed to query default input config")?;

        let sample_rate = supported.sample_rate().0;
        let channels = usize::from(supported.channels());
        let config: StreamConfig = supported.clone().into();
        let samples = Arc::new(Mutex::new(Vec::new()));
        let writer = Arc::clone(&samples);

        let err_fn = |err| log::error!("audio input stream error: {err}");

        let stream = match supported.sample_format() {
            SampleFormat::I8 => build_stream::<i8>(&device, &config, channels, writer, err_fn)?,
            SampleFormat::I16 => {
                build_stream::<i16>(&device, &config, channels, writer, err_fn)?
            }
            SampleFormat::I32 => {
                build_stream::<i32>(&device, &config, channels, writer, err_fn)?
            }
            SampleFormat::I64 => {
                build_stream::<i64>(&device, &config, channels, writer, err_fn)?
            }
            SampleFormat::U8 => build_stream::<u8>(&device, &config, channels, writer, err_fn)?,
            SampleFormat::U16 => {
                build_stream::<u16>(&device, &config, channels, writer, err_fn)?
            }
            SampleFormat::U32 => {
                build_stream::<u32>(&device, &config, channels, writer, err_fn)?
            }
            SampleFormat::U64 => {
                build_stream::<u64>(&device, &config, channels, writer, err_fn)?
            }
            SampleFormat::F32 => {
                build_stream::<f32>(&device, &config, channels, writer, err_fn)?
            }
            SampleFormat::F64 => {
                build_stream::<f64>(&device, &config, channels, writer, err_fn)?
            }
            other => return Err(anyhow!("unsupported input sample format: {:?}", other)),
        };

        stream.play().context("failed to start input stream")?;

        Ok(RecordingSession {
            stream,
            samples,
            sample_rate,
        })
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    channels: usize,
    samples: Arc<Mutex<Vec<f32>>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<Stream>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    let stream = device.build_input_stream(
        config,
        move |data: &[T], _| {
            if let Ok(mut buffer) = samples.lock() {
                for frame in data.chunks(channels.max(1)) {
                    let mut mixed = 0.0f32;
                    for sample in frame {
                        mixed += f32::from_sample(*sample);
                    }
                    buffer.push(mixed / frame.len() as f32);
                }
            }
        },
        err_fn,
        None,
    )?;
    Ok(stream)
}
