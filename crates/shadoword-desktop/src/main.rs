use anyhow::{anyhow, Context, Result};
use arboard::Clipboard;
use eframe::egui;
use egui::{Color32, CornerRadius, Frame, Margin, RichText, Stroke, Vec2};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
#[cfg(not(target_os = "linux"))]
use global_hotkey::hotkey::HotKey;
#[cfg(not(target_os = "linux"))]
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
#[cfg(target_os = "linux")]
use handy_keys::{Hotkey as HandyHotkey, HotkeyId as HandyHotkeyId, HotkeyManager as HandyHotkeyManager, HotkeyState as HandyHotkeyState};
use shadoword_core::{
    AudioInput, DeviceListResponse, EngineKind, InputDeviceInfo, LocalService, MicrophoneRecorder,
    OnnxQuantization, OrtxAccelerator, PasteMethod, RecordingSession, ServiceMode, ServiceStatus,
    ShadowwordConfig, TranscriptResponse, TranscriptionService, TypingTool, WhisperAccelerator,
};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};
use tracing_subscriber::EnvFilter;

// ── Black & White palette ────────────────────────────────────────────────────

const BG: Color32 = Color32::from_rgb(18, 18, 18);
const BG_SIDEBAR: Color32 = Color32::from_rgb(12, 12, 12);
const BG_ROW: Color32 = Color32::from_rgb(24, 24, 24);
const BG_ROW_ALT: Color32 = Color32::from_rgb(20, 20, 20);
const BG_INPUT: Color32 = Color32::from_rgb(30, 30, 30);
const BG_HOVER: Color32 = Color32::from_rgb(40, 40, 40);
const BORDER: Color32 = Color32::from_rgb(50, 50, 50);
const TEXT: Color32 = Color32::from_rgb(230, 230, 230);
const TEXT_DIM: Color32 = Color32::from_rgb(140, 140, 140);
const TEXT_MUTED: Color32 = Color32::from_rgb(80, 80, 80);
const ACCENT: Color32 = Color32::WHITE;
const NAV_ACTIVE: Color32 = Color32::from_rgb(45, 45, 45);
const REC_RED: Color32 = Color32::from_rgb(200, 30, 30);
const STATUS_OK: Color32 = Color32::from_rgb(60, 200, 60);

// ── Entry ────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = ShadowwordConfig::load()?;
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
            apply_theme(&cc.egui_ctx);
            Ok(Box::new(ShadowwordApp::new(config.clone())))
        }),
    )
    .map_err(|e| anyhow!(e.to_string()))
}

// ── Theme ────────────────────────────────────────────────────────────────────

fn apply_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();

    v.panel_fill = BG;
    v.window_fill = BG;
    v.extreme_bg_color = BG_INPUT;
    v.faint_bg_color = BG_ROW;
    v.code_bg_color = BG_INPUT;
    v.hyperlink_color = ACCENT;
    v.warn_fg_color = TEXT;
    v.error_fg_color = TEXT;

    v.selection.bg_fill = Color32::from_rgba_premultiplied(255, 255, 255, 25);
    v.selection.stroke = Stroke::new(1.0, TEXT_DIM);
    v.window_stroke = Stroke::new(1.0, BORDER);

    v.widgets.noninteractive.bg_fill = BG_ROW;
    v.widgets.noninteractive.weak_bg_fill = BG_ROW;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.noninteractive.corner_radius = CornerRadius::same(4);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);

    v.widgets.inactive.bg_fill = BG_INPUT;
    v.widgets.inactive.weak_bg_fill = BG_INPUT;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.inactive.corner_radius = CornerRadius::same(4);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_DIM);

    v.widgets.hovered.bg_fill = BG_HOVER;
    v.widgets.hovered.weak_bg_fill = BG_HOVER;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, TEXT_DIM);
    v.widgets.hovered.corner_radius = CornerRadius::same(4);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);

    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.weak_bg_fill = ACCENT;
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.active.corner_radius = CornerRadius::same(4);
    v.widgets.active.fg_stroke = Stroke::new(1.5, BG);

    v.widgets.open.bg_fill = BG_HOVER;
    v.widgets.open.weak_bg_fill = BG_HOVER;
    v.widgets.open.bg_stroke = Stroke::new(1.0, TEXT_DIM);
    v.widgets.open.corner_radius = CornerRadius::same(4);
    v.widgets.open.fg_stroke = Stroke::new(1.0, TEXT);

    ctx.set_visuals(v);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 4.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    ctx.set_style(style);
}

// ── Pages ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Page {
    General,
    Models,
    History,
    Settings,
    About,
}

struct ConnectionTestResult {
    success: bool,
    message: String,
}

struct HistoryEntry {
    text: String,
    engine: EngineKind,
    elapsed_ms: u128,
    timestamp: String,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy)]
enum HotkeyEventState {
    Pressed,
    Released,
}

#[cfg(target_os = "linux")]
enum LinuxHotkeyCommand {
    Register {
        shortcut: String,
        response: mpsc::Sender<Result<(), String>>,
    },
    Unregister {
        response: mpsc::Sender<Result<(), String>>,
    },
    Shutdown,
}

#[cfg(target_os = "linux")]
struct LinuxHotkeyBackend {
    command_tx: mpsc::Sender<LinuxHotkeyCommand>,
    event_rx: Receiver<HotkeyEventState>,
    thread_handle: Option<JoinHandle<()>>,
}

