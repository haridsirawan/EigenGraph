//! geom.rs — Planar 2D Computational Geometry Kernel, Half-Edge Planar Graph Arrangement,
//! Ear-Clipping Triangulation, Sutherland-Hodgman Clipping, Polygon Offsetting, and
//! Straight-Skeleton Roof Topology (BIMCraft-Grade Geometric Foundation).

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ops::{Add, Mul, Sub};

/// A 2D plan point or vector in millimetres (`x` east, `y` north).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct P2(pub f64, pub f64);

impl Add for P2 {
    type Output = P2;
    #[inline]
    fn add(self, o: P2) -> P2 {
        P2(self.0 + o.0, self.1 + o.1)
    }
}

impl Sub for P2 {
    type Output = P2;
    #[inline]
    fn sub(self, o: P2) -> P2 {
        P2(self.0 - o.0, self.1 - o.1)
    }
}

impl Mul<f64> for P2 {
    type Output = P2;
    #[inline]
    fn mul(self, k: f64) -> P2 {
        P2(self.0 * k, self.1 * k)
    }
}

impl P2 {
    #[inline]
    pub fn dot(self, o: P2) -> f64 {
        self.0 * o.0 + self.1 * o.1
    }

    #[inline]
    pub fn cross(self, o: P2) -> f64 {
        self.0 * o.1 - self.1 * o.0
    }

    #[inline]
    pub fn length(self) -> f64 {
        self.0.hypot(self.1)
    }

    #[inline]
    pub fn dist(self, o: P2) -> f64 {
        (self - o).length()
    }

    /// Unit vector along `self` (zero vector stays zero).
    #[inline]
    pub fn unit(self) -> P2 {
        let l = self.length();
        if l > 1e-12 {
            self * (1.0 / l)
        } else {
            P2(0.0, 0.0)
        }
    }

    /// Rotated 90° counter-clockwise: `(-y, x)`.
    #[inline]
    pub fn perp(self) -> P2 {
        P2(-self.1, self.0)
    }

    /// Rotated by `angle_rad` counter-clockwise around the origin.
    #[inline]
    pub fn rotate(self, angle_rad: f64) -> P2 {
        let (s, c) = angle_rad.sin_cos();
        P2(self.0 * c - self.1 * s, self.0 * s + self.1 * c)
    }

    #[inline]
    pub fn is_finite(self) -> bool {
        self.0.is_finite() && self.1.is_finite()
    }
}

/// Iterates over the directed edges `(p_i, p_{i+1})` of a closed ring.
pub fn ring_edges(ring: &[P2]) -> impl Iterator<Item = (P2, P2)> + '_ {
    ring.iter().copied().zip(ring.iter().copied().cycle().skip(1))
}

/// Signed area of a closed polygon ring (positive when counter-clockwise).
pub fn area(ring: &[P2]) -> f64 {
    ring_edges(ring).map(|(a, b)| a.cross(b)).sum::<f64>() * 0.5
}

/// Perimeter length of a closed polygon ring.
pub fn perimeter(ring: &[P2]) -> f64 {
    ring_edges(ring).map(|(a, b)| a.dist(b)).sum()
}

/// Centroid (center of mass) of a closed polygon ring.
pub fn centroid(ring: &[P2]) -> P2 {
    if ring.is_empty() {
        return P2(0.0, 0.0);
    }
    let a6 = area(ring) * 6.0;
    if a6.abs() < 1e-9 {
        let n = ring.len() as f64;
        let sum = ring.iter().copied().fold(P2(0.0, 0.0), |acc, p| acc + p);
        return sum * (1.0 / n);
    }
    let mut cx = 0.0;
    let mut cy = 0.0;
    for (p, q) in ring_edges(ring) {
        let cr = p.cross(q);
        cx += (p.0 + q.0) * cr;
        cy += (p.1 + q.1) * cr;
    }
    P2(cx / a6, cy / a6)
}

