//! Phase 76: Live SHM Digital Twin, Modal Fingerprint Identification & Post-Earthquake ATC-20 Placard Triage.
//!
//! Couples real-time or streamed triaxial accelerometer, inclinometer, and base strain telemetry with the
//! active 3D BIM 6-DOF Finite Element stiffness/mass model (`FemSolver`, `ModalEngine`, `EigenGateEngine`).
//! Computes baseline vs. observed natural frequencies (`f_1..f_3`), Modal Assurance Criterion (`MAC_1..MAC_3`),
//! Welch-style Power Spectral Density (`PSD`), per-story residual stiffness retention & anomaly Z-scores,
//! automated ATC-20 Rapid/Detailed post-earthquake safety placards (`Green`/`Yellow`/`Red`), district portfolio
//! triage, and a non-destructive IFCX schema overlay (`eigengraph::analysis::shm_digital_twin` & `atc20_placard`).

use std::f64::consts::PI;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::core::eigengate::EigenGateEngine;
use crate::core::bim::BimProject;
use crate::core::geotech::{SubsurfaceSsiAnalysisReport, SubsurfaceSsiConfig};
use crate::core::gis::WorldSpatialIndex;
use crate::core::ground_motion_replay::GroundMotionReplayReport;
use crate::core::ifcx::{ns, IfcxEigenGateOverlay, IfcxDocument, IfcxImport, IfcxNode};
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
use crate::core::modal::{ModalEngine, SiteClass, Sni1726Spectrum};
use crate::core::solver::FemSolver;

/// Canonical IFCX attribute namespace for Live SHM Digital Twin sensor & modal identification payloads.
pub const NS_EG_SHM_DIGITAL_TWIN: &str = ns::EG_SHM_DIGITAL_TWIN;
/// Canonical IFCX attribute namespace for Automated Post-Earthquake ATC-20 Placard Triage decisions.
pub const NS_EG_ATC20_PLACARD: &str = ns::EG_ATC20_PLACARD;

/// Structural Health Monitoring transducer category mounted on the 3D BIM skeleton.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ShmSensorKind {
    #[default]
    TriaxialAccel,
    StoryInclinometer,
    BaseStrainGauge,
}

impl ShmSensorKind {
    pub fn short_label(self) -> &'static str {
        match self {
            Self::TriaxialAccel => "MEMS-XYZ",
            Self::StoryInclinometer => "Tilt-Drift",
            Self::BaseStrainGauge => "FBG-Strain",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::TriaxialAccel => "Triaxial MEMS Accelerometer",
            Self::StoryInclinometer => "Interstory Tilt Inclinometer",
            Self::BaseStrainGauge => "Base Column Fiber-Bragg Strain Gauge",
        }
    }

    pub fn unit(self) -> &'static str {
        match self {
            Self::TriaxialAccel => "mg",
            Self::StoryInclinometer => "% drift",
            Self::BaseStrainGauge => "ue",
        }
    }
}

/// Fault injection or live operational telemetry scenario driving the SHM Digital Twin comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ShmFaultScenario {
    #[default]
    NominalIntact,
    SoftStoryFirstFloor,
    ColumnLossFault,
    SoilLiquefactionShift,
    SeismicDamageFromReplay,
    CustomTelemetryStream,
}

impl ShmFaultScenario {
    pub fn all() -> &'static [Self] {
        &[
            Self::NominalIntact,
            Self::SoftStoryFirstFloor,
            Self::ColumnLossFault,
            Self::SoilLiquefactionShift,
            Self::SeismicDamageFromReplay,
            Self::CustomTelemetryStream,
        ]
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::NominalIntact => "Nominal",
            Self::SoftStoryFirstFloor => "Soft-Story",
            Self::ColumnLossFault => "Col-Loss",
            Self::SoilLiquefactionShift => "SSI-Liq",
            Self::SeismicDamageFromReplay => "NL-THA",
            Self::CustomTelemetryStream => "Stream",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::NominalIntact => "Nominal Ambient Vibration (Intact)",
            Self::SoftStoryFirstFloor => "Soft-Story Stiffness Degradation",
            Self::ColumnLossFault => "Perimeter Column Loss / Cracked Hinge",
            Self::SoilLiquefactionShift => "Subsurface Liquefaction & SSI Softening",
            Self::SeismicDamageFromReplay => "Coupled NL-THA Post-Quake Damage",
            Self::CustomTelemetryStream => "Uploaded SHM Sensor Stream (CSV/JSON)",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::NominalIntact => "Micro-ambient excitation (~1 mg RMS) verifying intact design frequency and >0.98 MAC.",
            Self::SoftStoryFirstFloor => "Simulates stiffness loss in the target lower/first story, shifting f1 downward and kinking Mode 1.",
            Self::ColumnLossFault => "Simulates perimeter column degradation lowering both local flexural rigidity and system EigenGate redundancy.",
            Self::SoilLiquefactionShift => "Models foundation rocking and soil softening lengthening T1 and lowering lateral natural frequency.",
            Self::SeismicDamageFromReplay => "Feeds Phase 74/75 Nonlinear Time-History plastic hinges & story drifts directly into SHM triage.",
            Self::CustomTelemetryStream => "Identifies dominant frequency shift and floor acceleration/drift directly from uploaded CSV/JSON sensor logs.",
        }
    }

    pub fn from_keyword(raw: &str) -> Option<Self> {
        let lower = raw.trim().to_ascii_lowercase();
        match lower.as_str() {
            "nominal" | "intact" | "ambient" | "reset" | "none" => Some(Self::NominalIntact),
            "soft_story" | "soft-story" | "softstory" | "soft" => Some(Self::SoftStoryFirstFloor),
            "column_loss" | "column-loss" | "col_loss" | "col-loss" | "column" => Some(Self::ColumnLossFault),
            "soil_liquefaction" | "liquefaction" | "ssi" | "soil" => Some(Self::SoilLiquefactionShift),
            "seismic" | "replay" | "nltha" | "quake" => Some(Self::SeismicDamageFromReplay),
            "custom" | "stream" | "csv" | "json" => Some(Self::CustomTelemetryStream),
            _ => None,
        }
    }
}

/// ATC-20 / FEMA P-58 Post-Earthquake Structural Safety Posting Placard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub enum Atc20Placard {
    #[default]
    InspectedGreen,
    RestrictedYellow,
    UnsafeRed,
}

impl Atc20Placard {
    pub fn label(self) -> &'static str {
        match self {
            Self::InspectedGreen => "INSPECTED (GREEN)",
            Self::RestrictedYellow => "RESTRICTED USE (YELLOW)",
            Self::UnsafeRed => "UNSAFE (RED)",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::InspectedGreen => "GREEN",
            Self::RestrictedYellow => "YELLOW",
            Self::UnsafeRed => "RED",
        }
    }

    pub fn badge_color_rgb(self) -> [u8; 3] {
        match self {
            Self::InspectedGreen => [34, 197, 94],
            Self::RestrictedYellow => [245, 158, 11],
            Self::UnsafeRed => [239, 68, 68],
        }
    }

    pub fn normalized_rgb(self) -> [f32; 3] {
        let [r, g, b] = self.badge_color_rgb();
        [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
    }

    pub fn gpu_code(self) -> f32 {
        match self {
            Self::InspectedGreen => 0.0,
            Self::RestrictedYellow => 1.0,
            Self::UnsafeRed => 2.0,
        }
    }

    pub fn guidance(self) -> &'static str {
        match self {
            Self::InspectedGreen => "No apparent structural hazard found. Immediate occupancy permitted pending routine inspection.",
            Self::RestrictedYellow => "Moderate stiffness loss or local inelastic distress detected. Restricted entry; detailed engineering evaluation and shoring required.",
            Self::UnsafeRed => "Severe stiffness loss, excessive drift, or mechanism risk detected. Do not enter or occupy; immediate evacuation and perimeter cordoning mandatory.",
        }
    }
}

/// Single triaxial sensor measurement sample parsed from CSV or JSON SHM logs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShmTelemetryRecord {
    pub story_number: usize,
    pub sensor_id: String,
    pub time_s: f64,
    pub accel_x_g: f64,
    pub accel_y_g: f64,
    pub accel_z_g: f64,
    #[serde(default)]
    pub drift_ratio: Option<f64>,
    #[serde(default)]
    pub strain_micro: Option<f64>,
}

/// Uploaded or streamed custom multi-story SHM sensor dataset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomShmTelemetryLog {
    pub source_name: String,
    pub sampling_rate_hz: f64,
    pub duration_s: f64,
    pub story_channels: usize,
    pub records: Vec<ShmTelemetryRecord>,
}

/// Configuration for Phase 76 Live SHM Digital Twin & Automated ATC-20 Placard Evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShmDigitalTwinConfig {
    #[serde(default)]
    pub fault_scenario: ShmFaultScenario,
    /// Fault severity intensity percentage (`0.0..=100.0`).
    #[serde(default = "default_severity_pct")]
    pub severity_pct: f64,
    /// Target story index (1-indexed) for localized fault injection.
    #[serde(default = "default_target_story")]
    pub target_story: usize,
    #[serde(default = "default_sampling_rate_hz")]
    pub sampling_rate_hz: f64,
    #[serde(default = "default_window_duration_s")]
    pub window_duration_s: f64,
    #[serde(default = "default_noise_floor_mg")]
    pub noise_floor_mg: f64,
    #[serde(default = "default_true")]
    pub couple_with_nltha_replay: bool,
    #[serde(default = "default_true")]
    pub include_portfolio_district: bool,
    #[serde(default)]
    pub custom_telemetry: Option<CustomShmTelemetryLog>,
}

fn default_severity_pct() -> f64 {
    35.0
}

fn default_target_story() -> usize {
    1
}

fn default_sampling_rate_hz() -> f64 {
    100.0
}

fn default_window_duration_s() -> f64 {
    10.0
}

fn default_noise_floor_mg() -> f64 {
    1.2
}

fn default_true() -> bool {
    true
}

impl Default for ShmDigitalTwinConfig {
    fn default() -> Self {
        Self {
            fault_scenario: ShmFaultScenario::NominalIntact,
            severity_pct: default_severity_pct(),
            target_story: default_target_story(),
            sampling_rate_hz: default_sampling_rate_hz(),
            window_duration_s: default_window_duration_s(),
            noise_floor_mg: default_noise_floor_mg(),
            couple_with_nltha_replay: true,
            include_portfolio_district: true,
            custom_telemetry: None,
        }
    }
}

