//! webgpu_earth/compute_pass.rs — Pure WebGPU `@compute` WGSL Multi-Hazard Particle Pipeline (Phase 79).
//!
//! Dispatches `@compute @workgroup_size(64)` before Pass 2 (`eigengraph_earth_site_pass`) to update
//! 512–4,096 traveling seismic wavefield ripples and Venturi urban wind streamline particles around
//! `WorldSpatialIndex` building obstacles directly into a shared `STORAGE | VERTEX` buffer—zero
//! CPU-GPU readback stalls per frame, with an exact CPU fallback (`step_hazard_particles_cpu`).

use std::borrow::Cow;

use eframe::wgpu;

use super::{struct_as_u8_slice, GpuEarthVertex};

pub const MAX_GPU_HAZARD_PARTICLES: u32 = 4096;
pub const MAX_GPU_HAZARD_OBSTACLES: usize = 64;

/// 64-byte (`16 x f32`) uniform buffer for `HAZARD_COMPUTE_WGSL`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HazardComputeParams {
    pub time_secs: f32,
    pub pga_g: f32,
    pub wave_speed_ms: f32,
    pub wave_azimuth_rad: f32,
    pub wind_speed_ms: f32,
    pub wind_azimuth_rad: f32,
    pub vs30_ms: f32,
    pub particle_count_f32: f32,
    /// Bit 0 (`1.0`): seismic particles, Bit 1 (`2.0`): Venturi wind particles, Bit 2 (`4.0`): continuous preview.
    pub mode_flags_f32: f32,
    pub domain_radius_m: f32,
    pub obstacle_count_f32: f32,
    pub intensity_scale: f32,
    pub active_cx: f32,
    pub active_cy: f32,
    pub active_half_w: f32,
    pub active_half_d: f32,
}

/// 16-byte obstacle footprint descriptor `(center_east_m, center_north_m, radius_m, height_m)`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HazardObstacleGpu {
    pub xy_r_h: [f32; 4],
}

/// Frame inputs passed to `GpuHazardParticlePipeline::dispatch` (or `step_hazard_particles_cpu`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HazardComputeInput {
    pub enabled: bool,
    pub params: HazardComputeParams,
    pub obstacles: Vec<HazardObstacleGpu>,
}

/// Pure WGSL `@compute` shader writing `6` vertices (`2` triangles = oriented ribbon quad) per
/// particle into flat `array<f32>` (`14` floats per vertex = `56` byte `GpuEarthVertex` stride).
pub const HAZARD_COMPUTE_WGSL: &str = r#"
struct HazardParams {
    time_secs: f32,
    pga_g: f32,
    wave_speed_ms: f32,
    wave_azimuth_rad: f32,
    wind_speed_ms: f32,
    wind_azimuth_rad: f32,
    vs30_ms: f32,
    particle_count_f32: f32,
    mode_flags_f32: f32,
    domain_radius_m: f32,
    obstacle_count_f32: f32,
    intensity_scale: f32,
    active_cx: f32,
    active_cy: f32,
    active_half_w: f32,
    active_half_d: f32,
};

@group(0) @binding(0) var<uniform> params: HazardParams;
@group(0) @binding(1) var<storage, read> obstacles: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> out_verts: array<f32>;

fn hash_u32(idx: u32) -> f32 {
    var x: u32 = idx * 747796405u + 2891336453u;
    x = ((x >> ((x >> 28u) + 4u)) ^ x) * 277803737u;
    x = (x >> 22u) ^ x;
    return f32(x) / 4294967296.0;
}

