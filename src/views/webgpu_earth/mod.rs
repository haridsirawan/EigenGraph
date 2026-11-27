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
    pub section_clip_enabled: bool,
    pub section_clip_terrain: bool,
    pub section_clip_min_enu: [f32; 3],
    pub section_clip_max_enu: [f32; 3],
    pub section_cut_rim_m: f32,
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
            section_clip_enabled: false,
            section_clip_terrain: false,
            section_clip_min_enu: [-30.0, -30.0, -10.0],
            section_clip_max_enu: [30.0, 30.0, 24.0],
            section_cut_rim_m: 0.14,
        }
    }

    /// Configures 3D hardware WGSL Section Box / Active Level Cut Plane clipping bounds (RTC ENU meters).
    pub fn with_section_box(
        mut self,
        section: &crate::views::universal_canvas::SectionBoxConfig,
    ) -> Self {
        let (lo, hi) = section.normalized_bounds();
        self.section_clip_enabled = section.enabled;
        self.section_clip_terrain = section.clip_terrain_and_city;
        self.section_clip_min_enu = lo;
        self.section_clip_max_enu = hi;
        self
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
            clip_min_and_mode: [0.0; 4],
            clip_max_and_band: [0.0; 4],
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
            clip_min_and_mode: [
                self.section_clip_min_enu[0],
                self.section_clip_min_enu[1],
                self.section_clip_min_enu[2],
                if self.section_clip_enabled { 1.0 } else { 0.0 },
            ],
            clip_max_and_band: [
                self.section_clip_max_enu[0],
                self.section_clip_max_enu[1],
                self.section_clip_max_enu[2],
                self.section_cut_rim_m,
            ],
        }
    }

    fn to_site_unclipped_pass_uniforms(&self) -> EarthPassUniforms {
        let mut u = self.to_site_pass_uniforms();
        u.clip_min_and_mode = [0.0; 4];
        u.clip_max_and_band = [0.0; 4];
        u
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
    clip_min_and_mode: [f32; 4],
    clip_max_and_band: [f32; 4],
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
    clip_min_and_mode: vec4<f32>,
    clip_max_and_band: vec4<f32>,
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
    let clip_active = u.clip_min_and_mode.w;
    var cut_rim = 0.0;
    if (clip_active > 0.5) {
        let cmin = u.clip_min_and_mode.xyz;
        let cmax = u.clip_max_and_band.xyz;
        let wp = f_in.world_pos;
        if (wp.x < cmin.x || wp.x > cmax.x || wp.y < cmin.y || wp.y > cmax.y || wp.z < cmin.z || wp.z > cmax.z) {
            discard;
        }
        let band = max(u.clip_max_and_band.w, 0.02);
        let d_min = min(wp.x - cmin.x, min(wp.y - cmin.y, wp.z - cmin.z));
        let d_max = min(cmax.x - wp.x, min(cmax.y - wp.y, cmax.z - wp.z));
        let d_edge = min(d_min, d_max);
        cut_rim = 1.0 - smoothstep(0.0, band, d_edge);
    }

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
    var out_alpha = f_in.color.a;

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
    } else if (kind >= 32.5) {
        // Update v0.8.7.7 (7/10 -> v0.8.8): Pure Analytical WGSL Subsurface Urban Utilities & Underground X-Ray (0% raster)
        let speed = 2.6 + clamp(height_m, 0.0, 1.0) * 2.4;
        let pulse = 0.78 + 0.28 * sin(phase * 2.8 - u_time * speed);
        let ndotv = max(abs(dot(n, v)), 0.14);
        let rim = pow(1.0 - ndotv, 1.45) * 0.52;
        col = f_in.color.rgb * (pulse + rim);
        out_alpha = f_in.color.a;
    } else if (kind >= 31.5) {
        // Update v0.8.7.6 (6/10 -> v0.8.8): Pure Analytical WGSL 3D Clash Detection Beacons & Overlap Boxes (0% raster)
        let sev = clamp(height_m, 0.2, 1.0);
        let pulse = 0.72 + 0.36 * sin(u_time * (3.0 + sev * 4.5) + phase * 1.5);
        let ndotv = max(abs(dot(n, v)), 0.10);
        let rim = pow(1.0 - ndotv, 1.45) * 0.55;
        col = f_in.color.rgb * (pulse + rim);
        out_alpha = f_in.color.a;
    } else if (kind >= 30.5) {
        // Update v0.8.7.5 (5/10 -> v0.8.8): Pure Analytical WGSL 3D Building MEP Ducts, Pipes & Risers (0% raster)
        let flow_wave = 0.84 + 0.16 * sin(phase * 2.4 - u_time * 3.0);
        let ndotv = max(abs(dot(n, v)), 0.14);
        let rim = pow(1.0 - ndotv, 1.65) * 0.36;
        col = f_in.color.rgb * (0.76 + 0.24 * flow_wave + rim) * mix(0.65, 1.0, lit);
        out_alpha = f_in.color.a;
    } else if (kind >= 29.5) {
        // Update v0.8.6.9 (9/10 -> v0.8.7): Emissive City Traffic Headlights (height_m > 0.5) & Taillights
        let boost = 1.0 + max(u_night, u_gold * 0.65) * 0.85;
        let beam = select(vec3<f32>(1.0, 0.22, 0.18), vec3<f32>(1.0, 0.96, 0.80), height_m > 0.5);
        col = beam * boost;
        out_alpha = 0.96;
    } else if (kind >= 28.5) {
        // Update v0.8.6.5 (5/10 -> v0.8.7): Realistic Coastal Ocean & River Water Surface (0% raster)
        let depth_m = max(height_m, 0.0);
        let p2 = f_in.world_pos.xy;
        let w_a = sin(p2.x * 0.22 + p2.y * 0.14 - u_time * 1.45);
        let w_b = cos(p2.x * 0.16 - p2.y * 0.27 - u_time * 1.85);
        let w_c = sin(p2.x * 0.58 + p2.y * 0.49 + u_time * 2.55) * 0.45;
        let w_d = cos(p2.x * 0.94 - p2.y * 0.82 - u_time * 3.10) * 0.25;
        let wave_amp = mix(0.06, 0.13, smoothstep(0.3, 6.0, depth_m));
        let n_w = normalize(vec3<f32>(
            -(w_a * 0.36 + w_b * 0.24 + w_c) * wave_amp,
            -(w_b * 0.34 - w_a * 0.22 + w_d) * wave_amp,
            1.0
        ));
        let shallow_col = vec3<f32>(0.15, 0.72, 0.76);
        let deep_col = vec3<f32>(0.02, 0.13, 0.29);
        let absorb = exp(-depth_m * vec3<f32>(0.42, 0.15, 0.07));
        let water_body = mix(deep_col, shallow_col, absorb);

        let surf_wave = sin(depth_m * 6.8 - u_time * 2.35 + phase * 4.2 + w_a * 1.4);
        let surf_band = smoothstep(0.35, 0.92, surf_wave) * (1.0 - smoothstep(0.15, 2.2, depth_m));
        let shore_froth = (1.0 - smoothstep(0.02, 0.42, depth_m)) * (0.72 + 0.28 * sin(p2.x * 0.65 - u_time * 2.0));
        let crest_foam = smoothstep(0.74, 0.96, abs(w_a * w_b) + abs(w_c)) * smoothstep(2.0, 9.0, depth_m) * 0.20;
        let foam = clamp(surf_band * 0.85 + shore_froth + crest_foam, 0.0, 1.0);

        let ndotl_w = max(dot(n_w, l), 0.0);
        let hv_w = l + v;
        let h_w = select(n_w, normalize(hv_w), dot(hv_w, hv_w) > 1e-8);
        let spec_w = pow(max(dot(n_w, h_w), 0.0), 110.0) * ndotl_w * 0.95;
        let fresnel_w = 0.04 + 0.96 * pow(clamp(1.0 - max(dot(n_w, v), 0.0), 0.0, 1.0), 4.0);
        let day_sky = mix(u_atmo * 1.15, vec3<f32>(0.38, 0.66, 0.92), 0.35);
        let sky_refl = mix(day_sky, vec3<f32>(0.98, 0.64, 0.36), u_gold * 0.55) * (1.0 - u_night * 0.72);
        let w_lit = mix(1.0, 0.32 + 0.68 * ndotl_w, u_lit);
        var base_water = mix(water_body * sun_tint * w_lit, sky_refl, clamp(fresnel_w * 0.76, 0.0, 0.85));
        base_water += sun_tint * spec_w * (1.0 - foam * 0.60);
        let foam_col = vec3<f32>(0.93, 0.97, 0.99) * sun_tint * (0.45 + 0.55 * w_lit);
        col = mix(base_water, foam_col, foam);
        out_alpha = clamp(0.46 + depth_m * 0.12 + foam * 0.45, 0.48, 0.96);
    } else if (kind >= 27.5) {
        // Update v0.8.6.5 (5/10 -> v0.8.7): Realistic Procedural Biome & Stratified Rock Mountain Terrain (0% raster)
        let px = f_in.world_pos.x;
        let py = f_in.world_pos.y;
        let n1 = sin(px * 0.055 + cos(py * 0.048) * 1.3) * cos(py * 0.052 - sin(px * 0.041) * 1.1);
        let n2 = sin(px * 0.17 - py * 0.14 + n1 * 1.6) * cos(px * 0.13 + py * 0.19);
        let n3 = sin(px * 0.46 + py * 0.41) * 0.5;
        let fbm = clamp(0.5 + 0.30 * n1 + 0.15 * n2 + 0.08 * n3, 0.0, 1.0);

        let elev_j = height_m + (fbm - 0.5) * 4.2;
        let seabed = mix(vec3<f32>(0.08, 0.22, 0.24), vec3<f32>(0.74, 0.70, 0.52), smoothstep(-10.0, -0.2, height_m));
        let wet_sand = mix(vec3<f32>(0.58, 0.52, 0.38), vec3<f32>(0.84, 0.78, 0.58), smoothstep(-0.3, 0.8, height_m));
        let grass_col = mix(vec3<f32>(0.24, 0.50, 0.22), vec3<f32>(0.36, 0.60, 0.27), fbm);
        let forest_col = mix(vec3<f32>(0.11, 0.31, 0.15), vec3<f32>(0.19, 0.42, 0.20), fbm);
        let rock_high = mix(vec3<f32>(0.27, 0.28, 0.30), vec3<f32>(0.44, 0.42, 0.39), fbm);
        let snow_cap = vec3<f32>(0.88, 0.91, 0.95);

        var biome = seabed;
        biome = mix(biome, wet_sand, smoothstep(-0.4, 0.4, elev_j));
        biome = mix(biome, grass_col, smoothstep(2.0, 5.5, elev_j));
        biome = mix(biome, forest_col, smoothstep(14.0, 26.0, elev_j));
        biome = mix(biome, rock_high, smoothstep(42.0, 56.0, elev_j));
        let snow_mask = smoothstep(68.0, 80.0, elev_j) * (1.0 - smoothstep(0.20, 0.50, phase));
        biome = mix(biome, snow_cap, snow_mask);

        let strata = 0.80 + 0.20 * sin(height_m * 1.45 + n1 * 2.2);
        let cliff_rock = mix(vec3<f32>(0.30, 0.29, 0.28), vec3<f32>(0.46, 0.43, 0.39), fbm) * strata;
        let cliff_m = smoothstep(0.15, 0.42, phase);
        var albedo = mix(biome, cliff_rock, cliff_m);

        let contour_p = fract(height_m / 5.0);
        let contour_line = (smoothstep(0.0, 0.03, contour_p) - smoothstep(0.03, 0.07, contour_p)) * step(0.2, height_m);
        albedo = mix(albedo, albedo * 0.72, contour_line * 0.22);

        let rim_terr = pow(max(1.0 - max(dot(n, v), 0.0), 0.0), 3.6);
        col = albedo * sun_tint * lit * hemi * ao + u_atmo * (rim_terr * 0.10);
    } else if (kind >= 26.5) {
        // Update v0.8.6.4 (4/10 -> v0.8.7): Pure Analytical WGSL Interior Daylight Autonomy & Lux Grid (0% raster)
        let lux_n = clamp(height_m, 0.0, 1.0);
        let shimmer = 0.86 + 0.14 * sin(phase * 2.1 - u_time * (1.4 + lux_n * 1.8));
        let ndotv = max(abs(dot(n, v)), 0.18);
        let rim = pow(1.0 - ndotv, 1.50) * (0.24 + 0.28 * lux_n);
        col = f_in.color.rgb * (0.80 + 0.20 * shimmer + rim);
    } else if (kind >= 25.5) {
        // Update v0.8.6.1 (1/10 -> v0.8.7): Pure Analytical WGSL 3D Solar Irradiation, Shadow & OTTV Heatmap (0% raster)
        let heat = clamp(height_m, 0.0, 1.0);
        let solar_pulse = 0.82 + 0.18 * sin(phase * 1.7 - u_time * (1.8 + heat * 2.2));
        let ndotv = max(abs(dot(n, v)), 0.16);
        let rim = pow(1.0 - ndotv, 1.55) * (0.28 + 0.30 * heat);
        col = f_in.color.rgb * (0.76 + 0.24 * solar_pulse + rim);
    } else if (kind >= 24.5) {
        // Update v0.8.5.8 (8/10 -> v0.8.6): Pure Analytical WGSL Multi-Option Architectural Massing Explorer (0% raster)
        let tier_wave = 0.84 + 0.16 * sin(f_in.world_pos.z * 2.1 - u_time * 1.6);
        let ndotv = max(abs(dot(n, v)), 0.16);
        let rim = pow(1.0 - ndotv, 1.55) * 0.46;
        col = f_in.color.rgb * (0.76 + 0.24 * tier_wave + rim);
    } else if (kind >= 23.5) {
        // Update v0.8.5.7: Pure Analytical WGSL Spatial Program 3D Stacked & Packed Space Blocks (0% raster)
        let band = 0.86 + 0.14 * sin(f_in.world_pos.z * 1.9 + u_time * 1.4);
        let ndotv = max(abs(dot(n, v)), 0.16);
        let rim = pow(1.0 - ndotv, 1.55) * 0.42;
        col = f_in.color.rgb * (0.78 + 0.22 * band + rim);
    } else if (kind >= 22.5) {
        // Update v0.8.5.6: Pure Analytical WGSL Site Zoning Buildable Envelope & Sun Path (0% raster)
        let grid_z = 0.82 + 0.18 * sin(f_in.world_pos.z * 2.4 - u_time * 2.6);
        let ndotv = max(abs(dot(n, v)), 0.14);
        let rim = pow(1.0 - ndotv, 1.45) * 0.52;
        col = f_in.color.rgb * (0.76 + 0.24 * grid_z + rim);
    } else if (kind >= 21.5) {
        // Update v0.8.4.9: Pure Analytical WGSL 4D/5D/6D Construction Sequence & Crane (0% raster)
        let scan = 0.78 + 0.22 * sin(f_in.world_pos.z * 3.0 - u_time * 3.6);
        let ndotv = max(abs(dot(n, v)), 0.14);
        let rim = pow(1.0 - ndotv, 1.5) * 0.44;
        col = f_in.color.rgb * (0.74 + 0.26 * scan + rim);
    } else if (kind >= 20.5) {
        // Update v0.8.4.8: Pure Analytical WGSL Deep Foundation Pile Group & Winkler Springs (0% raster)
        let soil_wave = 0.76 + 0.24 * sin(f_in.world_pos.z * 2.2 - u_time * 3.2);
        let load_norm = clamp(height_m, 0.0, 1.0);
        let ndotv = max(abs(dot(n, v)), 0.14);
        let rim = pow(1.0 - ndotv, 1.5) * (0.40 + 0.30 * load_norm);
        col = f_in.color.rgb * (0.74 + 0.28 * soil_wave + rim);
    } else if (kind >= 19.5) {
        // Update v0.8.4.7: Pure Analytical WGSL Structural Reliability FORM/SORM Beta & Risk Beacons (0% raster)
        let pulse = 0.74 + 0.28 * sin(phase * 1.8 - u_time * 4.2);
        let risk = clamp(height_m, 0.0, 1.0);
        let ndotv = max(abs(dot(n, v)), 0.12);
        let rim = pow(1.0 - ndotv, 1.45) * (0.42 + 0.34 * risk);
        col = f_in.color.rgb * (0.72 + 0.30 * pulse + rim);
    } else if (kind >= 18.5) {
        // Update v0.8.4.6: Pure Analytical WGSL Composite Metal Deck & Shear Studs (0% raster)
        let deck_rib = 0.78 + 0.20 * sin(f_in.world_pos.x * 9.0 + u_time * 3.0);
        let ndotv = max(abs(dot(n, v)), 0.12);
        let rim = pow(1.0 - ndotv, 1.5) * 0.44;
        col = f_in.color.rgb * (0.74 + 0.28 * deck_rib + rim);
    } else if (kind >= 17.5) {
        // Update v0.8.4.5: Pure Analytical WGSL Post-Tensioned Tendon Drape & Anchorages (0% raster)
        let tension = 0.72 + 0.34 * smoothstep(0.20, 0.95, sin(phase * 1.5 - u_time * 4.8));
        let ndotv = max(abs(dot(n, v)), 0.12);
        let rim = pow(1.0 - ndotv, 1.5) * 0.48;
        col = f_in.color.rgb * (0.72 + 0.36 * tension + rim);
    } else if (kind >= 16.5) {
        // Update v0.8.4.4: Pure Analytical WGSL Steel Seismic Connection Plates, Bolts & Panel Zone (0% raster)
        let glint = 0.78 + 0.24 * sin(phase * 1.6 - u_time * 3.4);
        let dcr_norm = clamp(height_m, 0.0, 1.0);
        let ndotv = max(abs(dot(n, v)), 0.12);
        let rim = pow(1.0 - ndotv, 1.4) * (0.45 + 0.25 * dcr_norm);
        col = f_in.color.rgb * (0.74 + 0.30 * glint + rim);
    } else if (kind >= 15.5) {
        // Update v0.8.4.3: Pure Analytical WGSL Special RC Shear Wall SBE Zones & Diagonal X-Cage (0% raster)
        let cage = 0.76 + 0.24 * sin(phase * 1.4 - u_time * 3.8);
        let dcr_norm = clamp(height_m, 0.0, 1.0);
        let ndotv = max(abs(dot(n, v)), 0.12);
        let rim = pow(1.0 - ndotv, 1.6) * (0.38 + 0.22 * dcr_norm);
        col = f_in.color.rgb * (0.72 + 0.34 * cage + rim);
    } else if (kind >= 14.5) {
        // Update v0.8.4.2: Pure Analytical WGSL Wind Aeroelasticity Facade Pressure & Flow Vectors (0% raster)
        let flow = 0.72 + 0.34 * smoothstep(0.20, 0.95, sin(phase * 1.3 - u_time * 5.8));
        let p_norm = clamp(height_m, 0.0, 1.0);
        let ndotv = max(abs(dot(n, v)), 0.12);
        let rim = pow(1.0 - ndotv, 1.6) * (0.40 + 0.25 * p_norm);
        col = f_in.color.rgb * (0.68 + 0.42 * flow + rim);
    } else if (kind >= 13.5) {
        // Phase 84.1: Pure Analytical WGSL Seismic Base Isolator Pads, Moat Ring & FVD Dampers (0% raster)
        let osc = 0.78 + 0.24 * sin(u_time * 3.2 + phase * 1.2);
        let stress = clamp(height_m, 0.0, 1.0);
        let ndotv = max(abs(dot(n, v)), 0.12);
        let rim = pow(1.0 - ndotv, 1.6) * (0.42 + 0.20 * stress);
        col = f_in.color.rgb * (osc + rim);
    } else if (kind >= 12.5) {
        // Phase 84: Pure Analytical WGSL Extreme Events Blast Wavefront & Fire Thermal Plume (0% raster)
        let wave_ring = 0.70 + 0.38 * smoothstep(0.15, 0.95, sin(phase * 1.1 - u_time * 6.0));
        let heat = clamp(height_m, 0.0, 1.0);
        let flicker = 0.84 + 0.24 * sin(u_time * (3.5 + heat * 6.0) + f_in.world_pos.x * 0.7 + f_in.world_pos.z * 1.1);
        let ndotv = max(abs(dot(n, v)), 0.10);
        let rim = pow(1.0 - ndotv, 1.5) * 0.52;
        col = f_in.color.rgb * (wave_ring * 0.62 + flicker * 0.58 + rim);
    } else if (kind >= 11.5) {
        // Phase 83: Pure Analytical WGSL EigenFlow Woodbury P1->P2 Load-Path & Pushover Hinges (0% raster)
        let flow_wave = 0.72 + 0.36 * smoothstep(0.25, 0.95, sin(phase * 1.4 - u_time * 5.2));
        let sev = clamp(height_m, 0.0, 1.0);
        let pulse = 0.85 + 0.25 * sin(u_time * (2.8 + sev * 4.5) + f_in.world_pos.z * 0.8);
        let ndotv = max(abs(dot(n, v)), 0.10);
        let rim = pow(1.0 - ndotv, 1.6) * 0.48;
        col = f_in.color.rgb * (flow_wave * 0.65 + pulse * 0.55 + rim);
    } else if (kind >= 10.5) {
        // Phase 82: Pure Analytical WGSL Field QA/QC Pour-Strips, ITP Hold Pins & NCR Beacons (0% raster)
        let severity = clamp(height_m, 0.0, 1.0);
        let rate = 2.0 + severity * 4.5;
        let pulse = 0.80 + 0.20 * sin(u_time * rate + f_in.world_pos.z * 0.9);
        let ndotv = max(abs(dot(n, v)), 0.10);
        let rim = pow(1.0 - ndotv, 1.6) * 0.45;
        col = f_in.color.rgb * (0.92 + 0.35 * pulse + rim);
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

    if (cut_rim > 0.001) {
        let cut_col = vec3<f32>(0.22, 0.88, 0.96);
        col = mix(col, cut_col, cut_rim * 0.72);
    }

    return vec4<f32>(clamp(col, vec3<f32>(0.0), vec3<f32>(1.0)), out_alpha);
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
    xray_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    composite_bgl: wgpu::BindGroupLayout,
    composite_sampler: wgpu::Sampler,
    targets: Option<OffscreenEarthTargets>,
    rendered_this_frame: bool,
    planet_uniform_buf: wgpu::Buffer,
    site_uniform_buf: wgpu::Buffer,
    site_unclipped_uniform_buf: wgpu::Buffer,
    planet_bind_group: wgpu::BindGroup,
    site_bind_group: wgpu::BindGroup,
    site_unclipped_bind_group: wgpu::BindGroup,
    split_terrain_clip: bool,
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
    field_qaqc: GpuMeshSlot,
    eigenflow: GpuMeshSlot,
    extreme_events: GpuMeshSlot,
    isolation: GpuMeshSlot,
    wind_aero: GpuMeshSlot,
    shear_wall: GpuMeshSlot,
    steel_connection: GpuMeshSlot,
    prestress: GpuMeshSlot,
    composite_deck: GpuMeshSlot,
    reliability_form: GpuMeshSlot,
    deep_foundation: GpuMeshSlot,
    construction_lca: GpuMeshSlot,
    zoning_envelope: GpuMeshSlot,
    spatial_program: GpuMeshSlot,
    massing_study: GpuMeshSlot,
    solar_envelope: GpuMeshSlot,
    daylight_interior: GpuMeshSlot,
    mep_systems: GpuMeshSlot,
    clash_guardrail: GpuMeshSlot,
    urban_utilities: GpuMeshSlot,
    urban_utilities_xray: bool,
    water_surface: GpuMeshSlot,
    city_ambience: GpuMeshSlot,
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
        let build_pipeline = |label: &str, depth_write: bool, depth_compare: wgpu::CompareFunction| {
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
                    depth_compare: Some(depth_compare),
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

        let opaque_pipeline = build_pipeline(
            "eigengraph_earth_opaque",
            true,
            wgpu::CompareFunction::LessEqual,
        );
        let translucent_pipeline = build_pipeline(
            "eigengraph_earth_translucent",
            false,
            wgpu::CompareFunction::LessEqual,
        );
        let xray_pipeline = build_pipeline(
            "eigengraph_earth_xray",
            false,
            wgpu::CompareFunction::Always,
        );

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
        let site_unclipped_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("eigengraph_site_unclipped_uniforms"),
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
        let site_unclipped_bind_group =
            make_bg("eigengraph_site_unclipped_bg", &site_unclipped_uniform_buf);

        Self {
            target_format,
            opaque_pipeline,
            translucent_pipeline,
            xray_pipeline,
            composite_pipeline,
            composite_bgl,
            composite_sampler,
            targets: None,
            rendered_this_frame: false,
            planet_uniform_buf,
            site_uniform_buf,
            site_unclipped_uniform_buf,
            planet_bind_group,
            site_bind_group,
            site_unclipped_bind_group,
            split_terrain_clip: false,
            planet_theme_uploaded: None,
            planet: GpuMeshSlot::default(),
            site: GpuMeshSlot::default(),
            water_surface: GpuMeshSlot::default(),
            city_ambience: GpuMeshSlot::default(),
            bim: GpuMeshSlot::default(),
            hinges: GpuMeshSlot::default(),
            subsurface: GpuMeshSlot::default(),
            portfolio: GpuMeshSlot::default(),
            hazard: GpuMeshSlot::default(),
            fragility_loss: GpuMeshSlot::default(),
            shm_triage: GpuMeshSlot::default(),
            evacuation: GpuMeshSlot::default(),
            risk_heatmap: GpuMeshSlot::default(),
            field_qaqc: GpuMeshSlot::default(),
            eigenflow: GpuMeshSlot::default(),
            extreme_events: GpuMeshSlot::default(),
            isolation: GpuMeshSlot::default(),
            wind_aero: GpuMeshSlot::default(),
            shear_wall: GpuMeshSlot::default(),
            steel_connection: GpuMeshSlot::default(),
            prestress: GpuMeshSlot::default(),
            composite_deck: GpuMeshSlot::default(),
            reliability_form: GpuMeshSlot::default(),
            deep_foundation: GpuMeshSlot::default(),
            construction_lca: GpuMeshSlot::default(),
            zoning_envelope: GpuMeshSlot::default(),
            spatial_program: GpuMeshSlot::default(),
            massing_study: GpuMeshSlot::default(),
            solar_envelope: GpuMeshSlot::default(),
            daylight_interior: GpuMeshSlot::default(),
            mep_systems: GpuMeshSlot::default(),
            clash_guardrail: GpuMeshSlot::default(),
            urban_utilities: GpuMeshSlot::default(),
            urban_utilities_xray: true,
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
            || self.water_surface.index_count > 0
            || self.city_ambience.index_count > 0
            || self.bim.index_count > 0
            || self.hinges.index_count > 0
            || self.subsurface.index_count > 0
            || self.portfolio.index_count > 0
            || self.hazard.index_count > 0
            || self.fragility_loss.index_count > 0
            || self.shm_triage.index_count > 0
            || self.evacuation.index_count > 0
            || self.risk_heatmap.index_count > 0
            || self.field_qaqc.index_count > 0
            || self.eigenflow.index_count > 0
            || self.extreme_events.index_count > 0
            || self.isolation.index_count > 0
            || self.wind_aero.index_count > 0
            || self.shear_wall.index_count > 0
            || self.steel_connection.index_count > 0
            || self.prestress.index_count > 0
            || self.composite_deck.index_count > 0
            || self.reliability_form.index_count > 0
            || self.deep_foundation.index_count > 0
            || self.construction_lca.index_count > 0
            || self.zoning_envelope.index_count > 0
            || self.spatial_program.index_count > 0
            || self.massing_study.index_count > 0
            || self.solar_envelope.index_count > 0
            || self.daylight_interior.index_count > 0
            || self.mep_systems.index_count > 0
            || self.clash_guardrail.index_count > 0
            || self.urban_utilities.index_count > 0
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
            let terrain_bg = if self.split_terrain_clip {
                &self.site_unclipped_bind_group
            } else {
                &self.site_bind_group
            };
            pass.set_bind_group(0, terrain_bg, &[]);
            self.site.draw(&mut pass);
            pass.set_bind_group(0, &self.site_bind_group, &[]);
            self.bim.draw(&mut pass);

            pass.set_pipeline(&self.translucent_pipeline);
            pass.set_bind_group(0, terrain_bg, &[]);
            self.water_surface.draw(&mut pass);
            self.city_ambience.draw(&mut pass);
            pass.set_bind_group(0, &self.site_bind_group, &[]);
            self.hinges.draw(&mut pass);
            self.subsurface.draw(&mut pass);
            self.risk_heatmap.draw(&mut pass);
            self.field_qaqc.draw(&mut pass);
            self.eigenflow.draw(&mut pass);
            self.extreme_events.draw(&mut pass);
            self.isolation.draw(&mut pass);
            self.wind_aero.draw(&mut pass);
            self.shear_wall.draw(&mut pass);
            self.steel_connection.draw(&mut pass);
            self.prestress.draw(&mut pass);
            self.composite_deck.draw(&mut pass);
            self.reliability_form.draw(&mut pass);
            self.deep_foundation.draw(&mut pass);
            self.construction_lca.draw(&mut pass);
            self.zoning_envelope.draw(&mut pass);
            self.spatial_program.draw(&mut pass);
            self.massing_study.draw(&mut pass);
            self.solar_envelope.draw(&mut pass);
            self.daylight_interior.draw(&mut pass);
            self.mep_systems.draw(&mut pass);
            self.clash_guardrail.draw(&mut pass);
            self.portfolio.draw(&mut pass);
            self.hazard.draw(&mut pass);
            self.hazard_compute.draw(&mut pass);
            self.fragility_loss.draw(&mut pass);
            self.shm_triage.draw(&mut pass);
            self.evacuation.draw(&mut pass);
            if self.urban_utilities.index_count > 0 {
                if self.urban_utilities_xray {
                    pass.set_pipeline(&self.xray_pipeline);
                } else {
                    pass.set_pipeline(&self.translucent_pipeline);
                }
                self.urban_utilities.draw(&mut pass);
            }
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
    pending_field_qaqc: Option<(u64, GpuMeshPayload)>,
    pending_eigenflow: Option<(u64, GpuMeshPayload)>,
    pending_extreme_events: Option<(u64, GpuMeshPayload)>,
    pending_isolation: Option<(u64, GpuMeshPayload)>,
    pending_wind: Option<(u64, GpuMeshPayload)>,
    pending_shear_wall: Option<(u64, GpuMeshPayload)>,
    pending_connection: Option<(u64, GpuMeshPayload)>,
    pending_prestress: Option<(u64, GpuMeshPayload)>,
    pending_composite: Option<(u64, GpuMeshPayload)>,
    pending_reliability: Option<(u64, GpuMeshPayload)>,
    pending_foundation: Option<(u64, GpuMeshPayload)>,
    pending_construction_lca: Option<(u64, GpuMeshPayload)>,
    pending_zoning_envelope: Option<(u64, GpuMeshPayload)>,
    pending_spatial_program: Option<(u64, GpuMeshPayload)>,
    pending_massing_study: Option<(u64, GpuMeshPayload)>,
    pending_solar_envelope: Option<(u64, GpuMeshPayload)>,
    pending_daylight_interior: Option<(u64, GpuMeshPayload)>,
    pending_mep_systems: Option<(u64, GpuMeshPayload)>,
    pending_clash_guardrail: Option<(u64, GpuMeshPayload)>,
    pending_urban_utilities: Option<(u64, GpuMeshPayload)>,
    urban_utilities_xray: bool,
    pending_water_surface: Option<(u64, GpuMeshPayload)>,
    pending_city_ambience: Option<(u64, GpuMeshPayload)>,
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
        if let Some(ref fq_in) = self.pending_field_qaqc {
            if res.field_qaqc.revision_uploaded != fq_in.0 {
                res.field_qaqc
                    .upload(device, queue, "field_qaqc_mesh", fq_in.0, &fq_in.1);
            }
        }
        if let Some(ref ef_in) = self.pending_eigenflow {
            if res.eigenflow.revision_uploaded != ef_in.0 {
                res.eigenflow
                    .upload(device, queue, "eigenflow_mesh", ef_in.0, &ef_in.1);
            }
        }
        if let Some(ref ex_in) = self.pending_extreme_events {
            if res.extreme_events.revision_uploaded != ex_in.0 {
                res.extreme_events
                    .upload(device, queue, "extreme_events_mesh", ex_in.0, &ex_in.1);
            }
        }
        if let Some(ref is_in) = self.pending_isolation {
            if res.isolation.revision_uploaded != is_in.0 {
                res.isolation
                    .upload(device, queue, "isolation_mesh", is_in.0, &is_in.1);
            }
        }
        if let Some(ref wi_in) = self.pending_wind {
            if res.wind_aero.revision_uploaded != wi_in.0 {
                res.wind_aero
                    .upload(device, queue, "wind_aero_mesh", wi_in.0, &wi_in.1);
            }
        }
        if let Some(ref sw_in) = self.pending_shear_wall {
            if res.shear_wall.revision_uploaded != sw_in.0 {
                res.shear_wall
                    .upload(device, queue, "shear_wall_mesh", sw_in.0, &sw_in.1);
            }
        }
        if let Some(ref cn_in) = self.pending_connection {
            if res.steel_connection.revision_uploaded != cn_in.0 {
                res.steel_connection
                    .upload(device, queue, "steel_connection_mesh", cn_in.0, &cn_in.1);
            }
        }
        if let Some(ref pt_in) = self.pending_prestress {
            if res.prestress.revision_uploaded != pt_in.0 {
                res.prestress
                    .upload(device, queue, "prestress_tendon_mesh", pt_in.0, &pt_in.1);
            }
        }
        if let Some(ref cp_in) = self.pending_composite {
            if res.composite_deck.revision_uploaded != cp_in.0 {
                res.composite_deck
                    .upload(device, queue, "composite_deck_mesh", cp_in.0, &cp_in.1);
            }
        }
        if let Some(ref rel_in) = self.pending_reliability {
            if res.reliability_form.revision_uploaded != rel_in.0 {
                res.reliability_form
                    .upload(device, queue, "reliability_form_mesh", rel_in.0, &rel_in.1);
            }
        }
        if let Some(ref fnd_in) = self.pending_foundation {
            if res.deep_foundation.revision_uploaded != fnd_in.0 {
                res.deep_foundation
                    .upload(device, queue, "deep_foundation_mesh", fnd_in.0, &fnd_in.1);
            }
        }
        if let Some(ref lca_in) = self.pending_construction_lca {
            if res.construction_lca.revision_uploaded != lca_in.0 {
                res.construction_lca
                    .upload(device, queue, "construction_lca_mesh", lca_in.0, &lca_in.1);
            }
        }
        if let Some(ref zon_in) = self.pending_zoning_envelope {
            if res.zoning_envelope.revision_uploaded != zon_in.0 {
                res.zoning_envelope
                    .upload(device, queue, "zoning_envelope_mesh", zon_in.0, &zon_in.1);
            }
        }
        if let Some(ref prg_in) = self.pending_spatial_program {
            if res.spatial_program.revision_uploaded != prg_in.0 {
                res.spatial_program
                    .upload(device, queue, "spatial_program_mesh", prg_in.0, &prg_in.1);
            }
        }
        if let Some(ref ms_in) = self.pending_massing_study {
            if res.massing_study.revision_uploaded != ms_in.0 {
                res.massing_study
                    .upload(device, queue, "massing_study_mesh", ms_in.0, &ms_in.1);
            }
        }
        if let Some(ref sol_in) = self.pending_solar_envelope {
            if res.solar_envelope.revision_uploaded != sol_in.0 {
                res.solar_envelope
                    .upload(device, queue, "solar_envelope_mesh", sol_in.0, &sol_in.1);
            }
        }
        if let Some(ref dl_in) = self.pending_daylight_interior {
            if res.daylight_interior.revision_uploaded != dl_in.0 {
                res.daylight_interior
                    .upload(device, queue, "daylight_interior_mesh", dl_in.0, &dl_in.1);
            }
        }
        if let Some(ref mep_in) = self.pending_mep_systems {
            if res.mep_systems.revision_uploaded != mep_in.0 {
                res.mep_systems
                    .upload(device, queue, "mep_systems_mesh", mep_in.0, &mep_in.1);
            }
        }
        if let Some(ref cl_in) = self.pending_clash_guardrail {
            if res.clash_guardrail.revision_uploaded != cl_in.0 {
                res.clash_guardrail
                    .upload(device, queue, "clash_guardrail_mesh", cl_in.0, &cl_in.1);
            }
        }
        res.urban_utilities_xray = self.urban_utilities_xray;
        if let Some(ref util_in) = self.pending_urban_utilities {
            if res.urban_utilities.revision_uploaded != util_in.0 {
                res.urban_utilities
                    .upload(device, queue, "urban_utilities_mesh", util_in.0, &util_in.1);
            }
        }
        if let Some(ref ws_in) = self.pending_water_surface {
            if res.water_surface.revision_uploaded != ws_in.0 {
                res.water_surface
                    .upload(device, queue, "water_surface_mesh", ws_in.0, &ws_in.1);
            }
        }
        if let Some(ref ca_in) = self.pending_city_ambience {
            if res.city_ambience.revision_uploaded != ca_in.0 {
                res.city_ambience
                    .upload(device, queue, "city_ambience_mesh", ca_in.0, &ca_in.1);
            }
        }
        if let Some(ref h_cmp) = self.pending_hazard_compute {
            res.hazard_compute.dispatch(queue, egui_encoder, h_cmp);
        } else {
            res.hazard_compute.active_vertex_count = 0;
        }

        let pu = self.uniforms.to_planet_pass_uniforms();
        let su = self.uniforms.to_site_pass_uniforms();
        let su_unclipped = self.uniforms.to_site_unclipped_pass_uniforms();
        res.split_terrain_clip =
            self.uniforms.section_clip_enabled && !self.uniforms.section_clip_terrain;
        queue.write_buffer(&res.planet_uniform_buf, 0, struct_as_u8_slice(&pu));
        queue.write_buffer(&res.site_uniform_buf, 0, struct_as_u8_slice(&su));
        queue.write_buffer(
            &res.site_unclipped_uniform_buf,
            0,
            struct_as_u8_slice(&su_unclipped),
        );

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
    cached_field_qaqc_hash: u64,
    cached_eigenflow_hash: u64,
    cached_extreme_events_hash: u64,
    cached_isolation_hash: u64,
    cached_wind_hash: u64,
    cached_shear_wall_hash: u64,
    cached_connection_hash: u64,
    cached_prestress_hash: u64,
    cached_composite_hash: u64,
    cached_reliability_hash: u64,
    cached_foundation_hash: u64,
    cached_construction_lca_hash: u64,
    cached_zoning_envelope_hash: u64,
    cached_spatial_program_hash: u64,
    cached_massing_study_hash: u64,
    cached_solar_envelope_hash: u64,
    cached_daylight_interior_hash: u64,
    cached_mep_systems_hash: u64,
    cached_clash_guardrail_hash: u64,
    cached_urban_utilities_hash: u64,
    cached_water_surface_hash: u64,
    cached_city_ambience_hash: u64,
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
    pub last_field_qaqc_tris: usize,
    pub last_eigenflow_tris: usize,
    pub last_extreme_events_tris: usize,
    pub last_isolation_tris: usize,
    pub last_wind_tris: usize,
    pub last_shear_wall_tris: usize,
    pub last_connection_tris: usize,
    pub last_prestress_tris: usize,
    pub last_composite_tris: usize,
    pub last_reliability_tris: usize,
    pub last_foundation_tris: usize,
    pub last_construction_lca_tris: usize,
    pub last_zoning_envelope_tris: usize,
    pub last_spatial_program_tris: usize,
    pub last_massing_study_tris: usize,
    pub last_solar_envelope_tris: usize,
    pub last_daylight_interior_tris: usize,
    pub last_mep_systems_tris: usize,
    pub last_clash_guardrail_tris: usize,
    pub last_urban_utilities_tris: usize,
    pub last_water_surface_tris: usize,
    pub last_city_ambience_tris: usize,
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
        realistic_landscape: bool,
        sea_level_m: f32,
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
        mix(&mut h, u64::from(realistic_landscape) * 71);
        mix(&mut h, sea_level_m.to_bits() as u64);
        if let Some(t) = tin {
            mix(&mut h, t.points.len() as u64);
            mix(&mut h, t.triangles.len() as u64);
            mix(&mut h, t.min_elevation.to_bits() as u64);
            mix(&mut h, (t.max_elevation * 100.0) as i32 as u64);
            mix(&mut h, t.extent_x_m.to_bits() as u64);
            mix(&mut h, t.revision.wrapping_mul(0x9E37_79B9_7F4A_7C15));
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
        realistic_landscape: bool,
        sea_level_m: f32,
        active_site_tin: Option<&TinSurfaceMesh>,
        world_index: &WorldSpatialIndex,
        active_rel_path: Option<&str>,
        urban_vectors: &UrbanVectorLayerCache,
        city_scene: Option<&IfcxCityScene>,
        water_surface_input: Option<CoastalWaterGpuInput<'_>>,
        active_bim: Option<ActiveBimGpuInput<'_>>,
        hazard_input: Option<DistrictHazardGpuInput<'_>>,
        subsurface_input: Option<SubsurfaceSsiGpuInput<'_>>,
        portfolio_input: Option<PortfolioGpuOverlayInput<'_>>,
        hinge_input: Option<PlasticHingeGpuInput<'_>>,
        fragility_input: Option<FragilityLossGpuInput<'_>>,
        shm_input: Option<ShmTriageGpuInput<'_>>,
        evacuation_input: Option<EvacuationLifelineGpuInput<'_>>,
        risk_heatmap_input: Option<DistrictRiskHeatmapGpuInput<'_>>,
        field_qaqc_input: Option<FieldQaqcGpuInput<'_>>,
        eigenflow_input: Option<EigenFlowGpuInput<'_>>,
        extreme_events_input: Option<ExtremeEventsGpuInput<'_>>,
        isolation_input: Option<IsolationGpuInput<'_>>,
        wind_input: Option<WindGpuInput<'_>>,
        shear_wall_input: Option<ShearWallGpuInput<'_>>,
        connection_input: Option<ConnectionGpuInput<'_>>,
        prestress_input: Option<PrestressGpuInput<'_>>,
        composite_input: Option<CompositeGpuInput<'_>>,
        reliability_input: Option<ReliabilityGpuInput<'_>>,
        foundation_input: Option<FoundationGpuInput<'_>>,
        construction_lca_input: Option<ConstructionLcaGpuInput<'_>>,
        zoning_envelope_input: Option<ZoningEnvelopeGpuInput<'_>>,
        spatial_program_input: Option<SpatialProgramGpuInput<'_>>,
        massing_study_input: Option<MassingStudyGpuInput<'_>>,
        solar_envelope_input: Option<SolarEnvelopeGpuInput<'_>>,
        daylight_interior_input: Option<DaylightInteriorGpuInput<'_>>,
        mep_systems_input: Option<MepSystemsGpuInput<'_>>,
        clash_guardrail_input: Option<ClashGuardrailGpuInput<'_>>,
        urban_utilities_input: Option<UrbanUtilitiesGpuInput<'_>>,
        hazard_compute_input: Option<HazardComputeInput>,
        section_box: &crate::views::universal_canvas::SectionBoxConfig,
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
        )
        .with_section_box(section_box);

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
                realistic_landscape,
                sea_level_m,
                active_site_tin,
                world_index,
                active_rel_path,
                urban_vectors,
                city_scene,
            );
            if rev != self.cached_site_hash {
                let mesh = build_rtc_site_scene_mesh_with_landscape(
                    theme,
                    site_origin,
                    world_map.zoom_level,
                    show_terrain_contours,
                    active_site_tin,
                    world_index,
                    active_rel_path,
                    urban_vectors,
                    city_scene,
                    realistic_landscape,
                    sea_level_m,
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

        let pending_field_qaqc = if render_site_pass {
            if let Some(ref fq_in) = field_qaqc_input {
                let mut rev: u64 = (fq_in.report.waypoints.len() as u64)
                    ^ ((fq_in.report.open_ncr_count as u64) << 12)
                    ^ ((fq_in.report.itp_cleared_count as u64) << 20)
                    ^ ((fq_in.heading_deg.round() as u64) << 28)
                    ^ u64::from(fq_in.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                for wp in &fq_in.report.waypoints {
                    rev = rev.wrapping_mul(131).wrapping_add(wp.id as u64 + wp.status as u64 * 17);
                }
                if rev != self.cached_field_qaqc_hash {
                    let mesh = build_rtc_field_qaqc_mesh(fq_in);
                    self.last_field_qaqc_tris = mesh.indices.len() / 3;
                    self.cached_field_qaqc_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_field_qaqc_hash != 0 {
                self.cached_field_qaqc_hash = 0;
                self.last_field_qaqc_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_eigenflow = if render_site_pass {
            if let Some(ref ef_in) = eigenflow_input {
                let wb = &ef_in.report.woodbury;
                let pat = ef_in.report.config.load_pattern as u64;
                let rev: u64 = (wb.disrupted_element_id as u64).wrapping_mul(131)
                    ^ wb.lambda_val.to_bits()
                    ^ (pat << 18)
                    ^ ((ef_in.active_step_idx as u64) << 24)
                    ^ ((wb.redistributed_p2.len() as u64) << 36)
                    ^ ((ef_in.heading_deg.round() as u64) << 44)
                    ^ u64::from(ef_in.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_eigenflow_hash {
                    let mesh = build_rtc_eigenflow_mesh(ef_in);
                    self.last_eigenflow_tris = mesh.indices.len() / 3;
                    self.cached_eigenflow_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_eigenflow_hash != 0 {
                self.cached_eigenflow_hash = 0;
                self.last_eigenflow_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_extreme_events = if render_site_pass {
            if let Some(ref ex_in) = extreme_events_input {
                let c = &ex_in.report.config;
                let rev: u64 = ((c.mode as u64) << 4)
                    ^ ((c.explosive_type as u64) << 8)
                    ^ c.explosive_mass_kg.to_bits()
                    ^ c.standoff_distance_m.to_bits().rotate_left(7)
                    ^ c.exposure_minutes.to_bits().rotate_left(13)
                    ^ ((c.fire_story_idx as u64) << 28)
                    ^ ((c.fire_curve as u64) << 36)
                    ^ ((c.protection_kind as u64) << 40)
                    ^ ((ex_in.heading_deg.round() as u64) << 46)
                    ^ u64::from(ex_in.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_extreme_events_hash {
                    let mesh = build_rtc_extreme_events_mesh(ex_in);
                    self.last_extreme_events_tris = mesh.indices.len() / 3;
                    self.cached_extreme_events_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_extreme_events_hash != 0 {
                self.cached_extreme_events_hash = 0;
                self.last_extreme_events_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_isolation = if render_site_pass {
            if let Some(ref is_in) = isolation_input {
                let c = &is_in.report.config;
                let rev: u64 = ((c.isolator_type as u64) << 4)
                    ^ c.lrb_outer_diameter_mm.to_bits()
                    ^ c.fps_radius_mm.to_bits().rotate_left(7)
                    ^ c.fvd_damping_c.to_bits().rotate_left(13)
                    ^ ((c.num_dampers as u64) << 24)
                    ^ ((is_in.report.pads_3d.len() as u64) << 32)
                    ^ ((is_in.heading_deg.round() as u64) << 44)
                    ^ u64::from(is_in.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_isolation_hash {
                    let mesh = build_rtc_isolation_mesh(is_in);
                    self.last_isolation_tris = mesh.indices.len() / 3;
                    self.cached_isolation_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_isolation_hash != 0 {
                self.cached_isolation_hash = 0;
                self.last_isolation_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_wind = if render_site_pass {
            if let Some(ref wi_in) = wind_input {
                let c = &wi_in.report.config;
                let rev: u64 = ((c.exposure as u64) << 4)
                    ^ ((c.risk_category as u64) << 8)
                    ^ ((c.enclosure as u64) << 12)
                    ^ c.basic_wind_speed_ms.to_bits()
                    ^ c.wind_azimuth_deg.to_bits().rotate_left(9)
                    ^ c.damping_ratio.to_bits().rotate_left(17)
                    ^ (u64::from(c.occupancy_residential) << 20)
                    ^ ((wi_in.report.panels_3d.len() as u64) << 28)
                    ^ ((wi_in.heading_deg.round() as u64) << 44)
                    ^ u64::from(wi_in.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_wind_hash {
                    let mesh = build_rtc_wind_mesh(wi_in);
                    self.last_wind_tris = mesh.indices.len() / 3;
                    self.cached_wind_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_wind_hash != 0 {
                self.cached_wind_hash = 0;
                self.last_wind_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_shear_wall = if render_site_pass {
            if let Some(ref sw_in) = shear_wall_input {
                let c = &sw_in.report.config;
                let rev: u64 = c.wall_thickness_mm.to_bits()
                    ^ c.fc_mpa.to_bits().rotate_left(7)
                    ^ c.design_drift_ratio.to_bits().rotate_left(13)
                    ^ c.cb_clear_span_m.to_bits().rotate_left(19)
                    ^ c.cb_depth_m.to_bits().rotate_left(23)
                    ^ c.cb_diag_bar_dia_mm.to_bits().rotate_left(29)
                    ^ ((c.cb_bars_per_group as u64) << 12)
                    ^ ((sw_in.report.piers_3d.len() as u64) << 24)
                    ^ ((sw_in.heading_deg.round() as u64) << 44)
                    ^ u64::from(sw_in.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_shear_wall_hash {
                    let mesh = build_rtc_shear_wall_mesh(sw_in);
                    self.last_shear_wall_tris = mesh.indices.len() / 3;
                    self.cached_shear_wall_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_shear_wall_hash != 0 {
                self.cached_shear_wall_hash = 0;
                self.last_shear_wall_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_connection = if render_site_pass {
            if let Some(ref cn_in) = connection_input {
                let c = &cn_in.report.config;
                let rev: u64 = ((c.connection_type as u64) << 4)
                    ^ ((c.bolt_grade as u64) << 8)
                    ^ c.bolt_diameter_mm.to_bits()
                    ^ c.plate_thickness_mm.to_bits().rotate_left(7)
                    ^ ((c.bolt_rows as u64) << 16)
                    ^ ((cn_in.report.joints_3d.len() as u64) << 24)
                    ^ ((cn_in.heading_deg.round() as u64) << 44)
                    ^ u64::from(cn_in.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_connection_hash {
                    let mesh = build_rtc_connection_mesh(cn_in);
                    self.last_connection_tris = mesh.indices.len() / 3;
                    self.cached_connection_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_connection_hash != 0 {
                self.cached_connection_hash = 0;
                self.last_connection_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_prestress = if render_site_pass {
            if let Some(ref pt_in) = prestress_input {
                let c = &pt_in.report.config;
                let rev: u64 = ((c.profile_kind as u64) << 4)
                    ^ ((c.method as u64) << 8)
                    ^ ((c.archetype as u64) << 12)
                    ^ ((c.num_strands as u64) << 16)
                    ^ c.eccentricity_mid_mm.to_bits()
                    ^ c.fc_prime_mpa.to_bits().rotate_left(9)
                    ^ ((pt_in.report.bays_3d.len() as u64) << 28)
                    ^ ((pt_in.heading_deg.round() as u64) << 44)
                    ^ u64::from(pt_in.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_prestress_hash {
                    let mesh = build_rtc_prestress_mesh(pt_in);
                    self.last_prestress_tris = mesh.indices.len() / 3;
                    self.cached_prestress_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_prestress_hash != 0 {
                self.cached_prestress_hash = 0;
                self.last_prestress_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_composite = if render_site_pass {
            if let Some(ref cp_in) = composite_input {
                let c = &cp_in.report.config;
                let rev: u64 = ((c.deck_orientation as u64) << 4)
                    ^ ((c.studs_per_rib as u64) << 8)
                    ^ ((c.studs_provided as u64) << 12)
                    ^ (u64::from(c.unshored) << 20)
                    ^ c.total_slab_h_mm.to_bits()
                    ^ c.slab_fc_mpa.to_bits().rotate_left(11)
                    ^ ((cp_in.report.bays_3d.len() as u64) << 28)
                    ^ ((cp_in.heading_deg.round() as u64) << 44)
                    ^ u64::from(cp_in.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_composite_hash {
                    let mesh = build_rtc_composite_mesh(cp_in);
                    self.last_composite_tris = mesh.indices.len() / 3;
                    self.cached_composite_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_composite_hash != 0 {
                self.cached_composite_hash = 0;
                self.last_composite_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_reliability = if render_site_pass {
            if let Some(ref rel_in) = reliability_input {
                let c = &rel_in.report.config;
                let rev: u64 = ((c.method as u64) << 4)
                    ^ ((c.limit_state as u64) << 8)
                    ^ c.target_beta.to_bits()
                    ^ c.demand_to_capacity_ratio.to_bits().rotate_left(13)
                    ^ c.resistance_cov.to_bits().rotate_left(23)
                    ^ c.seismic_fraction.to_bits().rotate_left(31)
                    ^ ((rel_in.report.members_3d.len() as u64) << 28)
                    ^ ((rel_in.heading_deg.round() as u64) << 44)
                    ^ u64::from(rel_in.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_reliability_hash {
                    let mesh = build_rtc_reliability_mesh(rel_in);
                    self.last_reliability_tris = mesh.indices.len() / 3;
                    self.cached_reliability_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_reliability_hash != 0 {
                self.cached_reliability_hash = 0;
                self.last_reliability_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_foundation = if render_site_pass {
            if let Some(ref fnd_in) = foundation_input {
                let c = &fnd_in.report.config;
                let rev: u64 = ((c.soil_type as u64) << 4)
                    ^ ((c.piles_x as u64) << 8)
                    ^ ((c.piles_y as u64) << 12)
                    ^ c.pile_length_m.to_bits()
                    ^ c.pile_diameter_m.to_bits().rotate_left(13)
                    ^ c.cap_thickness_m.to_bits().rotate_left(23)
                    ^ c.column_pu_kn.to_bits().rotate_left(31)
                    ^ ((fnd_in.report.piles_3d.len() as u64) << 28)
                    ^ ((fnd_in.heading_deg.round() as u64) << 44)
                    ^ u64::from(fnd_in.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_foundation_hash {
                    let mesh = build_rtc_foundation_mesh(fnd_in);
                    self.last_foundation_tris = mesh.indices.len() / 3;
                    self.cached_foundation_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_foundation_hash != 0 {
                self.cached_foundation_hash = 0;
                self.last_foundation_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_construction_lca = if render_site_pass {
            if let Some(ref lca_in) = construction_lca_input {
                let c = &lca_in.report.config;
                let rev: u64 = (c.active_day as u64)
                    ^ ((c.steel_route as u64) << 12)
                    ^ ((c.concrete_mix as u64) << 16)
                    ^ ((c.crane_crews as u64) << 20)
                    ^ ((lca_in.report.story_zones_3d.len() as u64) << 28)
                    ^ ((lca_in.heading_deg.round() as u64) << 44)
                    ^ u64::from(lca_in.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_construction_lca_hash {
                    let mesh = build_rtc_construction_lca_mesh(lca_in);
                    self.last_construction_lca_tris = mesh.indices.len() / 3;
                    self.cached_construction_lca_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_construction_lca_hash != 0 {
                self.cached_construction_lca_hash = 0;
                self.last_construction_lca_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_zoning_envelope = if render_site_pass {
            if let Some(ref zon_in) = zoning_envelope_input {
                let c = &zon_in.report.config;
                let rev: u64 = ((c.preset as u64) << 2)
                    ^ c.lot_width_m.to_bits()
                    ^ c.lot_depth_m.to_bits().rotate_left(7)
                    ^ c.kdb_bcr_pct.to_bits().rotate_left(13)
                    ^ c.klb_far.to_bits().rotate_left(19)
                    ^ c.max_height_m.to_bits().rotate_left(25)
                    ^ c.setback_front_m.to_bits().rotate_left(31)
                    ^ c.setback_side_m.to_bits().rotate_left(37)
                    ^ c.solar_hour_local.to_bits().rotate_left(43)
                    ^ ((c.podium_stories as u64) << 10)
                    ^ ((c.solar_month as u64) << 16)
                    ^ u64::from(c.show_sun_path).wrapping_mul(0x517C_C1B7_2722_0A95)
                    ^ u64::from(zon_in.report.overall_compliant).wrapping_mul(0x94D0_49BB_1331_11EB)
                    ^ ((zon_in.heading_deg.round() as u64) << 48)
                    ^ u64::from(zon_in.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_zoning_envelope_hash {
                    let mesh = build_rtc_zoning_envelope_mesh(zon_in);
                    self.last_zoning_envelope_tris = mesh.indices.len() / 3;
                    self.cached_zoning_envelope_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_zoning_envelope_hash != 0 {
                self.cached_zoning_envelope_hash = 0;
                self.last_zoning_envelope_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_spatial_program = if render_site_pass {
            if let Some(ref prg_in) = spatial_program_input {
                let c = &prg_in.report.config;
                let rev: u64 = ((c.typology as u64) << 2)
                    ^ ((c.spaces.len() as u64) << 8)
                    ^ prg_in.report.total_nfa_m2.to_bits()
                    ^ c.circulation_factor_pct.to_bits().rotate_left(11)
                    ^ c.core_ratio_pct.to_bits().rotate_left(19)
                    ^ ((prg_in.report.stacked_stories as u64) << 24)
                    ^ ((prg_in.report.packed_blocks_3d.len() as u64) << 32)
                    ^ c.selected_story_filter
                        .map_or(0xFFFF, |f| (f as u64) + 1)
                        .rotate_left(40)
                    ^ ((prg_in.heading_deg.round() as u64) << 48)
                    ^ u64::from(prg_in.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_spatial_program_hash {
                    let mesh = build_rtc_spatial_program_mesh(prg_in);
                    self.last_spatial_program_tris = mesh.indices.len() / 3;
                    self.cached_spatial_program_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_spatial_program_hash != 0 {
                self.cached_spatial_program_hash = 0;
                self.last_spatial_program_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_massing_study = if render_site_pass {
            if let Some(ref ms_in) = massing_study_input {
                let c = &ms_in.report.config;
                let active_gfa_bits = ms_in
                    .report
                    .active_option()
                    .map_or(0, |o| o.gfa_m2.to_bits() ^ o.estimated_eigengate_ari.to_bits());
                let rev: u64 = ((ms_in.report.selected_index as u64 + 1) << 2)
                    ^ ((c.structural_system as u64 + 1) << 8)
                    ^ c.bay_x_m.to_bits().rotate_left(13)
                    ^ c.bay_y_m.to_bits().rotate_left(21)
                    ^ c.core_wall_thick_mm.to_bits().rotate_left(29)
                    ^ u64::from(c.show_structural_grid_preview).rotate_left(37)
                    ^ active_gfa_bits
                    ^ ((ms_in.heading_deg.round() as u64) << 48)
                    ^ u64::from(ms_in.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_massing_study_hash {
                    let mesh = build_rtc_massing_study_mesh(ms_in);
                    self.last_massing_study_tris = mesh.indices.len() / 3;
                    self.cached_massing_study_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_massing_study_hash != 0 {
                self.cached_massing_study_hash = 0;
                self.last_massing_study_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_solar_envelope = if render_site_pass {
            if let Some(ref solar) = solar_envelope_input.as_ref().filter(|s| s.visible) {
                let c = &solar.report.config;
                let rev: u64 = ((c.glazing_preset as u64 + 1) << 2)
                    ^ ((c.shading_strategy as u64 + 1) << 7)
                    ^ c.wwr_pct.to_bits().rotate_left(11)
                    ^ c.shgc.to_bits().rotate_left(17)
                    ^ c.u_wall_w_m2k.to_bits().rotate_left(23)
                    ^ c.shading_depth_m.to_bits().rotate_left(29)
                    ^ ((c.inspection_month as u64) << 33)
                    ^ c.inspection_hour_local.to_bits().rotate_left(39)
                    ^ u64::from(c.show_shadow_polygon).rotate_left(43)
                    ^ solar.report.overall_ottv_w_m2.to_bits().rotate_left(47)
                    ^ solar.report.sun_altitude_deg.to_bits().rotate_left(53)
                    ^ ((solar.heading_deg.round() as u64) << 57)
                    ^ u64::from(solar.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_solar_envelope_hash {
                    let mesh = build_rtc_solar_envelope_mesh(solar);
                    self.last_solar_envelope_tris = mesh.indices.len() / 3;
                    self.cached_solar_envelope_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_solar_envelope_hash != 0 {
                self.cached_solar_envelope_hash = 0;
                self.last_solar_envelope_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_daylight_interior = if render_site_pass {
            if let Some(ref dl) = daylight_interior_input.as_ref().filter(|d| d.visible) {
                let c = &dl.report.config;
                let rev: u64 = ((c.finish_palette as u64 + 1) << 2)
                    ^ ((c.space_typology as u64 + 1) << 7)
                    ^ c.wwr_pct.to_bits().rotate_left(11)
                    ^ c.visible_transmittance_vlt.to_bits().rotate_left(17)
                    ^ c.shading_transmittance.to_bits().rotate_left(23)
                    ^ ((c.active_story_index as u64) << 29)
                    ^ c.work_plane_height_m.to_bits().rotate_left(35)
                    ^ dl.report.sda_300_50_pct.to_bits().rotate_left(41)
                    ^ dl.report.mean_lux.to_bits().rotate_left(47)
                    ^ ((dl.heading_deg.round() as u64) << 55)
                    ^ u64::from(dl.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_daylight_interior_hash {
                    let mesh = build_rtc_daylight_interior_mesh(dl);
                    self.last_daylight_interior_tris = mesh.indices.len() / 3;
                    self.cached_daylight_interior_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_daylight_interior_hash != 0 {
                self.cached_daylight_interior_hash = 0;
                self.last_daylight_interior_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_water_surface = if render_site_pass {
            if let Some(ref ws) = water_surface_input.as_ref().filter(|w| w.visible) {
                let t = ws.tin;
                let rev: u64 = (t.points.len() as u64).wrapping_mul(1_000_003)
                    ^ (t.triangles.len() as u64).wrapping_mul(2_654_435_761)
                    ^ ws.sea_level_m.to_bits() as u64
                    ^ t.min_elevation.to_bits().rotate_left(13) as u64
                    ^ t.max_elevation.to_bits().rotate_left(23) as u64
                    ^ t.revision.wrapping_mul(0x9E37_79B9_7F4A_7C15)
                    ^ u64::from(ws.visible).wrapping_mul(0x517C_C1B7_2722_0A95);
                if rev != self.cached_water_surface_hash {
                    let mesh = build_rtc_coastal_water_mesh(ws);
                    self.last_water_surface_tris = mesh.indices.len() / 3;
                    self.cached_water_surface_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_water_surface_hash != 0 {
                self.cached_water_surface_hash = 0;
                self.last_water_surface_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let ambience_active = render_site_pass
            && city_scene.is_some_and(|c| {
                !c.is_empty() && (c.ambience.show_animated_traffic || c.ambience.show_dynamic_shadows)
            });
        let pending_city_ambience = if ambience_active {
            if let Some(city) = city_scene {
                let anim_bucket = if city.ambience.show_animated_traffic && !city.roads.is_empty() {
                    (time_secs * 24.0).floor() as u64
                } else {
                    0
                };
                let sun_q = ((uniforms.sun_dir_site[0] * 100.0).round() as i64 as u64)
                    ^ ((uniforms.sun_dir_site[1] * 100.0).round() as i64 as u64).rotate_left(13)
                    ^ ((uniforms.sun_dir_site[2] * 100.0).round() as i64 as u64).rotate_left(27);
                let rev: u64 = city.content_revision_hash()
                    ^ (anim_bucket.wrapping_mul(1_000_003))
                    ^ sun_q
                    ^ sea_level_m.to_bits() as u64
                    ^ active_site_tin.map(|t| t.revision).unwrap_or(0)
                    ^ u64::from(city.ambience.show_animated_traffic).wrapping_mul(73)
                    ^ u64::from(city.ambience.show_dynamic_shadows).wrapping_mul(131);
                if rev != self.cached_city_ambience_hash {
                    let mesh = build_rtc_city_ambience_mesh(&CityAmbienceGpuInput {
                        city,
                        tin: active_site_tin,
                        sea_level_m,
                        sun_dir_site: uniforms.sun_dir_site,
                        time_secs,
                        visible: true,
                    });
                    self.last_city_ambience_tris = mesh.indices.len() / 3;
                    self.cached_city_ambience_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else {
                None
            }
        } else if self.cached_city_ambience_hash != 0 {
            self.cached_city_ambience_hash = 0;
            self.last_city_ambience_tris = 0;
            Some((0, GpuMeshPayload::default()))
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

        let pending_mep_systems = if render_site_pass {
            if let Some(ref mp) = mep_systems_input.as_ref().filter(|m| m.visible) {
                let c = &mp.report.config;
                let vis_mask = u64::from(c.show_hvac)
                    | (u64::from(c.show_plumbing) << 1)
                    | (u64::from(c.show_electrical) << 2)
                    | (u64::from(c.show_fire) << 3);
                let rev: u64 = ((c.hvac_preset as u64 + 1) << 2)
                    ^ ((c.fire_hazard as u64 + 1) << 6)
                    ^ (vis_mask << 10)
                    ^ ((mp.report.segments_3d.len() as u64) << 16)
                    ^ mp.report.cooling_tr.to_bits().rotate_left(23)
                    ^ mp.report.elec_demand_kva.to_bits().rotate_left(31)
                    ^ c.plenum_drop_below_slab_m.to_bits().rotate_left(39)
                    ^ ((mp.heading_deg.round() as u64) << 51)
                    ^ u64::from(mp.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_mep_systems_hash {
                    let mesh = build_rtc_mep_systems_mesh(mp);
                    self.last_mep_systems_tris = mesh.indices.len() / 3;
                    self.cached_mep_systems_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_mep_systems_hash != 0 {
                self.cached_mep_systems_hash = 0;
                self.last_mep_systems_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let pending_clash_guardrail = if render_site_pass {
            if let Some(ref cl) = clash_guardrail_input.as_ref().filter(|c| c.visible) {
                let c = &cl.report.config;
                let rev: u64 = ((cl.report.clashes.len() as u64) << 2)
                    ^ ((cl.report.hard_clashes as u64) << 12)
                    ^ ((cl.report.clearance_warnings as u64) << 20)
                    ^ c.clearance_tolerance_mm.to_bits().rotate_left(27)
                    ^ (u64::from(c.show_hard_only) << 39)
                    ^ (u64::from(c.check_structure_vs_mep) << 40)
                    ^ (u64::from(c.check_mep_vs_mep) << 41)
                    ^ (u64::from(c.include_sleeve_advisories) << 42)
                    ^ ((cl.heading_deg.round() as u64) << 51)
                    ^ u64::from(cl.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_clash_guardrail_hash {
                    let mesh = build_rtc_clash_guardrail_mesh(cl);
                    self.last_clash_guardrail_tris = mesh.indices.len() / 3;
                    self.cached_clash_guardrail_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_clash_guardrail_hash != 0 {
                self.cached_clash_guardrail_hash = 0;
                self.last_clash_guardrail_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

        let urban_utilities_xray = urban_utilities_input
            .as_ref()
            .map_or(true, |u| u.report.config.xray_underground);
        let pending_urban_utilities = if render_site_pass {
            if let Some(ref ut) = urban_utilities_input.as_ref().filter(|u| u.visible) {
                let c = &ut.report.config;
                let vis_mask = u64::from(c.show_water)
                    | (u64::from(c.show_sewer) << 1)
                    | (u64::from(c.show_power) << 2)
                    | (u64::from(c.xray_underground) << 3);
                let rev: u64 = (vis_mask << 2)
                    ^ ((ut.report.segments_3d.len() as u64) << 10)
                    ^ ((ut.report.plants_3d.len() as u64) << 20)
                    ^ ut.report.total_water_length_m.to_bits().rotate_left(13)
                    ^ ut.report.total_sewer_length_m.to_bits().rotate_left(29)
                    ^ ut.report.total_power_length_m.to_bits().rotate_left(41)
                    ^ c.water_burial_depth_m.to_bits().rotate_left(53)
                    ^ u64::from(ut.visible).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                if rev != self.cached_urban_utilities_hash {
                    let mesh = build_rtc_urban_utilities_mesh(ut);
                    self.last_urban_utilities_tris = mesh.indices.len() / 3;
                    self.cached_urban_utilities_hash = rev;
                    Some((rev, mesh))
                } else {
                    None
                }
            } else if self.cached_urban_utilities_hash != 0 {
                self.cached_urban_utilities_hash = 0;
                self.last_urban_utilities_tris = 0;
                Some((0, GpuMeshPayload::default()))
            } else {
                None
            }
        } else {
            None
        };

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
                pending_field_qaqc,
                pending_eigenflow,
                pending_extreme_events,
                pending_isolation,
                pending_wind,
                pending_shear_wall,
                pending_connection,
                pending_prestress,
                pending_composite,
                pending_reliability,
                pending_foundation,
                pending_construction_lca,
                pending_zoning_envelope,
                pending_spatial_program,
                pending_massing_study,
                pending_solar_envelope,
                pending_daylight_interior,
                pending_mep_systems,
                pending_clash_guardrail,
                pending_urban_utilities,
                urban_utilities_xray,
                pending_water_surface,
                pending_city_ambience,
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
        let field_qaqc_animated = field_qaqc_input.as_ref().is_some_and(|f| f.visible);
        let eigenflow_animated = eigenflow_input.as_ref().is_some_and(|e| e.visible);
        let extreme_events_animated = extreme_events_input.as_ref().is_some_and(|x| x.visible);
        let isolation_animated = isolation_input.as_ref().is_some_and(|i| i.visible);
        let wind_animated = wind_input.as_ref().is_some_and(|w| w.visible);
        let shear_wall_animated = shear_wall_input.as_ref().is_some_and(|s| s.visible);
        let connection_animated = connection_input.as_ref().is_some_and(|c| c.visible);
        let prestress_animated = prestress_input.as_ref().is_some_and(|p| p.visible);
        let composite_animated = composite_input.as_ref().is_some_and(|cp| cp.visible);
        let reliability_animated = reliability_input.as_ref().is_some_and(|r| r.visible);
        let foundation_animated = foundation_input.as_ref().is_some_and(|f| f.visible);
        let construction_lca_animated = construction_lca_input.as_ref().is_some_and(|l| l.visible);
        let zoning_animated = zoning_envelope_input.as_ref().is_some_and(|z| z.visible);
        let program_animated = spatial_program_input.as_ref().is_some_and(|p| p.visible);
        let massing_animated = massing_study_input.as_ref().is_some_and(|m| m.visible);
        let solar_animated = solar_envelope_input.as_ref().is_some_and(|s| s.visible);
        let daylight_animated = daylight_interior_input.as_ref().is_some_and(|d| d.visible);
        let mep_animated = mep_systems_input.as_ref().is_some_and(|m| m.visible);
        let clash_animated = clash_guardrail_input.as_ref().is_some_and(|c| c.visible);
        let urban_utilities_animated = urban_utilities_input.as_ref().is_some_and(|u| u.visible);
        let water_animated = water_surface_input.as_ref().is_some_and(|w| w.visible);
        let ambience_animated = render_site_pass
            && city_scene.is_some_and(|c| c.ambience.show_animated_traffic && !c.roads.is_empty());
        let frame_ms = if hazard_animated
            || compute_animated
            || subsurface_animated
            || replay_animated
            || shm_animated
            || evac_animated
            || risk_heatmap_animated
            || field_qaqc_animated
            || eigenflow_animated
            || extreme_events_animated
            || isolation_animated
            || wind_animated
            || shear_wall_animated
            || connection_animated
            || prestress_animated
            || composite_animated
            || reliability_animated
            || foundation_animated
            || construction_lca_animated
            || zoning_animated
            || program_animated
            || massing_animated
            || solar_animated
            || daylight_animated
            || mep_animated
            || clash_animated
            || urban_utilities_animated
            || water_animated
            || ambience_animated
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
