//! Phase 78: Live WebSocket SHM Telemetry Streaming, Rolling-Window Anomaly Watchdog &
//! Closed-Loop ATC-20 → Evacuation Lifeline Cascade (`eigengraph::analysis::shm_watchdog`).
//!
//! Consumes real-time triaxial packets from `web_bridge::shm_watchdog_poll_packets()` (or a
//! built-in deterministic stream simulator for offline verification), maintains a sliding
//! ring buffer with STA/LTA seismic onset detection, rolling fundamental frequency shift
//! tracking, and hysteresis-guarded anomaly Z-scores, and automatically escalates Phase 76
//! ATC-20 placards and Phase 77 A* evacuation routing when structural distress exceeds limits.

use std::f64::consts::PI;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::core::bim::BimProject;
use crate::core::ifcx::{ns, IfcxDocument, IfcxImport, IfcxNode};
use crate::core::shm_twin::{Atc20Placard, CustomShmTelemetryLog, ShmTelemetryRecord};
use crate::core::web_bridge::{self, ShmTelemetryPacket};

/// Canonical IFCX attribute namespace for Phase 78 Live SHM Watchdog telemetry & alert ledgers.
pub const NS_EG_SHM_WATCHDOG: &str = ns::EG_SHM_WATCHDOG;

fn default_ws_url() -> String {
    "wss://shm.eigengraph.net/ws/telemetry".to_string()
}
fn default_ring_capacity() -> usize {
    256
}
fn default_sta_window() -> usize {
    12
}
fn default_lta_window() -> usize {
    64
}
fn default_sta_lta_trigger() -> f64 {
    2.6
}
fn default_freq_warn_pct() -> f64 {
    8.0
}
fn default_freq_crit_pct() -> f64 {
    18.0
}
fn default_hysteresis() -> usize {
    2
}
fn default_true() -> bool {
    true
}
fn default_rate_hz() -> f64 {
    20.0
}

/// Built-in deterministic live SHM stream profile when no external WebSocket server is attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WatchdogSimMode {
    #[default]
    Off,
    AmbientNominal,
    AftershockPulse,
    ProgressiveSoftStory,
}

impl WatchdogSimMode {
    pub fn all() -> &'static [Self] {
        &[
            Self::Off,
            Self::AmbientNominal,
            Self::AftershockPulse,
            Self::ProgressiveSoftStory,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::AmbientNominal => "Ambient Nominal Stream",
            Self::AftershockPulse => "Live Aftershock Pulse",
            Self::ProgressiveSoftStory => "Progressive Soft-Story Drift",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Off => "Stop",
            Self::AmbientNominal => "Ambient",
            Self::AftershockPulse => "Aftershock",
            Self::ProgressiveSoftStory => "Soft-Story",
        }
    }

    pub fn from_keyword(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "off" | "stop" | "none" => Some(Self::Off),
            "ambient" | "nominal" | "ok" => Some(Self::AmbientNominal),
            "aftershock" | "pulse" | "quake" | "shock" => Some(Self::AftershockPulse),
            "soft_story" | "soft-story" | "softstory" | "soft" | "damage" => {
                Some(Self::ProgressiveSoftStory)
            }
            _ => None,
        }
    }
}

/// Real-time Watchdog escalation severity derived from STA/LTA, rolling `f1` shift, and Z-score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub enum WatchdogAlertLevel {
    #[default]
    Standby,
    Monitoring,
    Advisory,
    Warning,
    CriticalTrigger,
}

impl WatchdogAlertLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Standby => "STANDBY",
            Self::Monitoring => "MONITORING",
            Self::Advisory => "ADVISORY",
            Self::Warning => "WARNING",
            Self::CriticalTrigger => "CRITICAL TRIGGER",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Standby => "STBY",
            Self::Monitoring => "OK",
            Self::Advisory => "ADV",
            Self::Warning => "WARN",
            Self::CriticalTrigger => "CRIT",
        }
    }

    pub fn badge_color_rgb(self) -> [u8; 3] {
        match self {
            Self::Standby => [120, 144, 168],
            Self::Monitoring => [34, 197, 94],
            Self::Advisory => [56, 189, 248],
            Self::Warning => [245, 158, 11],
            Self::CriticalTrigger => [239, 68, 68],
        }
    }
}

