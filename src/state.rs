use std::{path::Path, path::PathBuf, time::Duration};

use eframe::egui::TextureHandle;

use crate::{
    audio::discover_audio_devices,
    capture::{Display, PreviewCapture, discover_displays},
    config::{RecordingConfig, RecordingSettings},
    recording::{Recorder, RecordingSummary, encoder_label, is_hardware_encoder},
    system::{discover_encoders, locate_ffmpeg},
};

pub struct AppState {
    pub displays: Vec<Display>,
    pub selected_display_id: Option<u32>,
    pub ffmpeg: Option<PathBuf>,
    pub encoders: Vec<String>,
    pub audio_devices: Vec<String>,
    pub settings: RecordingSettings,
    pub recorder: Option<Recorder>,
    pub preview: Option<PreviewCapture>,
    pub preview_texture: Option<TextureHandle>,
    pub notice: Option<String>,
    pub last_summary: Option<RecordingSummary>,
}

impl AppState {
    pub fn new(saved_settings: Option<RecordingSettings>) -> Self {
        let (displays, mut notice) = match discover_displays() {
            Ok(displays) => (displays, None),
            Err(error) => (Vec::new(), Some(error)),
        };
        let selected_display_id = displays
            .iter()
            .find(|display| display.primary)
            .or_else(|| displays.first())
            .map(|display| display.id);
        let ffmpeg = locate_ffmpeg();
        let mut settings = saved_settings.unwrap_or_default();
        let mut encoders = Vec::new();
        let mut audio_devices = Vec::new();

        if let Some(path) = &ffmpeg {
            match discover_encoders(path) {
                Ok(found) => encoders = found,
                Err(error) => notice = Some(error),
            }
            match discover_audio_devices(path) {
                Ok(found) => audio_devices = found,
                Err(error) => notice = Some(error),
            }
        } else {
            notice =
                Some("FFmpeg was not found. Install FFmpeg and add ffmpeg.exe to PATH.".to_owned());
        }

        if !encoders.iter().any(|encoder| encoder == &settings.encoder) {
            if let Some(encoder) = encoders.first() {
                settings.encoder = encoder.clone();
            }
        }
        if settings
            .audio_device
            .as_ref()
            .is_some_and(|device| !audio_devices.contains(device))
        {
            settings.audio_device = None;
            notice =
                Some("The saved audio device is unavailable and has been disabled.".to_owned());
        }
        let preview = selected_display_id.map(PreviewCapture::start);

        Self {
            displays,
            selected_display_id,
            ffmpeg,
            encoders,
            audio_devices,
            settings,
            recorder: None,
            preview,
            preview_texture: None,
            notice,
            last_summary: None,
        }
    }

    pub fn selected_display(&self) -> Option<&Display> {
        self.displays
            .iter()
            .find(|display| Some(display.id) == self.selected_display_id)
    }

    pub fn selected_config(&self) -> Result<RecordingConfig, String> {
        let ffmpeg = self
            .ffmpeg
            .clone()
            .ok_or_else(|| "Install FFmpeg and add it to PATH to record.".to_owned())?;
        let display = self
            .selected_display()
            .cloned()
            .ok_or_else(|| "No monitor is available for capture.".to_owned())?;
        if !self
            .encoders
            .iter()
            .any(|encoder| encoder == &self.settings.encoder)
        {
            return Err("The selected video encoder is not available in FFmpeg.".to_owned());
        }
        Ok(self.settings.for_display(ffmpeg, display))
    }

    pub fn start_recording(&mut self) {
        match self
            .selected_config()
            .and_then(|config| Recorder::start(&config))
        {
            Ok(recorder) => {
                self.notice = None;
                self.last_summary = None;
                self.recorder = Some(recorder);
            }
            Err(error) => self.notice = Some(error),
        }
    }

    pub fn stop_recording(&mut self) {
        let Some(recorder) = self.recorder.take() else {
            return;
        };
        match recorder.stop() {
            Ok(summary) => {
                self.notice = Some(saved_notice(&summary));
                self.last_summary = Some(summary);
            }
            Err(error) => self.notice = Some(error),
        }
    }

    pub fn poll_recording(&mut self) {
        let result = self.recorder.as_mut().map(Recorder::poll_exit);
        match result {
            Some(Ok(Some(status))) => {
                let Some(mut recorder) = self.recorder.take() else {
                    return;
                };
                if let Err(error) = recorder.finish_readers() {
                    self.notice = Some(error);
                    return;
                }

                let frames = recorder
                    .progress()
                    .map(|progress| progress.frames)
                    .unwrap_or_default();
                if !status.success()
                    && frames == 0
                    && is_hardware_encoder(recorder.encoder())
                    && self.encoders.iter().any(|encoder| encoder == "libx264")
                {
                    self.start_software_fallback(recorder);
                    return;
                }

                match recorder.summary(status) {
                    Ok(summary) => {
                        self.notice = Some(saved_notice(&summary));
                        self.last_summary = Some(summary);
                    }
                    Err(error) => self.notice = Some(error),
                }
            }
            Some(Err(error)) => self.notice = Some(error),
            _ => {}
        }
    }

