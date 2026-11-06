use std::time::Duration;

use crate::{
    capture::discover_displays,
    config::{Container, DEFAULT_BITRATE_KBPS, RecordingSettings},
    recording::Recorder,
    state::AppState,
    system::{discover_encoders, locate_ffmpeg},
    ui,
};

pub struct AyRexApp {
    state: AppState,
}

impl AyRexApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(eframe::egui::Visuals::dark());
        let settings = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, "recording_settings"));
        Self {
            state: AppState::new(settings),
        }
    }
}

impl eframe::App for AyRexApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.state.poll_recording();
        ui::update_preview(&mut self.state, ctx);
        ctx.request_repaint_after(Duration::from_millis(120));
        ui::show_dashboard(ctx, &mut self.state);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "recording_settings", &self.state.settings);
    }
}

impl Drop for AyRexApp {
    fn drop(&mut self) {
        self.state.stop_recording();
    }
}

pub fn run_smoke_record(seconds: u64) -> Result<(), String> {
    let displays = discover_displays()?;
    let display = displays
        .iter()
        .find(|display| display.primary)
        .or_else(|| displays.first())
        .cloned()
        .ok_or_else(|| "No monitor is available for capture.".to_owned())?;
    let ffmpeg = locate_ffmpeg().ok_or_else(|| "FFmpeg was not found on PATH.".to_owned())?;
    let encoders = discover_encoders(&ffmpeg)?;
    if !encoders.iter().any(|encoder| encoder == "libx264") {
        return Err("The smoke recording requires the FFmpeg libx264 encoder.".to_owned());
    }
    let settings = RecordingSettings {
        resolution_index: 1,
        bitrate_kbps: DEFAULT_BITRATE_KBPS,
        encoder: "libx264".to_owned(),
        container: Container::Mkv,
        output_dir: std::env::temp_dir().join("AyRexSmokeTest"),
        ..RecordingSettings::default()
    };
    let config = settings.for_display(ffmpeg, display);
    let summary = Recorder::smoke_record(&config, Duration::from_secs(seconds))?;
    println!(
        "Saved {} ({} bytes, {} frames, {}).",
        summary.output_path.display(),
        summary.file_size,
        summary.frames,
        crate::recording::format_duration(summary.elapsed)
    );
    Ok(())
}
