//! gpu_fragility_mc.rs — FEMA P-58 Monte Carlo Probabilistic Fragility Accelerator (`@compute` WGSL + CPU Parity).
//!
//! Upgrades single-point mean FEMA P-58 estimates into full probabilistic loss, downtime, embodied
//! carbon, collapse risk, and Arup REDi™ resilience tier distributions (`256..=16,384` realizations)
//! using deterministic PCG32 + Box–Muller lognormal EDP sampling in pure WGSL `@compute` shaders,
//! with an exact mathematical CPU fallback for headless CI and non-WebGPU environments.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::core::bim::BimProject;
use crate::core::fema_p58::REDiResilienceTier;
use crate::core::geotech::SubsurfaceSsiAnalysisReport;
use crate::core::gis::DistrictMultiHazardReport;
use crate::core::ground_motion_replay::GroundMotionReplayReport;
use crate::core::ifcx::{ns, IfcxDocument, IfcxNode};

pub const NS_EG_GPU_MONTE_CARLO_LOSS: &str = ns::EG_GPU_MONTE_CARLO_LOSS;

/// Execution backend used for a Monte Carlo fragility simulation run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ComputeBackendKind {
    WebGpuCompute,
    #[default]
    CpuVectorReference,
}

impl ComputeBackendKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::WebGpuCompute => "WebGPU @compute",
            Self::CpuVectorReference => "CPU Vector",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::WebGpuCompute => "GPU @compute",
            Self::CpuVectorReference => "CPU Vec",
        }
    }
}

/// Persisted configuration for WebGPU `@compute` multi-hazard particles and FEMA P-58 Monte Carlo.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MonteCarloFragilityConfig {
    #[serde(default = "default_mc_samples")]
    pub sample_count: u32,
    #[serde(default = "default_mc_seed")]
    pub seed: u32,
    #[serde(default = "default_edp_beta")]
    pub edp_dispersion_beta: f64,
    #[serde(default = "default_collapse_median")]
    pub collapse_drift_median: f64,
    #[serde(default = "default_collapse_beta")]
    pub collapse_beta: f64,
    #[serde(default)]
    pub gpu_particles_enabled: bool,
    #[serde(default = "default_particle_count")]
    pub gpu_particle_count: u32,
}

fn default_mc_samples() -> u32 {
    4096
}
fn default_mc_seed() -> u32 {
    79_2026
}
fn default_edp_beta() -> f64 {
    0.32
}
fn default_collapse_median() -> f64 {
    0.045
}
fn default_collapse_beta() -> f64 {
    0.40
}
fn default_particle_count() -> u32 {
    1024
}

impl Default for MonteCarloFragilityConfig {
    fn default() -> Self {
        Self {
            sample_count: default_mc_samples(),
            seed: default_mc_seed(),
            edp_dispersion_beta: default_edp_beta(),
            collapse_drift_median: default_collapse_median(),
            collapse_beta: default_collapse_beta(),
            gpu_particles_enabled: false,
            gpu_particle_count: default_particle_count(),
        }
    }
}

impl MonteCarloFragilityConfig {
    pub fn cycle_particle_count(&mut self) {
        self.gpu_particle_count = match self.gpu_particle_count {
            0..=768 => 1024,
            769..=1536 => 2048,
            1537..=3072 => 4096,
            _ => 512,
        };
    }

    pub fn normalized_sample_count(&self) -> u32 {
        self.sample_count.clamp(256, 16_384)
    }

    pub fn normalized_particle_count(&self) -> u32 {
        self.gpu_particle_count.clamp(128, 4096)
    }
}

/// Per-story demand & component quantity input fed uniformly into WGSL `@compute` and CPU fallback.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct StoryDemandInput {
    pub story_number: usize,
    pub pidr: f64,
    pub pfa_g: f64,
    pub rdr: f64,
    pub struct_qty: f64,
    pub ns_drift_qty: f64,
    pub mep_qty: f64,
    pub contents_qty: f64,
    pub story_weight: f64,
}

/// 32-byte tightly aligned GPU storage struct matching `StoryDemand` in WGSL (`8 x f32`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StoryDemandGpu {
    pub pidr: f32,
    pub pfa_g: f32,
    pub rdr: f32,
    pub struct_qty: f32,
    pub ns_drift_qty: f32,
    pub mep_qty: f32,
    pub contents_qty: f32,
    pub story_weight: f32,
}

/// 32-byte uniform header passed to `FRAGILITY_MC_WGSL`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct McUniformsGpu {
    pub sample_count: u32,
    pub story_count: u32,
    pub seed: u32,
    pub edp_beta: f32,
    pub collapse_median: f32,
    pub collapse_beta: f32,
    pub replacement_cost_usd: f32,
    pub pad0: u32,
}

/// Point on the empirical Loss Exceedance Curve `(loss_usd, P[Loss >= loss_usd])`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct LossExceedancePoint {
    pub loss_usd: f64,
    pub exceedance_prob: f64,
}

/// Empirical downtime histogram bucket.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct DowntimeHistogramBin {
    pub min_days: f64,
    pub max_days: f64,
    pub probability_pct: f64,
}

/// Empirical probability (%) of achieving each Arup REDi™ recovery tier across realizations.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct RediTierProbabilities {
    pub platinum_pct: f64,
    pub gold_pct: f64,
    pub silver_pct: f64,
    pub standard_pct: f64,
}

