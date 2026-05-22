use anyhow::{anyhow, Context, Result};
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use shadoword_core::{PasteMethod, ShadowwordConfig, TypingTool};
use std::process::{Command, Stdio};
use std::time::Duration;

// ── Platform detection ───────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn is_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
        || matches!(
            std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
            Some("wayland")
        )
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

// ── Linux clipboard write via wl-copy ────────────────────────────────────────

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

// ── Linux direct typing helpers ──────────────────────────────────────────────

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
        Err(anyhow!(
            "wtype failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
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
        Err(anyhow!(
            "kwtype failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
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
        Err(anyhow!(
            "xdotool failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
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
        Err(anyhow!(
            "ydotool failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
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
    let output = child
        .wait_with_output()
        .context("failed to wait for dotool")?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "dotool failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

// ── Linux direct typing dispatcher ───────────────────────────────────────────

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

// ── Key combo via enigo (non-Linux or fallback) ──────────────────────────────

fn send_paste_ctrl_v(enigo: &mut Enigo) -> Result<()> {
    #[cfg(target_os = "macos")]
    let (modifier_key, v_key_code) = (Key::Meta, Key::Other(9));
    #[cfg(target_os = "windows")]
    let (modifier_key, v_key_code) = (Key::Control, Key::Other(0x56));
    #[cfg(target_os = "linux")]
    let (modifier_key, v_key_code) = (Key::Control, Key::Unicode('v'));

    enigo
        .key(modifier_key, Direction::Press)
        .context("failed to press modifier")?;
    enigo
        .key(v_key_code, Direction::Click)
        .context("failed to click V")?;
    std::thread::sleep(Duration::from_millis(100));
    enigo
        .key(modifier_key, Direction::Release)
        .context("failed to release modifier")?;
    Ok(())
}

fn send_paste_ctrl_shift_v(enigo: &mut Enigo) -> Result<()> {
    #[cfg(target_os = "macos")]
    let (modifier_key, v_key_code) = (Key::Meta, Key::Other(9));
    #[cfg(target_os = "windows")]
    let (modifier_key, v_key_code) = (Key::Control, Key::Other(0x56));
    #[cfg(target_os = "linux")]
    let (modifier_key, v_key_code) = (Key::Control, Key::Unicode('v'));

    enigo
        .key(modifier_key, Direction::Press)
        .context("failed to press modifier")?;
    enigo
        .key(Key::Shift, Direction::Press)
        .context("failed to press shift")?;
    enigo
        .key(v_key_code, Direction::Click)
        .context("failed to click V")?;
    std::thread::sleep(Duration::from_millis(100));
    enigo
        .key(Key::Shift, Direction::Release)
        .context("failed to release shift")?;
    enigo
        .key(modifier_key, Direction::Release)
        .context("failed to release modifier")?;
    Ok(())
}

fn send_paste_shift_insert(enigo: &mut Enigo) -> Result<()> {
    #[cfg(target_os = "windows")]
    let insert_key_code = Key::Other(0x2D);
    #[cfg(not(target_os = "windows"))]
    let insert_key_code = Key::Other(0x76);

    enigo
        .key(Key::Shift, Direction::Press)
        .context("failed to press shift")?;
    enigo
        .key(insert_key_code, Direction::Click)
        .context("failed to click insert")?;
    std::thread::sleep(Duration::from_millis(100));
    enigo
        .key(Key::Shift, Direction::Release)
        .context("failed to release shift")?;
    Ok(())
}

// ── Linux key combo via native tools ─────────────────────────────────────────

#[cfg(target_os = "linux")]
fn send_key_combo_via_wtype(paste_method: PasteMethod) -> Result<()> {
    let args: Vec<&str> = match paste_method {
        PasteMethod::CtrlV => vec!["-M", "ctrl", "-k", "v"],
        PasteMethod::CtrlShiftV => vec!["-M", "ctrl", "-M", "shift", "-k", "v"],
        PasteMethod::ShiftInsert => vec!["-M", "shift", "-k", "Insert"],
        _ => return Err(anyhow!("Unsupported paste method")),
    };
    let output = Command::new("wtype")
        .args(&args)
        .output()
        .context("failed to execute wtype")?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "wtype failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
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
    let output = Command::new("ydotool")
        .args(&args)
        .output()
        .context("failed to execute ydotool")?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "ydotool failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
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
        Err(anyhow!(
            "xdotool failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
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

// ── Clipboard paste ──────────────────────────────────────────────────────────

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
            let mut clipboard =
                Clipboard::new().context("failed to access clipboard")?;
            clipboard
                .set_text(text.to_string())
                .context("failed to write clipboard")?;
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        let mut clipboard =
            Clipboard::new().context("failed to access clipboard")?;
        clipboard
            .set_text(text.to_string())
            .context("failed to write clipboard")?;
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

// ── Main output dispatcher ───────────────────────────────────────────────────

pub(crate) fn apply_output(config: &ShadowwordConfig, text: &str) -> Result<()> {
    if config.output.copy_to_clipboard {
        let mut clipboard = Clipboard::new().context("failed to access clipboard")?;
        clipboard
            .set_text(text.to_string())
            .context("failed to write clipboard")?;
    }

    let legacy_direct = config.output.type_into_active_window
        && config.output.paste_method == PasteMethod::None;
    let paste_method = if legacy_direct {
        PasteMethod::Direct
    } else {
        config.output.paste_method
    };

    if paste_method != PasteMethod::None {
        let mut enigo =
            Enigo::new(&Settings::default()).context("failed to init enigo")?;
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
                paste_via_clipboard(
                    &mut enigo,
                    text,
                    paste_method,
                    config.output.paste_delay_ms,
                )?;
            }
        }
    }

    Ok(())
}
