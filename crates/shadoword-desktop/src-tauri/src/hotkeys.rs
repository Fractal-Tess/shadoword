use handy_keys::{Hotkey, HotkeyId, HotkeyManager, HotkeyState as HandyState};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tauri::AppHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEventState {
    Pressed,
    Released,
}

enum Command {
    Register {
        shortcut: String,
        response: mpsc::Sender<Result<(), String>>,
    },
    Shutdown,
}

pub struct HotkeyBackend {
    command_tx: mpsc::Sender<Command>,
    manager_handle: Option<JoinHandle<()>>,
    listener_handle: Option<JoinHandle<()>>,
}

impl HotkeyBackend {
    pub fn new(app: AppHandle) -> Result<Self, String> {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let manager_handle = thread::Builder::new()
            .name("shadoword-hotkey-manager".to_string())
            .spawn(move || {
                let manager = HotkeyManager::new_with_blocking().or_else(|blocking_error| {
                    tracing::warn!(%blocking_error, "blocking global shortcuts unavailable; trying non-blocking registration");
                    HotkeyManager::new()
                });
                let manager = match manager {
                    Ok(manager) => manager,
                    Err(error) => {
                        let _ = ready_tx.send(Err(format!(
                            "failed to initialize global shortcut manager: {error}"
                        )));
                        return;
                    }
                };
                let _ = ready_tx.send(Ok(()));
                let mut registered_id: Option<HotkeyId> = None;
                loop {
                    while let Some(event) = manager.try_recv() {
                        if Some(event.id) != registered_id {
                            continue;
                        }
                        let state = match event.state {
                            HandyState::Pressed => HotkeyEventState::Pressed,
                            HandyState::Released => HotkeyEventState::Released,
                        };
                        if event_tx.send(state).is_err() {
                            return;
                        }
                    }
                    match command_rx.recv_timeout(Duration::from_millis(10)) {
                        Ok(Command::Register { shortcut, response }) => {
                            let result = register_shortcut(&manager, &mut registered_id, &shortcut);
                            let _ = response.send(result);
                        }
                        Ok(Command::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                }
                if let Some(id) = registered_id {
                    let _ = manager.unregister(id);
                }
            })
            .map_err(|error| format!("failed to spawn global shortcut manager: {error}"))?;

        ready_rx
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| "timed out initializing global shortcut manager".to_string())??;

        let listener_handle = thread::Builder::new()
            .name("shadoword-hotkey-listener".to_string())
            .spawn(move || {
                while let Ok(event) = event_rx.recv() {
                    crate::commands::handle_hotkey_event(&app, event);
                }
            })
            .map_err(|error| format!("failed to spawn global shortcut listener: {error}"))?;

        Ok(Self {
            command_tx,
            manager_handle: Some(manager_handle),
            listener_handle: Some(listener_handle),
        })
    }

    pub fn register(&self, shortcut: &str) -> Result<(), String> {
        validate_shortcut(shortcut)?;
        let (response_tx, response_rx) = mpsc::channel();
        self.command_tx
            .send(Command::Register {
                shortcut: shortcut.trim().to_ascii_lowercase(),
                response: response_tx,
            })
            .map_err(|_| "global shortcut manager is not running".to_string())?;
        response_rx
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| "timed out registering global shortcut".to_string())?
    }
}

impl Drop for HotkeyBackend {
    fn drop(&mut self) {
        let _ = self.command_tx.send(Command::Shutdown);
        if let Some(handle) = self.manager_handle.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.listener_handle.take() {
            let _ = handle.join();
        }
    }
}

fn register_shortcut(
    manager: &HotkeyManager,
    registered_id: &mut Option<HotkeyId>,
    shortcut: &str,
) -> Result<(), String> {
    let hotkey = shortcut
        .parse::<Hotkey>()
        .map_err(|error| format!("invalid global shortcut: {error}"))?;
    let next = manager
        .register(hotkey)
        .map_err(|error| format!("failed to register global shortcut: {error}"))?;
    if let Some(previous) = registered_id.take() {
        if let Err(error) = manager.unregister(previous) {
            let _ = manager.unregister(next);
            *registered_id = Some(previous);
            return Err(format!(
                "failed to replace previous global shortcut: {error}"
            ));
        }
    }
    *registered_id = Some(next);
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShortcutKeyKind {
    Text,
    Function,
    NonText,
}

pub fn validate_shortcut(shortcut: &str) -> Result<(), String> {
    let trimmed = shortcut.trim();
    if trimmed.is_empty() {
        return Err("shortcut cannot be empty".to_string());
    }
    let mut has_modifier = false;
    let mut key = None;
    for part in trimmed
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "alt" | "shift" | "super" | "meta" | "cmd" | "command" => {
                has_modifier = true
            }
            _ if key.is_none() => key = Some(part),
            _ => return Err("shortcut must contain only one non-modifier key".to_string()),
        }
    }
    let key = key.ok_or_else(|| "shortcut must include a key".to_string())?;
    match classify_key(key) {
        Some(ShortcutKeyKind::Function | ShortcutKeyKind::NonText) => Ok(()),
        Some(ShortcutKeyKind::Text) if has_modifier => Ok(()),
        Some(ShortcutKeyKind::Text) => {
            Err(format!("shortcut '{key}' needs Ctrl, Alt, Shift, or Super"))
        }
        None => Err(format!("unsupported shortcut key '{key}'")),
    }
}

fn classify_key(key: &str) -> Option<ShortcutKeyKind> {
    let key = key.trim().to_ascii_lowercase();
    if key
        .strip_prefix('f')
        .and_then(|number| number.parse::<u8>().ok())
        .is_some_and(|number| (1..=24).contains(&number))
    {
        return Some(ShortcutKeyKind::Function);
    }
    if matches!(
        key.as_str(),
        "escape"
            | "esc"
            | "tab"
            | "enter"
            | "return"
            | "backspace"
            | "insert"
            | "delete"
            | "home"
            | "end"
            | "pageup"
            | "pagedown"
            | "up"
            | "down"
            | "left"
            | "right"
            | "capslock"
            | "printscreen"
            | "scrolllock"
            | "pause"
            | "playpause"
            | "stop"
            | "prevtrack"
            | "nexttrack"
    ) {
        return Some(ShortcutKeyKind::NonText);
    }
    (key == "space" || key.chars().count() == 1).then_some(ShortcutKeyKind::Text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_safe_global_shortcuts() {
        assert!(validate_shortcut("f2").is_ok());
        assert!(validate_shortcut("ctrl+space").is_ok());
        assert!(validate_shortcut("insert").is_ok());
        assert!(validate_shortcut("a").is_err());
        assert!(validate_shortcut("ctrl+a+b").is_err());
    }
}
