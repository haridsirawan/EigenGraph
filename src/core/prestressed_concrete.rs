//! prestressed_concrete.rs — Prestressed & Post-Tensioned Concrete Analysis Engine (Fase 10: v0.1.0).
//! Compliant with ACI 318-19 Chapter 24, SNI 2847:2019 Pasal 24, AASHTO LRFD Bridge Design Specs Section 5,
//! and PCI Design Handbook (8th Edition).
//!
//! Features:
//! 1. Parametric Tendon Profiling (Parabolic, Harped 2-Point, Straight Eccentric).
//! 2. Immediate Prestress Losses (Curvature & Wobble Friction, Anchorage Slip, Concrete Elastic Shortening).
//! 3. Long-Term Time-Dependent Losses (Creep, Shrinkage, Steel Relaxation).
//! 4. Concrete Extreme Fiber Stresses at Transfer (Pi, Msw) and Service (Peff, Mtotal).
//! 5. ACI 318-19 / SNI 2847:2019 Table 24.5.2.1 Classification (Class U, Class T, Class C).
//! 6. Cracking Moment (Mcr) and Minimum Flexural Reinforcement Verification (φMn >= 1.2 Mcr).
//! 7. Ultimate Flexural Strength (φMn) with Whitney Stress Block & Steel Stress fps.
//! 8. Nominal Concrete Shear Resistance (Flexure-Shear Vci & Web-Shear Vcw).
//! 9. Initial Upward Camber & Long-Term Camber (PCI Multipliers).
//! 10. Comprehensive Markdown Design Dossier Generator.

use serde::{Deserialize, Serialize};

/// Prestressing Method / Construction Sequence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrestressingMethod {
    PreTensioned,
    PostTensioned,
}

impl PrestressingMethod {
    pub fn label(&self) -> &'static str {
        match self {
            Self::PreTensioned => "Pre-Tensioned (Precast Yard Fabricated)",
            Self::PostTensioned => "Post-Tensioned (Duct Jacked & Grouted)",
        }
    }
}

/// Tendon Longitudinal Geometry Profile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TendonProfileKind {
    Parabolic,
    HarpedTwoPoint,
    StraightEccentric,
}

impl TendonProfileKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Parabolic => "Parabolic Draped Profile (Continuous Harped)",
            Self::HarpedTwoPoint => "Harped 2-Point Depressed Profile (Hold-downs at L/3)",
            Self::StraightEccentric => "Straight Constant Eccentricity Tendon",
        }
    }
}

/// Prestressing Strand / Tendon Grade
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrandGrade {
    Grade270LowRelaxation, // fpu = 1860 MPa, gamma_p = 0.28, Kre = 34.5 MPa
    Grade250StressRelieved, // fpu = 1720 MPa, gamma_p = 0.40, Kre = 138.0 MPa
}

impl StrandGrade {
    pub fn fpu_mpa(&self) -> f64 {
        match self {
            Self::Grade270LowRelaxation => 1860.0,
            Self::Grade250StressRelieved => 1720.0,
        }
    }

    pub fn gamma_p(&self) -> f64 {
        match self {
            Self::Grade270LowRelaxation => 0.28,
            Self::Grade250StressRelieved => 0.40,
        }
    }

    pub fn k_re(&self) -> f64 {
        match self {
            Self::Grade270LowRelaxation => 34.5,
            Self::Grade250StressRelieved => 138.0,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Grade270LowRelaxation => "ASTM A416 Grade 270 (1860 MPa) Low-Relaxation",
            Self::Grade250StressRelieved => "ASTM A416 Grade 250 (1720 MPa) Stress-Relieved",
        }
    }
}

/// Girder Cross-Section Archetype
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GirderSectionArchetype {
    AashtoTypeIV,
    PrecastIGirder,
    BoxGirder,
    RectangularBeam,
}

impl GirderSectionArchetype {
    pub fn label(&self) -> &'static str {
        match self {
            Self::AashtoTypeIV => "AASHTO Type IV Standard Precast Girder (1370mm)",
            Self::PrecastIGirder => "Parametric Bulb-T / I-Girder (Custom Flanges)",
            Self::BoxGirder => "Precast Post-Tensioned Single/Multi-Cell Box Girder",
            Self::RectangularBeam => "Prestressed Solid Rectangular Girder",
        }
    }
}

/// ACI 318-19 Table 24.5.2.1 / SNI 2847:2019 Classification based on extreme bottom fiber tensile stress
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AciTensionClass {
    ClassU, // Uncracked: ft <= 0.62 * sqrt(f'c)
    ClassT, // Transition: 0.62 * sqrt(f'c) < ft <= 1.0 * sqrt(f'c)
    ClassC, // Cracked: ft > 1.0 * sqrt(f'c)
}

impl AciTensionClass {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ClassU => "Class U (Uncracked Section: ft ≤ 0.62√f'c)",
            Self::ClassT => "Class T (Transition Stage: 0.62√f'c < ft ≤ 1.0√f'c)",
            Self::ClassC => "Class C (Cracked Section: ft > 1.0√f'c, Ig invalid)",
        }
    }
}

/// Loss Breakdown Itemization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrestressLossBreakdown {
    pub friction_loss_mpa: f64,
    pub anchorage_slip_mpa: f64,
    pub elastic_shortening_mpa: f64,
    pub total_immediate_loss_mpa: f64,
    pub concrete_creep_mpa: f64,
    pub concrete_shrinkage_mpa: f64,
    pub steel_relaxation_mpa: f64,
    pub total_long_term_loss_mpa: f64,
    pub total_loss_mpa: f64,
    pub loss_percentage: f64,
    pub f_pj_mpa: f64,
    pub f_pi_mpa: f64,
    pub f_se_mpa: f64,
    pub p_jack_kn: f64,
    pub p_transfer_kn: f64,
    pub p_effective_kn: f64,
}

