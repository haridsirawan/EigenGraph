//! core/construction_lca_studio.rs — Update `v0.8.4.9` (`9/10` -> `v0.8.5`):
//! 4D/5D/6D Construction Sequence, AHSP Cost S-Curve & Embodied Carbon LCA 3D Studio
//! (`CPM` / `Permen PUPR AHSP` / `EN 15978 A1-A5`).
//!
//! Couples live `BimProject` levels/footprint/quantities and `SiteEnvironmentalCoupling`
//! climate rain delay + remote logistics into [`ScheduleEngine::compute_cpm`],
//! [`ScheduleEngine::generate_scurve`], [`CostEngine`], and [`CarbonSteelRoute`] LCA,
//! generating 3D ENU story erection sequence prisms, a slewing tower crane, and
//! non-destructive `eigengraph::analysis::construction_4d5d6d` IFCX overlays.

use serde::{Deserialize, Serialize};

use crate::core::bim::BimProject;
use crate::core::carbon::CarbonSteelRoute;
use crate::core::cost::CostEngine;
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
use crate::core::schedule::{ConstructionActivity, ScheduleEngine};

/// Canonical EigenGraph IFCX attribute namespace for 4D/5D/6D Construction & LCA overlays.
pub const NS_EG_CONSTRUCTION_4D5D6D: &str = "eigengraph::analysis::construction_4d5d6d";

/// Structural steel manufacturing & smelting route for EN 15978 A1-A3 embodied carbon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SteelSmeltingRoute {
    BfBofStandard,
    EafRecycled,
    GreenHydrogenDri,
}

impl SteelSmeltingRoute {
    pub fn to_core(self) -> CarbonSteelRoute {
        match self {
            Self::BfBofStandard => CarbonSteelRoute::BfBofStandard,
            Self::EafRecycled => CarbonSteelRoute::EafRecycled,
            Self::GreenHydrogenDri => CarbonSteelRoute::GreenHydrogenDri,
        }
    }

    pub fn label(self) -> &'static str {
        self.to_core().display_name()
    }

    pub fn emission_factor_kg_per_kg(self) -> f64 {
        self.to_core().emission_factor_kg_per_kg()
    }
}

/// Ready-mix structural concrete binder blend for EN 15978 A1-A3 embodied carbon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConcreteMixGrade {
    OpcStandard,
    Ggbs30LowCarbon,
    GeopolymerGreen,
}

impl ConcreteMixGrade {
    pub fn label(self) -> &'static str {
        match self {
            Self::OpcStandard => "Standard OPC (CEM I)",
            Self::Ggbs30LowCarbon => "Low-Carbon 30% GGBS",
            Self::GeopolymerGreen => "Geopolymer Green Mix",
        }
    }

    pub fn emission_kg_per_m3(self) -> f64 {
        match self {
            Self::OpcStandard => 340.0,
            Self::Ggbs30LowCarbon => 235.0,
            Self::GeopolymerGreen => 145.0,
        }
    }
}

/// Persisted configuration for the 4D/5D/6D Construction Sequence, Cost S-Curve & LCA 3D Studio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstructionLcaConfig {
    pub active_day: usize,
    pub steel_route: SteelSmeltingRoute,
    pub concrete_mix: ConcreteMixGrade,
    pub transport_distance_km: f64,
    pub crane_crews: usize,
    pub steel_rate_idr_kg: f64,
    pub concrete_rate_idr_m3: f64,
    pub include_rain_delay: bool,
}

impl Default for ConstructionLcaConfig {
    fn default() -> Self {
        Self {
            active_day: 90,
            steel_route: SteelSmeltingRoute::EafRecycled,
            concrete_mix: ConcreteMixGrade::Ggbs30LowCarbon,
            transport_distance_km: 65.0,
            crane_crews: 2,
            steel_rate_idr_kg: 24_500.0,
            concrete_rate_idr_m3: 1_450_000.0,
            include_rain_delay: true,
        }
    }
}

/// Serializable 4D WBS Critical Path activity DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WbsActivityDto {
    pub id: usize,
    pub name: String,
    pub duration_days: usize,
    pub early_start: usize,
    pub early_finish: usize,
    pub total_float: usize,
    pub is_critical: bool,
    pub weight_pct: f64,
    pub status: String,
}

