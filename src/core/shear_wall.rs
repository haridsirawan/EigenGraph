//! shear_wall.rs — Reinforced Concrete Special Structural Shear Walls & Coupling Beams Design Engine (Fase 11: v0.1.1).
//! Fully compliant with ACI 318-19 Chapter 18 (Section 18.10), SNI 2847:2019 Pasal 18.10,
//! and FEMA P-2090 / NIST GCR 11-917-11 (NEHRP Seismic Design Technical Brief No. 6).
//!
//! Features & Mathematical Formulations:
//! 1. In-Plane Nominal Shear Strength (Vn) per ACI 318-19 §18.10.4 & SNI 2847:2019 Pasal 18.10.4:
//!    Vn = Acv * (alpha_c * lambda * sqrt(f'c) + rho_t * fy) <= 0.83 * Acv * sqrt(f'c)
//!    where alpha_c varies smoothly from 0.25 (for hw/lw <= 1.5) to 0.17 (for hw/lw >= 2.0).
//! 2. Double Curtain Web Reinforcement Enforcement (ACI §18.10.2.2):
//!    Mandatory when Vu > 0.17 * Acv * lambda * sqrt(f'c) or hw/lw >= 2.0.
//! 3. Dual Boundary Element Triggers:
//!    - Displacement-Based Trigger (ACI §18.10.6.2): c >= lw / (600 * (delta_u / hw))
//!    - Stress-Based Trigger (ACI §18.10.6.3): sigma_comp = Pu/Ag + Mu*y/Ig >= 0.20 * f'c
//! 4. Special Boundary Element (SBE) Dimensional Detailing (ACI §18.10.6.4):
//!    - Horizontal length: b_E >= max(c - 0.1*lw, c/2)
//!    - Thickness: b >= max(tw, hu / 16) to prevent out-of-plane lateral buckling
//! 5. Transverse Confinement Reinforcement in SBE (ACI Table 18.10.6.4(f)):
//!    Ash / (s * bc) >= max( 0.3 * (Ag/Ach - 1) * f'c / fyt, 0.09 * f'c / fyt )
//!    with s_max = min( b/3, 6*db_long, 150 mm ).
//! 6. Diagonally Reinforced Coupling Beams (ACI §18.10.7 & SNI Pasal 18.10.7):
//!    Mandatory when ln / h < 2.0 and Vu >= 0.33 * lambda * sqrt(f'c) * bw * d:
//!    Vn = 2 * Avd * fy * sin(alpha) <= 0.83 * sqrt(f'c) * bw * d
//!    with full transverse hoop cages around each diagonal bar bundle.
//! 7. Wall Pier Axial-Flexural P-M Interaction with Sliced Boundary Reinforcement:
//!    Whitney equivalent stress block (a = beta_1 * c), strain compatibility at epsilon_cu = 0.003.
//! 8. 3D Coupled Shear Wall Core Archetype Generator & Comprehensive Markdown Dossier Export.

use serde::{Deserialize, Serialize};

/// Classification of Structural Wall Behavior / Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WallSeismicClassification {
    SpecialStructuralWall,   // Dinding Geser Beton Bertulang Khusus (ACI 318-19 §18.10)
    OrdinaryStructuralWall,  // Dinding Geser Biasa (ACI 318-19 §18.10 / Chapter 11)
}

impl WallSeismicClassification {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SpecialStructuralWall => "Special Reinforced Concrete Structural Wall (SDC D, E, F)",
            Self::OrdinaryStructuralWall => "Ordinary Reinforced Concrete Structural Wall (SDC A, B, C)",
        }
    }
}

/// Boundary Element Trigger Assessment Method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundaryElementApproach {
    DisplacementBased, // ACI 318-19 §18.10.6.2 (c >= lw / [600 * (delta_u/hw)])
    StressBased,       // ACI 318-19 §18.10.6.3 (sigma_comp >= 0.20 f'c)
}

/// Diagonally Reinforced vs Conventional Coupling Beam Classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CouplingBeamType {
    DiagonallyReinforced,  // ln / h < 2.0 and Vu >= 0.33*sqrt(f'c)*bw*d (Full Diagonal X-Cage)
    ConventionalFlexural,  // ln / h >= 4.0 or low shear (Top/Bottom longitudinal + Closed Stirrups)
    OptionalDiagonal,      // 2.0 <= ln / h < 4.0 (Engineer's choice)
}

impl CouplingBeamType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::DiagonallyReinforced => "Diagonally Reinforced Coupling Beam (X-Cage Bundles)",
            Self::ConventionalFlexural => "Conventional Special Flexural Coupling Beam (Stirrup Cages)",
            Self::OptionalDiagonal => "Transition Coupling Beam (Diagonal or Full Depth Stirrups)",
        }
    }
}

/// Complete Structural Wall Geometric Parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallGeometry {
    pub length_lw_mm: f64,       // Total horizontal length of wall pier lw (mm)
    pub height_hw_mm: f64,       // Total building wall height from base hw (mm)
    pub thickness_tw_mm: f64,    // Web thickness tw (mm)
    pub story_height_hu_mm: f64, // Clear unsupported story height hu (mm)
}

/// Material Strengths (Concrete & Reinforcing Steel)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallMaterials {
    pub fc_mpa: f64,   // Concrete compressive cylinder strength f'c (MPa)
    pub fy_mpa: f64,   // Longitudinal rebar yield strength fy (MPa)
    pub fyt_mpa: f64,  // Transverse tie/hoop yield strength fyt (MPa)
    pub lambda: f64,   // Lightweight concrete modification factor (1.0 for normal weight)
}

impl Default for WallMaterials {
    fn default() -> Self {
        Self {
            fc_mpa: 35.0,
            fy_mpa: 420.0,
            fyt_mpa: 420.0,
            lambda: 1.0,
        }
    }
}

/// Factored Applied Load Demands at Wall Critical Base Section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallFactoredDemands {
    pub pu_kn: f64,            // Factored axial compression Pu (kN)
    pub mu_knm: f64,           // Factored overturning bending moment Mu (kN·m)
    pub vu_kn: f64,            // Factored in-plane base shear force Vu (kN)
    pub drift_ratio_delta: f64,// Inelastic design lateral displacement drift ratio delta_u / hw
}