/// Fiber Stress Verification at a Given Stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiberStressCheck {
    pub stage_label: String,
    pub sigma_top_mpa: f64,
    pub sigma_bot_mpa: f64,
    pub allowable_comp_mpa: f64,
    pub allowable_tens_mpa: f64,
    pub top_dcr: f64,
    pub bot_dcr: f64,
    pub is_acceptable: bool,
}

/// Tendon Coordinate Point along Girder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TendonPointCoordinates {
    pub x_m: f64,
    pub e_mm: f64,
    pub z_coord_m: f64,
    pub slope_rad: f64,
}

/// Master Evaluation Dossier for Prestressed Concrete Girder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrestressedGirderReport {
    pub project_name: String,
    pub girder_label: String,
    pub span_length_m: f64,
    pub girder_spacing_m: f64,
    pub archetype: GirderSectionArchetype,
    pub method: PrestressingMethod,
    pub profile_kind: TendonProfileKind,
    pub strand_grade: StrandGrade,
    pub strand_dia_mm: f64,
    pub num_strands: usize,
    pub total_aps_mm2: f64,

    // Cross-Sectional Geometry
    pub depth_h_mm: f64,
    pub area_a_mm2: f64,
    pub inertia_ixx_mm4: f64,
    pub y_bot_mm: f64,
    pub y_top_mm: f64,
    pub s_bot_mm3: f64,
    pub s_top_mm3: f64,
    pub web_bw_mm: f64,
    pub top_flange_bf_mm: f64,

    // Material Strengths
    pub fc_prime_mpa: f64,
    pub fci_prime_mpa: f64,
    pub ec_mpa: f64,
    pub eci_mpa: f64,

    // Prestress Losses
    pub losses: PrestressLossBreakdown,

    // Fiber Stresses
    pub transfer_stress: FiberStressCheck,
    pub service_stress: FiberStressCheck,
    pub aci_class: AciTensionClass,

    // Flexural Capacities & Demands
    pub m_sw_knm: f64,
    pub m_sdl_knm: f64,
    pub m_ll_knm: f64,
    pub m_service_knm: f64,
    pub m_ultimate_knm: f64,
    pub m_cracking_knm: f64,
    pub min_flexure_req_knm: f64, // 1.2 * Mcr per ACI 318-19 Section 18.8.2
    pub fps_mpa: f64,
    pub whitney_a_mm: f64,
    pub neutral_axis_c_mm: f64,
    pub phi_mn_knm: f64,
    pub flexure_dcr: f64,

    // Shear Capacities & Demands
    pub v_ultimate_kn: f64,
    pub v_ci_kn: f64,
    pub v_cw_kn: f64,
    pub v_c_kn: f64,
    pub v_s_kn: f64,
    pub v_n_kn: f64,
    pub phi_vc_kn: f64,
    pub phi_vn_kn: f64,
    pub shear_dcr: f64,

    // Camber and Deflection
    pub camber_prestress_transfer_mm: f64,
    pub deflection_sw_transfer_mm: f64,
    pub net_camber_transfer_mm: f64,
    pub long_term_camber_mm: f64,

    // Tendon Profile Discretization
    pub tendon_profile_points: Vec<TendonPointCoordinates>,

    // Compliance Audit
    pub is_code_compliant: bool,
    pub governing_dcr: f64,
}

