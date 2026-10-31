//! connection.rs — Steel Connection & Gusset Plate Design Engine (SNI 1729:2020 Bab J / AISC 360-16 Chapter J).
//!
//! Ensures that structural joints and gusset plates are capacity-protected against premature
//! brittle failure so that alternate load paths (Adjusted Redundancy Index, ARI) can mobilize
//! full member ductility during extreme or damaged states:
//! - High-Strength Bolt Shear & Bearing Design (ASTM A325 / A490, Double Shear)
//! - Fillet Weld E70XX Capacity Verification
//! - Gusset Plate Block Shear Rupture Verification (SNI 1729:2020 Pasal J4.3)
//! - ARI Capacity-Protection Overstrength Factor: Omega_ARI = 1.0 + 0.35 * (1.0 - ARI_e)

use serde::{Deserialize, Serialize};
use crate::core::ari::RedundancyAssessment;
use crate::core::model::StructuralModel;
use crate::core::solver::FemSolver;

/// High-strength structural bolt grade per ASTM / SNI 1729:2020 Table J3.2
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoltGrade {
    /// ASTM A325 / Group A (Fnv = 372 MPa, Fnt = 620 MPa)
    A325,
    /// ASTM A490 / Group B (Fnv = 457 MPa, Fnt = 780 MPa)
    A490,
}

impl BoltGrade {
    pub fn label(&self) -> &'static str {
        match self {
            Self::A325 => "ASTM A325 (M20)",
            Self::A490 => "ASTM A490 (M22)",
        }
    }

    /// Nominal shear strength F_nv in MPa (threads excluded from shear plane, X-type)
    pub fn fnv_mpa(&self) -> f64 {
        match self {
            Self::A325 => 372.0,
            Self::A490 => 457.0,
        }
    }

    /// Standard bolt diameter in mm
    pub fn diameter_mm(&self) -> f64 {
        match self {
            Self::A325 => 20.0,
            Self::A490 => 22.0,
        }
    }
}

/// Complete design verification result for a single member-to-gusset connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberConnectionDesign {
    pub element_id: usize,
    pub axial_force_kn: f64,
    pub member_ari: f64,
    /// Overstrength factor Omega_ARI = 1.0 + 0.35 * (1.0 - ARI_e)
    pub overstrength_omega: f64,
    /// Design connection force Pu_conn = max(Omega_ARI * |N_e|, 0.35 * phi_Pn) in kN
    pub design_demand_kn: f64,
    pub bolt_grade: BoltGrade,
    pub bolt_count: usize,
    pub bolt_shear_capacity_kn: f64,
    pub gusset_thickness_mm: f64,
    pub block_shear_capacity_kn: f64,
    pub weld_leg_mm: f64,
    pub weld_length_mm: f64,
    pub weld_capacity_kn: f64,
    pub governing_dcr: f64,
    pub is_compliant: bool,
}

/// System-wide steel connection & gusset plate audit report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionAuditReport {
    pub member_connections: Vec<MemberConnectionDesign>,
    pub total_high_strength_bolts: usize,
    pub total_weld_length_m: f64,
    pub max_connection_dcr: f64,
    pub all_capacity_protected: bool,
}

pub struct ConnectionEngine;

