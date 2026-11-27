//! Pure-vector 3D mesh builders for Planetary Earth, IFCX City-Builder, Active BIM/FEM, and District Multi-Hazard GPU passes.

use egui::Color32;

use crate::core::bim::{BimElementKind, BimProject};
use crate::core::geotech::{
    GeotechEngine, SubsurfaceSsiAnalysisReport, SubsurfaceSsiConfig,
};
use crate::core::gis::city_builder::IfcxCityScene;
use crate::core::gis::district_hazard::{
    evaluate_seismic_wave_displacement_m, DistrictHazardConfig, DistrictMultiHazardReport,
};
use crate::core::gis::map_widget::{SlippyMapState, VectorEarthTheme};
use crate::core::gis::neighborhood_physics::{NeighborhoodAuditReport, PoundingSeverity};
use crate::core::gis::tin_surface::TinSurfaceMesh;
use crate::core::gis::vector_layers::{UrbanVectorKind, UrbanVectorLayerCache};
use crate::core::gis::world_index::WorldSpatialIndex;
use crate::core::ifcx::{IfcxDocument, IfcxStage};
use crate::models::StructuralMember;
use crate::views::universal_canvas::EarthBimLens;

use super::{GpuEarthVertex, GpuMeshPayload};

#[inline]
pub(super) fn color_to_rgba(c: Color32) -> [f32; 4] {
    [
        c.r() as f32 / 255.0,
        c.g() as f32 / 255.0,
        c.b() as f32 / 255.0,
        c.a() as f32 / 255.0,
    ]
}

#[inline]
pub(super) fn mix_rgb(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let k = t.clamp(0.0, 1.0);
    [
        a[0] * (1.0 - k) + b[0] * k,
        a[1] * (1.0 - k) + b[1] * k,
        a[2] * (1.0 - k) + b[2] * k,
        1.0,
    ]
}