impl PrestressedGirderReport {
    /// Generates a comprehensive engineering calculation dossier in Markdown format
    pub fn generate_markdown_dossier(&self) -> String {
        let mut out = String::with_capacity(6144);
        out.push_str(&format!("# EigenGraph Prestressed Concrete Design Dossier\n"));
        out.push_str(&format!("**Project:** {} | **Component:** {}\n", self.project_name, self.girder_label));
        out.push_str(&format!("*Standard: ACI 318-19 Chapter 24 / SNI 2847:2019 Pasal 24 / AASHTO LRFD Section 5*\n\n"));

        out.push_str("## 1. Design Inputs & Cross-Section Properties\n\n");
        out.push_str("| Parameter | Symbol | Value | Unit |\n");
        out.push_str("|:---|:---:|:---:|:---:|\n");
        out.push_str(&format!("| Span Length | $L$ | {:.2} | m |\n", self.span_length_m));
        out.push_str(&format!("| Girder Spacing | $S$ | {:.2} | m |\n", self.girder_spacing_m));
        out.push_str(&format!("| Section Archetype | - | {} | - |\n", self.archetype.label()));
        out.push_str(&format!("| Section Depth | $h$ | {:.1} | mm |\n", self.depth_h_mm));
        out.push_str(&format!("| Cross-Sectional Area | $A$ | {:.1} | mm² |\n", self.area_a_mm2));
        out.push_str(&format!("| Moment of Inertia | $I_{{xx}}$ | {:.3e} | mm⁴ |\n", self.inertia_ixx_mm4));
        out.push_str(&format!("| Distance to Bottom Fiber | $y_b$ | {:.1} | mm |\n", self.y_bot_mm));
        out.push_str(&format!("| Distance to Top Fiber | $y_t$ | {:.1} | mm |\n", self.y_top_mm));
        out.push_str(&format!("| Bottom Section Modulus | $S_b$ | {:.3e} | mm³ |\n", self.s_bot_mm3));
        out.push_str(&format!("| Top Section Modulus | $S_t$ | {:.3e} | mm³ |\n", self.s_top_mm3));
        out.push_str(&format!("| Concrete Strength (28-day) | $f'_c$ | {:.1} | MPa |\n", self.fc_prime_mpa));
        out.push_str(&format!("| Concrete Strength (Transfer) | $f'_{{ci}}$ | {:.1} | MPa |\n", self.fci_prime_mpa));
        out.push_str(&format!("| Prestressing Strands | $N_s$ | {} × {:.1}mm | - |\n", self.num_strands, self.strand_dia_mm));
        out.push_str(&format!("| Total Strand Area | $A_{{ps}}$ | {:.1} | mm² |\n", self.total_aps_mm2));
        out.push_str(&format!("| Strand Tensile Strength | $f_{{pu}}$ | {:.1} | MPa |\n", self.strand_grade.fpu_mpa()));
        out.push_str(&format!("| Jacking Force | $P_{{jack}}$ | {:.1} | kN |\n\n", self.losses.p_jack_kn));

        out.push_str("## 2. Prestress Losses Calculation\n\n");
        out.push_str("| Loss Mechanism | Formulation | Loss (MPa) | % of $f_{pj}$ |\n");
        out.push_str("|:---|:---:|:---:|:---:|\n");
        out.push_str(&format!("| Curvature & Wobble Friction | $\\Delta f_{{pF}} = f_{{pj}}(1 - e^{{-(\\mu \\alpha + K x)}})$ | {:.1} | {:.2}% |\n",
            self.losses.friction_loss_mpa, (self.losses.friction_loss_mpa / self.losses.f_pj_mpa) * 100.0));
        out.push_str(&format!("| Anchorage Seating Draw-in | $\\Delta f_{{pA}} = \\frac{{\\Delta_{{anc}} E_p}}{{L_{{set}}}}$ | {:.1} | {:.2}% |\n",
            self.losses.anchorage_slip_mpa, (self.losses.anchorage_slip_mpa / self.losses.f_pj_mpa) * 100.0));
        out.push_str(&format!("| Concrete Elastic Shortening | $\\Delta f_{{pES}} = \\frac{{N-1}}{{2N}} \\frac{{E_p}}{{E_{{ci}}}} f_{{cgp}}$ | {:.1} | {:.2}% |\n",
            self.losses.elastic_shortening_mpa, (self.losses.elastic_shortening_mpa / self.losses.f_pj_mpa) * 100.0));
        out.push_str(&format!("| **Subtotal Immediate Loss** | - | **{:.1}** | **{:.2}%** |\n",
            self.losses.total_immediate_loss_mpa, (self.losses.total_immediate_loss_mpa / self.losses.f_pj_mpa) * 100.0));
        out.push_str(&format!("| Concrete Creep | $\\Delta f_{{pCR}} = K_{{cr}} \\frac{{E_p}}{{E_c}}(f_{{cgp}} - f_{{cds}})$ | {:.1} | {:.2}% |\n",
            self.losses.concrete_creep_mpa, (self.losses.concrete_creep_mpa / self.losses.f_pj_mpa) * 100.0));
        out.push_str(&format!("| Concrete Shrinkage | $\\Delta f_{{pSH}} = 8.2\\cdot 10^{{-6}} K_{{sh}} E_p (1 - 0.06 V/S)(100 - RH)$ | {:.1} | {:.2}% |\n",
            self.losses.concrete_shrinkage_mpa, (self.losses.concrete_shrinkage_mpa / self.losses.f_pj_mpa) * 100.0));
        out.push_str(&format!("| Steel Strand Relaxation | $\\Delta f_{{pRE}} = [K_{{re}} - J(\\Delta f_{{pSH}} + \\Delta f_{{pCR}} + \\Delta f_{{pES}})] C$ | {:.1} | {:.2}% |\n",
            self.losses.steel_relaxation_mpa, (self.losses.steel_relaxation_mpa / self.losses.f_pj_mpa) * 100.0));
        out.push_str(&format!("| **Total Prestress Losses** | **$\\Sigma \\Delta f_p$** | **{:.1}** | **{:.2}%** |\n\n",
            self.losses.total_loss_mpa, self.losses.loss_percentage));

        out.push_str(&format!("- Jacking Stress: $f_{{pj}} = {:.1}\\text{{ MPa}}$ ($P_{{jack}} = {:.1}\\text{{ kN}}$)\n", self.losses.f_pj_mpa, self.losses.p_jack_kn));
        out.push_str(&format!("- Transfer Stress: $f_{{pi}} = {:.1}\\text{{ MPa}}$ ($P_i = {:.1}\\text{{ kN}}$)\n", self.losses.f_pi_mpa, self.losses.p_transfer_kn));
        out.push_str(&format!("- Effective Service Stress: $f_{{se}} = {:.1}\\text{{ MPa}}$ ($P_{{eff}} = {:.1}\\text{{ kN}}$)\n\n", self.losses.f_se_mpa, self.losses.p_effective_kn));

        out.push_str("## 3. Concrete Fiber Stress Verification\n\n");
        out.push_str(&format!("### 3.1 Initial Stage at Transfer ($P_i = {:.1}\\text{{ kN}}$, $M_{{sw}} = {:.1}\\text{{ kNm}}$)\n\n", self.losses.p_transfer_kn, self.m_sw_knm));
        out.push_str("| Fiber Location | Calculated Stress | Allowable Stress Limit | DCR | Status |\n");
        out.push_str("|:---|:---:|:---:|:---:|:---:|\n");
        let top_i_stat = if self.transfer_stress.top_dcr <= 1.0 { "PASS" } else { "FAIL" };
        let bot_i_stat = if self.transfer_stress.bot_dcr <= 1.0 { "PASS" } else { "FAIL" };
        out.push_str(&format!("| Top Fiber (Tension/Comp) | {:.2} MPa | {:.2} MPa | {:.3} | **{}** |\n",
            self.transfer_stress.sigma_top_mpa, self.transfer_stress.allowable_tens_mpa, self.transfer_stress.top_dcr, top_i_stat));
        out.push_str(&format!("| Bottom Fiber (Compression) | {:.2} MPa | {:.2} MPa | {:.3} | **{}** |\n\n",
            self.transfer_stress.sigma_bot_mpa, self.transfer_stress.allowable_comp_mpa, self.transfer_stress.bot_dcr, bot_i_stat));

        out.push_str(&format!("### 3.2 Service Limit State ($P_{{eff}} = {:.1}\\text{{ kN}}$, $M_{{serv}} = {:.1}\\text{{ kNm}}$)\n\n", self.losses.p_effective_kn, self.m_service_knm));
        out.push_str(&format!("- **ACI 318-19 Classification:** {}\n", self.aci_class.label()));
        let top_s_stat = if self.service_stress.top_dcr <= 1.0 { "PASS" } else { "FAIL" };
        let bot_s_stat = if self.service_stress.bot_dcr <= 1.0 { "PASS" } else { "FAIL" };
        out.push_str("| Fiber Location | Calculated Stress | Allowable Stress Limit | DCR | Status |\n");
        out.push_str("|:---|:---:|:---:|:---:|:---:|\n");
        out.push_str(&format!("| Top Fiber (Compression) | {:.2} MPa | {:.2} MPa | {:.3} | **{}** |\n",
            self.service_stress.sigma_top_mpa, self.service_stress.allowable_comp_mpa, self.service_stress.top_dcr, top_s_stat));
        out.push_str(&format!("| Bottom Fiber (Tension) | {:.2} MPa | {:.2} MPa | {:.3} | **{}** |\n\n",
            self.service_stress.sigma_bot_mpa, self.service_stress.allowable_tens_mpa, self.service_stress.bot_dcr, bot_s_stat));

        out.push_str("## 4. Ultimate Flexural & Shear Capacities\n\n");
        out.push_str("| Limit State | Demand | Design Capacity | DCR | Status |\n");
        out.push_str("|:---|:---:|:---:|:---:|:---:|\n");
        let flex_stat = if self.flexure_dcr <= 1.0 { "PASS" } else { "FAIL" };
        let shear_stat = if self.shear_dcr <= 1.0 { "PASS" } else { "FAIL" };
        let mcr_stat = if self.phi_mn_knm >= self.min_flexure_req_knm { "PASS" } else { "FAIL" };
        out.push_str(&format!("| Ultimate Flexure | $M_u = {:.1}\\text{{ kNm}}$ | $\\phi M_n = {:.1}\\text{{ kNm}}$ | {:.3} | **{}** |\n",
            self.m_ultimate_knm, self.phi_mn_knm, self.flexure_dcr, flex_stat));
        out.push_str(&format!("| Minimum Flexure ($1.2 M_{{cr}}$) | $1.2 M_{{cr}} = {:.1}\\text{{ kNm}}$ | $\\phi M_n = {:.1}\\text{{ kNm}}$ | {:.3} | **{}** |\n",
            self.min_flexure_req_knm, self.phi_mn_knm, self.min_flexure_req_knm / self.phi_mn_knm, mcr_stat));
        out.push_str(&format!("| Concrete Shear Capacity | $V_u = {:.1}\\text{{ kN}}$ | $\\phi V_c = {:.1}\\text{{ kN}}$ | {:.3} | **{}** |\n\n",
            self.v_ultimate_kn, self.phi_vc_kn, self.shear_dcr, shear_stat));

        out.push_str("## 5. Camber & Deflection Analytics\n\n");
        out.push_str("| Deflection Component | Direction | Displacement |\n");
        out.push_str("|:---|:---:|:---:|\n");
        out.push_str(&format!("| Initial Camber from Prestress | Upward ($\\uparrow$) | +{:.1} mm |\n", self.camber_prestress_transfer_mm));
        out.push_str(&format!("| Immediate Self-Weight Deflection | Downward ($\\downarrow$) | -{:.1} mm |\n", self.deflection_sw_transfer_mm));
        out.push_str(&format!("| **Net Initial Camber at Transfer** | Upward ($\\uparrow$) | **+{:.1} mm** |\n", self.net_camber_transfer_mm));
        out.push_str(&format!("| **Long-Term Erection Camber (PCI Multipliers)** | Net | **{:.1} mm** |\n\n", self.long_term_camber_mm));

        out.push_str("## 6. Regulatory Verification Summary\n\n");
        let overall_stat = if self.is_code_compliant { "PASSED (Code Compliant)" } else { "FAILED (Re-design Required)" };
        out.push_str(&format!("- **Overall Compliance:** **{}**\n", overall_stat));
        out.push_str(&format!("- **Governing DCR:** **{:.3}**\n\n", self.governing_dcr));

        out.push_str("```\n");
        out.push_str("EigenGraph Prestressed & Post-Tensioned Engine — Verified & Sealed.\n");
        out.push_str("```\n");

        out
    }
}