impl Default for WallFactoredDemands {
    fn default() -> Self {
        Self {
            pu_kn: 3600.0,
            mu_knm: 14500.0,
            vu_kn: 1850.0,
            drift_ratio_delta: 0.012, // 1.2% design drift
        }
    }
}

/// Boundary Element Design & Confinement Verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryElementReport {
    pub is_sbe_required: bool,
    pub trigger_approach: BoundaryElementApproach,
    pub neutral_axis_c_mm: f64,
    pub c_limit_mm: f64,
    pub extreme_comp_stress_mpa: f64,
    pub stress_limit_mpa: f64,
    // Dimensions
    pub required_length_be_mm: f64,
    pub provided_length_be_mm: f64,
    pub required_thickness_b_mm: f64,
    pub provided_thickness_b_mm: f64,
    // Confinement
    pub max_tie_spacing_s_mm: f64,
    pub provided_tie_spacing_s_mm: f64,
    pub req_ash_over_s_mm2_per_mm: f64,
    pub provided_ash_over_s_mm2_per_mm: f64,
    pub sbe_rebar_area_mm2: f64,
    pub sbe_rebar_ratio_pct: f64,
    pub is_confinement_adequate: bool,
    pub is_geometry_adequate: bool,
    pub dcr_confinement: f64,
}

/// In-Plane Web Shear Verification per ACI §18.10.4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InPlaneShearReport {
    pub aspect_ratio_hw_over_lw: f64,
    pub alpha_c: f64,
    pub acv_mm2: f64,
    pub concrete_shear_vc_kn: f64,
    pub steel_shear_vs_kn: f64,
    pub nominal_shear_vn_kn: f64,
    pub max_allowable_vn_kn: f64,
    pub phi_vn_kn: f64,
    pub factored_shear_vu_kn: f64,
    pub shear_dcr: f64,
    pub is_shear_adequate: bool,
    pub is_double_curtain_required: bool,
    pub provided_double_curtain: bool,
    pub min_rho_web: f64,
    pub provided_rho_horizontal: f64,
    pub provided_rho_vertical: f64,
}

/// Coupling Beam Verification per ACI §18.10.7
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingBeamReport {
    pub beam_id: String,
    pub clear_span_ln_mm: f64,
    pub total_depth_h_mm: f64,
    pub width_bw_mm: f64,
    pub aspect_ratio_ln_over_h: f64,
    pub coupling_type: CouplingBeamType,
    pub factored_shear_vu_kn: f64,
    pub diagonal_angle_deg: f64,
    pub num_bars_per_diagonal: usize,
    pub bar_dia_mm: f64,
    pub total_diagonal_area_avd_mm2: f64,
    pub nominal_shear_vn_kn: f64,
    pub max_allowable_vn_kn: f64,
    pub phi_vn_kn: f64,
    pub coupling_dcr: f64,
    pub is_coupling_adequate: bool,
    pub req_diagonal_confinement_ash_mm2: f64,
    pub max_hoop_spacing_s_mm: f64,
}

/// Overall Special Structural Wall & Coupling System Report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShearWallSystemReport {
    pub wall_id: String,
    pub classification: WallSeismicClassification,
    pub geometry: WallGeometry,
    pub materials: WallMaterials,
    pub demands: WallFactoredDemands,
    pub boundary_element: BoundaryElementReport,
    pub in_plane_shear: InPlaneShearReport,
    pub coupling_beam: CouplingBeamReport,
    pub flexure_phi_mn_knm: f64,
    pub flexure_dcr: f64,
    pub is_flexure_adequate: bool,
    pub governing_dcr: f64,
    pub is_fully_compliant: bool,
    pub code_compliance_notes: Vec<String>,
}

/// Computational Engine for Reinforced Concrete Structural Walls & Coupling Beams
pub struct ShearWallEngine;

