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
            .selected_text(selected.unwrap_or("Ekran seçilmedi"))
            .show_ui(ui, |ui| {
                for display in &state.displays {
                    let label = format!(
                        "{} ({}x{}){}",
                        display.name,
                        display.width,
                        display.height,
                        if display.primary { " [Ana Ekran]" } else { "" }
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
    ui.heading("Kayıt Ayarları");
    ui.add_space(4.0);

    ui.add_enabled_ui(state.recorder.is_none(), |ui| {
        ui.group(|ui| {
            ui.label(egui::RichText::new("Temel Video & Ses").strong());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Çözünürlük:");
                egui::ComboBox::from_id_salt("resolution")
                    .selected_text(match state.settings.resolution_index {
                        1 => "1920 x 1080 (Full HD)",
                        2 => "Özel",
                        _ => "1280 x 720 (HD)",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut state.settings.resolution_index,
                            0,
                            "1280 x 720 (HD)",
                        );
                        ui.selectable_value(
                            &mut state.settings.resolution_index,
                            1,
                            "1920 x 1080 (Full HD)",
                        );
                        ui.selectable_value(&mut state.settings.resolution_index, 2, "Özel");
                    });
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

            ui.horizontal(|ui| {
                ui.label("Kare Hızı (FPS):");
                egui::ComboBox::from_id_salt("fps")
                    .selected_text(if state.settings.fps == 0 {
                        format!("Özel ({} FPS)", state.settings.custom_fps)
                    } else {
                        format!("{} FPS", state.settings.fps)
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.settings.fps, 60, "60 FPS");
                        ui.selectable_value(&mut state.settings.fps, 30, "30 FPS");
                        ui.selectable_value(&mut state.settings.fps, 0, "Özel");
                    });
            });

            if state.settings.fps == 0 {
                ui.horizontal(|ui| {
                    ui.label("Özel FPS:");
                    ui.add(
                        egui::DragValue::new(&mut state.settings.custom_fps)
                            .range(1..=144)
                            .suffix(" FPS"),
                    );
                });
            }

            ui.horizontal(|ui| {
                ui.label("Kodlayıcı:");
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
            });

            ui.horizontal(|ui| {
                ui.label("Kayıt Biçimi:");
                ui.selectable_value(&mut state.settings.container, Container::Mkv, "MKV");
                ui.selectable_value(&mut state.settings.container, Container::Mp4, "MP4");
            });

            ui.horizontal(|ui| {
                ui.label("Video Bit Hızı:");
                ui.add(
                    egui::DragValue::new(&mut state.settings.bitrate_kbps)
                        .range(500..=100_000)
                        .suffix(" kbps"),
                );
            });

            ui.separator();

            ui.label(egui::RichText::new("Ses Girişleri").strong());
            ui.horizontal(|ui| {
                ui.label("🎤 Mikrofon:");
                egui::ComboBox::from_id_salt("audio")
                    .selected_text(state.settings.audio_device.as_deref().unwrap_or("Kapalı"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.settings.audio_device, None, "Kapalı");
                        for device in &state.audio_devices {
                            ui.selectable_value(
                                &mut state.settings.audio_device,
                                Some(device.clone()),
                                device,
                            );
                        }
                    });
            });

            if state.settings.audio_device.is_some() {
                ui.horizontal(|ui| {
                    ui.label("Mikrofon Düzeyi:");
                    ui.add(egui::Slider::new(
                        &mut state.settings.audio_volume,
                        0.0..=1.0,
                    ));
                });
                ui.checkbox(&mut state.settings.audio_muted, "Mikrofonu sessize al");
            }

            ui.checkbox(
                &mut state.settings.system_audio_enabled,
                if cfg!(windows) {
                    "🔊 Masaüstü sesini kaydet (WASAPI)"
                } else {
                    "🔊 Masaüstü sesini kaydet (PipeWire / Pulse)"
                },
            );

            if state.settings.system_audio_enabled {
                ui.horizontal(|ui| {
                    ui.label("Masaüstü Düzeyi:");
                    ui.add(egui::Slider::new(
                        &mut state.settings.system_audio_volume,
                        0.0..=1.0,
                    ));
                });
            }

            ui.separator();

            ui.label(egui::RichText::new("Kayıt Klasörü").strong());
            ui.horizontal(|ui| {
                ui.add(egui::Label::new(state.settings.output_dir.display().to_string()).truncate())
                    .on_hover_text(state.settings.output_dir.display().to_string());
                if ui.button("Gözat…").clicked() {
                    state.choose_output_dir();
                }
            });
        });

        ui.add_space(4.0);

        ui.collapsing("📡 Canlı Yayın (Kick / Twitch / RTMP)", |ui| {
            ui.checkbox(
                &mut state.settings.stream_enabled,
                "Yayını etkinleştir (kayıtla birlikte)",
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
                                .hint_text("Yayın anahtarınızı yapıştırın"),
                        );
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
                    }
                }
                ui.small(format!(
                    "En fazla {} kbps desteklenir.",
                    state.settings.stream_platform.max_bitrate_kbps()
                ));
            }
        });

        ui.collapsing("⚙ Gelişmiş Ayarlar & Efektler", |ui| {
            #[cfg(windows)]
            {
                let dxgi_available = state
                    .selected_display()
                    .is_some_and(|display| display.dxgi_output_index.is_some());
                ui.add_enabled_ui(dxgi_available, |ui| {
                    ui.checkbox(
                        &mut state.settings.use_desktop_duplication,
                        "DXGI Desktop Duplication (Akıcı GPU yakalama)",
                    );
                });
                if !dxgi_available {
                    ui.small("Seçilen ekran DXGI ile eşleşmedi; GDI yakalama kullanılır.");
                }
            }

            ui.collapsing("Replay Buffer (Anlık Tekrar Kaydı)", |ui| {
                ui.checkbox(
                    &mut state.settings.replay_buffer_enabled,
                    "Replay Buffer'ı etkinleştir",
                );
                if state.settings.replay_buffer_enabled {
                    ui.add(
                        egui::Slider::new(&mut state.settings.replay_buffer_seconds, 5..=300)
                            .suffix(" saniye"),
                    );
                    ui.small("Kayıt sırasında son saniyeleri tamponda tutar; butona basınca ayrı klip kaydeder.");
                }
            });

            if state.settings.audio_device.is_some() {
                ui.collapsing("Ses Filtreleri (Gürültü Engelleme)", |ui| {
                    ui.checkbox(&mut state.settings.noise_suppression, "Gürültü azaltma (afftdn)");
                    ui.checkbox(&mut state.settings.noise_gate, "Noise Gate");
                    ui.checkbox(&mut state.settings.compressor, "Kompresör");
                    ui.checkbox(&mut state.settings.limiter, "Limitleyici");
                });
            }

            ui.collapsing("Video Renk ve Keskinlik", |ui| {
                ui.add(egui::Slider::new(&mut state.settings.brightness, -0.5..=0.5).text("Parlaklık"));
                ui.add(egui::Slider::new(&mut state.settings.contrast, 0.0..=2.0).text("Kontrast"));
                ui.add(egui::Slider::new(&mut state.settings.saturation, 0.0..=3.0).text("Doygunluk"));
                ui.add(egui::Slider::new(&mut state.settings.sharpen, 0.0..=2.0).text("Keskinlik"));
                ui.checkbox(
                    &mut state.settings.camera_chroma_key,
                    "Kamerada yeşil perde (Chroma Key)",
                );
            });

            ui.horizontal(|ui| {
                if ui.button("Profil Kaydet…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name("AyRex-Profili.json")
                        .add_filter("AyRex profili", &["json"])
                        .save_file()
                    {
                        state.save_profile(&path);
                    }
                }
                if ui.button("Profil Yükle…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("AyRex profili", &["json"])
                        .pick_file()
                    {
                        state.load_profile(&path);
                    }
                }
            });
        });
    });
}
