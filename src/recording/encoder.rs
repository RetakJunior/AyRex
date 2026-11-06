use std::{
    path::Path,
    process::{Command, Stdio},
};

use crate::config::{Container, RecordingConfig};

pub fn build_ffmpeg_command(config: &RecordingConfig, output_path: &Path) -> Command {
    let mut command = Command::new(&config.ffmpeg);
    command.args([
        "-hide_banner",
        "-loglevel",
        "warning",
        "-y",
        "-stats_period",
        "0.5",
        "-progress",
        "pipe:1",
        "-thread_queue_size",
        "512",
        "-f",
        "gdigrab",
        "-framerate",
        &config.fps.to_string(),
        "-offset_x",
        &config.display.x.to_string(),
        "-offset_y",
        &config.display.y.to_string(),
        "-video_size",
        &format!("{}x{}", config.display.width, config.display.height),
        "-i",
        "desktop",
    ]);

    if let Some(device) = &config.audio_device {
        command.args([
            "-thread_queue_size",
            "512",
            "-f",
            "dshow",
            "-i",
            &format!("audio={device}"),
        ]);
    }

    command.args([
        "-map",
        "0:v:0",
        "-vf",
        &format!("scale={}:{}:flags=bilinear", config.width, config.height),
        "-r",
        &config.fps.to_string(),
        "-fps_mode",
        "cfr",
        "-c:v",
        &config.encoder,
    ]);

    match config.encoder.as_str() {
        "libx264" => {
            command.args(["-preset", "veryfast"]);
        }
        "libx265" => {
            command.args(["-preset", "ultrafast"]);
        }
        "h264_nvenc" | "hevc_nvenc" => {
            command.args(["-preset", "p4"]);
        }
        "h264_qsv" | "hevc_qsv" => {
            command.args(["-preset", "faster"]);
        }
        "h264_amf" | "hevc_amf" => {
            command.args(["-quality", "speed"]);
        }
        _ => {}
    }

    let bitrate = format!("{}k", config.bitrate_kbps);
    let buffer_size = format!("{}k", config.bitrate_kbps.saturating_mul(2));
    command.args([
        "-pix_fmt",
        "yuv420p",
        "-b:v",
        &bitrate,
        "-maxrate",
        &bitrate,
        "-bufsize",
        &buffer_size,
        "-g",
        &(config.fps.saturating_mul(2)).to_string(),
    ]);

    if config.audio_device.is_some() {
        command.args([
            "-map",
            "1:a:0",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-af",
            &format!("volume={:.2}", config.audio_volume),
        ]);
    }

    command.args(["-f", config.container.muxer()]);
    if config.container == Container::Mp4 {
        command.args(["-movflags", "+faststart"]);
    }
    command.arg(output_path);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

pub fn encoder_label(encoder: &str) -> &str {
    match encoder {
        "libx264" => "H.264 (software)",
        "libx265" => "H.265 (software)",
        "h264_nvenc" => "H.264 (NVIDIA)",
        "hevc_nvenc" => "H.265 (NVIDIA)",
        "h264_qsv" => "H.264 (Intel Quick Sync)",
        "hevc_qsv" => "H.265 (Intel Quick Sync)",
        "h264_amf" => "H.264 (AMD)",
        "hevc_amf" => "H.265 (AMD)",
        _ => encoder,
    }
}

pub fn is_hardware_encoder(encoder: &str) -> bool {
    encoder.ends_with("_nvenc") || encoder.ends_with("_qsv") || encoder.ends_with("_amf")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::Display;
    use std::path::PathBuf;

    fn test_config(audio_device: Option<String>) -> RecordingConfig {
        RecordingConfig {
            ffmpeg: PathBuf::from("ffmpeg.exe"),
            display: Display {
                id: 1,
                name: "Primary".to_owned(),
                x: -100,
                y: 0,
                width: 1366,
                height: 768,
                primary: true,
            },
            output_dir: PathBuf::from("recordings"),
            width: 1920,
            height: 1080,
            fps: 30,
            bitrate_kbps: 6_000,
            encoder: "libx264".to_owned(),
            container: Container::Mp4,
            audio_device,
            audio_volume: 0.5,
        }
    }

    #[test]
    fn command_uses_selected_capture_and_encoding_settings() {
        let command = build_ffmpeg_command(&test_config(None), Path::new("test.mp4"));
        let args = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(args.contains("-offset_x -100"));
        assert!(args.contains("-video_size 1366x768"));
        assert!(args.contains("scale=1920:1080"));
        assert!(args.contains("-r 30"));
        assert!(args.contains("-b:v 6000k"));
        assert!(args.contains("-f mp4"));
    }

    #[test]
    fn command_maps_selected_audio_input() {
        let command = build_ffmpeg_command(
            &test_config(Some("Microphone Device".to_owned())),
            Path::new("test.mp4"),
        );
        let args = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(args.contains("audio=Microphone Device"));
        assert!(args.contains("-map 1:a:0"));
        assert!(args.contains("-c:a aac"));
        assert!(args.contains("volume=0.50"));
    }
}