#[cfg(target_os = "linux")]
impl LinuxHotkeyBackend {
    fn new() -> Result<Self, String> {
        let (command_tx, command_rx) = mpsc::channel::<LinuxHotkeyCommand>();
        let (event_tx, event_rx) = mpsc::channel::<HotkeyEventState>();

        let thread_handle = thread::spawn(move || {
            let manager = HandyHotkeyManager::new_with_blocking().or_else(|blocking_error| {
                tracing::warn!(
                    "Failed to initialize handy-keys in blocking mode: {blocking_error}. Falling back to non-blocking mode."
                );
                HandyHotkeyManager::new()
            });

            let manager = match manager {
                Ok(manager) => manager,
                Err(error) => {
                    tracing::error!("Failed to initialize handy-keys hotkey manager: {error}");
                    return;
                }
            };

            let mut registered_id: Option<HandyHotkeyId> = None;

            loop {
                while let Some(event) = manager.try_recv() {
                    if Some(event.id) != registered_id {
                        continue;
                    }

                    let mapped = match event.state {
                        HandyHotkeyState::Pressed => HotkeyEventState::Pressed,
                        HandyHotkeyState::Released => HotkeyEventState::Released,
                    };

                    if event_tx.send(mapped).is_err() {
                        return;
                    }
                }

                match command_rx.recv_timeout(Duration::from_millis(10)) {
                    Ok(LinuxHotkeyCommand::Register { shortcut, response }) => {
                        let result = (|| {
                            if let Some(id) = registered_id.take() {
                                manager
                                    .unregister(id)
                                    .map_err(|error| format!("Failed to unregister shortcut: {error}"))?;
                            }

                            let hotkey = shortcut
                                .parse::<HandyHotkey>()
                                .map_err(|error| format!("Invalid shortcut: {error}"))?;
                            let id = manager
                                .register(hotkey)
                                .map_err(|error| format!("Failed to register: {error}"))?;
                            registered_id = Some(id);
                            Ok(())
                        })();

                        let _ = response.send(result);
                    }
                    Ok(LinuxHotkeyCommand::Unregister { response }) => {
                        let result = if let Some(id) = registered_id.take() {
                            manager
                                .unregister(id)
                                .map_err(|error| format!("Failed to unregister shortcut: {error}"))
                        } else {
                            Ok(())
                        };
                        let _ = response.send(result);
                    }
                    Ok(LinuxHotkeyCommand::Shutdown) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        Ok(Self {
            command_tx,
            event_rx,
            thread_handle: Some(thread_handle),
        })
    }

    fn register(&self, shortcut: &str) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.command_tx
            .send(LinuxHotkeyCommand::Register {
                shortcut: shortcut.trim().to_string(),
                response: tx,
            })
            .map_err(|_| "Failed to send register command".to_string())?;
        rx.recv()
            .map_err(|_| "Failed to receive register response".to_string())?
    }

    fn unregister(&self) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.command_tx
            .send(LinuxHotkeyCommand::Unregister { response: tx })
            .map_err(|_| "Failed to send unregister command".to_string())?;
        rx.recv()
            .map_err(|_| "Failed to receive unregister response".to_string())?
    }

    fn try_recv(&self) -> Option<HotkeyEventState> {
        self.event_rx.try_recv().ok()
    }
}

#[cfg(target_os = "linux")]
impl Drop for LinuxHotkeyBackend {
    fn drop(&mut self) {
        let _ = self.command_tx.send(LinuxHotkeyCommand::Shutdown);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
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
    engine: EngineKind,
    accuracy: f32,
    speed: f32,
    recommended: bool,
    languages: &'static str,
    is_directory: bool,
}

const MODEL_CATALOG: &[CatalogModel] = &[
    CatalogModel {
        id: "parakeet-tdt-0.6b-v3",
        name: "Parakeet V3",
        description: "Fast and accurate, 25 European languages",
        filename: "parakeet-tdt-0.6b-v3-int8",
        url: "https://blob.handy.computer/parakeet-v3-int8.tar.gz",
        size_mb: 456,
        engine: EngineKind::Parakeet,
        accuracy: 0.80,
        speed: 0.85,
        recommended: true,
        languages: "25 EU languages",
        is_directory: true,
    },
    CatalogModel {
        id: "parakeet-tdt-0.6b-v2",
        name: "Parakeet V2",
        description: "Best for English speakers",
        filename: "parakeet-tdt-0.6b-v2-int8",
        url: "https://blob.handy.computer/parakeet-v2-int8.tar.gz",
        size_mb: 451,
        engine: EngineKind::Parakeet,
        accuracy: 0.85,
        speed: 0.85,
        recommended: false,
        languages: "English",
        is_directory: true,
    },
    CatalogModel {
        id: "whisper-small",
        name: "Whisper Small",
        description: "Fast, fairly accurate, 99 languages",
        filename: "ggml-small.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        size_mb: 465,
        engine: EngineKind::Whisper,
        accuracy: 0.60,
        speed: 0.85,
        recommended: false,
        languages: "99 languages",
        is_directory: false,
    },
    CatalogModel {
        id: "whisper-medium",
        name: "Whisper Medium",
        description: "Good accuracy, medium speed",
        filename: "whisper-medium-q4_1.bin",
        url: "https://blob.handy.computer/whisper-medium-q4_1.bin",
        size_mb: 469,
        engine: EngineKind::Whisper,
        accuracy: 0.75,
        speed: 0.60,
        recommended: false,
        languages: "99 languages",
        is_directory: false,
    },
    CatalogModel {
        id: "whisper-turbo",
        name: "Whisper Turbo",
        description: "Balanced accuracy and speed",
        filename: "ggml-large-v3-turbo.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin",
        size_mb: 1549,
        engine: EngineKind::Whisper,
        accuracy: 0.80,
        speed: 0.40,
        recommended: false,
        languages: "99 languages",
        is_directory: false,
    },
    CatalogModel {
        id: "whisper-large",
        name: "Whisper Large",
        description: "Highest accuracy, slowest",
        filename: "ggml-large-v3-q5_0.bin",
        url: "https://blob.handy.computer/ggml-large-v3-q5_0.bin",
        size_mb: 1031,
        engine: EngineKind::Whisper,
        accuracy: 0.85,
        speed: 0.30,
        recommended: false,
        languages: "99 languages",
        is_directory: false,
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

// ── UI helpers ───────────────────────────────────────────────────────────────

fn section_heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(12.0);
    ui.label(
        RichText::new(text)
            .size(11.0)
            .color(TEXT_MUTED)
            .strong(),
    );
    ui.add_space(2.0);
}

/// A setting row matching the Tauri layout: label on left, content on right.
fn setting_row(ui: &mut egui::Ui, bg: Color32, add_contents: impl FnOnce(&mut egui::Ui)) {
    Frame::new()
        .fill(bg)
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(16, 10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add_contents(ui);
        });
}

/// Two-column setting row: label left, widget right.
fn setting_row_lr(
    ui: &mut egui::Ui,
    bg: Color32,
    label: &str,
    add_right: impl FnOnce(&mut egui::Ui),
) {
    setting_row(ui, bg, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).size(14.0).color(TEXT));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), add_right);
        });
    });
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_date(days: u64) -> (u64, u64, u64) {
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

fn animated_dots(t: f64) -> &'static str {
    match ((t * 3.0) as usize) % 4 {
        1 => ".",
        2 => "..",
        3 => "...",
        _ => "",
    }
}

// ── Application ──────────────────────────────────────────────────────────────

struct ShadowwordApp {
    config: ShadowwordConfig,
    prev_config: ShadowwordConfig,
    local_service: Arc<LocalService>,
    active_recording: Option<RecordingSession>,
    response_rx: Option<Receiver<Result<TranscriptResponse, String>>>,
    available_inputs: Vec<InputDeviceInfo>,
    remote_status: Option<ServiceStatus>,
    connection_test: Option<ConnectionTestResult>,
    transcript: String,
    history: Vec<HistoryEntry>,
    status_line: String,
    page: Page,
    #[cfg(not(target_os = "linux"))]
    hotkey_manager: Option<GlobalHotKeyManager>,
    #[cfg(not(target_os = "linux"))]
    registered_hotkey: Option<HotKey>,
    #[cfg(target_os = "linux")]
    hotkey_manager: Option<LinuxHotkeyBackend>,
    editing_shortcut: bool,
    shortcut_error: Option<String>,
    active_downloads: HashMap<String, ActiveDownload>,
    preloading: bool,
    preload_rx: Option<Receiver<Result<(), String>>>,
    models_dir: Option<std::path::PathBuf>,
}

impl ShadowwordApp {
    #[cfg(not(target_os = "linux"))]
    fn register_hotkey(
        manager: &GlobalHotKeyManager,
        shortcut: &str,
    ) -> Result<HotKey, String> {
        let shortcut = shortcut.trim();
        let hotkey = shortcut
            .parse::<HotKey>()
            .map_err(|error| format!("Invalid shortcut: {error}"))?;
        manager
            .register(hotkey)
            .map_err(|error| format!("Failed to register: {error}"))?;
        Ok(hotkey)
    }

