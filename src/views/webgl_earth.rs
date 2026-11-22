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

use crate::core::bim::{BimElementKind, BimProject};
use crate::core::gis::city_builder::{IfcxCityScene, IfcxZoneKind};
use crate::core::gis::district_hazard::{
    evaluate_seismic_wave_displacement_m, DistrictHazardConfig, DistrictMultiHazardReport,
};
use crate::core::gis::earth_globe::{
    lat_lon_to_unit_sphere, GlobeCamera, GlobeScreenTransform, PlanetaryWorldState,
};
use crate::core::gis::map_widget::{SlippyMapState, VectorEarthTheme, EARTH_RADIUS_METERS};
use crate::core::gis::neighborhood_physics::{NeighborhoodAuditReport, PoundingSeverity};
use crate::core::gis::tin_surface::TinSurfaceMesh;
use crate::core::gis::vector_layers::{UrbanVectorKind, UrbanVectorLayerCache};
use crate::core::gis::world_index::WorldSpatialIndex;
use crate::core::ifcx::{IfcxDocument, IfcxStage};
use crate::models::StructuralMember;
use crate::views::universal_canvas::EarthBimLens;

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

/// Converts a pure-vector `earth_globe.ifcx` document (`usd::usdgeom::mesh` nodes) directly 1:1
/// into a GPU-ready `GpuMeshPayload` VBO/IBO buffer with zero raster pixels.
pub fn build_gpu_mesh_from_ifcx_earth_layer(doc: &IfcxDocument) -> GpuMeshPayload {
    let mut vertices = Vec::with_capacity(14_000);
    let mut indices = Vec::with_capacity(32_000);

    for node in &doc.data {
        let Some(mesh) = node.mesh() else {
            continue;
        };
        if mesh.points.is_empty() {
            continue;
        }
        let base_v = vertices.len() as u32;
        let default_col = mesh.display_color.unwrap_or([0.45, 0.60, 0.72, 1.0]);
        let default_meta = [mesh.material_kind, 0.0, 1.0, 0.0];

        for (i, p) in mesh.points.iter().enumerate() {
            let n = mesh.normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]);
            let c = mesh.vertex_colors.get(i).copied().unwrap_or(default_col);
            let m = mesh.vertex_meta.get(i).copied().unwrap_or(default_meta);
            vertices.push(GpuEarthVertex {
                pos: [p[0] as f32, p[1] as f32, p[2] as f32],
                normal: [n[0] as f32, n[1] as f32, n[2] as f32],
                color: c,
                meta: m,
            });
        }
        for [i0, i1, i2] in mesh.triangulated_indices() {
            if i0 < mesh.points.len() && i1 < mesh.points.len() && i2 < mesh.points.len() {
                indices.push(base_v + i0 as u32);
                indices.push(base_v + i1 as u32);
                indices.push(base_v + i2 as u32);
            }
        }
    }

    GpuMeshPayload { vertices, indices }
}