/// Full probabilistic FEMA P-58 Monte Carlo Fragility report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MonteCarloFragilityReport {
    pub config: MonteCarloFragilityConfig,
    pub backend: ComputeBackendKind,
    pub sample_count: u32,
    pub story_count: usize,
    pub replacement_cost_usd: f64,
    pub mean_loss_usd: f64,
    pub p10_loss_usd: f64,
    pub p50_loss_usd: f64,
    pub p90_loss_usd: f64,
    pub cvar95_loss_usd: f64,
    pub p50_downtime_days: f64,
    pub p90_downtime_days: f64,
    pub p50_carbon_kg: f64,
    pub p90_carbon_kg: f64,
    pub collapse_prob_pct: f64,
    pub red_tag_prob_pct: f64,
    pub redi_probs: RediTierProbabilities,
    pub exceedance_curve: Vec<LossExceedancePoint>,
    pub downtime_bins: Vec<DowntimeHistogramBin>,
    pub summary: String,
}

impl MonteCarloFragilityReport {
    /// Dominant likely REDi™ tier (cumulative probability >= 50% benchmark).
    pub fn likely_redi_tier(&self) -> REDiResilienceTier {
        if self.redi_probs.platinum_pct >= 50.0 {
            REDiResilienceTier::Platinum
        } else if self.redi_probs.platinum_pct + self.redi_probs.gold_pct >= 50.0 {
            REDiResilienceTier::Gold
        } else if self.redi_probs.platinum_pct
            + self.redi_probs.gold_pct
            + self.redi_probs.silver_pct
            >= 50.0
        {
            REDiResilienceTier::Silver
        } else {
            REDiResilienceTier::StandardCode
        }
    }

    /// Exports non-destructive IFCX overlay (`eigengraph::analysis::gpu_monte_carlo_loss`).
    pub fn build_ifcx_overlay(&self, project: &BimProject) -> IfcxDocument {
        let proj_name = if project.project_name.trim().is_empty() {
            "EigenGraph_BIM"
        } else {
            project.project_name.trim()
        };
        let mut doc = IfcxDocument::new(&format!("{proj_name}_gpu_monte_carlo_loss"));
        doc.schemas.insert(
            ns::EG_GPU_MONTE_CARLO_LOSS.to_string(),
            json!({
                "value": {
                    "dataType": "Object",
                    "spec": "EigenGraph Phase 79 WebGPU @compute FEMA P-58 Monte Carlo Loss Overlay"
                }
            }),
        );
        let root_path = format!("/{proj_name}/analysis/gpu_monte_carlo_loss");
        let mut attrs = std::collections::BTreeMap::new();
        attrs.insert(
            ns::EG_GPU_MONTE_CARLO_LOSS.to_string(),
            json!({
                "schema": "eigengraph-gpu-monte-carlo-v1",
                "backend": self.backend.label(),
                "sample_count": self.sample_count,
                "seed": self.config.seed,
                "story_count": self.story_count,
                "replacement_cost_usd": self.replacement_cost_usd,
                "mean_loss_usd": self.mean_loss_usd,
                "p10_loss_usd": self.p10_loss_usd,
                "p50_loss_usd": self.p50_loss_usd,
                "pml90_loss_usd": self.p90_loss_usd,
                "cvar95_loss_usd": self.cvar95_loss_usd,
                "p50_downtime_days": self.p50_downtime_days,
                "p90_downtime_days": self.p90_downtime_days,
                "p50_carbon_kg": self.p50_carbon_kg,
                "p90_carbon_kg": self.p90_carbon_kg,
                "collapse_prob_pct": self.collapse_prob_pct,
                "red_tag_prob_pct": self.red_tag_prob_pct,
                "redi_probs": {
                    "platinum_pct": self.redi_probs.platinum_pct,
                    "gold_pct": self.redi_probs.gold_pct,
                    "silver_pct": self.redi_probs.silver_pct,
                    "standard_pct": self.redi_probs.standard_pct,
                },
                "summary": self.summary,
            }),
        );
        doc.data.push(IfcxNode {
            path: root_path,
            children: std::collections::BTreeMap::new(),
            inherits: std::collections::BTreeMap::new(),
            attributes: attrs,
        });
        doc
    }
}

