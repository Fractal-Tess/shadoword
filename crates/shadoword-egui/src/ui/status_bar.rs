use egui::{Frame, Margin, RichText};

use crate::ui::constants::*;
use crate::ShadowwordApp;

pub fn show(ctx: &egui::Context, app: &ShadowwordApp) {
    egui::TopBottomPanel::bottom("footer")
        .frame(
            Frame::new()
                .fill(BG_SIDEBAR)
                .inner_margin(Margin::symmetric(16, 8)),
        )
        .show_separator_line(false)
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(&app.status_line)
                        .size(11.0)
                        .color(TEXT_MUTED),
                );
            });
        });
}
