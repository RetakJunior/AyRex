use std::{
    io::Write,
    process::ChildStdin,
    sync::mpsc::{self, SyncSender, TryRecvError},
    thread::{self, JoinHandle},
    time::Duration,
};

use scrcap::{
    CaptureConfig, CaptureDesc, CaptureDescriptor, Target, VideoConfig, frame::VideoFrame,
};

use crate::recording::RawVideoInput;

pub(crate) fn is_wayland_session() -> bool {
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|value| value.eq_ignore_ascii_case("wayland"))
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

pub(crate) struct WaylandCapture {
    capture: CaptureDesc,
    first_frame: Vec<u8>,
    input: RawVideoInput,
}

impl WaylandCapture {
    pub(crate) fn start(fps: u32) -> Result<Self, String> {
        let capture = CaptureConfig {
            video: VideoConfig {
                channel_capacity: 2,
                hide: Vec::new(),
                target: Target::Primary,
                fps: Some(fps.max(1)),
            },
            audio: None,
        }
        .create()
        .map_err(|error| format!("Ubuntu ekran paylaşımı başlatılamadı: {error}"))?;
        let frame = capture
            .video()
            .recv_timeout(Duration::from_secs(15))
            .map_err(|error| format!("PipeWire'dan ekran karesi alınamadı: {error}"))?;
        let (input, first_frame) = raw_video_frame(frame)?;

        Ok(Self {
            capture,
            first_frame,
            input,
        })
    }

    pub(crate) fn input(&self) -> RawVideoInput {
        self.input.clone()
    }

    pub(crate) fn cancel(self) {
        self.capture.terminate();
    }

    pub(crate) fn pipe_frames(
        self,
        stdin: ChildStdin,
    ) -> (SyncSender<()>, JoinHandle<Result<(), String>>) {
        let (stop_sender, stop_receiver) = mpsc::sync_channel(1);
        let handle = thread::spawn(move || {
            let capture = self.capture;
            let mut stdin = stdin;
            let input = self.input;

            let result = (|| {
                stdin
                    .write_all(&self.first_frame)
                    .map_err(|error| format!("FFmpeg'e ekran karesi gönderilemedi: {error}"))?;

                loop {
                    match stop_receiver.try_recv() {
                        Ok(()) | Err(TryRecvError::Disconnected) => break,
                        Err(TryRecvError::Empty) => {}
                    }

                    match capture.video().recv_timeout(Duration::from_millis(100)) {
                        Ok(frame) => {
                            let (next_input, data) = raw_video_frame(frame)?;
                            if next_input.width != input.width
                                || next_input.height != input.height
                                || next_input.pixel_format != input.pixel_format
                            {
                                return Err(
                                    "Ekran kayıt sırasında boyut veya biçim değiştirdi.".to_owned()
                                );
                            }
                            stdin.write_all(&data).map_err(|error| {
                                format!("FFmpeg'e ekran karesi gönderilemedi: {error}")
                            })?;
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                            return Err(
                                "PipeWire ekran akışı beklenmedik biçimde kapandı.".to_owned()
                            );
                        }
                    }
                }
                Ok(())
            })();

            capture.terminate();
            drop(stdin);
            result
        });
        (stop_sender, handle)
    }
}

fn raw_video_frame(frame: VideoFrame) -> Result<(RawVideoInput, Vec<u8>), String> {
    use scrcap::PixFmt;

    let pixel_format = match frame.pix_fmt {
        PixFmt::Bgra => "bgra",
        PixFmt::Rgba => "rgba",
        PixFmt::Argb => "argb",
        PixFmt::Abgr => "abgr",
        PixFmt::Bgr0 => "bgr0",
        PixFmt::Rgb0 => "rgb0",
        _ => return Err("PipeWire beklenmeyen bir piksel biçimi gönderdi.".to_owned()),
    };
    let (width, height) = frame.size;
    let expected_size = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(frame.pix_fmt.bytes_per_pixel()))
        .ok_or_else(|| "Ekran çözünürlüğü çok büyük.".to_owned())?;
    if width == 0 || height == 0 || frame.vframe.len() != expected_size {
        return Err("PipeWire eksik veya geçersiz bir ekran karesi gönderdi.".to_owned());
    }

    Ok((
        RawVideoInput {
            width,
            height,
            pixel_format,
        },
        frame.vframe,
    ))
}
