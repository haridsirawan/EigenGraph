//! dem_stream.rs — Pure-Rust Offline Vector Topography & Survey-to-TIN Builder (Phase 64 / v0.6.4).
//!
//! Converts pure-vector 3D topographic geometry (OpenBIM IFC 4.3 `IfcSite` `IfcTriangulatedFaceSet`
//! meshes and local CSV/XYZ survey point clouds) into centered `TinSurfaceMesh` surfaces for Earth
//! Vector rendering with zero external raster PNG downloads or `image` crate dependencies.

use super::tin_surface::{TinPoint, TinSurfaceMesh};

/// Converts explicit pure-vector 3D coordinates `(x_m, y_m, z_m)` and optional triangle index triplets
/// (such as OpenBIM IFC4/IFC4.3 `IfcTriangulatedFaceSet`) into a centered `TinSurfaceMesh`.
pub fn build_tin_from_3d_triangles(
    coords_m: &[(f64, f64, f64)],
    triangles: &[[usize; 3]],
) -> Result<TinSurfaceMesh, String> {
    if coords_m.len() < 3 {
        return Err("Vector terrain requires at least 3 3D coordinates.".to_string());
    }
    let min_x = coords_m.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let max_x = coords_m.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    let min_y = coords_m.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let max_y = coords_m.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    let mid_x = 0.5 * (min_x + max_x);
    let mid_y = 0.5 * (min_y + max_y);

    let pts: Vec<TinPoint> = coords_m
        .iter()
        .enumerate()
        .map(|(id, &(x, y, z))| TinPoint {
            id,
            x: (x - mid_x) as f32,
            y: (y - mid_y) as f32,
            z: z as f32,
        })
        .collect();

    if !triangles.is_empty() {
        Ok(TinSurfaceMesh::from_explicit_triangles(pts, triangles.to_vec()))
    } else {
        let step = (pts.len() / 600).max(1);
        let sampled: Vec<TinPoint> = pts
            .into_iter()
            .step_by(step)
            .enumerate()
            .map(|(id, mut p)| {
                p.id = id;
                p
            })
            .collect();
        Ok(TinSurfaceMesh::triangulate_bowyer_watson(sampled))
    }
}

/// Converts raw CSV/XYZ (`x, y, z`) topographic survey text directly into a `TinSurfaceMesh`
/// centered around `(0, 0)` local tangent meters for Earth Vector draping.
pub fn parse_survey_xyz_to_tin(content: &str) -> Result<TinSurfaceMesh, String> {
    let mut raw: Vec<(f32, f32, f32)> = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let cleaned = trimmed.replace([',', ';', '\t'], " ");
        let nums: Vec<f32> = cleaned
            .split_whitespace()
            .filter_map(|t| t.parse::<f32>().ok())
            .collect();
        if nums.len() >= 3 {
            raw.push((nums[0], nums[1], nums[2]));
        }
    }
    if raw.len() < 3 {
        return Err("Survey file requires at least 3 numeric (X, Y, Z) points.".to_string());
    }
    let min_x = raw.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
    let max_x = raw.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
    let min_y = raw.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
    let max_y = raw.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
    let mid_x = 0.5 * (min_x + max_x);
    let mid_y = 0.5 * (min_y + max_y);

    let step = (raw.len() / 600).max(1);
    let pts: Vec<TinPoint> = raw
        .into_iter()
        .step_by(step)
        .enumerate()
        .map(|(id, (x, y, z))| TinPoint {
            id,
            x: x - mid_x,
            y: y - mid_y,
            z,
        })
        .collect();
    Ok(TinSurfaceMesh::triangulate_bowyer_watson(pts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase64_pure_vector_tin_and_survey_conversion() {
        let coords = [
            (0.0, 0.0, 10.0),
            (50.0, 0.0, 12.5),
            (50.0, 50.0, 18.0),
            (0.0, 50.0, 14.0),
            (25.0, 25.0, 21.0),
        ];
        let tris = [[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]];
        let explicit_tin = build_tin_from_3d_triangles(&coords, &tris).expect("Build explicit IFC TIN");
        assert_eq!(explicit_tin.points.len(), 5);
        assert_eq!(explicit_tin.triangles.len(), 4);
        assert!((explicit_tin.max_elevation - 21.0).abs() < 1e-3);
        assert!((explicit_tin.min_elevation - 10.0).abs() < 1e-3);

        let xyz = "0,0,10.0\n50,0,12.5\n50,50,18.0\n0,50,14.0\n25,25,21.0\n";
        let survey_tin = parse_survey_xyz_to_tin(xyz).expect("Parse survey XYZ");
        assert_eq!(survey_tin.points.len(), 5);
        assert_eq!(survey_tin.triangles.len(), 4);
    }
}
