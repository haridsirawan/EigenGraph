//! webgl_earth.rs — 100% Pure-Rust Native WebGL2 / OpenGL (`egui_glow`) 3D Vector Earth Renderer (Phase 65 / v0.6.5).
//!
//! Renders the curved 3D Vector Earth globe, extruded global continents & Indonesian archipelago,
//! mountain ridges, major rivers, animated procedural oceans, Rayleigh atmospheric limb glow,
//! 3D `IfcSite` TIN topographic terrain, and extruded OpenBIM IFC city massing with hardware
//! 24-bit Z-buffer, building-base Ambient Occlusion, procedural architectural facades/night windows,
//! and `f64 -> f32` Relative-to-Center (RTC) Floating-Origin precision — using ZERO external crates
//! beyond `eframe`'s built-in `glow` and ZERO raster pixel textures.

use std::sync::{Arc, Mutex};

use eframe::egui_glow;
use eframe::glow::{self, HasContext as _};
use egui::{Color32, Pos2, Rect};

use crate::core::gis::earth_globe::{
    lat_lon_to_unit_sphere, GlobeCamera, GlobeScreenTransform, PlanetaryWorldState,
};
use crate::core::gis::map_widget::{
    SlippyMapState, VectorEarthTheme, EARTH_RADIUS_METERS, WORLD_LANDMASSES, WORLD_MAJOR_RIVERS,
    WORLD_MOUNTAIN_RIDGES,
};
use crate::core::gis::tin_surface::TinSurfaceMesh;
use crate::core::gis::vector_layers::{UrbanVectorKind, UrbanVectorLayerCache};
use crate::core::gis::world_index::WorldSpatialIndex;

/// Compact GPU vertex layout (`56 bytes`, `14 × f32`) shared by both Planetary and RTC Site passes.
///
/// Zero texture coordinates or raster samplers are required: all lighting, ocean wave normals,
/// ambient occlusion, and architectural floor/window facades are synthesized analytically in GLSL.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuEarthVertex {
    /// 3D position: unit-sphere ECEF in Planetary pass, or local tangent meters `(E, N, Up)` in RTC Site pass.
    pub pos: [f32; 3],
    /// Outward 3D unit surface normal.
    pub normal: [f32; 3],
    /// Linearized RGBA base color (`0.0..=1.0`).
    pub color: [f32; 4],
    /// Analytical shader metadata:
    /// - `meta[0]`: `material_kind` (`0.0` = ocean, `1.0` = land/TIN terrain, `2.0` = extruded building, `3.0` = vector accent/road/ridge)
    /// - `meta[1]`: world elevation / wall height in meters (drives procedural floor bands & window mullions)
    /// - `meta[2]`: baked Ambient Occlusion factor (`0.55..=1.0`)
    /// - `meta[3]`: horizontal facade / wave phase parameter
    pub meta: [f32; 4],
}

/// CPU-prepared pure-vector mesh payload ready for one-time VBO/IBO upload onto the GPU.
#[derive(Debug, Clone, Default)]
pub struct GpuMeshPayload {
    pub vertices: Vec<GpuEarthVertex>,
    pub indices: Vec<u32>,
}

/// Double-precision-derived (`f64 -> f32` RTC) per-frame WebGL2 camera & atmospheric lighting uniforms.
#[derive(Debug, Clone, Copy)]
pub struct RtcCameraUniforms {
    /// Column-major 4×4 MVP transforming unit-sphere Earth coordinates `[x, y, z, 1]` to WebGL clip space.
    pub planet_mvp: [f32; 16],
    /// Column-major 4×4 Floating-Origin RTC MVP transforming local site meters `[east_m, north_m, up_m, 1]`
    /// relative to `site_origin_lat_lon` directly to WebGL clip space with sub-millimeter `f32` stability.
    pub site_mvp: [f32; 16],
    /// Unit vector toward the sun in Earth world space (Pass 1).
    pub sun_dir_world: [f32; 3],
    /// Unit vector toward the sun in local site ENU coordinates (Pass 2).
    pub sun_dir_site: [f32; 3],
    /// Camera eye position in unit-sphere Earth world space (Pass 1).
    pub cam_eye_world: [f32; 3],
    /// Camera eye position in local site ENU meters (Pass 2).
    pub cam_eye_site: [f32; 3],
    /// Clear / space backdrop RGB (`0..1`).
    pub space_bg: [f32; 3],
    /// Horizon fog RGB (`0..1`).
    pub fog_color: [f32; 3],
    /// Rayleigh atmosphere limb glow RGB (`0..1`).
    pub atmo_color: [f32; 3],
    /// Distance fog density (meters^-1 for Pass 2; `0.0` for orbital view).
    pub fog_density: f32,
    /// Elapsed animation clock in seconds (drives ocean wave shimmer).
    pub time_secs: f32,
    /// Night-side blend factor `[0.0..1.0]` (`1.0` activates glowing procedural building windows).
    pub night_mix: f32,
    /// Golden-hour sunset warmth `[0.0..1.0]`.
    pub golden_mix: f32,
    /// Enables directional day/night shading.
    pub enable_lighting: f32,
    /// Whether Pass 2 (RTC local site TIN + buildings) should execute this frame.
    pub render_site_pass: bool,
}

