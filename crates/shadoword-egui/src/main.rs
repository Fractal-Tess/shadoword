use anyhow::{anyhow, Context, Result};
use eframe::egui;
use egui::{Frame, Margin};
use crate::ui::constants::BG;
use crate::ui::Page;
#[cfg(not(target_os = "linux"))]
use global_hotkey::hotkey::HotKey;
#[cfg(not(target_os = "linux"))]
use global_hotkey::GlobalHotKeyManager;
#[cfg(target_os = "linux")]
use crate::hotkeys::LinuxHotkeyBackend;
use shadoword_core::{
    InputDeviceInfo, LocalService, MicrophoneRecorder, PasteMethod, RecordingSession, ServiceMode,
    ServiceStatus, ShadowwordConfig, StreamEvent, TranscriptResponse,
    TypingTool, WhisperAccelerator,
};
use std::collections::HashMap;
use std::thread;
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

mod hotkeys;
mod output;
mod platform;
mod recording;
mod remote;
mod ui;
mod transcription;

// ── Entry ────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = ShadowwordConfig::load()?;
    platform::setup_tray();
    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 640.0])
            .with_min_inner_size([700.0, 450.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Shadow Word",
        native_options,
        Box::new(move |cc| {
            ui::theme::apply(&cc.egui_ctx);
            Ok(Box::new(ShadowwordApp::new(config.clone())))
        }),
    )
    .map_err(|e| anyhow!(e.to_string()))
}



pub(crate) struct ConnectionTestResult {
    success: bool,
    message: String,
}

pub(crate) struct HistoryEntry {
    text: String,
    engine: String,
    elapsed_ms: u128,
    timestamp: String,
}

// ── Model catalog ───────────────────────────────────────────────────────────

#[allow(dead_code)]
struct CatalogModel {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    filename: &'static str,
    url: &'static str,
    size_mb: u32,
    accuracy: f32,
    speed: f32,
    recommended: bool,
    languages: &'static str,
}

const MODEL_CATALOG: &[CatalogModel] = &[
    CatalogModel {
        id: "whisper-small",
        name: "Whisper Small",
        description: "Fast, fairly accurate, 99 languages",
        filename: "ggml-small.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        size_mb: 465,
        accuracy: 0.60,
        speed: 0.85,
        recommended: false,
        languages: "99 languages",
    },
    CatalogModel {
        id: "whisper-medium",
        name: "Whisper Medium",
        description: "Good accuracy, medium speed",
        filename: "whisper-medium-q4_1.bin",
        url: "https://blob.handy.computer/whisper-medium-q4_1.bin",
        size_mb: 469,
        accuracy: 0.75,
        speed: 0.60,
        recommended: false,
        languages: "99 languages",
    },
    CatalogModel {
        id: "whisper-turbo",
        name: "Whisper Turbo",
        description: "Balanced accuracy and speed",
        filename: "ggml-large-v3-turbo.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin",
        size_mb: 1549,
        accuracy: 0.80,
        speed: 0.40,
        recommended: true,
        languages: "99 languages",
    },
    CatalogModel {
        id: "whisper-large",
        name: "Whisper Large",
        description: "Highest accuracy, slowest",
        filename: "ggml-large-v3-q5_0.bin",
        url: "https://blob.handy.computer/ggml-large-v3-q5_0.bin",
        size_mb: 1031,
        accuracy: 0.85,
        speed: 0.30,
        recommended: false,
        languages: "99 languages",
    },
];

struct DownloadProgress {
    downloaded: u64,
    total: u64,
    error: Option<String>,
    done: bool,
}

struct ActiveDownload {
    rx: Receiver<DownloadProgress>,
    downloaded: u64,
    total: u64,
    name: String,
}

