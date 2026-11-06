use eframe::egui::{self, ColorImage, TextureOptions};

use crate::config::{SceneSourceKind, SourceTransform};
use crate::state::AppState;

pub(crate) fn update_preview(state: &mut AppState, ctx: &egui::Context) {
    let Some(preview) = &state.preview else {
        return;
    };
    match preview.try_receive() {
        Ok(Some(frame)) => {
            let image =
                ColorImage::from_rgba_unmultiplied([frame.width, frame.height], &frame.pixels);
            if let Some(texture) = &mut state.preview_texture {
                texture.set(image, TextureOptions::LINEAR);
            } else {
                state.preview_texture =
                    Some(ctx.load_texture("screen-preview", image, TextureOptions::LINEAR));
            }
        }
        Ok(None) => {}
        Err(error) => state.notice = Some(format!("Screen preview failed: {error}")),
    }
}

pub(crate) fn show_preview(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.heading("Canlı Önizleme");
        ui.add_enabled_ui(state.recorder.is_none(), |ui| {
            if ui
                .selectable_label(state.capture_region_selecting, "Kayıt alanı seç")
                .clicked()
            {
                state.capture_region_selecting = !state.capture_region_selecting;
                state.capture_region_drag_start = None;
            }
            let has_region = state
                .settings
                .scenes
                .get(state.settings.active_scene)
                .is_some_and(|scene| scene.capture_region.is_some());
            if has_region && ui.small_button("Alanı sıfırla").clicked() {
                if let Some(scene) = state.settings.scenes.get_mut(state.settings.active_scene) {
                    scene.capture_region = None;
                }
            }
        });
    });
    if state.capture_region_selecting {
        ui.small("Önizlemede sürükleyerek kayıt alanını seç.");
    }
    if let Some(display) = state.selected_display() {
        ui.label(format!(
            "{}  |  {} x {}",
            display.name, display.width, display.height
        ));
    }
    ui.add_space(8.0);
    let available = ui.available_size();
    let canvas_size = fit_size([available.x, available.y], [16.0, 9.0]);
    let (canvas, _) = ui.allocate_exact_size(
        egui::vec2(canvas_size[0], canvas_size[1]),
        if state.capture_region_selecting {
            egui::Sense::drag()
        } else {
            egui::Sense::hover()
        },
    );
    ui.painter()
        .rect_filled(canvas, 4.0, egui::Color32::from_rgb(12, 14, 17));
    if let Some(texture) = &state.preview_texture {
        let image = fit_rect(canvas, texture.size_vec2());
        ui.painter().image(
            texture.id(),
            image,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        let (output_width, output_height) = state.settings.resolution();
        let allow_edit = state.recorder.is_none();
        if state.capture_region_selecting {
            let response = ui.interact(
                image,
                egui::Id::new("capture-region-select"),
                egui::Sense::drag(),
            );
            if response.drag_started()
                && let Some(pointer) = response.interact_pointer_pos()
            {
                state.capture_region_drag_start = Some([
                    ((pointer.x - image.left()) / image.width()).clamp(0.0, 1.0),
                    ((pointer.y - image.top()) / image.height()).clamp(0.0, 1.0),
                ]);
            }
            if response.dragged()
                && let (Some(start), Some(pointer)) = (
                    state.capture_region_drag_start,
                    response.interact_pointer_pos(),
                )
            {
                let end = [
                    ((pointer.x - image.left()) / image.width()).clamp(0.0, 1.0),
                    ((pointer.y - image.top()) / image.height()).clamp(0.0, 1.0),
                ];
                let left = start[0].min(end[0]);
                let top = start[1].min(end[1]);
                let width = (start[0] - end[0]).abs();
                let height = (start[1] - end[1]).abs();
                let selected = egui::Rect::from_min_size(
                    egui::pos2(
                        image.left() + left * image.width(),
                        image.top() + top * image.height(),
                    ),
                    egui::vec2(width * image.width(), height * image.height()),
                );
                ui.painter().rect_filled(
                    selected,
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(40, 150, 255, 64),
                );
                ui.painter().rect_stroke(
                    selected,
                    1.0,
                    egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(65, 175, 255)),
                    egui::StrokeKind::Inside,
                );
            }
            if response.drag_stopped()
                && let (Some(start), Some(pointer)) = (
                    state.capture_region_drag_start,
                    response.interact_pointer_pos(),
                )
            {
                let end = [
                    ((pointer.x - image.left()) / image.width()).clamp(0.0, 1.0),
                    ((pointer.y - image.top()) / image.height()).clamp(0.0, 1.0),
                ];
                let left = start[0].min(end[0]);
                let top = start[1].min(end[1]);
                let region = crate::config::CaptureRegion {
                    x: left,
                    y: top,
                    width: (start[0] - end[0]).abs(),
                    height: (start[1] - end[1]).abs(),
                }
                .clamped();
                if region.width >= 0.02
                    && region.height >= 0.02
                    && let Some(scene) = state.settings.scenes.get_mut(state.settings.active_scene)
                {
                    scene.capture_region = Some(region);
                }
                state.capture_region_drag_start = None;
                state.capture_region_selecting = false;
            }
        }
        if let Some(region) = state
            .settings
            .scenes
            .get(state.settings.active_scene)
            .and_then(|scene| scene.capture_region)
        {
            let region = region.clamped();
            let rect = egui::Rect::from_min_size(
                egui::pos2(
                    image.left() + region.x * image.width(),
                    image.top() + region.y * image.height(),
                ),
                egui::vec2(region.width * image.width(), region.height * image.height()),
            );
            ui.painter().rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(55, 190, 255)),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                rect.left_top() + egui::vec2(5.0, 4.0),
                egui::Align2::LEFT_TOP,
                "KAYIT ALANI",
                egui::FontId::proportional(11.0),
                egui::Color32::from_rgb(55, 190, 255),
            );
        }
        if let Some(scene) = state.settings.scenes.get_mut(state.settings.active_scene) {
            for (index, source) in scene.sources.iter_mut().enumerate() {
                if !source.visible || source.kind == SceneSourceKind::Display {
                    continue;
                }
                let transform = source
                    .transform
                    .unwrap_or_else(|| SourceTransform::for_kind(source.kind))
                    .clamped();
                let source_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        image.left() + transform.x * image.width(),
                        image.top() + transform.y * image.height(),
                    ),
                    egui::vec2(
                        transform.width * image.width(),
                        transform.height * image.height(),
                    ),
                );
                let response = ui.interact(
                    source_rect,
                    egui::Id::new(("scene-source", index)),
                    if allow_edit && !state.capture_region_selecting {
                        egui::Sense::drag()
                    } else {
                        egui::Sense::hover()
                    },
                );
                if response.dragged() {
                    let delta = ui.input(|input| input.pointer.delta());
                    let mut moved = transform;
                    moved.x += delta.x / image.width().max(1.0);
                    moved.y += delta.y / image.height().max(1.0);
                    source.transform = Some(moved.clamped());
                }
                let active = response.hovered() || response.dragged();
                let stroke = if active {
                    egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(94, 190, 255))
                } else {
                    egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(242, 190, 74))
                };
                ui.painter()
                    .rect_filled(source_rect, 3.0, egui::Color32::from_black_alpha(50));
                ui.painter()
                    .rect_stroke(source_rect, 3.0, stroke, egui::StrokeKind::Inside);
                ui.painter().text(
                    source_rect.left_top() + egui::vec2(6.0, 5.0),
                    egui::Align2::LEFT_TOP,
                    &source.name,
                    egui::FontId::proportional(13.0),
                    egui::Color32::WHITE,
                );
                if allow_edit {
                    let handle = egui::Rect::from_min_size(
                        source_rect.right_bottom() - egui::vec2(14.0, 14.0),
                        egui::vec2(14.0, 14.0),
                    );
                    let resize = ui.interact(
                        handle,
                        egui::Id::new(("scene-source-resize", index)),
                        egui::Sense::drag(),
                    );
                    ui.painter()
                        .rect_filled(handle, 2.0, egui::Color32::from_rgb(94, 190, 255));
                    if resize.dragged() {
                        let delta = ui.input(|input| input.pointer.delta());
                        let mut resized = source
                            .transform
                            .unwrap_or_else(|| SourceTransform::for_kind(source.kind));
                        resized.width += delta.x / image.width().max(1.0);
                        resized.height += delta.y / image.height().max(1.0);
                        source.transform = Some(resized.clamped());
                    }
                }
            }
        }
        if allow_edit {
            ui.painter().text(
                image.left_bottom() + egui::vec2(8.0, -8.0),
                egui::Align2::LEFT_BOTTOM,
                format!("Sürükle · boyutlandır · çıktı {output_width}×{output_height}"),
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
        }
    } else {
        ui.painter().text(
            canvas.center(),
            egui::Align2::CENTER_CENTER,
            "Ekran yakalama bekleniyor...",
            egui::FontId::proportional(16.0),
            egui::Color32::from_gray(155),
        );
    }
}

fn fit_size(available: [f32; 2], aspect: [f32; 2]) -> [f32; 2] {
    let available_aspect = available[0] / available[1].max(1.0);
    let target_aspect = aspect[0] / aspect[1];
    if available_aspect > target_aspect {
        [available[1] * target_aspect, available[1]]
    } else {
        [available[0], available[0] / target_aspect]
    }
}

fn fit_rect(bounds: egui::Rect, image_size: egui::Vec2) -> egui::Rect {
    let size = fit_size(
        [bounds.width(), bounds.height()],
        [image_size.x.max(1.0), image_size.y.max(1.0)],
    );
    egui::Rect::from_center_size(bounds.center(), egui::vec2(size[0], size[1]))
}
