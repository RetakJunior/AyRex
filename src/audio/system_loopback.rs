#[cfg(target_os = "windows")]
use std::{
    net::{TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[cfg(target_os = "windows")]
pub struct SystemAudioCapture {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<(), String>>>,
}

#[cfg(not(target_os = "windows"))]
pub struct SystemAudioCapture;

impl SystemAudioCapture {
    #[cfg(target_os = "windows")]
    pub fn start() -> Result<(Self, u16), String> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|error| format!("Sistem sesi için yerel ses hattı açılamadı: {error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("Sistem ses hattı yapılandırılamadı: {error}"))?;
        let port = listener
            .local_addr()
            .map_err(|error| format!("Sistem ses hattı adresi okunamadı: {error}"))?
            .port();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("AyRex WASAPI loopback".to_owned())
            .spawn(move || capture_worker(listener, worker_stop, ready_tx))
            .map_err(|error| format!("Sistem sesi yakalama iş parçacığı başlatılamadı: {error}"))?;

        match ready_rx.recv_timeout(Duration::from_secs(12)) {
            Ok(Ok(())) => Ok((
                Self {
                    stop,
                    worker: Some(worker),
                },
                port,
            )),
            Ok(Err(error)) => {
                let _ = worker.join();
                Err(error)
            }
            Err(error) => {
                stop.store(true, Ordering::Release);
                let _ = worker.join();
                Err(format!("WASAPI masaüstü sesi başlatılamadı: {error}"))
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn start() -> Result<(Self, u16), String> {
        Err("Masaüstü sesi yakalama şu anda Windows WASAPI gerektiriyor.".to_owned())
    }

    pub fn stop(&self) {
        #[cfg(target_os = "windows")]
        self.stop.store(true, Ordering::Release);
    }

    pub fn finish(&mut self) -> Result<(), String> {
        self.stop();
        #[cfg(target_os = "windows")]
        if let Some(worker) = self.worker.take() {
            return worker.join().map_err(|_| {
                "WASAPI ses yakalama iş parçacığı beklenmedik biçimde durdu.".to_owned()
            })?;
        }
        Ok(())
    }
}

impl Drop for SystemAudioCapture {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

#[cfg(target_os = "windows")]
fn capture_worker(
    listener: TcpListener,
    stop: Arc<AtomicBool>,
    ready: mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    if let Err(error) = wasapi::initialize_mta().ok() {
        let message = format!("Windows ses oturumu başlatılamadı: {error}");
        let _ = ready.send(Err(message.clone()));
        return Err(message);
    }
    let result = capture_on_initialized_thread(listener, stop, ready);
    wasapi::deinitialize();
    result
}

#[cfg(target_os = "windows")]
fn capture_on_initialized_thread(
    listener: TcpListener,
    stop: Arc<AtomicBool>,
    ready: mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    use wasapi::{DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};

    let setup = (|| {
        let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
        let device = enumerator
            .get_default_device(&Direction::Render)
            .map_err(|error| format!("Varsayılan Windows ses çıkışı bulunamadı: {error}"))?;
        let mut client = device
            .get_iaudioclient()
            .map_err(|error| format!("Windows ses çıkışı açılamadı: {error}"))?;
        let (device_period, _) = client
            .get_device_period()
            .map_err(|error| format!("Windows ses çıkışı süresi okunamadı: {error}"))?;
        let format = WaveFormat::new(32, 32, &SampleType::Float, 48_000, 2, None);
        let mode = StreamMode::EventsShared {
            autoconvert: true,
            buffer_duration_hns: device_period.max(100_000),
        };
        client
            .initialize_client(&format, &Direction::Capture, &mode)
            .map_err(|error| format!("WASAPI loopback akışı hazırlanamadı: {error}"))?;
        let event = client
            .set_get_eventhandle()
            .map_err(|error| format!("WASAPI ses olayı açılamadı: {error}"))?;
        let capture = client
            .get_audiocaptureclient()
            .map_err(|error| format!("WASAPI ses okuyucusu açılamadı: {error}"))?;
        Ok::<_, String>((client, event, capture))
    })();

    let (client, event, capture) = match setup {
        Ok(session) => session,
        Err(error) => {
            let _ = ready.send(Err(format!(
                "Masaüstü sesi yakalama başlatılamadı: {error}"
            )));
            return Err(error);
        }
    };
    let _ = ready.send(Ok(()));

    let mut stream = loop {
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(format!("FFmpeg ses hattına bağlanamadı: {error}")),
        }
    };
    stream
        .set_nodelay(true)
        .map_err(|error| format!("FFmpeg ses hattı gecikmesi ayarlanamadı: {error}"))?;
    client
        .start_stream()
        .map_err(|error| format!("WASAPI loopback kaydı başlatılamadı: {error}"))?;

    const BYTES_PER_FRAME: usize = 8; // 48 kHz, stereo, 32-bit float
    const SILENT_FRAMES_PER_TICK: usize = 480; // 10 ms
    while !stop.load(Ordering::Acquire) {
        let mut wrote_audio = false;
        match event.wait_for_event(10) {
            Ok(()) => {
                while let Some(frame_count) = capture
                    .get_next_packet_size()
                    .map_err(|error| format!("WASAPI ses paketi okunamadı: {error}"))?
                {
                    if frame_count == 0 {
                        break;
                    }
                    let mut bytes = vec![0; frame_count as usize * BYTES_PER_FRAME];
                    let (read_frames, info) = capture
                        .read_from_device(&mut bytes)
                        .map_err(|error| format!("Masaüstü sesi okunamadı: {error}"))?;
                    if info.flags.silent {
                        bytes.fill(0);
                    } else {
                        bytes.truncate(read_frames as usize * BYTES_PER_FRAME);
                    }
                    write_audio(&mut stream, &bytes, &stop)?;
                    wrote_audio |= read_frames > 0;
                }
            }
            Err(wasapi::WasapiError::EventTimeout) => {}
            Err(error) => return Err(format!("WASAPI ses olayı beklenemedi: {error}")),
        }
        if !wrote_audio {
            write_audio(
                &mut stream,
                &[0; SILENT_FRAMES_PER_TICK * BYTES_PER_FRAME],
                &stop,
            )?;
        }
    }
    let _ = client.stop_stream();
    Ok(())
}

#[cfg(target_os = "windows")]
fn write_audio(stream: &mut TcpStream, bytes: &[u8], stop: &AtomicBool) -> Result<(), String> {
    if let Err(error) = std::io::Write::write_all(stream, bytes) {
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        return Err(format!("FFmpeg masaüstü ses hattı kapandı: {error}"));
    }
    Ok(())
}
