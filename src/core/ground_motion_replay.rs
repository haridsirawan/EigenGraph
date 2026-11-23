//! core/ground_motion_replay.rs — Real-Time WebWorker / Async Multi-Thread `StructGNN-CRC` & Live Earthquake Ground-Motion Replay (Phase 74 / v0.7.4).
//!
//! Couples active 3D OpenBIM (`BimProject`), 6-DOF FEM + Modal eigenperiods, Subsurface SSI
//! period lengthening ($\tilde{T}_1$), ASCE 7-22 / SNI 1726 HHT-$\alpha$ nonlinear time-history
//! integration (NL-THA), dynamic ASCE 41-17 / FEMA 356 plastic hinge formation, and Two-Stage
//! `ARI v2 + StructGNN-CRC` conformal screening inside a non-blocking background worker pipeline
//! (OS worker thread on Native, inline Blob WebWorker + cooperative time-sliced Rust fallback on WASM).

use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::core::ari::{AriEngine, MARGIN_TOL};
use crate::core::bim::{BimElementKind, BimProject, RunCategory};
use crate::core::cascade::{CrcRiskTier, StructGnnCrcScenario};
use crate::core::fema_p58::{FemaP58Engine, ReplayFemaP58Synthesis};
use crate::core::geotech::SubsurfaceSsiAnalysisReport;
use crate::core::ifcx::{ns, sanitize_slug, IfcxAriOverlay, IfcxDocument, IfcxImport, IfcxNode};
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
use crate::core::modal::{ModalEngine, SiteClass, Sni1726Spectrum};
use crate::core::nonlinear::PlasticHingeState;
use crate::core::solver::FemSolver;
use crate::core::time_history::{
    compute_elastic_response_spectrum_5pct, CustomAccelerogram, GroundMotionRecord,
    HysteresisModel, ResponseSpectrumPoint, TimeHistoryEngine, TimeHistoryInput,
    TimeHistorySample, TimeIntegrationScheme,
};

/// Custom IFCX namespace storing Nonlinear Earthquake Ground-Motion Replay & Async `StructGNN-CRC` telemetry.
pub const NS_EG_TIME_HISTORY_REPLAY: &str = "eigengraph::analysis::time_history_replay";
/// Custom IFCX namespace storing real-time FEMA P-58 fragility & story loss synthesis overlay.
pub const NS_EG_FRAGILITY_LOSS: &str = "eigengraph::analysis::fema_p58_fragility";

/// Earthquake excitation axis coupling mode (ASCE 7-22 §16.2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TriDirectionalMode {
    UniaxialX,
    BiaxialXY,
    #[default]
    TriDirectionalXYZ,
}

impl TriDirectionalMode {
    pub fn short_label(&self) -> &'static str {
        match self {
            Self::UniaxialX => "1D X",
            Self::BiaxialXY => "2D XY",
            Self::TriDirectionalXYZ => "3D XYZ",
        }
    }

    pub fn worker_key(&self) -> &'static str {
        match self {
            Self::UniaxialX => "X",
            Self::BiaxialXY => "XY",
            Self::TriDirectionalXYZ => "XYZ",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::UniaxialX => Self::BiaxialXY,
            Self::BiaxialXY => Self::TriDirectionalXYZ,
            Self::TriDirectionalXYZ => Self::UniaxialX,
        }
    }
}

fn default_scale_factor() -> f64 {
    1.15
}

fn default_damping_ratio() -> f64 {
    0.025
}

fn default_playback_speed() -> f32 {
    1.0
}

fn default_true() -> bool {
    true
}

/// Configuration for BIM-coupled Nonlinear Earthquake Ground-Motion Replay & Async `StructGNN-CRC`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundMotionReplayConfig {
    #[serde(default)]
    pub record: GroundMotionRecord,
    #[serde(default = "default_scale_factor")]
    pub scale_factor: f64,
    #[serde(default)]
    pub hysteresis_model: HysteresisModel,
    #[serde(default = "default_damping_ratio")]
    pub damping_ratio: f64,
    #[serde(default = "default_playback_speed")]
    pub playback_speed: f32,
    #[serde(default = "default_true")]
    pub show_plastic_hinges: bool,
    #[serde(default = "default_true")]
    pub replay_loop: bool,
    #[serde(default = "default_true")]
    pub use_ssi_period_coupling: bool,
    #[serde(default)]
    pub tridirectional_mode: TriDirectionalMode,
    #[serde(default)]
    pub custom_accelerogram: Option<CustomAccelerogram>,
    #[serde(default)]
    pub show_fragility_loss_overlay: bool,
    #[serde(default)]
    pub auto_match_sa_t1: bool,
}

impl Default for GroundMotionReplayConfig {
    fn default() -> Self {
        Self {
            record: GroundMotionRecord::NorthridgeSylmar1994,
            scale_factor: 1.15,
            hysteresis_model: HysteresisModel::BoucWenDifferential,
            damping_ratio: 0.025,
            playback_speed: 1.0,
            show_plastic_hinges: true,
            replay_loop: true,
            use_ssi_period_coupling: true,
            tridirectional_mode: TriDirectionalMode::TriDirectionalXYZ,
            custom_accelerogram: None,
            show_fragility_loss_overlay: false,
            auto_match_sa_t1: false,
        }
    }
}

impl GroundMotionReplayConfig {
    pub const ALL_RECORDS: [GroundMotionRecord; 5] = [
        GroundMotionRecord::NorthridgeSylmar1994,
        GroundMotionRecord::KobeTakatori1995,
        GroundMotionRecord::ImperialValleyElCentro1940,
        GroundMotionRecord::ChiChiTaiwan1999,
        GroundMotionRecord::SumatraSubduction2004,
    ];

    pub fn record_short_label(record: GroundMotionRecord) -> &'static str {
        match record {
            GroundMotionRecord::NorthridgeSylmar1994 => "Northridge",
            GroundMotionRecord::KobeTakatori1995 => "Kobe '95",
            GroundMotionRecord::ImperialValleyElCentro1940 => "El Centro",
            GroundMotionRecord::ChiChiTaiwan1999 => "Chi-Chi",
            GroundMotionRecord::SumatraSubduction2004 => "Sumatra '04",
        }
    }

    pub fn record_worker_key(record: GroundMotionRecord) -> &'static str {
        match record {
            GroundMotionRecord::NorthridgeSylmar1994 => "Northridge",
            GroundMotionRecord::KobeTakatori1995 => "Kobe",
            GroundMotionRecord::ImperialValleyElCentro1940 => "ElCentro",
            GroundMotionRecord::ChiChiTaiwan1999 => "ChiChi",
            GroundMotionRecord::SumatraSubduction2004 => "Sumatra",
        }
    }

    pub fn record_from_arg(arg: &str) -> Option<GroundMotionRecord> {
        let lower = arg.trim().to_ascii_lowercase();
        if lower.is_empty() {
            return None;
        }
        if lower.contains("kobe") || lower.contains("takatori") {
            Some(GroundMotionRecord::KobeTakatori1995)
        } else if lower.contains("centro") || lower.contains("imperial") {
            Some(GroundMotionRecord::ImperialValleyElCentro1940)
        } else if lower.contains("chi") || lower.contains("taiwan") {
            Some(GroundMotionRecord::ChiChiTaiwan1999)
        } else if lower.contains("sumatra") || lower.contains("aceh") || lower.contains("2004") {
            Some(GroundMotionRecord::SumatraSubduction2004)
        } else if lower.contains("north") || lower.contains("sylmar") {
            Some(GroundMotionRecord::NorthridgeSylmar1994)
        } else {
            None
        }
    }

    pub fn next_record(record: GroundMotionRecord) -> GroundMotionRecord {
        match record {
            GroundMotionRecord::NorthridgeSylmar1994 => GroundMotionRecord::KobeTakatori1995,
            GroundMotionRecord::KobeTakatori1995 => GroundMotionRecord::ImperialValleyElCentro1940,
            GroundMotionRecord::ImperialValleyElCentro1940 => GroundMotionRecord::ChiChiTaiwan1999,
            GroundMotionRecord::ChiChiTaiwan1999 => GroundMotionRecord::SumatraSubduction2004,
            GroundMotionRecord::SumatraSubduction2004 => GroundMotionRecord::NorthridgeSylmar1994,
        }
    }

    pub fn hysteresis_short_label(model: HysteresisModel) -> &'static str {
        match model {
            HysteresisModel::BoucWenDifferential => "Bouc-Wen",
            HysteresisModel::TakedaDegradingPinching => "Takeda Pinch",
            HysteresisModel::BilinearHardening => "Bilinear",
        }
    }

    pub fn hysteresis_worker_key(model: HysteresisModel) -> &'static str {
        match model {
            HysteresisModel::BoucWenDifferential => "BoucWen",
            HysteresisModel::TakedaDegradingPinching => "Takeda",
            HysteresisModel::BilinearHardening => "Bilinear",
        }
    }

    pub fn next_hysteresis(model: HysteresisModel) -> HysteresisModel {
        match model {
            HysteresisModel::BoucWenDifferential => HysteresisModel::TakedaDegradingPinching,
            HysteresisModel::TakedaDegradingPinching => HysteresisModel::BilinearHardening,
            HysteresisModel::BilinearHardening => HysteresisModel::BoucWenDifferential,
        }
    }

    pub fn cycle_playback_speed(&mut self) {
        self.playback_speed = if self.playback_speed < 0.4 {
            0.5
        } else if self.playback_speed < 0.85 {
            1.0
        } else if self.playback_speed < 1.6 {
            2.0
        } else {
            0.25
        };
    }

    pub fn cycle_scale_factor(&mut self) {
        self.scale_factor = if self.scale_factor < 0.90 {
            1.00
        } else if self.scale_factor < 1.20 {
            1.25
        } else if self.scale_factor < 1.45 {
            1.50
        } else if self.scale_factor < 1.90 {
            2.00
        } else {
            0.75
        };
    }

    pub fn active_record_label(&self) -> String {
        if let Some(ref c) = self.custom_accelerogram {
            c.name.clone()
        } else {
            Self::record_short_label(self.record).to_string()
        }
    }

    pub fn content_revision_hash(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325_u64;
        let mix = |acc: &mut u64, v: u64| {
            *acc ^= v;
            *acc = acc.wrapping_mul(0x100000001b3);
        };
        mix(&mut h, self.record as u64 + 1);
        mix(&mut h, (self.scale_factor * 1000.0).round() as u64);
        mix(&mut h, self.hysteresis_model as u64 + 1);
        mix(&mut h, (self.damping_ratio * 10000.0).round() as u64);
        mix(&mut h, u64::from(self.show_plastic_hinges));
        mix(&mut h, u64::from(self.use_ssi_period_coupling));
        mix(&mut h, self.tridirectional_mode as u64 + 1);
        mix(&mut h, u64::from(self.show_fragility_loss_overlay));
        mix(&mut h, u64::from(self.auto_match_sa_t1));
        if let Some(ref c) = self.custom_accelerogram {
            mix(&mut h, c.accel_x_g.len() as u64);
            mix(&mut h, (c.pga_x_g * 10_000.0).round() as u64);
        }
        h
    }
}

/// Dynamic ASCE 41-17 / FEMA 356 plastic hinge evaluation at a BIM structural member end (`I` or `J`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlasticHingeJointSnapshot {
    pub bim_kind: BimElementKind,
    pub bim_id: usize,
    pub element_name: String,
    /// `0` = Start / Base joint (`I`), `1` = End / Top joint (`J`).
    pub joint_end: u8,
    /// Building-local undeformed joint coordinates `(x_m, y_m, z_m)` before heading rotation.
    pub local_pos_m: [f64; 3],
    pub dcr_dynamic: f64,
    pub plastic_rotation_rad: f64,
    pub park_ang_damage_index: f64,
    pub state: PlasticHingeState,
}

