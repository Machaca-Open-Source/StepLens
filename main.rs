mod app;
mod models;
mod capture;
mod hooks;
mod storage;
mod llm;
mod export;
mod ui;

use anyhow::Result;
use app::CapturerApp;

fn main() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Capturer - Step Recorder"),
        // Enable always-on-top for overlay window
        ..Default::default()
    };

    eframe::run_native(
        "Capturer",
        options,
        Box::new(|_cc| {
            match CapturerApp::new() {
                Ok(app) => Box::new(app),
                Err(e) => {
                    eprintln!("Failed to initialize app: {}", e);
                    std::process::exit(1);
                }
            }
        }),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run app: {}", e))?;

    Ok(())
}
