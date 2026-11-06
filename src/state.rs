use std::{path::Path, path::PathBuf, time::Duration};

use eframe::egui::TextureHandle;

use crate::{
    audio::{discover_audio_devices, discover_video_devices},
    capture::{Display, PreviewCapture, WindowTarget, discover_displays, discover_windows},
    config::{RecordingConfig, RecordingSettings, SceneSource, SceneSourceKind},
    performance::PerformanceSampler,
    recording::{Recorder, RecordingSummary, encoder_label, is_hardware_encoder},
    system::{discover_encoders, locate_ffmpeg},
};

pub struct AppState {
    pub displays: Vec<Display>,
    pub selected_display_id: Option<u32>,
    pub ffmpeg: Option<PathBuf>,
    pub encoders: Vec<String>,
    pub audio_devices: Vec<String>,
    pub camera_devices: Vec<String>,
    pub windows: Vec<WindowTarget>,
    pub settings: RecordingSettings,
    pub stream_key: String,
    pub custom_stream_url: String,
    pub recorder: Option<Recorder>,
    pub preview: Option<PreviewCapture>,
    pub preview_texture: Option<TextureHandle>,
    pub notice: Option<String>,
    pub last_summary: Option<RecordingSummary>,
    pub performance: PerformanceSampler,
    pub capture_region_selecting: bool,
    pub capture_region_drag_start: Option<[f32; 2]>,
}

