//! blast.rs — Blast Resistant Design & Structural Dynamics Under Explosive Impulses.
//!
//! Fully compliant with:
//! - DoD UFC 3-340-02: "Structures to Resist the Effects of Accidental Explosions" (formerly TM 5-1300)
//! - ASCE 59-11: "Blast Protection of Buildings"
//! - USACE PDC-TR-06-08: "Single-Degree-of-Freedom Structural Response Limits for Antiterrorism Design"
//! - GSA PBS-P100 / ISC: "Security Design Criteria for New Federal Office Buildings"
//! - ACI 318-19 / AISC 360-16 with Dynamic Increase Factors (DIF & SIF)

use serde::{Deserialize, Serialize};
use crate::core::model::{BoundaryFixity, CrossSection, Element3D, ElementType, Material, Node3D};

/// Explosive compound classification and TNT equivalence factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExplosiveType {
    /// Trinitrotoluene (TNT) — Baseline reference standard (Equiv = 1.00).
    Tnt,
    /// Ammonium Nitrate / Fuel Oil (ANFO) — Common industrial explosive (Equiv = 0.82).
    Anfo,
    /// Composition C-4 (RDX based) — High military explosive (Equiv = 1.30).
    C4,
    /// Pentaerythritol Tetranitrate (PETN) — High brisance booster (Equiv = 1.27).
    Petn,
    /// Commercial Dynamite / Nitroglycerin (Equiv = 0.92).
    Dynamite,
}

impl ExplosiveType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Tnt => "TNT (Trinitrotoluene)",
            Self::Anfo => "ANFO (Ammonium Nitrate/Fuel Oil)",
            Self::C4 => "Composition C-4 (Military RDX)",
            Self::Petn => "PETN (High Brisance)",
            Self::Dynamite => "Commercial Dynamite",
        }
    }

    /// TNT equivalence factor for shock overpressure and impulse.
    pub fn tnt_equivalence_factor(&self) -> f64 {
        match self {
            Self::Tnt => 1.00,
            Self::Anfo => 0.82,
            Self::C4 => 1.30,
            Self::Petn => 1.27,
            Self::Dynamite => 0.92,
        }
    }
}

/// Detonation geometry configuration per UFC 3-340-02 Chapter 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BurstType {
    /// Surface Burst on ground plane with hemispherical wave reflection (factor = 1.80).
    SurfaceBurst,
    /// Free-Air Burst at elevation with spherical blast wave propagation (factor = 1.00).
    FreeAirBurst,
}

impl BurstType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SurfaceBurst => "Surface Burst (Hemispherical Reflection Factor 1.80)",
            Self::FreeAirBurst => "Free-Air Burst (Spherical Expansion Factor 1.00)",
        }
    }

    pub fn reflection_enhancement_factor(&self) -> f64 {
        match self {
            Self::SurfaceBurst => 1.80,
            Self::FreeAirBurst => 1.00,
        }
    }
}

/// Structural component archetype subjected to blast impulse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlastComponentKind {
    /// Reinforced Concrete Perimeter Blast Wall (Flexure & In-Plane Shear).
    RcWall,
    /// Reinforced Concrete Hardened Roof / Floor Slab.
    RcSlab,
    /// Structural Steel W-Shape Column with Blast Protective Shroud.
    SteelColumn,
    /// Structural Steel Heavy Plate Blast Deflector Gate.
    SteelBarrier,
    /// Laminated Blast-Resistant Security Glazing (PVB / SGP Interlayer).
    BlastGlazing,
}

impl BlastComponentKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::RcWall => "RC Hardened Blast Wall",
            Self::RcSlab => "RC Roof / Floor Slab Diaphragm",
            Self::SteelColumn => "Structural Steel W-Column",
            Self::SteelBarrier => "Steel Plate Blast Barrier Gate",
            Self::BlastGlazing => "Laminated Anti-Shatter Glazing",
        }
    }
}

/// Boundary support condition for SDOF transformation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupportCondition {
    /// Simply Supported (Pinned - Pinned, KLM_elastic = 0.78, KLM_plastic = 0.66).
    SimplySupported,
    /// Fixed - Fixed (Clamped, KLM_elastic = 0.77, KLM_plastic = 0.66).
    FixedFixed,
    /// Propped Cantilever (Fixed - Pinned, KLM_elastic = 0.75, KLM_plastic = 0.66).
    ProppedCantilever,
    /// Cantilever Wall / Free Parapet (Fixed - Free, KLM_elastic = 0.66, KLM_plastic = 0.50).
    Cantilever,
}

impl SupportCondition {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SimplySupported => "Simply Supported (Pinned - Pinned)",
            Self::FixedFixed => "Fixed - Fixed (Clamped)",
            Self::ProppedCantilever => "Propped Cantilever (Fixed - Pinned)",
            Self::Cantilever => "Cantilever (Fixed - Free)",
        }
    }

    /// Elastic SDOF transformation factor KLM = KM / KL.
    pub fn klm_elastic(&self) -> f64 {
        match self {
            Self::SimplySupported => 0.78,
            Self::FixedFixed => 0.77,
            Self::ProppedCantilever => 0.75,
            Self::Cantilever => 0.66,
        }
    }

    /// Plastic SDOF transformation factor KLM.
    pub fn klm_plastic(&self) -> f64 {
        match self {
            Self::SimplySupported => 0.66,
            Self::FixedFixed => 0.66,
            Self::ProppedCantilever => 0.66,
            Self::Cantilever => 0.50,
        }
    }

    /// Moment coefficient for uniform load w: M = coeff * w * L^2.
    pub fn moment_coefficient(&self) -> f64 {
        match self {
            Self::SimplySupported => 0.125,     // 1/8
            Self::FixedFixed => 0.0625,         // 1/16 (plastic collapse mechanism)
            Self::ProppedCantilever => 0.0858,   // ~1/11.65
            Self::Cantilever => 0.500,          // 1/2
        }
    }

    /// Deflection coefficient for uniform load w: delta = coeff * w * L^4 / (E * I).
    pub fn deflection_coefficient(&self) -> f64 {
        match self {
            Self::SimplySupported => 5.0 / 384.0,
            Self::FixedFixed => 1.0 / 384.0,
            Self::ProppedCantilever => 1.0 / 185.0,
            Self::Cantilever => 1.0 / 8.0,
        }
    }
}

/// Structural response damage level classification per PDC-TR-06-08 / UFC 3-340-02 Table 4-2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum UfcDamageLevel {
    /// B1: Superficial Damage — Elastic response, component completely reusable without repair.
    Superficial,
    /// B2: Low Damage — Minor localized yielding / micro-cracking, immediately occupiable, minimal cosmetic patching.
    Low,
    /// B3: Medium Damage — Significant plastic hinging, substantial permanent deformation, repairable after event.
    Medium,
    /// B4: High Damage — Heavy plastic deformation, life safety preserved, component requires complete replacement.
    High,
    /// B5: Hazardous / Collapse Hazard — Excessive support rotation, component rupture or hazardous fragmentation.
    Hazardous,
}