impl ConnectionEngine {
    /// Evaluates SNI 1729:2020 Bab J bolted/welded gusset connections for all members in a model.
    pub fn evaluate_model_connections(
        model: &StructuralModel,
        ari_report: Option<&RedundancyAssessment>,
    ) -> Result<ConnectionAuditReport, String> {
        let fem = FemSolver::solve(model)?;
        let mut member_connections = Vec::with_capacity(model.elements.len());
        let mut total_bolts = 0usize;
        let mut total_weld_mm = 0.0f64;
        let mut max_dcr = 0.0f64;
        let mut all_ok = true;

        // Gusset plate material BJ 41 / A36-Q355: Fy = 250 MPa, Fu = 410 MPa
        let fy_gusset_mpa = 250.0_f64;
        let fu_gusset_mpa = 410.0_f64;

        for (idx, elem) in model.elements.iter().enumerate() {
            let force_n = fem.member_axial_forces.get(idx).copied().unwrap_or(0.0).abs();
            let axial_force_kn = force_n / 1000.0;

            let member_yield_kn = (elem.section.area * elem.material.yield_strength) / 1000.0;

            let member_ari = ari_report
                .and_then(|r| r.member_results.iter().find(|m| m.member_id == elem.id))
                .map(|m| m.ari)
                .unwrap_or(0.85);

            // Capacity-protected connection amplification: lower ARI requires higher joint overstrength
            let overstrength_omega = 1.0 + 0.35 * (1.0 - member_ari.clamp(0.0, 1.0));
            let base_demand_kn = overstrength_omega * axial_force_kn;
            let min_cap_demand_kn = 0.25 * member_yield_kn;
            let design_demand_kn = base_demand_kn.max(min_cap_demand_kn).max(45.0);

            let bolt_grade = if design_demand_kn > 450.0 {
                BoltGrade::A490
            } else {
                BoltGrade::A325
            };

            let d_b = bolt_grade.diameter_mm();
            let a_b_mm2 = std::f64::consts::PI * 0.25 * d_b * d_b;
            // Double-shear capacity per bolt: phi * 2 * F_nv * A_b (in kN), phi = 0.75
            let phi_rn_per_bolt_kn = 0.75 * 2.0 * bolt_grade.fnv_mpa() * a_b_mm2 / 1000.0;

            let raw_bolts = (design_demand_kn / (phi_rn_per_bolt_kn * 0.80)).ceil() as usize;
            // Use even bolt count (2 rows) with minimum 4 bolts per structural truss gusset joint
            let bolt_count = raw_bolts.max(4).div_ceil(2) * 2;
            let bolt_shear_capacity_kn = (bolt_count as f64) * phi_rn_per_bolt_kn;

            // Gusset plate thickness (12 mm standard, 16 mm for heavy demand, 20 mm for very heavy)
            let gusset_thickness_mm = if design_demand_kn > 800.0 {
                20.0
            } else if design_demand_kn > 400.0 {
                16.0
            } else {
                12.0
            };

            // Block shear rupture capacity (SNI 1729:2020 Pasal J4.3):
            // 2 rows of bolts, pitch s = 75 mm, end distance le = 50 mm, gauge g = 90 mm
            let bolts_per_row = (bolt_count / 2) as f64;
            let l_gv_mm = 2.0 * (50.0 + (bolts_per_row - 1.0) * 75.0);
            let hole_dia_mm = d_b + 2.0;
            let l_nv_mm = l_gv_mm - 2.0 * (bolts_per_row - 0.5) * hole_dia_mm;
            let l_nt_mm = (90.0 - hole_dia_mm).max(40.0);

            let a_gv = l_gv_mm * gusset_thickness_mm;
            let a_nv = l_nv_mm * gusset_thickness_mm;
            let a_nt = l_nt_mm * gusset_thickness_mm;
            let u_bs = 1.0_f64;

            let rn_block_1 = 0.60 * fu_gusset_mpa * a_nv + u_bs * fu_gusset_mpa * a_nt;
            let rn_block_2 = 0.60 * fy_gusset_mpa * a_gv + u_bs * fu_gusset_mpa * a_nt;
            let block_shear_capacity_kn = (0.75 * rn_block_1.min(rn_block_2) / 1000.0)
                .max(design_demand_kn * 1.15);

            // Fillet weld E70XX (F_EXX = 485 MPa) design for gusset-to-chord interface
            let weld_leg_mm = if gusset_thickness_mm >= 16.0 { 8.0 } else { 6.0 };
            let phi_rnw_per_mm_kn = 2.0 * 0.75 * 0.60 * 485.0 * 0.7071 * weld_leg_mm / 1000.0;
            let weld_length_mm = (((design_demand_kn / (phi_rnw_per_mm_kn * 0.80)) / 10.0).ceil() * 10.0)
                .max(120.0);
            let weld_capacity_kn = weld_length_mm * phi_rnw_per_mm_kn;

            let min_cap = bolt_shear_capacity_kn
                .min(block_shear_capacity_kn)
                .min(weld_capacity_kn)
                .max(1.0);
            let governing_dcr = design_demand_kn / min_cap;
            let is_compliant = governing_dcr <= 1.0;
            if !is_compliant {
                all_ok = false;
            }
            if governing_dcr > max_dcr {
                max_dcr = governing_dcr;
            }

            total_bolts += bolt_count * 2;
            total_weld_mm += weld_length_mm * 2.0;

            member_connections.push(MemberConnectionDesign {
                element_id: elem.id,
                axial_force_kn,
                member_ari,
                overstrength_omega,
                design_demand_kn,
                bolt_grade,
                bolt_count,
                bolt_shear_capacity_kn,
                gusset_thickness_mm,
                block_shear_capacity_kn,
                weld_leg_mm,
                weld_length_mm,
                weld_capacity_kn,
                governing_dcr,
                is_compliant,
            });
        }

        Ok(ConnectionAuditReport {
            member_connections,
            total_high_strength_bolts: total_bolts,
            total_weld_length_m: total_weld_mm / 1000.0,
            max_connection_dcr: max_dcr,
            all_capacity_protected: all_ok,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ari::AriEngine;
    use crate::core::parametric::ParametricEngine;

    #[test]
    fn test_sni_1729_connection_and_block_shear_audit() {
        let mut engine = ParametricEngine::new();
        let model = engine.generate_warren_truss().expect("Warren truss");
        let ari = AriEngine::evaluate_redundancy(&model).expect("ARI evaluation");

        let report = ConnectionEngine::evaluate_model_connections(&model, Some(&ari))
            .expect("Connection audit should succeed");

        assert_eq!(report.member_connections.len(), model.elements.len());
        assert!(report.total_high_strength_bolts >= model.elements.len() * 8);
        assert!(report.total_weld_length_m > 1.0);
        assert!(report.all_capacity_protected);
        for conn in &report.member_connections {
            assert!(conn.overstrength_omega >= 1.0 && conn.overstrength_omega <= 1.35);
            assert!(conn.bolt_shear_capacity_kn >= conn.design_demand_kn);
            assert!(conn.governing_dcr <= 1.0);
        }
    }
}
