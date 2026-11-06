use std::{
    collections::VecDeque,
    fs,
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStderr, ChildStdout, ExitStatus},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    config::RecordingConfig,
    recording::{FfmpegProgress, encoder::build_ffmpeg_command, output::unique_output_path},
};

pub struct Recorder {
    child: Child,
    config: RecordingConfig,
    output_path: std::path::PathBuf,
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
        let mut child = build_ffmpeg_command(config, &output_path)
            .spawn()
            .map_err(|error| format!("Could not start FFmpeg: {error}"))?;

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
        self.summary(status)
    }

    pub fn finish_readers(&mut self) -> Result<(), String> {
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
            warning: self.warning.clone(),
        })
    }

    pub fn stderr_message(&self) -> String {
        self.stderr_tail
            .lock()
            .map(|lines| lines.iter().cloned().collect::<Vec<_>>().join("\n"))
            .unwrap_or_else(|error| error.to_string())
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
        if self.child.try_wait().ok().flatten().is_none() {
            if let Some(mut stdin) = self.child.stdin.take() {
                let _ = stdin.write_all(b"q\n");
                let _ = stdin.flush();
            }
            let _ = self.child.wait();
            let _ = self.finish_readers();
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
    pub warning: Option<String>,
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