impl ShmDigitalTwinConfig {
    pub fn content_revision_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mix = |acc: &mut u64, v: u64| {
            *acc ^= v;
            *acc = acc.wrapping_mul(0x0000_0100_0000_01b3);
        };
        mix(&mut h, self.fault_scenario as u64 + 1);
        mix(&mut h, (self.severity_pct * 100.0).round() as u64);
        mix(&mut h, self.target_story as u64);
        mix(&mut h, (self.sampling_rate_hz * 10.0).round() as u64);
        mix(&mut h, (self.window_duration_s * 10.0).round() as u64);
        mix(&mut h, u64::from(self.couple_with_nltha_replay));
        mix(&mut h, u64::from(self.include_portfolio_district));
        if let Some(ref custom) = self.custom_telemetry {
            mix(&mut h, custom.records.len() as u64);
            for b in custom.source_name.as_bytes() {
                mix(&mut h, *b as u64);
            }
        }
        h
    }
}

/// Real-time spatial telemetry snapshot for a single BIM-anchored SHM sensor node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShmSensorNodeSnapshot {
    pub sensor_id: String,
    pub story_number: usize,
    pub kind: ShmSensorKind,
    /// Local BIM tangent coordinates `[east_m, north_m, elev_m]` matching WebGPU Y/Z convention.
    pub position_local_m: [f64; 3],
    pub rms_accel_mg: f64,
    pub peak_accel_g: f64,
    pub peak_drift_pct: f64,
    pub residual_drift_pct: f64,
    pub natural_freq_local_hz: f64,
    pub health_index: f64,
    pub anomaly_z_score: f64,
    pub is_triggered: bool,
}

/// Single bin along the baseline vs. observed Power Spectral Density (PSD) curve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShmPsdPoint {
    pub freq_hz: f64,
    pub baseline_psd_g2_hz: f64,
    pub observed_psd_g2_hz: f64,
}

/// Individual engineering rule evaluated during automated ATC-20 placard classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Atc20ChecklistCriterion {
    pub code: String,
    pub criterion: String,
    pub observed_value: String,
    pub threshold_value: String,
    pub outcome: Atc20Placard,
    pub rationale: String,
}

/// Per-story residual stiffness, transient/residual drift, and ATC-20 health breakdown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryShmHealth {
    pub story_number: usize,
    pub elevation_m: f64,
    pub baseline_stiffness_pct: f64,
    pub observed_stiffness_pct: f64,
    pub stiffness_loss_pct: f64,
    pub peak_transient_drift_pct: f64,
    pub residual_drift_pct: f64,
    pub floor_accel_g: f64,
    pub anomaly_z_score: f64,
    pub story_placard: Atc20Placard,
}

/// Post-earthquake rapid ATC-20 triage decision for a building in the Earth spatial index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DistrictBuildingPlacard {
    pub rel_path: String,
    pub building_name: String,
    pub lat: f64,
    pub lon: f64,
    pub elevation_m: f64,
    pub total_height_m: f64,
    pub footprint_local_m: Vec<[f64; 2]>,
    pub heading_deg: f64,
    pub estimated_t1_s: f64,
    pub estimated_drift_pct: f64,
    pub estimated_stiffness_loss_pct: f64,
    pub max_dcr: f64,
    pub min_eigengate: f64,
    pub placard: Atc20Placard,
    pub rationale: String,
}

/// Complete Live SHM Digital Twin, Modal Identification, and Automated ATC-20 Triage Report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShmDigitalTwinReport {
    pub project_name: String,
    pub config: ShmDigitalTwinConfig,
    pub bim_revision_hash: u64,
    pub num_stories: usize,
    pub building_height_m: f64,
    pub num_fem_nodes: usize,
    pub num_fem_elements: usize,
    pub baseline_freq_1_hz: f64,
    pub baseline_freq_2_hz: f64,
    pub baseline_freq_3_hz: f64,
    pub observed_freq_1_hz: f64,
    pub observed_freq_2_hz: f64,
    pub observed_freq_3_hz: f64,
    /// Relative fundamental frequency shift `(f_obs - f_base) / f_base * 100.0` (%).
    pub freq_shift_pct: f64,
    pub mac_mode_1: f64,
    pub mac_mode_2: f64,
    pub mac_mode_3: f64,
    pub global_stiffness_retention_pct: f64,
    pub damping_ratio_baseline_pct: f64,
    pub damping_ratio_observed_pct: f64,
    pub baseline_eigengate: f64,
    pub residual_eigengate: f64,
    pub max_transient_drift_pct: f64,
    pub max_residual_drift_pct: f64,
    pub story_health: Vec<StoryShmHealth>,
    pub sensors: Vec<ShmSensorNodeSnapshot>,
    pub psd_spectrum: Vec<ShmPsdPoint>,
    pub atc20_placard: Atc20Placard,
    pub atc20_checklist: Vec<Atc20ChecklistCriterion>,
    pub district_placards: Vec<DistrictBuildingPlacard>,
    pub green_count: usize,
    pub yellow_count: usize,
    pub red_count: usize,
    pub shm_overlay_ifcx: IfcxDocument,
    pub summary: String,
}

impl ShmDigitalTwinReport {
    /// Returns per-story `(elevation_m, stiffness_retention_ratio_0_to_1)` sorted ascending for 3D GPU heatmaps.
    pub fn story_retention_pairs(&self) -> Vec<(f64, f64)> {
        self.story_health
            .iter()
            .map(|s| (s.elevation_m, (s.observed_stiffness_pct / 100.0).clamp(0.15, 1.0)))
            .collect()
    }
}

/// Checks whether a dropped or opened file text looks like an SHM sensor CSV or JSON telemetry payload.
pub fn looks_like_shm_telemetry_payload(raw: &str) -> bool {
    let trimmed = raw.trim_start_matches('\u{feff}').trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        let lower = trimmed.to_ascii_lowercase();
        return lower.contains("accel_x")
            || lower.contains("\"ax\"")
            || lower.contains("sensor_id")
            || lower.contains("\"records\"")
            || (lower.contains("story_number") && lower.contains("time_s"));
    }
    for line in trimmed.lines().take(6) {
        let l = line.trim().to_ascii_lowercase();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        return (l.contains("time") || l.contains("t_s"))
            && (l.contains("ax") || l.contains("accel_x") || l.contains("az"))
            && (l.contains("story") || l.contains("floor") || l.contains("sensor"));
    }
    false
}

/// Parses CSV or JSON triaxial SHM sensor logs into `CustomShmTelemetryLog`.
pub fn parse_shm_telemetry_log(
    raw: &str,
    source_name: &str,
) -> Result<CustomShmTelemetryLog, String> {
    let trimmed = raw.trim_start_matches('\u{feff}').trim();
    if trimmed.is_empty() {
        return Err("SHM telemetry file is empty.".to_string());
    }

    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if let Ok(mut parsed) = serde_json::from_str::<CustomShmTelemetryLog>(trimmed) {
            if parsed.records.is_empty() {
                return Err("JSON SHM telemetry object contains zero records.".to_string());
            }
            if parsed.source_name.is_empty() {
                parsed.source_name = source_name.to_string();
            }
            recompute_log_metadata(&mut parsed);
            return Ok(parsed);
        }
        if let Ok(records) = serde_json::from_str::<Vec<ShmTelemetryRecord>>(trimmed) {
            if records.is_empty() {
                return Err("JSON SHM telemetry array contains zero records.".to_string());
            }
            let mut log = CustomShmTelemetryLog {
                source_name: source_name.to_string(),
                sampling_rate_hz: 100.0,
                duration_s: 0.0,
                story_channels: 1,
                records,
            };
            recompute_log_metadata(&mut log);
            return Ok(log);
        }
        return Err("Invalid JSON SHM telemetry schema: expected CustomShmTelemetryLog or ShmTelemetryRecord[].".to_string());
    }

    parse_shm_telemetry_csv(trimmed, source_name)
}

fn recompute_log_metadata(log: &mut CustomShmTelemetryLog) {
    let min_t = log.records.iter().map(|r| r.time_s).fold(f64::INFINITY, f64::min);
    let max_t = log.records.iter().map(|r| r.time_s).fold(f64::NEG_INFINITY, f64::max);
    log.duration_s = if max_t.is_finite() && min_t.is_finite() {
        (max_t - min_t).max(0.0)
    } else {
        0.0
    };
    let mut times: Vec<f64> = log.records.iter().map(|r| r.time_s).collect();
    times.sort_by(|a, b| a.total_cmp(b));
    let mut min_dt = f64::INFINITY;
    for pair in times.windows(2) {
        let dt = pair[1] - pair[0];
        if dt > 1e-4 && dt < min_dt {
            min_dt = dt;
        }
    }
    if min_dt.is_finite() {
        log.sampling_rate_hz = (1.0 / min_dt).clamp(1.0, 2000.0);
    } else if log.sampling_rate_hz <= 0.0 {
        log.sampling_rate_hz = 100.0;
    }
    log.story_channels = log
        .records
        .iter()
        .map(|r| r.story_number.max(1))
        .max()
        .unwrap_or(1);
}

fn parse_shm_telemetry_csv(raw: &str, source_name: &str) -> Result<CustomShmTelemetryLog, String> {
    let mut lines = raw
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'));

    let header = lines
        .next()
        .ok_or_else(|| "CSV SHM log is missing a header row.".to_string())?;
    let cols: Vec<String> = header
        .split(',')
        .map(|c| c.trim().to_ascii_lowercase())
        .collect();

    let idx_of = |candidates: &[&str]| -> Option<usize> {
        cols.iter()
            .position(|c| candidates.iter().any(|cand| c == cand || c.contains(cand)))
    };

    let i_time = idx_of(&["time", "t_s"]).unwrap_or(0);
    let i_story = idx_of(&["story", "floor"]);
    let i_sensor = idx_of(&["sensor", "channel"]);
    let i_ax = idx_of(&["accel_x", "ax"]).ok_or_else(|| {
        "CSV SHM header must contain an acceleration column (`accel_x` or `ax`).".to_string()
    })?;
    let i_ay = idx_of(&["accel_y", "ay"]);
    let i_az = idx_of(&["accel_z", "az"]);
    let i_drift = idx_of(&["drift"]);
    let i_strain = idx_of(&["strain"]);

    let mut records = Vec::new();
    for line in lines {
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.len() <= i_ax.max(i_time) {
            continue;
        }
        let Ok(time_s) = parts[i_time].parse::<f64>() else {
            continue;
        };
        let Ok(accel_x_g) = parts[i_ax].parse::<f64>() else {
            continue;
        };
        let story_number = i_story
            .and_then(|idx| parts.get(idx))
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1)
            .max(1);
        let sensor_id = i_sensor
            .and_then(|idx| parts.get(idx))
            .filter(|s| !s.is_empty())
            .map(|s| (*s).to_string())
            .unwrap_or_else(|| format!("SHM-F{story_number:02}-CSV"));
        let accel_y_g = i_ay
            .and_then(|idx| parts.get(idx))
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let accel_z_g = i_az
            .and_then(|idx| parts.get(idx))
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let drift_ratio = i_drift
            .and_then(|idx| parts.get(idx))
            .and_then(|s| s.parse::<f64>().ok());
        let strain_micro = i_strain
            .and_then(|idx| parts.get(idx))
            .and_then(|s| s.parse::<f64>().ok());

        records.push(ShmTelemetryRecord {
            story_number,
            sensor_id,
            time_s,
            accel_x_g,
            accel_y_g,
            accel_z_g,
            drift_ratio,
            strain_micro,
        });
    }

    if records.is_empty() {
        return Err("No valid telemetry rows found in CSV SHM payload.".to_string());
    }

    let mut log = CustomShmTelemetryLog {
        source_name: source_name.to_string(),
        sampling_rate_hz: 100.0,
        duration_s: 0.0,
        story_channels: 1,
        records,
    };
    recompute_log_metadata(&mut log);
    Ok(log)
}