    fn new(config: ShadowwordConfig) -> Self {
        let local_service = Arc::new(LocalService::new(config.clone()));
        let available_inputs = MicrophoneRecorder::list_input_devices().unwrap_or_default();

        #[cfg(not(target_os = "linux"))]
        let (hotkey_manager, registered_hotkey, shortcut_error) = match GlobalHotKeyManager::new() {
            Ok(manager) => {
                let registered_hotkey = match Self::register_hotkey(&manager, &config.hotkey.shortcut) {
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
            available_inputs,
            remote_status: None,
            connection_test: None,
            transcript: String::new(),
            history: Vec::new(),
            status_line: "Ready".to_string(),
            page: Page::General,
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

    // ── Business logic ──────────────────────────────────────────────────────

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

    fn test_remote_connection(&mut self) {
        self.connection_test = None;
        match remote_status(&self.config) {
            Ok(status) => {
                self.connection_test = Some(ConnectionTestResult {
                    success: true,
                    message: format!(
                        "Connected — engine: {:?}, model loaded: {}, sample rate: {}Hz",
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
            self.config.engine = model.engine;
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
        let is_directory = model.is_directory;
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

                if is_directory {
                    let tmp_path = dest.with_extension("tar.gz");
                    let mut file = std::fs::File::create(&tmp_path)
                        .context("failed to create temp file")?;
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
                    drop(file);
                    let tar_file = std::fs::File::open(&tmp_path)?;
                    let decoder = flate2::read::GzDecoder::new(tar_file);
                    let mut archive = tar::Archive::new(decoder);
                    let parent = dest.parent().context("no parent dir")?;
                    archive.unpack(parent).context("failed to extract tar.gz")?;
                    let _ = std::fs::remove_file(&tmp_path);
                } else {
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

    fn poll_hotkey_events(&mut self) {
        if self.hotkey_manager.is_none() {
            return;
        }

        #[cfg(not(target_os = "linux"))]
        {
        let receiver = GlobalHotKeyEvent::receiver();
        while let Ok(event) = receiver.try_recv() {
            if Some(event.id) != self.registered_hotkey.map(|h| h.id()) {
                continue;
            }

            let transcribing = self.response_rx.is_some();
            if transcribing {
                continue;
            }

            let recording = self.active_recording.is_some();

            if self.config.hotkey.push_to_talk {
                // Push-to-talk: press = start, release = stop
                match event.state {
                    HotKeyState::Pressed if !recording => self.start_recording(),
                    HotKeyState::Released if recording => self.stop_recording(),
                    _ => {}
                }
            } else {
                // Toggle: press = toggle start/stop
                if event.state == HotKeyState::Pressed {
                    if recording {
                        self.stop_recording();
                    } else {
                        self.start_recording();
                    }
                }
            }
        }
        }

        #[cfg(target_os = "linux")]
        while let Some(event) = self.hotkey_manager.as_ref().and_then(|manager| manager.try_recv()) {
            let transcribing = self.response_rx.is_some();
            if transcribing {
                continue;
            }

            let recording = self.active_recording.is_some();

            if self.config.hotkey.push_to_talk {
                match event {
                    HotkeyEventState::Pressed if !recording => self.start_recording(),
                    HotkeyEventState::Released if recording => self.stop_recording(),
                    _ => {}
                }
            } else if matches!(event, HotkeyEventState::Pressed) {
                if recording {
                    self.stop_recording();
                } else {
                    self.start_recording();
                }
            }
        }
    }

    fn re_register_hotkey(&mut self) {
        let Some(manager) = &self.hotkey_manager else {
            return;
        };

        let shortcut = self.config.hotkey.shortcut.trim().to_string();
        if shortcut.is_empty() {
            self.shortcut_error = Some("Shortcut cannot be empty".to_string());
            return;
        }

        #[cfg(not(target_os = "linux"))]
        {
        // Unregister old
        if let Some(old) = self.registered_hotkey.take() {
            let _ = manager.unregister(old);
        }

        match Self::register_hotkey(manager, &shortcut) {
            Ok(hk) => {
                self.registered_hotkey = Some(hk);
                self.shortcut_error = None;
            }
            Err(error) => self.shortcut_error = Some(error),
        }
        }

        #[cfg(target_os = "linux")]
        {
            match manager.unregister().and_then(|_| manager.register(&shortcut)) {
                Ok(()) => self.shortcut_error = None,
                Err(error) => self.shortcut_error = Some(error),
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

                let timestamp = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| {
                        let secs = d.as_secs();
                        let hours = (secs / 3600) % 24;
                        let mins = (secs / 60) % 60;
                        let days = secs / 86400;
                        let (y, m, d) = days_to_date(days);
                        format!("{y}-{m:02}-{d:02} {hours:02}:{mins:02}")
                    })
                    .unwrap_or_else(|_| "unknown".to_string());

                self.history.insert(
                    0,
                    HistoryEntry {
                        text: response.text,
                        engine: response.engine,
                        elapsed_ms: response.elapsed_ms,
                        timestamp,
                    },
                );

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

    fn has_background_work(&self) -> bool {
        self.active_recording.is_some()
            || self.response_rx.is_some()
            || self.preloading
            || !self.active_downloads.is_empty()
    }

    // ── Sidebar ─────────────────────────────────────────────────────────────

    fn show_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .frame(
                Frame::new()
                    .fill(BG_SIDEBAR)
                    .inner_margin(Margin::symmetric(0, 0)),
            )
            .exact_width(150.0)
            .resizable(false)
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.set_min_width(ui.available_width());

                // Logo
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("SHADOW")
                            .size(22.0)
                            .color(ACCENT)
                            .strong(),
                    );
                    ui.label(
                        RichText::new("WORD")
                            .size(12.0)
                            .color(TEXT_DIM)
                            .strong(),
                    );
                });
                ui.add_space(24.0);

                // Nav items
                let nav_items = [
                    (Page::General, "General"),
                    (Page::Models, "Models"),
                    (Page::History, "History"),
                    (Page::Settings, "Settings"),
                    (Page::About, "About"),
                ];

                for (page, label) in nav_items {
                    let active = self.page == page;
                    let bg = if active { NAV_ACTIVE } else { BG_SIDEBAR };
                    let text_color = if active { ACCENT } else { TEXT_DIM };

                    let btn = Frame::new()
                        .fill(bg)
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(Margin::symmetric(16, 8))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.label(
                                RichText::new(label).size(14.0).color(text_color),
                            );
                        });

                    if btn.response.interact(egui::Sense::click()).clicked() {
                        self.page = page;
                    }

                    if !active && btn.response.interact(egui::Sense::hover()).hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                }

                // Active model at bottom
                let inferring = self.response_rx.is_some();
                let loaded = self.local_service.is_loaded();
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(8.0);
                    let model_name = MODEL_CATALOG
                        .iter()
                        .find(|m| {
                            self.model_path_for(m)
                                .map(|p| p == self.config.model_path)
                                .unwrap_or(false)
                        })
                        .map(|m| m.name)
                        .unwrap_or("No model");
                    Frame::new()
                        .inner_margin(Margin::symmetric(12, 6))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let dot_color = if inferring {
                                    Color32::from_rgb(60, 120, 255)
                                } else if loaded {
                                    STATUS_OK
                                } else {
                                    REC_RED
                                };
                                let (dot_rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                                ui.painter()
                                    .circle_filled(dot_rect.center(), 4.0, dot_color);
                                ui.label(
                                    RichText::new(model_name)
                                        .size(11.0)
                                        .color(TEXT_DIM),
                                );
                            });
                        });
                });
            });
    }

