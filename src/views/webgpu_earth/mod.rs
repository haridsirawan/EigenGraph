//! webgpu_earth/mod.rs — 100% Pure-Rust Native WebGPU (`egui_wgpu` + WGSL) 3D Vector Earth Renderer (Phase 75 / v0.7.5).
//!
//! Renders the curved 3D Vector Earth globe, extruded global continents & Indonesian archipelago,
//! mountain ridges, major rivers, animated procedural oceans, Rayleigh atmospheric limb glow,
//! 3D `IfcSite` TIN topographic terrain, IFCX City-Builder roads/bridges/parcels, unified active
//! BIM/FEM/Modal/N-1/Cascade/Replay/Diff geometry, subsurface geotechnical stratigraphy & piles,
//! district multi-hazard waves, district portfolio retrofit crowns, 3D plastic hinge beacons, and
//! FEMA P-58 per-story loss halos in hardware 24-bit Z-buffered WebGPU (`wgpu` + WGSL) — using
//! 0% WebGL, 0% GLSL, 0% `glow`, and 0% raster pixel textures.

use std::borrow::Cow;
use std::num::NonZeroU64;

use eframe::egui_wgpu;
use eframe::wgpu;
use egui::{Color32, Pos2, Rect};

use crate::core::gis::city_builder::IfcxCityScene;
use crate::core::gis::earth_globe::{
    lat_lon_to_unit_sphere, GlobeCamera, GlobeScreenTransform, PlanetaryWorldState,
};
use crate::core::gis::map_widget::{SlippyMapState, VectorEarthTheme, EARTH_RADIUS_METERS};
use crate::core::gis::tin_surface::TinSurfaceMesh;
use crate::core::gis::vector_layers::UrbanVectorLayerCache;
use crate::core::gis::world_index::WorldSpatialIndex;
use crate::views::universal_canvas::EarthBimLens;

mod compute_pass;
mod mesh_builders;
pub use compute_pass::*;
pub use mesh_builders::*;

/// Compact GPU vertex layout (`56 bytes`, `14 x f32`) shared by Planetary and RTC Site WebGPU passes.
///
/// Zero texture coordinates or raster samplers are required: all lighting, ocean wave normals,
/// ambient occlusion, procedural building facades, sedimentary bedding, and hinge pulses are
/// synthesized analytically in pure WGSL.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuEarthVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    pub meta: [f32; 4],
}

/// CPU-prepared pure-vector mesh payload ready for GPU vertex/index upload.
#[derive(Debug, Clone, Default)]
pub struct GpuMeshPayload {
    pub vertices: Vec<GpuEarthVertex>,
    pub indices: Vec<u32>,
}

/// Double-precision-derived (`f64 -> f32` RTC) per-frame WebGPU camera & atmospheric uniforms (`NDC z in [0, 1]`).
#[derive(Debug, Clone, Copy)]
pub struct RtcCameraUniforms {
    pub planet_mvp: [f32; 16],
    pub site_mvp: [f32; 16],
    pub sun_dir_world: [f32; 3],
    pub sun_dir_site: [f32; 3],
    pub cam_eye_world: [f32; 3],
    pub cam_eye_site: [f32; 3],
    pub space_bg: [f32; 3],
    pub fog_color: [f32; 3],
    pub atmo_color: [f32; 3],
    pub fog_density: f32,
    pub time_secs: f32,
    pub night_mix: f32,
    pub golden_mix: f32,
    pub enable_lighting: f32,
    pub render_site_pass: bool,
}

impl RtcCameraUniforms {
    /// Computes exact `f64 -> f32` Planetary and Floating-Origin RTC WebGPU (`z in [0, 1]`) matrices
    /// matching `GlobeScreenTransform::project_lat_lon_elev` to sub-pixel precision (`< 0.05 px`).
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

        let rot_to_tilted = |v: [f64; 3]| -> [f64; 3] {
            let ph = xform.world_to_tangent_h_pub(v);
            [ph[0], ph[1] * ct + ph[2] * st, -ph[1] * st + ph[2] * ct]
        };

        // --- Pass 1: Planetary Globe MVP (unit sphere R = 1.0, WebGPU depth z in [0, 1]) ---
        let near_p = (d_surf * 0.02).clamp(1.0e-6, 0.25);
        let far_p = (d_surf + 2.4).max(near_p + 0.05);
        let a_p = far_p / (far_p - near_p);
        let b_p = (-far_p * near_p) / (far_p - near_p);

        let c0 = rot_to_tilted([1.0, 0.0, 0.0]);
        let c1 = rot_to_tilted([0.0, 1.0, 0.0]);
        let c2 = rot_to_tilted([0.0, 0.0, 1.0]);

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

        // --- Pass 2: Floating-Origin RTC Local Site MVP (WebGPU depth z in [0, 1]) ---
        let (slat_deg, slon_deg) = site_origin_lat_lon;
        let o_site = lat_lon_to_unit_sphere(slat_deg, slon_deg);
        let o_cam_target = lat_lon_to_unit_sphere(xform.center_lat_deg, xform.center_lon_deg);

        let lat0 = slat_deg.clamp(-89.999, 89.999).to_radians();
        let lon0 = slon_deg.to_radians();
        let east_w = [lon0.cos(), 0.0, -lon0.sin()];
        let north_w = [-lat0.sin() * lon0.sin(), lat0.cos(), -lat0.sin() * lon0.cos()];
        let up_w = o_site;

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
        let a_s = far_m / (far_m - near_m);
        let b_s = (-far_m * near_m) / (far_m - near_m);

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
            fog_color[i] =
                fog_color[i] * (1.0 - night_base * 0.72) + night_fog[i] * (night_base * 0.72);
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

    /// Projects a world unit-sphere point through `planet_mvp` into screen pixels.
    pub fn project_planet_to_screen(&self, p: [f32; 3], rect: Rect) -> Option<Pos2> {
        project_mvp_to_screen(&self.planet_mvp, p, rect, 1e-6).map(|(pos, _)| pos)
    }

    /// Returns normalized WebGPU clip depth `z_ndc` (`0.0..=1.0`) for Planetary pass verification.
    pub fn project_planet_ndc_z(&self, p: [f32; 3], rect: Rect) -> Option<f32> {
        project_mvp_to_screen(&self.planet_mvp, p, rect, 1e-6).map(|(_, z)| z)
    }

    /// Projects a local ENU site point (meters) through `site_mvp` into screen pixels.
    pub fn project_site_to_screen(&self, enu_m: [f32; 3], rect: Rect) -> Option<Pos2> {
        project_mvp_to_screen(&self.site_mvp, enu_m, rect, 1e-4).map(|(pos, _)| pos)
    }