/// Extracts calibrated story-level EDP inputs for FEMA P-58 Monte Carlo from BIM + active analyses.
pub fn extract_story_demands_for_mc(
    project: &BimProject,
    replay_opt: Option<&GroundMotionReplayReport>,
    hazard_opt: Option<&DistrictMultiHazardReport>,
    ssi_opt: Option<&SubsurfaceSsiAnalysisReport>,
) -> (Vec<StoryDemandInput>, f64) {
    let n_stories = project.levels.len().saturating_sub(1).clamp(1, 24);
    let col_count = (project.columns.len() as f64).max(4.0);
    let beam_count = (project.runs.len() as f64).max(4.0);
    let wall_count = project.walls.len() as f64;
    let elems_per_story = ((col_count + beam_count + wall_count) / (n_stories as f64)).max(6.0);

    let pga_g = replay_opt
        .map(|r| r.scaled_pga_g)
        .or_else(|| hazard_opt.map(|h| h.pga_g))
        .unwrap_or(0.32)
        .clamp(0.05, 1.80);
    let ssi_amp = ssi_opt
        .map(|s| {
            if s.is_liquefaction_critical {
                1.24
            } else {
                1.0 + 0.008 * s.period_lengthening_pct.max(0.0)
            }
        })
        .unwrap_or(1.0);
    let base_drift = replay_opt
        .map(|r| r.peak_interstory_drift_ratio)
        .unwrap_or_else(|| (0.0085 * (pga_g / 0.32) * ssi_amp).clamp(0.002, 0.055));

    let mut stories = Vec::with_capacity(n_stories);
    for s in 1..=n_stories {
        let story_frac = (s as f64) / (n_stories as f64);
        let replay_story = replay_opt.and_then(|r| {
            r.fragility_loss
                .story_losses
                .iter()
                .find(|st| st.story_number == s)
        });
        let pidr = replay_story
            .map(|st| st.peak_drift_ratio)
            .unwrap_or_else(|| {
                // First-mode concentration near lower/mid stories
                let shape = 1.15 - 0.35 * (story_frac - 0.35).abs();
                (base_drift * shape).clamp(0.0015, 0.085)
            });
        let pfa_g = (pga_g * (0.95 + 1.15 * story_frac)).clamp(0.08, 3.20);
        let rdr = if pidr > 0.012 {
            (pidr - 0.010) * 0.48
        } else {
            pidr * 0.12
        };
        let floor_area_factor = (elems_per_story / 12.0).clamp(0.75, 4.5);
        stories.push(StoryDemandInput {
            story_number: s,
            pidr,
            pfa_g,
            rdr,
            struct_qty: (elems_per_story * 0.55).max(4.0),
            ns_drift_qty: 6.0 * floor_area_factor,
            mep_qty: 5.0 * floor_area_factor,
            contents_qty: 4.0 * floor_area_factor,
            story_weight: (1.0 / (n_stories as f64)).max(0.04),
        });
    }

    let floor_area_per_story_m2 = ((elems_per_story * 18.0).clamp(160.0, 1800.0)) as f64;
    let replacement_cost_usd = (n_stories as f64 * floor_area_per_story_m2 * 1650.0).max(450_000.0);
    (stories, replacement_cost_usd)
}

#[inline]
fn pcg_step_cpu(state: &mut u32) -> u32 {
    let old = *state;
    *state = old.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let shifted = (old >> ((old >> 28).wrapping_add(4))) ^ old;
    let xorshifted = shifted.wrapping_mul(277_803_737);
    (xorshifted >> 22) ^ xorshifted
}

#[inline]
fn norm_cdf_f32(z: f32) -> f32 {
    let sign = if z < 0.0 { -1.0_f32 } else { 1.0_f32 };
    let x = z.abs() * std::f32::consts::FRAC_1_SQRT_2;
    let t = 1.0_f32 / (1.0_f32 + 0.3275911_f32 * x);
    let y = 1.0_f32
        - (((((1.061405429_f32 * t - 1.453152027_f32) * t) + 1.421413741_f32) * t
            - 0.284496736_f32)
            * t
            + 0.254829592_f32)
            * t
            * (-x * x).exp();
    0.5_f32 * (1.0_f32 + sign * y)
}

#[inline]
fn lognorm_exceed_f32(demand: f32, theta: f32, beta: f32) -> f32 {
    if demand <= 1.0e-7_f32 {
        return 0.0;
    }
    let z = (demand / theta.max(1.0e-5)).ln() / beta.max(0.05);
    norm_cdf_f32(z).clamp(0.0, 1.0)
}

#[inline]
fn rand_u01_cpu(rng: &mut u32) -> f32 {
    let v = pcg_step_cpu(rng);
    ((v as f32) + 0.5) / 4_294_967_296.0
}

#[inline]
fn rand_norm_cpu(rng: &mut u32) -> f32 {
    let u1 = rand_u01_cpu(rng).clamp(1.0e-6, 0.999_999);
    let u2 = rand_u01_cpu(rng);
    (-2.0 * u1.ln()).sqrt() * (std::f32::consts::TAU * u2).cos()
}

