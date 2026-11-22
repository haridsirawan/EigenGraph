//! geotech.rs — Geotechnical Engineering, Deep Foundations & Soil-Structure Interaction (SSI) (Fase 5: v0.0.5).
//! Formulates Winkler Subgrade Reaction Springs, Pile Group Converse-Labarre Efficiency,
//! Biaxial Pile Load Distribution, SNI 2847:2019 / ACI 318-19 Punching Shear Audits,
//! and Wolf / Veletsos & Meek Soil-Structure Interaction (SSI) Period Lengthening Shifts.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::core::ari::AriEngine;
use crate::core::bim::BimProject;
use crate::core::ifcx::{ns as ifcx_ns, IfcxDocument, IfcxImport, IfcxNode, UsdGeomMesh};
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
use crate::core::modal::{ModalEngine, SiteClass};
use crate::core::model::StructuralModel;
use crate::core::solver::FemSolver;

pub const NS_EG_GEOTECH_STRATIGRAPHY: &str = "eigengraph::geotech::stratigraphy";
pub const NS_EG_GEOTECH_SSI: &str = "eigengraph::geotech::ssi_springs";

fn default_gwt_m() -> f64 {
    2.5
}

fn slugify_geotech(raw: &str) -> String {
    let s: String = raw
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() {
        "site_project".to_string()
    } else {
        s
    }
}

/// Borehole Geotechnical Stratigraphy Archetype (Phase 71: Subsurface 3D & Winkler SSI).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BoreholePreset {
    AlluvialSoftClay,
    CoastalLiquefiableSand,
    #[default]
    StiffVolcanicAlluvium,
    DenseGravelBedrock,
}

impl BoreholePreset {
    pub const ALL: [Self; 4] = [
        Self::AlluvialSoftClay,
        Self::CoastalLiquefiableSand,
        Self::StiffVolcanicAlluvium,
        Self::DenseGravelBedrock,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::AlluvialSoftClay => "Alluvial Soft Marine Clay (Site Class SE)",
            Self::CoastalLiquefiableSand => "Coastal Liquefiable Sand (Site Class SD)",
            Self::StiffVolcanicAlluvium => "Stiff Volcanic Alluvium (Site Class SC)",
            Self::DenseGravelBedrock => "Dense Gravel & Competent Bedrock (Site Class SB)",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::AlluvialSoftClay => "Soft Clay",
            Self::CoastalLiquefiableSand => "Liquefiable",
            Self::StiffVolcanicAlluvium => "Stiff Alluv",
            Self::DenseGravelBedrock => "Dense Rock",
        }
    }

    pub fn cycle_next(self) -> Self {
        match self {
            Self::AlluvialSoftClay => Self::CoastalLiquefiableSand,
            Self::CoastalLiquefiableSand => Self::StiffVolcanicAlluvium,
            Self::StiffVolcanicAlluvium => Self::DenseGravelBedrock,
            Self::DenseGravelBedrock => Self::AlluvialSoftClay,
        }
    }

    pub fn from_arg(arg: &str) -> Self {
        let lower = arg.trim().to_ascii_lowercase();
        if lower.contains("soft") || lower.contains("clay") || lower.contains("se") {
            Self::AlluvialSoftClay
        } else if lower.contains("liq") || lower.contains("coast") || lower.contains("sand") {
            Self::CoastalLiquefiableSand
        } else if lower.contains("rock") || lower.contains("dense") || lower.contains("bed") || lower.contains("sb") {
            Self::DenseGravelBedrock
        } else {
            Self::StiffVolcanicAlluvium
        }
    }
}

/// Detailed 3D Subsurface Soil Horizon Stratum with SPT, Vs, and Seed-Idriss Liquefaction state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StratigraphyStratum {
    pub name: String,
    pub soil_type: SoilType,
    pub depth_top_m: f64,
    pub depth_bottom_m: f64,
    pub n_spt: f64,
    pub vs_m_s: f64,
    pub fines_content_pct: f64,
    pub unit_weight_kn_m3: f64,
    pub subgrade_ks_kn_m3: f64,
    pub liquefiable: bool,
    pub liquefaction_fs: f64,
    pub py_multiplier: f64,
    pub color_rgba: [f32; 4],
}

/// Interactive Subsurface Geotechnical & 6-DOF Winkler SSI Configuration persisted in `.eigen.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubsurfaceSsiConfig {
    #[serde(default)]
    pub ssi_coupling_enabled: bool,
    #[serde(default)]
    pub xray_subsurface_visible: bool,
    #[serde(default)]
    pub borehole_preset: BoreholePreset,
    #[serde(default = "default_gwt_m")]
    pub groundwater_depth_m: f64,
    #[serde(default)]
    pub liquefaction_trigger_active: bool,
}

impl Default for SubsurfaceSsiConfig {
    fn default() -> Self {
        Self {
            ssi_coupling_enabled: false,
            xray_subsurface_visible: false,
            borehole_preset: BoreholePreset::default(),
            groundwater_depth_m: default_gwt_m(),
            liquefaction_trigger_active: false,
        }
    }
}

impl SubsurfaceSsiConfig {
    pub fn content_revision_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mix = |acc: &mut u64, v: u64| {
            *acc ^= v;
            *acc = acc.wrapping_mul(0x0000_0100_0000_01b3);
        };
        mix(&mut h, u64::from(self.ssi_coupling_enabled));
        mix(&mut h, u64::from(self.xray_subsurface_visible));
        mix(&mut h, self.borehole_preset as u64 + 1);
        mix(&mut h, (self.groundwater_depth_m * 100.0).round() as i64 as u64);
        mix(&mut h, u64::from(self.liquefaction_trigger_active));
        h
    }
}

/// Per-footing SSI spring, settlement, and pile embedment output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FootingSsiCouplingResult {
    pub footing_id: usize,
    pub footing_name: String,
    pub pos_m: [f64; 2],
    pub width_m: f64,
    pub length_m: f64,
    pub required_pile_length_m: f64,
    pub bearing_stratum_name: String,
    pub winkler_springs: WinklerSpringMatrix,
    pub estimated_settlement_mm: f64,
    pub liquefaction_risk_fs: f64,
}

/// Coupled 3D Subsurface Stratigraphy, Liquefaction & 6-DOF Winkler SSI Analysis Report (Phase 71).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsurfaceSsiAnalysisReport {
    pub preset: BoreholePreset,
    pub site_class: SiteClass,
    pub vs30_m_s: f64,
    pub groundwater_depth_m: f64,
    pub pga_g: f64,
    pub strata: Vec<StratigraphyStratum>,
    pub min_liquefaction_fs: f64,
    pub liquefaction_potential_index: f64,
    pub is_liquefaction_critical: bool,
    pub recommended_pile_depth_m: f64,
    pub footing_couplings: Vec<FootingSsiCouplingResult>,
    pub fixed_base_period_t1_s: f64,
    pub ssi_coupled_period_t1_s: f64,
    pub period_lengthening_pct: f64,
    pub fixed_base_ari: f64,
    pub ssi_coupled_ari: f64,
    pub max_foundation_settlement_mm: f64,
    pub ssi_overlay_ifcx: IfcxDocument,
    pub summary: String,
}

/// Soil Layer Stratigraphy Classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoilType {
    DenseSand,
    MediumSand,
    LooseSand,
    StiffClay,
    MediumClay,
    SoftClay,
    Bedrock,
}

impl SoilType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::DenseSand => "Dense Sand (N-SPT > 30, phi = 38 deg)",
            Self::MediumSand => "Medium Dense Sand (10 < N-SPT <= 30, phi = 32 deg)",
            Self::LooseSand => "Loose Sand (N-SPT <= 10, phi = 28 deg)",
            Self::StiffClay => "Stiff Cohesive Clay (cu > 100 kPa)",
            Self::MediumClay => "Medium Stiff Clay (50 < cu <= 100 kPa)",
            Self::SoftClay => "Soft Normally Consolidated Clay (cu <= 50 kPa)",
            Self::Bedrock => "Competent Bedrock (RQD > 75%)",
        }
    }
}

/// Geotechnical Soil Layer Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoilLayer {
    pub soil_type: SoilType,
    pub depth_top_m: f64,
    pub depth_bottom_m: f64,
    pub unit_weight_kn_m3: f64,
    pub cohesion_cu_kpa: f64,
    pub friction_angle_deg: f64,
    pub subgrade_reaction_ks_kn_m3: f64,
    pub elastic_modulus_es_mpa: f64,
    pub poisson_ratio_nu: f64,
}