/// Persisted configuration for Phase 78 Live WebSocket SHM Watchdog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShmWatchdogConfig {
    #[serde(default = "default_ws_url")]
    pub ws_url: String,
    #[serde(default)]
    pub sim_mode: WatchdogSimMode,
    #[serde(default = "default_ring_capacity")]
    pub ring_capacity: usize,
    #[serde(default = "default_sta_window")]
    pub sta_window: usize,
    #[serde(default = "default_lta_window")]
    pub lta_window: usize,
    #[serde(default = "default_sta_lta_trigger")]
    pub sta_lta_trigger_ratio: f64,
    #[serde(default = "default_freq_warn_pct")]
    pub freq_drop_warning_pct: f64,
    #[serde(default = "default_freq_crit_pct")]
    pub freq_drop_critical_pct: f64,
    #[serde(default = "default_hysteresis")]
    pub hysteresis_windows: usize,
    #[serde(default = "default_true")]
    pub auto_update_atc20: bool,
    #[serde(default = "default_true")]
    pub auto_reroute_evacuation: bool,
    #[serde(default = "default_rate_hz")]
    pub sampling_rate_hz: f64,
}

impl Default for ShmWatchdogConfig {
    fn default() -> Self {
        Self {
            ws_url: default_ws_url(),
            sim_mode: WatchdogSimMode::Off,
            ring_capacity: default_ring_capacity(),
            sta_window: default_sta_window(),
            lta_window: default_lta_window(),
            sta_lta_trigger_ratio: default_sta_lta_trigger(),
            freq_drop_warning_pct: default_freq_warn_pct(),
            freq_drop_critical_pct: default_freq_crit_pct(),
            hysteresis_windows: default_hysteresis(),
            auto_update_atc20: true,
            auto_reroute_evacuation: true,
            sampling_rate_hz: default_rate_hz(),
        }
    }
}

/// Single sliding-window sample recorded by the SHM Watchdog ring buffer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchdogSamplePoint {
    pub seq: u64,
    pub time_s: f64,
    pub story_number: usize,
    pub sensor_id: String,
    pub accel_x_g: f64,
    pub accel_y_g: f64,
    pub accel_z_g: f64,
    pub drift_ratio: f64,
    pub strain_micro: f64,
    pub observed_freq_hz: f64,
    pub sta_lta_ratio: f64,
}

/// Chronological alert or closed-loop cascade entry emitted by the Watchdog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchdogEventEntry {
    pub seq: u64,
    pub timestamp_label: String,
    pub level: WatchdogAlertLevel,
    pub summary: String,
    pub triggered_atc20: Option<Atc20Placard>,
    pub triggered_evac_reroute: bool,
}

/// Outcome returned per UI/simulation tick driving closed-loop ATC-20 & Evacuation updates.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ShmWatchdogTickOutcome {
    pub new_packets: usize,
    pub escalated: bool,
    pub should_refresh_shm: bool,
    pub should_reroute_evacuation: bool,
    pub alert_banner: Option<String>,
}

/// Live runtime state of the Phase 78 Rolling-Window SHM Watchdog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShmWatchdogState {
    pub config: ShmWatchdogConfig,
    pub ws_connected: bool,
    pub ws_status: String,
    pub ring: Vec<WatchdogSamplePoint>,
    pub freq_history_hz: Vec<(f64, f64)>,
    pub sta_lta_history: Vec<(f64, f64)>,
    pub events: Vec<WatchdogEventEntry>,
    pub total_packets_ingested: u64,
    pub sim_elapsed_s: f64,
    pub sim_accum_s: f64,
    pub baseline_freq_1_hz: f64,
    pub current_freq_1_hz: f64,
    pub current_freq_shift_pct: f64,
    pub current_sta_lta: f64,
    pub peak_sta_lta: f64,
    pub peak_accel_g: f64,
    pub peak_drift_pct: f64,
    pub anomaly_z_score: f64,
    pub alert_level: WatchdogAlertLevel,
    pub consecutive_exceedances: usize,
    pub last_escalated_level: WatchdogAlertLevel,
    pub auto_reroute_count: usize,
    #[serde(skip)]
    pub last_watchdog_overlay_ifcx: Option<IfcxDocument>,
}