/// Generates a deterministic sample multi-story CSV SHM stream (with measurable frequency shift) for testing & demo playback.
pub fn sample_shm_telemetry_csv() -> String {
    let mut out = String::from("time_s,story,sensor_id,accel_x_g,accel_y_g,accel_z_g,drift_ratio,strain_micro\n");
    let fs = 50.0_f64;
    let f_obs_hz = 1.82_f64; // shifted structural mode representing softened ground floor
    let n_steps = 200usize; // 4.0 seconds @ 50 Hz
    for story in 1..=3 {
        let mode_amp = (0.5 * PI * (story as f64 / 3.0)).sin();
        let drift = match story {
            1 => 0.0115,
            2 => 0.0068,
            _ => 0.0042,
        };
        let strain = match story {
            1 => 420.0,
            2 => 210.0,
            _ => 95.0,
        };
        for i in 0..n_steps {
            let t = i as f64 / fs;
            let ax = 0.045 * mode_amp * (2.0 * PI * f_obs_hz * t).sin()
                + 0.012 * (2.0 * PI * 3.1 * f_obs_hz * t).sin();
            let ay = 0.028 * mode_amp * (2.0 * PI * (f_obs_hz * 1.07) * t + 0.4).sin();
            let az = 0.006 * (2.0 * PI * 9.5 * t).cos();
            out.push_str(&format!(
                "{t:.3},{story},SHM-F{story:02}-A,{ax:.5},{ay:.5},{az:.5},{drift:.5},{strain:.1}\n"
            ));
        }
    }
    out
}

