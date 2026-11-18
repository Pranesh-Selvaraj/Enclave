//! Enclave desktop — a fully Rust shell over the shared `core-api`.
//!
//! This slice opens a vault, lists notes, proves that the desktop can consume
//! the core as a normal Rust crate (no C ABI), and edits a note through the
//! headless engine with autosave. IME and accessibility are the next gate.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod brand;
mod core_worker;
mod editor;

fn main() -> eframe::Result {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icons/icon.png"))
        .expect("the Enclave window icon");
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([680.0, 600.0])
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };

    eframe::run_native(
        "Enclave",
        options,
        Box::new(|cc| Ok(Box::new(app::EnclaveApp::new(cc)))),
    )
}