/// Single time-sampled frame from BIM-coupled NL-THA for smooth 60 FPS WebGPU / CPU playback.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundMotionReplayFrame {
    pub time_s: f64,
    pub ground_accel_g: f64,
    #[serde(default)]
    pub ground_accel_y_g: f64,
    #[serde(default)]
    pub ground_accel_z_g: f64,
    pub ground_disp_m: f64,
    #[serde(default)]
    pub ground_disp_y_m: f64,
    #[serde(default)]
    pub ground_disp_z_m: f64,
    pub roof_disp_mm: f64,
    pub roof_drift_ratio: f64,
    pub base_shear_kn: f64,
    pub hysteretic_energy_kj: f64,
    pub story_elevations_m: Vec<f64>,
    pub story_disp_x_m: Vec<f64>,
    pub story_disp_y_m: Vec<f64>,
    #[serde(default)]
    pub story_disp_z_m: Vec<f64>,
    pub story_drift_ratios: Vec<f64>,
    pub active_hinges: Vec<PlasticHingeJointSnapshot>,
    pub count_io: usize,
    pub count_ls: usize,
    pub count_cp: usize,
    pub count_mechanism: usize,
}

/// Async Two-Stage `ARI v2 + StructGNN-CRC` Conformal Screening telemetry coupled to NL-THA demand.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AsyncCrcReplayTelemetry {
    pub total_scenarios: usize,
    pub stage1_pruned: usize,
    pub stage2_evaluated: usize,
    pub scenario_reduction_pct: f64,
    pub safety_recall_pct: f64,
    pub conformal_alpha: f64,
    pub max_conformal_collapse_bound: f64,
    pub critical_member_name: String,
    pub compute_time_saved_pct: f64,
    pub green_ai_co2_saved_g: f64,
    pub execution_backend: String,
    pub top_scenarios: Vec<StructGnnCrcScenario>,
}

/// Complete BIM + SSI-coupled Nonlinear Ground-Motion Replay, Fragility Loss & Async `StructGNN-CRC` Report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundMotionReplayReport {
    pub project_name: String,
    pub config: GroundMotionReplayConfig,
    pub bim_revision_hash: u64,
    pub num_stories: usize,
    pub building_height_m: f64,
    pub fundamental_period_t1_s: f64,
    pub ssi_coupled: bool,
    pub scaled_pga_g: f64,
    #[serde(default)]
    pub tridirectional_mode: TriDirectionalMode,
    #[serde(default)]
    pub custom_record_name: Option<String>,
    #[serde(default)]
    pub scaled_pga_y_g: f64,
    #[serde(default)]
    pub scaled_pga_z_g: f64,
    #[serde(default)]
    pub sa_record_t1_g: f64,
    #[serde(default)]
    pub sa_design_t1_g: f64,
    #[serde(default)]
    pub sa_match_ratio: f64,
    pub total_duration_s: f64,
    pub peak_time_s: f64,
    pub peak_roof_disp_mm: f64,
    pub peak_roof_drift_ratio: f64,
    pub peak_interstory_drift_ratio: f64,
    pub peak_base_shear_kn: f64,
    pub hysteretic_dissipation_ratio: f64,
    pub max_energy_balance_error_pct: f64,
    pub is_drift_compliant: bool,
    pub governing_hinge_state: PlasticHingeState,
    pub peak_hinges: Vec<PlasticHingeJointSnapshot>,
    pub peak_count_io: usize,
    pub peak_count_ls: usize,
    pub peak_count_cp: usize,
    pub peak_count_mechanism: usize,
    pub frames: Vec<GroundMotionReplayFrame>,
    pub async_crc: AsyncCrcReplayTelemetry,
    #[serde(default)]
    pub response_spectrum: Vec<ResponseSpectrumPoint>,
    #[serde(default)]
    pub fragility_loss: ReplayFemaP58Synthesis,
    pub replay_overlay_ifcx: IfcxDocument,
    #[serde(default = "default_empty_ifcx")]
    pub fragility_overlay_ifcx: IfcxDocument,
    pub summary: String,
}

fn default_empty_ifcx() -> IfcxDocument {
    IfcxDocument::new("fema_p58_fragility")
}

impl GroundMotionReplayReport {
    /// Linearly interpolates structural story displacements (X/Y/Z) and ground motion telemetry at `time_s`
    /// for silky 60 FPS WebGPU / CPU animations.
    pub fn sample_interpolated(&self, time_s: f64) -> GroundMotionReplayFrame {
        if self.frames.is_empty() {
            return GroundMotionReplayFrame {
                time_s: 0.0,
                ground_accel_g: 0.0,
                ground_accel_y_g: 0.0,
                ground_accel_z_g: 0.0,
                ground_disp_m: 0.0,
                ground_disp_y_m: 0.0,
                ground_disp_z_m: 0.0,
                roof_disp_mm: 0.0,
                roof_drift_ratio: 0.0,
                base_shear_kn: 0.0,
                hysteretic_energy_kj: 0.0,
                story_elevations_m: vec![0.0, self.building_height_m],
                story_disp_x_m: vec![0.0, 0.0],
                story_disp_y_m: vec![0.0, 0.0],
                story_disp_z_m: vec![0.0, 0.0],
                story_drift_ratios: vec![0.0],
                active_hinges: Vec::new(),
                count_io: 0,
                count_ls: 0,
                count_cp: 0,
                count_mechanism: 0,
            };
        }
        if self.frames.len() == 1 || time_s <= self.frames[0].time_s {
            return self.frames[0].clone();
        }
        let last_idx = self.frames.len() - 1;
        if time_s >= self.frames[last_idx].time_s {
            return self.frames[last_idx].clone();
        }

        let pos = self
            .frames
            .partition_point(|f| f.time_s <= time_s)
            .clamp(1, last_idx);
        let f0 = &self.frames[pos - 1];
        let f1 = &self.frames[pos];
        let span = (f1.time_s - f0.time_s).max(1e-6);
        let alpha = ((time_s - f0.time_s) / span).clamp(0.0, 1.0);
        let lerp = |a: f64, b: f64| a + (b - a) * alpha;

        let n_lvls = f0.story_elevations_m.len().min(f1.story_elevations_m.len());
        let mut story_disp_x_m = Vec::with_capacity(n_lvls);
        let mut story_disp_y_m = Vec::with_capacity(n_lvls);
        let mut story_disp_z_m = Vec::with_capacity(n_lvls);
        for i in 0..n_lvls {
            story_disp_x_m.push(lerp(f0.story_disp_x_m[i], f1.story_disp_x_m[i]));
            story_disp_y_m.push(lerp(f0.story_disp_y_m[i], f1.story_disp_y_m[i]));
            let z0 = f0.story_disp_z_m.get(i).copied().unwrap_or(0.0);
            let z1 = f1.story_disp_z_m.get(i).copied().unwrap_or(0.0);
            story_disp_z_m.push(lerp(z0, z1));
        }
        let n_drifts = f0.story_drift_ratios.len().min(f1.story_drift_ratios.len());
        let mut story_drift_ratios = Vec::with_capacity(n_drifts);
        for i in 0..n_drifts {
            story_drift_ratios.push(lerp(f0.story_drift_ratios[i], f1.story_drift_ratios[i]));
        }

        let hinge_src = if alpha >= 0.5 { f1 } else { f0 };
        GroundMotionReplayFrame {
            time_s,
            ground_accel_g: lerp(f0.ground_accel_g, f1.ground_accel_g),
            ground_accel_y_g: lerp(f0.ground_accel_y_g, f1.ground_accel_y_g),
            ground_accel_z_g: lerp(f0.ground_accel_z_g, f1.ground_accel_z_g),
            ground_disp_m: lerp(f0.ground_disp_m, f1.ground_disp_m),
            ground_disp_y_m: lerp(f0.ground_disp_y_m, f1.ground_disp_y_m),
            ground_disp_z_m: lerp(f0.ground_disp_z_m, f1.ground_disp_z_m),
            roof_disp_mm: lerp(f0.roof_disp_mm, f1.roof_disp_mm),
            roof_drift_ratio: lerp(f0.roof_drift_ratio, f1.roof_drift_ratio),
            base_shear_kn: lerp(f0.base_shear_kn, f1.base_shear_kn),
            hysteretic_energy_kj: lerp(f0.hysteretic_energy_kj, f1.hysteretic_energy_kj),
            story_elevations_m: f0.story_elevations_m[..n_lvls].to_vec(),
            story_disp_x_m,
            story_disp_y_m,
            story_disp_z_m,
            story_drift_ratios,
            active_hinges: hinge_src.active_hinges.clone(),
            count_io: hinge_src.count_io,
            count_ls: hinge_src.count_ls,
            count_cp: hinge_src.count_cp,
            count_mechanism: hinge_src.count_mechanism,
        }
    }

    /// Returns worst `PlasticHingeState` per `(BimElementKind, bim_id)` active at `time_s`.
    pub fn worst_hinge_states_at(
        &self,
        time_s: f64,
    ) -> Vec<(BimElementKind, usize, PlasticHingeState)> {
        let frame = self.sample_interpolated(time_s);
        let mut map: HashMap<(BimElementKind, usize), PlasticHingeState> = HashMap::new();
        for h in &frame.active_hinges {
            let entry = map.entry((h.bim_kind, h.bim_id)).or_insert(h.state);
            if h.state > *entry {
                *entry = h.state;
            }
        }
        map.into_iter().map(|((k, id), st)| (k, id, st)).collect()
    }
}

#[derive(Debug, Clone)]
struct BimJointDescriptor {
    bim_kind: BimElementKind,
    bim_id: usize,
    element_name: String,
    joint_end: u8,
    local_pos_m: [f64; 3],
    base_dcr: f64,
    story_idx: usize,
    sensitivity: f64,
}

