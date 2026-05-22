#[cfg(not(target_os = "linux"))]
use global_hotkey::hotkey::HotKey;
#[cfg(not(target_os = "linux"))]
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
#[cfg(target_os = "linux")]
use handy_keys::{
    Hotkey as HandyHotkey, HotkeyId as HandyHotkeyId, HotkeyManager as HandyHotkeyManager,
    HotkeyState as HandyHotkeyState,
};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::ShadowwordApp;

// ── Linux hotkey backend types ───────────────────────────────────────────────

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy)]
pub(crate) enum HotkeyEventState {
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
pub(crate) struct LinuxHotkeyBackend {
    command_tx: mpsc::Sender<LinuxHotkeyCommand>,
    event_rx: Receiver<HotkeyEventState>,
    thread_handle: Option<JoinHandle<()>>,
}

#[cfg(target_os = "linux")]
impl LinuxHotkeyBackend {
    pub(crate) fn new() -> Result<Self, String> {
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

    pub(crate) fn register(&self, shortcut: &str) -> Result<(), String> {
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

// ── Non-Linux hotkey registration helper ─────────────────────────────────────

#[cfg(not(target_os = "linux"))]
pub(crate) fn register_hotkey(
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

// ── ShadowwordApp hotkey methods ─────────────────────────────────────────────

impl ShadowwordApp {
    pub(crate) fn poll_hotkey_events(&mut self) {
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

    pub(crate) fn re_register_hotkey(&mut self) {
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

        match register_hotkey(manager, &shortcut) {
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
}
