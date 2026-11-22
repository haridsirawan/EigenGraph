//! portfolio_retrofit.rs — Autonomous Portfolio-Scale `ARI v2` Retrofit Optimization Across Multi-Building Districts (Phase 73 / v0.7.3).
//!
//! Performs district-level multi-objective Pareto optimization across structural portfolios:
//! - Jointly optimizes `ARI v2` resilience lift, upfront retrofit cost (`USD`), and embodied carbon (`kgCO2e`)
//! - Sweeps budget/carbon scalarizations across all four [`RetrofitStrategy`] modes per vulnerable building
//! - Prunes dominated portfolios into a non-dominated Pareto frontier (`PortfolioParetoPoint`)
//! - Emits non-destructive OpenBIM IFCX district overlays (`eigengraph::portfolio::retrofit_plan`)

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::core::bim::BimProject;
use crate::core::gis::map_widget::SlippyMapState;
use crate::core::gis::world_index::WorldSpatialIndex;
use crate::core::ifcx::{ns as ifcx_ns, sanitize_slug, IfcxDocument, IfcxNode};
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
use crate::core::retrofit::RetrofitStrategy;
use crate::models::StructuralMember;

/// Custom IFCX namespace storing multi-building district portfolio retrofit optimization results.
pub const NS_EG_PORTFOLIO_RETROFIT: &str = "eigengraph::portfolio::retrofit_plan";

/// Multi-objective selection preference on the district Pareto frontier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PortfolioOptimizationObjective {
    MaxAriPerDollar,
    #[default]
    BalancedCostCarbon,
    MinEmbodiedCarbon,
    MaxDistrictResilience,
}

impl PortfolioOptimizationObjective {
    pub fn short_label(self) -> &'static str {
        match self {
            Self::MaxAriPerDollar => "ARI/$",
            Self::BalancedCostCarbon => "Balanced",
            Self::MinEmbodiedCarbon => "Low-CO2",
            Self::MaxDistrictResilience => "Max ARI",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::MaxAriPerDollar => "Maximize ARI Gain per USD",
            Self::BalancedCostCarbon => "Balanced Cost & Embodied Carbon Knee",
            Self::MinEmbodiedCarbon => "Minimize Embodied Carbon (kgCO2e)",
            Self::MaxDistrictResilience => "Maximize District Min-ARI Resilience",
        }
    }

    pub fn cycle_next(self) -> Self {
        match self {
            Self::MaxAriPerDollar => Self::BalancedCostCarbon,
            Self::BalancedCostCarbon => Self::MinEmbodiedCarbon,
            Self::MinEmbodiedCarbon => Self::MaxDistrictResilience,
            Self::MaxDistrictResilience => Self::MaxAriPerDollar,
        }
    }

    pub fn from_arg(arg: &str) -> Self {
        let lower = arg.trim().to_ascii_lowercase();
        if lower.contains("carbon") || lower.contains("co2") || lower.contains("green") {
            Self::MinEmbodiedCarbon
        } else if lower.contains("dollar") || lower.contains("cost") || lower.contains("usd") {
            Self::MaxAriPerDollar
        } else if lower.contains("max") || lower.contains("resil") || lower.contains("safe") {
            Self::MaxDistrictResilience
        } else {
            Self::BalancedCostCarbon
        }
    }
}

fn default_budget_cap_usd() -> f64 {
    250_000.0
}

fn default_carbon_cap_kg() -> f64 {
    120_000.0
}

fn default_target_min_ari() -> f64 {
    0.75
}

/// Persisted portfolio optimization constraints & active frontier selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioRetrofitConfig {
    #[serde(default = "default_budget_cap_usd")]
    pub budget_cap_usd: f64,
    #[serde(default = "default_carbon_cap_kg")]
    pub carbon_cap_kg_co2e: f64,
    #[serde(default = "default_target_min_ari")]
    pub target_min_ari: f64,
    #[serde(default)]
    pub objective: PortfolioOptimizationObjective,
    #[serde(default)]
    pub active_frontier_index: usize,
    #[serde(default)]
    pub overlay_visible: bool,
}

impl Default for PortfolioRetrofitConfig {
    fn default() -> Self {
        Self {
            budget_cap_usd: default_budget_cap_usd(),
            carbon_cap_kg_co2e: default_carbon_cap_kg(),
            target_min_ari: default_target_min_ari(),
            objective: PortfolioOptimizationObjective::default(),
            active_frontier_index: 0,
            overlay_visible: false,
        }
    }
}

