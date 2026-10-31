//! reliability.rs — Structural Reliability (FOSM Beta-Index), FEMA P-58 Seismic Fragility Curves,
//! and Exact 3D Spatial Clash Detection Engine for EigenGraph.

use crate::core::model::StructuralModel;
use crate::core::solver::FemSolver;

/// High-precision Standard Normal Cumulative Distribution Function Phi(z)
/// using Abramowitz & Stegun Formula 7.1.26 (|error| < 1.5e-7).
pub fn standard_normal_cdf(z: f64) -> f64 {
    let sign = if z < 0.0 { -1.0 } else { 1.0 };
    let x = z.abs() / std::f64::consts::SQRT_2;

    let a1 =  0.254829592;
    let a2 = -0.284496736;
    let a3 =  1.421413741;
    let a4 = -1.453152027;
    let a5 =  1.061405429;
    let p  =  0.3275911;

    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

    0.5 * (1.0 + sign * y)
}

/// Log-Normal FOSM Reliability Assessment for a single structural member
#[derive(Debug, Clone)]
pub struct MemberReliabilityResult {
    pub member_id: usize,
    pub member_name: String,
    pub demand_kn: f64,
    pub capacity_kn: f64,
    pub beta_index: f64,
    pub failure_probability: f64,
    pub meets_target_beta: bool,
}

/// Discrete Point on a Multi-State Seismic / Collapse Fragility Curve (FEMA P-58)
#[derive(Debug, Clone)]
pub struct FragilityCurvePoint {
    pub pga_g: f64,
    pub prob_slight: f64,
    pub prob_moderate: f64,
    pub prob_extensive: f64,
    pub prob_complete_collapse: f64,
}

/// 3D Geometric Clash Detection Result between two non-incident structural members
#[derive(Debug, Clone)]
pub struct SpatialClash {
    pub member_a_id: usize,
    pub member_b_id: usize,
    pub member_a_name: String,
    pub member_b_name: String,
    pub centerline_distance_m: f64,
    pub net_clearance_m: f64,
    pub closest_point_xyz: [f64; 3],
    pub is_hard_clash: bool,
}

/// Complete Structural Reliability & Spatial Clash Report
#[derive(Debug, Clone)]
pub struct ReliabilityAndClashReport {
    pub min_beta_index: f64,
    pub max_failure_probability: f64,
    pub target_beta: f64,
    pub member_reliabilities: Vec<MemberReliabilityResult>,
    pub fragility_curve: Vec<FragilityCurvePoint>,
    pub detected_clashes: Vec<SpatialClash>,
}

pub struct ReliabilityEngine;

impl ReliabilityEngine {
    /// Computes the Log-Normal Reliability Index beta and Failure Probability Pf = Phi(-beta)
    /// given mean resistance mu_r, mean demand mu_s, resistance COV v_r, and demand COV v_s.
    pub fn compute_lognormal_beta(mu_r: f64, mu_s: f64, v_r: f64, v_s: f64) -> (f64, f64) {
        let safe_s = mu_s.abs().max(1e-6);
        let safe_r = mu_r.abs().max(1e-3);

        let var_term = ((1.0 + v_s * v_s) / (1.0 + v_r * v_r)).sqrt();
        let numerator = ((safe_r / safe_s) * var_term).ln();
        let denom = ((1.0 + v_r * v_r) * (1.0 + v_s * v_s)).ln().sqrt().max(1e-6);

        let beta = (numerator / denom).clamp(-5.0, 10.0);
        let pf = standard_normal_cdf(-beta);
        (beta, pf)
    }

    /// Generates a FEMA P-58 / ASCE 41 Multi-State Fragility Curve calibrated to system redundancy (min_ari)
    pub fn generate_fragility_curve(min_ari: f64, steps: usize) -> Vec<FragilityCurvePoint> {
        let ari_factor = min_ari.clamp(0.25, 1.0);
        // Median PGA thresholds (in g) scaled by structural redundancy
        let theta_slight = 0.18 * ari_factor;
        let theta_moderate = 0.38 * ari_factor;
        let theta_extensive = 0.68 * ari_factor;
        let theta_collapse = 1.10 * ari_factor;

        let beta_disp = 0.40; // Log-normal standard deviation (dispersion)
        let n = steps.max(5);
        let mut curve = Vec::with_capacity(n);

        for i in 0..=n {
            let pga = 0.05 + (1.45 * (i as f64) / (n as f64));
            let p_slight = standard_normal_cdf((pga / theta_slight).ln() / beta_disp);
            let p_mod = standard_normal_cdf((pga / theta_moderate).ln() / beta_disp);
            let p_ext = standard_normal_cdf((pga / theta_extensive).ln() / beta_disp);
            let p_col = standard_normal_cdf((pga / theta_collapse).ln() / beta_disp);

            curve.push(FragilityCurvePoint {
                pga_g: pga,
                prob_slight: p_slight,
                prob_moderate: p_mod,
                prob_extensive: p_ext,
                prob_complete_collapse: p_col,
            });
        }

        curve
    }

