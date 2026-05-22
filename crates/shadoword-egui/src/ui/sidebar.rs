use egui::{Color32, CornerRadius, Frame, Margin, RichText, Vec2};

use crate::ui::constants::*;
use crate::ui::Page;
use crate::{MODEL_CATALOG, ShadowwordApp};

pub fn show(ctx: &egui::Context, app: &mut ShadowwordApp) {
    egui::SidePanel::left("sidebar")
        .frame(
            Frame::new()
                .fill(BG_SIDEBAR)
                .inner_margin(Margin::symmetric(0, 0)),
        )
        .exact_width(150.0)
        .resizable(false)
        .show_separator_line(false)
        .show(ctx, |ui| {
            ui.set_min_width(ui.available_width());

            // Logo
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("SHADOW")
                        .size(22.0)
                        .color(ACCENT)
                        .strong(),
                );
                ui.label(
                    RichText::new("WORD")
                        .size(12.0)
                        .color(TEXT_DIM)
                        .strong(),
                );
            });
            ui.add_space(24.0);

            // Nav items
            let nav_items = [
                (Page::General, "General"),
                (Page::Models, "Models"),
                (Page::History, "History"),
                (Page::Settings, "Settings"),
                (Page::About, "About"),
            ];

            for (page, label) in nav_items {
                let active = app.page == page;
                let bg = if active { NAV_ACTIVE } else { BG_SIDEBAR };
                let text_color = if active { ACCENT } else { TEXT_DIM };

                let btn = Frame::new()
                    .fill(bg)
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::symmetric(16, 8))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.label(
                            RichText::new(label).size(14.0).color(text_color),
                        );
                    });

                if btn.response.interact(egui::Sense::click()).clicked() {
                    app.page = page;
                }

                if !active && btn.response.interact(egui::Sense::hover()).hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
            }

            // Active model at bottom
            let inferring = app.response_rx.is_some();
            let loaded = app.local_service.is_loaded();
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(8.0);
                let model_name = MODEL_CATALOG
                    .iter()
                    .find(|m| {
                        app.model_path_for(m)
                            .map(|p| p == app.config.model_path)
                            .unwrap_or(false)
                    })
                    .map(|m| m.name)
                    .unwrap_or("No model");
                Frame::new()
                    .inner_margin(Margin::symmetric(12, 6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let dot_color = if inferring {
                                Color32::from_rgb(60, 120, 255)
                            } else if loaded {
                                STATUS_OK
                            } else {
                                REC_RED
                            };
                            let (dot_rect, _) =
                                ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                            ui.painter()
                                .circle_filled(dot_rect.center(), 4.0, dot_color);
                            ui.label(
                                RichText::new(model_name)
                                    .size(11.0)
                                    .color(TEXT_DIM),
                            );
                        });
                    });
            });
        });
}
