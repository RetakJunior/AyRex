use std::{path::Path, path::PathBuf, process::Command};

pub fn locate_ffmpeg() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("AYREX_FFMPEG") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let executable = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    if let Some(sibling) = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|directory| directory.join(executable)))
        .filter(|path| path.is_file())
    {
        return Some(sibling);
    }
    std::env::split_paths(&std::env::var_os("PATH")?).find_map(|directory| {
        let candidate = directory.join(executable);
        candidate.is_file().then_some(candidate)
    })
}

pub fn discover_encoders(ffmpeg: &Path) -> Result<Vec<String>, String> {
    let output = Command::new(ffmpeg)
        .args(["-hide_banner", "-encoders"])
        .output()
        .map_err(|error| format!("Could not query FFmpeg encoders: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Could not query FFmpeg encoders: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let supported = [
        "libx264",
        "libx265",
        "h264_nvenc",
        "hevc_nvenc",
        "h264_qsv",
        "hevc_qsv",
        "h264_amf",
        "hevc_amf",
    ];
    let found = supported
        .iter()
        .filter(|encoder| {
            text.lines()
                .any(|line| line.split_whitespace().any(|field| field == **encoder))
        })
        .map(|encoder| (*encoder).to_owned())
        .collect::<Vec<_>>();
    if found.is_empty() {
        return Err("FFmpeg has no supported H.264 or H.265 encoder.".to_owned());
    }
    Ok(found)
}
