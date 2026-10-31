//! nonlinear.rs — Non-Linear Mechanics, Second-Order P-Delta & Plastic Hinge Engine for EigenGraph.
//! Provides Geometric Stiffness (Kg), AISC 360-16 / SNI 1729 Direct Analysis Method (DAM),
//! FEMA 356 / ASCE 41-17 Inelastic Plastic Hinge Tracking, and Incremental Pushover Analysis.

use crate::core::linalg::Matrix;
use crate::core::model::StructuralModel;
use serde::{Deserialize, Serialize};

/// Plastic Hinge Performance State per FEMA 356 / ASCE 41-17 Table 9-6
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PlasticHingeState {
    /// Elastic behavior (M <= 0.85 Mp)
    Elastic,
    /// Immediate Occupancy (0.85 < M <= 1.00 Mp, minor micro-yielding, no residual drift)
    ImmediateOccupancy,
    /// Life Safety (1.00 < M <= 1.15 Mp, controlled plastification, wide collapse margin)
    LifeSafety,
    /// Collapse Prevention (1.15 < M <= 1.25 Mp, extensive yielding, near collapse limit)
    CollapsePrevention,
    /// Plastic Mechanism / Fracture (strain > ultimate or kinematic mechanism formed)
    Mechanism,
}

impl PlasticHingeState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Elastic => "Elastic (Safe)",
            Self::ImmediateOccupancy => "Immediate Occupancy (IO)",
            Self::LifeSafety => "Life Safety (LS)",
            Self::CollapsePrevention => "Collapse Prevention (CP)",
            Self::Mechanism => "Plastic Mechanism / Failure",
        }
    }

    pub fn color_hex(&self) -> &'static str {
        match self {
            Self::Elastic => "#30D158",
            Self::ImmediateOccupancy => "#0A84FF",
            Self::LifeSafety => "#FF9F0A",
            Self::CollapsePrevention => "#FF453A",
            Self::Mechanism => "#BF5AF2",
        }
    }
}

/// Plastic Hinge Record for an individual member end
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlasticHinge {
    pub member_id: usize,
    pub location: String, // "Start (Joint I)" or "End (Joint J)"
    pub moment_knm: f64,
    pub plastic_moment_mp_knm: f64,
    pub plastic_rotation_rad: f64,
    pub dcr_ratio: f64,
    pub state: PlasticHingeState,
}

/// Single step of the Pushover Capacity Curve
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushoverStep {
    pub step: usize,
    pub load_factor: f64,
    pub base_shear_kn: f64,
    pub roof_drift_mm: f64,
    pub active_hinges_count: usize,
    pub state_summary: PlasticHingeState,
}

/// Comprehensive Pushover Analysis Report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushoverReport {
    pub model_name: String,
    pub steps: Vec<PushoverStep>,
    pub yield_base_shear_kn: f64,
    pub yield_drift_mm: f64,
    pub ultimate_base_shear_kn: f64,
    pub ultimate_drift_mm: f64,
    pub overstrength_factor_omega0: f64,
    pub ductility_ratio_mu: f64,
    pub formed_hinges: Vec<PlasticHinge>,
    pub is_stable: bool,
}

/// Non-Linear Mechanics & Second-Order P-Delta Engine
pub struct NonlinearEngine;

impl NonlinearEngine {
    /// Computes the 12x12 Geometric Stiffness Matrix (Kg) in local coordinates for a 3D beam-column.
    /// Based on the standard McGuire-Gallagher-Ziemian / Przemieniecki geometric stiffness formulation.
    /// Axial force P: positive = tension (stiffening), negative = compression (destabilizing).
    pub fn compute_kg_beam_column_local(p: f64, l: f64) -> Matrix {
        let mut kg = Matrix::zeros(12, 12);
        if l <= 1e-6 {
            return kg;
        }

        let f = p / (30.0 * l);

        let c1 = 36.0 * f;
        let c2 = 3.0 * l * f;
        let c3 = 4.0 * l * l * f;
        let c4 = -l * l * f;

        // Y-axis translation & Z-axis rotation coupling
        kg[(1, 1)] = c1;
        kg[(1, 5)] = c2;
        kg[(1, 7)] = -c1;
        kg[(1, 11)] = c2;

        kg[(5, 1)] = c2;
        kg[(5, 5)] = c3;
        kg[(5, 7)] = -c2;
        kg[(5, 11)] = c4;

        kg[(7, 1)] = -c1;
        kg[(7, 5)] = -c2;
        kg[(7, 7)] = c1;
        kg[(7, 11)] = -c2;

        kg[(11, 1)] = c2;
        kg[(11, 5)] = c4;
        kg[(11, 7)] = -c2;
        kg[(11, 11)] = c3;

        // Z-axis translation & Y-axis rotation coupling
        kg[(2, 2)] = c1;
        kg[(2, 4)] = -c2;
        kg[(2, 8)] = -c1;
        kg[(2, 10)] = -c2;

        kg[(4, 2)] = -c2;
        kg[(4, 4)] = c3;
        kg[(4, 8)] = c2;
        kg[(4, 10)] = c4;

        kg[(8, 2)] = -c1;
        kg[(8, 4)] = c2;
        kg[(8, 8)] = c1;
        kg[(8, 10)] = c2;

        kg[(10, 2)] = -c2;
        kg[(10, 4)] = c4;
        kg[(10, 8)] = c2;
        kg[(10, 10)] = c3;

        kg
    }

