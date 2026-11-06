use eframe::egui::{self, ColorImage, TextureOptions};

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

pub(crate) fn show_preview(ui: &mut egui::Ui, state: &AppState) {
    ui.heading("Screen preview");
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
        egui::Sense::hover(),
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
    } else {
        ui.painter().text(
            canvas.center(),
            egui::Align2::CENTER_CENTER,
            "Waiting for screen capture...",
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