    /// Computes exact minimum 3D distance between two finite line segments P(s) = p1 + s*(q1-p1) and Q(t) = p2 + t*(q2-p2)
    /// for s, t in [0, 1], returning (distance, midpoint_xyz).
    pub fn segment_to_segment_distance_3d(
        p1: [f64; 3],
        q1: [f64; 3],
        p2: [f64; 3],
        q2: [f64; 3],
    ) -> (f64, [f64; 3]) {
        let d1 = [q1[0] - p1[0], q1[1] - p1[1], q1[2] - p1[2]];
        let d2 = [q2[0] - p2[0], q2[1] - p2[1], q2[2] - p2[2]];
        let r = [p1[0] - p2[0], p1[1] - p2[1], p1[2] - p2[2]];

        let a = d1[0] * d1[0] + d1[1] * d1[1] + d1[2] * d1[2];
        let e = d2[0] * d2[0] + d2[1] * d2[1] + d2[2] * d2[2];
        let f = d2[0] * r[0] + d2[1] * r[1] + d2[2] * r[2];

        let mut s: f64;
        let mut t: f64;

        if a <= 1e-12 && e <= 1e-12 {
            s = 0.0;
            t = 0.0;
        } else if a <= 1e-12 {
            s = 0.0;
            t = (f / e).clamp(0.0, 1.0);
        } else {
            let c = d1[0] * r[0] + d1[1] * r[1] + d1[2] * r[2];
            if e <= 1e-12 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else {
                let b = d1[0] * d2[0] + d1[1] * d2[1] + d1[2] * d2[2];
                let denom = a * e - b * b;

                if denom != 0.0 {
                    s = ((b * f - c * e) / denom).clamp(0.0, 1.0);
                } else {
                    s = 0.0;
                }

                t = (b * s + f) / e;
                if t < 0.0 {
                    t = 0.0;
                    s = (-c / a).clamp(0.0, 1.0);
                } else if t > 1.0 {
                    t = 1.0;
                    s = ((b - c) / a).clamp(0.0, 1.0);
                }
            }
        }

        let c1 = [p1[0] + d1[0] * s, p1[1] + d1[1] * s, p1[2] + d1[2] * s];
        let c2 = [p2[0] + d2[0] * t, p2[1] + d2[1] * t, p2[2] + d2[2] * t];
        let diff = [c1[0] - c2[0], c1[1] - c2[1], c1[2] - c2[2]];
        let dist = (diff[0] * diff[0] + diff[1] * diff[1] + diff[2] * diff[2]).sqrt();
        let mid = [
            0.5 * (c1[0] + c2[0]),
            0.5 * (c1[1] + c2[1]),
            0.5 * (c1[2] + c2[2]),
        ];

        (dist, mid)
    }

    /// Scans all non-incident member pairs in a 3D StructuralModel for physical interference (clash detection).
    pub fn detect_3d_spatial_clashes(model: &StructuralModel, clearance_tol_m: f64) -> Vec<SpatialClash> {
        let mut clashes = Vec::new();
        let m = model.elements.len();

        for i in 0..m {
            for j in (i + 1)..m {
                let e1 = &model.elements[i];
                let e2 = &model.elements[j];

                // Skip elements that share a common topological connection node
                if e1.start_node == e2.start_node
                    || e1.start_node == e2.end_node
                    || e1.end_node == e2.start_node
                    || e1.end_node == e2.end_node
                {
                    continue;
                }

                if let (Some(n1a), Some(n1b), Some(n2a), Some(n2b)) = (
                    model.find_node(e1.start_node),
                    model.find_node(e1.end_node),
                    model.find_node(e2.start_node),
                    model.find_node(e2.end_node),
                ) {
                    let p1 = [n1a.x, n1a.y, n1a.z];
                    let q1 = [n1b.x, n1b.y, n1b.z];
                    let p2 = [n2a.x, n2a.y, n2a.z];
                    let q2 = [n2b.x, n2b.y, n2b.z];

                    let (center_dist, pt) = Self::segment_to_segment_distance_3d(p1, q1, p2, q2);
                    let r1 = 0.5 * e1.section.depth.max(e1.section.width);
                    let r2 = 0.5 * e2.section.depth.max(e2.section.width);
                    let net_clearance = center_dist - (r1 + r2);

                    if net_clearance <= clearance_tol_m {
                        clashes.push(SpatialClash {
                            member_a_id: e1.id,
                            member_b_id: e2.id,
                            member_a_name: e1.name.clone(),
                            member_b_name: e2.name.clone(),
                            centerline_distance_m: center_dist,
                            net_clearance_m: net_clearance,
                            closest_point_xyz: pt,
                            is_hard_clash: net_clearance <= 0.0,
                        });
                    }
                }
            }
        }

        clashes
    }