    fn start_software_fallback(&mut self, recorder: Recorder) {
        let hardware_error = recorder.stderr_message();
        let mut config = recorder.config().clone();
        config.encoder = "libx264".to_owned();
        match Recorder::start(&config) {
            Ok(mut fallback) => {
                let warning = if hardware_error.is_empty() {
                    "Hardware encoder unavailable; switched to H.264 software.".to_owned()
                } else {
                    format!(
                        "Hardware encoder unavailable; switched to H.264 software. {}",
                        hardware_error.lines().last().unwrap_or_default()
                    )
                };
                fallback.set_warning(warning.clone());
                self.notice = Some(warning);
                self.recorder = Some(fallback);
            }
            Err(error) => {
                self.notice = Some(format!(
                    "Hardware recording failed: {hardware_error}\nSoftware fallback failed: {error}"
                ));
            }
        }
    }

    pub fn select_display(&mut self, id: u32) {
        self.selected_display_id = Some(id);
        self.preview_texture = None;
        if let Some(preview) = &self.preview {
            preview.select_monitor(id);
        } else {
            self.preview = Some(PreviewCapture::start(id));
        }
    }

    pub fn choose_output_dir(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_directory(&self.settings.output_dir)
            .pick_folder()
        {
            self.settings.output_dir = path;
        }
    }

    pub fn open_output_folder(&mut self) {
        if let Err(error) = std::process::Command::new("explorer.exe")
            .arg(&self.settings.output_dir)
            .spawn()
        {
            self.notice = Some(format!("Could not open the recording folder: {error}"));
        }
    }

    pub fn recording_duration(&self) -> Duration {
        self.recorder
            .as_ref()
            .map(Recorder::elapsed)
            .unwrap_or_default()
    }

    pub fn recording_status(&self) -> &'static str {
        let Some(recorder) = &self.recorder else {
            return "Ready";
        };
        recorder
            .progress()
            .map(|progress| {
                if progress.frames > 0 {
                    "Recording"
                } else {
                    "Starting"
                }
            })
            .unwrap_or("Status unavailable")
    }

    pub fn recording_size(&self) -> u64 {
        self.recorder
            .as_ref()
            .and_then(|recorder| std::fs::metadata(recorder.output_path()).ok())
            .map(|metadata| metadata.len())
            .or_else(|| self.last_summary.as_ref().map(|summary| summary.file_size))
            .unwrap_or(0)
    }

    pub fn current_fps(&self) -> Option<f32> {
        self.recorder
            .as_ref()
            .and_then(|recorder| recorder.progress().ok()?.fps)
    }

    pub fn dropped_frames(&self) -> Option<u64> {
        self.recorder
            .as_ref()
            .and_then(|recorder| recorder.progress().ok()?.dropped_frames)
    }

    pub fn current_encoder(&self) -> Option<&str> {
        self.recorder.as_ref().map(Recorder::encoder).or_else(|| {
            self.last_summary
                .as_ref()
                .map(|summary| summary.encoder.as_str())
        })
    }

    pub fn current_path(&self) -> Option<&Path> {
        self.recorder
            .as_ref()
            .map(Recorder::output_path)
            .or_else(|| {
                self.last_summary
                    .as_ref()
                    .map(|summary| summary.output_path.as_path())
            })
    }

    pub fn encoder_display_name(&self) -> &str {
        self.current_encoder()
            .map(encoder_label)
            .unwrap_or_else(|| encoder_label(&self.settings.encoder))
    }

    pub fn audio_status(&self) -> &str {
        self.recorder
            .as_ref()
            .and_then(Recorder::audio_device)
            .or_else(|| {
                self.last_summary
                    .as_ref()
                    .and_then(|summary| summary.audio_device.as_deref())
            })
            .unwrap_or("Off")
    }
}

fn saved_notice(summary: &RecordingSummary) -> String {
    match &summary.warning {
        Some(warning) => format!(
            "{warning} Recording saved: {}",
            summary.output_path.display()
        ),
        None => format!("Recording saved: {}", summary.output_path.display()),
    }
}

pub fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        (seconds / 60) % 60,
        seconds % 60
    )
}