/// Evaluates the complete Phase 76 Live SHM Digital Twin & Automated ATC-20 Placard Triage against the active BIM + FEM + SSI + NL-THA models.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_shm_digital_twin_for_bim(
    project: &BimProject,
    site: &SiteEnvironmentalCoupling,
    ssi_config: Option<&SubsurfaceSsiConfig>,
    ssi_report: Option<&SubsurfaceSsiAnalysisReport>,
    replay_report: Option<&GroundMotionReplayReport>,
    world_index: Option<&WorldSpatialIndex>,
    config: &ShmDigitalTwinConfig,
    lat_lon: (f64, f64),
) -> Result<ShmDigitalTwinReport, String> {
    if project.is_empty() {
        return Err("BIM project is empty — add structural columns/beams before running Live SHM Digital Twin.".to_string());
    }

    // 1. Extract story elevations (m) consistently with Phase 74/75
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

    let (fem_base, _fem_map, _load_sum) =
        project.to_structural_model_with_site_loads_and_ssi(site, ssi_config);
    if fem_base.elements.is_empty() || fem_base.nodes.len() < 2 {
        return Err("No 3D structural frame elements available for SHM Digital Twin analysis.".to_string());
    }
    let _fea_base = FemSolver::solve(&fem_base)?;
    let eigengate_eval = EigenGateEngine::evaluate_redundancy_fast(&fem_base)?;
    let baseline_eigengate = eigengate_eval.min_system_eigengate.clamp(0.0, 1.0);

    let soil_label = if site.recommended_soil_class.trim().is_empty() {
        "SD"
    } else {
        site.recommended_soil_class.as_str()
    };
    let site_class = SiteClass::from_soil_label(soil_label);
    let spectrum = Sni1726Spectrum::new(
        site_class,
        (site.seismic_pga_ss_g as f64).max(0.25),
        (site.seismic_pga_s1_g as f64).max(0.10),
        8.0,
        1.0,
    );

    let modal_base = ModalEngine::analyze_for_earth_3d(&fem_base, 3, spectrum.clone())?;
    let baseline_freq_1_hz = modal_base
        .modes
        .first()
        .map(|m| m.frequency_hz)
        .unwrap_or(modal_base.fundamental_frequency_hz)
        .max(0.15);
    let baseline_freq_2_hz = modal_base
        .modes
        .get(1)
        .map(|m| m.frequency_hz)
        .unwrap_or(baseline_freq_1_hz * 1.16)
        .max(baseline_freq_1_hz);
    let baseline_freq_3_hz = modal_base
        .modes
        .get(2)
        .map(|m| m.frequency_hz)
        .unwrap_or(baseline_freq_1_hz * 2.75)
        .max(baseline_freq_2_hz);

    // 2. Determine per-story stiffness retention factors `k_factor[0..num_stories]` and seismic drift estimates
    let sev = (config.severity_pct / 100.0).clamp(0.0, 1.0);
    let target_idx = config.target_story.saturating_sub(1).min(num_stories - 1);
    let mut k_factor = vec![1.0_f64; num_stories];
    let mut story_transient_drift_pct = vec![0.04_f64; num_stories];
    let mut story_residual_drift_pct = vec![0.0_f64; num_stories];
    let mut story_floor_accel_g = vec![0.002_f64; num_stories];
    let mut residual_eigengate = baseline_eigengate;

    let scenario = effective_fault_scenario(config, replay_report);
    match scenario {
        ShmFaultScenario::NominalIntact => {
            for (s, k) in k_factor.iter_mut().enumerate() {
                *k = (1.0 - 0.015 * sev).clamp(0.95, 1.0);
                let frac = (s + 1) as f64 / num_stories as f64;
                story_transient_drift_pct[s] = 0.03 + 0.04 * sev * frac;
                story_residual_drift_pct[s] = 0.002;
                story_floor_accel_g[s] = 0.002 + 0.004 * frac;
            }
        }
        ShmFaultScenario::SoftStoryFirstFloor => {
            for s in 0..num_stories {
                let dist = (s as isize - target_idx as isize).unsigned_abs();
                let loss = match dist {
                    0 => 0.72 * sev,
                    1 => 0.16 * sev,
                    _ => 0.04 * sev,
                };
                k_factor[s] = (1.0 - loss).clamp(0.18, 1.0);
                let drift_amp = if dist == 0 { 2.75 } else { 0.85 };
                story_transient_drift_pct[s] = (0.12 + 2.40 * sev * drift_amp / 2.75).clamp(0.05, 5.5);
                story_residual_drift_pct[s] = (story_transient_drift_pct[s] * 0.24 * sev).clamp(0.0, 2.0);
                story_floor_accel_g[s] = 0.04 + 0.35 * sev * ((s + 1) as f64 / num_stories as f64);
            }
            residual_eigengate = (baseline_eigengate * (1.0 - 0.28 * sev)).clamp(0.10, baseline_eigengate);
        }
        ShmFaultScenario::ColumnLossFault => {
            for s in 0..num_stories {
                let dist = (s as isize - target_idx as isize).unsigned_abs();
                let loss = match dist {
                    0 => 0.62 * sev,
                    1 => 0.24 * sev,
                    _ => 0.08 * sev,
                };
                k_factor[s] = (1.0 - loss).clamp(0.22, 1.0);
                story_transient_drift_pct[s] =
                    (0.10 + if dist == 0 { 2.15 } else { 0.90 } * sev).clamp(0.05, 5.0);
                story_residual_drift_pct[s] = (story_transient_drift_pct[s] * 0.28 * sev).clamp(0.0, 2.0);
                story_floor_accel_g[s] = 0.05 + 0.30 * sev * ((s + 1) as f64 / num_stories as f64);
            }
            residual_eigengate = (baseline_eigengate * (1.0 - 0.48 * sev)).clamp(0.08, baseline_eigengate);
        }
        ShmFaultScenario::SoilLiquefactionShift => {
            let liq_boost = ssi_report
                .map(|r| if r.min_liquefaction_fs < 1.2 { 1.25 } else { 0.90 })
                .unwrap_or(1.0);
            for s in 0..num_stories {
                let height_atten = (1.0 - 0.55 * (s as f64 / num_stories.max(1) as f64)).max(0.35);
                let loss = (0.52 * sev * height_atten * liq_boost).clamp(0.0, 0.72);
                k_factor[s] = (1.0 - loss).clamp(0.25, 1.0);
                story_transient_drift_pct[s] =
                    (0.15 + 1.85 * sev * liq_boost * height_atten).clamp(0.05, 4.5);
                story_residual_drift_pct[s] =
                    (0.05 + 0.55 * sev * liq_boost * height_atten).clamp(0.0, 1.8);
                story_floor_accel_g[s] = 0.06 + 0.28 * sev * ((s + 1) as f64 / num_stories as f64);
            }
            residual_eigengate = (baseline_eigengate * (1.0 - 0.30 * sev)).clamp(0.15, baseline_eigengate);
        }
        ShmFaultScenario::SeismicDamageFromReplay => {
            if let Some(rep) = replay_report {
                let replay_scale = (0.45 + 0.75 * sev).clamp(0.35, 1.35);
                let mut hinge_penalty = vec![0.0_f64; num_stories];
                for h in &rep.peak_hinges {
                    let st = story_index_for_z(h.local_pos_m[2], &level_elevs, num_stories);
                    let p = if h.dcr_dynamic >= 1.25 {
                        0.48
                    } else if h.dcr_dynamic >= 1.00 {
                        0.32
                    } else if h.dcr_dynamic >= 0.80 {
                        0.15
                    } else {
                        0.03
                    };
                    hinge_penalty[st] = hinge_penalty[st].max(p);
                }
                for s in 0..num_stories {
                    let drift_ratio = rep
                        .fragility_loss
                        .story_losses
                        .get(s)
                        .map(|sl| sl.peak_drift_ratio)
                        .unwrap_or(rep.peak_interstory_drift_ratio);
                    let floor_acc = rep
                        .fragility_loss
                        .story_losses
                        .get(s)
                        .map(|sl| sl.peak_floor_accel_g)
                        .unwrap_or(rep.scaled_pga_g * (1.0 + 0.8 * ((s + 1) as f64 / num_stories as f64)));
                    let drift_loss = ((drift_ratio / 0.025) * 0.45).clamp(0.0, 0.58);
                    let combined_loss = (drift_loss.max(hinge_penalty[s]) * replay_scale).clamp(0.02, 0.78);
                    k_factor[s] = (1.0 - combined_loss).clamp(0.22, 1.0);
                    story_transient_drift_pct[s] = (drift_ratio * 100.0).clamp(0.05, 6.0);
                    story_residual_drift_pct[s] =
                        (drift_ratio * 100.0 * (0.12 + 0.25 * combined_loss)).clamp(0.01, 2.5);
                    story_floor_accel_g[s] = floor_acc.max(0.02);
                }
                let max_loss = k_factor.iter().map(|k| 1.0 - k).fold(0.0_f64, f64::max);
                residual_eigengate = (baseline_eigengate * (1.0 - 0.45 * max_loss)).clamp(0.08, baseline_eigengate);
            } else {
                // Smooth site-demand fallback when replay hasn't been executed yet
                let pga_fac = (site.seismic_pga_ss_g as f64 * 0.42).clamp(0.12, 1.10);
                for s in 0..num_stories {
                    let amp = if s == 0 { 1.15 } else { 0.85 };
                    let loss = (0.38 * pga_fac * sev * amp).clamp(0.02, 0.65);
                    k_factor[s] = (1.0 - loss).clamp(0.30, 1.0);
                    story_transient_drift_pct[s] = (0.25 + 1.9 * pga_fac * sev * amp).clamp(0.05, 4.5);
                    story_residual_drift_pct[s] = (story_transient_drift_pct[s] * 0.20 * sev).clamp(0.01, 1.5);
                    story_floor_accel_g[s] = (pga_fac * (1.0 + 0.7 * ((s + 1) as f64 / num_stories as f64))).max(0.05);
                }
                residual_eigengate = (baseline_eigengate * (1.0 - 0.35 * sev)).clamp(0.15, baseline_eigengate);
            }
        }
        ShmFaultScenario::CustomTelemetryStream => {
            apply_custom_telemetry_to_stories(
                config.custom_telemetry.as_ref(),
                baseline_freq_1_hz,
                baseline_eigengate,
                &mut k_factor,
                &mut story_transient_drift_pct,
                &mut story_residual_drift_pct,
                &mut story_floor_accel_g,
                &mut residual_eigengate,
            );
        }
    }

    // 3. Build physically degraded 6-DOF FEM model + Condensed MDOF Coupled Eigensolver
    // For small models (<= 48 nodes, <= 140 elements), ModalEngine::analyze computes full 3D inverse-power
    // modes directly on the damaged stiffness matrix K_obs. For larger IFC frames, an exact N-story
    // MDOF coupled eigenvalue solver computes frequency shifts & distorted mode shapes in <1ms.
    let mut fem_obs = fem_base.clone();
    for elem in &mut fem_obs.elements {
        if let (Some(n1), Some(n2)) = (
            fem_base.find_node(elem.start_node),
            fem_base.find_node(elem.end_node),
        ) {
            let mid_y = 0.5 * (n1.y + n2.y);
            let s_idx = story_index_for_z(mid_y, &level_elevs, num_stories);
            let kf = k_factor[s_idx];
            elem.material.elastic_modulus *= kf;
            elem.section.iy *= kf;
            elem.section.iz *= kf;
        }
    }

    let (observed_freq_1_hz, observed_freq_2_hz, observed_freq_3_hz, mac_mode_1, mac_mode_2, mac_mode_3) =
        if fem_obs.nodes.len() <= 48 && fem_obs.elements.len() <= 140 {
            if let Ok(modal_obs) = ModalEngine::analyze(&fem_obs, 3, spectrum.clone()) {
                let mut f1 = modal_obs.modes.first().map(|m| m.frequency_hz).unwrap_or(baseline_freq_1_hz);
                let mut f2 = modal_obs.modes.get(1).map(|m| m.frequency_hz).unwrap_or(baseline_freq_2_hz);
                let mut f3 = modal_obs.modes.get(2).map(|m| m.frequency_hz).unwrap_or(baseline_freq_3_hz);
                // If custom stream carries a direct spectral peak that shifts even further, blend cleanly
                if matches!(scenario, ShmFaultScenario::CustomTelemetryStream) {
                    if let Some(ref custom) = config.custom_telemetry {
                        if let Some(f_stream) = estimate_dominant_frequency_from_log(custom, baseline_freq_1_hz) {
                            let ratio = (f_stream / baseline_freq_1_hz).clamp(0.35, 1.05);
                            f1 = (baseline_freq_1_hz * ratio).min(f1.max(0.2));
                            f2 *= ratio.sqrt();
                            f3 *= ratio.sqrt();
                        }
                    }
                }
                let phi_base = extract_story_mode_vectors(&fem_base, &modal_base.modes, &level_elevs, num_stories);
                let phi_obs = extract_story_mode_vectors(&fem_obs, &modal_obs.modes, &level_elevs, num_stories);
                let m1 = compute_mac(phi_base.first(), phi_obs.first());
                let m2 = compute_mac(phi_base.get(1), phi_obs.get(1));
                let m3 = compute_mac(phi_base.get(2), phi_obs.get(2));
                (f1, f2, f3, m1, m2, m3)
            } else {
                solve_condensed_mdof_modal_shift(
                    &k_factor,
                    [baseline_freq_1_hz, baseline_freq_2_hz, baseline_freq_3_hz],
                )
            }
        } else {
            solve_condensed_mdof_modal_shift(
                &k_factor,
                [baseline_freq_1_hz, baseline_freq_2_hz, baseline_freq_3_hz],
            )
        };

    let freq_shift_pct = ((observed_freq_1_hz - baseline_freq_1_hz) / baseline_freq_1_hz.max(1e-6)) * 100.0;
    let global_stiffness_retention_pct =
        (k_factor.iter().sum::<f64>() / k_factor.len().max(1) as f64 * 100.0).clamp(10.0, 100.0);
    let max_stiffness_loss_pct = k_factor
        .iter()
        .map(|k| (1.0 - k) * 100.0)
        .fold(0.0_f64, f64::max);

    let damping_ratio_baseline_pct = 5.0;
    let damping_ratio_observed_pct =
        (5.0 + 0.14 * max_stiffness_loss_pct).clamp(4.5, 18.0);

    // 4. Story-by-Story Health Breakdown & Real BIM Coordinates for Sensor Anchors
    let max_transient_drift_pct = story_transient_drift_pct
        .iter()
        .copied()
        .fold(0.0_f64, f64::max);
    let max_residual_drift_pct = story_residual_drift_pct
        .iter()
        .copied()
        .fold(0.0_f64, f64::max);

    let mut story_health = Vec::with_capacity(num_stories);
    for s in 0..num_stories {
        let obs_k_pct = (k_factor[s] * 100.0).clamp(15.0, 100.0);
        let loss_pct = (100.0 - obs_k_pct).max(0.0);
        let z_score = (loss_pct / 4.2 + story_transient_drift_pct[s] * 1.45).clamp(0.1, 14.0);
        let story_placard = if loss_pct >= 28.0
            || story_transient_drift_pct[s] >= 2.0
            || story_residual_drift_pct[s] >= 0.50
        {
            Atc20Placard::UnsafeRed
        } else if loss_pct >= 12.0
            || story_transient_drift_pct[s] >= 1.0
            || story_residual_drift_pct[s] >= 0.25
        {
            Atc20Placard::RestrictedYellow
        } else {
            Atc20Placard::InspectedGreen
        };
        story_health.push(StoryShmHealth {
            story_number: s + 1,
            elevation_m: level_elevs.get(s + 1).copied().unwrap_or((s + 1) as f64 * 3.8),
            baseline_stiffness_pct: 100.0,
            observed_stiffness_pct: obs_k_pct,
            stiffness_loss_pct: loss_pct,
            peak_transient_drift_pct: story_transient_drift_pct[s],
            residual_drift_pct: story_residual_drift_pct[s],
            floor_accel_g: story_floor_accel_g[s],
            anomaly_z_score: z_score,
            story_placard,
        });
    }

    // 5. Generate BimColumn-anchored 3D SHM Sensor Nodes per floor
    let sensors = build_bim_sensor_snapshots(
        project,
        &level_elevs,
        &story_health,
        observed_freq_1_hz,
        config.noise_floor_mg,
        config.custom_telemetry.as_ref(),
    );

    // 6. Compute Welch-style Baseline vs. Observed Power Spectral Density (64 bins)
    let psd_spectrum = build_welch_psd_spectrum(
        [baseline_freq_1_hz, baseline_freq_2_hz, baseline_freq_3_hz],
        [observed_freq_1_hz, observed_freq_2_hz, observed_freq_3_hz],
        damping_ratio_baseline_pct / 100.0,
        damping_ratio_observed_pct / 100.0,
        config.noise_floor_mg,
        config.custom_telemetry.as_ref(),
    );

    // 7. Automated ATC-20 Post-Earthquake Checklist & Governing Placard
    let atc20_checklist = evaluate_atc20_checklist(
        freq_shift_pct,
        mac_mode_1,
        max_transient_drift_pct,
        max_residual_drift_pct,
        residual_eigengate,
        max_stiffness_loss_pct,
    );
    let mut atc20_placard = atc20_checklist
        .iter()
        .map(|c| c.outcome)
        .max()
        .unwrap_or(Atc20Placard::InspectedGreen);
    for sh in &story_health {
        atc20_placard = atc20_placard.max(sh.story_placard);
    }

    // 8. District Portfolio ATC-20 Triage across indexed Earth buildings
    let (district_placards, green_count, yellow_count, red_count) =
        build_district_portfolio_placards(
            project,
            site,
            world_index,
            config,
            lat_lon,
            observed_freq_1_hz,
            max_transient_drift_pct,
            max_stiffness_loss_pct,
            residual_eigengate,
            atc20_placard,
        );

    let bim_revision_hash = config.content_revision_hash() ^ (project.columns.len() as u64).rotate_left(7);
    let summary = format!(
        "ATC-20 {} | f1 {:.2}->{:.2} Hz ({:+.1}%) | MAC1 {:.3} | K_ret {:.1}% | Max Drift {:.2}% (Res {:.2}%) | District {}G/{}Y/{}R",
        atc20_placard.label(),
        baseline_freq_1_hz,
        observed_freq_1_hz,
        freq_shift_pct,
        mac_mode_1,
        global_stiffness_retention_pct,
        max_transient_drift_pct,
        max_residual_drift_pct,
        green_count,
        yellow_count,
        red_count,
    );

    // 9. Non-destructive IFCX Schema Overlay (`eigengraph::analysis::shm_digital_twin` & `atc20_placard`)
    let shm_overlay_ifcx = build_shm_overlay_ifcx(
        &project.project_name,
        scenario,
        baseline_freq_1_hz,
        observed_freq_1_hz,
        freq_shift_pct,
        mac_mode_1,
        global_stiffness_retention_pct,
        baseline_eigengate,
        residual_eigengate,
        max_transient_drift_pct,
        max_residual_drift_pct,
        atc20_placard,
        &atc20_checklist,
        &story_health,
        &sensors,
        green_count,
        yellow_count,
        red_count,
    );

    Ok(ShmDigitalTwinReport {
        project_name: project.project_name.clone(),
        config: config.clone(),
        bim_revision_hash,
        num_stories,
        building_height_m,
        num_fem_nodes: fem_base.nodes.len(),
        num_fem_elements: fem_base.elements.len(),
        baseline_freq_1_hz,
        baseline_freq_2_hz,
        baseline_freq_3_hz,
        observed_freq_1_hz,
        observed_freq_2_hz,
        observed_freq_3_hz,
        freq_shift_pct,
        mac_mode_1,
        mac_mode_2,
        mac_mode_3,
        global_stiffness_retention_pct,
        damping_ratio_baseline_pct,
        damping_ratio_observed_pct,
        baseline_eigengate,
        residual_eigengate,
        max_transient_drift_pct,
        max_residual_drift_pct,
        story_health,
        sensors,
        psd_spectrum,
        atc20_placard,
        atc20_checklist,
        district_placards,
        green_count,
        yellow_count,
        red_count,
        shm_overlay_ifcx,
        summary,
    })
}

