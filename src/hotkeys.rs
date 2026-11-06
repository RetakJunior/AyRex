#[cfg(windows)]
use std::{
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyAction {
    StartRecording,
    StopRecording,
}

#[cfg(windows)]
pub struct GlobalHotkeys {
    receiver: Receiver<HotkeyAction>,
    thread_id: u32,
    worker: Option<JoinHandle<()>>,
}

#[cfg(windows)]
impl GlobalHotkeys {
    pub fn start() -> Result<Self, String> {
        use windows_sys::Win32::{
            Foundation::GetLastError,
            System::Threading::GetCurrentThreadId,
            UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey},
            UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY},
        };

        const START_ID: i32 = 1;
        const STOP_ID: i32 = 2;
        const MODIFIERS: u32 = 0x0002 | 0x0004 | 0x4000; // Ctrl, Shift, no key repeat

        let (sender, receiver) = mpsc::channel();
        let (ready_sender, ready_receiver) = mpsc::sync_channel::<Result<u32, String>>(1);
        let worker = thread::Builder::new()
            .name("AyRex global shortcuts".to_owned())
            .spawn(move || unsafe {
                let thread_id = GetCurrentThreadId();
                let start_registered =
                    RegisterHotKey(std::ptr::null_mut(), START_ID, MODIFIERS, u32::from(b'R')) != 0;
                if !start_registered {
                    let error = GetLastError();
                    let _ = ready_sender.send(Err(format!(
                        "Ctrl+Shift+R kısayolu kaydedilemedi (Windows hata {error})."
                    )));
                    return;
                }
                let stop_registered =
                    RegisterHotKey(std::ptr::null_mut(), STOP_ID, MODIFIERS, u32::from(b'S')) != 0;
                if !stop_registered {
                    let error = GetLastError();
                    UnregisterHotKey(std::ptr::null_mut(), START_ID);
                    let _ = ready_sender.send(Err(format!(
                        "Ctrl+Shift+S kısayolu kaydedilemedi (Windows hata {error})."
                    )));
                    return;
                }
                if ready_sender.send(Ok(thread_id)).is_err() {
                    UnregisterHotKey(std::ptr::null_mut(), START_ID);
                    UnregisterHotKey(std::ptr::null_mut(), STOP_ID);
                    return;
                }

                let mut message: MSG = std::mem::zeroed();
                loop {
                    let result = GetMessageW(&mut message, std::ptr::null_mut(), 0, 0);
                    if result <= 0 {
                        break;
                    }
                    if message.message == WM_HOTKEY {
                        let action = match message.wParam as i32 {
                            START_ID => Some(HotkeyAction::StartRecording),
                            STOP_ID => Some(HotkeyAction::StopRecording),
                            _ => None,
                        };
                        if let Some(action) = action
                            && sender.send(action).is_err()
                        {
                            break;
                        }
                    }
                }
                UnregisterHotKey(std::ptr::null_mut(), START_ID);
                UnregisterHotKey(std::ptr::null_mut(), STOP_ID);
            })
            .map_err(|error| format!("Global kısayol dinleyicisi başlatılamadı: {error}"))?;

        let thread_id = match ready_receiver.recv() {
            Ok(Ok(thread_id)) => thread_id,
            Ok(Err(error)) => {
                let _ = worker.join();
                return Err(error);
            }
            Err(error) => {
                let _ = worker.join();
                return Err(format!("Global kısayol dinleyicisi yanıt vermedi: {error}"));
            }
        };
        Ok(Self {
            receiver,
            thread_id,
            worker: Some(worker),
        })
    }

    pub fn try_next(&self) -> Option<HotkeyAction> {
        self.receiver.try_recv().ok()
    }
}

#[cfg(windows)]
impl Drop for GlobalHotkeys {
    fn drop(&mut self) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};
        let posted = unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0) } != 0;
        if posted && let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(not(windows))]
pub struct GlobalHotkeys;

#[cfg(not(windows))]
impl GlobalHotkeys {
    pub fn start() -> Result<Self, String> {
        Err("Global hotkeys are only available on Windows.".to_owned())
    }

    pub fn try_next(&self) -> Option<HotkeyAction> {
        None
    }
}

pub fn detect(ctrl: bool, shift: bool, start: bool, stop: bool) -> Option<HotkeyAction> {
    if !ctrl || !shift {
        return None;
    }
    if start {
        Some(HotkeyAction::StartRecording)
    } else if stop {
        Some(HotkeyAction::StopRecording)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_require_ctrl_shift_and_choose_the_pressed_action() {
        assert_eq!(
            detect(true, true, true, false),
            Some(HotkeyAction::StartRecording)
        );
        assert_eq!(
            detect(true, true, false, true),
            Some(HotkeyAction::StopRecording)
        );
        assert_eq!(detect(true, false, true, false), None);
    }
}
