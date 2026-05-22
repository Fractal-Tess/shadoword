use arboard::Clipboard;
use egui::{CornerRadius, Frame, Margin, RichText};

use crate::ui::constants::*;
use crate::ui::helpers::section_heading;
use crate::ShadowwordApp;

pub fn show(ui: &mut egui::Ui, app: &mut ShadowwordApp) {
    section_heading(ui, "HISTORY");

    if app.history.is_empty() {
        ui.add_space(24.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("No transcriptions yet")
                    .size(14.0)
                    .color(TEXT_MUTED),
            );
        });
        return;
    }

    let mut to_delete: Option<usize> = None;
    let mut to_copy: Option<usize> = None;

    for (i, entry) in app.history.iter().enumerate() {
        let bg = if i % 2 == 0 { BG_ROW } else { BG_ROW_ALT };

        Frame::new()
            .fill(bg)
            .corner_radius(CornerRadius::same(4))
            .inner_margin(Margin::symmetric(16, 10))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                // Header: timestamp + actions
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&entry.timestamp)
                            .size(12.0)
                            .color(TEXT_DIM)
                            .strong(),
                    );

                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if ui
                                .small_button(RichText::new("Delete").size(11.0))
                                .clicked()
                            {
                                to_delete = Some(i);
                            }
                            if ui
                                .small_button(RichText::new("Copy").size(11.0))
                                .clicked()
                            {
                                to_copy = Some(i);
                            }
                            ui.label(
                                RichText::new(format!(
                                    "{} - {}ms",
                                    entry.engine, entry.elapsed_ms
                                ))
                                .size(11.0)
                                .color(TEXT_MUTED),
                            );
                        },
                    );
                });

                ui.add_space(4.0);

                // Transcript text
                ui.label(
                    RichText::new(&entry.text)
                        .size(13.0)
                        .color(TEXT)
                        .italics(),
                );
            });
    }

    // Apply actions after iteration
    if let Some(i) = to_copy {
        if let Some(entry) = app.history.get(i) {
            if let Ok(mut clipboard) = Clipboard::new() {
                let _ = clipboard.set_text(entry.text.clone());
                app.status_line = "Copied to clipboard".to_string();
            }
        }
    }
    if let Some(i) = to_delete {
        app.history.remove(i);
    }
}