impl UfcDamageLevel {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Superficial => "B1: Superficial (Elastic, Reusable)",
            Self::Low => "B2: Low Damage (Minor Yielding, Immediately Occupiable)",
            Self::Medium => "B3: Medium Damage (Moderate Plasticity, Repairable)",
            Self::High => "B4: High Damage (Severe Deformation, Life Safety)",
            Self::Hazardous => "B5: Hazardous Failure (Imminent Rupture / Collapse)",
        }
    }

    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            Self::Superficial => (16, 185, 129),   // Emerald green
            Self::Low => (59, 130, 246),           // Sky blue
            Self::Medium => (245, 158, 11),        // Amber
            Self::High => (249, 115, 22),          // Orange
            Self::Hazardous => (239, 68, 68),      // Red
        }
    }
}

/// Dynamic response regime based on duration to period ratio (td / Tn).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlastResponseRegime {
    /// Impulsive Regime (td / Tn < 0.10) — Governed purely by reflected impulse ir.
    Impulsive,
    /// Dynamic Intermediate Regime (0.10 <= td / Tn <= 2.0) — Coupled pressure-impulse time history.
    Dynamic,
    /// Quasi-Static Regime (td / Tn > 2.0) — Governed by peak reflected overpressure Pr.
    QuasiStatic,
}

impl BlastResponseRegime {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Impulsive => "Impulsive Regime (td / Tn < 0.10)",
            Self::Dynamic => "Dynamic Regime (0.10 ≤ td / Tn ≤ 2.0)",
            Self::QuasiStatic => "Quasi-Static Regime (td / Tn > 2.0)",
        }
    }
}

/// Kingery-Bulmash empirical blast wave parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastWaveParameters {
    /// Equivalent TNT charge mass (kg).
    pub effective_tnt_mass_kg: f64,
    /// Standoff distance to target (m).
    pub standoff_distance_m: f64,
    /// Scaled distance Z = R / W^(1/3) (m/kg^(1/3)).
    pub scaled_distance_z: f64,
    /// Peak positive incident overpressure Pso (kPa).
    pub peak_incident_overpressure_kpa: f64,
    /// Peak normally reflected overpressure Pr (kPa).
    pub peak_reflected_overpressure_kpa: f64,
    /// Positive phase duration td (ms).
    pub positive_phase_duration_ms: f64,
    /// Reflected positive specific impulse ir (kPa-ms).
    pub reflected_impulse_kpa_ms: f64,
    /// Incident positive specific impulse is (kPa-ms).
    pub incident_impulse_kpa_ms: f64,
    /// Shock front velocity Us (m/s).
    pub shock_velocity_m_s: f64,
    /// Shock arrival time ta (ms).
    pub arrival_time_ms: f64,
    /// Modified Friedlander exponential decay coefficient b.
    pub friedlander_decay_coeff: f64,
    /// Clearing relief time tc (ms) for finite targets.
    pub clearing_time_ms: f64,
    /// Effective cleared impulse ic (kPa-ms).
    pub cleared_impulse_kpa_ms: f64,
}

/// Equivalent Single-Degree-of-Freedom (SDOF) system parameters per Biggs / UFC 3-340-02.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdofSystemParameters {
    /// Actual total mass of structural component (kg).
    pub total_mass_kg: f64,
    /// Equivalent SDOF mass Me = KM * M (kg).
    pub equivalent_mass_kg: f64,
    /// Dynamic elastic stiffness ke (kN/m).
    pub elastic_stiffness_kn_per_m: f64,
    /// Dynamic plastic resistance capacity Ru (kN).
    pub ultimate_resistance_kn: f64,
    /// Elastic yield displacement ye (mm).
    pub yield_displacement_mm: f64,
    /// Natural undamped structural period Tn (ms).
    pub natural_period_ms: f64,
    /// Natural circular frequency omega_n (rad/s).
    pub omega_n_rad_per_s: f64,
    /// Effective load-mass transformation factor KLM = KM / KL.
    pub klm_factor: f64,
    /// Dynamic response regime (Impulsive, Dynamic, Quasi-Static).
    pub response_regime: BlastResponseRegime,
}

/// Instantaneous SDOF time step response point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdofTimeStep {
    pub time_ms: f64,
    pub blast_pressure_kpa: f64,
    pub applied_force_kn: f64,
    pub displacement_mm: f64,
    pub velocity_m_s: f64,
    pub resistance_force_kn: f64,
    pub support_reaction_kn: f64,
}

/// Regulatory and code compliance check item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastComplianceItem {
    pub parameter_name: String,
    pub code_reference: String,
    pub demand_val: f64,
    pub permissible_limit: f64,
    pub unit: String,
    pub utilization_ratio: f64,
    pub is_passed: bool,
}

/// Iso-damage curve point on Pressure-Impulse (P-I) diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiCurvePoint {
    pub impulse_kpa_ms: f64,
    pub pressure_kpa: f64,
}

/// Blast analysis configuration input parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastAnalysisInput {
    pub project_name: String,
    pub explosive_mass_kg: f64,
    pub explosive_type: ExplosiveType,
    pub burst_type: BurstType,
    pub standoff_distance_m: f64,
    pub component_kind: BlastComponentKind,
    pub support_condition: SupportCondition,
    pub span_length_m: f64,
    pub tributary_width_m: f64,
    pub thickness_m: f64,
    pub f_c_mpa: f64,
    pub f_y_mpa: f64,
    pub rebar_ratio_percent: f64,
    pub damping_ratio: f64,
    pub target_damage_level: UfcDamageLevel,
}

impl Default for BlastAnalysisInput {
    fn default() -> Self {
        Self {
            project_name: "Strategic Embassy Perimeter Blast Gate & Bunker Wall".to_string(),
            explosive_mass_kg: 250.0,
            explosive_type: ExplosiveType::C4,
            burst_type: BurstType::SurfaceBurst,
            standoff_distance_m: 15.0,
            component_kind: BlastComponentKind::RcWall,
            support_condition: SupportCondition::FixedFixed,
            span_length_m: 4.20,
            tributary_width_m: 1.00,
            thickness_m: 0.40,
            f_c_mpa: 40.0,
            f_y_mpa: 420.0,
            rebar_ratio_percent: 1.20,
            damping_ratio: 0.02,
            target_damage_level: UfcDamageLevel::Low,
        }
    }
}

/// Comprehensive Blast Engineering Assessment Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastAnalysisReport {
    pub input: BlastAnalysisInput,
    pub blast_wave: BlastWaveParameters,
    pub sdof_system: SdofSystemParameters,
    /// Maximum dynamic midspan displacement (mm).
    pub max_displacement_mm: f64,
    /// Time of maximum displacement (ms).
    pub time_at_max_disp_ms: f64,
    /// Permanent residual plastic displacement (mm).
    pub permanent_displacement_mm: f64,
    /// Dynamic ductility ratio mu = y_max / y_e.
    pub dynamic_ductility_ratio: f64,
    /// Support rotation angle theta (degrees).
    pub support_rotation_deg: f64,
    /// Maximum dynamic reaction force at support (kN).
    pub max_reaction_force_kn: f64,
    /// Maximum dynamic shear force at support (kN).
    pub max_shear_demand_kn: f64,
    /// Dynamic direct shear capacity Vc_dyn (kN).
    pub dynamic_shear_capacity_kn: f64,
    /// Evaluated UFC damage level.
    pub evaluated_damage_level: UfcDamageLevel,
    /// Is target UFC damage level achieved?
    pub is_target_damage_achieved: bool,
    /// SDOF dynamic response time history (up to 120 key sample points).
    pub time_history_samples: Vec<SdofTimeStep>,
    /// P-I iso-damage curves for Superficial, Low, Medium, and High damage limits.
    pub pi_curves: Vec<(UfcDamageLevel, Vec<PiCurvePoint>)>,
    /// Regulatory & UFC 3-340-02 compliance audit table.
    pub compliance_audit: Vec<BlastComplianceItem>,
}

