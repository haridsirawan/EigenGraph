//! main.rs — Cross-platform entry point for EigenGraph (Native Desktop & WebAssembly).

use eigengraph::app::EigenGraphApp;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    use eframe::NativeOptions;
    use egui::Vec2;

    let mut native_options = NativeOptions::default();
    // Main egui render pass uses depth_buffer = 0 (3D Vector Earth owns an offscreen Depth24PlusStencil8 pass
    // so wgpu 30 never emits an invalid `stencilReadOnly: true` flag on depth-only Depth24Plus attachments).
    native_options.depth_buffer = 0;
    if let eframe::egui_wgpu::WgpuSetup::CreateNew(ref mut create_new) =
        native_options.wgpu_options.wgpu_setup
    {
        create_new.instance_descriptor.backends = eframe::wgpu::Backends::PRIMARY;
    }
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
    // Main egui render pass uses depth_buffer = 0 (3D Vector Earth owns an offscreen Depth24PlusStencil8 pass
    // so wgpu 30 never emits an invalid `stencilReadOnly: true` flag on depth-only Depth24Plus attachments).
    web_options.depth_buffer = 0;
    if let eframe::egui_wgpu::WgpuSetup::CreateNew(ref mut create_new) =
        web_options.wgpu_options.wgpu_setup
    {
        create_new.instance_descriptor.backends = eframe::wgpu::Backends::BROWSER_WEBGPU;
        create_new.device_descriptor = std::sync::Arc::new(|adapter| {
            let limits = adapter.limits();
            eframe::wgpu::DeviceDescriptor {
                label: Some("eigengraph_webgpu_device"),
                required_limits: eframe::wgpu::Limits {
                    max_texture_dimension_2d: limits.max_texture_dimension_2d.min(8192),
                    ..eframe::wgpu::Limits::downlevel_defaults()
                },
                ..Default::default()
            }
        });
    }

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
                        "<div style=\"max-width:480px;padding:24px;border-radius:14px;background:#0f1928;border:1px solid #ef4444;color:#f8fafc;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;text-align:center;\">\
                         <h2 style=\"margin:0 0 8px;color:#ef4444;font-size:18px;\">WebGPU / WebAssembly Initialization Error</h2>\
                         <p style=\"margin:0;color:#cbd5e1;font-size:13px;line-height:1.55;\">EigenGraph runs 100% natively on WebGPU (0% WebGL). Please open this workstation over HTTPS in Chrome/Edge 113+, Safari 26+, or Firefox 141+ with hardware WebGPU enabled.</p>\
                         </div>",
                    );
                    log::error!("Failed to start EigenGraph WebGPU WebRunner: {e:?}");
                }
            }
        }
    });
}
