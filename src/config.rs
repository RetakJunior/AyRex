use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::capture::Display;
use crate::streaming::StreamPlatform;

pub const DEFAULT_BITRATE_KBPS: u32 = 6_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Container {
    Mkv,
    Mp4,
}

impl Container {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Mkv => "mkv",
            Self::Mp4 => "mp4",
        }
    }

    pub fn muxer(self) -> &'static str {
        match self {
            Self::Mkv => "matroska",
            Self::Mp4 => "mp4",
        }
    }
}

#[derive(Clone, Debug)]
pub struct RecordingConfig {
    pub ffmpeg: PathBuf,
    pub display: Display,
    pub capture_region: Option<CaptureRegion>,
    pub output_dir: PathBuf,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub encoder: String,
    pub container: Container,
    pub audio_device: Option<String>,
    pub system_audio_enabled: bool,
    pub system_audio_volume: f32,
    pub use_desktop_duplication: bool,
    pub scene_sources: Vec<SceneSource>,
    pub camera_device: Option<String>,
    pub image_source: Option<PathBuf>,
    pub window_title: Option<String>,
    pub video_source: Option<PathBuf>,
    pub text_overlay: Option<String>,
    pub camera_transform: Option<SourceTransform>,
    pub image_transform: Option<SourceTransform>,
    pub video_transform: Option<SourceTransform>,
    pub text_transform: Option<SourceTransform>,
    pub replay_buffer_seconds: u32,
    pub noise_suppression: bool,
    pub noise_gate: bool,
    pub compressor: bool,
    pub limiter: bool,
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub sharpen: f32,
    pub camera_chroma_key: bool,
    pub audio_volume: f32,
    pub stream_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
    pub display_id: u32,
    #[serde(default)]
    pub sources: Vec<SceneSource>,
    #[serde(default)]
    pub capture_region: Option<CaptureRegion>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CaptureRegion {
    /// Normalized offsets and size within the selected monitor.
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Default for CaptureRegion {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        }
    }
}

