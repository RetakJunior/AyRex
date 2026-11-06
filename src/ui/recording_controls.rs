use eframe::egui;

use crate::state::{AppState, format_duration};

pub(crate) fn show_recording_controls(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal_centered(|ui| {
        if ui
            .add_enabled(
                state.recorder.is_none(),
                egui::Button::new("● Kaydı Başlat"),
            )
            .clicked()
        {
            state.start_recording();
        }
        if ui
            .add_enabled(state.recorder.is_some(), egui::Button::new("■ Durdur"))
            .clicked()
        {
            state.stop_recording();
        }
        let replay_status = state.recorder.as_ref().and_then(|recorder| {
            (recorder.config().replay_buffer_seconds > 0).then(|| {
                (
                    recorder.config().replay_buffer_seconds,
                    recorder.replay_buffered_seconds(),
                )
            })
        });
        if let Some((requested, _buffered)) = replay_status
            && ui
                .button(format!("↺ Son {requested} sn'i kaydet"))
                .clicked()
        {
            state.save_replay();
        }
        if let Some((requested, buffered)) = replay_status {
            ui.small(format!("Buffer {buffered}/{requested} sn"));
        }
        ui.separator();
        ui.small("Ctrl+Shift+R: Başlat · Ctrl+Shift+S: Durdur");
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
