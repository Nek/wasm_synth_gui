#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use crate::main;
    use wasm_bindgen::prelude::*;

    // Prevent `wasm_bindgen` from autostarting main on all spawned threads
    #[wasm_bindgen(start)]
    pub fn dummy_main() {}

    // Export explicit run function to start main
    #[wasm_bindgen]
    pub fn run() {
        main();
    }
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "wasm synth gui",
        native_options,
        Box::new(|cc| Box::new(wasm_synth_gui::TemplateApp::new(cc))),
    );
}

// when compiling to web using trunk.
#[cfg(target_arch = "wasm32")]
fn main() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    let web_options = eframe::WebOptions::default();
    eframe::start_web(
        "canv",
        web_options,
        Box::new(|cc| Box::new(wasm_synth_gui::TemplateApp::new(cc))),
    )
    .expect("failed to start eframe");

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();
}
