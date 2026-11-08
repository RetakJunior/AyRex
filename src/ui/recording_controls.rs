use eframe::egui;

use crate::state::{AppState, format_duration};

pub(crate) fn show_recording_controls(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal_centered(|ui| {
        let is_recording = state.recorder.is_some();
        if is_recording {
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("■ KAYDI DURDUR")
                            .color(egui::Color32::WHITE)
                            .strong(),
                    )
                    .fill(egui::Color32::from_rgb(180, 40, 40))
                    .min_size(egui::vec2(120.0, 36.0)),
                )
                .clicked()
            {
                state.stop_recording();
            }
        } else {
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("● KAYDI BAŞLAT")
                            .color(egui::Color32::WHITE)
                            .strong(),
                    )
                    .fill(egui::Color32::from_rgb(38, 140, 68))
                    .min_size(egui::vec2(120.0, 36.0)),
                )
                .clicked()
            {
                state.start_recording();
            }
        }

        ui.add_space(8.0);
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
                .button(format!("↺ Son {requested} sn klip al"))
                .clicked()
        {
            state.save_replay();
        }
        if let Some((requested, buffered)) = replay_status {
            ui.small(format!("Tampon: {buffered}/{requested} sn"));
        }

        ui.separator();
        let timer_color = if is_recording {
            egui::Color32::from_rgb(255, 80, 80)
        } else {
            egui::Color32::from_gray(180)
        };
        ui.label(
            egui::RichText::new(format!("⏱ {}", format_duration(state.recording_duration())))
                .color(timer_color)
                .monospace()
                .size(16.0),
        );

        ui.separator();
        ui.label(state.recording_status());

        ui.separator();
        ui.small("Kısayol: Ctrl+Shift+R (Başlat) · Ctrl+Shift+S (Durdur)");

        if let Some(error) = &state.notice {
            ui.separator();
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
    });
}
