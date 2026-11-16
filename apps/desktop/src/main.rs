//! Enclave desktop — a fully Rust shell over the shared `core-api`.
//!
//! This slice opens a vault, lists notes, proves that the desktop can consume
//! the core as a normal Rust crate (no C ABI), and edits a note through the
//! headless engine with autosave. IME and accessibility are the next gate.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod core_worker;
mod editor;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([680.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Enclave",
        options,
        Box::new(|cc| Ok(Box::new(app::EnclaveApp::new(cc)))),
    )
}
