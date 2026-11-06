pub mod app;
pub mod audio;
pub mod capture;
pub mod config;
pub mod recording;
pub mod state;
pub mod system;
pub mod ui;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("--smoke-record") {
        let seconds = args
            .next()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(30);
        app::run_smoke_record(seconds).map_err(std::io::Error::other)?;
        return Ok(());
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
