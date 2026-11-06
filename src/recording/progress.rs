use std::time::Duration;

#[derive(Default, Debug)]
pub struct FfmpegProgress {
    pub frames: u64,
    pub fps: Option<f32>,
    pub dropped_frames: Option<u64>,
    pub elapsed: Duration,
}

impl FfmpegProgress {
    pub fn update(&mut self, line: &str) {
        let Some((key, value)) = line.split_once('=') else {
            return;
        };
        match key {
            "frame" => self.frames = value.parse().unwrap_or(self.frames),
            "fps" => self.fps = value.parse().ok(),
            "drop_frames" => self.dropped_frames = value.parse().ok(),
            "out_time_us" => {
                if let Ok(micros) = value.parse::<u64>() {
                    self.elapsed = Duration::from_micros(micros);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ffmpeg_progress_values() {
        let mut progress = FfmpegProgress::default();
        progress.update("frame=900");
        progress.update("fps=29.97");
        progress.update("drop_frames=2");
        progress.update("out_time_us=30000000");

        assert_eq!(progress.frames, 900);
        assert_eq!(progress.fps, Some(29.97));
        assert_eq!(progress.dropped_frames, Some(2));
        assert_eq!(progress.elapsed, Duration::from_secs(30));
    }
}
