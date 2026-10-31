//! csg.rs — 3D Solid Mesh Kernel, Non-Recursive BSP Boolean CSG (`subtract`),
//! True 3D Wide-Flange (WF) / Tube / Extruded Polygon Mesh Generators,
//! Plane Slicing for Live Section Cuts, and Bounding Volume Hierarchy (`Bvh3D`) Ray-Picking.

use crate::core::geom::{triangulate, P2};
use serde::{Deserialize, Serialize};

/// 3D point or vector in millimetres (`[x, y, z]`, where `z` is up).
pub type P3 = [f64; 3];

/// Maximum triangles allowed per operand in Boolean CSG operations.
pub const MAX_CSG_TRIANGLES: usize = 20_000;
const EPS: f64 = 1e-5;
const SLIVER: f64 = 1e-6;

#[inline]
pub fn add3(a: P3, b: P3) -> P3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

#[inline]
pub fn sub3(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
pub fn mul3(a: P3, k: f64) -> P3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

#[inline]
pub fn dot3(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
pub fn cross3(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[inline]
pub fn len3(a: P3) -> f64 {
    dot3(a, a).sqrt()
}

#[inline]
pub fn unit3(a: P3) -> P3 {
    let l = len3(a);
    if l > 1e-12 {
        mul3(a, 1.0 / l)
    } else {
        [0.0, 0.0, 0.0]
    }
}

#[inline]
pub fn tri_normal(t: &[P3; 3]) -> P3 {
    unit3(cross3(sub3(t[1], t[0]), sub3(t[2], t[0])))
}

/// Signed volume of a closed outward-facing triangle mesh via the divergence theorem.
pub fn mesh_volume(tris: &[[P3; 3]]) -> f64 {
    tris.iter()
        .map(|[a, b, c]| dot3(*a, cross3(*b, *c)) / 6.0)
        .sum()
}

/// Total surface area of a triangle mesh.
pub fn mesh_surface_area(tris: &[[P3; 3]]) -> f64 {
    tris.iter()
        .map(|[a, b, c]| len3(cross3(sub3(*b, *a), sub3(*c, *a))) * 0.5)
        .sum()
}

/// Work budget meter for non-recursive BSP CSG evaluation to guarantee termination.
pub struct CsgMeter {
    pub work: u64,
    pub splits: u64,
}

impl Default for CsgMeter {
    fn default() -> Self {
        Self {
            work: 40_000_000,
            splits: 400_000,
        }
    }
}

impl CsgMeter {
    fn record_work(&mut self, corners: usize) -> Result<(), String> {
        self.work = self
            .work
            .checked_sub(corners as u64 + 1)
            .ok_or_else(|| "CSG work budget exceeded: mesh has too many faces".to_string())?;
        Ok(())
    }

    fn record_split(&mut self) -> Result<(), String> {
        self.splits = self
            .splits
            .checked_sub(1)
            .ok_or_else(|| "CSG split budget exceeded: mesh has too many faces".to_string())?;
        Ok(())
    }
}

#[derive(Clone)]
struct CsgPoly {
    v: Vec<P3>,
    n: P3,
    w: f64,
}

impl CsgPoly {
    fn from_tri(t: &[P3; 3]) -> Option<Self> {
        let n_raw = cross3(sub3(t[1], t[0]), sub3(t[2], t[0]));
        let l = len3(n_raw);
        if !(l > 2.0 * SLIVER && l.is_finite()) {
            return None;
        }
        let n = mul3(n_raw, 1.0 / l);
        let w = dot3(n, t[0]);
        if !w.is_finite() {
            return None;
        }
        Some(Self {
            v: t.to_vec(),
            n,
            w,
        })
    }

    fn flip(&mut self) {
        self.v.reverse();
        self.n = mul3(self.n, -1.0);
        self.w = -self.w;
    }

    fn triangles(&self) -> impl Iterator<Item = [P3; 3]> + '_ {
        let first = self.v.first().copied().unwrap_or_default();
        self.v
            .windows(2)
            .skip(1)
            .map(move |w| [first, w[0], w[1]])
            .filter(|[a, b, c]| {
                let n = cross3(sub3(*b, *a), sub3(*c, *a));
                dot3(n, n) > (2.0 * SLIVER).powi(2)
            })
    }
}

enum PlaneSide {
    On(CsgPoly, bool),
    Front(CsgPoly),
    Back(CsgPoly),
    Across(Option<CsgPoly>, Option<CsgPoly>),
}

fn divide_poly(n: P3, w: f64, p: CsgPoly) -> PlaneSide {
    let kinds: Vec<i8> = p
        .v
        .iter()
        .map(|c| {
            let d = dot3(n, *c) - w;
            if d > EPS {
                1
            } else if d < -EPS {
                -1
            } else {
                0
            }
        })
        .collect();

    match (kinds.contains(&1), kinds.contains(&-1)) {
        (false, false) => {
            let co_same = dot3(n, p.n) > 0.0;
            PlaneSide::On(p, co_same)
        }
        (true, false) => PlaneSide::Front(p),
        (false, true) => PlaneSide::Back(p),
        (true, true) => {
            let mut front = Vec::new();
            let mut back = Vec::new();
            let m = p.v.len();
            for i in 0..m {
                let a = p.v[i];
                let b = p.v[(i + 1) % m];
                let ka = kinds[i];
                let kb = kinds[(i + 1) % m];
                if ka != -1 {
                    front.push(a);
                }
                if ka != 1 {
                    back.push(a);
                }
                if ka * kb == -1 {
                    let denom = dot3(n, sub3(b, a));
                    if denom.abs() > 1e-18 {
                        let t = (w - dot3(n, a)) / denom;
                        let c = add3(a, mul3(sub3(b, a), t));
                        front.push(c);
                        back.push(c);
                    }
                }
            }
            let part = |v: Vec<P3>| {
                (v.len() >= 3).then_some(CsgPoly {
                    v,
                    n: p.n,
                    w: p.w,
                })
            };
            PlaneSide::Across(part(front), part(back))
        }
    }
}

struct BspNode {
    n: P3,
    w: f64,
    polys: Vec<CsgPoly>,
    front: Option<usize>,
    back: Option<usize>,
}

struct BspTree {
    nodes: Vec<BspNode>,
}

impl BspTree {
    fn from_tris(tris: &[[P3; 3]], meter: &mut CsgMeter) -> Result<Self, String> {
        let mut bsp = Self { nodes: Vec::new() };
        bsp.build(tris.iter().filter_map(CsgPoly::from_tri).collect(), meter)?;
        Ok(bsp)
    }

    fn new_node(first: &CsgPoly) -> BspNode {
        BspNode {
            n: first.n,
            w: first.w,
            polys: Vec::new(),
            front: None,
            back: None,
        }
    }

    fn build(&mut self, polys: Vec<CsgPoly>, meter: &mut CsgMeter) -> Result<(), String> {
        let Some(first) = polys.first() else {
            return Ok(());
        };
        if self.nodes.is_empty() {
            self.nodes.push(Self::new_node(first));
        }
        let mut stack = vec![(0usize, polys)];
        while let Some((i, polys)) = stack.pop() {
            let Some((n, w)) = self.nodes.get(i).map(|x| (x.n, x.w)) else {
                continue;
            };
            let mut here = Vec::new();
            let mut front = Vec::new();
            let mut back = Vec::new();
            for p in polys {
                meter.record_work(p.v.len())?;
                match divide_poly(n, w, p) {
                    PlaneSide::On(p, _) => here.push(p),
                    PlaneSide::Front(p) => front.push(p),
                    PlaneSide::Back(p) => back.push(p),
                    PlaneSide::Across(f, b) => {
                        meter.record_split()?;
                        front.extend(f);
                        back.extend(b);
                    }
                }
            }
            if let Some(node) = self.nodes.get_mut(i) {
                node.polys.extend(here);
            }
            for (list, ahead) in [(front, true), (back, false)] {
                let Some(first) = list.first() else { continue };
                let known = self
                    .nodes
                    .get(i)
                    .and_then(|x| if ahead { x.front } else { x.back });
                let child = known.unwrap_or_else(|| {
                    let c = self.nodes.len();
                    self.nodes.push(Self::new_node(first));
                    c
                });
                if let Some(node) = self.nodes.get_mut(i) {
                    if ahead {
                        node.front = Some(child);
                    } else {
                        node.back = Some(child);
                    }
                }
                stack.push((child, list));
            }
        }
        Ok(())
    }

    fn clip(&self, polys: Vec<CsgPoly>, meter: &mut CsgMeter) -> Result<Vec<CsgPoly>, String> {
        if self.nodes.is_empty() {
            return Ok(polys);
        }
        let mut kept = Vec::new();
        let mut stack = vec![(0usize, polys)];
        while let Some((i, polys)) = stack.pop() {
            let Some(node) = self.nodes.get(i) else {
                continue;
            };
            let mut front = Vec::new();
            let mut back = Vec::new();
            for p in polys {
                meter.record_work(p.v.len())?;
                match divide_poly(node.n, node.w, p) {
                    PlaneSide::On(p, true) | PlaneSide::Front(p) => front.push(p),
                    PlaneSide::On(p, false) | PlaneSide::Back(p) => back.push(p),
                    PlaneSide::Across(f, b) => {
                        meter.record_split()?;
                        front.extend(f);
                        back.extend(b);
                    }
                }
            }
            match node.front {
                Some(c) => stack.push((c, front)),
                None => kept.extend(front),
            }
            if let Some(c) = node.back {
                stack.push((c, back));
            }
        }
        Ok(kept)
    }

    fn clip_to(&mut self, other: &BspTree, meter: &mut CsgMeter) -> Result<(), String> {
        for node in &mut self.nodes {
            node.polys = other.clip(std::mem::take(&mut node.polys), meter)?;
        }
        Ok(())
    }

    fn invert(&mut self) {
        for node in &mut self.nodes {
            node.polys.iter_mut().for_each(CsgPoly::flip);
            node.n = mul3(node.n, -1.0);
            node.w = -node.w;
            std::mem::swap(&mut node.front, &mut node.back);
        }
    }

    fn polygons(&self) -> Vec<CsgPoly> {
        self.nodes
            .iter()
            .flat_map(|n| n.polys.iter().cloned())
            .collect()
    }
}

/// Computes the 3D Axis-Aligned Bounding Box `(min_pt, max_pt)` of a triangle mesh.
pub fn mesh_bounds(tris: &[[P3; 3]]) -> (P3, P3) {
    tris.iter().flatten().fold(
        ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]),
        |(lo, hi), p| {
            (
                [lo[0].min(p[0]), lo[1].min(p[1]), lo[2].min(p[2])],
                [hi[0].max(p[0]), hi[1].max(p[1]), hi[2].max(p[2])],
            )
        },
    )
}

