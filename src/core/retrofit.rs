//! retrofit.rs — Automated Structural Retrofit & Non-Destructive `.ifcx` Strengthening Engine (v0.6.9).
//!
//! Synthesizes code-informed strengthening interventions from live site-coupled 6-DOF FEM &
//! EigenGate (`EigenGate`) screening:
//! - RC Column & Beam Jacketing (`+120 mm` columns, `+60x120 mm` beams) + Pad Footing Enlargement
//! - Steel Buckling-Restrained Braces (`BRB`) injected across low-EigenGate structural bays
//! - RC Structural Shear Wall Core Infill (`IfcWall`, `t = 250 mm`)
//! - Non-destructive IFCX Delta Layer (`eigengraph::bim::retrofit_delta`) ready for
//!   `IfcxStage::compose` stacking over base building `.ifcx` models.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::core::eigengate::{EigenGateEngine, MARGIN_TOL};
use crate::core::bim::{
    BimElementKind, BimProject, BimRun, BimWall, RunCategory, RunJustify,
};
use crate::core::ifcx::{ns, sanitize_slug, IfcxEigenGateOverlay, IfcxDocument, IfcxImport, IfcxNode, UsdGeomMesh};
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
use crate::core::solver::FemSolver;

/// Custom IFCX namespace storing non-destructive structural retrofit metadata & cost/carbon deltas.
pub const NS_EG_RETROFIT: &str = "eigengraph::bim::retrofit_delta";

/// Selectable structural retrofit intervention strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RetrofitStrategy {
    #[default]
    AutoBalanced,
    BrbBracingOnly,
    RcJacketingOnly,
    ShearWallCore,
}

impl RetrofitStrategy {
    pub fn short_label(self) -> &'static str {
        match self {
            Self::AutoBalanced => "Auto",
            Self::BrbBracingOnly => "BRB",
            Self::RcJacketingOnly => "Jacket",
            Self::ShearWallCore => "Wall",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::AutoBalanced => "Auto-Balanced (BRB + Jacket + ShearWall)",
            Self::BrbBracingOnly => "Steel Buckling-Restrained Braces (BRB)",
            Self::RcJacketingOnly => "RC Column & Beam Jacketing",
            Self::ShearWallCore => "RC Structural Shear Wall Infill",
        }
    }

    pub fn cycle_next(self) -> Self {
        match self {
            Self::AutoBalanced => Self::BrbBracingOnly,
            Self::BrbBracingOnly => Self::RcJacketingOnly,
            Self::RcJacketingOnly => Self::ShearWallCore,
            Self::ShearWallCore => Self::AutoBalanced,
        }
    }

    pub fn from_arg(arg: &str) -> Self {
        let lower = arg.trim().to_ascii_lowercase();
        if lower.contains("brb") || lower.contains("brace") {
            Self::BrbBracingOnly
        } else if lower.contains("jacket") || lower.contains("col") || lower.contains("rc") {
            Self::RcJacketingOnly
        } else if lower.contains("wall") || lower.contains("core") || lower.contains("shear") {
            Self::ShearWallCore
        } else {
            Self::AutoBalanced
        }
    }
}

/// Specific physical intervention applied to a BIM element during retrofit synthesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetrofitActionKind {
    ColumnJacketing,
    BeamStrengthening,
    BrbBraceAddition,
    ShearWallInfill,
    FootingEnlargement,
}

impl RetrofitActionKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::ColumnJacketing => "ColumnJacketing",
            Self::BeamStrengthening => "BeamStrengthening",
            Self::BrbBraceAddition => "BrbBraceAddition",
            Self::ShearWallInfill => "ShearWallInfill",
            Self::FootingEnlargement => "FootingEnlargement",
        }
    }
}

/// Per-element strengthening ledger emitted by [`synthesize_structural_retrofit`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrofitElementAction {
    pub kind: RetrofitActionKind,
    pub element_id: usize,
    pub element_name: String,
    pub story_level_id: usize,
    pub before_spec: String,
    pub after_spec: String,
    pub added_concrete_m3: f64,
    pub added_steel_kg: f64,
    pub cost_delta_usd: f64,
    pub carbon_delta_kg_co2e: f64,
}

/// Complete before/after 6-DOF verification report + composable `.ifcx` delta layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuralRetrofitReport {
    pub strategy: RetrofitStrategy,
    pub before_min_eigengate: f64,
    pub after_min_eigengate: f64,
    pub before_avg_eigengate: f64,
    pub after_avg_eigengate: f64,
    pub before_max_dcr_pct: f64,
    pub after_max_dcr_pct: f64,
    pub before_critical_count: usize,
    pub after_critical_count: usize,
    pub before_svd_mechanisms: usize,
    pub after_svd_mechanisms: usize,
    pub jacketed_columns: usize,
    pub strengthened_beams: usize,
    pub added_brb_braces: usize,
    pub added_shear_walls: usize,
    pub enlarged_footings: usize,
    pub added_concrete_m3: f64,
    pub added_steel_kg: f64,
    pub estimated_cost_usd: f64,
    pub embodied_carbon_kg_co2e: f64,
    pub pareto_efficiency_score: f64,
    pub actions: Vec<RetrofitElementAction>,
    pub retrofit_delta_ifcx: IfcxDocument,
    pub retrofitted_project: BimProject,
    pub summary: String,
}

#[derive(Debug, Clone, Copy)]
struct ElementHealthMetrics {
    min_eigengate: f64,
    avg_eigengate: f64,
    max_dcr_pct: f64,
    critical_count: usize,
    svd_mechanisms: usize,
}

fn evaluate_site_coupled_health(
    project: &BimProject,
    site_coupling: &SiteEnvironmentalCoupling,
) -> Result<
    (
        ElementHealthMetrics,
        Vec<(usize, BimElementKind, usize)>,
        HashMap<(BimElementKind, usize), (f64, f64)>,
    ),
    String,