    // ── Footer / status bar ─────────────────────────────────────────────────

    fn show_footer(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("footer")
            .frame(
                Frame::new()
                    .fill(BG_SIDEBAR)
                    .inner_margin(Margin::symmetric(16, 8)),
            )
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(&self.status_line)
                            .size(11.0)
                            .color(TEXT_MUTED),
                    );
                });
            });
    }

    // ── Page: General ───────────────────────────────────────────────────────

    fn show_general_page(&mut self, ui: &mut egui::Ui) {
        let time = ui.ctx().input(|i| i.time);
        let recording = self.active_recording.is_some();
        let transcribing = self.response_rx.is_some();

        if recording || transcribing {
            ui.ctx().request_repaint();
        }

        section_heading(ui, "RECORD");

        // Record button area
        ui.add_space(16.0);
        ui.vertical_centered(|ui| {
            let radius: f32 = 36.0;
            let size = Vec2::splat(radius * 2.0 + 32.0);
            let (resp, painter) = ui.allocate_painter(size, egui::Sense::click());
            let center = resp.rect.center();

            // Outer ring / glow
            if recording {
                let pulse = ((time * 2.5).sin() * 0.5 + 0.5) as f32;
                painter.circle_filled(
                    center,
                    radius + 4.0 + pulse * 8.0,
                    Color32::from_rgba_unmultiplied(200, 30, 30, (20.0 + pulse * 40.0) as u8),
                );
            } else if resp.hovered() && !transcribing {
                painter.circle_filled(
                    center,
                    radius + 5.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 15),
                );
            }

            // Main circle
            let fill = if recording {
                REC_RED
            } else if transcribing {
                TEXT_DIM
            } else if resp.hovered() {
                ACCENT
            } else {
                Color32::from_rgb(200, 200, 200)
            };
            painter.circle_filled(center, radius, fill);

            // Inner icon
            let icon_color = Color32::from_rgb(12, 12, 12);
            if recording {
                painter.rect_filled(
                    egui::Rect::from_center_size(center, Vec2::splat(20.0)),
                    CornerRadius::same(3),
                    icon_color,
                );
            } else {
                painter.circle_filled(center, 12.0, icon_color);
            }

            // Click
            if resp.clicked() && !transcribing {
                if recording {
                    self.stop_recording();
                } else {
                    self.start_recording();
                }
            }

            // Label
            ui.add_space(10.0);
            let dots = animated_dots(time);
            let label = if recording {
                RichText::new(format!("Recording{dots}"))
                    .color(REC_RED)
                    .size(13.0)
            } else if transcribing {
                RichText::new(format!("Transcribing{dots}"))
                    .color(TEXT_DIM)
                    .size(13.0)
            } else {
                RichText::new("Click to record")
                    .color(TEXT_MUTED)
                    .size(13.0)
            };
            ui.label(label);
        });

        ui.add_space(16.0);

        section_heading(ui, "TRANSCRIPT");

        Frame::new()
            .fill(BG_ROW)
            .corner_radius(CornerRadius::same(4))
            .stroke(Stroke::new(1.0, BORDER))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.transcript)
                                .font(egui::TextStyle::Monospace)
                                .desired_rows(8)
                                .desired_width(f32::INFINITY)
                                .frame(false)
                                .hint_text(
                                    RichText::new("Transcript will appear here...")
                                        .color(TEXT_MUTED),
                                ),
                        );
                    });
            });
    }

    // ── Page: Settings ──────────────────────────────────────────────────────

    fn show_settings_page(&mut self, ui: &mut egui::Ui) {
        // ── MODE ────────────────────────────────────────────────────────
        section_heading(ui, "MODE");

        setting_row_lr(ui, BG_ROW, "Service Mode", |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.config.mode,
                    ServiceMode::Local,
                    RichText::new("Local").size(13.0),
                );
                ui.selectable_value(
                    &mut self.config.mode,
                    ServiceMode::Remote,
                    RichText::new("Remote").size(13.0),
                );
            });
        });

        // ── HOTKEY ──────────────────────────────────────────────────────
        section_heading(ui, "HOTKEY");

        setting_row_lr(ui, BG_ROW, "Push To Talk", |ui| {
            ui.checkbox(&mut self.config.hotkey.push_to_talk, "");
        });

        setting_row(ui, BG_ROW_ALT, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Shortcut").size(14.0).color(TEXT));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.editing_shortcut {
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.config.hotkey.shortcut)
                                .desired_width(180.0)
                                .hint_text("e.g. ctrl+space"),
                        );
                        if resp.lost_focus() {
                            self.editing_shortcut = false;
                        } else {
                            resp.request_focus();
                        }
                    } else {
                        let label_text = if self.config.hotkey.shortcut.trim().is_empty() {
                            "Not set"
                        } else {
                            &self.config.hotkey.shortcut
                        };
                        if ui
                            .button(RichText::new(label_text).size(13.0).monospace())
                            .clicked()
                        {
                            self.editing_shortcut = true;
                        }
                    }
                });
            });
        });

        if let Some(err) = &self.shortcut_error {
            setting_row(ui, BG_ROW, |ui| {
                ui.label(RichText::new(err).size(12.0).color(REC_RED));
            });
        }

        // ── SOUND ───────────────────────────────────────────────────────
        section_heading(ui, "SOUND");

        let device_name = self.device_name().to_string();
        setting_row(ui, BG_ROW, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Microphone").size(14.0).color(TEXT));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Refresh").clicked() {
                        self.refresh_local_devices();
                    }
                    egui::ComboBox::new("mic_combo", "")
                        .selected_text(&device_name)
                        .width(220.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.config.recording.input_device,
                                None,
                                "System default",
                            );
                            for dev in &self.available_inputs {
                                let label = if dev.is_default {
                                    format!("{} (default)", dev.name)
                                } else {
                                    dev.name.clone()
                                };
                                ui.selectable_value(
                                    &mut self.config.recording.input_device,
                                    Some(dev.name.clone()),
                                    label,
                                );
                            }
                        });
                });
            });
        });

        setting_row_lr(ui, BG_ROW_ALT, "Sample Rate", |ui| {
            ui.add(
                egui::DragValue::new(&mut self.config.recording.sample_rate)
                    .range(8_000..=96_000)
                    .speed(100.0)
                    .suffix(" Hz"),
            );
        });

        // ── OUTPUT ──────────────────────────────────────────────────────
        section_heading(ui, "OUTPUT");

        setting_row_lr(ui, BG_ROW, "Copy to Clipboard", |ui| {
            ui.checkbox(&mut self.config.output.copy_to_clipboard, "");
        });

        setting_row_lr(ui, BG_ROW_ALT, "Active Window Output", |ui| {
            egui::ComboBox::new("paste_method", "")
                .selected_text(match self.config.output.paste_method {
                    PasteMethod::None => "None",
                    PasteMethod::Direct => "Direct",
                    PasteMethod::CtrlV => "Ctrl+V",
                    PasteMethod::CtrlShiftV => "Ctrl+Shift+V",
                    PasteMethod::ShiftInsert => "Shift+Insert",
                })
                .width(180.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.config.output.paste_method, PasteMethod::None, "None");
                    ui.selectable_value(&mut self.config.output.paste_method, PasteMethod::Direct, "Direct");
                    ui.selectable_value(&mut self.config.output.paste_method, PasteMethod::CtrlV, "Ctrl+V");
                    ui.selectable_value(&mut self.config.output.paste_method, PasteMethod::CtrlShiftV, "Ctrl+Shift+V");
                    ui.selectable_value(&mut self.config.output.paste_method, PasteMethod::ShiftInsert, "Shift+Insert");
                });
        });

        #[cfg(target_os = "linux")]
        setting_row_lr(ui, BG_ROW, "Typing Tool", |ui| {
            egui::ComboBox::new("typing_tool", "")
                .selected_text(match self.config.output.typing_tool {
                    TypingTool::Auto => "Auto",
                    TypingTool::Wtype => "wtype",
                    TypingTool::Kwtype => "kwtype",
                    TypingTool::Dotool => "dotool",
                    TypingTool::Ydotool => "ydotool",
                    TypingTool::Xdotool => "xdotool",
                })
                .width(180.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.config.output.typing_tool, TypingTool::Auto, "Auto");
                    ui.selectable_value(&mut self.config.output.typing_tool, TypingTool::Wtype, "wtype");
                    ui.selectable_value(&mut self.config.output.typing_tool, TypingTool::Kwtype, "kwtype");
                    ui.selectable_value(&mut self.config.output.typing_tool, TypingTool::Dotool, "dotool");
                    ui.selectable_value(&mut self.config.output.typing_tool, TypingTool::Ydotool, "ydotool");
                    ui.selectable_value(&mut self.config.output.typing_tool, TypingTool::Xdotool, "xdotool");
                });
        });

        setting_row_lr(ui, BG_ROW_ALT, "Paste Delay", |ui| {
            ui.add(
                egui::DragValue::new(&mut self.config.output.paste_delay_ms)
                    .range(0..=1000)
                    .speed(5.0)
                    .suffix(" ms"),
            );
        });

        // ── REMOTE (conditional) ────────────────────────────────────────
        if self.config.mode == ServiceMode::Remote {
            section_heading(ui, "REMOTE DAEMON");

            setting_row(ui, BG_ROW, |ui| {
                ui.label(RichText::new("Endpoint").size(14.0).color(TEXT));
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.config.remote.endpoint)
                        .desired_width(ui.available_width()),
                );
            });

            setting_row(ui, BG_ROW_ALT, |ui| {
                ui.label(RichText::new("Listen Address").size(14.0).color(TEXT));
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.config.daemon.listen_addr)
                        .desired_width(ui.available_width()),
                );
            });

            setting_row(ui, BG_ROW, |ui| {
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Test Connection").size(13.0)).clicked() {
                        self.test_remote_connection();
                    }
                    if ui.button(RichText::new("Pull Config").size(13.0)).clicked() {
                        self.pull_remote_config();
                    }
                    if ui.button(RichText::new("Push Config").size(13.0)).clicked() {
                        self.push_remote_config();
                    }
                });
            });

            if let Some(result) = &self.connection_test {
                let (color, icon) = if result.success {
                    (STATUS_OK, "OK")
                } else {
                    (REC_RED, "FAIL")
                };
                setting_row(ui, BG_ROW_ALT, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(icon).size(12.0).color(color).strong());
                        ui.label(RichText::new(&result.message).size(12.0).color(TEXT_DIM));
                    });
                });
            }
        }

    }

    // ── Page: Models ─────────────────────────────────────────────────────────

    fn show_models_page(&mut self, ui: &mut egui::Ui) {
        let is_preloading = self.preloading;
        let model_loaded = self.local_service.is_loaded();

        // ── Acceleration ────────────────────────────────────────────────
        section_heading(ui, "ACCELERATION");

        setting_row_lr(ui, BG_ROW, "ORT Accelerator", |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.config.ort_accelerator, OrtxAccelerator::Auto, "Auto");
                ui.selectable_value(&mut self.config.ort_accelerator, OrtxAccelerator::Cpu, "CPU");
                ui.selectable_value(&mut self.config.ort_accelerator, OrtxAccelerator::Cuda, "CUDA");
            });
        });

        setting_row_lr(ui, BG_ROW_ALT, "ONNX Quantization", |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.config.onnx_quantization, OnnxQuantization::Fp32, "FP32");
                ui.selectable_value(&mut self.config.onnx_quantization, OnnxQuantization::Fp16, "FP16");
                ui.selectable_value(&mut self.config.onnx_quantization, OnnxQuantization::Int8, "INT8");
                ui.selectable_value(&mut self.config.onnx_quantization, OnnxQuantization::Int4, "INT4");
            });
        });

        setting_row_lr(ui, BG_ROW, "Whisper Accelerator", |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.config.whisper_accelerator, WhisperAccelerator::Auto, "Auto");
                ui.selectable_value(&mut self.config.whisper_accelerator, WhisperAccelerator::Cpu, "CPU");
                ui.selectable_value(&mut self.config.whisper_accelerator, WhisperAccelerator::Gpu, "GPU");
            });
        });

        setting_row_lr(ui, BG_ROW_ALT, "Preload on Startup", |ui| {
            ui.checkbox(&mut self.config.preload_on_startup, "");
        });

        setting_row_lr(ui, BG_ROW, "Model in Memory", |ui| {
            if model_loaded {
                ui.label(RichText::new("Loaded").size(12.0).color(STATUS_OK).strong());
            } else if is_preloading {
                ui.ctx().request_repaint();
                ui.label(RichText::new("Loading...").size(12.0).color(TEXT_DIM));
            } else if ui.button(RichText::new("Preload").size(12.0)).clicked() {
                self.start_preload();
            }
        });

        // ── Models ──────────────────────────────────────────────────────
        section_heading(ui, "MODELS");

        let current_path = self.config.model_path.clone();

        // Snapshot download state to avoid borrow issues
        let dl_state: HashMap<String, (u64, u64)> = self
            .active_downloads
            .iter()
            .map(|(id, dl)| (id.clone(), (dl.downloaded, dl.total)))
            .collect();
        let has_active_downloads = !dl_state.is_empty();
        if has_active_downloads {
            ui.ctx().request_repaint();
        }

        let mut action: Option<(&'static str, &'static str)> = None;

        for model in MODEL_CATALOG {
            let installed = self.is_model_installed(model);
            let is_active = self
                .model_path_for(model)
                .map(|p| p == current_path)
                .unwrap_or(false);
            let dl = dl_state.get(model.id);
            let is_downloading = dl.is_some();

            let bg = if is_active { BG_HOVER } else { BG_ROW };
            let border = if is_active {
                ACCENT
            } else if model.recommended {
                TEXT_DIM
            } else {
                BORDER
            };

            Frame::new()
                .fill(bg)
                .corner_radius(CornerRadius::same(4))
                .stroke(Stroke::new(1.0, border))
                .inner_margin(Margin::symmetric(14, 8))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());

                    // Single compact row: name + bars + status/action
                    ui.horizontal(|ui| {
                        // Left: name + badges
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(model.name).size(13.0).color(TEXT).strong());
                                if model.recommended {
                                    ui.label(RichText::new("REC").size(9.0).color(ACCENT).strong());
                                }
                                if is_active {
                                    ui.label(RichText::new("ACTIVE").size(9.0).color(STATUS_OK).strong());
                                }
                            });
                            // Speed/quality bars inline
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                ui.label(RichText::new("Spd").size(10.0).color(TEXT_MUTED));
                                let (r, _) =
                                    ui.allocate_exact_size(Vec2::new(60.0, 6.0), egui::Sense::hover());
                                ui.painter().rect_filled(r, CornerRadius::same(3), BG_INPUT);
                                let mut f = r;
                                f.set_right(r.left() + r.width() * model.speed);
                                ui.painter().rect_filled(f, CornerRadius::same(3), ACCENT);

                                ui.label(RichText::new("Qual").size(10.0).color(TEXT_MUTED));
                                let (r, _) =
                                    ui.allocate_exact_size(Vec2::new(60.0, 6.0), egui::Sense::hover());
                                ui.painter().rect_filled(r, CornerRadius::same(3), BG_INPUT);
                                let mut f = r;
                                f.set_right(r.left() + r.width() * model.accuracy);
                                ui.painter().rect_filled(f, CornerRadius::same(3), ACCENT);

                                ui.label(
                                    RichText::new(format!("{} MB", model.size_mb))
                                        .size(10.0)
                                        .color(TEXT_MUTED),
                                );
                                ui.label(
                                    RichText::new(model.languages).size(10.0).color(TEXT_MUTED),
                                );
                            });
                        });

                        // Right: actions
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if is_downloading {
                                let (downloaded, total) = dl.copied().unwrap_or((0, 0));
                                let dl_mb = downloaded as f32 / (1024.0 * 1024.0);
                                let total_mb = total as f32 / (1024.0 * 1024.0);
                                let progress = if total > 0 {
                                    downloaded as f32 / total as f32
                                } else {
                                    0.0
                                };

                                ui.vertical(|ui| {
                                    let (bar_rect, _) = ui.allocate_exact_size(
                                        Vec2::new(120.0, 5.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(
                                        bar_rect,
                                        CornerRadius::same(2),
                                        BG_INPUT,
                                    );
                                    let mut filled = bar_rect;
                                    filled.set_right(
                                        bar_rect.left() + bar_rect.width() * progress,
                                    );
                                    ui.painter().rect_filled(
                                        filled,
                                        CornerRadius::same(2),
                                        ACCENT,
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "{:.0}/{:.0} MB",
                                            dl_mb, total_mb
                                        ))
                                        .size(10.0)
                                        .color(TEXT_DIM),
                                    );
                                });
                            } else if installed {
                                if ui
                                    .small_button(RichText::new("Delete").size(11.0))
                                    .clicked()
                                {
                                    action = Some(("delete", model.id));
                                }
                                if !is_active
                                    && ui
                                        .small_button(RichText::new("Select").size(11.0))
                                        .clicked()
                                {
                                    action = Some(("select", model.id));
                                }
                            } else {
                                if ui
                                    .small_button(RichText::new("Download").size(11.0))
                                    .clicked()
                                {
                                    action = Some(("download", model.id));
                                }
                            }
                        });
                    });
                });
            ui.add_space(2.0);
        }

        if let Some((act, id)) = action {
            match act {
                "select" => {
                    if let Some(model) = MODEL_CATALOG.iter().find(|m| m.id == id) {
                        self.select_model(model);
                    }
                }
                "download" => self.start_download(id),
                "delete" => {
                    if let Some(model) = MODEL_CATALOG.iter().find(|m| m.id == id) {
                        if let Some(path) = self.model_path_for(model) {
                            if model.is_directory {
                                let _ = std::fs::remove_dir_all(&path);
                            } else {
                                let _ = std::fs::remove_file(&path);
                            }
                            self.status_line = format!("Deleted {}", model.name);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // ── Page: History ───────────────────────────────────────────────────────

    fn show_history_page(&mut self, ui: &mut egui::Ui) {
        section_heading(ui, "HISTORY");

        if self.history.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("No transcriptions yet")
                        .size(14.0)
                        .color(TEXT_MUTED),
                );
            });
            return;
        }

        let mut to_delete: Option<usize> = None;
        let mut to_copy: Option<usize> = None;

        for (i, entry) in self.history.iter().enumerate() {
            let bg = if i % 2 == 0 { BG_ROW } else { BG_ROW_ALT };

            Frame::new()
                .fill(bg)
                .corner_radius(CornerRadius::same(4))
                .inner_margin(Margin::symmetric(16, 10))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());

                    // Header: timestamp + actions
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&entry.timestamp)
                                .size(12.0)
                                .color(TEXT_DIM)
                                .strong(),
                        );

                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui
                                    .small_button(RichText::new("Delete").size(11.0))
                                    .clicked()
                                {
                                    to_delete = Some(i);
                                }
                                if ui
                                    .small_button(RichText::new("Copy").size(11.0))
                                    .clicked()
                                {
                                    to_copy = Some(i);
                                }
                                ui.label(
                                    RichText::new(format!(
                                        "{:?} - {}ms",
                                        entry.engine, entry.elapsed_ms
                                    ))
                                    .size(11.0)
                                    .color(TEXT_MUTED),
                                );
                            },
                        );
                    });

                    ui.add_space(4.0);

                    // Transcript text
                    ui.label(
                        RichText::new(&entry.text)
                            .size(13.0)
                            .color(TEXT)
                            .italics(),
                    );
                });
        }

        // Apply actions after iteration
        if let Some(i) = to_copy {
            if let Some(entry) = self.history.get(i) {
                if let Ok(mut clipboard) = Clipboard::new() {
                    let _ = clipboard.set_text(entry.text.clone());
                    self.status_line = "Copied to clipboard".to_string();
                }
            }
        }
        if let Some(i) = to_delete {
            self.history.remove(i);
        }
    }

    // ── Page: About ─────────────────────────────────────────────────────────

    fn show_about_page(&self, ui: &mut egui::Ui) {
        section_heading(ui, "ABOUT");

        setting_row_lr(ui, BG_ROW, "Version", |ui| {
            ui.label(RichText::new("0.8.0").size(14.0).color(TEXT_DIM));
        });

        setting_row_lr(ui, BG_ROW_ALT, "Engine", |ui| {
            ui.label(
                RichText::new(format!("{:?}", self.config.engine))
                    .size(14.0)
                    .color(TEXT_DIM),
            );
        });

        setting_row(ui, BG_ROW, |ui| {
            ui.label(RichText::new("Config Directory").size(14.0).color(TEXT));
            ui.add_space(4.0);
            let config_dir = ShadowwordConfig::config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "unknown".to_string());
            Frame::new()
                .fill(BG_INPUT)
                .corner_radius(CornerRadius::same(4))
                .inner_margin(Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(config_dir)
                            .size(12.0)
                            .color(TEXT_DIM)
                            .monospace(),
                    );
                });
        });

        section_heading(ui, "ACKNOWLEDGEMENTS");

        setting_row(ui, BG_ROW, |ui| {
            ui.label(RichText::new("Whisper.cpp").size(14.0).color(TEXT).strong());
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Shadow Word uses Whisper.cpp for fast, local speech-to-text processing.",
                )
                .size(13.0)
                .color(TEXT_DIM),
            );
        });
    }
}