    /// Returns normalized WebGPU clip depth `z_ndc` (`0.0..=1.0`) for RTC Site pass verification.
    pub fn project_site_ndc_z(&self, enu_m: [f32; 3], rect: Rect) -> Option<f32> {
        project_mvp_to_screen(&self.site_mvp, enu_m, rect, 1e-4).map(|(_, z)| z)
    }

    fn to_planet_pass_uniforms(&self) -> EarthPassUniforms {
        EarthPassUniforms {
            mvp: self.planet_mvp,
            sun_and_lit: [
                self.sun_dir_world[0],
                self.sun_dir_world[1],
                self.sun_dir_world[2],
                self.enable_lighting,
            ],
            eye_and_fog_d: [
                self.cam_eye_world[0],
                self.cam_eye_world[1],
                self.cam_eye_world[2],
                0.0,
            ],
            fog_and_time: [
                self.fog_color[0],
                self.fog_color[1],
                self.fog_color[2],
                self.time_secs,
            ],
            atmo_and_night: [
                self.atmo_color[0],
                self.atmo_color[1],
                self.atmo_color[2],
                self.night_mix,
            ],
            golden_pad: [self.golden_mix, 0.0, 0.0, 0.0],
        }
    }

    fn to_site_pass_uniforms(&self) -> EarthPassUniforms {
        EarthPassUniforms {
            mvp: self.site_mvp,
            sun_and_lit: [
                self.sun_dir_site[0],
                self.sun_dir_site[1],
                self.sun_dir_site[2],
                self.enable_lighting,
            ],
            eye_and_fog_d: [
                self.cam_eye_site[0],
                self.cam_eye_site[1],
                self.cam_eye_site[2],
                self.fog_density,
            ],
            fog_and_time: [
                self.fog_color[0],
                self.fog_color[1],
                self.fog_color[2],
                self.time_secs,
            ],
            atmo_and_night: [
                self.atmo_color[0],
                self.atmo_color[1],
                self.atmo_color[2],
                self.night_mix,
            ],
            golden_pad: [self.golden_mix, 0.0, 0.0, 0.0],
        }
    }
}

fn project_mvp_to_screen(m: &[f32; 16], p: [f32; 3], rect: Rect, min_w: f32) -> Option<(Pos2, f32)> {
    let cx = m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12];
    let cy = m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13];
    let cz = m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14];
    let cw = m[3] * p[0] + m[7] * p[1] + m[11] * p[2] + m[15];
    if cw <= min_w {
        return None;
    }
    let ndc_x = cx / cw;
    let ndc_y = cy / cw;
    let ndc_z = cz / cw;
    Some((
        Pos2::new(
            rect.left() + (ndc_x * 0.5 + 0.5) * rect.width(),
            rect.top() + (0.5 - ndc_y * 0.5) * rect.height(),
        ),
        ndc_z,
    ))
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct EarthPassUniforms {
    mvp: [f32; 16],
    sun_and_lit: [f32; 4],
    eye_and_fog_d: [f32; 4],
    fog_and_time: [f32; 4],
    atmo_and_night: [f32; 4],
    golden_pad: [f32; 4],
}

fn struct_as_u8_slice<T: Copy>(val: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts((val as *const T) as *const u8, std::mem::size_of::<T>()) }
}

fn vertices_as_u8_slice(verts: &[GpuEarthVertex]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            verts.as_ptr() as *const u8,
            std::mem::size_of_val(verts),
        )
    }
}

fn indices_as_u8_slice(indices: &[u32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            indices.as_ptr() as *const u8,
            std::mem::size_of_val(indices),
        )
    }
}

const EARTH_WGSL: &str = r#"
struct EarthPassUniforms {
    mvp: mat4x4<f32>,
    sun_and_lit: vec4<f32>,
    eye_and_fog_d: vec4<f32>,
    fog_and_time: vec4<f32>,
    atmo_and_night: vec4<f32>,
    golden_pad: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: EarthPassUniforms;

struct VertexIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) v_meta: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) v_meta: vec4<f32>,
    @location(4) cam_dist: f32,
};

@vertex
fn vs_main(v_in: VertexIn) -> VertexOut {
    var v_out: VertexOut;
    let clip = u.mvp * vec4<f32>(v_in.pos, 1.0);
    v_out.clip_pos = clip;
    v_out.world_pos = v_in.pos;
    v_out.normal = v_in.normal;
    v_out.color = v_in.color;
    v_out.v_meta = v_in.v_meta;
    v_out.cam_dist = max(clip.w, 0.0);
    return v_out;
}

