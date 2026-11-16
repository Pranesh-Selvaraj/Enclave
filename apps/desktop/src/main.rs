//! Enclave desktop — a fully Rust shell over the shared `core-api`.
//!
//! This first slice deliberately stops at the foundation: the app opens a
//! vault, lists notes and proves that the desktop can consume the core as a
//! normal Rust crate (no C ABI). The headless editor engine is the next gate;
//! the placeholder editor in `app.rs` will be replaced by it.

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
