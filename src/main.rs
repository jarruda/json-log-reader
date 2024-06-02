#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use pretty_env_logger;
use log::info;

fn main() -> eframe::Result<()> {
    pretty_env_logger::init();
    info!("Starting!");

    start_puffin_server();
    puffin::set_scopes_on(true);

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

fn start_puffin_server() {
    match puffin_http::Server::new("127.0.0.1:8585") {
        Ok(puffin_server) => {
            // We can store the server if we want, but in this case we just want
            // it to keep running. Dropping it closes the server, so let's not drop it!
            #[allow(clippy::mem_forget)]
            std::mem::forget(puffin_server);
            info!("Started puffin server on localhost:8585");
        }
        Err(err) => {
            eprintln!("Failed to start puffin server: {err}");
        }
    };
}