@fragment
fn fs_main(f_in: VertexOut) -> @location(0) vec4<f32> {
    var n = select(vec3<f32>(0.0, 0.0, 1.0), normalize(f_in.normal), dot(f_in.normal, f_in.normal) > 1e-8);
    let sun_raw = u.sun_and_lit.xyz;
    let l = select(vec3<f32>(0.0, 0.0, 1.0), normalize(sun_raw), dot(sun_raw, sun_raw) > 1e-8);
    let view_raw = u.eye_and_fog_d.xyz - f_in.world_pos;
    let v = select(vec3<f32>(0.0, 0.0, 1.0), normalize(view_raw), dot(view_raw, view_raw) > 1e-8);

    let kind = f_in.v_meta.x;
    let height_m = f_in.v_meta.y;
    let ao = f_in.v_meta.z;
    let phase = f_in.v_meta.w;
    let u_time = u.fog_and_time.w;
    let u_night = u.atmo_and_night.w;
    let u_gold = u.golden_pad.x;
    let u_lit = u.sun_and_lit.w;
    let u_fog_d = u.eye_and_fog_d.w;
    let u_fog_c = u.fog_and_time.xyz;
    let u_atmo = u.atmo_and_night.xyz;

    if (kind < 0.5) {
        let w1 = sin(f_in.world_pos.x * 58.0 + u_time * 1.15) * cos(f_in.world_pos.z * 54.0 - u_time * 0.85);
        let w2 = sin((f_in.world_pos.y * 74.0 + f_in.world_pos.x * 42.0) - u_time * 1.45);
        let ripple = n + vec3<f32>(w1 * 0.028, w2 * 0.022, (w1 - w2) * 0.022);
        n = normalize(ripple);
    }

    let ndotl = dot(n, l);
    let day_diffuse = smoothstep(-0.22, 0.85, ndotl);
    let lit = mix(1.0, mix(0.24, 1.0, day_diffuse), u_lit);
    let hemi = 0.84 + 0.16 * clamp(n.z * 0.5 + 0.5, 0.0, 1.0);
    let sun_tint = mix(vec3<f32>(1.0, 0.99, 0.96), vec3<f32>(1.0, 0.72, 0.44), u_gold * 0.75);
    var col = f_in.color.rgb * sun_tint * lit * hemi * ao;

    if (kind < 0.5) {
        let hv = l + v;
        let h = select(n, normalize(hv), dot(hv, hv) > 1e-8);
        let spec = pow(max(dot(n, h), 0.0), 42.0) * max(ndotl, 0.0) * 0.42;
        let fresnel = pow(max(1.0 - max(dot(n, v), 0.0), 0.0), 2.8);
        col += sun_tint * spec + u_atmo * (fresnel * 0.58);
    } else if (kind > 1.5 && kind < 2.5) {
        let is_wall = 1.0 - step(0.85, abs(n.z));
        let story_v = fract(height_m / 3.4);
        let mullion_u = fract(phase / 2.6);
        let win_band = step(0.24, story_v) * step(story_v, 0.76) * step(0.18, mullion_u) * step(mullion_u, 0.82);
        let glass_mask = is_wall * win_band * step(1.5, height_m);
        let day_glass = mix(col * 0.72, u_atmo * 0.95, 0.38);
        col = mix(col, day_glass, glass_mask * (1.0 - u_night) * 0.55);
        let lit_hash = fract(sin(floor(height_m / 3.4) * 12.9898 + floor(phase / 2.6) * 78.233) * 43758.5453);
        let lit_on = step(0.38, lit_hash);
        let win_glow = mix(vec3<f32>(1.0, 0.84, 0.52), vec3<f32>(0.52, 0.88, 1.0), step(0.78, lit_hash));
        let night_strength = max(u_night, u_gold * 0.45);
        col += win_glow * (glass_mask * lit_on * night_strength * 0.82);
    } else if (kind >= 9.5) {
        // Phase 80: Pure Analytical WGSL District Risk Field Iso-Contour Heatmap (0% raster)
        let r_val = clamp(height_m, 0.0, 1.0);
        let contour_phase = fract(r_val * 8.0);
        let iso_line = smoothstep(0.0, 0.06, contour_phase) * smoothstep(0.12, 0.06, contour_phase);
        let pulse = 0.88 + 0.12 * sin(u_time * 2.2 - r_val * 5.0);
        let boost = mix(f_in.color.rgb * pulse, vec3<f32>(0.98, 0.99, 1.0), iso_line * 0.55);
        col = boost;
    } else if (kind >= 8.5) {
        // Phase 77: Autonomous Evacuation Corridors, Debris Cordons & Assembly Beacons
        let flow = 0.72 + 0.34 * smoothstep(0.35, 0.95, sin(phase * 0.65 - u_time * 4.5));
        let ndotv = max(abs(dot(n, v)), 0.12);
        let rim = pow(1.0 - ndotv, 1.8) * 0.38;
        col = f_in.color.rgb * (flow + rim);
    } else if (kind >= 7.5) {
        // Phase 76: Live SHM Sensor Beacons & Post-Earthquake ATC-20 Placard Shields
        let alarm_rate = 2.6 + f_in.v_meta.w * 5.2;
        let pulse = 0.72 + 0.28 * sin(u_time * alarm_rate + f_in.world_pos.z * 0.8);
        let ndotv = max(abs(dot(n, v)), 0.08);
        let rim = pow(1.0 - ndotv, 1.8) * 0.55;
        col = f_in.color.rgb * (0.95 + 0.45 * pulse + rim);
    } else if (kind >= 6.5) {
        // FEMA P-58 Per-Story Loss Halo Bands
        let band_pulse = 0.78 + 0.22 * sin(u_time * 2.4 + f_in.world_pos.z * 0.6);
        col = mix(col, f_in.color.rgb * 1.15, 0.45) * band_pulse;
    } else if (kind >= 5.5) {
        // Pulsing 3D Plastic Hinge Beacons & Ground-Motion Pulse Ring
        let pulse = 0.68 + 0.32 * sin(u_time * (3.2 + phase * 6.0));
        col = mix(col, f_in.color.rgb * 1.18, 0.55) * pulse;
    } else if (kind >= 4.5) {
        let wave = 0.5 + 0.5 * sin(f_in.world_pos.x * 1.8 + u_time * 1.6) * cos(f_in.world_pos.y * 1.6 - u_time * 1.2);
        col += vec3<f32>(0.08, 0.24, 0.32) * wave;
    } else if (kind >= 3.5 && kind < 4.5) {
        let bed = 0.88 + 0.12 * sin(f_in.world_pos.z * 7.5 + sin(f_in.world_pos.x * 0.45 + f_in.world_pos.y * 0.35) * 1.4);
        col *= bed;
    } else if (kind < 1.5) {
        let fresnel = pow(max(1.0 - max(dot(n, v), 0.0), 0.0), 3.2);
        col += u_atmo * (fresnel * 0.16);
    }

    if (u_fog_d > 0.0) {
        let fog_f = 1.0 - exp(-f_in.cam_dist * u_fog_d);
        col = mix(col, u_fog_c, clamp(fog_f, 0.0, 0.82));
    }

    return vec4<f32>(clamp(col, vec3<f32>(0.0), vec3<f32>(1.0)), f_in.color.a);
}
"#;

const COMPOSITE_WGSL: &str = r#"
@group(0) @binding(0) var earth_color_tex: texture_2d<f32>;
@group(0) @binding(1) var earth_sampler: sampler;

struct CompositeOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_composite(@builtin(vertex_index) vid: u32) -> CompositeOut {
    let x = f32((vid & 1u) << 2u) - 1.0;
    let y = f32((vid & 2u) << 1u) - 1.0;
    var c_out: CompositeOut;
    c_out.pos = vec4<f32>(x, y, 0.0, 1.0);
    c_out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return c_out;
}

@fragment
fn fs_composite(c_in: CompositeOut) -> @location(0) vec4<f32> {
    return textureSample(earth_color_tex, earth_sampler, c_in.uv);
}
"#;

struct GpuMeshSlot {
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    vertex_capacity: u64,
    index_capacity: u64,
    index_count: u32,
    revision_uploaded: u64,
}

impl Default for GpuMeshSlot {
    fn default() -> Self {
        Self {
            vertex_buffer: None,
            index_buffer: None,
            vertex_capacity: 0,
            index_capacity: 0,
            index_count: 0,
            revision_uploaded: 0,
        }
    }
}