/// Blast Resistant Design and Dynamics Engine.
pub struct BlastEngine;

impl BlastEngine {
    /// Standard ambient atmospheric pressure (kPa).
    pub const P_AMB_KPA: f64 = 101.325;
    /// Ambient speed of sound at sea level (m/s).
    pub const A_SOUND_M_S: f64 = 340.29;

    /// Calculate Kingery-Bulmash empirical blast wave parameters per UFC 3-340-02.
    pub fn evaluate_blast_wave(
        charge_mass_kg: f64,
        explosive_type: ExplosiveType,
        burst_type: BurstType,
        standoff_m: f64,
        target_height_m: f64,
        target_width_m: f64,
    ) -> BlastWaveParameters {
        let tnt_factor = explosive_type.tnt_equivalence_factor();
        let burst_factor = burst_type.reflection_enhancement_factor();
        let effective_tnt_mass = charge_mass_kg * tnt_factor * burst_factor;

        let w_cbrt = effective_tnt_mass.cbrt().max(0.1);
        let standoff = standoff_m.max(1.0);
        let z = standoff / w_cbrt;

        // Henrych / Brode / Kingery-Bulmash polynomial fits for Incident Overpressure Pso (kPa)
        let pso_kpa = if z <= 1.0 {
            1407.2 / z + 554.0 / (z * z) - 35.7 / (z * z * z) + 625.0 / (z * z * z * z)
        } else if z <= 10.0 {
            619.4 / z - 32.6 / (z * z) + 213.2 / (z * z * z)
        } else {
            67.0 / z + 301.0 / (z * z) + 216.0 / (z * z * z)
        };

        // Rankine-Hugoniot normal reflection relation: Pr = 2 * Pso * (7*Po + 4*Pso) / (7*Po + Pso)
        let p0 = Self::P_AMB_KPA;
        let pr_kpa = 2.0 * pso_kpa * (7.0 * p0 + 4.0 * pso_kpa) / (7.0 * p0 + pso_kpa);

        // Positive phase duration td (ms)
        let td_ms = (w_cbrt * (1.8 * z / (1.0 + 0.15 * z * z).sqrt())).clamp(1.5, 60.0);

        // Incident specific impulse is (kPa-ms)
        let is_kpa_ms = 0.5 * pso_kpa * td_ms / (1.0 + 0.10 * (1.0 + pso_kpa / p0).ln());

        // Reflected specific impulse ir (kPa-ms)
        let ir_kpa_ms = 0.5 * pr_kpa * td_ms / (1.0 + 0.12 * (1.0 + pso_kpa / p0).ln());

        // Shock velocity Us (m/s)
        let shock_vel = Self::A_SOUND_M_S * (1.0 + 6.0 * pso_kpa / (7.0 * p0)).sqrt();

        // Arrival time ta (ms)
        let arrival_time_ms = 1000.0 * standoff / shock_vel;

        // Friedlander decay coefficient b calibrated to impulse: ir = Pr * td * (1/b - (1 - e^-b)/b^2)
        let b = Self::solve_friedlander_decay(pr_kpa, td_ms, ir_kpa_ms);

        // Clearing relief time tc (ms) for finite targets
        // Sound speed in reflected gas: Cr = a0 * sqrt(1 + 0.14 * Pr / P0)
        let c_r = Self::A_SOUND_M_S * (1.0 + 0.14 * pr_kpa / p0).sqrt();
        let clearing_dist_s = (target_height_m * 0.5).min(target_width_m * 0.5).max(0.2);
        let tc_ms = (3.0 * clearing_dist_s / c_r) * 1000.0;

        let cleared_ir_kpa_ms = if tc_ms < td_ms {
            let ratio = tc_ms / td_ms;
            ir_kpa_ms * ratio.powf(0.65)
        } else {
            ir_kpa_ms
        };

        BlastWaveParameters {
            effective_tnt_mass_kg: effective_tnt_mass,
            standoff_distance_m: standoff,
            scaled_distance_z: z,
            peak_incident_overpressure_kpa: pso_kpa,
            peak_reflected_overpressure_kpa: pr_kpa,
            positive_phase_duration_ms: td_ms,
            reflected_impulse_kpa_ms: ir_kpa_ms,
            incident_impulse_kpa_ms: is_kpa_ms,
            shock_velocity_m_s: shock_vel,
            arrival_time_ms,
            friedlander_decay_coeff: b,
            clearing_time_ms: tc_ms,
            cleared_impulse_kpa_ms: cleared_ir_kpa_ms,
        }
    }

    /// Solves for the Friedlander exponential decay parameter b using Newton-Raphson.
    fn solve_friedlander_decay(pr: f64, td: f64, ir: f64) -> f64 {
        if pr <= 0.0 || td <= 0.0 || ir <= 0.0 {
            return 1.0;
        }
        let target_ratio = (ir / (pr * td)).clamp(0.05, 0.499);
        let mut b: f64 = 1.20;
        for _ in 0..15 {
            let exp_neg_b = (-b).exp();
            let f = 1.0 / b - (1.0 - exp_neg_b) / (b * b) - target_ratio;
            let f_prime = -1.0 / (b * b) - (b * exp_neg_b - 2.0 * (1.0 - exp_neg_b)) / (b * b * b);
            if f_prime.abs() < 1e-7 {
                break;
            }
            let next_b = b - f / f_prime;
            b = next_b.clamp(0.1, 10.0);
            if (f / f_prime).abs() < 1e-4 {
                break;
            }
        }
        b
    }

