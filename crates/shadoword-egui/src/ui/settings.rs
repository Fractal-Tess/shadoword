use egui::RichText;

use crate::ui::constants::*;
use crate::ui::helpers::{section_heading, setting_row, setting_row_lr};
use crate::{PasteMethod, ServiceMode, ShadowwordApp, TypingTool};

pub fn show(ui: &mut egui::Ui, app: &mut ShadowwordApp) {
    // ── MODE ────────────────────────────────────────────────────────
    section_heading(ui, "MODE");

    setting_row_lr(ui, BG_ROW, "Service Mode", |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut app.config.mode,
                ServiceMode::Local,
                RichText::new("Local").size(13.0),
            );
            ui.selectable_value(
                &mut app.config.mode,
                ServiceMode::Remote,
                RichText::new("Remote").size(13.0),
            );
        });
    });

    // ── HOTKEY ──────────────────────────────────────────────────────
    section_heading(ui, "HOTKEY");

    setting_row_lr(ui, BG_ROW, "Push To Talk", |ui| {
        ui.checkbox(&mut app.config.hotkey.push_to_talk, "");
    });

    setting_row(ui, BG_ROW_ALT, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Shortcut").size(14.0).color(TEXT));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if app.editing_shortcut {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut app.config.hotkey.shortcut)
                            .desired_width(180.0)
                            .hint_text("e.g. ctrl+space"),
                    );
                    if resp.lost_focus() {
                        app.editing_shortcut = false;
                    } else {
                        resp.request_focus();
                    }
                } else {
                    let label_text = if app.config.hotkey.shortcut.trim().is_empty() {
                        "Not set"
                    } else {
                        &app.config.hotkey.shortcut
                    };
                    if ui
                        .button(RichText::new(label_text).size(13.0).monospace())
                        .clicked()
                    {
                        app.editing_shortcut = true;
                    }
                }
            });
        });
    });

    if let Some(err) = &app.shortcut_error {
        setting_row(ui, BG_ROW, |ui| {
            ui.label(RichText::new(err).size(12.0).color(REC_RED));
        });
    }

    // ── SOUND ───────────────────────────────────────────────────────
    section_heading(ui, "SOUND");

    let device_name = app.device_name().to_string();
    setting_row(ui, BG_ROW, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Microphone").size(14.0).color(TEXT));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Refresh").clicked() {
                    app.refresh_local_devices();
                }
                egui::ComboBox::new("mic_combo", "")
                    .selected_text(&device_name)
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut app.config.recording.input_device,
                            None,
                            "System default",
                        );
                        for dev in &app.available_inputs {
                            let label = if dev.is_default {
                                format!("{} (default)", dev.name)
                            } else {
                                dev.name.clone()
                            };
                            ui.selectable_value(
                                &mut app.config.recording.input_device,
                                Some(dev.name.clone()),
                                label,
                            );
                        }
                    });
            });
        });
    });

    setting_row_lr(ui, BG_ROW_ALT, "Sample Rate", |ui| {
        ui.add(
            egui::DragValue::new(&mut app.config.recording.sample_rate)
                .range(8_000..=96_000)
                .speed(100.0)
                .suffix(" Hz"),
        );
    });

    // ── OUTPUT ──────────────────────────────────────────────────────
    section_heading(ui, "OUTPUT");

    setting_row_lr(ui, BG_ROW, "Copy to Clipboard", |ui| {
        ui.checkbox(&mut app.config.output.copy_to_clipboard, "");
    });

    setting_row_lr(ui, BG_ROW_ALT, "Active Window Output", |ui| {
        egui::ComboBox::new("paste_method", "")
            .selected_text(match app.config.output.paste_method {
                PasteMethod::None => "None",
                PasteMethod::Direct => "Direct",
                PasteMethod::CtrlV => "Ctrl+V",
                PasteMethod::CtrlShiftV => "Ctrl+Shift+V",
                PasteMethod::ShiftInsert => "Shift+Insert",
            })
            .width(180.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.config.output.paste_method, PasteMethod::None, "None");
                ui.selectable_value(&mut app.config.output.paste_method, PasteMethod::Direct, "Direct");
                ui.selectable_value(&mut app.config.output.paste_method, PasteMethod::CtrlV, "Ctrl+V");
                ui.selectable_value(&mut app.config.output.paste_method, PasteMethod::CtrlShiftV, "Ctrl+Shift+V");
                ui.selectable_value(&mut app.config.output.paste_method, PasteMethod::ShiftInsert, "Shift+Insert");
            });
    });

    #[cfg(target_os = "linux")]
    setting_row_lr(ui, BG_ROW, "Typing Tool", |ui| {
        egui::ComboBox::new("typing_tool", "")
            .selected_text(match app.config.output.typing_tool {
                TypingTool::Auto => "Auto",
                TypingTool::Wtype => "wtype",
                TypingTool::Kwtype => "kwtype",
                TypingTool::Dotool => "dotool",
                TypingTool::Ydotool => "ydotool",
                TypingTool::Xdotool => "xdotool",
            })
            .width(180.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.config.output.typing_tool, TypingTool::Auto, "Auto");
                ui.selectable_value(&mut app.config.output.typing_tool, TypingTool::Wtype, "wtype");
                ui.selectable_value(&mut app.config.output.typing_tool, TypingTool::Kwtype, "kwtype");
                ui.selectable_value(&mut app.config.output.typing_tool, TypingTool::Dotool, "dotool");
                ui.selectable_value(&mut app.config.output.typing_tool, TypingTool::Ydotool, "ydotool");
                ui.selectable_value(&mut app.config.output.typing_tool, TypingTool::Xdotool, "xdotool");
            });
    });

    setting_row_lr(ui, BG_ROW_ALT, "Paste Delay", |ui| {
        ui.add(
            egui::DragValue::new(&mut app.config.output.paste_delay_ms)
                .range(0..=1000)
                .speed(5.0)
                .suffix(" ms"),
        );
    });

    // ── REMOTE (conditional) ────────────────────────────────────────
    if app.config.mode == ServiceMode::Remote {
        section_heading(ui, "REMOTE DAEMON");

        setting_row(ui, BG_ROW, |ui| {
            ui.label(RichText::new("Endpoint").size(14.0).color(TEXT));
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::singleline(&mut app.config.remote.endpoint)
                    .desired_width(ui.available_width()),
            );
        });

        setting_row(ui, BG_ROW_ALT, |ui| {
            ui.label(RichText::new("Listen Address").size(14.0).color(TEXT));
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::singleline(&mut app.config.daemon.listen_addr)
                    .desired_width(ui.available_width()),
            );
        });

        setting_row(ui, BG_ROW, |ui| {
            ui.horizontal(|ui| {
                if ui.button(RichText::new("Test Connection").size(13.0)).clicked() {
                    app.test_remote_connection();
                }
                if ui.button(RichText::new("Pull Config").size(13.0)).clicked() {
                    app.pull_remote_config();
                }
                if ui.button(RichText::new("Push Config").size(13.0)).clicked() {
                    app.push_remote_config();
                }
            });
        });

        if let Some(result) = &app.connection_test {
            let (color, icon) = if result.success {
                (STATUS_OK, "OK")
            } else {
                (REC_RED, "FAIL")
            };
            setting_row(ui, BG_ROW_ALT, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(icon).size(12.0).color(color).strong());
                    ui.label(RichText::new(&result.message).size(12.0).color(TEXT_DIM));
                });
            });
        }
    }
}