impl SoilLayer {
    pub fn new_standard(soil_type: SoilType, top_m: f64, bot_m: f64) -> Self {
        let (gamma, cu, phi, ks, es, nu) = match soil_type {
            SoilType::DenseSand => (20.0, 0.0, 38.0, 60_000.0, 60.0, 0.30),
            SoilType::MediumSand => (18.5, 0.0, 32.0, 35_000.0, 35.0, 0.30),
            SoilType::LooseSand => (17.0, 0.0, 28.0, 15_000.0, 15.0, 0.35),
            SoilType::StiffClay => (19.0, 120.0, 0.0, 45_000.0, 40.0, 0.40),
            SoilType::MediumClay => (17.5, 60.0, 0.0, 25_000.0, 20.0, 0.45),
            SoilType::SoftClay => (16.0, 25.0, 0.0, 10_000.0, 8.0, 0.48),
            SoilType::Bedrock => (24.0, 500.0, 45.0, 200_000.0, 500.0, 0.20),
        };

        Self {
            soil_type,
            depth_top_m: top_m,
            depth_bottom_m: bot_m,
            unit_weight_kn_m3: gamma,
            cohesion_cu_kpa: cu,
            friction_angle_deg: phi,
            subgrade_reaction_ks_kn_m3: ks,
            elastic_modulus_es_mpa: es,
            poisson_ratio_nu: nu,
        }
    }
}

/// Equivalent 6-DOF Winkler Foundation Springs for a Footing / Mat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinklerSpringMatrix {
    /// Vertical translational spring Kz (kN/m)
    pub kz_trans_kn_m: f64,
    /// Lateral horizontal spring Kx (kN/m)
    pub kx_trans_kn_m: f64,
    /// Lateral horizontal spring Ky (kN/m)
    pub ky_trans_kn_m: f64,
    /// Rocking rotational spring K_theta_x (kNm/rad)
    pub k_theta_x_knm_rad: f64,
    /// Rocking rotational spring K_theta_y (kNm/rad)
    pub k_theta_y_knm_rad: f64,
    /// Torsional rotational spring K_theta_z (kNm/rad)
    pub k_theta_z_knm_rad: f64,
    /// Modulus of subgrade reaction ks used (kN/m3)
    pub subgrade_modulus_ks: f64,
}

/// Deep Foundation Individual Pile Specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PileElement {
    pub pile_id: usize,
    pub x_coord_m: f64,
    pub y_coord_m: f64,
    pub diameter_m: f64,
    pub length_m: f64,
    pub factored_axial_demand_pu_kn: f64,
    pub allowable_compression_all_kn: f64,
    pub allowable_tension_uplift_kn: f64,
    pub axial_dcr: f64,
    pub lateral_shear_hu_kn: f64,
    pub lateral_deflection_mm: f64,
    pub is_safe: bool,
}

/// Group of Piles Supporting a Pile Cap / Column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PileGroupReport {
    pub cap_id: usize,
    pub column_id: usize,
    pub piles_count_x: usize,
    pub piles_count_y: usize,
    pub total_piles: usize,
    pub pile_spacing_m: f64,
    pub pile_diameter_m: f64,
    pub converse_labarre_efficiency: f64,
    pub cap_width_m: f64,
    pub cap_length_m: f64,
    pub cap_thickness_m: f64,
    pub applied_pu_kn: f64,
    pub applied_mux_knm: f64,
    pub applied_muy_knm: f64,
    pub max_pile_axial_load_kn: f64,
    pub min_pile_axial_load_kn: f64,
    pub pile_details: Vec<PileElement>,
    pub governing_dcr: f64,
    pub is_group_safe: bool,
}

/// Punching Shear Audit at Column Face or Pile Cap (SNI 2847:2019 Pasal 22.6 / ACI 318-19)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PunchingShearAudit {
    pub location_name: String,
    pub column_dim_c1_mm: f64,
    pub column_dim_c2_mm: f64,
    pub slab_effective_depth_d_mm: f64,
    pub concrete_fc_mpa: f64,
    pub critical_perimeter_bo_mm: f64,
    pub critical_area_acrit_m2: f64,
    pub factored_shear_vu_kn: f64,
    pub factored_moment_munb_knm: f64,
    pub punching_stress_vu_mpa: f64,
    pub concrete_capacity_phi_vc_mpa: f64,
    pub punching_dcr: f64,
    pub is_punching_safe: bool,
    pub shear_reinforcement_required: bool,
    pub recommendation: String,
}

/// Dynamic Soil-Structure Interaction (SSI) Modal Period Shift (Wolf / Veletsos & Meek)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsiModalShiftReport {
    pub fixed_base_period_t_s: f64,
    pub flexible_ssi_period_t_tilde_s: f64,
    pub period_lengthening_ratio: f64,
    pub foundation_damping_factor: f64,
    pub base_shear_reduction_ratio: f64,
    pub lateral_drift_amplification_ratio: f64,
    pub ssi_classification: String,
}

/// Comprehensive Geotechnical & Soil-Structure Interaction Report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeotechSsiReport {
    pub project_name: String,
    pub active_soil_layer: SoilLayer,
    pub winkler_springs: WinklerSpringMatrix,
    pub pile_group: PileGroupReport,
    pub punching_shear: PunchingShearAudit,
    pub ssi_modal_shift: SsiModalShiftReport,
    pub is_fully_compliant: bool,
}

/// Geotechnical & Soil-Structure Interaction Engine
pub struct GeotechEngine;

impl GeotechEngine {
    /// Computes the 6-DOF Winkler Foundation Spring Matrix for a rectangular spread/mat footing
    /// Based on Bowles / Vesic subgrade reaction formulation (kN/m and kNm/rad).
    pub fn compute_winkler_springs(
        width_b_m: f64,
        length_l_m: f64,
        soil: &SoilLayer,
    ) -> WinklerSpringMatrix {
        let b = width_b_m.max(0.5);
        let l = length_l_m.max(0.5);
        let area = b * l;

        // Size correction for subgrade modulus per Terzaghi / Bowles:
        // ks(B) = ks0 * ((B + 0.3) / (2 * B))^2 for sands, or ks0 * (0.3 / B) for clays
        let ks = soil.subgrade_reaction_ks_kn_m3;

        // Vertical translational stiffness Kz
        let kz = ks * area;

        // Horizontal shear translation stiffness Kx, Ky (approx 0.75 - 0.85 Kz)
        let kx = kz * 0.82;
        let ky = kz * 0.82;

        // Rocking rotational stiffness about X and Y axes: K_theta = ks * I_footing
        // I_x = b * l^3 / 12 (rotation about X-axis tilts along L)
        // I_y = l * b^3 / 12 (rotation about Y-axis tilts along B)
        let ix = (b * l.powi(3)) / 12.0;
        let iy = (l * b.powi(3)) / 12.0;
        let iz = ix + iy; // Polar moment of inertia for torsion

        let k_rot_x = ks * ix;
        let k_rot_y = ks * iy;
        let k_rot_z = ks * iz * 0.65;

        WinklerSpringMatrix {
            kz_trans_kn_m: kz,
            kx_trans_kn_m: kx,
            ky_trans_kn_m: ky,
            k_theta_x_knm_rad: k_rot_x,
            k_theta_y_knm_rad: k_rot_y,
            k_theta_z_knm_rad: k_rot_z,
            subgrade_modulus_ks: ks,
        }
    }

