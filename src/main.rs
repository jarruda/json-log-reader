#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use pretty_env_logger;

fn main() -> eframe::Result<()> {
    use log::info;

    pretty_env_logger::init();

    info!("Starting!");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_maximized(true)
            .with_icon(
                // NOTE: Adding an icon is optional
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .unwrap(),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "JSON Log Reader",
        native_options,
        Box::new(|cc| Box::new(json_log_reader::TemplateApp::new(cc))),
    )
}
