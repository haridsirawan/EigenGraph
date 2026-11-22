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

use crate::core::gis::city_builder::IfcxCityScene;
use crate::core::gis::earth_globe::{
    lat_lon_to_unit_sphere, GlobeCamera, GlobeScreenTransform, PlanetaryWorldState,
};
use crate::core::gis::map_widget::{SlippyMapState, VectorEarthTheme, EARTH_RADIUS_METERS};
use crate::core::gis::tin_surface::TinSurfaceMesh;
use crate::core::gis::vector_layers::UrbanVectorLayerCache;
use crate::core::gis::world_index::WorldSpatialIndex;
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


mod mesh_builders;
pub use mesh_builders::*;

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
    subsurface_vao: glow::VertexArray,
    subsurface_vbo: glow::Buffer,
    subsurface_ibo: glow::Buffer,
    subsurface_index_count: i32,
    subsurface_revision_uploaded: u64,
    portfolio_vao: glow::VertexArray,
    portfolio_vbo: glow::Buffer,
    portfolio_ibo: glow::Buffer,
    portfolio_index_count: i32,
    portfolio_revision_uploaded: u64,
    hinge_vao: glow::VertexArray,
    hinge_vbo: glow::Buffer,
    hinge_ibo: glow::Buffer,
    hinge_index_count: i32,
    hinge_revision_uploaded: u64,
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
                }} else if (kind >= 5.5) {{
                    // Analytical pulsing 3D Plastic Hinge Beacons & Ground-Motion Pulse Ring (0% raster)
                    float pulse = 0.68 + 0.32 * sin(u_time * (3.2 + phase * 6.0));
                    col = mix(col, v_color.rgb * 1.18, 0.55) * pulse;
                }} else if (kind >= 4.5) {{
                    // Animated Groundwater Phreatic Table Surface Ripples (100% procedural)
                    float wave = 0.5 + 0.5 * sin(v_pos.x * 1.8 + u_time * 1.6) * cos(v_pos.y * 1.6 - u_time * 1.2);
                    col += vec3(0.08, 0.24, 0.32) * wave;
                }} else if (kind >= 3.5 && kind < 4.5) {{
                    // Subsurface Geological Stratigraphy Bedding & Sedimentary Striations (0% raster)
                    float bed = 0.88 + 0.12 * sin(v_pos.z * 7.5 + sin(v_pos.x * 0.45 + v_pos.y * 0.35) * 1.4);
                    col *= bed;
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
            let (subsurface_vao, subsurface_vbo, subsurface_ibo) = setup_vao_pair()?;
            let (portfolio_vao, portfolio_vbo, portfolio_ibo) = setup_vao_pair()?;
            let (hinge_vao, hinge_vbo, hinge_ibo) = setup_vao_pair()?;

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
                subsurface_vao,
                subsurface_vbo,
                subsurface_ibo,
                subsurface_index_count: 0,
                subsurface_revision_uploaded: 0,
                portfolio_vao,
                portfolio_vbo,
                portfolio_ibo,
                portfolio_index_count: 0,
                portfolio_revision_uploaded: 0,
                hinge_vao,
                hinge_vbo,
                hinge_ibo,
                hinge_index_count: 0,
                hinge_revision_uploaded: 0,
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

    fn upload_subsurface(&mut self, gl: &glow::Context, rev: u64, mesh: &GpuMeshPayload) {
        unsafe {
            gl.bind_vertex_array(Some(self.subsurface_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.subsurface_vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                vertices_as_u8_slice(&mesh.vertices),
                glow::DYNAMIC_DRAW,
            );
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.subsurface_ibo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                indices_as_u8_slice(&mesh.indices),
                glow::DYNAMIC_DRAW,
            );
            gl.bind_vertex_array(None);
        }
        self.subsurface_index_count = mesh.indices.len() as i32;
        self.subsurface_revision_uploaded = rev;
    }

    fn upload_portfolio(&mut self, gl: &glow::Context, rev: u64, mesh: &GpuMeshPayload) {
        unsafe {
            gl.bind_vertex_array(Some(self.portfolio_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.portfolio_vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                vertices_as_u8_slice(&mesh.vertices),
                glow::DYNAMIC_DRAW,
            );
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.portfolio_ibo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                indices_as_u8_slice(&mesh.indices),
                glow::DYNAMIC_DRAW,
            );
            gl.bind_vertex_array(None);
        }
        self.portfolio_index_count = mesh.indices.len() as i32;
        self.portfolio_revision_uploaded = rev;
    }

    fn upload_hinges(&mut self, gl: &glow::Context, rev: u64, mesh: &GpuMeshPayload) {
        unsafe {
            gl.bind_vertex_array(Some(self.hinge_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.hinge_vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                vertices_as_u8_slice(&mesh.vertices),
                glow::DYNAMIC_DRAW,
            );
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.hinge_ibo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                indices_as_u8_slice(&mesh.indices),
                glow::DYNAMIC_DRAW,
            );
            gl.bind_vertex_array(None);
        }
        self.hinge_index_count = mesh.indices.len() as i32;
        self.hinge_revision_uploaded = rev;
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

            // --- Pass 2: Floating-Origin RTC Local Site Scene (3D IfcSite TIN + City-Builder + Extruded IFCX + Active BIM/FEM/Modal/N-1/Replay + Subsurface + Portfolio + Multi-Hazard + Hinges) ---
            if u.render_site_pass
                && (self.site_index_count > 0
                    || self.bim_index_count > 0
                    || self.subsurface_index_count > 0
                    || self.portfolio_index_count > 0
                    || self.hazard_index_count > 0
                    || self.hinge_index_count > 0)
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
                if self.hinge_index_count > 0 {
                    gl.depth_mask(false);
                    gl.bind_vertex_array(Some(self.hinge_vao));
                    gl.draw_elements(glow::TRIANGLES, self.hinge_index_count, glow::UNSIGNED_INT, 0);
                    gl.depth_mask(true);
                }
                if self.subsurface_index_count > 0 {
                    gl.depth_mask(false);
                    gl.bind_vertex_array(Some(self.subsurface_vao));
                    gl.draw_elements(glow::TRIANGLES, self.subsurface_index_count, glow::UNSIGNED_INT, 0);
                    gl.depth_mask(true);
                }
                if self.portfolio_index_count > 0 {
                    gl.depth_mask(false);
                    gl.bind_vertex_array(Some(self.portfolio_vao));
                    gl.draw_elements(glow::TRIANGLES, self.portfolio_index_count, glow::UNSIGNED_INT, 0);
                    gl.depth_mask(true);
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
            gl.delete_vertex_array(self.subsurface_vao);
            gl.delete_buffer(self.subsurface_vbo);
            gl.delete_buffer(self.subsurface_ibo);
            gl.delete_vertex_array(self.portfolio_vao);
            gl.delete_buffer(self.portfolio_vbo);
            gl.delete_buffer(self.portfolio_ibo);
            gl.delete_vertex_array(self.hinge_vao);
            gl.delete_buffer(self.hinge_vbo);
            gl.delete_buffer(self.hinge_ibo);
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
    cached_subsurface_hash: u64,
    cached_portfolio_hash: u64,
    cached_hinge_hash: u64,
    pub last_planet_tris: usize,
    pub last_site_tris: usize,
    pub last_bim_tris: usize,
    pub last_hazard_tris: usize,
    pub last_subsurface_tris: usize,
    pub last_portfolio_tris: usize,
    pub last_hinge_tris: usize,
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
            // Quantize animated sine phase at 1000 steps so GPU modal vibration updates smoothly at 60 FPS
            mix(&mut h, ((input.modal_phase_sin * 1000.0).round() as i32) as u64);
        }
        if input.lens == EarthBimLens::TimeHistoryReplay {
            mix(
                &mut h,
                ((input.replay_ground_shake_xy_m[0] * 10_000.0).round() as i64) as u64,
            );
            mix(
                &mut h,
                ((input.replay_ground_shake_xy_m[1] * 10_000.0).round() as i64) as u64,
            );
            for d in input.replay_story_disp_xy_m {
                mix(&mut h, ((d[0] * 10_000.0).round() as i64) as u64);
                mix(&mut h, ((d[1] * 10_000.0).round() as i64) as u64);
            }
            for &(k, id, st) in input.replay_hinge_states {
                mix(&mut h, (k as u64 + 1) * 1_000_003 ^ id as u64 ^ (st as u64 * 97));
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
        subsurface_input: Option<SubsurfaceSsiGpuInput<'_>>,
        portfolio_input: Option<PortfolioGpuOverlayInput<'_>>,
        hinge_input: Option<PlasticHingeGpuInput<'_>>,
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

        // Prepare 3D Subsurface Geological Stratigraphy, Liquefaction & Groundwater Table X-Ray mesh
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

        // Prepare 3D District Portfolio Retrofit Crown Overlay mesh
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

        // Prepare 3D ASCE 41-17 Plastic Hinge Beacons & Seismic Ground-Pulse Ring mesh
        let pending_hinges = if render_site_pass {
            if let Some(ref hg_in) = hinge_input {
                let mut rev = ((hg_in.frame.time_s * 1000.0).round() as u64).wrapping_mul(2_654_435_761)
                    ^ (hg_in.frame.active_hinges.len() as u64).wrapping_mul(1_000_003)
                    ^ u64::from(hg_in.show_hinges)
                    ^ ((hg_in.deform_scale * 100.0).round() as u64)
                    ^ ((hg_in.heading_deg * 100.0).round() as i64 as u64);
                rev ^= (hg_in.frame.ground_accel_g * 10_000.0).round() as i64 as u64;
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

        let planet_payload = Arc::new(Mutex::new(pending_planet));
        let site_payload = Arc::new(Mutex::new(pending_site));
        let bim_payload = Arc::new(Mutex::new(pending_bim));
        let hazard_payload = Arc::new(Mutex::new(pending_hazard));
        let subsurface_payload = Arc::new(Mutex::new(pending_subsurface));
        let portfolio_payload = Arc::new(Mutex::new(pending_portfolio));
        let hinge_payload = Arc::new(Mutex::new(pending_hinges));

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
                if let Ok(mut slot) = subsurface_payload.lock() {
                    if let Some((rev, mesh)) = slot.take() {
                        if pipe.subsurface_revision_uploaded != rev {
                            pipe.upload_subsurface(gl, rev, &mesh);
                        }
                    }
                }
                if let Ok(mut slot) = portfolio_payload.lock() {
                    if let Some((rev, mesh)) = slot.take() {
                        if pipe.portfolio_revision_uploaded != rev {
                            pipe.upload_portfolio(gl, rev, &mesh);
                        }
                    }
                }
                if let Ok(mut slot) = hinge_payload.lock() {
                    if let Some((rev, mesh)) = slot.take() {
                        if pipe.hinge_revision_uploaded != rev {
                            pipe.upload_hinges(gl, rev, &mesh);
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
        // Smooth ocean shimmer, 60 FPS GPU modal oscillation, groundwater ripples, seismic waves, and NL-THA replay
        let hazard_animated = hazard_input
            .as_ref()
            .is_some_and(|h| h.config.is_animated());
        let subsurface_animated = subsurface_input
            .as_ref()
            .is_some_and(|s| s.config.xray_subsurface_visible);
        let replay_animated = hinge_input.is_some()
            || active_bim
                .as_ref()
                .is_some_and(|b| b.lens == EarthBimLens::TimeHistoryReplay);
        let frame_ms = if hazard_animated
            || subsurface_animated
            || replay_animated
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