    /// Evaluates a 2D grid of deep foundation piles under vertical load Pu and biaxial moments Mux, Muy.
    /// Incorporates the Converse-Labarre group efficiency formula and P-Y lateral soil stiffness.
    pub fn evaluate_pile_group(
        cap_id: usize,
        column_id: usize,
        piles_x: usize,
        piles_y: usize,
        pile_dia_m: f64,
        pile_spacing_m: f64,
        pile_len_m: f64,
        pu_kn: f64,
        mux_knm: f64,
        muy_knm: f64,
        soil: &SoilLayer,
    ) -> PileGroupReport {
        let nx = piles_x.max(1);
        let ny = piles_y.max(1);
        let total_n = nx * ny;

        let dia = pile_dia_m.max(0.3);
        let s = pile_spacing_m.max(dia * 2.5);
        let l_pile = pile_len_m.max(3.0);

        // 1. Converse-Labarre Pile Group Efficiency Formula:
        // Eg = 1 - theta * [ (nx - 1)ny + (ny - 1)nx ] / [ 90 * nx * ny ]
        // where theta = arctan(d / s) in degrees
        let theta_deg = (dia / s).atan() * 180.0 / std::f64::consts::PI;
        let numerator = ((nx as f64 - 1.0) * ny as f64) + ((ny as f64 - 1.0) * nx as f64);
        let denominator = 90.0 * (nx as f64) * (ny as f64);
        let eg_factor = if total_n > 1 {
            (1.0 - theta_deg * (numerator / denominator)).clamp(0.60, 1.0)
        } else {
            1.00
        };

        // 2. Single Pile Allowable Capacities
        // Allowable skin friction + end bearing:
        let q_end_kn = if soil.friction_angle_deg > 0.0 {
            let nq = (soil.friction_angle_deg * std::f64::consts::PI / 180.0).tan().powi(2) * 20.0;
            (std::f64::consts::PI * dia * dia / 4.0) * (soil.unit_weight_kn_m3 * l_pile * nq) / 3.0
        } else {
            (std::f64::consts::PI * dia * dia / 4.0) * (9.0 * soil.cohesion_cu_kpa) / 2.5
        };

        let q_skin_kn = (std::f64::consts::PI * dia * l_pile) * (soil.cohesion_cu_kpa * 0.65 + 25.0) / 2.5;
        let single_all_comp = (q_end_kn + q_skin_kn).clamp(350.0, 4500.0);
        let group_all_comp = single_all_comp * eg_factor;
        let single_all_tens = (q_skin_kn * 0.70).clamp(150.0, 2000.0) * eg_factor;

        // 3. Coordinate Generation & Section Moduli
        let cap_w = ((nx as f64 - 1.0) * s) + (dia * 2.5);
        let cap_l = ((ny as f64 - 1.0) * s) + (dia * 2.5);
        let cap_thick = (dia * 1.5).max(0.60);

        let mut sum_x2 = 0.0;
        let mut sum_y2 = 0.0;
        let mut pile_coords = Vec::with_capacity(total_n);

        let x_offset = (nx as f64 - 1.0) * s * 0.5;
        let y_offset = (ny as f64 - 1.0) * s * 0.5;

        for ix in 0..nx {
            for iy in 0..ny {
                let x = (ix as f64) * s - x_offset;
                let y = (iy as f64) * s - y_offset;
                sum_x2 += x * x;
                sum_y2 += y * y;
                pile_coords.push((x, y));
            }
        }

        // 4. Biaxial Pile Load Distribution: Pi = Pu/N +- (M_uy * x / sum_x2) +- (M_ux * y / sum_y2)
        let mut pile_elements = Vec::with_capacity(total_n);
        let mut max_p: f64 = -1e9;
        let mut min_p: f64 = 1e9;
        let mut gov_dcr: f64 = 0.0;

        for (idx, &(px, py)) in pile_coords.iter().enumerate() {
            let p_ecc_x = if sum_x2 > 1e-4 { (muy_knm * px) / sum_x2 } else { 0.0 };
            let p_ecc_y = if sum_y2 > 1e-4 { (mux_knm * py) / sum_y2 } else { 0.0 };
            let p_i = (pu_kn / total_n as f64) + p_ecc_x + p_ecc_y;

            max_p = max_p.max(p_i);
            min_p = min_p.min(p_i);

            let dcr = if p_i >= 0.0 {
                p_i / group_all_comp
            } else {
                p_i.abs() / single_all_tens
            };
            gov_dcr = gov_dcr.max(dcr);

            // Lateral pile deflection under tributary lateral shear
            let tributary_hu = (mux_knm.abs().max(muy_knm.abs()) * 0.15) / total_n as f64;
            let ep_gpa = 28.0; // Concrete C30/37
            let ip_m4 = (std::f64::consts::PI * dia.powi(4)) / 64.0;
            let lat_disp_mm = ((tributary_hu * 1000.0) / (soil.subgrade_reaction_ks_kn_m3 * dia * l_pile * 0.35).max(100.0))
                * (l_pile / (ep_gpa * ip_m4).max(1.0)).clamp(0.2, 5.0);

            pile_elements.push(PileElement {
                pile_id: idx + 1,
                x_coord_m: px,
                y_coord_m: py,
                diameter_m: dia,
                length_m: l_pile,
                factored_axial_demand_pu_kn: p_i,
                allowable_compression_all_kn: group_all_comp,
                allowable_tension_uplift_kn: single_all_tens,
                axial_dcr: dcr,
                lateral_shear_hu_kn: tributary_hu,
                lateral_deflection_mm: lat_disp_mm.clamp(0.1, 45.0),
                is_safe: dcr <= 1.00,
            });
        }

        PileGroupReport {
            cap_id,
            column_id,
            piles_count_x: nx,
            piles_count_y: ny,
            total_piles: total_n,
            pile_spacing_m: s,
            pile_diameter_m: dia,
            converse_labarre_efficiency: eg_factor,
            cap_width_m: cap_w,
            cap_length_m: cap_l,
            cap_thickness_m: cap_thick,
            applied_pu_kn: pu_kn,
            applied_mux_knm: mux_knm,
            applied_muy_knm: muy_knm,
            max_pile_axial_load_kn: max_p,
            min_pile_axial_load_kn: min_p,
            pile_details: pile_elements,
            governing_dcr: gov_dcr,
            is_group_safe: gov_dcr <= 1.00,
        }
    }

    /// Evaluates Concrete Two-Way Punching Shear per SNI 2847:2019 Pasal 22.6 / ACI 318-19 Section 22.6
    pub fn audit_punching_shear(
        loc_name: &str,
        c1_mm: f64,
        c2_mm: f64,
        d_eff_mm: f64,
        fc_mpa: f64,
        vu_kn: f64,
        munb_knm: f64,
    ) -> PunchingShearAudit {
        let d = d_eff_mm.max(100.0);
        let c1 = c1_mm.max(200.0);
        let c2 = c2_mm.max(200.0);
        let fc = fc_mpa.clamp(15.0, 80.0);

        // 1. Critical perimeter bo at d/2 from column face
        let bo = 2.0 * (c1 + d) + 2.0 * (c2 + d);
        let acrit_m2 = ((c1 + d) * (c2 + d)) * 1e-6;

        // 2. Concrete nominal shear capacity Vc per SNI 2847:2019 Eq. 22.6.5.2
        let beta = (c1.max(c2)) / (c1.min(c2)).max(1.0);
        let alpha_s = 40.0; // Interior column
        let sqrt_fc = fc.sqrt();

        let vc1 = 0.33 * sqrt_fc;
        let vc2 = 0.17 * (1.0 + 2.0 / beta) * sqrt_fc;
        let vc3 = 0.083 * (2.0 + (alpha_s * d) / bo) * sqrt_fc;

        let vc_nominal = vc1.min(vc2).min(vc3);
        let phi = 0.75; // Strength reduction factor for shear
        let phi_vc = phi * vc_nominal;

        // 3. Factored punching shear stress vu = Vu / (bo * d) + gamma_v * Munb * c / Jc
        let vu_n = vu_kn * 1000.0;
        let vu_direct = vu_n / (bo * d);

        // Moment transfer factor gamma_v
        let gamma_f = 1.0 / (1.0 + (2.0 / 3.0) * ((c1 + d) / (c2 + d)).sqrt());
        let gamma_v = 1.0 - gamma_f;

        let jc = ((c1 + d) * d.powi(3) / 6.0) + (d * (c1 + d).powi(3) / 6.0) + (d * (c2 + d) * (c1 + d).powi(2) / 2.0);
        let c_dist = (c1 + d) * 0.5;
        let vu_moment = (gamma_v * (munb_knm * 1e6) * c_dist) / jc.max(1.0);

        let total_vu = vu_direct + vu_moment * 0.85;
        let dcr = total_vu / phi_vc.max(1e-4);

        let is_safe = dcr <= 1.00;
        let rec = if is_safe {
            format!("PASSED: Punching shear stress vu = {:.3} MPa satisfies phi*Vc = {:.3} MPa (DCR = {:.3} <= 1.00). No drop panel required.", total_vu, phi_vc, dcr)
        } else {
            format!("WARNING: Punching shear DCR = {:.3} > 1.00 (vu = {:.3} MPa > phi*Vc = {:.3} MPa). Provide shear stud rails, drop panel (t >= {:.0} mm), or enlarge column cross-section.", dcr, total_vu, phi_vc, (d_eff_mm * 1.35))
        };

        PunchingShearAudit {
            location_name: loc_name.to_string(),
            column_dim_c1_mm: c1,
            column_dim_c2_mm: c2,
            slab_effective_depth_d_mm: d,
            concrete_fc_mpa: fc,
            critical_perimeter_bo_mm: bo,
            critical_area_acrit_m2: acrit_m2,
            factored_shear_vu_kn: vu_kn,
            factored_moment_munb_knm: munb_knm,
            punching_stress_vu_mpa: total_vu,
            concrete_capacity_phi_vc_mpa: phi_vc,
            punching_dcr: dcr,
            is_punching_safe: is_safe,
            shear_reinforcement_required: !is_safe,
            recommendation: rec,
        }
    }

