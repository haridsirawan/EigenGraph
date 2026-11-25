//! core/reliability_studio.rs — Update `v0.8.4.7` (`7/10` -> `v0.8.5`):
//! Structural Reliability (`FORM/SORM` Beta-Index, Sensitivity Factors & FEMA P-58 Fragility)
//! 3D Studio (`ASCE 7-22` / `EN 1990` / `ISO 2394`).
//!
//! Couples live `BimProject` columns/levels and `SiteEnvironmentalCoupling` seismic hazard into
//! Rackwitz-Fiessler Hasofer-Lind First-Order Reliability Method (`FORM`), Breitung curvature-
//! corrected Second-Order Reliability Method (`SORM`), Log-Normal `FOSM`, importance sensitivity
//! tornado (`alpha^2`), multi-state FEMA P-58 fragility curves, 3D ENU member risk contours + beta
//! hotspot beacons, and non-destructive `eigengraph::analysis::reliability_form` IFCX overlays.

use serde::{Deserialize, Serialize};

use crate::core::bim::BimProject;
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
use crate::core::reliability::{standard_normal_cdf, ReliabilityEngine};

/// Canonical EigenGraph IFCX attribute namespace for Structural Reliability FORM/SORM overlays.
pub const NS_EG_RELIABILITY_FORM: &str = "eigengraph::analysis::reliability_form";

/// Selected probabilistic reliability solver order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReliabilityMethod {
    FormHasoferLind,
    SormBreitung,
    FosmLogNormal,
}

impl ReliabilityMethod {
    pub fn label(self) -> &'static str {
        match self {
            Self::FormHasoferLind => "FORM (Hasofer-Lind)",
            Self::SormBreitung => "SORM (Breitung)",
            Self::FosmLogNormal => "FOSM (Log-Normal)",
        }
    }
}

/// Governing structural limit-state mode evaluated in the Reliability Studio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LimitStateKind {
    FlexurePlastic,
    AxialColumn,
    InterstoryDrift,
    ShearBrittle,
}

impl LimitStateKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::FlexurePlastic => "Plastic Flexure",
            Self::AxialColumn => "Axial-Moment Column",
            Self::InterstoryDrift => "Seismic Drift",
            Self::ShearBrittle => "Brittle Shear",
        }
    }
}

/// Persisted configuration for the Structural Reliability `FORM/SORM` 3D Studio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReliabilityStudioConfig {
    pub method: ReliabilityMethod,
    pub limit_state: LimitStateKind,
    pub target_beta: f64,
    pub resistance_cov: f64,
    pub dead_load_cov: f64,
    pub live_load_cov: f64,
    pub seismic_load_cov: f64,
    pub demand_to_capacity_ratio: f64,
    pub live_to_dead_ratio: f64,
    pub seismic_fraction: f64,
}

impl Default for ReliabilityStudioConfig {
    fn default() -> Self {
        Self {
            method: ReliabilityMethod::SormBreitung,
            limit_state: LimitStateKind::FlexurePlastic,
            target_beta: 3.5,
            resistance_cov: 0.11,
            dead_load_cov: 0.10,
            live_load_cov: 0.25,
            seismic_load_cov: 0.35,
            demand_to_capacity_ratio: 0.68,
            live_to_dead_ratio: 0.55,
            seismic_fraction: 0.28,
        }
    }
}

/// Hasofer-Lind directional cosine and variance importance at the Most Probable Failure Point (MPP).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensitivityFactor {
    pub variable_name: String,
    pub symbol: String,
    pub alpha: f64,
    pub importance_pct: f64,
    pub design_point_u: f64,
    pub design_point_x: f64,
}

/// Serializable FEMA P-58 / ASCE 41 multi-state seismic fragility curve point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FragilityPointDto {
    pub pga_g: f64,
    pub prob_slight: f64,
    pub prob_moderate: f64,
    pub prob_extensive: f64,
    pub prob_complete_collapse: f64,
}

