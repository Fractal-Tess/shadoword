use anyhow::{anyhow, Context, Result};
use arboard::Clipboard;
use eframe::egui;
use enigo::{Enigo, Keyboard, Settings};
use shadowword_core::{
    AudioInput, DeviceListResponse, EngineKind, InputDeviceInfo, LocalService, MicrophoneRecorder,
    OnnxQuantization, OrtxAccelerator, RecordingSession, ServiceMode, ServiceStatus,
    ShadowwordConfig, TranscriptResponse, TranscriptionService,
    WhisperAccelerator,
};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = ShadowwordConfig::load()?;
    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "Shadoword",
        native_options,
        Box::new(move |_| Ok(Box::new(ShadowwordApp::new(config.clone())))),
    )
    .map_err(|error| anyhow!(error.to_string()))
}

struct ShadowwordApp {
    config: ShadowwordConfig,
    local_service: Arc<LocalService>,
    active_recording: Option<RecordingSession>,
    response_rx: Option<Receiver<Result<TranscriptResponse, String>>>,
    available_inputs: Vec<InputDeviceInfo>,
    remote_status: Option<ServiceStatus>,
    transcript: String,
    status_line: String,
}

impl ShadowwordApp {
    fn new(config: ShadowwordConfig) -> Self {
        let local_service = Arc::new(LocalService::new(config.clone()));
        let available_inputs = MicrophoneRecorder::list_input_devices().unwrap_or_default();

        Self {
            config,
            local_service,
            active_recording: None,
            response_rx: None,
            available_inputs,
            remote_status: None,
            transcript: String::new(),
            status_line: "Ready".to_string(),
        }
    }

    fn start_recording(&mut self) {
        match MicrophoneRecorder::start(self.config.recording.input_device.as_deref()) {
            Ok(session) => {
                self.status_line = "Recording from local microphone...".to_string();
                self.active_recording = Some(session);
            }
            Err(error) => {
                self.status_line = format!("Failed to start recording: {error}");
            }
        }
    }

    fn stop_recording(&mut self) {
        let Some(session) = self.active_recording.take() else {
            return;
        };

        match session.stop() {
            Ok(audio) => {
                self.status_line = if self.config.mode == ServiceMode::Local {
                    "Transcribing locally...".to_string()
                } else {
                    "Uploading audio to remote daemon...".to_string()
                };

                let (tx, rx) = mpsc::channel();
                let config = self.config.clone();
                let local = Arc::clone(&self.local_service);
                thread::spawn(move || {
                    let result = if config.mode == ServiceMode::Local {
                        local.transcribe_audio(audio).map_err(|error| error.to_string())
                    } else {
                        remote_transcribe(&config, audio).map_err(|error| error.to_string())
                    };
                    let _ = tx.send(result);
                });
                self.response_rx = Some(rx);
            }
            Err(error) => {
                self.status_line = format!("Failed to finish recording: {error}");
            }
        }
    }

    fn save_local_config(&mut self) {
        match self.local_service.update_config(self.config.clone()) {
            Ok(()) => {
                self.status_line = "Desktop configuration saved".to_string();
            }
            Err(error) => {
                self.status_line = format!("Failed to save desktop configuration: {error}");
            }
        }
    }

    fn refresh_local_devices(&mut self) {
        match MicrophoneRecorder::list_input_devices() {
            Ok(devices) => {
                self.available_inputs = devices;
                self.status_line = "Refreshed local microphone list".to_string();
            }
            Err(error) => {
                self.status_line = format!("Failed to list microphones: {error}");
            }
        }
    }

    fn pull_remote_status(&mut self) {
        match remote_status(&self.config) {
            Ok(status) => {
                self.status_line = format!(
                    "Remote daemon ready: engine={:?}, loaded={}, sample_rate={}",
                    status.engine, status.model_loaded, status.sample_rate
                );
                self.remote_status = Some(status);
            }
            Err(error) => {
                self.status_line = format!("Failed to fetch remote status: {error}");
                self.remote_status = None;
            }
        }
    }

    fn pull_remote_config(&mut self) {
        match remote_get_config(&self.config) {
            Ok(remote_config) => {
                let endpoint = self.config.remote.endpoint.clone();
                let mode = self.config.mode;
                let input_device = self.config.recording.input_device.clone();

                self.config = remote_config;
                self.config.remote.endpoint = endpoint;
                self.config.mode = mode;
                self.config.recording.input_device = input_device;
                self.status_line = "Pulled daemon configuration".to_string();
            }
            Err(error) => {
                self.status_line = format!("Failed to pull daemon configuration: {error}");
            }
        }
    }