/// Exact CPU parity implementation of `FRAGILITY_MC_WGSL`.
fn evaluate_single_realization_cpu(
    tid: u32,
    uniforms: &McUniformsGpu,
    stories: &[StoryDemandGpu],
) -> [f32; 4] {
    let mut rng = uniforms
        .seed
        ^ tid.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    let _ = pcg_step_cpu(&mut rng);
    let _ = pcg_step_cpu(&mut rng);

    let eps_global = rand_norm_cpu(&mut rng);
    let eps_pfa = rand_norm_cpu(&mut rng);
    let collapse_rand = rand_u01_cpu(&mut rng);

    let mut total_loss = 0.0_f32;
    let mut total_days = 0.0_f32;
    let mut total_carbon = 0.0_f32;
    let mut max_pidr = 0.0_f32;

    for (idx, st) in stories.iter().enumerate() {
        let eps_story = rand_norm_cpu(&mut rng);
        let pidr = (st.pidr * (uniforms.edp_beta * (0.72 * eps_global + 0.69 * eps_story)).exp())
            .clamp(0.0003, 0.25);
        let pfa = (st.pfa_g * (uniforms.edp_beta * 0.85 * (0.70 * eps_pfa + 0.71 * eps_story)).exp())
            .clamp(0.02, 5.0);
        let rdr = (st.rdr * (uniforms.edp_beta * (0.75 * eps_global + 0.66 * eps_story)).exp())
            .clamp(0.0001, 0.15);
        max_pidr = max_pidr.max(pidr);

        // 1. Structural frame (SMRF / RC columns+beams, cost ~$6.8k/$22k/$48k for DS1/DS2/DS3)
        let p_s1 = lognorm_exceed_f32(pidr, 0.0085, 0.34);
        let p_s2 = lognorm_exceed_f32(pidr, 0.0185, 0.36).min(p_s1);
        let p_s3 = lognorm_exceed_f32(pidr, 0.0320, 0.38).min(p_s2);
        let e_struct_cost =
            st.struct_qty * ((p_s1 - p_s2) * 4_500.0 + (p_s2 - p_s3) * 16_500.0 + p_s3 * 36_000.0);
        let e_struct_days =
            st.struct_qty * ((p_s1 - p_s2) * 0.45 + (p_s2 - p_s3) * 1.65 + p_s3 * 3.80);
        let e_struct_co2 =
            st.struct_qty * ((p_s1 - p_s2) * 1_100.0 + (p_s2 - p_s3) * 4_200.0 + p_s3 * 9_500.0);

        // 2. Non-structural drift-sensitive facade/partitions
        let p_n1 = lognorm_exceed_f32(pidr, 0.0065, 0.38);
        let p_n2 = lognorm_exceed_f32(pidr, 0.0155, 0.36).min(p_n1);
        let e_ns_cost = st.ns_drift_qty * ((p_n1 - p_n2) * 3_200.0 + p_n2 * 10_800.0);
        let e_ns_days = st.ns_drift_qty * ((p_n1 - p_n2) * 0.25 + p_n2 * 0.90);
        let e_ns_co2 = st.ns_drift_qty * ((p_n1 - p_n2) * 620.0 + p_n2 * 2_100.0);

        // 3. MEP / ceilings acceleration-sensitive
        let p_m1 = lognorm_exceed_f32(pfa, 0.48, 0.40);
        let p_m2 = lognorm_exceed_f32(pfa, 1.05, 0.42).min(p_m1);
        let e_mep_cost = st.mep_qty * ((p_m1 - p_m2) * 2_800.0 + p_m2 * 11_500.0);
        let e_mep_days = st.mep_qty * ((p_m1 - p_m2) * 0.20 + p_m2 * 0.85);
        let e_mep_co2 = st.mep_qty * ((p_m1 - p_m2) * 420.0 + p_m2 * 1_650.0);

        // 4. Residual drift / irreparable tilt surcharge
        let p_irr = lognorm_exceed_f32(rdr, 0.010, 0.30);
        let irr_cost = p_irr * st.story_weight * uniforms.replacement_cost_usd * 0.45;
        let irr_days = p_irr * 14.0;
        let irr_co2 = irr_cost * 0.28;

        let _ = idx;
        total_loss += e_struct_cost + e_ns_cost + e_mep_cost + irr_cost;
        total_days += (e_struct_days + e_ns_days + e_mep_days + irr_days) * 0.42;
        total_carbon += e_struct_co2 + e_ns_co2 + e_mep_co2 + irr_co2;
    }

    let p_col = lognorm_exceed_f32(max_pidr, uniforms.collapse_median, uniforms.collapse_beta);
    let collapsed = collapse_rand < p_col;
    let red_tag = collapsed || max_pidr >= 0.025 || total_loss >= uniforms.replacement_cost_usd * 0.40;
    if collapsed {
        total_loss = uniforms.replacement_cost_usd;
        total_days = 240.0;
        total_carbon = uniforms.replacement_cost_usd * 0.42;
    } else {
        total_loss = total_loss.min(uniforms.replacement_cost_usd);
        total_days = total_days.clamp(0.2, 220.0);
    }

    let flag = (if collapsed { 1.0 } else { 0.0 }) + (if red_tag { 2.0 } else { 0.0 });
    [total_loss, total_days, total_carbon, flag]
}

/// WGSL `@compute` shader mirroring `evaluate_single_realization_cpu` cleanly across workgroups of 64.
pub const FRAGILITY_MC_WGSL: &str = r#"
struct McUniforms {
    sample_count: u32,
    story_count: u32,
    seed: u32,
    edp_beta: f32,
    collapse_median: f32,
    collapse_beta: f32,
    replacement_cost_usd: f32,
    pad0: u32,
};

struct StoryDemandGpu {
    pidr: f32,
    pfa_g: f32,
    rdr: f32,
    struct_qty: f32,
    ns_drift_qty: f32,
    mep_qty: f32,
    contents_qty: f32,
    story_weight: f32,
};

@group(0) @binding(0) var<uniform> params: McUniforms;
@group(0) @binding(1) var<storage, read> stories: array<StoryDemandGpu>;
@group(0) @binding(2) var<storage, read_write> results: array<vec4<f32>>;

