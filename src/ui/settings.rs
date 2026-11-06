use eframe::egui;

use crate::{config::Container, recording::encoder_label, state::AppState};

pub(crate) fn show_display_selector(ui: &mut egui::Ui, state: &mut AppState) {
    ui.add_enabled_ui(state.recorder.is_none(), |ui| {
        let selected = state
            .selected_display()
            .map(|display| display.name.as_str());
        let mut selected_id = None;
        egui::ComboBox::from_id_salt("monitor")
            .selected_text(selected.unwrap_or("No monitor"))
            .show_ui(ui, |ui| {
                for display in &state.displays {
                    let label = format!(
                        "{} ({}x{}){}",
                        display.name,
                        display.width,
                        display.height,
                        if display.primary { " - Primary" } else { "" }
                    );
                    if ui
                        .selectable_label(Some(display.id) == state.selected_display_id, label)
                        .clicked()
                    {
                        selected_id = Some(display.id);
                    }
                }
            });
        if let Some(id) = selected_id {
            state.select_display(id);
        }
    });
}

pub(crate) fn show_settings(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Recording");
    ui.add_space(8.0);
    ui.add_enabled_ui(state.recorder.is_none(), |ui| {
        ui.label("Resolution");
        egui::ComboBox::from_id_salt("resolution")
            .selected_text(match state.settings.resolution_index {
                1 => "1920 x 1080",
                2 => "Custom",
                _ => "1280 x 720",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.settings.resolution_index, 0, "1280 x 720");
                ui.selectable_value(&mut state.settings.resolution_index, 1, "1920 x 1080");
                ui.selectable_value(&mut state.settings.resolution_index, 2, "Custom");
            });
        if state.settings.resolution_index == 2 {
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut state.settings.custom_width)
                        .range(2..=7680)
                        .suffix(" w"),
                );
                ui.add(
                    egui::DragValue::new(&mut state.settings.custom_height)
                        .range(2..=4320)
                        .suffix(" h"),
                );
            });
        }

        ui.add_space(8.0);
        ui.label("Frame rate");
        egui::ComboBox::from_id_salt("fps")
            .selected_text(if state.settings.fps == 0 {
                format!("Custom ({} FPS)", state.settings.custom_fps)
            } else {
                format!("{} FPS", state.settings.fps)
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.settings.fps, 30, "30 FPS");
                ui.selectable_value(&mut state.settings.fps, 60, "60 FPS");
                ui.selectable_value(&mut state.settings.fps, 0, "Custom");
            });
        if state.settings.fps == 0 {
            ui.add(
                egui::DragValue::new(&mut state.settings.custom_fps)
                    .range(1..=120)
                    .suffix(" FPS"),
            );
        }

        ui.add_space(8.0);
        ui.label("Encoder");
        egui::ComboBox::from_id_salt("encoder")
            .selected_text(encoder_label(&state.settings.encoder))
            .show_ui(ui, |ui| {
                for encoder in &state.encoders {
                    ui.selectable_value(
                        &mut state.settings.encoder,
                        encoder.clone(),
                        encoder_label(encoder),
                    );
                }
            });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Video bitrate");
            ui.add(
                egui::DragValue::new(&mut state.settings.bitrate_kbps)
                    .range(250..=100_000)
                    .suffix(" kbps"),
            );
        });

        ui.add_space(8.0);
        ui.label("File format");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut state.settings.container, Container::Mkv, "MKV");
            ui.selectable_value(&mut state.settings.container, Container::Mp4, "MP4");
        });

        ui.add_space(8.0);
        ui.label("Audio input (optional)");
        egui::ComboBox::from_id_salt("audio")
            .selected_text(state.settings.audio_device.as_deref().unwrap_or("Off"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.settings.audio_device, None, "Off");
                for device in &state.audio_devices {
                    ui.selectable_value(
                        &mut state.settings.audio_device,
                        Some(device.clone()),
                        device,
                    );
                }
            });
        if state.settings.audio_device.is_some() {
            ui.horizontal(|ui| {
                ui.label("Input volume");
                ui.add(egui::Slider::new(
                    &mut state.settings.audio_volume,
                    0.0..=1.0,
                ));
            });
            ui.checkbox(&mut state.settings.audio_muted, "Mute audio input");
        }

        ui.add_space(8.0);
        ui.label("Output folder");
        ui.horizontal(|ui| {
            ui.add(egui::Label::new(state.settings.output_dir.display().to_string()).truncate())
                .on_hover_text(state.settings.output_dir.display().to_string());
            if ui.button("Browse").clicked() {
                state.choose_output_dir();
            }
        });
    });
}
