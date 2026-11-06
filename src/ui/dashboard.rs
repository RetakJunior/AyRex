use eframe::egui;

use crate::{state::AppState, ui};

pub fn show_dashboard(ctx: &egui::Context, state: &mut AppState) {
    egui::TopBottomPanel::top("header")
        .exact_height(50.0)
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.heading("AyRex");
                ui.separator();
                ui.label("Screen recorder");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui::show_display_selector(ui, state);
                });
            });
        });

    egui::SidePanel::right("configuration")
        .default_width(300.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui::show_settings(ui, state);
            ui.separator();
            ui::show_status(ui, state);
        });

    egui::TopBottomPanel::bottom("recording_controls")
        .exact_height(68.0)
        .show(ctx, |ui| ui::show_recording_controls(ui, state));

    egui::CentralPanel::default().show(ctx, |ui| {
        ui::show_preview(ui, state);
    });
}