fn write_vert(base: u32, pos: vec3<f32>, col: vec4<f32>, intensity: f32) {
    out_verts[base + 0u] = pos.x;
    out_verts[base + 1u] = pos.y;
    out_verts[base + 2u] = pos.z;
    out_verts[base + 3u] = 0.0;
    out_verts[base + 4u] = 0.0;
    out_verts[base + 5u] = 1.0;
    out_verts[base + 6u] = col.r;
    out_verts[base + 7u] = col.g;
    out_verts[base + 8u] = col.b;
    out_verts[base + 9u] = col.a;
    out_verts[base + 10u] = 6.0;
    out_verts[base + 11u] = intensity;
    out_verts[base + 12u] = 0.0;
    out_verts[base + 13u] = 1.0;
}

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let tid = gid.x;
    let total = u32(max(params.particle_count_f32, 0.0));
    if (tid >= total) {
        return;
    }

    let r0 = hash_u32(tid * 3u + 1u);
    let r1 = hash_u32(tid * 3u + 2u);
    let r2 = hash_u32(tid * 3u + 3u);
    let dom = max(params.domain_radius_m, 40.0);

    let flags = u32(params.mode_flags_f32 + 0.5);
    let has_seismic = (flags & 1u) != 0u;
    let has_wind = (flags & 2u) != 0u;
    let is_seismic = select(false, select(has_seismic, (tid & 1u) == 0u, has_wind), has_seismic);

    var p_center = vec3<f32>(0.0, 0.0, 1.5);
    var dir_xy = vec2<f32>(1.0, 0.0);
    var half_len: f32 = 3.5;
    var half_w: f32 = 0.75;
    var col = vec4<f32>(0.22, 0.78, 0.98, 0.78);
    var intensity: f32 = 0.5;

    if (is_seismic) {
        let dir = vec2<f32>(cos(params.wave_azimuth_rad), sin(params.wave_azimuth_rad));
        let ortho = vec2<f32>(-dir.y, dir.x);
        let speed = clamp(params.wave_speed_ms * 0.25, 18.0, 95.0);
        let s_pos = ((r0 * 2.0 - 1.0) * dom + params.time_secs * speed) % (2.0 * dom);
        let signed_s = select(s_pos, s_pos - 2.0 * dom, s_pos > dom);
        let wrapped_s = select(signed_s, signed_s + 2.0 * dom, signed_s < -dom);
        let lat_off = (r1 * 2.0 - 1.0) * dom * 0.85;
        let phase = wrapped_s * 0.095 - params.time_secs * 5.2;
        let amp = clamp(params.pga_g * (360.0 / max(params.vs30_ms, 140.0)), 0.2, 2.2) * sin(phase);
        let base_xy = vec2<f32>(params.active_cx, params.active_cy) + dir * wrapped_s + ortho * lat_off;
        p_center = vec3<f32>(base_xy.x, base_xy.y, 1.1 + max(amp, 0.0) * 1.4);
        dir_xy = ortho;
        half_len = 5.2 + 2.2 * abs(amp);
        half_w = 0.72;
        intensity = clamp(0.45 + 0.55 * abs(sin(phase)), 0.15, 1.0);
        col = vec4<f32>(1.0, 0.38 + 0.42 * (1.0 - intensity), 0.16, 0.42 + 0.36 * intensity);
    } else {
        let dir = vec2<f32>(cos(params.wind_azimuth_rad), sin(params.wind_azimuth_rad));
        let ortho = vec2<f32>(-dir.y, dir.x);
        let speed = clamp(params.wind_speed_ms * 0.9, 12.0, 68.0);
        let travel = ((r0 * 2.0 * dom + params.time_secs * speed) % (2.0 * dom)) - dom;
        let lane = (r1 * 2.0 - 1.0) * dom * 0.88;
        var base_xy = vec2<f32>(params.active_cx, params.active_cy) + dir * travel + ortho * lane;
        var speedup: f32 = 1.0;

        let n_obs = min(u32(params.obstacle_count_f32 + 0.5), 64u);
        for (var k: u32 = 0u; k < n_obs; k = k + 1u) {
            let obs = obstacles[k];
            let delta = base_xy - obs.xy;
            let dist = length(delta);
            let r_eff = max(obs.z, 6.0);
            if (dist < r_eff * 2.2 && dist > 0.01) {
                let push = (r_eff * 2.2 - dist) / (r_eff * 2.2);
                let perp = normalize(vec2<f32>(-delta.y * select(1.0, -1.0, dot(delta, ortho) < 0.0), delta.x * select(1.0, -1.0, dot(delta, ortho) < 0.0)));
                base_xy = base_xy + normalize(delta) * (push * r_eff * 0.85) + perp * (push * 1.4);
                speedup = max(speedup, 1.0 + 0.55 * push);
            }
        }
        let elev = 3.2 + r2 * 24.0;
        p_center = vec3<f32>(base_xy.x, base_xy.y, elev);
        dir_xy = dir;
        half_len = 4.0 * speedup;
        half_w = 0.45;
        intensity = clamp((speedup - 1.0) * 1.4 + 0.30, 0.20, 1.0);
        col = vec4<f32>(0.24 + 0.72 * (speedup - 1.0), 0.84 - 0.35 * (speedup - 1.0), 0.98 - 0.55 * (speedup - 1.0), 0.55 + 0.30 * intensity);
    }

    let side = vec2<f32>(-dir_xy.y, dir_xy.x) * half_w;
    let fwd = dir_xy * half_len;
    let v0 = p_center + vec3<f32>(-fwd.x - side.x, -fwd.y - side.y, 0.0);
    let v1 = p_center + vec3<f32>(fwd.x - side.x, fwd.y - side.y, 0.0);
    let v2 = p_center + vec3<f32>(fwd.x + side.x, fwd.y + side.y, 0.0);
    let v3 = p_center + vec3<f32>(-fwd.x + side.x, -fwd.y + side.y, 0.0);

    let fbase = tid * 84u;
    write_vert(fbase + 0u, v0, col, intensity);
    write_vert(fbase + 14u, v1, col, intensity);
    write_vert(fbase + 28u, v2, col, intensity);
    write_vert(fbase + 42u, v0, col, intensity);
    write_vert(fbase + 56u, v2, col, intensity);
    write_vert(fbase + 70u, v3, col, intensity);
}
"#;