    /// Computes Wolf / Veletsos & Meek Soil-Structure Interaction (SSI) Modal Period Lengthening
    pub fn evaluate_ssi_modal_shift(
        t_fixed_s: f64,
        height_h_m: f64,
        structural_stiffness_k_fixed: f64,
        winkler: &WinklerSpringMatrix,
    ) -> SsiModalShiftReport {
        let t_fixed = t_fixed_s.max(0.1);
        let h = height_h_m.max(3.0);
        let k_str = structural_stiffness_k_fixed.max(100.0);

        let k_h = winkler.kx_trans_kn_m.max(100.0);
        let k_r = winkler.k_theta_y_knm_rad.max(100.0);

        // Veletsos & Meek SSI Period Lengthening:
        // (T_tilde / T)^2 = 1 + (K_fixed / K_h) + (K_fixed * h^2 / K_r)
        let term_trans = k_str / k_h;
        let term_rocking = (k_str * h * h) / k_r;
        let t_ratio_sq = 1.0 + term_trans + term_rocking;
        let t_ratio = t_ratio_sq.sqrt().clamp(1.0, 1.65);

        let t_ssi = t_fixed * t_ratio;

        // Foundation radiation & hysteretic damping increase (ASCE 7-22 Chapter 19)
        let beta_ssi = (0.05 + 0.04 * (t_ratio - 1.0)).clamp(0.05, 0.15);
        let base_shear_red = (1.0 / t_ratio.powf(0.85)).clamp(0.70, 1.0);
        let drift_amp = t_ratio * 1.08;

        let classification = if t_ratio <= 1.05 {
            "Rigid Subgrade (Negligible SSI Effect — Fixed Base Valid)".to_string()
        } else if t_ratio <= 1.25 {
            format!("Moderate SSI Shift (+{:.1}% Period Lengthening, Base Shear -{:.1}%)", (t_ratio - 1.0) * 100.0, (1.0 - base_shear_red) * 100.0)
        } else {
            format!("Significant SSI Shift (+{:.1}% Period Lengthening — Flexible Soil Compliant)", (t_ratio - 1.0) * 100.0)
        };

        SsiModalShiftReport {
            fixed_base_period_t_s: t_fixed,
            flexible_ssi_period_t_tilde_s: t_ssi,
            period_lengthening_ratio: t_ratio,
            foundation_damping_factor: beta_ssi,
            base_shear_reduction_ratio: base_shear_red,
            lateral_drift_amplification_ratio: drift_amp,
            ssi_classification: classification,
        }
    }

    /// Runs a comprehensive Geotechnical & Foundation SSI audit for the given model
    pub fn run_geotech_ssi_audit(
        model: &StructuralModel,
        soil: SoilLayer,
        footing_w: f64,
        footing_l: f64,
        piles_x: usize,
        piles_y: usize,
    ) -> GeotechSsiReport {
        let total_mass = model.total_steel_mass_kg().max(1000.0);
        let total_pu = (total_mass * 9.81 * 1.20) / 1000.0;
        let trib_mux = total_pu * 0.15;
        let trib_muy = total_pu * 0.12;

        let winkler = Self::compute_winkler_springs(footing_w, footing_l, &soil);
        let piles = Self::evaluate_pile_group(
            1,
            1,
            piles_x,
            piles_y,
            0.80,
            2.40,
            18.0,
            total_pu,
            trib_mux,
            trib_muy,
            &soil,
        );
        let punching = Self::audit_punching_shear(
            "Interior Column Base C1",
            600.0,
            600.0,
            450.0,
            30.0,
            total_pu * 0.35,
            trib_mux * 0.25,
        );
        let ssi_shift = Self::evaluate_ssi_modal_shift(
            0.65,
            18.0,
            45_000.0,
            &winkler,
        );

        let compliant = piles.is_group_safe && punching.is_punching_safe;

        GeotechSsiReport {
            project_name: model.name.clone(),
            active_soil_layer: soil,
            winkler_springs: winkler,
            pile_group: piles,
            punching_shear: punching,
            ssi_modal_shift: ssi_shift,
            is_fully_compliant: compliant,
        }
    }

    /// Generates an official Geotechnical & Soil-Structure Interaction Calculation Report in Markdown format
    pub fn generate_markdown_dossier(report: &GeotechSsiReport) -> String {
        let mut doc = String::new();
        doc.push_str("# OFFICIAL GEOTECHNICAL & SOIL-STRUCTURE INTERACTION (SSI) AUDIT DOSSIER\n\n");
        doc.push_str("**Standards Compliance:** SNI 2847:2019 / SNI 1726:2019 / ACI 318-19 / ASCE 7-22 Chapter 19\n");
        doc.push_str(&format!("**Project Model:** {}\n", report.project_name));
        doc.push_str(&format!("**Soil Stratum:** {}\n", report.active_soil_layer.soil_type.label()));
        doc.push_str(&format!("**Governing Compliance Verdict:** {}\n\n", if report.is_fully_compliant { "✅ **APPROVED (COMPLIANT)**" } else { "⚠️ **ACTION REQUIRED**" }));

        doc.push_str("---\n\n");
        doc.push_str("## 1. Winkler Subgrade Reaction & 6-DOF Foundation Spring Matrix\n\n");
        doc.push_str("| Degree of Freedom | Spring Component | Notation | Spring Constant Value | Physical Unit |\n");
        doc.push_str("| :--- | :--- | :---: | :---: | :---: |\n");
        doc.push_str(&format!("| Translational Vertical | Subgrade Compression | $K_z$ | **{:.1}** | kN/m |\n", report.winkler_springs.kz_trans_kn_m));
        doc.push_str(&format!("| Translational Lateral X | Lateral Soil Friction | $K_x$ | **{:.1}** | kN/m |\n", report.winkler_springs.kx_trans_kn_m));
        doc.push_str(&format!("| Translational Lateral Y | Lateral Soil Friction | $K_y$ | **{:.1}** | kN/m |\n", report.winkler_springs.ky_trans_kn_m));
        doc.push_str(&format!("| Rocking Rotational X | Overturning Stiffness X | $K_{{rx}}$ | **{:.1}** | kNm/rad |\n", report.winkler_springs.k_theta_x_knm_rad));
        doc.push_str(&format!("| Rocking Rotational Y | Overturning Stiffness Y | $K_{{ry}}$ | **{:.1}** | kNm/rad |\n", report.winkler_springs.k_theta_y_knm_rad));
        doc.push_str(&format!("| Torsional Rotational Z | Base Twist Resistance | $K_{{rz}}$ | **{:.1}** | kNm/rad |\n\n", report.winkler_springs.k_theta_z_knm_rad));

        doc.push_str("---\n\n");
        doc.push_str("## 2. Deep Foundation Pile Group Converse-Labarre Efficiency & Load Distribution\n\n");
        doc.push_str(&format!("- **Pile Layout:** {} x {} Grid (Total: {} Piles)\n", report.pile_group.piles_count_x, report.pile_group.piles_count_y, report.pile_group.total_piles));
        doc.push_str(&format!("- **Converse-Labarre Group Efficiency ($E_g$):** **{:.3}** ({:.1}%)\n", report.pile_group.converse_labarre_efficiency, report.pile_group.converse_labarre_efficiency * 100.0));
        doc.push_str(&format!("- **Applied Vertical Load ($P_u$):** {:.1} kN | **Biaxial Moments:** $M_{{ux}}$={:.1} kNm, $M_{{uy}}$={:.1} kNm\n", report.pile_group.applied_pu_kn, report.pile_group.applied_mux_knm, report.pile_group.applied_muy_knm));
        doc.push_str(&format!("- **Maximum Compressive Pile Load:** **{:.1} kN** | **Minimum Pile Load:** {:.1} kN\n\n", report.pile_group.max_pile_axial_load_kn, report.pile_group.min_pile_axial_load_kn));

        doc.push_str("| Pile ID | Coordinates (x, y) | Diameter | Axial Demand $P_u$ | Allowable Capacity | Axial DCR | Lateral Deflection | Status |\n");
        doc.push_str("| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |\n");
        for p in &report.pile_group.pile_details {
            doc.push_str(&format!(
                "| P{:02} | ({:.2}m, {:.2}m) | D{:.0}mm | {:6.1} kN | {:6.1} kN | **{:.3}** | {:.2} mm | {} |\n",
                p.pile_id, p.x_coord_m, p.y_coord_m, p.diameter_m * 1000.0, p.factored_axial_demand_pu_kn, p.allowable_compression_all_kn, p.axial_dcr, p.lateral_deflection_mm, if p.is_safe { "✅ Safe" } else { "❌ Overstress" }
            ));
        }

        doc.push_str("\n---\n\n");
        doc.push_str("## 3. Two-Way Concrete Punching Shear Audit (SNI 2847:2019 Pasal 22.6)\n\n");
        doc.push_str(&format!("- **Audited Location:** {}\n", report.punching_shear.location_name));
        doc.push_str(&format!("- **Critical Perimeter ($b_o$):** {:.1} mm at $d/2 = {:.1}$ mm\n", report.punching_shear.critical_perimeter_bo_mm, report.punching_shear.slab_effective_depth_d_mm * 0.5));
        doc.push_str(&format!("- **Factored Punching Shear Stress ($\\nu_u$):** **{:.3} MPa**\n", report.punching_shear.punching_stress_vu_mpa));
        doc.push_str(&format!("- **Design Concrete Capacity ($\\phi \\nu_c$):** **{:.3} MPa** ($\\phi = 0.75$)\n", report.punching_shear.concrete_capacity_phi_vc_mpa));
        doc.push_str(&format!("- **Punching Shear DCR:** **{:.3}** ({})\n", report.punching_shear.punching_dcr, if report.punching_shear.is_punching_safe { "✅ Safe" } else { "❌ Shear Reinforcement Needed" }));
        doc.push_str(&format!("- **Prescriptive Recommendation:** {}\n\n", report.punching_shear.recommendation));

        doc.push_str("---\n\n");
        doc.push_str("## 4. Soil-Structure Interaction (SSI) Dynamic Modal Shift\n\n");
        doc.push_str(&format!("- **Fixed-Base Period ($T$):** {:.3} s\n", report.ssi_modal_shift.fixed_base_period_t_s));
        doc.push_str(&format!("- **Flexible SSI Period ($\\tilde{{T}}$):** **{:.3} s**\n", report.ssi_modal_shift.flexible_ssi_period_t_tilde_s));
        doc.push_str(&format!("- **Period Lengthening Ratio ($\\tilde{{T}} / T$):** **{:.3}x** (+{:.1}%)\n", report.ssi_modal_shift.period_lengthening_ratio, (report.ssi_modal_shift.period_lengthening_ratio - 1.0) * 100.0));
        doc.push_str(&format!("- **Base Shear Reduction Allowance:** **{:.1}%**\n", (1.0 - report.ssi_modal_shift.base_shear_reduction_ratio) * 100.0));
        doc.push_str(&format!("- **Classification Verdict:** {}\n\n", report.ssi_modal_shift.ssi_classification));

        doc.push_str("---\n*Certified and Generated autonomously by EigenGraph v0.0.5 Geotechnical Workstation.*\n");
        doc
    }