impl GpuMeshSlot {
    fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        rev: u64,
        mesh: &GpuMeshPayload,
    ) {
        self.revision_uploaded = rev;
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            self.index_count = 0;
            return;
        }
        let v_bytes = vertices_as_u8_slice(&mesh.vertices);
        let i_bytes = indices_as_u8_slice(&mesh.indices);
        let v_size = (v_bytes.len() as u64).max(64);
        let i_size = (i_bytes.len() as u64).max(64);

        if self.vertex_buffer.is_none() || v_size > self.vertex_capacity {
            let cap = v_size.next_power_of_two().max(256);
            self.vertex_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: cap,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.vertex_capacity = cap;
        }
        if self.index_buffer.is_none() || i_size > self.index_capacity {
            let cap = i_size.next_power_of_two().max(256);
            self.index_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: cap,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.index_capacity = cap;
        }
        if let Some(ref vb) = self.vertex_buffer {
            queue.write_buffer(vb, 0, v_bytes);
        }
        if let Some(ref ib) = self.index_buffer {
            queue.write_buffer(ib, 0, i_bytes);
        }
        self.index_count = mesh.indices.len() as u32;
    }

    fn draw(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        if self.index_count == 0 {
            return;
        }
        if let (Some(vb), Some(ib)) = (&self.vertex_buffer, &self.index_buffer) {
            render_pass.set_vertex_buffer(0, vb.slice(..));
            render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.index_count, 0, 0..1);
        }
    }
}

struct OffscreenEarthTargets {
    size: [u32; 2],
    _color_tex: wgpu::Texture,
    color_view: wgpu::TextureView,
    _depth_tex: wgpu::Texture,
    depth_view: wgpu::TextureView,
    composite_bg: wgpu::BindGroup,
}

/// GPU pipelines, uniform bind groups, offscreen depth/color targets, and reusable vertex/index buffers
/// stored inside `egui_wgpu::CallbackResources`.
pub struct WebGpuEarthResources {
    target_format: wgpu::TextureFormat,
    opaque_pipeline: wgpu::RenderPipeline,
    translucent_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    composite_bgl: wgpu::BindGroupLayout,
    composite_sampler: wgpu::Sampler,
    targets: Option<OffscreenEarthTargets>,
    rendered_this_frame: bool,
    planet_uniform_buf: wgpu::Buffer,
    site_uniform_buf: wgpu::Buffer,
    planet_bind_group: wgpu::BindGroup,
    site_bind_group: wgpu::BindGroup,
    planet_theme_uploaded: Option<VectorEarthTheme>,
    planet: GpuMeshSlot,
    site: GpuMeshSlot,
    bim: GpuMeshSlot,
    hinges: GpuMeshSlot,
    subsurface: GpuMeshSlot,
    portfolio: GpuMeshSlot,
    hazard: GpuMeshSlot,
    fragility_loss: GpuMeshSlot,
    shm_triage: GpuMeshSlot,
    evacuation: GpuMeshSlot,
    risk_heatmap: GpuMeshSlot,
    hazard_compute: GpuHazardParticlePipeline,
}

impl WebGpuEarthResources {
    pub(crate) fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("eigengraph_webgpu_earth_wgsl"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(EARTH_WGSL)),
        });

        let uniform_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("eigengraph_earth_uniform_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(std::mem::size_of::<EarthPassUniforms>() as u64),
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("eigengraph_earth_pipeline_layout"),
            bind_group_layouts: &[Some(&uniform_bgl)],
            immediate_size: 0,
        });

        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuEarthVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x3,
                1 => Float32x3,
                2 => Float32x4,
                3 => Float32x4
            ],
        };

        // Always use Depth24PlusStencil8 with explicit depth_ops & stencil_ops on our offscreen pass
        // so wgpu 30's WebGPU backend emits `stencilReadOnly: false` (valid under W3C WebGPU spec).
        let depth_format = wgpu::TextureFormat::Depth24PlusStencil8;
        let build_pipeline = |label: &str, depth_write: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(vertex_layout.clone())],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: depth_format,
                    depth_write_enabled: Some(depth_write),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview_mask: None,
                cache: None,
            })
        };

        let opaque_pipeline = build_pipeline("eigengraph_earth_opaque", true);
        let translucent_pipeline = build_pipeline("eigengraph_earth_translucent", false);

        let composite_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("eigengraph_earth_composite_wgsl"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(COMPOSITE_WGSL)),
        });
        let composite_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("eigengraph_earth_composite_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let composite_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("eigengraph_earth_composite_layout"),
            bind_group_layouts: &[Some(&composite_bgl)],
            immediate_size: 0,
        });
        let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("eigengraph_earth_composite_pipeline"),
            layout: Some(&composite_layout),
            vertex: wgpu::VertexState {
                module: &composite_module,
                entry_point: Some("vs_composite"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &composite_module,
                entry_point: Some("fs_composite"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });
        let composite_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("eigengraph_earth_composite_sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let u_size = std::mem::size_of::<EarthPassUniforms>() as wgpu::BufferAddress;
        let planet_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("eigengraph_planet_uniforms"),
            size: u_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let site_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("eigengraph_site_uniforms"),
            size: u_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let make_bg = |label: &str, buf: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &uniform_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf.as_entire_binding(),
                }],
            })
        };
        let planet_bind_group = make_bg("eigengraph_planet_bg", &planet_uniform_buf);
        let site_bind_group = make_bg("eigengraph_site_bg", &site_uniform_buf);

        Self {
            target_format,
            opaque_pipeline,
            translucent_pipeline,
            composite_pipeline,
            composite_bgl,
            composite_sampler,
            targets: None,
            rendered_this_frame: false,
            planet_uniform_buf,
            site_uniform_buf,
            planet_bind_group,
            site_bind_group,
            planet_theme_uploaded: None,
            planet: GpuMeshSlot::default(),
            site: GpuMeshSlot::default(),
            bim: GpuMeshSlot::default(),
            hinges: GpuMeshSlot::default(),
            subsurface: GpuMeshSlot::default(),
            portfolio: GpuMeshSlot::default(),
            hazard: GpuMeshSlot::default(),
            fragility_loss: GpuMeshSlot::default(),
            shm_triage: GpuMeshSlot::default(),
            evacuation: GpuMeshSlot::default(),
            risk_heatmap: GpuMeshSlot::default(),
            hazard_compute: GpuHazardParticlePipeline::new(device),
        }
    }

    pub(crate) fn ensure_targets(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let max_dim = device.limits().max_texture_dimension_2d.max(64);
        let w = width.clamp(1, max_dim);
        let h = height.clamp(1, max_dim);
        if self.targets.as_ref().is_some_and(|t| t.size == [w, h]) {
            return;
        }
        let color_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("eigengraph_earth_color_target"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.target_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let color_view = color_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("eigengraph_earth_depth_stencil_target"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let composite_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("eigengraph_earth_composite_bg"),
            layout: &self.composite_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.composite_sampler),
                },
            ],
        });
        self.targets = Some(OffscreenEarthTargets {
            size: [w, h],
            _color_tex: color_tex,
            color_view,
            _depth_tex: depth_tex,
            depth_view,
            composite_bg,
        });
    }

    pub(crate) fn render_offscreen_passes(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        render_site_pass: bool,
    ) {
        let Some(ref target) = self.targets else {
            return;
        };

        // Pass 1: Planetary Vector Globe in dedicated full [0.0, 1.0] Depth24PlusStencil8 pass
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("eigengraph_earth_planet_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.color_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Discard,
                    }),
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            if self.planet.index_count > 0 {
                pass.set_pipeline(&self.opaque_pipeline);
                pass.set_bind_group(0, &self.planet_bind_group, &[]);
                self.planet.draw(&mut pass);
            }
        }

        // Pass 2: Floating-Origin RTC Local Site Scene with fresh [0.0, 1.0] 24-bit depth
        let has_site = self.site.index_count > 0
            || self.bim.index_count > 0
            || self.hinges.index_count > 0
            || self.subsurface.index_count > 0
            || self.portfolio.index_count > 0
            || self.hazard.index_count > 0
            || self.fragility_loss.index_count > 0
            || self.shm_triage.index_count > 0
            || self.evacuation.index_count > 0
            || self.risk_heatmap.index_count > 0
            || self.hazard_compute.active_vertex_count > 0;
        if render_site_pass && has_site {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("eigengraph_earth_site_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.color_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Discard,
                    }),
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.opaque_pipeline);
            pass.set_bind_group(0, &self.site_bind_group, &[]);
            self.site.draw(&mut pass);
            self.bim.draw(&mut pass);

            pass.set_pipeline(&self.translucent_pipeline);
            pass.set_bind_group(0, &self.site_bind_group, &[]);
            self.hinges.draw(&mut pass);
            self.subsurface.draw(&mut pass);
            self.risk_heatmap.draw(&mut pass);
            self.portfolio.draw(&mut pass);
            self.hazard.draw(&mut pass);
            self.hazard_compute.draw(&mut pass);
            self.fragility_loss.draw(&mut pass);
            self.shm_triage.draw(&mut pass);
            self.evacuation.draw(&mut pass);
        }
    }
}