> {
    let (fem, fem_map, _) = project.to_structural_model_with_site_loads(site_coupling);
    if fem.elements.is_empty() {
        return Err("No analytical 3D frame elements available to retrofit.".to_string());
    }
    let fea = FemSolver::solve(&fem)?;
    let eval = EigenGateEngine::evaluate_redundancy_fast(&fem)?;

    let mut fem_to_bim: HashMap<usize, (BimElementKind, usize)> =
        HashMap::with_capacity(fem_map.len());
    for &(fid, kind, bid) in &fem_map {
        fem_to_bim.insert(fid, (kind, bid));
    }
    let mut fem_to_pmv: HashMap<usize, f64> = HashMap::with_capacity(fem.elements.len());
    for (idx, el) in fem.elements.iter().enumerate() {
        let pmv = fea.member_dcr_interactions.get(idx).copied().unwrap_or(0.0);
        fem_to_pmv.insert(el.id, pmv);
    }

    let mut max_dcr = 0.0_f64;
    let mut svd_mechs = usize::from(!eval.kinematic_guard.is_stable);
    let mut bim_metrics: HashMap<(BimElementKind, usize), (f64, f64)> = HashMap::new();

    for res in &eval.member_results {
        let pmv = fem_to_pmv.get(&res.member_id).copied().unwrap_or(0.0);
        let eff_dcr = res.utilization_initial.max(pmv);
        max_dcr = max_dcr.max(eff_dcr * 100.0);
        if res.svd_margin <= MARGIN_TOL {
            svd_mechs += 1;
        }
        if let Some(&(kind, bid)) = fem_to_bim.get(&res.member_id) {
            let entry = bim_metrics.entry((kind, bid)).or_insert((1.0, 0.0));
            entry.0 = entry.0.min(res.eigengate);
            entry.1 = entry.1.max(eff_dcr);
        }
    }

    let metrics = ElementHealthMetrics {
        min_eigengate: eval.min_system_eigengate,
        avg_eigengate: eval.average_eigengate,
        max_dcr_pct: max_dcr,
        critical_count: eval.critical_members_count,
        svd_mechanisms: svd_mechs,
    };
    Ok((metrics, fem_map, bim_metrics))
}

fn sorted_levels_asc(project: &BimProject) -> Vec<(usize, f64)> {
    let mut lvls: Vec<(usize, f64)> = project
        .levels
        .iter()
        .map(|l| (l.id, l.elevation_mm))
        .collect();
    lvls.sort_by(|a, b| a.1.total_cmp(&b.1));
    lvls
}

fn level_elev_mm(project: &BimProject, level_id: usize) -> f64 {
    project
        .levels
        .iter()
        .find(|l| l.id == level_id)
        .map(|l| l.elevation_mm)
        .unwrap_or(0.0)
}

fn next_level_elev_mm(project: &BimProject, base_level_id: usize, fallback_h: f64) -> f64 {
    let z0 = level_elev_mm(project, base_level_id);
    project
        .levels
        .iter()
        .filter(|l| l.elevation_mm > z0 + 10.0)
        .map(|l| l.elevation_mm)
        .fold(f64::INFINITY, f64::min)
        .min(z0 + fallback_h)
}

fn rank_bottleneck_ids(
    all_ids: &[usize],
    kind: BimElementKind,
    bim_metrics: &HashMap<(BimElementKind, usize), (f64, f64)>,
) -> HashSet<usize> {
    if all_ids.is_empty() {
        return HashSet::new();
    }
    let mut scored: Vec<(usize, f64, bool)> = all_ids
        .iter()
        .map(|&id| {
            let (eigengate, dcr) = bim_metrics.get(&(kind, id)).copied().unwrap_or((0.85, 0.45));
            let is_bottleneck = eigengate < 0.80 || dcr > 0.70;
            let severity = dcr + (1.0 - eigengate).max(0.0);
            (id, severity, is_bottleneck)
        })
        .collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let flagged: HashSet<usize> = scored
        .iter()
        .filter(|&&(_, _, bottleneck)| bottleneck)
        .map(|&(id, _, _)| id)
        .collect();
    if !flagged.is_empty() {
        return flagged;
    }
    // Strengthen uniform baseline frames uniformly so no un-jacketed members cap min_eigengate
    all_ids.iter().copied().collect()
}

fn unit_economics(concrete_m3: f64, steel_kg: f64) -> (f64, f64) {
    // Market baseline: $165/m3 structural concrete + $2.10/kg fabricated steel/rebar
    let cost_usd = concrete_m3 * 165.0 + steel_kg * 2.10;
    // Embodied carbon (ICE v3): 320 kgCO2e/m3 concrete + 1.85 kgCO2e/kg steel
    let carbon_kg = concrete_m3 * 320.0 + steel_kg * 1.85;
    (cost_usd, carbon_kg)
}

fn segments_match_endpoints(a0: crate::core::P2, a1: crate::core::P2, b0: crate::core::P2, b1: crate::core::P2) -> bool {
    const TOL_MM: f64 = 150.0;
    (a0.dist(b0) <= TOL_MM && a1.dist(b1) <= TOL_MM)
        || (a0.dist(b1) <= TOL_MM && a1.dist(b0) <= TOL_MM)
}