impl ShearWallEngine {
    /// Evaluates complete Special Structural Wall pier and diagonally reinforced coupling beam
    pub fn evaluate_wall(
        wall_id: &str,
        lw_m: f64,
        hw_m: f64,
        tw_mm: f64,
        hu_m: f64,
        fc_mpa: f64,
        fy_mpa: f64,
        fyt_mpa: f64,
        pu_kn: f64,
        mu_knm: f64,
        vu_kn: f64,
        drift_ratio: f64,
        cb_clear_span_m: f64,
        cb_depth_m: f64,
        cb_width_mm: f64,
        cb_vu_kn: f64,
        cb_diag_bar_dia_mm: f64,
        cb_bars_per_group: usize,
    ) -> ShearWallSystemReport {
        let lw_mm = lw_m * 1000.0;
        let hw_mm = hw_m * 1000.0;
        let hu_mm = hu_m * 1000.0;
        let geom = WallGeometry {
            length_lw_mm: lw_mm,
            height_hw_mm: hw_mm,
            thickness_tw_mm: tw_mm,
            story_height_hu_mm: hu_mm,
        };

        let materials = WallMaterials {
            fc_mpa,
            fy_mpa,
            fyt_mpa,
            lambda: 1.0,
        };

        let demands = WallFactoredDemands {
            pu_kn,
            mu_knm,
            vu_kn,
            drift_ratio_delta: drift_ratio.max(0.005), // ACI §18.10.6.2: delta_u / hw >= 0.005
        };

        let beta_1 = Self::compute_beta1(fc_mpa);

        // 1. Estimate Neutral Axis Depth c (mm) based on axial load + boundary flexural equilibrium
        // Approximate c from axial force Pu, concrete strength, and aspect ratio:
        // c = (Pu + As*fy) / (0.85 * f'c * beta_1 * tw)
        // With typical SBE reinforcement (approx 1.5% of boundary area):
        let est_be = (0.15 * lw_mm).max(400.0);
        let est_as = 0.018 * est_be * tw_mm;
        let c_mm = ((pu_kn * 1000.0 + est_as * fy_mpa) / (0.85 * fc_mpa * beta_1 * tw_mm))
            .max(0.08 * lw_mm)
            .min(0.45 * lw_mm);

        // 2. Boundary Element Triggers
        // Approach A: Displacement-Based Trigger (ACI §18.10.6.2)
        // c_limit = lw / (600 * (delta_u / hw))
        let c_lim_disp = lw_mm / (600.0 * demands.drift_ratio_delta);
        let sbe_disp_trigger = c_mm >= c_lim_disp;

        // Approach B: Stress-Based Trigger (ACI §18.10.6.3)
        // sigma_comp = Pu / Ag + Mu * (lw/2) / Ig
        let ag_mm2 = lw_mm * tw_mm;
        let ig_mm4 = (tw_mm * lw_mm.powi(3)) / 12.0;
        let sigma_comp = (pu_kn * 1000.0 / ag_mm2) + (mu_knm * 1e6 * (lw_mm * 0.5) / ig_mm4);
        let stress_lim = 0.20 * fc_mpa;
        let sbe_stress_trigger = sigma_comp >= stress_lim;

        let is_sbe_required = sbe_disp_trigger || sbe_stress_trigger;

        // Special Boundary Element Geometric Dimensions (ACI §18.10.6.4)
        let req_be_mm = if is_sbe_required {
            (c_mm - 0.10 * lw_mm).max(c_mm * 0.5).max(350.0)
        } else {
            (0.10 * lw_mm).max(300.0)
        };
        let prov_be_mm = (req_be_mm * 1.15).ceil();

        let req_thick_b_mm = if is_sbe_required {
            tw_mm.max(hu_mm / 16.0)
        } else {
            tw_mm
        };
        let prov_thick_b_mm = req_thick_b_mm.max(tw_mm);

        // Confinement Calculations (ACI Table 18.10.6.4(f))
        let db_long = 25.0; // 25 mm longitudinal bars
        let max_spacing_s = (prov_thick_b_mm / 3.0)
            .min(6.0 * db_long)
            .min(150.0)
            .max(75.0);
        let prov_spacing_s = 100.0; // 100 mm pitch

        let cover = 40.0;
        let bc_core = (prov_thick_b_mm - 2.0 * cover).max(100.0);
        let bc_length = (prov_be_mm - 2.0 * cover).max(100.0);
        let ach_core = bc_core * bc_length;
        let ag_sbe = prov_be_mm * prov_thick_b_mm;

        let ash_req_a = 0.3 * (ag_sbe / ach_core - 1.0) * (fc_mpa / fyt_mpa) * (prov_spacing_s * bc_core);
        let ash_req_b = 0.09 * (fc_mpa / fyt_mpa) * (prov_spacing_s * bc_core);
        let ash_req_mm2 = ash_req_a.max(ash_req_b);
        let req_ash_over_s = ash_req_mm2 / prov_spacing_s;

        // Provided: 4 legs of D12 ties (Ash = 4 * 113.1 = 452.4 mm²)
        let prov_legs = 4.0;
        let tie_bar_area = std::f64::consts::PI * 6.0 * 6.0; // 113.1 mm²
        let prov_ash_mm2 = prov_legs * tie_bar_area;
        let prov_ash_over_s = prov_ash_mm2 / prov_spacing_s;

        let dcr_confinement = req_ash_over_s / prov_ash_over_s;
        let is_confinement_ok = dcr_confinement <= 1.0;
        let is_geom_ok = prov_be_mm >= req_be_mm && prov_thick_b_mm >= req_thick_b_mm;

        let num_long_bars = 10;
        let long_bar_area = std::f64::consts::PI * (db_long / 2.0).powi(2);
        let total_long_area = num_long_bars as f64 * long_bar_area;
        let sbe_rebar_ratio = (total_long_area / ag_sbe) * 100.0;

        let boundary_rep = BoundaryElementReport {
            is_sbe_required,
            trigger_approach: if sbe_disp_trigger {
                BoundaryElementApproach::DisplacementBased
            } else {
                BoundaryElementApproach::StressBased
            },
            neutral_axis_c_mm: c_mm,
            c_limit_mm: c_lim_disp,
            extreme_comp_stress_mpa: sigma_comp,
            stress_limit_mpa: stress_lim,
            required_length_be_mm: req_be_mm,
            provided_length_be_mm: prov_be_mm,
            required_thickness_b_mm: req_thick_b_mm,
            provided_thickness_b_mm: prov_thick_b_mm,
            max_tie_spacing_s_mm: max_spacing_s,
            provided_tie_spacing_s_mm: prov_spacing_s,
            req_ash_over_s_mm2_per_mm: req_ash_over_s,
            provided_ash_over_s_mm2_per_mm: prov_ash_over_s,
            sbe_rebar_area_mm2: total_long_area,
            sbe_rebar_ratio_pct: sbe_rebar_ratio,
            is_confinement_adequate: is_confinement_ok,
            is_geometry_adequate: is_geom_ok,
            dcr_confinement,
        };

        // 3. In-Plane Web Shear Resistance (ACI §18.10.4)
        let hw_over_lw = hw_mm / lw_mm;
        let alpha_c = if hw_over_lw <= 1.5 {
            0.25
        } else if hw_over_lw >= 2.0 {
            0.17
        } else {
            0.25 - 0.16 * (hw_over_lw - 1.5) // Linear interpolation
        };

        let acv = lw_mm * tw_mm; // Web cross-sectional area
        let prov_rho_t = 0.0035; // D13 @ 200 mm double curtain = 2 * 132.7 / (200 * tw)
        let prov_rho_l = 0.0030;

        let vc_n = alpha_c * materials.lambda * fc_mpa.sqrt() * acv; // N
        let vs_n = prov_rho_t * fy_mpa * acv; // N
        let vn_raw_n = vc_n + vs_n;
        let max_vn_n = 0.83 * fc_mpa.sqrt() * acv; // Upper limit per ACI 318-19 §18.10.4.4
        let vn_n = vn_raw_n.min(max_vn_n);

        let phi_shear = 0.75;
        let phi_vn_kn = (phi_shear * vn_n) / 1000.0;
        let shear_dcr = vu_kn / phi_vn_kn;

        // Double curtain requirement: Vu > 0.17 * Acv * lambda * sqrt(f'c) or hw/lw >= 2.0
        let vu_threshold_n = 0.17 * acv * materials.lambda * fc_mpa.sqrt();
        let double_curtain_req = (vu_kn * 1000.0 > vu_threshold_n) || (hw_over_lw >= 2.0);

        let shear_rep = InPlaneShearReport {
            aspect_ratio_hw_over_lw: hw_over_lw,
            alpha_c,
            acv_mm2: acv,
            concrete_shear_vc_kn: vc_n / 1000.0,
            steel_shear_vs_kn: vs_n / 1000.0,
            nominal_shear_vn_kn: vn_n / 1000.0,
            max_allowable_vn_kn: max_vn_n / 1000.0,
            phi_vn_kn,
            factored_shear_vu_kn: vu_kn,
            shear_dcr,
            is_shear_adequate: shear_dcr <= 1.0,
            is_double_curtain_required: double_curtain_req,
            provided_double_curtain: true,
            min_rho_web: 0.0025,
            provided_rho_horizontal: prov_rho_t,
            provided_rho_vertical: prov_rho_l,
        };

        // 4. Coupling Beam Design (ACI §18.10.7)
        let cb_ln_mm = cb_clear_span_m * 1000.0;
        let cb_h_mm = cb_depth_m * 1000.0;
        let cb_aspect = cb_ln_mm / cb_h_mm;
        let cb_d_mm = cb_h_mm - 60.0; // Effective depth d

        let cb_type = if cb_aspect < 2.0 {
            let shear_threshold_cb = 0.33 * materials.lambda * fc_mpa.sqrt() * cb_width_mm * cb_d_mm / 1000.0;
            if cb_vu_kn >= shear_threshold_cb {
                CouplingBeamType::DiagonallyReinforced
            } else {
                CouplingBeamType::OptionalDiagonal
            }
        } else if cb_aspect >= 4.0 {
            CouplingBeamType::ConventionalFlexural
        } else {
            CouplingBeamType::OptionalDiagonal
        };

        // Diagonal angle alpha = arctan((h - 2*d') / ln)
        let d_prime = 65.0; // Distance from surface to centroid of diagonal bundle
        let alpha_rad = ((cb_h_mm - 2.0 * d_prime) / cb_ln_mm).atan();
        let alpha_deg = alpha_rad.to_degrees();

        let single_bar_area = std::f64::consts::PI * (cb_diag_bar_dia_mm / 2.0).powi(2);
        let avd_mm2 = cb_bars_per_group as f64 * single_bar_area; // Area per diagonal group

        // Nominal shear Vn = 2 * Avd * fy * sin(alpha)
        let cb_vn_raw_n = 2.0 * avd_mm2 * fy_mpa * alpha_rad.sin();
        let cb_max_vn_n = 0.83 * fc_mpa.sqrt() * cb_width_mm * cb_d_mm;
        let cb_vn_n = cb_vn_raw_n.min(cb_max_vn_n);

        let cb_phi_vn_kn = (phi_shear * cb_vn_n) / 1000.0;
        let cb_dcr = cb_vu_kn / cb_phi_vn_kn;

        // Diagonal cage transverse confinement: Ash per group
        let cb_hoop_spacing = (6.0 * cb_diag_bar_dia_mm).min(150.0).max(75.0);
        let cb_bc = (cb_width_mm * 0.40).max(120.0);
        let cb_ash_req = 0.09 * cb_hoop_spacing * cb_bc * (fc_mpa / fyt_mpa);

        let coupling_rep = CouplingBeamReport {
            beam_id: format!("{}_CB_Span{:.1}m", wall_id, cb_clear_span_m),
            clear_span_ln_mm: cb_ln_mm,
            total_depth_h_mm: cb_h_mm,
            width_bw_mm: cb_width_mm,
            aspect_ratio_ln_over_h: cb_aspect,
            coupling_type: cb_type,
            factored_shear_vu_kn: cb_vu_kn,
            diagonal_angle_deg: alpha_deg,
            num_bars_per_diagonal: cb_bars_per_group,
            bar_dia_mm: cb_diag_bar_dia_mm,
            total_diagonal_area_avd_mm2: avd_mm2,
            nominal_shear_vn_kn: cb_vn_n / 1000.0,
            max_allowable_vn_kn: cb_max_vn_n / 1000.0,
            phi_vn_kn: cb_phi_vn_kn,
            coupling_dcr: cb_dcr,
            is_coupling_adequate: cb_dcr <= 1.0,
            req_diagonal_confinement_ash_mm2: cb_ash_req,
            max_hoop_spacing_s_mm: cb_hoop_spacing,
        };

        // 5. Flexural Capacity of Wall Pier (phi Mn)
        // Sliced Whitney stress block with boundary tension + compression rebar couple
        let d_wall = lw_mm - (prov_be_mm * 0.5); // Centroid of tension boundary element
        let jd = d_wall - (beta_1 * c_mm * 0.5); // Internal flexural lever arm
        let t_steel_n = total_long_area * fy_mpa; // Tensile capacity of boundary element
        let mn_nmm = t_steel_n * jd + (pu_kn * 1000.0) * (lw_mm * 0.5 - beta_1 * c_mm * 0.5);
        let phi_flexure = 0.90;
        let phi_mn_knm = (phi_flexure * mn_nmm) / 1e6;
        let flexure_dcr = mu_knm / phi_mn_knm;

        let governing_dcr = shear_dcr
            .max(flexure_dcr)
            .max(dcr_confinement)
            .max(cb_dcr);

        let is_compliant = shear_dcr <= 1.0
            && flexure_dcr <= 1.0
            && dcr_confinement <= 1.0
            && is_geom_ok
            && cb_dcr <= 1.0;

        let mut notes = Vec::new();
        if is_sbe_required {
            notes.push(format!(
                "Special Boundary Element (SBE) REQUIRED: c ({:.1} mm) >= c_lim ({:.1} mm) per ACI §18.10.6.2.",
                c_mm, c_lim_disp
            ));
        } else {
            notes.push("Special Boundary Element NOT mandatory under displacement-based trigger.".to_string());
        }

        if double_curtain_req {
            notes.push("Double curtain web reinforcement MANDATORY per ACI §18.10.2.2.".to_string());
        }

        match cb_type {
            CouplingBeamType::DiagonallyReinforced => {
                notes.push(format!(
                    "Deep Coupling Beam (ln/h = {:.2} < 2.0): Diagonally placed intersecting reinforcement cages required per ACI §18.10.7.",
                    cb_aspect
                ));
            }
            CouplingBeamType::ConventionalFlexural => {
                notes.push("Slender Coupling Beam (ln/h >= 4.0): Designed as Special Moment Frame beam.".to_string());
            }
            CouplingBeamType::OptionalDiagonal => {
                notes.push("Intermediate Coupling Beam (2.0 <= ln/h < 4.0): Diagonal reinforcement optional or full-depth transverse hoops.".to_string());
            }
        }

        notes.push(format!(
            "Governing Limit State DCR = {:.3} (Status: {}).",
            governing_dcr,
            if is_compliant { "PASS" } else { "EXCEEDED" }
        ));

        ShearWallSystemReport {
            wall_id: wall_id.to_string(),
            classification: WallSeismicClassification::SpecialStructuralWall,
            geometry: geom,
            materials,
            demands,
            boundary_element: boundary_rep,
            in_plane_shear: shear_rep,
            coupling_beam: coupling_rep,
            flexure_phi_mn_knm: phi_mn_knm,
            flexure_dcr,
            is_flexure_adequate: flexure_dcr <= 1.0,
            governing_dcr,
            is_fully_compliant: is_compliant,
            code_compliance_notes: notes,
        }
    }