/// Boolean CSG Subtraction (`solid \ void`): cuts `void` out of `solid` using non-recursive BSP trees.
/// Both `solid` and `void` must be closed outward-oriented triangle meshes.
pub fn csg_subtract(
    solid: &[[P3; 3]],
    void: &[[P3; 3]],
    meter: &mut CsgMeter,
) -> Result<Vec<[P3; 3]>, String> {
    if solid.len() > MAX_CSG_TRIANGLES || void.len() > MAX_CSG_TRIANGLES {
        return Err(format!(
            "CSG operand exceeds maximum triangle count ({MAX_CSG_TRIANGLES})"
        ));
    }
    let ((lo_s, hi_s), (lo_v, hi_v)) = (mesh_bounds(solid), mesh_bounds(void));
    if (0..3).any(|k| lo_v[k] > hi_s[k] + EPS || hi_v[k] < lo_s[k] - EPS) {
        return Ok(solid.to_vec());
    }
    let mut a = BspTree::from_tris(solid, meter)?;
    let mut b = BspTree::from_tris(void, meter)?;
    a.invert();
    a.clip_to(&b, meter)?;
    b.clip_to(&a, meter)?;
    b.invert();
    b.clip_to(&a, meter)?;
    b.invert();
    a.build(b.polygons(), meter)?;
    a.invert();
    Ok(a.polygons().iter().flat_map(CsgPoly::triangles).collect())
}