impl RtcCameraUniforms {
    /// Computes exact `f64 -> f32` Planetary and Floating-Origin RTC WebGL2 matrices matching
    /// `GlobeScreenTransform::project_lat_lon_elev` to sub-pixel precision (`< 0.05 px`).
    pub fn from_globe_xform(
        xform: &GlobeScreenTransform,
        theme: VectorEarthTheme,
        camera: &GlobeCamera,
        site_origin_lat_lon: (f64, f64),
        time_secs: f32,
        render_site_pass: bool,
    ) -> Self {
        let pal = theme.palette();
        let w = (xform.rect.width().max(1.0)) as f64;
        let h = (xform.rect.height().max(1.0)) as f64;
        let sx = (2.0 * xform.f_px) / w;
        let sy = (2.0 * xform.f_px) / h;

        let st = xform.tilt_rad.sin();
        let ct = xform.tilt_rad.cos();
        let d_surf = xform.d_surf;

        // Build exact 3×3 world -> tilted-tangent rotation in f64
        let rot_to_tilted = |v: [f64; 3]| -> [f64; 3] {
            let ph = xform.world_to_tangent_h_pub(v);
            [ph[0], ph[1] * ct + ph[2] * st, -ph[1] * st + ph[2] * ct]
        };

        // --- Pass 1: Planetary Globe MVP (unit sphere R = 1.0) ---
        let near_p = (d_surf * 0.02).clamp(1.0e-6, 0.25);
        let far_p = (d_surf + 2.4).max(near_p + 0.05);
        let a_p = (far_p + near_p) / (far_p - near_p);
        let b_p = (-2.0 * far_p * near_p) / (far_p - near_p);

        let c0 = rot_to_tilted([1.0, 0.0, 0.0]);
        let c1 = rot_to_tilted([0.0, 1.0, 0.0]);
        let c2 = rot_to_tilted([0.0, 0.0, 1.0]);

        // Translation for subtracting [0, 0, 1] in tangent-H before tilt: [0, -st, -ct]
        let planet_mvp: [f32; 16] = [
            // Col 0
            (sx * c0[0]) as f32,
            (sy * c0[1]) as f32,
            (-a_p * c0[2]) as f32,
            (-c0[2]) as f32,
            // Col 1
            (sx * c1[0]) as f32,
            (sy * c1[1]) as f32,
            (-a_p * c1[2]) as f32,
            (-c1[2]) as f32,
            // Col 2
            (sx * c2[0]) as f32,
            (sy * c2[1]) as f32,
            (-a_p * c2[2]) as f32,
            (-c2[2]) as f32,
            // Col 3
            0.0,
            (sy * (-st)) as f32,
            (a_p * (d_surf + ct) + b_p) as f32,
            (d_surf + ct) as f32,
        ];

        // --- Pass 2: Floating-Origin RTC Local Site MVP (vertices in local ENU meters) ---
        // Subtract O_cam_target in f64 on CPU and scale all rows by R_EARTH so clip coordinates
        // stay O(10..1000) in f32 with zero jitter even at 1 mm zoom!
        let (slat_deg, slon_deg) = site_origin_lat_lon;
        let o_site = lat_lon_to_unit_sphere(slat_deg, slon_deg);
        let o_cam_target = lat_lon_to_unit_sphere(xform.center_lat_deg, xform.center_lon_deg);

        let lat0 = slat_deg.clamp(-89.999, 89.999).to_radians();
        let lon0 = slon_deg.to_radians();
        let east_w = [lon0.cos(), 0.0, -lon0.sin()];
        let north_w = [-lat0.sin() * lon0.sin(), lat0.cos(), -lat0.sin() * lon0.cos()];
        let up_w = o_site;

        // Exact double-precision relative offset in meters between site anchor and camera target
        let delta_o_m = [
            (o_site[0] - o_cam_target[0]) * EARTH_RADIUS_METERS,
            (o_site[1] - o_cam_target[1]) * EARTH_RADIUS_METERS,
            (o_site[2] - o_cam_target[2]) * EARTH_RADIUS_METERS,
        ];
        let t_site = rot_to_tilted(delta_o_m);
        let e_t = rot_to_tilted(east_w);
        let n_t = rot_to_tilted(north_w);
        let u_t = rot_to_tilted(up_w);

        let d_surf_m = d_surf * EARTH_RADIUS_METERS;
        let near_m = (d_surf_m * 0.012).clamp(0.4, 250.0);
        let far_m = (d_surf_m * 36.0).clamp(6_000.0, 160_000.0);
        let a_s = (far_m + near_m) / (far_m - near_m);
        let b_s = (-2.0 * far_m * near_m) / (far_m - near_m);

        let site_mvp: [f32; 16] = [
            // Col 0 (East meters)
            (sx * e_t[0]) as f32,
            (sy * e_t[1]) as f32,
            (-a_s * e_t[2]) as f32,
            (-e_t[2]) as f32,
            // Col 1 (North meters)
            (sx * n_t[0]) as f32,
            (sy * n_t[1]) as f32,
            (-a_s * n_t[2]) as f32,
            (-n_t[2]) as f32,
            // Col 2 (Up meters)
            (sx * u_t[0]) as f32,
            (sy * u_t[1]) as f32,
            (-a_s * u_t[2]) as f32,
            (-u_t[2]) as f32,
            // Col 3 (Floating-Origin RTC translation in meters)
            (sx * t_site[0]) as f32,
            (sy * t_site[1]) as f32,
            (a_s * (d_surf_m - t_site[2]) + b_s) as f32,
            (d_surf_m - t_site[2]) as f32,
        ];

        // Sun & eye vectors for both World and local ENU site frames
        let sun_w = xform.sun_dir_world;
        let sun_dir_world = [sun_w[0] as f32, sun_w[1] as f32, sun_w[2] as f32];
        let dot3 = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let sun_dir_site = [
            dot3(sun_w, east_w) as f32,
            dot3(sun_w, north_w) as f32,
            dot3(sun_w, up_w) as f32,
        ];

        let cam_dist = 1.0 + d_surf;
        let cam_eye_world = [
            (xform.cam_dir_world[0] * cam_dist) as f32,
            (xform.cam_dir_world[1] * cam_dist) as f32,
            (xform.cam_dir_world[2] * cam_dist) as f32,
        ];
        // Eye vector relative to site origin in local ENU meters
        let eye_delta_m = [
            (xform.cam_dir_world[0] * cam_dist - o_site[0]) * EARTH_RADIUS_METERS,
            (xform.cam_dir_world[1] * cam_dist - o_site[1]) * EARTH_RADIUS_METERS,
            (xform.cam_dir_world[2] * cam_dist - o_site[2]) * EARTH_RADIUS_METERS,
        ];
        let cam_eye_site = [
            dot3(eye_delta_m, east_w) as f32,
            dot3(eye_delta_m, north_w) as f32,
            dot3(eye_delta_m, up_w) as f32,
        ];

        // Solar elevation at the focus site drives smooth Day -> Golden Hour -> Night transitions
        let local_sun_elev = dot3(sun_w, o_cam_target) as f32;
        let night_base = if theme == VectorEarthTheme::NightTwin {
            0.88_f32
        } else {
            ((0.08 - local_sun_elev) / 0.30).clamp(0.0, 1.0)
        };
        let golden_mix = if theme == VectorEarthTheme::NightTwin {
            0.12_f32
        } else {
            (1.0 - ((local_sun_elev - 0.14).abs() / 0.24)).clamp(0.0, 1.0)
        };

        let c32_to_rgb = |c: Color32| [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0];
        let base_fog = c32_to_rgb(pal.sky_horizon);
        let warm_fog = [0.82_f32, 0.46, 0.28];
        let night_fog = [0.035_f32, 0.055, 0.105];
        let mut fog_color = [
            base_fog[0] * (1.0 - golden_mix * 0.45) + warm_fog[0] * (golden_mix * 0.45),
            base_fog[1] * (1.0 - golden_mix * 0.45) + warm_fog[1] * (golden_mix * 0.45),
            base_fog[2] * (1.0 - golden_mix * 0.45) + warm_fog[2] * (golden_mix * 0.45),
        ];
        for i in 0..3 {
            fog_color[i] = fog_color[i] * (1.0 - night_base * 0.72) + night_fog[i] * (night_base * 0.72);
        }

        let fog_density = if xform.zoom_level >= 11.5 {
            (0.00018 / (d_surf_m as f32 / 600.0).clamp(0.35, 8.0)).clamp(0.000015, 0.00028)
        } else {
            0.0
        };

        Self {
            planet_mvp,
            site_mvp,
            sun_dir_world,
            sun_dir_site,
            cam_eye_world,
            cam_eye_site,
            space_bg: c32_to_rgb(pal.space_bg),
            fog_color,
            atmo_color: [
                pal.atmosphere_rgb[0] as f32 / 255.0,
                pal.atmosphere_rgb[1] as f32 / 255.0,
                pal.atmosphere_rgb[2] as f32 / 255.0,
            ],
            fog_density,
            time_secs,
            night_mix: night_base,
            golden_mix,
            enable_lighting: if camera.day_night_lighting { 1.0 } else { 0.0 },
            render_site_pass,
        }
    }

    /// Projects a world unit-sphere point through `planet_mvp` into screen pixels for verification tests.
    pub fn project_planet_to_screen(&self, p: [f32; 3], rect: Rect) -> Option<Pos2> {
        let m = &self.planet_mvp;
        let cx = m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12];
        let cy = m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13];
        let cw = m[3] * p[0] + m[7] * p[1] + m[11] * p[2] + m[15];
        if cw <= 1e-6 {
            return None;
        }
        let ndc_x = cx / cw;
        let ndc_y = cy / cw;
        Some(Pos2::new(
            rect.left() + (ndc_x * 0.5 + 0.5) * rect.width(),
            rect.top() + (0.5 - ndc_y * 0.5) * rect.height(),
        ))
    }

    /// Projects a local ENU site point (meters) through `site_mvp` into screen pixels for verification tests.
    pub fn project_site_to_screen(&self, enu_m: [f32; 3], rect: Rect) -> Option<Pos2> {
        let m = &self.site_mvp;
        let cx = m[0] * enu_m[0] + m[4] * enu_m[1] + m[8] * enu_m[2] + m[12];
        let cy = m[1] * enu_m[0] + m[5] * enu_m[1] + m[9] * enu_m[2] + m[13];
        let cw = m[3] * enu_m[0] + m[7] * enu_m[1] + m[11] * enu_m[2] + m[15];
        if cw <= 1e-4 {
            return None;
        }
        let ndc_x = cx / cw;
        let ndc_y = cy / cw;
        Some(Pos2::new(
            rect.left() + (ndc_x * 0.5 + 0.5) * rect.width(),
            rect.top() + (0.5 - ndc_y * 0.5) * rect.height(),
        ))
    }
}

#[inline]
fn color_to_rgba(c: Color32) -> [f32; 4] {
    [
        c.r() as f32 / 255.0,
        c.g() as f32 / 255.0,
        c.b() as f32 / 255.0,
        c.a() as f32 / 255.0,
    ]
}

#[inline]
fn mix_rgb(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let k = t.clamp(0.0, 1.0);
    [
        a[0] * (1.0 - k) + b[0] * k,
        a[1] * (1.0 - k) + b[1] * k,
        a[2] * (1.0 - k) + b[2] * k,
        1.0,
    ]
}

#[inline]
fn norm3_f64(v: [f64; 3]) -> [f64; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l > 1e-12 {
        [v[0] / l, v[1] / l, v[2] / l]
    } else {
        [0.0, 1.0, 0.0]
    }
}