// ── eframe integration ──────────────────────────────────────────────────────

impl eframe::App for ShadowwordApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.hotkey_manager.is_some() {
            ctx.request_repaint_after(Duration::from_millis(33));
        }

        if self.has_background_work() {
            ctx.request_repaint_after(Duration::from_millis(16));
        }

        self.poll_hotkey_events();
        self.poll_background_work();
        self.poll_downloads();
        self.poll_preload();
        self.auto_save_if_changed();
        self.show_sidebar(ctx);
        self.show_footer(ctx);

        egui::CentralPanel::default()
            .frame(Frame::new().fill(BG).inner_margin(Margin::symmetric(24, 12)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    match self.page {
                        Page::General => self.show_general_page(ui),
                        Page::Models => self.show_models_page(ui),
                        Page::History => self.show_history_page(ui),
                        Page::Settings => self.show_settings_page(ui),
                        Page::About => self.show_about_page(ui),
                    }
                });
            });
    }
}

// ── Remote helpers ──────────────────────────────────────────────────────────

fn remote_transcribe(config: &ShadowwordConfig, audio: AudioInput) -> Result<TranscriptResponse> {
    let client = reqwest::blocking::Client::new();
    let wav = shadoword_core::wav::encode_wav(&audio)?;
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

#[cfg(target_os = "linux")]
fn is_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
        || matches!(std::env::var("XDG_SESSION_TYPE").ok().as_deref(), Some("wayland"))
}