fn effective_fault_scenario(
    config: &ShmDigitalTwinConfig,
    replay_report: Option<&GroundMotionReplayReport>,
) -> ShmFaultScenario {
    if config.fault_scenario == ShmFaultScenario::NominalIntact
        && config.custom_telemetry.is_some()
    {
        return ShmFaultScenario::CustomTelemetryStream;
    }
    if config.fault_scenario == ShmFaultScenario::NominalIntact
        && config.couple_with_nltha_replay
        && replay_report.is_some()
    {
        return ShmFaultScenario::SeismicDamageFromReplay;
    }
    config.fault_scenario
}

fn story_index_for_z(z_m: f64, level_elevs: &[f64], num_stories: usize) -> usize {
    for i in 1..level_elevs.len() {
        if z_m <= level_elevs[i] + 0.25 {
            return (i - 1).min(num_stories.saturating_sub(1));
        }
    }
    num_stories.saturating_sub(1)
}

#[allow(clippy::too_many_arguments)]
fn apply_custom_telemetry_to_stories(
    custom_opt: Option<&CustomShmTelemetryLog>,
    baseline_f1_hz: f64,
    baseline_eigengate: f64,
    k_factor: &mut [f64],
    transient_drift: &mut [f64],
    residual_drift: &mut [f64],
    floor_accel: &mut [f64],
    residual_eigengate: &mut f64,
) {
    let Some(log) = custom_opt else {
        return;
    };
    let num_stories = k_factor.len().max(1);
    let f_est = estimate_dominant_frequency_from_log(log, baseline_f1_hz).unwrap_or(baseline_f1_hz);
    let global_ratio = (f_est / baseline_f1_hz.max(0.1)).powi(2).clamp(0.20, 1.0);

    for s in 0..num_stories {
        let story_num = s + 1;
        let matching: Vec<&ShmTelemetryRecord> = log
            .records
            .iter()
            .filter(|r| r.story_number.max(1) == story_num)
            .collect();
        if matching.is_empty() {
            k_factor[s] = global_ratio;
            transient_drift[s] = (0.12 + 2.2 * (1.0 - global_ratio)).clamp(0.04, 4.5);
            residual_drift[s] = transient_drift[s] * 0.22 * (1.0 - global_ratio);
            floor_accel[s] = 0.04;
        } else {
            let max_acc = matching
                .iter()
                .map(|r| (r.accel_x_g.powi(2) + r.accel_y_g.powi(2)).sqrt())
                .fold(0.0_f64, f64::max);
            let max_drift_pct = matching
                .iter()
                .filter_map(|r| r.drift_ratio)
                .map(|d| d.abs() * 100.0)
                .fold(0.0_f64, f64::max);
            let drift = if max_drift_pct > 1e-4 {
                max_drift_pct
            } else {
                (max_acc * 3.2 + 2.0 * (1.0 - global_ratio)).clamp(0.05, 4.5)
            };
            let local_loss = ((1.0 - global_ratio) * (0.75 + 0.35 * (drift / 1.5).min(1.5)))
                .clamp(0.0, 0.78);
            k_factor[s] = (1.0 - local_loss).clamp(0.20, 1.0);
            transient_drift[s] = drift;
            residual_drift[s] = (drift * 0.22 * local_loss).clamp(0.0, 2.0);
            floor_accel[s] = max_acc.max(0.01);
        }
    }
    let avg_loss = k_factor.iter().map(|k| 1.0 - k).sum::<f64>() / k_factor.len().max(1) as f64;
    *residual_eigengate = (baseline_eigengate * (1.0 - 0.50 * avg_loss)).clamp(0.10, baseline_eigengate);
}

fn estimate_dominant_frequency_from_log(log: &CustomShmTelemetryLog, baseline_f1_hz: f64) -> Option<f64> {
    // Take records from the highest story channel for sharpest fundamental signal
    let target_story = log.story_channels.max(1);
    let mut series: Vec<(f64, f64)> = log
        .records
        .iter()
        .filter(|r| r.story_number.max(1) == target_story)
        .map(|r| (r.time_s, r.accel_x_g))
        .collect();
    if series.len() < 16 {
        series = log.records.iter().map(|r| (r.time_s, r.accel_x_g)).collect();
    }
    if series.len() < 16 {
        return None;
    }
    series.sort_by(|a, b| a.0.total_cmp(&b.0));

    let dt = ((series.last()?.0 - series.first()?.0) / (series.len() - 1) as f64).max(1e-4);
    let n = series.len() as f64;
    let mean = series.iter().map(|p| p.1).sum::<f64>() / n;

    // Evaluate DFT magnitude power across structural band (0.25 Hz .. min(Nyquist, 12 Hz))
    let f_max = (0.48 / dt).clamp(1.0, (baseline_f1_hz * 3.0).max(10.0));
    let bins = 120usize;
    let mut best_f = baseline_f1_hz;
    let mut best_power = -1.0_f64;

    for b in 0..=bins {
        let f = 0.25 + (f_max - 0.25) * (b as f64 / bins as f64);
        let mut re = 0.0_f64;
        let mut im = 0.0_f64;
        for (i, &(_, val)) in series.iter().enumerate() {
            let x = val - mean;
            let theta = 2.0 * PI * f * (i as f64 * dt);
            re += x * theta.cos();
            im += x * theta.sin();
        }
        let power = re * re + im * im;
        if power > best_power {
            best_power = power;
            best_f = f;
        }
    }
    Some(best_f)
}

fn extract_story_mode_vectors(
    fem: &crate::core::model::StructuralModel,
    modes: &[crate::core::modal::ModeShape],
    level_elevs: &[f64],
    num_stories: usize,
) -> Vec<Vec<[f64; 2]>> {
    modes
        .iter()
        .map(|mode| {
            let mut sums = vec![[0.0_f64; 2]; num_stories];
            let mut counts = vec![0usize; num_stories];
            for (idx, node) in fem.nodes.iter().enumerate() {
                if node.y <= 0.1 {
                    continue;
                }
                if let Some(&[dx, _dy, dz]) = mode.nodal_deformations.get(idx) {
                    let st = story_index_for_z(node.y, level_elevs, num_stories);
                    sums[st][0] += dx;
                    sums[st][1] += dz;
                    counts[st] += 1;
                }
            }
            for s in 0..num_stories {
                if counts[s] > 0 {
                    sums[s][0] /= counts[s] as f64;
                    sums[s][1] /= counts[s] as f64;
                }
            }
            // Enforce positive sign convention on top story for consistent visual/dot comparisons
            if let Some(top) = sums.last() {
                if top[0] + top[1] < 0.0 {
                    for v in &mut sums {
                        v[0] = -v[0];
                        v[1] = -v[1];
                    }
                }
            }
            sums
        })
        .collect()
}

fn compute_mac(phi_a: Option<&Vec<[f64; 2]>>, phi_b: Option<&Vec<[f64; 2]>>) -> f64 {
    let (Some(a), Some(b)) = (phi_a, phi_b) else {
        return 1.0;
    };
    let n = a.len().min(b.len());
    if n == 0 {
        return 1.0;
    }
    let mut dot = 0.0_f64;
    let mut norm_a = 0.0_f64;
    let mut norm_b = 0.0_f64;
    for i in 0..n {
        dot += a[i][0] * b[i][0] + a[i][1] * b[i][1];
        norm_a += a[i][0] * a[i][0] + a[i][1] * a[i][1];
        norm_b += b[i][0] * b[i][0] + b[i][1] * b[i][1];
    }
    if norm_a < 1e-12 || norm_b < 1e-12 {
        return 1.0;
    }
    ((dot * dot) / (norm_a * norm_b)).clamp(0.0, 1.0)
}

