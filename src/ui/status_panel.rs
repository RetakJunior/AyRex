use eframe::egui;

use crate::{
    recording::format_bytes,
    state::{AppState, format_duration},
};

pub(crate) fn show_status(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Performans ve Durum");
    ui.label(format!(
        "Durum: {}",
        match state.recording_status() {
            "Recording" => "Kayıt alınıyor",
            "Starting" => "Başlatılıyor",
            _ => "Hazır",
        }
    ));
    ui.label(format!(
        "Süre: {}",
        format_duration(state.recording_duration())
    ));
    let (width, height) = state.settings.resolution();
    ui.label(format!("Çözünürlük: {width}x{height}"));
    ui.label(format!(
        "Kare/sn: {}",
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
        "Dosya boyutu: {}",
        format_bytes(state.recording_size())
    ));
    ui.label(format!("Kodlayıcı: {}", state.encoder_display_name()));
    ui.label(format!(
        "CPU: {}",
        state
            .performance
            .cpu_percent
            .map(|value| format!("{value:.0}%"))
            .unwrap_or_else(|| "Ölçülüyor".to_owned())
    ));
    ui.label(format!(
        "RAM: {}",
        state
            .performance
            .ram_percent
            .map(|value| format!(
                "{value:.0}% ({} / {})",
                crate::recording::format_bytes(state.performance.ram_used.unwrap_or_default()),
                crate::recording::format_bytes(state.performance.ram_total.unwrap_or_default())
            ))
            .unwrap_or_else(|| "Ölçülemiyor".to_owned())
    ));
    ui.label(format!(
        "Atlanan kare: {}",
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
            "{} kare {} içinde kaydedildi",
            summary.frames,
            format_duration(summary.elapsed)
        ));
    }
    ui.label(format!("Ses girişi: {}", state.audio_status()));
    if ui.button("Kayıt klasörünü aç").clicked() {
        state.open_output_folder();
    }
}
