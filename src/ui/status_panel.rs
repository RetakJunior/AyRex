use eframe::egui;

use crate::{
    recording::format_bytes,
    state::{AppState, format_duration},
};

pub(crate) fn show_status(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Current recording");
    ui.label(format!("Status: {}", state.recording_status()));
    ui.label(format!(
        "Duration: {}",
        format_duration(state.recording_duration())
    ));
    let (width, height) = state.settings.resolution();
    ui.label(format!("Resolution: {width}x{height}"));
    ui.label(format!(
        "FPS: {}",
        state
            .current_fps()
            .map(|fps| format!("{fps:.1}"))
            .unwrap_or_else(|| if state.recorder.is_some() {
                "Starting".to_owned()
            } else {
                "-".to_owned()
            })
    ));
    ui.label(format!(
        "File size: {}",
        format_bytes(state.recording_size())
    ));
    ui.label(format!("Encoder: {}", state.encoder_display_name()));
    ui.label(format!(
        "Dropped frames: {}",
        state
            .dropped_frames()
            .map(|count| count.to_string())
            .unwrap_or_else(|| "-".to_owned())
    ));
    if let Some(path) = state.current_path() {
        ui.add(egui::Label::new(path.display().to_string()).truncate())
            .on_hover_text(path.display().to_string());
    }
    if let Some(summary) = &state.last_summary {
        ui.label(format!(
            "Saved {} frames in {}",
            summary.frames,
            format_duration(summary.elapsed)
        ));
    }
    ui.label(format!("Audio input: {}", state.audio_status()));
    if ui.button("Open recording folder").clicked() {
        state.open_output_folder();
    }
}