/// Generates an outward-oriented 12-triangle box mesh from `lo` to `hi`.
pub fn cuboid_mesh(lo: P3, hi: P3) -> Vec<[P3; 3]> {
    let v = |x: usize, y: usize, z: usize| -> P3 {
        [[lo[0], hi[0]][x], [lo[1], hi[1]][y], [lo[2], hi[2]][z]]
    };
    [
        [v(0, 0, 1), v(1, 0, 1), v(1, 1, 1), v(0, 1, 1)], // Top (+Z)
        [v(0, 0, 0), v(0, 1, 0), v(1, 1, 0), v(1, 0, 0)], // Bottom (-Z)
        [v(0, 0, 0), v(1, 0, 0), v(1, 0, 1), v(0, 0, 1)], // South (-Y)
        [v(1, 1, 0), v(0, 1, 0), v(0, 1, 1), v(1, 1, 1)], // North (+Y)
        [v(0, 1, 0), v(0, 0, 0), v(0, 0, 1), v(0, 1, 1)], // West (-X)
        [v(1, 0, 0), v(1, 1, 0), v(1, 1, 1), v(1, 0, 1)], // East (+X)
    ]
    .iter()
    .flat_map(|[a, b, c, d]| [[*a, *b, *c], [*a, *c, *d]])
    .collect()
}

