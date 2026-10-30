//! ari.rs — Adjusted Redundancy Index (ARI) & SVD Kinematic Mechanism Guard.
//! Implements doctoral research formulation for Progressive Collapse and Indeterminacy Analytics.

use crate::core::linalg::Matrix;
use crate::core::model::StructuralModel;
use crate::core::solver::FemSolver;

/// Diagnostic report from the SVD Kinematic Guard
#[derive(Debug, Clone)]
pub struct KinematicGuardReport {
    /// Number of active unconstrained degrees of freedom
    pub active_dofs: usize,
    /// Numerical rank of the stiffness matrix K
    pub rank: usize,
    /// Dimension of the nullspace (rank deficit = mechanisms count)
    pub nullspace_dimension: usize,
    /// True if rank == active_dofs (no unconstrained kinematic mechanisms)
    pub is_stable: bool,
    /// Largest singular value sigma_max
    pub sigma_max: f64,
    /// Smallest non-zero singular value
    pub sigma_min: f64,
    /// Matrix condition number kappa = sigma_max / sigma_min
    pub condition_number: f64,
    /// Kinematic mechanism mode shapes (nullspace vectors)
    pub mechanism_modes: Vec<Vec<f64>>,
}

/// Member-by-Member Redundancy & Criticality Evaluation
#[derive(Debug, Clone)]
pub struct MemberRedundancyResult {
    pub member_id: usize,
    pub member_name: String,
    /// Initial baseline axial force in kN
    pub force_kn: f64,
    /// Capacity in kN
    pub capacity_kn: f64,
    /// Initial baseline utilization ratio (0.0 to 1.0)
    pub utilization_initial: f64,
    /// Maximum utilization in surrounding members after this member is removed
    pub max_redistribution_utilization: f64,
    /// Adjusted Redundancy Index ARI_i (0.0 to 1.0, higher is more redundant)
    pub ari: f64,
    /// Redundancy Ratio RR_i
    pub rr: f64,
    /// Criticality Index CI_i (1.0 - RR_i)
    pub ci: f64,
    /// SVD safety margin (0.0 to 1.0)
    pub svd_margin: f64,
    /// True if member loss causes collapse or mechanism
    pub is_critical: bool,
}

/// Complete Structural Redundancy Analytics Report
#[derive(Debug, Clone)]
pub struct RedundancyAssessment {
    pub kinematic_guard: KinematicGuardReport,
    pub member_results: Vec<MemberRedundancyResult>,
    /// Minimum ARI across all members in the structure
    pub min_system_ari: f64,
    /// Average ARI of the structure
    pub average_ari: f64,
    /// Count of critical members with ARI < 0.70
    pub critical_members_count: usize,
}

pub struct AriEngine;

impl AriEngine {
    /// Runs full SVD Kinematic Guard verification on a global stiffness matrix
    pub fn verify_kinematics(global_k: &Matrix, tol: f64) -> KinematicGuardReport {
        let (rank, null_dim, null_vectors) = global_k.rank_and_nullspace(tol);
        let (_, s, _) = global_k.svd();

        let sigma_max = s.first().copied().unwrap_or(0.0);
        let sigma_min = s.iter().filter(|&&v| v > tol * sigma_max).last().copied().unwrap_or(0.0);
        let condition_number = if sigma_min > 1e-15 { sigma_max / sigma_min } else { f64::INFINITY };

        KinematicGuardReport {
            active_dofs: global_k.rows,
            rank,
            nullspace_dimension: null_dim,
            is_stable: null_dim == 0,
            sigma_max,
            sigma_min,
            condition_number,
            mechanism_modes: null_vectors,
        }
    }

