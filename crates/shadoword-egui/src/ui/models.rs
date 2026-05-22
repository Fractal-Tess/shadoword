use egui::{CornerRadius, Frame, Margin, RichText, Stroke, Vec2};
use std::collections::HashMap;

use crate::ui::constants::*;
use crate::ui::helpers::{section_heading, setting_row_lr};
use crate::{MODEL_CATALOG, ShadowwordApp, WhisperAccelerator};

pub fn show(ui: &mut egui::Ui, app: &mut ShadowwordApp) {
    let is_preloading = app.preloading;
    let model_loaded = app.local_service.is_loaded();

    // ── Acceleration ────────────────────────────────────────────────
    section_heading(ui, "ACCELERATION");

    setting_row_lr(ui, BG_ROW, "Whisper Accelerator", |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut app.config.whisper_accelerator, WhisperAccelerator::Auto, "Auto");
            ui.selectable_value(&mut app.config.whisper_accelerator, WhisperAccelerator::Cpu, "CPU");
            ui.selectable_value(&mut app.config.whisper_accelerator, WhisperAccelerator::Gpu, "GPU");
        });
    });

    setting_row_lr(ui, BG_ROW_ALT, "Preload on Startup", |ui| {
        ui.checkbox(&mut app.config.preload_on_startup, "");
    });

    setting_row_lr(ui, BG_ROW, "Model in Memory", |ui| {
        if model_loaded {
            ui.label(RichText::new("Loaded").size(12.0).color(STATUS_OK).strong());
        } else if is_preloading {
            ui.ctx().request_repaint();
            ui.label(RichText::new("Loading...").size(12.0).color(TEXT_DIM));
        } else if ui.button(RichText::new("Preload").size(12.0)).clicked() {
            app.start_preload();
        }
    });

    // ── Models ──────────────────────────────────────────────────────
    section_heading(ui, "MODELS");

    let current_path = app.config.model_path.clone();

    // Snapshot download state to avoid borrow issues
    let dl_state: HashMap<String, (u64, u64)> = app
        .active_downloads
        .iter()
        .map(|(id, dl)| (id.clone(), (dl.downloaded, dl.total)))
        .collect();
    let has_active_downloads = !dl_state.is_empty();
    if has_active_downloads {
        ui.ctx().request_repaint();
    }

    let mut action: Option<(&'static str, &'static str)> = None;

    for model in MODEL_CATALOG {
        let installed = app.is_model_installed(model);
        let is_active = app
            .model_path_for(model)
            .map(|p| p == current_path)
            .unwrap_or(false);
        let dl = dl_state.get(model.id);
        let is_downloading = dl.is_some();

        let bg = if is_active { BG_HOVER } else { BG_ROW };
        let border = if is_active {
            ACCENT
        } else if model.recommended {
            TEXT_DIM
        } else {
            BORDER
        };

        Frame::new()
            .fill(bg)
            .corner_radius(CornerRadius::same(4))
            .stroke(Stroke::new(1.0, border))
            .inner_margin(Margin::symmetric(14, 8))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                // Single compact row: name + bars + status/action
                ui.horizontal(|ui| {
                    // Left: name + badges
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(model.name).size(13.0).color(TEXT).strong());
                            if model.recommended {
                                ui.label(RichText::new("REC").size(9.0).color(ACCENT).strong());
                            }
                            if is_active {
                                ui.label(RichText::new("ACTIVE").size(9.0).color(STATUS_OK).strong());
                            }
                        });
                        // Speed/quality bars inline
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            ui.label(RichText::new("Spd").size(10.0).color(TEXT_MUTED));
                            let (r, _) =
                                ui.allocate_exact_size(Vec2::new(60.0, 6.0), egui::Sense::hover());
                            ui.painter().rect_filled(r, CornerRadius::same(3), BG_INPUT);
                            let mut f = r;
                            f.set_right(r.left() + r.width() * model.speed);
                            ui.painter().rect_filled(f, CornerRadius::same(3), ACCENT);

                            ui.label(RichText::new("Qual").size(10.0).color(TEXT_MUTED));
                            let (r, _) =
                                ui.allocate_exact_size(Vec2::new(60.0, 6.0), egui::Sense::hover());
                            ui.painter().rect_filled(r, CornerRadius::same(3), BG_INPUT);
                            let mut f = r;
                            f.set_right(r.left() + r.width() * model.accuracy);
                            ui.painter().rect_filled(f, CornerRadius::same(3), ACCENT);

                            ui.label(
                                RichText::new(format!("{} MB", model.size_mb))
                                    .size(10.0)
                                    .color(TEXT_MUTED),
                            );
                            ui.label(
                                RichText::new(model.languages).size(10.0).color(TEXT_MUTED),
                            );
                        });
                    });

                    // Right: actions
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if is_downloading {
                            let (downloaded, total) = dl.copied().unwrap_or((0, 0));
                            let dl_mb = downloaded as f32 / (1024.0 * 1024.0);
                            let total_mb = total as f32 / (1024.0 * 1024.0);
                            let progress = if total > 0 {
                                downloaded as f32 / total as f32
                            } else {
                                0.0
                            };

                            ui.vertical(|ui| {
                                let (bar_rect, _) = ui.allocate_exact_size(
                                    Vec2::new(120.0, 5.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    bar_rect,
                                    CornerRadius::same(2),
                                    BG_INPUT,
                                );
                                let mut filled = bar_rect;
                                filled.set_right(
                                    bar_rect.left() + bar_rect.width() * progress,
                                );
                                ui.painter().rect_filled(
                                    filled,
                                    CornerRadius::same(2),
                                    ACCENT,
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "{:.0}/{:.0} MB",
                                        dl_mb, total_mb
                                    ))
                                    .size(10.0)
                                    .color(TEXT_DIM),
                                );
                            });
                        } else if installed {
                            if ui
                                .small_button(RichText::new("Delete").size(11.0))
                                .clicked()
                            {
                                action = Some(("delete", model.id));
                            }
                            if !is_active
                                && ui
                                    .small_button(RichText::new("Select").size(11.0))
                                    .clicked()
                            {
                                action = Some(("select", model.id));
                            }
                        } else {
                            if ui
                                .small_button(RichText::new("Download").size(11.0))
                                .clicked()
                            {
                                action = Some(("download", model.id));
                            }
                        }
                    });
                });
            });
        ui.add_space(2.0);
    }

    if let Some((act, id)) = action {
        match act {
            "select" => {
                if let Some(model) = MODEL_CATALOG.iter().find(|m| m.id == id) {
                    app.select_model(model);
                }
            }
            "download" => app.start_download(id),
            "delete" => {
                if let Some(model) = MODEL_CATALOG.iter().find(|m| m.id == id) {
                    if let Some(path) = app.model_path_for(model) {
                        let _ = std::fs::remove_file(&path);
                        app.status_line = format!("Deleted {}", model.name);
                    }
                }
            }
            _ => {}
        }
    }
}