impl Default for ShmWatchdogState {
    fn default() -> Self {
        Self {
            config: ShmWatchdogConfig::default(),
            ws_connected: false,
            ws_status: "Idle (Connect ws:// or start Live Sim)".to_string(),
            ring: Vec::new(),
            freq_history_hz: Vec::new(),
            sta_lta_history: Vec::new(),
            events: Vec::new(),
            total_packets_ingested: 0,
            sim_elapsed_s: 0.0,
            sim_accum_s: 0.0,
            baseline_freq_1_hz: 2.25,
            current_freq_1_hz: 2.25,
            current_freq_shift_pct: 0.0,
            current_sta_lta: 1.0,
            peak_sta_lta: 1.0,
            peak_accel_g: 0.0,
            peak_drift_pct: 0.0,
            anomaly_z_score: 0.0,
            alert_level: WatchdogAlertLevel::Standby,
            consecutive_exceedances: 0,
            last_escalated_level: WatchdogAlertLevel::Standby,
            auto_reroute_count: 0,
            last_watchdog_overlay_ifcx: None,
        }
    }
}

impl ShmWatchdogState {
    pub fn is_streaming(&self) -> bool {
        self.ws_connected || self.config.sim_mode != WatchdogSimMode::Off
    }

    pub fn reset_stream_buffers(&mut self) {
        self.ring.clear();
        self.freq_history_hz.clear();
        self.sta_lta_history.clear();
        self.total_packets_ingested = 0;
        self.sim_elapsed_s = 0.0;
        self.sim_accum_s = 0.0;
        self.current_freq_1_hz = self.baseline_freq_1_hz;
        self.current_freq_shift_pct = 0.0;
        self.current_sta_lta = 1.0;
        self.peak_sta_lta = 1.0;
        self.peak_accel_g = 0.0;
        self.peak_drift_pct = 0.0;
        self.anomaly_z_score = 0.0;
        self.alert_level = if self.is_streaming() {
            WatchdogAlertLevel::Monitoring
        } else {
            WatchdogAlertLevel::Standby
        };
        self.consecutive_exceedances = 0;
        self.last_escalated_level = WatchdogAlertLevel::Standby;
    }

    pub fn start_simulator(&mut self, mode: WatchdogSimMode) {
        self.config.sim_mode = mode;
        if mode == WatchdogSimMode::Off {
            if !self.ws_connected {
                self.alert_level = WatchdogAlertLevel::Standby;
            }
            self.ws_status = "Live stream simulator stopped.".to_string();
        } else {
            self.reset_stream_buffers();
            self.alert_level = WatchdogAlertLevel::Monitoring;
            self.ws_status = format!("Live Simulator: {}", mode.label());
            self.push_event(
                WatchdogAlertLevel::Monitoring,
                format!("Started deterministic SHM stream simulator ({})", mode.label()),
                None,
                false,
            );
        }
    }

    pub fn push_event(
        &mut self,
        level: WatchdogAlertLevel,
        summary: String,
        triggered_atc20: Option<Atc20Placard>,
        triggered_evac_reroute: bool,
    ) {
        let seq = self.events.len() as u64 + 1;
        self.events.push(WatchdogEventEntry {
            seq,
            timestamp_label: web_bridge::unix_timestamp_label(),
            level,
            summary,
            triggered_atc20,
            triggered_evac_reroute,
        });
        if self.events.len() > 24 {
            self.events.remove(0);
        }
    }