    /// Executes full FOSM Reliability, Fragility Curve, and 3D Clash Assessment
    pub fn evaluate_full_reliability(
        model: &StructuralModel,
        target_beta: f64,
        min_ari: f64,
    ) -> Result<ReliabilityAndClashReport, String> {
        let sol = FemSolver::solve(model)?;
        let v_r = 0.10; // Resistance COV for structural steel (JCSS / AISC)
        let v_s = 0.15; // Load effect COV

        let mut member_reliabilities = Vec::with_capacity(model.elements.len());
        let mut min_beta = 10.0f64;
        let mut max_pf = 0.0f64;

        for (i, elem) in model.elements.iter().enumerate() {
            let demand = sol.member_axial_forces[i].abs();
            let capacity = sol.member_capacities[i] / 0.90; // Nominal unfactored mean resistance
            let (beta, pf) = Self::compute_lognormal_beta(capacity * 1.08, demand.max(1000.0), v_r, v_s);

            min_beta = min_beta.min(beta);
            max_pf = max_pf.max(pf);

            member_reliabilities.push(MemberReliabilityResult {
                member_id: elem.id,
                member_name: elem.name.clone(),
                demand_kn: demand / 1000.0,
                capacity_kn: capacity / 1000.0,
                beta_index: beta,
                failure_probability: pf,
                meets_target_beta: beta >= target_beta,
            });
        }

        let fragility_curve = Self::generate_fragility_curve(min_ari, 10);
        let detected_clashes = Self::detect_3d_spatial_clashes(model, 0.05);

        Ok(ReliabilityAndClashReport {
            min_beta_index: min_beta,
            max_failure_probability: max_pf,
            target_beta,
            member_reliabilities,
            fragility_curve,
            detected_clashes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::{CrossSection, Element3D, Node3D};

    #[test]
    fn test_standard_normal_cdf_and_fosm_beta() {
        assert!((standard_normal_cdf(0.0) - 0.50).abs() < 1e-6);
        assert!(standard_normal_cdf(3.8) > 0.9999);
        assert!(standard_normal_cdf(-3.8) < 0.0002);

        let (beta, pf) = ReliabilityEngine::compute_lognormal_beta(500.0, 200.0, 0.10, 0.15);
        assert!(beta > 3.8);
        assert!(pf < 1e-4);
    }

    #[test]
    fn test_3d_spatial_clash_detection_intersecting_diagonals() {
        let mut model = StructuralModel::new("X-Bracing Unconnected Diagonals");
        model.add_node(Node3D::new(0, 0.0, 0.0, 0.0));
        model.add_node(Node3D::new(1, 4.0, 4.0, 0.0));
        model.add_node(Node3D::new(2, 0.0, 4.0, 0.0));
        model.add_node(Node3D::new(3, 4.0, 0.0, 0.0));

        // Diagonal 1: (0,0,0) -> (4,4,0), Diagonal 2: (0,4,0) -> (4,0,0)
        // They intersect at (2.0, 2.0, 0.0) without a shared node!
        model.add_element(Element3D::new(1, "Diag-1", 0, 1).with_section(CrossSection::wf_200x100()));
        model.add_element(Element3D::new(2, "Diag-2", 2, 3).with_section(CrossSection::wf_200x100()));

        let clashes = ReliabilityEngine::detect_3d_spatial_clashes(&model, 0.02);
        assert_eq!(clashes.len(), 1);
        assert!(clashes[0].is_hard_clash);
        assert!(clashes[0].centerline_distance_m < 1e-6);
        assert!((clashes[0].closest_point_xyz[0] - 2.0).abs() < 1e-6);
        assert!((clashes[0].closest_point_xyz[1] - 2.0).abs() < 1e-6);
    }
}