    fn push_remote_config(&mut self) {
        match remote_update_config(&self.config, &self.config) {
            Ok(updated) => {
                self.remote_status = None;
                self.status_line = format!(
                    "Pushed daemon configuration to {}",
                    updated.daemon.listen_addr
                );
            }
            Err(error) => {
                self.status_line = format!("Failed to push daemon configuration: {error}");
            }
        }
    }

    fn poll_background_work(&mut self) {
        let Some(rx) = &self.response_rx else {
            return;
        };

        match rx.try_recv() {
            Ok(Ok(response)) => {
                self.transcript = response.text.clone();
                self.status_line =
                    format!("Transcribed in {}ms with {:?}", response.elapsed_ms, response.engine);
                let _ = apply_output(&self.config, &response.text);
                self.response_rx = None;
            }
            Ok(Err(error)) => {
                self.status_line = format!("Transcription failed: {error}");
                self.response_rx = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.status_line = "Background worker disconnected".to_string();
                self.response_rx = None;
            }
        }
    }

    fn device_name(&self) -> &str {
        self.config
            .recording
            .input_device
            .as_deref()
            .unwrap_or("System default")
    }
}

impl eframe::App for ShadowwordApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_background_work();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Shadoword");
            ui.label("A standalone speech-to-text desktop client that can also target a remote daemon.");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Mode");
                ui.selectable_value(&mut self.config.mode, ServiceMode::Local, "Local");
                ui.selectable_value(&mut self.config.mode, ServiceMode::Remote, "Remote");
            });

            ui.group(|ui| {
                ui.heading("Desktop");
                ui.label("The desktop app always records from this machine's microphone.");

                egui::ComboBox::from_label("Input device")
                    .selected_text(self.device_name())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.config.recording.input_device,
                            None,
                            "System default",
                        );

                        for device in &self.available_inputs {
                            let label = if device.is_default {
                                format!("{} (default)", device.name)
                            } else {
                                device.name.clone()
                            };
                            ui.selectable_value(
                                &mut self.config.recording.input_device,
                                Some(device.name.clone()),
                                label,
                            );
                        }
                    });

                ui.horizontal(|ui| {
                    ui.label("Capture / target sample rate");
                    ui.add(
                        egui::DragValue::new(&mut self.config.recording.sample_rate)
                            .range(8_000..=96_000)
                            .speed(100.0),
                    );
                    if ui.button("Refresh Devices").clicked() {
                        self.refresh_local_devices();
                    }
                });

                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.config.output.copy_to_clipboard,
                        "Copy transcript to clipboard",
                    );
                    ui.checkbox(
                        &mut self.config.output.type_into_active_window,
                        "Type transcript into active window",
                    );
                });

                if ui.button("Save Desktop Config").clicked() {
                    self.save_local_config();
                }
            });

            ui.add_space(8.0);

            ui.group(|ui| {
                ui.heading("Inference");

                ui.horizontal(|ui| {
                    ui.label("Engine");
                    ui.selectable_value(&mut self.config.engine, EngineKind::Parakeet, "Parakeet");
                    ui.selectable_value(&mut self.config.engine, EngineKind::Whisper, "Whisper");
                });

                ui.horizontal(|ui| {
                    ui.label("ORT accelerator");
                    ui.selectable_value(
                        &mut self.config.ort_accelerator,
                        OrtxAccelerator::Auto,
                        "Auto",
                    );
                    ui.selectable_value(
                        &mut self.config.ort_accelerator,
                        OrtxAccelerator::Cpu,
                        "CPU",
                    );
                    ui.selectable_value(
                        &mut self.config.ort_accelerator,
                        OrtxAccelerator::Cuda,
                        "CUDA",
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("ONNX quantization");
                    ui.selectable_value(
                        &mut self.config.onnx_quantization,
                        OnnxQuantization::Fp32,
                        "FP32",
                    );
                    ui.selectable_value(
                        &mut self.config.onnx_quantization,
                        OnnxQuantization::Fp16,
                        "FP16",
                    );
                    ui.selectable_value(
                        &mut self.config.onnx_quantization,
                        OnnxQuantization::Int8,
                        "INT8",
                    );
                    ui.selectable_value(
                        &mut self.config.onnx_quantization,
                        OnnxQuantization::Int4,
                        "INT4",
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Whisper accelerator");
                    ui.selectable_value(
                        &mut self.config.whisper_accelerator,
                        WhisperAccelerator::Auto,
                        "Auto",
                    );
                    ui.selectable_value(
                        &mut self.config.whisper_accelerator,
                        WhisperAccelerator::Cpu,
                        "CPU",
                    );
                    ui.selectable_value(
                        &mut self.config.whisper_accelerator,
                        WhisperAccelerator::Gpu,
                        "GPU",
                    );
                });

                ui.label("Model path");
                let mut model_path = self.config.model_path.display().to_string();
                if ui.text_edit_singleline(&mut model_path).changed() {
                    self.config.model_path = model_path.into();
                }
            });

            ui.add_space(8.0);

            ui.group(|ui| {
                ui.heading("Remote Daemon");
                ui.label("Remote mode still records audio locally, then sends WAV audio to the daemon.");

                ui.horizontal(|ui| {
                    ui.label("Daemon endpoint");
                    ui.text_edit_singleline(&mut self.config.remote.endpoint);
                });

                ui.horizontal(|ui| {
                    ui.label("Daemon listen addr");
                    ui.text_edit_singleline(&mut self.config.daemon.listen_addr);
                });

                ui.horizontal(|ui| {
                    if ui.button("Remote Status").clicked() {
                        self.pull_remote_status();
                    }
                    if ui.button("Pull Remote Config").clicked() {
                        self.pull_remote_config();
                    }
                    if ui.button("Push Remote Config").clicked() {
                        self.push_remote_config();
                    }
                });

                if let Some(status) = &self.remote_status {
                    ui.label(format!(
                        "Remote engine={:?}, loaded={}, onnx={:?}, ort={:?}, whisper={:?}, sample_rate={}",
                        status.engine,
                        status.model_loaded,
                        status.onnx_quantization,
                        status.ort_accelerator,
                        status.whisper_accelerator,
                        status.sample_rate
                    ));
                }
            });

            ui.add_space(8.0);

            ui.group(|ui| {
                ui.heading("Run");
                ui.horizontal(|ui| {
                    if self.active_recording.is_none() {
                        if ui.button("Start Recording").clicked() {
                            self.start_recording();
                        }
                    } else if ui.button("Stop Recording").clicked() {
                        self.stop_recording();
                    }
                });
            });

            ui.separator();
            ui.label(format!("Status: {}", self.status_line));
            ui.add(
                egui::TextEdit::multiline(&mut self.transcript)
                    .desired_rows(12)
                    .hint_text("Transcript output"),
            );
        });
    }
}