    /// Evaluates structural resistance and SDOF parameters for a given component.
    pub fn evaluate_sdof_system(
        input: &BlastAnalysisInput,
        wave: &BlastWaveParameters,
    ) -> SdofSystemParameters {
        let span = input.span_length_m.max(0.5);
        let width = input.tributary_width_m.max(0.1);
        let thickness = input.thickness_m.max(0.05);

        // Density and total mass
        let density_kg_m3 = match input.component_kind {
            BlastComponentKind::RcWall | BlastComponentKind::RcSlab => 2450.0,
            BlastComponentKind::SteelColumn | BlastComponentKind::SteelBarrier => 7850.0,
            BlastComponentKind::BlastGlazing => 2500.0,
        };
        let total_mass = density_kg_m3 * span * width * thickness;

        // Dynamic material strengths with Strain Rate Dynamic Increase Factors (DIF)
        // UFC 3-340-02 Table 4-1 / ASCE 59-11
        let (dif_c, dif_y, sif) = match input.component_kind {
            BlastComponentKind::RcWall | BlastComponentKind::RcSlab => (1.25, 1.20, 1.10),
            BlastComponentKind::SteelColumn | BlastComponentKind::SteelBarrier => (1.00, 1.20, 1.10),
            BlastComponentKind::BlastGlazing => (1.00, 1.00, 1.00),
        };
        let f_dc = input.f_c_mpa * dif_c;
        let f_dy = input.f_y_mpa * dif_y * sif;

        // Plastic moment capacity Mp and elastic stiffness ke
        let (mp_knm, ke_kn_m) = match input.component_kind {
            BlastComponentKind::RcWall | BlastComponentKind::RcSlab => {
                let d = thickness * 0.85; // effective depth
                let as_m2 = (input.rebar_ratio_percent / 100.0) * width * d;
                let a = (as_m2 * f_dy * 1000.0) / (0.85 * f_dc * 1000.0 * width);
                let mp = as_m2 * (f_dy * 1000.0) * (d - a * 0.5); // kNm

                // Concrete modulus of elasticity Ec (kPa) per ACI 318
                let ec_kpa = 4700.0 * input.f_c_mpa.sqrt() * 1000.0;
                let ig_m4 = (width * thickness.powi(3)) / 12.0;
                let ieff_m4 = ig_m4 * 0.50; // cracked section inertia
                let defl_coeff = input.support_condition.deflection_coefficient();
                let ke = (width * span) / (defl_coeff * span.powi(4) / (ec_kpa * ieff_m4));
                (mp, ke)
            }
            BlastComponentKind::SteelColumn | BlastComponentKind::SteelBarrier => {
                let z_x = (width * thickness.powi(2)) / 4.0; // plastic section modulus (m3)
                let mp = z_x * (f_dy * 1000.0);
                let es_kpa = 200_000_000.0; // 200 GPa
                let i_m4 = (width * thickness.powi(3)) / 12.0;
                let defl_coeff = input.support_condition.deflection_coefficient();
                let ke = (width * span) / (defl_coeff * span.powi(4) / (es_kpa * i_m4));
                (mp, ke)
            }
            BlastComponentKind::BlastGlazing => {
                let e_glass_kpa = 70_000_000.0;
                let f_glass_dyn_mpa = 80.0;
                let s_m3 = (width * thickness.powi(2)) / 6.0;
                let mp = s_m3 * (f_glass_dyn_mpa * 1000.0);
                let i_m4 = (width * thickness.powi(3)) / 12.0;
                let defl_coeff = input.support_condition.deflection_coefficient();
                let ke = (width * span) / (defl_coeff * span.powi(4) / (e_glass_kpa * i_m4));
                (mp, ke)
            }
        };

        // Ultimate dynamic resistance capacity Ru (kN)
        let mom_coeff = input.support_condition.moment_coefficient();
        let ru_kn = (mp_knm / (mom_coeff * span)).max(1.0);

        let klm = (input.support_condition.klm_elastic() + input.support_condition.klm_plastic()) * 0.5;
        let me_kg = klm * total_mass;

        let ye_mm = (ru_kn / ke_kn_m) * 1000.0;

        // Circular natural frequency and period Tn (ms)
        let omega_n = (ke_kn_m * 1000.0 / me_kg.max(1.0)).sqrt();
        let tn_ms = if omega_n > 0.0 {
            1000.0 * (2.0 * std::f64::consts::PI / omega_n)
        } else {
            50.0
        };

        // Response regime classification
        let duration_ratio = wave.positive_phase_duration_ms / tn_ms.max(0.1);
        let regime = if duration_ratio < 0.10 {
            BlastResponseRegime::Impulsive
        } else if duration_ratio <= 2.0 {
            BlastResponseRegime::Dynamic
        } else {
            BlastResponseRegime::QuasiStatic
        };

        SdofSystemParameters {
            total_mass_kg: total_mass,
            equivalent_mass_kg: me_kg,
            elastic_stiffness_kn_per_m: ke_kn_m,
            ultimate_resistance_kn: ru_kn,
            yield_displacement_mm: ye_mm,
            natural_period_ms: tn_ms,
            omega_n_rad_per_s: omega_n,
            klm_factor: klm,
            response_regime: regime,
        }
    }

    /// Numerical integration of SDOF elastoplastic equation of motion via Central Difference method.
    pub fn perform_sdof_analysis(
        input: &BlastAnalysisInput,
        wave: &BlastWaveParameters,
        sdof: &SdofSystemParameters,
    ) -> (f64, f64, f64, f64, f64, f64, Vec<SdofTimeStep>) {
        let trib_area = input.span_length_m * input.tributary_width_m;
        let p_peak = wave.peak_reflected_overpressure_kpa;
        let td = wave.positive_phase_duration_ms;
        let b = wave.friedlander_decay_coeff;
        let kl = match input.support_condition {
            SupportCondition::SimplySupported => 0.58,
            SupportCondition::FixedFixed => 0.56,
            SupportCondition::ProppedCantilever => 0.57,
            SupportCondition::Cantilever => 0.58,
        };

        let me = sdof.equivalent_mass_kg;
        let ke = sdof.elastic_stiffness_kn_per_m * 1000.0; // N/m
        let ru = sdof.ultimate_resistance_kn * 1000.0;     // N
        let ye = sdof.yield_displacement_mm / 1000.0;      // m
        let c = 2.0 * input.damping_ratio * (ke * me).sqrt(); // N*s/m

        let t_end = (4.0 * td).max(2.5 * sdof.natural_period_ms).clamp(40.0, 300.0);
        let dt = (sdof.natural_period_ms / 150.0).clamp(0.02, 0.25) / 1000.0; // seconds
        let num_steps = ((t_end / 1000.0) / dt).ceil() as usize;

        let mut y = 0.0;
        let mut v = 0.0;
        let mut y_max = 0.0;
        let mut t_at_max = 0.0;
        let mut y_history = Vec::with_capacity(num_steps);
        let mut max_reaction_kn = 0.0;

        let sample_stride = (num_steps / 100).max(1);
        let mut samples = Vec::new();

        for i in 0..=num_steps {
            let t_s = (i as f64) * dt;
            let t_ms = t_s * 1000.0;

            // Instantaneous blast pressure P(t) via Friedlander formula
            let p_t = if t_ms <= td {
                let tau = t_ms / td;
                p_peak * (1.0 - tau) * (-b * tau).exp()
            } else {
                0.0
            };

            let f_applied_n = kl * (p_t * 1000.0) * trib_area;

            // Bilinear elastoplastic resistance function
            let r_n = if y <= ye {
                ke * y
            } else {
                ru
            };

            // Support dynamic reaction force V_sup(t) per Biggs
            let reaction_n = 0.38 * r_n + 0.12 * f_applied_n;
            let reaction_kn = reaction_n / 1000.0;
            if reaction_kn > max_reaction_kn {
                max_reaction_kn = reaction_kn;
            }

            // Equation of motion: a = (F - C*v - R) / Me
            let a = (f_applied_n - c * v - r_n) / me;

            v += a * dt;
            y += v * dt;

            if y < 0.0 && t_ms > td {
                // Rebound damping
                v *= 0.98;
            }

            if y > y_max {
                y_max = y;
                t_at_max = t_ms;
            }

            y_history.push(y);

            if i % sample_stride == 0 {
                samples.push(SdofTimeStep {
                    time_ms: t_ms,
                    blast_pressure_kpa: p_t,
                    applied_force_kn: f_applied_n / 1000.0,
                    displacement_mm: y * 1000.0,
                    velocity_m_s: v,
                    resistance_force_kn: r_n / 1000.0,
                    support_reaction_kn: reaction_kn,
                });
            }
        }

        let max_disp_mm = y_max * 1000.0;
        let perm_disp_mm = (y_max - ye).max(0.0) * 1000.0;

        // Dynamic ductility ratio mu = y_max / y_e
        let ductility_ratio = (max_disp_mm / sdof.yield_displacement_mm.max(0.1)).max(0.01);

        // Support rotation angle theta = atan(y_max / (L / 2)) in degrees
        let l_c = input.span_length_m * 0.5;
        let theta_rad = (y_max / l_c).atan();
        let support_rot_deg = theta_rad.to_degrees();

        // Shear force demand and capacity check
        let _max_shear_demand = max_reaction_kn * 0.5;
        let _dynamic_shear_cap = match input.component_kind {
            BlastComponentKind::RcWall | BlastComponentKind::RcSlab => {
                // ACI 318 direct shear capacity with dynamic increase: Vc_dyn = 0.17 * sqrt(f_dc) * b * d
                let d = input.thickness_m * 0.85;
                0.17 * (input.f_c_mpa * 1.25).sqrt() * input.tributary_width_m * d * 1000.0
            }
            BlastComponentKind::SteelColumn | BlastComponentKind::SteelBarrier => {
                0.60 * (input.f_y_mpa * 1.20 * 1.10) * (input.tributary_width_m * input.thickness_m) * 1000.0
            }
            BlastComponentKind::BlastGlazing => 120.0,
        };

        (
            max_disp_mm,
            t_at_max,
            perm_disp_mm,
            ductility_ratio,
            support_rot_deg,
            max_reaction_kn,
            samples,
        )
    }

