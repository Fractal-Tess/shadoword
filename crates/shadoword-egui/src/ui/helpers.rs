use egui::{Color32, CornerRadius, Frame, Margin, RichText};

use crate::ui::constants::*;

// ── UI helpers ───────────────────────────────────────────────────────────────

pub fn section_heading(ui: &mut egui::Ui, text: &str) {
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
pub fn setting_row(ui: &mut egui::Ui, bg: Color32, add_contents: impl FnOnce(&mut egui::Ui)) {
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
pub fn setting_row_lr(
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

pub fn animated_dots(t: f64) -> &'static str {
    match ((t * 3.0) as usize) % 4 {
        1 => ".",
        2 => "..",
        3 => "...",
        _ => "",
    }
}
