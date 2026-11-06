use std::{
    collections::HashSet,
    path::Path,
    process::{Command, Stdio},
};

use crate::config::{Container, RecordingConfig, SceneSource, SceneSourceKind, SourceTransform};

#[cfg(test)]
pub fn build_ffmpeg_command(config: &RecordingConfig, output_path: &Path) -> Command {
    build_ffmpeg_command_with_replay(config, output_path, None, None)
}

pub(crate) fn build_ffmpeg_command_with_replay(
    config: &RecordingConfig,
    output_path: &Path,
    replay_directory: Option<&Path>,
    system_audio_port: Option<u16>,
) -> Command {
    let replay_enabled = config.replay_buffer_seconds > 0 && replay_directory.is_some();
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
    ]);
    let legacy_window_capture = config.scene_sources.is_empty() && config.window_title.is_some();
    let use_dxgi_capture = config.use_desktop_duplication
        && config.display.dxgi_output_index.is_some()
        && !legacy_window_capture;
    if legacy_window_capture && let Some(title) = &config.window_title {
        #[cfg(windows)]
        command.args([
            "-f",
            "gdigrab",
            "-framerate",
            &config.fps.to_string(),
            "-i",
            &format!("title={title}"),
        ]);
        #[cfg(target_os = "linux")]
        let window_id = title.parse::<u32>().map_or_else(
            |_| title.clone(),
            |id| format!("0x{id:x}"),
        );
        #[cfg(target_os = "linux")]
        command.args([
            "-f",
            "x11grab",
            "-framerate",
            &config.fps.to_string(),
            "-window_id",
            &window_id,
            "-i",
            &std::env::var("DISPLAY").unwrap_or_else(|_| ":0.0".to_owned()),
        ]);
    } else if use_dxgi_capture {
        let mut source = format!(
            "ddagrab=output_idx={}:framerate={}",
            config.display.dxgi_output_index.unwrap_or_default(),
            config.fps
        );
        if let Some(region) = config.capture_region {
            let region = region.clamped();
            let offset_x = (region.x * config.display.width as f32).round() as i32;
            let offset_y = (region.y * config.display.height as f32).round() as i32;
            let capture_width = (region.width * config.display.width as f32)
                .round()
                .max(2.0) as u32;
            let capture_height = (region.height * config.display.height as f32)
                .round()
                .max(2.0) as u32;
            source.push_str(&format!(
                ":video_size={capture_width}x{capture_height}:offset_x={offset_x}:offset_y={offset_y}"
            ));
        }
        command.args(["-f", "lavfi", "-i", &source]);
    } else {
        let region = config.capture_region.unwrap_or_default().clamped();
        let offset_x = config.display.x + (region.x * config.display.width as f32).round() as i32;
        let offset_y = config.display.y + (region.y * config.display.height as f32).round() as i32;
        let capture_width = (region.width * config.display.width as f32)
            .round()
            .max(2.0) as u32;
        let capture_height = (region.height * config.display.height as f32)
            .round()
            .max(2.0) as u32;
        #[cfg(windows)]
        command.args([
            "-f",
            "gdigrab",
            "-framerate",
            &config.fps.to_string(),
            "-offset_x",
            &offset_x.to_string(),
            "-offset_y",
            &offset_y.to_string(),
            "-video_size",
            &format!("{}x{}", capture_width, capture_height),
            "-i",
            "desktop",
        ]);
        #[cfg(target_os = "linux")]
        {
            let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0.0".to_owned());
            let window_id = source.value.parse::<u32>().map_or_else(
                |_| source.value.clone(),
                |id| format!("0x{id:x}"),
            );
            command.args([
                "-f",
                "x11grab",
                "-framerate",
                &config.fps.to_string(),
                "-video_size",
                &format!("{}x{}", capture_width, capture_height),
                "-i",
                &format!("{display}+{offset_x},{offset_y}"),
            ]);
        }
    }

    let mut next_input = 1;
    let microphone_index = if let Some(device) = &config.audio_device {
        #[cfg(windows)]
        let window_id = source.value.parse::<u32>().map_or_else(
            |_| source.value.clone(),
            |id| format!("0x{id:x}"),
        );
        command.args([
            "-thread_queue_size",
            "512",
            "-f",
            "dshow",
            "-i",
            &format!("audio={device}"),
        ]);
        #[cfg(target_os = "linux")]
        command.args([
            "-thread_queue_size",
            "512",
            "-f",
            "pulse",
            "-i",
            device,
        ]);
        let index = next_input;
        next_input += 1;
        Some(index)
    } else {
        None
    };

    #[cfg(windows)]
    let system_audio_index = system_audio_port.map(|port| {
        command.args([
            "-thread_queue_size",
            "512",
            "-f",
            "f32le",
            "-ar",
            "48000",
            "-ac",
            "2",
            "-i",
            &format!("tcp://127.0.0.1:{port}"),
        ]);
        let index = next_input;
        next_input += 1;
        index
    });
    #[cfg(target_os = "linux")]
    let system_audio_index = config.system_audio_enabled.then(|| {
        command.args([
            "-thread_queue_size",
            "512",
            "-f",
            "pulse",
            "-i",
            "@DEFAULT_MONITOR@",
        ]);
        let index = next_input;
        next_input += 1;
        index
    });
    #[cfg(not(any(windows, target_os = "linux")))]
    let system_audio_index = system_audio_port.map(|_| {
        let index = next_input;
        next_input += 1;
        index
    });
    let scene_sources = scene_source_items(config);
    let source_inputs =
        append_scene_source_inputs(&mut command, config, scene_sources, &mut next_input);
    let has_scene_overlays = !source_inputs.is_empty();
    let shared_output = replay_enabled || config.stream_url.is_some();

    let mix_audio = microphone_index.is_some() && system_audio_index.is_some();
    let mut filter_graphs = Vec::new();
    if has_scene_overlays {
        filter_graphs.push(source_filter(config, &source_inputs, use_dxgi_capture));
    }
    if mix_audio {
        filter_graphs.push(audio_mix_filter(
            config,
            microphone_index.expect("mixed audio requires a microphone"),
            system_audio_index.expect("mixed audio requires system audio"),
            config.stream_url.is_some(),
        ));
    }
    if !filter_graphs.is_empty() {
        command.args(["-filter_complex", &filter_graphs.join(";")]);
    }

    if has_scene_overlays {
        command.args(["-map", "[outv]"]);
    } else {
        command.args([
            "-map",
            "0:v:0",
            "-vf",
            &video_filter(config, use_dxgi_capture),
        ]);
    }

    if shared_output {
        append_video_encoder(&mut command, config, replay_enabled);
        if mix_audio {
            append_separate_audio_encoder(&mut command, config.stream_url.is_some());
        } else {
            append_audio_encoder(
                &mut command,
                config,
                microphone_index,
                system_audio_index,
                None,
            );
        }
        let output_directory = output_path.parent().unwrap_or_else(|| Path::new("."));
        let output_name = output_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let audio_selection = if mix_audio {
            ":select=\\'v:0,a:0,a:1\\'"
        } else {
            ""
        };
        let mut targets = vec![format!(
            "[f={}:onfail=abort{}{}]{}",
            config.container.muxer(),
            audio_selection,
            if config.container == Container::Mp4 {
                ":movflags=+faststart"
            } else {
                ""
            },
            output_name
        )];
        if replay_enabled {
            let replay_directory = replay_directory.expect("replay mode has a replay directory");
            let replay_relative = replay_directory
                .strip_prefix(output_directory)
                .unwrap_or(replay_directory)
                .to_string_lossy()
                .replace('\\', "/");
            let slots = config
                .replay_buffer_seconds
                .saturating_add(config.replay_buffer_seconds.min(30))
                .saturating_add(2);
            targets.push(format!(
                "[f=segment:segment_format=mpegts:segment_time=1:segment_wrap={slots}:segment_list={replay_relative}/segments.csv:segment_list_size={slots}:segment_list_type=csv:reset_timestamps=1{audio_selection}:onfail=ignore]{replay_relative}/replay-%04d.ts"
            ));
        }
        if let Some(url) = config.stream_url.as_deref() {
            let muxer = crate::streaming::muxer_for_url(url)
                .map(|value| value.ffmpeg_name())
                .unwrap_or("flv");
            let stream_selection = if mix_audio {
                ":select=\\'v:0,a:2\\'"
            } else {
                ""
            };
            targets.push(format!(
                "[f={muxer}:onfail=ignore{stream_selection}]{}",
                escape_tee_target(url)
            ));
        }
        command.args(["-flags:v", "+global_header", "-use_fifo", "1", "-f", "tee"]);
        command.arg(targets.join("|"));
        command.current_dir(output_directory);
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        return command;
    }

    command.args([
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

    if mix_audio {
        append_separate_audio_encoder(&mut command, false);
    } else {
        append_audio_encoder(
            &mut command,
            config,
            microphone_index,
            system_audio_index,
            None,
        );
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

struct SceneSourceInput {
    source: SceneSource,
    input_index: Option<usize>,
}

fn scene_source_items(config: &RecordingConfig) -> Vec<SceneSource> {
    let mut items = if config.scene_sources.is_empty() {
        let mut legacy = Vec::new();
        if let Some(device) = &config.camera_device {
            legacy.push(legacy_source(
                "Camera",
                SceneSourceKind::Camera,
                device.clone(),
                config.camera_transform,
            ));
        }
        if let Some(path) = &config.image_source {
            legacy.push(legacy_source(
                "Image",
                SceneSourceKind::Image,
                path.to_string_lossy().into_owned(),
                config.image_transform,
            ));
        }
        if let Some(path) = &config.video_source {
            legacy.push(legacy_source(
                "Video",
                SceneSourceKind::Video,
                path.to_string_lossy().into_owned(),
                config.video_transform,
            ));
        }
        if let Some(text) = &config.text_overlay {
            legacy.push(legacy_source(
                "Text",
                SceneSourceKind::Text,
                text.clone(),
                config.text_transform,
            ));
        }
        legacy
    } else {
        config
            .scene_sources
            .iter()
            .filter(|source| source.visible && source.kind != SceneSourceKind::Display)
            .cloned()
            .collect()
    };

    let mut seen_cameras = HashSet::new();
    items.retain(|source| {
        source.kind != SceneSourceKind::Camera || seen_cameras.insert(source.value.clone())
    });
    items
}

fn legacy_source(
    name: &str,
    kind: SceneSourceKind,
    value: String,
    transform: Option<SourceTransform>,
) -> SceneSource {
    SceneSource {
        name: name.to_owned(),
        kind,
        value,
        visible: true,
        transform,
    }
}

fn append_scene_source_inputs(
    command: &mut Command,
    config: &RecordingConfig,
    sources: Vec<SceneSource>,
    next_input: &mut usize,
) -> Vec<SceneSourceInput> {
    let mut inputs = Vec::with_capacity(sources.len());
    for source in sources {
        let input_index = match source.kind {
            SceneSourceKind::Camera => {
                #[cfg(windows)]
                command.args([
                    "-thread_queue_size",
                    "512",
                    "-f",
                    "dshow",
                    "-video_size",
                    "1280x720",
                    "-framerate",
                    &config.fps.to_string(),
                    "-i",
                    &format!("video={}", source.value),
                ]);
                #[cfg(target_os = "linux")]
                command.args([
                    "-thread_queue_size",
                    "512",
                    "-f",
                    "v4l2",
                    "-video_size",
                    "1280x720",
                    "-framerate",
                    &config.fps.to_string(),
                    "-i",
                    &source.value,
                ]);
                Some(allocate_input_index(next_input))
            }
            SceneSourceKind::Window => {
                #[cfg(windows)]
                command.args([
                    "-thread_queue_size",
                    "512",
                    "-f",
                    "gdigrab",
                    "-framerate",
                    &config.fps.to_string(),
                    "-i",
                    &format!("title={}", source.value),
                ]);
                #[cfg(target_os = "linux")]
                {
                    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0.0".to_owned());
                    command.args([
                        "-thread_queue_size",
                        "512",
                        "-f",
                        "x11grab",
                        "-framerate",
                        &config.fps.to_string(),
                        "-window_id",
                        &window_id,
                        "-i",
                        &display,
                    ]);
                }
                Some(allocate_input_index(next_input))
            }
            SceneSourceKind::Image => {
                command.args([
                    "-loop",
                    "1",
                    "-framerate",
                    &config.fps.to_string(),
                    "-i",
                    &source.value,
                ]);
                Some(allocate_input_index(next_input))
            }
            SceneSourceKind::Video => {
                command.args([
                    "-thread_queue_size",
                    "512",
                    "-stream_loop",
                    "-1",
                    "-re",
                    "-i",
                    &source.value,
                ]);
                Some(allocate_input_index(next_input))
            }
            SceneSourceKind::Text => None,
            SceneSourceKind::Display => continue,
        };
        inputs.push(SceneSourceInput {
            source,
            input_index,
        });
    }
    inputs
}

fn allocate_input_index(next_input: &mut usize) -> usize {
    let index = *next_input;
    *next_input += 1;
    index
}

fn source_filter(
    config: &RecordingConfig,
    sources: &[SceneSourceInput],
    dxgi_capture: bool,
) -> String {
    let mut graph = format!(
        "[0:v]{}{}[screen]",
        if dxgi_capture {
            "hwdownload,format=bgra,"
        } else {
            ""
        },
        super::filters::video_filter(config)
    );
    let mut base = "screen".to_owned();
    for (source_index, input) in sources.iter().enumerate() {
        let source = &input.source;
        let geometry = source_geometry(config, source.transform, source.kind);
        let layer = format!("source_{source_index}");
        let merged = format!("merged_{source_index}");
        if source.kind == SceneSourceKind::Text {
            let text = source.value.trim();
            if text.is_empty() {
                continue;
            }
            let height_font = geometry.height / 3;
            let width_font =
                (geometry.width as f32 / (text.chars().count().max(1) as f32 * 0.62)) as u32;
            let font_size = height_font.min(width_font).clamp(10, 160);
            graph.push_str(&format!(
                ";[{}]drawtext={}:text='{}':expansion=none:fontcolor=white:fontsize={}:box=1:boxcolor=black@0.55:boxborderw=12:x={}:y={}[{}]",
                base,
                if cfg!(windows) {
                    "fontfile='C\\:/Windows/Fonts/arial.ttf'"
                } else {
                    "font='Sans'"
                },
                escape_drawtext_text(text),
                font_size,
                geometry.x,
                geometry.y,
                merged
            ));
            base = merged;
            continue;
        }

        let input_index = input
            .input_index
            .expect("video scene sources have a media input");
        let chroma = if source.kind == SceneSourceKind::Camera && config.camera_chroma_key {
            ",colorkey=0x00FF00:0.30:0.10"
        } else {
            ""
        };
        graph.push_str(&format!(
            ";[{input_index}:v]crop=w=iw*{:.4}:h=ih*{:.4}:x=iw*{:.4}:y=ih*{:.4}{chroma},scale={}:{}:force_original_aspect_ratio=decrease[{layer}];[{}][{layer}]overlay={}:{}:eof_action=pass[{merged}]",
            geometry.crop_width,
            geometry.crop_height,
            geometry.crop_left,
            geometry.crop_top,
            geometry.width,
            geometry.height,
            base,
            geometry.x,
            geometry.y
        ));
        base = merged;
    }
    let overlay = format!("[{base}]null");
    format!("{graph};{overlay}[outv]")
}

fn video_filter(config: &RecordingConfig, dxgi_capture: bool) -> String {
    let filters = super::filters::video_filter(config);
    if dxgi_capture {
        format!("hwdownload,format=bgra,{filters}")
    } else {
        filters
    }
}

fn append_audio_encoder(
    command: &mut Command,
    config: &RecordingConfig,
    microphone_index: Option<usize>,
    system_audio_index: Option<usize>,
    mixed_audio_label: Option<&str>,
) {
    if let Some(label) = mixed_audio_label {
        command.args(["-map", label, "-c:a", "aac", "-b:a", "192k"]);
    } else if let Some(index) = microphone_index {
        command.args([
            "-map",
            &format!("{index}:a:0"),
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-af",
            &super::filters::audio_filter(config),
        ]);
    } else if let Some(index) = system_audio_index {
        command.args([
            "-map",
            &format!("{index}:a:0"),
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-af",
            &format!("volume={:.2}", config.system_audio_volume.clamp(0.0, 1.0)),
        ]);
    }
}

fn append_separate_audio_encoder(command: &mut Command, include_live_mix: bool) {
    command.args(["-map", "[mictrack]", "-map", "[systemtrack]"]);
    if include_live_mix {
        command.args(["-map", "[liveaudio]"]);
    }
    command.args([
        "-c:a",
        "aac",
        "-b:a",
        "192k",
        "-metadata:s:a:0",
        "title=Mikrofon",
        "-metadata:s:a:1",
        "title=Masaüstü",
    ]);
    if include_live_mix {
        command.args(["-metadata:s:a:2", "title=Yayın miks"]);
    }
}

fn audio_mix_filter(
    config: &RecordingConfig,
    microphone_index: usize,
    system_audio_index: usize,
    streaming: bool,
) -> String {
    let mic_filters = super::filters::audio_filter(config);
    let system_volume = config.system_audio_volume.clamp(0.0, 1.0);
    let system_branch = if streaming {
        "asplit=2[systemtrack][systemstream]"
    } else {
        "anull[systemtrack]"
    };
    let microphone_branch = if streaming {
        "asplit=2[mictrack][micstream]"
    } else {
        "anull[mictrack]"
    };
    let mut graph = format!(
        "[{microphone_index}:a:0]{mic_filters},{microphone_branch};[{system_audio_index}:a:0]volume={system_volume:.2},{system_branch}"
    );
    if streaming {
        graph.push_str(
            ";[micstream][systemstream]amix=inputs=2:duration=longest:dropout_transition=0:normalize=0[liveaudio]",
        );
    }
    graph
}

fn append_video_encoder(command: &mut Command, config: &RecordingConfig, replay: bool) {
    command.args([
        "-r",
        &config.fps.to_string(),
        "-fps_mode",
        "cfr",
        "-c:v",
        &config.encoder,
    ]);
    append_encoder_preset(command, &config.encoder);
    let bitrate = format!("{}k", config.bitrate_kbps);
    let buffer_size = format!("{}k", config.bitrate_kbps.saturating_mul(2));
    if config.stream_url.is_some() {
        command.args(["-minrate", &bitrate]);
        command.args(["-profile:v", "main"]);
        if config.encoder == "libx264" {
            command.args(["-tune", "zerolatency"]);
        }
    }
    let keyframe_interval = if replay {
        config.fps.max(1)
    } else {
        config.fps.saturating_mul(2)
    };
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
        &keyframe_interval.to_string(),
    ]);
}

fn append_encoder_preset(command: &mut Command, encoder: &str) {
    match encoder {
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
}

fn escape_tee_target(target: &str) -> String {
    let mut escaped = String::with_capacity(target.len());
    for character in target.chars() {
        if matches!(character, '\\' | '|' | '[' | ']') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

struct SourceGeometry {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    crop_left: f32,
    crop_top: f32,
    crop_width: f32,
    crop_height: f32,
}

fn source_geometry(
    config: &RecordingConfig,
    transform: Option<SourceTransform>,
    kind: SceneSourceKind,
) -> SourceGeometry {
    let transform = transform
        .unwrap_or_else(|| SourceTransform::for_kind(kind))
        .clamped();
    let width = (config.width as f32 * transform.width).round().max(2.0) as u32;
    let height = (config.height as f32 * transform.height).round().max(2.0) as u32;
    SourceGeometry {
        x: (config.width as f32 * transform.x).round() as u32,
        y: (config.height as f32 * transform.y).round() as u32,
        width,
        height,
        crop_left: transform.crop_left,
        crop_top: transform.crop_top,
        crop_width: 1.0 - transform.crop_left - transform.crop_right,
        crop_height: 1.0 - transform.crop_top - transform.crop_bottom,
    }
}

fn escape_drawtext_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, '\\' | ':' | '\'' | ',' | '[' | ']' | ';') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
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
                dxgi_output_index: None,
            },
            capture_region: None,
            output_dir: PathBuf::from("recordings"),
            width: 1920,
            height: 1080,
            fps: 30,
            bitrate_kbps: 6_000,
            encoder: "libx264".to_owned(),
            container: Container::Mp4,
            audio_device,
            system_audio_enabled: false,
            system_audio_volume: 1.0,
            use_desktop_duplication: true,
            scene_sources: Vec::new(),
            camera_device: None,
            image_source: None,
            window_title: None,
            video_source: None,
            text_overlay: None,
            camera_transform: None,
            image_transform: None,
            video_transform: None,
            text_transform: None,
            replay_buffer_seconds: 0,
            noise_suppression: false,
            noise_gate: false,
            compressor: false,
            limiter: false,
            brightness: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            sharpen: 0.0,
            camera_chroma_key: false,
            audio_volume: 0.5,
            stream_url: None,
        }
    }

    fn scene_source(kind: SceneSourceKind, name: &str, value: &str) -> SceneSource {
        SceneSource {
            name: name.to_owned(),
            kind,
            value: value.to_owned(),
            visible: true,
            transform: None,
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

        #[cfg(windows)]
        assert!(args.contains("-offset_x -100"));
        #[cfg(target_os = "linux")]
        assert!(args.contains("DISPLAY+-100,0") || args.contains(":0.0+-100,0"));
        assert!(args.contains("-video_size 1366x768"));
        assert!(args.contains("scale=1920:1080"));
        assert!(args.contains("-r 30"));
        assert!(args.contains("-b:v 6000k"));
        assert!(args.contains("-f mp4"));
    }

    #[test]
    fn command_converts_normalized_capture_region_to_monitor_coordinates() {
        let mut config = test_config(None);
        config.capture_region = Some(crate::config::CaptureRegion {
            x: 0.25,
            y: 0.10,
            width: 0.50,
            height: 0.75,
        });
        let command = build_ffmpeg_command(&config, Path::new("test.mp4"));
        let args = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");

        #[cfg(windows)]
        {
            assert!(args.contains("-offset_x 242"));
            assert!(args.contains("-offset_y 77"));
        }
        #[cfg(target_os = "linux")]
        assert!(args.contains(":0.0+242,77"));
        assert!(args.contains("-video_size 683x576"));
    }

    #[test]
    #[cfg(windows)]
    fn desktop_duplication_uses_the_mapped_dxgi_output_and_downloads_frames_for_filters() {
        let mut config = test_config(None);
        config.display.dxgi_output_index = Some(2);
        config.capture_region = Some(crate::config::CaptureRegion {
            x: 0.25,
            y: 0.10,
            width: 0.50,
            height: 0.75,
        });
        let args = build_ffmpeg_command(&config, Path::new("test.mp4"))
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(args.contains(
            "-f lavfi -i ddagrab=output_idx=2:framerate=30:video_size=683x576:offset_x=342:offset_y=77"
        ));
        assert!(args.contains("-vf hwdownload,format=bgra,scale=1920:1080"));
        assert!(!args.contains("-f gdigrab"));
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

        #[cfg(windows)]
        assert!(args.contains("audio=Microphone Device"));
        #[cfg(target_os = "linux")]
        assert!(args.contains("-f pulse -i Microphone Device"));
        assert!(args.contains("-map 1:a:0"));
        assert!(args.contains("-c:a aac"));
        assert!(args.contains("volume=0.50"));
    }

    #[test]
    #[cfg(windows)]
    fn system_audio_uses_a_local_float_stream_and_is_written_to_its_own_track() {
        let mut config = test_config(Some("Microphone".to_owned()));
        config.system_audio_enabled = true;
        config.system_audio_volume = 0.75;
        let command = build_ffmpeg_command_with_replay(
            &config,
            Path::new("recordings/mixed.mkv"),
            None,
            Some(45_321),
        );
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(args.contains("-f f32le -ar 48000 -ac 2 -i tcp://127.0.0.1:45321"));
        assert!(args.contains("[1:a:0]volume=0.50,anull[mictrack]"));
        assert!(args.contains("[2:a:0]volume=0.75,anull[systemtrack]"));
        assert!(args.contains("-map [mictrack] -map [systemtrack]"));
        assert!(args.contains("title=Mikrofon"));
        assert!(args.contains("title=Masaüstü"));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn system_audio_uses_the_default_pulse_monitor_source() {
        let mut config = test_config(Some("default".to_owned()));
        config.system_audio_enabled = true;
        let args = build_ffmpeg_command(&config, Path::new("recordings/mixed.mkv"))
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(args.contains("-f pulse -i @DEFAULT_MONITOR@"));
        assert!(args.contains("-map 2:a:0"));
    }

    #[test]
    fn separate_tracks_are_recorded_while_streaming_uses_a_distinct_mix() {
        let mut config = test_config(Some("Microphone".to_owned()));
        config.system_audio_enabled = true;
        config.stream_url = Some("rtmps://example.invalid/live/key".to_owned());
        let args = build_ffmpeg_command_with_replay(
            &config,
            Path::new("recordings/mixed.mkv"),
            None,
            Some(45_321),
        )
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ");

        assert!(args.contains("[1:a:0]volume=0.50,asplit=2[mictrack][micstream]"));
        assert!(args.contains("[2:a:0]volume=1.00,asplit=2[systemtrack][systemstream]"));
        assert!(args.contains("[micstream][systemstream]amix=inputs=2:duration=longest:dropout_transition=0:normalize=0[liveaudio]"));
        assert!(args.contains("-map [mictrack] -map [systemtrack]"));
        assert!(args.contains("-map [liveaudio]"));
        assert!(args.contains("title=Yayın miks"));
    }

    #[test]
    fn replay_uses_separate_recording_tracks_and_a_mixed_live_stream() {
        let mut config = test_config(Some("Microphone".to_owned()));
        config.system_audio_enabled = true;
        config.replay_buffer_seconds = 12;
        config.stream_url = Some("rtmps://example.invalid/live/key".to_owned());
        let command = build_ffmpeg_command_with_replay(
            &config,
            Path::new("recordings/record.mkv"),
            Some(Path::new("recordings/.replay")),
            Some(45_321),
        );
        let args = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(args.contains("-map [mictrack] -map [systemtrack] -map [liveaudio]"));
        assert!(args.contains("select=\\'v:0,a:0,a:1\\'"));
        assert!(args.contains("select=\\'v:0,a:2\\'"));
        assert!(args.contains("title=Mikrofon"));
        assert!(args.contains("title=Masaüstü"));
    }

    #[test]
    fn streaming_tees_one_encode_and_does_not_abort_local_recording_on_network_failure() {
        let mut config = test_config(None);
        config.stream_url = Some("rtmps://example.invalid/live/key".to_owned());
        let args = build_ffmpeg_command(&config, Path::new("test.mp4"))
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(args.contains("-use_fifo 1 -f tee"));
        assert!(args.contains("[f=mp4:onfail=abort:movflags=+faststart]test.mp4"));
        assert!(args.contains("[f=flv:onfail=ignore]rtmps://example.invalid/live/key"));
        assert!(args.contains("-minrate 6000k"));
        assert!(args.contains("-profile:v main"));
        assert!(args.contains("-tune zerolatency"));
        assert!(args.contains("-g 60"));
    }

    #[test]
    fn command_uses_mpegts_for_srt_streaming() {
        let mut config = test_config(None);
        config.stream_url = Some("srt://127.0.0.1:9000?mode=caller".to_owned());
        let args = build_ffmpeg_command(&config, Path::new("test.mkv"))
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(args.contains("[f=mpegts:onfail=ignore]srt://127.0.0.1:9000?mode=caller"));
    }

    #[test]
    fn camera_is_composited_into_the_recording() {
        let mut config = test_config(None);
        config.camera_device = Some("USB Camera".to_owned());
        let args = build_ffmpeg_command(&config, Path::new("test.mkv"))
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        #[cfg(windows)]
        assert!(args.contains("video=USB Camera"));
        #[cfg(target_os = "linux")]
        assert!(args.contains("-f v4l2"));
        assert!(args.contains("scale=499:292:force_original_aspect_ratio=decrease[source_0]"));
        assert!(args.contains("overlay=1382:734:eof_action=pass[merged_0]"));
        assert!(args.contains("[merged_0]null[outv]"));
    }

    #[test]
    fn still_image_is_added_as_an_overlay_input() {
        let mut config = test_config(None);
        config.image_source = Some(PathBuf::from("logo image.png"));
        let args = build_ffmpeg_command(&config, Path::new("test.mkv"))
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(args.contains("-loop 1 -framerate 30 -i logo image.png"));
        assert!(args.contains("scale=499:292:force_original_aspect_ratio=decrease[source_0]"));
        assert!(args.contains("overlay=38:734:eof_action=pass[merged_0]"));
    }

    #[test]
    fn scene_graph_supports_multiple_window_image_and_text_sources() {
        let mut config = test_config(None);
        config.scene_sources = vec![
            SceneSource::display(),
            scene_source(
                SceneSourceKind::Window,
                "AyRex Window",
                if cfg!(target_os = "linux") {
                    "234"
                } else {
                    "AyRex Test Window"
                },
            ),
            scene_source(SceneSourceKind::Image, "Logo one", "logo one.png"),
            scene_source(SceneSourceKind::Image, "Logo two", "logo two.png"),
            scene_source(SceneSourceKind::Text, "Title", "First title"),
            scene_source(SceneSourceKind::Text, "Subtitle", "Second title"),
        ];
        let args = build_ffmpeg_command(&config, Path::new("test.mkv"))
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");

        #[cfg(windows)]
        {
            assert!(args.contains("-i desktop"));
            assert!(args.contains("title=AyRex Test Window"));
        }
        #[cfg(target_os = "linux")]
        assert!(args.contains("-window_id 0xea"));
        assert!(args.contains("-i logo one.png"));
        assert!(args.contains("-i logo two.png"));
        assert_eq!(args.matches("drawtext=").count(), 2);
        assert!(args.contains("[merged_4]null[outv]"));
    }

    #[test]
    fn audio_mixer_filters_are_applied_to_the_microphone_stream() {
        let mut config = test_config(Some("Mic".to_owned()));
        config.noise_suppression = true;
        config.noise_gate = true;
        config.compressor = true;
        config.limiter = true;
        let args = build_ffmpeg_command(&config, Path::new("test.mkv"))
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(args.contains("afftdn=nf=-25,agate=threshold=0.02:ratio=4:attack=20:release=250,acompressor=threshold=0.125:ratio=3:attack=20:release=250,alimiter=limit=0.95"));
    }

    #[test]
    fn color_filters_are_included_in_the_video_path() {
        let mut config = test_config(None);
        config.brightness = 0.1;
        config.contrast = 1.2;
        config.saturation = 1.5;
        config.sharpen = 0.4;
        let args = build_ffmpeg_command(&config, Path::new("test.mkv"))
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            args.contains("eq=brightness=0.100:contrast=1.200:saturation=1.500,unsharp=5:5:0.40")
        );
    }

    #[test]
    #[cfg(windows)]
    fn window_capture_uses_the_selected_window_title() {
        let mut config = test_config(None);
        config.window_title = Some("Untitled — AyRex".to_owned());
        let args = build_ffmpeg_command(&config, Path::new("test.mkv"))
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(args.contains("title=Untitled — AyRex"));
        assert!(!args.contains("desktop"));
    }

    #[test]
    fn text_source_is_rendered_by_ffmpeg_as_a_literal_overlay() {
        let mut config = test_config(None);
        config.text_overlay = Some("Merhaba: AyRex; % yayında".to_owned());
        let args = build_ffmpeg_command(&config, Path::new("test.mkv"))
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        #[cfg(windows)]
        assert!(args.contains("drawtext=fontfile='C\\:/Windows/Fonts/arial.ttf'"));
        #[cfg(target_os = "linux")]
        assert!(args.contains("drawtext=font='Sans'"));
        assert!(args.contains("text='Merhaba\\: AyRex\\; % yayında':expansion=none"));
        assert!(args.contains("[merged_0]null[outv]"));
    }

    #[test]
    fn overlay_position_size_and_crop_are_applied_to_ffmpeg_graph() {
        let mut config = test_config(None);
        config.camera_device = Some("USB Camera".to_owned());
        config.camera_transform = Some(SourceTransform {
            x: 0.5,
            y: 0.25,
            width: 0.4,
            height: 0.5,
            crop_left: 0.1,
            crop_top: 0.05,
            crop_right: 0.1,
            crop_bottom: 0.1,
        });
        let args = build_ffmpeg_command(&config, Path::new("test.mkv"))
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(args.contains("crop=w=iw*0.8000:h=ih*0.8500:x=iw*0.1000:y=ih*0.0500"));
        assert!(args.contains("scale=768:540:force_original_aspect_ratio=decrease"));
        assert!(args.contains("overlay=960:270:eof_action=pass"));
    }

    #[test]
    fn replay_buffer_tees_one_encoded_stream_to_recording_segments_and_live_output() {
        let mut config = test_config(Some("Microphone".to_owned()));
        config.replay_buffer_seconds = 12;
        config.stream_url = Some("srt://127.0.0.1:9000?mode=caller".to_owned());
        let command = build_ffmpeg_command_with_replay(
            &config,
            Path::new("recordings/record.mkv"),
            Some(Path::new("recordings/.replay")),
            None,
        );
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let joined = args.join(" ");
        assert!(joined.contains("-flags:v +global_header"));
        assert!(joined.contains("-use_fifo 1 -f tee"));
        assert!(joined.contains("[f=mp4:onfail=abort:movflags=+faststart]record.mkv"));
        assert!(joined.contains("segment_wrap=26"));
        assert!(joined.contains("segment_list=.replay/segments.csv"));
        assert!(joined.contains("[f=mpegts:onfail=ignore]srt://127.0.0.1:9000?mode=caller"));
        assert_eq!(args.iter().filter(|arg| arg.as_str() == "-c:v").count(), 1);
        assert_eq!(args.iter().filter(|arg| arg.as_str() == "-c:a").count(), 1);
    }
}
