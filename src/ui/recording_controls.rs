use eframe::egui;

use crate::state::{AppState, format_duration};

pub(crate) fn show_recording_controls(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal_centered(|ui| {
        if ui
            .add_enabled(
                state.recorder.is_none(),
                egui::Button::new("Start recording"),
            )
            .clicked()
        {
            state.start_recording();
        }
        if ui
            .add_enabled(
                state.recorder.is_some(),
                egui::Button::new("Stop recording"),
            )
            .clicked()
        {
            state.stop_recording();
        }
        ui.separator();
        ui.strong(format_duration(state.recording_duration()));
        ui.separator();
        ui.label(state.recording_status());
        if let Some(error) = &state.notice {
            ui.separator();
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
    });
}