#[cfg(target_os = "linux")]
fn is_kde_wayland() -> bool {
    is_wayland()
        && std::env::var("XDG_CURRENT_DESKTOP")
            .map(|value| value.to_lowercase().contains("kde"))
            .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn command_exists(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn write_clipboard_via_wl_copy(text: &str) -> Result<()> {
    let status = Command::new("wl-copy")
        .arg("--")
        .arg(text)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("failed to execute wl-copy")?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("wl-copy failed"))
    }
}

#[cfg(target_os = "linux")]
fn type_text_via_wtype(text: &str) -> Result<()> {
    let output = Command::new("wtype")
        .arg("--")
        .arg(text)
        .output()
        .context("failed to execute wtype")?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!("wtype failed: {}", String::from_utf8_lossy(&output.stderr).trim()))
    }
}

#[cfg(target_os = "linux")]
fn type_text_via_kwtype(text: &str) -> Result<()> {
    let output = Command::new("kwtype")
        .arg("--")
        .arg(text)
        .output()
        .context("failed to execute kwtype")?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!("kwtype failed: {}", String::from_utf8_lossy(&output.stderr).trim()))
    }
}

#[cfg(target_os = "linux")]
fn type_text_via_xdotool(text: &str) -> Result<()> {
    let output = Command::new("xdotool")
        .arg("type")
        .arg("--clearmodifiers")
        .arg("--")
        .arg(text)
        .output()
        .context("failed to execute xdotool")?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!("xdotool failed: {}", String::from_utf8_lossy(&output.stderr).trim()))
    }
}

