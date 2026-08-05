// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    {
        // WebKitGTK's Wayland backend can terminate with a protocol error under
        // NVIDIA/Hyprland. Set these before Tauri initializes GTK so packaged,
        // archive, and development launches all use the stable XWayland path.
        std::env::set_var("GDK_BACKEND", "x11");
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    shadoword_desktop_lib::run();
}
