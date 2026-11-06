use eframe::egui;

use crate::{
    config::{SceneSource, SceneSourceKind, SourceTransform},
    state::AppState,
};

pub(crate) fn show_scenes(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Sahneler");
    ui.add_space(6.0);
    ui.add_enabled_ui(state.recorder.is_none(), |ui| {
        ui.checkbox(&mut state.settings.studio_mode, "Studio modu");
        if state.settings.studio_mode {
            let program_name = state
                .settings
                .scenes
                .get(state.settings.program_scene)
                .map(|scene| scene.name.as_str())
                .unwrap_or("Sahne yok");
            egui::ComboBox::from_id_salt("program_scene")
                .selected_text(format!("Program: {program_name}"))
                .show_ui(ui, |ui| {
                    for (index, scene) in state.settings.scenes.iter().enumerate() {
                        ui.selectable_value(&mut state.settings.program_scene, index, &scene.name);
                    }
                });
            if ui.button("Önizlemeyi Programa gönder · Cut").clicked() {
                state.settings.program_scene = state.settings.active_scene;
            }
            ui.small("Kayıt ve yayın Program sahnesini kullanır.");
        }
    });
    ui.add_space(6.0);
    let mut selected = None;
    for (index, scene) in state.settings.scenes.iter().enumerate() {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    index == state.settings.active_scene,
                    format!(
                        "{}{}",
                        if index == state.settings.active_scene {
                            "▶ "
                        } else {
                            ""
                        },
                        scene.name
                    ),
                )
                .clicked()
            {
                selected = Some(index);
            }
            if state.settings.studio_mode && index == state.settings.program_scene {
                ui.small("PROGRAM");
            }
        });
    }
    if let Some(index) = selected {
        state.select_scene(index);
    }

    ui.add_space(8.0);
    ui.add_enabled_ui(state.recorder.is_none(), |ui| {
        ui.horizontal(|ui| {
            if ui.button("+").on_hover_text("Sahne ekle").clicked() {
                state.add_scene();
            }
            if ui.button("Kopyala").clicked() {
                state.duplicate_scene();
            }
            if ui.button("−").on_hover_text("Seçili sahneyi sil").clicked() {
                state.delete_scene();
            }
        });
        if let Some(scene) = state.settings.scenes.get_mut(state.settings.active_scene) {
            ui.add_space(6.0);
            ui.label("Sahne adı");
            ui.text_edit_singleline(&mut scene.name);
            ui.add_space(5.0);
            ui.label("Sahne kaynağı");
            let display = state
                .displays
                .iter()
                .find(|display| display.id == scene.display_id);
            if let Some(display) = display {
                ui.small("Görüntü kaynağı");
                ui.label(format!(
                    "▣ {} · {}×{}",
                    display.name, display.width, display.height
                ));
            }
            ui.add_space(8.0);
            ui.label("Kaynaklar · birden fazla eklenebilir");
            let mut add_window = None;
            egui::ComboBox::from_id_salt("scene_window_add")
                .selected_text("Pencere kaynağı ekle")
                .show_ui(ui, |ui| {
                    for window in &state.windows {
                        let label = format!("{} — {}", window.app, window.title);
                        if ui.selectable_label(false, label).clicked() {
                            add_window = Some((window.app.clone(), window.title.clone()));
                        }
                    }
                });
            if let Some((app, title)) = add_window {
                if !scene
                    .sources
                    .iter()
                    .any(|source| source.kind == SceneSourceKind::Window && source.value == title)
                {
                    scene.sources.push(SceneSource {
                        name: format!("{app} — {title}"),
                        kind: SceneSourceKind::Window,
                        value: title,
                        visible: true,
                        transform: None,
                    });
                }
            }
            if state.windows.is_empty() {
                ui.small("Açık pencere bulunamadı");
            }
            let mut add_camera = None;
            egui::ComboBox::from_id_salt("scene_camera_add")
                .selected_text("Kamera kaynağı ekle")
                .show_ui(ui, |ui| {
                    for camera in &state.camera_devices {
                        if scene.sources.iter().any(|source| {
                            source.kind == SceneSourceKind::Camera && source.value == *camera
                        }) {
                            continue;
                        }
                        if ui.selectable_label(false, camera).clicked() {
                            add_camera = Some(camera.clone());
                        }
                    }
                });
            if let Some(camera) = add_camera {
                scene.sources.push(SceneSource {
                    name: camera.clone(),
                    kind: SceneSourceKind::Camera,
                    value: camera,
                    visible: true,
                    transform: None,
                });
            }
            if state.camera_devices.is_empty() {
                ui.small("Kamera bulunamadı (FFmpeg DirectShow)");
            }
            if ui.button("Görsel kaynağı ekle…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Görsel", &["png", "jpg", "jpeg", "webp", "bmp"])
                    .pick_file()
                {
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    scene.sources.push(SceneSource {
                        name,
                        kind: SceneSourceKind::Image,
                        value: path.to_string_lossy().into_owned(),
                        visible: true,
                        transform: None,
                    });
                }
            }
            if ui.button("Video kaynağı ekle…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Video", &["mp4", "mkv", "mov", "webm", "avi"])
                    .pick_file()
                {
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    scene.sources.push(SceneSource {
                        name,
                        kind: SceneSourceKind::Video,
                        value: path.to_string_lossy().into_owned(),
                        visible: true,
                        transform: None,
                    });
                }
            }
            if ui.button("Metin kaynağı ekle").clicked() {
                let count = scene
                    .sources
                    .iter()
                    .filter(|source| source.kind == SceneSourceKind::Text)
                    .count();
                scene.sources.push(SceneSource {
                    name: format!("Metin {}", count + 1),
                    kind: SceneSourceKind::Text,
                    value: "AyRex".to_owned(),
                    visible: true,
                    transform: None,
                });
            }
            let mut remove_source = None;
            for (index, source) in scene.sources.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add_enabled(
                        source.kind != SceneSourceKind::Display,
                        egui::Checkbox::new(&mut source.visible, ""),
                    );
                    ui.add(
                        egui::Label::new(match source.kind {
                            SceneSourceKind::Display => "▣ Ekran".to_owned(),
                            SceneSourceKind::Camera => format!("◉ {}", source.name),
                            SceneSourceKind::Image => format!("▧ {}", source.name),
                            SceneSourceKind::Window => format!("▣ {}", source.name),
                            SceneSourceKind::Video => format!("▶ {}", source.name),
                            SceneSourceKind::Text => format!("T {}", source.name),
                        })
                        .truncate(),
                    );
                    if source.kind != SceneSourceKind::Display && ui.small_button("×").clicked() {
                        remove_source = Some(index);
                    }
                });
                if source.kind != SceneSourceKind::Display {
                    egui::CollapsingHeader::new("Konum · boyut · kırpma")
                        .id_salt(("source-transform", index))
                        .show(ui, |ui| {
                            let transform = source
                                .transform
                                .get_or_insert_with(|| SourceTransform::for_kind(source.kind));
                            ui.horizontal(|ui| {
                                percent_value(ui, "X", &mut transform.x, 100.0);
                                percent_value(ui, "Y", &mut transform.y, 100.0);
                            });
                            ui.horizontal(|ui| {
                                percent_value(ui, "Genişlik", &mut transform.width, 100.0);
                                percent_value(ui, "Yükseklik", &mut transform.height, 100.0);
                            });
                            ui.horizontal(|ui| {
                                percent_value(ui, "Sol kırp", &mut transform.crop_left, 80.0);
                                percent_value(ui, "Üst", &mut transform.crop_top, 80.0);
                            });
                            ui.horizontal(|ui| {
                                percent_value(ui, "Sağ", &mut transform.crop_right, 80.0);
                                percent_value(ui, "Alt", &mut transform.crop_bottom, 80.0);
                            });
                            *transform = transform.clamped();
                        });
                }
                if source.kind == SceneSourceKind::Text {
                    ui.text_edit_singleline(&mut source.value);
                }
            }
            if let Some(index) = remove_source {
                scene.sources.remove(index);
            }
        }
    });
}

fn percent_value(ui: &mut egui::Ui, label: &str, value: &mut f32, maximum: f64) {
    ui.label(label);
    let mut percent = (*value * 100.0) as f64;
    ui.add(
        egui::DragValue::new(&mut percent)
            .speed(0.2)
            .range(0.0..=maximum)
            .suffix("%"),
    );
    *value = (percent as f32 / 100.0).clamp(0.0, 1.0);
}
