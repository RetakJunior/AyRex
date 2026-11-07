mod encoder;
mod filters;
mod output;
mod progress;
mod recorder;

pub(crate) use encoder::RawVideoInput;
pub use encoder::{encoder_label, is_hardware_encoder};
pub use filters::{audio_filter, video_filter};
pub use output::{format_bytes, format_duration};
pub use progress::FfmpegProgress;
pub use recorder::{Recorder, RecordingSummary};

#[cfg(test)]
pub use encoder::build_ffmpeg_command;
