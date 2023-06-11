#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use pretty_env_logger;

fn main() -> eframe::Result<()> {
    use log::info;

    pretty_env_logger::init();

    info!("Starting!");

    // Log to stdout (if you run with `RUST_LOG=debug`).
    // tracing_subscriber::fmt::init();

    let mut native_options = eframe::NativeOptions::default();
    native_options.maximized = true;
    eframe::run_native(
        "JSON Log Reader",
        native_options,
        Box::new(|cc| Box::new(json_log_reader::TemplateApp::new(cc))),
    )
}