    /// Ingests a single live SHM packet into the sliding ring buffer and updates rolling STA/LTA,
    /// dominant frequency shift, Z-score, and hysteresis-guarded alert state.
    pub fn ingest_packet(&mut self, pkt: &ShmTelemetryPacket, num_stories: usize) {
        let cap = self.config.ring_capacity.clamp(32, 1024);
        let dt = (1.0 / self.config.sampling_rate_hz.clamp(2.0, 200.0)).max(0.005);
        let t_s = self
            .ring
            .last()
            .map(|p| p.time_s + dt)
            .unwrap_or(0.0);

        let ax = f64::from(pkt.accel_x_g.unwrap_or(pkt.accel_g));
        let ay = f64::from(pkt.accel_y_g.unwrap_or(pkt.accel_g * 0.55));
        let az = f64::from(pkt.accel_z_g.unwrap_or(pkt.accel_g * 0.28));
        let mag_g = ax.hypot(ay);

        let obs_f = if pkt.freq_hz > 0.15 {
            f64::from(pkt.freq_hz)
        } else {
            self.current_freq_1_hz
        };
        let drift_ratio = f64::from(
            pkt.drift_ratio
                .unwrap_or_else(|| (mag_g * 0.032) as f32),
        )
        .abs();

        self.total_packets_ingested += 1;
        let seq = self.total_packets_ingested;
        let story = pkt.story_number.clamp(1, num_stories.max(1));

        self.ring.push(WatchdogSamplePoint {
            seq,
            time_s: t_s,
            story_number: story,
            sensor_id: if pkt.sensor_id.trim().is_empty() {
                format!("MEMS-L{story}")
            } else {
                pkt.sensor_id.clone()
            },
            accel_x_g: ax,
            accel_y_g: ay,
            accel_z_g: az,
            drift_ratio,
            strain_micro: f64::from(pkt.strain_micro),
            observed_freq_hz: obs_f,
            sta_lta_ratio: 1.0,
        });
        while self.ring.len() > cap {
            self.ring.remove(0);
        }

        // Compute STA / LTA over sliding ring buffer
        let n = self.ring.len();
        let sta_n = self.config.sta_window.clamp(4, 64).min(n);
        let lta_n = self.config.lta_window.clamp(sta_n, 256).min(n);
        let sta_mean = self.ring[n - sta_n..]
            .iter()
            .map(|p| p.accel_x_g.hypot(p.accel_y_g))
            .sum::<f64>()
            / (sta_n as f64);
        let lta_mean = self.ring[n - lta_n..]
            .iter()
            .map(|p| p.accel_x_g.hypot(p.accel_y_g))
            .sum::<f64>()
            / (lta_n as f64);
        let sta_lta = (sta_mean / lta_mean.max(0.0045)).clamp(0.2, 25.0);
        if let Some(last) = self.ring.last_mut() {
            last.sta_lta_ratio = sta_lta;
        }

        // Exponentially smoothed dominant frequency over latest STA window
        let recent_f = self.ring[n - sta_n..]
            .iter()
            .map(|p| p.observed_freq_hz)
            .sum::<f64>()
            / (sta_n as f64);
        self.current_freq_1_hz = recent_f;
        let base_f = self.baseline_freq_1_hz.max(0.2);
        self.current_freq_shift_pct = ((recent_f - base_f) / base_f) * 100.0;
        let freq_drop_pct = (-self.current_freq_shift_pct).max(0.0);

        self.current_sta_lta = sta_lta;
        self.peak_sta_lta = self.peak_sta_lta.max(sta_lta);
        self.peak_accel_g = self.peak_accel_g.max(mag_g);
        self.peak_drift_pct = self.peak_drift_pct.max(drift_ratio * 100.0);

        // Anomaly Z-score combining acceleration burst + frequency drop
        let lta_rms = (self.ring[n - lta_n..]
            .iter()
            .map(|p| p.accel_x_g.powi(2) + p.accel_y_g.powi(2))
            .sum::<f64>()
            / (lta_n as f64))
            .sqrt()
            .max(0.004);
        let accel_z = ((mag_g - lta_mean) / lta_rms).max(0.0);
        self.anomaly_z_score = accel_z.max(freq_drop_pct * 0.45);

        self.freq_history_hz.push((t_s, recent_f));
        if self.freq_history_hz.len() > cap {
            self.freq_history_hz.remove(0);
        }
        self.sta_lta_history.push((t_s, sta_lta));
        if self.sta_lta_history.len() > cap {
            self.sta_lta_history.remove(0);
        }

        // Hysteresis-guarded Watchdog escalation
        let trigger_hit = sta_lta >= self.config.sta_lta_trigger_ratio
            || freq_drop_pct >= self.config.freq_drop_warning_pct
            || drift_ratio >= 0.008;
        if trigger_hit {
            self.consecutive_exceedances = self.consecutive_exceedances.saturating_add(1);
        } else if self.consecutive_exceedances > 0 {
            self.consecutive_exceedances -= 1;
        }

        let candidate = if self.consecutive_exceedances >= self.config.hysteresis_windows.max(1) {
            if freq_drop_pct >= self.config.freq_drop_critical_pct
                || drift_ratio >= 0.015
                || (sta_lta >= self.config.sta_lta_trigger_ratio * 1.45 && freq_drop_pct >= 6.0)
            {
                WatchdogAlertLevel::CriticalTrigger
            } else {
                WatchdogAlertLevel::Warning
            }
        } else if sta_lta >= self.config.sta_lta_trigger_ratio * 0.72 || freq_drop_pct >= 3.5 {
            WatchdogAlertLevel::Advisory
        } else {
            WatchdogAlertLevel::Monitoring
        };

        self.alert_level = candidate;
    }