#[cfg(target_os = "linux")]
fn type_text_via_ydotool(text: &str) -> Result<()> {
    let output = Command::new("ydotool")
        .arg("type")
        .arg("--")
        .arg(text)
        .output()
        .context("failed to execute ydotool")?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!("ydotool failed: {}", String::from_utf8_lossy(&output.stderr).trim()))
    }
}

#[cfg(target_os = "linux")]
fn type_text_via_dotool(text: &str) -> Result<()> {
    use std::io::Write;

    let mut child = Command::new("dotool")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn dotool")?;
    if let Some(mut stdin) = child.stdin.take() {
        writeln!(stdin, "type {}", text).context("failed to write to dotool stdin")?;
    }
    let output = child.wait_with_output().context("failed to wait for dotool")?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!("dotool failed: {}", String::from_utf8_lossy(&output.stderr).trim()))
    }
}

#[cfg(target_os = "linux")]
fn try_direct_typing_linux(text: &str, preferred_tool: TypingTool) -> Result<bool> {
    let try_tool = |tool: TypingTool| -> Result<bool> {
        match tool {
            TypingTool::Wtype if command_exists("wtype") => {
                type_text_via_wtype(text)?;
                Ok(true)
            }
            TypingTool::Kwtype if command_exists("kwtype") => {
                type_text_via_kwtype(text)?;
                Ok(true)
            }
            TypingTool::Dotool if command_exists("dotool") => {
                type_text_via_dotool(text)?;
                Ok(true)
            }
            TypingTool::Ydotool if command_exists("ydotool") => {
                type_text_via_ydotool(text)?;
                Ok(true)
            }
            TypingTool::Xdotool if command_exists("xdotool") => {
                type_text_via_xdotool(text)?;
                Ok(true)
            }
            TypingTool::Auto => Ok(false),
            _ => Err(anyhow!("Requested typing tool is not available")),
        }
    };

    if preferred_tool != TypingTool::Auto {
        return try_tool(preferred_tool);
    }

    if is_wayland() {
        if is_kde_wayland() && command_exists("kwtype") {
            type_text_via_kwtype(text)?;
            return Ok(true);
        }
        if !is_kde_wayland() && command_exists("wtype") {
            type_text_via_wtype(text)?;
            return Ok(true);
        }
        if command_exists("dotool") {
            type_text_via_dotool(text)?;
            return Ok(true);
        }
        if command_exists("ydotool") {
            type_text_via_ydotool(text)?;
            return Ok(true);
        }
    } else {
        if command_exists("xdotool") {
            type_text_via_xdotool(text)?;
            return Ok(true);
        }
        if command_exists("ydotool") {
            type_text_via_ydotool(text)?;
            return Ok(true);
        }
    }

    Ok(false)
}

fn send_paste_ctrl_v(enigo: &mut Enigo) -> Result<()> {
    #[cfg(target_os = "macos")]
    let (modifier_key, v_key_code) = (Key::Meta, Key::Other(9));
    #[cfg(target_os = "windows")]
    let (modifier_key, v_key_code) = (Key::Control, Key::Other(0x56));
    #[cfg(target_os = "linux")]
    let (modifier_key, v_key_code) = (Key::Control, Key::Unicode('v'));

    enigo.key(modifier_key, Direction::Press).context("failed to press modifier")?;
    enigo.key(v_key_code, Direction::Click).context("failed to click V")?;
    std::thread::sleep(Duration::from_millis(100));
    enigo.key(modifier_key, Direction::Release).context("failed to release modifier")?;
    Ok(())
}

fn send_paste_ctrl_shift_v(enigo: &mut Enigo) -> Result<()> {
    #[cfg(target_os = "macos")]
    let (modifier_key, v_key_code) = (Key::Meta, Key::Other(9));
    #[cfg(target_os = "windows")]
    let (modifier_key, v_key_code) = (Key::Control, Key::Other(0x56));
    #[cfg(target_os = "linux")]
    let (modifier_key, v_key_code) = (Key::Control, Key::Unicode('v'));

    enigo.key(modifier_key, Direction::Press).context("failed to press modifier")?;
    enigo.key(Key::Shift, Direction::Press).context("failed to press shift")?;
    enigo.key(v_key_code, Direction::Click).context("failed to click V")?;
    std::thread::sleep(Duration::from_millis(100));
    enigo.key(Key::Shift, Direction::Release).context("failed to release shift")?;
    enigo.key(modifier_key, Direction::Release).context("failed to release modifier")?;
    Ok(())
}