fn apply_jacketing_pass(
    retro: &mut BimProject,
    bim_metrics: &HashMap<(BimElementKind, usize), (f64, f64)>,
    actions: &mut Vec<RetrofitElementAction>,
) -> (usize, usize, usize) {
    let col_ids: Vec<usize> = retro.columns.iter().map(|c| c.id).collect();
    let target_cols = rank_bottleneck_ids(&col_ids, BimElementKind::Column, bim_metrics);
    let mut jacketed_columns = 0usize;
    let mut enlarged_footings = 0usize;
    let mut jacketed_col_positions = Vec::new();

    for col in &mut retro.columns {
        if !target_cols.contains(&col.id) {
            continue;
        }
        let old_w = col.width_mm;
        let old_d = col.depth_mm;
        let new_w = (old_w + 150.0).min(1200.0);
        let new_d = (old_d + 150.0).min(1200.0);
        if new_w <= old_w + 1.0 && new_d <= old_d + 1.0 {
            continue;
        }
        let before_spec = if col.profile_name.trim().is_empty() {
            format!("{old_w:.0}x{old_d:.0} mm")
        } else {
            col.profile_name.clone()
        };
        col.width_mm = new_w;
        col.depth_mm = new_d;
        col.profile_name = format!("RC Jacket {new_w:.0}x{new_d:.0}");

        let z0 = retro
            .levels
            .iter()
            .find(|l| l.id == col.base_level_id)
            .map(|l| l.elevation_mm)
            .unwrap_or(0.0);
        let z1 = retro
            .levels
            .iter()
            .find(|l| l.id == col.top_level_id)
            .map(|l| l.elevation_mm)
            .unwrap_or(z0 + 3500.0);
        let h_m = ((z1 - z0).abs() * 0.001).max(2.5);
        let delta_a_m2 = ((new_w * new_d) - (old_w * old_d)).max(0.0) * 1e-6;
        let added_conc = delta_a_m2 * h_m;
        let added_steel = added_conc * 145.0;
        let (cost_usd, carbon_kg) = unit_economics(added_conc, added_steel);

        jacketed_columns += 1;
        jacketed_col_positions.push(col.pos);
        actions.push(RetrofitElementAction {
            kind: RetrofitActionKind::ColumnJacketing,
            element_id: col.id,
            element_name: col.name.clone(),
            story_level_id: col.base_level_id,
            before_spec,
            after_spec: col.profile_name.clone(),
            added_concrete_m3: added_conc,
            added_steel_kg: added_steel,
            cost_delta_usd: cost_usd,
            carbon_delta_kg_co2e: carbon_kg,
        });
    }

    // Enlarge any pad footings seated directly beneath jacketed columns
    for ftg in &mut retro.footings {
        if !jacketed_col_positions
            .iter()
            .any(|p| p.dist(ftg.pos) <= 600.0)
        {
            continue;
        }
        let old_w = ftg.width_mm;
        let old_l = ftg.length_mm;
        let old_t = ftg.thickness_mm;
        let new_w = (old_w + 300.0).min(8000.0);
        let new_l = (old_l + 300.0).min(8000.0);
        let new_t = (old_t + 100.0).min(2500.0);
        let before_spec = format!("{old_w:.0}x{old_l:.0}x{old_t:.0} mm");
        ftg.width_mm = new_w;
        ftg.length_mm = new_l;
        ftg.thickness_mm = new_t;
        let after_spec = format!("{new_w:.0}x{new_l:.0}x{new_t:.0} mm");

        let v_old = (old_w * old_l * old_t) * 1e-9;
        let v_new = (new_w * new_l * new_t) * 1e-9;
        let added_conc = (v_new - v_old).max(0.0);
        let added_steel = added_conc * 110.0;
        let (cost_usd, carbon_kg) = unit_economics(added_conc, added_steel);

        enlarged_footings += 1;
        actions.push(RetrofitElementAction {
            kind: RetrofitActionKind::FootingEnlargement,
            element_id: ftg.id,
            element_name: ftg.name.clone(),
            story_level_id: ftg.level_id,
            before_spec,
            after_spec,
            added_concrete_m3: added_conc,
            added_steel_kg: added_steel,
            cost_delta_usd: cost_usd,
            carbon_delta_kg_co2e: carbon_kg,
        });
    }

    // Strengthen bottleneck structural beams
    let beam_ids: Vec<usize> = retro
        .runs
        .iter()
        .filter(|r| r.category == RunCategory::StructuralBeam)
        .map(|r| r.id)
        .collect();
    let target_beams = rank_bottleneck_ids(&beam_ids, BimElementKind::Run, bim_metrics);
    let mut strengthened_beams = 0usize;

    for run in &mut retro.runs {
        if run.category != RunCategory::StructuralBeam || !target_beams.contains(&run.id) {
            continue;
        }
        let old_w = run.width_mm;
        let old_d = run.depth_mm;
        let new_w = (old_w + 80.0).min(1000.0);
        let new_d = (old_d + 150.0).min(1600.0);
        if new_w <= old_w + 1.0 && new_d <= old_d + 1.0 {
            continue;
        }
        let before_spec = if run.profile_name.trim().is_empty() {
            format!("{old_w:.0}x{old_d:.0} mm")
        } else {
            run.profile_name.clone()
        };
        run.width_mm = new_w;
        run.depth_mm = new_d;
        run.profile_name = if run.is_wf_profile {
            format!("WF Jacket {new_d:.0}x{new_w:.0}")
        } else {
            format!("RC Jacket {new_w:.0}x{new_d:.0}")
        };

        let len_m = (run.start.dist(run.end) * 0.001).max(1.0);
        let delta_a_m2 = ((new_w * new_d) - (old_w * old_d)).max(0.0) * 1e-6;
        let added_conc = delta_a_m2 * len_m;
        let added_steel = added_conc * 135.0;
        let (cost_usd, carbon_kg) = unit_economics(added_conc, added_steel);

        strengthened_beams += 1;
        actions.push(RetrofitElementAction {
            kind: RetrofitActionKind::BeamStrengthening,
            element_id: run.id,
            element_name: run.name.clone(),
            story_level_id: run.level_id,
            before_spec,
            after_spec: run.profile_name.clone(),
            added_concrete_m3: added_conc,
            added_steel_kg: added_steel,
            cost_delta_usd: cost_usd,
            carbon_delta_kg_co2e: carbon_kg,
        });
    }

    (jacketed_columns, strengthened_beams, enlarged_footings)
}