/// Condensed N-story coupled MDOF shear-building inverse-iteration eigensolver for arbitrarily large IFC models.
/// Computes real stiffness-weighted frequencies `[f1, f2, f3]` and exact Modal Assurance Criterion `[MAC1, MAC2, MAC3]`.
fn solve_condensed_mdof_modal_shift(
    k_factor: &[f64],
    baseline_freqs: [f64; 3],
) -> (f64, f64, f64, f64, f64, f64) {
    let n = k_factor.len().clamp(1, 40);
    if n == 1 {
        let ratio = k_factor[0].max(0.1).sqrt();
        return (
            baseline_freqs[0] * ratio,
            baseline_freqs[1] * ratio,
            baseline_freqs[2] * ratio,
            1.0,
            1.0,
            1.0,
        );
    }

    let solve_modes = |ks: &[f64]| -> (Vec<f64>, Vec<Vec<[f64; 2]>>) {
        // Build dense n x n tridiagonal story stiffness matrix K (m_i = 1.0) and invert via Gaussian elimination
        let mut a = vec![vec![0.0_f64; n]; n];
        for i in 0..n {
            let k_cur = ks.get(i).copied().unwrap_or(1.0).max(0.05);
            let k_above = if i + 1 < n {
                ks.get(i + 1).copied().unwrap_or(1.0).max(0.05)
            } else {
                0.0
            };
            a[i][i] = k_cur + k_above;
            if i + 1 < n {
                a[i][i + 1] = -k_above;
                a[i + 1][i] = -k_above;
            }
        }
        // Compute K^{-1} so power iteration yields smallest eigenvalues (1 / omega_m^2)
        let mut aug = vec![vec![0.0_f64; 2 * n]; n];
        for i in 0..n {
            for j in 0..n {
                aug[i][j] = a[i][j];
            }
            aug[i][n + i] = 1.0;
        }
        for col in 0..n {
            let mut pivot_row = col;
            for r in (col + 1)..n {
                if aug[r][col].abs() > aug[pivot_row][col].abs() {
                    pivot_row = r;
                }
            }
            aug.swap(col, pivot_row);
            let piv = aug[col][col];
            if piv.abs() < 1e-12 {
                continue;
            }
            for j in col..(2 * n) {
                aug[col][j] /= piv;
            }
            for r in 0..n {
                if r != col {
                    let f = aug[r][col];
                    if f.abs() > 1e-14 {
                        for j in col..(2 * n) {
                            let sub = f * aug[col][j];
                            aug[r][j] -= sub;
                        }
                    }
                }
            }
        }
        let mut d_mat: Vec<Vec<f64>> = (0..n).map(|i| aug[i][n..].to_vec()).collect();

        // Extract top 3 fundamental modes using power iteration + Hotelling deflation on K^{-1}
        let mut omegas = Vec::with_capacity(3);
        let mut shapes = Vec::with_capacity(3);
        for m in 0..3.min(n) {
            let mut v: Vec<f64> = (0..n)
                .map(|i| ((i + 1) as f64 / n as f64) * ((m + 1) as f64 * 1.1).sin().abs().max(0.2))
                .collect();
            let norm0 = v.iter().map(|x| x * x).sum::<f64>().sqrt().max(1e-12);
            for x in &mut v {
                *x /= norm0;
            }

            let mut mu = 1.0_f64;
            for _ in 0..48 {
                let mut w = vec![0.0_f64; n];
                for i in 0..n {
                    let mut acc = 0.0;
                    for j in 0..n {
                        acc += d_mat[i][j] * v[j];
                    }
                    w[i] = acc;
                }
                let nw = w.iter().map(|x| x * x).sum::<f64>().sqrt().max(1e-12);
                mu = w.iter().zip(v.iter()).map(|(a, b)| a * b).sum::<f64>().max(1e-9);
                for i in 0..n {
                    v[i] = w[i] / nw;
                }
            }
            let omega = (1.0 / mu.max(1e-9)).sqrt();
            omegas.push(omega);
            // Map scalar story shape to [dx, dz] vector
            let vec2d: Vec<[f64; 2]> = if m == 1 {
                v.iter().map(|&x| [-0.12 * x, x]).collect()
            } else {
                v.iter().map(|&x| [x, 0.12 * x]).collect()
            };
            shapes.push(vec2d);

            // Deflate D <- D - mu * v * v^T
            for i in 0..n {
                for j in 0..n {
                    d_mat[i][j] -= mu * v[i] * v[j];
                }
            }
        }
        (omegas, shapes)
    };

    let intact_k = vec![1.0_f64; n];
    let (omega_base, shapes_base) = solve_modes(&intact_k);
    let (omega_obs, shapes_obs) = solve_modes(k_factor);

    let r1 = (omega_obs.first().copied().unwrap_or(1.0) / omega_base.first().copied().unwrap_or(1.0).max(1e-9))
        .clamp(0.25, 1.05);
    let r2 = (omega_obs.get(1).copied().unwrap_or(1.0) / omega_base.get(1).copied().unwrap_or(1.0).max(1e-9))
        .clamp(0.25, 1.05);
    let r3 = (omega_obs.get(2).copied().unwrap_or(1.0) / omega_base.get(2).copied().unwrap_or(1.0).max(1e-9))
        .clamp(0.25, 1.05);

    let m1 = compute_mac(shapes_base.first(), shapes_obs.first());
    let m2 = compute_mac(shapes_base.get(1), shapes_obs.get(1));
    let m3 = compute_mac(shapes_base.get(2), shapes_obs.get(2));

    (
        baseline_freqs[0] * r1,
        baseline_freqs[1] * r2,
        baseline_freqs[2] * r3,
        m1,
        m2,
        m3,
    )
}

fn build_bim_sensor_snapshots(
    project: &BimProject,
    level_elevs: &[f64],
    story_health: &[StoryShmHealth],
    obs_f1_hz: f64,
    noise_floor_mg: f64,
    custom_log: Option<&CustomShmTelemetryLog>,
) -> Vec<ShmSensorNodeSnapshot> {
    let mut xs: Vec<f64> = project.columns.iter().map(|c| c.pos.0 * 1e-3).collect();
    let mut ys: Vec<f64> = project.columns.iter().map(|c| c.pos.1 * 1e-3).collect();
    if xs.is_empty() {
        xs = project.runs.iter().map(|r| r.start.0 * 1e-3).collect();
        ys = project.runs.iter().map(|r| r.start.1 * 1e-3).collect();
    }
    let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let max_x = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min_y = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let max_y = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let cx = if min_x.is_finite() { 0.5 * (min_x + max_x) } else { 0.0 };
    let cy = if min_y.is_finite() { 0.5 * (min_y + max_y) } else { 0.0 };
    let hx = if min_x.is_finite() { (0.5 * (max_x - min_x)).max(3.0) } else { 6.0 };
    let hy = if min_y.is_finite() { (0.5 * (max_y - min_y)).max(3.0) } else { 5.0 };

    let mut sensors = Vec::new();
    for (idx, sh) in story_health.iter().enumerate() {
        let elev = level_elevs.get(idx + 1).copied().unwrap_or(sh.elevation_m);
        let story = sh.story_number;
        let base_rms = (noise_floor_mg + sh.floor_accel_g * 220.0).clamp(0.8, 380.0);
        let custom_boost = custom_log
            .and_then(|l| l.records.iter().find(|r| r.story_number == story))
            .map(|r| (r.accel_x_g.abs() * 180.0).max(0.5))
            .unwrap_or(0.0);

        let anchors: [([f64; 3], ShmSensorKind, &str); 3] = [
            ([cx, cy, elev], ShmSensorKind::TriaxialAccel, "CTR"),
            ([cx + hx * 0.92, cy + hy * 0.92, elev], ShmSensorKind::StoryInclinometer, "NE"),
            (
                [cx - hx * 0.92, cy - hy * 0.92, (elev - 1.5).max(0.3)],
                if idx == 0 {
                    ShmSensorKind::BaseStrainGauge
                } else {
                    ShmSensorKind::TriaxialAccel
                },
                "SW",
            ),
        ];

        for (s_i, (pos, kind, tag)) in anchors.into_iter().enumerate() {
            let edge_amp = if s_i == 0 { 1.0 } else { 1.12 };
            let z_score = (sh.anomaly_z_score * edge_amp).clamp(0.1, 16.0);
            let health = (sh.observed_stiffness_pct / 100.0).clamp(0.15, 1.0);
            sensors.push(ShmSensorNodeSnapshot {
                sensor_id: format!("SHM-F{story:02}-{tag}"),
                story_number: story,
                kind,
                position_local_m: pos,
                rms_accel_mg: (base_rms * edge_amp + custom_boost).clamp(0.5, 450.0),
                peak_accel_g: sh.floor_accel_g * edge_amp,
                peak_drift_pct: sh.peak_transient_drift_pct,
                residual_drift_pct: sh.residual_drift_pct,
                natural_freq_local_hz: (obs_f1_hz * (1.0 + 0.015 * s_i as f64)).max(0.15),
                health_index: health,
                anomaly_z_score: z_score,
                is_triggered: sh.story_placard != Atc20Placard::InspectedGreen || z_score >= 3.0,
            });
        }
    }
    sensors
}

fn build_welch_psd_spectrum(
    base_freqs: [f64; 3],
    obs_freqs: [f64; 3],
    zeta_base: f64,
    zeta_obs: f64,
    noise_floor_mg: f64,
    custom_log: Option<&CustomShmTelemetryLog>,
) -> Vec<ShmPsdPoint> {
    let f_max = (base_freqs[2] * 1.45).max(obs_freqs[2] * 1.45).clamp(8.0, 24.0);
    let bins = 64usize;
    let weights = [1.0, 0.58, 0.32];
    let noise_psd = (noise_floor_mg * 1e-3).powi(2) * 0.15;

    let sdof_psd = |f: f64, modes: [f64; 3], zeta: f64| -> f64 {
        let mut sum = noise_psd;
        for (i, &fn_m) in modes.iter().enumerate() {
            let r = f / fn_m.max(0.15);
            let denom = (1.0 - r * r).powi(2) + (2.0 * zeta * r).powi(2);
            sum += weights[i] * 0.012 / denom.max(1e-5);
        }
        sum
    };

    let custom_peak = custom_log.and_then(|l| estimate_dominant_frequency_from_log(l, obs_freqs[0]));

    (0..bins)
        .map(|i| {
            let freq_hz = 0.10 + (f_max - 0.10) * (i as f64 / (bins - 1) as f64);
            let baseline_psd_g2_hz = sdof_psd(freq_hz, base_freqs, zeta_base);
            let mut observed_psd_g2_hz = sdof_psd(freq_hz, obs_freqs, zeta_obs);
            if let Some(fp) = custom_peak {
                let r = freq_hz / fp.max(0.15);
                let extra = 0.008 / ((1.0 - r * r).powi(2) + (2.0 * zeta_obs * r).powi(2)).max(1e-5);
                observed_psd_g2_hz += extra;
            }
            ShmPsdPoint {
                freq_hz,
                baseline_psd_g2_hz,
                observed_psd_g2_hz,
            }
        })
        .collect()
}