#[inline]
fn cross3_f64(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Simple 2D ear-clipping triangulator returning triangle index triplets into `pts`.
fn ear_clip_indices_2d(pts: &[[f64; 2]]) -> Vec<[usize; 3]> {
    let n = pts.len();
    if n < 3 {
        return Vec::new();
    }
    let mut area2 = 0.0_f64;
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        area2 += a[0] * b[1] - b[0] * a[1];
    }
    if area2.abs() < 1e-12 {
        return Vec::new();
    }
    let mut rem: Vec<usize> = if area2 > 0.0 {
        (0..n).collect()
    } else {
        (0..n).rev().collect()
    };

    let cross = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| -> f64 {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    };
    let in_tri = |p: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]| -> bool {
        cross(a, b, p) >= -1e-9 && cross(b, c, p) >= -1e-9 && cross(c, a, p) >= -1e-9
    };

    let mut tris = Vec::with_capacity(n.saturating_sub(2));
    let mut guard = 0usize;
    let max_iters = n * n + 8;
    while rem.len() > 3 && guard < max_iters {
        guard += 1;
        let m = rem.len();
        let mut clipped = false;
        for i in 0..m {
            let i0 = rem[(i + m - 1) % m];
            let i1 = rem[i];
            let i2 = rem[(i + 1) % m];
            let (a, b, c) = (pts[i0], pts[i1], pts[i2]);
            if cross(a, b, c) <= 1e-10 {
                continue;
            }
            let any_inside = rem
                .iter()
                .any(|&idx| idx != i0 && idx != i1 && idx != i2 && in_tri(pts[idx], a, b, c));
            if !any_inside {
                tris.push([i0, i1, i2]);
                rem.remove(i);
                clipped = true;
                break;
            }
        }
        if !clipped {
            break;
        }
    }
    if rem.len() == 3 {
        tris.push([rem[0], rem[1], rem[2]]);
    }
    tris
}

/// Recursively subdivides a spherical triangle so chords never dip beneath the curved ocean sphere.
fn push_subdivided_spherical_tri(
    out_verts: &mut Vec<GpuEarthVertex>,
    out_indices: &mut Vec<u32>,
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    radius: f64,
    color: [f32; 4],
    depth: u8,
) {
    let dist_sq = |u: [f64; 3], v: [f64; 3]| -> f64 {
        (u[0] - v[0]).powi(2) + (u[1] - v[1]).powi(2) + (u[2] - v[2]).powi(2)
    };
    let max_e2 = dist_sq(a, b).max(dist_sq(b, c)).max(dist_sq(c, a));
    if depth < 3 && max_e2 > 0.022 {
        let ab = norm3_f64([a[0] + b[0], a[1] + b[1], a[2] + b[2]]);
        let bc = norm3_f64([b[0] + c[0], b[1] + c[1], b[2] + c[2]]);
        let ca = norm3_f64([c[0] + a[0], c[1] + a[1], c[2] + a[2]]);
        push_subdivided_spherical_tri(out_verts, out_indices, a, ab, ca, radius, color, depth + 1);
        push_subdivided_spherical_tri(out_verts, out_indices, ab, b, bc, radius, color, depth + 1);
        push_subdivided_spherical_tri(out_verts, out_indices, ca, bc, c, radius, color, depth + 1);
        push_subdivided_spherical_tri(out_verts, out_indices, ab, bc, ca, radius, color, depth + 1);
        return;
    }
    let base = out_verts.len() as u32;
    for n in [a, b, c] {
        out_verts.push(GpuEarthVertex {
            pos: [(n[0] * radius) as f32, (n[1] * radius) as f32, (n[2] * radius) as f32],
            normal: [n[0] as f32, n[1] as f32, n[2] as f32],
            color,
            meta: [1.0, 0.0, 1.0, 0.0],
        });
    }
    out_indices.extend_from_slice(&[base, base + 1, base + 2]);
}

