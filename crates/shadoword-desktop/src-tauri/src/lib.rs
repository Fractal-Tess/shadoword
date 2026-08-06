#[cfg(not(feature = "local-runtime"))]
compile_error!("Shadoword desktop builds must include the local CPU runtime");

mod commands;
mod contracts;
mod hotkeys;
mod openrouter;
mod output;
mod recording;
mod remote;
mod remote_stream;
mod tray;

use commands::DesktopState;
use contracts::DesktopEvent;
use specta_typescript::Typescript;
use std::path::PathBuf;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, Wry};

fn bindings() -> tauri_specta::Builder<Wry> {
    tauri_specta::Builder::<Wry>::new()
        .error_handling(tauri_specta::ErrorHandlingMode::Throw)
        .commands(tauri_specta::collect_commands![
            commands::load_desktop_state,
            commands::get_recording_state,
            commands::list_input_devices,
            commands::save_desktop_settings,
            commands::reveal_desktop_secret,
            commands::copy_desktop_secret,
            commands::test_remote_connection,
            commands::list_openrouter_models,
            commands::test_openrouter_key,
            commands::refresh_remote_overview,
            commands::update_remote_runtime,
            commands::select_remote_model,
            commands::delete_remote_model,
            commands::start_remote_download,
            commands::poll_remote_download,
            commands::refresh_local_overview,
            commands::validate_local_inference_pool,
            commands::preload_local_model,
            commands::update_local_runtime,
            commands::select_local_model,
            commands::delete_local_model,
            commands::start_local_download,
            commands::poll_local_download,
            commands::start_recording,
            commands::cancel_recording,
            commands::stop_and_transcribe,
        ])
        .events(tauri_specta::collect_events![DesktopEvent])
}

pub fn export_bindings() -> Result<(), String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/lib/bindings.ts");
    bindings()
        .export(Typescript::default(), &path)
        .map_err(|error| error.to_string())?;
    let generated = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    std::fs::write(path, format!("{}\n", generated.trim_end())).map_err(|error| error.to_string())
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn hide_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show Shadoword", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide Shadoword", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &hide, &quit])?;
    let tray = TrayIconBuilder::with_id(tray::TRAY_ICON_ID)
        .icon(tray::idle_icon()?)
        .tooltip("Shadoword")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "hide" => hide_main_window(app),
            "quit" => {
                commands::shutdown(app);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    tray.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    export_bindings().expect("failed to export TypeScript bindings");

    let invoke_bindings = bindings();
    let state = DesktopState::load().expect("failed to initialize Shadoword desktop state");
    tauri::Builder::default()
        .manage(state)
        .setup(move |app| {
            bindings().mount_events(app);
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            setup_tray(app.handle())?;
            commands::setup_native(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<DesktopState>();
                if commands::close_to_tray(&state) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(invoke_bindings.invoke_handler())
        .build(tauri::generate_context!())
        .expect("failed to build Shadoword desktop shell")
        .run(|app, event| {
            if matches!(
                event,
                tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. }
            ) {
                commands::shutdown(app);
            }
        });
}