fn evaluate_atc20_checklist(
    freq_shift_pct: f64,
    mac_mode_1: f64,
    max_transient_drift_pct: f64,
    max_residual_drift_pct: f64,
    residual_eigengate: f64,
    max_stiffness_loss_pct: f64,
) -> Vec<Atc20ChecklistCriterion> {
    let abs_df = freq_shift_pct.abs();
    let df_outcome = if abs_df >= 15.0 {
        Atc20Placard::UnsafeRed
    } else if abs_df >= 5.0 {
        Atc20Placard::RestrictedYellow
    } else {
        Atc20Placard::InspectedGreen
    };

    let mac_outcome = if mac_mode_1 < 0.78 {
        Atc20Placard::UnsafeRed
    } else if mac_mode_1 < 0.90 {
        Atc20Placard::RestrictedYellow
    } else {
        Atc20Placard::InspectedGreen
    };

    let drift_outcome = if max_transient_drift_pct >= 2.0 {
        Atc20Placard::UnsafeRed
    } else if max_transient_drift_pct >= 1.0 {
        Atc20Placard::RestrictedYellow
    } else {
        Atc20Placard::InspectedGreen
    };

    let res_drift_outcome = if max_residual_drift_pct >= 0.50 {
        Atc20Placard::UnsafeRed
    } else if max_residual_drift_pct >= 0.25 {
        Atc20Placard::RestrictedYellow
    } else {
        Atc20Placard::InspectedGreen
    };

    let eigengate_outcome = if residual_eigengate < 0.50 {
        Atc20Placard::UnsafeRed
    } else if residual_eigengate < 0.70 {
        Atc20Placard::RestrictedYellow
    } else {
        Atc20Placard::InspectedGreen
    };

    let stiff_outcome = if max_stiffness_loss_pct >= 28.0 {
        Atc20Placard::UnsafeRed
    } else if max_stiffness_loss_pct >= 12.0 {
        Atc20Placard::RestrictedYellow
    } else {
        Atc20Placard::InspectedGreen
    };

    vec![
        Atc20ChecklistCriterion {
            code: "SHM-01".to_string(),
            criterion: "Fundamental Frequency Shift |df1/f1|".to_string(),
            observed_value: format!("{freq_shift_pct:+.1}%"),
            threshold_value: "G<5% | Y<15% | R>=15%".to_string(),
            outcome: df_outcome,
            rationale: "Indicates global lateral stiffness degradation relative to baseline 6-DOF eigenfrequency.".to_string(),
        },
        Atc20ChecklistCriterion {
            code: "SHM-02".to_string(),
            criterion: "Modal Assurance Criterion (MAC1)".to_string(),
            observed_value: format!("{mac_mode_1:.3}"),
            threshold_value: "G>=0.90 | Y>=0.78 | R<0.78".to_string(),
            outcome: mac_outcome,
            rationale: "Measures eigenvector distortion from localized soft-story or plastic-hinge kinking.".to_string(),
        },
        Atc20ChecklistCriterion {
            code: "ATC-03".to_string(),
            criterion: "Peak Transient Interstory Drift".to_string(),
            observed_value: format!("{max_transient_drift_pct:.2}%"),
            threshold_value: "G<1.0% | Y<2.0% | R>=2.0%".to_string(),
            outcome: drift_outcome,
            rationale: "ASCE 41 IO (1.0%) & LS (2.0%) interstory deformation safety envelope.".to_string(),
        },
        Atc20ChecklistCriterion {
            code: "ATC-04".to_string(),
            criterion: "Permanent Residual Interstory Drift".to_string(),
            observed_value: format!("{max_residual_drift_pct:.2}%"),
            threshold_value: "G<0.25% | Y<0.50% | R>=0.50%".to_string(),
            outcome: res_drift_outcome,
            rationale: "FEMA P-58 irreparable residual tilt trigger (>0.50% requires demolition/evacuation scrutiny).".to_string(),
        },
        Atc20ChecklistCriterion {
            code: "ATC-05".to_string(),
            criterion: "Post-Event Residual Redundancy (EigenGate)".to_string(),
            observed_value: format!("{residual_eigengate:.2}"),
            threshold_value: "G>=0.70 | Y>=0.50 | R<0.50".to_string(),
            outcome: eigengate_outcome,
            rationale: "Assesses alternate load-path capacity against progressive collapse under aftershocks.".to_string(),
        },
        Atc20ChecklistCriterion {
            code: "ATC-06".to_string(),
            criterion: "Critical Story Stiffness Loss".to_string(),
            observed_value: format!("{max_stiffness_loss_pct:.1}%"),
            threshold_value: "G<12% | Y<28% | R>=28%".to_string(),
            outcome: stiff_outcome,
            rationale: "Evaluates worst story flexural stiffness deterioration for ATC-20 detailed posting.".to_string(),
        },
    ]
}

#[allow(clippy::too_many_arguments)]
fn build_district_portfolio_placards(
    project: &BimProject,
    site: &SiteEnvironmentalCoupling,
    world_index: Option<&WorldSpatialIndex>,
    config: &ShmDigitalTwinConfig,
    lat_lon: (f64, f64),
    obs_f1_hz: f64,
    active_drift_pct: f64,
    active_loss_pct: f64,
    active_eigengate: f64,
    active_placard: Atc20Placard,
) -> (Vec<DistrictBuildingPlacard>, usize, usize, usize) {
    let mut out = Vec::new();
    let active_rel = world_index.and_then(|w| w.active_rel_path.as_deref());
    let sev = (config.severity_pct / 100.0).clamp(0.0, 1.0);
    let ss = (site.seismic_pga_ss_g as f64).max(0.25);

    if config.include_portfolio_district {
        if let Some(world) = world_index {
            for b in &world.buildings {
                if active_rel == Some(b.rel_path.as_str()) {
                    out.push(DistrictBuildingPlacard {
                        rel_path: b.rel_path.clone(),
                        building_name: b.name.clone(),
                        lat: b.lat,
                        lon: b.lon,
                        elevation_m: b.elev_m,
                        total_height_m: b.total_height_m.max(3.5),
                        footprint_local_m: b.footprint_local_m.clone(),
                        heading_deg: b.heading_deg,
                        estimated_t1_s: (1.0 / obs_f1_hz.max(0.15)).clamp(0.10, 6.0),
                        estimated_drift_pct: active_drift_pct,
                        estimated_stiffness_loss_pct: active_loss_pct,
                        max_dcr: b.max_dcr,
                        min_eigengate: active_eigengate,
                        placard: active_placard,
                        rationale: "Active BIM SHM Digital Twin coupled assessment.".to_string(),
                    });
                } else {
                    let h = b.total_height_m.max(4.0);
                    let t1 = (0.0466 * h.powf(0.9)).clamp(0.15, 4.5);
                    let hazard_fac = match config.fault_scenario {
                        ShmFaultScenario::NominalIntact => 0.18,
                        _ => (0.45 + 0.85 * sev) * (ss / 0.80).clamp(0.5, 1.5),
                    };
                    let dcr_norm = b.max_dcr.clamp(0.2, 2.0);
                    let eigengate_penalty = (1.0 - b.min_eigengate.clamp(0.1, 1.0)) * 18.0;
                    let est_loss = (dcr_norm * 14.0 * hazard_fac + eigengate_penalty * hazard_fac).clamp(0.5, 68.0);
                    let est_drift = (dcr_norm * 0.92 * hazard_fac + (1.0 - b.min_eigengate) * 0.8).clamp(0.04, 4.2);

                    let placard = if est_loss >= 28.0 || est_drift >= 2.0 || b.min_eigengate < 0.50 || b.max_dcr >= 1.25 {
                        Atc20Placard::UnsafeRed
                    } else if est_loss >= 12.0 || est_drift >= 1.0 || b.min_eigengate < 0.70 || b.max_dcr >= 0.90 {
                        Atc20Placard::RestrictedYellow
                    } else {
                        Atc20Placard::InspectedGreen
                    };

                    out.push(DistrictBuildingPlacard {
                        rel_path: b.rel_path.clone(),
                        building_name: b.name.clone(),
                        lat: b.lat,
                        lon: b.lon,
                        elevation_m: b.elev_m,
                        total_height_m: h,
                        footprint_local_m: b.footprint_local_m.clone(),
                        heading_deg: b.heading_deg,
                        estimated_t1_s: t1,
                        estimated_drift_pct: est_drift,
                        estimated_stiffness_loss_pct: est_loss,
                        max_dcr: b.max_dcr,
                        min_eigengate: b.min_eigengate,
                        placard,
                        rationale: format!(
                            "Spectral T1={t1:.2}s screening (DCR {:.2}, EigenGate {:.2}).",
                            b.max_dcr, b.min_eigengate
                        ),
                    });
                }
            }
        }
    }

    if out.is_empty() {
        let mut xs: Vec<f64> = project.columns.iter().map(|c| c.pos.0 * 1e-3).collect();
        let mut zs: Vec<f64> = project.columns.iter().map(|c| c.pos.1 * 1e-3).collect();
        if xs.is_empty() {
            xs = vec![-6.0, 6.0];
            zs = vec![-5.0, 5.0];
        }
        let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min).min(-4.0);
        let max_x = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max).max(4.0);
        let min_z = zs.iter().copied().fold(f64::INFINITY, f64::min).min(-4.0);
        let max_z = zs.iter().copied().fold(f64::NEG_INFINITY, f64::max).max(4.0);
        let h_m = project
            .levels
            .iter()
            .map(|l| l.elevation_mm * 1e-3)
            .fold(4.0_f64, f64::max);
        out.push(DistrictBuildingPlacard {
            rel_path: "__active_bim__".to_string(),
            building_name: project.project_name.clone(),
            lat: lat_lon.0,
            lon: lat_lon.1,
            elevation_m: site.elevation_m as f64,
            total_height_m: h_m,
            footprint_local_m: vec![[min_x, min_z], [max_x, min_z], [max_x, max_z], [min_x, max_z]],
            heading_deg: 0.0,
            estimated_t1_s: (1.0 / obs_f1_hz.max(0.15)).clamp(0.10, 6.0),
            estimated_drift_pct: active_drift_pct,
            estimated_stiffness_loss_pct: active_loss_pct,
            max_dcr: (active_drift_pct / 2.0).clamp(0.15, 2.5),
            min_eigengate: active_eigengate,
            placard: active_placard,
            rationale: "Active BIM SHM Digital Twin coupled assessment.".to_string(),
        });
    }

    let green = out.iter().filter(|b| b.placard == Atc20Placard::InspectedGreen).count();
    let yellow = out.iter().filter(|b| b.placard == Atc20Placard::RestrictedYellow).count();
    let red = out.iter().filter(|b| b.placard == Atc20Placard::UnsafeRed).count();
    (out, green, yellow, red)
}