impl CaptureRegion {
    pub fn clamped(mut self) -> Self {
        self.width = self.width.clamp(0.01, 1.0);
        self.height = self.height.clamp(0.01, 1.0);
        self.x = self.x.clamp(0.0, 1.0 - self.width);
        self.y = self.y.clamp(0.0, 1.0 - self.height);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneSourceKind {
    Display,
    Camera,
    Image,
    Window,
    Video,
    Text,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneSource {
    pub name: String,
    pub kind: SceneSourceKind,
    #[serde(default)]
    pub value: String,
    #[serde(default = "default_source_visible")]
    pub visible: bool,
    #[serde(default)]
    pub transform: Option<SourceTransform>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SourceTransform {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub crop_left: f32,
    pub crop_top: f32,
    pub crop_right: f32,
    pub crop_bottom: f32,
}

impl Default for SourceTransform {
    fn default() -> Self {
        Self {
            x: 0.02,
            y: 0.02,
            width: 0.25,
            height: 0.25,
            crop_left: 0.0,
            crop_top: 0.0,
            crop_right: 0.0,
            crop_bottom: 0.0,
        }
    }
}

impl SourceTransform {
    pub fn for_kind(kind: SceneSourceKind) -> Self {
        let mut transform = Self::default();
        match kind {
            SceneSourceKind::Camera | SceneSourceKind::Video => {
                transform.x = 0.72;
                transform.y = 0.68;
                transform.width = 0.26;
                transform.height = 0.27;
            }
            SceneSourceKind::Image => {
                transform.x = 0.02;
                transform.y = 0.68;
                transform.width = 0.26;
                transform.height = 0.27;
            }
            SceneSourceKind::Text => {
                transform.x = 0.02;
                transform.y = 0.03;
                transform.width = 0.50;
                transform.height = 0.14;
            }
            SceneSourceKind::Display | SceneSourceKind::Window => {}
        }
        transform
    }

    pub fn clamped(mut self) -> Self {
        self.width = self.width.clamp(0.02, 1.0);
        self.height = self.height.clamp(0.02, 1.0);
        self.x = self.x.clamp(0.0, 1.0 - self.width);
        self.y = self.y.clamp(0.0, 1.0 - self.height);
        self.crop_left = self.crop_left.clamp(0.0, 0.8);
        self.crop_top = self.crop_top.clamp(0.0, 0.8);
        self.crop_right = self.crop_right.clamp(0.0, 0.8 - self.crop_left);
        self.crop_bottom = self.crop_bottom.clamp(0.0, 0.8 - self.crop_top);
        self
    }
}

fn default_source_visible() -> bool {
    true
}

impl SceneSource {
    pub fn display() -> Self {
        Self {
            name: "Ekran Yakalama".to_owned(),
            kind: SceneSourceKind::Display,
            value: String::new(),
            visible: true,
            transform: None,
        }
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::Mkv
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RecordingSettings {
    pub resolution_index: usize,
    pub custom_width: u32,
    pub custom_height: u32,
    pub fps: u32,
    pub custom_fps: u32,
    pub bitrate_kbps: u32,
    pub encoder: String,
    pub container: Container,
    pub audio_device: Option<String>,
    pub system_audio_enabled: bool,
    pub system_audio_volume: f32,
    pub use_desktop_duplication: bool,
    pub camera_device: Option<String>,
    pub image_source: Option<PathBuf>,
    pub noise_suppression: bool,
    pub noise_gate: bool,
    pub compressor: bool,
    pub limiter: bool,
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub sharpen: f32,
    pub camera_chroma_key: bool,
    pub audio_volume: f32,
    pub audio_muted: bool,
    pub output_dir: PathBuf,
    pub scenes: Vec<Scene>,
    pub active_scene: usize,
    pub studio_mode: bool,
    pub program_scene: usize,
    pub stream_enabled: bool,
    pub stream_platform: StreamPlatform,
    pub stream_url: String,
    pub replay_buffer_enabled: bool,
    pub replay_buffer_seconds: u32,
}

impl Default for RecordingSettings {
    fn default() -> Self {
        Self {
            resolution_index: 0,
            custom_width: 1920,
            custom_height: 1080,
            fps: 30,
            custom_fps: 30,
            bitrate_kbps: DEFAULT_BITRATE_KBPS,
            encoder: "libx264".to_owned(),
            container: Container::Mkv,
            audio_device: None,
            system_audio_enabled: false,
            system_audio_volume: 1.0,
            use_desktop_duplication: true,
            camera_device: None,
            image_source: None,
            noise_suppression: false,
            noise_gate: false,
            compressor: false,
            limiter: false,
            brightness: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            sharpen: 0.0,
            camera_chroma_key: false,
            audio_volume: 1.0,
            audio_muted: false,
            output_dir: default_output_dir(),
            scenes: Vec::new(),
            active_scene: 0,
            studio_mode: false,
            program_scene: 0,
            stream_enabled: false,
            stream_platform: StreamPlatform::Kick,
            stream_url: StreamPlatform::Kick.default_server().to_owned(),
            replay_buffer_enabled: false,
            replay_buffer_seconds: 30,
        }
    }
}

impl RecordingSettings {
    pub fn recording_scene_index(&self) -> usize {
        if self.studio_mode {
            self.program_scene
        } else {
            self.active_scene
        }
    }

    pub fn resolution(&self) -> (u32, u32) {
        match self.resolution_index {
            1 => (1920, 1080),
            2 => (
                self.custom_width.max(2) & !1,
                self.custom_height.max(2) & !1,
            ),
            _ => (1280, 720),
        }
    }

    pub fn frame_rate(&self) -> u32 {
        if self.fps == 0 {
            self.custom_fps.clamp(1, 120)
        } else {
            self.fps
        }
    }

    pub fn for_display(&self, ffmpeg: PathBuf, display: Display) -> RecordingConfig {
        self.for_scene(ffmpeg, display, self.active_scene)
    }

    pub fn for_scene(
        &self,
        ffmpeg: PathBuf,
        display: Display,
        scene_index: usize,
    ) -> RecordingConfig {
        let (width, height) = self.resolution();
        let scene = self.scenes.get(scene_index);
        RecordingConfig {
            ffmpeg,
            display,
            capture_region: scene
                .and_then(|scene| scene.capture_region)
                .map(CaptureRegion::clamped),
            output_dir: self.output_dir.clone(),
            width,
            height,
            fps: self.frame_rate(),
            bitrate_kbps: self.bitrate_kbps.clamp(250, 100_000),
            encoder: self.encoder.clone(),
            container: self.container,
            audio_device: self.audio_device.clone(),
            system_audio_enabled: self.system_audio_enabled,
            system_audio_volume: self.system_audio_volume.clamp(0.0, 1.0),
            use_desktop_duplication: self.use_desktop_duplication,
            scene_sources: scene.map(|scene| scene.sources.clone()).unwrap_or_default(),
            camera_device: scene.and_then(|scene| {
                scene
                    .sources
                    .iter()
                    .find(|source| source.visible && source.kind == SceneSourceKind::Camera)
                    .map(|source| source.value.clone())
            }),
            image_source: scene.and_then(|scene| {
                scene
                    .sources
                    .iter()
                    .find(|source| source.visible && source.kind == SceneSourceKind::Image)
                    .map(|source| PathBuf::from(&source.value))
            }),
            window_title: scene.and_then(|scene| {
                scene
                    .sources
                    .iter()
                    .find(|source| source.visible && source.kind == SceneSourceKind::Window)
                    .map(|source| source.value.clone())
            }),
            video_source: scene.and_then(|scene| {
                scene
                    .sources
                    .iter()
                    .find(|source| source.visible && source.kind == SceneSourceKind::Video)
                    .map(|source| PathBuf::from(&source.value))
            }),
            text_overlay: scene.and_then(|scene| {
                scene
                    .sources
                    .iter()
                    .find(|source| source.visible && source.kind == SceneSourceKind::Text)
                    .map(|source| source.value.clone())
            }),
            camera_transform: scene.and_then(|scene| {
                scene
                    .sources
                    .iter()
                    .find(|source| source.visible && source.kind == SceneSourceKind::Camera)
                    .map(|source| {
                        source
                            .transform
                            .unwrap_or_else(|| SourceTransform::for_kind(source.kind))
                            .clamped()
                    })
            }),
            image_transform: scene.and_then(|scene| {
                scene
                    .sources
                    .iter()
                    .find(|source| source.visible && source.kind == SceneSourceKind::Image)
                    .map(|source| {
                        source
                            .transform
                            .unwrap_or_else(|| SourceTransform::for_kind(source.kind))
                            .clamped()
                    })
            }),
            video_transform: scene.and_then(|scene| {
                scene
                    .sources
                    .iter()
                    .find(|source| source.visible && source.kind == SceneSourceKind::Video)
                    .map(|source| {
                        source
                            .transform
                            .unwrap_or_else(|| SourceTransform::for_kind(source.kind))
                            .clamped()
                    })
            }),
            text_transform: scene.and_then(|scene| {
                scene
                    .sources
                    .iter()
                    .find(|source| source.visible && source.kind == SceneSourceKind::Text)
                    .map(|source| {
                        source
                            .transform
                            .unwrap_or_else(|| SourceTransform::for_kind(source.kind))
                            .clamped()
                    })
            }),
            noise_suppression: self.noise_suppression,
            noise_gate: self.noise_gate,
            compressor: self.compressor,
            limiter: self.limiter,
            brightness: self.brightness,
            contrast: self.contrast,
            saturation: self.saturation,
            sharpen: self.sharpen,
            camera_chroma_key: self.camera_chroma_key,
            audio_volume: if self.audio_muted {
                0.0
            } else {
                self.audio_volume.clamp(0.0, 1.0)
            },
            stream_url: self
                .stream_enabled
                .then(|| self.stream_url.trim().to_owned())
                .filter(|url| !url.is_empty()),
            replay_buffer_seconds: if self.replay_buffer_enabled {
                self.replay_buffer_seconds.clamp(5, 600)
            } else {
                0
            },
        }
    }
}

pub fn window_icon() -> Result<eframe::egui::IconData, image::ImageError> {
    let image = image::load_from_memory(include_bytes!("logo/ayrex.png"))?.into_rgba8();
    let (width, height) = image.dimensions();
    Ok(eframe::egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

pub fn default_output_dir() -> PathBuf {
    let home_variable = if cfg!(windows) {
        "USERPROFILE"
    } else {
        "HOME"
    };
    std::env::var_os(home_variable)
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Videos")
        .join("AyRex")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_resolution_is_even_and_fps_is_bounded() {
        let settings = RecordingSettings {
            resolution_index: 2,
            custom_width: 1919,
            custom_height: 1079,
            fps: 0,
            custom_fps: 240,
            ..RecordingSettings::default()
        };
        assert_eq!(settings.resolution(), (1918, 1078));
        assert_eq!(settings.frame_rate(), 120);
    }

    #[test]
    fn recording_settings_round_trip_and_keep_defaults_for_new_fields() {
        let settings = RecordingSettings {
            resolution_index: 1,
            audio_muted: true,
            ..RecordingSettings::default()
        };
        let json = serde_json::to_string(&settings).expect("settings should serialize");
        let restored: RecordingSettings =
            serde_json::from_str(&json).expect("saved settings should restore");
        assert_eq!(restored.resolution(), (1920, 1080));
        assert!(restored.audio_muted);
        assert_eq!(restored.output_dir, settings.output_dir);

        let older = r#"{"resolution_index":1}"#;
        let restored: RecordingSettings =
            serde_json::from_str(older).expect("older settings should use defaults");
        assert_eq!(restored.resolution(), (1920, 1080));
        assert_eq!(restored.encoder, "libx264");
    }

    #[test]
    fn muting_audio_applies_to_the_real_ffmpeg_input() {
        let mut settings = RecordingSettings {
            audio_device: Some("Microphone".to_owned()),
            audio_muted: true,
            ..RecordingSettings::default()
        };
        let display = Display {
            id: 1,
            name: "Primary".to_owned(),
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            primary: true,
            dxgi_output_index: None,
        };
        let config = settings.for_display(PathBuf::from("ffmpeg.exe"), display);
        assert_eq!(config.audio_volume, 0.0);

        settings.audio_muted = false;
        let config = settings.for_display(
            PathBuf::from("ffmpeg.exe"),
            Display {
                id: 1,
                name: "Primary".to_owned(),
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                primary: true,
                dxgi_output_index: None,
            },
        );
        assert_eq!(config.audio_volume, 1.0);
    }

    #[test]
    fn studio_mode_routes_recording_to_program_scene() {
        let mut settings = RecordingSettings {
            active_scene: 0,
            program_scene: 1,
            studio_mode: true,
            scenes: vec![
                Scene {
                    name: "Önizleme".to_owned(),
                    display_id: 1,
                    sources: vec![SceneSource::display()],
                    capture_region: None,
                },
                Scene {
                    name: "Program".to_owned(),
                    display_id: 2,
                    sources: vec![
                        SceneSource::display(),
                        SceneSource {
                            name: "Kamera B".to_owned(),
                            kind: SceneSourceKind::Camera,
                            value: "Kamera B".to_owned(),
                            visible: true,
                            transform: None,
                        },
                    ],
                    capture_region: Some(CaptureRegion {
                        x: 0.1,
                        y: 0.2,
                        width: 0.5,
                        height: 0.6,
                    }),
                },
            ],
            ..RecordingSettings::default()
        };
        let display = |id| Display {
            id,
            name: format!("Ekran {id}"),
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            primary: id == 1,
            dxgi_output_index: None,
        };
        let config = settings.for_scene(
            PathBuf::from("ffmpeg.exe"),
            display(2),
            settings.recording_scene_index(),
        );
        assert_eq!(config.display.id, 2);
        assert_eq!(config.camera_device.as_deref(), Some("Kamera B"));
        assert_eq!(config.capture_region.unwrap().x, 0.1);

        settings.studio_mode = false;
        assert_eq!(settings.recording_scene_index(), 0);
    }
}