fn pcg_step(state: ptr<function, u32>) -> u32 {
    let old = *state;
    *state = old * 747796405u + 2891336453u;
    let shifted = (old >> ((old >> 28u) + 4u)) ^ old;
    let xorshifted = shifted * 277803737u;
    return (xorshifted >> 22u) ^ xorshifted;
}

fn rand_u01(state: ptr<function, u32>) -> f32 {
    let v = pcg_step(state);
    return (f32(v) + 0.5) / 4294967296.0;
}

fn rand_norm(state: ptr<function, u32>) -> f32 {
    let u1 = clamp(rand_u01(state), 0.000001, 0.999999);
    let u2 = rand_u01(state);
    return sqrt(-2.0 * log(u1)) * cos(6.283185307 * u2);
}

fn norm_cdf(z: f32) -> f32 {
    let sign = select(1.0, -1.0, z < 0.0);
    let x = abs(z) * 0.707106781;
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let poly = (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t + 0.254829592) * t;
    let y = 1.0 - poly * exp(-x * x);
    return 0.5 * (1.0 + sign * y);
}

fn lognorm_exceed(demand: f32, theta: f32, beta: f32) -> f32 {
    if (demand <= 0.0000001) {
        return 0.0;
    }
    let z = log(demand / max(theta, 0.00001)) / max(beta, 0.05);
    return clamp(norm_cdf(z), 0.0, 1.0);
}

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let tid = gid.x;
    if (tid >= params.sample_count) {
        return;
    }
    var rng: u32 = params.seed ^ (tid * 1664525u + 1013904223u);
    let _w0 = pcg_step(&rng);
    let _w1 = pcg_step(&rng);

    let eps_global = rand_norm(&rng);
    let eps_pfa = rand_norm(&rng);
    let collapse_rand = rand_u01(&rng);

    var total_loss: f32 = 0.0;
    var total_days: f32 = 0.0;
    var total_carbon: f32 = 0.0;
    var max_pidr: f32 = 0.0;

    let n_st = params.story_count;
    for (var s: u32 = 0u; s < n_st; s = s + 1u) {
        let st = stories[s];
        let eps_story = rand_norm(&rng);
        let pidr = clamp(st.pidr * exp(params.edp_beta * (0.72 * eps_global + 0.69 * eps_story)), 0.0003, 0.25);
        let pfa = clamp(st.pfa_g * exp(params.edp_beta * 0.85 * (0.70 * eps_pfa + 0.71 * eps_story)), 0.02, 5.0);
        let rdr = clamp(st.rdr * exp(params.edp_beta * (0.75 * eps_global + 0.66 * eps_story)), 0.0001, 0.15);
        max_pidr = max(max_pidr, pidr);

        let p_s1 = lognorm_exceed(pidr, 0.0085, 0.34);
        let p_s2 = min(lognorm_exceed(pidr, 0.0185, 0.36), p_s1);
        let p_s3 = min(lognorm_exceed(pidr, 0.0320, 0.38), p_s2);
        let e_struct_cost = st.struct_qty * ((p_s1 - p_s2) * 4500.0 + (p_s2 - p_s3) * 16500.0 + p_s3 * 36000.0);
        let e_struct_days = st.struct_qty * ((p_s1 - p_s2) * 0.45 + (p_s2 - p_s3) * 1.65 + p_s3 * 3.80);
        let e_struct_co2 = st.struct_qty * ((p_s1 - p_s2) * 1100.0 + (p_s2 - p_s3) * 4200.0 + p_s3 * 9500.0);

        let p_n1 = lognorm_exceed(pidr, 0.0065, 0.38);
        let p_n2 = min(lognorm_exceed(pidr, 0.0155, 0.36), p_n1);
        let e_ns_cost = st.ns_drift_qty * ((p_n1 - p_n2) * 3200.0 + p_n2 * 10800.0);
        let e_ns_days = st.ns_drift_qty * ((p_n1 - p_n2) * 0.25 + p_n2 * 0.90);
        let e_ns_co2 = st.ns_drift_qty * ((p_n1 - p_n2) * 620.0 + p_n2 * 2100.0);

        let p_m1 = lognorm_exceed(pfa, 0.48, 0.40);
        let p_m2 = min(lognorm_exceed(pfa, 1.05, 0.42), p_m1);
        let e_mep_cost = st.mep_qty * ((p_m1 - p_m2) * 2800.0 + p_m2 * 11500.0);
        let e_mep_days = st.mep_qty * ((p_m1 - p_m2) * 0.20 + p_m2 * 0.85);
        let e_mep_co2 = st.mep_qty * ((p_m1 - p_m2) * 420.0 + p_m2 * 1650.0);

        let p_irr = lognorm_exceed(rdr, 0.010, 0.30);
        let irr_cost = p_irr * st.story_weight * params.replacement_cost_usd * 0.45;
        let irr_days = p_irr * 14.0;
        let irr_co2 = irr_cost * 0.28;

        total_loss = total_loss + e_struct_cost + e_ns_cost + e_mep_cost + irr_cost;
        total_days = total_days + (e_struct_days + e_ns_days + e_mep_days + irr_days) * 0.42;
        total_carbon = total_carbon + e_struct_co2 + e_ns_co2 + e_mep_co2 + irr_co2;
    }

    let p_col = lognorm_exceed(max_pidr, params.collapse_median, params.collapse_beta);
    let collapsed = collapse_rand < p_col;
    let red_tag = collapsed || max_pidr >= 0.025 || total_loss >= params.replacement_cost_usd * 0.40;
    if (collapsed) {
        total_loss = params.replacement_cost_usd;
        total_days = 240.0;
        total_carbon = params.replacement_cost_usd * 0.42;
    } else {
        total_loss = min(total_loss, params.replacement_cost_usd);
        total_days = clamp(total_days, 0.2, 220.0);
    }

    let flag = select(0.0, 1.0, collapsed) + select(0.0, 2.0, red_tag);
    results[tid] = vec4<f32>(total_loss, total_days, total_carbon, flag);
}
"#;