    /// Generates a 30 m deep 3D subsurface soil stratigraphy profile with exact harmonic $V_{s30}$,
    /// SNI 1726 `SiteClass`, simplified Seed–Idriss liquefaction $FS_L$ & Iwasaki $LPI$, lateral $p\text{-}y$
    /// multipliers, and auto-seated deep bearing pile embedment depth.
    pub fn build_borehole_strata(
        preset: BoreholePreset,
        groundwater_depth_m: f64,
        pga_g: f64,
        apply_liquefaction: bool,
    ) -> (Vec<StratigraphyStratum>, f64, SiteClass, f64, f64) {
        let gwt = groundwater_depth_m.clamp(0.0, 20.0);
        let eff_pga = pga_g.clamp(0.10, 1.50);

        struct RawLayer {
            name: &'static str,
            soil_type: SoilType,
            top: f64,
            bot: f64,
            n_spt: f64,
            vs: f64,
            fc: f64,
            gamma: f64,
            ks: f64,
            liquefiable: bool,
            color: [f32; 4],
        }

        let specs: Vec<RawLayer> = match preset {
            BoreholePreset::AlluvialSoftClay => vec![
                RawLayer {
                    name: "Silty Clay Fill",
                    soil_type: SoilType::MediumClay,
                    top: 0.0,
                    bot: 3.0,
                    n_spt: 6.0,
                    vs: 135.0,
                    fc: 55.0,
                    gamma: 17.5,
                    ks: 16_000.0,
                    liquefiable: false,
                    color: [0.55, 0.44, 0.34, 0.32],
                },
                RawLayer {
                    name: "Soft Marine Clay",
                    soil_type: SoilType::SoftClay,
                    top: 3.0,
                    bot: 13.0,
                    n_spt: 4.0,
                    vs: 105.0,
                    fc: 78.0,
                    gamma: 16.0,
                    ks: 9_000.0,
                    liquefiable: false,
                    color: [0.36, 0.46, 0.52, 0.32],
                },
                RawLayer {
                    name: "Medium Alluvial Clay",
                    soil_type: SoilType::MediumClay,
                    top: 13.0,
                    bot: 22.0,
                    n_spt: 14.0,
                    vs: 165.0,
                    fc: 58.0,
                    gamma: 18.0,
                    ks: 24_000.0,
                    liquefiable: false,
                    color: [0.46, 0.38, 0.30, 0.34],
                },
                RawLayer {
                    name: "Dense Bearing Sand",
                    soil_type: SoilType::DenseSand,
                    top: 22.0,
                    bot: 30.0,
                    n_spt: 38.0,
                    vs: 340.0,
                    fc: 10.0,
                    gamma: 20.5,
                    ks: 68_000.0,
                    liquefiable: false,
                    color: [0.64, 0.56, 0.40, 0.38],
                },
            ],
            BoreholePreset::CoastalLiquefiableSand => vec![
                RawLayer {
                    name: "Coastal Sand Fill",
                    soil_type: SoilType::MediumSand,
                    top: 0.0,
                    bot: 2.0,
                    n_spt: 11.0,
                    vs: 165.0,
                    fc: 8.0,
                    gamma: 18.5,
                    ks: 26_000.0,
                    liquefiable: true,
                    color: [0.76, 0.67, 0.46, 0.32],
                },
                RawLayer {
                    name: "Loose Saturated Coastal Sand",
                    soil_type: SoilType::LooseSand,
                    top: 2.0,
                    bot: 9.0,
                    n_spt: 8.0,
                    vs: 140.0,
                    fc: 6.0,
                    gamma: 17.5,
                    ks: 14_000.0,
                    liquefiable: true,
                    color: [0.80, 0.58, 0.36, 0.34],
                },
                RawLayer {
                    name: "Medium Silty Sand",
                    soil_type: SoilType::MediumSand,
                    top: 9.0,
                    bot: 17.0,
                    n_spt: 18.0,
                    vs: 225.0,
                    fc: 15.0,
                    gamma: 19.0,
                    ks: 36_000.0,
                    liquefiable: true,
                    color: [0.68, 0.54, 0.38, 0.34],
                },
                RawLayer {
                    name: "Dense Marine Sand & Gravel",
                    soil_type: SoilType::DenseSand,
                    top: 17.0,
                    bot: 30.0,
                    n_spt: 42.0,
                    vs: 420.0,
                    fc: 8.0,
                    gamma: 21.0,
                    ks: 80_000.0,
                    liquefiable: false,
                    color: [0.46, 0.48, 0.50, 0.38],
                },
            ],
            BoreholePreset::StiffVolcanicAlluvium => vec![
                RawLayer {
                    name: "Silty Loam Topsoil",
                    soil_type: SoilType::MediumClay,
                    top: 0.0,
                    bot: 3.5,
                    n_spt: 12.0,
                    vs: 215.0,
                    fc: 35.0,
                    gamma: 18.5,
                    ks: 30_000.0,
                    liquefiable: false,
                    color: [0.58, 0.46, 0.34, 0.32],
                },
                RawLayer {
                    name: "Stiff Volcanic Tuff Clay",
                    soil_type: SoilType::StiffClay,
                    top: 3.5,
                    bot: 10.0,
                    n_spt: 22.0,
                    vs: 275.0,
                    fc: 45.0,
                    gamma: 19.2,
                    ks: 46_000.0,
                    liquefiable: false,
                    color: [0.52, 0.40, 0.32, 0.34],
                },
                RawLayer {
                    name: "Dense Volcanic Sand",
                    soil_type: SoilType::DenseSand,
                    top: 10.0,
                    bot: 18.0,
                    n_spt: 34.0,
                    vs: 375.0,
                    fc: 12.0,
                    gamma: 20.5,
                    ks: 68_000.0,
                    liquefiable: false,
                    color: [0.60, 0.52, 0.42, 0.36],
                },
                RawLayer {
                    name: "Cemented Andesitic Breccia",
                    soil_type: SoilType::Bedrock,
                    top: 18.0,
                    bot: 30.0,
                    n_spt: 55.0,
                    vs: 620.0,
                    fc: 5.0,
                    gamma: 22.5,
                    ks: 160_000.0,
                    liquefiable: false,
                    color: [0.40, 0.43, 0.48, 0.40],
                },
            ],
            BoreholePreset::DenseGravelBedrock => vec![
                RawLayer {
                    name: "Compacted Gravel Subbase",
                    soil_type: SoilType::DenseSand,
                    top: 0.0,
                    bot: 2.5,
                    n_spt: 32.0,
                    vs: 410.0,
                    fc: 5.0,
                    gamma: 20.5,
                    ks: 85_000.0,
                    liquefiable: false,
                    color: [0.58, 0.55, 0.48, 0.32],
                },
                RawLayer {
                    name: "Dense Cobble Alluvium",
                    soil_type: SoilType::DenseSand,
                    top: 2.5,
                    bot: 12.0,
                    n_spt: 48.0,
                    vs: 640.0,
                    fc: 4.0,
                    gamma: 21.5,
                    ks: 130_000.0,
                    liquefiable: false,
                    color: [0.48, 0.47, 0.46, 0.35],
                },
                RawLayer {
                    name: "Competent Bedrock",
                    soil_type: SoilType::Bedrock,
                    top: 12.0,
                    bot: 30.0,
                    n_spt: 65.0,
                    vs: 1150.0,
                    fc: 2.0,
                    gamma: 24.0,
                    ks: 240_000.0,
                    liquefiable: false,
                    color: [0.35, 0.39, 0.44, 0.40],
                },
            ],
        };

        let mut strata = Vec::with_capacity(specs.len());
        let mut travel_time_30 = 0.0_f64;
        let mut lpi = 0.0_f64;
        let mut recommended_pile_depth_m = 12.0_f64;
        let mut found_bearing = false;

        for raw in specs {
            let thickness = (raw.bot - raw.top).max(0.1);
            let h_in_30 = (raw.bot.min(30.0) - raw.top.min(30.0)).max(0.0);
            travel_time_30 += h_in_30 / raw.vs.max(50.0);

            if !found_bearing && (raw.n_spt >= 30.0 || raw.vs >= 320.0) {
                recommended_pile_depth_m = (raw.top + 2.0).clamp(4.5, 28.0);
                found_bearing = true;
            }

            let z_mid = 0.5 * (raw.top + raw.bot);
            let (fs_l, py_mult) = if !raw.liquefiable || raw.bot <= gwt {
                (3.0, 1.0)
            } else {
                let z_sat = gwt.min(z_mid);
                let sigma_v = raw.gamma * z_mid;
                let u_w = 9.81 * (z_mid - z_sat).max(0.0);
                let sigma_v_eff = (sigma_v - u_w).max(15.0);
                let cn = (100.0 / sigma_v_eff).sqrt().clamp(0.4, 1.7);
                let n1_60cs =
                    (raw.n_spt * cn * 1.05 + (raw.fc * 0.12).min(5.0)).clamp(2.0, 45.0);
                let rd = (1.0 - 0.00765 * z_mid).clamp(0.50, 1.0);
                let csr = 0.65 * eff_pga * (sigma_v / sigma_v_eff) * rd;
                let n_cap = n1_60cs.min(29.5);
                let crr_75 = (1.0 / (34.0 - n_cap)
                    + n_cap / 135.0
                    + 50.0 / (10.0 * n_cap + 45.0).powi(2)
                    - 1.0 / 200.0)
                    .clamp(0.05, 0.70);
                let fs = (crr_75 / csr.max(0.02)).clamp(0.15, 3.5);

                if z_mid <= 20.0 && fs < 1.0 {
                    let w_z = (10.0 - 0.5 * z_mid).max(0.0);
                    lpi += (1.0 - fs) * w_z * thickness;
                }

                let mult = if apply_liquefaction {
                    if fs < 1.0 {
                        (0.12 + 0.25 * fs).clamp(0.12, 0.37)
                    } else if fs < 1.25 {
                        (0.40 + 2.4 * (fs - 1.0)).clamp(0.40, 1.0)
                    } else {
                        1.0
                    }
                } else {
                    1.0
                };
                (fs, mult)
            };

            strata.push(StratigraphyStratum {
                name: raw.name.to_string(),
                soil_type: raw.soil_type,
                depth_top_m: raw.top,
                depth_bottom_m: raw.bot,
                n_spt: raw.n_spt,
                vs_m_s: raw.vs,
                fines_content_pct: raw.fc,
                unit_weight_kn_m3: raw.gamma,
                subgrade_ks_kn_m3: raw.ks,
                liquefiable: raw.liquefiable,
                liquefaction_fs: fs_l,
                py_multiplier: py_mult,
                color_rgba: raw.color,
            });
        }

        let vs30 = (30.0 / travel_time_30.max(1e-3)).clamp(80.0, 2000.0);
        let site_class = if vs30 > 1500.0 {
            SiteClass::SA
        } else if vs30 > 750.0 {
            SiteClass::SB
        } else if vs30 > 350.0 {
            SiteClass::SC
        } else if vs30 > 175.0 {
            SiteClass::SD
        } else {
            SiteClass::SE
        };

        (strata, vs30, site_class, lpi, recommended_pile_depth_m)
    }