fn send_paste_shift_insert(enigo: &mut Enigo) -> Result<()> {
    #[cfg(target_os = "windows")]
    let insert_key_code = Key::Other(0x2D);
    #[cfg(not(target_os = "windows"))]
    let insert_key_code = Key::Other(0x76);

    enigo.key(Key::Shift, Direction::Press).context("failed to press shift")?;
    enigo.key(insert_key_code, Direction::Click).context("failed to click insert")?;
    std::thread::sleep(Duration::from_millis(100));
    enigo.key(Key::Shift, Direction::Release).context("failed to release shift")?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn send_key_combo_via_wtype(paste_method: PasteMethod) -> Result<()> {
    let args: Vec<&str> = match paste_method {
        PasteMethod::CtrlV => vec!["-M", "ctrl", "-k", "v"],
        PasteMethod::CtrlShiftV => vec!["-M", "ctrl", "-M", "shift", "-k", "v"],
        PasteMethod::ShiftInsert => vec!["-M", "shift", "-k", "Insert"],
        _ => return Err(anyhow!("Unsupported paste method")),
    };
    let output = Command::new("wtype").args(&args).output().context("failed to execute wtype")?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!("wtype failed: {}", String::from_utf8_lossy(&output.stderr).trim()))
    }
}

#[cfg(target_os = "linux")]
fn send_key_combo_via_dotool(paste_method: PasteMethod) -> Result<()> {
    let command = match paste_method {
        PasteMethod::CtrlV => "echo key ctrl+v | dotool",
        PasteMethod::CtrlShiftV => "echo key ctrl+shift+v | dotool",
        PasteMethod::ShiftInsert => "echo key shift+insert | dotool",
        _ => return Err(anyhow!("Unsupported paste method")),
    };
    let status = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("failed to execute dotool")?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("dotool failed"))
    }
}

#[cfg(target_os = "linux")]
fn send_key_combo_via_ydotool(paste_method: PasteMethod) -> Result<()> {
    let args: Vec<&str> = match paste_method {
        PasteMethod::CtrlV => vec!["key", "29:1", "47:1", "47:0", "29:0"],
        PasteMethod::CtrlShiftV => vec!["key", "29:1", "42:1", "47:1", "47:0", "42:0", "29:0"],
        PasteMethod::ShiftInsert => vec!["key", "42:1", "110:1", "110:0", "42:0"],
        _ => return Err(anyhow!("Unsupported paste method")),
    };
    let output = Command::new("ydotool").args(&args).output().context("failed to execute ydotool")?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!("ydotool failed: {}", String::from_utf8_lossy(&output.stderr).trim()))
    }
}

#[cfg(target_os = "linux")]
fn send_key_combo_via_xdotool(paste_method: PasteMethod) -> Result<()> {
    let combo = match paste_method {
        PasteMethod::CtrlV => "ctrl+v",
        PasteMethod::CtrlShiftV => "ctrl+shift+v",
        PasteMethod::ShiftInsert => "shift+Insert",
        _ => return Err(anyhow!("Unsupported paste method")),
    };
    let output = Command::new("xdotool")
        .arg("key")
        .arg("--clearmodifiers")
        .arg(combo)
        .output()
        .context("failed to execute xdotool")?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!("xdotool failed: {}", String::from_utf8_lossy(&output.stderr).trim()))
    }
}

#[cfg(target_os = "linux")]
fn try_send_key_combo_linux(paste_method: PasteMethod) -> Result<bool> {
    if is_wayland() {
        if !is_kde_wayland() && command_exists("wtype") {
            send_key_combo_via_wtype(paste_method)?;
            return Ok(true);
        }
        if command_exists("dotool") {
            send_key_combo_via_dotool(paste_method)?;
            return Ok(true);
        }
        if command_exists("ydotool") {
            send_key_combo_via_ydotool(paste_method)?;
            return Ok(true);
        }
    } else {
        if command_exists("xdotool") {
            send_key_combo_via_xdotool(paste_method)?;
            return Ok(true);
        }
        if command_exists("ydotool") {
            send_key_combo_via_ydotool(paste_method)?;
            return Ok(true);
        }
    }
    Ok(false)
}

fn paste_via_clipboard(
    enigo: &mut Enigo,
    text: &str,
    paste_method: PasteMethod,
    paste_delay_ms: u64,
) -> Result<()> {
    let original_clipboard = Clipboard::new()
        .ok()
        .and_then(|mut clipboard| clipboard.get_text().ok())
        .unwrap_or_default();

    #[cfg(target_os = "linux")]
    {
        if is_wayland() && command_exists("wl-copy") {
            write_clipboard_via_wl_copy(text)?;
        } else {
            let mut clipboard = Clipboard::new().context("failed to access clipboard")?;
            clipboard.set_text(text.to_string()).context("failed to write clipboard")?;
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        let mut clipboard = Clipboard::new().context("failed to access clipboard")?;
        clipboard.set_text(text.to_string()).context("failed to write clipboard")?;
    }

    std::thread::sleep(Duration::from_millis(paste_delay_ms));

    #[cfg(target_os = "linux")]
    let handled = try_send_key_combo_linux(paste_method)?;
    #[cfg(not(target_os = "linux"))]
    let handled = false;

    if !handled {
        match paste_method {
            PasteMethod::CtrlV => send_paste_ctrl_v(enigo)?,
            PasteMethod::CtrlShiftV => send_paste_ctrl_shift_v(enigo)?,
            PasteMethod::ShiftInsert => send_paste_shift_insert(enigo)?,
            _ => return Err(anyhow!("Invalid paste method for clipboard paste")),
        }
    }

    std::thread::sleep(Duration::from_millis(50));

    #[cfg(target_os = "linux")]
    {
        if is_wayland() && command_exists("wl-copy") {
            let _ = write_clipboard_via_wl_copy(&original_clipboard);
        } else if let Ok(mut clipboard) = Clipboard::new() {
            let _ = clipboard.set_text(original_clipboard);
        }
    }

    #[cfg(not(target_os = "linux"))]
    if let Ok(mut clipboard) = Clipboard::new() {
        let _ = clipboard.set_text(original_clipboard);
    }

    Ok(())
}

fn apply_output(config: &ShadowwordConfig, text: &str) -> Result<()> {
    if config.output.copy_to_clipboard {
        let mut clipboard = Clipboard::new().context("failed to access clipboard")?;
        clipboard
            .set_text(text.to_string())
            .context("failed to write clipboard")?;
    }

    let legacy_direct = config.output.type_into_active_window && config.output.paste_method == PasteMethod::None;
    let paste_method = if legacy_direct {
        PasteMethod::Direct
    } else {
        config.output.paste_method
    };

    if paste_method != PasteMethod::None {
        let mut enigo = Enigo::new(&Settings::default()).context("failed to init enigo")?;
        match paste_method {
            PasteMethod::None => {}
            PasteMethod::Direct => {
                #[cfg(target_os = "linux")]
                if !try_direct_typing_linux(text, config.output.typing_tool)? {
                    enigo.text(text).context("failed to type transcript")?;
                }
                #[cfg(not(target_os = "linux"))]
                enigo.text(text).context("failed to type transcript")?;
            }
            PasteMethod::CtrlV | PasteMethod::CtrlShiftV | PasteMethod::ShiftInsert => {
                paste_via_clipboard(&mut enigo, text, paste_method, config.output.paste_delay_ms)?;
            }
        }
    }

    Ok(())
}