/// Builds the complete static GPU mesh for the 3D Pure-Rust Vector Earth Globe (Pass 1):
/// - High-resolution 64×32 UV-sphere ocean dome (`material_kind = 0.0`)
/// - Spherical extruded continents & detailed Indonesian archipelago (`material_kind = 1.0`, with coastal bevel skirts)
/// - 3D vector mountain ridge spines & major hydrological river ribbons (`material_kind = 3.0`)
pub fn build_planetary_vector_globe_mesh(theme: VectorEarthTheme) -> GpuMeshPayload {
    let pal = theme.palette();
    let mut vertices = Vec::with_capacity(14_000);
    let mut indices = Vec::with_capacity(32_000);

    // 1. Smooth 64×32 Spherical Ocean Dome (radius = 1.0)
    const LON_SEGS: usize = 64;
    const LAT_SEGS: usize = 32;
    let ocean_rgba = color_to_rgba(pal.deep_ocean);
    let ocean_base_v = vertices.len() as u32;
    for iy in 0..=LAT_SEGS {
        let v = iy as f64 / LAT_SEGS as f64;
        let lat = -std::f64::consts::FRAC_PI_2 + v * std::f64::consts::PI;
        let (slat, clat) = (lat.sin(), lat.cos());
        for ix in 0..=LON_SEGS {
            let u = ix as f64 / LON_SEGS as f64;
            let lon = -std::f64::consts::PI + u * std::f64::consts::TAU;
            let nx = clat * lon.sin();
            let ny = slat;
            let nz = clat * lon.cos();
            vertices.push(GpuEarthVertex {
                pos: [nx as f32, ny as f32, nz as f32],
                normal: [nx as f32, ny as f32, nz as f32],
                color: ocean_rgba,
                meta: [0.0, 0.0, 1.0, (u * 8.0) as f32],
            });
        }
    }
    let stride = (LON_SEGS + 1) as u32;
    for iy in 0..LAT_SEGS as u32 {
        for ix in 0..LON_SEGS as u32 {
            let i00 = ocean_base_v + iy * stride + ix;
            let i10 = i00 + 1;
            let i01 = i00 + stride;
            let i11 = i01 + 1;
            indices.extend_from_slice(&[i00, i10, i11, i00, i11, i01]);
        }
    }

    // 2. Spherical Extruded Vector Continents & Indonesian Archipelago (`WORLD_LANDMASSES`)
    let land_base = color_to_rgba(pal.land_base);
    let land_arid = color_to_rgba(pal.land_arid);
    let land_polar = color_to_rgba(pal.land_polar);
    let coast_col = color_to_rgba(pal.coast_stroke);
    const LAND_R: f64 = 1.0022;
    const SKIRT_R: f64 = 0.9995;

    for land in WORLD_LANDMASSES {
        if land.len() < 3 {
            continue;
        }
        let unit_pts: Vec<[f64; 3]> = land
            .iter()
            .map(|&(la, lo)| lat_lon_to_unit_sphere(la, lo))
            .collect();
        let avg_lat = land.iter().map(|&(la, _)| la).sum::<f64>() / land.len() as f64;
        let is_antarctica = avg_lat < -62.0;

        let proj_2d: Vec<[f64; 2]> = if is_antarctica {
            // South-Pole stereographic projection avoids +/-180 deg seam wrap completely
            unit_pts
                .iter()
                .map(|p| {
                    let denom = (1.0 - p[1]).max(0.15);
                    [p[0] / denom, p[2] / denom]
                })
                .collect()
        } else {
            let mut c = [0.0_f64; 3];
            for p in &unit_pts {
                c[0] += p[0];
                c[1] += p[1];
                c[2] += p[2];
            }
            let c_n = norm3_f64(c);
            let ref_up = if c_n[1].abs() < 0.92 {
                [0.0, 1.0, 0.0]
            } else {
                [1.0, 0.0, 0.0]
            };
            let u_ax = norm3_f64(cross3_f64(ref_up, c_n));
            let v_ax = norm3_f64(cross3_f64(c_n, u_ax));
            unit_pts
                .iter()
                .map(|p| {
                    let denom = (1.0 + p[0] * c_n[0] + p[1] * c_n[1] + p[2] * c_n[2]).max(0.25);
                    let u = (p[0] * u_ax[0] + p[1] * u_ax[1] + p[2] * u_ax[2]) / denom;
                    let v = (p[0] * v_ax[0] + p[1] * v_ax[1] + p[2] * v_ax[2]) / denom;
                    [u, v]
                })
                .collect()
        };

        let abs_lat = avg_lat.abs();
        let fill_col = if abs_lat > 65.0 {
            mix_rgb(land_base, land_polar, 0.72)
        } else if (18.0..=31.0).contains(&abs_lat) {
            mix_rgb(land_base, land_arid, 0.52)
        } else {
            land_base
        };

        for [i0, i1, i2] in ear_clip_indices_2d(&proj_2d) {
            push_subdivided_spherical_tri(
                &mut vertices,
                &mut indices,
                unit_pts[i0],
                unit_pts[i1],
                unit_pts[i2],
                LAND_R,
                fill_col,
                0,
            );
        }

        // Tactile 3D coastal cliff / bevel rim around each landmass & island
        for i in 0..unit_pts.len() {
            let a = unit_pts[i];
            let b = unit_pts[(i + 1) % unit_pts.len()];
            let chord = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let mid = norm3_f64([a[0] + b[0], a[1] + b[1], a[2] + b[2]]);
            let edge_n = norm3_f64(cross3_f64(chord, mid));
            let base = vertices.len() as u32;
            for (pt, r, ao) in [
                (a, LAND_R, 1.0_f32),
                (b, LAND_R, 1.0),
                (b, SKIRT_R, 0.70),
                (a, SKIRT_R, 0.70),
            ] {
                vertices.push(GpuEarthVertex {
                    pos: [(pt[0] * r) as f32, (pt[1] * r) as f32, (pt[2] * r) as f32],
                    normal: [edge_n[0] as f32, edge_n[1] as f32, edge_n[2] as f32],
                    color: coast_col,
                    meta: [1.0, 0.0, ao, 0.0],
                });
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    // 3. 3D Vector Mountain Ridge Spines & Major Hydrological Rivers (spherical ribbons)
    let push_spherical_ribbon = |verts: &mut Vec<GpuEarthVertex>,
                                 idxs: &mut Vec<u32>,
                                 chain: &[(f64, f64)],
                                 radius: f64,
                                 half_w_rad: f64,
                                 col: [f32; 4]| {
        for seg in chain.windows(2) {
            let a = lat_lon_to_unit_sphere(seg[0].0, seg[0].1);
            let b = lat_lon_to_unit_sphere(seg[1].0, seg[1].1);
            let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let mid = norm3_f64([a[0] + b[0], a[1] + b[1], a[2] + b[2]]);
            let side = norm3_f64(cross3_f64(dir, mid));
            let base = verts.len() as u32;
            let corners = [
                norm3_f64([a[0] - side[0] * half_w_rad, a[1] - side[1] * half_w_rad, a[2] - side[2] * half_w_rad]),
                norm3_f64([a[0] + side[0] * half_w_rad, a[1] + side[1] * half_w_rad, a[2] + side[2] * half_w_rad]),
                norm3_f64([b[0] + side[0] * half_w_rad, b[1] + side[1] * half_w_rad, b[2] + side[2] * half_w_rad]),
                norm3_f64([b[0] - side[0] * half_w_rad, b[1] - side[1] * half_w_rad, b[2] - side[2] * half_w_rad]),
            ];
            for c in corners {
                verts.push(GpuEarthVertex {
                    pos: [(c[0] * radius) as f32, (c[1] * radius) as f32, (c[2] * radius) as f32],
                    normal: [c[0] as f32, c[1] as f32, c[2] as f32],
                    color: col,
                    meta: [3.0, 0.0, 1.0, 0.0],
                });
            }
            idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    };

    let ridge_rgba = color_to_rgba(pal.ridge_stroke);
    for ridge in WORLD_MOUNTAIN_RIDGES {
        push_spherical_ribbon(&mut vertices, &mut indices, ridge, 1.0032, 0.0024, ridge_rgba);
    }
    let river_rgba = color_to_rgba(pal.river_stroke);
    for river in WORLD_MAJOR_RIVERS {
        push_spherical_ribbon(&mut vertices, &mut indices, river, 1.0028, 0.0018, river_rgba);
    }

    GpuMeshPayload { vertices, indices }
}

/// Pushes an extruded 3D prism with vertical building-base Ambient Occlusion (`ao: 0.56 -> 1.0`),
/// sunlit roof cap, and height-calibrated procedural facade metadata (`material_kind = 2.0`).
fn push_extruded_building_prism_enu(
    verts: &mut Vec<GpuEarthVertex>,
    idxs: &mut Vec<u32>,
    footprint_enu_m: &[[f64; 2]],
    base_z_m: f32,
    height_m: f32,
    wall_rgba: [f32; 4],
    roof_rgba: [f32; 4],
) {
    if footprint_enu_m.len() < 3 || height_m < 0.5 {
        return;
    }
    let top_z = base_z_m + height_m;

    // 1. Extruded facade walls with smooth base Ambient Occlusion & horizontal phase
    let mut run_len_m = 0.0_f32;
    let n = footprint_enu_m.len();
    for i in 0..n {
        let a = footprint_enu_m[i];
        let b = footprint_enu_m[(i + 1) % n];
        let dx = (b[0] - a[0]) as f32;
        let dy = (b[1] - a[1]) as f32;
        let seg_len = dx.hypot(dy);
        if seg_len < 0.05 {
            continue;
        }
        let nx = dy / seg_len;
        let ny = -dx / seg_len;
        let base = verts.len() as u32;
        let p0 = [a[0] as f32, a[1] as f32, base_z_m];
        let p1 = [b[0] as f32, b[1] as f32, base_z_m];
        let p2 = [b[0] as f32, b[1] as f32, top_z];
        let p3 = [a[0] as f32, a[1] as f32, top_z];

        verts.push(GpuEarthVertex {
            pos: p0,
            normal: [nx, ny, 0.0],
            color: wall_rgba,
            meta: [2.0, 0.0, 0.56, run_len_m],
        });
        verts.push(GpuEarthVertex {
            pos: p1,
            normal: [nx, ny, 0.0],
            color: wall_rgba,
            meta: [2.0, 0.0, 0.56, run_len_m + seg_len],
        });
        verts.push(GpuEarthVertex {
            pos: p2,
            normal: [nx, ny, 0.0],
            color: wall_rgba,
            meta: [2.0, height_m, 1.0, run_len_m + seg_len],
        });
        verts.push(GpuEarthVertex {
            pos: p3,
            normal: [nx, ny, 0.0],
            color: wall_rgba,
            meta: [2.0, height_m, 1.0, run_len_m],
        });
        idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        run_len_m += seg_len;
    }

    // 2. Triangulated rooftop cap (upward normal [0, 0, 1])
    let roof_tris = ear_clip_indices_2d(footprint_enu_m);
    if !roof_tris.is_empty() {
        let rbase = verts.len() as u32;
        for pt in footprint_enu_m {
            verts.push(GpuEarthVertex {
                pos: [pt[0] as f32, pt[1] as f32, top_z],
                normal: [0.0, 0.0, 1.0],
                color: roof_rgba,
                meta: [2.0, height_m, 1.0, 0.0],
            });
        }
        for [i0, i1, i2] in roof_tris {
            idxs.extend_from_slice(&[rbase + i0 as u32, rbase + i1 as u32, rbase + i2 as u32]);
        }
    }
}

/// Builds the Floating-Origin RTC 3D local city/site GPU mesh (Pass 2, vertices in local ENU meters
/// around `origin_lat_lon`):
/// - Ground terrain apron & buildable city-builder parcel pads (`zoom >= 12.8`)
/// - 3D triangulated `TinSurfaceMesh` (`IfcSite` / survey / procedural) with hypsometric colors & SNI 8460 steep slopes
/// - 3D extruded Vault OpenBIM IFC buildings (`WorldSpatialIndex`, within 15 km of site origin)
/// - 3D extruded local Vault vector context buildings (`UrbanVectorLayerCache`)
pub fn build_rtc_site_scene_mesh(
    theme: VectorEarthTheme,
    origin_lat_lon: (f64, f64),
    zoom_level: f32,
    show_terrain_contours: bool,
    active_site_tin: Option<&TinSurfaceMesh>,
    world_index: &WorldSpatialIndex,
    active_rel_path: Option<&str>,
    urban_vectors: &UrbanVectorLayerCache,
) -> GpuMeshPayload {
    let pal = theme.palette();
    let mut vertices = Vec::with_capacity(4_096);
    let mut indices = Vec::with_capacity(8_192);

    // 1. Stylized Ground Terrain Apron & City-Builder Parcel Pads at site scale
    if zoom_level >= 12.5 {
        let apron_half = 4_500.0_f32;
        let land_col = color_to_rgba(pal.land_base);
        let base = vertices.len() as u32;
        for (x, y) in [
            (-apron_half, -apron_half),
            (apron_half, -apron_half),
            (apron_half, apron_half),
            (-apron_half, apron_half),
        ] {
            vertices.push(GpuEarthVertex {
                pos: [x, y, -0.25],
                normal: [0.0, 0.0, 1.0],
                color: land_col,
                meta: [1.0, 0.0, 0.96, 0.0],
            });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);

        // Stylized raised buildable parcel pads separated by avenue channels
        let block_step = if zoom_level >= 16.5 { 60.0_f32 } else { 180.0_f32 };
        let road_half = block_step * 0.065;
        let pad_col = mix_rgb(color_to_rgba(pal.land_base), color_to_rgba(pal.parcel_fill), 0.55);
        for bx in -3..=2 {
            for by in -3..=2 {
                let x0 = bx as f32 * block_step + road_half;
                let x1 = (bx + 1) as f32 * block_step - road_half;
                let y0 = by as f32 * block_step + road_half;
                let y1 = (by + 1) as f32 * block_step - road_half;
                let pb = vertices.len() as u32;
                for (px, py) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
                    vertices.push(GpuEarthVertex {
                        pos: [px, py, -0.05],
                        normal: [0.0, 0.0, 1.0],
                        color: pad_col,
                        meta: [1.0, 0.0, 1.0, 0.0],
                    });
                }
                indices.extend_from_slice(&[pb, pb + 1, pb + 2, pb, pb + 2, pb + 3]);
            }
        }
    }

    // 2. 3D Triangulated `IfcSite` / Survey / Procedural TIN Terrain Surface
    if show_terrain_contours {
        if let Some(tin) = active_site_tin {
            let (ref_z, _) = tin.sample_elevation_and_slope(0.0, 0.0);
            let span_z = (tin.max_elevation - tin.min_elevation).max(1.0);
            let low_col = color_to_rgba(pal.land_base);
            let high_col = mix_rgb(color_to_rgba(pal.land_arid), [0.85, 0.90, 0.88, 1.0], 0.35);
            let hazard_col = [0.88_f32, 0.25, 0.25, 1.0];

            for tm in tin.triangle_metrics.iter().take(6_000) {
                let p0 = tin.points[tm.indices[0]];
                let p1 = tin.points[tm.indices[1]];
                let p2 = tin.points[tm.indices[2]];
                let n = tm.normal;
                let t_elev = ((tm.centroid[2] - tin.min_elevation) / span_z).clamp(0.0, 1.0);
                let col = if tm.is_steep_hazard {
                    mix_rgb(mix_rgb(low_col, high_col, t_elev), hazard_col, 0.58)
                } else {
                    mix_rgb(low_col, high_col, t_elev)
                };
                let tb = vertices.len() as u32;
                for p in [p0, p1, p2] {
                    vertices.push(GpuEarthVertex {
                        pos: [p.x, p.y, p.z - ref_z],
                        normal: n,
                        color: col,
                        meta: [1.0, (p.z - ref_z).max(0.0), 0.95, 0.0],
                    });
                }
                indices.extend_from_slice(&[tb, tb + 1, tb + 2]);
            }
        }
    }

    // 3. 3D Extruded Vault OpenBIM IFC Buildings from `WorldSpatialIndex` (within 15 km)
    let wall_base = match theme {
        VectorEarthTheme::CoastalBlueprint => [0.22_f32, 0.37, 0.49, 1.0],
        VectorEarthTheme::CityDaylight => [0.84_f32, 0.86, 0.88, 1.0],
        VectorEarthTheme::NightTwin => [0.16_f32, 0.22, 0.34, 1.0],
        VectorEarthTheme::Topographic => [0.76_f32, 0.78, 0.75, 1.0],
    };
    let roof_base = mix_rgb(wall_base, [0.95, 0.98, 1.0, 1.0], 0.22);

    let mut extruded_count = 0usize;
    for b in &world_index.buildings {
        if extruded_count >= 450 {
            break;
        }
        // Skip the active editable BIM building (rendered in high-detail structural pass on top)
        if active_rel_path.is_some_and(|p| p == b.rel_path) {
            continue;
        }
        let offset_mm = SlippyMapState::lat_lon_to_world_mm(origin_lat_lon, b.lat, b.lon);
        let cx_m = offset_mm[0] * 0.001;
        let cy_m = offset_mm[1] * 0.001;
        if cx_m.hypot(cy_m) > 15_000.0 {
            continue;
        }
        let rad = b.heading_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let fp: Vec<[f64; 2]> = b
            .footprint_local_m
            .iter()
            .map(|&[lx, ly]| [cx_m + lx * cr - ly * sr, cy_m + lx * sr + ly * cr])
            .collect();
        // Subtle ARI/DCR health tint blended into city-builder facade color
        let health_tint = if b.max_dcr > 1.0 || b.min_ari < 0.70 {
            [0.92_f32, 0.30, 0.30, 1.0]
        } else if b.max_dcr > 0.85 {
            [0.95_f32, 0.65, 0.22, 1.0]
        } else {
            wall_base
        };
        let wall_col = mix_rgb(wall_base, health_tint, 0.28);
        let roof_col = mix_rgb(roof_base, health_tint, 0.22);
        push_extruded_building_prism_enu(
            &mut vertices,
            &mut indices,
            &fp,
            0.0,
            b.total_height_m.clamp(3.2, 650.0) as f32,
            wall_col,
            roof_col,
        );
        extruded_count += 1;
    }

    // 4. 3D Extruded Local Vault Vector Context Buildings (from offline GeoJSON / IFC site layers)
    if zoom_level >= 12.5 && urban_vectors.feature_count() > 0 {
        let deg_span = 0.06_f64;
        let vis = urban_vectors.snapshot_visible(
            origin_lat_lon.0 - deg_span,
            origin_lat_lon.0 + deg_span,
            origin_lat_lon.1 - deg_span,
            origin_lat_lon.1 + deg_span,
            400,
        );
        let ctx_wall = mix_rgb(wall_base, color_to_rgba(pal.parcel_fill), 0.35);
        let ctx_roof = mix_rgb(ctx_wall, [0.92, 0.96, 1.0, 1.0], 0.18);
        for feat in vis {
            if feat.kind == UrbanVectorKind::ContextBuilding && feat.points_lat_lon.len() >= 3 {
                let fp: Vec<[f64; 2]> = feat
                    .points_lat_lon
                    .iter()
                    .map(|&(la, lo)| {
                        let mm = SlippyMapState::lat_lon_to_world_mm(origin_lat_lon, la, lo);
                        [mm[0] * 0.001, mm[1] * 0.001]
                    })
                    .collect();
                push_extruded_building_prism_enu(
                    &mut vertices,
                    &mut indices,
                    &fp,
                    0.0,
                    feat.height_m.clamp(3.5, 450.0),
                    ctx_wall,
                    ctx_roof,
                );
            }
        }
    }

    GpuMeshPayload { vertices, indices }
}

/// Safe byte slice reinterpretation for `#[repr(C)]` `GpuEarthVertex` arrays (zero external crates).
fn vertices_as_u8_slice(verts: &[GpuEarthVertex]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            verts.as_ptr() as *const u8,
            std::mem::size_of_val(verts),
        )
    }
}

/// Safe byte slice reinterpretation for `u32` GPU index arrays (zero external crates).
fn indices_as_u8_slice(indices: &[u32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            indices.as_ptr() as *const u8,
            std::mem::size_of_val(indices),
        )
    }
}

