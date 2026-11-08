use crate::config::RecordingConfig;

pub fn video_filter(config: &RecordingConfig) -> String {
    let mut filters = vec![format!(
        "scale={}:{}:flags=bilinear",
        config.width, config.height
    )];

    let has_eq = config.brightness.abs() > 0.001
        || (config.contrast - 1.0).abs() > 0.001
        || (config.saturation - 1.0).abs() > 0.001;
    if has_eq {
        filters.push(format!(
            "eq=brightness={:.3}:contrast={:.3}:saturation={:.3}",
            config.brightness.clamp(-0.5, 0.5),
            config.contrast.clamp(0.0, 2.0),
            config.saturation.clamp(0.0, 3.0)
        ));
    }

    if config.sharpen > 0.01 {
        filters.push(format!(
            "unsharp=5:5:{:.2}:5:5:0",
            config.sharpen.clamp(0.0, 2.0)
        ));
    }

    filters.join(",")
}

pub fn audio_filter(config: &RecordingConfig) -> String {
    let mut filters = vec![format!("volume={:.2}", config.audio_volume)];
    if config.noise_suppression {
        filters.push("afftdn=nf=-25".to_owned());
    }
    if config.noise_gate {
        filters.push("agate=threshold=0.02:ratio=4:attack=20:release=250".to_owned());
    }
    if config.compressor {
        filters.push("acompressor=threshold=0.125:ratio=3:attack=20:release=250".to_owned());
    }
    if config.limiter {
        filters.push("alimiter=limit=0.95".to_owned());
    }
    filters.join(",")
}
