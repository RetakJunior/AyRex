use std::{path::Path, process::Command};

pub fn discover_audio_devices(ffmpeg: &Path) -> Result<Vec<String>, String> {
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
        .map_err(|error| format!("Could not enumerate audio devices: {error}"))?;
    Ok(parse_audio_devices(&String::from_utf8_lossy(
        &output.stderr,
    )))
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

fn parse_audio_devices(stderr: &str) -> Vec<String> {
    let mut devices = Vec::new();
    for line in stderr.lines().filter(|line| line.contains("(audio)")) {
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
