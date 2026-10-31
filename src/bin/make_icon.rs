//! make_icon.rs — Tool to generate Apple HIG compliant macOS squircle App Icons.

use image::{ImageBuffer, Rgba, imageops::FilterType};
use std::path::Path;

fn rounded_box_sdf(x: f32, y: f32, cx: f32, cy: f32, w: f32, h: f32, r: f32) -> f32 {
    let qx = (x - cx).abs() - (w * 0.5 - r);
    let qy = (y - cy).abs() - (h * 0.5 - r);
    let ext_x = qx.max(0.0);
    let ext_y = qy.max(0.0);
    let int_d = (qx.max(qy)).min(0.0);
    (ext_x * ext_x + ext_y * ext_y).sqrt() + int_d - r
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src_path = "assets/icon.png";
    println!("Loading source image from {}...", src_path);
    let src_img = image::open(src_path)?.to_rgba8();

    let canvas_size: u32 = 1024;
    let card_size: f32 = 824.0;
    let corner_radius: f32 = 185.0;
    let center_x: f32 = 512.0;
    let center_y: f32 = 512.0;

    // Scale original graphic to fit comfortably with Apple HIG breathing room inside the squircle
    let content_size: u32 = 720;
    let scaled_content = image::imageops::resize(&src_img, content_size, content_size, FilterType::Lanczos3);
    let content_offset_x = ((canvas_size - content_size) / 2) as i32;
    let content_offset_y = ((canvas_size - content_size) / 2) as i32;

    let mut out_img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(canvas_size, canvas_size);

    for y in 0..canvas_size {
        let fy = y as f32 + 0.5;
        for x in 0..canvas_size {
            let fx = x as f32 + 0.5;

            // Distance to squircle card
            let dist = rounded_box_sdf(fx, fy, center_x, center_y, card_size, card_size, corner_radius);

            // Antialiased card coverage (0.0 to 1.0)
            let card_coverage = (0.5 - dist).clamp(0.0, 1.0);

            // Drop shadows (when outside or on edge of squircle)
            let mut shadow_a = 0.0f32;
            if card_coverage < 1.0 {
                // Key directional shadow (offset dy = 16px)
                let d_key = rounded_box_sdf(fx, fy - 16.0, center_x, center_y, card_size, card_size, corner_radius);
                if d_key > -10.0 {
                    let k = (d_key.max(0.0) / 16.0).powi(2);
                    shadow_a += 0.22 * (-0.5 * k).exp();
                }

                // Ambient soft shadow (offset dy = 6px)
                let d_amb = rounded_box_sdf(fx, fy - 6.0, center_x, center_y, card_size, card_size, corner_radius);
                if d_amb > -15.0 {
                    let a = (d_amb.max(0.0) / 36.0).powi(2);
                    shadow_a += 0.16 * (-0.5 * a).exp();
                }
                shadow_a = shadow_a.clamp(0.0, 1.0);
            }

            // Card background: subtle Apple ceramic gradient (#FFFFFF to #F3F5F9)
            let gradient_t = ((fy - 100.0) / card_size).clamp(0.0, 1.0);
            let bg_r = (255.0 * (1.0 - gradient_t) + 243.0 * gradient_t) as f32;
            let bg_g = (255.0 * (1.0 - gradient_t) + 245.0 * gradient_t) as f32;
            let bg_b = (255.0 * (1.0 - gradient_t) + 249.0 * gradient_t) as f32;

            // Fetch scaled lattice content pixel
            let cx = x as i32 - content_offset_x;
            let cy = y as i32 - content_offset_y;

            let (cont_r, cont_g, cont_b, cont_a) = if cx >= 0 && cx < content_size as i32 && cy >= 0 && cy < content_size as i32 {
                let p = scaled_content.get_pixel(cx as u32, cy as u32);
                (p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32 / 255.0)
            } else {
                (bg_r, bg_g, bg_b, 0.0)
            };

            // Composite content over squircle card background
            let card_rgb_r = cont_r * cont_a + bg_r * (1.0 - cont_a);
            let card_rgb_g = cont_g * cont_a + bg_g * (1.0 - cont_a);
            let card_rgb_b = cont_b * cont_a + bg_b * (1.0 - cont_a);

            // Subtle 1px inner rim stroke around squircle boundary
            let rim_factor = ((-dist).abs() - 1.0).clamp(-1.0, 1.0);
            let is_rim = dist > -2.0 && dist <= 0.5;
            let (rim_r, rim_g, rim_b) = if is_rim {
                let rim_stroke_a = (1.0 - (dist + 0.5).abs()).clamp(0.0, 0.12);
                (
                    card_rgb_r * (1.0 - rim_stroke_a),
                    card_rgb_g * (1.0 - rim_stroke_a),
                    card_rgb_b * (1.0 - rim_stroke_a)
                )
            } else {
                (card_rgb_r, card_rgb_g, card_rgb_b)
            };

            // Blend card over drop shadow
            let card_a = card_coverage;
            let out_a = card_a + shadow_a * (1.0 - card_a);

            if out_a <= 0.001 {
                out_img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            } else {
                let out_r = (rim_r * card_a + 0.0 * shadow_a * (1.0 - card_a)) / out_a;
                let out_g = (rim_g * card_a + 0.0 * shadow_a * (1.0 - card_a)) / out_a;
                let out_b = (rim_b * card_a + 0.0 * shadow_a * (1.0 - card_a)) / out_a;

                out_img.put_pixel(
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

    let out_path = "assets/icon.png";
    println!("Saving macOS squircle icon to {}...", out_path);
    out_img.save(out_path)?;
    println!("Successfully generated Apple HIG macOS squircle icon!");

    Ok(())
}
