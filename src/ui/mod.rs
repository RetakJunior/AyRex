mod dashboard;
mod preview;
mod recording_controls;
mod settings;
mod status_panel;

pub use dashboard::show_dashboard;

pub(crate) use preview::show_preview;
pub(crate) use preview::update_preview;
pub(crate) use recording_controls::show_recording_controls;
pub(crate) use settings::{show_display_selector, show_settings};
pub(crate) use status_panel::show_status;