/// Extrudes a 2D counter-clockwise polygon ring vertically from `z0` to `z1` into a closed 3D mesh.
pub fn extrude_ring_mesh(ring: &[P2], z0: f64, z1: f64) -> Vec<[P3; 3]> {
    if ring.len() < 3 {
        return Vec::new();
    }
    let (lo, hi) = if z0 <= z1 { (z0, z1) } else { (z1, z0) };
    let tris_2d = triangulate(ring);
    let mut out = Vec::with_capacity(tris_2d.len() * 2 + ring.len() * 2);

    // Bottom cap (facing -Z) and Top cap (facing +Z)
    for [i, j, k] in tris_2d {
        let (a, b, c) = (ring[i], ring[j], ring[k]);
        out.push([[a.0, a.1, lo], [c.0, c.1, lo], [b.0, b.1, lo]]);
        out.push([[a.0, a.1, hi], [b.0, b.1, hi], [c.0, c.1, hi]]);
    }

    // Perimeter side quads
    let n = ring.len();
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        let a0 = [a.0, a.1, lo];
        let b0 = [b.0, b.1, lo];
        let b1 = [b.0, b.1, hi];
        let a1 = [a.0, a.1, hi];
        out.push([a0, b0, b1]);
        out.push([a0, b1, a1]);
    }
    out
}

/// Builds an orthogonal local coordinate frame `(dir, e1, e2)` along segment `a -> b`.
#[inline]
pub fn local_frame_along(a: P3, b: P3) -> (P3, P3, P3) {
    let d = unit3(sub3(b, a));
    let horiz = (d[0] * d[0] + d[1] * d[1]).sqrt();
    let e1 = if horiz < 1e-6 {
        [1.0, 0.0, 0.0]
    } else {
        [-d[1] / horiz, d[0] / horiz, 0.0]
    };
    let e2 = unit3(cross3(d, e1));
    (d, e1, e2)
}

/// Generates a closed 3D oriented prism (`width` along `e1`, `depth` along `e2`) from `a` to `b`.
pub fn extrude_box_run_mesh(a: P3, b: P3, width: f64, depth: f64) -> Vec<[P3; 3]> {
    let (_, e1, e2) = local_frame_along(a, b);
    let hw = width * 0.5;
    let hd = depth * 0.5;
    let offsets = [(-hw, -hd), (hw, -hd), (hw, hd), (-hw, hd)];

    let ring_at = |c: P3| -> [P3; 4] {
        std::array::from_fn(|k| {
            let (u, v) = offsets[k];
            add3(c, add3(mul3(e1, u), mul3(e2, v)))
        })
    };
    let ra = ring_at(a);
    let rb = ring_at(b);

    let mut tris = Vec::with_capacity(12);
    // Start cap (-d) and End cap (+d)
    tris.push([ra[0], ra[2], ra[1]]);
    tris.push([ra[0], ra[3], ra[2]]);
    tris.push([rb[0], rb[1], rb[2]]);
    tris.push([rb[0], rb[2], rb[3]]);

    // 4 Side faces (outward normal: tangent x d = outward)
    for k in 0..4 {
        let next = (k + 1) % 4;
        tris.push([ra[k], ra[next], rb[next]]);
        tris.push([ra[k], rb[next], rb[k]]);
    }
    tris
}