fn apply_brb_bracing_pass(
    retro: &mut BimProject,
    bim_metrics: &HashMap<(BimElementKind, usize), (f64, f64)>,
    actions: &mut Vec<RetrofitElementAction>,
) -> usize {
    let lvls = sorted_levels_asc(retro);
    if lvls.len() < 2 {
        return 0;
    }

    let mut added_braces = 0usize;
    for win in lvls.windows(2).take(2) {
        let (base_id, base_elev) = win[0];
        let (_top_id, top_elev) = win[1];
        let story_h_mm = (top_elev - base_elev).max(3000.0);

        let cols_on_story: Vec<_> = retro
            .columns
            .iter()
            .filter(|c| c.base_level_id == base_id)
            .cloned()
            .collect();
        if cols_on_story.len() < 2 {
            continue;
        }

        // Collect all valid column bay pairs (preferring 2.5m..=14m orthogonal spans, falling back up to 22m)
        let mut candidates: Vec<(usize, usize, f64, bool)> = Vec::new();
        for i in 0..cols_on_story.len() {
            for j in (i + 1)..cols_on_story.len() {
                let ca = &cols_on_story[i];
                let cb = &cols_on_story[j];
                let d_mm = ca.pos.dist(cb.pos);
                if !(2500.0..=14_000.0).contains(&d_mm) {
                    continue;
                }
                let already_braced = retro.runs.iter().any(|r| {
                    r.category == RunCategory::StructuralBrace
                        && r.level_id == base_id
                        && segments_match_endpoints(r.start, r.end, ca.pos, cb.pos)
                });
                if already_braced {
                    continue;
                }
                let dx = (cb.pos.0 - ca.pos.0).abs();
                let dy = (cb.pos.1 - ca.pos.1).abs();
                let is_orthogonal = dx <= 250.0 || dy <= 250.0;
                let (eigengate_a, dcr_a) = bim_metrics
                    .get(&(BimElementKind::Column, ca.id))
                    .copied()
                    .unwrap_or((0.85, 0.45));
                let (eigengate_b, dcr_b) = bim_metrics
                    .get(&(BimElementKind::Column, cb.id))
                    .copied()
                    .unwrap_or((0.85, 0.45));
                let severity = (1.0 - eigengate_a.min(eigengate_b)).max(0.0)
                    + dcr_a.max(dcr_b)
                    + if is_orthogonal { 0.35 } else { 0.0 };
                let x_major = dx >= dy;
                candidates.push((i, j, severity, x_major));
            }
        }
        candidates.sort_by(|a, b| b.2.total_cmp(&a.2));

        // Add opposing X-braced BRB pair on up to 1 X-major and 1 Y-major bay per story interval
        // so both tension and compression diagonals share lateral shear cleanly.
        let mut chosen_x = false;
        let mut chosen_y = false;
        for &(ia, ib, _, x_major) in &candidates {
            if x_major && chosen_x {
                continue;
            }
            if !x_major && chosen_y {
                continue;
            }
            let ca = &cols_on_story[ia];
            let cb = &cols_on_story[ib];
            let len_m = (ca.pos.dist(cb.pos).hypot(story_h_mm) * 0.001).max(3.5);
            let added_steel = len_m * 92.0;
            let added_conc = len_m * 0.04;
            let (cost_usd, carbon_kg) = unit_economics(added_conc, added_steel);

            for (start_pt, end_pt, tag) in [(ca.pos, cb.pos, "A"), (cb.pos, ca.pos, "B")] {
                let bid = retro.alloc_id();
                let brace_name = format!("Retrofit BRB-{bid}{tag}");
                retro.runs.push(BimRun {
                    id: bid,
                    name: brace_name.clone(),
                    category: RunCategory::StructuralBrace,
                    level_id: base_id,
                    start: start_pt,
                    end: end_pt,
                    start_offset_mm: 0.0,
                    end_offset_mm: story_h_mm,
                    justify: RunJustify::Centre,
                    profile_name: "BRB 250x250 Core".to_string(),
                    width_mm: 250.0,
                    depth_mm: 250.0,
                    is_round: false,
                    is_wf_profile: true,
                    system_name: "EigenAI BRB Retrofit".to_string(),
                });
                added_braces += 1;
                actions.push(RetrofitElementAction {
                    kind: RetrofitActionKind::BrbBraceAddition,
                    element_id: bid,
                    element_name: brace_name,
                    story_level_id: base_id,
                    before_spec: "Unbraced Bay".to_string(),
                    after_spec: "BRB 250x250 Core".to_string(),
                    added_concrete_m3: added_conc,
                    added_steel_kg: added_steel,
                    cost_delta_usd: cost_usd,
                    carbon_delta_kg_co2e: carbon_kg,
                });
            }

            if x_major {
                chosen_x = true;
            } else {
                chosen_y = true;
            }
            if chosen_x && chosen_y {
                break;
            }
        }
    }

    added_braces
}