/// Simple 2D ear-clipping triangulator returning triangle index triplets into `pts`.
pub(super) fn ear_clip_indices_2d(pts: &[[f64; 2]]) -> Vec<[usize; 3]> {
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
pub(super) fn push_extruded_building_prism_enu(
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
pub(super) fn push_stylized_tree_enu(
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
pub(super) fn push_box_prism_enu(
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
pub(super) fn push_inclined_3d_strut_enu(
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

/// Pushes a mitered sloped quad segment strip between stations `a` and `b` at signed lateral
/// offset `[off_l, off_r]` and bottom/top elevations `z0..z1` with proper outward normals.
#[allow(clippy::too_many_arguments)]
fn push_sloped_ribbon_strip_enu(
    verts: &mut Vec<GpuEarthVertex>,
    idxs: &mut Vec<u32>,
    a: &crate::core::gis::RoadProfileSample,
    b: &crate::core::gis::RoadProfileSample,
    off_l: f64,
    off_r: f64,
    za_bot: f32,
    za_top: f32,
    zb_bot: f32,
    zb_top: f32,
    col: [f32; 4],
    kind: f32,
) {
    let pt = |s: &crate::core::gis::RoadProfileSample, off: f64, z: f32| -> [f32; 3] {
        [
            (s.east_m + s.normal[0] * off) as f32,
            (s.north_m + s.normal[1] * off) as f32,
            z,
        ]
    };
    let al_t = pt(a, off_l, za_top);
    let ar_t = pt(a, off_r, za_top);
    let br_t = pt(b, off_r, zb_top);
    let bl_t = pt(b, off_l, zb_top);
    let al_b = pt(a, off_l, za_bot);
    let ar_b = pt(a, off_r, za_bot);
    let br_b = pt(b, off_r, zb_bot);
    let bl_b = pt(b, off_l, zb_bot);

    // Top deck face
    let tb = verts.len() as u32;
    for pos in [al_t, ar_t, br_t, bl_t] {
        verts.push(GpuEarthVertex {
            pos,
            normal: [0.0, 0.0, 1.0],
            color: col,
            meta: [kind, pos[2].max(0.0), 1.0, 0.0],
        });
    }
    idxs.extend_from_slice(&[tb, tb + 1, tb + 2, tb, tb + 2, tb + 3]);

    let side_col = mix_rgb(col, [0.12, 0.14, 0.18, 1.0], 0.28);
    // Right side skirt
    let rb = verts.len() as u32;
    let rn = [-a.normal[0] as f32, -a.normal[1] as f32, 0.0];
    for pos in [ar_b, br_b, br_t, ar_t] {
        verts.push(GpuEarthVertex {
            pos,
            normal: rn,
            color: side_col,
            meta: [kind, pos[2].max(0.0), 0.85, 0.0],
        });
    }
    idxs.extend_from_slice(&[rb, rb + 1, rb + 2, rb, rb + 2, rb + 3]);

    // Left side skirt
    let lb = verts.len() as u32;
    let ln = [a.normal[0] as f32, a.normal[1] as f32, 0.0];
    for pos in [bl_b, al_b, al_t, bl_t] {
        verts.push(GpuEarthVertex {
            pos,
            normal: ln,
            color: side_col,
            meta: [kind, pos[2].max(0.0), 0.85, 0.0],
        });
    }
    idxs.extend_from_slice(&[lb, lb + 1, lb + 2, lb, lb + 2, lb + 3]);
}

/// Extrudes a terrain-coupled straight or curved Bézier `IfcRoad` / `IfcBridge` with piers,
/// bridge parapets, centerline striping, and signature coastal cable-stayed pylons.
#[allow(clippy::too_many_arguments)]
fn push_curved_road_or_bridge_enu(
    verts: &mut Vec<GpuEarthVertex>,
    idxs: &mut Vec<u32>,
    road: &crate::core::gis::IfcxRoadSegment,
    tin: Option<&TinSurfaceMesh>,
    sea_level_m: f32,
    asphalt_col: [f32; 4],
    curb_col: [f32; 4],
    stripe_col: [f32; 4],
) {
    let profile = road.sample_3d_profile(tin, sea_level_m);
    if profile.len() < 2 {
        return;
    }
    let hw = road.width_m * 0.5;
    let sw_hw = hw + if road.is_bridge { 0.85 } else { 1.35 };
    let has_water_span = profile.iter().any(|s| s.over_water);

    for (seg_idx, win) in profile.windows(2).enumerate() {
        let a = &win[0];
        let b = &win[1];
        if road.is_bridge || a.is_elevated || b.is_elevated {
            let za_bot = a.deck_z_rel_m - 0.85;
            let zb_bot = b.deck_z_rel_m - 0.85;
            // Box girder deck slab
            push_sloped_ribbon_strip_enu(
                verts,
                idxs,
                a,
                b,
                -sw_hw,
                sw_hw,
                za_bot,
                a.deck_z_rel_m,
                zb_bot,
                b.deck_z_rel_m,
                curb_col,
                3.0,
            );
            // Asphalt carriageway between parapets
            let inner = (hw - 0.35).max(1.2);
            push_sloped_ribbon_strip_enu(
                verts,
                idxs,
                a,
                b,
                -inner,
                inner,
                a.deck_z_rel_m,
                a.deck_z_rel_m + 0.08,
                b.deck_z_rel_m,
                b.deck_z_rel_m + 0.08,
                asphalt_col,
                3.0,
            );
            // Guardrail parapets
            for side in [-1.0_f64, 1.0] {
                let p_in = side * (sw_hw - 0.45);
                let p_out = side * sw_hw;
                let (ol, or) = if p_in < p_out { (p_in, p_out) } else { (p_out, p_in) };
                push_sloped_ribbon_strip_enu(
                    verts,
                    idxs,
                    a,
                    b,
                    ol,
                    or,
                    a.deck_z_rel_m,
                    a.deck_z_rel_m + 0.65,
                    b.deck_z_rel_m,
                    b.deck_z_rel_m + 0.65,
                    mix_rgb(curb_col, [0.95, 0.96, 0.98, 1.0], 0.35),
                    3.0,
                );
            }
        } else {
            // Terrain-hugging road sidewalk apron + asphalt carriageway
            push_sloped_ribbon_strip_enu(
                verts,
                idxs,
                a,
                b,
                -sw_hw,
                sw_hw,
                a.deck_z_rel_m - 0.25,
                a.deck_z_rel_m - 0.04,
                b.deck_z_rel_m - 0.25,
                b.deck_z_rel_m - 0.04,
                curb_col,
                3.0,
            );
            push_sloped_ribbon_strip_enu(
                verts,
                idxs,
                a,
                b,
                -hw,
                hw,
                a.deck_z_rel_m - 0.10,
                a.deck_z_rel_m + 0.05,
                b.deck_z_rel_m - 0.10,
                b.deck_z_rel_m + 0.05,
                asphalt_col,
                3.0,
            );
        }
        // Dashed lane markings along alternating stations
        if seg_idx % 2 == 0 {
            push_sloped_ribbon_strip_enu(
                verts,
                idxs,
                a,
                b,
                -0.16,
                0.16,
                a.deck_z_rel_m + 0.05,
                a.deck_z_rel_m + 0.11,
                b.deck_z_rel_m + 0.05,
                b.deck_z_rel_m + 0.11,
                stripe_col,
                3.0,
            );
        }
    }

    // Twin substructure piers beneath elevated spans & water bridges
    for (idx, s) in profile.iter().enumerate() {
        let clear_m = s.deck_z_rel_m - s.ground_z_rel_m;
        let on_pier_step = idx > 0 && idx + 1 < profile.len() && idx % 3 == 0;
        if on_pier_step && (road.is_bridge || clear_m > 1.6) {
            let p_bot = (s.ground_z_rel_m - 2.2).min(s.deck_z_rel_m - 1.5);
            let p_top = s.deck_z_rel_m - 0.80;
            let pr = 0.72_f64;
            for side in [-0.34_f64, 0.34] {
                let cx = s.east_m + s.normal[0] * road.width_m * side;
                let cy = s.north_m + s.normal[1] * road.width_m * side;
                let pfp = [
                    [cx - pr, cy - pr],
                    [cx + pr, cy - pr],
                    [cx + pr, cy + pr],
                    [cx - pr, cy + pr],
                ];
                push_box_prism_enu(verts, idxs, &pfp, p_bot, p_top, [0.58, 0.63, 0.68, 1.0], 3.0);
            }
        }
    }

    // Signature cable-stayed pylons & diagonal cables on coastal water bridges >= 65m
    if road.is_bridge && road.length_m() >= 65.0 && has_water_span && profile.len() >= 9 {
        let n_st = profile.len();
        let tower_indices = [n_st / 3, (2 * n_st) / 3];
        let tower_col = [0.84_f32, 0.88, 0.92, 1.0];
        let cable_col = [0.92_f32, 0.95, 0.98, 1.0];
        let pylon_h = 18.0_f32;
        for ti in tower_indices {
            let s = &profile[ti];
            let p_top = s.deck_z_rel_m + pylon_h;
            let mut heads = Vec::with_capacity(2);
            for side in [-1.0_f64, 1.0] {
                let cx = s.east_m + s.normal[0] * (sw_hw + 0.35) * side;
                let cy = s.north_m + s.normal[1] * (sw_hw + 0.35) * side;
                heads.push([cx, cy, (p_top - 1.5) as f64]);
                let pr = 0.85_f64;
                let pfp = [
                    [cx - pr, cy - pr],
                    [cx + pr, cy - pr],
                    [cx + pr, cy + pr],
                    [cx - pr, cy + pr],
                ];
                push_box_prism_enu(verts, idxs, &pfp, s.deck_z_rel_m - 0.5, p_top, tower_col, 3.0);

                // Anchor 4 pairs of stay cables fore and aft along the bridge deck
                for dk in [-3_isize, -2, -1, 1, 2, 3] {
                    let anchor_idx = (ti as isize + dk).clamp(0, (n_st - 1) as isize) as usize;
                    let anc = &profile[anchor_idx];
                    let ax = anc.east_m + anc.normal[0] * (sw_hw - 0.25) * side;
                    let ay = anc.north_m + anc.normal[1] * (sw_hw - 0.25) * side;
                    let az = (anc.deck_z_rel_m + 0.45) as f64;
                    let hz = (s.deck_z_rel_m + pylon_h - 1.2 - (dk.unsigned_abs() as f32 * 1.4)) as f64;
                    push_inclined_3d_strut_enu(
                        verts,
                        idxs,
                        [cx, cy, hz],
                        [ax, ay, az],
                        0.16,
                        0.16,
                        cable_col,
                    );
                }
            }
            if heads.len() == 2 {
                push_inclined_3d_strut_enu(verts, idxs, heads[0], heads[1], 1.1, 1.2, tower_col);
            }
        }
    }
}

/// Appends authored IFCX City-Builder objects (`IfcRoad`, `IfcBridge`, `IfcSpatialZone`,
/// context towers, instanced stylized trees, and `IfcGradedPad`) into the RTC site mesh.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_city_builder_scene_enu(
    verts: &mut Vec<GpuEarthVertex>,
    idxs: &mut Vec<u32>,
    theme: VectorEarthTheme,
    city: &IfcxCityScene,
    active_site_tin: Option<&TinSurfaceMesh>,
    sea_level_m: f32,
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
    let ref_z = active_site_tin
        .map(|t| t.sample_elevation_and_slope(0.0, 0.0).0)
        .unwrap_or(0.0);

    // 1. Authored Terrain-Coupled Straight / Curved Bézier `IfcRoad` & Auto-Bridge `IfcBridge`
    for road in &city.roads {
        push_curved_road_or_bridge_enu(
            verts,
            idxs,
            road,
            active_site_tin,
            sea_level_m,
            asphalt_col,
            curb_col,
            stripe_col,
        );
    }

    // 2. Authored `IfcSpatialZone` parcels, optional context towers & instanced stylized trees
    for zone in &city.zones {
        let ground_z = active_site_tin
            .map(|t| {
                t.sample_elevation_and_slope(zone.center_enu_m[0] as f32, zone.center_enu_m[1] as f32)
                    .0
                    - ref_z
            })
            .unwrap_or(0.0);
        let fp = zone.footprint_enu_m();
        let zcol = mix_rgb(zone.kind.fill_rgba(), [0.9, 0.94, 0.96, 1.0], 0.12);
        push_box_prism_enu(verts, idxs, &fp, ground_z, ground_z + 0.22, zcol, 1.0);

        if let Some(bldg) = zone.synthesize_procedural_building() {
            let rad = bldg.rotation_deg.to_radians();
            let (sr, cr) = (rad.sin(), rad.cos());
            let rot_rect = |w: f64, d: f64| -> Vec<[f64; 2]> {
                let hw = w * 0.5;
                let hd = d * 0.5;
                [[-hw, -hd], [hw, -hd], [hw, hd], [-hw, hd]]
                    .iter()
                    .map(|[lx, ly]| {
                        [
                            bldg.center_enu_m[0] + lx * cr - ly * sr,
                            bldg.center_enu_m[1] + lx * sr + ly * cr,
                        ]
                    })
                    .collect()
            };

            let podium_fp = rot_rect(bldg.podium_width_m, bldg.podium_depth_m);
            let pwall = mix_rgb(wall_base, bldg.wall_tint_rgba, 0.26);
            let proof = mix_rgb(roof_base, bldg.wall_tint_rgba, 0.16);
            let base_z = ground_z + 0.22;
            push_extruded_building_prism_enu(
                verts,
                idxs,
                &podium_fp,
                base_z,
                bldg.podium_height_m,
                pwall,
                proof,
            );

            let shaft_h = bldg.tower_height_m - bldg.podium_height_m;
            if shaft_h > 1.5 {
                let tower_fp = rot_rect(bldg.tower_width_m, bldg.tower_depth_m);
                let twall = mix_rgb(wall_base, bldg.wall_tint_rgba, 0.38);
                let troof = mix_rgb(roof_base, bldg.wall_tint_rgba, 0.20);
                push_extruded_building_prism_enu(
                    verts,
                    idxs,
                    &tower_fp,
                    base_z + bldg.podium_height_m,
                    shaft_h,
                    twall,
                    troof,
                );
            }

            if bldg.has_penthouse {
                let pent_fp = rot_rect(bldg.tower_width_m * 0.44, bldg.tower_depth_m * 0.44);
                let pent_wall = mix_rgb(proof, [0.32, 0.36, 0.42, 1.0], 0.35);
                push_extruded_building_prism_enu(
                    verts,
                    idxs,
                    &pent_fp,
                    base_z + bldg.tower_height_m,
                    3.0,
                    pent_wall,
                    proof,
                );
            }
        }

        for [tx, ty, th] in zone.instanced_trees_enu_m() {
            let tz = active_site_tin
                .map(|t| t.sample_elevation_and_slope(tx as f32, ty as f32).0 - ref_z)
                .unwrap_or(0.0);
            push_stylized_tree_enu(verts, idxs, tx as f32, ty as f32, tz + 0.18, th as f32, theme);
        }
    }

    // 2b. Procedural Biome Hillside & Coastal Forests (outside zoned lots & road clearances)
    if let Some(tin) = active_site_tin {
        for tr in city.synthesize_biome_forest_trees(tin, sea_level_m) {
            push_stylized_tree_enu(
                verts,
                idxs,
                tr.east_m,
                tr.north_m,
                tr.base_z_rel_m,
                tr.height_m,
                theme,
            );
        }
    }

    // 3. Authored `IfcGradedPad` foundation aprons
    for pad in &city.graded_pads {
        let pad_z = if active_site_tin.is_some() {
            (pad.target_elev_m as f32) - ref_z
        } else {
            0.0
        };
        let hw = pad.width_m * 0.5;
        let hd = pad.depth_m * 0.5;
        let pfp = [
            [pad.center_enu_m[0] - hw, pad.center_enu_m[1] - hd],
            [pad.center_enu_m[0] + hw, pad.center_enu_m[1] - hd],
            [pad.center_enu_m[0] + hw, pad.center_enu_m[1] + hd],
            [pad.center_enu_m[0] - hw, pad.center_enu_m[1] + hd],
        ];
        push_box_prism_enu(
            verts,
            idxs,
            &pfp,
            pad_z - 0.12,
            pad_z + 0.08,
            [0.58, 0.56, 0.50, 1.0],
            1.0,
        );
    }
}

/// Dynamic active 3D BIM + 6-DOF FEM + Modal + N-1 + Tri-Directional Ground-Motion Replay input for unified WebGPU Pass 2.
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
    pub subsurface_pile_depth_m: f32,
    pub heading_deg: f64,
    pub diff_overlay_active: bool,
    pub diff_added_bim: &'a [(BimElementKind, usize)],
    pub diff_modified_bim: &'a [(BimElementKind, usize)],
    pub diff_deleted_ghosts: &'a [([f64; 3], [f64; 3], String)],
    pub replay_story_elevs_m: &'a [f64],
    pub replay_story_disp_xyz_m: &'a [[f64; 3]],
    pub replay_ground_shake_xyz_m: [f64; 3],
    pub replay_hinge_states: &'a [(BimElementKind, usize, crate::core::nonlinear::PlasticHingeState)],
    pub shm_story_retention: &'a [(f64, f64)],
}

pub(super) fn structural_dcr_color(dcr: f64) -> [f32; 4] {
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

/// Builds the unified 3D Pure WebGPU mesh for the active editable `BimProject` inside the shared
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

    let sample_replay_sway_xyz = |z_m: f64| -> [f64; 3] {
        if input.replay_story_elevs_m.is_empty() || input.replay_story_disp_xyz_m.is_empty() {
            return [0.0, 0.0, 0.0];
        }
        let vis = (input.deform_scale * 0.25).clamp(1.0, 28.0);
        let shake_x = input.replay_ground_shake_xyz_m[0] * vis * 0.35;
        let shake_y = input.replay_ground_shake_xyz_m[1] * vis * 0.35;
        let shake_z = input.replay_ground_shake_xyz_m[2] * vis * 0.35;
        if z_m < 0.0 {
            let damp = (1.0 + z_m / 6.0).clamp(0.0, 1.0);
            return [
                (shake_x * damp).clamp(-2.8, 2.8),
                (shake_y * damp).clamp(-2.8, 2.8),
                (shake_z * damp).clamp(-0.65, 0.65),
            ];
        }
        let n = input
            .replay_story_elevs_m
            .len()
            .min(input.replay_story_disp_xyz_m.len());
        let mut dx = input.replay_story_disp_xyz_m[n - 1][0];
        let mut dy = input.replay_story_disp_xyz_m[n - 1][1];
        let mut dz = input.replay_story_disp_xyz_m[n - 1][2];
        if z_m <= input.replay_story_elevs_m[0] {
            dx = input.replay_story_disp_xyz_m[0][0];
            dy = input.replay_story_disp_xyz_m[0][1];
            dz = input.replay_story_disp_xyz_m[0][2];
        } else {
            for k in 1..n {
                let e0 = input.replay_story_elevs_m[k - 1];
                let e1 = input.replay_story_elevs_m[k];
                if z_m <= e1 {
                    let t = ((z_m - e0) / (e1 - e0).max(1e-4)).clamp(0.0, 1.0);
                    let d0 = input.replay_story_disp_xyz_m[k - 1];
                    let d1 = input.replay_story_disp_xyz_m[k];
                    dx = d0[0] + (d1[0] - d0[0]) * t;
                    dy = d0[1] + (d1[1] - d0[1]) * t;
                    dz = d0[2] + (d1[2] - d0[2]) * t;
                    break;
                }
            }
        }
        [
            (dx * vis + shake_x).clamp(-2.8, 2.8),
            (dy * vis + shake_y).clamp(-2.8, 2.8),
            (dz * vis + shake_z).clamp(-0.85, 0.85),
        ]
    };

    // Live structural displacement field for DeformedShape / ModalVibration / TimeHistoryReplay (meters in ENU)
    let anim_amp_m = match input.lens {
        EarthBimLens::DeformedShape => (input.deform_scale * 0.012).clamp(0.05, 1.8),
        EarthBimLens::ModalVibration => {
            (input.deform_scale * 0.016).clamp(0.08, 2.2) * (input.modal_phase_sin as f64)
        }
        _ => 0.0,
    };
    let mode_idx = input.active_modal_mode % 3;
    let deform_offset = |x_m: f64, y_m: f64, z_m: f64| -> [f64; 3] {
        if input.lens == EarthBimLens::TimeHistoryReplay && !input.replay_story_disp_xyz_m.is_empty()
        {
            return sample_replay_sway_xyz(z_m);
        }
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
    let avg_eigengate = if !input.members.is_empty() {
        input.members.iter().map(|m| m.eigengate).fold(1.0_f64, f64::min)
    } else {
        0.92
    };

    let elem_color = |kind: BimElementKind, bim_id: usize, z_mid: f64, default_col: [f32; 4]| -> [f32; 4] {
        if input.cascade_failed_bim.contains(&(kind, bim_id)) {
            return [0.98, 0.22, 0.18, 0.36]; // Translucent crimson-orange progressive collapse ghost
        }
        if input.n1_removed_bim == Some((kind, bim_id)) {
            return [0.96, 0.16, 0.24, 0.42]; // Crimson translucent N-1 removed ghost
        }
        if input.diff_overlay_active {
            if input.diff_added_bim.contains(&(kind, bim_id)) {
                return [0.16, 0.90, 0.54, 0.96];
            }
            if input.diff_modified_bim.contains(&(kind, bim_id)) {
                return [0.98, 0.72, 0.16, 0.96];
            }
            return [0.42, 0.50, 0.58, 0.36];
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
            EarthBimLens::EigenGateRedundancy => {
                let eigengate = member_opt.map(|m| m.eigengate).unwrap_or(avg_eigengate);
                structural_dcr_color(1.0 - eigengate)
            }
            EarthBimLens::MomentStress => {
                let mom = member_opt
                    .map(|m| (m.moment_my_knm.hypot(m.moment_mz_knm) / 250.0).clamp(0.0, 1.2))
                    .unwrap_or(0.45);
                structural_dcr_color(mom)
            }
            EarthBimLens::TimeHistoryReplay => {
                use crate::core::nonlinear::PlasticHingeState;
                if let Some((_, _, st)) = input
                    .replay_hinge_states
                    .iter()
                    .find(|&&(k, id, _)| k == kind && id == bim_id)
                {
                    match st {
                        PlasticHingeState::Elastic => [0.22, 0.68, 0.78, 0.92],
                        PlasticHingeState::ImmediateOccupancy => [0.04, 0.52, 1.00, 1.0],
                        PlasticHingeState::LifeSafety => [1.00, 0.62, 0.04, 1.0],
                        PlasticHingeState::CollapsePrevention => [1.00, 0.27, 0.23, 1.0],
                        PlasticHingeState::Mechanism => [0.75, 0.35, 0.95, 1.0],
                    }
                } else {
                    [0.24, 0.64, 0.74, 0.90]
                }
            }
            EarthBimLens::ShmDigitalTwin => {
                if input.shm_story_retention.is_empty() {
                    [0.16, 0.84, 0.56, 1.0]
                } else {
                    let retention = input
                        .shm_story_retention
                        .iter()
                        .find(|&&(elev, _)| z_mid <= elev + 0.35)
                        .or_else(|| input.shm_story_retention.last())
                        .map(|&(_, ret)| ret)
                        .unwrap_or(1.0);
                    if retention < 0.72 {
                        [0.96, 0.22, 0.22, 1.0] // Severe stiffness loss (<72% retained)
                    } else if retention < 0.88 {
                        [0.98, 0.68, 0.14, 1.0] // Moderate stiffness loss (72%..88% retained)
                    } else {
                        [0.16, 0.84, 0.56, 1.0] // Nominal intact stiffness (>=88% retained)
                    }
                }
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
        let col = elem_color(
            BimElementKind::Footing,
            ftg.id,
            0.5 * (z_bot + z_top) as f64,
            [0.48, 0.54, 0.60, 1.0],
        );
        push_box_prism_enu(&mut verts, &mut idxs, &fp, z_bot, z_top, col, 3.0);

        // 4 subsurface concrete pile shafts seated dynamically into the bearing stratum
        let pile_depth = input.subsurface_pile_depth_m.clamp(3.0, 28.0);
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
                z_bot - pile_depth,
                z_bot,
                [0.36, 0.42, 0.48, 1.0],
                3.0,
            );
            if pile_depth > 4.5 {
                let cpr = pr * 1.18;
                let tip_fp = [
                    rot_xy(px - cpr, py - cpr, 0.0),
                    rot_xy(px + cpr, py - cpr, 0.0),
                    rot_xy(px + cpr, py + cpr, 0.0),
                    rot_xy(px - cpr, py + cpr, 0.0),
                ];
                push_box_prism_enu(
                    &mut verts,
                    &mut idxs,
                    &tip_fp,
                    z_bot - pile_depth,
                    z_bot - pile_depth + 0.90,
                    [0.24, 0.74, 0.62, 1.0],
                    3.0,
                );
            }
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
        let dz_b = deform_offset(cx, cy, z0)[2];
        let dz_t = deform_offset(cx, cy, z1)[2];
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
        let col = elem_color(BimElementKind::Column, c.id, z_mid, default_col);
        push_box_prism_enu(
            &mut verts,
            &mut idxs,
            &fp,
            (z0 + dz_b) as f32,
            (z1 + dz_t).max(z0 + dz_b + 0.35) as f32,
            col,
            3.0,
        );
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
        let run_z_mid = level_elev_m(r.level_id) + 0.5 * (r.start_offset_mm + r.end_offset_mm) * 0.001;
        let col = elem_color(BimElementKind::Run, r.id, run_z_mid, default_col);
        let ax = r.start.0 * 0.001;
        let ay = r.start.1 * 0.001;
        let bx = r.end.0 * 0.001;
        let by = r.end.1 * 0.001;
        if is_inclined_brace {
            let z0_raw = level_elev_m(r.level_id) + r.start_offset_mm * 0.001 - sag * 0.5;
            let z1_raw = level_elev_m(r.level_id) + r.end_offset_mm * 0.001 - sag;
            let z0 = z0_raw + deform_offset(ax, ay, z0_raw)[2];
            let z1 = z1_raw + deform_offset(bx, by, z1_raw)[2];
            let p0_xy = rot_xy(ax, ay, z0_raw);
            let p1_xy = rot_xy(bx, by, z1_raw);
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
        let z_top_raw = (level_elev_m(r.level_id) + r.start_offset_mm * 0.001 - sag).max(0.15);
        let dz_r = deform_offset(0.5 * (ax + bx), 0.5 * (ay + by), z_top_raw)[2];
        let z_top = z_top_raw + dz_r;
        let z_bot = z_top - (r.depth_mm * 0.001).max(0.2);
        let dx = bx - ax;
        let dy = by - ay;
        let len = dx.hypot(dy).max(0.05);
        let hw = (r.width_mm * 0.0005).max(0.10);
        let nx = -dy / len * hw;
        let ny = dx / len * hw;
        let fp = [
            rot_xy(ax - nx, ay - ny, z_top_raw),
            rot_xy(bx - nx, by - ny, z_top_raw),
            rot_xy(bx + nx, by + ny, z_top_raw),
            rot_xy(ax + nx, ay + ny, z_top_raw),
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
        let dz_wb = deform_offset(ax, ay, z0)[2];
        let dz_wt = deform_offset(bx, by, z1)[2];
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
        let col = elem_color(BimElementKind::Wall, w.id, z_mid, default_w);
        push_box_prism_enu(
            &mut verts,
            &mut idxs,
            &fp,
            (z0 + dz_wb) as f32,
            (z1 + dz_wt).max(z0 + dz_wb + 0.40) as f32,
            col,
            2.0,
        );
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
        let dz_slab = deform_offset(0.0, 0.0, z_lvl)[2] as f32;
        let base_z = if s.is_roof {
            z_lvl as f32 + dz_slab
        } else {
            z_lvl as f32 - th + dz_slab
        };
        let fp: Vec<[f64; 2]> = s
            .boundary
            .iter()
            .map(|p| rot_xy(p.0 * 0.001, p.1 * 0.001, z_lvl))
            .collect();
        let col = elem_color(
            BimElementKind::Slab,
            s.id,
            z_lvl,
            if s.is_roof {
                [0.54, 0.68, 0.76, 0.88]
            } else {
                [0.68, 0.76, 0.82, 0.82]
            },
        );
        push_extruded_building_prism_enu(&mut verts, &mut idxs, &fp, base_z, th, col, col);
    }

    // 6. Deleted IFCX Diff AABB Ghost Volumes (Translucent Crimson)
    if input.diff_overlay_active {
        for (bmin, bmax, _) in input.diff_deleted_ghosts {
            let fp = [
                rot_xy(bmin[0], bmin[1], bmax[2]),
                rot_xy(bmax[0], bmin[1], bmax[2]),
                rot_xy(bmax[0], bmax[1], bmax[2]),
                rot_xy(bmin[0], bmax[1], bmax[2]),
            ];
            let z0 = bmin[2] as f32;
            let z1 = bmax[2].max(bmin[2] + 0.15) as f32;
            push_box_prism_enu(
                &mut verts,
                &mut idxs,
                &fp,
                z0,
                z1,
                [0.96, 0.22, 0.28, 0.34],
                3.0,
            );
        }
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// 3D Subsurface Geotechnical Stratigraphy & Groundwater Table X-Ray input for WebGPU Pass 2.
pub struct SubsurfaceSsiGpuInput<'a> {
    pub project: &'a BimProject,
    pub config: &'a SubsurfaceSsiConfig,
    pub report: Option<&'a SubsurfaceSsiAnalysisReport>,
    pub heading_deg: f64,
}

/// Builds translucent 3D geological soil horizons (`material_kind = 4.0`), pulsing amber-crimson
/// liquefaction-critical zones, cyan groundwater phreatic surface (`material_kind = 5.0`), and
/// borehole log casing beneath the active site footprint.
pub fn build_rtc_subsurface_ssi_mesh(input: &SubsurfaceSsiGpuInput<'_>) -> GpuMeshPayload {
    if !input.config.xray_subsurface_visible {
        return GpuMeshPayload::default();
    }
    let strata = input
        .report
        .map(|r| r.strata.clone())
        .unwrap_or_else(|| {
            GeotechEngine::build_borehole_strata(
                input.config.borehole_preset,
                input.config.groundwater_depth_m,
                0.35,
                input.config.liquefaction_trigger_active,
            )
            .0
        });

    let (min_x, max_x, min_y, max_y) = if let Some((bmin, bmax)) = input.project.plan_extents_mm() {
        (
            bmin[0] * 0.001 - 6.0,
            bmax[0] * 0.001 + 6.0,
            bmin[1] * 0.001 - 6.0,
            bmax[1] * 0.001 + 6.0,
        )
    } else {
        (-12.0, 12.0, -12.0, 12.0)
    };

    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_xy = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };
    let slab_fp = [
        rot_xy(min_x, min_y),
        rot_xy(max_x, min_y),
        rot_xy(max_x, max_y),
        rot_xy(min_x, max_y),
    ];

    let mut verts = Vec::with_capacity(512);
    let mut idxs = Vec::with_capacity(768);

    // 1. Subsurface Geological Strata (`material_kind = 4.0`)
    for s in &strata {
        let col = if input.config.liquefaction_trigger_active && s.liquefiable && s.liquefaction_fs < 1.15 {
            [0.95, 0.34, 0.16, 0.44]
        } else {
            s.color_rgba
        };
        push_box_prism_enu(
            &mut verts,
            &mut idxs,
            &slab_fp,
            -(s.depth_bottom_m as f32),
            -(s.depth_top_m as f32),
            col,
            4.0,
        );
    }

    // 2. Groundwater Phreatic Table (`material_kind = 5.0`)
    let gwt_z = -(input.config.groundwater_depth_m.clamp(0.0, 28.0) as f32);
    push_box_prism_enu(
        &mut verts,
        &mut idxs,
        &slab_fp,
        gwt_z - 0.12,
        gwt_z,
        [0.16, 0.76, 0.96, 0.42],
        5.0,
    );

    // 3. Borehole BH-01 Casing & Horizon Collar Markers
    let bx = max_x - 1.5;
    let by = max_y - 1.5;
    let r = 0.22;
    let bh_fp = [
        rot_xy(bx - r, by - r),
        rot_xy(bx + r, by - r),
        rot_xy(bx + r, by + r),
        rot_xy(bx - r, by + r),
    ];
    push_box_prism_enu(
        &mut verts,
        &mut idxs,
        &bh_fp,
        -30.0,
        0.65,
        [0.88, 0.94, 0.96, 0.92],
        3.0,
    );
    let cr_r = 0.38;
    let collar_fp = [
        rot_xy(bx - cr_r, by - cr_r),
        rot_xy(bx + cr_r, by - cr_r),
        rot_xy(bx + cr_r, by + cr_r),
        rot_xy(bx - cr_r, by + cr_r),
    ];
    for s in &strata {
        let z_top = -(s.depth_top_m as f32);
        push_box_prism_enu(
            &mut verts,
            &mut idxs,
            &collar_fp,
            z_top - 0.18,
            z_top + 0.04,
            [0.32, 0.84, 0.88, 0.94],
            3.0,
        );
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// District multi-hazard input bundle rendered in the shared 24-bit Z-buffered WebGPU RTC pass.
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
    build_rtc_site_scene_mesh_with_landscape(
        theme,
        origin_lat_lon,
        zoom_level,
        show_terrain_contours,
        active_site_tin,
        world_index,
        active_rel_path,
        urban_vectors,
        city_scene,
        true,
        0.0,
    )
}

/// Builds the Floating-Origin RTC 3D local city/site GPU mesh with configurable realistic biome
/// terrain shader mode (`material_kind = 28.0`) and coastal sea level datum (`m MSL`).
#[allow(clippy::too_many_arguments)]
pub fn build_rtc_site_scene_mesh_with_landscape(
    theme: VectorEarthTheme,
    origin_lat_lon: (f64, f64),
    zoom_level: f32,
    show_terrain_contours: bool,
    active_site_tin: Option<&TinSurfaceMesh>,
    world_index: &WorldSpatialIndex,
    active_rel_path: Option<&str>,
    urban_vectors: &UrbanVectorLayerCache,
    city_scene: Option<&IfcxCityScene>,
    realistic_landscape: bool,
    sea_level_m: f32,
) -> GpuMeshPayload {
    let pal = theme.palette();
    let mut vertices = Vec::with_capacity(8_192);
    let mut indices = Vec::with_capacity(16_384);
    let has_coastal_water_tin = show_terrain_contours
        && realistic_landscape
        && active_site_tin.is_some_and(|t| t.min_elevation < sea_level_m - 0.1);

    // 1. Stylized Ground Terrain Apron & City-Builder Parcel Pads at site scale
    // Suppressed when a realistic coastal TIN spans below sea level so seabed bathymetry stays visible.
    if zoom_level >= 12.5 && !has_coastal_water_tin {
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
        if !(show_terrain_contours && realistic_landscape && active_site_tin.is_some()) {
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
    }

    // 2. 3D Triangulated `IfcSite` / Survey / Procedural TIN Terrain Surface
    if show_terrain_contours {
        if let Some(tin) = active_site_tin {
            let (ref_z, _) = tin.sample_elevation_and_slope(0.0, 0.0);
            if realistic_landscape {
                let mut v_norms = vec![[0.0_f32, 0.0, 0.0]; tin.points.len()];
                for tm in &tin.triangle_metrics {
                    let w = tm.area_3d_m2.max(1e-4) as f32;
                    for &vi in &tm.indices {
                        if let Some(acc) = v_norms.get_mut(vi) {
                            acc[0] += tm.normal[0] * w;
                            acc[1] += tm.normal[1] * w;
                            acc[2] += tm.normal[2] * w;
                        }
                    }
                }
                for n in &mut v_norms {
                    let nz = n[2].max(0.05);
                    let len = (n[0] * n[0] + n[1] * n[1] + nz * nz).sqrt().max(1e-6);
                    n[0] /= len;
                    n[1] /= len;
                    n[2] = nz / len;
                }

                for tm in tin.triangle_metrics.iter().take(12_000) {
                    let tb = vertices.len() as u32;
                    for &idx in &tm.indices {
                        let p = tin.points[idx];
                        let vn = v_norms.get(idx).copied().unwrap_or(tm.normal);
                        let elev_sea = p.z - sea_level_m;
                        let slope_norm = (1.0 - vn[2].clamp(0.0, 1.0)).clamp(0.0, 1.0);
                        let ao = (0.82 + 0.18 * ((elev_sea + 12.0) / 36.0).clamp(0.0, 1.0))
                            .clamp(0.76, 1.0);
                        let col = if elev_sea < 0.0 {
                            [0.48, 0.52, 0.44, 1.0]
                        } else if elev_sea < 3.0 {
                            [0.82, 0.76, 0.58, 1.0]
                        } else if elev_sea < 32.0 {
                            [0.24, 0.52, 0.25, 1.0]
                        } else {
                            [0.34, 0.36, 0.38, 1.0]
                        };
                        vertices.push(GpuEarthVertex {
                            pos: [p.x, p.y, p.z - ref_z],
                            normal: vn,
                            color: col,
                            meta: [28.0, elev_sea, ao, slope_norm],
                        });
                    }
                    indices.extend_from_slice(&[tb, tb + 1, tb + 2]);
                }
            } else {
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
        // Subtle EigenGate/DCR health tint blended into city-builder facade color
        let health_tint = if b.max_dcr > 1.0 || b.min_eigengate < 0.70 {
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
        push_city_builder_scene_enu(
            &mut vertices,
            &mut indices,
            theme,
            city,
            active_site_tin,
            sea_level_m,
            wall_base,
            roof_base,
        );
    }

    GpuMeshPayload { vertices, indices }
}

/// Phase 73: 3D District Portfolio Retrofit Allocation & Structural Crown Overlay Input.
pub struct PortfolioGpuOverlayInput<'a> {
    pub report: &'a crate::core::portfolio_retrofit::DistrictPortfolioRetrofitReport,
    pub origin_lat_lon: (f64, f64),
}

/// Builds hardware 3D Pure WebGPU translucent retrofit envelopes & glowing rooftop parity crowns across
/// every multi-building parcel inside the active district Pareto frontier (`PortfolioParetoPoint`).
pub fn build_rtc_portfolio_retrofit_mesh(input: &PortfolioGpuOverlayInput<'_>) -> GpuMeshPayload {
    let Some(pt) = input.report.active_frontier() else {
        return GpuMeshPayload::default();
    };
    let mut verts = Vec::with_capacity(pt.decisions.len() * 64);
    let mut idxs = Vec::with_capacity(pt.decisions.len() * 96);

    for d in &pt.decisions {
        let off_mm = SlippyMapState::lat_lon_to_world_mm(input.origin_lat_lon, d.lat, d.lon);
        let cx_m = off_mm[0] * 0.001;
        let cy_m = off_mm[1] * 0.001;
        if cx_m.hypot(cy_m) > 15_000.0 || d.footprint_local_m.len() < 3 {
            continue;
        }
        let rad = d.heading_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let fp: Vec<[f64; 2]> = d
            .footprint_local_m
            .iter()
            .map(|&[lx, ly]| {
                let slx = lx * 1.04;
                let sly = ly * 1.04;
                [cx_m + slx * cr - sly * sr, cy_m + slx * sr + sly * cr]
            })
            .collect();
        let h_m = d.height_m.clamp(3.6, 650.0) as f32;

        if let Some(strat) = d.selected_strategy {
            let crown_col = match strat {
                crate::core::retrofit::RetrofitStrategy::AutoBalanced
                | crate::core::retrofit::RetrofitStrategy::BrbBracingOnly => {
                    [0.12_f32, 0.88, 0.62, 0.56]
                }
                crate::core::retrofit::RetrofitStrategy::RcJacketingOnly => {
                    [0.22_f32, 0.74, 0.96, 0.56]
                }
                crate::core::retrofit::RetrofitStrategy::ShearWallCore => {
                    [0.96_f32, 0.70, 0.18, 0.56]
                }
            };
            let shell_col = [crown_col[0], crown_col[1], crown_col[2], 0.22];
            push_extruded_building_prism_enu(&mut verts, &mut idxs, &fp, 0.0, h_m, shell_col, crown_col);
            push_extruded_building_prism_enu(&mut verts, &mut idxs, &fp, h_m, 1.2, crown_col, crown_col);
        } else if d.pre_min_eigengate < input.report.config.target_min_eigengate || d.pre_max_dcr > 0.85 {
            let crit_col = [0.94_f32, 0.28, 0.24, 0.45];
            push_extruded_building_prism_enu(&mut verts, &mut idxs, &fp, h_m, 1.1, crit_col, crit_col);
        }
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// Phase 74: 3D ASCE 41-17 Plastic Hinge Beacons & Ground-Motion Seismic Pulse Input.
pub struct PlasticHingeGpuInput<'a> {
    pub project: &'a BimProject,
    pub frame: &'a crate::core::ground_motion_replay::GroundMotionReplayFrame,
    pub show_hinges: bool,
    pub deform_scale: f64,
    pub heading_deg: f64,
}

fn push_octahedron_beacon_enu(
    verts: &mut Vec<GpuEarthVertex>,
    idxs: &mut Vec<u32>,
    c: [f32; 3],
    radius: f32,
    rgba: [f32; 4],
    severity_phase: f32,
    material_kind: f32,
) {
    let px = [c[0] + radius, c[1], c[2]];
    let nx = [c[0] - radius, c[1], c[2]];
    let py = [c[0], c[1] + radius, c[2]];
    let ny = [c[0], c[1] - radius, c[2]];
    let pz = [c[0], c[1], c[2] + radius * 1.15];
    let nz = [c[0], c[1], c[2] - radius * 1.15];
    let faces = [
        [px, py, pz],
        [py, nx, pz],
        [nx, ny, pz],
        [ny, px, pz],
        [py, px, nz],
        [nx, py, nz],
        [ny, nx, nz],
        [px, ny, nz],
    ];
    for [a, b, d] in faces {
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [d[0] - a[0], d[1] - a[1], d[2] - a[2]];
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let nl = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-5);
        let norm = [n[0] / nl, n[1] / nl, n[2] / nl];
        let base = verts.len() as u32;
        for pt in [a, b, d] {
            verts.push(GpuEarthVertex {
                pos: pt,
                normal: norm,
                color: rgba,
                meta: [material_kind, c[2], 1.0, severity_phase],
            });
        }
        idxs.extend_from_slice(&[base, base + 1, base + 2]);
    }
}

/// Builds 3D glowing octahedral ASCE 41-17 plastic hinge beacons (`material_kind = 6.0`) at yielded
/// structural member ends plus a dynamic seismic ground-pulse footprint ring around the building base.
pub fn build_rtc_plastic_hinge_mesh(input: &PlasticHingeGpuInput<'_>) -> GpuMeshPayload {
    use crate::core::nonlinear::PlasticHingeState;

    let mut verts = Vec::with_capacity(input.frame.active_hinges.len() * 24 + 96);
    let mut idxs = Vec::with_capacity(input.frame.active_hinges.len() * 24 + 96);

    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let vis = (input.deform_scale * 0.25).clamp(1.0, 28.0);
    let shake_x = input.frame.ground_disp_m * vis * 0.35;
    let shake_y = input.frame.ground_disp_y_m * vis * 0.35;
    let shake_z = input.frame.ground_disp_z_m * vis * 0.35;

    let sway_at_z = |z_m: f64| -> [f64; 3] {
        let elevs = &input.frame.story_elevations_m;
        let dx_list = &input.frame.story_disp_x_m;
        let dy_list = &input.frame.story_disp_y_m;
        let dz_list = &input.frame.story_disp_z_m;
        let n = elevs.len().min(dx_list.len()).min(dy_list.len());
        if n == 0 {
            return [shake_x, shake_y, shake_z];
        }
        let mut dx = dx_list[n - 1];
        let mut dy = dy_list[n - 1];
        let mut dz = dz_list.get(n - 1).copied().unwrap_or(0.0);
        if z_m <= elevs[0] {
            dx = dx_list[0];
            dy = dy_list[0];
            dz = dz_list.first().copied().unwrap_or(0.0);
        } else {
            for k in 1..n {
                if z_m <= elevs[k] {
                    let t = ((z_m - elevs[k - 1]) / (elevs[k] - elevs[k - 1]).max(1e-4))
                        .clamp(0.0, 1.0);
                    dx = dx_list[k - 1] + (dx_list[k] - dx_list[k - 1]) * t;
                    dy = dy_list[k - 1] + (dy_list[k] - dy_list[k - 1]) * t;
                    let z0 = dz_list.get(k - 1).copied().unwrap_or(0.0);
                    let z1 = dz_list.get(k).copied().unwrap_or(0.0);
                    dz = z0 + (z1 - z0) * t;
                    break;
                }
            }
        }
        [
            (dx * vis + shake_x).clamp(-2.8, 2.8),
            (dy * vis + shake_y).clamp(-2.8, 2.8),
            (dz * vis + shake_z).clamp(-0.85, 0.85),
        ]
    };

    if input.show_hinges {
        for h in &input.frame.active_hinges {
            let (rgba, radius, sev) = match h.state {
                PlasticHingeState::Elastic => continue,
                PlasticHingeState::ImmediateOccupancy => ([0.04, 0.56, 1.00, 0.88], 0.36_f32, 0.25_f32),
                PlasticHingeState::LifeSafety => ([1.00, 0.64, 0.08, 0.92], 0.44_f32, 0.55_f32),
                PlasticHingeState::CollapsePrevention => ([1.00, 0.26, 0.22, 0.95], 0.54_f32, 0.80_f32),
                PlasticHingeState::Mechanism => ([0.78, 0.34, 0.98, 0.98], 0.64_f32, 1.00_f32),
            };
            let d = sway_at_z(h.local_pos_m[2]);
            let lx = h.local_pos_m[0] + d[0];
            let ly = h.local_pos_m[1] + d[1];
            let rx = (lx * cr - ly * sr) as f32;
            let ry = (lx * sr + ly * cr) as f32;
            let rz = (h.local_pos_m[2] + d[2]) as f32;
            push_octahedron_beacon_enu(&mut verts, &mut idxs, [rx, ry, rz], radius, rgba, sev, 6.0);
        }
    }

    // Dynamic ground-motion base seismic pulse ring beneath building footprint
    let accel_intensity = (input.frame.ground_accel_g.abs() / 0.85).clamp(0.10, 1.0) as f32;
    let (cx, cy, base_rad) = if let Some((bmin, bmax)) = input.project.plan_extents_mm() {
        let mx = (bmin[0] + bmax[0]) * 0.0005;
        let my = (bmin[1] + bmax[1]) * 0.0005;
        let r = ((bmax[0] - bmin[0]).hypot(bmax[1] - bmin[1]) * 0.00055).max(5.0);
        (mx * cr - my * sr, mx * sr + my * cr, r)
    } else {
        (0.0, 0.0, 7.0)
    };
    let r_inner = (base_rad + accel_intensity as f64 * 1.4) as f32;
    let r_outer = r_inner + 0.65 + accel_intensity * 0.55;
    let ring_col = [
        0.22 + 0.76 * accel_intensity,
        0.82 - 0.52 * accel_intensity,
        0.88 - 0.56 * accel_intensity,
        0.48 + 0.34 * accel_intensity,
    ];
    let segs = 28usize;
    let base_v = verts.len() as u32;
    for i in 0..=segs {
        let th = (i as f32 / segs as f32) * std::f32::consts::TAU;
        let (st, ct) = (th.sin(), th.cos());
        verts.push(GpuEarthVertex {
            pos: [cx as f32 + r_inner * ct, cy as f32 + r_inner * st, 0.14],
            normal: [0.0, 0.0, 1.0],
            color: ring_col,
            meta: [6.0, 0.14, 1.0, accel_intensity],
        });
        verts.push(GpuEarthVertex {
            pos: [cx as f32 + r_outer * ct, cy as f32 + r_outer * st, 0.14],
            normal: [0.0, 0.0, 1.0],
            color: [ring_col[0], ring_col[1], ring_col[2], 0.10],
            meta: [6.0, 0.14, 1.0, accel_intensity],
        });
    }
    for i in 0..segs {
        let a = base_v + (i as u32) * 2;
        idxs.extend_from_slice(&[a, a + 1, a + 3, a, a + 3, a + 2]);
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// Phase 75: Translucent per-story FEMA P-58 repair-loss halo bands (`material_kind = 7.0`).
pub struct FragilityLossGpuInput<'a> {
    pub project: &'a BimProject,
    pub synthesis: &'a crate::core::fema_p58::ReplayFemaP58Synthesis,
    pub visible: bool,
    pub heading_deg: f64,
}

/// Builds 3D translucent perimeter halo bands along each BIM storey color-coded by FEMA P-58 repair loss ratio.
pub fn build_rtc_fragility_loss_mesh(input: &FragilityLossGpuInput<'_>) -> GpuMeshPayload {
    if !input.visible || input.synthesis.story_losses.is_empty() || input.project.is_empty() {
        return GpuMeshPayload::default();
    }
    let (bmin, bmax) = input
        .project
        .plan_extents_mm()
        .map(|(mn, mx)| (
            [mn[0] * 0.001 - 0.85, mn[1] * 0.001 - 0.85],
            [mx[0] * 0.001 + 0.85, mx[1] * 0.001 + 0.85],
        ))
        .unwrap_or(([-6.5, -5.5], [6.5, 5.5]));
    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_pt = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };

    let outer = [
        rot_pt(bmin[0], bmin[1]),
        rot_pt(bmax[0], bmin[1]),
        rot_pt(bmax[0], bmax[1]),
        rot_pt(bmin[0], bmax[1]),
    ];
    let mut verts = Vec::with_capacity(input.synthesis.story_losses.len() * 24);
    let mut idxs = Vec::with_capacity(input.synthesis.story_losses.len() * 36);

    for st in &input.synthesis.story_losses {
        let lr = st.loss_ratio;
        let rgba = if lr >= 0.15 {
            [0.98, 0.24, 0.24, 0.44]
        } else if lr >= 0.06 {
            [0.98, 0.62, 0.12, 0.40]
        } else if lr >= 0.02 {
            [0.16, 0.78, 0.92, 0.36]
        } else {
            [0.12, 0.82, 0.56, 0.32]
        };
        let z0 = (st.elevation_m - 0.42).max(0.10) as f32;
        let z1 = (st.elevation_m + 0.22) as f32;
        push_box_prism_enu(&mut verts, &mut idxs, &outer, z0, z1, rgba, 7.0);
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// Phase 76: 3D Live SHM Sensor Beacons, Rooftop ATC-20 Placard Shield & District Triage Crowns (`material_kind = 8.0`).
pub struct ShmTriageGpuInput<'a> {
    pub project: &'a BimProject,
    pub report: &'a crate::core::shm_twin::ShmDigitalTwinReport,
    pub visible: bool,
    pub heading_deg: f64,
    pub site_origin: (f64, f64),
}

/// Builds 3D Pure WebGPU SHM sensor beacons, active rooftop ATC-20 safety placard shields, and nearby
/// district ATC-20 building crowns encoded with `material_kind = 8.0` for pulsing emissive + Fresnel shading.
pub fn build_rtc_shm_triage_mesh(input: &ShmTriageGpuInput<'_>) -> GpuMeshPayload {
    if !input.visible || input.project.is_empty() {
        return GpuMeshPayload::default();
    }

    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());

    let mut verts = Vec::with_capacity(input.report.sensors.len() * 24 + 256);
    let mut idxs = Vec::with_capacity(input.report.sensors.len() * 24 + 256);

    // 1. Live SHM Triaxial Sensors / Inclinometers / Strain Gauges mounted on BIM skeleton
    for s in &input.report.sensors {
        let [lx, ly, lz] = s.position_local_m;
        let rx = (lx * cr - ly * sr) as f32;
        let ry = (lx * sr + ly * cr) as f32;
        let rz = lz as f32;
        let col = if s.health_index < 0.72 {
            [0.98, 0.24, 0.24, 0.98]
        } else if s.is_triggered || s.health_index < 0.88 {
            [0.98, 0.64, 0.12, 0.96]
        } else {
            [0.13, 0.85, 0.56, 0.94]
        };
        let radius = if s.is_triggered { 0.56_f32 } else { 0.42_f32 };
        let sev_phase = if s.is_triggered { 1.0_f32 } else { 0.35_f32 };
        push_octahedron_beacon_enu(&mut verts, &mut idxs, [rx, ry, rz], radius, col, sev_phase, 8.0);
    }

    // 2. Active Building Rooftop ATC-20 Placard Shield Prism & Crown Collar
    let (bmin, bmax) = input
        .project
        .plan_extents_mm()
        .map(|(mn, mx)| ([mn[0] * 0.001, mn[1] * 0.001], [mx[0] * 0.001, mx[1] * 0.001]))
        .unwrap_or(([-6.0, -5.0], [6.0, 5.0]));
    let cx = 0.5 * (bmin[0] + bmax[0]);
    let cy = 0.5 * (bmin[1] + bmax[1]);
    let hx = ((bmax[0] - bmin[0]) * 0.28).clamp(1.2, 4.5);
    let hy = ((bmax[1] - bmin[1]) * 0.28).clamp(1.2, 4.5);

    let rot_pt = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };
    let shield_fp = [
        rot_pt(cx - hx, cy - hy),
        rot_pt(cx + hx, cy - hy),
        rot_pt(cx + hx, cy + hy),
        rot_pt(cx - hx, cy + hy),
    ];
    let [pr, pg, pb] = input.report.atc20_placard.normalized_rgb();
    let shield_z0 = input.report.building_height_m as f32 + 1.4;
    let shield_z1 = shield_z0 + 1.5;
    let placard_sev = (0.25 + 0.35 * input.report.atc20_placard.gpu_code()).clamp(0.1, 1.0);
    let v_start = verts.len();
    push_box_prism_enu(
        &mut verts,
        &mut idxs,
        &shield_fp,
        shield_z0,
        shield_z1,
        [pr, pg, pb, 0.92],
        8.0,
    );
    for v in &mut verts[v_start..] {
        v.meta[3] = placard_sev;
    }

    // Rooftop perimeter collar
    let collar_fp = [
        rot_pt(bmin[0] - 0.5, bmin[1] - 0.5),
        rot_pt(bmax[0] + 0.5, bmin[1] - 0.5),
        rot_pt(bmax[0] + 0.5, bmax[1] + 0.5),
        rot_pt(bmin[0] - 0.5, bmax[1] + 0.5),
    ];
    let collar_z0 = (input.report.building_height_m as f32 - 0.15).max(0.5);
    let v_collar = verts.len();
    push_box_prism_enu(
        &mut verts,
        &mut idxs,
        &collar_fp,
        collar_z0,
        collar_z0 + 0.45,
        [pr, pg, pb, 0.52],
        8.0,
    );
    for v in &mut verts[v_collar..] {
        v.meta[3] = placard_sev * 0.85;
    }

    // 3. Nearby District Portfolio Buildings ATC-20 Placard Shields (within 3500 m)
    for b in &input.report.district_placards {
        let dlat = (b.lat - input.site_origin.0).abs();
        let dlon = (b.lon - input.site_origin.1).abs();
        if dlat < 1e-5 && dlon < 1e-5 {
            continue;
        }
        let off_mm = SlippyMapState::lat_lon_to_world_mm(input.site_origin, b.lat, b.lon);
        let bx_m = off_mm[0] * 0.001;
        let by_m = off_mm[1] * 0.001;
        if bx_m.hypot(by_m) > 3500.0 {
            continue;
        }
        let brad = b.heading_deg.to_radians();
        let (bsr, bcr) = (brad.sin(), brad.cos());
        let d_fp = [
            [bx_m + (-2.0 * bcr - -2.0 * bsr), by_m + (-2.0 * bsr + -2.0 * bcr)],
            [bx_m + (2.0 * bcr - -2.0 * bsr), by_m + (2.0 * bsr + -2.0 * bcr)],
            [bx_m + (2.0 * bcr - 2.0 * bsr), by_m + (2.0 * bsr + 2.0 * bcr)],
            [bx_m + (-2.0 * bcr - 2.0 * bsr), by_m + (-2.0 * bsr + 2.0 * bcr)],
        ];
        let [dr, dg, db] = b.placard.normalized_rgb();
        let dz0 = b.total_height_m.max(3.5) as f32 + 1.0;
        let dv_start = verts.len();
        push_box_prism_enu(
            &mut verts,
            &mut idxs,
            &d_fp,
            dz0,
            dz0 + 1.4,
            [dr, dg, db, 0.86],
            8.0,
        );
        let dsev = (0.25 + 0.35 * b.placard.gpu_code()).clamp(0.1, 1.0);
        for v in &mut verts[dv_start..] {
            v.meta[3] = dsev;
        }
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// Phase 77: 3D Post-Disaster Evacuation Corridors, ATC-20 Debris Cordons, Road Closures &
/// Safe Assembly Point Beacons (`material_kind = 9.0`).
pub struct EvacuationLifelineGpuInput<'a> {
    pub report: &'a crate::core::evacuation_routing::EvacuationLifelineReport,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU evacuation route ribbons, responder access corridors, ATC-20 debris cordon
/// rings, blocked road markers, and assembly safe-zone beacons (`material_kind = 9.0`).
pub fn build_rtc_evacuation_lifelines_mesh(
    input: &EvacuationLifelineGpuInput<'_>,
) -> GpuMeshPayload {
    use crate::core::evacuation_routing::{EvacuationMode, RoadEdgeStatus};
    use crate::core::shm_twin::Atc20Placard;

    if !input.visible {
        return GpuMeshPayload::default();
    }

    let mut verts = Vec::with_capacity(1024);
    let mut idxs = Vec::with_capacity(2048);

    // 1. ATC-20 Red / Yellow Building Debris Cordon Annular Rings
    for c in &input.report.cordons {
        if c.placard == Atc20Placard::InspectedGreen {
            continue;
        }
        let col = match c.placard {
            Atc20Placard::UnsafeRed => [0.96_f32, 0.20, 0.22, 0.44],
            Atc20Placard::RestrictedYellow => [0.96_f32, 0.68, 0.14, 0.36],
            Atc20Placard::InspectedGreen => continue,
        };
        let r_outer = c.cordon_radius_m as f32;
        let r_inner = (r_outer - 1.65).max(c.footprint_radius_m as f32 * 0.75);
        let segs = 28usize;
        let base_v = verts.len() as u32;
        for i in 0..=segs {
            let th = (i as f32 / segs as f32) * std::f32::consts::TAU;
            let (st, ct) = (th.sin(), th.cos());
            verts.push(GpuEarthVertex {
                pos: [
                    c.center_enu_m[0] as f32 + r_inner * ct,
                    c.center_enu_m[1] as f32 + r_inner * st,
                    0.28,
                ],
                normal: [0.0, 0.0, 1.0],
                color: [col[0], col[1], col[2], col[3] * 0.45],
                meta: [9.0, 0.28, 1.0, 0.0],
            });
            verts.push(GpuEarthVertex {
                pos: [
                    c.center_enu_m[0] as f32 + r_outer * ct,
                    c.center_enu_m[1] as f32 + r_outer * st,
                    0.28,
                ],
                normal: [0.0, 0.0, 1.0],
                color: col,
                meta: [9.0, 0.28, 1.0, 0.0],
            });
        }
        for i in 0..segs {
            let a = base_v + (i as u32) * 2;
            idxs.extend_from_slice(&[a, a + 1, a + 3, a, a + 3, a + 2]);
        }
    }

    // 2. Blocked & Degraded Road / Bridge Segments
    for e in &input.report.edges {
        if e.road_name == "Site Access Spur" || e.status == RoadEdgeStatus::OpenSafe {
            continue;
        }
        let (Some(nu), Some(nv)) = (input.report.nodes.get(e.u), input.report.nodes.get(e.v)) else {
            continue;
        };
        let ax = nu.pos_enu_m[0] as f32;
        let ay = nu.pos_enu_m[1] as f32;
        let bx = nv.pos_enu_m[0] as f32;
        let by = nv.pos_enu_m[1] as f32;
        let dx = bx - ax;
        let dy = by - ay;
        let len = dx.hypot(dy).max(0.2);
        let hw = (e.width_m as f32 * 0.24).clamp(0.85, 2.6);
        let nx = -dy / len * hw;
        let ny = dx / len * hw;
        let z = (0.5 * (nu.elev_m + nv.elev_m) + 0.30) as f32;
        let col = match e.status {
            RoadEdgeStatus::BlockedClosed => [0.96_f32, 0.18, 0.22, 0.68],
            RoadEdgeStatus::DegradedDetour => [0.96_f32, 0.66, 0.16, 0.56],
            RoadEdgeStatus::OpenSafe => continue,
        };
        let base = verts.len() as u32;
        for pt in [
            [ax - nx, ay - ny, z],
            [bx - nx, by - ny, z],
            [bx + nx, by + ny, z],
            [ax + nx, ay + ny, z],
        ] {
            verts.push(GpuEarthVertex {
                pos: pt,
                normal: [0.0, 0.0, 1.0],
                color: col,
                meta: [9.0, z, 1.0, 0.0],
            });
        }
        idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    // 3. Active Evacuation & Responder Flow Corridors (with cumulative distance in `meta[3]`)
    let show_ped = matches!(
        input.report.config.mode,
        EvacuationMode::PedestrianEvacuation | EvacuationMode::Both
    );
    let show_resp = matches!(
        input.report.config.mode,
        EvacuationMode::EmergencyResponder | EvacuationMode::Both
    );
    for route in &input.report.routes {
        if !route.reachable || route.waypoints_enu_m.len() < 2 {
            continue;
        }
        if route.is_responder_route && !show_resp {
            continue;
        }
        if !route.is_responder_route && !show_ped {
            continue;
        }
        let hw = if route.is_active_building { 1.45_f32 } else { 0.95_f32 };
        let z = if route.is_responder_route { 0.88_f32 } else { 0.68_f32 };
        let col = if route.is_responder_route {
            [0.22_f32, 0.72, 1.00, 0.90]
        } else if route.has_detour {
            [0.98_f32, 0.72, 0.20, 0.88]
        } else {
            [0.10_f32, 0.94, 0.72, 0.90]
        };
        let mut cum_dist = 0.0_f32;
        for seg in route.waypoints_enu_m.windows(2) {
            let ax = seg[0][0] as f32;
            let ay = seg[0][1] as f32;
            let bx = seg[1][0] as f32;
            let by = seg[1][1] as f32;
            let dx = bx - ax;
            let dy = by - ay;
            let seg_len = dx.hypot(dy).max(0.1);
            let nx = -dy / seg_len * hw;
            let ny = dx / seg_len * hw;
            let d0 = cum_dist;
            let d1 = cum_dist + seg_len;
            cum_dist = d1;
            let base = verts.len() as u32;
            verts.push(GpuEarthVertex {
                pos: [ax - nx, ay - ny, z],
                normal: [0.0, 0.0, 1.0],
                color: col,
                meta: [9.0, z, 1.0, d0],
            });
            verts.push(GpuEarthVertex {
                pos: [bx - nx, by - ny, z],
                normal: [0.0, 0.0, 1.0],
                color: col,
                meta: [9.0, z, 1.0, d1],
            });
            verts.push(GpuEarthVertex {
                pos: [bx + nx, by + ny, z],
                normal: [0.0, 0.0, 1.0],
                color: col,
                meta: [9.0, z, 1.0, d1],
            });
            verts.push(GpuEarthVertex {
                pos: [ax + nx, ay + ny, z],
                normal: [0.0, 0.0, 1.0],
                color: col,
                meta: [9.0, z, 1.0, d0],
            });
            idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    // 4. Safe Assembly Zone Beacons (`IfcSpatialZone` Parks / Plazas)
    for sp in &input.report.safe_points {
        let cx = sp.center_enu_m[0] as f32;
        let cy = sp.center_enu_m[1] as f32;
        let pad_fp = [
            [cx as f64 - 4.2, cy as f64 - 4.2],
            [cx as f64 + 4.2, cy as f64 - 4.2],
            [cx as f64 + 4.2, cy as f64 + 4.2],
            [cx as f64 - 4.2, cy as f64 + 4.2],
        ];
        push_box_prism_enu(
            &mut verts,
            &mut idxs,
            &pad_fp,
            0.18,
            0.55,
            [0.16, 0.92, 0.52, 0.65],
            9.0,
        );
        push_octahedron_beacon_enu(
            &mut verts,
            &mut idxs,
            [cx, cy, 6.2],
            1.65,
            [0.16, 0.95, 0.56, 0.94],
            1.5,
            9.0,
        );
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Phase 80 District Risk Field Heatmap (`material_kind = 10.0`).
pub struct DistrictRiskHeatmapGpuInput<'a> {
    pub report: &'a crate::core::ifcx_collab::DistrictRiskFieldReport,
    pub opacity: f32,
    pub visible: bool,
}

fn risk_ramp_rgba(risk_01: f32, opacity: f32) -> [f32; 4] {
    let t = risk_01.clamp(0.0, 1.0);
    let rgb = if t < 0.35 {
        let u = t / 0.35;
        mix_rgb([0.06, 0.72, 0.50, 1.0], [0.02, 0.71, 0.83, 1.0], u)
    } else if t < 0.65 {
        let u = (t - 0.35) / 0.30;
        mix_rgb([0.02, 0.71, 0.83, 1.0], [0.96, 0.62, 0.04, 1.0], u)
    } else {
        let u = (t - 0.65) / 0.35;
        mix_rgb([0.96, 0.62, 0.04, 1.0], [0.94, 0.24, 0.27, 1.0], u)
    };
    [rgb[0], rgb[1], rgb[2], (opacity * (0.45 + 0.45 * t)).clamp(0.15, 0.88)]
}

/// Builds a terrain-draped pure WebGPU District Risk Field Heatmap mesh (`material_kind = 10.0`)
/// with continuous IDW grid quads and elevated hotspot octahedron beacons.
pub fn build_rtc_district_risk_heatmap_mesh(
    input: &DistrictRiskHeatmapGpuInput<'_>,
) -> GpuMeshPayload {
    if !input.visible || input.report.cells.is_empty() {
        return GpuMeshPayload::default();
    }
    let cell_count = input.report.cells.len();
    let mut verts = Vec::with_capacity(cell_count * 4 + input.report.hotspots.len() * 24);
    let mut idxs = Vec::with_capacity(cell_count * 6 + input.report.hotspots.len() * 24);

    for cell in &input.report.cells {
        let z = cell.elev_m + 0.16 + cell.risk_01 * 0.45;
        let hs = cell.half_step_m * 0.96;
        let col = risk_ramp_rgba(cell.risk_01, input.opacity);
        let base = verts.len() as u32;
        for (dx, dy) in [(-hs, -hs), (hs, -hs), (hs, hs), (-hs, hs)] {
            verts.push(GpuEarthVertex {
                pos: [cell.cx_m + dx, cell.cy_m + dy, z],
                normal: [0.0, 0.0, 1.0],
                color: col,
                meta: [10.0, cell.risk_01, 1.0, cell.elev_m],
            });
        }
        idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    // Elevated hotspot risk beacons over high/critical parcels
    for h in input.report.hotspots.iter().take(12) {
        if h.risk_score < 0.45 {
            continue;
        }
        let col = risk_ramp_rgba(h.risk_score, 0.95);
        push_octahedron_beacon_enu(
            &mut verts,
            &mut idxs,
            [h.east_m, h.north_m, 9.5 + h.risk_score * 8.0],
            1.4 + h.risk_score * 1.1,
            [col[0], col[1], col[2], 0.92],
            h.risk_score,
            10.0,
        );
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Phase 82 Field Construction QA/QC Waypoints & Pour-Strip Overlays (`material_kind = 11.0`).
pub struct FieldQaqcGpuInput<'a> {
    pub report: &'a crate::core::field_fabrication::FieldFabricationReport,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU Field QA/QC concrete pour-strip volumes, structural ITP hold beacons,
/// and NCR punch pins (`material_kind = 11.0`, 0% raster) anchored in local ENU meters.
pub fn build_rtc_field_qaqc_mesh(input: &FieldQaqcGpuInput<'_>) -> GpuMeshPayload {
    use crate::core::field_fabrication::{FieldInspectionStatus, PourStripStatus};

    if !input.visible {
        return GpuMeshPayload::default();
    }

    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_xy = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };

    let mut verts = Vec::with_capacity(
        input.report.pour_strips.len() * 24 + input.report.waypoints.len() * 48,
    );
    let mut idxs = Vec::with_capacity(
        input.report.pour_strips.len() * 36 + input.report.waypoints.len() * 64,
    );

    // 1. Concrete Pour-Strip Volumes over Slab Levels
    for ps in &input.report.pour_strips {
        let [r, g, b] = ps.status.color_rgb();
        let alpha = match ps.status {
            PourStripStatus::CuredApproved => 0.28_f32,
            PourStripStatus::PouringScheduled => 0.34_f32,
            PourStripStatus::HoldPoint => 0.40_f32,
        };
        let col = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, alpha];
        let hw = ps.width_m * 0.5;
        let hd = ps.depth_m * 0.5;
        let fp = [
            rot_xy(ps.center_x_m - hw, ps.center_y_m - hd),
            rot_xy(ps.center_x_m + hw, ps.center_y_m - hd),
            rot_xy(ps.center_x_m + hw, ps.center_y_m + hd),
            rot_xy(ps.center_x_m - hw, ps.center_y_m + hd),
        ];
        let z0 = ps.z_m as f32;
        push_box_prism_enu(&mut verts, &mut idxs, &fp, z0, z0 + 0.14, col, 11.0);
    }

    // 2. Elevated 3D Octahedral QA/QC Waypoint & NCR Beacon Pins + Anchored Stems
    for wp in &input.report.waypoints {
        let [r, g, b] = wp.status.badge_color_rgb();
        let (col, sev, radius, lift_m) = match wp.status {
            FieldInspectionStatus::NcrOpen => (
                [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 0.96],
                1.0_f32,
                0.56_f32,
                2.6_f32,
            ),
            FieldInspectionStatus::HoldPoint => (
                [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 0.94],
                0.65_f32,
                0.48_f32,
                2.2_f32,
            ),
            FieldInspectionStatus::Cleared => (
                [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 0.85],
                0.15_f32,
                0.38_f32,
                1.8_f32,
            ),
        };
        let rxy = rot_xy(wp.local_x_m, wp.local_y_m);
        let z_anchor = wp.local_z_m as f32;
        let z_beacon = z_anchor + lift_m;
        let stem_fp = [
            [rxy[0] - 0.05, rxy[1] - 0.05],
            [rxy[0] + 0.05, rxy[1] - 0.05],
            [rxy[0] + 0.05, rxy[1] + 0.05],
            [rxy[0] - 0.05, rxy[1] + 0.05],
        ];
        push_box_prism_enu(
            &mut verts,
            &mut idxs,
            &stem_fp,
            z_anchor,
            z_beacon,
            [col[0], col[1], col[2], 0.72],
            11.0,
        );
        push_octahedron_beacon_enu(
            &mut verts,
            &mut idxs,
            [rxy[0] as f32, rxy[1] as f32, z_beacon],
            radius,
            col,
            sev,
            11.0,
        );
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

#[allow(clippy::too_many_arguments)]
fn push_animated_flow_strut_enu(
    verts: &mut Vec<GpuEarthVertex>,
    idxs: &mut Vec<u32>,
    p0_enu: [f64; 3],
    p1_enu: [f64; 3],
    width_m: f64,
    depth_m: f64,
    col: [f32; 4],
    severity: f32,
    material_kind: f32,
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
    let len_f = len as f32;
    for i in 0..4 {
        let ni = (i + 1) % 4;
        let base = verts.len() as u32;
        for (pos, phase) in [
            (a[i], 0.0_f32),
            (a[ni], 0.0_f32),
            (b[ni], len_f),
            (b[i], len_f),
        ] {
            verts.push(GpuEarthVertex {
                pos,
                normal: side_normals[i],
                color: col,
                meta: [material_kind, severity, 1.0, phase],
            });
        }
        idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

/// GPU input payload for Phase 83 EigenFlow Woodbury `P1 -> P2` Load-Path & Pushover Hinges (`material_kind = 12.0`).
pub struct EigenFlowGpuInput<'a> {
    pub report: &'a crate::core::eigenflow::EigenFlowStudioReport,
    pub active_step_idx: usize,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU EigenFlow disrupted `P1` ghost strut, surviving `P2` animated
/// load-redistribution conduits, and Pushover plastic hinge beacons (`material_kind = 12.0`, 0% raster).
pub fn build_rtc_eigenflow_mesh(input: &EigenFlowGpuInput<'_>) -> GpuMeshPayload {
    use crate::core::nonlinear::PlasticHingeState;

    if !input.visible {
        return GpuMeshPayload::default();
    }
    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_enu = |p: [f64; 3]| -> [f64; 3] {
        [p[0] * cr - p[1] * sr, p[0] * sr + p[1] * cr, p[2]]
    };

    let wb = &input.report.woodbury;
    let mut verts = Vec::with_capacity(
        128 + wb.redistributed_p2.len() * 64 + input.report.hinges_3d.len() * 32,
    );
    let mut idxs = Vec::with_capacity(
        256 + wb.redistributed_p2.len() * 96 + input.report.hinges_3d.len() * 48,
    );

    // 1. Disrupted primary load-path member P1 (pulsing crimson disruption sleeve + midpoint beacon)
    let p1_a = rot_enu(wb.disrupted_p0_enu_m);
    let p1_b = rot_enu(wb.disrupted_p1_enu_m);
    let sev_p1 = (1.0 - wb.lambda_val as f32).clamp(0.25, 1.0);
    push_animated_flow_strut_enu(
        &mut verts,
        &mut idxs,
        p1_a,
        p1_b,
        0.44,
        0.44,
        [0.98, 0.22, 0.28, 0.90],
        sev_p1,
        12.0,
    );
    push_octahedron_beacon_enu(
        &mut verts,
        &mut idxs,
        [
            (0.5 * (p1_a[0] + p1_b[0])) as f32,
            (0.5 * (p1_a[1] + p1_b[1])) as f32,
            (0.5 * (p1_a[2] + p1_b[2])) as f32,
        ],
        0.58,
        [1.0, 0.24, 0.30, 0.96],
        sev_p1,
        12.0,
    );

    // 2. Surviving P2 backup conduits with animated flow phase along member length
    for m in &wb.redistributed_p2 {
        let a = rot_enu(m.p0_enu_m);
        let b = rot_enu(m.p1_enu_m);
        let w_m = 0.20 + (m.share_pct / 100.0).clamp(0.05, 0.6) * 0.26;
        let (col, sev) = if m.is_critical_overload {
            ([1.0_f32, 0.28, 0.24, 0.94], 0.95_f32)
        } else if m.post_yield_dcr > 0.72 {
            ([1.0_f32, 0.68, 0.16, 0.92], 0.65_f32)
        } else {
            ([0.16_f32, 0.86, 0.96, 0.88], 0.30_f32)
        };
        push_animated_flow_strut_enu(&mut verts, &mut idxs, a, b, w_m, w_m, col, sev, 12.0);
        push_octahedron_beacon_enu(
            &mut verts,
            &mut idxs,
            [
                (0.5 * (a[0] + b[0])) as f32,
                (0.5 * (a[1] + b[1])) as f32,
                (0.5 * (a[2] + b[2])) as f32,
            ],
            0.30,
            col,
            sev,
            12.0,
        );
    }

    // 3. Pushover Step Plastic Hinges up to active scrubbed step
    for h in &input.report.hinges_3d {
        if h.formed_at_step <= input.active_step_idx {
            let p = rot_enu(h.pos_enu_m);
            let (col, sev, rad_m) = match h.state {
                PlasticHingeState::Elastic => ([0.18, 0.82, 0.45, 0.85], 0.15, 0.28),
                PlasticHingeState::ImmediateOccupancy => ([0.10, 0.54, 1.0, 0.92], 0.40, 0.36),
                PlasticHingeState::LifeSafety => ([1.0, 0.64, 0.12, 0.94], 0.70, 0.44),
                PlasticHingeState::CollapsePrevention => ([1.0, 0.26, 0.22, 0.96], 0.90, 0.52),
                PlasticHingeState::Mechanism => ([0.78, 0.34, 0.98, 0.98], 1.0, 0.60),
            };
            push_octahedron_beacon_enu(
                &mut verts,
                &mut idxs,
                [p[0] as f32, p[1] as f32, p[2] as f32],
                rad_m,
                col,
                sev,
                12.0,
            );
        }
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Phase 84 Extreme Events (Blast Shockwave Rings & Structural Fire Plumes, `material_kind = 13.0`).
pub struct ExtremeEventsGpuInput<'a> {
    pub report: &'a crate::core::extreme_events::ExtremeEventsReport,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU Extreme Events detonation shockwave rings, blast standoff/damage sleeves,
/// incandescent fire compartment volume, and thermally weakened column/beam halos (`material_kind = 13.0`, 0% raster).
pub fn build_rtc_extreme_events_mesh(input: &ExtremeEventsGpuInput<'_>) -> GpuMeshPayload {
    use crate::core::extreme_events::ExtremeEventMode;

    if !input.visible {
        return GpuMeshPayload::default();
    }
    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_enu = |p: [f64; 3]| -> [f64; 3] {
        [p[0] * cr - p[1] * sr, p[0] * sr + p[1] * cr, p[2]]
    };

    let rep = input.report;
    let show_blast = matches!(rep.config.mode, ExtremeEventMode::Blast | ExtremeEventMode::Combined);
    let show_fire = matches!(
        rep.config.mode,
        ExtremeEventMode::StructuralFire | ExtremeEventMode::Combined
    );

    let mut verts = Vec::with_capacity(512);
    let mut idxs = Vec::with_capacity(1024);

    if show_blast {
        let charge = rot_enu(rep.charge_pos_enu_m);
        let face = rot_enu(rep.target_face_enu_m);
        // Detonation beacon + standoff vector
        push_octahedron_beacon_enu(
            &mut verts,
            &mut idxs,
            [charge[0] as f32, charge[1] as f32, charge[2] as f32],
            0.85,
            [1.0, 0.28, 0.14, 0.96],
            1.0,
            13.0,
        );
        push_animated_flow_strut_enu(
            &mut verts,
            &mut idxs,
            charge,
            face,
            0.22,
            0.22,
            [1.0, 0.62, 0.16, 0.88],
            0.85,
            13.0,
        );

        // 3 Concentric Kingery-Bulmash Shockwave Rings on ground plane (z = 0.22 m)
        let r_base = rep.config.standoff_distance_m.clamp(3.0, 60.0);
        for (ring_idx, (frac, alpha)) in [(0.38_f64, 0.84_f32), (0.72, 0.62), (1.06, 0.42)]
            .iter()
            .enumerate()
        {
            let rad_m = (r_base * frac).max(1.8);
            let segs = 16usize;
            for s in 0..segs {
                let a0 = (s as f64 / segs as f64) * std::f64::consts::TAU;
                let a1 = ((s + 1) as f64 / segs as f64) * std::f64::consts::TAU;
                let q0 = [charge[0] + rad_m * a0.cos(), charge[1] + rad_m * a0.sin(), 0.22];
                let q1 = [charge[0] + rad_m * a1.cos(), charge[1] + rad_m * a1.sin(), 0.22];
                let sev = (1.0 - (ring_idx as f32) * 0.25).clamp(0.25, 1.0);
                push_animated_flow_strut_enu(
                    &mut verts,
                    &mut idxs,
                    q0,
                    q1,
                    0.26,
                    0.20,
                    [1.0, 0.36 + (ring_idx as f32) * 0.16, 0.12, *alpha],
                    sev,
                    13.0,
                );
            }
        }

        // Blast-damaged perimeter members color-coded by UFC Damage Level
        for m in &rep.blast_member_impacts {
            let (cr_u, cg_u, cb_u) = m.damage_level.color_rgb();
            let col = [
                cr_u as f32 / 255.0,
                cg_u as f32 / 255.0,
                cb_u as f32 / 255.0,
                0.92,
            ];
            let sev = (1.0 - m.residual_stiffness_lambda as f32).clamp(0.15, 1.0);
            push_animated_flow_strut_enu(
                &mut verts,
                &mut idxs,
                rot_enu(m.p0_enu_m),
                rot_enu(m.p1_enu_m),
                0.38,
                0.38,
                col,
                sev,
                13.0,
            );
        }
    }

    if show_fire {
        let fc = &rep.fire_compartment;
        let heat_norm = ((fc.gas_temp_c - 100.0) / 900.0).clamp(0.15, 1.0) as f32;
        let fire_col = [
            1.0_f32,
            (0.68 - 0.38 * heat_norm).clamp(0.20, 0.72),
            0.10,
            0.28 + 0.16 * heat_norm,
        ];
        let hw = fc.width_m * 0.5;
        let hd = fc.depth_m * 0.5;
        let rot_xy = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };
        let fp = [
            rot_xy(fc.center_x_m - hw, fc.center_y_m - hd),
            rot_xy(fc.center_x_m + hw, fc.center_y_m - hd),
            rot_xy(fc.center_x_m + hw, fc.center_y_m + hd),
            rot_xy(fc.center_x_m - hw, fc.center_y_m + hd),
        ];
        push_box_prism_enu(
            &mut verts,
            &mut idxs,
            &fp,
            (fc.z_bottom_m + 0.15) as f32,
            (fc.z_top_m - 0.15).max(fc.z_bottom_m + 1.0) as f32,
            fire_col,
            13.0,
        );

        for &(p0, p1, sev) in &rep.exposed_fire_members {
            let sev_f = sev as f32;
            let m_col = [1.0, (0.72 - 0.45 * sev_f).clamp(0.18, 0.75), 0.12, 0.90];
            push_animated_flow_strut_enu(
                &mut verts,
                &mut idxs,
                rot_enu(p0),
                rot_enu(p1),
                0.34,
                0.34,
                m_col,
                sev_f,
                13.0,
            );
        }
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Update `v0.8.4.1` Seismic Base Isolation Pads, Moat Ring & FVD Dampers (`material_kind = 14.0`).
pub struct IsolationGpuInput<'a> {
    pub report: &'a crate::core::base_isolation_studio::IsolationStudioReport,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU base isolator pads beneath each column, perimeter seismic moat clearance ring,
/// and diagonal Fluid Viscous Damper struts (`material_kind = 14.0`, 0% raster).
pub fn build_rtc_isolation_mesh(input: &IsolationGpuInput<'_>) -> GpuMeshPayload {
    if !input.visible {
        return GpuMeshPayload::default();
    }
    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_xy = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };
    let rot_enu = |p: [f64; 3]| -> [f64; 3] {
        [p[0] * cr - p[1] * sr, p[0] * sr + p[1] * cr, p[2]]
    };

    let rep = input.report;
    let mut verts = Vec::with_capacity(rep.pads_3d.len() * 48 + 128 + rep.dampers_3d.len() * 24);
    let mut idxs = Vec::with_capacity(rep.pads_3d.len() * 64 + 192 + rep.dampers_3d.len() * 36);

    // 1. Base Isolation Bearing Pads + Core Beacons at each column base
    for pad in &rep.pads_3d {
        let (col, sev) = if pad.buckling_sf >= 1.5 {
            ([0.14_f32, 0.84, 0.94, 0.92], 0.25_f32)
        } else if pad.buckling_sf >= 1.1 {
            ([1.0_f32, 0.68, 0.16, 0.94], 0.65_f32)
        } else {
            ([1.0_f32, 0.26, 0.24, 0.96], 0.95_f32)
        };
        let hw = pad.diameter_m * 0.55;
        let fp = [
            rot_xy(pad.pos_enu_m[0] - hw, pad.pos_enu_m[1] - hw),
            rot_xy(pad.pos_enu_m[0] + hw, pad.pos_enu_m[1] - hw),
            rot_xy(pad.pos_enu_m[0] + hw, pad.pos_enu_m[1] + hw),
            rot_xy(pad.pos_enu_m[0] - hw, pad.pos_enu_m[1] + hw),
        ];
        let z0 = pad.pos_enu_m[2] as f32;
        let z1 = z0 + pad.height_m as f32;
        push_box_prism_enu(&mut verts, &mut idxs, &fp, z0, z1, col, 14.0);
        let cxy = rot_xy(pad.pos_enu_m[0], pad.pos_enu_m[1]);
        push_octahedron_beacon_enu(
            &mut verts,
            &mut idxs,
            [cxy[0] as f32, cxy[1] as f32, z1 + 0.12],
            (pad.diameter_m as f32) * 0.28,
            col,
            sev,
            14.0,
        );
    }

    // 2. Perimeter Seismic Moat Clearance Ring around building base
    let m = &rep.moat_3d;
    let ow = m.inner_width_m * 0.5 + m.moat_gap_m;
    let od = m.inner_depth_m * 0.5 + m.moat_gap_m;
    let corners = [
        [m.center_x_m - ow, m.center_y_m - od, m.z_m],
        [m.center_x_m + ow, m.center_y_m - od, m.z_m],
        [m.center_x_m + ow, m.center_y_m + od, m.z_m],
        [m.center_x_m - ow, m.center_y_m + od, m.z_m],
    ];
    for i in 0..4 {
        let p0 = rot_enu(corners[i]);
        let p1 = rot_enu(corners[(i + 1) % 4]);
        push_animated_flow_strut_enu(
            &mut verts,
            &mut idxs,
            p0,
            p1,
            0.26,
            0.16,
            [1.0, 0.72, 0.18, 0.88],
            0.5,
            14.0,
        );
    }

    // 3. Perimeter Fluid Viscous Damper Struts
    for d in &rep.dampers_3d {
        push_animated_flow_strut_enu(
            &mut verts,
            &mut idxs,
            rot_enu(d.p0_enu_m),
            rot_enu(d.p1_enu_m),
            0.22,
            0.22,
            [0.24, 0.90, 0.62, 0.92],
            0.4,
            14.0,
        );
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Update `v0.8.4.2` Wind Aeroelasticity Facade Pressure Panels & Directional Flow Vectors (`material_kind = 15.0`).
pub struct WindGpuInput<'a> {
    pub report: &'a crate::core::wind_studio::WindStudioReport,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU facade cladding pressure panels (`Windward` cyan vs `Leeward/Sidewall` amber-crimson)
/// and animated directional wind flow vector shafts (`material_kind = 15.0`, 0% raster).
pub fn build_rtc_wind_mesh(input: &WindGpuInput<'_>) -> GpuMeshPayload {
    if !input.visible {
        return GpuMeshPayload::default();
    }
    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_xy = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };
    let rot_enu = |p: [f64; 3]| -> [f64; 3] {
        [p[0] * cr - p[1] * sr, p[0] * sr + p[1] * cr, p[2]]
    };

    let rep = input.report;
    let max_p = rep
        .panels_3d
        .iter()
        .map(|p| p.pressure_pa.abs())
        .fold(400.0_f64, f64::max);

    let mut verts = Vec::with_capacity(rep.panels_3d.len() * 24 + rep.arrows_3d.len() * 32);
    let mut idxs = Vec::with_capacity(rep.panels_3d.len() * 36 + rep.arrows_3d.len() * 48);

    // 1. Extruded facade cladding pressure slabs slightly offset outward along face normal
    for pan in &rep.panels_3d {
        let ratio = (pan.pressure_pa.abs() / max_p).clamp(0.15, 1.0) as f32;
        let col = if pan.is_windward {
            [0.10_f32, 0.72 + 0.24 * ratio, 1.0, 0.34 + 0.22 * ratio]
        } else if pan.face_label.contains("Leeward") {
            [1.0_f32, 0.32 + 0.16 * (1.0 - ratio), 0.22, 0.34 + 0.22 * ratio]
        } else {
            [1.0_f32, 0.68, 0.16, 0.28 + 0.18 * ratio]
        };

        let nx = pan.normal_xy[0];
        let ny = pan.normal_xy[1];
        let tx = -ny;
        let ty = nx;
        let offset = 0.22 + (ratio as f64) * 0.24;
        let cx = pan.center_enu_m[0] + nx * offset;
        let cy = pan.center_enu_m[1] + ny * offset;
        let hw = pan.width_m * 0.44;
        let th = 0.14;

        let fp = [
            rot_xy(cx - tx * hw - nx * th, cy - ty * hw - ny * th),
            rot_xy(cx + tx * hw - nx * th, cy + ty * hw - ny * th),
            rot_xy(cx + tx * hw + nx * th, cy + ty * hw + ny * th),
            rot_xy(cx - tx * hw + nx * th, cy - ty * hw + ny * th),
        ];
        let z0 = (pan.center_enu_m[2] - pan.height_m * 0.45) as f32;
        let z1 = (pan.center_enu_m[2] + pan.height_m * 0.45) as f32;
        push_box_prism_enu(&mut verts, &mut idxs, &fp, z0, z1, col, 15.0);
    }

    // 2. Directional inflow streamlines & leeward suction wake arrows
    for arr in &rep.arrows_3d {
        let p0 = rot_enu(arr.p0_enu_m);
        let p1 = rot_enu(arr.p1_enu_m);
        let (col, sev) = if arr.is_suction {
            ([1.0_f32, 0.44, 0.18, 0.88], 0.75_f32)
        } else {
            ([0.16_f32, 0.88, 1.0, 0.90], 0.55_f32)
        };
        push_animated_flow_strut_enu(&mut verts, &mut idxs, p0, p1, 0.24, 0.14, col, sev, 15.0);
        push_octahedron_beacon_enu(
            &mut verts,
            &mut idxs,
            [p1[0] as f32, p1[1] as f32, p1[2] as f32],
            0.34,
            col,
            sev,
            15.0,
        );
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Update `v0.8.4.3` Special RC Shear Wall SBE Zones & Diagonal X-Cage Coupling Beams (`material_kind = 16.0`).
pub struct ShearWallGpuInput<'a> {
    pub report: &'a crate::core::shear_wall_studio::ShearWallStudioReport,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU shear wall webs, end Special Boundary Element (SBE) plastic hinge zones,
/// and per-story crossed Diagonal X-Cage coupling beams (`material_kind = 16.0`, 0% raster).
pub fn build_rtc_shear_wall_mesh(input: &ShearWallGpuInput<'_>) -> GpuMeshPayload {
    if !input.visible {
        return GpuMeshPayload::default();
    }
    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_xy = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };
    let rot_enu = |p: [f64; 3]| -> [f64; 3] {
        [p[0] * cr - p[1] * sr, p[0] * sr + p[1] * cr, p[2]]
    };

    let rep = input.report;
    let mut verts = Vec::with_capacity(rep.piers_3d.len() * 96 + rep.coupling_beams_3d.len() * 64);
    let mut idxs = Vec::with_capacity(rep.piers_3d.len() * 144 + rep.coupling_beams_3d.len() * 96);

    // 1. Shear wall webs & end Special Boundary Element (SBE) confinement zones
    for pier in &rep.piers_3d {
        let dx = pier.p1_enu_m[0] - pier.p0_enu_m[0];
        let dy = pier.p1_enu_m[1] - pier.p0_enu_m[1];
        let len = dx.hypot(dy).max(0.5);
        let ux = dx / len;
        let uy = dy / len;
        let nx = -uy;
        let ny = ux;
        let ht = pier.thickness_m * 0.54;
        let z0 = pier.base_z_m as f32;
        let z_top = (pier.base_z_m + pier.height_m) as f32;
        let z_sbe = (pier.base_z_m + pier.sbe_plastic_height_m) as f32;

        // Translucent structural wall web envelope
        let fp_web = [
            rot_xy(pier.p0_enu_m[0] - nx * ht, pier.p0_enu_m[1] - ny * ht),
            rot_xy(pier.p1_enu_m[0] - nx * ht, pier.p1_enu_m[1] - ny * ht),
            rot_xy(pier.p1_enu_m[0] + nx * ht, pier.p1_enu_m[1] + ny * ht),
            rot_xy(pier.p0_enu_m[0] + nx * ht, pier.p0_enu_m[1] + ny * ht),
        ];
        push_box_prism_enu(
            &mut verts,
            &mut idxs,
            &fp_web,
            z0,
            z_top,
            [0.45, 0.72, 0.96, 0.34],
            16.0,
        );

        // Two end Special Boundary Element (SBE) confinement cages
        let sbe_col = if pier.is_compliant {
            [0.16_f32, 0.90, 0.60, 0.78]
        } else {
            [1.0_f32, 0.36, 0.24, 0.84]
        };
        let sbe_ht = ht * 1.18;
        let sbe_len = pier.sbe_length_m.min(len * 0.42);
        for (a_xy, b_xy) in [
            (
                pier.p0_enu_m,
                [pier.p0_enu_m[0] + ux * sbe_len, pier.p0_enu_m[1] + uy * sbe_len],
            ),
            (
                [pier.p1_enu_m[0] - ux * sbe_len, pier.p1_enu_m[1] - uy * sbe_len],
                pier.p1_enu_m,
            ),
        ] {
            let fp_sbe = [
                rot_xy(a_xy[0] - nx * sbe_ht, a_xy[1] - ny * sbe_ht),
                rot_xy(b_xy[0] - nx * sbe_ht, b_xy[1] - ny * sbe_ht),
                rot_xy(b_xy[0] + nx * sbe_ht, b_xy[1] + ny * sbe_ht),
                rot_xy(a_xy[0] + nx * sbe_ht, a_xy[1] + ny * sbe_ht),
            ];
            push_box_prism_enu(&mut verts, &mut idxs, &fp_sbe, z0, z_sbe, sbe_col, 16.0);
        }
    }

    // 2. Per-story Coupling Beams & Diagonal X-Cage reinforcement struts
    for cb in &rep.coupling_beams_3d {
        let dx = cb.p1_enu_m[0] - cb.p0_enu_m[0];
        let dy = cb.p1_enu_m[1] - cb.p0_enu_m[1];
        let len = dx.hypot(dy).max(0.4);
        let nx = -dy / len;
        let ny = dx / len;
        let hw = cb.width_m * 0.48;
        let z_top = cb.p0_enu_m[2] as f32;
        let z_bot = z_top - cb.depth_m as f32;
        let fp_cb = [
            rot_xy(cb.p0_enu_m[0] - nx * hw, cb.p0_enu_m[1] - ny * hw),
            rot_xy(cb.p1_enu_m[0] - nx * hw, cb.p1_enu_m[1] - ny * hw),
            rot_xy(cb.p1_enu_m[0] + nx * hw, cb.p1_enu_m[1] + ny * hw),
            rot_xy(cb.p0_enu_m[0] + nx * hw, cb.p0_enu_m[1] + ny * hw),
        ];
        push_box_prism_enu(
            &mut verts,
            &mut idxs,
            &fp_cb,
            z_bot,
            z_top,
            [0.95, 0.78, 0.24, 0.36],
            16.0,
        );

        let diag_col = if cb.coupling_dcr <= 1.0 {
            [0.14_f32, 0.86, 0.98, 0.92]
        } else {
            [1.0_f32, 0.28, 0.24, 0.94]
        };
        let z_d0 = cb.p0_enu_m[2] - cb.depth_m * 0.88;
        let z_d1 = cb.p0_enu_m[2] - cb.depth_m * 0.12;
        push_animated_flow_strut_enu(
            &mut verts,
            &mut idxs,
            rot_enu([cb.p0_enu_m[0], cb.p0_enu_m[1], z_d0]),
            rot_enu([cb.p1_enu_m[0], cb.p1_enu_m[1], z_d1]),
            0.16,
            0.16,
            diag_col,
            (cb.coupling_dcr as f32).clamp(0.1, 1.0),
            16.0,
        );
        if cb.is_diagonal_x_cage {
            push_animated_flow_strut_enu(
                &mut verts,
                &mut idxs,
                rot_enu([cb.p0_enu_m[0], cb.p0_enu_m[1], z_d1]),
                rot_enu([cb.p1_enu_m[0], cb.p1_enu_m[1], z_d0]),
                0.16,
                0.16,
                diag_col,
                (cb.coupling_dcr as f32).clamp(0.1, 1.0),
                16.0,
            );
        }
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Update `v0.8.4.4` Steel Seismic Connection Joint Plates, Panel Zones & Bolt Pins (`material_kind = 17.0`).
pub struct ConnectionGpuInput<'a> {
    pub report: &'a crate::core::connection_studio::ConnectionStudioReport,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU steel end-plate / shear-tab boxes, column panel-zone halos,
/// and high-strength bolt pin clusters (`material_kind = 17.0`, 0% raster).
pub fn build_rtc_connection_mesh(input: &ConnectionGpuInput<'_>) -> GpuMeshPayload {
    if !input.visible {
        return GpuMeshPayload::default();
    }
    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_xy = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };

    let rep = input.report;
    let mut verts = Vec::with_capacity(rep.joints_3d.len() * 48 + rep.bolts_3d.len() * 16);
    let mut idxs = Vec::with_capacity(rep.joints_3d.len() * 72 + rep.bolts_3d.len() * 24);

    // 1. Beam-column joint plates & panel zone envelopes
    for j in &rep.joints_3d {
        let (col, sev) = if !j.is_compliant {
            ([1.0_f32, 0.26, 0.24, 0.94], 0.95_f32)
        } else if j.doubler_required {
            ([1.0_f32, 0.70, 0.16, 0.90], 0.65_f32)
        } else {
            ([0.16_f32, 0.88, 0.96, 0.88], 0.30_f32)
        };
        let hw = j.plate_width_m * 0.65;
        let ht = j.plate_thick_m.max(0.08);
        let fp = [
            rot_xy(j.pos_enu_m[0] - ht, j.pos_enu_m[1] - hw),
            rot_xy(j.pos_enu_m[0] + ht, j.pos_enu_m[1] - hw),
            rot_xy(j.pos_enu_m[0] + ht, j.pos_enu_m[1] + hw),
            rot_xy(j.pos_enu_m[0] - ht, j.pos_enu_m[1] + hw),
        ];
        let z0 = (j.pos_enu_m[2] - j.plate_height_m * 0.5) as f32;
        let z1 = (j.pos_enu_m[2] + j.plate_height_m * 0.5) as f32;
        push_box_prism_enu(&mut verts, &mut idxs, &fp, z0, z1, col, 17.0);

        let cxy = rot_xy(j.pos_enu_m[0], j.pos_enu_m[1]);
        push_octahedron_beacon_enu(
            &mut verts,
            &mut idxs,
            [cxy[0] as f32, cxy[1] as f32, j.pos_enu_m[2] as f32],
            0.22,
            col,
            sev,
            17.0,
        );
    }

    // 2. High-strength bolt head beacons around primary joints
    for b in &rep.bolts_3d {
        let col = if b.is_tension_zone {
            [0.24_f32, 0.92, 1.0, 0.96]
        } else {
            [0.98_f32, 0.82, 0.24, 0.94]
        };
        let cxy = rot_xy(b.pos_enu_m[0], b.pos_enu_m[1]);
        push_octahedron_beacon_enu(
            &mut verts,
            &mut idxs,
            [cxy[0] as f32, cxy[1] as f32, b.pos_enu_m[2] as f32],
            (b.diameter_m as f32 * 2.2).clamp(0.05, 0.12),
            col,
            0.5,
            17.0,
        );
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Update `v0.8.4.5` Post-Tensioned Girder Tendon Drape & Anchorage Blocks (`material_kind = 18.0`).
pub struct PrestressGpuInput<'a> {
    pub report: &'a crate::core::prestress_studio::PrestressStudioReport,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU girder envelopes, parabolic/harped post-tensioning tendon ducts,
/// and live/dead anchorage blocks (`material_kind = 18.0`, 0% raster).
pub fn build_rtc_prestress_mesh(input: &PrestressGpuInput<'_>) -> GpuMeshPayload {
    if !input.visible {
        return GpuMeshPayload::default();
    }
    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_xy = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };
    let rot_enu = |p: [f64; 3]| -> [f64; 3] {
        [p[0] * cr - p[1] * sr, p[0] * sr + p[1] * cr, p[2]]
    };

    let rep = input.report;
    let mut verts = Vec::with_capacity(rep.bays_3d.len() * 140 + rep.anchorages_3d.len() * 16);
    let mut idxs = Vec::with_capacity(rep.bays_3d.len() * 210 + rep.anchorages_3d.len() * 24);

    for bay in &rep.bays_3d {
        let dx = bay.p1_enu_m[0] - bay.p0_enu_m[0];
        let dy = bay.p1_enu_m[1] - bay.p0_enu_m[1];
        let len = dx.hypot(dy).max(0.5);
        let nx = -dy / len;
        let ny = dx / len;
        let hw = bay.width_m * 0.45;
        let z_top = bay.p0_enu_m[2] as f32;
        let z_bot = z_top - bay.depth_m as f32;
        let fp = [
            rot_xy(bay.p0_enu_m[0] - nx * hw, bay.p0_enu_m[1] - ny * hw),
            rot_xy(bay.p1_enu_m[0] - nx * hw, bay.p1_enu_m[1] - ny * hw),
            rot_xy(bay.p1_enu_m[0] + nx * hw, bay.p1_enu_m[1] + ny * hw),
            rot_xy(bay.p0_enu_m[0] + nx * hw, bay.p0_enu_m[1] + ny * hw),
        ];
        push_box_prism_enu(
            &mut verts,
            &mut idxs,
            &fp,
            z_bot,
            z_top,
            [0.56, 0.78, 0.96, 0.24],
            18.0,
        );

        let tendon_col = if bay.is_compliant {
            [1.0_f32, 0.78, 0.18, 0.95]
        } else {
            [1.0_f32, 0.30, 0.24, 0.95]
        };
        for seg in bay.tendon_pts_enu_m.windows(2) {
            push_animated_flow_strut_enu(
                &mut verts,
                &mut idxs,
                rot_enu(seg[0]),
                rot_enu(seg[1]),
                0.14,
                0.14,
                tendon_col,
                0.75,
                18.0,
            );
        }
    }

    for anc in &rep.anchorages_3d {
        let col = if anc.is_live_end {
            [0.14_f32, 0.90, 1.0, 0.96]
        } else {
            [0.98_f32, 0.56, 0.16, 0.94]
        };
        let p = rot_enu(anc.pos_enu_m);
        push_octahedron_beacon_enu(
            &mut verts,
            &mut idxs,
            [p[0] as f32, p[1] as f32, p[2] as f32],
            0.28,
            col,
            0.85,
            18.0,
        );
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Update `v0.8.4.6` Composite Deck Strips & Headed Shear Studs (`material_kind = 19.0`).
pub struct CompositeGpuInput<'a> {
    pub report: &'a crate::core::composite_studio::CompositeStudioReport,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU composite concrete-metal deck strips and headed shear stud pins
/// (`material_kind = 19.0`, 0% raster).
pub fn build_rtc_composite_mesh(input: &CompositeGpuInput<'_>) -> GpuMeshPayload {
    if !input.visible {
        return GpuMeshPayload::default();
    }
    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_xy = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };

    let rep = input.report;
    let mut verts = Vec::with_capacity(rep.bays_3d.len() * 32 + rep.studs_3d.len() * 16);
    let mut idxs = Vec::with_capacity(rep.bays_3d.len() * 48 + rep.studs_3d.len() * 24);

    for bay in &rep.bays_3d {
        let dx = bay.p1_enu_m[0] - bay.p0_enu_m[0];
        let dy = bay.p1_enu_m[1] - bay.p0_enu_m[1];
        let len = dx.hypot(dy).max(0.5);
        let nx = -dy / len;
        let ny = dx / len;
        let hw = bay.eff_width_m * 0.45;
        let z0 = bay.p0_enu_m[2] as f32;
        let z1 = z0 + bay.slab_thick_m as f32;
        let col = if bay.is_compliant {
            [0.14_f32, 0.84, 0.72, 0.36]
        } else {
            [1.0_f32, 0.38, 0.24, 0.44]
        };
        let fp = [
            rot_xy(bay.p0_enu_m[0] - nx * hw, bay.p0_enu_m[1] - ny * hw),
            rot_xy(bay.p1_enu_m[0] - nx * hw, bay.p1_enu_m[1] - ny * hw),
            rot_xy(bay.p1_enu_m[0] + nx * hw, bay.p1_enu_m[1] + ny * hw),
            rot_xy(bay.p0_enu_m[0] + nx * hw, bay.p0_enu_m[1] + ny * hw),
        ];
        push_box_prism_enu(&mut verts, &mut idxs, &fp, z0, z1, col, 19.0);
    }

    for st in &rep.studs_3d {
        let cxy = rot_xy(st.pos_enu_m[0], st.pos_enu_m[1]);
        let col = if st.is_double_rib {
            [1.0_f32, 0.78, 0.16, 0.96]
        } else {
            [0.24_f32, 0.92, 1.0, 0.94]
        };
        push_octahedron_beacon_enu(
            &mut verts,
            &mut idxs,
            [cxy[0] as f32, cxy[1] as f32, st.pos_enu_m[2] as f32],
            (st.height_m as f32 * 0.8).clamp(0.08, 0.16),
            col,
            0.55,
            19.0,
        );
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Update `v0.8.4.7` Structural Reliability `FORM/SORM` Member Beta & Hotspots (`material_kind = 20.0`).
pub struct ReliabilityGpuInput<'a> {
    pub report: &'a crate::core::reliability_studio::ReliabilityStudioReport,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU structural reliability member contours & critical beta hotspot beacons
/// (`material_kind = 20.0`, 0% raster).
pub fn build_rtc_reliability_mesh(input: &ReliabilityGpuInput<'_>) -> GpuMeshPayload {
    if !input.visible {
        return GpuMeshPayload::default();
    }
    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_xy = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };

    let rep = input.report;
    let target = rep.config.target_beta;
    let mut verts = Vec::with_capacity(rep.members_3d.len() * 24 + rep.hotspots_3d.len() * 16);
    let mut idxs = Vec::with_capacity(rep.members_3d.len() * 36 + rep.hotspots_3d.len() * 24);

    for m in &rep.members_3d {
        let p0_xy = rot_xy(m.p0_enu_m[0], m.p0_enu_m[1]);
        let p1_xy = rot_xy(m.p1_enu_m[0], m.p1_enu_m[1]);
        let (col, severity) = if m.is_below_target {
            ([1.0_f32, 0.22, 0.28, 0.92], 0.95_f32)
        } else if m.beta_index < target + 0.5 {
            ([1.0_f32, 0.72, 0.16, 0.86], 0.55_f32)
        } else {
            ([0.16_f32, 0.88, 0.68, 0.78], 0.25_f32)
        };
        push_animated_flow_strut_enu(
            &mut verts,
            &mut idxs,
            [p0_xy[0], p0_xy[1], m.p0_enu_m[2]],
            [p1_xy[0], p1_xy[1], m.p1_enu_m[2]],
            0.22,
            0.22,
            col,
            severity,
            20.0,
        );
    }

    for hs in &rep.hotspots_3d {
        let cxy = rot_xy(hs.pos_enu_m[0], hs.pos_enu_m[1]);
        let (col, severity) = if hs.is_critical {
            ([1.0_f32, 0.20, 0.26, 0.96], 0.98_f32)
        } else {
            ([1.0_f32, 0.76, 0.18, 0.92], 0.58_f32)
        };
        push_octahedron_beacon_enu(
            &mut verts,
            &mut idxs,
            [cxy[0] as f32, cxy[1] as f32, hs.pos_enu_m[2] as f32],
            if hs.is_critical { 0.42 } else { 0.28 },
            col,
            severity,
            20.0,
        );
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Update `v0.8.4.8` Deep Foundation Pile Group, Pile Caps & Winkler Springs (`material_kind = 21.0`).
pub struct FoundationGpuInput<'a> {
    pub report: &'a crate::core::foundation_studio::FoundationStudioReport,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU deep foundation pile caps, subsurface pile shafts, and 6-DOF Winkler spring nodes
/// (`material_kind = 21.0`, 0% raster).
pub fn build_rtc_foundation_mesh(input: &FoundationGpuInput<'_>) -> GpuMeshPayload {
    if !input.visible {
        return GpuMeshPayload::default();
    }
    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_xy = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };

    let rep = input.report;
    let mut verts = Vec::with_capacity(
        rep.caps_3d.len() * 32 + rep.piles_3d.len() * 24 + rep.springs_3d.len() * 16,
    );
    let mut idxs = Vec::with_capacity(
        rep.caps_3d.len() * 48 + rep.piles_3d.len() * 36 + rep.springs_3d.len() * 24,
    );

    for cap in &rep.caps_3d {
        let hw = cap.width_m * 0.5;
        let hl = cap.length_m * 0.5;
        let cx = cap.center_enu_m[0];
        let cy = cap.center_enu_m[1];
        let z0 = (cap.center_enu_m[2] - cap.thickness_m * 0.5) as f32;
        let z1 = (cap.center_enu_m[2] + cap.thickness_m * 0.5) as f32;
        let col = if cap.is_compliant {
            [0.18_f32, 0.78, 0.56, 0.44]
        } else {
            [1.0_f32, 0.28, 0.24, 0.52]
        };
        let fp = [
            rot_xy(cx - hw, cy - hl),
            rot_xy(cx + hw, cy - hl),
            rot_xy(cx + hw, cy + hl),
            rot_xy(cx - hw, cy + hl),
        ];
        push_box_prism_enu(&mut verts, &mut idxs, &fp, z0, z1, col, 21.0);
    }

    for pile in &rep.piles_3d {
        let top_xy = rot_xy(pile.top_enu_m[0], pile.top_enu_m[1]);
        let tip_xy = rot_xy(pile.tip_enu_m[0], pile.tip_enu_m[1]);
        let (col, severity) = if !pile.is_safe {
            ([1.0_f32, 0.20, 0.26, 0.94], 0.95_f32)
        } else if pile.is_uplift {
            ([0.22_f32, 0.68, 1.0, 0.88], 0.60_f32)
        } else {
            ([0.14_f32, 0.86, 0.74, 0.84], 0.35_f32)
        };
        push_animated_flow_strut_enu(
            &mut verts,
            &mut idxs,
            [top_xy[0], top_xy[1], pile.top_enu_m[2]],
            [tip_xy[0], tip_xy[1], pile.tip_enu_m[2]],
            pile.diameter_m,
            pile.diameter_m,
            col,
            severity,
            21.0,
        );
    }

    for sp in &rep.springs_3d {
        let cxy = rot_xy(sp.pos_enu_m[0], sp.pos_enu_m[1]);
        push_octahedron_beacon_enu(
            &mut verts,
            &mut idxs,
            [cxy[0] as f32, cxy[1] as f32, sp.pos_enu_m[2] as f32],
            0.24,
            [0.98_f32, 0.78, 0.22, 0.90],
            0.50,
            21.0,
        );
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Update `v0.8.4.9` 4D/5D/6D Construction Sequence & Slewing Crane (`material_kind = 22.0`).
pub struct ConstructionLcaGpuInput<'a> {
    pub report: &'a crate::core::construction_lca_studio::ConstructionLcaStudioReport,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU 4D story progress prisms & slewing tower crane mast/jib
/// (`material_kind = 22.0`, 0% raster).
pub fn build_rtc_construction_lca_mesh(input: &ConstructionLcaGpuInput<'_>) -> GpuMeshPayload {
    if !input.visible {
        return GpuMeshPayload::default();
    }
    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_xy = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };

    let rep = input.report;
    let mut verts = Vec::with_capacity(rep.story_zones_3d.len() * 32 + 64);
    let mut idxs = Vec::with_capacity(rep.story_zones_3d.len() * 48 + 96);

    for z in &rep.story_zones_3d {
        let ratio = (z.completion_pct / 100.0).clamp(0.18, 1.0);
        let z0 = z.min_enu_m[2] as f32;
        let z1 = z0 + ((z.max_enu_m[2] - z.min_enu_m[2]) * ratio) as f32;
        let col = if z.completion_pct >= 99.5 {
            [0.14_f32, 0.84, 0.62, 0.42]
        } else if z.completion_pct > 0.5 {
            [1.0_f32, 0.68, 0.14, 0.56]
        } else {
            [0.22_f32, 0.64, 1.0, 0.22]
        };
        let fp = [
            rot_xy(z.min_enu_m[0], z.min_enu_m[1]),
            rot_xy(z.max_enu_m[0], z.min_enu_m[1]),
            rot_xy(z.max_enu_m[0], z.max_enu_m[1]),
            rot_xy(z.min_enu_m[0], z.max_enu_m[1]),
        ];
        push_box_prism_enu(&mut verts, &mut idxs, &fp, z0, z1, col, 22.0);
    }

    // Tower crane mast & slewing jib
    let c = &rep.crane_3d;
    let bxy = rot_xy(c.base_enu_m[0], c.base_enu_m[1]);
    let mast_top = [bxy[0], bxy[1], c.base_enu_m[2] + c.mast_height_m];
    let jib_ang = (c.slew_deg + input.heading_deg).to_radians();
    let jib_tip = [
        bxy[0] + c.jib_length_m * jib_ang.cos(),
        bxy[1] + c.jib_length_m * jib_ang.sin(),
        mast_top[2],
    ];
    let crane_col = [1.0_f32, 0.82, 0.12, 0.94];
    push_animated_flow_strut_enu(
        &mut verts,
        &mut idxs,
        [bxy[0], bxy[1], c.base_enu_m[2]],
        mast_top,
        0.9,
        0.9,
        crane_col,
        0.65,
        22.0,
    );
    push_animated_flow_strut_enu(
        &mut verts,
        &mut idxs,
        mast_top,
        jib_tip,
        0.7,
        0.7,
        crane_col,
        0.80,
        22.0,
    );
    push_octahedron_beacon_enu(
        &mut verts,
        &mut idxs,
        [jib_tip[0] as f32, jib_tip[1] as f32, (jib_tip[2] - 2.2) as f32],
        0.55,
        [1.0_f32, 0.42, 0.16, 0.96],
        0.90,
        22.0,
    );

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Update `v0.8.5.6` Site Zoning & 3D Buildable Envelope + Sun Path (`material_kind = 23.0`).
pub struct ZoningEnvelopeGpuInput<'a> {
    pub report: &'a crate::core::zoning_agent::SiteZoningReport,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU Site Zoning Buildable Envelope prisms, wireframe setback cages,
/// and analytical diurnal Sun Path arc + active solar beacon (`material_kind = 23.0`, 0% raster).
pub fn build_rtc_zoning_envelope_mesh(input: &ZoningEnvelopeGpuInput<'_>) -> GpuMeshPayload {
    if !input.visible {
        return GpuMeshPayload::default();
    }
    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_xy = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };
    let rot_3 = |p: [f64; 3]| -> [f64; 3] {
        let xy = rot_xy(p[0], p[1]);
        [xy[0], xy[1], p[2]]
    };

    let rep = input.report;
    let mut verts = Vec::with_capacity(rep.envelope_prisms_3d.len() * 64 + 128);
    let mut idxs = Vec::with_capacity(rep.envelope_prisms_3d.len() * 96 + 192);

    for p in &rep.envelope_prisms_3d {
        let (fill_col, edge_col) = match p.tier_kind {
            0 => (
                [0.24_f32, 0.42, 0.56, 0.34],
                [0.36_f32, 0.78, 0.96, 0.88],
            ),
            1 => (
                [0.14_f32, 0.72, 0.88, 0.24],
                [0.26_f32, 0.88, 1.00, 0.92],
            ),
            2 => (
                [0.16_f32, 0.84, 0.64, 0.20],
                [0.22_f32, 0.95, 0.72, 0.94],
            ),
            _ => (
                [0.94_f32, 0.24, 0.24, 0.26],
                [1.00_f32, 0.36, 0.32, 0.96],
            ),
        };
        let fp = [
            rot_xy(p.min_enu_m[0], p.min_enu_m[1]),
            rot_xy(p.max_enu_m[0], p.min_enu_m[1]),
            rot_xy(p.max_enu_m[0], p.max_enu_m[1]),
            rot_xy(p.min_enu_m[0], p.max_enu_m[1]),
        ];
        let z0 = p.min_enu_m[2] as f32;
        let z1 = p.max_enu_m[2] as f32;
        push_box_prism_enu(&mut verts, &mut idxs, &fp, z0, z1, fill_col, 23.0);

        let corners_bot = [
            [fp[0][0], fp[0][1], p.min_enu_m[2]],
            [fp[1][0], fp[1][1], p.min_enu_m[2]],
            [fp[2][0], fp[2][1], p.min_enu_m[2]],
            [fp[3][0], fp[3][1], p.min_enu_m[2]],
        ];
        let corners_top = [
            [fp[0][0], fp[0][1], p.max_enu_m[2]],
            [fp[1][0], fp[1][1], p.max_enu_m[2]],
            [fp[2][0], fp[2][1], p.max_enu_m[2]],
            [fp[3][0], fp[3][1], p.max_enu_m[2]],
        ];
        let edge_r = if p.tier_kind == 0 { 0.16 } else { 0.22 };
        for i in 0..4 {
            let next = (i + 1) % 4;
            push_animated_flow_strut_enu(
                &mut verts,
                &mut idxs,
                corners_top[i],
                corners_top[next],
                edge_r,
                edge_r,
                edge_col,
                0.75,
                23.0,
            );
            if p.tier_kind > 0 {
                push_animated_flow_strut_enu(
                    &mut verts,
                    &mut idxs,
                    corners_bot[i],
                    corners_top[i],
                    edge_r,
                    edge_r,
                    edge_col,
                    0.65,
                    23.0,
                );
            }
        }
    }

    if rep.config.show_sun_path && rep.sun_path_arc_3d.len() >= 2 {
        let arc_col = [1.0_f32, 0.80, 0.24, 0.90];
        for win in rep.sun_path_arc_3d.windows(2) {
            push_animated_flow_strut_enu(
                &mut verts,
                &mut idxs,
                rot_3(win[0].enu_pos_m),
                rot_3(win[1].enu_pos_m),
                0.35,
                0.35,
                arc_col,
                0.85,
                23.0,
            );
        }
        let sun_p = rot_3(rep.active_sun_pos_3d);
        push_octahedron_beacon_enu(
            &mut verts,
            &mut idxs,
            [sun_p[0] as f32, sun_p[1] as f32, sun_p[2] as f32],
            2.1,
            [1.0_f32, 0.88, 0.28, 0.98],
            0.95,
            23.0,
        );
        push_animated_flow_strut_enu(
            &mut verts,
            &mut idxs,
            sun_p,
            [0.0, 0.0, 0.5],
            0.18,
            0.18,
            [1.0_f32, 0.84, 0.36, 0.42],
            0.60,
            23.0,
        );
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Update `v0.8.5.7` Spatial Program Stacked & Packed 3D Space Blocks (`material_kind = 24.0`).
pub struct SpatialProgramGpuInput<'a> {
    pub report: &'a crate::core::program_agent::SpatialProgramReport,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU department-colored stacked & packed space blocks (`material_kind = 24.0`, 0% raster).
pub fn build_rtc_spatial_program_mesh(input: &SpatialProgramGpuInput<'_>) -> GpuMeshPayload {
    if !input.visible {
        return GpuMeshPayload::default();
    }
    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_xy = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };

    let rep = input.report;
    let mut verts = Vec::with_capacity(rep.packed_blocks_3d.len() * 48);
    let mut idxs = Vec::with_capacity(rep.packed_blocks_3d.len() * 72);

    for blk in &rep.packed_blocks_3d {
        let is_focused = rep
            .config
            .selected_story_filter
            .is_none_or(|f| f == blk.story_index);
        let mut col = blk.color_rgba;
        if !is_focused {
            col[3] = 0.14;
        }
        let fp = [
            rot_xy(blk.min_enu_m[0], blk.min_enu_m[1]),
            rot_xy(blk.max_enu_m[0], blk.min_enu_m[1]),
            rot_xy(blk.max_enu_m[0], blk.max_enu_m[1]),
            rot_xy(blk.min_enu_m[0], blk.max_enu_m[1]),
        ];
        let z0 = blk.min_enu_m[2] as f32;
        let z1 = blk.max_enu_m[2] as f32;
        push_box_prism_enu(&mut verts, &mut idxs, &fp, z0, z1, col, 24.0);

        if is_focused {
            let rim_col = [
                (col[0] + 0.18).min(1.0),
                (col[1] + 0.18).min(1.0),
                (col[2] + 0.18).min(1.0),
                0.88,
            ];
            let top_pts = [
                [fp[0][0], fp[0][1], blk.max_enu_m[2]],
                [fp[1][0], fp[1][1], blk.max_enu_m[2]],
                [fp[2][0], fp[2][1], blk.max_enu_m[2]],
                [fp[3][0], fp[3][1], blk.max_enu_m[2]],
            ];
            for i in 0..4 {
                let next = (i + 1) % 4;
                push_animated_flow_strut_enu(
                    &mut verts,
                    &mut idxs,
                    top_pts[i],
                    top_pts[next],
                    0.09,
                    0.09,
                    rim_col,
                    0.55,
                    24.0,
                );
            }
        }
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Update `v0.8.5.8` Multi-Option Massing Explorer (`material_kind = 25.0`).
pub struct MassingStudyGpuInput<'a> {
    pub report: &'a crate::core::massing_agent::MassingStudyReport,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU architectural massing tiers, story floor-plate ribbons, shear-wall core,
/// optional structural grid preview struts, and roof EigenGate ARI beacon (`material_kind = 25.0`, 0% raster).
pub fn build_rtc_massing_study_mesh(input: &MassingStudyGpuInput<'_>) -> GpuMeshPayload {
    if !input.visible {
        return GpuMeshPayload::default();
    }
    let Some(opt) = input.report.active_option() else {
        return GpuMeshPayload::default();
    };
    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_xy = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };

    let mut verts = Vec::with_capacity(opt.blocks_3d.len() * 64 + opt.stories * 32);
    let mut idxs = Vec::with_capacity(opt.blocks_3d.len() * 96 + opt.stories * 48);
    let story_h = input.report.story_height_m.clamp(2.8, 6.0);

    for blk in &opt.blocks_3d {
        let fp = [
            rot_xy(blk.min_enu_m[0], blk.min_enu_m[1]),
            rot_xy(blk.max_enu_m[0], blk.min_enu_m[1]),
            rot_xy(blk.max_enu_m[0], blk.max_enu_m[1]),
            rot_xy(blk.min_enu_m[0], blk.max_enu_m[1]),
        ];
        let z0 = blk.min_enu_m[2] as f32;
        let z1 = blk.max_enu_m[2] as f32;
        push_box_prism_enu(&mut verts, &mut idxs, &fp, z0, z1, blk.color_rgba, 25.0);

        let rim_col = if blk.is_core {
            [0.92, 0.96, 1.0, 0.90]
        } else {
            [
                (blk.color_rgba[0] + 0.22).min(1.0),
                (blk.color_rgba[1] + 0.22).min(1.0),
                (blk.color_rgba[2] + 0.22).min(1.0),
                0.86,
            ]
        };

        // Render clean horizontal floor-plate outline ribbons at every story slice inside the block
        let end_s = if blk.is_core { opt.stories } else { blk.end_story };
        let start_s = if blk.is_core { 1 } else { blk.start_story + 1 };
        for s in start_s..=end_s {
            let sz = (s as f64) * story_h;
            if sz > blk.max_enu_m[2] + 0.05 {
                break;
            }
            let pts = [
                [fp[0][0], fp[0][1], sz],
                [fp[1][0], fp[1][1], sz],
                [fp[2][0], fp[2][1], sz],
                [fp[3][0], fp[3][1], sz],
            ];
            for i in 0..4 {
                let next = (i + 1) % 4;
                push_animated_flow_strut_enu(
                    &mut verts,
                    &mut idxs,
                    pts[i],
                    pts[next],
                    if s == end_s { 0.14 } else { 0.07 },
                    if s == end_s { 0.14 } else { 0.07 },
                    rim_col,
                    0.45 + (s as f32) * 0.05,
                    25.0,
                );
            }
        }
    }

    // Optional structural grid preview columns along base bay grid
    if input.report.config.show_structural_grid_preview {
        let bay_x = input.report.config.bay_x_m.clamp(4.8, 10.8);
        let bay_y = input.report.config.bay_y_m.clamp(4.8, 10.8);
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for b in opt.blocks_3d.iter().filter(|b| !b.is_core && b.start_story == 0) {
            min_x = min_x.min(b.min_enu_m[0]);
            max_x = max_x.max(b.max_enu_m[0]);
            min_y = min_y.min(b.min_enu_m[1]);
            max_y = max_y.max(b.max_enu_m[1]);
        }
        if min_x.is_finite() {
            let nx = (((max_x - min_x) / bay_x).round() as usize).clamp(1, 12);
            let ny = (((max_y - min_y) / bay_y).round() as usize).clamp(1, 12);
            let col_col = [0.95, 0.97, 1.0, 0.65];
            let podium_top = (opt.podium_stories.max(1) as f64) * story_h;
            for ix in 0..=nx {
                let xm = min_x + (ix as f64) * ((max_x - min_x) / (nx as f64));
                for iy in 0..=ny {
                    let ym = min_y + (iy as f64) * ((max_y - min_y) / (ny as f64));
                    let p_rot = rot_xy(xm, ym);
                    push_animated_flow_strut_enu(
                        &mut verts,
                        &mut idxs,
                        [p_rot[0], p_rot[1], 0.0],
                        [p_rot[0], p_rot[1], podium_top],
                        0.12,
                        0.12,
                        col_col,
                        0.3,
                        25.0,
                    );
                }
            }
        }
    }

    // Roof crown EigenGate ARI Beacon
    let ari_col = if opt.estimated_eigengate_ari >= 0.78 {
        [0.13, 0.84, 0.56, 0.95]
    } else if opt.estimated_eigengate_ari >= 0.65 {
        [0.98, 0.72, 0.16, 0.95]
    } else {
        [0.94, 0.30, 0.30, 0.95]
    };
    let crown_z = (opt.total_height_m + 3.2) as f32;
    push_octahedron_beacon_enu(&mut verts, &mut idxs, [0.0, 0.0, crown_z], 1.15, ari_col, 0.45, 25.0);

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Update `v0.8.6.1` 3D Solar Irradiation, Shadow & OTTV Heatmap (`material_kind = 26.0`).
pub struct SolarEnvelopeGpuInput<'a> {
    pub report: &'a crate::core::solar_envelope::SolarEnvelopeReport,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU thermal irradiance facade/roof heatmap panels, cast ground shadows,
/// and incoming solar vector beam (`material_kind = 26.0`, 0% raster).
pub fn build_rtc_solar_envelope_mesh(input: &SolarEnvelopeGpuInput<'_>) -> GpuMeshPayload {
    if !input.visible {
        return GpuMeshPayload::default();
    }
    let rep = input.report;
    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_xy = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };
    let rot_n = |n: [f32; 3]| -> [f32; 3] {
        let nx = (n[0] as f64) * cr - (n[1] as f64) * sr;
        let ny = (n[0] as f64) * sr + (n[1] as f64) * cr;
        [nx as f32, ny as f32, n[2]]
    };

    let mut verts = Vec::with_capacity(rep.heatmap_panels_3d.len() * 4 + 32);
    let mut idxs = Vec::with_capacity(rep.heatmap_panels_3d.len() * 6 + 48);

    for p in &rep.heatmap_panels_3d {
        let base = verts.len() as u32;
        let norm = rot_n(p.normal_enu);
        let phase = (p.story_index as f32) * 0.25 + p.intensity_norm;
        for c in p.corners_enu_m {
            let rxy = rot_xy(c[0], c[1]);
            verts.push(GpuEarthVertex {
                pos: [rxy[0] as f32, rxy[1] as f32, c[2] as f32],
                normal: norm,
                color: p.color_rgba,
                meta: [26.0, p.intensity_norm, 1.0, phase],
            });
        }
        idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    if rep.config.show_shadow_polygon {
        let shadow_col = [0.05_f32, 0.08, 0.14, 0.52];
        for sq in &rep.shadow_quads_3d {
            let base = verts.len() as u32;
            for c in sq.corners_enu_m {
                let rxy = rot_xy(c[0], c[1]);
                verts.push(GpuEarthVertex {
                    pos: [rxy[0] as f32, rxy[1] as f32, c[2] as f32],
                    normal: [0.0, 0.0, 1.0],
                    color: shadow_col,
                    meta: [26.0, 0.05, 1.0, 0.0],
                });
            }
            idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    // Incoming solar beam indicator toward building crown
    if rep.sun_altitude_deg > 2.5 {
        let beam_start = [
            rep.sun_dir_enu[0] * 38.0,
            rep.sun_dir_enu[1] * 38.0,
            rep.building_height_m * 0.6 + rep.sun_dir_enu[2] * 38.0,
        ];
        let b0 = rot_xy(beam_start[0], beam_start[1]);
        let roof_mid = [
            0.5 * (rep.min_enu_xy_m[0] + rep.max_enu_xy_m[0]),
            0.5 * (rep.min_enu_xy_m[1] + rep.max_enu_xy_m[1]),
        ];
        let rc = rot_xy(roof_mid[0], roof_mid[1]);
        push_animated_flow_strut_enu(
            &mut verts,
            &mut idxs,
            [b0[0], b0[1], beam_start[2]],
            [rc[0], rc[1], rep.building_height_m + 0.5],
            0.22,
            0.22,
            [1.0, 0.85, 0.25, 0.90],
            0.8,
            26.0,
        );
        push_octahedron_beacon_enu(
            &mut verts,
            &mut idxs,
            [b0[0] as f32, b0[1] as f32, beam_start[2] as f32],
            1.4,
            [1.0, 0.88, 0.30, 0.95],
            0.65,
            26.0,
        );
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Update `v0.8.6.4` Interior Daylight Autonomy & Work-Plane Lux Grid (`material_kind = 27.0`).
pub struct DaylightInteriorGpuInput<'a> {
    pub report: &'a crate::core::daylight_interior::DaylightInteriorReport,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU interior work-plane lux heatmap cells, perimeter rails,
/// and a LEED v4.1 daylight credit status beacon (`material_kind = 27.0`, 0% raster).
pub fn build_rtc_daylight_interior_mesh(input: &DaylightInteriorGpuInput<'_>) -> GpuMeshPayload {
    if !input.visible {
        return GpuMeshPayload::default();
    }
    let rep = input.report;
    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_xy = |x: f64, y: f64| -> [f64; 2] { [x * cr - y * sr, x * sr + y * cr] };

    let mut verts = Vec::with_capacity(rep.cells_3d.len() * 4 + 48);
    let mut idxs = Vec::with_capacity(rep.cells_3d.len() * 6 + 64);

    for cell in &rep.cells_3d {
        let base = verts.len() as u32;
        let norm_lux = (cell.mean_lux / 1200.0).clamp(0.05, 1.0) as f32;
        let phase = (cell.ix as f32) * 0.22 + (cell.iy as f32) * 0.17;
        for c in cell.corners_enu_m {
            let rxy = rot_xy(c[0], c[1]);
            verts.push(GpuEarthVertex {
                pos: [rxy[0] as f32, rxy[1] as f32, c[2] as f32],
                normal: [0.0, 0.0, 1.0],
                color: cell.color_rgba,
                meta: [27.0, norm_lux, 1.0, phase],
            });
        }
        idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let wz = rep.story_elev_m + rep.config.work_plane_height_m + 0.04;
    let corners = [
        [rep.min_enu_xy_m[0], rep.min_enu_xy_m[1]],
        [rep.max_enu_xy_m[0], rep.min_enu_xy_m[1]],
        [rep.max_enu_xy_m[0], rep.max_enu_xy_m[1]],
        [rep.min_enu_xy_m[0], rep.max_enu_xy_m[1]],
    ];
    let rim_col = if rep.en_17037_compliant {
        [0.16, 0.88, 0.56, 0.90]
    } else {
        [0.98, 0.68, 0.18, 0.90]
    };
    for i in 0..4 {
        let p0 = rot_xy(corners[i][0], corners[i][1]);
        let p1 = rot_xy(corners[(i + 1) % 4][0], corners[(i + 1) % 4][1]);
        push_animated_flow_strut_enu(
            &mut verts,
            &mut idxs,
            [p0[0], p0[1], wz],
            [p1[0], p1[1], wz],
            0.12,
            0.12,
            rim_col,
            0.4,
            27.0,
        );
    }

    let mid = rot_xy(
        0.5 * (rep.min_enu_xy_m[0] + rep.max_enu_xy_m[0]),
        0.5 * (rep.min_enu_xy_m[1] + rep.max_enu_xy_m[1]),
    );
    push_octahedron_beacon_enu(
        &mut verts,
        &mut idxs,
        [mid[0] as f32, mid[1] as f32, (wz + 1.45) as f32],
        0.95,
        rim_col,
        0.65,
        27.0,
    );

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Update `v0.8.7.5` (`5/10` -> `v0.8.8`) 3D Building MEP Conduits, Risers & Plant (`material_kind = 31.0`).
#[derive(Clone, Copy)]
pub struct MepSystemsGpuInput<'a> {
    pub report: &'a crate::core::mep_engine::MepSystemsReport,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU MEP ducts, plumbing headers, cable trays, NFPA 13 sprinkler mains,
/// vertical core risers, and rooftop/ground plant equipment (`material_kind = 31.0`, 0% raster).
pub fn build_rtc_mep_systems_mesh(input: &MepSystemsGpuInput<'_>) -> GpuMeshPayload {
    use crate::core::mep_engine::MepDiscipline;

    if !input.visible {
        return GpuMeshPayload::default();
    }
    let rep = input.report;
    let cfg = &rep.config;
    let is_disc_visible = |disc: MepDiscipline| match disc {
        MepDiscipline::Hvac => cfg.show_hvac,
        MepDiscipline::Plumbing => cfg.show_plumbing,
        MepDiscipline::Electrical => cfg.show_electrical,
        MepDiscipline::FireSprinkler => cfg.show_fire,
    };

    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());
    let rot_enu = |p: [f64; 3]| -> [f64; 3] {
        [p[0] * cr - p[1] * sr, p[0] * sr + p[1] * cr, p[2]]
    };

    let mut verts = Vec::with_capacity(2_048);
    let mut idxs = Vec::with_capacity(3_072);

    for seg in &rep.segments_3d {
        if !is_disc_visible(seg.discipline) {
            continue;
        }
        let col = seg.discipline.rgba();
        let p0 = rot_enu(seg.start_enu_m);
        let p1 = rot_enu(seg.end_enu_m);
        push_animated_flow_strut_enu(
            &mut verts,
            &mut idxs,
            p0,
            p1,
            seg.width_m,
            seg.height_m,
            col,
            seg.discipline.discipline_code(),
            31.0,
        );
    }

    for eq in &rep.equipment_3d {
        if !is_disc_visible(eq.discipline) {
            continue;
        }
        let col = eq.discipline.rgba();
        let c = rot_enu(eq.center_enu_m);
        let hx = eq.size_enu_m[0] * 0.5;
        let hy = eq.size_enu_m[1] * 0.5;
        let rot_pt = |[lx, ly]: [f64; 2]| [c[0] + lx * cr - ly * sr, c[1] + lx * sr + ly * cr];
        let fp: [[f64; 2]; 4] = [
            rot_pt([-hx, -hy]),
            rot_pt([hx, -hy]),
            rot_pt([hx, hy]),
            rot_pt([-hx, hy]),
        ];
        let z0 = (c[2] - eq.size_enu_m[2] * 0.5) as f32;
        let z1 = (c[2] + eq.size_enu_m[2] * 0.5) as f32;
        push_box_prism_enu(&mut verts, &mut idxs, &fp, z0, z1, col, 31.0);
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input payload for Update `v0.8.7.6` (`6/10` -> `v0.8.8`) Unified 3D Clash Beacons (`material_kind = 32.0`).
#[derive(Clone, Copy)]
pub struct ClashGuardrailGpuInput<'a> {
    pub report: &'a crate::core::clash_engine::SpatialClashReport,
    pub heading_deg: f64,
    pub visible: bool,
}

/// Builds 3D Pure WebGPU pulsing clash octahedron beacons & interference highlight volumes (`material_kind = 32.0`).
pub fn build_rtc_clash_guardrail_mesh(input: &ClashGuardrailGpuInput<'_>) -> GpuMeshPayload {
    use crate::core::clash_engine::SpatialClashSeverity;

    if !input.visible {
        return GpuMeshPayload::default();
    }
    let rep = input.report;
    let rad = input.heading_deg.to_radians();
    let (sr, cr) = (rad.sin(), rad.cos());

    let mut verts = Vec::with_capacity(1_024);
    let mut idxs = Vec::with_capacity(1_536);

    for (idx, clash) in rep.clashes.iter().enumerate() {
        if rep.config.show_hard_only && clash.severity != SpatialClashSeverity::HardClash {
            continue;
        }
        let cx = clash.center_enu_m[0] * cr - clash.center_enu_m[1] * sr;
        let cy = clash.center_enu_m[0] * sr + clash.center_enu_m[1] * cr;
        let cz = clash.center_enu_m[2];
        let rgba = clash.severity.rgba();
        let beacon_r = match clash.severity {
            SpatialClashSeverity::HardClash => 0.42_f32,
            SpatialClashSeverity::ClearanceWarning => 0.30_f32,
            SpatialClashSeverity::SleeveAdvisory => 0.22_f32,
        };
        push_octahedron_beacon_enu(
            &mut verts,
            &mut idxs,
            [cx as f32, cy as f32, cz as f32],
            beacon_r,
            rgba,
            clash.severity.severity_norm(),
            32.0,
        );

        let hx = (clash.overlap_size_enu_m[0] * 0.5).clamp(0.10, 0.85);
        let hy = (clash.overlap_size_enu_m[1] * 0.5).clamp(0.10, 0.85);
        let hz = (clash.overlap_size_enu_m[2] * 0.5).clamp(0.10, 0.65);
        let rot_pt = |[lx, ly]: [f64; 2]| [cx + lx * cr - ly * sr, cy + lx * sr + ly * cr];
        let fp: [[f64; 2]; 4] = [
            rot_pt([-hx, -hy]),
            rot_pt([hx, -hy]),
            rot_pt([hx, hy]),
            rot_pt([-hx, hy]),
        ];
        let box_col = [rgba[0], rgba[1], rgba[2], (rgba[3] * 0.45).clamp(0.25, 0.60)];
        let mut v_start = verts.len();
        push_box_prism_enu(
            &mut verts,
            &mut idxs,
            &fp,
            (cz - hz) as f32,
            (cz + hz) as f32,
            box_col,
            32.0,
        );
        for v in &mut verts[v_start..] {
            v.meta[1] = clash.severity.severity_norm();
            v.meta[3] = (idx as f32) * 0.7;
        }
        let _ = &mut v_start;
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input parameters for realistic coastal ocean & river water surface rendering (`material_kind = 29.0`).
#[derive(Clone, Copy)]
pub struct CoastalWaterGpuInput<'a> {
    pub tin: &'a TinSurfaceMesh,
    pub sea_level_m: f32,
    pub visible: bool,
}

/// Builds the translucent 3D coastal ocean & river water mesh (`material_kind = 29.0`) around `input.tin`.
///
/// Each vertex encodes `[29.0, depth_m, 1.0, shore_phase]` in `meta` so `EARTH_WGSL` can synthesize
/// Beer-Lambert optical water depth absorption, multi-octave Gerstner ripples, and animated surf foam.
pub fn build_rtc_coastal_water_mesh(input: &CoastalWaterGpuInput<'_>) -> GpuMeshPayload {
    if !input.visible || input.tin.points.is_empty() || input.tin.min_elevation >= input.sea_level_m {
        return GpuMeshPayload::default();
    }
    let tin = input.tin;
    let (ref_z, _) = tin.sample_elevation_and_slope(0.0, 0.0);
    let z_water_rel = input.sea_level_m - ref_z;

    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for p in &tin.points {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }
    let skirt_m = 220.0_f32;
    let w_min_x = min_x - skirt_m;
    let w_max_x = max_x + skirt_m;
    let w_min_y = min_y - skirt_m;
    let w_max_y = max_y + skirt_m;
    let span_x = (w_max_x - w_min_x).max(10.0);
    let span_y = (w_max_y - w_min_y).max(10.0);

    let cols = 44usize;
    let rows = 44usize;
    let mut depths = Vec::with_capacity(cols * rows);
    let mut verts = Vec::with_capacity(cols * rows);

    for r in 0..rows {
        let v = r as f32 / (rows - 1) as f32;
        let wy = w_min_y + v * span_y;
        for c in 0..cols {
            let u = c as f32 / (cols - 1) as f32;
            let wx = w_min_x + u * span_x;
            let cx = wx.clamp(min_x, max_x);
            let cy = wy.clamp(min_y, max_y);
            let outside_dist = (wx - cx).hypot(wy - cy);
            let (sampled_z, _) = tin.sample_elevation_and_slope(cx, cy);
            let bed_z = if outside_dist <= 0.1 {
                sampled_z
            } else if sampled_z < input.sea_level_m + 1.2 {
                // Smooth bathymetry deepening into open ocean horizon outside coastal edges
                let t_far = (outside_dist / skirt_m).clamp(0.0, 1.0);
                sampled_z * (1.0 - t_far) + (-14.0) * t_far
            } else {
                // Dry highland edge: do not extend ocean beneath mountain cliffs
                sampled_z
            };
            let depth_m = (input.sea_level_m - bed_z).max(0.0);
            let shore_phase = (wx * 0.017 + wy * 0.013).sin() * 0.5 + 0.5;
            depths.push(depth_m);
            verts.push(GpuEarthVertex {
                pos: [wx, wy, z_water_rel],
                normal: [0.0, 0.0, 1.0],
                color: [0.06, 0.44, 0.62, 0.86],
                meta: [29.0, depth_m, 1.0, shore_phase],
            });
        }
    }

    let mut idxs = Vec::with_capacity((cols - 1) * (rows - 1) * 6);
    for r in 0..(rows - 1) {
        for c in 0..(cols - 1) {
            let i00 = r * cols + c;
            let i10 = r * cols + (c + 1);
            let i01 = (r + 1) * cols + c;
            let i11 = (r + 1) * cols + (c + 1);
            if depths[i00] > 0.01 || depths[i10] > 0.01 || depths[i01] > 0.01 {
                idxs.extend_from_slice(&[i00 as u32, i10 as u32, i01 as u32]);
            }
            if depths[i10] > 0.01 || depths[i11] > 0.01 || depths[i01] > 0.01 {
                idxs.extend_from_slice(&[i10 as u32, i11 as u32, i01 as u32]);
            }
        }
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}

/// GPU input parameters for Cities: Skylines II dynamic building sun shadows & animated traffic.
#[derive(Clone, Copy)]
pub struct CityAmbienceGpuInput<'a> {
    pub city: &'a IfcxCityScene,
    pub tin: Option<&'a TinSurfaceMesh>,
    pub sea_level_m: f32,
    pub sun_dir_site: [f32; 3],
    pub time_secs: f32,
    pub visible: bool,
}

/// Builds translucent dynamic ground cast shadows and animated 3D traffic vehicles with emissive
/// headlights & taillights (`material_kind = 30.0`) along straight/curved roads and bridges.
pub fn build_rtc_city_ambience_mesh(input: &CityAmbienceGpuInput<'_>) -> GpuMeshPayload {
    if !input.visible || input.city.is_empty() {
        return GpuMeshPayload::default();
    }
    let shadows = input
        .city
        .compute_dynamic_building_shadows(input.tin, input.sun_dir_site);
    let vehicles = input
        .city
        .sample_animated_traffic(input.tin, input.sea_level_m, input.time_secs);
    if shadows.is_empty() && vehicles.is_empty() {
        return GpuMeshPayload::default();
    }

    let mut verts = Vec::with_capacity(shadows.len() * 4 + vehicles.len() * 48);
    let mut idxs = Vec::with_capacity(shadows.len() * 6 + vehicles.len() * 72);

    // 1. Dynamic sun-azimuth cast shadows beneath zoned towers
    let sh_col = [0.04_f32, 0.07, 0.14, 0.36];
    for sh in &shadows {
        let base = verts.len() as u32;
        for &c in &sh.corners_enu_m {
            verts.push(GpuEarthVertex {
                pos: c,
                normal: [0.0, 0.0, 1.0],
                color: sh_col,
                meta: [3.0, 0.0, 1.0, 0.0],
            });
        }
        idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    // 2. Animated 3D cars/buses/trucks with warm headlights & crimson taillights (`material_kind = 30.0`)
    let push_oriented_box = |v: &mut Vec<GpuEarthVertex>,
                             idx_out: &mut Vec<u32>,
                             center: [f64; 2],
                             fwd: [f64; 2],
                             left: [f64; 2],
                             hl: f64,
                             hw: f64,
                             z0: f32,
                             z1: f32,
                             col: [f32; 4],
                             kind: f32,
                             meta_y: f32| {
        let pts2 = [
            [
                center[0] - fwd[0] * hl - left[0] * hw,
                center[1] - fwd[1] * hl - left[1] * hw,
            ],
            [
                center[0] + fwd[0] * hl - left[0] * hw,
                center[1] + fwd[1] * hl - left[1] * hw,
            ],
            [
                center[0] + fwd[0] * hl + left[0] * hw,
                center[1] + fwd[1] * hl + left[1] * hw,
            ],
            [
                center[0] - fwd[0] * hl + left[0] * hw,
                center[1] - fwd[1] * hl + left[1] * hw,
            ],
        ];
        let base = v.len() as u32;
        for &p in &pts2 {
            v.push(GpuEarthVertex {
                pos: [p[0] as f32, p[1] as f32, z0],
                normal: [0.0, 0.0, 1.0],
                color: col,
                meta: [kind, meta_y, 0.88, 0.0],
            });
        }
        for &p in &pts2 {
            v.push(GpuEarthVertex {
                pos: [p[0] as f32, p[1] as f32, z1],
                normal: [0.0, 0.0, 1.0],
                color: col,
                meta: [kind, meta_y, 1.0, 0.0],
            });
        }
        for &tri in &[
            [0_u32, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [2, 3, 7],
            [2, 7, 6],
            [1, 2, 6],
            [1, 6, 5],
            [3, 0, 4],
            [3, 4, 7],
        ] {
            idx_out.extend_from_slice(&[base + tri[0], base + tri[1], base + tri[2]]);
        }
    };

    for veh in &vehicles {
        let cx = veh.pos_enu_m[0] as f64;
        let cy = veh.pos_enu_m[1] as f64;
        let z0 = veh.pos_enu_m[2];
        let hl = veh.length_m * 0.5;
        let hw = veh.width_m * 0.5;
        push_oriented_box(
            &mut verts,
            &mut idxs,
            [cx, cy],
            veh.tangent,
            veh.normal,
            hl,
            hw,
            z0,
            z0 + veh.height_m,
            veh.body_rgba,
            3.0,
            veh.height_m,
        );
        // Front headlights (meta.y = 1.0)
        for side in [-0.62_f64, 0.62] {
            let hx = cx + veh.tangent[0] * (hl + 0.14) + veh.normal[0] * hw * side;
            let hy = cy + veh.tangent[1] * (hl + 0.14) + veh.normal[1] * hw * side;
            push_oriented_box(
                &mut verts,
                &mut idxs,
                [hx, hy],
                veh.tangent,
                veh.normal,
                0.24,
                0.20,
                z0 + 0.35,
                z0 + 0.72,
                [1.0, 0.96, 0.78, 0.96],
                30.0,
                1.0,
            );
        }
        // Rear taillights (meta.y = 0.0)
        for side in [-0.62_f64, 0.62] {
            let rx = cx - veh.tangent[0] * (hl + 0.12) + veh.normal[0] * hw * side;
            let ry = cy - veh.tangent[1] * (hl + 0.12) + veh.normal[1] * hw * side;
            push_oriented_box(
                &mut verts,
                &mut idxs,
                [rx, ry],
                veh.tangent,
                veh.normal,
                0.20,
                0.18,
                z0 + 0.38,
                z0 + 0.70,
                [0.98, 0.18, 0.16, 0.96],
                30.0,
                0.0,
            );
        }
    }

    GpuMeshPayload {
        vertices: verts,
        indices: idxs,
    }
}


