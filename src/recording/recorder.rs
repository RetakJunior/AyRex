use std::{
    collections::VecDeque,
    fs,
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStderr, ChildStdout, ExitStatus},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::{
    audio::SystemAudioCapture,
    config::RecordingConfig,
    recording::{
        FfmpegProgress, encoder::build_ffmpeg_command_with_replay, output::unique_output_path,
    },
};

pub struct Recorder {
    child: Child,
    config: RecordingConfig,
    output_path: std::path::PathBuf,
    replay_directory: Option<std::path::PathBuf>,
    system_audio_capture: Option<SystemAudioCapture>,
    progress: Arc<Mutex<FfmpegProgress>>,
    stderr_tail: Arc<Mutex<VecDeque<String>>>,
    progress_thread: Option<JoinHandle<()>>,
    stderr_thread: Option<JoinHandle<()>>,
    warning: Option<String>,
}

impl Recorder {
    pub fn start(config: &RecordingConfig) -> Result<Self, String> {
        fs::create_dir_all(&config.output_dir)
            .map_err(|error| format!("Could not create output folder: {error}"))?;
        let output_path = unique_output_path(&config.output_dir, config.container)?;
        let replay_directory = if config.replay_buffer_seconds > 0 {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| error.to_string())?
                .as_nanos();
            let path = config.output_dir.join(format!(".ayrex_replay_{stamp}"));
            fs::create_dir(&path)
                .map_err(|error| format!("Could not create replay buffer: {error}"))?;
            Some(path)
        } else {
            None
        };
        #[cfg(windows)]
        let (system_audio_capture, system_audio_port) = if config.system_audio_enabled {
            let (capture, port) = SystemAudioCapture::start()?;
            (Some(capture), Some(port))
        } else {
            (None, None)
        };
        #[cfg(not(windows))]
        let (system_audio_capture, system_audio_port) = (None, None);
        let mut command = build_ffmpeg_command_with_replay(
            config,
            &output_path,
            replay_directory.as_deref(),
            system_audio_port,
        );
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                if let Some(path) = &replay_directory {
                    let _ = fs::remove_dir_all(path);
                }
                return Err(format!("Could not start FFmpeg: {error}"));
            }
        };

        let progress = Arc::new(Mutex::new(FfmpegProgress::default()));
        let stderr_tail = Arc::new(Mutex::new(VecDeque::new()));
        let progress_thread = child
            .stdout
            .take()
            .map(|stdout| read_progress(stdout, Arc::clone(&progress)));
        let stderr_thread = child
            .stderr
            .take()
            .map(|stderr| read_stderr(stderr, Arc::clone(&stderr_tail)));

        Ok(Self {
            child,
            config: config.clone(),
            output_path,
            replay_directory,
            system_audio_capture,
            progress,
            stderr_tail,
            progress_thread,
            stderr_thread,
            warning: None,
        })
    }

    pub fn config(&self) -> &RecordingConfig {
        &self.config
    }

    pub fn output_path(&self) -> &std::path::Path {
        &self.output_path
    }

    pub fn encoder(&self) -> &str {
        &self.config.encoder
    }

    pub fn audio_device(&self) -> Option<&str> {
        self.config.audio_device.as_deref()
    }

    pub fn elapsed(&self) -> Duration {
        self.progress()
            .map(|progress| progress.elapsed)
            .unwrap_or_default()
    }

    pub fn progress(&self) -> Result<std::sync::MutexGuard<'_, FfmpegProgress>, String> {
        self.progress.lock().map_err(|error| error.to_string())
    }

    pub fn poll_exit(&mut self) -> Result<Option<ExitStatus>, String> {
        self.child
            .try_wait()
            .map_err(|error| format!("Could not check FFmpeg: {error}"))
    }

    pub fn stop(mut self) -> Result<RecordingSummary, String> {
        if let Some(capture) = &self.system_audio_capture {
            capture.stop();
        }
        if let Some(mut stdin) = self.child.stdin.take() {
            stdin
                .write_all(b"q\n")
                .and_then(|()| stdin.flush())
                .map_err(|error| format!("Could not stop FFmpeg cleanly: {error}"))?;
        }
        let status = self
            .child
            .wait()
            .map_err(|error| format!("Could not wait for FFmpeg: {error}"))?;
        self.finish_readers()?;
        let summary = self.summary(status);
        if let Some(directory) = self.replay_directory.take() {
            let _ = fs::remove_dir_all(directory);
        }
        summary
    }

    pub fn finish_readers(&mut self) -> Result<(), String> {
        if let Some(mut capture) = self.system_audio_capture.take()
            && let Err(error) = capture.finish()
        {
            self.warning = Some(format!("Masaüstü sesi yakalama durdu: {error}"));
        }
        let progress_thread = self.progress_thread.take();
        let stderr_thread = self.stderr_thread.take();
        for handle in progress_thread.into_iter().chain(stderr_thread) {
            handle
                .join()
                .map_err(|_| "A FFmpeg output reader stopped unexpectedly.".to_owned())?;
        }
        Ok(())
    }

    pub fn summary(&self, status: ExitStatus) -> Result<RecordingSummary, String> {
        let progress = self.progress.lock().map_err(|error| error.to_string())?;
        let file_size = fs::metadata(&self.output_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        if !status.success() || file_size == 0 {
            let details = self.stderr_message();
            return Err(format!(
                "Recording failed ({}). {}",
                status,
                if details.is_empty() {
                    "FFmpeg did not produce a video file.".to_owned()
                } else {
                    details
                }
            ));
        }
        Ok(RecordingSummary {
            output_path: self.output_path.clone(),
            file_size,
            frames: progress.frames,
            elapsed: progress.elapsed,
            encoder: self.config.encoder.clone(),
            audio_device: self.config.audio_device.clone(),
            system_audio_enabled: self.config.system_audio_enabled,
            warning: self.warning.clone().or_else(|| self.stream_warning()),
        })
    }

    pub fn stderr_message(&self) -> String {
        let message = self
            .stderr_tail
            .lock()
            .map(|lines| lines.iter().cloned().collect::<Vec<_>>().join("\n"))
            .unwrap_or_else(|error| error.to_string());
        crate::streaming::redact_transport_urls(&message)
    }

    fn stream_warning(&self) -> Option<String> {
        self.stream_failed().then(|| {
            "Yayın bağlantısı kurulamadı veya kesildi; yerel kayıt yine de kaydedildi. Sunucu adresini ve yayın anahtarını kontrol et.".to_owned()
        })
    }

    pub fn stream_failed(&self) -> bool {
        let errors = self.stderr_message().to_ascii_lowercase();
        self.config.stream_url.is_some()
            && errors.contains("slave muxer")
            && (errors.contains("failed") || errors.contains("error"))
    }

    pub fn replay_buffered_seconds(&self) -> u32 {
        let Some(directory) = &self.replay_directory else {
            return 0;
        };
        read_replay_segments(directory)
            .iter()
            .map(|segment| segment.duration_seconds)
            .sum::<f64>()
            .floor()
            .min(self.config.replay_buffer_seconds as f64) as u32
    }

    pub fn save_replay(&self, seconds: u32) -> Result<std::path::PathBuf, String> {
        let directory = self
            .replay_directory
            .as_ref()
            .ok_or_else(|| "Replay Buffer kayıt başlamadan etkinleştirilmelidir.".to_owned())?;
        let mut segments = read_replay_segments(directory);
        if segments.is_empty() {
            return Err("Replay Buffer hazırlanıyor; birkaç saniye sonra tekrar dene.".to_owned());
        }
        let latest_segment_end = segments
            .last()
            .map(|segment| segment.end_seconds)
            .unwrap_or(0.0);
        let recording_time = self
            .progress()
            .map(|progress| progress.elapsed.as_secs_f64())
            .unwrap_or_default();
        if recording_time > latest_segment_end + 4.0 {
            return Err("Replay Buffer bölümleri güncellenmiyor; kaydı denetle.".to_owned());
        }

        let requested = seconds.clamp(5, 600) as f64;
        let mut duration = 0.0;
        let mut selected = Vec::new();
        for segment in segments.drain(..).rev() {
            let path = directory.join(&segment.filename);
            if !path.is_file() {
                continue;
            }
            duration += segment.duration_seconds;
            selected.push(segment.filename);
            if duration >= requested {
                break;
            }
        }
        selected.reverse();
        if selected.is_empty() {
            return Err("Replay Buffer henüz kullanılabilir bir klip üretmedi.".to_owned());
        }

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let scratch = directory.join(format!(".save_{stamp}"));
        fs::create_dir(&scratch)
            .map_err(|error| format!("Could not prepare replay clip: {error}"))?;
        let result = self.write_replay_clip(directory, &scratch, &selected);
        let _ = fs::remove_dir_all(&scratch);
        result
    }

    fn write_replay_clip(
        &self,
        replay_directory: &std::path::Path,
        scratch: &std::path::Path,
        segment_names: &[String],
    ) -> Result<std::path::PathBuf, String> {
        let mut manifest = String::from("ffconcat version 1.0\n");
        for (index, name) in segment_names.iter().enumerate() {
            let source = replay_directory.join(name);
            let staged_name = format!("segment_{index:04}.ts");
            fs::copy(&source, scratch.join(&staged_name))
                .map_err(|error| format!("Could not read a replay buffer segment: {error}"))?;
            manifest.push_str(&format!("file '{staged_name}'\n"));
        }
        fs::write(scratch.join("replay.ffconcat"), manifest)
            .map_err(|error| format!("Could not prepare replay timeline: {error}"))?;

        fs::create_dir_all(&self.config.output_dir)
            .map_err(|error| format!("Could not create output folder: {error}"))?;
        let output_directory = fs::canonicalize(&self.config.output_dir)
            .map_err(|error| format!("Could not resolve output folder: {error}"))?;
        let output_path = unique_output_path(&output_directory, self.config.container)?;
        let ffmpeg =
            fs::canonicalize(&self.config.ffmpeg).unwrap_or_else(|_| self.config.ffmpeg.clone());
        let mut command = std::process::Command::new(ffmpeg);
        command.args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            "replay.ffconcat",
            "-c",
            "copy",
        ]);
        if self.config.container == crate::config::Container::Mp4 {
            command.args(["-movflags", "+faststart"]);
        }
        let output = command
            .arg(&output_path)
            .current_dir(scratch)
            .output()
            .map_err(|error| format!("Could not save replay clip: {error}"))?;
        if !output.status.success() {
            let _ = fs::remove_file(&output_path);
            let details = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            return Err(if details.is_empty() {
                "FFmpeg could not combine the replay buffer segments.".to_owned()
            } else {
                format!("FFmpeg could not combine the replay buffer: {details}")
            });
        }
        let size = fs::metadata(&output_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        if size == 0 {
            let _ = fs::remove_file(&output_path);
            return Err("FFmpeg produced an empty replay clip.".to_owned());
        }
        Ok(output_path)
    }

    pub fn set_warning(&mut self, warning: String) {
        self.warning = Some(warning);
    }

    pub fn smoke_record(
        config: &RecordingConfig,
        duration: Duration,
    ) -> Result<RecordingSummary, String> {
        let recorder = Self::start(config)?;
        println!("Recording desktop for {} seconds...", duration.as_secs());
        thread::sleep(duration);
        recorder.stop()
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        if let Some(capture) = &self.system_audio_capture {
            capture.stop();
        }
        if self.child.try_wait().ok().flatten().is_none() {
            if let Some(mut stdin) = self.child.stdin.take() {
                let _ = stdin.write_all(b"q\n");
                let _ = stdin.flush();
            }
            let _ = self.child.wait();
        }
        let _ = self.finish_readers();
        if let Some(directory) = self.replay_directory.take() {
            let _ = fs::remove_dir_all(directory);
        }
    }
}