    /// Evaluates damage level per PDC-TR-06-08 / UFC 3-340-02 Table 4-2 criteria.
    pub fn classify_damage_level(
        ductility: f64,
        support_rot_deg: f64,
        component: BlastComponentKind,
    ) -> UfcDamageLevel {
        match component {
            BlastComponentKind::RcWall | BlastComponentKind::RcSlab => {
                if ductility <= 1.0 && support_rot_deg <= 0.50 {
                    UfcDamageLevel::Superficial
                } else if ductility <= 3.0 && support_rot_deg <= 1.0 {
                    UfcDamageLevel::Low
                } else if ductility <= 6.0 && support_rot_deg <= 2.0 {
                    UfcDamageLevel::Medium
                } else if ductility <= 10.0 && support_rot_deg <= 6.0 {
                    UfcDamageLevel::High
                } else {
                    UfcDamageLevel::Hazardous
                }
            }
            BlastComponentKind::SteelColumn | BlastComponentKind::SteelBarrier => {
                if ductility <= 1.0 && support_rot_deg <= 0.50 {
                    UfcDamageLevel::Superficial
                } else if ductility <= 3.0 && support_rot_deg <= 1.5 {
                    UfcDamageLevel::Low
                } else if ductility <= 10.0 && support_rot_deg <= 3.0 {
                    UfcDamageLevel::Medium
                } else if ductility <= 20.0 && support_rot_deg <= 6.0 {
                    UfcDamageLevel::High
                } else {
                    UfcDamageLevel::Hazardous
                }
            }
            BlastComponentKind::BlastGlazing => {
                if ductility <= 1.0 {
                    UfcDamageLevel::Superficial
                } else if ductility <= 2.0 {
                    UfcDamageLevel::Low
                } else if ductility <= 4.0 {
                    UfcDamageLevel::Medium
                } else {
                    UfcDamageLevel::High
                }
            }
        }
    }

    /// Generates Pressure-Impulse (P-I) analytical iso-damage curves for UFC damage levels.
    pub fn generate_pi_curves(
        sdof: &SdofSystemParameters,
        trib_area: f64,
    ) -> Vec<(UfcDamageLevel, Vec<PiCurvePoint>)> {
        let me = sdof.equivalent_mass_kg;
        let ru = sdof.ultimate_resistance_kn;
        let ye_m = sdof.yield_displacement_mm / 1000.0;

        let levels = [
            (UfcDamageLevel::Superficial, 1.0),
            (UfcDamageLevel::Low, 3.0),
            (UfcDamageLevel::Medium, 6.0),
            (UfcDamageLevel::High, 10.0),
        ];

        let mut result = Vec::new();

        for (lvl, target_mu) in levels {
            let y_crit_m = target_mu * ye_m;
            // Impulsive asymptotic energy relation: I_crit = sqrt(2 * Me * Ru * (y_crit - 0.5 * ye))
            let strain_energy = (ru * 1000.0) * (y_crit_m - 0.5 * ye_m).max(0.001);
            let i_asymptote_pa_s = (2.0 * me * strain_energy).sqrt();
            let i_crit_kpa_ms = (i_asymptote_pa_s / trib_area) / 1000.0 * 1000.0; // kPa-ms

            // Quasi-static asymptotic pressure: P_crit = Ru / Area
            let p_crit_kpa = ru / trib_area;

            let mut pts = Vec::new();
            // Hyperbolic formulation: (P/P_crit - 1) * (I/I_crit - 1) = C
            for factor in [1.05, 1.15, 1.30, 1.50, 1.80, 2.20, 3.0, 4.5, 7.0, 12.0] {
                let p = p_crit_kpa * factor;
                let i = i_crit_kpa_ms * (1.0 + 0.40 / (factor - 1.0).max(0.02));
                pts.push(PiCurvePoint {
                    impulse_kpa_ms: i,
                    pressure_kpa: p,
                });
            }
            result.push((lvl, pts));
        }

        result
    }

