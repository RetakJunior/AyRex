use std::{thread, time::Duration};

use crate::{
    capture::discover_displays,
    config::{
        CaptureRegion, Container, DEFAULT_BITRATE_KBPS, RecordingSettings, Scene, SceneSource,
    },
    recording::Recorder,
    state::AppState,
    system::{discover_encoders, locate_ffmpeg},
    ui,
};

pub struct AyRexApp {
    state: AppState,
    global_hotkeys: Option<crate::hotkeys::GlobalHotkeys>,
}

impl AyRexApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(eframe::egui::Visuals::dark());
        let settings = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, "recording_settings"));
        let global_hotkeys = crate::hotkeys::GlobalHotkeys::start();
        let mut state = AppState::new(settings);
        let global_hotkeys = match global_hotkeys {
            Ok(listener) => Some(listener),
            Err(error) => {
                if state.notice.is_none() {
                    state.notice = Some(format!(
                        "Global kısayollar kullanılamıyor; AyRex odaktayken çalışır: {error}"
                    ));
                }
                None
            }
        };
        Self {
            state,
            global_hotkeys,
        }
    }
}

impl eframe::App for AyRexApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        let action = if let Some(hotkeys) = &self.global_hotkeys {
            hotkeys.try_next()
        } else {
            ctx.input(|input| {
                crate::hotkeys::detect(
                    input.modifiers.ctrl,
                    input.modifiers.shift,
                    input.key_pressed(eframe::egui::Key::R),
                    input.key_pressed(eframe::egui::Key::S),
                )
            })
        };
        match action {
            Some(crate::hotkeys::HotkeyAction::StartRecording) if self.state.recorder.is_none() => {
                self.state.start_recording()
            }
            Some(crate::hotkeys::HotkeyAction::StopRecording) if self.state.recorder.is_some() => {
                self.state.stop_recording()
            }
            _ => {}
        }
        self.state.poll_recording();
        ui::update_preview(&mut self.state, ctx);
        ctx.request_repaint_after(Duration::from_millis(120));
        ui::show_dashboard(ctx, &mut self.state);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let mut settings = self.state.settings.clone();
        settings.stream_url =
            crate::streaming::persistable_server(settings.stream_platform, &settings.stream_url);
        eframe::set_value(storage, "recording_settings", &settings);
    }
}

impl Drop for AyRexApp {
    fn drop(&mut self) {
        self.state.stop_recording();
    }
}

pub fn run_smoke_record(seconds: u64) -> Result<(), String> {
    run_smoke_record_inner(seconds, None, false)
}

pub fn run_smoke_region(seconds: u64) -> Result<(), String> {
    run_smoke_record_inner(
        seconds,
        Some(CaptureRegion {
            x: 0.1,
            y: 0.1,
            width: 0.8,
            height: 0.8,
        }),
        false,
    )
}

pub fn run_smoke_system_audio(seconds: u64) -> Result<(), String> {
    run_smoke_record_inner(seconds, None, true)
}

fn run_smoke_record_inner(
    seconds: u64,
    capture_region: Option<CaptureRegion>,
    system_audio_enabled: bool,
) -> Result<(), String> {
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
    let mut settings = RecordingSettings {
        resolution_index: 1,
        bitrate_kbps: DEFAULT_BITRATE_KBPS,
        encoder: "libx264".to_owned(),
        container: Container::Mkv,
        output_dir: std::env::temp_dir().join(if system_audio_enabled {
            "AyRexSystemAudioSmokeTest"
        } else {
            "AyRexSmokeTest"
        }),
        system_audio_enabled,
        ..RecordingSettings::default()
    };
    if let Some(capture_region) = capture_region {
        settings.scenes = vec![Scene {
            name: "Özel Alan Smoke Test".to_owned(),
            display_id: display.id,
            sources: vec![SceneSource::display()],
            capture_region: Some(capture_region),
        }];
    }
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

pub fn run_smoke_replay(seconds: u64) -> Result<(), String> {
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
        return Err("The replay smoke run requires the FFmpeg libx264 encoder.".to_owned());
    }
    let seconds = seconds.clamp(5, 600) as u32;
    let settings = RecordingSettings {
        resolution_index: 1,
        bitrate_kbps: DEFAULT_BITRATE_KBPS,
        encoder: "libx264".to_owned(),
        container: Container::Mkv,
        output_dir: std::env::temp_dir().join("AyRexReplaySmokeTest"),
        replay_buffer_enabled: true,
        replay_buffer_seconds: seconds,
        ..RecordingSettings::default()
    };
    let mut config = settings.for_display(ffmpeg, display);
    config.replay_buffer_seconds = seconds;
    let recorder = Recorder::start(&config)?;
    thread::sleep(Duration::from_secs(u64::from(seconds) + 3));
    let replay = recorder.save_replay(seconds)?;
    let recording = recorder.stop()?;
    println!(
        "Recording: {} ({} bytes). Replay clip: {} ({} bytes).",
        recording.output_path.display(),
        recording.file_size,
        replay.display(),
        std::fs::metadata(&replay)
            .map(|metadata| metadata.len())
            .unwrap_or(0)
    );
    Ok(())
}

pub fn check_global_hotkeys() -> Result<(), String> {
    let listener = crate::hotkeys::GlobalHotkeys::start()?;
    drop(listener);
    println!("Ctrl+Shift+R and Ctrl+Shift+S registered and released successfully.");
    Ok(())
}