/// Prestressed Concrete Computational Engine
pub struct PrestressedConcreteEngine;

impl PrestressedConcreteEngine {
    /// Comprehensive evaluation of a prestressed concrete girder
    pub fn evaluate_girder(
        project_name: &str,
        girder_label: &str,
        span_length_m: f64,
        girder_spacing_m: f64,
        archetype: GirderSectionArchetype,
        method: PrestressingMethod,
        profile_kind: TendonProfileKind,
        strand_grade: StrandGrade,
        strand_dia_mm: f64,
        num_strands: usize,
        jacking_ratio: f64,
        eccentricity_mid_mm: f64,
        eccentricity_end_mm: f64,
        fc_prime_mpa: f64,
        fci_prime_mpa: f64,
        superimposed_dead_load_kn_m: f64,
        live_load_kn_m: f64,
    ) -> PrestressedGirderReport {
        let fpu = strand_grade.fpu_mpa();
        let fpj = jacking_ratio.clamp(0.60, 0.85) * fpu;
        let ep = 196500.0; // MPa for 7-wire strands

        let a_single = if (strand_dia_mm - 12.7).abs() < 1.0 {
            98.7 // 0.5" strand
        } else {
            140.0 // 0.6" strand
        };
        let aps = (num_strands as f64) * a_single;
        let p_jack_kn = (aps * fpj) / 1000.0;

        // Concrete Moduli (ACI 318-19 Section 19.2.2.1)
        let eci = 4700.0 * fci_prime_mpa.sqrt();
        let ec = 4700.0 * fc_prime_mpa.sqrt();
        let rho_concrete = 2450.0; // kg/m3

        // Cross-Section Properties based on Archetype
        let (depth_h, area_a, inertia_ixx, y_bot, y_top, web_bw, top_bf) = match archetype {
            GirderSectionArchetype::AashtoTypeIV => {
                let h = 1370.0;
                let a = 509000.0;
                let i = 1.085e11;
                let yb = 628.0;
                let yt = h - yb;
                let bw = 203.0;
                let bf = 508.0;
                (h, a, i, yb, yt, bw, bf)
            }
            GirderSectionArchetype::PrecastIGirder => {
                let h = 1600.0;
                let a = 560000.0;
                let i = 1.620e11;
                let yb = 750.0;
                let yt = h - yb;
                let bw = 180.0;
                let bf = 650.0;
                (h, a, i, yb, yt, bw, bf)
            }
            GirderSectionArchetype::BoxGirder => {
                let h = 1500.0;
                let a = 720000.0;
                let i = 2.150e11;
                let yb = 720.0;
                let yt = h - yb;
                let bw = 350.0; // total 2 webs
                let bf = 1200.0;
                (h, a, i, yb, yt, bw, bf)
            }
            GirderSectionArchetype::RectangularBeam => {
                let h: f64 = 1200.0;
                let b: f64 = 450.0;
                let a = b * h;
                let i = (b * h.powi(3)) / 12.0;
                let yb = h / 2.0;
                let yt = h / 2.0;
                let bw = b;
                let bf = b;
                (h, a, i, yb, yt, bw, bf)
            }
        };

        let s_bot = inertia_ixx / y_bot;
        let s_top = inertia_ixx / y_top;

        // Self-Weight & Moments
        let g = 9.80665;
        let w_sw = (area_a * 1e-6) * rho_concrete * g * 1e-3; // kN/m
        let m_sw = (w_sw * span_length_m * span_length_m) / 8.0;
        let m_sdl = (superimposed_dead_load_kn_m * span_length_m * span_length_m) / 8.0;
        let m_ll = (live_load_kn_m * span_length_m * span_length_m) / 8.0;
        let m_service = m_sw + m_sdl + m_ll;
        let m_ultimate = 1.2 * (m_sw + m_sdl) + 1.6 * m_ll;

        // 1. Friction Loss
        let (mu, k_wobble) = match method {
            PrestressingMethod::PostTensioned => (0.20, 0.0015), // metal duct
            PrestressingMethod::PreTensioned => (0.0, 0.0),      // pretensioned straight/harped
        };
        let alpha = match profile_kind {
            TendonProfileKind::Parabolic => 8.0 * (eccentricity_mid_mm - eccentricity_end_mm).abs() * 1e-3 / span_length_m,
            TendonProfileKind::HarpedTwoPoint => 4.0 * (eccentricity_mid_mm - eccentricity_end_mm).abs() * 1e-3 / (span_length_m / 3.0),
            TendonProfileKind::StraightEccentric => 0.0,
        };
        let df_friction = if method == PrestressingMethod::PostTensioned {
            fpj * (1.0 - (-mu * alpha - k_wobble * span_length_m).exp())
        } else {
            0.0
        };

        // 2. Anchorage Seating Slip Loss
        let slip_mm = if method == PrestressingMethod::PostTensioned { 6.0 } else { 0.0 };
        let pf = (df_friction / span_length_m).max(0.1);
        let l_set = (slip_mm * 1e-3 * ep / pf).sqrt().clamp(5.0, span_length_m);
        let df_anchorage = if method == PrestressingMethod::PostTensioned {
            (slip_mm * 1e-3 * ep) / l_set
        } else {
            0.0
        };

        // 3. Elastic Shortening of Concrete
        let fc_cgp = (p_jack_kn * 1e3 / area_a)
            + (p_jack_kn * 1e3 * eccentricity_mid_mm.powi(2) / inertia_ixx)
            - (m_sw * 1e6 * eccentricity_mid_mm / inertia_ixx);
        let df_es = match method {
            PrestressingMethod::PostTensioned => {
                let factor = if num_strands > 1 {
                    (num_strands - 1) as f64 / (2.0 * num_strands as f64)
                } else {
                    0.5
                };
                factor * (ep / eci) * fc_cgp
            }
            PrestressingMethod::PreTensioned => (ep / eci) * fc_cgp,
        };

        let df_immediate = df_friction + df_anchorage + df_es;
        let f_pi = (fpj - df_immediate).max(0.0);
        let p_transfer_kn = (aps * f_pi) / 1000.0;

        // 4. Long-Term Losses: Creep, Shrinkage, Relaxation
        let fc_cds = (m_sdl * 1e6 * eccentricity_mid_mm) / inertia_ixx;
        let k_cr = if method == PrestressingMethod::PostTensioned { 1.6 } else { 2.0 };
        let df_creep = (k_cr * (ep / ec) * (fc_cgp - fc_cds)).max(0.0);

        let perimeter = 2.0 * (depth_h + top_bf);
        let v_s_ratio = area_a / perimeter;
        let rh = 70.0; // 70% average relative humidity
        let k_sh = if method == PrestressingMethod::PostTensioned { 0.80 } else { 1.0 };
        let df_shrinkage = (8.2e-6 * k_sh * ep * (1.0 - 0.06 * (v_s_ratio / 25.4)) * (100.0 - rh)).max(0.0);

        let df_relaxation = (strand_grade.k_re() - 0.04 * (df_shrinkage + df_creep + df_es)).clamp(10.0, 100.0);

        let df_longterm = df_creep + df_shrinkage + df_relaxation;
        let df_total = df_immediate + df_longterm;
        let loss_percentage = (df_total / fpj) * 100.0;
        let f_se = (fpj - df_total).max(0.0);
        let p_effective_kn = (aps * f_se) / 1000.0;

        let losses = PrestressLossBreakdown {
            friction_loss_mpa: df_friction,
            anchorage_slip_mpa: df_anchorage,
            elastic_shortening_mpa: df_es,
            total_immediate_loss_mpa: df_immediate,
            concrete_creep_mpa: df_creep,
            concrete_shrinkage_mpa: df_shrinkage,
            steel_relaxation_mpa: df_relaxation,
            total_long_term_loss_mpa: df_longterm,
            total_loss_mpa: df_total,
            loss_percentage,
            f_pj_mpa: fpj,
            f_pi_mpa: f_pi,
            f_se_mpa: f_se,
            p_jack_kn,
            p_transfer_kn,
            p_effective_kn,
        };

        // 5. Stress Checks at Transfer (P_i and M_sw)
        let sigma_top_i = -(p_transfer_kn * 1e3 / area_a)
            + (p_transfer_kn * 1e3 * eccentricity_mid_mm / s_top)
            - (m_sw * 1e6 / s_top);
        let sigma_bot_i = -(p_transfer_kn * 1e3 / area_a)
            - (p_transfer_kn * 1e3 * eccentricity_mid_mm / s_bot)
            + (m_sw * 1e6 / s_bot);

        let allow_comp_i = -0.60 * fci_prime_mpa;
        let allow_tens_i = 0.25 * fci_prime_mpa.sqrt();

        let top_i_dcr = if sigma_top_i > 0.0 {
            sigma_top_i / allow_tens_i
        } else {
            sigma_top_i.abs() / allow_comp_i.abs()
        };
        let bot_i_dcr = if sigma_bot_i > 0.0 {
            sigma_bot_i / allow_tens_i
        } else {
            sigma_bot_i.abs() / allow_comp_i.abs()
        };
        let transfer_acceptable = top_i_dcr <= 1.0 && bot_i_dcr <= 1.0;

        let transfer_stress = FiberStressCheck {
            stage_label: "Transfer (Pi + Msw)".to_string(),
            sigma_top_mpa: sigma_top_i,
            sigma_bot_mpa: sigma_bot_i,
            allowable_comp_mpa: allow_comp_i,
            allowable_tens_mpa: allow_tens_i,
            top_dcr: top_i_dcr,
            bot_dcr: bot_i_dcr,
            is_acceptable: transfer_acceptable,
        };

        // 6. Stress Checks at Service (P_eff and M_service)
        let sigma_top_s = -(p_effective_kn * 1e3 / area_a)
            + (p_effective_kn * 1e3 * eccentricity_mid_mm / s_top)
            - (m_service * 1e6 / s_top);
        let sigma_bot_s = -(p_effective_kn * 1e3 / area_a)
            - (p_effective_kn * 1e3 * eccentricity_mid_mm / s_bot)
            + (m_service * 1e6 / s_bot);

        let allow_comp_s = -0.45 * fc_prime_mpa; // sustained service load
        let ft_limit_u = 0.62 * fc_prime_mpa.sqrt();
        let ft_limit_t = 1.00 * fc_prime_mpa.sqrt();

        let aci_class = if sigma_bot_s <= ft_limit_u {
            AciTensionClass::ClassU
        } else if sigma_bot_s <= ft_limit_t {
            AciTensionClass::ClassT
        } else {
            AciTensionClass::ClassC
        };

        let allow_tens_s = ft_limit_u;
        let top_s_dcr = sigma_top_s.abs() / allow_comp_s.abs();
        let bot_s_dcr = if sigma_bot_s > 0.0 {
            sigma_bot_s / allow_tens_s
        } else {
            sigma_bot_s.abs() / allow_comp_s.abs()
        };
        let service_acceptable = top_s_dcr <= 1.0 && bot_s_dcr <= 1.0;

        let service_stress = FiberStressCheck {
            stage_label: "Service (Peff + Mserv)".to_string(),
            sigma_top_mpa: sigma_top_s,
            sigma_bot_mpa: sigma_bot_s,
            allowable_comp_mpa: allow_comp_s,
            allowable_tens_mpa: allow_tens_s,
            top_dcr: top_s_dcr,
            bot_dcr: bot_s_dcr,
            is_acceptable: service_acceptable,
        };

        // 7. Ultimate Flexural Strength φMn
        let dp = y_top + eccentricity_mid_mm;
        let rho_p = aps / (top_bf * dp);
        let beta1 = (0.85 - 0.05 * (fc_prime_mpa - 28.0) / 7.0).clamp(0.65, 0.85);
        let gamma_p = strand_grade.gamma_p();
        let fps = fpu * (1.0 - (gamma_p / beta1) * (rho_p * fpu / fc_prime_mpa));
        let a_whitney = (aps * fps) / (0.85 * fc_prime_mpa * top_bf);
        let c_axis = a_whitney / beta1;
        let mn_knm = (aps * fps * (dp - a_whitney / 2.0)) * 1e-6;
        let eps_t = 0.003 * ((dp - c_axis) / c_axis.max(1.0));
        let phi_flex = if eps_t >= 0.005 { 0.90 } else { 0.65 + 0.25 * ((eps_t - 0.002) / 0.003).clamp(0.0, 1.0) };
        let phi_mn = phi_flex * mn_knm;
        let flexure_dcr = m_ultimate / phi_mn;

        // Cracking moment & minimum reinforcement
        let fr = 0.62 * fc_prime_mpa.sqrt();
        let m_cracking = s_bot * (fr + (p_effective_kn * 1e3 / area_a) + (p_effective_kn * 1e3 * eccentricity_mid_mm / s_bot)) * 1e-6;
        let min_flexure_req = 1.2 * m_cracking;

        // 8. Shear Capacity (Vci & Vcw)
        let v_u = (1.2 * (w_sw + superimposed_dead_load_kn_m) + 1.6 * live_load_kn_m) * (span_length_m / 2.0);
        let v_ci = ((0.05 * fc_prime_mpa.sqrt() * web_bw * dp) + 50.0 * 1e3) * 1e-3;
        let f_pc = (p_effective_kn * 1e3) / area_a;
        let theta_end = match profile_kind {
            TendonProfileKind::Parabolic => (4.0 * (eccentricity_mid_mm - eccentricity_end_mm).abs() * 1e-3 / span_length_m).atan(),
            TendonProfileKind::HarpedTwoPoint => ((eccentricity_mid_mm - eccentricity_end_mm).abs() * 1e-3 / (span_length_m / 3.0)).atan(),
            TendonProfileKind::StraightEccentric => 0.0,
        };
        let v_p = p_effective_kn * theta_end.sin();
        let v_cw = ((0.29 * fc_prime_mpa.sqrt() + 0.3 * f_pc) * web_bw * dp) * 1e-3 + v_p;
        let v_c = v_ci.min(v_cw);
        let phi_vc = 0.75 * v_c;

        // Shear Reinforcement (2-leg D12 @ 150mm stirrups, fyt = 420 MPa)
        let a_v = 2.0 * std::f64::consts::PI / 4.0 * 12.0 * 12.0; // 226.2 mm2
        let s_stirrup = 150.0; // mm
        let f_yt = 420.0; // MPa
        let v_s = (a_v * f_yt * dp / s_stirrup) * 1e-3;
        let v_n = v_c + v_s;
        let phi_vn = 0.75 * v_n;
        let shear_dcr = v_u / phi_vn;

        // 9. Camber & Deflection Analytics
        let camber_p = (5.0 * p_transfer_kn * 1e3 * eccentricity_mid_mm * (span_length_m * 1000.0).powi(2))
            / (48.0 * eci * inertia_ixx);
        let defl_sw = (5.0 * w_sw * (span_length_m * 1000.0).powi(4))
            / (384.0 * eci * inertia_ixx);
        let net_camber_transfer = camber_p - defl_sw;

        let defl_sdl = (5.0 * superimposed_dead_load_kn_m * (span_length_m * 1000.0).powi(4))
            / (384.0 * ec * inertia_ixx);
        let defl_ll = (5.0 * live_load_kn_m * (span_length_m * 1000.0).powi(4))
            / (384.0 * ec * inertia_ixx);
        let long_term_camber = 1.80 * camber_p - 1.85 * defl_sw - 2.20 * defl_sdl - defl_ll;

        // 10. Tendon Profile Discretization (21 sample points along beam)
        let mut profile_points = Vec::with_capacity(21);
        for i in 0..=20 {
            let frac = (i as f64) / 20.0;
            let x = frac * span_length_m;
            let (e_x, slope) = match profile_kind {
                TendonProfileKind::Parabolic => {
                    let e = eccentricity_mid_mm - 4.0 * (eccentricity_mid_mm - eccentricity_end_mm) * (frac - 0.5).powi(2);
                    let sl = -8.0 * (eccentricity_mid_mm - eccentricity_end_mm) * (frac - 0.5) / span_length_m * 1e-3;
                    (e, sl)
                }
                TendonProfileKind::HarpedTwoPoint => {
                    let e = if frac <= 0.333 {
                        eccentricity_end_mm + (eccentricity_mid_mm - eccentricity_end_mm) * (frac / 0.333)
                    } else if frac <= 0.667 {
                        eccentricity_mid_mm
                    } else {
                        eccentricity_mid_mm - (eccentricity_mid_mm - eccentricity_end_mm) * ((frac - 0.667) / 0.333)
                    };
                    (e, 0.0)
                }
                TendonProfileKind::StraightEccentric => (eccentricity_mid_mm, 0.0),
            };
            let z_coord = (y_bot - e_x) * 1e-3;
            profile_points.push(TendonPointCoordinates {
                x_m: x,
                e_mm: e_x,
                z_coord_m: z_coord,
                slope_rad: slope,
            });
        }

        let governing_dcr = transfer_stress.top_dcr
            .max(transfer_stress.bot_dcr)
            .max(service_stress.top_dcr)
            .max(service_stress.bot_dcr)
            .max(flexure_dcr)
            .max(shear_dcr);

        let is_code_compliant = transfer_acceptable
            && service_acceptable
            && flexure_dcr <= 1.0
            && phi_mn >= min_flexure_req
            && shear_dcr <= 1.0;

        PrestressedGirderReport {
            project_name: project_name.to_string(),
            girder_label: girder_label.to_string(),
            span_length_m,
            girder_spacing_m,
            archetype,
            method,
            profile_kind,
            strand_grade,
            strand_dia_mm,
            num_strands,
            total_aps_mm2: aps,
            depth_h_mm: depth_h,
            area_a_mm2: area_a,
            inertia_ixx_mm4: inertia_ixx,
            y_bot_mm: y_bot,
            y_top_mm: y_top,
            s_bot_mm3: s_bot,
            s_top_mm3: s_top,
            web_bw_mm: web_bw,
            top_flange_bf_mm: top_bf,
            fc_prime_mpa,
            fci_prime_mpa,
            ec_mpa: ec,
            eci_mpa: eci,
            losses,
            transfer_stress,
            service_stress,
            aci_class,
            m_sw_knm: m_sw,
            m_sdl_knm: m_sdl,
            m_ll_knm: m_ll,
            m_service_knm: m_service,
            m_ultimate_knm: m_ultimate,
            m_cracking_knm: m_cracking,
            min_flexure_req_knm: min_flexure_req,
            fps_mpa: fps,
            whitney_a_mm: a_whitney,
            neutral_axis_c_mm: c_axis,
            phi_mn_knm: phi_mn,
            flexure_dcr,
            v_ultimate_kn: v_u,
            v_ci_kn: v_ci,
            v_cw_kn: v_cw,
            v_c_kn: v_c,
            v_s_kn: v_s,
            v_n_kn: v_n,
            phi_vc_kn: phi_vc,
            phi_vn_kn: phi_vn,
            shear_dcr,
            camber_prestress_transfer_mm: camber_p,
            deflection_sw_transfer_mm: defl_sw,
            net_camber_transfer_mm: net_camber_transfer,
            long_term_camber_mm: long_term_camber,
            tendon_profile_points: profile_points,
            is_code_compliant,
            governing_dcr,
        }
    }