/// GPU `@compute` pipeline + `STORAGE | VERTEX` particle buffer executed before Pass 2.
pub struct GpuHazardParticlePipeline {
    compute_pipeline: wgpu::ComputePipeline,
    params_buf: wgpu::Buffer,
    obstacles_buf: wgpu::Buffer,
    pub(crate) vertex_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pub max_particles: u32,
    pub active_vertex_count: u32,
}

impl GpuHazardParticlePipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("eigengraph_hazard_compute_wgsl"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(HAZARD_COMPUTE_WGSL)),
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("eigengraph_hazard_compute_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("eigengraph_hazard_compute_layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("eigengraph_hazard_compute_pipeline"),
            layout: Some(&layout),
            module: &module,
            entry_point: Some("cs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hazard_compute_params"),
            size: std::mem::size_of::<HazardComputeParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let obstacles_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hazard_compute_obstacles"),
            size: (MAX_GPU_HAZARD_OBSTACLES * std::mem::size_of::<HazardObstacleGpu>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let vert_bytes =
            (MAX_GPU_HAZARD_PARTICLES as u64) * 6 * (std::mem::size_of::<GpuEarthVertex>() as u64);
        let vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hazard_compute_vertices"),
            size: vert_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("eigengraph_hazard_compute_bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: obstacles_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: vertex_buf.as_entire_binding(),
                },
            ],
        });

        Self {
            compute_pipeline,
            params_buf,
            obstacles_buf,
            vertex_buf,
            bind_group,
            max_particles: MAX_GPU_HAZARD_PARTICLES,
            active_vertex_count: 0,
        }
    }

    pub fn dispatch(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        input: &HazardComputeInput,
    ) {
        if !input.enabled {
            self.active_vertex_count = 0;
            return;
        }
        let count = (input.params.particle_count_f32.round() as u32).clamp(64, self.max_particles);
        let mut params = input.params;
        params.particle_count_f32 = count as f32;
        let n_obs = input.obstacles.len().min(MAX_GPU_HAZARD_OBSTACLES);
        params.obstacle_count_f32 = n_obs as f32;

        queue.write_buffer(&self.params_buf, 0, struct_as_u8_slice(&params));
        if n_obs > 0 {
            let obs_bytes = unsafe {
                std::slice::from_raw_parts(
                    input.obstacles.as_ptr() as *const u8,
                    n_obs * std::mem::size_of::<HazardObstacleGpu>(),
                )
            };
            queue.write_buffer(&self.obstacles_buf, 0, obs_bytes);
        }

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("eigengraph_hazard_particle_compute_pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&self.compute_pipeline);
        cpass.set_bind_group(0, &self.bind_group, &[]);
        cpass.dispatch_workgroups(count.div_ceil(64), 1, 1);
        self.active_vertex_count = count * 6;
    }

    pub fn draw(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        if self.active_vertex_count == 0 {
            return;
        }
        render_pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        render_pass.draw(0..self.active_vertex_count, 0..1);
    }
}

#[inline]
fn hash_u32_cpu(idx: u32) -> f32 {
    let mut x = idx.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    x = ((x >> ((x >> 28).wrapping_add(4))) ^ x).wrapping_mul(277_803_737);
    x = (x >> 22) ^ x;
    (x as f32) / 4_294_967_296.0
}