fn apply_shear_wall_infill_pass(
    retro: &mut BimProject,
    bim_metrics: &HashMap<(BimElementKind, usize), (f64, f64)>,
    actions: &mut Vec<RetrofitElementAction>,
    max_per_axis: usize,
) -> usize {
    let lvls = sorted_levels_asc(retro);
    if lvls.len() < 2 {
        return 0;
    }
    let (base_id, _) = lvls[0];

    let cols_ground: Vec<_> = retro
        .columns
        .iter()
        .filter(|c| c.base_level_id == base_id)
        .cloned()
        .collect();
    if cols_ground.len() < 2 {
        return 0;
    }

    let mut candidates: Vec<(usize, usize, f64, bool)> = Vec::new();
    for i in 0..cols_ground.len() {
        for j in (i + 1)..cols_ground.len() {
            let ca = &cols_ground[i];
            let cb = &cols_ground[j];
            let d_mm = ca.pos.dist(cb.pos);
            if !(2200.0..=14_000.0).contains(&d_mm) {
                continue;
            }
            let has_wall = retro.walls.iter().any(|w| {
                w.is_structural
                    && w.base_level_id == base_id
                    && segments_match_endpoints(w.start, w.end, ca.pos, cb.pos)
            });
            let has_brace = retro.runs.iter().any(|r| {
                r.category == RunCategory::StructuralBrace
                    && r.level_id == base_id
                    && segments_match_endpoints(r.start, r.end, ca.pos, cb.pos)
            });
            if has_wall || has_brace {
                continue;
            }
            let dx = (cb.pos.0 - ca.pos.0).abs();
            let dy = (cb.pos.1 - ca.pos.1).abs();
            let orthogonal_bonus = if dx <= 250.0 || dy <= 250.0 { 0.5 } else { 0.0 };
            let (eigengate_a, dcr_a) = bim_metrics
                .get(&(BimElementKind::Column, ca.id))
                .copied()
                .unwrap_or((0.85, 0.45));
            let (eigengate_b, dcr_b) = bim_metrics
                .get(&(BimElementKind::Column, cb.id))
                .copied()
                .unwrap_or((0.85, 0.45));
            let score = (1.0 - eigengate_a.min(eigengate_b)).max(0.0) + dcr_a.max(dcr_b) + orthogonal_bonus;
            candidates.push((i, j, score, dx >= dy));
        }
    }
    candidates.sort_by(|a, b| b.2.total_cmp(&a.2));

    let mut added = 0usize;
    let mut selected_bays: Vec<(crate::core::P2, crate::core::P2)> = Vec::new();
    let mut chosen_x = 0usize;
    let mut chosen_y = 0usize;
    for &(ia, ib, _, x_major) in &candidates {
        if x_major && chosen_x >= max_per_axis {
            continue;
        }
        if !x_major && chosen_y >= max_per_axis {
            continue;
        }
        selected_bays.push((cols_ground[ia].pos, cols_ground[ib].pos));
        if x_major {
            chosen_x += 1;
        } else {
            chosen_y += 1;
        }
        if chosen_x >= max_per_axis && chosen_y >= max_per_axis {
            break;
        }
    }

    // Continue shear wall core vertically across up to 2 stories to avoid soft-story stiffness discontinuities
    for win in lvls.windows(2).take(2) {
        let (s_base_id, s_base_elev) = win[0];
        let (s_top_id, s_top_elev) = win[1];
        let s_h_m = ((s_top_elev - s_base_elev) * 0.001).max(3.0);
        for &(p_start, p_end) in &selected_bays {
            let has_wall = retro.walls.iter().any(|w| {
                w.is_structural
                    && w.base_level_id == s_base_id
                    && segments_match_endpoints(w.start, w.end, p_start, p_end)
            });
            let has_brace = retro.runs.iter().any(|r| {
                r.category == RunCategory::StructuralBrace
                    && r.level_id == s_base_id
                    && segments_match_endpoints(r.start, r.end, p_start, p_end)
            });
            if has_wall || has_brace {
                continue;
            }
            let wid = retro.alloc_id();
            let wall_name = format!("Retrofit ShearWall SW-{wid}");
            let thickness_mm = 300.0;
            let len_m = (p_start.dist(p_end) * 0.001).max(2.0);
            let added_conc = len_m * (thickness_mm * 0.001) * s_h_m;
            let added_steel = added_conc * 125.0;
            let (cost_usd, carbon_kg) = unit_economics(added_conc, added_steel);

            retro.walls.push(BimWall {
                id: wid,
                name: wall_name.clone(),
                base_level_id: s_base_id,
                top_level_id: Some(s_top_id),
                unconnected_height_mm: s_h_m * 1000.0,
                start: p_start,
                end: p_end,
                thickness_mm,
                is_structural: true,
                layers: BimWall::default_compound_layers(thickness_mm),
                openings: Vec::new(),
            });
            actions.push(RetrofitElementAction {
                kind: RetrofitActionKind::ShearWallInfill,
                element_id: wid,
                element_name: wall_name,
                story_level_id: s_base_id,
                before_spec: "Open Frame Bay".to_string(),
                after_spec: format!("RC Shear Wall t={thickness_mm:.0}mm"),
                added_concrete_m3: added_conc,
                added_steel_kg: added_steel,
                cost_delta_usd: cost_usd,
                carbon_delta_kg_co2e: carbon_kg,
            });
            added += 1;
        }
    }
    added
}