/// Holds live GPU program + VAO/VBO/IBO handles (strictly `Send + Sync` on both Native & `wasm32`
/// because zero `glow::UniformLocation` values are cached across frames).
struct GpuEarthPipeline {
    program: glow::Program,
    planet_vao: glow::VertexArray,
    planet_vbo: glow::Buffer,
    planet_ibo: glow::Buffer,
    planet_index_count: i32,
    planet_theme_uploaded: Option<VectorEarthTheme>,
    site_vao: glow::VertexArray,
    site_vbo: glow::Buffer,
    site_ibo: glow::Buffer,
    site_index_count: i32,
    site_revision_uploaded: u64,
}

impl GpuEarthPipeline {
    fn new(gl: &glow::Context) -> Result<Self, String> {
        let shader_ver = egui_glow::ShaderVersion::get(gl);
        if !shader_ver.is_new_shader_interface() {
            return Err("WebGL2 or OpenGL 3.1+ is required for GPU Vector Earth".to_string());
        }
        let header = shader_ver.version_declaration();
        let prec = if shader_ver.is_embedded() {
            "precision highp float;\n"
        } else {
            ""
        };

        let vert_src = format!(
            "{header}{prec}
            in vec3 a_pos;
            in vec3 a_normal;
            in vec4 a_color;
            in vec4 a_meta;

            uniform mat4 u_mvp;

            out vec3 v_pos;
            out vec3 v_normal;
            out vec4 v_color;
            out vec4 v_meta;
            out float v_cam_dist;

            void main() {{
                vec4 clip = u_mvp * vec4(a_pos, 1.0);
                gl_Position = clip;
                v_pos = a_pos;
                v_normal = a_normal;
                v_color = a_color;
                v_meta = a_meta;
                v_cam_dist = max(clip.w, 0.0);
            }}"
        );

        let frag_src = format!(
            "{header}{prec}
            in vec3 v_pos;
            in vec3 v_normal;
            in vec4 v_color;
            in vec4 v_meta;
            in float v_cam_dist;

            uniform vec3 u_sun_dir;
            uniform vec3 u_cam_eye;
            uniform vec3 u_fog_color;
            uniform vec3 u_atmo_color;
            uniform float u_fog_density;
            uniform float u_time;
            uniform float u_night_mix;
            uniform float u_golden_mix;
            uniform float u_enable_lighting;

            out vec4 out_color;

            void main() {{
                vec3 N = normalize(v_normal);
                vec3 L = normalize(u_sun_dir);
                vec3 V = normalize(u_cam_eye - v_pos);
                float kind = v_meta.x;
                float height_m = v_meta.y;
                float ao = v_meta.z;
                float phase = v_meta.w;

                // Animated procedural ocean surface micro-ripples (0% raster pixels)
                if (kind < 0.5) {{
                    float w1 = sin(v_pos.x * 58.0 + u_time * 1.15) * cos(v_pos.z * 54.0 - u_time * 0.85);
                    float w2 = sin((v_pos.y * 74.0 + v_pos.x * 42.0) - u_time * 1.45);
                    N = normalize(N + vec3(w1 * 0.028, w2 * 0.022, (w1 - w2) * 0.022));
                }}

                float ndotl = dot(N, L);
                float day_diffuse = smoothstep(-0.22, 0.85, ndotl);
                float lit = mix(1.0, mix(0.24, 1.0, day_diffuse), u_enable_lighting);
                // Hemisphere sky/ground bounce for clean City-Builder readability
                float hemi = 0.84 + 0.16 * clamp(N.z * 0.5 + 0.5, 0.0, 1.0);
                vec3 sun_tint = mix(vec3(1.0, 0.99, 0.96), vec3(1.0, 0.72, 0.44), u_golden_mix * 0.75);
                vec3 col = v_color.rgb * sun_tint * lit * hemi * ao;

                if (kind < 0.5) {{
                    // Ocean specular glint + Fresnel Rayleigh atmospheric limb glow
                    vec3 H = normalize(L + V);
                    float spec = pow(max(dot(N, H), 0.0), 42.0) * max(ndotl, 0.0) * 0.42;
                    float fresnel = pow(1.0 - max(dot(N, V), 0.0), 2.8);
                    col += sun_tint * spec + u_atmo_color * (fresnel * 0.58);
                }} else if (kind > 1.5 && kind < 2.5) {{
                    // Extruded City-Builder & IFC Building Facades (100% procedural floor & window bands)
                    float is_wall = 1.0 - step(0.85, abs(N.z));
                    float story_v = fract(height_m / 3.4);
                    float mullion_u = fract(phase / 2.6);
                    float win_band = step(0.24, story_v) * step(story_v, 0.76)
                                   * step(0.18, mullion_u) * step(mullion_u, 0.82);
                    float glass_mask = is_wall * win_band * step(1.5, height_m);
                    // Subtle daytime sky reflection on glass strips
                    vec3 day_glass = mix(col * 0.72, u_atmo_color * 0.95, 0.38);
                    col = mix(col, day_glass, glass_mask * (1.0 - u_night_mix) * 0.55);
                    // Procedural warm/cyan lit windows at Dusk & Night
                    float lit_hash = fract(sin(floor(height_m / 3.4) * 12.9898 + floor(phase / 2.6) * 78.233) * 43758.5453);
                    float lit_on = step(0.38, lit_hash);
                    vec3 win_glow = mix(vec3(1.0, 0.84, 0.52), vec3(0.52, 0.88, 1.0), step(0.78, lit_hash));
                    float night_strength = max(u_night_mix, u_golden_mix * 0.45);
                    col += win_glow * (glass_mask * lit_on * night_strength * 0.82);
                }} else if (kind < 1.5) {{
                    // Continents & 3D IfcSite TIN subtle coastal atmospheric rim
                    float fresnel = pow(1.0 - max(dot(N, V), 0.0), 3.2);
                    col += u_atmo_color * (fresnel * 0.16);
                }}

                // Exponential distance atmospheric horizon fog (Pass 2 city scale)
                if (u_fog_density > 0.0) {{
                    float fog_f = 1.0 - exp(-v_cam_dist * u_fog_density);
                    col = mix(col, u_fog_color, clamp(fog_f, 0.0, 0.82));
                }}

                out_color = vec4(clamp(col, 0.0, 1.0), v_color.a);
            }}"
        );

        unsafe {
            let program = gl.create_program().map_err(|e| format!("create_program: {e}"))?;
            let vs = gl
                .create_shader(glow::VERTEX_SHADER)
                .map_err(|e| format!("create_shader VS: {e}"))?;
            gl.shader_source(vs, &vert_src);
            gl.compile_shader(vs);
            if !gl.get_shader_compile_status(vs) {
                let log = gl.get_shader_info_log(vs);
                gl.delete_shader(vs);
                gl.delete_program(program);
                return Err(format!("Vertex shader compile error: {log}"));
            }

            let fs = gl
                .create_shader(glow::FRAGMENT_SHADER)
                .map_err(|e| format!("create_shader FS: {e}"))?;
            gl.shader_source(fs, &frag_src);
            gl.compile_shader(fs);
            if !gl.get_shader_compile_status(fs) {
                let log = gl.get_shader_info_log(fs);
                gl.delete_shader(vs);
                gl.delete_shader(fs);
                gl.delete_program(program);
                return Err(format!("Fragment shader compile error: {log}"));
            }

            gl.attach_shader(program, vs);
            gl.attach_shader(program, fs);
            gl.bind_attrib_location(program, 0, "a_pos");
            gl.bind_attrib_location(program, 1, "a_normal");
            gl.bind_attrib_location(program, 2, "a_color");
            gl.bind_attrib_location(program, 3, "a_meta");
            gl.link_program(program);
            gl.delete_shader(vs);
            gl.delete_shader(fs);

            if !gl.get_program_link_status(program) {
                let log = gl.get_program_info_log(program);
                gl.delete_program(program);
                return Err(format!("Shader program link error: {log}"));
            }

            let setup_vao_pair = || -> Result<(glow::VertexArray, glow::Buffer, glow::Buffer), String> {
                let vao = gl.create_vertex_array().map_err(|e| format!("create_vao: {e}"))?;
                let vbo = gl.create_buffer().map_err(|e| format!("create_vbo: {e}"))?;
                let ibo = gl.create_buffer().map_err(|e| format!("create_ibo: {e}"))?;
                gl.bind_vertex_array(Some(vao));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ibo));
                let stride = std::mem::size_of::<GpuEarthVertex>() as i32;
                gl.enable_vertex_attrib_array(0);
                gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);
                gl.enable_vertex_attrib_array(1);
                gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, stride, 12);
                gl.enable_vertex_attrib_array(2);
                gl.vertex_attrib_pointer_f32(2, 4, glow::FLOAT, false, stride, 24);
                gl.enable_vertex_attrib_array(3);
                gl.vertex_attrib_pointer_f32(3, 4, glow::FLOAT, false, stride, 40);
                gl.bind_vertex_array(None);
                Ok((vao, vbo, ibo))
            };

            let (planet_vao, planet_vbo, planet_ibo) = setup_vao_pair()?;
            let (site_vao, site_vbo, site_ibo) = setup_vao_pair()?;

            Ok(Self {
                program,
                planet_vao,
                planet_vbo,
                planet_ibo,
                planet_index_count: 0,
                planet_theme_uploaded: None,
                site_vao,
                site_vbo,
                site_ibo,
                site_index_count: 0,
                site_revision_uploaded: 0,
            })
        }
    }

    fn upload_planet(&mut self, gl: &glow::Context, theme: VectorEarthTheme, mesh: &GpuMeshPayload) {
        unsafe {
            gl.bind_vertex_array(Some(self.planet_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.planet_vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                vertices_as_u8_slice(&mesh.vertices),
                glow::STATIC_DRAW,
            );
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.planet_ibo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                indices_as_u8_slice(&mesh.indices),
                glow::STATIC_DRAW,
            );
            gl.bind_vertex_array(None);
        }
        self.planet_index_count = mesh.indices.len() as i32;
        self.planet_theme_uploaded = Some(theme);
    }

    fn upload_site(&mut self, gl: &glow::Context, rev: u64, mesh: &GpuMeshPayload) {
        unsafe {
            gl.bind_vertex_array(Some(self.site_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.site_vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                vertices_as_u8_slice(&mesh.vertices),
                glow::DYNAMIC_DRAW,
            );
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.site_ibo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                indices_as_u8_slice(&mesh.indices),
                glow::DYNAMIC_DRAW,
            );
            gl.bind_vertex_array(None);
        }
        self.site_index_count = mesh.indices.len() as i32;
        self.site_revision_uploaded = rev;
    }

    fn paint(&self, gl: &glow::Context, u: &RtcCameraUniforms) {
        unsafe {
            gl.use_program(Some(self.program));
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);
            gl.depth_mask(true);
            gl.disable(glow::CULL_FACE);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

            let loc_mvp = gl.get_uniform_location(self.program, "u_mvp");
            let loc_sun = gl.get_uniform_location(self.program, "u_sun_dir");
            let loc_eye = gl.get_uniform_location(self.program, "u_cam_eye");
            let loc_fog_c = gl.get_uniform_location(self.program, "u_fog_color");
            let loc_atmo = gl.get_uniform_location(self.program, "u_atmo_color");
            let loc_fog_d = gl.get_uniform_location(self.program, "u_fog_density");
            let loc_time = gl.get_uniform_location(self.program, "u_time");
            let loc_night = gl.get_uniform_location(self.program, "u_night_mix");
            let loc_gold = gl.get_uniform_location(self.program, "u_golden_mix");
            let loc_lit = gl.get_uniform_location(self.program, "u_enable_lighting");

            gl.uniform_3_f32(loc_fog_c.as_ref(), u.fog_color[0], u.fog_color[1], u.fog_color[2]);
            gl.uniform_3_f32(loc_atmo.as_ref(), u.atmo_color[0], u.atmo_color[1], u.atmo_color[2]);
            gl.uniform_1_f32(loc_time.as_ref(), u.time_secs);
            gl.uniform_1_f32(loc_night.as_ref(), u.night_mix);
            gl.uniform_1_f32(loc_gold.as_ref(), u.golden_mix);
            gl.uniform_1_f32(loc_lit.as_ref(), u.enable_lighting);

            // --- Pass 1: Planetary Vector Globe (unit sphere, full hardware Z-buffer) ---
            gl.clear_depth_f32(1.0);
            gl.clear(glow::DEPTH_BUFFER_BIT);
            if self.planet_index_count > 0 {
                gl.uniform_matrix_4_f32_slice(loc_mvp.as_ref(), false, &u.planet_mvp);
                gl.uniform_3_f32(loc_sun.as_ref(), u.sun_dir_world[0], u.sun_dir_world[1], u.sun_dir_world[2]);
                gl.uniform_3_f32(loc_eye.as_ref(), u.cam_eye_world[0], u.cam_eye_world[1], u.cam_eye_world[2]);
                gl.uniform_1_f32(loc_fog_d.as_ref(), 0.0);
                gl.bind_vertex_array(Some(self.planet_vao));
                gl.draw_elements(glow::TRIANGLES, self.planet_index_count, glow::UNSIGNED_INT, 0);
            }

            // --- Pass 2: Floating-Origin RTC Local Site Scene (3D IfcSite TIN + Extruded IFC Buildings) ---
            if u.render_site_pass && self.site_index_count > 0 {
                gl.clear_depth_f32(1.0);
                gl.clear(glow::DEPTH_BUFFER_BIT);
                gl.uniform_matrix_4_f32_slice(loc_mvp.as_ref(), false, &u.site_mvp);
                gl.uniform_3_f32(loc_sun.as_ref(), u.sun_dir_site[0], u.sun_dir_site[1], u.sun_dir_site[2]);
                gl.uniform_3_f32(loc_eye.as_ref(), u.cam_eye_site[0], u.cam_eye_site[1], u.cam_eye_site[2]);
                gl.uniform_1_f32(loc_fog_d.as_ref(), u.fog_density);
                gl.bind_vertex_array(Some(self.site_vao));
                gl.draw_elements(glow::TRIANGLES, self.site_index_count, glow::UNSIGNED_INT, 0);
            }

            gl.bind_vertex_array(None);
            gl.use_program(None);
            gl.disable(glow::DEPTH_TEST);
        }
    }

    fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.planet_vao);
            gl.delete_buffer(self.planet_vbo);
            gl.delete_buffer(self.planet_ibo);
            gl.delete_vertex_array(self.site_vao);
            gl.delete_buffer(self.site_vbo);
            gl.delete_buffer(self.site_ibo);
        }
    }
}

