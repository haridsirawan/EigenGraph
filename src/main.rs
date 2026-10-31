//! main.rs — Native desktop entry point for EigenGraph.

use eframe::NativeOptions;
use egui::Vec2;
use eigengraph::app::EigenGraphApp;

fn main() -> eframe::Result {
    let mut native_options = NativeOptions::default();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("EigenGraph — Desktop BIM & Structural Analytics")
        .with_inner_size(Vec2::new(1440.0, 900.0))
        .with_min_inner_size(Vec2::new(1024.0, 700.0))
        .with_transparent(false)
        .with_fullsize_content_view(true)
        .with_titlebar_shown(false)
        .with_title_shown(false)
        .with_active(true);

    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")) {
        viewport = viewport.with_icon(icon);
    }

    native_options.viewport = viewport;

    eframe::run_native(
        "EigenGraph",
        native_options,
        Box::new(|_cc| Ok(Box::new(EigenGraphApp::default()))),
    )
}
