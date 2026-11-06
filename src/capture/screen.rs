use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread,
    time::Duration,
};

use xcap::Monitor;

#[derive(Clone, Debug)]
pub struct Display {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
}

impl Display {
    fn from_monitor(monitor: &Monitor) -> Result<Self, String> {
        Ok(Self {
            id: monitor.id().map_err(|error| error.to_string())?,
            name: monitor
                .friendly_name()
                .or_else(|_| monitor.name())
                .map_err(|error| error.to_string())?,
            x: monitor.x().map_err(|error| error.to_string())?,
            y: monitor.y().map_err(|error| error.to_string())?,
            width: monitor.width().map_err(|error| error.to_string())?,
            height: monitor.height().map_err(|error| error.to_string())?,
            primary: monitor.is_primary().map_err(|error| error.to_string())?,
        })
    }
}

pub fn discover_displays() -> Result<Vec<Display>, String> {
    Monitor::all()
        .map_err(|error| format!("Could not enumerate monitors: {error}"))?
        .iter()
        .map(Display::from_monitor)
        .collect()
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
    selected_monitor: Arc<AtomicU32>,
    running: Arc<AtomicBool>,
    receiver: Receiver<PreviewMessage>,
}

impl PreviewCapture {
    pub fn start(monitor_id: u32) -> Self {
        let selected_monitor = Arc::new(AtomicU32::new(monitor_id));
        let running = Arc::new(AtomicBool::new(true));
        let (sender, receiver) = mpsc::sync_channel(1);
        thread::spawn({
            let selected_monitor = Arc::clone(&selected_monitor);
            let running = Arc::clone(&running);
            move || preview_loop(selected_monitor, running, sender)
        });
        Self {
            selected_monitor,
            running,
            receiver,
        }
    }

    pub fn select_monitor(&self, monitor_id: u32) {
        self.selected_monitor.store(monitor_id, Ordering::Relaxed);
    }

    pub fn try_receive(&self) -> Result<Option<PreviewFrame>, String> {
        let mut latest = None;
        while let Ok(message) = self.receiver.try_recv() {
            match message {
                PreviewMessage::Frame(frame) => latest = Some(frame),
                PreviewMessage::Error(error) => return Err(error),
            }
        }
        Ok(latest)
    }
}

impl Drop for PreviewCapture {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

fn preview_loop(
    selected_monitor: Arc<AtomicU32>,
    running: Arc<AtomicBool>,
    sender: SyncSender<PreviewMessage>,
) {
    let mut last_error = String::new();
    let monitors = match Monitor::all() {
        Ok(monitors) => monitors,
        Err(error) => {
            let _ = sender.try_send(PreviewMessage::Error(error.to_string()));
            return;
        }
    };

    while running.load(Ordering::Relaxed) {
        let selected_id = selected_monitor.load(Ordering::Relaxed);
        let capture_result = monitors
            .iter()
            .find(|monitor| monitor.id().ok() == Some(selected_id))
            .ok_or_else(|| "Selected monitor is no longer available.".to_owned())
            .and_then(|monitor| monitor.capture_image().map_err(|error| error.to_string()))
            .map(|image| PreviewFrame {
                width: image.width() as usize,
                height: image.height() as usize,
                pixels: image.into_raw(),
            });

        match capture_result {
            Ok(frame) => {
                last_error.clear();
                if let Err(TrySendError::Disconnected(_)) =
                    sender.try_send(PreviewMessage::Frame(frame))
                {
                    break;
                }
            }
            Err(error) if error != last_error => {
                last_error = error.clone();
                let _ = sender.try_send(PreviewMessage::Error(error));
            }
            Err(_) => {}
        }
        thread::sleep(Duration::from_millis(150));
    }
}