pub fn build_gpu_inputs(
    stories: &[StoryDemandInput],
    config: &MonteCarloFragilityConfig,
    replacement_cost_usd: f64,
) -> (McUniformsGpu, Vec<StoryDemandGpu>) {
    let gpu_stories: Vec<StoryDemandGpu> = stories
        .iter()
        .map(|s| StoryDemandGpu {
            pidr: s.pidr as f32,
            pfa_g: s.pfa_g as f32,
            rdr: s.rdr as f32,
            struct_qty: s.struct_qty as f32,
            ns_drift_qty: s.ns_drift_qty as f32,
            mep_qty: s.mep_qty as f32,
            contents_qty: s.contents_qty as f32,
            story_weight: s.story_weight as f32,
        })
        .collect();
    let uniforms = McUniformsGpu {
        sample_count: config.normalized_sample_count(),
        story_count: gpu_stories.len() as u32,
        seed: config.seed,
        edp_beta: config.edp_dispersion_beta.clamp(0.08, 0.85) as f32,
        collapse_median: config.collapse_drift_median.clamp(0.015, 0.12) as f32,
        collapse_beta: config.collapse_beta.clamp(0.15, 0.80) as f32,
        replacement_cost_usd: replacement_cost_usd.max(100_000.0) as f32,
        pad0: 0,
    };
    (uniforms, gpu_stories)
}

/// Aggregates `[loss_usd, downtime_days, carbon_kg, flags]` realizations into quantiles & curves.
pub fn aggregate_realizations(
    realizations: &[[f32; 4]],
    config: &MonteCarloFragilityConfig,
    backend: ComputeBackendKind,
    story_count: usize,
    replacement_cost_usd: f64,
) -> MonteCarloFragilityReport {
    let n = realizations.len().max(1);
    let mut losses = Vec::with_capacity(n);
    let mut days = Vec::with_capacity(n);
    let mut carbons = Vec::with_capacity(n);
    let mut collapse_cnt = 0usize;
    let mut red_tag_cnt = 0usize;
    let mut plat_cnt = 0usize;
    let mut gold_cnt = 0usize;
    let mut silv_cnt = 0usize;
    let mut std_cnt = 0usize;

    for r in realizations {
        let loss = (r[0] as f64).max(0.0);
        let d = (r[1] as f64).max(0.0);
        let co2 = (r[2] as f64).max(0.0);
        let flags = r[3];
        let collapsed = (flags % 2.0) >= 0.5;
        let red_tag = flags >= 1.5;

        if collapsed {
            collapse_cnt += 1;
        }
        if red_tag {
            red_tag_cnt += 1;
        }
        let lr = loss / replacement_cost_usd.max(1.0);
        if !collapsed && d <= 7.0 && lr <= 0.05 {
            plat_cnt += 1;
        } else if !collapsed && d <= 30.0 && lr <= 0.10 {
            gold_cnt += 1;
        } else if !collapsed && d <= 60.0 && lr <= 0.20 {
            silv_cnt += 1;
        } else {
            std_cnt += 1;
        }

        losses.push(loss);
        days.push(d);
        carbons.push(co2);
    }

    losses.sort_by(f64::total_cmp);
    days.sort_by(f64::total_cmp);
    carbons.sort_by(f64::total_cmp);

    let quantile = |sorted: &[f64], q: f64| -> f64 {
        let idx = ((sorted.len().saturating_sub(1)) as f64 * q.clamp(0.0, 1.0)).round() as usize;
        sorted.get(idx).copied().unwrap_or(0.0)
    };

    let mean_loss_usd = losses.iter().sum::<f64>() / (n as f64);
    let p10_loss_usd = quantile(&losses, 0.10);
    let p50_loss_usd = quantile(&losses, 0.50);
    let p90_loss_usd = quantile(&losses, 0.90);
    let tail_start = ((n as f64) * 0.95).floor() as usize;
    let tail_slice = &losses[tail_start.min(n - 1)..];
    let cvar95_loss_usd = tail_slice.iter().sum::<f64>() / (tail_slice.len().max(1) as f64);

    let p50_downtime_days = quantile(&days, 0.50);
    let p90_downtime_days = quantile(&days, 0.90);
    let p50_carbon_kg = quantile(&carbons, 0.50);
    let p90_carbon_kg = quantile(&carbons, 0.90);

    let max_curve_loss = p90_loss_usd.max(cvar95_loss_usd).max(10_000.0);
    let mut exceedance_curve = Vec::with_capacity(24);
    for k in 0..24 {
        let frac = (k as f64) / 23.0;
        let threshold = frac * max_curve_loss;
        let exceed = losses.iter().filter(|&&v| v >= threshold).count();
        exceedance_curve.push(LossExceedancePoint {
            loss_usd: threshold,
            exceedance_prob: (exceed as f64) / (n as f64),
        });
    }

    let bin_edges: [(f64, f64); 8] = [
        (0.0, 7.0),
        (7.0, 14.0),
        (14.0, 30.0),
        (30.0, 60.0),
        (60.0, 90.0),
        (90.0, 120.0),
        (120.0, 180.0),
        (180.0, 360.0),
    ];
    let downtime_bins: Vec<DowntimeHistogramBin> = bin_edges
        .iter()
        .enumerate()
        .map(|(bi, &(lo, hi))| {
            let cnt = days
                .iter()
                .filter(|&&d| {
                    if bi + 1 == bin_edges.len() {
                        d >= lo
                    } else {
                        d >= lo && d < hi
                    }
                })
                .count();
            DowntimeHistogramBin {
                min_days: lo,
                max_days: hi,
                probability_pct: (cnt as f64) * 100.0 / (n as f64),
            }
        })
        .collect();

    let collapse_prob_pct = (collapse_cnt as f64) * 100.0 / (n as f64);
    let red_tag_prob_pct = (red_tag_cnt as f64) * 100.0 / (n as f64);
    let redi_probs = RediTierProbabilities {
        platinum_pct: (plat_cnt as f64) * 100.0 / (n as f64),
        gold_pct: (gold_cnt as f64) * 100.0 / (n as f64),
        silver_pct: (silv_cnt as f64) * 100.0 / (n as f64),
        standard_pct: (std_cnt as f64) * 100.0 / (n as f64),
    };

    let summary = format!(
        "FEMA P-58 MC ({} x {}): P50 ${:.0}k | PML90 ${:.0}k | CVaR95 ${:.0}k | P90 {:.0}d | P(Col) {:.1}%",
        n,
        backend.short_label(),
        p50_loss_usd * 0.001,
        p90_loss_usd * 0.001,
        cvar95_loss_usd * 0.001,
        p90_downtime_days,
        collapse_prob_pct
    );

    MonteCarloFragilityReport {
        config: config.clone(),
        backend,
        sample_count: n as u32,
        story_count,
        replacement_cost_usd,
        mean_loss_usd,
        p10_loss_usd,
        p50_loss_usd,
        p90_loss_usd,
        cvar95_loss_usd,
        p50_downtime_days,
        p90_downtime_days,
        p50_carbon_kg,
        p90_carbon_kg,
        collapse_prob_pct,
        red_tag_prob_pct,
        redi_probs,
        exceedance_curve,
        downtime_bins,
        summary,
    }
}