impl PortfolioRetrofitConfig {
    pub fn content_revision_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mix = |acc: &mut u64, v: u64| {
            *acc ^= v;
            *acc = acc.wrapping_mul(0x0000_0100_0000_01b3);
        };
        mix(&mut h, self.budget_cap_usd.round() as u64);
        mix(&mut h, self.carbon_cap_kg_co2e.round() as u64);
        mix(&mut h, (self.target_min_ari * 1000.0).round() as u64);
        mix(&mut h, self.objective as u64 + 1);
        mix(&mut h, self.active_frontier_index as u64 + 11);
        mix(&mut h, u64::from(self.overlay_visible));
        h
    }
}

/// Selected retrofit decision for one building inside a district Pareto portfolio point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioBuildingDecision {
    pub building_id: u64,
    pub rel_path: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub height_m: f64,
    pub story_count: usize,
    pub member_count: usize,
    pub footprint_local_m: Vec<[f64; 2]>,
    pub heading_deg: f64,
    pub pre_min_ari: f64,
    pub pre_max_dcr: f64,
    pub selected_strategy: Option<RetrofitStrategy>,
    pub post_min_ari: f64,
    pub post_max_dcr: f64,
    pub estimated_cost_usd: f64,
    pub embodied_carbon_kg_co2e: f64,
    pub priority_rank: usize,
    pub rationale: String,
}

/// Non-dominated Pareto frontier operating point for district retrofit allocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioParetoPoint {
    pub frontier_id: usize,
    pub label: String,
    pub retrofitted_buildings: usize,
    pub total_buildings: usize,
    pub total_cost_usd: f64,
    pub total_carbon_kg_co2e: f64,
    pub district_min_ari_after: f64,
    pub district_avg_ari_after: f64,
    pub critical_buildings_remaining: usize,
    pub budget_utilization_pct: f64,
    pub carbon_utilization_pct: f64,
    pub decisions: Vec<PortfolioBuildingDecision>,
}

/// Complete multi-building district retrofit Pareto optimization report + `.ifcx` overlay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DistrictPortfolioRetrofitReport {
    pub config: PortfolioRetrofitConfig,
    pub total_buildings_evaluated: usize,
    pub pre_district_min_ari: f64,
    pub pre_district_avg_ari: f64,
    pub pre_critical_buildings: usize,
    pub selected_frontier_idx: usize,
    pub pareto_frontier: Vec<PortfolioParetoPoint>,
    pub portfolio_overlay_ifcx: IfcxDocument,
    pub summary: String,
}

impl DistrictPortfolioRetrofitReport {
    pub fn active_frontier(&self) -> Option<&PortfolioParetoPoint> {
        self.pareto_frontier
            .get(self.selected_frontier_idx)
            .or_else(|| self.pareto_frontier.first())
    }
}

#[derive(Clone)]
struct DistrictCandidate {
    id: u64,
    rel_path: String,
    name: String,
    lat: f64,
    lon: f64,
    height_m: f64,
    story_count: usize,
    member_count: usize,
    footprint_local_m: Vec<[f64; 2]>,
    heading_deg: f64,
    pre_min_ari: f64,
    pre_max_dcr: f64,
}

#[derive(Clone, Copy)]
struct StrategyCandidateEval {
    strategy: RetrofitStrategy,
    cost_usd: f64,
    carbon_kg: f64,
    post_min_ari: f64,
    post_max_dcr: f64,
    ari_gain: f64,
}

fn polygon_area_m2(poly: &[[f64; 2]]) -> f64 {
    if poly.len() < 3 {
        return 180.0;
    }
    let mut sum = 0.0;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        sum += a[0] * b[1] - b[0] * a[1];
    }
    (sum.abs() * 0.5).max(64.0)
}

fn eval_building_strategies(
    b: &DistrictCandidate,
    target_min_ari: f64,
    pga_ss_g: f64,
) -> Vec<StrategyCandidateEval> {
    let area_m2 = polygon_area_m2(&b.footprint_local_m);
    let vuln = ((target_min_ari - b.pre_min_ari).max(0.0) + (b.pre_max_dcr - 0.85).max(0.0))
        .clamp(0.05, 0.90);
    let scale = (b.story_count.max(1) as f64 / 3.0).max(0.75)
        * (area_m2 / 180.0).clamp(0.75, 4.2)
        * (0.82 + 0.32 * pga_ss_g.clamp(0.2, 1.5))
        * (0.65 + vuln);

    let specs = [
        (RetrofitStrategy::BrbBracingOnly, 42_000.0, 11_200.0, 0.27, 0.68),
        (RetrofitStrategy::RcJacketingOnly, 35_000.0, 16_900.0, 0.22, 0.64),
        (RetrofitStrategy::ShearWallCore, 54_500.0, 23_400.0, 0.29, 0.56),
        (RetrofitStrategy::AutoBalanced, 50_500.0, 15_200.0, 0.34, 0.58),
    ];

    let head = (1.0 - b.pre_min_ari).clamp(0.12, 0.65);
    specs
        .into_iter()
        .map(|(strategy, base_usd, base_co2, boost_k, dcr_k)| {
            let post_min_ari = (b.pre_min_ari + head * (boost_k / 0.38)).clamp(b.pre_min_ari, 0.97);
            let post_max_dcr = (b.pre_max_dcr * dcr_k).clamp(0.32, b.pre_max_dcr);
            StrategyCandidateEval {
                strategy,
                cost_usd: (base_usd * scale).round(),
                carbon_kg: (base_co2 * scale).round(),
                post_min_ari,
                post_max_dcr,
                ari_gain: (post_min_ari - b.pre_min_ari).max(0.0),
            }
        })
        .collect()
}

