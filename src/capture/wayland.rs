use std::{
    io::Write,
    process::ChildStdin,
    sync::mpsc::{self, SyncSender, TryRecvError},
    thread::{self, JoinHandle},
};

use scap::{
    capturer::{Capturer, Options},
    frame::{Frame, VideoFrame},
};

use crate::recording::RawVideoInput;

pub(crate) fn is_wayland_session() -> bool {
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|value| value.eq_ignore_ascii_case("wayland"))
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

pub(crate) struct WaylandCapture {
    capture: Capturer,
    first_frame: Vec<u8>,
    input: RawVideoInput,
}

impl WaylandCapture {
    pub(crate) fn start(fps: u32) -> Result<Self, String> {
        let capture = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Capturer::build(Options {
                fps: fps.max(1),
                show_cursor: true,
                ..Options::default()
            })
        }))
        .map_err(|_| {
            "Ubuntu ekran paylaşımı başlatılamadı. İzin penceresinde ekran seçip paylaşımı onaylayın."
                .to_owned()
        })?
        .map_err(|error| format!("Ubuntu ekran paylaşımı başlatılamadı: {error}"))?;

        let mut capture = capture;
        capture.start_capture();
        let frame = match capture.get_next_frame() {
            Ok(frame) => frame,
            Err(error) => {
                capture.stop_capture();
                return Err(format!("PipeWire'dan ekran karesi alınamadı: {error}"));
            }
        };
        let (input, first_frame) = match raw_video_frame(frame) {
            Ok(frame) => frame,
            Err(error) => {
                capture.stop_capture();
                return Err(error);
            }
        };

        Ok(Self {
            capture,
            first_frame,
            input,
        })
    }

    pub(crate) fn input(&self) -> RawVideoInput {
        self.input.clone()
    }

    pub(crate) fn cancel(mut self) {
        self.capture.stop_capture();
    }

    pub(crate) fn pipe_frames(
        self,
        stdin: ChildStdin,
    ) -> (SyncSender<()>, JoinHandle<Result<(), String>>) {
        let (stop_sender, stop_receiver) = mpsc::sync_channel(1);
        let handle = thread::spawn(move || {
            let Self {
                mut capture,
                first_frame,
                input,
            } = self;
            let mut stdin = stdin;

            let result = (|| {
                stdin
                    .write_all(&first_frame)
                    .map_err(|error| format!("FFmpeg'e ekran karesi gönderilemedi: {error}"))?;

                loop {
                    let frame = capture
                        .get_next_frame()
                        .map_err(|error| format!("PipeWire ekran akışı kapandı: {error}"))?;

                    match stop_receiver.try_recv() {
                        Ok(()) | Err(TryRecvError::Disconnected) => break,
                        Err(TryRecvError::Empty) => {}
                    }

                    let (next_input, data) = raw_video_frame(frame)?;
                    if next_input.width != input.width
                        || next_input.height != input.height
                        || next_input.pixel_format != input.pixel_format
                    {
                        return Err("Ekran kayıt sırasında boyut veya biçim değiştirdi.".to_owned());
                    }
                    stdin
                        .write_all(&data)
                        .map_err(|error| format!("FFmpeg'e ekran karesi gönderilemedi: {error}"))?;
                }
                Ok(())
            })();

            capture.stop_capture();
            drop(stdin);
            result
        });
        (stop_sender, handle)
    }
}

fn raw_video_frame(frame: Frame) -> Result<(RawVideoInput, Vec<u8>), String> {
    let (width, height, data) = match frame {
        Frame::Video(VideoFrame::BGRx(frame)) => (frame.width, frame.height, frame.data),
        Frame::Video(VideoFrame::BGR0(frame)) => (frame.width, frame.height, frame.data),
        _ => return Err("PipeWire beklenmeyen bir piksel biçimi gönderdi.".to_owned()),
    };
    let width = u32::try_from(width).map_err(|_| "Ekran genişliği geçersiz.".to_owned())?;
    let height = u32::try_from(height).map_err(|_| "Ekran yüksekliği geçersiz.".to_owned())?;
    let expected_size = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "Ekran çözünürlüğü çok büyük.".to_owned())?;
    if width == 0 || height == 0 || data.len() != expected_size {
        return Err("PipeWire eksik veya geçersiz bir ekran karesi gönderdi.".to_owned());
    }

    Ok((
        RawVideoInput {
            width,
            height,
            pixel_format: "bgr0",
        },
        data,
    ))
}