/// Generates a true 3D Wide-Flange (WF / I-Beam) mesh from `a` to `b` consisting of
/// top flange, bottom flange, and central web plates.
pub fn extrude_wf_section_mesh(
    a: P3,
    b: P3,
    depth: f64,
    width: f64,
    flange_thickness: f64,
    web_thickness: f64,
) -> Vec<[P3; 3]> {
    let (_, _, e2) = local_frame_along(a, b);
    let tf = flange_thickness.clamp(depth * 0.04, depth * 0.25);
    let tw = web_thickness.clamp(width * 0.03, width * 0.25);
    let z_offset = (depth - tf) * 0.5;

    let top_a = add3(a, mul3(e2, z_offset));
    let top_b = add3(b, mul3(e2, z_offset));
    let bot_a = add3(a, mul3(e2, -z_offset));
    let bot_b = add3(b, mul3(e2, -z_offset));

    let mut tris = Vec::with_capacity(36);
    tris.extend(extrude_box_run_mesh(top_a, top_b, width, tf));
    tris.extend(extrude_box_run_mesh(bot_a, bot_b, width, tf));
    tris.extend(extrude_box_run_mesh(a, b, tw, (depth - 2.0 * tf).max(tf)));
    tris
}

/// Generates a closed 3D circular tube/cylinder mesh of `sides` facets from `a` to `b`.
pub fn extrude_tube_mesh(a: P3, b: P3, diameter: f64, sides: usize) -> Vec<[P3; 3]> {
    let n_sides = sides.clamp(6, 64);
    let r = diameter * 0.5;
    let (_, e1, e2) = local_frame_along(a, b);

    let ring = |c: P3| -> Vec<P3> {
        (0..n_sides)
            .map(|k| {
                let theta = std::f64::consts::TAU * (k as f64) / (n_sides as f64);
                let (s, cs) = theta.sin_cos();
                add3(c, add3(mul3(e1, r * cs), mul3(e2, r * s)))
            })
            .collect()
    };

    let ra = ring(a);
    let rb = ring(b);
    let mut tris = Vec::with_capacity(4 * n_sides);

    for k in 0..n_sides {
        let next = (k + 1) % n_sides;
        tris.push([a, ra[next], ra[k]]);
        tris.push([b, rb[k], rb[next]]);
        tris.push([ra[k], ra[next], rb[k]]);
        tris.push([ra[next], rb[next], rb[k]]);
    }
    tris
}

/// Slices a 3D triangle mesh with an arbitrary cutting plane `n · p = w`,
/// returning all 3D line segments `(P3, P3)` where triangles cross the plane.
pub fn slice_mesh_with_plane(tris: &[[P3; 3]], plane_n: P3, plane_w: f64) -> Vec<(P3, P3)> {
    let n = unit3(plane_n);
    let mut segs = Vec::new();

    for tri in tris {
        let d = [
            dot3(n, tri[0]) - plane_w,
            dot3(n, tri[1]) - plane_w,
            dot3(n, tri[2]) - plane_w,
        ];
        let mut pts = Vec::with_capacity(2);
        for i in 0..3 {
            let j = (i + 1) % 3;
            let (di, dj) = (d[i], d[j]);
            if di.abs() <= EPS {
                pts.push(tri[i]);
            } else if (di > EPS && dj < -EPS) || (di < -EPS && dj > EPS) {
                let t = di / (di - dj);
                pts.push(add3(tri[i], mul3(sub3(tri[j], tri[i]), t)));
            }
        }
        if pts.len() >= 2 && len3(sub3(pts[0], pts[1])) > 1e-4 {
            segs.push((pts[0], pts[1]));
        }
    }
    segs
}

/// 3D Axis-Aligned Bounding Box (AABB) for BVH acceleration.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Aabb3D {
    pub min: P3,
    pub max: P3,
}

