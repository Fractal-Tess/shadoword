use egui::{Color32, CornerRadius, Stroke, Vec2};

use crate::ui::constants::*;

pub fn apply(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();

    v.panel_fill = BG;
    v.window_fill = BG;
    v.extreme_bg_color = BG_INPUT;
    v.faint_bg_color = BG_ROW;
    v.code_bg_color = BG_INPUT;
    v.hyperlink_color = ACCENT;
    v.warn_fg_color = TEXT;
    v.error_fg_color = TEXT;

    v.selection.bg_fill = Color32::from_rgba_premultiplied(255, 255, 255, 25);
    v.selection.stroke = Stroke::new(1.0, TEXT_DIM);
    v.window_stroke = Stroke::new(1.0, BORDER);

    v.widgets.noninteractive.bg_fill = BG_ROW;
    v.widgets.noninteractive.weak_bg_fill = BG_ROW;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.noninteractive.corner_radius = CornerRadius::same(4);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);

    v.widgets.inactive.bg_fill = BG_INPUT;
    v.widgets.inactive.weak_bg_fill = BG_INPUT;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.inactive.corner_radius = CornerRadius::same(4);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_DIM);

    v.widgets.hovered.bg_fill = BG_HOVER;
    v.widgets.hovered.weak_bg_fill = BG_HOVER;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, TEXT_DIM);
    v.widgets.hovered.corner_radius = CornerRadius::same(4);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);

    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.weak_bg_fill = ACCENT;
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.active.corner_radius = CornerRadius::same(4);
    v.widgets.active.fg_stroke = Stroke::new(1.5, BG);

    v.widgets.open.bg_fill = BG_HOVER;
    v.widgets.open.weak_bg_fill = BG_HOVER;
    v.widgets.open.bg_stroke = Stroke::new(1.0, TEXT_DIM);
    v.widgets.open.corner_radius = CornerRadius::same(4);
    v.widgets.open.fg_stroke = Stroke::new(1.0, TEXT);

    ctx.set_visuals(v);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 4.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    ctx.set_style(style);
}