    fn synthesize_sim_packet(&self, t_s: f64, num_stories: usize) -> ShmTelemetryPacket {
        let base_f = self.baseline_freq_1_hz.max(0.4);
        let story = (self.total_packets_ingested as usize % num_stories.max(1)) + 1;
        let story_factor = 0.45 + 0.55 * (story as f64 / num_stories.max(1) as f64);

        match self.config.sim_mode {
            WatchdogSimMode::Off => ShmTelemetryPacket::default(),
            WatchdogSimMode::AmbientNominal => {
                let ax = 0.0035 * (2.0 * PI * base_f * t_s).sin()
                    + 0.0012 * (2.0 * PI * base_f * 2.7 * t_s).cos();
                let ay = 0.0024 * (2.0 * PI * base_f * 1.05 * t_s + 0.5).sin();
                let az = 0.0015 * (2.0 * PI * 6.2 * t_s).sin();
                ShmTelemetryPacket {
                    sensor_id: format!("MEMS-L{story}"),
                    story_number: story,
                    strain_micro: (42.0 + 3.0 * (t_s * 1.5).sin()) as f32,
                    freq_hz: (base_f * (1.0 + 0.004 * (t_s * 0.7).sin())) as f32,
                    accel_g: ax.abs() as f32,
                    accel_x_g: Some(ax as f32),
                    accel_y_g: Some(ay as f32),
                    accel_z_g: Some(az as f32),
                    drift_ratio: Some((0.0005 * story_factor) as f32),
                    temp_c: 28.4,
                    timestamp: web_bridge::unix_timestamp_label(),
                }
            }
            WatchdogSimMode::AftershockPulse => {
                // Quiet ambient baseline for first 1.8 s so LTA establishes cleanly, then sharp shock!
                let (amp, freq_drop, drift) = if t_s < 1.8 {
                    (0.0045, 0.0, 0.0006)
                } else {
                    let tau = t_s - 1.8;
                    let env = (tau * 2.4).min(1.0) * (-0.12 * (tau - 1.2).max(0.0)).exp();
                    (0.008 + 0.28 * env * story_factor, 0.16 * (tau / 2.0).min(1.0), 0.014 * env)
                };
                let obs_f = base_f * (1.0 - freq_drop);
                let ax = amp * (2.0 * PI * obs_f * t_s).sin();
                let ay = amp * 0.65 * (2.0 * PI * obs_f * 1.12 * t_s + 0.7).sin();
                let az = amp * 0.38 * (2.0 * PI * 4.8 * t_s).cos();
                ShmTelemetryPacket {
                    sensor_id: format!("MEMS-L{story}"),
                    story_number: story,
                    strain_micro: (85.0 + 360.0 * amp) as f32,
                    freq_hz: obs_f as f32,
                    accel_g: ax.abs() as f32,
                    accel_x_g: Some(ax as f32),
                    accel_y_g: Some(ay as f32),
                    accel_z_g: Some(az as f32),
                    drift_ratio: Some(drift as f32),
                    temp_c: 29.1,
                    timestamp: web_bridge::unix_timestamp_label(),
                }
            }
            WatchdogSimMode::ProgressiveSoftStory => {
                let ramp = ((t_s - 1.2).max(0.0) / 3.0).clamp(0.0, 1.0);
                let freq_drop = 0.26 * ramp;
                let obs_f = base_f * (1.0 - freq_drop);
                let amp = 0.005 + 0.16 * ramp * if story == 1 { 1.35 } else { 0.85 };
                let drift = 0.0007 + 0.0175 * ramp * if story == 1 { 1.4 } else { 0.7 };
                let ax = amp * (2.0 * PI * obs_f * t_s).sin();
                let ay = amp * 0.58 * (2.0 * PI * obs_f * 1.08 * t_s).cos();
                let az = amp * 0.25 * (2.0 * PI * 5.2 * t_s).sin();
                ShmTelemetryPacket {
                    sensor_id: format!("MEMS-L{story}"),
                    story_number: story,
                    strain_micro: (60.0 + 520.0 * ramp) as f32,
                    freq_hz: obs_f as f32,
                    accel_g: ax.abs() as f32,
                    accel_x_g: Some(ax as f32),
                    accel_y_g: Some(ay as f32),
                    accel_z_g: Some(az as f32),
                    drift_ratio: Some(drift as f32),
                    temp_c: 29.6,
                    timestamp: web_bridge::unix_timestamp_label(),
                }
            }
        }
    }