/// Builds the complete static GPU mesh for the 3D Pure-Rust Vector Earth Globe (Pass 1)
/// directly from the live composable `earth_globe.ifcx` IFCX (IFC 5) document:
/// - High-resolution 64×32 UV-sphere ocean dome (`/earth/globe/ocean`, `material_kind = 0.0`)
/// - Spherical extruded continents & Indonesian archipelago (`/earth/globe/landmasses/*`, `material_kind = 1.0`)
/// - 3D vector mountain ridge spines & major hydrological rivers (`/earth/globe/ridges/*`, `/earth/globe/rivers/*`, `material_kind = 3.0`)
pub fn build_planetary_vector_globe_mesh(theme: VectorEarthTheme) -> GpuMeshPayload {
    let earth_ifcx = IfcxStage::build_earth_globe_layer(theme);
    build_gpu_mesh_from_ifcx_earth_layer(&earth_ifcx)
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
/// Pushes a pure-vector stylized low-poly 3D tree (trunk + 2-tier conical foliage, 0% raster pixels).
fn push_stylized_tree_enu(
    verts: &mut Vec<GpuEarthVertex>,
    idxs: &mut Vec<u32>,
    cx: f32,
    cy: f32,
    base_z: f32,
    total_h: f32,
    theme: VectorEarthTheme,
) {
    let trunk_h = (total_h * 0.34).clamp(1.2, 3.4);
    let trunk_r = (total_h * 0.045).clamp(0.18, 0.42);
    let trunk_col = [0.38_f32, 0.27, 0.19, 1.0];
    let foliage_col = match theme {
        VectorEarthTheme::CoastalBlueprint => [0.22_f32, 0.56, 0.50, 1.0],
        VectorEarthTheme::CityDaylight => [0.24_f32, 0.62, 0.36, 1.0],
        VectorEarthTheme::NightTwin => [0.14_f32, 0.38, 0.36, 1.0],
        VectorEarthTheme::Topographic => [0.38_f32, 0.56, 0.42, 1.0],
    };

    const SIDES: usize = 6;
    // Trunk prism
    for i in 0..SIDES {
        let a0 = (i as f32 / SIDES as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32 / SIDES as f32) * std::f32::consts::TAU;
        let (c0, s0) = (a0.cos(), a0.sin());
        let (c1, s1) = (a1.cos(), a1.sin());
        let nx = (c0 + c1) * 0.5;
        let ny = (s0 + s1) * 0.5;
        let b = verts.len() as u32;
        for (x, y, z, ao) in [
            (cx + c0 * trunk_r, cy + s0 * trunk_r, base_z, 0.65_f32),
            (cx + c1 * trunk_r, cy + s1 * trunk_r, base_z, 0.65),
            (cx + c1 * trunk_r * 0.8, cy + s1 * trunk_r * 0.8, base_z + trunk_h, 0.92),
            (cx + c0 * trunk_r * 0.8, cy + s0 * trunk_r * 0.8, base_z + trunk_h, 0.92),
        ] {
            verts.push(GpuEarthVertex {
                pos: [x, y, z],
                normal: [nx, ny, 0.0],
                color: trunk_col,
                meta: [3.0, z, ao, 0.0],
            });
        }
        idxs.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
    }

    // Two-tier conical foliage crown
    for (tier, (z_bot, z_top, rad, shade)) in [
        (base_z + trunk_h * 0.85, base_z + total_h * 0.78, total_h * 0.28, 0.92_f32),
        (base_z + total_h * 0.48, base_z + total_h, total_h * 0.21, 1.05),
    ]
    .iter()
    .enumerate()
    {
        let _ = tier;
        let col = [
            (foliage_col[0] * shade).clamp(0.0, 1.0),
            (foliage_col[1] * shade).clamp(0.0, 1.0),
            (foliage_col[2] * shade).clamp(0.0, 1.0),
            1.0,
        ];
        for i in 0..SIDES {
            let a0 = (i as f32 / SIDES as f32) * std::f32::consts::TAU;
            let a1 = ((i + 1) as f32 / SIDES as f32) * std::f32::consts::TAU;
            let am = 0.5 * (a0 + a1);
            let n = [am.cos() * 0.85, am.sin() * 0.85, 0.52];
            let b = verts.len() as u32;
            verts.push(GpuEarthVertex {
                pos: [cx + a0.cos() * rad, cy + a0.sin() * rad, *z_bot],
                normal: n,
                color: col,
                meta: [3.0, *z_bot, 0.80, 0.0],
            });
            verts.push(GpuEarthVertex {
                pos: [cx + a1.cos() * rad, cy + a1.sin() * rad, *z_bot],
                normal: n,
                color: col,
                meta: [3.0, *z_bot, 0.80, 0.0],
            });
            verts.push(GpuEarthVertex {
                pos: [cx, cy, *z_top],
                normal: [0.0, 0.0, 1.0],
                color: mix_rgb(col, [0.92, 0.98, 0.94, 1.0], 0.14),
                meta: [3.0, *z_top, 1.0, 0.0],
            });
            idxs.extend_from_slice(&[b, b + 1, b + 2]);
        }
    }
}

/// Pushes an oriented 3D rectangular prism between bottom elevation `z0` and top `z1` (`material_kind = 3.0`).
#[allow(clippy::too_many_arguments)]
fn push_box_prism_enu(
    verts: &mut Vec<GpuEarthVertex>,
    idxs: &mut Vec<u32>,
    fp: &[[f64; 2]; 4],
    z0: f32,
    z1: f32,
    col: [f32; 4],
    meta_kind: f32,
) {
    let top_col = mix_rgb(col, [0.96, 0.98, 1.0, 1.0], 0.12);
    // 4 vertical faces
    for i in 0..4 {
        let a = fp[i];
        let b = fp[(i + 1) % 4];
        let dx = (b[0] - a[0]) as f32;
        let dy = (b[1] - a[1]) as f32;
        let len = dx.hypot(dy).max(1e-4);
        let nx = dy / len;
        let ny = -dx / len;
        let base = verts.len() as u32;
        for (px, py, pz, ao) in [
            (a[0] as f32, a[1] as f32, z0, 0.72_f32),
            (b[0] as f32, b[1] as f32, z0, 0.72),
            (b[0] as f32, b[1] as f32, z1, 1.0),
            (a[0] as f32, a[1] as f32, z1, 1.0),
        ] {
            verts.push(GpuEarthVertex {
                pos: [px, py, pz],
                normal: [nx, ny, 0.0],
                color: col,
                meta: [meta_kind, pz.max(0.0), ao, 0.0],
            });
        }
        idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    // Top cap
    let tb = verts.len() as u32;
    for p in fp {
        verts.push(GpuEarthVertex {
            pos: [p[0] as f32, p[1] as f32, z1],
            normal: [0.0, 0.0, 1.0],
            color: top_col,
            meta: [meta_kind, z1.max(0.0), 1.0, 0.0],
        });
    }
    idxs.extend_from_slice(&[tb, tb + 1, tb + 2, tb, tb + 2, tb + 3]);
}

/// Pushes an oriented 3D rectangular prism strut between arbitrary 3D ENU endpoints
/// (used for inclined Buckling-Restrained Braces `BRB` and sloped structural members).
#[allow(clippy::too_many_arguments)]
fn push_inclined_3d_strut_enu(
    verts: &mut Vec<GpuEarthVertex>,
    idxs: &mut Vec<u32>,
    p0_enu: [f64; 3],
    p1_enu: [f64; 3],
    width_m: f64,
    depth_m: f64,
    col: [f32; 4],
) {
    let dx = p1_enu[0] - p0_enu[0];
    let dy = p1_enu[1] - p0_enu[1];
    let dz = p1_enu[2] - p0_enu[2];
    let len = (dx * dx + dy * dy + dz * dz).sqrt();
    if len < 1e-4 {
        return;
    }
    let u = [dx / len, dy / len, dz / len];
    let ref_up = if u[2].abs() < 0.92 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let cross = |a: [f64; 3], b: [f64; 3]| -> [f64; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let norm = |v: [f64; 3]| -> [f64; 3] {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9);
        [v[0] / l, v[1] / l, v[2] / l]
    };
    let side = norm(cross(u, ref_up));
    let up_perp = norm(cross(side, u));
    let hw = (width_m * 0.5).max(0.06);
    let hd = (depth_m * 0.5).max(0.06);

    let offset_pt = |p: [f64; 3], sx: f64, sy: f64| -> [f32; 3] {
        [
            (p[0] + side[0] * sx * hw + up_perp[0] * sy * hd) as f32,
            (p[1] + side[1] * sx * hw + up_perp[1] * sy * hd) as f32,
            (p[2] + side[2] * sx * hw + up_perp[2] * sy * hd) as f32,
        ]
    };
    let a = [
        offset_pt(p0_enu, -1.0, -1.0),
        offset_pt(p0_enu, 1.0, -1.0),
        offset_pt(p0_enu, 1.0, 1.0),
        offset_pt(p0_enu, -1.0, 1.0),
    ];
    let b = [
        offset_pt(p1_enu, -1.0, -1.0),
        offset_pt(p1_enu, 1.0, -1.0),
        offset_pt(p1_enu, 1.0, 1.0),
        offset_pt(p1_enu, -1.0, 1.0),
    ];

    let side_normals = [
        [-up_perp[0] as f32, -up_perp[1] as f32, -up_perp[2] as f32],
        [side[0] as f32, side[1] as f32, side[2] as f32],
        [up_perp[0] as f32, up_perp[1] as f32, up_perp[2] as f32],
        [-side[0] as f32, -side[1] as f32, -side[2] as f32],
    ];
    let top_col = mix_rgb(col, [1.0, 0.96, 0.86, 1.0], 0.14);
    for i in 0..4 {
        let ni = (i + 1) % 4;
        let base = verts.len() as u32;
        let face_col = if i == 2 { top_col } else { col };
        for (pos, ao) in [
            (a[i], 0.82_f32),
            (a[ni], 0.82),
            (b[ni], 1.0),
            (b[i], 1.0),
        ] {
            verts.push(GpuEarthVertex {
                pos,
                normal: side_normals[i],
                color: face_col,
                meta: [3.0, pos[2].max(0.0), ao, 0.0],
            });
        }
        idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    // Start & End Caps
    for (pts, n_dir) in [
        ([a[0], a[3], a[2], a[1]], [-u[0] as f32, -u[1] as f32, -u[2] as f32]),
        ([b[0], b[1], b[2], b[3]], [u[0] as f32, u[1] as f32, u[2] as f32]),
    ] {
        let cap_base = verts.len() as u32;
        for pos in pts {
            verts.push(GpuEarthVertex {
                pos,
                normal: n_dir,
                color: col,
                meta: [3.0, pos[2].max(0.0), 0.90, 0.0],
            });
        }
        idxs.extend_from_slice(&[cap_base, cap_base + 1, cap_base + 2, cap_base, cap_base + 2, cap_base + 3]);
    }
}

/// Appends authored IFCX City-Builder objects (`IfcRoad`, `IfcBridge`, `IfcSpatialZone`,
/// context towers, instanced stylized trees, and `IfcGradedPad`) into the RTC site mesh.
fn push_city_builder_scene_enu(
    verts: &mut Vec<GpuEarthVertex>,
    idxs: &mut Vec<u32>,
    theme: VectorEarthTheme,
    city: &IfcxCityScene,
    wall_base: [f32; 4],
    roof_base: [f32; 4],
) {
    let asphalt_col = match theme {
        VectorEarthTheme::CoastalBlueprint => [0.14_f32, 0.22, 0.30, 1.0],
        VectorEarthTheme::CityDaylight => [0.24_f32, 0.26, 0.30, 1.0],
        VectorEarthTheme::NightTwin => [0.09_f32, 0.12, 0.18, 1.0],
        VectorEarthTheme::Topographic => [0.34_f32, 0.36, 0.38, 1.0],
    };
    let curb_col = [0.76_f32, 0.80, 0.82, 1.0];
    let stripe_col = if theme == VectorEarthTheme::NightTwin {
        [0.98_f32, 0.86, 0.36, 1.0]
    } else {
        [0.94_f32, 0.94, 0.88, 1.0]
    };

    // 1. Authored `IfcRoad` & `IfcBridge` spans
    for road in &city.roads {
        let fp = road.footprint_enu_m();
        let dx = road.end_enu_m[0] - road.start_enu_m[0];
        let dy = road.end_enu_m[1] - road.start_enu_m[1];
        let len = dx.hypot(dy).max(1.0);
        let ux = dx / len;
        let uy = dy / len;
        let nx = -uy;
        let ny = ux;

        if road.is_bridge {
            let deck_top = road.deck_elev_m as f32;
            let deck_bot = (road.deck_elev_m - 0.85) as f32;
            push_box_prism_enu(verts, idxs, &fp, deck_bot, deck_top, curb_col, 3.0);
            // Asphalt roadway surface slightly raised inside bridge parapets
            let inner_hw = (road.width_m * 0.5 - 0.65).max(1.2);
            let asp_fp = [
                [road.start_enu_m[0] - nx * inner_hw, road.start_enu_m[1] - ny * inner_hw],
                [road.end_enu_m[0] - nx * inner_hw, road.end_enu_m[1] - ny * inner_hw],
                [road.end_enu_m[0] + nx * inner_hw, road.end_enu_m[1] + ny * inner_hw],
                [road.start_enu_m[0] + nx * inner_hw, road.start_enu_m[1] + ny * inner_hw],
            ];
            push_box_prism_enu(verts, idxs, &asp_fp, deck_top, deck_top + 0.08, asphalt_col, 3.0);

            // Bridge piers every `pier_spacing_m` down to subsurface foundations
            let span_count = ((len / road.pier_spacing_m.max(8.0)).ceil() as usize).clamp(2, 24);
            for k in 1..span_count {
                let frac = k as f64 / span_count as f64;
                let px = road.start_enu_m[0] + dx * frac;
                let py = road.start_enu_m[1] + dy * frac;
                let pr = 0.75_f64;
                for side in [-0.32, 0.32] {
                    let cx = px + nx * road.width_m * side;
                    let cy = py + ny * road.width_m * side;
                    let pier_fp = [
                        [cx - pr, cy - pr],
                        [cx + pr, cy - pr],
                        [cx + pr, cy + pr],
                        [cx - pr, cy + pr],
                    ];
                    push_box_prism_enu(verts, idxs, &pier_fp, -1.5, deck_bot, [0.58, 0.63, 0.68, 1.0], 3.0);
                }
            }
        } else {
            // Sidewalk base + asphalt carriageway + center lane markings
            let sw_hw = road.width_m * 0.5 + 1.2;
            let sw_fp = [
                [road.start_enu_m[0] - nx * sw_hw, road.start_enu_m[1] - ny * sw_hw],
                [road.end_enu_m[0] - nx * sw_hw, road.end_enu_m[1] - ny * sw_hw],
                [road.end_enu_m[0] + nx * sw_hw, road.end_enu_m[1] + ny * sw_hw],
                [road.start_enu_m[0] + nx * sw_hw, road.start_enu_m[1] + ny * sw_hw],
            ];
            push_box_prism_enu(verts, idxs, &sw_fp, 0.0, 0.12, curb_col, 3.0);
            push_box_prism_enu(verts, idxs, &fp, 0.04, 0.16, asphalt_col, 3.0);

            // Dashed center lane stripe
            let dash_step = 6.0_f64;
            let n_dashes = (len / dash_step).floor() as usize;
            for d in 0..n_dashes.min(120) {
                let s0 = d as f64 * dash_step + 1.2;
                let s1 = (s0 + 2.8).min(len - 0.8);
                if s1 <= s0 {
                    continue;
                }
                let shw = 0.14;
                let p0x = road.start_enu_m[0] + ux * s0;
                let p0y = road.start_enu_m[1] + uy * s0;
                let p1x = road.start_enu_m[0] + ux * s1;
                let p1y = road.start_enu_m[1] + uy * s1;
                let dash_fp = [
                    [p0x - nx * shw, p0y - ny * shw],
                    [p1x - nx * shw, p1y - ny * shw],
                    [p1x + nx * shw, p1y + ny * shw],
                    [p0x + nx * shw, p0y + ny * shw],
                ];
                push_box_prism_enu(verts, idxs, &dash_fp, 0.16, 0.19, stripe_col, 3.0);
            }
        }
    }

    // 2. Authored `IfcSpatialZone` parcels, optional context towers & instanced stylized trees
    for zone in &city.zones {
        let fp = zone.footprint_enu_m();
        let zcol = mix_rgb(zone.kind.fill_rgba(), [0.9, 0.94, 0.96, 1.0], 0.12);
        push_box_prism_enu(verts, idxs, &fp, 0.0, 0.16, zcol, 1.0);

        if zone.synthesize_context_towers && zone.kind != IfcxZoneKind::Park {
            let rad = zone.rotation_deg.to_radians();
            let (sr, cr) = (rad.sin(), rad.cos());
            let bw = (zone.width_m * 0.56).max(8.0);
            let bd = (zone.depth_m * 0.56).max(8.0);
            let hw = bw * 0.5;
            let hd = bd * 0.5;
            let tfp: Vec<[f64; 2]> = [[-hw, -hd], [hw, -hd], [hw, hd], [-hw, hd]]
                .iter()
                .map(|[lx, ly]| [zone.center_enu_m[0] + lx * cr - ly * sr, zone.center_enu_m[1] + lx * sr + ly * cr])
                .collect();
            let h_m = match zone.kind {
                IfcxZoneKind::Commercial => 28.0 + ((zone.id * 7) % 18) as f32,
                IfcxZoneKind::Civic => 18.0,
                IfcxZoneKind::Residential => 14.0 + ((zone.id * 5) % 10) as f32,
                IfcxZoneKind::Industrial => 10.5,
                IfcxZoneKind::Park => 0.0,
            };
            let twall = mix_rgb(wall_base, zone.kind.fill_rgba(), 0.22);
            let troof = mix_rgb(roof_base, zone.kind.fill_rgba(), 0.15);
            push_extruded_building_prism_enu(verts, idxs, &tfp, 0.16, h_m, twall, troof);
        }

        for [tx, ty, th] in zone.instanced_trees_enu_m() {
            push_stylized_tree_enu(verts, idxs, tx as f32, ty as f32, 0.16, th as f32, theme);
        }
    }

    // 3. Authored `IfcGradedPad` foundation aprons
    for pad in &city.graded_pads {
        let hw = pad.width_m * 0.5;
        let hd = pad.depth_m * 0.5;
        let pfp = [
            [pad.center_enu_m[0] - hw, pad.center_enu_m[1] - hd],
            [pad.center_enu_m[0] + hw, pad.center_enu_m[1] - hd],
            [pad.center_enu_m[0] + hw, pad.center_enu_m[1] + hd],
            [pad.center_enu_m[0] - hw, pad.center_enu_m[1] + hd],
        ];
        push_box_prism_enu(verts, idxs, &pfp, -0.06, 0.08, [0.58, 0.56, 0.50, 1.0], 1.0);
    }
}

/// Dynamic active 3D BIM + 6-DOF FEM + Modal + N-1 disruption input for unified WebGL2 Pass 2B.
pub struct ActiveBimGpuInput<'a> {
    pub project: &'a BimProject,
    pub members: &'a [StructuralMember],
    pub lens: EarthBimLens,
    pub erection_day_limit: usize,
    pub deform_scale: f64,
    pub active_modal_mode: usize,
    pub modal_phase_sin: f32,
    pub n1_removed_bim: Option<(BimElementKind, usize)>,
    pub cascade_failed_bim: &'a [(BimElementKind, usize)],
    pub cascade_step_progress: f32,
    pub heading_deg: f64,
}

fn structural_dcr_color(dcr: f64) -> [f32; 4] {
    if dcr >= 1.0 {
        [0.94, 0.24, 0.24, 1.0]
    } else if dcr >= 0.80 {
        [0.96, 0.62, 0.18, 1.0]
    } else if dcr >= 0.55 {
        [0.24, 0.72, 0.88, 1.0]
    } else {
        [0.16, 0.78, 0.56, 1.0]
    }
}

/// Builds the unified 3D WebGL2 mesh for the active editable `BimProject` inside the shared
/// 24-bit hardware Z-buffered RTC scene:
/// - Subsurface 3D pile caps + 4 deep foundation pile shafts beneath every `BimFooting`
/// - Columns, beams/runs, walls, and floor/roof slabs colored by active structural lens
///   (`Clean`, `DcrHeatmap`, `AriRedundancy`, `MomentStress`, `Erection4D`)
/// - 60 FPS animated `DeformedShape` & `ModalVibration` displacements ($\mathbf{p}(t) = \mathbf{p}_0 + s \sin(\omega t)\,\boldsymbol{\phi}_k$)
/// - Translucent crimson `N-1` disruption ghost on the removed structural member
pub fn build_rtc_active_bim_fem_mesh(input: &ActiveBimGpuInput<'_>) -> GpuMeshPayload {
    let proj = input.project;
    if proj.is_empty() {
        return GpuMeshPayload::default();
    }
    let mut verts = Vec::with_capacity(2_048);
    let mut idxs = Vec::with_capacity(4_096);

    let level_elev_m = |lvl_id: usize| -> f64 {
        proj.levels
            .iter()
            .find(|l| l.id == lvl_id)
            .map(|l| l.elevation_mm * 0.001)
            .unwrap_or(0.0)
    };
    let max_z_m = proj
        .levels
        .iter()
        .map(|l| l.elevation_mm * 0.001)
        .fold(3.6_f64, f64::max)
        .max(3.0);

    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());

    // Live structural displacement field for DeformedShape / ModalVibration (meters in ENU)
    let anim_amp_m = match input.lens {
        EarthBimLens::DeformedShape => (input.deform_scale * 0.012).clamp(0.05, 1.8),
        EarthBimLens::ModalVibration => {
            (input.deform_scale * 0.016).clamp(0.08, 2.2) * (input.modal_phase_sin as f64)
        }
        _ => 0.0,
    };
    let mode_idx = input.active_modal_mode % 3;
    let deform_offset = |x_m: f64, y_m: f64, z_m: f64| -> [f64; 3] {
        if anim_amp_m.abs() < 1e-6 || z_m <= 0.05 {
            return [0.0, 0.0, 0.0];
        }
        let xi = (z_m / max_z_m).clamp(0.0, 1.2);
        match mode_idx {
            0 => {
                // Fundamental translational sway along local X
                let phi = (std::f64::consts::FRAC_PI_2 * xi).sin();
                [anim_amp_m * phi, 0.0, 0.0]
            }
            1 => {
                // Second mode orthogonal sway along local Y
                let phi = (std::f64::consts::FRAC_PI_2 * xi).sin();
                [0.0, anim_amp_m * phi, 0.0]
            }
            _ => {
                // Third mode torsional twist about vertical Z axis
                let phi = xi * xi;
                let theta = anim_amp_m * 0.05 * phi;
                [-y_m * theta, x_m * theta, 0.0]
            }
        }
    };

    let rot_xy = |x_m: f64, y_m: f64, z_m: f64| -> [f64; 2] {
        let d = deform_offset(x_m, y_m, z_m);
        let lx = x_m + d[0];
        let ly = y_m + d[1];
        [lx * cr - ly * sr, lx * sr + ly * cr]
    };

    // Average structural health metrics for lens coloring
    let avg_dcr = if !input.members.is_empty() {
        input
            .members
            .iter()
            .map(|m| m.utilization.max(m.dcr_interaction))
            .fold(0.0_f64, f64::max)
    } else {
        0.45
    };
    let avg_ari = if !input.members.is_empty() {
        input.members.iter().map(|m| m.ari).fold(1.0_f64, f64::min)
    } else {
        0.92
    };

    let elem_color = |kind: BimElementKind, bim_id: usize, default_col: [f32; 4]| -> [f32; 4] {
        if input.cascade_failed_bim.contains(&(kind, bim_id)) {
            return [0.98, 0.22, 0.18, 0.36]; // Translucent crimson-orange progressive collapse ghost
        }
        if input.n1_removed_bim == Some((kind, bim_id)) {
            return [0.96, 0.16, 0.24, 0.42]; // Crimson translucent N-1 removed ghost
        }
        // Match corresponding StructuralMember by id/index if available
        let member_opt = input.members.get(bim_id % input.members.len().max(1));
        match input.lens {
            EarthBimLens::Clean | EarthBimLens::Erection4D => default_col,
            EarthBimLens::DcrHeatmap | EarthBimLens::DeformedShape | EarthBimLens::ModalVibration => {
                let dcr = member_opt
                    .map(|m| m.utilization.max(m.dcr_interaction))
                    .unwrap_or(avg_dcr);
                structural_dcr_color(dcr)
            }
            EarthBimLens::AriRedundancy => {
                let ari = member_opt.map(|m| m.ari).unwrap_or(avg_ari);
                structural_dcr_color(1.0 - ari)
            }
            EarthBimLens::MomentStress => {
                let mom = member_opt
                    .map(|m| (m.moment_my_knm.hypot(m.moment_mz_knm) / 250.0).clamp(0.0, 1.2))
                    .unwrap_or(0.45);
                structural_dcr_color(mom)
            }
        }
    };

    // 1. Subsurface 3D Pile Caps & 4 Deep Pile Shafts (`BimFooting`)
    for (idx, ftg) in proj.footings.iter().enumerate() {
        if input.lens == EarthBimLens::Erection4D && idx + 1 > input.erection_day_limit {
            continue;
        }
        let z_top = level_elev_m(ftg.level_id) as f32;
        let z_bot = z_top - (ftg.thickness_mm * 0.001) as f32;
        let hw = ftg.width_mm * 0.0005;
        let hl = ftg.length_mm * 0.0005;
        let cx = ftg.pos.0 * 0.001;
        let cy = ftg.pos.1 * 0.001;
        let fp = [
            rot_xy(cx - hw, cy - hl, 0.0),
            rot_xy(cx + hw, cy - hl, 0.0),
            rot_xy(cx + hw, cy + hl, 0.0),
            rot_xy(cx - hw, cy + hl, 0.0),
        ];
        let col = elem_color(BimElementKind::Footing, ftg.id, [0.48, 0.54, 0.60, 1.0]);
        push_box_prism_enu(&mut verts, &mut idxs, &fp, z_bot, z_top, col, 3.0);

        // 4 subsurface concrete pile shafts extending 4.0m below pile cap
        let pr = (ftg.width_mm * 0.00012).clamp(0.16, 0.35);
        for (sx, sy) in [(-0.55, -0.55), (0.55, -0.55), (0.55, 0.55), (-0.55, 0.55)] {
            let px = cx + sx * hw;
            let py = cy + sy * hl;
            let pfp = [
                rot_xy(px - pr, py - pr, 0.0),
                rot_xy(px + pr, py - pr, 0.0),
                rot_xy(px + pr, py + pr, 0.0),
                rot_xy(px - pr, py + pr, 0.0),
            ];
            push_box_prism_enu(
                &mut verts,
                &mut idxs,
                &pfp,
                z_bot - 4.0,
                z_bot,
                [0.36, 0.42, 0.48, 1.0],
                3.0,
            );
        }
    }

    // 2. Structural Columns (`BimColumn`) with live top/bottom modal shear displacement
    let domino_sag_m = (input.cascade_step_progress.clamp(0.15, 1.0) as f64) * 0.45;
    for (idx, c) in proj.columns.iter().enumerate() {
        if input.lens == EarthBimLens::Erection4D && idx + 5 > input.erection_day_limit {
            continue;
        }
        let z0 = level_elev_m(c.base_level_id);
        let sag = if input.cascade_failed_bim.contains(&(BimElementKind::Column, c.id)) {
            domino_sag_m * 0.65
        } else {
            0.0
        };
        let z1 = (level_elev_m(c.top_level_id) - sag).max(z0 + 0.4);
        let hw = c.width_mm * 0.0005;
        let hd = c.depth_mm * 0.0005;
        let cx = c.pos.0 * 0.001;
        let cy = c.pos.1 * 0.001;
        let z_mid = 0.5 * (z0 + z1);
        let fp = [
            rot_xy(cx - hw, cy - hd, z_mid),
            rot_xy(cx + hw, cy - hd, z_mid),
            rot_xy(cx + hw, cy + hd, z_mid),
            rot_xy(cx - hw, cy + hd, z_mid),
        ];
        let default_col = if c.profile_name.contains("Jacket") {
            [0.22, 0.82, 0.68, 1.0]
        } else {
            [0.35, 0.62, 0.74, 1.0]
        };
        let col = elem_color(BimElementKind::Column, c.id, default_col);
        push_box_prism_enu(&mut verts, &mut idxs, &fp, z0 as f32, z1 as f32, col, 3.0);
    }

    // 3. Structural Beams & Inclined BRB Braces (`BimRun`)
    for (idx, r) in proj.runs.iter().enumerate() {
        if input.lens == EarthBimLens::Erection4D && idx + 12 > input.erection_day_limit {
            continue;
        }
        let is_inclined_brace = r.category == crate::core::bim::RunCategory::StructuralBrace
            || (r.end_offset_mm - r.start_offset_mm).abs() > 10.0;
        let sag = if input.cascade_failed_bim.contains(&(BimElementKind::Run, r.id)) {
            domino_sag_m
        } else {
            0.0
        };
        let default_col = if r.category == crate::core::bim::RunCategory::StructuralBrace {
            [0.96, 0.68, 0.16, 1.0]
        } else if r.profile_name.contains("Jacket") {
            [0.22, 0.82, 0.68, 1.0]
        } else {
            [0.46, 0.72, 0.80, 1.0]
        };
        let col = elem_color(BimElementKind::Run, r.id, default_col);
        let ax = r.start.0 * 0.001;
        let ay = r.start.1 * 0.001;
        let bx = r.end.0 * 0.001;
        let by = r.end.1 * 0.001;
        if is_inclined_brace {
            let z0 = level_elev_m(r.level_id) + r.start_offset_mm * 0.001 - sag * 0.5;
            let z1 = level_elev_m(r.level_id) + r.end_offset_mm * 0.001 - sag;
            let p0_xy = rot_xy(ax, ay, z0);
            let p1_xy = rot_xy(bx, by, z1);
            push_inclined_3d_strut_enu(
                &mut verts,
                &mut idxs,
                [p0_xy[0], p0_xy[1], z0],
                [p1_xy[0], p1_xy[1], z1],
                (r.width_mm * 0.001).max(0.18),
                (r.depth_mm * 0.001).max(0.18),
                col,
            );
            continue;
        }
        let z_top = (level_elev_m(r.level_id) + r.start_offset_mm * 0.001 - sag).max(0.15);
        let z_bot = z_top - (r.depth_mm * 0.001).max(0.2);
        let dx = bx - ax;
        let dy = by - ay;
        let len = dx.hypot(dy).max(0.05);
        let hw = (r.width_mm * 0.0005).max(0.10);
        let nx = -dy / len * hw;
        let ny = dx / len * hw;
        let fp = [
            rot_xy(ax - nx, ay - ny, z_top),
            rot_xy(bx - nx, by - ny, z_top),
            rot_xy(bx + nx, by + ny, z_top),
            rot_xy(ax + nx, ay + ny, z_top),
        ];
        push_box_prism_enu(&mut verts, &mut idxs, &fp, z_bot as f32, z_top as f32, col, 3.0);
    }

    // 4. Compound & Shear Walls (`BimWall`)
    for (idx, w) in proj.walls.iter().enumerate() {
        if input.lens == EarthBimLens::Erection4D && idx + 20 > input.erection_day_limit {
            continue;
        }
        let z0 = level_elev_m(w.base_level_id);
        let sag = if input.cascade_failed_bim.contains(&(BimElementKind::Wall, w.id)) {
            domino_sag_m * 0.65
        } else {
            0.0
        };
        let z1 = (w
            .top_level_id
            .map(level_elev_m)
            .unwrap_or(z0 + w.unconnected_height_mm * 0.001)
            - sag)
            .max(z0 + 0.45);
        let ax = w.start.0 * 0.001;
        let ay = w.start.1 * 0.001;
        let bx = w.end.0 * 0.001;
        let by = w.end.1 * 0.001;
        let dx = bx - ax;
        let dy = by - ay;
        let len = dx.hypot(dy).max(0.05);
        let ht = (w.thickness_mm * 0.0005).max(0.08);
        let nx = -dy / len * ht;
        let ny = dx / len * ht;
        let z_mid = 0.5 * (z0 + z1);
        let fp = [
            rot_xy(ax - nx, ay - ny, z_mid),
            rot_xy(bx - nx, by - ny, z_mid),
            rot_xy(bx + nx, by + ny, z_mid),
            rot_xy(ax + nx, ay + ny, z_mid),
        ];
        let default_w = if w.name.contains("Retrofit") {
            [0.22, 0.82, 0.68, 0.94]
        } else if w.is_structural {
            [0.64, 0.74, 0.80, 0.92]
        } else {
            [0.80, 0.84, 0.88, 0.88]
        };
        let col = elem_color(BimElementKind::Wall, w.id, default_w);
        push_box_prism_enu(&mut verts, &mut idxs, &fp, z0 as f32, z1 as f32, col, 2.0);
    }

    // 5. Floor & Roof Slabs (`BimSlab`)
    for (idx, s) in proj.slabs.iter().enumerate() {
        if input.lens == EarthBimLens::Erection4D && idx + 16 > input.erection_day_limit {
            continue;
        }
        if s.boundary.len() < 3 {
            continue;
        }
        let z_lvl = level_elev_m(s.level_id);
        let th = (s.thickness_mm * 0.001).max(0.12) as f32;
        let base_z = if s.is_roof { z_lvl as f32 } else { z_lvl as f32 - th };
        let fp: Vec<[f64; 2]> = s
            .boundary
            .iter()
            .map(|p| rot_xy(p.0 * 0.001, p.1 * 0.001, z_lvl))
            .collect();
        let col = elem_color(
            BimElementKind::Slab,
            s.id,
            if s.is_roof {
                [0.54, 0.68, 0.76, 0.88]
            } else {
                [0.68, 0.76, 0.82, 0.82]
            },
        );
        push_extruded_building_prism_enu(&mut verts, &mut idxs, &fp, base_z, th, col, col);
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// District multi-hazard input bundle rendered in the shared 24-bit Z-buffered WebGL2 RTC pass.
pub struct DistrictHazardGpuInput<'a> {
    pub config: &'a DistrictHazardConfig,
    pub report: &'a DistrictMultiHazardReport,
    pub neighborhood: &'a NeighborhoodAuditReport,
    pub origin_lat_lon: (f64, f64),
    pub active_building_height_m: f64,
    pub active_site_tin: Option<&'a TinSurfaceMesh>,
}

/// Builds the Floating-Origin RTC 3D GPU mesh for animated SNI 1726 seismic wave rings,
/// inter-building seismic pounding corridors, Venturi urban wind streamlines, and 3D flood
/// sea-level plane against `IfcSite` TIN contours.
pub fn build_rtc_district_hazard_mesh(input: &DistrictHazardGpuInput<'_>) -> GpuMeshPayload {
    let cfg = input.config;
    if !cfg.any_active() {
        return GpuMeshPayload::default();
    }

    let mut verts = Vec::with_capacity(1_536);
    let mut idxs = Vec::with_capacity(3_072);

    let push_quad_double = |vs: &mut Vec<GpuEarthVertex>,
                            is: &mut Vec<u32>,
                            p0: [f32; 3],
                            p1: [f32; 3],
                            p2: [f32; 3],
                            p3: [f32; 3],
                            n: [f32; 3],
                            col: [f32; 4],
                            kind: f32| {
        let base = vs.len() as u32;
        for &p in &[p0, p1, p2, p3] {
            vs.push(GpuEarthVertex {
                pos: p,
                normal: n,
                color: col,
                meta: [kind, p[2], 1.0, 0.0],
            });
        }
        is.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    };

    // 1. Traveling SNI 1726 / ASCE 7 Seismic Wave Rings
    if cfg.seismic_wave_active {
        let segs = 48usize;
        let ring_count = 6usize;
        let ring_w = 4.5_f64;
        for rk in 0..ring_count {
            let r_center = ((cfg.sim_time_sec.max(0.0) * 42.0 + (rk as f64) * 52.0) % 310.0)
                .clamp(8.0, 310.0);
            let r_in = (r_center - ring_w * 0.5).max(3.5);
            let r_out = r_center + ring_w * 0.5;
            let fade = (1.0 - r_center / 320.0).clamp(0.12, 1.0) as f32;
            let pga_boost = (cfg.seismic_pga_g / 0.5).clamp(0.6, 1.4) as f32;
            let col = [
                0.98,
                (0.58 - 0.34 * (rk as f32 / ring_count as f32)).max(0.16),
                0.14,
                (0.56 * fade * pga_boost).min(0.78),
            ];
            let base = verts.len() as u32;
            for i in 0..=segs {
                let a = (i as f64 / segs as f64) * std::f64::consts::TAU;
                let (sa, ca) = (a.sin(), a.cos());
                let xi = r_in * ca;
                let yi = r_in * sa;
                let xo = r_out * ca;
                let yo = r_out * sa;
                let di = evaluate_seismic_wave_displacement_m(xi, yi, cfg.sim_time_sec, cfg);
                let do_ = evaluate_seismic_wave_displacement_m(xo, yo, cfg.sim_time_sec, cfg);
                let zi = 0.28 + di[2];
                let zo = 0.28 + do_[2];
                verts.push(GpuEarthVertex {
                    pos: [xi as f32 + di[0], yi as f32 + di[1], zi],
                    normal: [0.0, 0.0, 1.0],
                    color: col,
                    meta: [1.0, zi, 1.0, 0.0],
                });
                verts.push(GpuEarthVertex {
                    pos: [xo as f32 + do_[0], yo as f32 + do_[1], zo],
                    normal: [0.0, 0.0, 1.0],
                    color: col,
                    meta: [1.0, zo, 1.0, 0.0],
                });
            }
            for i in 0..segs {
                let k = base + (i as u32) * 2;
                idxs.extend_from_slice(&[k, k + 2, k + 3, k, k + 3, k + 1]);
            }
        }
    }

    // 2. Inter-Building Seismic Pounding Corridors
    if cfg.pounding_overlay_active {
        let pulse = (0.75 + 0.25 * (cfg.sim_time_sec * 5.2).sin()) as f32;
        for c in &input.neighborhood.contacts {
            let a_mm = SlippyMapState::lat_lon_to_world_mm(
                input.origin_lat_lon,
                c.closest_active_ll.0,
                c.closest_active_ll.1,
            );
            let b_mm = SlippyMapState::lat_lon_to_world_mm(
                input.origin_lat_lon,
                c.closest_neighbor_ll.0,
                c.closest_neighbor_ll.1,
            );
            let ax = a_mm[0] * 0.001;
            let ay = a_mm[1] * 0.001;
            let bx = b_mm[0] * 0.001;
            let by = b_mm[1] * 0.001;
            let dx = bx - ax;
            let dy = by - ay;
            let len = dx.hypot(dy).max(0.4);
            let hw = match c.severity {
                PoundingSeverity::PoundingRisk => 1.35,
                PoundingSeverity::TightBuffer => 0.95,
                PoundingSeverity::SafeClearance => 0.60,
            };
            let nx = -dy / len * hw;
            let ny = dx / len * hw;
            let z_link = (input.active_building_height_m.min(c.neighbor_height_m).clamp(4.0, 28.0)
                * 0.65) as f32;
            let col = match c.severity {
                PoundingSeverity::PoundingRisk => [0.96, 0.16, 0.24, (0.82 * pulse).min(0.92)],
                PoundingSeverity::TightBuffer => [0.96, 0.68, 0.16, 0.68],
                PoundingSeverity::SafeClearance => [0.22, 0.80, 0.72, 0.42],
            };
            let fp = [
                [ax - nx, ay - ny],
                [bx - nx, by - ny],
                [bx + nx, by + ny],
                [ax + nx, ay + ny],
            ];
            push_box_prism_enu(&mut verts, &mut idxs, &fp, z_link - 0.35, z_link + 0.35, col, 3.0);

            if c.severity == PoundingSeverity::PoundingRisk {
                let mx = 0.5 * (ax + bx);
                let my = 0.5 * (ay + by);
                let p_fp = [
                    [mx - 0.45, my - 0.45],
                    [mx + 0.45, my - 0.45],
                    [mx + 0.45, my + 0.45],
                    [mx - 0.45, my + 0.45],
                ];
                push_box_prism_enu(
                    &mut verts,
                    &mut idxs,
                    &p_fp,
                    0.0,
                    z_link + 3.5,
                    [0.98, 0.20, 0.28, 0.72],
                    3.0,
                );
            }
        }
    }

    // 3. Animated Venturi Urban Wind Streamlines
    if cfg.venturi_wind_active {
        for (si, seg) in input.report.streamlines.iter().enumerate() {
            let ax = seg.start_enu_m[0];
            let ay = seg.start_enu_m[1];
            let az = seg.start_enu_m[2];
            let bx = seg.end_enu_m[0];
            let by = seg.end_enu_m[1];
            let dx = bx - ax;
            let dy = by - ay;
            let len = dx.hypot(dy).max(1.0);
            let ux = dx / len;
            let uy = dy / len;
            let nx = -uy;
            let ny = ux;
            let speed_gain = if seg.is_corridor_accelerated { 0.48 } else { 0.28 };
            let width_m = if seg.is_corridor_accelerated { 1.15 } else { 0.75 };
            let col = if seg.is_corridor_accelerated {
                [0.22, 0.88, 1.0, 0.80]
            } else {
                [0.72, 0.90, 0.98, 0.48]
            };

            for packet in 0..3 {
                let t = ((cfg.sim_time_sec.max(0.0) * speed_gain
                    + (si as f64) * 0.13
                    + (packet as f64) * 0.333)
                    % 1.0)
                    .clamp(0.05, 0.90);
                let seg_len = if seg.is_corridor_accelerated { 16.0 } else { 22.0 };
                let s0 = t * (len - seg_len).max(4.0);
                let s1 = (s0 + seg_len).min(len);
                let p0 = [
                    (ax + ux * s0 - nx * width_m) as f32,
                    (ay + uy * s0 - ny * width_m) as f32,
                    az as f32,
                ];
                let p1 = [
                    (ax + ux * s1 - nx * width_m * 0.35) as f32,
                    (ay + uy * s1 - ny * width_m * 0.35) as f32,
                    az as f32,
                ];
                let p2 = [
                    (ax + ux * s1 + nx * width_m * 0.35) as f32,
                    (ay + uy * s1 + ny * width_m * 0.35) as f32,
                    az as f32,
                ];
                let p3 = [
                    (ax + ux * s0 + nx * width_m) as f32,
                    (ay + uy * s0 + ny * width_m) as f32,
                    az as f32,
                ];
                push_quad_double(&mut verts, &mut idxs, p0, p1, p2, p3, [0.0, 0.0, 1.0], col, 1.0);
            }
        }
    }

    // 4. Interactive 3D Flood / Sea-Level Plane vs. IfcSite TIN Contours
    if cfg.flood_plane_active {
        let wl = cfg.flood_water_level_m as f32;
        let span = 210.0_f32;
        let grid = 16usize;
        let water_col = [0.12, 0.48, 0.74, 0.58];
        let base_v = verts.len() as u32;
        for gy in 0..=grid {
            let fy = gy as f32 / grid as f32;
            let y = -span + fy * span * 2.0;
            for gx in 0..=grid {
                let fx = gx as f32 / grid as f32;
                let x = -span + fx * span * 2.0;
                verts.push(GpuEarthVertex {
                    pos: [x, y, wl],
                    normal: [0.0, 0.0, 1.0],
                    color: water_col,
                    // `kind = 0.0` uses the GLSL analytical ocean micro-ripple & solar glint shader
                    meta: [0.0, wl, 1.0, 0.0],
                });
            }
        }
        let stride = (grid + 1) as u32;
        for gy in 0..grid as u32 {
            for gx in 0..grid as u32 {
                let i00 = base_v + gy * stride + gx;
                let i10 = i00 + 1;
                let i01 = i00 + stride;
                let i11 = i01 + 1;
                idxs.extend_from_slice(&[i00, i10, i11, i00, i11, i01]);
            }
        }

        // Crisp shoreline foam ribbons wherever IfcSite TIN triangles cross `water_level_m`
        if let Some(tin) = input.active_site_tin {
            let foam_col = [0.86, 0.96, 1.0, 0.72];
            for tri in &tin.triangles {
                let (Some(a), Some(b), Some(c)) = (
                    tin.points.get(tri[0]),
                    tin.points.get(tri[1]),
                    tin.points.get(tri[2]),
                ) else {
                    continue;
                };
                let pts = [a, b, c];
                let below: Vec<&crate::core::gis::tin_surface::TinPoint> =
                    pts.iter().copied().filter(|p| p.z < wl).collect();
                let above: Vec<&crate::core::gis::tin_surface::TinPoint> =
                    pts.iter().copied().filter(|p| p.z >= wl).collect();
                if below.is_empty() || above.is_empty() {
                    continue;
                }
                let interp = |p0: &crate::core::gis::tin_surface::TinPoint,
                              p1: &crate::core::gis::tin_surface::TinPoint|
                 -> [f32; 3] {
                    let dz = (p1.z - p0.z).abs().max(1e-4);
                    let t = ((wl - p0.z).abs() / dz).clamp(0.0, 1.0);
                    [
                        p0.x + (p1.x - p0.x) * t,
                        p0.y + (p1.y - p0.y) * t,
                        wl + 0.08,
                    ]
                };
                let (e0, e1) = if below.len() == 1 && above.len() == 2 {
                    (interp(below[0], above[0]), interp(below[0], above[1]))
                } else {
                    (interp(above[0], below[0]), interp(above[0], below[1]))
                };
                let dx = e1[0] - e0[0];
                let dy = e1[1] - e0[1];
                let l = dx.hypot(dy).max(0.1);
                let nx = -dy / l * 0.45;
                let ny = dx / l * 0.45;
                push_quad_double(
                    &mut verts,
                    &mut idxs,
                    [e0[0] - nx, e0[1] - ny, e0[2]],
                    [e1[0] - nx, e1[1] - ny, e1[2]],
                    [e1[0] + nx, e1[1] + ny, e1[2]],
                    [e0[0] + nx, e0[1] + ny, e0[2]],
                    [0.0, 0.0, 1.0],
                    foam_col,
                    1.0,
                );
            }
        }
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// Builds the local Floating-Origin RTC GPU mesh for Pass 2 (all vertices in ENU meters relative to
/// `origin_lat_lon`):
/// - Stylized ground apron & city-builder buildable parcel pads (when zoomed to street/district scale)
/// - 3D triangulated `TinSurfaceMesh` (`IfcSite` / survey / procedural) with hypsometric colors & SNI 8460 steep slopes
/// - 3D authored IFCX City-Builder objects (`IfcRoad`, `IfcBridge`, `IfcSpatialZone`, stylized trees, graded pads)
/// - 3D extruded Vault OpenBIM IFCX buildings (`WorldSpatialIndex`, within 15 km of site origin)
/// - 3D extruded local Vault vector context buildings (`UrbanVectorLayerCache`)
#[allow(clippy::too_many_arguments)]
pub fn build_rtc_site_scene_mesh(
    theme: VectorEarthTheme,
    origin_lat_lon: (f64, f64),
    zoom_level: f32,
    show_terrain_contours: bool,
    active_site_tin: Option<&TinSurfaceMesh>,
    world_index: &WorldSpatialIndex,
    active_rel_path: Option<&str>,
    urban_vectors: &UrbanVectorLayerCache,
    city_scene: Option<&IfcxCityScene>,
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

    // 5. Phase 67: Authored IFCX City-Builder Infrastructure (`IfcRoad`, `IfcBridge`, `IfcSpatialZone`, Trees, Graded Pads)
    if let Some(city) = city_scene.filter(|c| !c.is_empty()) {
        push_city_builder_scene_enu(&mut vertices, &mut indices, theme, city, wall_base, roof_base);
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
    bim_vao: glow::VertexArray,
    bim_vbo: glow::Buffer,
    bim_ibo: glow::Buffer,
    bim_index_count: i32,
    bim_revision_uploaded: u64,
    hazard_vao: glow::VertexArray,
    hazard_vbo: glow::Buffer,
    hazard_ibo: glow::Buffer,
    hazard_index_count: i32,
    hazard_revision_uploaded: u64,
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
            let (bim_vao, bim_vbo, bim_ibo) = setup_vao_pair()?;
            let (hazard_vao, hazard_vbo, hazard_ibo) = setup_vao_pair()?;

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
                bim_vao,
                bim_vbo,
                bim_ibo,
                bim_index_count: 0,
                bim_revision_uploaded: 0,
                hazard_vao,
                hazard_vbo,
                hazard_ibo,
                hazard_index_count: 0,
                hazard_revision_uploaded: 0,
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

    fn upload_active_bim(&mut self, gl: &glow::Context, rev: u64, mesh: &GpuMeshPayload) {
        unsafe {
            gl.bind_vertex_array(Some(self.bim_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.bim_vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                vertices_as_u8_slice(&mesh.vertices),
                glow::DYNAMIC_DRAW,
            );
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.bim_ibo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                indices_as_u8_slice(&mesh.indices),
                glow::DYNAMIC_DRAW,
            );
            gl.bind_vertex_array(None);
        }
        self.bim_index_count = mesh.indices.len() as i32;
        self.bim_revision_uploaded = rev;
    }

    fn upload_hazard(&mut self, gl: &glow::Context, rev: u64, mesh: &GpuMeshPayload) {
        unsafe {
            gl.bind_vertex_array(Some(self.hazard_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.hazard_vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                vertices_as_u8_slice(&mesh.vertices),
                glow::DYNAMIC_DRAW,
            );
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.hazard_ibo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                indices_as_u8_slice(&mesh.indices),
                glow::DYNAMIC_DRAW,
            );
            gl.bind_vertex_array(None);
        }
        self.hazard_index_count = mesh.indices.len() as i32;
        self.hazard_revision_uploaded = rev;
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

            // --- Pass 2: Floating-Origin RTC Local Site Scene (3D IfcSite TIN + City-Builder + Extruded IFCX + Active BIM/FEM/Modal/N-1 + Multi-Hazard) ---
            if u.render_site_pass
                && (self.site_index_count > 0 || self.bim_index_count > 0 || self.hazard_index_count > 0)
            {
                gl.clear_depth_f32(1.0);
                gl.clear(glow::DEPTH_BUFFER_BIT);
                gl.uniform_matrix_4_f32_slice(loc_mvp.as_ref(), false, &u.site_mvp);
                gl.uniform_3_f32(loc_sun.as_ref(), u.sun_dir_site[0], u.sun_dir_site[1], u.sun_dir_site[2]);
                gl.uniform_3_f32(loc_eye.as_ref(), u.cam_eye_site[0], u.cam_eye_site[1], u.cam_eye_site[2]);
                gl.uniform_1_f32(loc_fog_d.as_ref(), u.fog_density);
                if self.site_index_count > 0 {
                    gl.bind_vertex_array(Some(self.site_vao));
                    gl.draw_elements(glow::TRIANGLES, self.site_index_count, glow::UNSIGNED_INT, 0);
                }
                if self.bim_index_count > 0 {
                    gl.bind_vertex_array(Some(self.bim_vao));
                    gl.draw_elements(glow::TRIANGLES, self.bim_index_count, glow::UNSIGNED_INT, 0);
                }
                if self.hazard_index_count > 0 {
                    gl.depth_mask(false);
                    gl.bind_vertex_array(Some(self.hazard_vao));
                    gl.draw_elements(glow::TRIANGLES, self.hazard_index_count, glow::UNSIGNED_INT, 0);
                    gl.depth_mask(true);
                }
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
            gl.delete_vertex_array(self.bim_vao);
            gl.delete_buffer(self.bim_vbo);
            gl.delete_buffer(self.bim_ibo);
            gl.delete_vertex_array(self.hazard_vao);
            gl.delete_buffer(self.hazard_vbo);
            gl.delete_buffer(self.hazard_ibo);
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
    cached_bim_hash: u64,
    cached_hazard_hash: u64,
    pub last_planet_tris: usize,
    pub last_site_tris: usize,
    pub last_bim_tris: usize,
    pub last_hazard_tris: usize,
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
    #[allow(clippy::too_many_arguments)]
    fn compute_site_hash(
        theme: VectorEarthTheme,
        origin: (f64, f64),
        zoom_bucket: i32,
        show_tin: bool,
        tin: Option<&TinSurfaceMesh>,
        world_index: &WorldSpatialIndex,
        active_rel: Option<&str>,
        urban_vectors: &UrbanVectorLayerCache,
        city_scene: Option<&IfcxCityScene>,
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
        if let Some(c) = city_scene {
            mix(&mut h, c.content_revision_hash());
        }
        h
    }

    fn compute_active_bim_hash(input: &ActiveBimGpuInput<'_>) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mix = |acc: &mut u64, val: u64| {
            *acc ^= val;
            *acc = acc.wrapping_mul(0x0000_0100_0000_01b3);
        };
        mix(&mut h, input.project.content_revision_hash());
        for c in &input.project.columns {
            mix(&mut h, u64::from(c.profile_name.contains("Jacket")));
        }
        for r in &input.project.runs {
            mix(&mut h, r.start_offset_mm.to_bits());
            mix(&mut h, r.end_offset_mm.to_bits());
            mix(&mut h, u64::from(r.profile_name.contains("Jacket")));
            mix(&mut h, r.category as u64);
        }
        for w in &input.project.walls {
            mix(&mut h, u64::from(w.name.contains("Retrofit")));
        }
        mix(&mut h, input.lens as u64 + 1);
        mix(&mut h, input.erection_day_limit as u64);
        mix(&mut h, (input.deform_scale * 100.0) as u64);
        mix(&mut h, input.active_modal_mode as u64);
        mix(&mut h, (input.heading_deg * 100.0) as i64 as u64);
        mix(&mut h, input.members.len() as u64);
        if let Some((k, id)) = input.n1_removed_bim {
            mix(&mut h, (k as u64 + 1) * 1_000_003 ^ id as u64);
        }
        mix(&mut h, input.cascade_failed_bim.len() as u64);
        for &(k, id) in input.cascade_failed_bim {
            mix(&mut h, (k as u64 + 7) * 2_000_003 ^ id as u64);
        }
        mix(
            &mut h,
            ((input.cascade_step_progress * 100.0).round() as u32) as u64,
        );
        if input.lens == EarthBimLens::ModalVibration {
            // Quantize animated sine phase at 1000 steps so GPU modal vibration updates smoothly at 60 FPS
            mix(&mut h, ((input.modal_phase_sin * 1000.0).round() as i32) as u64);
        }
        h
    }

    /// Schedules the hardware WebGL2 3D Vector Earth pass onto `painter`, uploading geometry ONLY
    /// when theme, site/city, or active BIM/FEM/Modal state changes. Returns `true` if GPU pass ran.
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
        city_scene: Option<&IfcxCityScene>,
        active_bim: Option<ActiveBimGpuInput<'_>>,
        hazard_input: Option<DistrictHazardGpuInput<'_>>,
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

        // Prepare RTC site + IFCX city-builder mesh upload only when zoomed in and content hash changes
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
                city_scene,
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
                    city_scene,
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

        // Prepare unified active 3D BIM + FEM/Modal/N-1 GPU mesh upload
        let pending_bim = if render_site_pass {
            if let Some(ref bim_in) = active_bim {
                let rev = Self::compute_active_bim_hash(bim_in);
                if rev != self.cached_bim_hash {
                    let mesh = build_rtc_active_bim_fem_mesh(bim_in);
                    self.last_bim_tris = mesh.indices.len() / 3;
                    self.cached_bim_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Prepare District Multi-Hazard (Seismic waves, Pounding corridors, Venturi wind, 3D Flood) GPU mesh
        let pending_hazard = if render_site_pass {
            if let Some(ref hz_in) = hazard_input {
                let rev = hz_in.config.content_revision_hash()
                    ^ ((hz_in.report.resilience_score * 100.0).round() as u64).wrapping_mul(2654435761)
                    ^ (hz_in.neighborhood.contacts.len() as u64).wrapping_mul(1_000_003);
                if rev != self.cached_hazard_hash {
                    let mesh = build_rtc_district_hazard_mesh(hz_in);
                    self.last_hazard_tris = mesh.indices.len() / 3;
                    self.cached_hazard_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_hazard_hash != 0 {
                self.cached_hazard_hash = 0;
                self.last_hazard_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let planet_payload = Arc::new(Mutex::new(pending_planet));
        let site_payload = Arc::new(Mutex::new(pending_site));
        let bim_payload = Arc::new(Mutex::new(pending_bim));
        let hazard_payload = Arc::new(Mutex::new(pending_hazard));

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
                if let Ok(mut slot) = bim_payload.lock() {
                    if let Some((rev, mesh)) = slot.take() {
                        if pipe.bim_revision_uploaded != rev {
                            pipe.upload_active_bim(gl, rev, &mesh);
                        }
                    }
                }
                if let Ok(mut slot) = hazard_payload.lock() {
                    if let Some((rev, mesh)) = slot.take() {
                        if pipe.hazard_revision_uploaded != rev {
                            pipe.upload_hazard(gl, rev, &mesh);
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
        // Smooth ocean shimmer, 60 FPS GPU modal oscillation, and animated seismic waves/wind
        let hazard_animated = hazard_input
            .as_ref()
            .is_some_and(|h| h.config.is_animated());
        let frame_ms = if hazard_animated
            || active_bim
                .as_ref()
                .is_some_and(|b| b.lens == EarthBimLens::ModalVibration)
        {
            16
        } else {
            33
        };
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(frame_ms));
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
                rel_path: "Projects/Bandung/Tower_B.ifcx".to_string(),
                name: "Tower B".to_string(),
                filename: "Tower_B.ifcx".to_string(),
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
            None,
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

    #[test]
    fn test_phase67_unified_gpu_city_builder_and_active_bim_modal_n1_mesh() {
        let mut city = IfcxCityScene::default();
        city.add_road([-40.0, 0.0], [40.0, 0.0], crate::core::gis::IfcxRoadKind::Arterial, false);
        city.add_road([0.0, -30.0], [0.0, 30.0], crate::core::gis::IfcxRoadKind::Collector, true);
        city.add_zone([24.0, 20.0], 26.0, 22.0, 0.0, IfcxZoneKind::Commercial);
        city.add_zone([-24.0, 20.0], 24.0, 24.0, 0.0, IfcxZoneKind::Park);

        let urban = UrbanVectorLayerCache::default();
        let world_idx = WorldSpatialIndex::default();
        let site_with_city = build_rtc_site_scene_mesh(
            VectorEarthTheme::CityDaylight,
            (-6.9175, 107.6191),
            17.5,
            false,
            None,
            &world_idx,
            None,
            &urban,
            Some(&city),
        );
        assert!(
            site_with_city.indices.len() / 3 > 500,
            "City-builder roads, bridge piers, context towers, and stylized trees must generate GPU triangles"
        );
        // Bridge deck should sit above 6.0m
        assert!(site_with_city.vertices.iter().any(|v| v.pos[2] >= 6.4));

        // Active 3D BIM + subsurface pile caps + GPU modal vibration + crimson N-1 ghost
        let (proj, _) = IfcxDocument::sample_villa_ifcx()
            .to_bim_project("Villa")
            .expect("villa project");
        let first_col_id = proj.columns[0].id;
        let clean_mesh = build_rtc_active_bim_fem_mesh(&ActiveBimGpuInput {
            project: &proj,
            members: &[],
            lens: EarthBimLens::Clean,
            erection_day_limit: 999,
            deform_scale: 40.0,
            active_modal_mode: 0,
            modal_phase_sin: 0.0,
            n1_removed_bim: None,
            cascade_failed_bim: &[],
            cascade_step_progress: 0.0,
            heading_deg: 0.0,
        });
        let modal_n1_mesh = build_rtc_active_bim_fem_mesh(&ActiveBimGpuInput {
            project: &proj,
            members: &[],
            lens: EarthBimLens::ModalVibration,
            erection_day_limit: 999,
            deform_scale: 60.0,
            active_modal_mode: 0,
            modal_phase_sin: 1.0,
            n1_removed_bim: Some((BimElementKind::Column, first_col_id)),
            cascade_failed_bim: &[],
            cascade_step_progress: 0.0,
            heading_deg: 0.0,
        });
        assert!(clean_mesh.indices.len() / 3 > 400);
        // Subsurface deep pile shafts extend below z = -3.0m
        assert!(clean_mesh.vertices.iter().any(|v| v.pos[2] < -3.0));
        // Modal vibration shifts elevated vertices along X
        let max_x_clean = clean_mesh.vertices.iter().map(|v| v.pos[0]).fold(f32::NEG_INFINITY, f32::max);
        let max_x_modal = modal_n1_mesh.vertices.iter().map(|v| v.pos[0]).fold(f32::NEG_INFINITY, f32::max);
        assert!(max_x_modal > max_x_clean + 0.25);
        // N-1 removed column renders with translucent crimson ghost alpha
        assert!(modal_n1_mesh.vertices.iter().any(|v| v.color[0] > 0.90 && v.color[3] < 0.55));
    }

    #[test]
    fn test_phase68_gpu_district_multi_hazard_mesh_generation() {
        use crate::core::gis::district_hazard::evaluate_district_multi_hazard;
        use crate::core::gis::neighborhood_physics::NeighborPoundingContact;

        let mut cfg = DistrictHazardConfig::default();
        let nb = NeighborhoodAuditReport {
            neighbors_evaluated: 1,
            pounding_risk_count: 1,
            venturi_corridor_count: 1,
            venturi_speedup_factor: 1.08,
            contacts: vec![NeighborPoundingContact {
                neighbor_rel_path: "nb.ifcx".to_string(),
                neighbor_name: "Tower NB".to_string(),
                neighbor_lat: -6.9176,
                neighbor_lon: 107.6193,
                neighbor_height_m: 24.0,
                neighbor_min_ari: 0.58,
                clear_gap_m: 0.22,
                required_gap_m: 0.52,
                severity: PoundingSeverity::PoundingRisk,
                closest_active_ll: (-6.9175, 107.6191),
                closest_neighbor_ll: (-6.9176, 107.6193),
                venturi_channel: true,
                inside_collapse_fall_zone: true,
            }],
            ..Default::default()
        };
        let tin = TinSurfaceMesh::generate_from_dem_preset(
            DemTerrainPreset::CoastalAlluvialBasin,
            240.0,
            240.0,
            12,
            12,
            1.0,
        );
        let proj = BimProject::sample_bimcraft_building();
        let city = IfcxCityScene::default();
        let rep = evaluate_district_multi_hazard(
            &cfg,
            &nb,
            Some(&tin),
            &WorldSpatialIndex::default(),
            (-6.9175, 107.6191),
            &proj,
            &city,
        );

        let empty_mesh = build_rtc_district_hazard_mesh(&DistrictHazardGpuInput {
            config: &cfg,
            report: &rep,
            neighborhood: &nb,
            origin_lat_lon: (-6.9175, 107.6191),
            active_building_height_m: 12.0,
            active_site_tin: Some(&tin),
        });
        assert!(empty_mesh.vertices.is_empty());

        cfg.seismic_wave_active = true;
        cfg.pounding_overlay_active = true;
        cfg.venturi_wind_active = true;
        cfg.flood_plane_active = true;
        cfg.flood_water_level_m = 4.0;
        cfg.sim_time_sec = 0.75;
        let rep_on = evaluate_district_multi_hazard(
            &cfg,
            &nb,
            Some(&tin),
            &WorldSpatialIndex::default(),
            (-6.9175, 107.6191),
            &proj,
            &city,
        );
        let full_mesh = build_rtc_district_hazard_mesh(&DistrictHazardGpuInput {
            config: &cfg,
            report: &rep_on,
            neighborhood: &nb,
            origin_lat_lon: (-6.9175, 107.6191),
            active_building_height_m: 12.0,
            active_site_tin: Some(&tin),
        });
        assert!(full_mesh.indices.len() / 3 > 250);
        assert!(full_mesh
            .vertices
            .iter()
            .all(|v| v.pos.iter().all(|f| f.is_finite()) && v.normal.iter().all(|f| f.is_finite())));
    }

    #[test]
    fn test_phase70_gpu_progressive_collapse_cascade_domino_mesh() {
        let proj = BimProject::sample_bimcraft_building();
        let failed = vec![
            (BimElementKind::Column, proj.columns[0].id),
            (BimElementKind::Run, proj.runs[0].id),
        ];
        let mesh = build_rtc_active_bim_fem_mesh(&ActiveBimGpuInput {
            project: &proj,
            members: &[],
            lens: EarthBimLens::Clean,
            erection_day_limit: 999,
            deform_scale: 35.0,
            active_modal_mode: 0,
            modal_phase_sin: 0.0,
            n1_removed_bim: None,
            cascade_failed_bim: &failed,
            cascade_step_progress: 0.8,
            heading_deg: 0.0,
        });
        assert!(mesh.indices.len() / 3 > 200);
        assert!(
            mesh.vertices
                .iter()
                .any(|v| v.color[0] > 0.94 && v.color[1] < 0.28 && v.color[3] < 0.42),
            "Cascading domino-failed members must render as translucent crimson-orange ghosts"
        );
    }
}
