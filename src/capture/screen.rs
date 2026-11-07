use std::{
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
};

use xcap::{Monitor, Window};

#[derive(Clone, Debug)]
pub struct Display {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
    pub dxgi_output_index: Option<u32>,
}

impl Display {
    fn from_monitor(monitor: &Monitor) -> Result<Self, String> {
        let id = monitor.id().map_err(|error| error.to_string())?;
        Ok(Self {
            id,
            name: monitor.name().map_err(|error| error.to_string())?,
            x: monitor.x().map_err(|error| error.to_string())?,
            y: monitor.y().map_err(|error| error.to_string())?,
            width: monitor.width().map_err(|error| error.to_string())?,
            height: monitor.height().map_err(|error| error.to_string())?,
            primary: monitor.is_primary().map_err(|error| error.to_string())?,
            dxgi_output_index: dxgi_output_index(id),
        })
    }
}

#[cfg(windows)]
fn dxgi_output_index(monitor_id: u32) -> Option<u32> {
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};

    // FFmpeg's ddagrab uses the default D3D11 adapter. Match outputs on that
    // same adapter so its output_idx refers to the selected monitor.
    unsafe {
        let factory = CreateDXGIFactory1::<IDXGIFactory1>().ok()?;
        let adapter = factory.EnumAdapters1(0).ok()?;
        for output_index in 0..32 {
            let Ok(output) = adapter.EnumOutputs(output_index) else {
                break;
            };
            let Ok(description) = output.GetDesc() else {
                continue;
            };
            if description.Monitor.0 as usize as u32 == monitor_id {
                return Some(output_index);
            }
        }
    }
    None
}

#[cfg(not(windows))]
fn dxgi_output_index(_monitor_id: u32) -> Option<u32> {
    None
}

pub fn discover_displays() -> Result<Vec<Display>, String> {
    Monitor::all()
        .map_err(|error| format!("Could not enumerate monitors: {error}"))?
        .iter()
        .map(Display::from_monitor)
        .collect()
}

#[derive(Clone, Debug)]
pub struct WindowTarget {
    pub id: u32,
    pub title: String,
    pub app: String,
    pub width: u32,
    pub height: u32,
}

pub fn discover_windows() -> Result<Vec<WindowTarget>, String> {
    Window::all()
        .map_err(|error| error.to_string())
        .map(|windows| {
            windows
                .into_iter()
                .filter_map(|window| {
                    let title = window.title().ok()?.trim().to_owned();
                    let width = window.width().ok()?;
                    let height = window.height().ok()?;
                    if title.is_empty()
                        || width < 32
                        || height < 32
                        || window.is_minimized().ok()?
                    {
                        return None;
                    }
                    Some(WindowTarget {
                        id: window.id().ok()?,
                        title,
                        app: window.app_name().unwrap_or_default(),
                        width,
                        height,
                    })
                })
                .collect()
        })
}

pub struct PreviewFrame {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

enum PreviewMessage {
    Frame(PreviewFrame),
    Error(String),
}

pub struct PreviewCapture {
    receiver: Receiver<PreviewMessage>,
}

impl PreviewCapture {
    pub fn start(monitor_id: u32) -> Self {
        let (sender, receiver) = mpsc::sync_channel(1);
        thread::spawn(move || capture_once(monitor_id, sender));
        Self { receiver }
    }

    pub fn try_receive(&self) -> Result<Option<PreviewFrame>, String> {
        match self.receiver.try_recv() {
            Ok(PreviewMessage::Frame(frame)) => Ok(Some(frame)),
            Ok(PreviewMessage::Error(error)) => Err(error),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => {
                Err("Ekran yakalama işlemi beklenmedik biçimde durdu.".to_owned())
            }
        }
    }
}

fn capture_once(monitor_id: u32, sender: SyncSender<PreviewMessage>) {
    let capture_result = Monitor::all()
        .map_err(|error| error.to_string())
        .and_then(|monitors| {
            monitors
                .into_iter()
                .find(|monitor| monitor.id().ok() == Some(monitor_id))
                .ok_or_else(|| "Seçili ekran artık bulunamadı.".to_owned())
                .and_then(|monitor| monitor.capture_image().map_err(|error| error.to_string()))
        })
        .map(|image| PreviewFrame {
            width: image.width() as usize,
            height: image.height() as usize,
            pixels: image.into_raw(),
        });
    let message = match capture_result {
        Ok(frame) => PreviewMessage::Frame(frame),
        Err(error) => PreviewMessage::Error(error),
    };
    let _ = sender.send(message);
}
