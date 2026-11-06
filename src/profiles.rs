use std::{fs, path::Path};

use crate::config::RecordingSettings;

pub fn save(path: &Path, settings: &RecordingSettings) -> Result<(), String> {
    let mut settings = settings.clone();
    settings.stream_url =
        crate::streaming::persistable_server(settings.stream_platform, &settings.stream_url);
    let json = serde_json::to_vec_pretty(&settings)
        .map_err(|error| format!("Profil kodlanamadı: {error}"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Profil klasörü oluşturulamadı: {error}"))?;
    }
    fs::write(path, json).map_err(|error| format!("Profil kaydedilemedi: {error}"))
}

pub fn load(path: &Path) -> Result<RecordingSettings, String> {
    let bytes = fs::read(path).map_err(|error| format!("Profil okunamadı: {error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("Profil dosyası geçersiz: {error}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn profile_round_trip_preserves_stream_server_but_never_the_stream_key() {
        let path = std::env::temp_dir().join(format!("ayrex-profile-{}.json", std::process::id()));
        let mut settings = crate::config::RecordingSettings::default();
        settings.stream_enabled = true;
        settings.stream_url = "rtmps://example.invalid/app/private-stream-key".to_owned();
        settings.noise_gate = true;
        crate::profiles::save(&path, &settings).expect("profile should save");
        let loaded = crate::profiles::load(&path).expect("profile should load");
        let contents = std::fs::read_to_string(&path).expect("profile file should be readable");
        assert!(loaded.stream_enabled && loaded.noise_gate);
        assert_eq!(loaded.stream_url, "rtmps://example.invalid/app");
        assert!(!contents.contains("private-stream-key"));
        let _ = std::fs::remove_file(path);
    }
}
