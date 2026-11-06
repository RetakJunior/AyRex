use std::path::Path;
#[cfg(any(windows, target_os = "linux"))]
use std::process::Command;

#[cfg(windows)]
pub fn discover_audio_devices(ffmpeg: &Path) -> Result<Vec<String>, String> {
    discover_devices(ffmpeg).map(|(audio, _)| audio)
}

#[cfg(target_os = "linux")]
pub fn discover_audio_devices(ffmpeg: &Path) -> Result<Vec<String>, String> {
    let output = Command::new(ffmpeg)
        .args(["-hide_banner", "-devices"])
        .output()
        .map_err(|error| format!("Could not query FFmpeg audio devices: {error}"))?;
    let devices = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "Could not query FFmpeg audio devices: {}",
            devices.trim()
        ));
    }
    if !devices.contains("pulse") {
        return Err(
            "FFmpeg does not provide PulseAudio input; install an FFmpeg build with PulseAudio support."
                .to_owned(),
        );
    }
    Ok(vec!["default".to_owned()])
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn discover_audio_devices(_ffmpeg: &Path) -> Result<Vec<String>, String> {
    Ok(Vec::new())
}

#[cfg(windows)]
pub fn discover_video_devices(ffmpeg: &Path) -> Result<Vec<String>, String> {
    discover_devices(ffmpeg).map(|(_, video)| video)
}

#[cfg(target_os = "linux")]
pub fn discover_video_devices(_ffmpeg: &Path) -> Result<Vec<String>, String> {
    let entries = std::fs::read_dir("/dev")
        .map_err(|error| format!("Could not enumerate camera devices in /dev: {error}"))?;
    let mut devices = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("Could not read camera device entry: {error}"))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.starts_with("video") {
            devices.push(format!("/dev/{name}"));
        }
    }
    devices.sort();
    Ok(devices)
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn discover_video_devices(_ffmpeg: &Path) -> Result<Vec<String>, String> {
    Ok(Vec::new())
}

#[cfg(windows)]
fn discover_devices(ffmpeg: &Path) -> Result<(Vec<String>, Vec<String>), String> {
    let output = Command::new(ffmpeg)
        .args([
            "-hide_banner",
            "-list_devices",
            "true",
            "-f",
            "dshow",
            "-i",
            "dummy",
        ])
        .output()
        .map_err(|error| format!("Could not enumerate audio/video devices: {error}"))?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok((
        parse_devices(&stderr, "(audio)"),
        parse_devices(&stderr, "(video)"),
    ))
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn device_parser_only_returns_quoted_audio_devices() {
        let output = r#"
[dshow @ 0001] "Camera" (video)
[dshow @ 0001] "Microphone" (audio)
[dshow @ 0001] "Microphone" (audio)
"#;
        let devices = super::parse_audio_devices(output);
        assert_eq!(devices, ["Microphone"]);
    }
}

#[cfg(windows)]
fn parse_devices(stderr: &str, kind: &str) -> Vec<String> {
    let mut devices = Vec::new();
    for line in stderr.lines().filter(|line| line.contains(kind)) {
        let Some((_, name)) = line.split_once("] ") else {
            continue;
        };
        let Some((_, quoted)) = name.split_once('"') else {
            continue;
        };
        let Some((device, _)) = quoted.split_once('"') else {
            continue;
        };
        if !devices.iter().any(|existing| existing == device) {
            devices.push(device.to_owned());
        }
    }
    devices
}

#[cfg(all(test, windows))]
fn parse_audio_devices(stderr: &str) -> Vec<String> {
    parse_devices(stderr, "(audio)")
}