    /// Advances the Live WebSocket / Simulator Watchdog by `dt_s` seconds and evaluates
    /// closed-loop escalation triggers.
    pub fn step_live_tick(
        &mut self,
        dt_s: f64,
        baseline_freq_1_hz: f64,
        num_stories: usize,
    ) -> ShmWatchdogTickOutcome {
        if baseline_freq_1_hz > 0.15 {
            self.baseline_freq_1_hz = baseline_freq_1_hz;
        }
        let (ws_conn, ws_msg) = web_bridge::shm_connection_snapshot();
        self.ws_connected = ws_conn;
        if ws_conn && !ws_msg.is_empty() {
            self.ws_status = ws_msg;
        }

        let mut outcome = ShmWatchdogTickOutcome::default();
        let incoming = web_bridge::shm_watchdog_poll_packets();
        for pkt in &incoming {
            self.ingest_packet(pkt, num_stories);
            outcome.new_packets += 1;
        }

        if self.config.sim_mode != WatchdogSimMode::Off {
            let period_s = 1.0 / self.config.sampling_rate_hz.clamp(2.0, 100.0);
            self.sim_accum_s += dt_s.clamp(0.0, 0.25);
            let mut emitted = 0usize;
            while self.sim_accum_s >= period_s && emitted < 8 {
                self.sim_accum_s -= period_s;
                self.sim_elapsed_s += period_s;
                let pkt = self.synthesize_sim_packet(self.sim_elapsed_s, num_stories);
                self.ingest_packet(&pkt, num_stories);
                outcome.new_packets += 1;
                emitted += 1;
            }
        }

        if outcome.new_packets > 0
            && self.alert_level >= WatchdogAlertLevel::Warning
            && self.alert_level > self.last_escalated_level
        {
            self.last_escalated_level = self.alert_level;
            outcome.escalated = true;
            outcome.should_refresh_shm = self.config.auto_update_atc20;
            outcome.should_reroute_evacuation = self.config.auto_reroute_evacuation;
            let banner = format!(
                "Watchdog {}: STA/LTA {:.2} • f1 {:+.1}% • Peak Drift {:.2}%",
                self.alert_level.label(),
                self.current_sta_lta,
                self.current_freq_shift_pct,
                self.peak_drift_pct
            );
            outcome.alert_banner = Some(banner.clone());
            self.push_event(
                self.alert_level,
                banner,
                None,
                outcome.should_reroute_evacuation,
            );
        } else if self.alert_level == WatchdogAlertLevel::Monitoring
            && self.last_escalated_level >= WatchdogAlertLevel::Warning
        {
            self.last_escalated_level = WatchdogAlertLevel::Monitoring;
        }

        outcome
    }

    /// Converts the active Watchdog ring buffer into a `CustomShmTelemetryLog` for Phase 76
    /// 6-DOF Modal Shift & ATC-20 Placard evaluation.
    pub fn to_custom_telemetry_log(&self) -> Option<CustomShmTelemetryLog> {
        if self.ring.is_empty() {
            return None;
        }
        let max_story = self
            .ring
            .iter()
            .map(|p| p.story_number)
            .max()
            .unwrap_or(1)
            .max(1);
        let t0 = self.ring.first().map(|p| p.time_s).unwrap_or(0.0);
        let t1 = self.ring.last().map(|p| p.time_s).unwrap_or(0.0);
        let records: Vec<ShmTelemetryRecord> = self
            .ring
            .iter()
            .map(|p| ShmTelemetryRecord {
                story_number: p.story_number,
                sensor_id: p.sensor_id.clone(),
                time_s: (p.time_s - t0).max(0.0),
                accel_x_g: p.accel_x_g,
                accel_y_g: p.accel_y_g,
                accel_z_g: p.accel_z_g,
                drift_ratio: Some(p.drift_ratio),
                strain_micro: Some(p.strain_micro),
            })
            .collect();
        Some(CustomShmTelemetryLog {
            source_name: if self.ws_connected {
                self.config.ws_url.clone()
            } else {
                format!("LiveWatchdog_{}", self.config.sim_mode.short_label())
            },
            sampling_rate_hz: self.config.sampling_rate_hz,
            duration_s: (t1 - t0).max(1.0),
            story_channels: max_story,
            records,
        })
    }

