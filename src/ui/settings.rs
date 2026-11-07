use eframe::egui;

use crate::{
    config::Container, recording::encoder_label, state::AppState, streaming::StreamPlatform,
};

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
    ui.heading("Kayıt ve Yayın");
    ui.add_space(8.0);
    ui.add_enabled_ui(state.recorder.is_none(), |ui| {
        #[cfg(windows)]
        {
            let dxgi_available = state
                .selected_display()
                .is_some_and(|display| display.dxgi_output_index.is_some());
            ui.add_enabled_ui(dxgi_available, |ui| {
                ui.checkbox(
                    &mut state.settings.use_desktop_duplication,
                    "Ekranı DXGI Desktop Duplication ile yakala",
                );
            });
            if !dxgi_available {
                ui.small("Seçilen ekran DXGI ile eşleşmedi; GDI yakalama kullanılır.");
            }
        }
        #[cfg(target_os = "linux")]
        {
            if crate::capture::is_wayland_session() {
                ui.small("Wayland kaydında Ubuntu ekran paylaşımı izni isteyecek; ekrandaki seçiciden kaydedilecek ekranı seç.");
            } else {
                ui.small("Linux ekran yakalama X11 oturumunu kullanıyor.");
            }
        }

        ui.horizontal(|ui| {
            if ui.button("Profil kaydet…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("AyRex profili.ayrex.json")
                    .add_filter("AyRex profili", &["json"])
                    .save_file()
                {
                    state.save_profile(&path);
                }
            }
            if ui.button("Profil yükle…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("AyRex profili", &["json"])
                    .pick_file()
                {
                    state.load_profile(&path);
                }
            }
        });
        ui.add_space(6.0);
        ui.label("Çözünürlük");
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
        ui.label("Kare hızı");
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
        ui.label("Kodlayıcı");
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
            ui.label("Video bit hızı");
            ui.add(
                egui::DragValue::new(&mut state.settings.bitrate_kbps)
                    .range(250..=100_000)
                    .suffix(" kbps"),
            );
        });

        ui.add_space(8.0);
        ui.label("Kayıt biçimi");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut state.settings.container, Container::Mkv, "MKV");
            ui.selectable_value(&mut state.settings.container, Container::Mp4, "MP4");
        });

        ui.add_space(8.0);
        ui.label("Mikrofon / ses girişi");
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
                ui.label("Giriş seviyesi");
                ui.add(egui::Slider::new(
                    &mut state.settings.audio_volume,
                    0.0..=1.0,
                ));
            });
            ui.checkbox(&mut state.settings.audio_muted, "Sesi kapat");
            ui.collapsing("Ses filtreleri", |ui| {
                ui.checkbox(&mut state.settings.noise_suppression, "Gürültü azaltma");
                ui.checkbox(&mut state.settings.noise_gate, "Noise Gate");
                ui.checkbox(&mut state.settings.compressor, "Compressor");
                ui.checkbox(&mut state.settings.limiter, "Limiter");
            });
        }
        ui.add_space(6.0);
        ui.checkbox(
            &mut state.settings.system_audio_enabled,
            if cfg!(windows) {
                "Masaüstü sesini yakala (WASAPI)"
            } else {
                "Masaüstü sesini yakala (PulseAudio / PipeWire)"
            },
        );
        if state.settings.system_audio_enabled {
            ui.horizontal(|ui| {
                ui.label("Masaüstü ses seviyesi");
                ui.add(egui::Slider::new(
                    &mut state.settings.system_audio_volume,
                    0.0..=1.0,
                ));
            });
            ui.small(if cfg!(windows) {
                "Varsayılan Windows ses çıkışı kayıt akışına eklenir."
            } else {
                "Varsayılan ses çıkışının monitor kaynağı kayıt akışına eklenir."
            });
        }

        ui.collapsing("Video filtreleri", |ui| {
            ui.add(egui::Slider::new(&mut state.settings.brightness, -0.5..=0.5).text("Parlaklık"));
            ui.add(egui::Slider::new(&mut state.settings.contrast, 0.0..=2.0).text("Kontrast"));
            ui.add(egui::Slider::new(&mut state.settings.saturation, 0.0..=3.0).text("Doygunluk"));
            ui.add(egui::Slider::new(&mut state.settings.sharpen, 0.0..=2.0).text("Keskinlik"));
            ui.checkbox(
                &mut state.settings.camera_chroma_key,
                "Kamerada yeşil perde (Chroma Key)",
            );
        });

        ui.separator();
        ui.checkbox(
            &mut state.settings.stream_enabled,
            "Yayın yap (kayıtla birlikte)",
        );
        if state.settings.stream_enabled {
            let current_platform = state.settings.stream_platform;
            let mut selected_platform = current_platform;
            egui::ComboBox::from_id_salt("stream_platform")
                .selected_text(current_platform.label())
                .show_ui(ui, |ui| {
                    for platform in [
                        StreamPlatform::Kick,
                        StreamPlatform::Twitch,
                        StreamPlatform::Custom,
                    ] {
                        ui.selectable_value(
                            &mut selected_platform,
                            platform,
                            platform.label(),
                        );
                    }
                });
            if selected_platform != current_platform {
                state.select_stream_platform(selected_platform);
            }

            match state.settings.stream_platform {
                StreamPlatform::Kick | StreamPlatform::Twitch => {
                    ui.label("Sunucu adresi");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.settings.stream_url)
                            .hint_text(state.settings.stream_platform.default_server()),
                    );
                    ui.label("Yayın anahtarı");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.stream_key)
                            .password(true)
                            .hint_text("Platform yayın panelindeki anahtar"),
                    );
                    ui.small("Anahtar diske kaydedilmez. Sunucu kök adresiyse /app otomatik eklenir.");
                    if state.settings.stream_platform == StreamPlatform::Kick {
                        ui.hyperlink_to(
                            "Kick yayın paneli",
                            "https://dashboard.kick.com/stream",
                        );
                    } else {
                        ui.hyperlink_to(
                            "Twitch yayın paneli",
                            "https://dashboard.twitch.tv/settings/stream",
                        );
                    }
                }
                StreamPlatform::Custom => {
                    ui.label("Özel RTMP/RTMPS veya SRT adresi");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.custom_stream_url)
                            .password(true)
                            .hint_text("rtmp(s)://sunucu/app/anahtar veya srt://sunucu:port"),
                    );
                    ui.small("Özel yayın adresi uygulama kapatılınca saklanmaz.");
                }
            }
            ui.small(format!(
                "En fazla {} kbps; yayın için H.264 kullan.",
                state.settings.stream_platform.max_bitrate_kbps()
            ));
        }

        ui.collapsing("Replay Buffer", |ui| {
            ui.checkbox(
                &mut state.settings.replay_buffer_enabled,
                "Kayıtla birlikte belleği döndür",
            );
            if state.settings.replay_buffer_enabled {
                ui.add(
                    egui::Slider::new(&mut state.settings.replay_buffer_seconds, 5..=600)
                        .suffix(" sn"),
                );
                ui.small("Kayıt sırasında son bölümler geçici tutulur; düğmeyle yeni bir klip olarak saklanır.");
            }
        });

        ui.add_space(8.0);
        ui.label("Kayıt klasörü");
        ui.horizontal(|ui| {
            ui.add(egui::Label::new(state.settings.output_dir.display().to_string()).truncate())
                .on_hover_text(state.settings.output_dir.display().to_string());
            if ui.button("Gözat").clicked() {
                state.choose_output_dir();
            }
        });
    });
}