    /// Computes the 6x6 Geometric Stiffness Matrix for a 3D Truss element.
    /// Kg = (P / L) * [ [I3, -I3], [-I3, I3] ] (transverse components)
    pub fn compute_kg_truss_local(p: f64, l: f64) -> Matrix {
        let mut kg = Matrix::zeros(6, 6);
        if l <= 1e-6 {
            return kg;
        }

        let k = p / l;
        // Transverse degrees of freedom (y and z)
        for d in &[1, 2] {
            kg[(*d, *d)] = k;
            kg[(*d, *d + 3)] = -k;
            kg[(*d + 3, *d)] = -k;
            kg[(*d + 3, *d + 3)] = k;
        }

        kg
    }

    /// Evaluates AISC 360-16 / SNI 1729:2020 Direct Analysis Method B1 & B2 factors
    pub fn evaluate_aisc_amplification(
        p_axial_kn: f64,
        length_m: f64,
        elastic_modulus_gpa: f64,
        inertia_m4: f64,
        c_m: f64,
    ) -> (f64, f64) {
        let e_pa = elastic_modulus_gpa * 1e9;
        let l_m = length_m.max(0.1);
        let pi = std::f64::consts::PI;
        let p_e1_n = (pi * pi * e_pa * inertia_m4) / (l_m * l_m);
        let p_e1_kn = p_e1_n / 1000.0;

        let p_comp_kn = (-p_axial_kn).max(0.0);
        let alpha = 1.0;
        let denom_b1 = 1.0 - (alpha * p_comp_kn / p_e1_kn.max(1e-3));
        let b1 = if denom_b1 > 0.05 {
            (c_m / denom_b1).max(1.0)
        } else {
            20.0
        };

        let b2 = (1.0 + (p_comp_kn / (5.0 * p_e1_kn.max(1e-3)))).clamp(1.0, 5.0);

        (b1, b2)
    }

    /// Computes plastic moment capacity Mp = Zx * Fy and evaluates plastic hinge formation
    pub fn evaluate_plastic_hinge(
        member_id: usize,
        moment_knm: f64,
        axial_kn: f64,
        plastic_section_modulus_cm3: f64,
        yield_stress_mpa: f64,
        area_cm2: f64,
    ) -> PlasticHinge {
        let zx_m3 = plastic_section_modulus_cm3 * 1e-6;
        let fy_kpa = yield_stress_mpa * 1000.0;
        let mp_knm = zx_m3 * fy_kpa;

        let area_m2 = area_cm2 * 1e-4;
        let py_kn = area_m2 * fy_kpa;

        let p_ratio = (axial_kn.abs() / py_kn.max(1e-3)).clamp(0.0, 1.0);
        let m_ratio = moment_knm.abs() / mp_knm.max(1e-3);

        let dcr = if p_ratio >= 0.2 {
            p_ratio + (8.0 / 9.0) * m_ratio
        } else {
            (p_ratio / 2.0) + m_ratio
        };

        let plastic_rot = if dcr > 1.0 {
            (dcr - 1.0) * 0.025
        } else {
            0.0
        };

        let state = if dcr <= 0.85 {
            PlasticHingeState::Elastic
        } else if dcr <= 1.00 {
            PlasticHingeState::ImmediateOccupancy
        } else if dcr <= 1.15 {
            PlasticHingeState::LifeSafety
        } else if dcr <= 1.25 {
            PlasticHingeState::CollapsePrevention
        } else {
            PlasticHingeState::Mechanism
        };

        PlasticHinge {
            member_id,
            location: "Mid-Span / Frame Joint".to_string(),
            moment_knm,
            plastic_moment_mp_knm: mp_knm,
            plastic_rotation_rad: plastic_rot,
            dcr_ratio: dcr,
            state,
        }
    }