fn remote_transcribe(config: &ShadowwordConfig, audio: AudioInput) -> Result<TranscriptResponse> {
    let client = reqwest::blocking::Client::new();
    let wav = shadowword_core::wav::encode_wav(&audio)?;
    let response = client
        .post(format!("{}/v1/transcribe-wav", config.remote.endpoint))
        .header("content-type", "audio/wav")
        .body(wav)
        .send()
        .context("failed to reach remote daemon")?
        .error_for_status()
        .context("daemon returned an error status")?
        .json::<TranscriptResponse>()
        .context("failed to decode daemon response")?;
    Ok(response)
}

fn remote_status(config: &ShadowwordConfig) -> Result<ServiceStatus> {
    reqwest::blocking::get(format!("{}/v1/status", config.remote.endpoint))
        .context("failed to reach remote daemon")?
        .error_for_status()
        .context("daemon returned an error status")?
        .json::<ServiceStatus>()
        .context("failed to decode daemon status")
}

fn remote_get_config(config: &ShadowwordConfig) -> Result<ShadowwordConfig> {
    reqwest::blocking::get(format!("{}/v1/config", config.remote.endpoint))
        .context("failed to reach remote daemon")?
        .error_for_status()
        .context("daemon returned an error status")?
        .json::<ShadowwordConfig>()
        .context("failed to decode daemon config")
}

fn remote_update_config(
    current: &ShadowwordConfig,
    next: &ShadowwordConfig,
) -> Result<ShadowwordConfig> {
    let client = reqwest::blocking::Client::new();
    client
        .put(format!("{}/v1/config", current.remote.endpoint))
        .json(next)
        .send()
        .context("failed to reach remote daemon")?
        .error_for_status()
        .context("daemon returned an error status")?
        .json::<ShadowwordConfig>()
        .context("failed to decode updated daemon config")
}

#[allow(dead_code)]
fn remote_devices(config: &ShadowwordConfig) -> Result<DeviceListResponse> {
    reqwest::blocking::get(format!("{}/v1/devices", config.remote.endpoint))
        .context("failed to reach remote daemon")?
        .error_for_status()
        .context("daemon returned an error status")?
        .json::<DeviceListResponse>()
        .context("failed to decode daemon device list")
}

fn apply_output(config: &ShadowwordConfig, text: &str) -> Result<()> {
    if config.output.copy_to_clipboard {
        let mut clipboard = Clipboard::new().context("failed to access clipboard")?;
        clipboard
            .set_text(text.to_string())
            .context("failed to write clipboard")?;
    }

    if config.output.type_into_active_window {
        let mut enigo = Enigo::new(&Settings::default()).context("failed to init enigo")?;
        enigo.text(text).context("failed to type transcript")?;
    }

    Ok(())
}