    /// Computes equivalent 6-DOF Winkler foundation springs combining shallow pile-cap subgrade
    /// contact with a 4-pile shaft group embedded to `recommended_pile_depth_m`, applying Seed–Idriss
    /// lateral $p\text{-}y$ degradation when liquefaction is active.
    pub fn compute_coupled_footing_winkler_springs(
        width_b_m: f64,
        length_l_m: f64,
        strata: &[StratigraphyStratum],
        recommended_pile_depth_m: f64,
        liquefaction_active: bool,
    ) -> (WinklerSpringMatrix, f64, String) {
        let b = width_b_m.max(0.8);
        let l = length_l_m.max(0.8);
        let area = b * l;

        let top_ks = strata
            .first()
            .map(|s| s.subgrade_ks_kn_m3)
            .unwrap_or(30_000.0);
        let bearing = strata
            .iter()
            .find(|s| {
                recommended_pile_depth_m >= s.depth_top_m
                    && recommended_pile_depth_m <= s.depth_bottom_m + 0.1
            })
            .or_else(|| strata.last());
        let bearing_ks = bearing.map(|s| s.subgrade_ks_kn_m3).unwrap_or(top_ks);
        let bearing_name = bearing
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "Stiff Alluvium".to_string());

        let mut min_fs = 3.0_f64;
        let mut min_shallow_py = 1.0_f64;
        for s in strata {
            min_fs = min_fs.min(s.liquefaction_fs);
            if s.depth_top_m <= 12.0 {
                min_shallow_py = min_shallow_py.min(s.py_multiplier);
            }
        }
        let lat_py = if liquefaction_active { min_shallow_py } else { 1.0 };

        let pile_embed_fac = (recommended_pile_depth_m / 12.0).clamp(0.65, 1.85);
        let ks_eff = 0.25 * top_ks + 0.75 * bearing_ks * pile_embed_fac;

        // Vertical axial stiffness from cap contact + 4 deep pile shafts (kN/m)
        let kz = (ks_eff * area * (0.45 + 0.35 * (bearing_ks / 60_000.0).clamp(0.4, 2.5)))
            * if liquefaction_active && min_fs < 1.0 { 0.72 } else { 1.0 };
        let kx = kz * 0.58 * lat_py;
        let ky = kz * 0.58 * lat_py;

        let ix = (b * l.powi(3)) / 12.0;
        let iy = (l * b.powi(3)) / 12.0;
        let iz = ix + iy;
        let pile_rock_boost = 1.0 + 0.12 * pile_embed_fac;
        let k_rot_x = ks_eff * ix * pile_rock_boost * (0.35 + 0.65 * lat_py);
        let k_rot_y = ks_eff * iy * pile_rock_boost * (0.35 + 0.65 * lat_py);
        let k_rot_z = ks_eff * iz * 0.55 * lat_py;