/// Manages WebGL2 / OpenGL Vector Earth GPU state with automatic CPU fallback in headless environments.
#[derive(Clone, Default)]
pub struct WebGlEarthState {
    pipeline: Option<Arc<Mutex<GpuEarthPipeline>>>,
    pub enabled: bool,
    pub last_init_error: Option<String>,
    cached_planet_theme: Option<VectorEarthTheme>,
    cached_site_hash: u64,
    pub last_planet_tris: usize,
    pub last_site_tris: usize,
}

impl std::fmt::Debug for WebGlEarthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebGlEarthState")
            .field("available", &self.pipeline.is_some())
            .field("enabled", &self.enabled)
            .field("last_planet_tris", &self.last_planet_tris)
            .field("last_site_tris", &self.last_site_tris)
            .finish()
    }
}

impl WebGlEarthState {
    /// Initializes GPU shaders & VAO/VBO handles from `eframe::CreationContext::gl`.
    pub fn init_gl(&mut self, gl: &Arc<glow::Context>) {
        match GpuEarthPipeline::new(gl) {
            Ok(pipe) => {
                self.pipeline = Some(Arc::new(Mutex::new(pipe)));
                self.enabled = true;
                self.last_init_error = None;
            }
            Err(err) => {
                self.pipeline = None;
                self.enabled = false;
                self.last_init_error = Some(err);
            }
        }
    }

