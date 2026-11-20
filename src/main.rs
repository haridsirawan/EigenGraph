//! main.rs — Cross-platform entry point for EigenGraph (Native Desktop & WebAssembly).

use eigengraph::app::EigenGraphApp;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    use eframe::NativeOptions;
    use egui::Vec2;

    let mut native_options = NativeOptions::default();
    // Phase 65: Request 24-bit hardware Z-buffer for WebGL2 / OpenGL 3D Vector Earth rendering
    native_options.depth_buffer = 24;
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("EigenGraph")
        .with_inner_size(Vec2::new(1440.0, 900.0))
        .with_min_inner_size(Vec2::new(360.0, 600.0))
        .with_transparent(false)
        .with_active(true);

    #[cfg(target_os = "macos")]
    {
        viewport = viewport
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false);
    }

    #[cfg(not(target_os = "macos"))]
    {
        viewport = viewport.with_decorations(true);
    }

    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")) {
        viewport = viewport.with_icon(icon);
    }

    native_options.viewport = viewport;

    eframe::run_native(
        "EigenGraph",
        native_options,
        Box::new(|cc| Ok(Box::new(EigenGraphApp::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast as _;

    eframe::WebLogger::init(log::LevelFilter::Info).ok();

    let mut web_options = eframe::WebOptions::default();
    web_options.depth_buffer = 24;

    wasm_bindgen_futures::spawn_local(async {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        let Some(canvas_el) = document.get_element_by_id("eigengraph_canvas") else {
            log::error!("Missing #eigengraph_canvas element in index.html");
            return;
        };
        let Ok(canvas) = canvas_el.dyn_into::<web_sys::HtmlCanvasElement>() else {
            log::error!("#eigengraph_canvas is not an HtmlCanvasElement");
            return;
        };

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(EigenGraphApp::new(cc)))),
            )
            .await;

        if let Some(loading) = document.get_element_by_id("loading_overlay") {
            match start_result {
                Ok(_) => {
                    loading.remove();
                }
                Err(e) => {
                    loading.set_inner_html(
                        "<div style=\"max-width:460px;padding:24px;border-radius:12px;background:#1a1e28;border:1px solid #ef4444;color:#f8fafc;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;text-align:center;\">\
                         <h2 style=\"margin:0 0 8px;color:#ef4444;font-size:18px;\">WebGL2 / WebAssembly Initialization Error</h2>\
                         <p style=\"margin:0;color:#a5b4ca;font-size:13px;line-height:1.5;\">EigenGraph requires a modern browser with WebGL2 and WebAssembly enabled.</p>\
                         </div>",
                    );
                    log::error!("Failed to start EigenGraph WebRunner: {e:?}");
                }
            }
        }
    });
}