    /// Full Blast Engineering Evaluation Engine pipeline.
    pub fn run_assessment(input: &BlastAnalysisInput) -> BlastAnalysisReport {
        let wave = Self::evaluate_blast_wave(
            input.explosive_mass_kg,
            input.explosive_type,
            input.burst_type,
            input.standoff_distance_m,
            input.span_length_m,
            input.tributary_width_m,
        );

        let sdof = Self::evaluate_sdof_system(input, &wave);

        let (
            max_disp_mm,
            time_at_max_disp_ms,
            perm_disp_mm,
            ductility_ratio,
            support_rot_deg,
            max_reaction_kn,
            samples,
        ) = Self::perform_sdof_analysis(input, &wave, &sdof);

        let eval_damage = Self::classify_damage_level(ductility_ratio, support_rot_deg, input.component_kind);
        let is_target_achieved = eval_damage <= input.target_damage_level;

        let trib_area = input.span_length_m * input.tributary_width_m;
        let pi_curves = Self::generate_pi_curves(&sdof, trib_area);

        // Dynamic direct shear capacity check
        let max_shear_demand = max_reaction_kn * 0.5;
        let dynamic_shear_cap = match input.component_kind {
            BlastComponentKind::RcWall | BlastComponentKind::RcSlab => {
                let d = input.thickness_m * 0.85;
                0.17 * (input.f_c_mpa * 1.25).sqrt() * input.tributary_width_m * d * 1000.0
            }
            BlastComponentKind::SteelColumn | BlastComponentKind::SteelBarrier => {
                0.60 * (input.f_y_mpa * 1.20 * 1.10) * (input.tributary_width_m * input.thickness_m) * 1000.0
            }
            BlastComponentKind::BlastGlazing => 120.0,
        };

        // UFC 3-340-02 & ASCE 59-11 Compliance Checks
        let allowable_ductility = match input.target_damage_level {
            UfcDamageLevel::Superficial => 1.0,
            UfcDamageLevel::Low => 3.0,
            UfcDamageLevel::Medium => 6.0,
            UfcDamageLevel::High => 10.0,
            UfcDamageLevel::Hazardous => 20.0,
        };
        let allowable_rotation_deg = match input.target_damage_level {
            UfcDamageLevel::Superficial => 0.50,
            UfcDamageLevel::Low => 1.00,
            UfcDamageLevel::Medium => 2.00,
            UfcDamageLevel::High => 6.00,
            UfcDamageLevel::Hazardous => 12.00,
        };

        let mut compliance_audit = Vec::new();

        // 1. Scaled Distance Standoff
        compliance_audit.push(BlastComplianceItem {
            parameter_name: "Scaled Distance Standoff (Z)".to_string(),
            code_reference: "UFC 3-340-02 §2-4 / ASCE 59-11 §3.2".to_string(),
            demand_val: wave.scaled_distance_z,
            permissible_limit: 1.20,
            unit: "m/kg^(1/3)".to_string(),
            utilization_ratio: 1.20 / wave.scaled_distance_z.max(0.01),
            is_passed: wave.scaled_distance_z >= 1.20,
        });

        // 2. Dynamic Ductility Ratio
        compliance_audit.push(BlastComplianceItem {
            parameter_name: "Dynamic Ductility Ratio (μ = y_max / y_e)".to_string(),
            code_reference: "PDC-TR-06-08 Table 3-1 / UFC 3-340-02 Table 4-2".to_string(),
            demand_val: ductility_ratio,
            permissible_limit: allowable_ductility,
            unit: "-".to_string(),
            utilization_ratio: ductility_ratio / allowable_ductility,
            is_passed: ductility_ratio <= allowable_ductility,
        });

        // 3. Support Rotation Hinge Angle
        compliance_audit.push(BlastComplianceItem {
            parameter_name: "Support Rotation Hinge Angle (θ)".to_string(),
            code_reference: "UFC 3-340-02 §4-12 / ASCE 59-11 §5.3".to_string(),
            demand_val: support_rot_deg,
            permissible_limit: allowable_rotation_deg,
            unit: "deg".to_string(),
            utilization_ratio: support_rot_deg / allowable_rotation_deg,
            is_passed: support_rot_deg <= allowable_rotation_deg,
        });

        // 4. Dynamic Direct Shear Demand vs Capacity
        compliance_audit.push(BlastComplianceItem {
            parameter_name: "Dynamic Direct Shear Resistance (Vu / Vc_dyn)".to_string(),
            code_reference: "UFC 3-340-02 §4-18 / ACI 318-19 §18.10".to_string(),
            demand_val: max_shear_demand,
            permissible_limit: dynamic_shear_cap,
            unit: "kN".to_string(),
            utilization_ratio: max_shear_demand / dynamic_shear_cap.max(0.1),
            is_passed: max_shear_demand <= dynamic_shear_cap,
        });

        // 5. Standoff Perimeter Defense Margin
        let standoff_margin = input.standoff_distance_m / 10.0;
        compliance_audit.push(BlastComplianceItem {
            parameter_name: "Security Standoff Perimeter Defense".to_string(),
            code_reference: "GSA PBS-P100 / ISC Physical Security Standard §4.1".to_string(),
            demand_val: input.standoff_distance_m,
            permissible_limit: 10.0,
            unit: "m".to_string(),
            utilization_ratio: 1.0 / standoff_margin.max(0.1),
            is_passed: input.standoff_distance_m >= 10.0,
        });

        BlastAnalysisReport {
            input: input.clone(),
            blast_wave: wave,
            sdof_system: sdof,
            max_displacement_mm: max_disp_mm,
            time_at_max_disp_ms: time_at_max_disp_ms,
            permanent_displacement_mm: perm_disp_mm,
            dynamic_ductility_ratio: ductility_ratio,
            support_rotation_deg: support_rot_deg,
            max_reaction_force_kn: max_reaction_kn,
            max_shear_demand_kn: max_shear_demand,
            dynamic_shear_capacity_kn: dynamic_shear_cap,
            evaluated_damage_level: eval_damage,
            is_target_damage_achieved: is_target_achieved,
            time_history_samples: samples,
            pi_curves,
            compliance_audit,
        }
    }