    /// True when the native WebGL2/OpenGL hardware pipeline is initialized and enabled by the user.
    pub fn is_active(&self) -> bool {
        self.enabled && self.pipeline.is_some()
    }

    /// True when a hardware GL context was successfully compiled (regardless of user toggle).
    pub fn is_available(&self) -> bool {
        self.pipeline.is_some()
    }

    /// Releases GPU shaders and vertex buffers cleanly on application exit.
    pub fn destroy(&mut self, gl: &glow::Context) {
        if let Some(arc) = self.pipeline.take() {
            if let Ok(pipe) = arc.lock() {
                pipe.destroy(gl);
            }
        }
    }

    /// Deterministic hash identifying when the RTC local site VBO needs re-meshing.
    fn compute_site_hash(
        theme: VectorEarthTheme,
        origin: (f64, f64),
        zoom_bucket: i32,
        show_tin: bool,
        tin: Option<&TinSurfaceMesh>,
        world_index: &WorldSpatialIndex,
        active_rel: Option<&str>,
        urban_vectors: &UrbanVectorLayerCache,
    ) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mix = |acc: &mut u64, val: u64| {
            *acc ^= val;
            *acc = acc.wrapping_mul(0x0000_0100_0000_01b3);
        };
        mix(&mut h, theme as u64 + 1);
        mix(&mut h, (origin.0 * 1e5).round() as i64 as u64);
        mix(&mut h, (origin.1 * 1e5).round() as i64 as u64);
        mix(&mut h, zoom_bucket as u64);
        mix(&mut h, u64::from(show_tin));
        if let Some(t) = tin {
            mix(&mut h, t.points.len() as u64);
            mix(&mut h, t.triangles.len() as u64);
            mix(&mut h, (t.max_elevation * 100.0) as i32 as u64);
        }
        mix(&mut h, world_index.buildings.len() as u64);
        for b in world_index.buildings.iter().take(64) {
            mix(&mut h, b.id as u64);
            mix(&mut h, (b.lat * 1e4).round() as i64 as u64);
            mix(&mut h, (b.lon * 1e4).round() as i64 as u64);
            mix(&mut h, (b.total_height_m * 10.0) as u64);
        }
        if let Some(rel) = active_rel {
            for byte in rel.bytes() {
                mix(&mut h, byte as u64);
            }
        }
        mix(&mut h, urban_vectors.feature_count() as u64);
        h
    }

    /// Schedules the hardware WebGL2 3D Vector Earth pass onto `painter`, uploading geometry ONLY
    /// when theme or site content changes (`O(1)` steady-state draw). Returns `true` if GPU pass ran.
    #[allow(clippy::too_many_arguments)]
    pub fn schedule_paint_pass(
        &mut self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        rect: Rect,
        earth_world: &PlanetaryWorldState,
        world_map: &SlippyMapState,
        ground_origin_lat_lon: (f64, f64),
        ground_origin_anchored: bool,
        show_terrain_contours: bool,
        active_site_tin: Option<&TinSurfaceMesh>,
        world_index: &WorldSpatialIndex,
        active_rel_path: Option<&str>,
        urban_vectors: &UrbanVectorLayerCache,
    ) -> bool {
        let Some(pipe_arc) = self.pipeline.clone() else {
            return false;
        };
        if !self.enabled || !rect.is_positive() {
            return false;
        }

        let theme = world_map.tile_provider;
        let xform = GlobeScreenTransform::from_view(
            rect,
            world_map.center_lat_lon.0,
            world_map.center_lat_lon.1,
            world_map.zoom_level,
            &earth_world.camera,
        );
        let site_origin = if ground_origin_anchored {
            ground_origin_lat_lon
        } else {
            world_map.center_lat_lon
        };
        let render_site_pass = world_map.zoom_level >= 10.5;
        let time_secs = (ui.ctx().input(|i| i.time) % 3600.0) as f32;
        let uniforms = RtcCameraUniforms::from_globe_xform(
            &xform,
            theme,
            &earth_world.camera,
            site_origin,
            time_secs,
            render_site_pass,
        );

        // Paint crisp space & atmosphere halo backdrop beneath the transparent GL sphere edges
        let pal = theme.palette();
        painter.rect_filled(rect, egui::CornerRadius::same(10), pal.space_bg);
        let r_limb = xform.globe_limb_radius_px();
        if earth_world.show_atmosphere && xform.d_surf > 0.25 && r_limb < rect.height() * 2.2 {
            let [ar, ag, ab] = pal.atmosphere_rgb;
            for (scale, alpha) in [(1.09_f32, 18_u8), (1.045, 36), (1.015, 68)] {
                painter.circle_filled(
                    rect.center(),
                    r_limb * scale,
                    Color32::from_rgba_unmultiplied(ar, ag, ab, alpha),
                );
            }
        }

        // Prepare planetary mesh upload only on theme change
        let pending_planet = if self.cached_planet_theme != Some(theme) {
            let mesh = build_planetary_vector_globe_mesh(theme);
            self.last_planet_tris = mesh.indices.len() / 3;
            self.cached_planet_theme = Some(theme);
            Some((theme, mesh))
        } else {
            None
        };

        // Prepare RTC site mesh upload only when zoomed in and content hash changes
        let zoom_bucket = if world_map.zoom_level >= 16.5 {
            2
        } else if world_map.zoom_level >= 12.5 {
            1
        } else {
            0
        };
        let pending_site = if render_site_pass {
            let rev = Self::compute_site_hash(
                theme,
                site_origin,
                zoom_bucket,
                show_terrain_contours,
                active_site_tin,
                world_index,
                active_rel_path,
                urban_vectors,
            );
            if rev != self.cached_site_hash {
                let mesh = build_rtc_site_scene_mesh(
                    theme,
                    site_origin,
                    world_map.zoom_level,
                    show_terrain_contours,
                    active_site_tin,
                    world_index,
                    active_rel_path,
                    urban_vectors,
                );
                self.last_site_tris = mesh.indices.len() / 3;
                self.cached_site_hash = rev;
                Some((rev, mesh))
            } else {
                None
            }
        } else {
            None
        };

        let planet_payload = Arc::new(Mutex::new(pending_planet));
        let site_payload = Arc::new(Mutex::new(pending_site));

        let cb = egui_glow::CallbackFn::new(move |_info, glow_painter| {
            let gl = glow_painter.gl();
            if let Ok(mut pipe) = pipe_arc.lock() {
                if let Ok(mut slot) = planet_payload.lock() {
                    if let Some((th, mesh)) = slot.take() {
                        if pipe.planet_theme_uploaded != Some(th) {
                            pipe.upload_planet(gl, th, &mesh);
                        }
                    }
                }
                if let Ok(mut slot) = site_payload.lock() {
                    if let Some((rev, mesh)) = slot.take() {
                        if pipe.site_revision_uploaded != rev {
                            pipe.upload_site(gl, rev, &mesh);
                        }
                    }
                }
                pipe.paint(gl, &uniforms);
            }
        });

        painter.add(egui::PaintCallback {
            rect,
            callback: Arc::new(cb),
        });
        // Smooth ocean shimmer animation at low overhead
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::gis::world_index::{GeoPlacementSource, WorldBuildingProxy};
    use crate::core::gis::DemTerrainPreset;

    #[test]
    fn test_phase65_rtc_mvp_matches_globe_screen_transform_subpixel() {
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), egui::Vec2::new(1280.0, 720.0));
        let cam = GlobeCamera {
            tilt_deg: 38.0,
            heading_deg: 27.0,
            ..GlobeCamera::default()
        };

        // 1. Orbital zoom planetary matrix parity
        let xf_orb = GlobeScreenTransform::from_view(rect, -6.2088, 106.8456, 2.4, &cam);
        let u_orb = RtcCameraUniforms::from_globe_xform(
            &xf_orb,
            VectorEarthTheme::CoastalBlueprint,
            &cam,
            (-6.2088, 106.8456),
            0.0,
            false,
        );
        for &(lat, lon) in &[(-6.2088, 106.8456), (-7.7956, 110.3695), (-0.9730, 116.7090)] {
            let (cpu_sp, _, front) = xf_orb.project_lat_lon_elev(lat, lon, 0.0).expect("cpu proj");
            assert!(front);
            let p_w = lat_lon_to_unit_sphere(lat, lon);
            let gpu_sp = u_orb
                .project_planet_to_screen([p_w[0] as f32, p_w[1] as f32, p_w[2] as f32], rect)
                .expect("gpu planet proj");
            let err_px = (cpu_sp - gpu_sp).length();
            assert!(
                err_px < 0.05,
                "Planetary MVP mismatch at ({lat},{lon}): cpu={cpu_sp:?} gpu={gpu_sp:?} err={err_px:.4}px"
            );
        }

        // 2. Street-scale Floating-Origin RTC matrix parity (zoom = 18.5, zero f32 jitter!)
        let site = (-6.2088, 106.8456);
        let xf_site = GlobeScreenTransform::from_view(rect, -6.2085, 106.8459, 18.5, &cam);
        let u_site = RtcCameraUniforms::from_globe_xform(
            &xf_site,
            VectorEarthTheme::CityDaylight,
            &cam,
            site,
            1.5,
            true,
        );
        for &(east_m, north_m, up_m) in &[(0.0_f64, 0.0, 0.0), (24.0, -16.0, 18.0), (-35.0, 42.0, 48.0)] {
            let ll = SlippyMapState::world_mm_to_lat_lon(site, [east_m * 1000.0, north_m * 1000.0]);
            let (cpu_sp, _, front) = xf_site.project_lat_lon_elev(ll.0, ll.1, up_m).expect("cpu site proj");
            assert!(front);
            let gpu_sp = u_site
                .project_site_to_screen([east_m as f32, north_m as f32, up_m as f32], rect)
                .expect("gpu rtc site proj");
            let err_px = (cpu_sp - gpu_sp).length();
            assert!(
                err_px < 0.25,
                "RTC Site MVP mismatch at ({east_m},{north_m},{up_m}): cpu={cpu_sp:?} gpu={gpu_sp:?} err={err_px:.4}px"
            );
        }
    }

    #[test]
    fn test_phase65_planetary_and_rtc_site_gpu_mesh_generation() {
        for vt in VectorEarthTheme::ALL {
            let globe = build_planetary_vector_globe_mesh(vt);
            assert!(globe.vertices.len() > 2_500, "Expected rich globe vertices for {vt:?}");
            assert!(globe.indices.len() >= 12_000, "Expected 4,000+ triangles for {vt:?}");
            assert_eq!(globe.indices.len() % 3, 0);
            for v in &globe.vertices {
                assert!(v.pos[0].is_finite() && v.pos[1].is_finite() && v.pos[2].is_finite());
                assert!(v.normal[0].is_finite() && v.normal[1].is_finite() && v.normal[2].is_finite());
            }
        }

        let origin = (-6.9175, 107.6191);
        let tin = TinSurfaceMesh::generate_from_dem_preset(
            DemTerrainPreset::VolcanicFoothillsRidge,
            300.0,
            300.0,
            16,
            16,
            1.0,
        );
        let mut world_idx = WorldSpatialIndex::default();
        world_idx.upsert_building(
            WorldBuildingProxy {
                id: 1,
                rel_path: "Projects/Bandung/Tower_B.ifc".to_string(),
                name: "Tower B".to_string(),
                filename: "Tower_B.ifc".to_string(),
                lat: origin.0,
                lon: origin.1,
                elev_m: 710.0,
                heading_deg: 15.0,
                footprint_local_m: vec![[-12.0, -9.0], [12.0, -9.0], [12.0, 9.0], [-12.0, 9.0]],
                total_height_m: 44.0,
                story_count: 12,
                member_count: 180,
                max_dcr: 0.78,
                min_ari: 0.84,
                source: GeoPlacementSource::UserPinnedOnEarth,
                address_hint: "Bandung".to_string(),
            },
            true,
        );
        let urban = UrbanVectorLayerCache::default();
        let site_mesh = build_rtc_site_scene_mesh(
            VectorEarthTheme::CoastalBlueprint,
            origin,
            17.5,
            true,
            Some(&tin),
            &world_idx,
            None,
            &urban,
        );
        assert!(site_mesh.indices.len() / 3 > 400, "Site TIN + extruded IFC tower mesh should be populated");
        assert!(
            site_mesh.vertices.iter().any(|v| (v.meta[0] - 2.0).abs() < 0.01 && v.meta[1] > 40.0),
            "Extruded IFC building vertices must encode height_m for procedural facade shading"
        );
        assert!(
            site_mesh.vertices.iter().any(|v| (v.meta[0] - 2.0).abs() < 0.01 && v.meta[2] < 0.65),
            "Building ground vertices must encode base Ambient Occlusion"
        );
    }
}
