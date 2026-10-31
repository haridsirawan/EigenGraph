//! make_icon.rs — Generates Apple HIG macOS Squircle App Icons and rounded cards for the Eigen Suite.

use image::{ImageBuffer, Rgba, imageops::FilterType};

/// Signed distance field for rounded rectangle / squircle with superellipse exponent `p`.
fn squircle_sdf(x: f32, y: f32, cx: f32, cy: f32, w: f32, h: f32, r: f32, p: f32) -> f32 {
    let half_w = w * 0.5;
    let half_h = h * 0.5;
    let straight_x = half_w - r;
    let straight_y = half_h - r;

    let qx = (x - cx).abs() - straight_x;
    let qy = (y - cy).abs() - straight_y;

    if qx > 0.0 && qy > 0.0 {
        // In corner region: superellipse continuous curvature
        (qx.powf(p) + qy.powf(p)).powf(1.0 / p) - r
    } else {
        // In straight edge or interior region
        qx.max(qy) - r
    }
}

/// Find non-white content bounding box.
fn find_content_bounds(img: &image::RgbaImage) -> (u32, u32, u32, u32) {
    let (w, h) = img.dimensions();
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0;
    let mut max_y = 0;

    for y in 4..(h - 4) {
        for x in 4..(w - 4) {
            let p = img.get_pixel(x, y);
            // Non-white pixel (background is white)
            if p[0] < 245 || p[1] < 245 || p[2] < 245 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    if min_x > max_x || min_y > max_y {
        (0, 0, w - 1, h - 1)
    } else {
        (min_x, min_y, max_x, max_y)
    }
}

/// Extract content sub-image within bounds.
fn crop_content(img: &image::RgbaImage, min_x: u32, min_y: u32, max_x: u32, max_y: u32) -> image::RgbaImage {
    let w = (max_x - min_x + 1).max(1);
    let h = (max_y - min_y + 1).max(1);
    let mut cropped = ImageBuffer::new(w, h);
    for y in 0..h {
        for x in 0..w {
            cropped.put_pixel(x, y, *img.get_pixel(min_x + x, min_y + y));
        }
    }
    cropped
}

/// Generate macOS App Icon (1024x1024 squircle)
fn generate_app_icon(src_path: &str, out_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating macOS App Icon from {} -> {}...", src_path, out_path);
    let src_img = image::open(src_path)?.to_rgba8();

    let (min_x, min_y, max_x, max_y) = find_content_bounds(&src_img);
    println!("  EigenGraph content bounds: ({}, {}) to ({}, {}) -> {}x{}",
        min_x, min_y, max_x, max_y, max_x - min_x + 1, max_y - min_y + 1);
    let cropped = crop_content(&src_img, min_x, min_y, max_x, max_y);

    let canvas_size: u32 = 1024;
    let card_size: f32 = 824.0;
    let corner_radius: f32 = 185.0;
    let center_x: f32 = 512.0;
    let center_y: f32 = 512.0;
    let superellipse_p: f32 = 4.4; // Apple squircle exponent

    // Scale artwork to fit comfortably inside the 824x824 squircle card (~660px high)
    let target_h: u32 = 660;
    let aspect = cropped.width() as f32 / cropped.height() as f32;
    let target_w = (target_h as f32 * aspect).round() as u32;

    let scaled = image::imageops::resize(&cropped, target_w, target_h, FilterType::Lanczos3);
    let offset_x = ((canvas_size - target_w) / 2) as i32;
    let offset_y = ((canvas_size - target_h) / 2) as i32;

    let mut out: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(canvas_size, canvas_size);

    for y in 0..canvas_size {
        let fy = y as f32 + 0.5;
        for x in 0..canvas_size {
            let fx = x as f32 + 0.5;

            let dist = squircle_sdf(fx, fy, center_x, center_y, card_size, card_size, corner_radius, superellipse_p);
            let card_cov = (0.5 - dist).clamp(0.0, 1.0);

            // Multi-layer drop shadows
            let mut shadow_a = 0.0f32;
            if card_cov < 1.0 {
                // Directional key shadow (dy = 16, sigma = 16)
                let d_key = squircle_sdf(fx, fy - 16.0, center_x, center_y, card_size, card_size, corner_radius, superellipse_p);
                if d_key > -5.0 {
                    let k = (d_key.max(0.0) / 16.0).powi(2);
                    shadow_a += 0.22 * (-0.5 * k).exp();
                }

                // Ambient soft shadow (dy = 6, sigma = 36)
                let d_amb = squircle_sdf(fx, fy - 6.0, center_x, center_y, card_size, card_size, corner_radius, superellipse_p);
                if d_amb > -10.0 {
                    let a = (d_amb.max(0.0) / 36.0).powi(2);
                    shadow_a += 0.16 * (-0.5 * a).exp();
                }
                shadow_a = shadow_a.clamp(0.0, 1.0);
            }

            // Ceramic white background gradient (#FFFFFF to #F3F5F9)
            let grad_t = ((fy - (center_y - card_size * 0.5)) / card_size).clamp(0.0, 1.0);
            let bg_r = 255.0 * (1.0 - grad_t) + 243.0 * grad_t;
            let bg_g = 255.0 * (1.0 - grad_t) + 245.0 * grad_t;
            let bg_b = 255.0 * (1.0 - grad_t) + 249.0 * grad_t;

            // Artwork pixel
            let art_x = x as i32 - offset_x;
            let art_y = y as i32 - offset_y;

            let (cont_r, cont_g, cont_b, cont_a) = if art_x >= 0 && art_x < target_w as i32 && art_y >= 0 && art_y < target_h as i32 {
                let p = scaled.get_pixel(art_x as u32, art_y as u32);
                let a = p[3] as f32 / 255.0;
                (p[0] as f32, p[1] as f32, p[2] as f32, a)
            } else {
                (bg_r, bg_g, bg_b, 0.0)
            };

            // Composite content over card background
            let card_r = cont_r * cont_a + bg_r * (1.0 - cont_a);
            let card_g = cont_g * cont_a + bg_g * (1.0 - cont_a);
            let card_b = cont_b * cont_a + bg_b * (1.0 - cont_a);

            // 1.5px subtle inner rim stroke
            let (rim_r, rim_g, rim_b) = if dist > -2.0 && dist <= 0.5 {
                let rim_stroke = (1.0 - (dist + 0.5).abs()).clamp(0.0, 0.12);
                (
                    card_r * (1.0 - rim_stroke),
                    card_g * (1.0 - rim_stroke),
                    card_r * (1.0 - rim_stroke)
                )
            } else {
                (card_r, card_g, card_b)
            };

            // Final composite over shadow & transparent background
            let card_a = card_cov;
            let out_a = card_a + shadow_a * (1.0 - card_a);

            if out_a <= 0.001 {
                out.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            } else {
                let out_r = (rim_r * card_a) / out_a;
                let out_g = (rim_g * card_a) / out_a;
                let out_b = (rim_b * card_a) / out_a;

                out.put_pixel(
                    x,
                    y,
                    Rgba([
                        out_r.round().clamp(0.0, 255.0) as u8,
                        out_g.round().clamp(0.0, 255.0) as u8,
                        out_b.round().clamp(0.0, 255.0) as u8,
                        (out_a * 255.0).round().clamp(0.0, 255.0) as u8,
                    ]),
                );
            }
        }
    }

    out.save(out_path)?;
    println!("  Done saving {}.", out_path);
    Ok(())
}

/// Generate horizontal rounded card logo (1024x571)
fn generate_rounded_card_logo(src_path: &str, out_path: &str, module_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating rounded card logo for {} from {} -> {}...", module_name, src_path, out_path);
    let src_img = image::open(src_path)?.to_rgba8();

    let (min_x, min_y, max_x, max_y) = find_content_bounds(&src_img);
    println!("  {} content bounds: ({}, {}) to ({}, {}) -> {}x{}",
        module_name, min_x, min_y, max_x, max_y, max_x - min_x + 1, max_y - min_y + 1);
    let cropped = crop_content(&src_img, min_x, min_y, max_x, max_y);

    let canvas_w: u32 = 1024;
    let canvas_h: u32 = 571;
    let card_w: f32 = 980.0;
    let card_h: f32 = 527.0;
    let corner_radius: f32 = 52.0;
    let center_x: f32 = (canvas_w as f32) * 0.5;
    let center_y: f32 = (canvas_h as f32) * 0.5;
    let superellipse_p: f32 = 3.6;

    // Scale artwork to fit comfortably inside the 980x527 card
    // Maximum height: 380px, maximum width: 880px
    let max_avail_w = 880.0f32;
    let max_avail_h = 380.0f32;
    let scale = (max_avail_w / cropped.width() as f32).min(max_avail_h / cropped.height() as f32);

    let target_w = (cropped.width() as f32 * scale).round() as u32;
    let target_h = (cropped.height() as f32 * scale).round() as u32;

    let scaled = image::imageops::resize(&cropped, target_w, target_h, FilterType::Lanczos3);
    let offset_x = ((canvas_w - target_w) / 2) as i32;
    let offset_y = ((canvas_h - target_h) / 2) as i32;

    let mut out: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(canvas_w, canvas_h);

    for y in 0..canvas_h {
        let fy = y as f32 + 0.5;
        for x in 0..canvas_w {
            let fx = x as f32 + 0.5;

            let dist = squircle_sdf(fx, fy, center_x, center_y, card_w, card_h, corner_radius, superellipse_p);
            let card_cov = (0.5 - dist).clamp(0.0, 1.0);

            // Drop shadows (ambient + key)
            let mut shadow_a = 0.0f32;
            if card_cov < 1.0 {
                // Key shadow (dy = 8, sigma = 12)
                let d_key = squircle_sdf(fx, fy - 8.0, center_x, center_y, card_w, card_h, corner_radius, superellipse_p);
                if d_key > -5.0 {
                    let k = (d_key.max(0.0) / 12.0).powi(2);
                    shadow_a += 0.18 * (-0.5 * k).exp();
                }

                // Ambient shadow (dy = 3, sigma = 24)
                let d_amb = squircle_sdf(fx, fy - 3.0, center_x, center_y, card_w, card_h, corner_radius, superellipse_p);
                if d_amb > -8.0 {
                    let a = (d_amb.max(0.0) / 24.0).powi(2);
                    shadow_a += 0.12 * (-0.5 * a).exp();
                }
                shadow_a = shadow_a.clamp(0.0, 1.0);
            }

            // Ceramic white background gradient
            let grad_t = ((fy - (center_y - card_h * 0.5)) / card_h).clamp(0.0, 1.0);
            let bg_r = 255.0 * (1.0 - grad_t) + 245.0 * grad_t;
            let bg_g = 255.0 * (1.0 - grad_t) + 247.0 * grad_t;
            let bg_b = 255.0 * (1.0 - grad_t) + 250.0 * grad_t;

            // Artwork pixel
            let art_x = x as i32 - offset_x;
            let art_y = y as i32 - offset_y;

            let (cont_r, cont_g, cont_b, cont_a) = if art_x >= 0 && art_x < target_w as i32 && art_y >= 0 && art_y < target_h as i32 {
                let p = scaled.get_pixel(art_x as u32, art_y as u32);
                let a = p[3] as f32 / 255.0;
                (p[0] as f32, p[1] as f32, p[2] as f32, a)
            } else {
                (bg_r, bg_g, bg_b, 0.0)
            };

            // Composite content over card background
            let card_r = cont_r * cont_a + bg_r * (1.0 - cont_a);
            let card_g = cont_g * cont_a + bg_g * (1.0 - cont_a);
            let card_b = cont_b * cont_a + bg_b * (1.0 - cont_a);

            // 1.5px subtle inner rim stroke
            let (rim_r, rim_g, rim_b) = if dist > -2.0 && dist <= 0.5 {
                let rim_stroke = (1.0 - (dist + 0.5).abs()).clamp(0.0, 0.10);
                (
                    card_r * (1.0 - rim_stroke),
                    card_g * (1.0 - rim_stroke),
                    card_r * (1.0 - rim_stroke)
                )
            } else {
                (card_r, card_g, card_b)
            };

            let card_a = card_cov;
            let out_a = card_a + shadow_a * (1.0 - card_a);

            if out_a <= 0.001 {
                out.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            } else {
                let out_r = (rim_r * card_a) / out_a;
                let out_g = (rim_g * card_a) / out_a;
                let out_b = (rim_b * card_a) / out_a;

                out.put_pixel(
                    x,
                    y,
                    Rgba([
                        out_r.round().clamp(0.0, 255.0) as u8,
                        out_g.round().clamp(0.0, 255.0) as u8,
                        out_b.round().clamp(0.0, 255.0) as u8,
                        (out_a * 255.0).round().clamp(0.0, 255.0) as u8,
                    ]),
                );
            }
        }
    }

    out.save(out_path)?;
    println!("  Done saving {}.", out_path);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_uploads = "/Users/harids.irawan/.gemini/antigravity/brain/7e668206-55ce-45f8-a5c0-2d62e4621623/.user_uploaded";

    let eigengraph_cube = format!("{}/media_1793435921782_231a23c1.png", base_uploads);
    let eigengraph_logo = format!("{}/media_1793428720102_eb66c9a8.png", base_uploads);
    let eigengate_logo   = format!("{}/media_1793428720099_d1e60057.png", base_uploads);
    let eigenflow_logo   = format!("{}/media_1793428720121_51acf427.png", base_uploads);

    // 1. Generate App Icon (macOS squircle 1024x1024)
    generate_app_icon(&eigengraph_cube, "assets/icon.png")?;

    // 2. Generate Rounded Card Logos (1024x571)
    generate_rounded_card_logo(&eigengraph_logo, "assets/logo.png", "EigenGraph")?;
    generate_rounded_card_logo(&eigengate_logo, "assets/eigengate.png", "EigenGate")?;
    generate_rounded_card_logo(&eigenflow_logo, "assets/eigenflow.png", "EigenFlow")?;

    println!("All assets successfully generated with Apple-grade continuous curvature rounded corners!");
    Ok(())
}