impl Aabb3D {
    pub fn from_tri(t: &[P3; 3]) -> Self {
        Self {
            min: [
                t[0][0].min(t[1][0]).min(t[2][0]),
                t[0][1].min(t[1][1]).min(t[2][1]),
                t[0][2].min(t[1][2]).min(t[2][2]),
            ],
            max: [
                t[0][0].max(t[1][0]).max(t[2][0]),
                t[0][1].max(t[1][1]).max(t[2][1]),
                t[0][2].max(t[1][2]).max(t[2][2]),
            ],
        }
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            min: [
                self.min[0].min(other.min[0]),
                self.min[1].min(other.min[1]),
                self.min[2].min(other.min[2]),
            ],
            max: [
                self.max[0].max(other.max[0]),
                self.max[1].max(other.max[1]),
                self.max[2].max(other.max[2]),
            ],
        }
    }

    pub fn centroid(&self) -> P3 {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ]
    }

    /// Slab method ray-AABB intersection test.
    pub fn intersect_ray(&self, origin: P3, inv_dir: P3, max_t: f64) -> bool {
        let mut tmin = 0.0_f64;
        let mut tmax = max_t;
        for k in 0..3 {
            let t1 = (self.min[k] - origin[k]) * inv_dir[k];
            let t2 = (self.max[k] - origin[k]) * inv_dir[k];
            let (ta, tb) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
            tmin = tmin.max(ta);
            tmax = tmax.min(tb);
            if tmax < tmin {
                return false;
            }
        }
        true
    }
}

#[derive(Clone, Debug)]
enum BvhNodeKind {
    Leaf { start: usize, count: usize },
    Internal { left: usize, right: usize },
}

#[derive(Clone, Debug)]
struct BvhNode {
    aabb: Aabb3D,
    kind: BvhNodeKind,
}

/// 3D Bounding Volume Hierarchy (BVH) for sub-millisecond Ray-Triangle Picking in 3D Viewports.
#[derive(Clone, Debug, Default)]
pub struct Bvh3D {
    nodes: Vec<BvhNode>,
    primitives: Vec<(usize, [P3; 3], Aabb3D)>,
}

impl Bvh3D {
    /// Builds a median-split BVH tree over `(element_id, triangle)` primitives.
    pub fn build(items: Vec<(usize, [P3; 3])>) -> Self {
        let mut primitives: Vec<(usize, [P3; 3], Aabb3D)> = items
            .into_iter()
            .map(|(id, tri)| (id, tri, Aabb3D::from_tri(&tri)))
            .collect();
        let mut bvh = Self {
            nodes: Vec::new(),
            primitives: Vec::new(),
        };
        if primitives.is_empty() {
            return bvh;
        }
        let n = primitives.len();
        bvh.build_recursive(&mut primitives, 0, n);
        bvh.primitives = primitives;
        bvh
    }

    fn build_recursive(
        &mut self,
        prims: &mut [(usize, [P3; 3], Aabb3D)],
        start: usize,
        end: usize,
    ) -> usize {
        let slice = &prims[start..end];
        let aabb = slice
            .iter()
            .skip(1)
            .fold(slice[0].2, |acc, item| acc.union(item.2));
        let count = end - start;
        let node_idx = self.nodes.len();
        self.nodes.push(BvhNode {
            aabb,
            kind: BvhNodeKind::Leaf { start, count },
        });

        if count <= 4 {
            return node_idx;
        }

        let extent = sub3(aabb.max, aabb.min);
        let axis = if extent[0] >= extent[1] && extent[0] >= extent[2] {
            0
        } else if extent[1] >= extent[2] {
            1
        } else {
            2
        };

        prims[start..end].sort_by(|a, b| {
            a.2.centroid()[axis].total_cmp(&b.2.centroid()[axis])
        });
        let mid = start + count / 2;
        let left = self.build_recursive(prims, start, mid);
        let right = self.build_recursive(prims, mid, end);
        self.nodes[node_idx].kind = BvhNodeKind::Internal { left, right };
        node_idx
    }