/// Runs deterministic CPU reference FEMA P-58 Monte Carlo simulation (exact mirror of WGSL `@compute`).
pub fn run_fema_p58_monte_carlo_cpu(
    project: &BimProject,
    replay_opt: Option<&GroundMotionReplayReport>,
    hazard_opt: Option<&DistrictMultiHazardReport>,
    ssi_opt: Option<&SubsurfaceSsiAnalysisReport>,
    config: &MonteCarloFragilityConfig,
) -> MonteCarloFragilityReport {
    let (stories, replacement_cost_usd) =
        extract_story_demands_for_mc(project, replay_opt, hazard_opt, ssi_opt);
    let (uniforms, gpu_stories) = build_gpu_inputs(&stories, config, replacement_cost_usd);
    let n = uniforms.sample_count as usize;
    let mut realizations = Vec::with_capacity(n);
    for tid in 0..uniforms.sample_count {
        realizations.push(evaluate_single_realization_cpu(tid, &uniforms, &gpu_stories));
    }
    aggregate_realizations(
        &realizations,
        config,
        ComputeBackendKind::CpuVectorReference,
        stories.len(),
        replacement_cost_usd,
    )
}

/// Executes `FRAGILITY_MC_WGSL` directly on native WebGPU (`wgpu`) hardware (`Metal`/`Vulkan`/`DX12`).
#[cfg(not(target_arch = "wasm32"))]
pub fn run_fema_p58_monte_carlo_wgpu(
    device: &eframe::wgpu::Device,
    queue: &eframe::wgpu::Queue,
    project: &BimProject,
    replay_opt: Option<&GroundMotionReplayReport>,
    hazard_opt: Option<&DistrictMultiHazardReport>,
    ssi_opt: Option<&SubsurfaceSsiAnalysisReport>,
    config: &MonteCarloFragilityConfig,
) -> Result<MonteCarloFragilityReport, String> {
    use eframe::wgpu;
    use std::borrow::Cow;

    let (stories, replacement_cost_usd) =
        extract_story_demands_for_mc(project, replay_opt, hazard_opt, ssi_opt);
    let (uniforms, gpu_stories) = build_gpu_inputs(&stories, config, replacement_cost_usd);
    if gpu_stories.is_empty() || uniforms.sample_count == 0 {
        return Err("Empty story inputs for Monte Carlo".to_string());
    }

    let u_bytes = unsafe {
        std::slice::from_raw_parts(
            (&uniforms as *const McUniformsGpu) as *const u8,
            std::mem::size_of::<McUniformsGpu>(),
        )
    };
    let s_bytes = unsafe {
        std::slice::from_raw_parts(
            gpu_stories.as_ptr() as *const u8,
            std::mem::size_of_val(gpu_stories.as_slice()),
        )
    };
    let out_size = (uniforms.sample_count as u64) * 16;

    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("eigengraph_fema_p58_mc_wgsl"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(FRAGILITY_MC_WGSL)),
    });
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("eigengraph_mc_bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("eigengraph_mc_layout"),
        bind_group_layouts: &[Some(&bgl)],
        immediate_size: 0,
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("eigengraph_mc_compute_pipeline"),
        layout: Some(&layout),
        module: &module,
        entry_point: Some("cs_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });

    let u_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("mc_uniforms"),
        size: u_bytes.len() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let s_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("mc_stories"),
        size: (s_bytes.len() as u64).max(32),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let out_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("mc_out"),
        size: out_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("mc_readback"),
        size: out_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    queue.write_buffer(&u_buf, 0, u_bytes);
    queue.write_buffer(&s_buf, 0, s_bytes);

    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("eigengraph_mc_bg"),
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: u_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: s_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: out_buf.as_entire_binding(),
            },
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("eigengraph_mc_encoder"),
    });
    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("eigengraph_mc_compute_pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&pipeline);
        cpass.set_bind_group(0, &bg, &[]);
        cpass.dispatch_workgroups(uniforms.sample_count.div_ceil(64), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&out_buf, 0, &read_buf, 0, out_size);
    queue.submit([encoder.finish()]);

    let map_res: std::sync::Arc<std::sync::Mutex<Option<Result<(), wgpu::BufferAsyncError>>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let map_cb = std::sync::Arc::clone(&map_res);
    read_buf.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        if let Ok(mut g) = map_cb.lock() {
            *g = Some(r);
        }
    });
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    let status = map_res
        .lock()
        .map_err(|e| format!("MC lock poisoned: {e}"))?
        .take()
        .ok_or_else(|| "MC GPU buffer map callback missing".to_string())?;
    status.map_err(|e| format!("MC GPU buffer map failed: {e:?}"))?;

    let raw_bytes = read_buf
        .slice(..)
        .get_mapped_range()
        .map_err(|e| format!("MC mapped range failed: {e:?}"))?;
    let mut realizations = Vec::with_capacity(uniforms.sample_count as usize);
    for chunk in raw_bytes.chunks_exact(16) {
        realizations.push([
            f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]),
            f32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]),
            f32::from_le_bytes([chunk[8], chunk[9], chunk[10], chunk[11]]),
            f32::from_le_bytes([chunk[12], chunk[13], chunk[14], chunk[15]]),
        ]);
    }
    drop(raw_bytes);
    read_buf.unmap();

    Ok(aggregate_realizations(
        &realizations,
        config,
        ComputeBackendKind::WebGpuCompute,
        stories.len(),
        replacement_cost_usd,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ifcx::IfcxStage;

    #[test]
    fn phase79_fema_p58_monte_carlo_quantiles_determinism_and_ifcx_overlay() {
        let proj = BimProject::sample_bimcraft_building();
        let cfg = MonteCarloFragilityConfig {
            sample_count: 1024,
            seed: 79_2026,
            edp_dispersion_beta: 0.32,
            ..Default::default()
        };
        let r1 = run_fema_p58_monte_carlo_cpu(&proj, None, None, None, &cfg);
        let r2 = run_fema_p58_monte_carlo_cpu(&proj, None, None, None, &cfg);
        assert_eq!(r1.sample_count, 1024);
        assert_eq!(r1.p50_loss_usd, r2.p50_loss_usd);
        assert_eq!(r1.p90_loss_usd, r2.p90_loss_usd);
        assert!(r1.p10_loss_usd <= r1.p50_loss_usd);
        assert!(r1.p50_loss_usd <= r1.p90_loss_usd);
        assert!(r1.p90_loss_usd <= r1.cvar95_loss_usd + 1.0);
        assert!(r1.p50_downtime_days <= r1.p90_downtime_days);
        assert_eq!(r1.exceedance_curve.len(), 24);
        assert_eq!(r1.downtime_bins.len(), 8);
        let tier_sum = r1.redi_probs.platinum_pct
            + r1.redi_probs.gold_pct
            + r1.redi_probs.silver_pct
            + r1.redi_probs.standard_pct;
        assert!((tier_sum - 100.0).abs() < 0.05);

        let severe_haz = DistrictMultiHazardReport {
            pga_g: 0.88,
            ..Default::default()
        };
        let r_sev = run_fema_p58_monte_carlo_cpu(&proj, None, Some(&severe_haz), None, &cfg);
        assert!(r_sev.p90_loss_usd > r1.p90_loss_usd);
        assert!(r_sev.collapse_prob_pct >= r1.collapse_prob_pct);

        let base = IfcxDocument::default();
        let overlay = r1.build_ifcx_overlay(&proj);
        assert_eq!(overlay.data.len(), 1);
        assert!(overlay.data[0].attributes.contains_key(ns::EG_GPU_MONTE_CARLO_LOSS));
        let composed = IfcxStage::compose_with_monte_carlo_loss_overlay(&base, &overlay);
        assert!(composed
            .data
            .iter()
            .any(|n| n.attributes.contains_key(ns::EG_GPU_MONTE_CARLO_LOSS)));
    }
}