pub struct RecordingSummary {
    pub output_path: std::path::PathBuf,
    pub file_size: u64,
    pub frames: u64,
    pub elapsed: Duration,
    pub encoder: String,
    pub audio_device: Option<String>,
    pub system_audio_enabled: bool,
    pub warning: Option<String>,
}

struct ReplaySegment {
    filename: String,
    duration_seconds: f64,
    end_seconds: f64,
}

fn read_replay_segments(directory: &std::path::Path) -> Vec<ReplaySegment> {
    let Ok(contents) = fs::read_to_string(directory.join("segments.csv")) else {
        return Vec::new();
    };
    contents
        .lines()
        .filter_map(|line| {
            let mut columns = line.split(',');
            let filename = columns.next()?.trim();
            let start = columns.next()?.parse::<f64>().ok()?;
            let end = columns.next()?.parse::<f64>().ok()?;
            let safe_name = std::path::Path::new(filename)
                .file_name()?
                .to_string_lossy()
                .into_owned();
            let duration_seconds = end - start;
            (safe_name.starts_with("replay-")
                && safe_name.ends_with(".ts")
                && duration_seconds.is_finite()
                && duration_seconds > 0.0)
                .then_some(ReplaySegment {
                    filename: safe_name,
                    duration_seconds,
                    end_seconds: end,
                })
        })
        .collect()
}