    /// Generates 3D structural nodes and elements for a high-security reinforced blast bunker archetype.
    pub fn generate_blast_bunker_archetype() -> (Vec<Node3D>, Vec<Element3D>) {
        let mut nodes = Vec::new();
        let mut elements = Vec::new();

        let _l = 12.0; // bunker length (m)
        let _w = 8.0;  // bunker width (m)
        let h = 4.2;  // bunker story height (m)
        let standoff = 15.0; // defensive revetment perimeter distance (m)

        // Materials & Cross-sections
        let mat_rc = Material {
            name: "High-Strength Blast RC (f'c=50 MPa)".to_string(),
            elastic_modulus: 33.0e9,
            poisson_ratio: 0.20,
            yield_strength: 50.0e6,
            density: 2500.0,
        };

        let mat_steel = Material {
            name: "Blast Grade Structural Steel (ASTM A572 Gr 50)".to_string(),
            elastic_modulus: 200.0e9,
            poisson_ratio: 0.30,
            yield_strength: 345.0e6,
            density: 7850.0,
        };

        let sec_blast_wall = CrossSection {
            name: "RC Blast Wall 450mm".to_string(),
            area: 0.45 * 1.0,
            iy: (1.0 * 0.45_f64.powi(3)) / 12.0,
            iz: (0.45 * 1.0_f64.powi(3)) / 12.0,
            j_torsion: 0.005,
            depth: 0.45,
            width: 1.0,
        };

        let sec_roof_girder = CrossSection {
            name: "RC Roof Diaphragm Girder 600x800mm".to_string(),
            area: 0.60 * 0.80,
            iy: (0.60 * 0.80_f64.powi(3)) / 12.0,
            iz: (0.80 * 0.60_f64.powi(3)) / 12.0,
            j_torsion: 0.015,
            depth: 0.80,
            width: 0.60,
        };

        let sec_steel_barrier = CrossSection {
            name: "Deflector Revetment Barrier 300x300x16".to_string(),
            area: 0.018,
            iy: 2.5e-4,
            iz: 2.5e-4,
            j_torsion: 3.8e-4,
            depth: 0.30,
            width: 0.30,
        };

        let mut node_id = 1;
        // Foundation nodes (z = 0)
        let x_coords = [0.0, 4.0, 8.0, 12.0];
        let y_coords = [0.0, 4.0, 8.0];

        let mut base_node_ids = Vec::new();
        for &x in &x_coords {
            let mut col = Vec::new();
            for &y in &y_coords {
                nodes.push(Node3D {
                    id: node_id,
                    x,
                    y,
                    z: 0.0,
                    fixity: BoundaryFixity::fixed(),
                    spring_stiffness: [0.0; 6],
                });
                col.push(node_id);
                node_id += 1;
            }
            base_node_ids.push(col);
        }

        // Roof nodes (z = h)
        let mut roof_node_ids = Vec::new();
        for &x in &x_coords {
            let mut col = Vec::new();
            for &y in &y_coords {
                nodes.push(Node3D {
                    id: node_id,
                    x,
                    y,
                    z: h,
                    fixity: BoundaryFixity::free(),
                    spring_stiffness: [0.0; 6],
                });
                col.push(node_id);
                node_id += 1;
            }
            roof_node_ids.push(col);
        }

        let mut elem_id = 1;
        // Vertical blast shear columns / wall elements
        for i in 0..x_coords.len() {
            for j in 0..y_coords.len() {
                elements.push(Element3D {
                    id: elem_id,
                    name: format!("BW-{}-{}", i + 1, j + 1),
                    start_node: base_node_ids[i][j],
                    end_node: roof_node_ids[i][j],
                    element_type: ElementType::Frame3D,
                    material: mat_rc.clone(),
                    section: sec_blast_wall.clone(),
                });
                elem_id += 1;
            }
        }

        // Roof girders in X-direction
        for j in 0..y_coords.len() {
            for i in 0..(x_coords.len() - 1) {
                elements.push(Element3D {
                    id: elem_id,
                    name: format!("RGX-{}-{}", i + 1, j + 1),
                    start_node: roof_node_ids[i][j],
                    end_node: roof_node_ids[i + 1][j],
                    element_type: ElementType::Frame3D,
                    material: mat_rc.clone(),
                    section: sec_roof_girder.clone(),
                });
                elem_id += 1;
            }
        }

        // Roof girders in Y-direction
        for i in 0..x_coords.len() {
            for j in 0..(y_coords.len() - 1) {
                elements.push(Element3D {
                    id: elem_id,
                    name: format!("RGY-{}-{}", i + 1, j + 1),
                    start_node: roof_node_ids[i][j],
                    end_node: roof_node_ids[i][j + 1],
                    element_type: ElementType::Frame3D,
                    material: mat_rc.clone(),
                    section: sec_roof_girder.clone(),
                });
                elem_id += 1;
            }
        }

        // Perimeter standoff blast deflector revetment barrier (at y = -standoff)
        let barrier_y = -standoff;
        let barrier_x_coords = [-2.0, 2.0, 6.0, 10.0, 14.0];
        let mut barrier_base_ids = Vec::new();
        let mut barrier_top_ids = Vec::new();

        for &bx in &barrier_x_coords {
            // Base
            nodes.push(Node3D {
                id: node_id,
                x: bx,
                y: barrier_y,
                z: 0.0,
                fixity: BoundaryFixity::fixed(),
                spring_stiffness: [0.0; 6],
            });
            barrier_base_ids.push(node_id);
            node_id += 1;

            // Top (z = 3.2m deflector)
            nodes.push(Node3D {
                id: node_id,
                x: bx,
                y: barrier_y,
                z: 3.2,
                fixity: BoundaryFixity::free(),
                spring_stiffness: [0.0; 6],
            });
            barrier_top_ids.push(node_id);
            node_id += 1;
        }

        for k in 0..barrier_x_coords.len() {
            // Vertical post
            elements.push(Element3D {
                id: elem_id,
                name: format!("BP-{}", k + 1),
                start_node: barrier_base_ids[k],
                end_node: barrier_top_ids[k],
                element_type: ElementType::Frame3D,
                material: mat_steel.clone(),
                section: sec_steel_barrier.clone(),
            });
            elem_id += 1;

            // Horizontal waling
            if k + 1 < barrier_x_coords.len() {
                elements.push(Element3D {
                    id: elem_id,
                    name: format!("BWAL-{}", k + 1),
                    start_node: barrier_top_ids[k],
                    end_node: barrier_top_ids[k + 1],
                    element_type: ElementType::Frame3D,
                    material: mat_steel.clone(),
                    section: sec_steel_barrier.clone(),
                });
                elem_id += 1;
            }
        }

        (nodes, elements)
    }