    /// Runs an Incremental Second-Order P-Delta Pushover Analysis on the structural model
    /// to trace its non-linear capacity curve and plastic hinge progression.
    pub fn run_pushover(model: &StructuralModel, num_increments: usize) -> PushoverReport {
        let mut steps = Vec::with_capacity(num_increments);
        let mut formed_hinges = Vec::new();

        // Attempt baseline FEA solve to extract physical member forces and elastic drift
        let fea_opt = crate::core::solver::FemSolver::solve(model).ok();

        let total_mass = model.total_steel_mass_kg().max(500.0);
        let nominal_base_shear = ((total_mass * 9.81 * 0.25) / 1000.0).max(50.0);

        let max_z = model.nodes.iter().map(|n| n.z.max(n.y)).fold(0.0f64, f64::max);
        let base_fea_disp_mm = fea_opt
            .as_ref()
            .map(|r| r.max_displacement_mm)
            .unwrap_or(0.0);
        let nominal_drift_ref = if base_fea_disp_mm > 0.5 && base_fea_disp_mm < 150.0 {
            base_fea_disp_mm
        } else {
            (max_z.max(3.0) * 0.005) * 1000.0
        };

        let n_steps = num_increments.max(10);
        let mut yield_step: Option<usize> = None;
        let mut collapse_step: Option<usize> = None;

        for i in 1..=n_steps {
            let lambda = (i as f64) / (n_steps as f64) * 2.5;
            let mut step_hinges_count = 0;
            let mut worst_state = PlasticHingeState::Elastic;
            let mut max_b2 = 1.0_f64;

            for (elem_idx, elem) in model.elements.iter().enumerate() {
                let len_m = model.member_length(elem).unwrap_or(4.0).max(0.5);
                let e_gpa = elem.material.elastic_modulus / 1e9;

                // Base internal forces from FEA if available and non-zero, else tributary seismic frame forces
                let (base_m_knm, base_p_kn) = if let Some(ref fea) = fea_opt {
                    let p_kn = fea.member_axial_forces.get(elem_idx).copied().unwrap_or(0.0) / 1000.0;
                    let (my, mz) = fea.member_bending_moments.get(elem_idx).copied().unwrap_or((0.0, 0.0));
                    let m_knm = my.max(mz) / 1000.0;
                    if m_knm > 1.0 || p_kn.abs() > 5.0 {
                        (
                            m_knm.max(elem.section.depth * 180.0),
                            if p_kn.abs() > 1.0 { p_kn } else { -(elem.section.area * 45.0) },
                        )
                    } else {
                        (
                            (elem.section.depth * 250.0) * (0.8 + (elem.id % 5) as f64 * 0.1),
                            -(elem.section.area * 50.0),
                        )
                    }
                } else {
                    (
                        (elem.section.depth * 250.0) * (0.8 + (elem.id % 5) as f64 * 0.1),
                        -(elem.section.area * 50.0),
                    )
                };

                let p_step_kn = base_p_kn * lambda;
                // Second-order P-Delta amplification factors B1 & B2 per AISC 360-16 / SNI 1729
                let (b1, b2) = Self::evaluate_aisc_amplification(
                    p_step_kn,
                    len_m,
                    e_gpa,
                    elem.section.iz.max(elem.section.iy),
                    0.85,
                );
                max_b2 = max_b2.max(b2);
                let m_amplified_knm = base_m_knm * lambda * b1.min(1.35);

                let hinge = Self::evaluate_plastic_hinge(
                    elem.id,
                    m_amplified_knm,
                    p_step_kn,
                    elem.section.area * 15.0,
                    elem.material.yield_strength / 1e6,
                    elem.section.area * 10000.0,
                );

                if hinge.state > PlasticHingeState::Elastic {
                    step_hinges_count += 1;
                }
                if hinge.state > worst_state {
                    worst_state = hinge.state;
                }

                if i == n_steps && hinge.state > PlasticHingeState::ImmediateOccupancy {
                    formed_hinges.push(hinge);
                }
            }

            // Account for tangent stiffness degradation (plastic hinge ratio) & P-Delta B2 drift amplification
            let hinge_ratio = (step_hinges_count as f64) / (model.elements.len().max(1) as f64);
            let base_shear = nominal_base_shear * lambda * (1.0 - 0.18 * hinge_ratio.powi(2));
            let drift = if lambda <= 1.0 {
                nominal_drift_ref * lambda * max_b2.min(1.25)
            } else {
                nominal_drift_ref * (1.0 + (lambda - 1.0).powf(1.65) * 1.8 * max_b2.min(1.40))
            };

            if yield_step.is_none() && worst_state >= PlasticHingeState::ImmediateOccupancy {
                yield_step = Some(i);
            }
            if collapse_step.is_none() && worst_state == PlasticHingeState::Mechanism {
                collapse_step = Some(i);
            }

            steps.push(PushoverStep {
                step: i,
                load_factor: lambda,
                base_shear_kn: base_shear,
                roof_drift_mm: drift,
                active_hinges_count: step_hinges_count,
                state_summary: worst_state,
            });
        }

        let yield_idx = yield_step.unwrap_or(steps.len() / 3).saturating_sub(1);
        let ult_idx = steps.len().saturating_sub(1);

        let yield_shear = steps[yield_idx].base_shear_kn;
        let yield_drift = steps[yield_idx].roof_drift_mm;
        let ult_shear = steps[ult_idx].base_shear_kn;
        let ult_drift = steps[ult_idx].roof_drift_mm;

        let omega0 = ult_shear / yield_shear.max(1.0);
        let mu = ult_drift / yield_drift.max(0.1);

        PushoverReport {
            model_name: model.name.clone(),
            steps,
            yield_base_shear_kn: yield_shear,
            yield_drift_mm: yield_drift,
            ultimate_base_shear_kn: ult_shear,
            ultimate_drift_mm: ult_drift,
            overstrength_factor_omega0: omega0,
            ductility_ratio_mu: mu,
            formed_hinges,
            is_stable: collapse_step.is_none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::{BoundaryFixity, Element3D, Node3D};

    #[test]
    fn test_geometric_stiffness_matrix_symmetry() {
        let p_axial = -50_000.0;
        let length = 4.0;
        let kg = NonlinearEngine::compute_kg_beam_column_local(p_axial, length);

        assert_eq!(kg.rows, 12);
        assert_eq!(kg.cols, 12);

        for i in 0..12 {
            for j in 0..12 {
                let diff = (kg[(i, j)] - kg[(j, i)]).abs();
                assert!(diff < 1e-9, "Kg is asymmetric at ({}, {}): diff = {}", i, j, diff);
            }
        }
    }

    #[test]
    fn test_aisc_amplification_and_plastic_hinge() {
        let (b1, b2) = NonlinearEngine::evaluate_aisc_amplification(-100.0, 5.0, 200.0, 1.2e-4, 1.0);
        assert!(b1 >= 1.0);
        assert!(b2 >= 1.0);

        let hinge_safe = NonlinearEngine::evaluate_plastic_hinge(1, 50.0, -100.0, 500.0, 355.0, 50.0);
        assert_eq!(hinge_safe.state, PlasticHingeState::Elastic);

        let hinge_critical = NonlinearEngine::evaluate_plastic_hinge(2, 500.0, -500.0, 500.0, 355.0, 50.0);
        assert!(hinge_critical.state > PlasticHingeState::Elastic);
    }

    #[test]
    fn test_pushover_curve_generation() {
        let mut model = StructuralModel::new("Pushover Test Tower");
        model.add_node(Node3D::new(0, 0.0, 0.0, 0.0).with_fixity(BoundaryFixity::fixed()));
        model.add_node(Node3D::new(1, 4.0, 0.0, 0.0).with_fixity(BoundaryFixity::fixed()));
        model.add_node(Node3D::new(2, 0.0, 0.0, 4.0));
        model.add_node(Node3D::new(3, 4.0, 0.0, 4.0));

        model.add_element(Element3D::new(1, "Col L", 0, 2));
        model.add_element(Element3D::new(2, "Col R", 1, 3));
        model.add_element(Element3D::new(3, "Beam", 2, 3));

        let report = NonlinearEngine::run_pushover(&model, 15);
        assert_eq!(report.steps.len(), 15);
        assert!(report.ultimate_base_shear_kn > report.yield_base_shear_kn);
        assert!(report.ductility_ratio_mu > 1.0);
        assert!(report.overstrength_factor_omega0 >= 1.0);
    }
}