/// 3D ENU structural member annotated with local reliability index `beta`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReliabilityMember3D {
    pub member_id: String,
    pub p0_enu_m: [f64; 3],
    pub p1_enu_m: [f64; 3],
    pub beta_index: f64,
    pub failure_probability: f64,
    pub dcr: f64,
    pub is_below_target: bool,
}

/// 3D ENU critical reliability hotspot beacon marking low-`beta` members.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BetaHotspot3D {
    pub label: String,
    pub pos_enu_m: [f64; 3],
    pub beta_index: f64,
    pub is_critical: bool,
}

/// Compact serializable IFCX overlay payload under `eigengraph::analysis::reliability_form`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReliabilityIfcxOverlay {
    pub schema_version: String,
    pub method: String,
    pub limit_state: String,
    pub target_beta: f64,
    pub fosm_beta: f64,
    pub form_beta: f64,
    pub sorm_beta: f64,
    pub active_beta: f64,
    pub failure_probability: f64,
    pub curvature_kappa: f64,
    pub dominant_variable: String,
    pub dominant_importance_pct: f64,
    pub below_target_count: usize,
    pub total_members: usize,
    pub is_overall_compliant: bool,
    pub summary_verdict: String,
}

/// Unified Structural Reliability `FORM/SORM` & FEMA P-58 Fragility Studio Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReliabilityStudioReport {
    pub config: ReliabilityStudioConfig,
    pub fosm_beta: f64,
    pub fosm_pf: f64,
    pub form_beta: f64,
    pub form_pf: f64,
    pub sorm_beta: f64,
    pub sorm_pf: f64,
    pub active_beta: f64,
    pub active_pf: f64,
    pub curvature_kappa: f64,
    pub iterations: usize,
    pub sensitivities: Vec<SensitivityFactor>,
    pub fragility_curve: Vec<FragilityPointDto>,
    pub members_3d: Vec<ReliabilityMember3D>,
    pub hotspots_3d: Vec<BetaHotspot3D>,
    pub min_member_beta: f64,
    pub below_target_count: usize,
    pub is_overall_compliant: bool,
    pub summary_verdict: String,
}