/// Deterministic CPU mirror of `HAZARD_COMPUTE_WGSL` for CI / non-WebGPU vector canvas fallback.
pub fn step_hazard_particles_cpu(input: &HazardComputeInput) -> Vec<GpuEarthVertex> {
    if !input.enabled {
        return Vec::new();
    }
    let count = (input.params.particle_count_f32.round() as u32).clamp(1, MAX_GPU_HAZARD_PARTICLES);
    let mut out = Vec::with_capacity((count as usize) * 6);
    let p = &input.params;
    let dom = p.domain_radius_m.max(40.0);
    let flags = (p.mode_flags_f32 + 0.5) as u32;
    let has_seismic = (flags & 1) != 0;
    let has_wind = (flags & 2) != 0;

    for tid in 0..count {
        let r0 = hash_u32_cpu(tid * 3 + 1);
        let r1 = hash_u32_cpu(tid * 3 + 2);
        let r2 = hash_u32_cpu(tid * 3 + 3);
        let is_seismic = if has_seismic && has_wind {
            (tid & 1) == 0
        } else {
            has_seismic
        };

        let (p_center, dir_xy, half_len, half_w, col, intensity) = if is_seismic {
            let dir = [p.wave_azimuth_rad.cos(), p.wave_azimuth_rad.sin()];
            let ortho = [-dir[1], dir[0]];
            let speed = (p.wave_speed_ms * 0.25).clamp(18.0, 95.0);
            let mut s_pos = ((r0 * 2.0 - 1.0) * dom + p.time_secs * speed) % (2.0 * dom);
            if s_pos > dom {
                s_pos -= 2.0 * dom;
            }
            if s_pos < -dom {
                s_pos += 2.0 * dom;
            }
            let lat_off = (r1 * 2.0 - 1.0) * dom * 0.85;
            let phase = s_pos * 0.095 - p.time_secs * 5.2;
            let amp = (p.pga_g * (360.0 / p.vs30_ms.max(140.0))).clamp(0.2, 2.2) * phase.sin();
            let bx = p.active_cx + dir[0] * s_pos + ortho[0] * lat_off;
            let by = p.active_cy + dir[1] * s_pos + ortho[1] * lat_off;
            let inten = (0.45 + 0.55 * phase.sin().abs()).clamp(0.15, 1.0);
            let c = [1.0, 0.38 + 0.42 * (1.0 - inten), 0.16, 0.42 + 0.36 * inten];
            ([bx, by, 1.1 + amp.max(0.0) * 1.4], ortho, 5.2 + 2.2 * amp.abs(), 0.72, c, inten)
        } else {
            let dir = [p.wind_azimuth_rad.cos(), p.wind_azimuth_rad.sin()];
            let ortho = [-dir[1], dir[0]];
            let speed = (p.wind_speed_ms * 0.9).clamp(12.0, 68.0);
            let travel = ((r0 * 2.0 * dom + p.time_secs * speed) % (2.0 * dom)) - dom;
            let lane = (r1 * 2.0 - 1.0) * dom * 0.88;
            let mut bx = p.active_cx + dir[0] * travel + ortho[0] * lane;
            let mut by = p.active_cy + dir[1] * travel + ortho[1] * lane;
            let mut speedup = 1.0_f32;
            for obs in input.obstacles.iter().take(MAX_GPU_HAZARD_OBSTACLES) {
                let dx = bx - obs.xy_r_h[0];
                let dy = by - obs.xy_r_h[1];
                let dist = dx.hypot(dy);
                let r_eff = obs.xy_r_h[2].max(6.0);
                if dist < r_eff * 2.2 && dist > 0.01 {
                    let push = (r_eff * 2.2 - dist) / (r_eff * 2.2);
                    let nx = dx / dist;
                    let ny = dy / dist;
                    bx += nx * (push * r_eff * 0.85);
                    by += ny * (push * r_eff * 0.85);
                    speedup = speedup.max(1.0 + 0.55 * push);
                }
            }
            let inten = ((speedup - 1.0) * 1.4 + 0.30).clamp(0.20, 1.0);
            let c = [
                0.24 + 0.72 * (speedup - 1.0),
                0.84 - 0.35 * (speedup - 1.0),
                0.98 - 0.55 * (speedup - 1.0),
                0.55 + 0.30 * inten,
            ];
            ([bx, by, 3.2 + r2 * 24.0], dir, 4.0 * speedup, 0.45, c, inten)
        };

        let side = [-dir_xy[1] * half_w, dir_xy[0] * half_w];
        let fwd = [dir_xy[0] * half_len, dir_xy[1] * half_len];
        let v0 = [p_center[0] - fwd[0] - side[0], p_center[1] - fwd[1] - side[1], p_center[2]];
        let v1 = [p_center[0] + fwd[0] - side[0], p_center[1] + fwd[1] - side[1], p_center[2]];
        let v2 = [p_center[0] + fwd[0] + side[0], p_center[1] + fwd[1] + side[1], p_center[2]];
        let v3 = [p_center[0] - fwd[0] + side[0], p_center[1] - fwd[1] + side[1], p_center[2]];
        for pos in [v0, v1, v2, v0, v2, v3] {
            out.push(GpuEarthVertex {
                pos,
                normal: [0.0, 0.0, 1.0],
                color: col,
                meta: [6.0, intensity, 0.0, 1.0],
            });
        }
    }
    out
}
