//! main.rs — Native desktop entry point for EigenGraph.

use eframe::NativeOptions;
use egui::Vec2;
use eigengraph::app::EigenGraphApp;

fn main() -> eframe::Result {
    let mut native_options = NativeOptions::default();
    native_options.viewport = egui::ViewportBuilder::default()
        .with_title("EigenGraph — Desktop BIM & Structural Analytics")
        .with_inner_size(Vec2::new(1440.0, 900.0))
        .with_min_inner_size(Vec2::new(1024.0, 700.0))
        .with_transparent(false)
        .with_active(true);

    eframe::run_native(
        "EigenGraph",
        native_options,
        Box::new(|_cc| Ok(Box::new(EigenGraphApp::default()))),
    )
}