/// Weekly 5D Cost & 6D Carbon S-Curve sample point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SCurvePointDto {
    pub week_number: usize,
    pub day_number: usize,
    pub weekly_planned_pct: f64,
    pub cumulative_planned_pct: f64,
    pub cumulative_cost_idr_m: f64,
    pub cumulative_carbon_ton: f64,
}

/// EN 15978 Cradle-to-Site Life Cycle Assessment stage summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LcaStageDto {
    pub stage_code: String,
    pub stage_name: String,
    pub emission_ton_co2e: f64,
    pub percentage: f64,
}

/// 3D ENU story-by-story 4D construction progress prism with 5D cost & 6D carbon attribution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorySequenceZone3D {
    pub story_id: String,
    pub min_enu_m: [f64; 3],
    pub max_enu_m: [f64; 3],
    pub start_day: usize,
    pub finish_day: usize,
    pub completion_pct: f64,
    pub cost_idr_m: f64,
    pub carbon_ton_co2e: f64,
    pub is_critical_path: bool,
}

/// 3D ENU tower crane geometry slewing according to the active 4D simulation day.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TowerCrane3D {
    pub base_enu_m: [f64; 3],
    pub mast_height_m: f64,
    pub jib_length_m: f64,
    pub slew_deg: f64,
}

/// Compact serializable IFCX overlay payload under `eigengraph::analysis::construction_4d5d6d`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstructionLcaIfcxOverlay {
    pub schema_version: String,
    pub active_day: usize,
    pub total_duration_days: usize,
    pub physical_progress_pct: f64,
    pub total_budget_idr_billion: f64,
    pub spent_to_date_idr_billion: f64,
    pub steel_tonnage: f64,
    pub concrete_m3: f64,
    pub total_embodied_carbon_ton: f64,
    pub baseline_carbon_ton: f64,
    pub carbon_reduction_pct: f64,
    pub carbon_intensity_kg_m2: f64,
    pub greenship_leed_tier: String,
    pub critical_path_count: usize,
    pub summary_verdict: String,
}

/// Unified 4D Construction Sequence, 5D Cost S-Curve & 6D Embodied Carbon LCA Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructionLcaStudioReport {
    pub config: ConstructionLcaConfig,
    pub total_duration_days: usize,
    pub physical_progress_pct: f64,
    pub total_budget_idr: f64,
    pub spent_to_date_idr: f64,
    pub total_budget_label: String,
    pub steel_tonnage: f64,
    pub concrete_m3: f64,
    pub gfa_m2: f64,
    pub a1_a3_carbon_ton: f64,
    pub a4_carbon_ton: f64,
    pub a5_carbon_ton: f64,
    pub total_embodied_carbon_ton: f64,
    pub baseline_carbon_ton: f64,
    pub carbon_reduction_pct: f64,
    pub carbon_intensity_kg_m2: f64,
    pub greenship_leed_tier: String,
    pub activities: Vec<WbsActivityDto>,
    pub scurve: Vec<SCurvePointDto>,
    pub lca_stages: Vec<LcaStageDto>,
    pub story_zones_3d: Vec<StorySequenceZone3D>,
    pub crane_3d: TowerCrane3D,
    pub critical_path_count: usize,
    pub summary_verdict: String,
}

impl ConstructionLcaStudioReport {
    /// Converts the 4D/5D/6D studio report into a compact non-destructive `.ifcx` overlay payload.
    pub fn to_ifcx_overlay(&self) -> ConstructionLcaIfcxOverlay {
        ConstructionLcaIfcxOverlay {
            schema_version: "construction-4d5d6d-ifcx-v1".to_string(),
            active_day: self.config.active_day,
            total_duration_days: self.total_duration_days,
            physical_progress_pct: self.physical_progress_pct,
            total_budget_idr_billion: self.total_budget_idr / 1e9,
            spent_to_date_idr_billion: self.spent_to_date_idr / 1e9,
            steel_tonnage: self.steel_tonnage,
            concrete_m3: self.concrete_m3,
            total_embodied_carbon_ton: self.total_embodied_carbon_ton,
            baseline_carbon_ton: self.baseline_carbon_ton,
            carbon_reduction_pct: self.carbon_reduction_pct,
            carbon_intensity_kg_m2: self.carbon_intensity_kg_m2,
            greenship_leed_tier: self.greenship_leed_tier.clone(),
            critical_path_count: self.critical_path_count,
            summary_verdict: self.summary_verdict.clone(),
        }
    }
}

