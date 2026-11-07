mod screen;

#[cfg(target_os = "linux")]
mod wayland;

pub use screen::{
    Display, PreviewCapture, PreviewFrame, WindowTarget, discover_displays, discover_windows,
};

#[cfg(target_os = "linux")]
pub(crate) use wayland::{WaylandCapture, is_wayland_session};