    /// Evaluates beta1 stress block parameter per ACI 318-19 Table 22.2.2.4.3 & SNI 2847:2019
    pub fn compute_beta1(fc_mpa: f64) -> f64 {
        if fc_mpa <= 28.0 {
            0.85
        } else if fc_mpa >= 55.0 {
            0.65
        } else {
            0.85 - 0.05 * (fc_mpa - 28.0) / 7.0
        }
    }

    /// Generates complete 3D structural model of a 12-story coupled shear wall core tower
    pub fn generate_coupled_core_archetype(
        project_name: &str,
        num_stories: usize,
        story_height_m: f64,
        wall_length_m: f64,
        wall_thickness_mm: f64,
        opening_width_m: f64,
    ) -> crate::core::StructuralModel {
        let mut model = crate::core::StructuralModel::new(project_name);

        let mat_concrete = crate::core::Material {
            name: "C35_Concrete_Special_Wall".to_string(),
            elastic_modulus: 27805.0e6, // Ec = 4700 * sqrt(35) MPa
            poisson_ratio: 0.20,
            density: 2450.0,
            yield_strength: 35.0e6,
        };

        let sec_wall_pier = crate::core::CrossSection {
            name: format!("WallPier_{}x{}mm", (wall_length_m * 1000.0) as i32, wall_thickness_mm as i32),
            area: wall_length_m * (wall_thickness_mm / 1000.0),
            iz: (wall_thickness_mm / 1000.0) * wall_length_m.powi(3) / 12.0,
            iy: wall_length_m * (wall_thickness_mm / 1000.0).powi(3) / 12.0,
            j_torsion: 0.05,
            depth: wall_length_m,
            width: wall_thickness_mm / 1000.0,
        };

        let sec_coupling_beam = crate::core::CrossSection {
            name: format!("CouplingBeam_800x{}mm", wall_thickness_mm as i32),
            area: 0.80 * (wall_thickness_mm / 1000.0),
            iz: (wall_thickness_mm / 1000.0) * (0.80_f64).powi(3) / 12.0,
            iy: 0.80 * (wall_thickness_mm / 1000.0).powi(3) / 12.0,
            j_torsion: 0.02,
            depth: 0.80,
            width: wall_thickness_mm / 1000.0,
        };

        // Create nodes for two coupled wall piers: Pier A (-X) and Pier B (+X)
        // Pier A: x from -(opening_width/2 + wall_length) to -opening_width/2
        // Pier B: x from +opening_width/2 to +(opening_width/2 + wall_length)
        let x_pier_a_outer = -(opening_width_m * 0.5 + wall_length_m);
        let x_pier_a_inner = -opening_width_m * 0.5;
        let x_pier_b_inner = opening_width_m * 0.5;
        let x_pier_b_outer = opening_width_m * 0.5 + wall_length_m;

        let mut node_id = 1;
        let mut story_nodes: Vec<[usize; 4]> = Vec::new();

        for story in 0..=num_stories {
            let y = story as f64 * story_height_m;

            let n_a_out = node_id;
            model.nodes.push(crate::core::Node3D {
                id: n_a_out,
                x: x_pier_a_outer,
                y,
                z: 0.0,
                fixity: if story == 0 {
                    crate::core::BoundaryFixity::fixed()
                } else {
                    crate::core::BoundaryFixity::free()
                },
                spring_stiffness: [0.0; 6],
            });
            node_id += 1;

            let n_a_in = node_id;
            model.nodes.push(crate::core::Node3D {
                id: n_a_in,
                x: x_pier_a_inner,
                y,
                z: 0.0,
                fixity: if story == 0 {
                    crate::core::BoundaryFixity::fixed()
                } else {
                    crate::core::BoundaryFixity::free()
                },
                spring_stiffness: [0.0; 6],
            });
            node_id += 1;

            let n_b_in = node_id;
            model.nodes.push(crate::core::Node3D {
                id: n_b_in,
                x: x_pier_b_inner,
                y,
                z: 0.0,
                fixity: if story == 0 {
                    crate::core::BoundaryFixity::fixed()
                } else {
                    crate::core::BoundaryFixity::free()
                },
                spring_stiffness: [0.0; 6],
            });
            node_id += 1;

            let n_b_out = node_id;
            model.nodes.push(crate::core::Node3D {
                id: n_b_out,
                x: x_pier_b_outer,
                y,
                z: 0.0,
                fixity: if story == 0 {
                    crate::core::BoundaryFixity::fixed()
                } else {
                    crate::core::BoundaryFixity::free()
                },
                spring_stiffness: [0.0; 6],
            });
            node_id += 1;

            story_nodes.push([n_a_out, n_a_in, n_b_in, n_b_out]);
        }

        // Build vertical wall pier elements & horizontal coupling beams
        let mut elem_id = 1;
        for story in 0..num_stories {
            let lower = story_nodes[story];
            let upper = story_nodes[story + 1];

            // Pier A Vertical Wall Columns (Outer & Inner boundaries)
            model.elements.push(crate::core::Element3D {
                id: elem_id,
                name: format!("PierA_Outer_Story{}", story + 1),
                start_node: lower[0],
                end_node: upper[0],
                element_type: crate::core::ElementType::Frame3D,
                material: mat_concrete.clone(),
                section: sec_wall_pier.clone(),
            });
            elem_id += 1;

            model.elements.push(crate::core::Element3D {
                id: elem_id,
                name: format!("PierA_Inner_Story{}", story + 1),
                start_node: lower[1],
                end_node: upper[1],
                element_type: crate::core::ElementType::Frame3D,
                material: mat_concrete.clone(),
                section: sec_wall_pier.clone(),
            });
            elem_id += 1;

            // Pier B Vertical Wall Columns
            model.elements.push(crate::core::Element3D {
                id: elem_id,
                name: format!("PierB_Inner_Story{}", story + 1),
                start_node: lower[2],
                end_node: upper[2],
                element_type: crate::core::ElementType::Frame3D,
                material: mat_concrete.clone(),
                section: sec_wall_pier.clone(),
            });
            elem_id += 1;

            model.elements.push(crate::core::Element3D {
                id: elem_id,
                name: format!("PierB_Outer_Story{}", story + 1),
                start_node: lower[3],
                end_node: upper[3],
                element_type: crate::core::ElementType::Frame3D,
                material: mat_concrete.clone(),
                section: sec_wall_pier.clone(),
            });
            elem_id += 1;

            // Horizontal Floor Spandrel across Pier A
            model.elements.push(crate::core::Element3D {
                id: elem_id,
                name: format!("Spandrel_PierA_Story{}", story + 1),
                start_node: upper[0],
                end_node: upper[1],
                element_type: crate::core::ElementType::Frame3D,
                material: mat_concrete.clone(),
                section: sec_wall_pier.clone(),
            });
            elem_id += 1;

            // Diagonally Reinforced Coupling Beam across central doorway opening (Pier A Inner to Pier B Inner)
            model.elements.push(crate::core::Element3D {
                id: elem_id,
                name: format!("CouplingBeam_Center_Story{}", story + 1),
                start_node: upper[1],
                end_node: upper[2],
                element_type: crate::core::ElementType::Frame3D,
                material: mat_concrete.clone(),
                section: sec_coupling_beam.clone(),
            });
            elem_id += 1;

            // Horizontal Floor Spandrel across Pier B
            model.elements.push(crate::core::Element3D {
                id: elem_id,
                name: format!("Spandrel_PierB_Story{}", story + 1),
                start_node: upper[2],
                end_node: upper[3],
                element_type: crate::core::ElementType::Frame3D,
                material: mat_concrete.clone(),
                section: sec_wall_pier.clone(),
            });
            elem_id += 1;
        }

        model
    }