struct WebGpuEarthPaintCallback {
    viewport_rect: Rect,
    uniforms: RtcCameraUniforms,
    pending_planet: Option<(VectorEarthTheme, GpuMeshPayload)>,
    pending_site: Option<(u64, GpuMeshPayload)>,
    pending_bim: Option<(u64, GpuMeshPayload)>,
    pending_hazard: Option<(u64, GpuMeshPayload)>,
    pending_subsurface: Option<(u64, GpuMeshPayload)>,
    pending_portfolio: Option<(u64, GpuMeshPayload)>,
    pending_hinges: Option<(u64, GpuMeshPayload)>,
    pending_fragility: Option<(u64, GpuMeshPayload)>,
    pending_shm: Option<(u64, GpuMeshPayload)>,
    pending_evacuation: Option<(u64, GpuMeshPayload)>,
    pending_risk_heatmap: Option<(u64, GpuMeshPayload)>,
    pending_hazard_compute: Option<HazardComputeInput>,
}

impl egui_wgpu::CallbackTrait for WebGpuEarthPaintCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
        egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let Some(res) = callback_resources.get_mut::<WebGpuEarthResources>() else {
            return Vec::new();
        };
        if let Some((th, ref mesh)) = self.pending_planet {
            if res.planet_theme_uploaded != Some(th) {
                res.planet.upload(device, queue, "planet_mesh", 1, mesh);
                res.planet_theme_uploaded = Some(th);
            }
        }
        if let Some((rev, ref mesh)) = self.pending_site {
            if res.site.revision_uploaded != rev {
                res.site.upload(device, queue, "site_mesh", rev, mesh);
            }
        }
        if let Some((rev, ref mesh)) = self.pending_bim {
            if res.bim.revision_uploaded != rev {
                res.bim.upload(device, queue, "bim_mesh", rev, mesh);
            }
        }
        if let Some((rev, ref mesh)) = self.pending_hazard {
            if res.hazard.revision_uploaded != rev {
                res.hazard.upload(device, queue, "hazard_mesh", rev, mesh);
            }
        }
        if let Some((rev, ref mesh)) = self.pending_subsurface {
            if res.subsurface.revision_uploaded != rev {
                res.subsurface.upload(device, queue, "subsurface_mesh", rev, mesh);
            }
        }
        if let Some((rev, ref mesh)) = self.pending_portfolio {
            if res.portfolio.revision_uploaded != rev {
                res.portfolio.upload(device, queue, "portfolio_mesh", rev, mesh);
            }
        }
        if let Some((rev, ref mesh)) = self.pending_hinges {
            if res.hinges.revision_uploaded != rev {
                res.hinges.upload(device, queue, "hinge_mesh", rev, mesh);
            }
        }
        if let Some((rev, ref mesh)) = self.pending_fragility {
            if res.fragility_loss.revision_uploaded != rev {
                res.fragility_loss
                    .upload(device, queue, "fragility_loss_mesh", rev, mesh);
            }
        }
        if let Some((rev, ref mesh)) = self.pending_shm {
            if res.shm_triage.revision_uploaded != rev {
                res.shm_triage
                    .upload(device, queue, "shm_triage_mesh", rev, mesh);
            }
        }
        if let Some(ref ev_in) = self.pending_evacuation {
            if res.evacuation.revision_uploaded != ev_in.0 {
                res.evacuation
                    .upload(device, queue, "evacuation_mesh", ev_in.0, &ev_in.1);
            }
        }
        if let Some(ref rh_in) = self.pending_risk_heatmap {
            if res.risk_heatmap.revision_uploaded != rh_in.0 {
                res.risk_heatmap
                    .upload(device, queue, "risk_heatmap_mesh", rh_in.0, &rh_in.1);
            }
        }
        if let Some(ref h_cmp) = self.pending_hazard_compute {
            res.hazard_compute.dispatch(queue, egui_encoder, h_cmp);
        } else {
            res.hazard_compute.active_vertex_count = 0;
        }

        let pu = self.uniforms.to_planet_pass_uniforms();
        let su = self.uniforms.to_site_pass_uniforms();
        queue.write_buffer(&res.planet_uniform_buf, 0, struct_as_u8_slice(&pu));
        queue.write_buffer(&res.site_uniform_buf, 0, struct_as_u8_slice(&su));

        let vp = egui::epaint::ViewportInPixels::from_points(
            &self.viewport_rect,
            screen_descriptor.pixels_per_point,
            screen_descriptor.size_in_pixels,
        );
        if vp.width_px <= 0 || vp.height_px <= 0 {
            res.rendered_this_frame = false;
            return Vec::new();
        }
        res.ensure_targets(device, vp.width_px as u32, vp.height_px as u32);
        res.render_offscreen_passes(egui_encoder, self.uniforms.render_site_pass);
        res.rendered_this_frame = true;
        Vec::new()
    }

    fn paint(
        &self,
        info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        let Some(res) = callback_resources.get::<WebGpuEarthResources>() else {
            return;
        };
        if !res.rendered_this_frame {
            return;
        }
        let Some(ref target) = res.targets else {
            return;
        };
        let vp = info.viewport_in_pixels();
        let cp = info.clip_rect_in_pixels();
        let max_w = info.screen_size_px[0];
        let max_h = info.screen_size_px[1];
        if max_w == 0 || max_h == 0 || vp.width_px <= 0 || vp.height_px <= 0 || cp.width_px <= 0 || cp.height_px <= 0 {
            return;
        }
        let sx = (vp.left_px.max(0) as u32).min(max_w.saturating_sub(1));
        let sy = (vp.top_px.max(0) as u32).min(max_h.saturating_sub(1));
        let sw = (vp.width_px as u32).min(max_w.saturating_sub(sx));
        let sh = (vp.height_px as u32).min(max_h.saturating_sub(sy));

        let cx = (cp.left_px.max(0) as u32).min(max_w.saturating_sub(1));
        let cy = (cp.top_px.max(0) as u32).min(max_h.saturating_sub(1));
        let cw = (cp.width_px as u32).min(max_w.saturating_sub(cx));
        let ch = (cp.height_px as u32).min(max_h.saturating_sub(cy));
        if sw == 0 || sh == 0 || cw == 0 || ch == 0 {
            return;
        }
        render_pass.set_viewport(sx as f32, sy as f32, sw as f32, sh as f32, 0.0, 1.0);
        render_pass.set_scissor_rect(cx, cy, cw, ch);
        render_pass.set_pipeline(&res.composite_pipeline);
        render_pass.set_bind_group(0, &target.composite_bg, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

/// Manages 100% Pure WebGPU (`egui_wgpu`) Vector Earth state with CPU vector fallback in headless tests.
#[derive(Clone, Default)]
pub struct WebGpuEarthState {
    ready: bool,
    pub enabled: bool,
    pub adapter_label: String,
    pub last_init_error: Option<String>,
    cached_planet_theme: Option<VectorEarthTheme>,
    cached_site_hash: u64,
    cached_bim_hash: u64,
    cached_hazard_hash: u64,
    cached_subsurface_hash: u64,
    cached_portfolio_hash: u64,
    cached_hinge_hash: u64,
    cached_fragility_hash: u64,
    cached_shm_hash: u64,
    cached_evacuation_hash: u64,
    cached_risk_heatmap_hash: u64,
    pub last_planet_tris: usize,
    pub last_site_tris: usize,
    pub last_bim_tris: usize,
    pub last_hazard_tris: usize,
    pub last_subsurface_tris: usize,
    pub last_portfolio_tris: usize,
    pub last_hinge_tris: usize,
    pub last_fragility_tris: usize,
    pub last_shm_tris: usize,
    pub last_evacuation_tris: usize,
    pub last_risk_heatmap_tris: usize,
    pub last_compute_particles: u32,
    #[cfg(not(target_arch = "wasm32"))]
    wgpu_device: Option<wgpu::Device>,
    #[cfg(not(target_arch = "wasm32"))]
    wgpu_queue: Option<wgpu::Queue>,
}

impl std::fmt::Debug for WebGpuEarthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebGpuEarthState")
            .field("ready", &self.ready)
            .field("enabled", &self.enabled)
            .field("adapter", &self.adapter_label)
            .field("last_planet_tris", &self.last_planet_tris)
            .field("last_site_tris", &self.last_site_tris)
            .finish()
    }
}