    /// Möller–Trumbore ray-triangle intersection, returning `Some((element_id, t_distance, hit_point_3d))`.
    pub fn ray_pick(&self, origin: P3, dir: P3) -> Option<(usize, f64, P3)> {
        if self.nodes.is_empty() {
            return None;
        }
        let u_dir = unit3(dir);
        let inv_dir = [
            1.0 / if u_dir[0].abs() < 1e-12 { 1e-12 } else { u_dir[0] },
            1.0 / if u_dir[1].abs() < 1e-12 { 1e-12 } else { u_dir[1] },
            1.0 / if u_dir[2].abs() < 1e-12 { 1e-12 } else { u_dir[2] },
        ];

        let mut closest_t = f64::INFINITY;
        let mut best_hit: Option<(usize, f64, P3)> = None;
        let mut stack = vec![0usize];

        while let Some(idx) = stack.pop() {
            let node = &self.nodes[idx];
            if !node.aabb.intersect_ray(origin, inv_dir, closest_t) {
                continue;
            }
            match node.kind {
                BvhNodeKind::Leaf { start, count } => {
                    for &(elem_id, ref tri, _) in &self.primitives[start..start + count] {
                        if let Some(t) = ray_intersect_triangle(origin, u_dir, tri) {
                            if t > 1e-4 && t < closest_t {
                                closest_t = t;
                                best_hit = Some((elem_id, t, add3(origin, mul3(u_dir, t))));
                            }
                        }
                    }
                }
                BvhNodeKind::Internal { left, right } => {
                    stack.push(left);
                    stack.push(right);
                }
            }
        }
        best_hit
    }
}

/// Möller–Trumbore ray-triangle intersection algorithm.
pub fn ray_intersect_triangle(origin: P3, dir: P3, tri: &[P3; 3]) -> Option<f64> {
    let e1 = sub3(tri[1], tri[0]);
    let e2 = sub3(tri[2], tri[0]);
    let h = cross3(dir, e2);
    let a = dot3(e1, h);
    if a.abs() < 1e-9 {
        return None;
    }
    let f = 1.0 / a;
    let s = sub3(origin, tri[0]);
    let u = f * dot3(s, h);
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = cross3(s, e1);
    let v = f * dot3(dir, q);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = f * dot3(e2, q);
    (t > 1e-6).then_some(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csg_boolean_subtract_door_and_window_voids_from_wall() {
        // 4000mm x 200mm x 3000mm wall (Volume = 2.4e9 mm³)
        let wall = cuboid_mesh([0.0, 0.0, 0.0], [4000.0, 200.0, 3000.0]);
        // 900mm x 400mm x 2100mm door void cutting through the wall (Cut volume = 900 * 200 * 2100 = 3.78e8 mm³)
        let door_void = cuboid_mesh([1500.0, -100.0, 0.0], [2400.0, 300.0, 2100.0]);

        let cut_wall =
            csg_subtract(&wall, &door_void, &mut CsgMeter::default()).expect("CSG subtract should succeed");
        let expected_vol = 2.4e9 - 3.78e8;
        assert!((mesh_volume(&cut_wall) - expected_vol).abs() < 1.0);
    }

    #[test]
    fn test_wf_and_tube_mesh_extrusion_and_bvh_ray_picking() {
        let wf = extrude_wf_section_mesh(
            [0.0, 0.0, 3000.0],
            [6000.0, 0.0, 3000.0],
            400.0,
            200.0,
            13.0,
            8.0,
        );
        assert_eq!(wf.len(), 36);
        assert!(mesh_volume(&wf) > 0.0);

        let tube = extrude_tube_mesh([0.0, 2000.0, 0.0], [0.0, 2000.0, 4000.0], 300.0, 24);
        assert_eq!(tube.len(), 96);

        let mut prims = Vec::new();
        for t in wf {
            prims.push((101, t));
        }
        for t in tube {
            prims.push((202, t));
        }
        let bvh = Bvh3D::build(prims);

        let hit_wf = bvh
            .ray_pick([3000.0, -2000.0, 3000.0], [0.0, 1.0, 0.0])
            .expect("Ray should hit WF beam #101");
        assert_eq!(hit_wf.0, 101);

        let hit_tube = bvh
            .ray_pick([-2000.0, 2000.0, 2000.0], [1.0, 0.0, 0.0])
            .expect("Ray should hit CHS column #202");
        assert_eq!(hit_tube.0, 202);
    }
}