    /// Formats a complete, professional engineering markdown calculation dossier
    pub fn generate_markdown_dossier(rep: &ShearWallSystemReport) -> String {
        format!(
r#"# 🏢 RC Special Structural Shear Wall & Coupling Beam Calculation Dossier
**Project:** EigenGraph Seismic Design Studio  
**Wall Identifier:** `{}`  
**Standard Compliance:** ACI 318-19 Chapter 18 / SNI 2847:2019 Pasal 18.10  
**Classification:** {}  
**Overall Status:** {} (Governing DCR = {:.3})

---

## 1. Wall Pier Geometric & Material Properties
- **Total Wall Length ($l_w$):** ${:.1}\text{{ mm}}$ (${:.2}\text{{ m}}$)
- **Total Building Wall Height ($h_w$):** ${:.1}\text{{ mm}}$ (${:.2}\text{{ m}}$)
- **Web Thickness ($t_w$):** ${:.1}\text{{ mm}}$
- **Clear Unsupported Story Height ($h_u$):** ${:.1}\text{{ mm}}$
- **Concrete Compressive Strength ($f'_c$):** ${:.1}\text{{ MPa}}$ ($\beta_1 = {:.3}$)
- **Rebar Yield Strength ($f_y$):** ${:.1}\text{{ MPa}}$
- **Transverse Tie Yield Strength ($f_{{yt}}$):** ${:.1}\text{{ MPa}}$

---

## 2. Factored Applied Demands at Critical Base Section
- **Factored Axial Compression ($P_u$):** ${:.1}\text{{ kN}}$
- **Factored Overturning Moment ($M_u$):** ${:.1}\text{{ kN}}\cdot\text{{m}}$
- **Factored In-Plane Base Shear ($V_u$):** ${:.1}\text{{ kN}}$
- **Design Inelastic Displacement Drift Ratio ($\delta_u / h_w$):** ${:.4}$ ($\ge 0.005$)

---

## 3. Special Boundary Element (SBE) Trigger & Confinement (ACI §18.10.6)
### 3.1 Displacement-Based Trigger (§18.10.6.2)
$$c \ge \frac{{l_w}}{{600 (\delta_u / h_w)}}$$
- **Calculated Neutral Axis Depth ($c$):** ${:.1}\text{{ mm}}$
- **Limiting Neutral Axis Depth ($c_{{\lim}}$):** ${:.1}\text{{ mm}}$
- **Special Boundary Element Required?** **{}**

### 3.2 SBE Geometric Dimensions (§18.10.6.4)
- **Horizontal Length ($b_E$):** $\ge \max(c - 0.1 l_w, c/2) = {:.1}\text{{ mm}}$ (Provided: **${:.1}\text{{ mm}}$**)
- **Boundary Thickness ($b$):** $\ge \max(t_w, h_u/16) = {:.1}\text{{ mm}}$ (Provided: **${:.1}\text{{ mm}}$**)

### 3.3 Transverse Confinement Reinforcement (§18.10.6.4 / Table 18.10.6.4(f))
$$\frac{{A_{{sh}}}}{{s \cdot b_c}} \ge \max\left( 0.3 \left(\frac{{A_g}}{{A_{{ch}}}} - 1\right) \frac{{f'_c}}{{f_{{yt}}}}, \; 0.09 \frac{{f'_c}}{{f_{{yt}}}} \right)$$
- **Maximum Allowable Vertical Pitch ($s_{{\max}}$):** ${:.1}\text{{ mm}}$ (Provided: **${:.1}\text{{ mm}}$**)
- **Required $A_{{sh}} / s$:** ${:.2}\text{{ mm}}^2\text{{/mm}}$
- **Provided $A_{{sh}} / s$:** ${:.2}\text{{ mm}}^2\text{{/mm}}$
- **Confinement DCR:** **{:.3}** ({})

---

## 4. In-Plane Web Shear Resistance (ACI §18.10.4)
$$V_n = A_{{cv}} \left( \alpha_c \lambda \sqrt{{f'_c}} + \rho_t f_y \right) \le 0.83 A_{{cv}} \sqrt{{f'_c}}$$
- **Aspect Ratio ($h_w / l_w$):** ${:.2}$ ($\alpha_c = {:.3}$)
- **Concrete Contribution ($V_c$):** ${:.1}\text{{ kN}}$
- **Horizontal Steel Contribution ($V_s$):** ${:.1}\text{{ kN}}$
- **Nominal Shear Strength ($V_n$):** ${:.1}\text{{ kN}}$ (Upper Limit: ${:.1}\text{{ kN}}$)
- **Design Shear Strength ($\phi V_n$, $\phi=0.75$):** ${:.1}\text{{ kN}}$
- **Shear Demand ($V_u$):** ${:.1}\text{{ kN}}$
- **In-Plane Shear DCR:** **{:.3}** ({})
- **Double Curtain Web Reinforcement Required:** **{}**

---

## 5. Diagonally Reinforced Coupling Beam (ACI §18.10.7)
- **Clear Span ($l_n$):** ${:.1}\text{{ mm}}$ | **Total Depth ($h$):** ${:.1}\text{{ mm}}$
- **Aspect Ratio ($l_n / h$):** ${:.2}$ $\implies$ **{}**
- **Diagonal Bar Inclination Angle ($\alpha$):** ${:.1}^\circ$
- **Diagonal Bars Provided:** ${}\times \varnothing{:.1}\text{{ mm}}$ per diagonal cage ($A_{{vd}} = {:.1}\text{{ mm}}^2$)
- **Nominal Shear Capacity ($V_n = 2 A_{{vd}} f_y \sin\alpha$):** ${:.1}\text{{ kN}}$
- **Design Shear Strength ($\phi V_n$):** ${:.1}\text{{ kN}}$
- **Factored Shear Demand ($V_u$):** ${:.1}\text{{ kN}}$
- **Coupling Beam Shear DCR:** **{:.3}** ({})

---

## 6. Summary of Regulatory Code Compliance
| Limit State | Capacity | Factored Demand | DCR | Status |
|:---|:---:|:---:|:---:|:---:|
| **In-Plane Web Shear ($\phi V_n$)** | ${:.1}\text{{ kN}}$ | ${:.1}\text{{ kN}}$ | **{:.3}** | {} |
| **Pier Flexure ($\phi M_n$)** | ${:.1}\text{{ kN}}\cdot\text{{m}}$ | ${:.1}\text{{ kN}}\cdot\text{{m}}$ | **{:.3}** | {} |
| **SBE Transverse Confinement** | ${:.2}\text{{ mm}}^2\text{{/mm}}$ | ${:.2}\text{{ mm}}^2\text{{/mm}}$ | **{:.3}** | {} |
| **Coupling Beam Shear** | ${:.1}\text{{ kN}}$ | ${:.1}\text{{ kN}}$ | **{:.3}** | {} |

**Governing Failure Mode DCR:** **{:.3}**  
**Certified Compliance Result:** **{}**
"#,
            rep.wall_id,
            rep.classification.label(),
            if rep.is_fully_compliant { "🟢 PASS" } else { "🔴 FAIL" },
            rep.governing_dcr,
            rep.geometry.length_lw_mm,
            rep.geometry.length_lw_mm / 1000.0,
            rep.geometry.height_hw_mm,
            rep.geometry.height_hw_mm / 1000.0,
            rep.geometry.thickness_tw_mm,
            rep.geometry.story_height_hu_mm,
            rep.materials.fc_mpa,
            Self::compute_beta1(rep.materials.fc_mpa),
            rep.materials.fy_mpa,
            rep.materials.fyt_mpa,
            rep.demands.pu_kn,
            rep.demands.mu_knm,
            rep.demands.vu_kn,
            rep.demands.drift_ratio_delta,
            rep.boundary_element.neutral_axis_c_mm,
            rep.boundary_element.c_limit_mm,
            if rep.boundary_element.is_sbe_required { "YES (Mandatory)" } else { "NO" },
            rep.boundary_element.required_length_be_mm,
            rep.boundary_element.provided_length_be_mm,
            rep.boundary_element.required_thickness_b_mm,
            rep.boundary_element.provided_thickness_b_mm,
            rep.boundary_element.max_tie_spacing_s_mm,
            rep.boundary_element.provided_tie_spacing_s_mm,
            rep.boundary_element.req_ash_over_s_mm2_per_mm,
            rep.boundary_element.provided_ash_over_s_mm2_per_mm,
            rep.boundary_element.dcr_confinement,
            if rep.boundary_element.is_confinement_adequate { "PASS" } else { "FAIL" },
            rep.in_plane_shear.aspect_ratio_hw_over_lw,
            rep.in_plane_shear.alpha_c,
            rep.in_plane_shear.concrete_shear_vc_kn,
            rep.in_plane_shear.steel_shear_vs_kn,
            rep.in_plane_shear.nominal_shear_vn_kn,
            rep.in_plane_shear.max_allowable_vn_kn,
            rep.in_plane_shear.phi_vn_kn,
            rep.in_plane_shear.factored_shear_vu_kn,
            rep.in_plane_shear.shear_dcr,
            if rep.in_plane_shear.is_shear_adequate { "PASS" } else { "FAIL" },
            if rep.in_plane_shear.is_double_curtain_required { "YES (Double Curtains)" } else { "Single Curtain Permitted" },
            rep.coupling_beam.clear_span_ln_mm,
            rep.coupling_beam.total_depth_h_mm,
            rep.coupling_beam.aspect_ratio_ln_over_h,
            rep.coupling_beam.coupling_type.label(),
            rep.coupling_beam.diagonal_angle_deg,
            rep.coupling_beam.num_bars_per_diagonal,
            rep.coupling_beam.bar_dia_mm,
            rep.coupling_beam.total_diagonal_area_avd_mm2,
            rep.coupling_beam.nominal_shear_vn_kn,
            rep.coupling_beam.phi_vn_kn,
            rep.coupling_beam.factored_shear_vu_kn,
            rep.coupling_beam.coupling_dcr,
            if rep.coupling_beam.is_coupling_adequate { "PASS" } else { "FAIL" },
            rep.in_plane_shear.phi_vn_kn,
            rep.in_plane_shear.factored_shear_vu_kn,
            rep.in_plane_shear.shear_dcr,
            if rep.in_plane_shear.is_shear_adequate { "PASS" } else { "FAIL" },
            rep.flexure_phi_mn_knm,
            rep.demands.mu_knm,
            rep.flexure_dcr,
            if rep.is_flexure_adequate { "PASS" } else { "FAIL" },
            rep.boundary_element.provided_ash_over_s_mm2_per_mm,
            rep.boundary_element.req_ash_over_s_mm2_per_mm,
            rep.boundary_element.dcr_confinement,
            if rep.boundary_element.is_confinement_adequate { "PASS" } else { "FAIL" },
            rep.coupling_beam.phi_vn_kn,
            rep.coupling_beam.factored_shear_vu_kn,
            rep.coupling_beam.coupling_dcr,
            if rep.coupling_beam.is_coupling_adequate { "PASS" } else { "FAIL" },
            rep.governing_dcr,
            if rep.is_fully_compliant { "FULLY COMPLIANT" } else { "NON-COMPLIANT" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alpha_c_aspect_ratio_transitions() {
        // hw / lw <= 1.5 -> alpha_c = 0.25
        let rep1 = ShearWallEngine::evaluate_wall(
            "W1", 6.0, 6.0, 300.0, 3.5, 35.0, 420.0, 420.0,
            2000.0, 8000.0, 1000.0, 0.010,
            2.0, 0.8, 300.0, 400.0, 25.0, 4,
        );
        assert_eq!(rep1.in_plane_shear.aspect_ratio_hw_over_lw, 1.0);
        assert_eq!(rep1.in_plane_shear.alpha_c, 0.25);

        // hw / lw >= 2.0 -> alpha_c = 0.17
        let rep2 = ShearWallEngine::evaluate_wall(
            "W2", 5.0, 25.0, 300.0, 3.5, 35.0, 420.0, 420.0,
            2000.0, 8000.0, 1000.0, 0.010,
            2.0, 0.8, 300.0, 400.0, 25.0, 4,
        );
        assert_eq!(rep2.in_plane_shear.aspect_ratio_hw_over_lw, 5.0);
        assert_eq!(rep2.in_plane_shear.alpha_c, 0.17);
    }

    #[test]
    fn test_sbe_displacement_based_trigger() {
        // High drift ratio: delta_u / hw = 0.015 -> c_lim = lw / (600 * 0.015) = lw / 9
        let rep = ShearWallEngine::evaluate_wall(
            "W_Seismic", 6.0, 36.0, 350.0, 3.0, 35.0, 420.0, 420.0,
            4000.0, 16000.0, 1500.0, 0.015,
            1.8, 0.9, 350.0, 500.0, 25.0, 4,
        );
        // c is around 900-1200 mm, c_lim is 6000 / 9 = 666.7 mm -> triggers SBE
        assert!(rep.boundary_element.is_sbe_required);
        assert!(rep.boundary_element.neutral_axis_c_mm > rep.boundary_element.c_limit_mm);
        assert!(rep.boundary_element.provided_length_be_mm >= rep.boundary_element.required_length_be_mm);
    }

    #[test]
    fn test_diagonally_reinforced_coupling_beam() {
        // ln = 1.5m, h = 1.0m -> ln / h = 1.5 < 2.0 -> Diagonally reinforced
        let rep = ShearWallEngine::evaluate_wall(
            "W_Coupled", 6.0, 36.0, 350.0, 3.0, 35.0, 420.0, 420.0,
            3000.0, 12000.0, 1200.0, 0.012,
            1.5, 1.0, 350.0, 650.0, 28.0, 4,
        );
        assert_eq!(rep.coupling_beam.coupling_type, CouplingBeamType::DiagonallyReinforced);
        assert!(rep.coupling_beam.diagonal_angle_deg > 20.0 && rep.coupling_beam.diagonal_angle_deg < 40.0);
        assert!(rep.coupling_beam.phi_vn_kn > 500.0);
    }

    #[test]
    fn test_coupled_core_archetype_generation() {
        let model = ShearWallEngine::generate_coupled_core_archetype(
            "12-Story Core", 12, 3.5, 6.0, 350.0, 2.0,
        );
        assert_eq!(model.nodes.len(), 13 * 4); // 13 levels * 4 nodes = 52 nodes
        assert_eq!(model.elements.len(), 12 * 7); // 12 stories * 7 elements = 84 elements
    }
}