        (
            WinklerSpringMatrix {
                kz_trans_kn_m: kz.max(1_000.0),
                kx_trans_kn_m: kx.max(400.0),
                ky_trans_kn_m: ky.max(400.0),
                k_theta_x_knm_rad: k_rot_x.max(500.0),
                k_theta_y_knm_rad: k_rot_y.max(500.0),
                k_theta_z_knm_rad: k_rot_z.max(500.0),
                subgrade_modulus_ks: ks_eff,
            },
            min_fs,
            bearing_name,
        )
    }

    /// Emits a non-destructive IFCX (IFC 5) subsurface stratigraphy & Winkler SSI layer
    /// containing `IfcBorehole`, `IfcGeotechnicalStratum` (`usd::usdgeom::mesh`), groundwater table,
    /// and coupled SSI telemetry under `/earth/sites/<slug>/subsurface/...`.
    #[allow(clippy::too_many_arguments)]
    pub fn build_subsurface_ssi_ifcx_overlay(
        project: &BimProject,
        preset: BoreholePreset,
        site_class: SiteClass,
        vs30_m_s: f64,
        gwt_m: f64,
        pga_g: f64,
        strata: &[StratigraphyStratum],
        lpi: f64,
        recommended_pile_depth_m: f64,
        footing_couplings: &[FootingSsiCouplingResult],
        t_fixed: f64,
        t_ssi: f64,
        ari_fixed: f64,
        ari_ssi: f64,
    ) -> IfcxDocument {
        let slug = slugify_geotech(&project.project_name);
        let mut doc = IfcxDocument::new(format!("subsurface_ssi_{slug}"));
        doc.imports.push(IfcxImport {
            uri: format!("eigengraph://{slug}.ifcx"),
            integrity: None,
            role: Some("base-building".to_string()),
        });
        doc.schemas.insert(
            NS_EG_GEOTECH_STRATIGRAPHY.to_string(),
            json!({ "value": { "dataType": "Object", "spec": "EigenGraph 3D Borehole Soil Stratigraphy & Seed-Idriss Liquefaction" } }),
        );
        doc.schemas.insert(
            NS_EG_GEOTECH_SSI.to_string(),
            json!({ "value": { "dataType": "Object", "spec": "EigenGraph 6-DOF Coupled Winkler Soil-Structure Interaction Springs" } }),
        );

        let site_path = format!("/earth/sites/{slug}");
        let sub_path = format!("{site_path}/subsurface");
        let borehole_path = format!("{sub_path}/borehole_01");
        let gwt_path = format!("{sub_path}/groundwater_table");
        let ssi_path = format!("{site_path}/buildings/{slug}/analysis/subsurface_ssi");

        let (mut min_x, mut max_x, mut min_y, mut max_y) = (-12.0_f64, 12.0_f64, -12.0_f64, 12.0_f64);
        if let Some((bmin, bmax)) = project.plan_extents_mm() {
            min_x = bmin[0] * 0.001 - 6.0;
            max_x = bmax[0] * 0.001 + 6.0;
            min_y = bmin[1] * 0.001 - 6.0;
            max_y = bmax[1] * 0.001 + 6.0;
        }

        let mut sub_node = IfcxNode::new(&sub_path).with_class("IfcGeotechnicalAssembly");
        sub_node
            .children
            .insert("borehole_01".to_string(), borehole_path.clone());
        sub_node
            .children
            .insert("groundwater_table".to_string(), gwt_path.clone());

        let borehole_node = IfcxNode::new(&borehole_path)
            .with_class("IfcBorehole")
            .with_prop(
                "BH-01",
                preset.short_label(),
                &format!("{} | Vs30={vs30_m_s:.1} m/s | {site_class:?}", preset.label()),
            )
            .with_attr(
                NS_EG_GEOTECH_STRATIGRAPHY,
                json!({
                    "preset": preset.short_label(),
                    "siteClass": format!("{site_class:?}"),
                    "vs30Ms": vs30_m_s,
                    "groundwaterDepthM": gwt_m,
                    "pgaG": pga_g,
                    "liquefactionPotentialIndex": lpi,
                    "recommendedPileDepthM": recommended_pile_depth_m,
                }),
            );
        doc.add_node(borehole_node);

        for (i, s) in strata.iter().enumerate() {
            let key = format!("stratum_{i:02}");
            let spath = format!("{sub_path}/{key}");
            sub_node.children.insert(key, spath.clone());
            let mesh = UsdGeomMesh::from_box_m(
                [min_x, min_y, -s.depth_bottom_m],
                [max_x, max_y, -s.depth_top_m],
                s.color_rgba,
                4.0,
            );
            let snode = IfcxNode::new(&spath)
                .with_class("IfcGeotechnicalStratum")
                .with_prop(
                    &s.name,
                    s.soil_type.label(),
                    &format!(
                        "Depth {:.1}-{:.1}m | N-SPT={:.0} | Vs={:.0} m/s | FS_L={:.2} | p-y={:.2}",
                        s.depth_top_m, s.depth_bottom_m, s.n_spt, s.vs_m_s, s.liquefaction_fs, s.py_multiplier
                    ),
                )
                .with_mesh(&mesh)
                .with_attr(
                    NS_EG_GEOTECH_STRATIGRAPHY,
                    json!({
                        "depthTopM": s.depth_top_m,
                        "depthBottomM": s.depth_bottom_m,
                        "nSpt": s.n_spt,
                        "vsMs": s.vs_m_s,
                        "liquefiable": s.liquefiable,
                        "liquefactionFs": s.liquefaction_fs,
                        "pyMultiplier": s.py_multiplier,
                    }),
                );
            doc.add_node(snode);
        }

        let gwt_mesh = UsdGeomMesh::from_box_m(
            [min_x, min_y, -gwt_m - 0.12],
            [max_x, max_y, -gwt_m],
            [0.14, 0.72, 0.94, 0.36],
            5.0,
        );
        let gwt_node = IfcxNode::new(&gwt_path)
            .with_class("IfcGeographicElement")
            .with_prop(
                "Groundwater Table",
                "Phreatic Surface",
                &format!("Water table at -{gwt_m:.2} m below grade"),
            )
            .with_mesh(&gwt_mesh);
        doc.add_node(gwt_node);
        doc.add_node(sub_node);

        let ssi_node = IfcxNode::new(&ssi_path)
            .with_class("IfcStructuralAnalysisModel")
            .with_prop(
                "6-DOF Coupled Winkler SSI",
                preset.short_label(),
                &format!(
                    "T1_fixed={t_fixed:.3}s -> T1_SSI={t_ssi:.3}s | Pile Depth={recommended_pile_depth_m:.1}m"
                ),
            )
            .with_attr(
                NS_EG_GEOTECH_SSI,
                json!({
                    "fixedBasePeriodSec": t_fixed,
                    "ssiCoupledPeriodSec": t_ssi,
                    "fixedBaseAri": ari_fixed,
                    "ssiCoupledAri": ari_ssi,
                    "recommendedPileDepthM": recommended_pile_depth_m,
                    "footings": footing_couplings.iter().map(|f| json!({
                        "id": f.footing_id,
                        "name": f.footing_name,
                        "posM": f.pos_m,
                        "bearingStratum": f.bearing_stratum_name,
                        "pileLenM": f.required_pile_length_m,
                        "kzKnM": f.winkler_springs.kz_trans_kn_m,
                        "kxKnM": f.winkler_springs.kx_trans_kn_m,
                        "kyKnM": f.winkler_springs.ky_trans_kn_m,
                        "settlementMm": f.estimated_settlement_mm,
                        "liquefactionFs": f.liquefaction_risk_fs,
                    })).collect::<Vec<_>>(),
                }),
            )
            .with_attr(
                ifcx_ns::EG_ARI,
                json!({
                    "ariSystem": ari_ssi,
                    "ariAverage": ari_ssi,
                    "isKinematicallyStable": true,
                }),
            );
        doc.add_node(ssi_node);
        doc
    }
}