/// Evaluates 4D CPM Construction Schedule, 5D AHSP Cost S-Curve, and 6D EN 15978 Embodied Carbon LCA
/// coupled directly to live `BimProject` levels/footprint and site climate/logistics multipliers.
pub fn evaluate_construction_lca_studio(
    bim: &BimProject,
    site: Option<&SiteEnvironmentalCoupling>,
    config: &ConstructionLcaConfig,
) -> ConstructionLcaStudioReport {
    let mut cfg = config.clone();
    cfg.active_day = cfg.active_day.min(540);
    cfg.transport_distance_km = cfg.transport_distance_km.clamp(5.0, 1500.0);
    cfg.crane_crews = cfg.crane_crews.clamp(1, 6);
    cfg.steel_rate_idr_kg = cfg.steel_rate_idr_kg.clamp(12_000.0, 60_000.0);
    cfg.concrete_rate_idr_m3 = cfg.concrete_rate_idr_m3.clamp(750_000.0, 4_000_000.0);

    // 1. Extract BIM building envelope & unique story elevations
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for c in &bim.columns {
        min_x = min_x.min(c.pos.0 * 0.001);
        max_x = max_x.max(c.pos.0 * 0.001);
        min_y = min_y.min(c.pos.1 * 0.001);
        max_y = max_y.max(c.pos.1 * 0.001);
    }
    if !min_x.is_finite() || (max_x - min_x) < 4.0 {
        min_x = 0.0;
        max_x = 14.0;
        min_y = 0.0;
        max_y = 10.0;
    }

    let mut elevs_m: Vec<f64> = bim
        .levels
        .iter()
        .map(|l| l.elevation_mm * 0.001)
        .collect();
    elevs_m.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    elevs_m.dedup_by(|a, b| (*a - *b).abs() < 0.25);
    if elevs_m.len() < 2 {
        elevs_m = vec![0.0, 3.8, 7.6, 11.4];
    }
    let stories = (elevs_m.len() - 1).clamp(1, 10);
    let footprint_m2 = (max_x - min_x).max(6.0) * (max_y - min_y).max(6.0);
    let gfa_m2 = footprint_m2 * (stories as f64);

    // 2. Material Quantities (concrete m3 + structural steel & rebar tonnage)
    let slab_vol = footprint_m2 * 0.22 * (stories as f64);
    let frame_vol = (bim.columns.len().max(4) as f64) * 0.45 * 0.45 * 3.8
        + (bim.footings.len().max(4) as f64) * 1.44;
    let concrete_m3 = (slab_vol + frame_vol).max(gfa_m2 * 0.32);
    let steel_tonnage = ((concrete_m3 * 0.145) + (gfa_m2 * 0.018)).max(12.0);
    let steel_kg = steel_tonnage * 1000.0;

    // 3. Site Rain Delay & Remote Logistics Coupling + 5D AHSP Budget
    let rain_pct = if cfg.include_rain_delay {
        site.map(|s| f64::from(s.climate_rain_delay_pct)).unwrap_or(8.0)
    } else {
        0.0
    };
    let logistics_mult = site
        .map(|s| f64::from(s.remote_logistics_multiplier).clamp(1.0, 1.4))
        .unwrap_or(1.0);
    let rain_mult = 1.0 + (rain_pct / 100.0) * 0.5;
    let crew_speed = 1.0 / (0.65 + 0.35 * (cfg.crane_crews as f64));

    let direct_steel_idr = steel_kg * (cfg.steel_rate_idr_kg + 17_500.0) * logistics_mult;
    let direct_conc_idr = concrete_m3 * cfg.concrete_rate_idr_m3 * logistics_mult;
    let direct_finishes_idr = gfa_m2 * 1_850_000.0;
    let direct_total_idr = direct_steel_idr + direct_conc_idr + direct_finishes_idr;
    // AHSP 10% OH&P + 11% VAT
    let total_budget_idr = direct_total_idr * 1.10 * 1.11;
    let total_budget_label = format!("IDR {}", CostEngine::format_idr(total_budget_idr));

    // 4. 4D WBS Activities & CPM Solve
    let scale_days = |base: f64| -> usize {
        ((base * rain_mult * crew_speed).round() as usize).clamp(4, 120)
    };
    let mut raw_acts = vec![
        ConstructionActivity {
            id: 1,
            name: "WBS-01 Piling & Subsurface Mobilization".to_string(),
            duration_days: ((20.0 * rain_mult).round() as usize).max(10),
            predecessors: vec![],
            early_start: 0,
            early_finish: 0,
            late_start: 0,
            late_finish: 0,
            total_float: 0,
            free_float: 0,
            is_critical: false,
            weight_percentage: 12.0,
        },
        ConstructionActivity {
            id: 2,
            name: "WBS-02 Pile Caps, Tie Beams & Raft Slab".to_string(),
            duration_days: ((16.0 * rain_mult).round() as usize).max(8),
            predecessors: vec![1],
            early_start: 0,
            early_finish: 0,
            late_start: 0,
            late_finish: 0,
            total_float: 0,
            free_float: 0,
            is_critical: false,
            weight_percentage: 14.0,
        },
    ];
    let story_weight = 52.0 / (stories as f64);
    for s in 0..stories {
        let id = 3 + s;
        let pred = if s == 0 { 2 } else { 2 + s };
        raw_acts.push(ConstructionActivity {
            id,
            name: format!("WBS-{id:02} Superstructure Story L{} Frame & Deck", s + 1),
            duration_days: scale_days(18.0),
            predecessors: vec![pred],
            early_start: 0,
            early_finish: 0,
            late_start: 0,
            late_finish: 0,
            total_float: 0,
            free_float: 0,
            is_critical: false,
            weight_percentage: story_weight,
        });
    }
    let last_story_id = 2 + stories;
    let mep_id = last_story_id + 1;
    let hand_id = mep_id + 1;
    raw_acts.push(ConstructionActivity {
        id: mep_id,
        name: format!("WBS-{mep_id:02} Facade Cladding & MEP Risers"),
        duration_days: scale_days(16.0),
        predecessors: vec![2.min(last_story_id)],
        early_start: 0,
        early_finish: 0,
        late_start: 0,
        late_finish: 0,
        total_float: 0,
        free_float: 0,
        is_critical: false,
        weight_percentage: 14.0,
    });
    raw_acts.push(ConstructionActivity {
        id: hand_id,
        name: format!("WBS-{hand_id:02} Testing, Commissioning & Handover"),
        duration_days: 10,
        predecessors: vec![last_story_id, mep_id],
        early_start: 0,
        early_finish: 0,
        late_start: 0,
        late_finish: 0,
        total_float: 0,
        free_float: 0,
        is_critical: false,
        weight_percentage: 8.0,
    });

    let cpm_acts = ScheduleEngine::compute_cpm(raw_acts).unwrap_or_default();
    let total_duration_days = cpm_acts
        .iter()
        .map(|a| a.early_finish)
        .max()
        .unwrap_or(60)
        .max(1);

    // 5. 6D EN 15978 Embodied Carbon LCA (A1-A5)
    let a1_a3_steel_t = (steel_kg * cfg.steel_route.emission_factor_kg_per_kg()) / 1000.0;
    let a1_a3_conc_t = (concrete_m3 * cfg.concrete_mix.emission_kg_per_m3()) / 1000.0;
    let a1_a3_carbon_ton = a1_a3_steel_t + a1_a3_conc_t;

    let concrete_tonnage = concrete_m3 * 2.40;
    let a4_carbon_ton =
        ((steel_tonnage + concrete_tonnage) * cfg.transport_distance_km * 0.082) / 1000.0;
    let a5_carbon_ton =
        (steel_tonnage * 26.5 + concrete_m3 * 14.0 + (cfg.crane_crews as f64) * 180.0) / 1000.0;
    let total_embodied_carbon_ton = (a1_a3_carbon_ton + a4_carbon_ton + a5_carbon_ton).max(1.0);

    let base_a1_a3_t = (steel_kg * SteelSmeltingRoute::BfBofStandard.emission_factor_kg_per_kg()
        + concrete_m3 * ConcreteMixGrade::OpcStandard.emission_kg_per_m3())
        / 1000.0;
    let baseline_carbon_ton = (base_a1_a3_t + a4_carbon_ton + a5_carbon_ton).max(total_embodied_carbon_ton);
    let carbon_reduction_pct = if baseline_carbon_ton > 1e-6 {
        ((baseline_carbon_ton - total_embodied_carbon_ton) / baseline_carbon_ton * 100.0)
            .clamp(0.0, 95.0)
    } else {
        0.0
    };
    let carbon_intensity_kg_m2 = (total_embodied_carbon_ton * 1000.0) / gfa_m2.max(1.0);
    let greenship_leed_tier = if carbon_reduction_pct >= 50.0 {
        "LEED Platinum / GBCI Greenship Platinum".to_string()
    } else if carbon_reduction_pct >= 35.0 {
        "LEED Gold / GBCI Greenship Gold".to_string()
    } else if carbon_reduction_pct >= 20.0 {
        "LEED Silver / GBCI Greenship Silver".to_string()
    } else if carbon_reduction_pct >= 10.0 {
        "LEED Certified / GBCI Bronze".to_string()
    } else {
        "Baseline Conventional Industrial Mix".to_string()
    };

    let lca_stages = vec![
        LcaStageDto {
            stage_code: "A1-A3".to_string(),
            stage_name: "Product Manufacturing (Steel & Concrete)".to_string(),
            emission_ton_co2e: a1_a3_carbon_ton,
            percentage: (a1_a3_carbon_ton / total_embodied_carbon_ton) * 100.0,
        },
        LcaStageDto {
            stage_code: "A4".to_string(),
            stage_name: format!("Site Freight Transport ({:.0} km)", cfg.transport_distance_km),
            emission_ton_co2e: a4_carbon_ton,
            percentage: (a4_carbon_ton / total_embodied_carbon_ton) * 100.0,
        },
        LcaStageDto {
            stage_code: "A5".to_string(),
            stage_name: "Cranage, Falsework & Site Installation".to_string(),
            emission_ton_co2e: a5_carbon_ton,
            percentage: (a5_carbon_ton / total_embodied_carbon_ton) * 100.0,
        },
    ];

    // 6. S-Curve & Progress at `active_day`
    let core_scurve = ScheduleEngine::generate_scurve(&cpm_acts, total_budget_idr);
    let scurve: Vec<SCurvePointDto> = core_scurve
        .into_iter()
        .map(|pt| SCurvePointDto {
            week_number: pt.week_number,
            day_number: pt.day_number,
            weekly_planned_pct: pt.weekly_planned_pct,
            cumulative_planned_pct: pt.cumulative_planned_pct,
            cumulative_cost_idr_m: pt.cumulative_cost_idr / 1e6,
            cumulative_carbon_ton: (pt.cumulative_planned_pct / 100.0) * total_embodied_carbon_ton,
        })
        .collect();

    let mut physical_progress_pct = 0.0_f64;
    let total_w: f64 = cpm_acts.iter().map(|a| a.weight_percentage).sum::<f64>().max(1.0);
    let mut activities = Vec::with_capacity(cpm_acts.len());
    for a in &cpm_acts {
        let dur = a.duration_days.max(1) as f64;
        let done_days = cfg.active_day.saturating_sub(a.early_start).min(a.duration_days) as f64;
        let frac = (done_days / dur).clamp(0.0, 1.0);
        physical_progress_pct += frac * (a.weight_percentage / total_w) * 100.0;
        let status = if cfg.active_day >= a.early_finish {
            "DONE".to_string()
        } else if cfg.active_day >= a.early_start {
            "ACTIVE".to_string()
        } else {
            "PLAN".to_string()
        };
        activities.push(WbsActivityDto {
            id: a.id,
            name: a.name.clone(),
            duration_days: a.duration_days,
            early_start: a.early_start,
            early_finish: a.early_finish,
            total_float: a.total_float,
            is_critical: a.is_critical,
            weight_pct: a.weight_percentage,
            status,
        });
    }
    physical_progress_pct = physical_progress_pct.clamp(0.0, 100.0);
    let spent_to_date_idr = total_budget_idr * (physical_progress_pct / 100.0);
    let critical_path_count = activities.iter().filter(|a| a.is_critical).count();

    // 7. 3D Story Erection Zones + Slewing Tower Crane
    let mut story_zones_3d = Vec::with_capacity(stories);
    for s in 0..stories {
        let z0 = elevs_m[s];
        let z1 = elevs_m[s + 1];
        let act = &activities[2 + s];
        let dur = act.duration_days.max(1) as f64;
        let comp = ((cfg.active_day.saturating_sub(act.early_start).min(act.duration_days) as f64)
            / dur
            * 100.0)
            .clamp(0.0, 100.0);
        story_zones_3d.push(StorySequenceZone3D {
            story_id: format!("L{}", s + 1),
            min_enu_m: [min_x, min_y, z0],
            max_enu_m: [max_x, max_y, z1],
            start_day: act.early_start,
            finish_day: act.early_finish,
            completion_pct: comp,
            cost_idr_m: (total_budget_idr / 1e6) * (0.65 / stories as f64),
            carbon_ton_co2e: total_embodied_carbon_ton / (stories as f64),
            is_critical_path: act.is_critical,
        });
    }
    let top_z = *elevs_m.last().unwrap_or(&11.4);
    let span_max = (max_x - min_x).max(max_y - min_y).max(10.0);
    let crane_3d = TowerCrane3D {
        base_enu_m: [max_x + 2.5, max_y + 2.5, 0.0],
        mast_height_m: top_z + 6.5,
        jib_length_m: (span_max * 0.95).clamp(12.0, 32.0),
        slew_deg: ((cfg.active_day as f64) * 17.5) % 360.0,
    };

    let summary_verdict = format!(
        "Day {}/{}: {:.0}% 4D Complete ({} CP tasks) | 5D {} | 6D {:.1} tCO2e (-{:.0}% vs BOF/OPC, {:.0} kg/m2)",
        cfg.active_day,
        total_duration_days,
        physical_progress_pct,
        critical_path_count,
        total_budget_label,
        total_embodied_carbon_ton,
        carbon_reduction_pct,
        carbon_intensity_kg_m2,
    );

    ConstructionLcaStudioReport {
        config: cfg,
        total_duration_days,
        physical_progress_pct,
        total_budget_idr,
        spent_to_date_idr,
        total_budget_label,
        steel_tonnage,
        concrete_m3,
        gfa_m2,
        a1_a3_carbon_ton,
        a4_carbon_ton,
        a5_carbon_ton,
        total_embodied_carbon_ton,
        baseline_carbon_ton,
        carbon_reduction_pct,
        carbon_intensity_kg_m2,
        greenship_leed_tier,
        activities,
        scurve,
        lca_stages,
        story_zones_3d,
        crane_3d,
        critical_path_count,
        summary_verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bim::BimProject;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};

    #[test]
    fn test_phase84_9_construction_4d_cpm_5d_scurve_and_6d_lca() {
        let bim = BimProject::sample_bimcraft_building();
        let rep = evaluate_construction_lca_studio(
            &bim,
            None,
            &ConstructionLcaConfig {
                active_day: 90,
                steel_route: SteelSmeltingRoute::GreenHydrogenDri,
                concrete_mix: ConcreteMixGrade::GeopolymerGreen,
                ..ConstructionLcaConfig::default()
            },
        );
        assert!(rep.total_duration_days > 30);
        assert!(!rep.activities.is_empty());
        assert!(rep.activities.iter().any(|a| a.is_critical));
        assert!(!rep.scurve.is_empty());
        assert!(rep.total_budget_idr > 0.0);
        assert!(rep.total_embodied_carbon_ton < rep.baseline_carbon_ton);
        assert!(rep.carbon_reduction_pct > 15.0);
        assert!(!rep.story_zones_3d.is_empty());
    }

    #[test]
    fn test_phase84_9_construction_lca_ifcx_overlay_roundtrip() {
        let bim = BimProject::sample_bimcraft_building();
        let rep = evaluate_construction_lca_studio(&bim, None, &ConstructionLcaConfig::default());
        let base = IfcxDocument::from_bim_project(&bim, None, 0.0, None, None, None);
        let composed =
            IfcxStage::compose_with_construction_lca_overlay(&base, &rep.to_ifcx_overlay());
        let json = composed.to_json_pretty();
        assert!(json.contains("eigengraph::analysis::construction_4d5d6d"));
        assert!(json.contains("/earth/analysis/construction_4d5d6d"));
        assert!(!json.contains("ARI"));
    }
}