    /// Emits a non-destructive IFCX overlay (`eigengraph::analysis::shm_watchdog`) capturing the
    /// rolling stream window, STA/LTA trigger state, and closed-loop escalation ledger.
    pub fn build_ifcx_overlay(&self, bim_project: &BimProject) -> IfcxDocument {
        let raw_name = if bim_project.project_name.trim().is_empty() {
            "active_project".to_string()
        } else {
            bim_project.project_name.clone()
        };
        let slug: String = raw_name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();
        let mut doc = IfcxDocument::new(&format!("{slug}_shm_watchdog_overlay"));
        doc.imports.push(IfcxImport {
            uri: format!("eigengraph://{slug}.ifcx"),
            integrity: None,
            role: Some("base-building".to_string()),
        });
        doc.schemas.insert(
            NS_EG_SHM_WATCHDOG.to_string(),
            json!({
                "value": {
                    "dataType": "Object",
                    "spec": "EigenGraph Phase 78 Live WebSocket SHM Watchdog & Closed-Loop Cascade Schema"
                }
            }),
        );

        let site_path = format!("/earth/sites/{slug}");
        let bldg_path = format!("{site_path}/buildings/{slug}");
        let wd_path = format!("{bldg_path}/analysis/shm_watchdog");

        let payload = json!({
            "wsUrl": self.config.ws_url,
            "wsConnected": self.ws_connected,
            "simMode": self.config.sim_mode.short_label(),
            "alertLevel": self.alert_level.label(),
            "packetsIngested": self.total_packets_ingested,
            "ringSamples": self.ring.len(),
            "baselineFreq1Hz": self.baseline_freq_1_hz,
            "currentFreq1Hz": self.current_freq_1_hz,
            "freqShiftPct": self.current_freq_shift_pct,
            "currentStaLta": self.current_sta_lta,
            "peakStaLta": self.peak_sta_lta,
            "peakAccelG": self.peak_accel_g,
            "peakDriftPct": self.peak_drift_pct,
            "anomalyZScore": self.anomaly_z_score,
            "autoRerouteCount": self.auto_reroute_count,
            "events": self.events.iter().map(|e| json!({
                "seq": e.seq,
                "timestamp": e.timestamp_label,
                "level": e.level.short_label(),
                "summary": e.summary,
                "triggeredAtc20": e.triggered_atc20.map(|p| p.short_label()),
                "triggeredEvacReroute": e.triggered_evac_reroute,
            })).collect::<Vec<_>>(),
        });

        let mut bldg_node = IfcxNode::new(&bldg_path).with_class("IfcBuilding");
        bldg_node
            .children
            .insert("shm_watchdog".to_string(), wd_path.clone());
        bldg_node
            .attributes
            .insert(NS_EG_SHM_WATCHDOG.to_string(), payload.clone());
        doc.add_node(bldg_node);

        let mut wd_node = IfcxNode::new(&wd_path)
            .with_class("IfcEvent")
            .with_prop(
                "Live SHM Watchdog",
                self.alert_level.label(),
                &format!(
                    "Packets {} | STA/LTA {:.2} (Peak {:.2}) | f1 {:+.1}% | Auto-Reroutes {}",
                    self.total_packets_ingested,
                    self.current_sta_lta,
                    self.peak_sta_lta,
                    self.current_freq_shift_pct,
                    self.auto_reroute_count,
                ),
            );
        wd_node
            .attributes
            .insert(NS_EG_SHM_WATCHDOG.to_string(), payload);
        doc.add_node(wd_node);
        doc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::evacuation_routing::{evaluate_evacuation_and_lifelines, EvacuationRoutingConfig};
    use crate::core::gis::city_builder::IfcxCityScene;
    use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
    use crate::core::shm_twin::{
        evaluate_shm_digital_twin_for_bim, ShmDigitalTwinConfig, ShmFaultScenario,
    };

    #[test]
    fn phase78_websocket_triaxial_packet_parser_and_fanout() {
        let json_pkt = r#"{"sensor_id":"MEMS-L2","story_number":2,"strain_micro":142.5,"freq_hz":1.88,"accel_x_g":0.18,"accel_y_g":0.11,"accel_z_g":0.06,"drift_ratio":0.012}"#;
        let parsed = web_bridge::parse_shm_packet_payload(json_pkt).expect("JSON packet parses");
        assert_eq!(parsed.sensor_id, "MEMS-L2");
        assert_eq!(parsed.story_number, 2);
        assert!((parsed.accel_x_g.unwrap_or(0.0) - 0.18).abs() < 1e-5);
        assert!((parsed.drift_ratio.unwrap_or(0.0) - 0.012).abs() < 1e-5);

        let csv_pkt = web_bridge::parse_shm_packet_payload("SG-03,95.0,2.10,0.04,28.0,3,0.045,0.021,0.01,0.004")
            .expect("Extended CSV packet parses");
        assert_eq!(csv_pkt.story_number, 3);
        assert!((csv_pkt.accel_y_g.unwrap_or(0.0) - 0.021).abs() < 1e-5);

        // Drain any stale entries then test dual fan-out
        let _ = web_bridge::shm_poll_packets();
        let _ = web_bridge::shm_watchdog_poll_packets();
        web_bridge::shm_inject_packet(parsed.clone());
        let fac_pkts = web_bridge::shm_poll_packets();
        let wd_pkts = web_bridge::shm_watchdog_poll_packets();
        assert_eq!(fac_pkts.len(), 1);
        assert_eq!(wd_pkts.len(), 1);
        assert_eq!(wd_pkts[0].sensor_id, "MEMS-L2");
    }

    #[test]
    fn phase78_shm_watchdog_sta_lta_trigger_and_closed_loop_evac_cascade() {
        let proj = BimProject::sample_bimcraft_building();
        let site = SiteEnvironmentalCoupling::default();
        let city = IfcxCityScene::default();

        let mut wd = ShmWatchdogState::default();
        wd.start_simulator(WatchdogSimMode::AftershockPulse);

        let mut triggered_outcome = ShmWatchdogTickOutcome::default();
        // Run 4.5 seconds of deterministic 20 Hz streaming (ambient baseline -> aftershock shock)
        for _ in 0..90 {
            let out = wd.step_live_tick(0.05, 2.25, 2);
            if out.should_refresh_shm {
                triggered_outcome = out;
                break;
            }
        }

        assert!(wd.total_packets_ingested >= 40);
        assert!(wd.peak_sta_lta >= wd.config.sta_lta_trigger_ratio);
        assert!(wd.alert_level >= WatchdogAlertLevel::Warning);
        assert!(triggered_outcome.escalated);
        assert!(triggered_outcome.should_refresh_shm);
        assert!(triggered_outcome.should_reroute_evacuation);

        // Convert ring buffer into Phase 76 CustomShmTelemetryLog & verify ATC-20 escalation
        let custom_log = wd.to_custom_telemetry_log().expect("Ring log");
        let shm_cfg = ShmDigitalTwinConfig {
            fault_scenario: ShmFaultScenario::CustomTelemetryStream,
            custom_telemetry: Some(custom_log),
            ..Default::default()
        };
        let shm_rep = evaluate_shm_digital_twin_for_bim(
            &proj,
            &site,
            None,
            None,
            None,
            None,
            &shm_cfg,
            (-6.2088, 106.8456),
        )
        .expect("SHM evaluation from watchdog ring buffer succeeds");
        assert!(shm_rep.atc20_placard >= Atc20Placard::RestrictedYellow);

        // Closed-loop Phase 77 evacuation rerouting from escalated SHM placard
        let evac_rep = evaluate_evacuation_and_lifelines(
            &proj,
            &city,
            Some(&shm_rep),
            None,
            None,
            None,
            None,
            None,
            &EvacuationRoutingConfig::default(),
            (-6.2088, 106.8456),
        )
        .expect("Closed-loop evacuation routing succeeds");
        assert!(evac_rep.active_evac_route.as_ref().is_some_and(|r| r.reachable));

        let overlay = wd.build_ifcx_overlay(&proj);
        assert!(overlay.to_json_pretty().contains(NS_EG_SHM_WATCHDOG));
    }
}
