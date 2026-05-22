use egui::{Color32, CornerRadius, Frame, Margin, RichText, Stroke, Vec2};

use crate::ui::constants::*;
use crate::ui::helpers::{animated_dots, section_heading};
use crate::ShadowwordApp;

pub fn show(ui: &mut egui::Ui, app: &mut ShadowwordApp) {
    let time = ui.ctx().input(|i| i.time);
    let recording = app.active_recording.is_some();
    let streaming = app.stream_event_rx.is_some();
    let transcribing = app.response_rx.is_some();

    if recording || streaming || transcribing {
        ui.ctx().request_repaint();
    }

    section_heading(ui, "RECORD");

    // Record button area
    ui.add_space(16.0);
    ui.vertical_centered(|ui| {
        let radius: f32 = 36.0;
        let size = Vec2::splat(radius * 2.0 + 32.0);
        let (resp, painter) = ui.allocate_painter(size, egui::Sense::click());
        let center = resp.rect.center();

        // Outer ring / glow
        if recording {
            let pulse = ((time * 2.5).sin() * 0.5 + 0.5) as f32;
            painter.circle_filled(
                center,
                radius + 4.0 + pulse * 8.0,
                Color32::from_rgba_unmultiplied(200, 30, 30, (20.0 + pulse * 40.0) as u8),
            );
        } else if resp.hovered() && !transcribing {
            painter.circle_filled(
                center,
                radius + 5.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, 15),
            );
        }

        // Main circle
        let fill = if recording {
            REC_RED
        } else if transcribing {
            TEXT_DIM
        } else if resp.hovered() {
            ACCENT
        } else {
            Color32::from_rgb(200, 200, 200)
        };
        painter.circle_filled(center, radius, fill);

        // Inner icon
        let icon_color = Color32::from_rgb(12, 12, 12);
        if recording {
            painter.rect_filled(
                egui::Rect::from_center_size(center, Vec2::splat(20.0)),
                CornerRadius::same(3),
                icon_color,
            );
        } else {
            painter.circle_filled(center, 12.0, icon_color);
        }

        // Click
        if resp.clicked() && !transcribing {
            if recording {
                app.stop_recording();
            } else {
                app.start_recording();
            }
        }

        // Label
        ui.add_space(10.0);
        let dots = animated_dots(time);
        let label = if recording && streaming {
            RichText::new(format!("Recording + Streaming{dots}"))
                .color(REC_RED)
                .size(13.0)
        } else if recording {
            RichText::new(format!("Recording{dots}"))
                .color(REC_RED)
                .size(13.0)
        } else if transcribing {
            RichText::new(format!("Transcribing{dots}"))
                .color(TEXT_DIM)
                .size(13.0)
        } else if streaming {
            RichText::new(format!("Recording + Streaming{dots}"))
                .color(TEXT_DIM)
                .size(13.0)
        } else {
            RichText::new("Click to record")
                .color(TEXT_MUTED)
                .size(13.0)
        };
        ui.label(label);
    });

    ui.add_space(16.0);

    section_heading(ui, "TRANSCRIPT");

    Frame::new()
        .fill(BG_ROW)
        .corner_radius(CornerRadius::same(4))
        .stroke(Stroke::new(1.0, BORDER))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut app.transcript)
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(8)
                            .desired_width(f32::INFINITY)
                            .frame(false)
                            .hint_text(
                                RichText::new("Transcript will appear here...")
                                    .color(TEXT_MUTED),
                            ),
                    );
                });
        });
}