fn read_progress(stdout: ChildStdout, progress: Arc<Mutex<FfmpegProgress>>) -> JoinHandle<()> {
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Ok(mut progress) = progress.lock() {
                progress.update(&line);
            }
        }
    })
}

fn read_stderr(stderr: ChildStderr, stderr_tail: Arc<Mutex<VecDeque<String>>>) -> JoinHandle<()> {
    thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if let Ok(mut lines) = stderr_tail.lock() {
                lines.push_back(line);
                while lines.len() > 30 {
                    lines.pop_front();
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_csv_ignores_incomplete_rows_and_keeps_chronological_durations() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be valid")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("AyRexReplayCsv-{stamp}"));
        fs::create_dir(&directory).expect("temporary replay folder should be created");
        fs::write(
            directory.join("segments.csv"),
            "replay-0000.ts,0.000000,1.000000\ntruncated\nreplay-0001.ts,1.000000,2.250000\n",
        )
        .expect("segment list should be written");

        let segments = read_replay_segments(&directory);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].filename, "replay-0000.ts");
        assert!((segments[0].duration_seconds - 1.0).abs() < f64::EPSILON);
        assert!((segments[1].duration_seconds - 1.25).abs() < f64::EPSILON);
        assert!((segments[1].end_seconds - 2.25).abs() < f64::EPSILON);

        fs::remove_dir_all(directory).expect("temporary replay folder should be removed");
    }
}