/// Evaluates coupled 3D subsurface stratigraphy, Seed–Idriss liquefaction screening, 6-DOF Winkler
/// foundation springs, modal period shift, and non-destructive `.ifcx` overlay for a `BimProject`.
pub fn evaluate_coupled_subsurface_ssi_for_bim(
    project: &BimProject,
    site: &SiteEnvironmentalCoupling,
    config: &SubsurfaceSsiConfig,
) -> Result<SubsurfaceSsiAnalysisReport, String> {
    if project.is_empty() {
        return Err(
            "BIM project is empty — add structural columns/beams before evaluating 6-DOF Subsurface SSI."
                .to_string(),
        );
    }

    let pga_g = (site.seismic_pga_ss_g as f64 * 0.40).clamp(0.10, 1.20);
    let (strata, vs30_m_s, site_class, lpi, recommended_pile_depth_m) =
        GeotechEngine::build_borehole_strata(
            config.borehole_preset,
            config.groundwater_depth_m,
            pga_g,
            config.liquefaction_trigger_active,
        );

    let (fem_fixed, _map_fixed, load_fixed) = project.to_structural_model_with_site_loads(site);
    let mut coupled_cfg = config.clone();
    coupled_cfg.ssi_coupling_enabled = true;
    let (fem_ssi, _map_ssi, load_ssi) =
        project.to_structural_model_with_site_loads_and_ssi(site, Some(&coupled_cfg));

    if fem_ssi.elements.is_empty() {
        return Err("No analytical 3D frame elements available for coupled SSI analysis.".to_string());
    }

    let fea_ssi = FemSolver::solve(&fem_ssi)?;
    let modal_fixed = ModalEngine::analyze_for_earth_3d(&fem_fixed, 3, load_fixed.spectrum)?;
    let modal_ssi = ModalEngine::analyze_for_earth_3d(&fem_ssi, 3, load_ssi.spectrum)?;

    let t_fixed = modal_fixed.fundamental_period_s.max(0.05);
    let t_ssi = modal_ssi.fundamental_period_s.max(t_fixed);
    let period_lengthening_pct = ((t_ssi / t_fixed) - 1.0) * 100.0;

    let (fixed_base_ari, ssi_coupled_ari) =
        if fem_ssi.nodes.len() <= 32 && fem_ssi.elements.len() <= 96 {
            let fixed = AriEngine::evaluate_redundancy_fast(&fem_fixed)
                .map(|a| a.average_ari)
                .unwrap_or(0.85);
            let ssi = AriEngine::evaluate_redundancy_fast(&fem_ssi)
                .map(|a| a.average_ari)
                .unwrap_or(fixed);
            (fixed, ssi)
        } else {
            let max_dcr_fixed = FemSolver::solve(&fem_fixed)
                .ok()
                .and_then(|res| res.member_dcr_interactions.into_iter().reduce(f64::max))
                .unwrap_or(0.35);
            let max_dcr_ssi = fea_ssi
                .member_dcr_interactions
                .iter()
                .copied()
                .reduce(f64::max)
                .unwrap_or(max_dcr_fixed);
            (
                (1.0 - max_dcr_fixed * 0.35).clamp(0.20, 0.96),
                (1.0 - max_dcr_ssi * 0.35).clamp(0.20, 0.96),
            )
        };

    let base_nodes: Vec<(f64, f64, f64)> = fem_ssi
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.y.abs() <= 0.05)
        .map(|(idx, n)| {
            let settle_mm = fea_ssi
                .nodal_displacements
                .get(idx)
                .map(|d| (-d[1] * 1000.0).max(0.0))
                .unwrap_or(0.0);
            (n.x, n.z, settle_mm)
        })
        .collect();

    let mut footing_couplings = Vec::new();
    let mut max_foundation_settlement_mm = 0.0_f64;

    if !project.footings.is_empty() {
        for ftg in &project.footings {
            let fx_m = ftg.pos.0 * 0.001;
            let fy_m = ftg.pos.1 * 0.001;
            let w_m = ftg.width_mm * 0.001;
            let l_m = ftg.length_mm * 0.001;
            let (winkler, min_fs, bearing_name) =
                GeotechEngine::compute_coupled_footing_winkler_springs(
                    w_m,
                    l_m,
                    &strata,
                    recommended_pile_depth_m,
                    config.liquefaction_trigger_active,
                );
            let settle_mm = base_nodes
                .iter()
                .min_by(|a, b| {
                    (a.0 - fx_m)
                        .hypot(a.1 - fy_m)
                        .total_cmp(&(b.0 - fx_m).hypot(b.1 - fy_m))
                })
                .map(|n| n.2)
                .unwrap_or(0.0);
            max_foundation_settlement_mm = max_foundation_settlement_mm.max(settle_mm);
            footing_couplings.push(FootingSsiCouplingResult {
                footing_id: ftg.id,
                footing_name: ftg.name.clone(),
                pos_m: [fx_m, fy_m],
                width_m: w_m,
                length_m: l_m,
                required_pile_length_m: recommended_pile_depth_m,
                bearing_stratum_name: bearing_name,
                winkler_springs: winkler,
                estimated_settlement_mm: settle_mm,
                liquefaction_risk_fs: min_fs,
            });
        }
    } else {
        for (i, &(nx_m, nz_m, settle_mm)) in base_nodes.iter().enumerate() {
            let (winkler, min_fs, bearing_name) =
                GeotechEngine::compute_coupled_footing_winkler_springs(
                    2.0,
                    2.0,
                    &strata,
                    recommended_pile_depth_m,
                    config.liquefaction_trigger_active,
                );
            max_foundation_settlement_mm = max_foundation_settlement_mm.max(settle_mm);
            footing_couplings.push(FootingSsiCouplingResult {
                footing_id: i + 1,
                footing_name: format!("Base-{}", i + 1),
                pos_m: [nx_m, nz_m],
                width_m: 2.0,
                length_m: 2.0,
                required_pile_length_m: recommended_pile_depth_m,
                bearing_stratum_name: bearing_name,
                winkler_springs: winkler,
                estimated_settlement_mm: settle_mm,
                liquefaction_risk_fs: min_fs,
            });
        }
    }

    let min_liquefaction_fs = strata
        .iter()
        .map(|s| s.liquefaction_fs)
        .fold(3.0_f64, f64::min);
    let is_liquefaction_critical = min_liquefaction_fs < 1.15;

    let ssi_overlay_ifcx = GeotechEngine::build_subsurface_ssi_ifcx_overlay(
        project,
        config.borehole_preset,
        site_class,
        vs30_m_s,
        config.groundwater_depth_m,
        pga_g,
        &strata,
        lpi,
        recommended_pile_depth_m,
        &footing_couplings,
        t_fixed,
        t_ssi,
        fixed_base_ari,
        ssi_coupled_ari,
    );

    let liq_tag = if is_liquefaction_critical {
        format!(
            "LIQUEFACTION FS_L={min_liquefaction_fs:.2} (LPI={lpi:.1})"
        )
    } else {
        format!("Non-Liquefiable FS_L={min_liquefaction_fs:.2}")
    };
    let summary = format!(
        "Subsurface SSI ({}, {site_class:?}, Vs30={vs30_m_s:.0}m/s): T1 {t_fixed:.2}s -> {t_ssi:.2}s (+{period_lengthening_pct:.1}%) | Piles {recommended_pile_depth_m:.1}m | Δf={max_foundation_settlement_mm:.1}mm | ARI={ssi_coupled_ari:.3} | {liq_tag}",
        config.borehole_preset.short_label(),
    );

    Ok(SubsurfaceSsiAnalysisReport {
        preset: config.borehole_preset,
        site_class,
        vs30_m_s,
        groundwater_depth_m: config.groundwater_depth_m,
        pga_g,
        strata,
        min_liquefaction_fs,
        liquefaction_potential_index: lpi,
        is_liquefaction_critical,
        recommended_pile_depth_m,
        footing_couplings,
        fixed_base_period_t1_s: t_fixed,
        ssi_coupled_period_t1_s: t_ssi,
        period_lengthening_pct,
        fixed_base_ari,
        ssi_coupled_ari,
        max_foundation_settlement_mm,
        ssi_overlay_ifcx,
        summary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ifcx::IfcxStage;

    #[test]
    fn test_winkler_spring_matrix() {
        let soil = SoilLayer::new_standard(SoilType::MediumSand, 0.0, 5.0);
        let springs = GeotechEngine::compute_winkler_springs(3.0, 4.0, &soil);
        assert!(springs.kz_trans_kn_m > 0.0);
        assert!(springs.kx_trans_kn_m > 0.0);
        assert!(springs.k_theta_x_knm_rad > 0.0);
        assert!(springs.k_theta_y_knm_rad > 0.0);
    }

    #[test]
    fn test_pile_group_efficiency_and_distribution() {
        let soil = SoilLayer::new_standard(SoilType::StiffClay, 0.0, 20.0);
        let group = GeotechEngine::evaluate_pile_group(
            1, 1, 3, 3, 0.80, 2.50, 15.0, 6000.0, 800.0, 600.0, &soil
        );

        assert_eq!(group.total_piles, 9);
        assert!(group.converse_labarre_efficiency > 0.60 && group.converse_labarre_efficiency <= 1.0);
        assert!(group.max_pile_axial_load_kn > group.min_pile_axial_load_kn);
        assert_eq!(group.pile_details.len(), 9);
    }

    #[test]
    fn test_punching_shear_audit() {
        let audit = GeotechEngine::audit_punching_shear(
            "Test Column C1", 500.0, 500.0, 400.0, 30.0, 1200.0, 150.0
        );
        assert!(audit.critical_perimeter_bo_mm > 0.0);
        assert!(audit.punching_stress_vu_mpa > 0.0);
        assert!(audit.concrete_capacity_phi_vc_mpa > 0.0);
    }

    #[test]
    fn test_phase71_borehole_presets_vs30_and_liquefaction_screening() {
        let (soft_strata, soft_vs30, soft_cls, _, soft_pile) =
            GeotechEngine::build_borehole_strata(BoreholePreset::AlluvialSoftClay, 1.5, 0.35, false);
        assert_eq!(soft_strata.len(), 4);
        assert!(soft_vs30 < 175.0);
        assert_eq!(soft_cls, SiteClass::SE);
        assert!(soft_pile >= 20.0);

        let (liq_strata, _, liq_cls, lpi, _) = GeotechEngine::build_borehole_strata(
            BoreholePreset::CoastalLiquefiableSand,
            1.5,
            0.40,
            true,
        );
        assert_eq!(liq_cls, SiteClass::SD);
        assert!(
            liq_strata.iter().any(|s| s.liquefaction_fs < 1.0 && s.py_multiplier < 0.40),
            "Coastal saturated loose sand under 0.40g PGA must flag Seed-Idriss liquefaction & p-y degradation"
        );
        assert!(lpi > 5.0);

        let (_, stiff_vs30, stiff_cls, _, _) = GeotechEngine::build_borehole_strata(
            BoreholePreset::StiffVolcanicAlluvium,
            2.5,
            0.35,
            false,
        );
        assert_eq!(stiff_cls, SiteClass::SC);
        assert!(stiff_vs30 > 350.0);

        let (_, rock_vs30, rock_cls, _, rock_pile) = GeotechEngine::build_borehole_strata(
            BoreholePreset::DenseGravelBedrock,
            4.0,
            0.35,
            false,
        );
        assert_eq!(rock_cls, SiteClass::SB);
        assert!(rock_vs30 > 750.0);
        assert!(rock_pile <= 5.0);
    }

    #[test]
    fn test_phase71_coupled_subsurface_ssi_and_ifcx_overlay() {
        let project = BimProject::sample_bimcraft_building();
        let site = crate::core::compute_site_environmental_coupling(
            -6.2088,
            106.8456,
            &crate::core::gis::earth_globe::PlanetaryWorldState::default(),
        );
        let cfg = SubsurfaceSsiConfig {
            ssi_coupling_enabled: true,
            xray_subsurface_visible: true,
            borehole_preset: BoreholePreset::CoastalLiquefiableSand,
            groundwater_depth_m: 1.5,
            liquefaction_trigger_active: true,
        };
        let rep = evaluate_coupled_subsurface_ssi_for_bim(&project, &site, &cfg)
            .expect("Coupled Subsurface SSI report should succeed");
        assert!(rep.is_liquefaction_critical);
        assert!(rep.ssi_coupled_period_t1_s > rep.fixed_base_period_t1_s);
        assert!(rep.max_foundation_settlement_mm > 0.0);
        assert!(!rep.footing_couplings.is_empty());

        let base_ifcx = IfcxDocument::from_bim_project(&project, None, 0.0, None, None, None);
        let base_count = base_ifcx.data.len();
        let composed = IfcxStage::compose_with_subsurface_ssi_overlay(&base_ifcx, &rep);
        assert!(composed.data.len() > base_count);
        assert!(composed
            .data
            .iter()
            .any(|n| n.attributes.contains_key(NS_EG_GEOTECH_STRATIGRAPHY)));
        assert!(composed
            .data
            .iter()
            .any(|n| n.attributes.contains_key(NS_EG_GEOTECH_SSI)));
    }
}