fn assemble_district_candidates(
    world_index: &WorldSpatialIndex,
    active_project: &BimProject,
    active_rel_path: Option<&str>,
    active_members: &[StructuralMember],
    origin_lat_lon: (f64, f64),
) -> Vec<DistrictCandidate> {
    let active_ari_opt = (!active_members.is_empty())
        .then(|| active_members.iter().map(|m| m.ari).fold(1.0_f64, f64::min));
    let active_dcr_opt = (!active_members.is_empty()).then(|| {
        active_members
            .iter()
            .map(|m| m.dcr_interaction.max(m.utilization))
            .fold(0.0_f64, f64::max)
    });

    if world_index.buildings.len() >= 2 {
        return world_index
            .buildings
            .iter()
            .take(48)
            .map(|b| {
                let is_active = active_rel_path.is_some_and(|p| p == b.rel_path);
                let pre_min_ari = if is_active {
                    active_ari_opt.unwrap_or(b.min_ari).clamp(0.15, 0.99)
                } else {
                    b.min_ari.clamp(0.15, 0.99)
                };
                let pre_max_dcr = if is_active {
                    active_dcr_opt.unwrap_or(b.max_dcr).clamp(0.20, 2.20)
                } else {
                    b.max_dcr.clamp(0.20, 2.20)
                };
                DistrictCandidate {
                    id: b.id,
                    rel_path: b.rel_path.clone(),
                    name: b.name.clone(),
                    lat: b.lat,
                    lon: b.lon,
                    height_m: b.total_height_m.max(3.6),
                    story_count: b.story_count.max(1),
                    member_count: b.member_count.max(8),
                    footprint_local_m: if b.footprint_local_m.len() >= 3 {
                        b.footprint_local_m.clone()
                    } else {
                        vec![[-8.0, -6.0], [8.0, -6.0], [8.0, 6.0], [-8.0, 6.0]]
                    },
                    heading_deg: b.heading_deg,
                    pre_min_ari,
                    pre_max_dcr,
                }
            })
            .collect();
    }

    // Active project + 4 contextual district archetypes so optimization works even with an empty Vault
    let (active_fp, active_h, active_stories, active_mem_cnt) = if !active_project.is_empty() {
        let (bmin, bmax) = active_project
            .plan_extents_mm()
            .unwrap_or(([-7500.0, -6000.0], [7500.0, 6000.0]));
        let cx = 0.5 * (bmin[0] + bmax[0]) * 0.001;
        let cy = 0.5 * (bmin[1] + bmax[1]) * 0.001;
        let fp = vec![
            [bmin[0] * 0.001 - cx, bmin[1] * 0.001 - cy],
            [bmax[0] * 0.001 - cx, bmin[1] * 0.001 - cy],
            [bmax[0] * 0.001 - cx, bmax[1] * 0.001 - cy],
            [bmin[0] * 0.001 - cx, bmax[1] * 0.001 - cy],
        ];
        let h = active_project
            .levels
            .iter()
            .map(|l| l.elevation_mm * 0.001)
            .fold(3.6_f64, f64::max)
            .max(3.6);
        let stories = active_project.levels.len().saturating_sub(1).max(1);
        let mems = (active_project.columns.len() + active_project.runs.len()).max(12);
        (fp, h, stories, mems)
    } else {
        (
            vec![[-7.5, -6.0], [7.5, -6.0], [7.5, 6.0], [-7.5, 6.0]],
            10.8,
            3,
            28,
        )
    };

    let active_name = if active_project.project_name.trim().is_empty() {
        "Active Reference Building".to_string()
    } else {
        active_project.project_name.clone()
    };

    let mut list = vec![DistrictCandidate {
        id: 1,
        rel_path: active_rel_path
            .unwrap_or("active/reference_building.ifcx")
            .to_string(),
        name: active_name,
        lat: origin_lat_lon.0,
        lon: origin_lat_lon.1,
        height_m: active_h,
        story_count: active_stories,
        member_count: active_mem_cnt,
        footprint_local_m: active_fp,
        heading_deg: 0.0,
        pre_min_ari: active_ari_opt.unwrap_or(0.56).clamp(0.15, 0.99),
        pre_max_dcr: active_dcr_opt.unwrap_or(1.04).clamp(0.20, 2.20),
    }];

    let synth_specs = [
        (2, "district/civic_transit_hall.ifcx", "Civic Transit Hall", -48.0, 26.0, 14.4, 4, 38, 10.0, 8.0, 0.46, 1.14),
        (3, "district/coastal_clinic_tower.ifcx", "Coastal Clinic Tower", 52.0, 18.0, 21.6, 6, 54, 12.0, 9.5, 0.58, 0.98),
        (4, "district/timber_steel_school.ifcx", "Timber-Steel School", -42.0, -56.0, 10.8, 3, 28, 11.0, 7.0, 0.69, 0.87),
        (5, "district/perimeter_resilience_hub.ifcx", "Ductile Perimeter Hub", 58.0, -48.0, 18.0, 5, 44, 10.5, 9.0, 0.86, 0.68),
    ];

    for (id, rel, name, ex, ny, h, st, mc, hw, hd, ari, dcr) in synth_specs {
        let (lat, lon) = SlippyMapState::world_mm_to_lat_lon(origin_lat_lon, [ex * 1000.0, ny * 1000.0]);
        list.push(DistrictCandidate {
            id,
            rel_path: rel.to_string(),
            name: name.to_string(),
            lat,
            lon,
            height_m: h,
            story_count: st,
            member_count: mc,
            footprint_local_m: vec![[-hw, -hd], [hw, -hd], [hw, hd], [-hw, hd]],
            heading_deg: 0.0,
            pre_min_ari: ari,
            pre_max_dcr: dcr,
        });
    }
    list
}