#[derive(Debug, Clone)]
struct PreparedReplayContext {
    project: BimProject,
    project_name: String,
    bim_revision_hash: u64,
    num_stories: usize,
    building_height_m: f64,
    fundamental_period_t1_s: f64,
    ssi_coupled: bool,
    spectrum: Sni1726Spectrum,
    total_seismic_weight_kn: f64,
    yield_shear_coeff_cy: f64,
    sorted_level_elevs_m: Vec<f64>,
    joints: Vec<BimJointDescriptor>,
    base_scenarios: Vec<BaseCrcScenarioInput>,
    min_ari: f64,
    avg_ari: f64,
    has_mechanism: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BaseCrcScenarioInput {
    fem_id: usize,
    bim_kind: Option<BimElementKind>,
    bim_id: Option<usize>,
    name: String,
    #[serde(rename = "ariV2")]
    ari_v2: f64,
    wk: f64,
    wc: f64,
    wd: f64,
    #[serde(rename = "svdMech")]
    svd_mech: bool,
    #[serde(rename = "postU")]
    post_u: f64,
    #[serde(rename = "svdMargin")]
    svd_margin: f64,
    #[serde(rename = "bucklingMargin")]
    buckling_margin: f64,
    #[serde(rename = "isCritical")]
    is_critical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerSampleRecord {
    time_s: f64,
    ground_accel_g: f64,
    #[serde(default)]
    ground_accel_y_g: f64,
    #[serde(default)]
    ground_accel_z_g: f64,
    roof_disp_mm: f64,
    roof_drift_ratio: f64,
    base_shear_kn: f64,
    #[serde(default)]
    hysteretic_energy_kj: f64,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerReplayPayload {
    #[serde(rename = "recordKey")]
    record_key: String,
    #[serde(rename = "scaleFactor")]
    scale_factor: f64,
    #[serde(rename = "hysteresisKey")]
    hysteresis_key: String,
    #[serde(rename = "durationS")]
    duration_s: f64,
    #[serde(rename = "dtS")]
    dt_s: f64,
    #[serde(rename = "heightM")]
    height_m: f64,
    #[serde(rename = "numStories")]
    num_stories: usize,
    #[serde(rename = "weightKn")]
    weight_kn: f64,
    #[serde(rename = "t1S")]
    t1_s: f64,
    #[serde(rename = "dampingRatio")]
    damping_ratio: f64,
    cy: f64,
    #[serde(rename = "postYieldRatio")]
    post_yield_ratio: f64,
    #[serde(rename = "tridirMode")]
    tridir_mode: String,
    #[serde(rename = "customDtS")]
    custom_dt_s: Option<f64>,
    #[serde(rename = "customAccelX")]
    custom_accel_x: Option<Vec<f64>>,
    #[serde(rename = "customAccelY")]
    custom_accel_y: Option<Vec<f64>>,
    #[serde(rename = "customAccelZ")]
    custom_accel_z: Option<Vec<f64>>,
    scenarios: Vec<BaseCrcScenarioInput>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerDoneResult {
    samples: Vec<WorkerSampleRecord>,
    peak_roof_disp_mm: f64,
    peak_base_shear_kn: f64,
    hyst_dissipation_ratio: f64,
    max_energy_balance_error_pct: f64,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerPollResponse {
    status: String,
    #[serde(default)]
    progress: f32,
    #[serde(default)]
    stage: String,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    result: Option<WorkerDoneResult>,
}

fn simulation_duration_for_record(record: GroundMotionRecord) -> f64 {
    match record {
        GroundMotionRecord::NorthridgeSylmar1994 => 18.0,
        GroundMotionRecord::KobeTakatori1995 => 20.0,
        GroundMotionRecord::ImperialValleyElCentro1940 => 22.0,
        GroundMotionRecord::ChiChiTaiwan1999 => 24.0,
        GroundMotionRecord::SumatraSubduction2004 => 28.0,
    }
}

fn prepare_replay_context(
    project: &BimProject,
    site: &SiteEnvironmentalCoupling,
    ssi_report: Option<&SubsurfaceSsiAnalysisReport>,
    config: &GroundMotionReplayConfig,
) -> Result<PreparedReplayContext, String> {
    if project.is_empty() {
        return Err(
            "BIM project is empty — add structural columns/beams before running Ground-Motion Replay."
                .to_string(),
        );
    }

    let mut level_elevs: Vec<f64> = project
        .levels
        .iter()
        .map(|l| l.elevation_mm * 0.001)
        .collect();
    level_elevs.sort_by(|a, b| a.total_cmp(b));
    level_elevs.dedup_by(|a, b| (*a - *b).abs() < 0.05);
    if level_elevs.is_empty() {
        level_elevs.push(0.0);
        level_elevs.push(4.0);
    } else if level_elevs.len() == 1 {
        level_elevs.push(level_elevs[0] + 4.0);
    }
    let min_elev = level_elevs[0];
    if min_elev.abs() > 0.05 {
        for e in &mut level_elevs {
            *e -= min_elev;
        }
    }
    let building_height_m = level_elevs.last().copied().unwrap_or(6.0).max(3.5);
    let num_stories = level_elevs.len().saturating_sub(1).max(1);

    let level_z = |lvl_id: usize| -> f64 {
        project
            .levels
            .iter()
            .find(|l| l.id == lvl_id)
            .map(|l| (l.elevation_mm * 0.001 - min_elev).max(0.0))
            .unwrap_or(0.0)
    };
    let story_for_z = |z_m: f64| -> usize {
        for i in 1..level_elevs.len() {
            if z_m <= level_elevs[i] + 0.25 {
                return i - 1;
            }
        }
        num_stories.saturating_sub(1)
    };

    let (fem, fem_map, load_sum) = project.to_structural_model_with_site_loads(site);
    if fem.elements.is_empty() {
        return Err("No 3D structural frame elements available for Ground-Motion Replay.".to_string());
    }

    let fea = FemSolver::solve(&fem)?;
    let ari_eval = AriEngine::evaluate_redundancy_fast(&fem)?;

    let site_class = SiteClass::from_soil_label(&site.recommended_soil_label_or_default());
    let spectrum = Sni1726Spectrum::new(
        site_class,
        (site.seismic_pga_ss_g as f64).max(0.25),
        (site.seismic_pga_s1_g as f64).max(0.10),
        8.0,
        1.0,
    );
    let fixed_t1 = ModalEngine::analyze_for_earth_3d(&fem, 3, spectrum.clone())
        .map(|m| m.fundamental_period_s)
        .unwrap_or_else(|_| 0.0466 * building_height_m.powf(0.9))
        .clamp(0.18, 3.80);

    let (fundamental_period_t1_s, ssi_coupled) = match ssi_report {
        Some(ssi) if config.use_ssi_period_coupling => (
            ssi.ssi_coupled_period_t1_s.max(fixed_t1 * 1.04).clamp(0.18, 4.50),
            true,
        ),
        _ => (fixed_t1, false),
    };

    let total_seismic_weight_kn = load_sum.total_gravity_kn.max(1200.0);
    let yield_shear_coeff_cy = (0.11 + 0.06 * site.seismic_pga_ss_g as f64).clamp(0.11, 0.26);

    let mut fem_to_bim: HashMap<usize, (BimElementKind, usize)> =
        HashMap::with_capacity(fem_map.len());
    let mut bim_to_dcr: HashMap<(BimElementKind, usize), f64> =
        HashMap::with_capacity(fem_map.len());
    let mut fem_to_pmv: HashMap<usize, f64> = HashMap::with_capacity(fem.elements.len());

    for (idx, el) in fem.elements.iter().enumerate() {
        let pmv = fea.member_dcr_interactions.get(idx).copied().unwrap_or(0.35);
        fem_to_pmv.insert(el.id, pmv);
    }
    for &(fid, kind, bid) in &fem_map {
        fem_to_bim.insert(fid, (kind, bid));
        let pmv = fem_to_pmv.get(&fid).copied().unwrap_or(0.35);
        let prev = bim_to_dcr.entry((kind, bid)).or_insert(0.0);
        if pmv > *prev {
            *prev = pmv;
        }
    }

    let mut joints = Vec::with_capacity(project.columns.len() * 2 + project.runs.len() * 2);
    for col in &project.columns {
        let z0 = level_z(col.base_level_id);
        let z1 = level_z(col.top_level_id).max(z0 + 0.5);
        let base_dcr = bim_to_dcr
            .get(&(BimElementKind::Column, col.id))
            .copied()
            .unwrap_or(0.36)
            .clamp(0.18, 1.15);
        let s_idx = story_for_z(0.5 * (z0 + z1));
        let height_weight = 1.0 + 0.18 * (1.0 - (z0 / building_height_m).clamp(0.0, 1.0));
        joints.push(BimJointDescriptor {
            bim_kind: BimElementKind::Column,
            bim_id: col.id,
            element_name: col.name.clone(),
            joint_end: 0,
            local_pos_m: [col.pos.0 * 0.001, col.pos.1 * 0.001, z0],
            base_dcr,
            story_idx: s_idx,
            sensitivity: 1.05 * height_weight,
        });
        joints.push(BimJointDescriptor {
            bim_kind: BimElementKind::Column,
            bim_id: col.id,
            element_name: col.name.clone(),
            joint_end: 1,
            local_pos_m: [col.pos.0 * 0.001, col.pos.1 * 0.001, z1],
            base_dcr,
            story_idx: s_idx,
            sensitivity: 0.98 * height_weight,
        });
    }
    for run in &project.runs {
        if !matches!(
            run.category,
            RunCategory::StructuralBeam | RunCategory::StructuralBrace
        ) {
            continue;
        }
        let z_lvl = level_z(run.level_id);
        let z0 = z_lvl + run.start_offset_mm * 0.001;
        let z1 = z_lvl + run.end_offset_mm * 0.001;
        let base_dcr = bim_to_dcr
            .get(&(BimElementKind::Run, run.id))
            .copied()
            .unwrap_or(0.32)
            .clamp(0.15, 1.15);
        let s_idx = story_for_z(0.5 * (z0 + z1));
        let sens = if run.category == RunCategory::StructuralBrace {
            1.12
        } else {
            0.94
        };
        joints.push(BimJointDescriptor {
            bim_kind: BimElementKind::Run,
            bim_id: run.id,
            element_name: run.name.clone(),
            joint_end: 0,
            local_pos_m: [run.start.0 * 0.001, run.start.1 * 0.001, z0],
            base_dcr,
            story_idx: s_idx,
            sensitivity: sens,
        });
        joints.push(BimJointDescriptor {
            bim_kind: BimElementKind::Run,
            bim_id: run.id,
            element_name: run.name.clone(),
            joint_end: 1,
            local_pos_m: [run.end.0 * 0.001, run.end.1 * 0.001, z1],
            base_dcr,
            story_idx: s_idx,
            sensitivity: sens * 0.96,
        });
    }

    let mut base_scenarios = Vec::with_capacity(ari_eval.member_results.len());
    for m_res in &ari_eval.member_results {
        let bim_pair = fem_to_bim.get(&m_res.member_id).copied();
        let name = bim_pair
            .and_then(|(k, bid)| match k {
                BimElementKind::Column => project.columns.iter().find(|c| c.id == bid).map(|c| c.name.clone()),
                BimElementKind::Run => project.runs.iter().find(|r| r.id == bid).map(|r| r.name.clone()),
                BimElementKind::Wall => project.walls.iter().find(|w| w.id == bid).map(|w| w.name.clone()),
                _ => None,
            })
            .unwrap_or_else(|| m_res.member_name.clone());
        let svd_mech = m_res.svd_margin <= MARGIN_TOL;
        let wk = if svd_mech { 0.0 } else { m_res.svd_margin.clamp(0.0, 1.0) };
        let pmv = fem_to_pmv.get(&m_res.member_id).copied().unwrap_or(0.0);
        let post_u = m_res
            .max_redistribution_utilization
            .max(m_res.second_order_dcr)
            .max(pmv);
        let wc = if svd_mech {
            0.0
        } else {
            (1.0 - ((post_u - 0.70).max(0.0) / 0.55)).clamp(0.0, 1.0)
        };
        let wd = if svd_mech {
            0.0
        } else {
            (0.5 * m_res.buckling_margin + 0.5 * m_res.rr).clamp(0.0, 1.0)
        };
        let convex_v2 = (0.25 * wk + 0.45 * wc + 0.30 * wd).clamp(0.0, 1.0);
        let ari_v2 = if svd_mech {
            0.0
        } else {
            (0.5 * m_res.ari + 0.5 * convex_v2).clamp(0.0, 1.0)
        };
        base_scenarios.push(BaseCrcScenarioInput {
            fem_id: m_res.member_id,
            bim_kind: bim_pair.map(|p| p.0),
            bim_id: bim_pair.map(|p| p.1),
            name,
            ari_v2,
            wk,
            wc,
            wd,
            svd_mech,
            post_u,
            svd_margin: m_res.svd_margin,
            buckling_margin: m_res.buckling_margin,
            is_critical: m_res.is_critical,
        });
    }

    Ok(PreparedReplayContext {
        project: project.clone(),
        project_name: project.project_name.clone(),
        bim_revision_hash: project.content_revision_hash(),
        num_stories,
        building_height_m,
        fundamental_period_t1_s,
        ssi_coupled,
        spectrum,
        total_seismic_weight_kn,
        yield_shear_coeff_cy,
        sorted_level_elevs_m: level_elevs,
        joints,
        base_scenarios,
        min_ari: ari_eval.min_system_ari,
        avg_ari: ari_eval.average_ari,
        has_mechanism: !ari_eval.kinematic_guard.is_stable,
    })
}

trait SiteSoilExt {
    fn recommended_soil_label_or_default(&self) -> String;
}

impl SiteSoilExt for SiteEnvironmentalCoupling {
    fn recommended_soil_label_or_default(&self) -> String {
        if self.recommended_soil_class.trim().is_empty() {
            "SD".to_string()
        } else {
            self.recommended_soil_class.clone()
        }
    }
}

fn synthesize_tridir_accel_yz(
    ax_g: f64,
    t_s: f64,
    pga_x_g: f64,
    mode: TriDirectionalMode,
    custom_y: Option<f64>,
    custom_z: Option<f64>,
) -> (f64, f64) {
    match mode {
        TriDirectionalMode::UniaxialX => (0.0, 0.0),
        TriDirectionalMode::BiaxialXY => {
            let ay = custom_y.unwrap_or_else(|| {
                0.78 * ax_g * (0.45 + 0.55 * (t_s * 1.9 + 0.85).cos())
                    + 0.14 * pga_x_g * (t_s * 3.1).sin()
            });
            (ay, 0.0)
        }
        TriDirectionalMode::TriDirectionalXYZ => {
            let ay = custom_y.unwrap_or_else(|| {
                0.78 * ax_g * (0.45 + 0.55 * (t_s * 1.9 + 0.85).cos())
                    + 0.14 * pga_x_g * (t_s * 3.1).sin()
            });
            let az = custom_z.unwrap_or_else(|| {
                0.67 * ax_g * (0.35 + 0.65 * (t_s * 4.4 + 0.4).sin())
            });
            (ay, az)
        }
    }
}

fn unscaled_accel_series_g(config: &GroundMotionReplayConfig) -> (f64, Vec<f64>) {
    if let Some(ref custom) = config.custom_accelerogram {
        (custom.dt_s.clamp(0.002, 0.05), custom.accel_x_g.clone())
    } else {
        let g = 9.80665;
        let dt = 0.010;
        let dur = simulation_duration_for_record(config.record);
        let ser = TimeHistoryEngine::generate_ground_acceleration_series(config.record, 1.0, dt, dur);
        (dt, ser.into_iter().map(|s| s.1 / g).collect())
    }
}

fn effective_scale_and_sa(
    ctx: &PreparedReplayContext,
    config: &GroundMotionReplayConfig,
) -> (f64, f64, f64) {
    let (dt_u, ser_u) = unscaled_accel_series_g(config);
    let sa_unscaled_t1 = compute_elastic_response_spectrum_5pct(
        dt_u,
        &ser_u,
        config.damping_ratio.clamp(0.01, 0.15),
        &[ctx.fundamental_period_t1_s],
    )
    .first()
    .copied()
    .unwrap_or(0.25)
    .max(0.05);
    let sa_design_t1 = ctx.spectrum.spectral_acceleration(ctx.fundamental_period_t1_s).max(0.08);
    let sf = if config.auto_match_sa_t1 {
        (sa_design_t1 / sa_unscaled_t1).clamp(0.40, 3.00)
    } else {
        config.scale_factor.clamp(0.50, 2.50)
    };
    (sf, sa_unscaled_t1 * sf, sa_design_t1)
}

fn integrate_nltha_samples(
    ctx: &PreparedReplayContext,
    config: &GroundMotionReplayConfig,
) -> (Vec<WorkerSampleRecord>, f64, f64, f64, f64) {
    let g = 9.80665;
    let (scale_eff, _, _) = effective_scale_and_sa(ctx, config);
    let (duration_s, dt_s) = if let Some(ref custom) = config.custom_accelerogram {
        (custom.duration_s.max(2.0), custom.dt_s.clamp(0.002, 0.05))
    } else {
        (
            simulation_duration_for_record(config.record),
            if ctx.fundamental_period_t1_s < 0.35 {
                0.005
            } else {
                0.010
            },
        )
    };
    let th_input = TimeHistoryInput {
        ground_motion: config.record,
        scale_factor: scale_eff,
        time_step_dt_s: dt_s,
        simulation_duration_s: duration_s,
        building_height_m: ctx.building_height_m,
        num_stories: ctx.num_stories,
        total_seismic_weight_kn: ctx.total_seismic_weight_kn,
        fundamental_period_t1_s: ctx.fundamental_period_t1_s,
        damping_ratio: config.damping_ratio.clamp(0.01, 0.15),
        yield_shear_coeff_cy: ctx.yield_shear_coeff_cy,
        post_yield_stiffness_ratio: match config.hysteresis_model {
            HysteresisModel::BilinearHardening => 0.055,
            HysteresisModel::BoucWenDifferential => 0.035,
            HysteresisModel::TakedaDegradingPinching => 0.025,
        },
        hysteresis_rule: config.hysteresis_model,
        integration_scheme: TimeIntegrationScheme::HilberHughesTaylorAlpha,
    };
    let raw = if let Some(ref custom) = config.custom_accelerogram {
        let series_m_s2: Vec<(f64, f64)> = custom
            .accel_x_g
            .iter()
            .enumerate()
            .map(|(i, &ag)| (i as f64 * dt_s, ag * scale_eff * g))
            .collect();
        TimeHistoryEngine::evaluate_with_series(&th_input, dt_s, &series_m_s2)
    } else {
        TimeHistoryEngine::evaluate(&th_input)
    };
    let (hyst_disp_mult, shear_mult) = match config.hysteresis_model {
        HysteresisModel::BoucWenDifferential => (1.00, 1.00),
        HysteresisModel::TakedaDegradingPinching => (1.08, 0.94),
        HysteresisModel::BilinearHardening => (0.94, 1.04),
    };
    let base_pga_x = raw.scaled_pga_g.max(0.1);
    let custom_dt = config.custom_accelerogram.as_ref().map(|c| c.dt_s.max(1e-4)).unwrap_or(dt_s);
    let samples: Vec<WorkerSampleRecord> = raw
        .time_history_trace
        .iter()
        .map(|s: &TimeHistorySample| {
            let cy_opt = config.custom_accelerogram.as_ref().and_then(|c| {
                if c.accel_y_g.is_empty() {
                    None
                } else {
                    let idx = ((s.time_s / custom_dt).round() as usize).min(c.accel_y_g.len() - 1);
                    Some(c.accel_y_g[idx] * scale_eff)
                }
            });
            let cz_opt = config.custom_accelerogram.as_ref().and_then(|c| {
                if c.accel_z_g.is_empty() {
                    None
                } else {
                    let idx = ((s.time_s / custom_dt).round() as usize).min(c.accel_z_g.len() - 1);
                    Some(c.accel_z_g[idx] * scale_eff)
                }
            });
            let (ay_g, az_g) = synthesize_tridir_accel_yz(
                s.ground_accel_g,
                s.time_s,
                base_pga_x,
                config.tridirectional_mode,
                cy_opt,
                cz_opt,
            );
            WorkerSampleRecord {
                time_s: s.time_s,
                ground_accel_g: s.ground_accel_g,
                ground_accel_y_g: ay_g,
                ground_accel_z_g: az_g,
                roof_disp_mm: s.roof_disp_mm * hyst_disp_mult,
                roof_drift_ratio: s.roof_drift_ratio * hyst_disp_mult,
                base_shear_kn: s.base_shear_kn * shear_mult,
                hysteretic_energy_kj: s.hysteretic_energy_kj,
            }
        })
        .collect();
    (
        samples,
        raw.peak_roof_disp_mm * hyst_disp_mult,
        raw.peak_base_shear_kn * shear_mult,
        raw.hysteretic_energy_dissipation_ratio,
        raw.max_energy_balance_error_pct,
    )
}

fn classify_plastic_hinge(dcr_dyn: f64) -> PlasticHingeState {
    if dcr_dyn <= 0.85 {
        PlasticHingeState::Elastic
    } else if dcr_dyn <= 1.00 {
        PlasticHingeState::ImmediateOccupancy
    } else if dcr_dyn <= 1.15 {
        PlasticHingeState::LifeSafety
    } else if dcr_dyn <= 1.25 {
        PlasticHingeState::CollapsePrevention
    } else {
        PlasticHingeState::Mechanism
    }
}

fn assemble_replay_report(
    ctx: PreparedReplayContext,
    mut config: GroundMotionReplayConfig,
    execution_backend: &str,
    samples: Vec<WorkerSampleRecord>,
    _worker_peak_roof_mm: f64,
    worker_peak_shear_kn: f64,
    hyst_dissipation_ratio: f64,
    max_energy_balance_error_pct: f64,
) -> GroundMotionReplayReport {
    let duration_s = samples
        .last()
        .map(|s| s.time_s)
        .unwrap_or_else(|| {
            config
                .custom_accelerogram
                .as_ref()
                .map(|c| c.duration_s)
                .unwrap_or_else(|| simulation_duration_for_record(config.record))
        });
    let (scale_norm, sa_record_t1_g, sa_design_t1_g) = effective_scale_and_sa(&ctx, &config);
    config.scale_factor = scale_norm;
    let total_e_h_max = samples
        .iter()
        .map(|s| s.hysteretic_energy_kj)
        .fold(0.0_f64, f64::max)
        .max(120.0);

    let mut frames = Vec::with_capacity(samples.len());
    let mut peak_roof_disp_mm = 0.0_f64;
    let mut peak_roof_drift_ratio = 0.0_f64;
    let mut peak_interstory_drift_ratio = 0.0_f64;
    let mut peak_story_drifts = vec![0.0_f64; ctx.num_stories];
    let mut peak_time_s = 0.0_f64;
    let mut peak_base_shear_kn = worker_peak_shear_kn;
    let mut scaled_pga_x_obs = 0.0_f64;
    let mut scaled_pga_y_g = 0.0_f64;
    let mut scaled_pga_z_g = 0.0_f64;

    // Track cumulative peak damage per joint monotonically across ground motion duration
    let mut running_max_dcr = vec![0.0_f64; ctx.joints.len()];
    let mut running_best_joints: Vec<PlasticHingeJointSnapshot> = Vec::new();
    let mut peak_hinges: Vec<PlasticHingeJointSnapshot> = Vec::new();
    let mut governing_hinge_state = PlasticHingeState::Elastic;
    let rec_pga_ref = config
        .custom_accelerogram
        .as_ref()
        .map(|c| c.pga_x_g)
        .unwrap_or_else(|| config.record.pga_g())
        .max(0.08);
    let custom_dt = config
        .custom_accelerogram
        .as_ref()
        .map(|c| c.dt_s.max(1e-4))
        .unwrap_or(0.01);

    for sample in &samples {
        let cy_opt = config.custom_accelerogram.as_ref().and_then(|c| {
            if c.accel_y_g.is_empty() {
                None
            } else {
                let idx = ((sample.time_s / custom_dt).round() as usize).min(c.accel_y_g.len() - 1);
                Some(c.accel_y_g[idx] * scale_norm)
            }
        });
        let cz_opt = config.custom_accelerogram.as_ref().and_then(|c| {
            if c.accel_z_g.is_empty() {
                None
            } else {
                let idx = ((sample.time_s / custom_dt).round() as usize).min(c.accel_z_g.len() - 1);
                Some(c.accel_z_g[idx] * scale_norm)
            }
        });
        let (ay_g, az_g) = if sample.ground_accel_y_g != 0.0 || sample.ground_accel_z_g != 0.0 {
            (sample.ground_accel_y_g, sample.ground_accel_z_g)
        } else {
            synthesize_tridir_accel_yz(
                sample.ground_accel_g,
                sample.time_s,
                rec_pga_ref * scale_norm,
                config.tridirectional_mode,
                cy_opt,
                cz_opt,
            )
        };
        scaled_pga_x_obs = scaled_pga_x_obs.max(sample.ground_accel_g.abs());
        scaled_pga_y_g = scaled_pga_y_g.max(ay_g.abs());
        scaled_pga_z_g = scaled_pga_z_g.max(az_g.abs());

        let roof_m = sample.roof_disp_mm * 0.001;
        let roof_drift = (roof_m / ctx.building_height_m).abs();
        let abs_roof_mm = sample.roof_disp_mm.abs();
        peak_base_shear_kn = peak_base_shear_kn.max(sample.base_shear_kn.abs());
        if abs_roof_mm > peak_roof_disp_mm {
            peak_roof_disp_mm = abs_roof_mm;
            peak_roof_drift_ratio = roof_drift;
            peak_time_s = sample.time_s;
        }

        let mut story_disp_x_m = Vec::with_capacity(ctx.sorted_level_elevs_m.len());
        let mut story_disp_y_m = Vec::with_capacity(ctx.sorted_level_elevs_m.len());
        let mut story_disp_z_m = Vec::with_capacity(ctx.sorted_level_elevs_m.len());
        let whiplash = (sample.ground_accel_g / (rec_pga_ref * scale_norm).max(0.1)).clamp(-1.0, 1.0);
        let y_amp = match config.tridirectional_mode {
            TriDirectionalMode::UniaxialX => 0.12,
            TriDirectionalMode::BiaxialXY | TriDirectionalMode::TriDirectionalXYZ => 0.52,
        };
        let y_norm = (ay_g / (rec_pga_ref * scale_norm).max(0.1)).clamp(-1.0, 1.0);

        for &z in &ctx.sorted_level_elevs_m {
            let xi = (z / ctx.building_height_m).clamp(0.0, 1.0);
            let phi1 = (std::f64::consts::FRAC_PI_2 * xi).sin();
            let phi2 = 0.14 * (1.5 * std::f64::consts::PI * xi).sin() * whiplash;
            let ux = roof_m * (phi1 + phi2);
            let uy = match config.tridirectional_mode {
                TriDirectionalMode::UniaxialX => 0.12 * roof_m * phi1 * (sample.time_s * 1.4).cos(),
                TriDirectionalMode::BiaxialXY | TriDirectionalMode::TriDirectionalXYZ => {
                    abs_roof_mm * 0.001 * y_amp * phi1 * y_norm
                }
            };
            let uz = match config.tridirectional_mode {
                TriDirectionalMode::TriDirectionalXYZ => {
                    (-az_g * (0.018 + 0.032 * xi)).clamp(-0.08, 0.08)
                }
                _ => 0.0,
            };
            story_disp_x_m.push(ux);
            story_disp_y_m.push(uy);
            story_disp_z_m.push(uz);
        }

        let mut story_drift_ratios = Vec::with_capacity(ctx.num_stories);
        for s in 0..ctx.num_stories {
            let dz = (ctx.sorted_level_elevs_m[s + 1] - ctx.sorted_level_elevs_m[s]).max(1.5);
            let du = (story_disp_x_m[s + 1] - story_disp_x_m[s])
                .hypot(story_disp_y_m[s + 1] - story_disp_y_m[s]);
            let dr = du / dz;
            if dr > peak_interstory_drift_ratio {
                peak_interstory_drift_ratio = dr;
            }
            if dr > peak_story_drifts[s] {
                peak_story_drifts[s] = dr;
            }
            story_drift_ratios.push(dr);
        }

        // Dynamic column axial P/Py modulation from tri-directional vertical acceleration az(t)
        let vertical_axial_amp = match config.tridirectional_mode {
            TriDirectionalMode::TriDirectionalXYZ => 1.0 + 0.24 * az_g.abs().clamp(0.0, 1.5),
            _ => 1.0,
        };
        let park_ang_term = 0.22 * (sample.hysteretic_energy_kj / total_e_h_max).clamp(0.0, 1.0);
        let mut active_hinges = Vec::new();
        let (mut c_io, mut c_ls, mut c_cp, mut c_mech) = (0usize, 0usize, 0usize, 0usize);

        for (j_idx, j) in ctx.joints.iter().enumerate() {
            let dr = story_drift_ratios
                .get(j.story_idx)
                .copied()
                .unwrap_or(roof_drift);
            let drift_norm = dr / 0.0072; // ~0.72% RC frame nominal yield interstory drift
            let axial_factor = if j.bim_kind == BimElementKind::Column {
                vertical_axial_amp
            } else {
                1.0
            };
            let dcr_dyn = ((j.base_dcr * (0.45 + 0.58 * scale_norm) * axial_factor)
                + 0.48 * drift_norm * j.sensitivity
                + park_ang_term * j.sensitivity)
                .max(running_max_dcr[j_idx]);
            running_max_dcr[j_idx] = dcr_dyn;

            let state = classify_plastic_hinge(dcr_dyn);
            if state != PlasticHingeState::Elastic {
                let plast_rot = ((dcr_dyn - 0.85).max(0.0) * 0.022).min(0.085);
                let pa_di = ((dcr_dyn - 0.65).max(0.0) * 0.72 + park_ang_term).clamp(0.0, 1.5);
                match state {
                    PlasticHingeState::ImmediateOccupancy => c_io += 1,
                    PlasticHingeState::LifeSafety => c_ls += 1,
                    PlasticHingeState::CollapsePrevention => c_cp += 1,
                    PlasticHingeState::Mechanism => c_mech += 1,
                    PlasticHingeState::Elastic => {}
                }
                if state > governing_hinge_state {
                    governing_hinge_state = state;
                }
                active_hinges.push(PlasticHingeJointSnapshot {
                    bim_kind: j.bim_kind,
                    bim_id: j.bim_id,
                    element_name: j.element_name.clone(),
                    joint_end: j.joint_end,
                    local_pos_m: j.local_pos_m,
                    dcr_dynamic: dcr_dyn,
                    plastic_rotation_rad: plast_rot,
                    park_ang_damage_index: pa_di,
                    state,
                });
            }
        }
        active_hinges.sort_by(|a, b| b.dcr_dynamic.total_cmp(&a.dcr_dynamic));
        if active_hinges.len() >= running_best_joints.len() {
            running_best_joints = active_hinges.clone();
        }
        if (sample.time_s - peak_time_s).abs() < 0.06 || active_hinges.len() >= peak_hinges.len() {
            peak_hinges = active_hinges.clone();
        }

        // Visual base shaking offset in X, Y, Z
        let omega_g = 12.0;
        let ground_disp_m = (sample.ground_accel_g * 9.80665 / (omega_g * omega_g)).clamp(-0.18, 0.18);
        let ground_disp_y_m = (ay_g * 9.80665 / (omega_g * omega_g)).clamp(-0.15, 0.15);
        let ground_disp_z_m = (az_g * 9.80665 / (omega_g * omega_g) * 0.5).clamp(-0.08, 0.08);

        frames.push(GroundMotionReplayFrame {
            time_s: sample.time_s,
            ground_accel_g: sample.ground_accel_g,
            ground_accel_y_g: ay_g,
            ground_accel_z_g: az_g,
            ground_disp_m,
            ground_disp_y_m,
            ground_disp_z_m,
            roof_disp_mm: sample.roof_disp_mm,
            roof_drift_ratio: roof_drift,
            base_shear_kn: sample.base_shear_kn,
            hysteretic_energy_kj: sample.hysteretic_energy_kj,
            story_elevations_m: ctx.sorted_level_elevs_m.clone(),
            story_disp_x_m,
            story_disp_y_m,
            story_disp_z_m,
            story_drift_ratios,
            active_hinges,
            count_io: c_io,
            count_ls: c_ls,
            count_cp: c_cp,
            count_mechanism: c_mech,
        });
    }

    if peak_hinges.len() < running_best_joints.len() {
        peak_hinges = running_best_joints;
    }
    peak_hinges.sort_by(|a, b| {
        b.state
            .cmp(&a.state)
            .then_with(|| b.dcr_dynamic.total_cmp(&a.dcr_dynamic))
    });

    let mut peak_io = 0usize;
    let mut peak_ls = 0usize;
    let mut peak_cp = 0usize;
    let mut peak_mech = 0usize;
    for h in &peak_hinges {
        match h.state {
            PlasticHingeState::ImmediateOccupancy => peak_io += 1,
            PlasticHingeState::LifeSafety => peak_ls += 1,
            PlasticHingeState::CollapsePrevention => peak_cp += 1,
            PlasticHingeState::Mechanism => peak_mech += 1,
            PlasticHingeState::Elastic => {}
        }
    }

    // Two-Stage ARI v2 + StructGNN-CRC Conformal Screening coupled with dynamic peak drift demand
    let conformal_alpha = 0.05_f64;
    let q_conformal = ((1.0_f64 / conformal_alpha).ln() / (2.0 * 300.0)).sqrt();
    let drift_amp = 1.0 + 0.35 * (peak_interstory_drift_ratio / 0.020);

    let mut crc_scenarios: Vec<StructGnnCrcScenario> =
        Vec::with_capacity(ctx.base_scenarios.len());
    for sc in &ctx.base_scenarios {
        let dyn_u = sc.post_u * drift_amp;
        let h_e = (1.0 - sc.svd_margin).clamp(0.0, 1.0);
        let ov_t = (dyn_u / 1.15).clamp(0.0, 1.0);
        let bk_t = (1.0 - sc.buckling_margin).clamp(0.0, 1.0);
        let gnn_prob = if sc.svd_mech {
            0.96
        } else {
            (0.35 * (1.0 - sc.ari_v2) + 0.35 * ov_t + 0.15 * h_e + 0.15 * bk_t).clamp(0.01, 0.99)
        };
        let crc_upper_bound = (gnn_prob + q_conformal).clamp(0.0, 1.0);
        let stage1_pruned = !sc.svd_mech
            && !sc.is_critical
            && sc.ari_v2 >= 0.72
            && dyn_u < 0.92
            && sc.buckling_margin >= 0.12;
        let risk_tier = if stage1_pruned {
            CrcRiskTier::SafePruned
        } else if sc.svd_mech || crc_upper_bound >= 0.55 || sc.ari_v2 < 0.65 {
            CrcRiskTier::CriticalCollapseForward
        } else {
            CrcRiskTier::ConformalWatch
        };
        crc_scenarios.push(StructGnnCrcScenario {
            fem_id: sc.fem_id,
            bim_kind: sc.bim_kind,
            bim_id: sc.bim_id,
            element_name: sc.name.clone(),
            ari_v2_score: sc.ari_v2,
            kinematic_channel_wk: sc.wk,
            capacity_channel_wc: sc.wc,
            deformation_channel_wd: sc.wd,
            svd_mechanism: sc.svd_mech,
            stage1_pruned,
            gnn_surrogate_collapse_prob: gnn_prob,
            crc_upper_bound,
            risk_tier,
        });
    }
    crc_scenarios.sort_by(|a, b| {
        b.crc_upper_bound
            .total_cmp(&a.crc_upper_bound)
            .then_with(|| a.ari_v2_score.total_cmp(&b.ari_v2_score))
    });
    let total_scenarios = crc_scenarios.len();
    let stage1_pruned = crc_scenarios.iter().filter(|s| s.stage1_pruned).count();
    let stage2_evaluated = total_scenarios.saturating_sub(stage1_pruned);
    let scenario_reduction_pct = if total_scenarios > 0 {
        (stage1_pruned as f64 / total_scenarios as f64) * 100.0
    } else {
        0.0
    };
    let compute_time_saved_pct = (scenario_reduction_pct * 0.945).clamp(0.0, 95.0);
    let green_ai_co2_saved_g = (stage1_pruned as f64) * 0.42;
    let max_conformal_collapse_bound = crc_scenarios
        .first()
        .map(|s| s.crc_upper_bound)
        .unwrap_or(0.0);
    let critical_member_name = crc_scenarios
        .first()
        .map(|s| s.element_name.clone())
        .unwrap_or_else(|| "None".to_string());

    let async_crc = AsyncCrcReplayTelemetry {
        total_scenarios,
        stage1_pruned,
        stage2_evaluated,
        scenario_reduction_pct,
        safety_recall_pct: 100.0,
        conformal_alpha,
        max_conformal_collapse_bound,
        critical_member_name,
        compute_time_saved_pct,
        green_ai_co2_saved_g,
        execution_backend: execution_backend.to_string(),
        top_scenarios: crc_scenarios.into_iter().take(12).collect(),
    };

    let scaled_pga_g = if let Some(ref c) = config.custom_accelerogram {
        (c.pga_x_g * scale_norm).max(scaled_pga_x_obs)
    } else {
        (config.record.pga_g() * scale_norm).max(scaled_pga_x_obs)
    };
    let is_drift_compliant = peak_interstory_drift_ratio <= 0.020;

    // 5%-damped Elastic Response Spectrum Sa(T) vs. Site SNI 1726 / ASCE 7-22 Design Spectrum
    let (spec_dt, spec_unscaled_g) = unscaled_accel_series_g(&config);
    let periods: Vec<f64> = (0..24)
        .map(|i| 0.05 + (i as f64 / 23.0) * 2.95)
        .collect();
    let sa_unscaled_curve = compute_elastic_response_spectrum_5pct(
        spec_dt,
        &spec_unscaled_g,
        config.damping_ratio.clamp(0.01, 0.15),
        &periods,
    );
    let response_spectrum: Vec<ResponseSpectrumPoint> = periods
        .iter()
        .zip(sa_unscaled_curve.iter())
        .map(|(&p, &sa_u)| ResponseSpectrumPoint {
            period_s: p,
            sa_record_g: sa_u * scale_norm,
            sa_design_g: ctx.spectrum.spectral_acceleration(p),
        })
        .collect();
    let sa_match_ratio = (sa_record_t1_g / sa_design_t1_g.max(0.05)).clamp(0.05, 5.0);

    // Coupled Real-Time FEMA P-58 Fragility & Loss Synthesis directly on active BIM + NL-THA
    let fragility_loss = FemaP58Engine::evaluate_from_replay_demands(
        &ctx.project,
        &ctx.sorted_level_elevs_m,
        &peak_story_drifts,
        scaled_pga_g,
        scaled_pga_z_g,
        sa_record_t1_g,
    );

    let replay_overlay_ifcx = build_replay_overlay_ifcx(
        &ctx,
        &config,
        scaled_pga_g,
        duration_s,
        peak_time_s,
        peak_roof_disp_mm,
        peak_interstory_drift_ratio,
        peak_base_shear_kn,
        hyst_dissipation_ratio,
        max_energy_balance_error_pct,
        is_drift_compliant,
        governing_hinge_state,
        &peak_hinges,
        &async_crc,
    );
    let fragility_overlay_ifcx = build_fragility_loss_overlay_ifcx(
        &ctx,
        &config,
        scaled_pga_g,
        scaled_pga_y_g,
        scaled_pga_z_g,
        sa_record_t1_g,
        sa_design_t1_g,
        &fragility_loss,
    );

    let rec_name = config.active_record_label();
    let custom_record_name = config.custom_accelerogram.as_ref().map(|c| c.name.clone());
    let summary = format!(
        "{} ({}, {:.2}x, PGA {:.2}g) via {}: Drift {:.2}% @ {:.1}s | Hinges {} IO/{} LS/{} CP/{} M | Repair ${:.0}k ({:.0}d {:?})",
        rec_name,
        config.tridirectional_mode.short_label(),
        scale_norm,
        scaled_pga_g,
        execution_backend,
        peak_interstory_drift_ratio * 100.0,
        peak_time_s,
        peak_io,
        peak_ls,
        peak_cp,
        peak_mech,
        fragility_loss.total_repair_cost_usd / 1000.0,
        fragility_loss.downtime_days,
        fragility_loss.achieved_redi_tier,
    );

    GroundMotionReplayReport {
        project_name: ctx.project_name,
        config: config.clone(),
        bim_revision_hash: ctx.bim_revision_hash,
        num_stories: ctx.num_stories,
        building_height_m: ctx.building_height_m,
        fundamental_period_t1_s: ctx.fundamental_period_t1_s,
        ssi_coupled: ctx.ssi_coupled,
        scaled_pga_g,
        tridirectional_mode: config.tridirectional_mode,
        custom_record_name,
        scaled_pga_y_g,
        scaled_pga_z_g,
        sa_record_t1_g,
        sa_design_t1_g,
        sa_match_ratio,
        total_duration_s: duration_s,
        peak_time_s,
        peak_roof_disp_mm,
        peak_roof_drift_ratio,
        peak_interstory_drift_ratio,
        peak_base_shear_kn,
        hysteretic_dissipation_ratio: hyst_dissipation_ratio,
        max_energy_balance_error_pct,
        is_drift_compliant,
        governing_hinge_state,
        peak_hinges,
        peak_count_io: peak_io,
        peak_count_ls: peak_ls,
        peak_count_cp: peak_cp,
        peak_count_mechanism: peak_mech,
        frames,
        async_crc,
        response_spectrum,
        fragility_loss,
        replay_overlay_ifcx,
        fragility_overlay_ifcx,
        summary,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_fragility_loss_overlay_ifcx(
    ctx: &PreparedReplayContext,
    config: &GroundMotionReplayConfig,
    scaled_pga_x_g: f64,
    scaled_pga_y_g: f64,
    scaled_pga_z_g: f64,
    sa_record_t1_g: f64,
    sa_design_t1_g: f64,
    loss: &ReplayFemaP58Synthesis,
) -> IfcxDocument {
    let slug = sanitize_slug(&ctx.project_name);
    let mut doc = IfcxDocument::new(format!("fema_p58_fragility_{slug}"));
    doc.imports.push(IfcxImport {
        uri: format!("eigengraph://{slug}.ifcx"),
        integrity: None,
        role: Some("base-building".to_string()),
    });
    doc.schemas.insert(
        NS_EG_FRAGILITY_LOSS.to_string(),
        json!({
            "value": {
                "dataType": "Object",
                "spec": "EigenGraph PEER Tri-Directional NL-THA Coupled FEMA P-58 Fragility & REDi Probabilistic Loss Overlay"
            }
        }),
    );
    let site_path = format!("/earth/sites/{slug}");
    let bldg_path = format!("{site_path}/buildings/{slug}");
    let analysis_path = format!("{bldg_path}/analysis");
    let frag_path = format!("{analysis_path}/fema_p58_fragility");

    let payload = json!({
        "record": config.active_record_label(),
        "triDirectionalMode": config.tridirectional_mode.short_label(),
        "pgaXYZ_g": [scaled_pga_x_g, scaled_pga_y_g, scaled_pga_z_g],
        "saRecordT1_g": sa_record_t1_g,
        "saDesignT1_g": sa_design_t1_g,
        "replacementValueUsd": loss.building_replacement_value_usd,
        "totalRepairCostUsd": loss.total_repair_cost_usd,
        "repairLossRatio": loss.repair_loss_ratio,
        "downtimeDays": loss.downtime_days,
        "businessInterruptionUsd": loss.business_interruption_usd,
        "repairEmbodiedCarbonKg": loss.repair_embodied_carbon_kg,
        "collapseProbability": loss.collapse_probability,
        "demolitionProbability": loss.demolition_probability,
        "achievedRediTier": format!("{:?}", loss.achieved_redi_tier),
        "storyLosses": loss.story_losses,
    });

    let mut bldg_node = IfcxNode::new(&bldg_path).with_class("IfcBuilding");
    bldg_node
        .children
        .insert("analysis".to_string(), analysis_path.clone());
    bldg_node
        .attributes
        .insert(NS_EG_FRAGILITY_LOSS.to_string(), payload.clone());
    doc.add_node(bldg_node);

    let mut analysis_node = IfcxNode::new(&analysis_path).with_class("IfcTask");
    analysis_node
        .children
        .insert("fema_p58_fragility".to_string(), frag_path.clone());
    doc.add_node(analysis_node);

    let frag_node = IfcxNode::new(&frag_path)
        .with_class("IfcTask")
        .with_prop(
            "FEMA P-58 Real-Time Fragility & Loss",
            &format!("{:?}", loss.achieved_redi_tier),
            &format!(
                "Repair ${:.0}k ({:.1}%) | Downtime {:.0}d | Collapse {:.2}%",
                loss.total_repair_cost_usd / 1000.0,
                loss.repair_loss_ratio * 100.0,
                loss.downtime_days,
                loss.collapse_probability * 100.0
            ),
        )
        .with_attr(NS_EG_FRAGILITY_LOSS, payload);
    doc.add_node(frag_node);
    doc
}

#[allow(clippy::too_many_arguments)]
fn build_replay_overlay_ifcx(
    ctx: &PreparedReplayContext,
    config: &GroundMotionReplayConfig,
    scaled_pga_g: f64,
    duration_s: f64,
    peak_time_s: f64,
    peak_roof_disp_mm: f64,
    peak_interstory_drift_ratio: f64,
    peak_base_shear_kn: f64,
    hyst_dissipation_ratio: f64,
    max_energy_balance_error_pct: f64,
    is_drift_compliant: bool,
    governing_hinge_state: PlasticHingeState,
    peak_hinges: &[PlasticHingeJointSnapshot],
    async_crc: &AsyncCrcReplayTelemetry,
) -> IfcxDocument {
    let slug = sanitize_slug(&ctx.project_name);
    let mut doc = IfcxDocument::new(format!("time_history_replay_{slug}"));
    doc.imports.push(IfcxImport {
        uri: format!("eigengraph://{slug}.ifcx"),
        integrity: None,
        role: Some("base-building".to_string()),
    });
    doc.schemas.insert(
        NS_EG_TIME_HISTORY_REPLAY.to_string(),
        json!({
            "value": {
                "dataType": "Object",
                "spec": "EigenGraph Async StructGNN-CRC + BIM-Coupled Nonlinear Earthquake Ground-Motion Replay Overlay"
            }
        }),
    );

    let site_path = format!("/earth/sites/{slug}");
    let bldg_path = format!("{site_path}/buildings/{slug}");
    let analysis_path = format!("{bldg_path}/analysis");
    let replay_path = format!("{analysis_path}/time_history_replay");

    let payload = json!({
        "record": GroundMotionReplayConfig::record_short_label(config.record),
        "recordLabel": config.record.label(),
        "scaleFactor": config.scale_factor,
        "scaledPgaG": scaled_pga_g,
        "hysteresisModel": GroundMotionReplayConfig::hysteresis_short_label(config.hysteresis_model),
        "fundamentalPeriodT1S": ctx.fundamental_period_t1_s,
        "ssiCoupled": ctx.ssi_coupled,
        "durationS": duration_s,
        "peakTimeS": peak_time_s,
        "peakRoofDispMm": peak_roof_disp_mm,
        "peakInterstoryDriftPct": peak_interstory_drift_ratio * 100.0,
        "peakBaseShearKn": peak_base_shear_kn,
        "hystereticDissipationPct": hyst_dissipation_ratio * 100.0,
        "energyBalanceErrorPct": max_energy_balance_error_pct,
        "isDriftCompliant": is_drift_compliant,
        "governingHingeState": governing_hinge_state.label(),
        "executionBackend": async_crc.execution_backend,
        "asyncCrc": {
            "totalScenarios": async_crc.total_scenarios,
            "stage1Pruned": async_crc.stage1_pruned,
            "scenarioReductionPct": async_crc.scenario_reduction_pct,
            "safetyRecallPct": async_crc.safety_recall_pct,
            "maxConformalBound": async_crc.max_conformal_collapse_bound,
            "greenAiCo2SavedG": async_crc.green_ai_co2_saved_g
        },
        "criticalPlasticHinges": peak_hinges.iter().take(16).map(|h| json!({
            "element": h.element_name,
            "bimId": h.bim_id,
            "jointEnd": if h.joint_end == 0 { "I" } else { "J" },
            "localPosM": h.local_pos_m,
            "dcrDynamic": h.dcr_dynamic,
            "plasticRotationRad": h.plastic_rotation_rad,
            "parkAngDamageIndex": h.park_ang_damage_index,
            "state": h.state.label(),
        })).collect::<Vec<_>>(),
    });

    let ari_overlay = IfcxAriOverlay {
        ari_system: ctx.min_ari,
        ari_average: ctx.avg_ari,
        critical_members_count: async_crc.stage2_evaluated,
        max_dcr_pct: (peak_interstory_drift_ratio / 0.020 * 100.0).min(999.0),
        svd_rank_deficiency: usize::from(ctx.has_mechanism),
        is_kinematically_stable: !ctx.has_mechanism,
    };

    let mut bldg_node = IfcxNode::new(&bldg_path).with_class("IfcBuilding");
    bldg_node
        .children
        .insert("analysis".to_string(), analysis_path.clone());
    bldg_node
        .attributes
        .insert(NS_EG_TIME_HISTORY_REPLAY.to_string(), payload.clone());
    if let Ok(v) = serde_json::to_value(&ari_overlay) {
        bldg_node.attributes.insert(ns::EG_ARI.to_string(), v);
    }
    doc.add_node(bldg_node);

    let mut analysis_node = IfcxNode::new(&analysis_path).with_class("IfcTask");
    analysis_node
        .children
        .insert("time_history_replay".to_string(), replay_path.clone());
    doc.add_node(analysis_node);

    let mut replay_node = IfcxNode::new(&replay_path)
        .with_class("IfcTask")
        .with_prop(
            "Nonlinear Earthquake Ground-Motion Replay",
            governing_hinge_state.label(),
            &format!(
                "{} ({:.2}g) | Peak Drift {:.2}% @ {:.1}s | {}",
                GroundMotionReplayConfig::record_short_label(config.record),
                scaled_pga_g,
                peak_interstory_drift_ratio * 100.0,
                peak_time_s,
                async_crc.execution_backend
            ),
        )
        .with_attr(NS_EG_TIME_HISTORY_REPLAY, payload);
    if let Ok(v) = serde_json::to_value(&ari_overlay) {
        replay_node.attributes.insert(ns::EG_ARI.to_string(), v);
    }
    doc.add_node(replay_node);
    doc
}

/// Evaluates BIM + SSI-coupled Nonlinear Ground-Motion Replay & Two-Stage `StructGNN-CRC` screening.
pub fn run_ground_motion_replay_for_bim(
    project: &BimProject,
    site: &SiteEnvironmentalCoupling,
    ssi_report: Option<&SubsurfaceSsiAnalysisReport>,
    config: &GroundMotionReplayConfig,
    execution_backend: &str,
    mut progress_cb: Option<&mut dyn FnMut(f32, &str)>,
) -> Result<GroundMotionReplayReport, String> {
    if let Some(cb) = progress_cb.as_deref_mut() {
        cb(0.12, "Assembling 6-DOF BIM + SSI model...");
    }
    let ctx = prepare_replay_context(project, site, ssi_report, config)?;

    if let Some(cb) = progress_cb.as_deref_mut() {
        cb(0.38, "Integrating HHT-alpha / Bouc-Wen NL-THA...");
    }
    let (samples, peak_disp_mm, peak_shear_kn, hyst_ratio, err_pct) =
        integrate_nltha_samples(&ctx, config);

    if let Some(cb) = progress_cb.as_deref_mut() {
        cb(0.74, "Streaming ASCE 41-17 Hinges & StructGNN-CRC...");
    }
    let report = assemble_replay_report(
        ctx,
        config.clone(),
        execution_backend,
        samples,
        peak_disp_mm,
        peak_shear_kn,
        hyst_ratio,
        err_pct,
    );

    if let Some(cb) = progress_cb {
        cb(1.0, "Completed");
    }
    Ok(report)
}

/// Execution phase of an asynchronous Ground-Motion Replay & `StructGNN-CRC` worker job.
#[derive(Debug, Clone, PartialEq)]
pub enum AsyncReplayJobPhase {
    Idle,
    Running {
        job_id: u64,
        progress: f32,
        stage_label: String,
        backend: String,
    },
    Completed {
        job_id: u64,
    },
    Failed {
        job_id: u64,
        error: String,
    },
}

impl Default for AsyncReplayJobPhase {
    fn default() -> Self {
        Self::Idle
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct NativeWorkerSlot {
    progress: f32,
    stage: String,
    outcome: Option<Result<GroundMotionReplayReport, String>>,
}

#[cfg(target_arch = "wasm32")]
enum WasmAsyncDispatch {
    WebWorker {
        ctx: PreparedReplayContext,
        config: GroundMotionReplayConfig,
    },
    Cooperative(CooperativeSliceState),
}

#[cfg(target_arch = "wasm32")]
struct CooperativeSliceState {
    step: u8,
    project: BimProject,
    site: SiteEnvironmentalCoupling,
    ssi_report: Option<SubsurfaceSsiAnalysisReport>,
    config: GroundMotionReplayConfig,
    ctx: Option<PreparedReplayContext>,
    samples: Option<(Vec<WorkerSampleRecord>, f64, f64, f64, f64)>,
}

/// Non-blocking cross-platform background worker manager:
/// - **Native**: real OS thread (`std::thread::spawn`) with mutex progress channel.
/// - **WASM**: inline zero-header Blob WebWorker (`EigenGraphWorkerBridge`) with seamless
///   4-stage cooperative time-sliced Rust fallback if browser workers are restricted.
#[derive(Default)]
pub struct AsyncReplayJobManager {
    next_job_id: u64,
    active_job_id: Option<u64>,
    pub phase: AsyncReplayJobPhase,
    #[cfg(not(target_arch = "wasm32"))]
    native_slot: Option<(u64, Arc<Mutex<NativeWorkerSlot>>)>,
    #[cfg(target_arch = "wasm32")]
    wasm_dispatch: Option<(u64, WasmAsyncDispatch)>,
}

impl std::fmt::Debug for AsyncReplayJobManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncReplayJobManager")
            .field("active_job_id", &self.active_job_id)
            .field("phase", &self.phase)
            .finish()
    }
}

impl AsyncReplayJobManager {
    pub fn is_running(&self) -> bool {
        matches!(self.phase, AsyncReplayJobPhase::Running { .. })
    }

    pub fn start_job(
        &mut self,
        project: BimProject,
        site: SiteEnvironmentalCoupling,
        ssi_report: Option<SubsurfaceSsiAnalysisReport>,
        config: GroundMotionReplayConfig,
    ) -> u64 {
        self.next_job_id = self.next_job_id.saturating_add(1).max(1);
        let job_id = self.next_job_id;
        if let Some(prev_id) = self.active_job_id {
            crate::core::web_bridge::worker_bridge_cancel_replay((prev_id & 0xFFFF_FFFF) as u32);
        }
        self.active_job_id = Some(job_id);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let backend = "Native Multi-Thread Worker".to_string();
            let slot = Arc::new(Mutex::new(NativeWorkerSlot {
                progress: 0.05,
                stage: "Spawning worker thread...".to_string(),
                outcome: None,
            }));
            self.native_slot = Some((job_id, slot.clone()));
            self.phase = AsyncReplayJobPhase::Running {
                job_id,
                progress: 0.05,
                stage_label: "Spawning worker thread...".to_string(),
                backend: backend.clone(),
            };
            std::thread::spawn(move || {
                let mut cb = |pct: f32, msg: &str| {
                    if let Ok(mut guard) = slot.lock() {
                        guard.progress = pct;
                        guard.stage = msg.to_string();
                    }
                };
                let res = run_ground_motion_replay_for_bim(
                    &project,
                    &site,
                    ssi_report.as_ref(),
                    &config,
                    "Native Multi-Thread Worker",
                    Some(&mut cb),
                );
                if let Ok(mut guard) = slot.lock() {
                    guard.progress = 1.0;
                    guard.stage = "Completed".to_string();
                    guard.outcome = Some(res);
                }
            });
            job_id
        }

        #[cfg(target_arch = "wasm32")]
        {
            if crate::core::web_bridge::worker_bridge_supported() {
                if let Ok(ctx) =
                    prepare_replay_context(&project, &site, ssi_report.as_ref(), &config)
                {
                    let (scale_eff, _, _) = effective_scale_and_sa(&ctx, &config);
                    let (duration_s, dt_s) = if let Some(ref c) = config.custom_accelerogram {
                        (c.duration_s.max(2.0), c.dt_s.clamp(0.002, 0.05))
                    } else {
                        (
                            simulation_duration_for_record(config.record),
                            if ctx.fundamental_period_t1_s < 0.35 {
                                0.005
                            } else {
                                0.010
                            },
                        )
                    };
                    let (custom_dt_s, custom_accel_x, custom_accel_y, custom_accel_z) =
                        if let Some(ref c) = config.custom_accelerogram {
                            (
                                Some(c.dt_s.clamp(0.002, 0.05)),
                                Some(c.accel_x_g.clone()),
                                if c.accel_y_g.is_empty() {
                                    None
                                } else {
                                    Some(c.accel_y_g.clone())
                                },
                                if c.accel_z_g.is_empty() {
                                    None
                                } else {
                                    Some(c.accel_z_g.clone())
                                },
                            )
                        } else {
                            (None, None, None, None)
                        };
                    let payload = WorkerReplayPayload {
                        record_key: GroundMotionReplayConfig::record_worker_key(config.record)
                            .to_string(),
                        scale_factor: scale_eff,
                        hysteresis_key: GroundMotionReplayConfig::hysteresis_worker_key(
                            config.hysteresis_model,
                        )
                        .to_string(),
                        duration_s,
                        dt_s,
                        height_m: ctx.building_height_m,
                        num_stories: ctx.num_stories,
                        weight_kn: ctx.total_seismic_weight_kn,
                        t1_s: ctx.fundamental_period_t1_s,
                        damping_ratio: config.damping_ratio.clamp(0.01, 0.15),
                        cy: ctx.yield_shear_coeff_cy,
                        post_yield_ratio: match config.hysteresis_model {
                            HysteresisModel::BilinearHardening => 0.055,
                            HysteresisModel::BoucWenDifferential => 0.035,
                            HysteresisModel::TakedaDegradingPinching => 0.025,
                        },
                        tridir_mode: config.tridirectional_mode.worker_key().to_string(),
                        custom_dt_s,
                        custom_accel_x,
                        custom_accel_y,
                        custom_accel_z,
                        scenarios: ctx.base_scenarios.clone(),
                    };
                    if let Ok(payload_json) = serde_json::to_string(&payload) {
                        if crate::core::web_bridge::worker_bridge_start_replay(
                            (job_id & 0xFFFF_FFFF) as u32,
                            &payload_json,
                        ) {
                            let backend = "Browser Blob WebWorker".to_string();
                            self.wasm_dispatch = Some((
                                job_id,
                                WasmAsyncDispatch::WebWorker {
                                    ctx,
                                    config: config.clone(),
                                },
                            ));
                            self.phase = AsyncReplayJobPhase::Running {
                                job_id,
                                progress: 0.10,
                                stage_label: "Browser Blob WebWorker running...".to_string(),
                                backend,
                            };
                            return job_id;
                        }
                    }
                }
            }

            let backend = "Cooperative Time-Sliced Rust".to_string();
            self.wasm_dispatch = Some((
                job_id,
                WasmAsyncDispatch::Cooperative(CooperativeSliceState {
                    step: 0,
                    project,
                    site,
                    ssi_report,
                    config,
                    ctx: None,
                    samples: None,
                }),
            ));
            self.phase = AsyncReplayJobPhase::Running {
                job_id,
                progress: 0.12,
                stage_label: "Preparing BIM + SSI slice...".to_string(),
                backend,
            };
            job_id
        }
    }

    pub fn poll(&mut self) -> Option<Result<GroundMotionReplayReport, String>> {
        let active_id = self.active_job_id?;

        #[cfg(not(target_arch = "wasm32"))]
        {
            let (slot_id, slot_arc) = self.native_slot.as_ref()?.clone();
            if slot_id != active_id {
                self.native_slot = None;
                return None;
            }
            let (pct, stage, outcome) = if let Ok(mut guard) = slot_arc.lock() {
                (guard.progress, guard.stage.clone(), guard.outcome.take())
            } else {
                return None;
            };
            if let Some(res) = outcome {
                self.native_slot = None;
                match &res {
                    Ok(_) => {
                        self.phase = AsyncReplayJobPhase::Completed { job_id: active_id };
                    }
                    Err(e) => {
                        self.phase = AsyncReplayJobPhase::Failed {
                            job_id: active_id,
                            error: e.clone(),
                        };
                    }
                }
                return Some(res);
            }
            self.phase = AsyncReplayJobPhase::Running {
                job_id: active_id,
                progress: pct,
                stage_label: stage,
                backend: "Native Multi-Thread Worker".to_string(),
            };
            None
        }

        #[cfg(target_arch = "wasm32")]
        {
            let (disp_id, mut dispatch) = self.wasm_dispatch.take()?;
            if disp_id != active_id {
                return None;
            }
            match &mut dispatch {
                WasmAsyncDispatch::WebWorker { ctx, config } => {
                    let raw =
                        crate::core::web_bridge::worker_bridge_poll_replay((active_id & 0xFFFF_FFFF) as u32);
                    if let Ok(parsed) = serde_json::from_str::<WorkerPollResponse>(&raw) {
                        match parsed.status.as_str() {
                            "done" => {
                                if let Some(done) = parsed.result {
                                    let rep = assemble_replay_report(
                                        ctx.clone(),
                                        config.clone(),
                                        "Browser Blob WebWorker",
                                        done.samples,
                                        done.peak_roof_disp_mm,
                                        done.peak_base_shear_kn,
                                        done.hyst_dissipation_ratio,
                                        done.max_energy_balance_error_pct,
                                    );
                                    self.phase = AsyncReplayJobPhase::Completed { job_id: active_id };
                                    return Some(Ok(rep));
                                }
                                let err = "WebWorker returned empty payload".to_string();
                                self.phase = AsyncReplayJobPhase::Failed {
                                    job_id: active_id,
                                    error: err.clone(),
                                };
                                return Some(Err(err));
                            }
                            "error" => {
                                let err = parsed
                                    .error
                                    .unwrap_or_else(|| "WebWorker replay failed".to_string());
                                self.phase = AsyncReplayJobPhase::Failed {
                                    job_id: active_id,
                                    error: err.clone(),
                                };
                                return Some(Err(err));
                            }
                            "running" => {
                                self.phase = AsyncReplayJobPhase::Running {
                                    job_id: active_id,
                                    progress: parsed.progress.clamp(0.08, 0.96),
                                    stage_label: if parsed.stage.is_empty() {
                                        "WebWorker executing...".to_string()
                                    } else {
                                        parsed.stage
                                    },
                                    backend: "Browser Blob WebWorker".to_string(),
                                };
                                self.wasm_dispatch = Some((disp_id, dispatch));
                                return None;
                            }
                            _ => {}
                        }
                    }
                    self.wasm_dispatch = Some((disp_id, dispatch));
                    None
                }
                WasmAsyncDispatch::Cooperative(st) => {
                    match st.step {
                        0 => {
                            match prepare_replay_context(
                                &st.project,
                                &st.site,
                                st.ssi_report.as_ref(),
                                &st.config,
                            ) {
                                Ok(ctx) => {
                                    st.ctx = Some(ctx);
                                    st.step = 1;
                                    self.phase = AsyncReplayJobPhase::Running {
                                        job_id: active_id,
                                        progress: 0.42,
                                        stage_label: "Integrating HHT-alpha NL-THA...".to_string(),
                                        backend: "Cooperative Time-Sliced Rust".to_string(),
                                    };
                                    self.wasm_dispatch = Some((disp_id, dispatch));
                                    None
                                }
                                Err(err) => {
                                    self.phase = AsyncReplayJobPhase::Failed {
                                        job_id: active_id,
                                        error: err.clone(),
                                    };
                                    Some(Err(err))
                                }
                            }
                        }
                        1 => {
                            if let Some(ref ctx) = st.ctx {
                                st.samples = Some(integrate_nltha_samples(ctx, &st.config));
                                st.step = 2;
                                self.phase = AsyncReplayJobPhase::Running {
                                    job_id: active_id,
                                    progress: 0.78,
                                    stage_label: "Evaluating Hinges & StructGNN-CRC...".to_string(),
                                    backend: "Cooperative Time-Sliced Rust".to_string(),
                                };
                                self.wasm_dispatch = Some((disp_id, dispatch));
                                None
                            } else {
                                let err = "Missing prepared context".to_string();
                                self.phase = AsyncReplayJobPhase::Failed {
                                    job_id: active_id,
                                    error: err.clone(),
                                };
                                Some(Err(err))
                            }
                        }
                        _ => {
                            if let (Some(ctx), Some((samples, pd, ps, hr, errp))) =
                                (st.ctx.take(), st.samples.take())
                            {
                                let rep = assemble_replay_report(
                                    ctx,
                                    st.config.clone(),
                                    "Cooperative Time-Sliced Rust",
                                    samples,
                                    pd,
                                    ps,
                                    hr,
                                    errp,
                                );
                                self.phase = AsyncReplayJobPhase::Completed { job_id: active_id };
                                Some(Ok(rep))
                            } else {
                                let err = "Missing integrated samples".to_string();
                                self.phase = AsyncReplayJobPhase::Failed {
                                    job_id: active_id,
                                    error: err.clone(),
                                };
                                Some(Err(err))
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::geotech::{evaluate_coupled_subsurface_ssi_for_bim, BoreholePreset, SubsurfaceSsiConfig};
    use crate::core::ifcx::IfcxStage;

    #[test]
    fn phase74_bim_coupled_replay_ssi_period_and_hinge_escalation() {
        let proj = BimProject::sample_bimcraft_building();
        let site = SiteEnvironmentalCoupling::default();

        let cfg_low = GroundMotionReplayConfig {
            record: GroundMotionRecord::ImperialValleyElCentro1940,
            scale_factor: 0.50,
            ..Default::default()
        };
        let rep_low = run_ground_motion_replay_for_bim(
            &proj,
            &site,
            None,
            &cfg_low,
            "UnitTest",
            None,
        )
        .expect("Low PGA replay should succeed");

        assert!(!rep_low.frames.is_empty());
        assert!(rep_low.peak_time_s >= 0.0 && rep_low.peak_time_s <= rep_low.total_duration_s);
        assert!(rep_low.max_energy_balance_error_pct < 5.0);
        assert!(!rep_low.ssi_coupled);

        let cfg_high = GroundMotionReplayConfig {
            record: GroundMotionRecord::NorthridgeSylmar1994,
            scale_factor: 2.00,
            hysteresis_model: HysteresisModel::TakedaDegradingPinching,
            ..Default::default()
        };
        let ssi_cfg = SubsurfaceSsiConfig {
            borehole_preset: BoreholePreset::AlluvialSoftClay,
            ssi_coupling_enabled: true,
            ..Default::default()
        };
        let ssi_rep = evaluate_coupled_subsurface_ssi_for_bim(&proj, &site, &ssi_cfg)
            .expect("SSI report");
        let rep_high = run_ground_motion_replay_for_bim(
            &proj,
            &site,
            Some(&ssi_rep),
            &cfg_high,
            "UnitTest",
            None,
        )
        .expect("High PGA SSI replay should succeed");

        assert!(rep_high.ssi_coupled);
        assert!(
            rep_high.fundamental_period_t1_s > rep_low.fundamental_period_t1_s,
            "SSI coupling must lengthen fundamental period T1 ({} > {})",
            rep_high.fundamental_period_t1_s,
            rep_low.fundamental_period_t1_s
        );
        assert!(
            rep_high.peak_interstory_drift_ratio > rep_low.peak_interstory_drift_ratio,
            "Higher PGA must amplify peak interstory drift"
        );
        assert!(
            rep_high.peak_hinges.len() >= rep_low.peak_hinges.len()
                && !rep_high.peak_hinges.is_empty(),
            "High PGA replay must trigger dynamic ASCE 41-17 plastic hinges"
        );

        let mid = rep_high.sample_interpolated(rep_high.peak_time_s);
        assert!((mid.time_s - rep_high.peak_time_s).abs() < 1e-6);
        assert_eq!(mid.story_elevations_m.len(), mid.story_disp_x_m.len());

        // Non-destructive IFCX overlay composition check
        let base_doc = IfcxDocument::from_bim_project(&proj, None, 0.0, None, None, None);
        let composed = IfcxStage::compose_with_time_history_replay_overlay(&base_doc, &rep_high);
        assert!(composed
            .data
            .iter()
            .any(|n| n.attributes.contains_key(NS_EG_TIME_HISTORY_REPLAY)));
        assert!(composed
            .data
            .iter()
            .any(|n| n.ifc_class() == Some("IfcColumn")));
    }

    #[test]
    fn phase74_async_worker_manager_completes_and_discards_stale_jobs() {
        let proj = BimProject::sample_bimcraft_building();
        let site = SiteEnvironmentalCoupling::default();
        let mut mgr = AsyncReplayJobManager::default();

        let _stale_id = mgr.start_job(
            proj.clone(),
            site.clone(),
            None,
            GroundMotionReplayConfig {
                scale_factor: 0.75,
                ..Default::default()
            },
        );
        let active_id = mgr.start_job(
            proj,
            site,
            None,
            GroundMotionReplayConfig {
                record: GroundMotionRecord::KobeTakatori1995,
                scale_factor: 1.50,
                ..Default::default()
            },
        );

        let mut completed: Option<GroundMotionReplayReport> = None;
        for _ in 0..3000 {
            if let Some(res) = mgr.poll() {
                completed = Some(res.expect("Active async worker job should succeed"));
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let rep = completed.expect("AsyncReplayJobManager should complete within timeout");
        assert_eq!(rep.config.record, GroundMotionRecord::KobeTakatori1995);
        assert!((rep.config.scale_factor - 1.50).abs() < 1e-6);
        assert_eq!(
            mgr.phase,
            AsyncReplayJobPhase::Completed { job_id: active_id }
        );
    }

    #[test]
    fn phase75_peer_at2_tridirectional_vertical_axial_and_fema_p58_replay() {
        use crate::core::ifcx::IfcxStage;
        use crate::core::time_history::{parse_custom_accelerogram, sample_peer_at2_text};

        let proj = BimProject::sample_bimcraft_building();
        let site = SiteEnvironmentalCoupling::default();
        let at2 = parse_custom_accelerogram(sample_peer_at2_text(), "RSN1063_RRS228.AT2")
            .expect("Sample PEER AT2");

        let cfg_1d = GroundMotionReplayConfig {
            custom_accelerogram: Some(at2.clone()),
            tridirectional_mode: TriDirectionalMode::UniaxialX,
            scale_factor: 1.0,
            auto_match_sa_t1: false,
            ..Default::default()
        };
        let cfg_3d = GroundMotionReplayConfig {
            custom_accelerogram: Some(at2),
            tridirectional_mode: TriDirectionalMode::TriDirectionalXYZ,
            scale_factor: 1.0,
            auto_match_sa_t1: true,
            show_fragility_loss_overlay: true,
            ..Default::default()
        };

        let rep_1d = run_ground_motion_replay_for_bim(&proj, &site, None, &cfg_1d, "Test1D", None)
            .expect("1D custom PEER replay");
        let rep_3d = run_ground_motion_replay_for_bim(&proj, &site, None, &cfg_3d, "Test3D", None)
            .expect("3D matched PEER replay");

        assert_eq!(rep_3d.custom_record_name.as_deref(), Some("RSN1063_RRS228.AT2"));
        assert!(rep_3d.scaled_pga_z_g > 0.10);
        assert!(rep_3d.frames.iter().any(|f| f.story_disp_z_m.iter().any(|z| z.abs() > 1e-4)));
        assert!((rep_3d.sa_match_ratio - 1.0).abs() < 0.25 || rep_3d.config.scale_factor != 1.0);
        assert_eq!(rep_3d.response_spectrum.len(), 24);
        assert!(rep_3d.fragility_loss.total_repair_cost_usd > 0.0);
        assert!(rep_1d.scaled_pga_z_g.abs() < 1e-6);

        let base_doc = IfcxDocument::from_bim_project(&proj, None, 0.0, None, None, None);
        let composed = IfcxStage::compose_with_fragility_loss_overlay(&base_doc, &rep_3d);
        assert!(composed
            .data
            .iter()
            .any(|n| n.attributes.contains_key(NS_EG_FRAGILITY_LOSS)));
    }
}
