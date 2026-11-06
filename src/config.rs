use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::capture::Display;

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
    pub output_dir: PathBuf,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub encoder: String,
    pub container: Container,
    pub audio_device: Option<String>,
    pub audio_volume: f32,
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
    pub audio_volume: f32,
    pub audio_muted: bool,
    pub output_dir: PathBuf,
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
            audio_volume: 1.0,
            audio_muted: false,
            output_dir: default_output_dir(),
        }
    }
}

impl RecordingSettings {
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
        let (width, height) = self.resolution();
        RecordingConfig {
            ffmpeg,
            display,
            output_dir: self.output_dir.clone(),
            width,
            height,
            fps: self.frame_rate(),
            bitrate_kbps: self.bitrate_kbps.clamp(250, 100_000),
            encoder: self.encoder.clone(),
            container: self.container,
            audio_device: self.audio_device.clone(),
            audio_volume: if self.audio_muted {
                0.0
            } else {
                self.audio_volume.clamp(0.0, 1.0)
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
    std::env::var_os("USERPROFILE")
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
            },
        );
        assert_eq!(config.audio_volume, 1.0);
    }
}