fn build_retrofit_delta_ifcx(
    retro: &BimProject,
    strategy: RetrofitStrategy,
    before: ElementHealthMetrics,
    after: ElementHealthMetrics,
    actions: &[RetrofitElementAction],
    counts: (usize, usize, usize, usize, usize),
    totals: (f64, f64, f64, f64, f64),
) -> IfcxDocument {
    let (jacketed_cols, strengthened_beams, added_brbs, added_walls, enlarged_ftgs) = counts;
    let (added_conc_m3, added_steel_kg, cost_usd, carbon_kg, pareto_score) = totals;
    let slug = sanitize_slug(&retro.project_name);
    let site_path = format!("/earth/sites/{slug}");
    let bldg_path = format!("{site_path}/buildings/{slug}");
    let delta_path = format!("{bldg_path}/retrofit/delta_v1");

    let mut doc = IfcxDocument::new(format!("retrofit_delta_{slug}"));
    doc.imports.push(IfcxImport {
        uri: format!("eigengraph://sites/{slug}.ifcx"),
        integrity: None,
        role: Some("base-building".to_string()),
    });

    let mut bldg_node = IfcxNode::new(&bldg_path).with_class("IfcBuilding");
    bldg_node
        .children
        .insert("retrofit_delta".to_string(), delta_path.clone());
    if let Ok(v) = serde_json::to_value(retro) {
        bldg_node
            .attributes
            .insert(ns::EG_BIM_PROJECT.to_string(), v);
    }

    let eigengate_overlay = IfcxEigenGateOverlay {
        eigengate_system: after.min_eigengate,
        eigengate_average: after.avg_eigengate,
        critical_members_count: after.critical_count,
        max_dcr_pct: after.max_dcr_pct,
        svd_rank_deficiency: after.svd_mechanisms,
        is_kinematically_stable: after.svd_mechanisms == 0,
    };
    if let Ok(v) = serde_json::to_value(&eigengate_overlay) {
        bldg_node.attributes.insert(ns::EG_EIGENGATE.to_string(), v);
    }

    let delta_summary_json = json!({
        "strategy": strategy.label(),
        "beforeMinEigenGate": before.min_eigengate,
        "afterMinEigenGate": after.min_eigengate,
        "beforeMaxDcrPct": before.max_dcr_pct,
        "afterMaxDcrPct": after.max_dcr_pct,
        "beforeCriticalCount": before.critical_count,
        "afterCriticalCount": after.critical_count,
        "beforeSvdMechanisms": before.svd_mechanisms,
        "afterSvdMechanisms": after.svd_mechanisms,
        "jacketedColumns": jacketed_cols,
        "strengthenedBeams": strengthened_beams,
        "addedBrbBraces": added_brbs,
        "addedShearWalls": added_walls,
        "enlargedFootings": enlarged_ftgs,
        "addedConcreteM3": added_conc_m3,
        "addedSteelKg": added_steel_kg,
        "estimatedCostUsd": cost_usd,
        "embodiedCarbonKgCo2e": carbon_kg,
        "paretoEfficiencyScore": pareto_score,
        "actionCount": actions.len(),
    });
    bldg_node
        .attributes
        .insert(NS_EG_RETROFIT.to_string(), delta_summary_json.clone());
    doc.add_node(bldg_node);

    let mut delta_node = IfcxNode::new(&delta_path)
        .with_class("IfcTask")
        .with_prop(
            "Automated Structural Retrofit Delta",
            strategy.label(),
            &format!(
                "EigenGate {:.3}->{:.3}, DCR {:.1}%->{:.1}%, Cost ${:.0}, Carbon {:.0} kgCO2e",
                before.min_eigengate,
                after.min_eigengate,
                before.max_dcr_pct,
                after.max_dcr_pct,
                cost_usd,
                carbon_kg
            ),
        )
        .with_attr(NS_EG_RETROFIT, delta_summary_json);
    if let Ok(v) = serde_json::to_value(&eigengate_overlay) {
        delta_node.attributes.insert(ns::EG_EIGENGATE.to_string(), v);
    }
    doc.add_node(delta_node);

    for act in actions {
        let action_payload = json!({
            "actionKind": act.kind.label(),
            "elementId": act.element_id,
            "beforeSpec": act.before_spec,
            "afterSpec": act.after_spec,
            "addedConcreteM3": act.added_concrete_m3,
            "addedSteelKg": act.added_steel_kg,
            "costDeltaUsd": act.cost_delta_usd,
            "carbonDeltaKgCo2e": act.carbon_delta_kg_co2e,
        });
        match act.kind {
            RetrofitActionKind::ColumnJacketing => {
                if let Some(col) = retro.columns.iter().find(|c| c.id == act.element_id) {
                    let z0 = level_elev_mm(retro, col.base_level_id);
                    let z1 = retro
                        .levels
                        .iter()
                        .find(|l| l.id == col.top_level_id)
                        .map(|l| l.elevation_mm)
                        .unwrap_or_else(|| next_level_elev_mm(retro, col.base_level_id, 3500.0));
                    let tris = col.mesh_3d(z0, z1.max(z0 + 300.0));
                    let mesh = UsdGeomMesh::from_triangles_mm(&tris, [0.22, 0.82, 0.68, 1.0], 2.0);
                    let path = format!(
                        "{bldg_path}/storeys/L{}/columns/col_{}",
                        col.base_level_id, col.id
                    );
                    doc.add_node(
                        IfcxNode::new(path)
                            .with_class("IfcColumn")
                            .with_prop(&col.name, &col.profile_name, "Retrofitted RC Jacketed Column")
                            .with_attr(
                                ns::EG_BIM_ELEMENT,
                                json!({
                                    "kind": "column",
                                    "id": col.id,
                                    "widthMm": col.width_mm,
                                    "depthMm": col.depth_mm,
                                    "profile": col.profile_name
                                }),
                            )
                            .with_attr(NS_EG_RETROFIT, action_payload)
                            .with_mesh(&mesh),
                    );
                }
            }
            RetrofitActionKind::BeamStrengthening | RetrofitActionKind::BrbBraceAddition => {
                if let Some(run) = retro.runs.iter().find(|r| r.id == act.element_id) {
                    let z_lvl = level_elev_mm(retro, run.level_id);
                    let tris = run.mesh_3d(z_lvl);
                    let (class_code, color, desc) = if run.category == RunCategory::StructuralBrace {
                        ("IfcMember", [0.96, 0.68, 0.16, 1.0], "Buckling-Restrained Brace (BRB)")
                    } else {
                        ("IfcBeam", [0.22, 0.82, 0.68, 1.0], "Retrofitted Structural Beam")
                    };
                    let mesh = UsdGeomMesh::from_triangles_mm(&tris, color, 2.0);
                    let path = format!("{bldg_path}/storeys/L{}/runs/run_{}", run.level_id, run.id);
                    doc.add_node(
                        IfcxNode::new(path)
                            .with_class(class_code)
                            .with_prop(&run.name, &run.profile_name, desc)
                            .with_attr(
                                ns::EG_BIM_ELEMENT,
                                json!({
                                    "kind": "run",
                                    "id": run.id,
                                    "widthMm": run.width_mm,
                                    "depthMm": run.depth_mm,
                                    "profile": run.profile_name
                                }),
                            )
                            .with_attr(NS_EG_RETROFIT, action_payload)
                            .with_mesh(&mesh),
                    );
                }
            }
            RetrofitActionKind::ShearWallInfill => {
                if let Some(wall) = retro.walls.iter().find(|w| w.id == act.element_id) {
                    let z0 = level_elev_mm(retro, wall.base_level_id);
                    let z1 = wall
                        .top_level_id
                        .and_then(|tid| retro.levels.iter().find(|l| l.id == tid).map(|l| l.elevation_mm))
                        .unwrap_or(z0 + wall.unconnected_height_mm.max(2400.0));
                    let tris = wall.mesh_3d(z0, z1.max(z0 + 300.0));
                    let mesh = UsdGeomMesh::from_triangles_mm(&tris, [0.22, 0.82, 0.68, 1.0], 2.0);
                    let path = format!(
                        "{bldg_path}/storeys/L{}/walls/wall_{}",
                        wall.base_level_id, wall.id
                    );
                    doc.add_node(
                        IfcxNode::new(path)
                            .with_class("IfcWall")
                            .with_prop(
                                &wall.name,
                                "Retrofitted RC Structural Shear Wall",
                                &format!("t = {:.0} mm", wall.thickness_mm),
                            )
                            .with_attr(
                                ns::EG_BIM_ELEMENT,
                                json!({
                                    "kind": "wall",
                                    "id": wall.id,
                                    "thicknessMm": wall.thickness_mm,
                                    "isStructural": wall.is_structural
                                }),
                            )
                            .with_attr(NS_EG_RETROFIT, action_payload)
                            .with_mesh(&mesh),
                    );
                }
            }
            RetrofitActionKind::FootingEnlargement => {
                if let Some(ftg) = retro.footings.iter().find(|f| f.id == act.element_id) {
                    let z_lvl = level_elev_mm(retro, ftg.level_id);
                    let tris = ftg.mesh_3d(z_lvl);
                    let mesh = UsdGeomMesh::from_triangles_mm(&tris, [0.36, 0.72, 0.66, 1.0], 2.0);
                    let path = format!(
                        "{bldg_path}/storeys/L{}/footings/footing_{}",
                        ftg.level_id, ftg.id
                    );
                    doc.add_node(
                        IfcxNode::new(path)
                            .with_class("IfcFooting")
                            .with_prop(
                                &ftg.name,
                                "Enlarged RC Pad Footing",
                                &format!(
                                    "{:.0}x{:.0}x{:.0} mm",
                                    ftg.width_mm, ftg.length_mm, ftg.thickness_mm
                                ),
                            )
                            .with_attr(
                                ns::EG_BIM_ELEMENT,
                                json!({
                                    "kind": "footing",
                                    "id": ftg.id,
                                    "widthMm": ftg.width_mm,
                                    "lengthMm": ftg.length_mm,
                                    "thicknessMm": ftg.thickness_mm
                                }),
                            )
                            .with_attr(NS_EG_RETROFIT, action_payload)
                            .with_mesh(&mesh),
                    );
                }
            }
        }
    }

    doc
}