impl WebGpuEarthState {
    /// Initializes pure WebGPU (`wgpu` + WGSL) render pipelines and uniform buffers inside `RenderState`.
    pub fn init_wgpu(&mut self, render_state: &egui_wgpu::RenderState) {
        let info = render_state.adapter.get_info();
        let backend_lbl = match info.backend {
            wgpu::Backend::BrowserWebGpu => "WebGPU",
            wgpu::Backend::Metal => "WebGPU (Metal)",
            wgpu::Backend::Vulkan => "WebGPU (Vulkan)",
            wgpu::Backend::Dx12 => "WebGPU (DX12)",
            _ => "WebGPU",
        };
        render_state
            .device
            .on_uncaptured_error(std::sync::Arc::new(|err| {
                #[cfg(target_arch = "wasm32")]
                log::error!("WebGPU uncaptured device error: {err}");
                #[cfg(not(target_arch = "wasm32"))]
                eprintln!("WebGPU uncaptured device error: {err}");
            }));
        let resources = WebGpuEarthResources::new(&render_state.device, render_state.target_format);
        render_state
            .renderer
            .write()
            .callback_resources
            .insert(resources);
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.wgpu_device = Some(render_state.device.clone());
            self.wgpu_queue = Some(render_state.queue.clone());
        }
        self.ready = true;
        self.enabled = true;
        self.adapter_label = backend_lbl.to_string();
        self.last_init_error = None;
    }

    /// Attempts hardware `@compute` WGSL FEMA P-58 Monte Carlo execution when native `wgpu` Device & Queue are available.
    pub fn try_run_mc_on_gpu(
        &self,
        project: &crate::core::BimProject,
        replay_opt: Option<&crate::core::ground_motion_replay::GroundMotionReplayReport>,
        hazard_opt: Option<&crate::core::gis::DistrictMultiHazardReport>,
        ssi_opt: Option<&crate::core::geotech::SubsurfaceSsiAnalysisReport>,
        config: &crate::core::gpu_fragility_mc::MonteCarloFragilityConfig,
    ) -> Option<crate::core::gpu_fragility_mc::MonteCarloFragilityReport> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let (Some(ref dev), Some(ref q)) = (&self.wgpu_device, &self.wgpu_queue) {
                return crate::core::gpu_fragility_mc::run_fema_p58_monte_carlo_wgpu(
                    dev, q, project, replay_opt, hazard_opt, ssi_opt, config,
                )
                .ok();
            }
        }
        #[cfg(target_arch = "wasm32")]
        let _ = (project, replay_opt, hazard_opt, ssi_opt, config);
        None
    }

    /// True when the native WebGPU pipeline is initialized and enabled by the user.
    pub fn is_active(&self) -> bool {
        self.enabled && self.ready
    }

    /// True when a WebGPU adapter & pipeline were successfully initialized.
    pub fn is_available(&self) -> bool {
        self.ready
    }

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
        mix(
            &mut h,
            ((input.subsurface_pile_depth_m * 10.0).round() as u32) as u64,
        );
        mix(&mut h, u64::from(input.diff_overlay_active));
        mix(&mut h, input.diff_added_bim.len() as u64);
        mix(&mut h, input.diff_modified_bim.len() as u64);
        mix(&mut h, input.diff_deleted_ghosts.len() as u64);
        for &(k, id) in input.diff_added_bim {
            mix(&mut h, (k as u64 + 13) * 1_000_033 ^ id as u64);
        }
        for &(k, id) in input.diff_modified_bim {
            mix(&mut h, (k as u64 + 29) * 1_000_037 ^ id as u64);
        }
        if input.lens == EarthBimLens::ModalVibration {
            mix(&mut h, ((input.modal_phase_sin * 1000.0).round() as i32) as u64);
        }
        if input.lens == EarthBimLens::TimeHistoryReplay {
            mix(
                &mut h,
                ((input.replay_ground_shake_xyz_m[0] * 10_000.0).round() as i64) as u64,
            );
            mix(
                &mut h,
                ((input.replay_ground_shake_xyz_m[1] * 10_000.0).round() as i64) as u64,
            );
            mix(
                &mut h,
                ((input.replay_ground_shake_xyz_m[2] * 10_000.0).round() as i64) as u64,
            );
            for d in input.replay_story_disp_xyz_m {
                mix(&mut h, ((d[0] * 10_000.0).round() as i64) as u64);
                mix(&mut h, ((d[1] * 10_000.0).round() as i64) as u64);
                mix(&mut h, ((d[2] * 10_000.0).round() as i64) as u64);
            }
            for &(k, id, st) in input.replay_hinge_states {
                mix(&mut h, (k as u64 + 1) * 1_000_003 ^ id as u64 ^ (st as u64 * 97));
            }
        }
        if input.lens == EarthBimLens::ShmDigitalTwin {
            for &(elev, ret) in input.shm_story_retention {
                mix(&mut h, ((elev * 100.0).round() as i64) as u64);
                mix(&mut h, ((ret * 1000.0).round() as i64) as u64);
            }
        }
        h
    }

    fn compute_subsurface_hash(input: &SubsurfaceSsiGpuInput<'_>) -> u64 {
        let mut h = 0xcbf29ce484222325_u64;
        let mix = |acc: &mut u64, v: u64| {
            *acc ^= v;
            *acc = acc.wrapping_mul(0x100000001b3);
        };
        mix(&mut h, u64::from(input.config.ssi_coupling_enabled));
        mix(&mut h, u64::from(input.config.xray_subsurface_visible));
        mix(&mut h, input.config.borehole_preset as u64 + 1);
        mix(&mut h, input.config.groundwater_depth_m.to_bits());
        mix(&mut h, u64::from(input.config.liquefaction_trigger_active));
        mix(&mut h, (input.heading_deg * 100.0).round() as i64 as u64);
        mix(&mut h, input.project.columns.len() as u64);
        mix(&mut h, input.project.footings.len() as u64);
        if let Some((bmin, bmax)) = input.project.plan_extents_mm() {
            mix(&mut h, bmin[0].to_bits() ^ bmax[0].to_bits());
            mix(&mut h, bmin[1].to_bits() ^ bmax[1].to_bits());
        }
        if let Some(rep) = input.report {
            mix(&mut h, (rep.vs30_m_s * 10.0).round() as u64);
            mix(&mut h, (rep.liquefaction_potential_index * 100.0).round() as u64);
            mix(&mut h, (rep.recommended_pile_depth_m * 10.0).round() as u64);
        }
        h
    }

    /// Schedules the 100% Pure WebGPU (`egui_wgpu`) 3D Vector Earth pass onto `painter`.
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
        subsurface_input: Option<SubsurfaceSsiGpuInput<'_>>,
        portfolio_input: Option<PortfolioGpuOverlayInput<'_>>,
        hinge_input: Option<PlasticHingeGpuInput<'_>>,
        fragility_input: Option<FragilityLossGpuInput<'_>>,
        shm_input: Option<ShmTriageGpuInput<'_>>,
        evacuation_input: Option<EvacuationLifelineGpuInput<'_>>,
        risk_heatmap_input: Option<DistrictRiskHeatmapGpuInput<'_>>,
        hazard_compute_input: Option<HazardComputeInput>,
    ) -> bool {
        if !self.ready || !self.enabled || !rect.is_positive() {
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

        let pending_planet = if self.cached_planet_theme != Some(theme) {
            let mesh = build_planetary_vector_globe_mesh(theme);
            self.last_planet_tris = mesh.indices.len() / 3;
            self.cached_planet_theme = Some(theme);
            Some((theme, mesh))
        } else {
            None
        };

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

        let pending_subsurface = if render_site_pass {
            if let Some(ref ss_in) = subsurface_input {
                let rev = Self::compute_subsurface_hash(ss_in);
                if rev != self.cached_subsurface_hash {
                    let mesh = build_rtc_subsurface_ssi_mesh(ss_in);
                    self.last_subsurface_tris = mesh.indices.len() / 3;
                    self.cached_subsurface_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_subsurface_hash != 0 {
                self.cached_subsurface_hash = 0;
                self.last_subsurface_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_portfolio = if render_site_pass {
            if let Some(ref pf_in) = portfolio_input {
                let rev = pf_in.report.config.content_revision_hash()
                    ^ (pf_in.report.selected_frontier_idx as u64 + 1).wrapping_mul(1_000_003)
                    ^ (pf_in.report.total_buildings_evaluated as u64).wrapping_mul(2_654_435_761);
                if rev != self.cached_portfolio_hash {
                    let mesh = build_rtc_portfolio_retrofit_mesh(pf_in);
                    self.last_portfolio_tris = mesh.indices.len() / 3;
                    self.cached_portfolio_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_portfolio_hash != 0 {
                self.cached_portfolio_hash = 0;
                self.last_portfolio_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_hinges = if render_site_pass {
            if let Some(ref hg_in) = hinge_input {
                let mut rev = ((hg_in.frame.time_s * 1000.0).round() as u64).wrapping_mul(2_654_435_761)
                    ^ (hg_in.frame.active_hinges.len() as u64).wrapping_mul(1_000_003)
                    ^ u64::from(hg_in.show_hinges)
                    ^ ((hg_in.deform_scale * 100.0).round() as u64)
                    ^ ((hg_in.heading_deg * 100.0).round() as i64 as u64);
                rev ^= (hg_in.frame.ground_accel_g * 10_000.0).round() as i64 as u64;
                rev ^= (hg_in.frame.ground_accel_z_g * 10_000.0).round() as i64 as u64;
                if rev != self.cached_hinge_hash {
                    let mesh = build_rtc_plastic_hinge_mesh(hg_in);
                    self.last_hinge_tris = mesh.indices.len() / 3;
                    self.cached_hinge_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_hinge_hash != 0 {
                self.cached_hinge_hash = 0;
                self.last_hinge_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_fragility = if render_site_pass {
            if let Some(ref fg_in) = fragility_input {
                let rev = (fg_in.synthesis.story_losses.len() as u64 + 1).wrapping_mul(1_000_003)
                    ^ ((fg_in.synthesis.total_repair_cost_usd * 10.0).round() as u64)
                    ^ u64::from(fg_in.visible).wrapping_mul(2_654_435_761)
                    ^ ((fg_in.heading_deg * 100.0).round() as i64 as u64);
                if rev != self.cached_fragility_hash {
                    let mesh = build_rtc_fragility_loss_mesh(fg_in);
                    self.last_fragility_tris = mesh.indices.len() / 3;
                    self.cached_fragility_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_fragility_hash != 0 {
                self.cached_fragility_hash = 0;
                self.last_fragility_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_shm = if render_site_pass {
            if let Some(ref sh_in) = shm_input {
                let rev = sh_in.report.bim_revision_hash
                    ^ ((sh_in.report.sensors.len() as u64 + 1).wrapping_mul(1_000_003))
                    ^ ((sh_in.report.atc20_placard as u64 + 1).wrapping_mul(2_654_435_761))
                    ^ ((sh_in.report.district_placards.len() as u64).wrapping_mul(97))
                    ^ u64::from(sh_in.visible).wrapping_mul(73)
                    ^ ((sh_in.heading_deg * 100.0).round() as i64 as u64);
                if rev != self.cached_shm_hash {
                    let mesh = build_rtc_shm_triage_mesh(sh_in);
                    self.last_shm_tris = mesh.indices.len() / 3;
                    self.cached_shm_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_shm_hash != 0 {
                self.cached_shm_hash = 0;
                self.last_shm_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_evacuation = if render_site_pass {
            if let Some(ref ev_in) = evacuation_input {
                let rev = ev_in.report.config.content_revision_hash()
                    ^ ((ev_in.report.edges.len() as u64 + 1).wrapping_mul(1_000_003))
                    ^ ((ev_in.report.blocked_edges as u64 + 1).wrapping_mul(2_654_435_761))
                    ^ ((ev_in.report.routes.len() as u64).wrapping_mul(131))
                    ^ u64::from(ev_in.visible).wrapping_mul(97);
                if rev != self.cached_evacuation_hash {
                    let mesh = build_rtc_evacuation_lifelines_mesh(ev_in);
                    self.last_evacuation_tris = mesh.indices.len() / 3;
                    self.cached_evacuation_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_evacuation_hash != 0 {
                self.cached_evacuation_hash = 0;
                self.last_evacuation_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_risk_heatmap = if render_site_pass {
            if let Some(ref rh_in) = risk_heatmap_input {
                let rev = rh_in.report.content_hash
                    ^ ((rh_in.opacity * 100.0).round() as u64).wrapping_mul(1_000_003)
                    ^ u64::from(rh_in.visible).wrapping_mul(2_654_435_761);
                if rev != self.cached_risk_heatmap_hash {
                    let mesh = build_rtc_district_risk_heatmap_mesh(rh_in);
                    self.last_risk_heatmap_tris = mesh.indices.len() / 3;
                    self.cached_risk_heatmap_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_risk_heatmap_hash != 0 {
                self.cached_risk_heatmap_hash = 0;
                self.last_risk_heatmap_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_hazard_compute = if render_site_pass {
            if let Some(hc) = hazard_compute_input {
                if hc.enabled {
                    self.last_compute_particles =
                        (hc.params.particle_count_f32.round() as u32).clamp(64, MAX_GPU_HAZARD_PARTICLES);
                    Some(hc)
                } else {
                    self.last_compute_particles = 0;
                    None
                }
            } else {
                self.last_compute_particles = 0;
                None
            }
        } else {
            self.last_compute_particles = 0;
            None
        };
        let compute_animated = pending_hazard_compute.is_some();

        painter.add(egui_wgpu::Callback::new_paint_callback(
            rect,
            WebGpuEarthPaintCallback {
                viewport_rect: rect,
                uniforms,
                pending_planet,
                pending_site,
                pending_bim,
                pending_hazard,
                pending_subsurface,
                pending_portfolio,
                pending_hinges,
                pending_fragility,
                pending_shm,
                pending_evacuation,
                pending_risk_heatmap,
                pending_hazard_compute,
            },
        ));

        let hazard_animated = hazard_input
            .as_ref()
            .is_some_and(|h| h.config.is_animated());
        let subsurface_animated = subsurface_input
            .as_ref()
            .is_some_and(|s| s.config.xray_subsurface_visible);
        let replay_animated = hinge_input.is_some()
            || fragility_input.as_ref().is_some_and(|f| f.visible)
            || active_bim
                .as_ref()
                .is_some_and(|b| b.lens == EarthBimLens::TimeHistoryReplay);
        let shm_animated = shm_input.as_ref().is_some_and(|s| s.visible)
            || active_bim
                .as_ref()
                .is_some_and(|b| b.lens == EarthBimLens::ShmDigitalTwin);
        let evac_animated = evacuation_input.as_ref().is_some_and(|e| e.visible);
        let risk_heatmap_animated = risk_heatmap_input.as_ref().is_some_and(|r| r.visible);
        let frame_ms = if hazard_animated
            || compute_animated
            || subsurface_animated
            || replay_animated
            || shm_animated
            || evac_animated
            || risk_heatmap_animated
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
mod tests;
