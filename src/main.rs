#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use pretty_env_logger;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
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

// when compiling to web using trunk.
#[cfg(target_arch = "wasm32")]
fn main() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::start_web(
            "the_canvas_id", // hardcode it
            web_options,
            Box::new(|cc| Box::new(json_log_reader::TemplateApp::new(cc))),
        )
        .await
        .expect("failed to start eframe");
    });
}