/// Synthesizes an automated structural retrofit plan + non-destructive `.ifcx` delta layer
/// verified against site-coupled 6-DOF FEM & `EigenGate` redundancy metrics.
pub fn synthesize_structural_retrofit(
    project: &BimProject,
    strategy: RetrofitStrategy,
    site_coupling: &SiteEnvironmentalCoupling,
) -> Result<StructuralRetrofitReport, String> {
    if project.is_empty() {
        return Err(
            "Active BIM model is empty. Create or load a building before running Retrofit."
                .to_string(),
        );
    }

    let (before, _fem_map, bim_metrics) = evaluate_site_coupled_health(project, site_coupling)?;
    let mut retro = project.clone();
    let mut actions = Vec::new();

    let mut jacketed_columns = 0usize;
    let mut strengthened_beams = 0usize;
    let mut added_brb_braces = 0usize;
    let mut added_shear_walls = 0usize;
    let mut enlarged_footings = 0usize;

    match strategy {
        RetrofitStrategy::AutoBalanced => {
            let (jc, sb, ef) = apply_jacketing_pass(&mut retro, &bim_metrics, &mut actions);
            jacketed_columns += jc;
            strengthened_beams += sb;
            enlarged_footings += ef;

            added_brb_braces += apply_brb_bracing_pass(&mut retro, &bim_metrics, &mut actions);

            if site_coupling.seismic_pga_ss_g >= 0.45
                || before.max_dcr_pct >= 80.0
                || retro.walls.is_empty()
            {
                added_shear_walls +=
                    apply_shear_wall_infill_pass(&mut retro, &bim_metrics, &mut actions, 1);
            }
        }
        RetrofitStrategy::BrbBracingOnly => {
            added_brb_braces += apply_brb_bracing_pass(&mut retro, &bim_metrics, &mut actions);
            let (jc, sb, ef) = apply_jacketing_pass(&mut retro, &bim_metrics, &mut actions);
            jacketed_columns += jc;
            strengthened_beams += sb;
            enlarged_footings += ef;
        }
        RetrofitStrategy::RcJacketingOnly => {
            let (jc, sb, ef) = apply_jacketing_pass(&mut retro, &bim_metrics, &mut actions);
            jacketed_columns += jc;
            strengthened_beams += sb;
            enlarged_footings += ef;
        }
        RetrofitStrategy::ShearWallCore => {
            added_shear_walls +=
                apply_shear_wall_infill_pass(&mut retro, &bim_metrics, &mut actions, 2);
            let (jc, sb, ef) = apply_jacketing_pass(&mut retro, &bim_metrics, &mut actions);
            jacketed_columns += jc;
            strengthened_beams += sb;
            enlarged_footings += ef;
            if added_shear_walls == 0 {
                added_brb_braces += apply_brb_bracing_pass(&mut retro, &bim_metrics, &mut actions);
            }
        }
    }

    // Fallback safety: if no geometric intervention was synthesized yet, apply RC jacketing
    if actions.is_empty() {
        let (jc, sb, ef) = apply_jacketing_pass(&mut retro, &bim_metrics, &mut actions);
        jacketed_columns += jc;
        strengthened_beams += sb;
        enlarged_footings += ef;
    }

    let (after, _, _) = evaluate_site_coupled_health(&retro, site_coupling)?;

    let added_concrete_m3: f64 = actions.iter().map(|a| a.added_concrete_m3).sum();
    let added_steel_kg: f64 = actions.iter().map(|a| a.added_steel_kg).sum();
    let estimated_cost_usd: f64 = actions.iter().map(|a| a.cost_delta_usd).sum();
    let embodied_carbon_kg_co2e: f64 = actions.iter().map(|a| a.carbon_delta_kg_co2e).sum();

    let eigengate_gain = (after.min_eigengate - before.min_eigengate).max(0.0) * 100.0
        + (after.avg_eigengate - before.avg_eigengate).max(0.0) * 50.0;
    let dcr_drop = (before.max_dcr_pct - after.max_dcr_pct).max(0.0);
    let cost_penalty = (estimated_cost_usd / 2500.0).clamp(0.0, 25.0);
    let pareto_efficiency_score = (55.0 + eigengate_gain * 1.4 + dcr_drop * 0.7 - cost_penalty).clamp(15.0, 99.5);

    let retrofit_delta_ifcx = build_retrofit_delta_ifcx(
        &retro,
        strategy,
        before,
        after,
        &actions,
        (
            jacketed_columns,
            strengthened_beams,
            added_brb_braces,
            added_shear_walls,
            enlarged_footings,
        ),
        (
            added_concrete_m3,
            added_steel_kg,
            estimated_cost_usd,
            embodied_carbon_kg_co2e,
            pareto_efficiency_score,
        ),
    );

    let summary = format!(
        "[Retrofit {}]: EigenGate {:.3}->{:.3} | Max DCR {:.1}%->{:.1}% | +{} BRB, +{} Jacket Cols, +{} Beams, +{} ShearWall, +{} Footings | Cost ${:.0} | Embodied Carbon {:.0} kgCO2e",
        strategy.short_label(),
        before.min_eigengate,
        after.min_eigengate,
        before.max_dcr_pct,
        after.max_dcr_pct,
        added_brb_braces,
        jacketed_columns,
        strengthened_beams,
        added_shear_walls,
        enlarged_footings,
        estimated_cost_usd,
        embodied_carbon_kg_co2e,
    );

    Ok(StructuralRetrofitReport {
        strategy,
        before_min_eigengate: before.min_eigengate,
        after_min_eigengate: after.min_eigengate,
        before_avg_eigengate: before.avg_eigengate,
        after_avg_eigengate: after.avg_eigengate,
        before_max_dcr_pct: before.max_dcr_pct,
        after_max_dcr_pct: after.max_dcr_pct,
        before_critical_count: before.critical_count,
        after_critical_count: after.critical_count,
        before_svd_mechanisms: before.svd_mechanisms,
        after_svd_mechanisms: after.svd_mechanisms,
        jacketed_columns,
        strengthened_beams,
        added_brb_braces,
        added_shear_walls,
        enlarged_footings,
        added_concrete_m3,
        added_steel_kg,
        estimated_cost_usd,
        embodied_carbon_kg_co2e,
        pareto_efficiency_score,
        actions,
        retrofit_delta_ifcx,
        retrofitted_project: retro,
        summary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bim::BimCommandBus;
    use crate::core::ifcx::IfcxStage;

    #[test]
    fn phase69_retrofit_synthesizes_and_improves_eigengate_across_all_strategies() {
        let mut bus = BimCommandBus::new();
        bus.execute_natural_language("portal 3x2 @6m 2floors")
            .expect("portal frame");
        let site = SiteEnvironmentalCoupling::default();

        for strategy in [
            RetrofitStrategy::AutoBalanced,
            RetrofitStrategy::BrbBracingOnly,
            RetrofitStrategy::RcJacketingOnly,
            RetrofitStrategy::ShearWallCore,
        ] {
            let report = synthesize_structural_retrofit(&bus.project, strategy, &site)
                .expect("retrofit synthesis should succeed");
            assert!(
                !report.actions.is_empty(),
                "Strategy {strategy:?} must produce concrete/steel retrofit actions"
            );
            assert!(
                report.after_min_eigengate + 1e-6 >= report.before_min_eigengate,
                "Strategy {strategy:?} reduced min EigenGate ({} -> {})",
                report.before_min_eigengate,
                report.after_min_eigengate
            );
            assert!(
                report.after_max_dcr_pct <= report.before_max_dcr_pct + 1.0,
                "Strategy {strategy:?} spiked max DCR ({} -> {})",
                report.before_max_dcr_pct,
                report.after_max_dcr_pct
            );
            assert!(report.estimated_cost_usd > 0.0);
            assert!(report.embodied_carbon_kg_co2e > 0.0);
            assert!(
                report
                    .retrofit_delta_ifcx
                    .data
                    .iter()
                    .any(|n| n.attributes.contains_key(NS_EG_RETROFIT)),
                "Delta IFCX must carry eigengraph::bim::retrofit_delta attributes"
            );
            assert!(
                report
                    .retrofit_delta_ifcx
                    .data
                    .iter()
                    .any(|n| n.attributes.contains_key(ns::EG_EIGENGATE)),
                "Delta IFCX must carry eigengraph::eigengate verification attributes"
            );
        }

        // Verify non-destructive IFCX stage composition
        let auto_rep = synthesize_structural_retrofit(
            &bus.project,
            RetrofitStrategy::AutoBalanced,
            &site,
        )
        .expect("auto retrofit");
        assert!(auto_rep.added_brb_braces >= 1);
        assert!(auto_rep.jacketed_columns >= 1);

        let base_ifcx = IfcxDocument::from_bim_project(
            &bus.project,
            Some((site.lat_deg, site.lon_deg, f64::from(site.elevation_m))),
            0.0,
            Some(&site.site_label),
            None,
            None,
        );
        let composed = IfcxStage::compose_with_retrofit_delta(&base_ifcx, &auto_rep);
        assert!(composed.data.len() > base_ifcx.data.len());
        assert!(composed
            .data
            .iter()
            .any(|n| n.attributes.contains_key(NS_EG_RETROFIT)));
        assert!(composed
            .data
            .iter()
            .any(|n| n.attributes.contains_key(ns::EG_EIGENGATE)));
    }
}
