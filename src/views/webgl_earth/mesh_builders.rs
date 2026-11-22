//! Pure-vector 3D mesh builders for Planetary Earth, IFCX City-Builder, Active BIM/FEM, and District Multi-Hazard GPU passes.

use egui::Color32;

use crate::core::bim::{BimElementKind, BimProject};
use crate::core::geotech::{
    GeotechEngine, SubsurfaceSsiAnalysisReport, SubsurfaceSsiConfig,
};
use crate::core::gis::city_builder::{IfcxCityScene, IfcxZoneKind};
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

/// Appends authored IFCX City-Builder objects (`IfcRoad`, `IfcBridge`, `IfcSpatialZone`,
/// context towers, instanced stylized trees, and `IfcGradedPad`) into the RTC site mesh.
pub(super) fn push_city_builder_scene_enu(
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
    pub subsurface_pile_depth_m: f32,
    pub heading_deg: f64,
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

/// 3D Subsurface Geotechnical Stratigraphy & Groundwater Table X-Ray input for WebGL2 Pass 2.
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

