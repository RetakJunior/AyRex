use eframe::egui;

use crate::{state::AppState, ui};

pub fn show_dashboard(ctx: &egui::Context, state: &mut AppState) {
    egui::TopBottomPanel::top("header")
        .exact_height(46.0)
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.heading("AyRex");
                ui.colored_label(egui::Color32::from_rgb(85, 175, 255), "v0.1.8");
                ui.separator();
                ui::show_display_selector(ui, state);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(ffmpeg) = &state.ffmpeg {
                        ui.small(format!(
                            "FFmpeg: {}",
                            ffmpeg.file_name().unwrap_or_default().to_string_lossy()
                        ));
                    } else {
                        ui.colored_label(egui::Color32::LIGHT_RED, "⚠ FFmpeg bulunamadı");
                    }
                });
            });
        });

    egui::TopBottomPanel::bottom("recording_controls")
        .exact_height(64.0)
        .show(ctx, |ui| ui::show_recording_controls(ui, state));

    egui::SidePanel::left("scenes")
        .default_width(190.0)
        .resizable(true)
        .show(ctx, |ui| ui::show_scenes(ui, state));

    egui::SidePanel::right("configuration")
        .default_width(310.0)
        .resizable(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui::show_settings(ui, state);
                ui.separator();
                ui::show_status(ui, state);
            });
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        ui::show_preview(ui, state);
    });
}