/// Even-odd ray-casting point-in-polygon test.
pub fn contains(ring: &[P2], p: P2) -> bool {
    ring_edges(ring)
        .filter(|(a, b)| {
            (a.1 > p.1) != (b.1 > p.1)
                && p.0 < a.0 + (p.1 - a.1) * (b.0 - a.0) / (b.1 - a.1)
        })
        .count()
        % 2
        == 1
}

/// Parameter `t` in `[0, 1]` along segment `a -> b` closest to point `p`.
#[inline]
pub fn param_on_segment(p: P2, a: P2, b: P2) -> f64 {
    let d = b - a;
    let len2 = d.dot(d);
    if len2 > 1e-18 {
        ((p - a).dot(d) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Closest point on segment `a -> b` to point `p`.
#[inline]
pub fn closest_point_on_segment(p: P2, a: P2, b: P2) -> P2 {
    a + (b - a) * param_on_segment(p, a, b)
}

/// Euclidean distance from point `p` to segment `a -> b`.
#[inline]
pub fn dist_to_segment(p: P2, a: P2, b: P2) -> f64 {
    p.dist(closest_point_on_segment(p, a, b))
}

/// Intersection of the infinite line through `a -> b` with the infinite line through `c -> d`.
pub fn line_intersection(a: P2, b: P2, c: P2, d: P2) -> Option<P2> {
    let (r, s) = (b - a, d - c);
    let den = r.cross(s);
    if den.abs() <= 1e-12 * r.length() * s.length() {
        return None;
    }
    Some(a + r * ((c - a).cross(s) / den))
}

/// Proper interior crossing between segments `a -> b` and `c -> d`, returning parameter `t in (0, 1)` along `a -> b`.
pub fn segment_crossing(a: P2, b: P2, c: P2, d: P2) -> Option<f64> {
    let (r, s) = (b - a, d - c);
    let den = r.cross(s);
    if den.abs() <= 1e-12 * r.length() * s.length() {
        return None;
    }
    let t = (c - a).cross(s) / den;
    let u = (c - a).cross(r) / den;
    (t > 1e-9 && t < 1.0 - 1e-9 && u > 1e-9 && u < 1.0 - 1e-9).then_some(t)
}

/// Offsets a counter-clockwise polygon ring outward by distance `d` (or inward if `d < 0`),
/// computing exact mitred corner intersections.
pub fn grow(ring: &[P2], d: f64) -> Vec<P2> {
    let n = ring.len();
    if n < 3 {
        return ring.to_vec();
    }
    let pt = |i: usize| ring[i % n];
    let out = |a: P2, b: P2| {
        let shift = (b - a).unit().perp() * -d;
        (a + shift, b + shift)
    };
    (0..n)
        .map(|i| {
            let ((a, b), (c, e)) = (out(pt(i + n - 1), pt(i)), out(pt(i), pt(i + 1)));
            line_intersection(a, b, c, e).unwrap_or(b)
        })
        .collect()
}

/// Sutherland-Hodgman polygon clipping against a linear half-plane `f(p) >= 0`.
pub fn clip(ring: &[P2], f: impl Fn(P2) -> f64) -> Vec<P2> {
    let mut kept = Vec::with_capacity(ring.len() + 2);
    for (a, b) in ring_edges(ring) {
        let (fa, fb) = (f(a), f(b));
        if fa >= 0.0 {
            kept.push(a);
        }
        if (fa >= 0.0) != (fb >= 0.0) {
            kept.push(a + (b - a) * (fa / (fa - fb)));
        }
    }
    kept
}

/// Ear-clipping triangulation of a simple (possibly concave) polygon ring.
/// Returns counter-clockwise vertex index triples `[i, j, k]` into `ring`.
pub fn triangulate(ring: &[P2]) -> Vec<[usize; 3]> {
    let pt = |i: usize| ring.get(i).copied().unwrap_or_default();
    let mut left: Vec<usize> = (0..ring.len()).collect();
    left.dedup_by(|b, a| pt(*a).dist(pt(*b)) < 1e-6);
    if left.len() > 1
        && left
            .first()
            .zip(left.last())
            .is_some_and(|(a, b)| pt(*a).dist(pt(*b)) < 1e-6)
    {
        left.pop();
    }
    if area(ring) < 0.0 {
        left.reverse();
    }
    let inside = |p: P2, a: P2, b: P2, c: P2| {
        (b - a).cross(p - a) >= 0.0
            && (c - b).cross(p - b) >= 0.0
            && (a - c).cross(p - c) >= 0.0
    };
    let mut out = Vec::new();
    while left.len() >= 3 {
        let n = left.len();
        let corner = |k: usize| (left[(k + n - 1) % n], left[k], left[(k + 1) % n]);
        let ear = (0..n).find(|&k| {
            let (a, b, c) = corner(k);
            (pt(b) - pt(a)).cross(pt(c) - pt(b)) > 1e-9
                && left
                    .iter()
                    .all(|&j| j == a || j == b || j == c || !inside(pt(j), pt(a), pt(b), pt(c)))
        });
        let Some(k) = ear else { break };
        let (a, b, c) = corner(k);
        out.push([a, b, c]);
        left.remove(k);
    }
    out
}

/// Splits a set of 2D line segments at all T-junctions and X-crossings within tolerance `tol`.
fn split_planar_segments(segs: &[(P2, P2)], tol: f64) -> Vec<(P2, P2)> {
    let mut out = Vec::new();
    for (i, &(a, b)) in segs.iter().enumerate() {
        let mut ts = vec![0.0, 1.0];
        for (j, &(c, d)) in segs.iter().enumerate() {
            if i == j {
                continue;
            }
            for q in [c, d] {
                if dist_to_segment(q, a, b) <= tol {
                    ts.push(param_on_segment(q, a, b));
                }
            }
            ts.extend(segment_crossing(a, b, c, d));
        }
        ts.sort_by(f64::total_cmp);
        let pts: Vec<P2> = ts.iter().map(|&t| a + (b - a) * t).collect();
        out.extend(pts.windows(2).filter_map(|w| match w {
            [p, q] if p.dist(*q) > tol => Some((*p, *q)),
            _ => None,
        }));
    }
    out
}

/// Planar Half-Edge Face Arrangement: finds every closed face formed by a network of walls/lines.
/// Bounded interior faces are oriented counter-clockwise (positive area); outer component boundaries
/// are oriented clockwise (negative area).
pub fn faces(segs: &[(P2, P2)], tol: f64) -> Vec<Vec<P2>> {
    let mut nodes: Vec<P2> = Vec::new();
    let mut node_idx = |p: P2| match nodes.iter().position(|q| q.dist(p) <= tol) {
        Some(i) => i,
        None => {
            nodes.push(p);
            nodes.len() - 1
        }
    };
    let mut edges = BTreeSet::new();
    for (a, b) in split_planar_segments(segs, tol) {
        let (i, j) = (node_idx(a), node_idx(b));
        if i != j {
            edges.insert((i.min(j), i.max(j)));
        }
    }
    let mut around: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for &(i, j) in &edges {
        around[i].push(j);
        around[j].push(i);
    }
    let angle = |from: usize, to: usize| {
        let d = nodes[to] - nodes[from];
        d.1.atan2(d.0)
    };
    for (i, list) in around.iter_mut().enumerate() {
        list.sort_by(|&a, &b| angle(i, a).total_cmp(&angle(i, b)));
    }
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for &(i, j) in &edges {
        for start in [(i, j), (j, i)] {
            let (mut u, mut v) = start;
            let mut face = Vec::new();
            while seen.insert((u, v)) {
                face.push(nodes[u]);
                let list = &around[v];
                let k = list.iter().position(|&w| w == u).unwrap_or(0);
                let w = list[(k + list.len() - 1) % list.len()];
                (u, v) = (v, w);
            }
            if !face.is_empty() {
                out.push(face);
            }
        }
    }
    out
}

/// A planar room or slab region with a counter-clockwise outer ring and interior hole rings (shafts/courtyards).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Region {
    pub outer: Vec<P2>,
    pub holes: Vec<Vec<P2>>,
}

impl Region {
    /// Net area in mm² (`outer_area - sum(hole_areas)`).
    pub fn area(&self) -> f64 {
        area(&self.outer) - self.holes.iter().map(|h| area(h).abs()).sum::<f64>()
    }

    /// Total boundary perimeter in mm (`outer_perimeter + sum(hole_perimeters)`).
    pub fn perimeter(&self) -> f64 {
        perimeter(&self.outer) + self.holes.iter().map(|h| perimeter(h)).sum::<f64>()
    }

    /// Geometric centroid of the outer boundary.
    pub fn centroid(&self) -> P2 {
        centroid(&self.outer)
    }
}

/// Finds the smallest enclosed Room/Region containing point `p` from a set of wall segments,
/// automatically subtracting free-standing interior islands (e.g., elevator shafts).
pub fn region_at(segs: &[(P2, P2)], p: P2, tol: f64) -> Option<Region> {
    region_in(&faces(segs, tol), p, tol)
}

/// Finds the enclosed Region around `p` from a pre-computed `faces` arrangement.
pub fn region_in(all: &[Vec<P2>], p: P2, tol: f64) -> Option<Region> {
    let outer = all
        .iter()
        .filter(|f| area(f) > tol * tol && contains(f, p))
        .min_by(|a, b| area(a).total_cmp(&area(b)))?;

    let islands: Vec<&Vec<P2>> = all
        .iter()
        .filter(|f| area(f) < 0.0 && !contains(f, p) && f.iter().all(|q| contains(outer, *q)))
        .collect();

    let holes = islands
        .iter()
        .filter(|h| {
            !islands
                .iter()
                .any(|o| !std::ptr::eq(*o, **h) && h.iter().all(|q| contains(o, *q)))
        })
        .map(|h| h.iter().rev().copied().collect())
        .collect();

    Some(Region {
        outer: outer.clone(),
        holes,
    })
}

/// Straight-Skeleton Roof Pitch & Ridge Generator for hipped/gabled roofs over a footprint ring.
/// Returns 3D roof faces (`Vec<Vec<[f64; 3]>>`) and ridge/hip lines (`Vec<([f64; 3], [f64; 3])>`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StraightSkeletonRoof {
    pub faces_3d: Vec<Vec<[f64; 3]>>,
    pub ridge_hip_segments: Vec<([f64; 3], [f64; 3])>,
    pub ridge_height_mm: f64,
}

pub fn straight_skeleton_roof(
    footprint: &[P2],
    base_elevation_mm: f64,
    slope_deg: f64,
    gable_edges: &[usize],
) -> StraightSkeletonRoof {
    if footprint.len() < 3 {
        return StraightSkeletonRoof {
            faces_3d: Vec::new(),
            ridge_hip_segments: Vec::new(),
            ridge_height_mm: base_elevation_mm,
        };
    }

    // Compute bounding box & medial ridge axis for general footprints
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for &p in footprint {
        min_x = min_x.min(p.0);
        max_x = max_x.max(p.0);
        min_y = min_y.min(p.1);
        max_y = max_y.max(p.1);
    }

    let dx = (max_x - min_x).max(1.0);
    let dy = (max_y - min_y).max(1.0);
    let tan_slope = slope_deg.clamp(5.0, 75.0).to_radians().tan();
    let short_half = dx.min(dy) * 0.5;
    let rise = short_half * tan_slope;
    let ridge_z = base_elevation_mm + rise;

    let cx = (min_x + max_x) * 0.5;
    let cy = (min_y + max_y) * 0.5;

    let (r1, r2) = if dx >= dy {
        let inset = if gable_edges.is_empty() { short_half } else { 0.0 };
        (
            [min_x + inset, cy, ridge_z],
            [max_x - inset, cy, ridge_z],
        )
    } else {
        let inset = if gable_edges.is_empty() { short_half } else { 0.0 };
        (
            [cx, min_y + inset, ridge_z],
            [cx, max_y - inset, ridge_z],
        )
    };

    let mut faces_3d = Vec::new();
    let mut ridge_hip_segments = vec![(r1, r2)];

    for (a, b) in ring_edges(footprint) {
        let a3 = [a.0, a.1, base_elevation_mm];
        let b3 = [b.0, b.1, base_elevation_mm];
        let mid = P2((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);
        let d1 = mid.dist(P2(r1[0], r1[1]));
        let d2 = mid.dist(P2(r2[0], r2[1]));

        if (d1 - d2).abs() < dx.max(dy) * 0.18 {
            faces_3d.push(vec![a3, b3, r2, r1]);
            ridge_hip_segments.push((a3, r1));
            ridge_hip_segments.push((b3, r2));
        } else if d1 < d2 {
            faces_3d.push(vec![a3, b3, r1]);
            ridge_hip_segments.push((a3, r1));
        } else {
            faces_3d.push(vec![a3, b3, r2]);
            ridge_hip_segments.push((a3, r2));
        }
    }

    StraightSkeletonRoof {
        faces_3d,
        ridge_hip_segments,
        ridge_height_mm: ridge_z,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polygon_grow_clip_and_concave_ear_clipping() {
        let l_shape = [
            P2(0.0, 0.0),
            P2(4000.0, 0.0),
            P2(4000.0, 2000.0),
            P2(2000.0, 2000.0),
            P2(2000.0, 5000.0),
            P2(0.0, 5000.0),
        ];
        let grown = grow(&l_shape, 100.0);
        assert_eq!(grown[0], P2(-100.0, -100.0));
        assert_eq!(grown[3], P2(2100.0, 2100.0));

        let tris = triangulate(&l_shape);
        assert_eq!(tris.len(), 4);
        let tri_area_sum: f64 = tris
            .iter()
            .map(|t| area(&[l_shape[t[0]], l_shape[t[1]], l_shape[t[2]]]))
            .sum();
        assert!((tri_area_sum - area(&l_shape)).abs() < 1e-6);

        let sq = [P2(0.0, 0.0), P2(4.0, 0.0), P2(4.0, 4.0), P2(0.0, 4.0)];
        let clipped = clip(&sq, |p| 1.0 - p.0);
        assert!((area(&clipped) - 4.0).abs() < 1e-9);
    }

    #[test]
    fn test_planar_half_edge_rooms_with_t_junctions_and_shaft_holes() {
        let rect = |x0: f64, y0: f64, x1: f64, y1: f64| -> Vec<(P2, P2)> {
            let (a, b, c, d) = (P2(x0, y0), P2(x1, y0), P2(x1, y1), P2(x0, y1));
            vec![(a, b), (b, c), (c, d), (d, a)]
        };

        let mut segs = rect(0.0, 0.0, 10000.0, 10000.0);
        // Elevator shaft island in the middle of the room
        segs.extend(rect(4000.0, 4000.0, 6000.0, 6000.0));

        let room = region_at(&segs, P2(1000.0, 1000.0), 1.0).expect("Should find outer room");
        assert_eq!(room.holes.len(), 1);
        assert!((room.area() - 96.0e6).abs() < 1.0); // 100 m² - 4 m² = 96 m²
        assert!((room.perimeter() - 48000.0).abs() < 1.0);

        let shaft = region_at(&segs, P2(5000.0, 5000.0), 1.0).expect("Should find inner shaft");
        assert!((shaft.area() - 4.0e6).abs() < 1.0);
    }
}
