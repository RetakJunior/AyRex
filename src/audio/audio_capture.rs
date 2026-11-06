use std::{path::Path, process::Command};

pub fn discover_audio_devices(ffmpeg: &Path) -> Result<Vec<String>, String> {
    discover_devices(ffmpeg).map(|(audio, _)| audio)
}

pub fn discover_video_devices(ffmpeg: &Path) -> Result<Vec<String>, String> {
    discover_devices(ffmpeg).map(|(_, video)| video)
}

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

#[cfg(test)]
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

#[cfg(test)]
fn parse_audio_devices(stderr: &str) -> Vec<String> {
    parse_devices(stderr, "(audio)")
}