fn solve_scalarized_portfolio(
    candidates: &[DistrictCandidate],
    strats_per_bldg: &[Vec<StrategyCandidateEval>],
    config: &PortfolioRetrofitConfig,
    budget_limit: f64,
    carbon_limit: f64,
    carbon_weight: f64,
) -> (Vec<Option<usize>>, f64, f64, f64, f64, usize) {
    let n = candidates.len();
    let mut chosen: Vec<Option<usize>> = vec![None; n];
    let mut used_usd = 0.0;
    let mut used_co2 = 0.0;

    // Score best upgrade option per building with priority for lowest ARI / highest ARI gain per penalized cost
    let mut ranked: Vec<(usize, usize, f64)> = Vec::new();
    for (i, b) in candidates.iter().enumerate() {
        let needs_fix = b.pre_min_ari < config.target_min_ari || b.pre_max_dcr > 0.85;
        if !needs_fix {
            continue;
        }
        let urgency = 1.0 + (config.target_min_ari - b.pre_min_ari).max(0.0) * 3.5;
        if let Some((best_s_idx, _ev, score)) = strats_per_bldg[i]
            .iter()
            .enumerate()
            .map(|(s_idx, ev)| {
                let denom = (ev.cost_usd / 10_000.0) + carbon_weight * (ev.carbon_kg / 5_000.0);
                let sc = (ev.ari_gain * urgency * 100.0) / denom.max(0.25);
                (s_idx, ev, sc)
            })
            .max_by(|a, b| a.2.total_cmp(&b.2))
        {
            ranked.push((i, best_s_idx, score));
        }
    }
    ranked.sort_by(|a, b| b.2.total_cmp(&a.2));

    for &(b_idx, s_idx, _) in &ranked {
        let ev = strats_per_bldg[b_idx][s_idx];
        if used_usd + ev.cost_usd <= budget_limit + 1e-6
            && used_co2 + ev.carbon_kg <= carbon_limit + 1e-6
        {
            chosen[b_idx] = Some(s_idx);
            used_usd += ev.cost_usd;
            used_co2 += ev.carbon_kg;
        }
    }

    // Try greedy strategy substitution on selected buildings if upgrades fit inside remaining headroom
    for &(b_idx, _, _) in &ranked {
        if let Some(cur_s) = chosen[b_idx] {
            let cur_ev = strats_per_bldg[b_idx][cur_s];
            for (alt_s, alt_ev) in strats_per_bldg[b_idx].iter().enumerate() {
                if alt_ev.post_min_ari > cur_ev.post_min_ari + 0.02
                    && carbon_weight < 0.8
                    && (used_usd - cur_ev.cost_usd + alt_ev.cost_usd) <= budget_limit + 1e-6
                    && (used_co2 - cur_ev.carbon_kg + alt_ev.carbon_kg) <= carbon_limit + 1e-6
                {
                    used_usd = used_usd - cur_ev.cost_usd + alt_ev.cost_usd;
                    used_co2 = used_co2 - cur_ev.carbon_kg + alt_ev.carbon_kg;
                    chosen[b_idx] = Some(alt_s);
                    break;
                }
            }
        }
    }

    let mut min_ari = 1.0_f64;
    let mut sum_ari = 0.0_f64;
    let mut crit_rem = 0usize;
    for (i, b) in candidates.iter().enumerate() {
        let (post_ari, post_dcr) = match chosen[i] {
            Some(s) => (
                strats_per_bldg[i][s].post_min_ari,
                strats_per_bldg[i][s].post_max_dcr,
            ),
            None => (b.pre_min_ari, b.pre_max_dcr),
        };
        min_ari = min_ari.min(post_ari);
        sum_ari += post_ari;
        if post_ari < 0.70 || post_dcr > 1.0 {
            crit_rem += 1;
        }
    }
    let avg_ari = sum_ari / candidates.len().max(1) as f64;
    (chosen, used_usd, used_co2, min_ari, avg_ari, crit_rem)
}

