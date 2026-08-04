use crate::contracts::RecordingPhase;
use tauri::image::Image;
use tauri::AppHandle;

pub const TRAY_ICON_ID: &str = "shadoword-main-tray";

const IDLE_ICON_BYTES: &[u8] = include_bytes!("../icons/tray-idle.png");
const RECORDING_ICON_BYTES: &[u8] = include_bytes!("../icons/tray-recording.png");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrayIconState {
    Idle,
    Recording,
}

impl From<RecordingPhase> for TrayIconState {
    fn from(phase: RecordingPhase) -> Self {
        match phase {
            RecordingPhase::Recording => Self::Recording,
            RecordingPhase::Idle | RecordingPhase::Finalizing => Self::Idle,
        }
    }
}

fn icon_for_state(state: TrayIconState) -> tauri::Result<Image<'static>> {
    let bytes = match state {
        TrayIconState::Idle => IDLE_ICON_BYTES,
        TrayIconState::Recording => RECORDING_ICON_BYTES,
    };
    Image::from_bytes(bytes)
}

pub fn idle_icon() -> tauri::Result<Image<'static>> {
    icon_for_state(TrayIconState::Idle)
}

pub fn set_icon_for_phase(app: &AppHandle, phase: RecordingPhase) {
    if let Err(error) = try_set_icon_for_phase(app, phase) {
        tracing::warn!(?phase, %error, "failed to update tray icon");
    }
}

fn try_set_icon_for_phase(app: &AppHandle, phase: RecordingPhase) -> Result<(), String> {
    let tray = app
        .tray_by_id(TRAY_ICON_ID)
        .ok_or_else(|| format!("tray '{TRAY_ICON_ID}' is not registered"))?;
    let icon = icon_for_state(phase.into()).map_err(|error| error.to_string())?;
    tray.set_icon(Some(icon)).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_phase_uses_recording_icon() {
        assert_eq!(
            TrayIconState::from(RecordingPhase::Recording),
            TrayIconState::Recording
        );
    }

    #[test]
    fn finalizing_phase_uses_idle_icon() {
        assert_eq!(
            TrayIconState::from(RecordingPhase::Finalizing),
            TrayIconState::Idle
        );
    }

    #[test]
    fn idle_phase_uses_idle_icon() {
        assert_eq!(
            TrayIconState::from(RecordingPhase::Idle),
            TrayIconState::Idle
        );
    }

    #[test]
    fn embedded_tray_icons_are_compact() {
        let idle = icon_for_state(TrayIconState::Idle).expect("decode idle tray icon");
        let recording =
            icon_for_state(TrayIconState::Recording).expect("decode recording tray icon");
        assert_eq!(
            [
                (idle.width(), idle.height()),
                (recording.width(), recording.height())
            ],
            [(32, 32), (32, 32)]
        );
    }

    #[test]
    fn idle_asset_is_transparent_gray() {
        let icon = icon_for_state(TrayIconState::Idle).expect("decode idle tray icon");
        let pixels = icon.rgba().chunks_exact(4);
        assert!(
            pixels.clone().any(|pixel| pixel[3] == 0)
                && pixels.clone().any(|pixel| pixel == [141, 147, 157, 255])
        );
    }

    #[test]
    fn recording_asset_is_transparent_scarlet() {
        let icon = icon_for_state(TrayIconState::Recording).expect("decode recording tray icon");
        let pixels = icon.rgba().chunks_exact(4);
        assert!(
            pixels.clone().any(|pixel| pixel[3] == 0)
                && pixels.clone().any(|pixel| pixel == [239, 35, 60, 255])
        );
    }
}