    /// Evaluates the complete Adjusted Redundancy Index (ARI) for the structural model
    pub fn evaluate_redundancy(model: &StructuralModel) -> Result<RedundancyAssessment, String> {
        // 1. Baseline analysis of intact structure
        let baseline = FemSolver::solve(model)?;
        let kinematic_guard = Self::verify_kinematics(&baseline.k_free, 1e-7);

        let mut member_results = Vec::new();

        for (elem_idx, elem) in model.elements.iter().enumerate() {
            let f_init = baseline.member_axial_forces[elem_idx];
            let cap = baseline.member_capacities[elem_idx];
            let u_init = baseline.member_utilizations[elem_idx];

            // 2. Simulate member removal: create perturbed model without element i
            let mut perturbed_model = model.clone();
            perturbed_model.elements.remove(elem_idx);

            // Re-solve damaged model
            match FemSolver::solve(&perturbed_model) {
                Ok(damaged_result) => {
                    // Check if remaining structure became a kinematic mechanism
                    let damaged_guard = Self::verify_kinematics(&damaged_result.k_free, 1e-7);
                    if !damaged_guard.is_stable {
                        // Collapse mechanism triggered upon member removal
                        member_results.push(MemberRedundancyResult {
                            member_id: elem.id,
                            member_name: elem.name.clone(),
                            force_kn: f_init / 1000.0,
                            capacity_kn: cap / 1000.0,
                            utilization_initial: u_init,
                            max_redistribution_utilization: 1.0,
                            ari: 0.0,
                            rr: 0.0,
                            ci: 1.0,
                            svd_margin: 0.0,
                            is_critical: true,
                        });
                        continue;
                    }

                    // Find maximum utilization among remaining members
                    let max_u_damaged = damaged_result.member_utilizations.iter().copied().fold(0.0, f64::max);

                    // Formulate Adjusted Redundancy Index (ARI)
                    // ARI = 1.0 - (U_damaged - U_intact) / (1.0 - U_intact)
                    let delta_u = (max_u_damaged - u_init).max(0.0);
                    let reserve = (1.0 - u_init).max(0.05);
                    let ari_val = (1.0 - (delta_u / reserve)).clamp(0.0, 1.0);
                    let rr_val = (1.0 - (max_u_damaged - u_init).abs()).clamp(0.0, 1.0);
                    let ci_val = 1.0 - rr_val;
                    let svd_margin = (1.0 / (damaged_guard.condition_number.log10().max(1.0) / 12.0)).clamp(0.0, 1.0);

                    member_results.push(MemberRedundancyResult {
                        member_id: elem.id,
                        member_name: elem.name.clone(),
                        force_kn: f_init / 1000.0,
                        capacity_kn: cap / 1000.0,
                        utilization_initial: u_init,
                        max_redistribution_utilization: max_u_damaged,
                        ari: ari_val,
                        rr: rr_val,
                        ci: ci_val,
                        svd_margin,
                        is_critical: ari_val < 0.70 || max_u_damaged >= 1.0,
                    });
                }
                Err(_) => {
                    // Singular matrix: structure collapses without this member
                    member_results.push(MemberRedundancyResult {
                        member_id: elem.id,
                        member_name: elem.name.clone(),
                        force_kn: f_init / 1000.0,
                        capacity_kn: cap / 1000.0,
                        utilization_initial: u_init,
                        max_redistribution_utilization: 1.0,
                        ari: 0.0,
                        rr: 0.0,
                        ci: 1.0,
                        svd_margin: 0.0,
                        is_critical: true,
                    });
                }
            }
        }

        let min_system_ari = member_results.iter().map(|m| m.ari).fold(1.0, f64::min);
        let average_ari = if !member_results.is_empty() {
            member_results.iter().map(|m| m.ari).sum::<f64>() / (member_results.len() as f64)
        } else {
            0.0
        };
        let critical_members_count = member_results.iter().filter(|m| m.is_critical).count();

        Ok(RedundancyAssessment {
            kinematic_guard,
            member_results,
            min_system_ari,
            average_ari,
            critical_members_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::{BoundaryFixity, CrossSection, Element3D, ElementType, Material, NodalLoad, Node3D};

    #[test]
    fn test_ari_indeterminate_truss() {
        let mut model = StructuralModel::new("Redundant 3-bar planar truss");

        // Nodes
        // Node 0: (0, 0) - Pin support
        // Node 1: (4, 0) - Roller Y support
        // Node 2: (2, 2) - Loaded Node
        model.add_node(Node3D::new(0, 0.0, 0.0, 0.0).with_fixity(BoundaryFixity::fixed()));
        model.add_node(Node3D::new(1, 4.0, 0.0, 0.0).with_fixity(BoundaryFixity::fixed()));
        // Node 2: (2, 2) - Loaded Node (Free in X and Y, restrained in out-of-plane Z and rotations)
        let mut n2_fixity = BoundaryFixity::free();
        n2_fixity.uz = true;
        n2_fixity.rx = true;
        n2_fixity.ry = true;
        n2_fixity.rz = true;
        model.add_node(Node3D::new(2, 2.0, 2.0, 0.0).with_fixity(n2_fixity));

        // Elements
        let mut e1 = Element3D::new(1, "Left Diagonal", 0, 2);
        e1.element_type = ElementType::Truss3D;
        e1.section = CrossSection::w14x90();
        e1.material = Material::structural_steel();
        model.add_element(e1);

        let mut e2 = Element3D::new(2, "Right Diagonal", 1, 2);
        e2.element_type = ElementType::Truss3D;
        e2.section = CrossSection::w14x90();
        e2.material = Material::structural_steel();
        model.add_element(e2);

        // Downward load on Node 2
        model.add_load(NodalLoad::force_y(2, -50_000.0)); // -50 kN

        let report = AriEngine::evaluate_redundancy(&model).expect("Should evaluate redundancy");
        assert!(report.kinematic_guard.is_stable);
        assert_eq!(report.member_results.len(), 2);
    }
}