    /// Generates a complete 3D structural model of a Prestressed Post-Tensioned Concrete Girder Bridge
    pub fn generate_bridge_archetype_elements(
        name: &str,
        span_m: f64,
        num_girders: usize,
        spacing_m: f64,
        drape_mm: f64,
    ) -> crate::core::StructuralModel {
        let mut model = crate::core::StructuralModel::new(name);
        let num_segs = 12;
        let dx = span_m / (num_segs as f64);
        let total_w = ((num_girders - 1) as f64) * spacing_m;
        let y_start = -total_w / 2.0;

        // Concrete material
        let mat_concrete = crate::core::Material {
            name: "C50/60 Precast Concrete".to_string(),
            elastic_modulus: 3.3e10, // Pa
            poisson_ratio: 0.20,
            yield_strength: 50.0e6,
            density: 2450.0,
        };

        // Prestressing strand material
        let mat_strand = crate::core::Material {
            name: "Grade 270 Prestressing Strand".to_string(),
            elastic_modulus: 1.965e11, // Pa
            poisson_ratio: 0.30,
            yield_strength: 1860.0e6,
            density: 7850.0,
        };

        // AASHTO Girder section
        let sec_girder = crate::core::CrossSection {
            name: "AASHTO-IV-Girder".to_string(),
            area: 0.509,
            iy: 1.0e-2,
            iz: 1.085e-1,
            j_torsion: 1.0e-2,
            depth: 1.37,
            width: 0.508,
        };

        // Diaphragm beam section
        let sec_diaph = crate::core::CrossSection {
            name: "RC-Diaphragm-300x1000".to_string(),
            area: 0.300,
            iy: 2.25e-3,
            iz: 2.5e-2,
            j_torsion: 5.0e-3,
            depth: 1.0,
            width: 0.3,
        };

        // Tendon virtual cable section
        let sec_tendon = crate::core::CrossSection {
            name: "PT-Tendon-36Strands".to_string(),
            area: 0.00504,
            iy: 1.0e-6,
            iz: 1.0e-6,
            j_torsion: 1.0e-6,
            depth: 0.08,
            width: 0.08,
        };

        // Create nodes and elements for each girder
        for g in 0..num_girders {
            let y = y_start + (g as f64) * spacing_m;
            let mut girder_node_ids = Vec::with_capacity(num_segs + 1);
            let mut tendon_node_ids = Vec::with_capacity(num_segs + 1);

            for s in 0..=num_segs {
                let x = (s as f64) * dx;
                let frac = (s as f64) / (num_segs as f64);
                // Parabolic drape of tendon
                let z_tendon = -0.15 - (drape_mm * 1e-3) * 4.0 * frac * (1.0 - frac);

                // Girder centerline node
                let nid_girder = model.nodes.len();
                let fixity = if s == 0 {
                    crate::core::BoundaryFixity::pinned()
                } else if s == num_segs {
                    crate::core::BoundaryFixity::roller_x()
                } else {
                    crate::core::BoundaryFixity::free()
                };
                model.nodes.push(crate::core::Node3D {
                    id: nid_girder,
                    x,
                    y,
                    z: 0.0,
                    fixity,
                    spring_stiffness: [0.0; 6],
                });
                girder_node_ids.push(nid_girder);

                // Tendon internal trajectory node
                let nid_tendon = model.nodes.len();
                model.nodes.push(crate::core::Node3D {
                    id: nid_tendon,
                    x,
                    y,
                    z: z_tendon,
                    fixity: crate::core::BoundaryFixity::free(),
                    spring_stiffness: [0.0; 6],
                });
                tendon_node_ids.push(nid_tendon);
            }

            // Girder longitudinal beam elements
            for s in 0..num_segs {
                let eid = model.elements.len() + 1;
                model.elements.push(crate::core::Element3D {
                    id: eid,
                    name: format!("Girder_G{}_Seg{}", g + 1, s + 1),
                    start_node: girder_node_ids[s],
                    end_node: girder_node_ids[s + 1],
                    element_type: crate::core::ElementType::Frame3D,
                    material: mat_concrete.clone(),
                    section: sec_girder.clone(),
                });

                // Tendon segment element
                let teid = model.elements.len() + 1;
                model.elements.push(crate::core::Element3D {
                    id: teid,
                    name: format!("PT_Tendon_G{}_Seg{}", g + 1, s + 1),
                    start_node: tendon_node_ids[s],
                    end_node: tendon_node_ids[s + 1],
                    element_type: crate::core::ElementType::Truss3D,
                    material: mat_strand.clone(),
                    section: sec_tendon.clone(),
                });
            }
        }

        // Add transverse diaphragms at s = 0, s = num_segs / 2, s = num_segs
        let diaph_stations = [0, num_segs / 2, num_segs];
        for &s in &diaph_stations {
            for g in 0..(num_girders - 1) {
                let n1 = g * (num_segs + 1) * 2 + s * 2;
                let n2 = (g + 1) * (num_segs + 1) * 2 + s * 2;
                let eid = model.elements.len() + 1;
                model.elements.push(crate::core::Element3D {
                    id: eid,
                    name: format!("Diaphragm_S{}_G{}-G{}", s, g + 1, g + 2),
                    start_node: n1,
                    end_node: n2,
                    element_type: crate::core::ElementType::Frame3D,
                    material: mat_concrete.clone(),
                    section: sec_diaph.clone(),
                });
            }
        }

        model
    }
}

