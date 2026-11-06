pub mod app;
pub mod audio;
pub mod capture;
pub mod config;
pub mod hotkeys;
pub mod performance;
pub mod profiles;
pub mod recording;
pub mod state;
pub mod streaming;
pub mod system;
pub mod ui;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mode = args.next();
    match mode.as_deref() {
        Some("--smoke-record")
        | Some("--smoke-region")
        | Some("--smoke-system-audio")
        | Some("--smoke-replay") => {
            let seconds = args
                .next()
                .map(|value| value.parse::<u64>())
                .transpose()?
                .unwrap_or(30);
            if mode.as_deref() == Some("--smoke-record") {
                app::run_smoke_record(seconds)
            } else if mode.as_deref() == Some("--smoke-region") {
                app::run_smoke_region(seconds)
            } else if mode.as_deref() == Some("--smoke-system-audio") {
                app::run_smoke_system_audio(seconds)
            } else {
                app::run_smoke_replay(seconds)
            }
            .map_err(std::io::Error::other)?;
            return Ok(());
        }
        Some("--check-hotkeys") => {
            app::check_global_hotkeys().map_err(std::io::Error::other)?;
            return Ok(());
        }
        _ => {}
    }

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    let options = eframe::NativeOptions {
        viewport: options.viewport.with_icon(config::window_icon()?),
        ..options
    };

    eframe::run_native(
        "AyRex",
        options,
        Box::new(|cc| Ok(Box::new(app::AyRexApp::new(cc)))),
    )?;
    Ok(())
}
