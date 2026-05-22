use egui::{CornerRadius, Frame, Margin, RichText};

use crate::ui::constants::*;
use crate::ui::helpers::{section_heading, setting_row, setting_row_lr};
use crate::{ShadowwordApp, ShadowwordConfig};

pub fn show(ui: &mut egui::Ui, _app: &ShadowwordApp) {
    section_heading(ui, "ABOUT");

    setting_row_lr(ui, BG_ROW, "Version", |ui| {
        ui.label(RichText::new("0.8.0").size(14.0).color(TEXT_DIM));
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
