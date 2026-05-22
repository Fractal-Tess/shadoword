use eframe::egui;
use std::thread;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use crate::ShadowwordApp;

// ── Tray icon construction ───────────────────────────────────────────────────

pub fn create_tray_icon() -> tray_icon::Icon {
    let width = 32;
    let height = 32;
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    let cx = width as f32 / 2.0 - 0.5;
    let cy = height as f32 / 2.0 - 0.5;
    let radius = 13.0;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = ((y * width + x) * 4) as usize;

            if dist <= radius {
                rgba[idx] = 255;
                rgba[idx + 1] = 255;
                rgba[idx + 2] = 255;
                rgba[idx + 3] = 255;
            } else if dist <= radius + 1.0 {
                let alpha = ((radius + 1.0 - dist) * 255.0) as u8;
                rgba[idx] = 255;
                rgba[idx + 1] = 255;
                rgba[idx + 2] = 255;
                rgba[idx + 3] = alpha;
            }
        }
    }

    tray_icon::Icon::from_rgba(rgba, width, height).unwrap()
}

pub fn build_tray_icon() -> tray_icon::TrayIcon {
    let show_item = MenuItem::with_id("show", "Show Shadow Word", true, None);
    let quit_item = MenuItem::with_id("quit", "Quit", true, None);

    let menu = Menu::new();
    menu.append(&show_item).unwrap();
    menu.append(&quit_item).unwrap();

    TrayIconBuilder::new()
        .with_icon(create_tray_icon())
        .with_menu(Box::new(menu))
        .with_tooltip("Shadow Word")
        .with_menu_on_left_click(false)
        .build()
        .unwrap()
}

pub fn setup_tray() {
    #[cfg(target_os = "linux")]
    {
        thread::spawn(|| {
            gtk::init().unwrap();
            let _tray_icon = build_tray_icon();
            gtk::main();
        });
    }

    #[cfg(not(target_os = "linux"))]
    {
        let tray_icon = build_tray_icon();
        let _ = Box::leak(Box::new(tray_icon));
    }
}

// ── ShadowwordApp tray/platform methods ──────────────────────────────────────

impl ShadowwordApp {
    pub(crate) fn show_window(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.window_hidden = false;
    }

    pub(crate) fn hide_window(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        self.window_hidden = true;
    }

    pub(crate) fn toggle_window_visibility(&mut self, ctx: &egui::Context) {
        if self.window_hidden {
            self.show_window(ctx);
        } else {
            self.hide_window(ctx);
        }
    }

    pub(crate) fn poll_tray_menu_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id().as_ref() == "show" {
                self.show_window(ctx);
            }

            if event.id().as_ref() == "quit" {
                self.quitting = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    pub(crate) fn poll_tray_icon_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button,
                button_state,
                ..
            } = event
            {
                if button == MouseButton::Left && button_state == MouseButtonState::Up {
                    self.toggle_window_visibility(ctx);
                }
            }
        }
    }
}
