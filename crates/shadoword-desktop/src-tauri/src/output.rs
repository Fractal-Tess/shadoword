use anyhow::{anyhow, Context, Result};
use arboard::{Clipboard, ImageData};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use shadoword_core::{OutputConfig, PasteMethod};
use std::borrow::Cow;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use wait_timeout::ChildExt;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionKind {
    Wayland,
    X11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputBackend {
    Wtype,
    Xdotool,
    Enigo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OutputAction {
    Clipboard,
    Direct(OutputBackend),
    Paste(OutputBackend),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OutputPlan {
    actions: Vec<OutputAction>,
}

#[derive(Debug, Clone, Copy)]
struct OutputEnvironment {
    session: SessionKind,
    has_wtype: bool,
    has_xdotool: bool,
}

pub fn apply_output(config: &OutputConfig, text: &str) -> Result<()> {
    let plan = plan_output(
        config.copy_to_clipboard,
        config.paste_method,
        current_environment(),
    );
    apply_plan(&plan, config, text)
}

pub fn apply_streaming_segment_output(config: &OutputConfig, text: &str) -> Result<()> {
    let plan = plan_output(false, config.paste_method, current_environment());
    apply_plan(&plan, config, text)
}

pub fn apply_final_clipboard(config: &OutputConfig, text: &str) -> Result<()> {
    if config.copy_to_clipboard {
        write_clipboard(text)?;
    }
    Ok(())
}

fn current_environment() -> OutputEnvironment {
    OutputEnvironment {
        session: if is_wayland() {
            SessionKind::Wayland
        } else {
            SessionKind::X11
        },
        has_wtype: command_exists("wtype"),
        has_xdotool: command_exists("xdotool"),
    }
}

fn plan_output(
    copy_to_clipboard: bool,
    paste_method: PasteMethod,
    environment: OutputEnvironment,
) -> OutputPlan {
    let mut actions = Vec::new();
    if copy_to_clipboard {
        actions.push(OutputAction::Clipboard);
    }
    match paste_method {
        PasteMethod::None => {}
        PasteMethod::Direct => actions.push(OutputAction::Direct(active_backend(environment))),
        PasteMethod::CtrlV | PasteMethod::CtrlShiftV | PasteMethod::ShiftInsert => {
            actions.push(OutputAction::Paste(active_backend(environment)));
        }
    }
    OutputPlan { actions }
}

fn active_backend(environment: OutputEnvironment) -> OutputBackend {
    match environment.session {
        SessionKind::Wayland if environment.has_wtype => OutputBackend::Wtype,
        SessionKind::X11 if environment.has_xdotool => OutputBackend::Xdotool,
        _ => OutputBackend::Enigo,
    }
}

fn apply_plan(plan: &OutputPlan, config: &OutputConfig, text: &str) -> Result<()> {
    for action in &plan.actions {
        match *action {
            OutputAction::Clipboard => write_clipboard(text)?,
            OutputAction::Direct(backend) => type_text(text, backend)?,
            OutputAction::Paste(backend) => {
                paste_via_clipboard(text, config.paste_method, backend, config.paste_delay_ms)?
            }
        }
    }
    Ok(())
}

fn is_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
        || matches!(
            std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
            Some("wayland")
        )
}

fn command_exists(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn run_command(command: &mut Command, name: &str) -> Result<std::process::Output> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to start {name}"))?;
    if child
        .wait_timeout(COMMAND_TIMEOUT)
        .with_context(|| format!("failed while waiting for {name}"))?
        .is_none()
    {
        let _ = child.kill();
        let _ = child.wait();
        return Err(anyhow!("{name} timed out"));
    }
    child
        .wait_with_output()
        .with_context(|| format!("failed to collect {name} output"))
}

fn write_clipboard(text: &str) -> Result<()> {
    Clipboard::new()
        .context("failed to access clipboard")?
        .set_text(text)
        .context("failed to write transcript to clipboard")
}

enum ClipboardSnapshot {
    Text(String),
    Image {
        width: usize,
        height: usize,
        bytes: Vec<u8>,
    },
    Empty,
}

fn read_clipboard() -> Result<ClipboardSnapshot> {
    let mut clipboard = Clipboard::new().context("failed to access clipboard")?;
    if let Ok(text) = clipboard.get_text() {
        return Ok(ClipboardSnapshot::Text(text));
    }
    if let Ok(image) = clipboard.get_image() {
        return Ok(ClipboardSnapshot::Image {
            width: image.width,
            height: image.height,
            bytes: image.bytes.into_owned(),
        });
    }
    Ok(ClipboardSnapshot::Empty)
}

fn restore_clipboard(snapshot: ClipboardSnapshot) -> Result<()> {
    let mut clipboard = Clipboard::new().context("failed to access clipboard")?;
    match snapshot {
        ClipboardSnapshot::Text(text) => clipboard
            .set_text(text)
            .context("failed to restore clipboard text"),
        ClipboardSnapshot::Image {
            width,
            height,
            bytes,
        } => clipboard
            .set_image(ImageData {
                width,
                height,
                bytes: Cow::Owned(bytes),
            })
            .context("failed to restore clipboard image"),
        ClipboardSnapshot::Empty => clipboard.clear().context("failed to clear clipboard"),
    }
}

fn type_text(text: &str, backend: OutputBackend) -> Result<()> {
    let native = match backend {
        OutputBackend::Wtype => run_text_command("wtype", &["--", text]),
        OutputBackend::Xdotool => {
            run_text_command("xdotool", &["type", "--clearmodifiers", "--", text])
        }
        OutputBackend::Enigo => return type_text_via_enigo(text),
    };
    native.or_else(|native_error| {
        type_text_via_enigo(text).with_context(|| {
            format!("native typing backend failed ({native_error}); Enigo fallback also failed")
        })
    })
}

fn run_text_command(program: &str, args: &[&str]) -> Result<()> {
    let output = run_command(Command::new(program).args(args), program)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "{program} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn type_text_via_enigo(text: &str) -> Result<()> {
    Enigo::new(&Settings::default())
        .context("failed to initialize Enigo")?
        .text(text)
        .context("failed to type transcript")
}

fn paste_via_clipboard(
    text: &str,
    method: PasteMethod,
    backend: OutputBackend,
    delay_ms: u64,
) -> Result<()> {
    let original = read_clipboard()?;
    write_clipboard(text)?;
    let paste_result = (|| {
        thread::sleep(Duration::from_millis(delay_ms));
        send_key_combo(method, backend)?;
        thread::sleep(Duration::from_millis(50));
        Ok(())
    })();
    let restore_result = restore_clipboard(original).context("failed to restore clipboard");
    paste_result.and(restore_result)
}

fn send_key_combo(method: PasteMethod, backend: OutputBackend) -> Result<()> {
    let native = match backend {
        OutputBackend::Wtype => {
            let args = match method {
                PasteMethod::CtrlV => &["-M", "ctrl", "-k", "v"][..],
                PasteMethod::CtrlShiftV => &["-M", "ctrl", "-M", "shift", "-k", "v"][..],
                PasteMethod::ShiftInsert => &["-M", "shift", "-k", "Insert"][..],
                _ => return Err(anyhow!("unsupported clipboard paste method")),
            };
            run_text_command("wtype", args)
        }
        OutputBackend::Xdotool => {
            let combo = match method {
                PasteMethod::CtrlV => "ctrl+v",
                PasteMethod::CtrlShiftV => "ctrl+shift+v",
                PasteMethod::ShiftInsert => "shift+Insert",
                _ => return Err(anyhow!("unsupported clipboard paste method")),
            };
            run_text_command("xdotool", &["key", "--clearmodifiers", combo])
        }
        OutputBackend::Enigo => return send_key_combo_via_enigo(method),
    };
    native.or_else(|native_error| {
        send_key_combo_via_enigo(method).with_context(|| {
            format!("native paste backend failed ({native_error}); Enigo fallback also failed")
        })
    })
}

fn send_key_combo_via_enigo(method: PasteMethod) -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default()).context("failed to initialize Enigo")?;
    let modifiers: &[Key] = match method {
        PasteMethod::CtrlV => &[Key::Control],
        PasteMethod::CtrlShiftV => &[Key::Control, Key::Shift],
        PasteMethod::ShiftInsert => &[Key::Shift],
        _ => return Err(anyhow!("unsupported clipboard paste method")),
    };
    for modifier in modifiers {
        enigo.key(*modifier, Direction::Press)?;
    }
    let key = if method == PasteMethod::ShiftInsert {
        Key::Other(0x76)
    } else {
        Key::Unicode('v')
    };
    let click = enigo
        .key(key, Direction::Click)
        .context("failed to send paste key");
    thread::sleep(Duration::from_millis(100));
    let mut release = Ok(());
    for modifier in modifiers.iter().rev() {
        if let Err(error) = enigo.key(*modifier, Direction::Release) {
            release = Err(anyhow!(error));
        }
    }
    click.and(release)
}