impl ReliabilityStudioReport {
    /// Converts the reliability studio report into a compact non-destructive `.ifcx` overlay payload.
    pub fn to_ifcx_overlay(&self) -> ReliabilityIfcxOverlay {
        let dominant = self
            .sensitivities
            .iter()
            .max_by(|a, b| {
                a.importance_pct
                    .partial_cmp(&b.importance_pct)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        ReliabilityIfcxOverlay {
            schema_version: "reliability-form-ifcx-v1".to_string(),
            method: self.config.method.label().to_string(),
            limit_state: self.config.limit_state.label().to_string(),
            target_beta: self.config.target_beta,
            fosm_beta: self.fosm_beta,
            form_beta: self.form_beta,
            sorm_beta: self.sorm_beta,
            active_beta: self.active_beta,
            failure_probability: self.active_pf,
            curvature_kappa: self.curvature_kappa,
            dominant_variable: dominant
                .map(|s| s.variable_name.clone())
                .unwrap_or_else(|| "Resistance (R)".to_string()),
            dominant_importance_pct: dominant.map(|s| s.importance_pct).unwrap_or(0.0),
            below_target_count: self.below_target_count,
            total_members: self.members_3d.len(),
            is_overall_compliant: self.is_overall_compliant,
            summary_verdict: self.summary_verdict.clone(),
        }
    }
}

/// High-accuracy Standard Normal Inverse CDF `Phi^{-1}(p)` using Peter Acklam's rational approximation.
pub fn standard_normal_inv_cdf(p: f64) -> f64 {
    let p_clamped = p.clamp(1e-15, 1.0 - 1e-15);
    const A1: f64 = -3.969683028665376e+01;
    const A2: f64 = 2.209460984245205e+02;
    const A3: f64 = -2.759285104469687e+02;
    const A4: f64 = 1.383577518672690e+02;
    const A5: f64 = -3.066479806614716e+01;
    const A6: f64 = 2.506628277459239e+00;

    const B1: f64 = -5.447609879822406e+01;
    const B2: f64 = 1.615858368580409e+02;
    const B3: f64 = -1.556989798598866e+02;
    const B4: f64 = 6.680131188771972e+01;
    const B5: f64 = -1.328068155288572e+01;

    const C1: f64 = -7.784894002430293e-03;
    const C2: f64 = -3.223964580411365e-01;
    const C3: f64 = -2.400758277161838e+00;
    const C4: f64 = -2.549732539343734e+00;
    const C5: f64 = 4.374664141464968e+00;
    const C6: f64 = 2.938163982698783e+00;

    const D1: f64 = 7.784695709041462e-03;
    const D2: f64 = 3.224671290700398e-01;
    const D3: f64 = 2.445134137142996e+00;
    const D4: f64 = 3.754408661907416e+00;

    const P_LOW: f64 = 0.02425;
    const P_HIGH: f64 = 1.0 - P_LOW;

    if p_clamped < P_LOW {
        let q = (-2.0 * p_clamped.ln()).sqrt();
        (((((C1 * q + C2) * q + C3) * q + C4) * q + C5) * q + C6)
            / ((((D1 * q + D2) * q + D3) * q + D4) * q + 1.0)
    } else if p_clamped <= P_HIGH {
        let q = p_clamped - 0.5;
        let r = q * q;
        (((((A1 * r + A2) * r + A3) * r + A4) * r + A5) * r + A6) * q
            / (((((B1 * r + B2) * r + B3) * r + B4) * r + B5) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p_clamped).ln()).sqrt();
        -(((((C1 * q + C2) * q + C3) * q + C4) * q + C5) * q + C6)
            / ((((D1 * q + D2) * q + D3) * q + D4) * q + 1.0)
    }
}

/// Computes Log-Normal distribution parameters `(lambda, zeta)` from `(mean, cov)`.
#[inline]
fn lognormal_params(mean: f64, cov: f64) -> (f64, f64) {
    let safe_m = mean.max(1e-6);
    let zeta = (1.0 + cov * cov).ln().sqrt().max(1e-6);
    let lambda = safe_m.ln() - 0.5 * zeta * zeta;
    (lambda, zeta)
}

/// Evaluates Structural Reliability (`FOSM`, `FORM` HL-RF, `SORM` Breitung), Hasofer-Lind
/// sensitivity importance factors, FEMA P-58 fragility curves, and 3D ENU member risk contours
/// coupled to live `BimProject` columns/levels and `SiteEnvironmentalCoupling`.
pub fn evaluate_reliability_studio(
    bim: &BimProject,
    site: Option<&SiteEnvironmentalCoupling>,
    config: &ReliabilityStudioConfig,
) -> ReliabilityStudioReport {
    let mut cfg = config.clone();
    cfg.target_beta = cfg.target_beta.clamp(2.0, 5.5);
    cfg.resistance_cov = cfg.resistance_cov.clamp(0.05, 0.30);
    cfg.dead_load_cov = cfg.dead_load_cov.clamp(0.04, 0.25);
    cfg.live_load_cov = cfg.live_load_cov.clamp(0.10, 0.45);
    cfg.seismic_load_cov = cfg.seismic_load_cov.clamp(0.15, 0.65);
    cfg.demand_to_capacity_ratio = cfg.demand_to_capacity_ratio.clamp(0.25, 1.25);
    cfg.live_to_dead_ratio = cfg.live_to_dead_ratio.clamp(0.10, 2.0);
    cfg.seismic_fraction = cfg.seismic_fraction.clamp(0.0, 0.80);

    let (bias_r, cov_r_mult) = match cfg.limit_state {
        LimitStateKind::FlexurePlastic => (1.12, 1.00),
        LimitStateKind::AxialColumn => (1.08, 1.06),
        LimitStateKind::InterstoryDrift => (1.04, 1.12),
        LimitStateKind::ShearBrittle => (1.16, 1.18),
    };
    let v_r = (cfg.resistance_cov * cov_r_mult).clamp(0.05, 0.38);

    // Couple site seismic acceleration slightly into lateral fraction when active
    let site_seis_nudge = site
        .map(|s| (f64::from(s.seismic_pga_ss_g) * 0.14).clamp(0.0, 0.18))
        .unwrap_or(0.0);
    let eff_seis_frac = (cfg.seismic_fraction + site_seis_nudge).clamp(0.02, 0.80);

    // Normalized Limit-State Means: g(R, D, L, E) = R - (D + L + E), R_nominal = 1.0
    let mu_r = bias_r;
    let total_demand_mean = cfg.demand_to_capacity_ratio;
    let gravity_mean = total_demand_mean * (1.0 - eff_seis_frac);
    let mu_d = (gravity_mean / (1.0 + cfg.live_to_dead_ratio)).max(0.02);
    let mu_l = (gravity_mean - mu_d).max(0.01);
    let mu_e = (total_demand_mean - mu_d - mu_l).max(0.01);

    let (lam_r, zeta_r) = lognormal_params(mu_r, v_r);
    let sigma_d = (mu_d * cfg.dead_load_cov).max(1e-5);
    let (lam_l, zeta_l) = lognormal_params(mu_l, cfg.live_load_cov);
    let (lam_e, zeta_e) = lognormal_params(mu_e, cfg.seismic_load_cov);

    // 1. Baseline FOSM Log-Normal Index
    let var_s = (sigma_d * sigma_d)
        + (mu_l * cfg.live_load_cov).powi(2)
        + (mu_e * cfg.seismic_load_cov).powi(2);
    let v_s_eff = (var_s.sqrt() / (mu_d + mu_l + mu_e).max(1e-6)).clamp(0.04, 0.85);
    let (fosm_beta, fosm_pf) =
        ReliabilityEngine::compute_lognormal_beta(mu_r, mu_d + mu_l + mu_e, v_r, v_s_eff);

    // 2. Rackwitz-Fiessler Hasofer-Lind FORM Iteration
    let mut u = [0.0_f64; 4];
    let mut alpha = [-0.60_f64, 0.35, 0.45, 0.55];
    let mut iterations = 0_usize;
    let mut x_curr = [mu_r, mu_d, mu_l, mu_e];
    let mut last_norm = 1.0_f64;

    for iter in 1..=30 {
        iterations = iter;
        let x_r = (lam_r + zeta_r * u[0]).exp().max(1e-6);
        let x_d = (mu_d + sigma_d * u[1]).max(1e-5);
        let x_l = (lam_l + zeta_l * u[2]).exp().max(1e-6);
        let x_e = (lam_e + zeta_e * u[3]).exp().max(1e-6);
        x_curr = [x_r, x_d, x_l, x_e];

        let g_val = x_r - (x_d + x_l + x_e);
        let sig_r_eq = x_r * zeta_r;
        let sig_l_eq = x_l * zeta_l;
        let sig_e_eq = x_e * zeta_e;

        let grad_u = [sig_r_eq, -sigma_d, -sig_l_eq, -sig_e_eq];
        let norm = grad_u.iter().map(|v| v * v).sum::<f64>().sqrt().max(1e-9);
        last_norm = norm;
        for k in 0..4 {
            alpha[k] = -grad_u[k] / norm;
        }
        let dot_au: f64 = (0..4).map(|k| alpha[k] * u[k]).sum();
        let coeff = dot_au + g_val / norm;

        let mut max_delta = 0.0_f64;
        for k in 0..4 {
            let target_uk = (alpha[k] * coeff).clamp(-9.0, 9.0);
            let next_uk = 0.25 * u[k] + 0.75 * target_uk;
            max_delta = max_delta.max((next_uk - u[k]).abs());
            u[k] = next_uk;
        }
        if max_delta < 1e-6 {
            break;
        }
    }

    let u_norm = u.iter().map(|v| v * v).sum::<f64>().sqrt();
    let g_origin = mu_r - (mu_d + mu_l + mu_e);
    let sign = if g_origin >= 0.0 { 1.0 } else { -1.0 };
    let form_beta = (u_norm * sign).clamp(-4.0, 9.0);
    let form_pf = standard_normal_cdf(-form_beta);

    // 3. SORM Breitung Asymptotic Curvature Correction
    // Exact Hessian contribution of log-normal transforms at the design point projected onto tangent hyperplane
    let kappa_eff = ((1.0 - alpha[0] * alpha[0]) * zeta_r * zeta_r * x_curr[0]
        - (1.0 - alpha[2] * alpha[2]) * zeta_l * zeta_l * x_curr[2]
        - (1.0 - alpha[3] * alpha[3]) * zeta_e * zeta_e * x_curr[3])
        / last_norm.max(1e-6);
    let curvature_kappa = kappa_eff.clamp(-0.65, 0.85);
    let breitung_factor = (1.0 + form_beta.max(0.0) * curvature_kappa).clamp(0.35, 3.5);
    let sorm_pf = if form_beta > 0.0 {
        (form_pf * breitung_factor.powf(-0.5)).clamp(1e-15, 0.999999)
    } else {
        form_pf
    };
    let sorm_beta = (-standard_normal_inv_cdf(sorm_pf)).clamp(-4.0, 9.0);

    let (active_beta, active_pf) = match cfg.method {
        ReliabilityMethod::FormHasoferLind => (form_beta, form_pf),
        ReliabilityMethod::SormBreitung => (sorm_beta, sorm_pf),
        ReliabilityMethod::FosmLogNormal => (fosm_beta, fosm_pf),
    };

    // Normalize alpha^2 importance strictly to 100.0%
    let alpha_sq_sum = alpha.iter().map(|a| a * a).sum::<f64>().max(1e-9);
    let var_meta = [
        ("Resistance (R)", "R"),
        ("Dead Load (D)", "D"),
        ("Live Load (L)", "L"),
        ("Lateral Load (E)", "E"),
    ];
    let sensitivities: Vec<SensitivityFactor> = (0..4)
        .map(|k| SensitivityFactor {
            variable_name: var_meta[k].0.to_string(),
            symbol: var_meta[k].1.to_string(),
            alpha: alpha[k],
            importance_pct: (alpha[k] * alpha[k] / alpha_sq_sum) * 100.0,
            design_point_u: u[k],
            design_point_x: x_curr[k],
        })
        .collect();

    // 4. FEMA P-58 Multi-State Fragility Curve
    let eigengate_proxy = (active_beta / 4.2).clamp(0.32, 1.0);
    let fragility_curve = ReliabilityEngine::generate_fragility_curve(eigengate_proxy, 12)
        .into_iter()
        .map(|pt| FragilityPointDto {
            pga_g: pt.pga_g,
            prob_slight: pt.prob_slight,
            prob_moderate: pt.prob_moderate,
            prob_extensive: pt.prob_extensive,
            prob_complete_collapse: pt.prob_complete_collapse,
        })
        .collect();

    // 5. 3D ENU Structural Member Reliability & Hotspot Beacons
    let mut elevs_m: Vec<f64> = bim
        .levels
        .iter()
        .map(|l| l.elevation_mm * 0.001)
        .collect();
    elevs_m.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    elevs_m.dedup_by(|a, b| (*a - *b).abs() < 0.25);
    if elevs_m.len() < 2 {
        elevs_m = vec![0.0, 3.8, 7.6];
    }
    let h_max_m = (*elevs_m.last().unwrap_or(&7.6)).max(3.8);

    let mut members_3d = Vec::new();
    let method_delta = active_beta - fosm_beta;

    if bim.columns.is_empty() {
        let coords = [[0.0, 0.0], [6.0, 0.0], [6.0, 6.0], [0.0, 6.0]];
        for (ci, [cx, cy]) in coords.iter().enumerate() {
            for si in 0..(elevs_m.len() - 1).min(2) {
                let z0 = elevs_m[si];
                let z1 = elevs_m[si + 1];
                let height_ratio = z0 / h_max_m;
                let dcr_m = (cfg.demand_to_capacity_ratio
                    * (1.06 - 0.14 * height_ratio + (ci as f64) * 0.025))
                    .clamp(0.22, 1.30);
                let (b_raw, _) = ReliabilityEngine::compute_lognormal_beta(bias_r, dcr_m, v_r, v_s_eff);
                let beta_m = (b_raw + method_delta).clamp(-3.0, 8.5);
                let pf_m = standard_normal_cdf(-beta_m);
                members_3d.push(ReliabilityMember3D {
                    member_id: format!("C{}-L{}", ci + 1, si + 1),
                    p0_enu_m: [*cx, *cy, z0],
                    p1_enu_m: [*cx, *cy, z1],
                    beta_index: beta_m,
                    failure_probability: pf_m,
                    dcr: dcr_m,
                    is_below_target: beta_m < cfg.target_beta,
                });
            }
        }
    } else {
        for (ci, col) in bim.columns.iter().take(18).enumerate() {
            let x_m = col.pos.0 * 0.001;
            let y_m = col.pos.1 * 0.001;
            let z_base = bim
                .levels
                .iter()
                .find(|l| l.id == col.base_level_id)
                .map(|l| l.elevation_mm * 0.001)
                .unwrap_or(0.0);
            let z_top = bim
                .levels
                .iter()
                .find(|l| l.id == col.top_level_id)
                .map(|l| l.elevation_mm * 0.001)
                .unwrap_or(z_base + 3.8);
            let height_ratio = (z_base / h_max_m).clamp(0.0, 1.0);
            let ecc_factor = 1.0 + (((ci % 5) as f64) - 2.0) * 0.035;
            let dcr_m = (cfg.demand_to_capacity_ratio * (1.07 - 0.16 * height_ratio) * ecc_factor)
                .clamp(0.22, 1.30);
            let (b_raw, _) = ReliabilityEngine::compute_lognormal_beta(bias_r, dcr_m, v_r, v_s_eff);
            let beta_m = (b_raw + method_delta).clamp(-3.0, 8.5);
            let pf_m = standard_normal_cdf(-beta_m);
            members_3d.push(ReliabilityMember3D {
                member_id: col.name.clone(),
                p0_enu_m: [x_m, y_m, z_base],
                p1_enu_m: [x_m, y_m, z_top],
                beta_index: beta_m,
                failure_probability: pf_m,
                dcr: dcr_m,
                is_below_target: beta_m < cfg.target_beta,
            });
        }
    }

    // Add representative roof/floor girders spanning between adjacent columns
    if members_3d.len() >= 2 {
        let count = members_3d.len().min(6);
        let z_deck = elevs_m.get(1).copied().unwrap_or(3.8);
        for i in 0..(count - 1) {
            let a = &members_3d[i];
            let b = &members_3d[i + 1];
            let span = (b.p0_enu_m[0] - a.p0_enu_m[0]).hypot(b.p0_enu_m[1] - a.p0_enu_m[1]);
            if (2.5..=16.0).contains(&span) {
                let dcr_g = (cfg.demand_to_capacity_ratio * 0.96).clamp(0.22, 1.25);
                let (b_raw, _) =
                    ReliabilityEngine::compute_lognormal_beta(bias_r, dcr_g, v_r, v_s_eff);
                let beta_g = (b_raw + method_delta).clamp(-3.0, 8.5);
                members_3d.push(ReliabilityMember3D {
                    member_id: format!("G-{}", i + 1),
                    p0_enu_m: [a.p0_enu_m[0], a.p0_enu_m[1], z_deck],
                    p1_enu_m: [b.p0_enu_m[0], b.p0_enu_m[1], z_deck],
                    beta_index: beta_g,
                    failure_probability: standard_normal_cdf(-beta_g),
                    dcr: dcr_g,
                    is_below_target: beta_g < cfg.target_beta,
                });
            }
        }
    }

    let min_member_beta = members_3d
        .iter()
        .map(|m| m.beta_index)
        .fold(active_beta, f64::min);
    let below_target_count = members_3d.iter().filter(|m| m.is_below_target).count();

    let mut sorted_idx: Vec<usize> = (0..members_3d.len()).collect();
    sorted_idx.sort_by(|&a, &b| {
        members_3d[a]
            .beta_index
            .partial_cmp(&members_3d[b].beta_index)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let hotspots_3d: Vec<BetaHotspot3D> = sorted_idx
        .into_iter()
        .take(6)
        .map(|idx| {
            let m = &members_3d[idx];
            BetaHotspot3D {
                label: format!("{} (b={:.2})", m.member_id, m.beta_index),
                pos_enu_m: [
                    0.5 * (m.p0_enu_m[0] + m.p1_enu_m[0]),
                    0.5 * (m.p0_enu_m[1] + m.p1_enu_m[1]),
                    0.5 * (m.p0_enu_m[2] + m.p1_enu_m[2]),
                ],
                beta_index: m.beta_index,
                is_critical: m.is_below_target,
            }
        })
        .collect();

    let is_overall_compliant = active_beta >= cfg.target_beta && below_target_count == 0;
    let summary_verdict = format!(
        "{} [{}]: beta={:.2} (target {:.2}) | Pf={:.2e} | FORM={:.2} / SORM={:.2} / FOSM={:.2} | {} below target [{}]",
        cfg.method.label(),
        cfg.limit_state.label(),
        active_beta,
        cfg.target_beta,
        active_pf,
        form_beta,
        sorm_beta,
        fosm_beta,
        below_target_count,
        if is_overall_compliant { "PASS" } else { "REVIEW" }
    );

    ReliabilityStudioReport {
        config: cfg,
        fosm_beta,
        fosm_pf,
        form_beta,
        form_pf,
        sorm_beta,
        sorm_pf,
        active_beta,
        active_pf,
        curvature_kappa,
        iterations,
        sensitivities,
        fragility_curve,
        members_3d,
        hotspots_3d,
        min_member_beta,
        below_target_count,
        is_overall_compliant,
        summary_verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bim::BimProject;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};

    #[test]
    fn test_phase84_7_reliability_form_sorm_sensitivity_and_fragility() {
        let bim = BimProject::sample_bimcraft_building();
        let rep = evaluate_reliability_studio(
            &bim,
            None,
            &ReliabilityStudioConfig {
                method: ReliabilityMethod::SormBreitung,
                limit_state: LimitStateKind::InterstoryDrift,
                target_beta: 3.5,
                demand_to_capacity_ratio: 0.65,
                ..ReliabilityStudioConfig::default()
            },
        );
        assert_eq!(rep.sensitivities.len(), 4);
        let imp_sum: f64 = rep.sensitivities.iter().map(|s| s.importance_pct).sum();
        assert!((imp_sum - 100.0).abs() < 0.5);
        assert!(rep.form_beta > 1.0);
        assert!(rep.sorm_beta.is_finite());
        assert!(rep.fosm_beta.is_finite());
        assert!(rep.active_pf > 0.0 && rep.active_pf < 1.0);
        assert!(!rep.fragility_curve.is_empty());
        assert!(!rep.members_3d.is_empty());
        assert!(!rep.hotspots_3d.is_empty());
    }

    #[test]
    fn test_phase84_7_reliability_ifcx_overlay_roundtrip() {
        let bim = BimProject::sample_bimcraft_building();
        let rep = evaluate_reliability_studio(&bim, None, &ReliabilityStudioConfig::default());
        let base = IfcxDocument::from_bim_project(&bim, None, 0.0, None, None, None);
        let composed = IfcxStage::compose_with_reliability_overlay(&base, &rep.to_ifcx_overlay());
        let json = composed.to_json_pretty();
        assert!(json.contains("eigengraph::analysis::reliability_form"));
        assert!(json.contains("/earth/analysis/reliability_form"));
        assert!(!json.contains("ARI"));
    }
}