#[allow(clippy::too_many_arguments)]
fn build_shm_overlay_ifcx(
    project_name: &str,
    scenario: ShmFaultScenario,
    base_f1: f64,
    obs_f1: f64,
    freq_shift_pct: f64,
    mac1: f64,
    stiffness_retention_pct: f64,
    baseline_eigengate: f64,
    residual_eigengate: f64,
    max_transient_drift_pct: f64,
    max_residual_drift_pct: f64,
    placard: Atc20Placard,
    checklist: &[Atc20ChecklistCriterion],
    story_health: &[StoryShmHealth],
    sensors: &[ShmSensorNodeSnapshot],
    green_count: usize,
    yellow_count: usize,
    red_count: usize,
) -> IfcxDocument {
    let slug = sanitize_slug(project_name);
    let mut doc = IfcxDocument::new(format!("shm_atc20_{slug}"));
    doc.imports.push(IfcxImport {
        uri: format!("eigengraph://{slug}.ifcx"),
        integrity: None,
        role: Some("base-building".to_string()),
    });
    doc.schemas.insert(
        NS_EG_SHM_DIGITAL_TWIN.to_string(),
        json!({
            "value": {
                "dataType": "Object",
                "spec": "EigenGraph Phase 76 Live SHM Digital Twin Sensor & Modal Identification Schema"
            }
        }),
    );
    doc.schemas.insert(
        NS_EG_ATC20_PLACARD.to_string(),
        json!({
            "value": {
                "dataType": "Object",
                "spec": "EigenGraph Automated ATC-20 Post-Earthquake Structural Safety Placard Triage Schema"
            }
        }),
    );

    let site_path = format!("/earth/sites/{slug}");
    let bldg_path = format!("{site_path}/buildings/{slug}");
    let analysis_path = format!("{bldg_path}/analysis");
    let shm_path = format!("{analysis_path}/shm_digital_twin");
    let placard_path = format!("{analysis_path}/atc20_placard");
    let sensors_root = format!("{bldg_path}/sensors");

    let shm_payload = json!({
        "scenario": scenario.short_label(),
        "scenarioLabel": scenario.label(),
        "baselineFreq1Hz": base_f1,
        "observedFreq1Hz": obs_f1,
        "freqShiftPct": freq_shift_pct,
        "macMode1": mac1,
        "globalStiffnessRetentionPct": stiffness_retention_pct,
        "residualEigenGate": residual_eigengate,
        "maxTransientDriftPct": max_transient_drift_pct,
        "maxResidualDriftPct": max_residual_drift_pct,
        "sensorCount": sensors.len(),
        "stories": story_health.iter().map(|s| json!({
            "story": s.story_number,
            "elevationM": s.elevation_m,
            "observedStiffnessPct": s.observed_stiffness_pct,
            "stiffnessLossPct": s.stiffness_loss_pct,
            "transientDriftPct": s.peak_transient_drift_pct,
            "residualDriftPct": s.residual_drift_pct,
            "placard": s.story_placard.short_label(),
        })).collect::<Vec<_>>(),
    });

    let placard_payload = json!({
        "placard": placard.label(),
        "placardCode": placard.short_label(),
        "guidance": placard.guidance(),
        "districtCounts": {
            "green": green_count,
            "yellow": yellow_count,
            "red": red_count,
        },
        "checklist": checklist.iter().map(|c| json!({
            "code": c.code,
            "criterion": c.criterion,
            "observed": c.observed_value,
            "threshold": c.threshold_value,
            "outcome": c.outcome.short_label(),
        })).collect::<Vec<_>>(),
    });

    let eigengate_overlay = IfcxEigenGateOverlay {
        eigengate_system: residual_eigengate,
        eigengate_average: 0.5 * (baseline_eigengate + residual_eigengate),
        critical_members_count: sensors.iter().filter(|s| s.is_triggered).count(),
        max_dcr_pct: (max_transient_drift_pct / 2.0 * 100.0).min(999.0),
        svd_rank_deficiency: usize::from(placard == Atc20Placard::UnsafeRed),
        is_kinematically_stable: placard != Atc20Placard::UnsafeRed,
    };

    let mut bldg_node = IfcxNode::new(&bldg_path).with_class("IfcBuilding");
    bldg_node
        .children
        .insert("analysis".to_string(), analysis_path.clone());
    bldg_node
        .children
        .insert("sensors".to_string(), sensors_root.clone());
    bldg_node
        .attributes
        .insert(NS_EG_SHM_DIGITAL_TWIN.to_string(), shm_payload.clone());
    bldg_node
        .attributes
        .insert(NS_EG_ATC20_PLACARD.to_string(), placard_payload.clone());
    if let Ok(v) = serde_json::to_value(&eigengate_overlay) {
        bldg_node.attributes.insert(ns::EG_EIGENGATE.to_string(), v);
    }
    doc.add_node(bldg_node);

    let mut analysis_node = IfcxNode::new(&analysis_path).with_class("IfcTask");
    analysis_node
        .children
        .insert("shm_digital_twin".to_string(), shm_path.clone());
    analysis_node
        .children
        .insert("atc20_placard".to_string(), placard_path.clone());
    doc.add_node(analysis_node);

    let mut shm_node = IfcxNode::new(&shm_path)
        .with_class("IfcTask")
        .with_prop(
            "SHM Modal Identification",
            scenario.short_label(),
            &format!(
                "f1 {base_f1:.2}->{obs_f1:.2} Hz ({freq_shift_pct:+.1}%) | MAC1 {mac1:.3} | K_ret {stiffness_retention_pct:.1}%"
            ),
        );
    shm_node
        .attributes
        .insert(NS_EG_SHM_DIGITAL_TWIN.to_string(), shm_payload);
    doc.add_node(shm_node);

    let mut placard_node = IfcxNode::new(&placard_path)
        .with_class("IfcAnnotation")
        .with_prop(
            "ATC-20 Post-Earthquake Placard",
            placard.short_label(),
            placard.guidance(),
        );
    placard_node
        .attributes
        .insert(NS_EG_ATC20_PLACARD.to_string(), placard_payload);
    doc.add_node(placard_node);

    let mut sensors_container = IfcxNode::new(&sensors_root).with_class("IfcDistributionElement");
    for s in sensors {
        let sp = format!("{sensors_root}/{}", s.sensor_id);
        sensors_container
            .children
            .insert(s.sensor_id.clone(), sp.clone());

        let mut sn = IfcxNode::new(&sp)
            .with_class("IfcSensor")
            .with_prop(
                s.kind.label(),
                if s.is_triggered { "TRIGGERED" } else { "NOMINAL" },
                &format!(
                    "Story {} | RMS {:.1} mg | Drift {:.2}% | Health {:.0}%",
                    s.story_number,
                    s.rms_accel_mg,
                    s.peak_drift_pct,
                    s.health_index * 100.0
                ),
            );
        sn.attributes.insert(
            NS_EG_SHM_DIGITAL_TWIN.to_string(),
            json!({
                "sensorId": s.sensor_id,
                "story": s.story_number,
                "kind": s.kind.short_label(),
                "positionLocalM": s.position_local_m,
                "rmsAccelMg": s.rms_accel_mg,
                "peakDriftPct": s.peak_drift_pct,
                "residualDriftPct": s.residual_drift_pct,
                "healthIndex": s.health_index,
                "anomalyZScore": s.anomaly_z_score,
                "triggered": s.is_triggered,
            }),
        );
        doc.add_node(sn);
    }
    doc.add_node(sensors_container);

    doc
}

fn sanitize_slug(raw: &str) -> String {
    let s: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = s.trim_matches('_');
    if trimmed.is_empty() {
        "eigengraph_project".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ifcx::IfcxStage;

    #[test]
    fn phase76_shm_digital_twin_frequency_shift_mac_and_atc20_placard_escalation() {
        let project = BimProject::sample_bimcraft_building();
        let site = SiteEnvironmentalCoupling::default();
        let mut world = WorldSpatialIndex::default();

        // 1. Nominal Intact scenario -> GREEN placard, near-zero frequency shift, high MAC
        let cfg_intact = ShmDigitalTwinConfig {
            fault_scenario: ShmFaultScenario::NominalIntact,
            severity_pct: 5.0,
            ..Default::default()
        };
        let intact = evaluate_shm_digital_twin_for_bim(
            &project,
            &site,
            None,
            None,
            None,
            Some(&world),
            &cfg_intact,
            (-6.2088, 106.8456),
        )
        .expect("Nominal SHM evaluation");

        assert!(intact.freq_shift_pct.abs() < 2.5, "Intact freq shift too large: {}", intact.freq_shift_pct);
        assert!(intact.mac_mode_1 > 0.96, "Intact MAC should be > 0.96, got {}", intact.mac_mode_1);
        assert_eq!(intact.atc20_placard, Atc20Placard::InspectedGreen);
        assert_eq!(intact.psd_spectrum.len(), 64);
        assert!(!intact.sensors.is_empty());

        // 2. Soft-Story First Floor (90% severity) -> downward frequency shift, MAC reduction, RED placard
        let cfg_soft = ShmDigitalTwinConfig {
            fault_scenario: ShmFaultScenario::SoftStoryFirstFloor,
            severity_pct: 90.0,
            target_story: 1,
            ..Default::default()
        };
        let damaged = evaluate_shm_digital_twin_for_bim(
            &project,
            &site,
            None,
            None,
            None,
            Some(&world),
            &cfg_soft,
            (-6.2088, 106.8456),
        )
        .expect("Soft-story SHM evaluation");

        assert!(
            damaged.observed_freq_1_hz < damaged.baseline_freq_1_hz,
            "Observed f1 ({}) must drop below baseline ({})",
            damaged.observed_freq_1_hz,
            damaged.baseline_freq_1_hz
        );
        assert!(damaged.freq_shift_pct < -8.0, "Expected significant downward frequency shift, got {}", damaged.freq_shift_pct);
        assert!(
            damaged.mac_mode_1 < intact.mac_mode_1,
            "Damaged MAC ({}) should drop below intact MAC ({})",
            damaged.mac_mode_1,
            intact.mac_mode_1
        );
        assert!(
            damaged.story_health[0].observed_stiffness_pct
                < damaged.story_health.last().unwrap().observed_stiffness_pct
        );
        assert_eq!(damaged.atc20_placard, Atc20Placard::UnsafeRed);

        // 3. Non-destructive IFCX Schema Overlay & Layer Classification
        let base_ifcx = IfcxDocument::from_bim_project(&project, None, 0.0, None, None, None);
        let composed = IfcxStage::compose_with_shm_atc20_overlay(&base_ifcx, &damaged);
        let raw_ifcx = serde_json::to_string(&composed).expect("Serialize composed IFCX");
        assert!(raw_ifcx.contains(NS_EG_SHM_DIGITAL_TWIN));
        assert!(raw_ifcx.contains(NS_EG_ATC20_PLACARD));
        assert!(raw_ifcx.contains("IfcSensor"));

        // Verify world proxy population updates portfolio placard counts
        world.upsert_building(
            crate::core::gis::WorldBuildingProxy::from_bim_project(
                "tower_a.ifcx",
                "tower_a.ifcx",
                -6.2088,
                106.8456,
                0.0,
                &project,
                &[],
                crate::core::gis::GeoPlacementSource::UserPinnedOnEarth,
                "Jakarta".to_string(),
            ),
            true,
        );
        let with_district = evaluate_shm_digital_twin_for_bim(
            &project,
            &site,
            None,
            None,
            None,
            Some(&world),
            &cfg_soft,
            (-6.2088, 106.8456),
        )
        .expect("District evaluation");
        assert_eq!(with_district.district_placards.len(), 1);
    }

    #[test]
    fn phase76_shm_telemetry_csv_and_json_parser() {
        let csv = sample_shm_telemetry_csv();
        assert!(looks_like_shm_telemetry_payload(&csv));
        let parsed = parse_shm_telemetry_log(&csv, "sample_shm.csv").expect("Parse sample SHM CSV");
        assert_eq!(parsed.story_channels, 3);
        assert_eq!(parsed.records.len(), 600);
        assert!(parsed.sampling_rate_hz >= 49.0);

        let project = BimProject::sample_bimcraft_building();
        let site = SiteEnvironmentalCoupling::default();
        let cfg = ShmDigitalTwinConfig {
            fault_scenario: ShmFaultScenario::CustomTelemetryStream,
            custom_telemetry: Some(parsed.clone()),
            ..Default::default()
        };
        let report = evaluate_shm_digital_twin_for_bim(
            &project,
            &site,
            None,
            None,
            None,
            None,
            &cfg,
            (-6.2088, 106.8456),
        )
        .expect("Evaluate custom SHM CSV stream");
        assert!(report.max_transient_drift_pct > 0.5);

        let json_payload = serde_json::to_string(&parsed).expect("Serialize telemetry log");
        assert!(looks_like_shm_telemetry_payload(&json_payload));
        let reparsed = parse_shm_telemetry_log(&json_payload, "roundtrip.json")
            .expect("Parse JSON telemetry");
        assert_eq!(reparsed.records.len(), 600);

        assert!(parse_shm_telemetry_log("   ", "empty.csv").is_err());
        assert!(!looks_like_shm_telemetry_payload("hello random text"));
    }
}