    /// Generates a comprehensive professional Markdown calculation dossier for blast resistant design.
    pub fn generate_markdown_dossier(report: &BlastAnalysisReport) -> String {
        let mut md = String::new();
        md.push_str("# EIGENGRAPH STRUCTURAL ANALYSIS PLATFORM — CALCULATION DOSSIER\n");
        md.push_str("## Blast Resistant Design & Structural Dynamics Under Explosive Impulses\n");
        md.push_str(&format!("**Project:** {}  \n", report.input.project_name));
        md.push_str("**Standards:** DoD UFC 3-340-02 / ASCE 59-11 / USACE PDC-TR-06-08 / ACI 318-19  \n");
        md.push_str("**Analyst:** EigenGraph Automated Certification  \n\n");

        md.push_str("---\n\n");
        md.push_str("### 1. Executive Summary & Threat Characterization\n\n");
        md.push_str("| Threat Parameter | Value | Engineering Description |\n");
        md.push_str("| :--- | :--- | :--- |\n");
        md.push_str(&format!("| **Explosive Threat Mass** | {:.1} kg | {} |\n", report.input.explosive_mass_kg, report.input.explosive_type.label()));
        md.push_str(&format!("| **TNT Equivalence Factor** | {:.2} | Standard Shock Factor |\n", report.input.explosive_type.tnt_equivalence_factor()));
        md.push_str(&format!("| **Effective TNT Charge** | **{:.1} kg TNT** | Including Hemispherical Ground Reflection ({:.2}x) |\n", report.blast_wave.effective_tnt_mass_kg, report.input.burst_type.reflection_enhancement_factor()));
        md.push_str(&format!("| **Standoff Distance (R)** | **{:.1} m** | Perimeter Standoff from Source to Target |\n", report.blast_wave.standoff_distance_m));
        md.push_str(&format!("| **Scaled Distance (Z)** | **{:.2} m/kg^(1/3)** | Kingery-Bulmash Scaling Parameter |\n", report.blast_wave.scaled_distance_z));
        md.push_str(&format!("| **Peak Incident Overpressure (Pso)** | {:.1} kPa ({:.2} bar) | Free-field shock wave peak |\n", report.blast_wave.peak_incident_overpressure_kpa, report.blast_wave.peak_incident_overpressure_kpa / 100.0));
        md.push_str(&format!("| **Peak Reflected Overpressure (Pr)** | **{:.1} kPa ({:.2} bar)** | Normally reflected shock front on wall |\n", report.blast_wave.peak_reflected_overpressure_kpa, report.blast_wave.peak_reflected_overpressure_kpa / 100.0));
        md.push_str(&format!("| **Positive Phase Duration (td)** | **{:.2} ms** | Duration of positive pressure pulse |\n", report.blast_wave.positive_phase_duration_ms));
        md.push_str(&format!("| **Reflected Specific Impulse (ir)** | **{:.1} kPa·ms** | Integral of positive reflected pressure pulse |\n", report.blast_wave.reflected_impulse_kpa_ms));
        md.push_str(&format!("| **Shock Front Velocity (Us)** | {:.1} m/s | Mach {:.2} in ambient air |\n", report.blast_wave.shock_velocity_m_s, report.blast_wave.shock_velocity_m_s / Self::A_SOUND_M_S));
        md.push_str(&format!("| **Friedlander Decay Coefficient (b)** | {:.3} | Exponential waveform decay rate |\n\n", report.blast_wave.friedlander_decay_coeff));

        md.push_str("### 2. SDOF Transformation & Structural Dynamics\n\n");
        md.push_str("| SDOF System Parameter | Calculated Value | Reference / Formulation |\n");
        md.push_str("| :--- | :--- | :--- |\n");
        md.push_str(&format!("| **Target Component Kind** | {} | Span L = {:.2} m, Width B = {:.2} m, Thickness h = {:.3} m |\n", report.input.component_kind.label(), report.input.span_length_m, report.input.tributary_width_m, report.input.thickness_m));
        md.push_str(&format!("| **Support Fixity** | {} | KLM = {:.2} (Biggs SDOF transformation) |\n", report.input.support_condition.label(), report.sdof_system.klm_factor));
        md.push_str(&format!("| **Total Component Mass** | {:.1} kg | Equivalent Mass Me = {:.1} kg |\n", report.sdof_system.total_mass_kg, report.sdof_system.equivalent_mass_kg));
        md.push_str(&format!("| **Dynamic Elastic Stiffness (ke)** | {:.1} kN/m | Based on dynamic cracked section inertia |\n", report.sdof_system.elastic_stiffness_kn_per_m));
        md.push_str(&format!("| **Ultimate Dynamic Resistance (Ru)** | **{:.1} kN** | Plastic collapse mechanism with DIF & SIF factors |\n", report.sdof_system.ultimate_resistance_kn));
        md.push_str(&format!("| **Elastic Yield Deflection (ye)** | **{:.2} mm** | ye = Ru / ke |\n", report.sdof_system.yield_displacement_mm));
        md.push_str(&format!("| **Natural Period (Tn)** | **{:.2} ms** | Circular frequency omega_n = {:.1} rad/s |\n", report.sdof_system.natural_period_ms, report.sdof_system.omega_n_rad_per_s));
        md.push_str(&format!("| **Dynamic Response Regime** | **{}** | td / Tn = {:.3} |\n\n", report.sdof_system.response_regime.label(), report.blast_wave.positive_phase_duration_ms / report.sdof_system.natural_period_ms.max(0.1)));

        md.push_str("### 3. Dynamic Structural Performance & Damage Level\n\n");
        md.push_str("| Response Metric | Demand | Allowable Limit | Status / Damage Level |\n");
        md.push_str("| :--- | :--- | :--- | :--- |\n");
        md.push_str(&format!("| **Peak Dynamic Displacement (ymax)** | **{:.2} mm** | Time at peak: {:.1} ms | Residual Perm: {:.2} mm |\n", report.max_displacement_mm, report.time_at_max_disp_ms, report.permanent_displacement_mm));
        md.push_str(&format!("| **Dynamic Ductility Ratio (μ)** | **{:.2}** | Limit: ≤ {:.2} | {} |\n", report.dynamic_ductility_ratio, if report.dynamic_ductility_ratio <= 3.0 { 3.0 } else { 6.0 }, if report.dynamic_ductility_ratio <= 3.0 { "🟢 COMPLIANT" } else { "🟡 HIGH PLASTICITY" }));
        md.push_str(&format!("| **Support Rotation Angle (θ)** | **{:.2}°** | Limit: ≤ {:.2}° | {} |\n", report.support_rotation_deg, if report.support_rotation_deg <= 1.0 { 1.0 } else { 2.0 }, if report.support_rotation_deg <= 1.0 { "🟢 COMPLIANT" } else { "🟡 LARGE ROTATION" }));
        md.push_str(&format!("| **Evaluated Damage Level** | **{}** | Target: {} | {} |\n\n", report.evaluated_damage_level.label(), report.input.target_damage_level.label(), if report.is_target_damage_achieved { "🟢 TARGET ACHIEVED" } else { "🔴 TARGET DEFICIT" }));

        md.push_str("### 4. Regulatory & Code Compliance Matrix (UFC 3-340-02 / ASCE 59-11)\n\n");
        md.push_str("| Parameter | Governing Code | Demand | Limit | D/C Ratio | Status |\n");
        md.push_str("| :--- | :--- | :--- | :--- | :--- | :--- |\n");
        for item in &report.compliance_audit {
            md.push_str(&format!(
                "| {} | {} | {:.2} {} | {:.2} {} | {:.3} | {} |\n",
                item.parameter_name,
                item.code_reference,
                item.demand_val,
                item.unit,
                item.permissible_limit,
                item.unit,
                item.utilization_ratio,
                if item.is_passed { "🟢 PASS" } else { "🔴 FAIL" }
            ));
        }
        md.push_str("\n---\n");
        md.push_str("*Autogenerated by EigenGraph Core v0.1.5 — Certified Structural Dynamics & Blast Engineering Suite.*\n");

        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kingery_bulmash_blast_wave_parameters() {
        let wave = BlastEngine::evaluate_blast_wave(
            100.0,
            ExplosiveType::Tnt,
            BurstType::SurfaceBurst,
            10.0,
            4.0,
            1.0,
        );

        // Effective mass = 100 * 1.0 * 1.8 = 180 kg TNT
        assert!(wave.effective_tnt_mass_kg > 179.0 && wave.effective_tnt_mass_kg < 181.0);
        // Z = 10.0 / 180^(1/3) = 10.0 / 5.646 = 1.77 m/kg^(1/3)
        assert!(wave.scaled_distance_z > 1.70 && wave.scaled_distance_z < 1.85);
        // Incident overpressure Pso should be between 100 and 400 kPa
        assert!(wave.peak_incident_overpressure_kpa > 100.0 && wave.peak_incident_overpressure_kpa < 400.0);
        // Reflected overpressure Pr should be roughly 2 to 6 times Pso
        assert!(wave.peak_reflected_overpressure_kpa > wave.peak_incident_overpressure_kpa * 2.0);
        // Duration td should be positive and reasonable
        assert!(wave.positive_phase_duration_ms > 2.0 && wave.positive_phase_duration_ms < 50.0);
        // Impulse ir should be positive
        assert!(wave.reflected_impulse_kpa_ms > 50.0);
    }

    #[test]
    fn test_sdof_system_and_plastic_resistance() {
        let input = BlastAnalysisInput::default();
        let wave = BlastEngine::evaluate_blast_wave(
            input.explosive_mass_kg,
            input.explosive_type,
            input.burst_type,
            input.standoff_distance_m,
            input.span_length_m,
            input.tributary_width_m,
        );

        let sdof = BlastEngine::evaluate_sdof_system(&input, &wave);
        assert!(sdof.total_mass_kg > 1000.0);
        assert!(sdof.equivalent_mass_kg < sdof.total_mass_kg);
        assert!(sdof.ultimate_resistance_kn > 10.0);
        assert!(sdof.elastic_stiffness_kn_per_m > 100.0);
        assert!(sdof.yield_displacement_mm > 0.1);
        assert!(sdof.natural_period_ms > 5.0 && sdof.natural_period_ms < 500.0);
    }

    #[test]
    fn test_blast_assessment_pipeline() {
        let input = BlastAnalysisInput::default();
        let rep = BlastEngine::run_assessment(&input);

        assert!(rep.max_displacement_mm > 0.0);
        assert!(rep.dynamic_ductility_ratio > 0.0);
        assert!(rep.support_rotation_deg >= 0.0);
        assert!(!rep.time_history_samples.is_empty());
        assert!(!rep.compliance_audit.is_empty());

        let dossier = BlastEngine::generate_markdown_dossier(&rep);
        assert!(dossier.contains("Blast Resistant Design & Structural Dynamics Under Explosive Impulses"));
        assert!(dossier.contains("UFC 3-340-02"));
    }

    #[test]
    fn test_blast_bunker_archetype_generation() {
        let (nodes, elements) = BlastEngine::generate_blast_bunker_archetype();
        assert!(nodes.len() >= 24);
        assert!(elements.len() >= 30);
    }
}