impl AppState {
    pub fn new(saved_settings: Option<RecordingSettings>) -> Self {
        let (displays, mut notice) = match discover_displays() {
            Ok(displays) => (displays, None),
            Err(error) => (Vec::new(), Some(error)),
        };
        let (windows, window_error) = match discover_windows() {
            Ok(windows) => (windows, None),
            Err(error) => (Vec::new(), Some(error)),
        };
        if notice.is_none() {
            notice = window_error;
        }
        let mut selected_display_id = displays
            .iter()
            .find(|display| display.primary)
            .or_else(|| displays.first())
            .map(|display| display.id);
        let ffmpeg = locate_ffmpeg();
        let mut settings = saved_settings.unwrap_or_default();
        let (stream_key, custom_stream_url) = detach_stream_credentials(&mut settings);
        if settings.scenes.is_empty() {
            settings.scenes = displays
                .iter()
                .map(|display| crate::config::Scene {
                    name: if display.primary {
                        "Ana Sahne".to_owned()
                    } else {
                        display.name.clone()
                    },
                    display_id: display.id,
                    sources: vec![SceneSource::display()],
                    capture_region: None,
                })
                .collect();
        }
        for (index, scene) in settings.scenes.iter_mut().enumerate() {
            if scene.sources.is_empty() {
                scene.sources.push(SceneSource::display());
                if index == settings.active_scene {
                    if let Some(camera) = &settings.camera_device {
                        scene.sources.push(SceneSource {
                            name: camera.clone(),
                            kind: SceneSourceKind::Camera,
                            value: camera.clone(),
                            visible: true,
                            transform: None,
                        });
                    }
                    if let Some(image) = &settings.image_source {
                        scene.sources.push(SceneSource {
                            name: image
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned(),
                            kind: SceneSourceKind::Image,
                            value: image.to_string_lossy().into_owned(),
                            visible: true,
                            transform: None,
                        });
                    }
                }
            }
        }
        settings.active_scene = settings
            .active_scene
            .min(settings.scenes.len().saturating_sub(1));
        settings.program_scene = settings
            .program_scene
            .min(settings.scenes.len().saturating_sub(1));
        if let Some(scene_display) = settings
            .scenes
            .get(settings.active_scene)
            .map(|scene| scene.display_id)
            && displays.iter().any(|display| display.id == scene_display)
        {
            selected_display_id = Some(scene_display);
        }
        let mut encoders = Vec::new();
        let mut audio_devices = Vec::new();
        let mut camera_devices = Vec::new();

        if let Some(path) = &ffmpeg {
            match discover_encoders(path) {
                Ok(found) => encoders = found,
                Err(error) => notice = Some(error),
            }
            match discover_audio_devices(path) {
                Ok(found) => audio_devices = found,
                Err(error) => notice = Some(error),
            }
            match discover_video_devices(path) {
                Ok(found) => camera_devices = found,
                Err(error) => notice = Some(error),
            }
        } else {
            notice = Some(if cfg!(windows) {
                "FFmpeg was not found. Install FFmpeg and add ffmpeg.exe to PATH.".to_owned()
            } else {
                "FFmpeg was not found. Install FFmpeg and add it to PATH.".to_owned()
            });
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
        if settings
            .camera_device
            .as_ref()
            .is_some_and(|device| !camera_devices.contains(device))
        {
            settings.camera_device = None;
        }
        let preview = selected_display_id.map(PreviewCapture::start);
        Self {
            displays,
            selected_display_id,
            ffmpeg,
            encoders,
            audio_devices,
            camera_devices,
            windows,
            settings,
            stream_key,
            custom_stream_url,
            recorder: None,
            preview,
            preview_texture: None,
            notice,
            last_summary: None,
            performance: PerformanceSampler::default(),
            capture_region_selecting: false,
            capture_region_drag_start: None,
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
        let scene_index = self.settings.recording_scene_index();
        let scene = self.settings.scenes.get(scene_index);
        let display = scene
            .and_then(|scene| {
                self.displays
                    .iter()
                    .find(|display| display.id == scene.display_id)
            })
            .or_else(|| self.selected_display())
            .cloned()
            .ok_or_else(|| "No monitor is available for capture.".to_owned())?;
        if !self
            .encoders
            .iter()
            .any(|encoder| encoder == &self.settings.encoder)
        {
            return Err("The selected video encoder is not available in FFmpeg.".to_owned());
        }
        let stream_url = if self.settings.stream_enabled {
            if self.settings.stream_platform != crate::streaming::StreamPlatform::Custom
                && self.settings.encoder != "libx264"
                && !self.settings.encoder.starts_with("h264_")
            {
                return Err("Kick ve Twitch yayınında H.264 kodlayıcı seç.".to_owned());
            }
            if self.settings.bitrate_kbps > self.settings.stream_platform.max_bitrate_kbps() {
                return Err(format!(
                    "{} için yayın bit hızı en fazla {} kbps olmalı.",
                    self.settings.stream_platform.label(),
                    self.settings.stream_platform.max_bitrate_kbps()
                ));
            }
            Some(crate::streaming::build_stream_url(
                self.settings.stream_platform,
                &self.settings.stream_url,
                &self.stream_key,
                &self.custom_stream_url,
            )?)
        } else {
            None
        };
        let mut config = self.settings.for_scene(ffmpeg, display, scene_index);
        config.stream_url = stream_url;
        config.replay_buffer_seconds = if self.settings.replay_buffer_enabled {
            self.settings.replay_buffer_seconds.clamp(5, 600)
        } else {
            0
        };
        if config
            .camera_device
            .as_ref()
            .is_some_and(|camera| !self.camera_devices.contains(camera))
        {
            return Err("Seçili kamera artık kullanılamıyor.".to_owned());
        }
        if config
            .window_title
            .as_ref()
            .is_some_and(|title| !self.windows.iter().any(|window| &window.title == title))
        {
            return Err("Sahnede seçilen pencere artık bulunamıyor.".to_owned());
        }
        if config
            .image_source
            .as_ref()
            .is_some_and(|path| !path.is_file())
        {
            return Err("Sahnedeki görsel dosyası bulunamadı.".to_owned());
        }
        if config
            .video_source
            .as_ref()
            .is_some_and(|path| !path.is_file())
        {
            return Err("Sahnedeki video dosyası bulunamadı.".to_owned());
        }
        for source in config.scene_sources.iter().filter(|source| source.visible) {
            match source.kind {
                SceneSourceKind::Camera
                    if !self
                        .camera_devices
                        .iter()
                        .any(|camera| camera == &source.value) =>
                {
                    return Err(format!("Sahnedeki kamera kullanılamıyor: {}", source.name));
                }
                SceneSourceKind::Window
                    if !self
                        .windows
                        .iter()
                        .any(|window| window.title == source.value) =>
                {
                    return Err(format!("Sahnedeki pencere bulunamadı: {}", source.name));
                }
                SceneSourceKind::Image if !std::path::Path::new(&source.value).is_file() => {
                    return Err(format!(
                        "Sahnedeki görsel dosyası bulunamadı: {}",
                        source.name
                    ));
                }
                SceneSourceKind::Video if !std::path::Path::new(&source.value).is_file() => {
                    return Err(format!(
                        "Sahnedeki video dosyası bulunamadı: {}",
                        source.name
                    ));
                }
                _ => {}
            }
        }
        Ok(config)
    }

    pub fn select_scene(&mut self, index: usize) {
        if let Some(scene) = self.settings.scenes.get(index).cloned() {
            self.settings.active_scene = index;
            self.selected_display_id = Some(scene.display_id);
            if let Some(preview) = &self.preview {
                preview.select_monitor(scene.display_id);
            }
        }
    }

    pub fn select_stream_platform(&mut self, platform: crate::streaming::StreamPlatform) {
        if self.settings.stream_platform == platform {
            return;
        }
        self.settings.stream_platform = platform;
        self.settings.stream_url = platform.default_server().to_owned();
        self.stream_key.clear();
        self.custom_stream_url.clear();
    }

    pub fn add_window_source(&mut self, id: u32) {
        let Some(window) = self.windows.iter().find(|window| window.id == id).cloned() else {
            return;
        };
        if let Some(scene) = self.settings.scenes.get_mut(self.settings.active_scene) {
            if !scene.sources.iter().any(|source| {
                source.kind == SceneSourceKind::Window && source.value == window.title
            }) {
                scene.sources.push(SceneSource {
                    name: format!("{} — {}", window.app, window.title),
                    kind: SceneSourceKind::Window,
                    value: window.title,
                    visible: true,
                    transform: None,
                });
            }
        }
    }

    pub fn add_scene(&mut self) {
        let display_id = self
            .selected_display_id
            .or_else(|| self.displays.first().map(|d| d.id));
        if let Some(display_id) = display_id {
            self.settings.scenes.push(crate::config::Scene {
                name: format!("Sahne {}", self.settings.scenes.len() + 1),
                display_id,
                sources: vec![SceneSource::display()],
                capture_region: None,
            });
            self.settings.active_scene = self.settings.scenes.len() - 1;
        }
    }

    pub fn duplicate_scene(&mut self) {
        if let Some(mut scene) = self
            .settings
            .scenes
            .get(self.settings.active_scene)
            .cloned()
        {
            scene.name.push_str(" Kopya");
            self.settings.scenes.push(scene);
            self.settings.active_scene = self.settings.scenes.len() - 1;
        }
    }

    pub fn delete_scene(&mut self) {
        if self.settings.scenes.len() > 1 && self.settings.active_scene < self.settings.scenes.len()
        {
            let removed = self.settings.active_scene;
            self.settings.scenes.remove(removed);
            if self.settings.program_scene == removed {
                self.settings.program_scene = removed.min(self.settings.scenes.len() - 1);
            } else if self.settings.program_scene > removed {
                self.settings.program_scene -= 1;
            }
            self.settings.active_scene = self
                .settings
                .active_scene
                .min(self.settings.scenes.len() - 1);
            let display_id = self.settings.scenes[self.settings.active_scene].display_id;
            self.select_display(display_id);
        }
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

    pub fn save_replay(&mut self) {
        let seconds = self.settings.replay_buffer_seconds;
        let Some(recorder) = &self.recorder else {
            self.notice = Some("Replay Buffer kaydı henüz çalışmıyor.".to_owned());
            return;
        };
        match recorder.save_replay(seconds) {
            Ok(path) => {
                self.notice = Some(format!("Tekrar klibi kaydedildi: {}", path.display()));
            }
            Err(error) => self.notice = Some(error),
        }
    }

    pub fn poll_recording(&mut self) {
        self.performance.refresh();
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
        if let Some(scene) = self.settings.scenes.get_mut(self.settings.active_scene) {
            scene.display_id = id;
            for source in &mut scene.sources {
                if source.kind == SceneSourceKind::Window {
                    source.visible = false;
                }
            }
        }
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

    pub fn save_profile(&mut self, path: &std::path::Path) {
        match crate::profiles::save(path, &self.settings) {
            Ok(()) => self.notice = Some("Profil kaydedildi.".to_owned()),
            Err(error) => self.notice = Some(error),
        }
    }

    pub fn load_profile(&mut self, path: &std::path::Path) {
        match crate::profiles::load(path) {
            Ok(mut settings) => {
                let (stream_key, custom_stream_url) = detach_stream_credentials(&mut settings);
                if settings.scenes.is_empty() {
                    settings.scenes = self
                        .displays
                        .iter()
                        .map(|display| crate::config::Scene {
                            name: display.name.clone(),
                            display_id: display.id,
                            sources: vec![SceneSource::display()],
                            capture_region: None,
                        })
                        .collect();
                }
                settings.active_scene = settings
                    .active_scene
                    .min(settings.scenes.len().saturating_sub(1));
                settings.program_scene = settings
                    .program_scene
                    .min(settings.scenes.len().saturating_sub(1));
                self.settings = settings;
                self.stream_key = stream_key;
                self.custom_stream_url = custom_stream_url;
                if let Some(display_id) = self
                    .settings
                    .scenes
                    .get(self.settings.active_scene)
                    .map(|scene| scene.display_id)
                {
                    self.select_display(display_id);
                }
                self.notice = Some("Profil yüklendi.".to_owned());
            }
            Err(error) => self.notice = Some(error),
        }
    }

    pub fn open_output_folder(&mut self) {
        let file_manager = if cfg!(windows) {
            "explorer.exe"
        } else if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        if let Err(error) = std::process::Command::new(file_manager)
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
                    if recorder.config().stream_url.is_some() {
                        if recorder.stream_failed() {
                            "Recording (stream failed)"
                        } else {
                            "Recording (stream configured)"
                        }
                    } else {
                        "Recording"
                    }
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

    pub fn audio_status(&self) -> String {
        let microphone = self
            .recorder
            .as_ref()
            .and_then(Recorder::audio_device)
            .or_else(|| {
                self.last_summary
                    .as_ref()
                    .and_then(|summary| summary.audio_device.as_deref())
            })
            .or(self.settings.audio_device.as_deref());
        let system_audio_enabled = self
            .recorder
            .as_ref()
            .map(|recorder| recorder.config().system_audio_enabled)
            .or_else(|| {
                self.last_summary
                    .as_ref()
                    .map(|summary| summary.system_audio_enabled)
            })
            .unwrap_or(self.settings.system_audio_enabled);
        match (microphone, system_audio_enabled) {
            (Some(microphone), true) => format!("{microphone} + Masaüstü sesi"),
            (Some(microphone), false) => microphone.to_owned(),
            (None, true) => "Masaüstü sesi".to_owned(),
            (None, false) => "Kapalı".to_owned(),
        }
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

fn detach_stream_credentials(settings: &mut RecordingSettings) -> (String, String) {
    let saved_url = std::mem::take(&mut settings.stream_url);
    if settings.stream_platform == crate::streaming::StreamPlatform::Custom {
        let custom_url = if saved_url == crate::streaming::StreamPlatform::Kick.default_server() {
            String::new()
        } else {
            saved_url
        };
        settings.stream_url = crate::streaming::StreamPlatform::Kick
            .default_server()
            .to_owned();
        return (String::new(), custom_url);
    }

    if saved_url.starts_with("srt://") {
        settings.stream_platform = crate::streaming::StreamPlatform::Custom;
        settings.stream_url = crate::streaming::StreamPlatform::Kick
            .default_server()
            .to_owned();
        return (String::new(), saved_url);
    }
    if settings.stream_platform == crate::streaming::StreamPlatform::Kick
        && saved_url.to_ascii_lowercase().contains("twitch")
    {
        settings.stream_platform = crate::streaming::StreamPlatform::Twitch;
    }
    let (server, key) = crate::streaming::split_legacy_stream_url(&saved_url);
    if key.is_none() && !saved_url.is_empty() && !is_rtmp_app_server(&server) {
        settings.stream_platform = crate::streaming::StreamPlatform::Custom;
        settings.stream_url = crate::streaming::StreamPlatform::Kick
            .default_server()
            .to_owned();
        return (String::new(), saved_url);
    }
    settings.stream_url = if server.is_empty() {
        settings.stream_platform.default_server().to_owned()
    } else {
        server
    };
    (key.unwrap_or_default(), String::new())
}

fn is_rtmp_app_server(url: &str) -> bool {
    let Some((_, remainder)) = url.split_once("://") else {
        return false;
    };
    let Some((authority, path)) = remainder.split_once('/') else {
        return false;
    };
    !authority.is_empty() && path.trim_end_matches('/') == "app"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_stream_keys_are_removed_from_persisted_settings() {
        let mut settings = RecordingSettings {
            stream_url: "rtmps://ingest.example/app/old-secret-key".to_owned(),
            ..RecordingSettings::default()
        };
        let (key, custom_url) = detach_stream_credentials(&mut settings);
        let persisted = serde_json::to_string(&settings).unwrap();
        assert_eq!(key, "old-secret-key");
        assert!(!persisted.contains("old-secret-key"));
        assert!(custom_url.is_empty());
        assert_eq!(settings.stream_url, "rtmps://ingest.example/app");
    }
}