fn build_portfolio_ifcx_document(
    config: &PortfolioRetrofitConfig,
    pre_avg_ari: f64,
    pre_min_ari: f64,
    active_point: Option<&PortfolioParetoPoint>,
    summary: &str,
) -> IfcxDocument {
    let mut doc = IfcxDocument::new("eigengraph_district_portfolio_retrofit");
    let total_cost = active_point.map(|p| p.total_cost_usd).unwrap_or(0.0);
    let total_co2 = active_point.map(|p| p.total_carbon_kg_co2e).unwrap_or(0.0);
    let post_avg = active_point.map(|p| p.district_avg_ari_after).unwrap_or(pre_avg_ari);
    let post_min = active_point.map(|p| p.district_min_ari_after).unwrap_or(pre_min_ari);

    let mut plan_node = IfcxNode::new("/earth/portfolio/retrofit_plan")
        .with_class("IfcSpatialZone")
        .with_prop(
            "District Portfolio ARI v2 Retrofit Plan",
            "Multi-Objective Cost/Carbon Pareto Optimization",
            summary,
        )
        .with_attr(
            NS_EG_PORTFOLIO_RETROFIT,
            json!({
                "objective": config.objective.short_label(),
                "budgetCapUsd": config.budget_cap_usd,
                "carbonCapKgCo2e": config.carbon_cap_kg_co2e,
                "targetMinAri": config.target_min_ari,
                "preDistrictMinAri": pre_min_ari,
                "preDistrictAvgAri": pre_avg_ari,
                "postDistrictMinAri": post_min,
                "postDistrictAvgAri": post_avg,
                "totalCostUsd": total_cost,
                "totalCarbonKgCo2e": total_co2,
            }),
        );

    let mut child_nodes = Vec::new();
    if let Some(pt) = active_point {
        for d in &pt.decisions {
            let slug = sanitize_slug(&d.name);
            let b_path = format!("/earth/portfolio/buildings/{slug}_{}", d.building_id);
            plan_node.children.insert(slug.clone(), b_path.clone());
            child_nodes.push(
                IfcxNode::new(b_path)
                    .with_class("IfcBuilding")
                    .with_prop(
                        &d.name,
                        d.selected_strategy
                            .map(|s| s.label())
                            .unwrap_or("Deferred / Intact Baseline"),
                        &d.rationale,
                    )
                    .with_attr(
                        ifcx_ns::EG_GEO,
                        json!({
                            "lat": d.lat,
                            "lon": d.lon,
                            "elevM": 0.0,
                            "headingDeg": d.heading_deg,
                        }),
                    )
                    .with_attr(
                        NS_EG_PORTFOLIO_RETROFIT,
                        json!({
                            "buildingId": d.building_id,
                            "strategy": d.selected_strategy.map(|s| s.short_label()).unwrap_or("None"),
                            "preMinAri": d.pre_min_ari,
                            "postMinAri": d.post_min_ari,
                            "preMaxDcr": d.pre_max_dcr,
                            "postMaxDcr": d.post_max_dcr,
                            "costUsd": d.estimated_cost_usd,
                            "carbonKgCo2e": d.embodied_carbon_kg_co2e,
                            "priorityRank": d.priority_rank,
                        }),
                    ),
            );
        }
    }

    doc.add_node(plan_node);
    for cn in child_nodes {
        doc.add_node(cn);
    }
    doc
}