/// Convert days since Unix epoch to (year, month, day).
pub(crate) fn days_to_date(days: u64) -> (u64, u64, u64) {
    // Civil calendar algorithm
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

// ── Application ──────────────────────────────────────────────────────────────

struct ShadowwordApp {
    pub(crate) config: ShadowwordConfig,
    prev_config: ShadowwordConfig,
    pub(crate) local_service: Arc<LocalService>,
    pub(crate) active_recording: Option<RecordingSession>,
    pub(crate) response_rx: Option<Receiver<Result<TranscriptResponse, String>>>,
    pub(crate) stream_event_rx: Option<Receiver<StreamEvent>>,
    available_inputs: Vec<InputDeviceInfo>,
    remote_status: Option<ServiceStatus>,
    connection_test: Option<ConnectionTestResult>,
    pub(crate) transcript: String,
    pub(crate) history: Vec<HistoryEntry>,
    pub(crate) status_line: String,
    page: Page,
    pub(crate) window_hidden: bool,
    pub(crate) quitting: bool,
    #[cfg(not(target_os = "linux"))]
    pub(crate) hotkey_manager: Option<GlobalHotKeyManager>,
    #[cfg(not(target_os = "linux"))]
    pub(crate) registered_hotkey: Option<HotKey>,
    #[cfg(target_os = "linux")]
    pub(crate) hotkey_manager: Option<LinuxHotkeyBackend>,
    editing_shortcut: bool,
    pub(crate) shortcut_error: Option<String>,
    active_downloads: HashMap<String, ActiveDownload>,
    preloading: bool,
    preload_rx: Option<Receiver<Result<(), String>>>,
    models_dir: Option<std::path::PathBuf>,
}

impl ShadowwordApp {
    fn new(config: ShadowwordConfig) -> Self {
        let local_service = Arc::new(LocalService::new(config.clone()));
        let available_inputs = MicrophoneRecorder::list_input_devices().unwrap_or_default();

        #[cfg(not(target_os = "linux"))]
        let (hotkey_manager, registered_hotkey, shortcut_error) = match GlobalHotKeyManager::new() {
            Ok(manager) => {
                let registered_hotkey = match hotkeys::register_hotkey(&manager, &config.hotkey.shortcut) {
                    Ok(hotkey) => Some(hotkey),
                    Err(error) => {
                        tracing::warn!("Failed to register startup hotkey '{}': {error}", config.hotkey.shortcut);
                        None
                    }
                };
                let shortcut_error = match &registered_hotkey {
                    Some(_) => None,
                    None => Some(format!("Failed to register shortcut: {}", config.hotkey.shortcut)),
                };
                if config.hotkey.shortcut.trim().is_empty() {
                    (Some(manager), None, Some("Shortcut cannot be empty".to_string()))
                } else {
                    (Some(manager), registered_hotkey, shortcut_error)
                }
            }
            Err(e) => {
                tracing::warn!("Failed to init global hotkey manager: {e}");
                (None, None, Some(format!("Failed to initialize global hotkey support: {e}")))
            }
        };

        #[cfg(target_os = "linux")]
        let (hotkey_manager, shortcut_error) = match LinuxHotkeyBackend::new() {
            Ok(manager) => {
                let shortcut_error = if config.hotkey.shortcut.trim().is_empty() {
                    Some("Shortcut cannot be empty".to_string())
                } else {
                    match manager.register(&config.hotkey.shortcut) {
                        Ok(()) => None,
                        Err(error) => {
                            tracing::warn!(
                                "Failed to register startup handy-keys hotkey '{}': {error}",
                                config.hotkey.shortcut
                            );
                            Some(error)
                        }
                    }
                };
                (Some(manager), shortcut_error)
            }
            Err(error) => {
                tracing::warn!("Failed to init handy-keys hotkey manager: {error}");
                (None, Some(format!("Failed to initialize global hotkey support: {error}")))
            }
        };

        let mut app = Self {
            prev_config: config.clone(),
            config,
            local_service,
            active_recording: None,
            response_rx: None,
            stream_event_rx: None,
            available_inputs,
            remote_status: None,
            connection_test: None,
            transcript: String::new(),
            history: Vec::new(),
            status_line: "Ready".to_string(),
            page: Page::General,
            window_hidden: false,
            quitting: false,
            hotkey_manager,
            #[cfg(not(target_os = "linux"))]
            registered_hotkey,
            editing_shortcut: false,
            shortcut_error,
            active_downloads: HashMap::new(),
            preloading: false,
            preload_rx: None,
            models_dir: ShadowwordConfig::models_dir().ok(),
        };

        if app.config.preload_on_startup && !app.config.model_path.as_os_str().is_empty() {
            app.start_preload();
        }

        app
    }

    fn auto_save_if_changed(&mut self) {
        if self.config != self.prev_config {
            let shortcut_changed = self.config.hotkey.shortcut != self.prev_config.hotkey.shortcut;

            match self.local_service.update_config(self.config.clone()) {
                Ok(()) => {
                    self.prev_config = self.config.clone();
                }
                Err(error) => {
                    self.status_line = format!("Failed to save configuration: {error}");
                }
            }

            if shortcut_changed {
                self.re_register_hotkey();
            }
        }
    }

    fn refresh_local_devices(&mut self) {
        match MicrophoneRecorder::list_input_devices() {
            Ok(devices) => {
                self.available_inputs = devices;
                self.status_line = "Refreshed microphone list".to_string();
            }
            Err(error) => {
                self.status_line = format!("Failed to list microphones: {error}");
            }
        }
    }

    fn pull_remote_config(&mut self) {
        match remote::remote_get_config(&self.config) {
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

    fn test_remote_connection(&mut self) {
        self.connection_test = None;
        match remote::remote_status(&self.config) {
            Ok(status) => {
                self.connection_test = Some(ConnectionTestResult {
                    success: true,
                    message: format!(
                        "Connected - engine: {}, model loaded: {}, sample rate: {}Hz",
                        status.engine, status.model_loaded, status.sample_rate
                    ),
                });
                self.remote_status = Some(status);
                self.status_line = "Remote connection successful".to_string();
            }
            Err(error) => {
                self.connection_test = Some(ConnectionTestResult {
                    success: false,
                    message: format!("Connection failed: {error}"),
                });
                self.remote_status = None;
                self.status_line = "Remote connection failed".to_string();
            }
        }
    }

    fn push_remote_config(&mut self) {
        match remote::remote_update_config(&self.config, &self.config) {
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

    fn is_model_installed(&self, model: &CatalogModel) -> bool {
        let Some(dir) = &self.models_dir else {
            return false;
        };
        let path = dir.join(model.filename);
        path.exists()
    }

    fn model_path_for(&self, model: &CatalogModel) -> Option<std::path::PathBuf> {
        self.models_dir.as_ref().map(|d| d.join(model.filename))
    }

    fn select_model(&mut self, model: &CatalogModel) {
        if let Some(path) = self.model_path_for(model) {
            self.config.model_path = path;
            self.status_line = format!("Selected {}", model.name);
        }
    }

    fn start_download(&mut self, model_id: &str) {
        if self.active_downloads.contains_key(model_id) {
            return;
        }
        let Some(model) = MODEL_CATALOG.iter().find(|m| m.id == model_id) else {
            return;
        };
        let Some(dir) = &self.models_dir else {
            self.status_line = "Cannot resolve models directory".to_string();
            return;
        };

        let url = model.url.to_string();
        let dest = dir.join(model.filename);
        let (tx, rx) = mpsc::channel();

        self.active_downloads.insert(
            model_id.to_string(),
            ActiveDownload {
                rx,
                downloaded: 0,
                total: 0,
                name: model.name.to_string(),
            },
        );
        self.status_line = format!("Downloading {}...", model.name);

        thread::spawn(move || {
            let result = (|| -> Result<()> {
                let client = reqwest::blocking::Client::new();
                let mut resp = client
                    .get(&url)
                    .send()
                    .context("failed to start download")?
                    .error_for_status()
                    .context("download returned error status")?;

                let total = resp.content_length().unwrap_or(0);
                let _ = tx.send(DownloadProgress { downloaded: 0, total, error: None, done: false });

                let mut file = std::fs::File::create(&dest)
                    .context("failed to create model file")?;
                let mut downloaded: u64 = 0;
                let mut buf = vec![0u8; 64 * 1024];
                loop {
                    let n = std::io::Read::read(&mut resp, &mut buf)
                        .context("download read error")?;
                    if n == 0 { break; }
                    std::io::Write::write_all(&mut file, &buf[..n])?;
                    downloaded += n as u64;
                    let _ = tx.send(DownloadProgress { downloaded, total, error: None, done: false });
                }
                let _ = tx.send(DownloadProgress { downloaded: 0, total, error: None, done: true });
                Ok(())
            })();
            if let Err(e) = result {
                let _ = tx.send(DownloadProgress { downloaded: 0, total: 0, error: Some(e.to_string()), done: true });
            }
        });
    }

    fn poll_downloads(&mut self) {
        let mut finished = Vec::new();
        for (id, dl) in &mut self.active_downloads {
            while let Ok(progress) = dl.rx.try_recv() {
                if let Some(err) = progress.error {
                    self.status_line = format!("Download failed: {err}");
                    finished.push(id.clone());
                    break;
                }
                dl.downloaded = progress.downloaded;
                if progress.total > 0 {
                    dl.total = progress.total;
                }
                if progress.done {
                    self.status_line = format!("{} downloaded", dl.name);
                    finished.push(id.clone());
                    break;
                }
            }
        }
        for id in finished {
            self.active_downloads.remove(&id);
        }
    }

    fn start_preload(&mut self) {
        self.preloading = true;
        self.status_line = "Preloading model...".to_string();
        let local = Arc::clone(&self.local_service);
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        thread::spawn(move || {
            let result = local.preload().map_err(|e| e.to_string());
            let _ = tx.send(result);
        });
        // Store in response_rx isn't ideal; use a separate channel
        // For simplicity, we'll poll this in poll_background_work area
        // Actually let's just add a preload receiver
        self.preload_rx = Some(rx);
    }

    fn poll_preload(&mut self) {
        let Some(rx) = &self.preload_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(())) => {
                self.status_line = "Model preloaded into memory".to_string();
                self.preloading = false;
                self.preload_rx = None;
            }
            Ok(Err(e)) => {
                self.status_line = format!("Preload failed: {e}");
                self.preloading = false;
                self.preload_rx = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.preloading = false;
                self.preload_rx = None;
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

    fn has_background_work(&self) -> bool {
        self.active_recording.is_some()
            || self.stream_event_rx.is_some()
            || self.response_rx.is_some()
            || self.preloading
            || !self.active_downloads.is_empty()
    }

}

// ── eframe integration ──────────────────────────────────────────────────────

impl eframe::App for ShadowwordApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.hide_window(ctx);
        }

        self.poll_tray_menu_events(ctx);
        self.poll_tray_icon_events(ctx);

        if self.hotkey_manager.is_some() {
            ctx.request_repaint_after(Duration::from_millis(33));
        }

        if self.has_background_work() {
            ctx.request_repaint_after(Duration::from_millis(16));
        }

        self.poll_hotkey_events();
        self.poll_stream_events();
        self.poll_background_work();
        self.poll_downloads();
        self.poll_preload();
        self.auto_save_if_changed();
        crate::ui::sidebar::show(ctx, self);
        crate::ui::status_bar::show(ctx, self);

        egui::CentralPanel::default()
            .frame(Frame::new().fill(BG).inner_margin(Margin::symmetric(24, 12)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    match self.page {
                        crate::ui::Page::General => crate::ui::general::show(ui, self),
                        crate::ui::Page::Models => crate::ui::models::show(ui, self),
                        crate::ui::Page::History => crate::ui::history::show(ui, self),
                        crate::ui::Page::Settings => crate::ui::settings::show(ui, self),
                        crate::ui::Page::About => crate::ui::about::show(ui, self),
                    }
                });
            });
    }
}