/// Evaluates autonomous portfolio-scale `ARI v2` retrofit optimization across all buildings
/// in the district and constructs a non-dominated Cost vs. Embodied Carbon vs. ARI Pareto frontier.
pub fn optimize_district_portfolio_retrofit(
    world_index: &WorldSpatialIndex,
    active_project: &BimProject,
    active_rel_path: Option<&str>,
    active_members: &[StructuralMember],
    origin_lat_lon: (f64, f64),
    site: &SiteEnvironmentalCoupling,
    config: &PortfolioRetrofitConfig,
) -> DistrictPortfolioRetrofitReport {
    let candidates = assemble_district_candidates(
        world_index,
        active_project,
        active_rel_path,
        active_members,
        origin_lat_lon,
    );
    let n = candidates.len();
    let pre_district_min_ari = candidates
        .iter()
        .map(|b| b.pre_min_ari)
        .fold(1.0_f64, f64::min);
    let pre_district_avg_ari = candidates.iter().map(|b| b.pre_min_ari).sum::<f64>() / n.max(1) as f64;
    let pre_critical_buildings = candidates
        .iter()
        .filter(|b| b.pre_min_ari < 0.70 || b.pre_max_dcr > 1.0)
        .count();

    let pga_ss_g = site.seismic_pga_ss_g as f64;
    let strats_per_bldg: Vec<Vec<StrategyCandidateEval>> = candidates
        .iter()
        .map(|b| eval_building_strategies(b, config.target_min_ari, pga_ss_g))
        .collect();

    // Sweep 12 budget/carbon scalarizations + 1 unconstrained Full Shield sweep
    let mut raw_sols: Vec<(Vec<Option<usize>>, f64, f64, f64, f64, usize)> = Vec::new();
    for b_mult in [0.40_f64, 0.70, 1.00, 1.35] {
        for c_wt in [0.10_f64, 0.60, 1.40] {
            let sol = solve_scalarized_portfolio(
                &candidates,
                &strats_per_bldg,
                config,
                (config.budget_cap_usd * b_mult).max(20_000.0),
                (config.carbon_cap_kg_co2e * (b_mult + 0.15)).max(15_000.0),
                c_wt,
            );
            raw_sols.push(sol);
        }
    }
    raw_sols.push(solve_scalarized_portfolio(
        &candidates,
        &strats_per_bldg,
        config,
        1.0e9,
        1.0e9,
        0.10,
    ));

    // Deduplicate identical decision vectors
    let mut unique: Vec<(Vec<Option<usize>>, f64, f64, f64, f64, usize)> = Vec::new();
    for sol in raw_sols {
        if !unique.iter().any(|u| u.0 == sol.0) {
            unique.push(sol);
        }
    }

    // Prune strictly dominated portfolios across (cost, carbon, avg_ari, min_ari)
    let mut nondom: Vec<(Vec<Option<usize>>, f64, f64, f64, f64, usize)> = Vec::new();
    for (idx, a) in unique.iter().enumerate() {
        let dominated = unique.iter().enumerate().any(|(j, b)| {
            if idx == j {
                return false;
            }
            let le = b.1 <= a.1 + 1e-6
                && b.2 <= a.2 + 1e-6
                && b.4 >= a.4 - 1e-6
                && b.3 >= a.3 - 1e-6;
            let strict = b.1 < a.1 - 1.0
                || b.2 < a.2 - 1.0
                || b.4 > a.4 + 1e-4
                || b.3 > a.3 + 1e-4;
            le && strict
        });
        if !dominated {
            nondom.push(a.clone());
        }
    }
    if nondom.is_empty() {
        nondom = unique;
    }
    nondom.sort_by(|a, b| a.1.total_cmp(&b.1));

    let mut pareto_frontier = Vec::with_capacity(nondom.len());
    for (fid, (chosen, tot_usd, tot_co2, min_ari, avg_ari, crit_rem)) in nondom.iter().enumerate() {
        let mut decisions: Vec<PortfolioBuildingDecision> = candidates
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let (strat, post_ari, post_dcr, usd, co2, rationale) = match chosen[i] {
                    Some(s_idx) => {
                        let ev = strats_per_bldg[i][s_idx];
                        (
                            Some(ev.strategy),
                            ev.post_min_ari,
                            ev.post_max_dcr,
                            ev.cost_usd,
                            ev.carbon_kg,
                            format!(
                                "{}: ARI {:.2}->{:.2}, DCR {:.0}%->{:.0}%",
                                ev.strategy.short_label(),
                                b.pre_min_ari,
                                ev.post_min_ari,
                                b.pre_max_dcr * 100.0,
                                ev.post_max_dcr * 100.0
                            ),
                        )
                    }
                    None => (
                        None,
                        b.pre_min_ari,
                        b.pre_max_dcr,
                        0.0,
                        0.0,
                        if b.pre_min_ari >= config.target_min_ari && b.pre_max_dcr <= 0.85 {
                            "Meets ARI target (no intervention needed)".to_string()
                        } else {
                            "Deferred under budget/carbon constraint".to_string()
                        },
                    ),
                };
                PortfolioBuildingDecision {
                    building_id: b.id,
                    rel_path: b.rel_path.clone(),
                    name: b.name.clone(),
                    lat: b.lat,
                    lon: b.lon,
                    height_m: b.height_m,
                    story_count: b.story_count,
                    member_count: b.member_count,
                    footprint_local_m: b.footprint_local_m.clone(),
                    heading_deg: b.heading_deg,
                    pre_min_ari: b.pre_min_ari,
                    pre_max_dcr: b.pre_max_dcr,
                    selected_strategy: strat,
                    post_min_ari: post_ari,
                    post_max_dcr: post_dcr,
                    estimated_cost_usd: usd,
                    embodied_carbon_kg_co2e: co2,
                    priority_rank: 0,
                    rationale,
                }
            })
            .collect();

        // Assign priority rank (lowest pre_min_ari & highest ARI gain first)
        let mut rank_order: Vec<usize> = (0..decisions.len()).collect();
        rank_order.sort_by(|&ia, &ib| {
            let da = &decisions[ia];
            let db = &decisions[ib];
            let a_sel = u8::from(da.selected_strategy.is_some());
            let b_sel = u8::from(db.selected_strategy.is_some());
            b_sel
                .cmp(&a_sel)
                .then_with(|| da.pre_min_ari.total_cmp(&db.pre_min_ari))
        });
        for (rk, &idx) in rank_order.iter().enumerate() {
            decisions[idx].priority_rank = rk + 1;
        }

        let ret_cnt = chosen.iter().filter(|s| s.is_some()).count();
        let label = if *tot_usd < 1.0 {
            "Baseline ($0)".to_string()
        } else if fid == 0 {
            "Low-Carbon Lean".to_string()
        } else if fid + 1 == nondom.len() && *crit_rem == 0 {
            "Full District Shield".to_string()
        } else if *tot_usd <= config.budget_cap_usd {
            "Balanced Knee".to_string()
        } else {
            format!("Expanded Tier {}", fid + 1)
        };

        pareto_frontier.push(PortfolioParetoPoint {
            frontier_id: fid + 1,
            label,
            retrofitted_buildings: ret_cnt,
            total_buildings: n,
            total_cost_usd: *tot_usd,
            total_carbon_kg_co2e: *tot_co2,
            district_min_ari_after: *min_ari,
            district_avg_ari_after: *avg_ari,
            critical_buildings_remaining: *crit_rem,
            budget_utilization_pct: ((*tot_usd / config.budget_cap_usd.max(1.0)) * 100.0).min(999.0),
            carbon_utilization_pct: ((*tot_co2 / config.carbon_cap_kg_co2e.max(1.0)) * 100.0).min(999.0),
            decisions,
        });
    }

    // Select preferred frontier index according to budget feasibility & objective
    let feasible_indices: Vec<usize> = pareto_frontier
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            p.total_cost_usd <= config.budget_cap_usd + 1e-6
                && p.total_carbon_kg_co2e <= config.carbon_cap_kg_co2e + 1e-6
                && p.retrofitted_buildings > 0
        })
        .map(|(i, _)| i)
        .collect();
    let pool: Vec<usize> = if !feasible_indices.is_empty() {
        feasible_indices
    } else {
        (0..pareto_frontier.len()).collect()
    };

    let selected_frontier_idx = match config.objective {
        PortfolioOptimizationObjective::MaxAriPerDollar => *pool
            .iter()
            .max_by(|&&a, &&b| {
                let pa = &pareto_frontier[a];
                let pb = &pareto_frontier[b];
                let eff_a = (pa.district_avg_ari_after - pre_district_avg_ari)
                    / (pa.total_cost_usd / 100_000.0).max(0.1);
                let eff_b = (pb.district_avg_ari_after - pre_district_avg_ari)
                    / (pb.total_cost_usd / 100_000.0).max(0.1);
                eff_a.total_cmp(&eff_b)
            })
            .unwrap_or(&0),
        PortfolioOptimizationObjective::MinEmbodiedCarbon => *pool
            .iter()
            .min_by(|&&a, &&b| {
                pareto_frontier[a]
                    .total_carbon_kg_co2e
                    .total_cmp(&pareto_frontier[b].total_carbon_kg_co2e)
            })
            .unwrap_or(&0),
        PortfolioOptimizationObjective::MaxDistrictResilience => *pool
            .iter()
            .max_by(|&&a, &&b| {
                pareto_frontier[a]
                    .district_avg_ari_after
                    .total_cmp(&pareto_frontier[b].district_avg_ari_after)
            })
            .unwrap_or(&0),
        PortfolioOptimizationObjective::BalancedCostCarbon => {
            *pool.last().unwrap_or(&0)
        }
    };

    let active_pt = pareto_frontier.get(selected_frontier_idx);
    let summary = if let Some(pt) = active_pt {
        format!(
            "Portfolio {}/{} bldgs upgraded ({}) • Avg ARI {:.2}->{:.2} • Min ARI {:.2}->{:.2} • ${:.0}k • {:.1}tCO2e",
            pt.retrofitted_buildings,
            pt.total_buildings,
            pt.label,
            pre_district_avg_ari,
            pt.district_avg_ari_after,
            pre_district_min_ari,
            pt.district_min_ari_after,
            pt.total_cost_usd / 1000.0,
            pt.total_carbon_kg_co2e / 1000.0
        )
    } else {
        "District Portfolio intact".to_string()
    };

    let portfolio_overlay_ifcx = build_portfolio_ifcx_document(
        config,
        pre_district_avg_ari,
        pre_district_min_ari,
        active_pt,
        &summary,
    );

    DistrictPortfolioRetrofitReport {
        config: config.clone(),
        total_buildings_evaluated: n,
        pre_district_min_ari,
        pre_district_avg_ari,
        pre_critical_buildings,
        selected_frontier_idx,
        pareto_frontier,
        portfolio_overlay_ifcx,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ifcx::IfcxStage;

    #[test]
    fn test_phase73_portfolio_retrofit_pareto_frontier_and_ifcx_overlay() {
        let proj = BimProject::sample_bimcraft_building();
        let site = SiteEnvironmentalCoupling::default();
        let cfg = PortfolioRetrofitConfig {
            budget_cap_usd: 220_000.0,
            carbon_cap_kg_co2e: 95_000.0,
            target_min_ari: 0.78,
            objective: PortfolioOptimizationObjective::BalancedCostCarbon,
            active_frontier_index: 0,
            overlay_visible: true,
        };
        let rep = optimize_district_portfolio_retrofit(
            &WorldSpatialIndex::default(),
            &proj,
            None,
            &[],
            (-6.2088, 106.8456),
            &site,
            &cfg,
        );

        assert!(rep.total_buildings_evaluated >= 5);
        assert!(!rep.pareto_frontier.is_empty());

        // Verify non-dominance across every pair of Pareto points
        for (i, a) in rep.pareto_frontier.iter().enumerate() {
            for (j, b) in rep.pareto_frontier.iter().enumerate() {
                if i == j {
                    continue;
                }
                let b_dominates_a = b.total_cost_usd <= a.total_cost_usd + 1e-6
                    && b.total_carbon_kg_co2e <= a.total_carbon_kg_co2e + 1e-6
                    && b.district_avg_ari_after >= a.district_avg_ari_after - 1e-6
                    && b.district_min_ari_after >= a.district_min_ari_after - 1e-6
                    && (b.total_cost_usd < a.total_cost_usd - 1.0
                        || b.total_carbon_kg_co2e < a.total_carbon_kg_co2e - 1.0
                        || b.district_avg_ari_after > a.district_avg_ari_after + 1e-4
                        || b.district_min_ari_after > a.district_min_ari_after + 1e-4);
                assert!(
                    !b_dominates_a,
                    "Pareto frontier point {} is dominated by {}",
                    a.frontier_id, b.frontier_id
                );
            }
        }

        let active = rep.active_frontier().expect("Active frontier exists");
        assert!(active.retrofitted_buildings >= 1);
        assert!(active.district_avg_ari_after > rep.pre_district_avg_ari);
        assert!(active.total_cost_usd <= cfg.budget_cap_usd + 1e-6);

        let base_ifcx = IfcxDocument::from_bim_project(&proj, None, 0.0, None, None, None);
        let composed = IfcxStage::compose_with_portfolio_retrofit_overlay(&base_ifcx, &rep);
        assert!(composed
            .data
            .iter()
            .any(|n| n.attributes.contains_key(NS_EG_PORTFOLIO_RETROFIT)));

        let diff = IfcxStage::compute_visual_diff(&base_ifcx, "Base", &composed, "Portfolio");
        assert!(diff.layer_counts.contains_key("Portfolio Retrofit"));
    }
}
