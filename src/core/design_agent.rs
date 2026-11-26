//! core/design_agent.rs — Update `v0.8.5.9` (`9/10` -> `v0.8.6`):
//! Chained Multi-Step AI Building Design Agent (Concept-to-Engineered-BIM Pipeline).
//!
//! Parses natural-language architectural / structural design briefs or RFPs and orchestrates
//! the full 6-stage Snaptrude-style pipeline:
//! 1. Brief & Parameter Extraction (`ParsedDesignBrief`)
//! 2. Site Zoning & Buildable Envelope (`evaluate_site_zoning`)
//! 3. Spatial Programming & 3D Stack/Pack (`evaluate_spatial_program`)
//! 4. Multi-Option Massing Comparison (`evaluate_massing_options`)
//! 5. 1-Click Mass-to-Engineered-BIM Synthesis (`convert_massing_option_to_bim`)
//! 6. Live 6-DOF FEM & `EigenGate` Redundancy Audit (`EigenGateEngine::evaluate_redundancy_fast`)

use serde::{Deserialize, Serialize};

use crate::core::bim::BimProject;
use crate::core::eigengate::EigenGateEngine;
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
use crate::core::massing_agent::{
    convert_massing_option_to_bim, evaluate_massing_options, MassToBimSummary, MassingArchetype,
    MassingStructuralSystem, MassingStudyConfig, MassingStudyReport,
};
use crate::core::program_agent::{
    evaluate_spatial_program, BuildingTypologyPreset, SpatialProgramConfig, SpatialProgramReport,
};
use crate::core::zoning_agent::{
    evaluate_site_zoning, SiteZoningConfig, SiteZoningReport, ZoningDistrictPreset,
};

/// Canonical EigenGraph IFCX attribute namespace for the Concept-to-Engineered-BIM AI Design Pipeline.
pub const NS_EG_DESIGN_PIPELINE: &str = "eigengraph::design::concept_pipeline";

/// Structured engineering & architectural parameters extracted from a natural-language design brief.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedDesignBrief {
    pub raw_prompt: String,
    pub zoning_preset: ZoningDistrictPreset,
    pub typology: BuildingTypologyPreset,
    pub preferred_archetype: Option<MassingArchetype>,
    pub structural_system: MassingStructuralSystem,
    pub lot_width_m: Option<f64>,
    pub lot_depth_m: Option<f64>,
    pub target_stories: Option<usize>,
    pub kdb_pct: Option<f64>,
    pub klb_far: Option<f64>,
    pub bay_m: Option<f64>,
    pub auto_convert_to_bim: bool,
}

/// Trace record for each stage in the 6-stage Chained AI Building Design Pipeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignPipelineStepTrace {
    pub stage_index: usize,
    pub stage_name: String,
    pub status_ok: bool,
    pub metric_summary: String,
}

/// Compact serializable IFCX overlay payload under `eigengraph::design::concept_pipeline`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignPipelineIfcxOverlay {
    pub schema_version: String,
    pub brief_summary: String,
    pub selected_archetype: String,
    pub structural_system: String,
    pub gfa_m2: f64,
    pub stories: usize,
    pub klb_far: f64,
    pub live_eigengate_ari: f64,
    pub live_fem_nodes: usize,
    pub live_fem_elements: usize,
    pub live_critical_members: usize,
    pub estimated_cost_usd: f64,
    pub embodied_carbon_tco2e: f64,
    pub stages_passed: usize,
    pub total_stages: usize,
    pub summary_verdict: String,
}

/// Complete report produced by the 6-stage Chained AI Building Design Pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignPipelineReport {
    pub brief: ParsedDesignBrief,
    pub zoning_config: SiteZoningConfig,
    pub program_config: SpatialProgramConfig,
    pub massing_config: MassingStudyConfig,
    pub zoning_report: SiteZoningReport,
    pub program_report: SpatialProgramReport,
    pub massing_report: MassingStudyReport,
    pub adopted_option_index: usize,
    pub mass_to_bim_summary: Option<MassToBimSummary>,
    pub live_eigengate_ari: f64,
    pub live_fem_nodes: usize,
    pub live_fem_elements: usize,
    pub live_critical_members: usize,
    pub steps: Vec<DesignPipelineStepTrace>,
    pub overall_ok: bool,
    pub summary_verdict: String,
}

impl DesignPipelineReport {
    pub fn to_ifcx_overlay(&self) -> DesignPipelineIfcxOverlay {
        let opt = self
            .massing_report
            .options
            .get(self.adopted_option_index)
            .or_else(|| self.massing_report.active_option());
        let stages_passed = self.steps.iter().filter(|s| s.status_ok).count();
        DesignPipelineIfcxOverlay {
            schema_version: "design-pipeline-ifcx-v1".to_string(),
            brief_summary: self.brief.raw_prompt.clone(),
            selected_archetype: opt
                .map(|o| o.archetype.label().to_string())
                .unwrap_or_else(|| self.massing_config.selected_archetype.label().to_string()),
            structural_system: self.massing_config.structural_system.label().to_string(),
            gfa_m2: opt.map_or(0.0, |o| o.gfa_m2),
            stories: opt.map_or(0, |o| o.stories),
            klb_far: opt.map_or(0.0, |o| o.klb_far),
            live_eigengate_ari: self.live_eigengate_ari,
            live_fem_nodes: self.live_fem_nodes,
            live_fem_elements: self.live_fem_elements,
            live_critical_members: self.live_critical_members,
            estimated_cost_usd: opt.map_or(0.0, |o| o.estimated_cost_usd),
            embodied_carbon_tco2e: opt.map_or(0.0, |o| o.embodied_carbon_tco2e),
            stages_passed,
            total_stages: self.steps.len(),
            summary_verdict: self.summary_verdict.clone(),
        }
    }
}

/// Returns true when a natural-language prompt expresses a full building concept / RFP brief
/// rather than a simple single-portal frame synthesis or slash command.
pub fn looks_like_building_design_brief(prompt: &str) -> bool {
    let trimmed = prompt.trim();
    if trimmed.is_empty() || trimmed.starts_with('/') {
        return false;
    }
    let lo = trimmed.to_ascii_lowercase();
    if lo.starts_with("portal ")
        || lo.starts_with("frame ")
        || lo.starts_with("box ")
        || lo.contains("build portal")
        || lo.contains("create portal")
    {
        return false;
    }
    let has_verb = [
        "design",
        "rfp",
        "feasibility",
        "concept",
        "propose",
        "masterplan",
        "buat gedung",
        "desain gedung",
        "rancang",
        "generate building",
    ]
    .iter()
    .any(|k| lo.contains(k));
    let has_domain_noun = [
        "tower",
        "mixed-use",
        "mixed use",
        "office",
        "apartment",
        "hospital",
        "campus",
        "kdb",
        "klb",
        "far",
        "courtyard",
        "podium",
        "stepped",
        "story",
        "stories",
        "floor",
        "lantai",
        "gedung",
        "lot",
    ]
    .iter()
    .any(|k| lo.contains(k));
    has_verb && has_domain_noun
}

fn parse_f64_after_keyword(lower: &str, keywords: &[&str]) -> Option<f64> {
    for kw in keywords {
        if let Some(pos) = lower.find(kw) {
            let tail = lower[pos + kw.len()..].trim_start_matches([':', '=', ' ']);
            let num_str: String = tail
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if let Ok(v) = num_str.parse::<f64>() {
                if v.is_finite() && v > 0.0 {
                    return Some(v);
                }
            }
        }
    }
    None
}

fn parse_lot_dimensions(lower: &str) -> Option<(f64, f64)> {
    let compact = lower.replace(" x ", "x");
    for raw_tok in compact.split_whitespace() {
        let tok = raw_tok.trim_end_matches(|c: char| !c.is_ascii_digit());
        if let Some((a, b)) = tok.split_once('x') {
            let a_num: String = a.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
            let b_num: String = b.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
            if let (Ok(w), Ok(d)) = (a_num.parse::<f64>(), b_num.parse::<f64>()) {
                if (12.0..=300.0).contains(&w) && (12.0..=300.0).contains(&d) {
                    return Some((w, d));
                }
            }
        }
    }
    None
}

fn parse_story_count(lower: &str) -> Option<usize> {
    let normalized = lower.replace('-', " ");
    let tokens: Vec<&str> = normalized.split_whitespace().collect();
    for i in 0..tokens.len() {
        let t = tokens[i].trim_matches(|c: char| !c.is_ascii_alphanumeric());
        if let Some(num) = t.strip_suffix('f') {
            if let Ok(n) = num.parse::<usize>() {
                if (2..=50).contains(&n) {
                    return Some(n);
                }
            }
        }
        if let Ok(n) = t.parse::<usize>() {
            if let Some(next) = tokens.get(i + 1) {
                let nxt = next.trim_matches(|c: char| !c.is_ascii_alphabetic());
                if matches!(nxt, "story" | "stories" | "floor" | "floors" | "lantai")
                    && (2..=50).contains(&n)
                {
                    return Some(n);
                }
            }
        }
    }
    None
}

/// Parses an architectural RFP or design brief prompt into zoning, typology, massing, and structural settings.
pub fn parse_design_brief_prompt(prompt: &str) -> ParsedDesignBrief {
    let lo = prompt.to_ascii_lowercase();

    let typology = if lo.contains("hospital") || lo.contains("clinic") || lo.contains("rumah sakit")
    {
        BuildingTypologyPreset::HealthcareHospital
    } else if lo.contains("campus") || lo.contains("university") || lo.contains("school") {
        BuildingTypologyPreset::UniversityCampus
    } else if lo.contains("office") || lo.contains("headquarters") || lo.contains("hq")
        || lo.contains("kantor")
    {
        BuildingTypologyPreset::OfficeHeadquarters
    } else if lo.contains("apartment")
        || lo.contains("residential")
        || lo.contains("condo")
        || lo.contains("apartemen")
    {
        BuildingTypologyPreset::ResidentialApartment
    } else {
        BuildingTypologyPreset::MixedUseTower
    };

    let zoning_preset = if lo.contains("cbd") || lo.contains("central business") {
        ZoningDistrictPreset::CommercialCbd
    } else if lo.contains("suburban") || lo.contains("low rise") || lo.contains("low-rise") {
        ZoningDistrictPreset::LowRiseSuburban
    } else if lo.contains("institutional")
        || typology == BuildingTypologyPreset::HealthcareHospital
        || typology == BuildingTypologyPreset::UniversityCampus
    {
        ZoningDistrictPreset::MidRiseInstitutional
    } else if typology == BuildingTypologyPreset::ResidentialApartment {
        ZoningDistrictPreset::ResidentialHighDensity
    } else {
        ZoningDistrictPreset::MixedUseUrban
    };

    let preferred_archetype = if lo.contains("courtyard") || lo.contains("atrium") {
        Some(MassingArchetype::CourtyardBlock)
    } else if lo.contains("linear") || lo.contains("bar") || lo.contains("slab") {
        Some(MassingArchetype::LinearSlabBar)
    } else if lo.contains("stepped") || lo.contains("terrace") || lo.contains("biophilic") {
        Some(MassingArchetype::SteppedTerrace)
    } else if lo.contains("podium") || lo.contains("tower") {
        Some(MassingArchetype::PodiumTower)
    } else {
        None
    };

    let structural_system = if lo.contains("diagrid") || lo.contains("braced") {
        MassingStructuralSystem::DiagridHybrid
    } else if lo.contains("steel") || lo.contains("composite") || lo.contains("baja") {
        MassingStructuralSystem::CompositeSteelCore
    } else {
        MassingStructuralSystem::RcMomentCore
    };

    let (lot_width_m, lot_depth_m) = match parse_lot_dimensions(&lo) {
        Some((w, d)) => (Some(w), Some(d)),
        None => (None, None),
    };
    let target_stories = parse_story_count(&lo);
    let kdb_pct = parse_f64_after_keyword(&lo, &["kdb", "bcr"]).map(|v| v.clamp(20.0, 90.0));
    let klb_far = parse_f64_after_keyword(&lo, &["klb", "far"]).map(|v| v.clamp(0.5, 18.0));
    let bay_m = parse_f64_after_keyword(&lo, &["bay", "grid"]).map(|v| v.clamp(4.8, 10.8));
    let auto_convert_to_bim =
        !(lo.contains("study only") || lo.contains("massing only") || lo.contains("no bim"));

    ParsedDesignBrief {
        raw_prompt: prompt.trim().to_string(),
        zoning_preset,
        typology,
        preferred_archetype,
        structural_system,
        lot_width_m,
        lot_depth_m,
        target_stories,
        kdb_pct,
        klb_far,
        bay_m,
        auto_convert_to_bim,
    }
}

/// Executes the full 6-stage Chained AI Building Design Pipeline:
/// Brief -> Site Zoning -> Spatial Program -> Multi-Option Massing -> 1-Click Mass-to-BIM -> Live EigenGate Audit.
pub fn execute_chained_design_pipeline(
    prompt: &str,
    base_bim: &BimProject,
    site: &SiteEnvironmentalCoupling,
    base_zoning: &SiteZoningConfig,
    base_program: &SpatialProgramConfig,
    base_massing: &MassingStudyConfig,
) -> (DesignPipelineReport, Option<BimProject>) {
    let brief = parse_design_brief_prompt(prompt);

    // Stage 1: Configure Zoning, Program & Massing from parsed brief
    let mut zoning_cfg = base_zoning.clone();
    zoning_cfg.apply_preset(brief.zoning_preset);
    if let (Some(w), Some(d)) = (brief.lot_width_m, brief.lot_depth_m) {
        zoning_cfg.lot_width_m = w;
        zoning_cfg.lot_depth_m = d;
    }
    if let Some(kdb) = brief.kdb_pct {
        zoning_cfg.kdb_bcr_pct = kdb;
    }
    if let Some(klb) = brief.klb_far {
        zoning_cfg.klb_far = klb;
    }
    if let Some(s) = brief.target_stories {
        let req_h = (s as f64) * zoning_cfg.typical_story_height_m;
        if req_h > zoning_cfg.max_height_m {
            zoning_cfg.max_height_m = (req_h + 1.0).min(240.0);
        }
        let lot_area = zoning_cfg.lot_width_m * zoning_cfg.lot_depth_m;
        let est_fp = (lot_area * (zoning_cfg.kdb_bcr_pct / 100.0) * 0.72).max(200.0);
        let needed_klb = (est_fp * (s as f64)) / lot_area.max(100.0);
        if brief.klb_far.is_none() && needed_klb > zoning_cfg.klb_far {
            zoning_cfg.klb_far = (needed_klb * 1.10).min(18.0);
        }
    }

    let mut program_cfg = base_program.clone();
    program_cfg.apply_typology(brief.typology);
    if let Some(s) = brief.target_stories {
        let lot_area = zoning_cfg.lot_width_m * zoning_cfg.lot_depth_m;
        let typ_fp = (lot_area * (zoning_cfg.kdb_bcr_pct / 100.0) * 0.72).max(240.0);
        let target_gfa = typ_fp * (s as f64);
        let cur_nfa: f64 = program_cfg.spaces.iter().map(|sp| sp.total_area_m2()).sum();
        let overhead = 1.0
            + (program_cfg.circulation_factor_pct + program_cfg.core_ratio_pct).clamp(10.0, 55.0)
                / 100.0;
        let cur_gfa = (cur_nfa * overhead).max(300.0);
        let scale = (target_gfa / cur_gfa).clamp(0.45, 3.2);
        for sp in &mut program_cfg.spaces {
            sp.unit_count = ((sp.unit_count as f64) * scale).round().max(1.0) as usize;
        }
    }

    let mut massing_cfg = base_massing.clone();
    massing_cfg.structural_system = brief.structural_system;
    if let Some(b) = brief.bay_m {
        massing_cfg.bay_x_m = b;
        massing_cfg.bay_y_m = b;
    }
    if let Some(pref) = brief.preferred_archetype {
        massing_cfg.selected_archetype = pref;
    }

    let mut steps = Vec::with_capacity(6);
    steps.push(DesignPipelineStepTrace {
        stage_index: 1,
        stage_name: "Brief Extraction".to_string(),
        status_ok: true,
        metric_summary: format!(
            "{} | {} | {}F req | Bay {:.1}m",
            brief.zoning_preset.short_label(),
            brief.typology.short_label(),
            brief.target_stories.unwrap_or(0),
            massing_cfg.bay_x_m,
        ),
    });

    // Stage 2: Site Zoning & Buildable Envelope
    let zoning_report = evaluate_site_zoning(base_bim, Some(site), &zoning_cfg);
    steps.push(DesignPipelineStepTrace {
        stage_index: 2,
        stage_name: "Site Zoning & Solar".to_string(),
        status_ok: zoning_report.overall_compliant,
        metric_summary: format!(
            "Lot {:.0}m² | Pad {:.1}x{:.1}m | MaxGFA {:.0}m² | Max {}F",
            zoning_report.lot_area_m2,
            zoning_report.buildable_pad_width_m,
            zoning_report.buildable_pad_depth_m,
            zoning_report.max_gfa_klb_m2,
            zoning_report.max_allowable_stories,
        ),
    });

    // Stage 3: Spatial Program & 3D Stack/Pack
    let program_report = evaluate_spatial_program(Some(&zoning_report), &program_cfg);
    steps.push(DesignPipelineStepTrace {
        stage_index: 3,
        stage_name: "Spatial Stack & Pack".to_string(),
        status_ok: program_report.fits_zoning_envelope && program_report.adjacency_score_pct >= 75.0,
        metric_summary: format!(
            "GFA {:.0}m² ({:.0}% KLB) | Eff {:.1}% | Adj {:.0}%",
            program_report.total_gfa_m2,
            program_report.gfa_utilization_pct,
            program_report.overall_efficiency_pct,
            program_report.adjacency_score_pct,
        ),
    });

    // Stage 4: Multi-Option Massing Explorer
    let mut massing_report =
        evaluate_massing_options(&zoning_report, &program_report, site, &massing_cfg);
    let adopted_option_index = if let Some(pref) = brief.preferred_archetype {
        MassingArchetype::ALL
            .iter()
            .position(|&a| a == pref)
            .unwrap_or(massing_report.recommended_index)
    } else {
        massing_report.recommended_index
    };
    massing_report.selected_index = adopted_option_index;
    if let Some(chosen) = massing_report.options.get(adopted_option_index) {
        massing_cfg.selected_archetype = chosen.archetype;
        massing_report.config.selected_archetype = chosen.archetype;
    }
    let chosen_opt = &massing_report.options[adopted_option_index];
    steps.push(DesignPipelineStepTrace {
        stage_index: 4,
        stage_name: "Massing Optimization".to_string(),
        status_ok: chosen_opt.zoning_compliant,
        metric_summary: format!(
            "{} ({}F, GFA {:.0}m², Score {:.0}/100)",
            chosen_opt.archetype.short_label(),
            chosen_opt.stories,
            chosen_opt.gfa_m2,
            chosen_opt.composite_score,
        ),
    });

    // Stage 5 & 6: 1-Click Mass-to-BIM & Live FEM + EigenGate Redundancy Audit
    let mut generated_bim = None;
    let mut mass_to_bim_summary = None;
    let mut live_eigengate_ari = chosen_opt.estimated_eigengate_ari;
    let mut live_fem_nodes = 0usize;
    let mut live_fem_elements = 0usize;
    let mut live_critical_members = 0usize;

    if brief.auto_convert_to_bim {
        let (bim, summary) = convert_massing_option_to_bim(&massing_report, adopted_option_index);
        steps.push(DesignPipelineStepTrace {
            stage_index: 5,
            stage_name: "Mass-to-Engineered-BIM".to_string(),
            status_ok: summary.columns > 0 && summary.beams > 0 && summary.slabs > 0,
            metric_summary: format!(
                "{}L | {} cols | {} beams | {} walls | {} slabs",
                summary.levels, summary.columns, summary.beams, summary.walls, summary.slabs
            ),
        });

        let fem = bim.to_structural_model();
        live_fem_nodes = fem.nodes.len();
        live_fem_elements = fem.elements.len();
        match EigenGateEngine::evaluate_redundancy_fast(&fem) {
            Ok(assessment) => {
                live_eigengate_ari = assessment.min_system_eigengate;
                live_critical_members = assessment.critical_members_count;
                steps.push(DesignPipelineStepTrace {
                    stage_index: 6,
                    stage_name: "EigenGate 6-DOF Audit".to_string(),
                    status_ok: assessment.kinematic_guard.is_stable
                        && assessment.min_system_eigengate >= 0.50,
                    metric_summary: format!(
                        "MinARI {:.2} | AvgARI {:.2} | Crit {} | {} DOFs",
                        assessment.min_system_eigengate,
                        assessment.average_eigengate,
                        assessment.critical_members_count,
                        assessment.kinematic_guard.active_dofs,
                    ),
                });
            }
            Err(err) => {
                steps.push(DesignPipelineStepTrace {
                    stage_index: 6,
                    stage_name: "EigenGate 6-DOF Audit".to_string(),
                    status_ok: false,
                    metric_summary: format!("Audit fallback ({err})"),
                });
            }
        }
        mass_to_bim_summary = Some(summary);
        generated_bim = Some(bim);
    } else {
        steps.push(DesignPipelineStepTrace {
            stage_index: 5,
            stage_name: "Mass-to-Engineered-BIM".to_string(),
            status_ok: true,
            metric_summary: "Deferred (study-only brief)".to_string(),
        });
        steps.push(DesignPipelineStepTrace {
            stage_index: 6,
            stage_name: "EigenGate 6-DOF Audit".to_string(),
            status_ok: chosen_opt.estimated_eigengate_ari >= 0.60,
            metric_summary: format!(
                "Pre-BIM EstARI {:.2} | MaxDCR {:.0}%",
                chosen_opt.estimated_eigengate_ari, chosen_opt.estimated_max_dcr_pct
            ),
        });
    }

    let passed = steps.iter().filter(|s| s.status_ok).count();
    let overall_ok = passed >= 5;
    let summary_verdict = format!(
        "Design Agent ({passed}/6): {} {}F • GFA {:.0}m² • KLB {:.2} • ARI {:.2} • ${:.1}M / {:.0}tCO2",
        chosen_opt.archetype.short_label(),
        chosen_opt.stories,
        chosen_opt.gfa_m2,
        chosen_opt.klb_far,
        live_eigengate_ari,
        chosen_opt.estimated_cost_usd / 1_000_000.0,
        chosen_opt.embodied_carbon_tco2e,
    );

    let report = DesignPipelineReport {
        brief,
        zoning_config: zoning_cfg,
        program_config: program_cfg,
        massing_config: massing_cfg,
        zoning_report,
        program_report,
        massing_report,
        adopted_option_index,
        mass_to_bim_summary,
        live_eigengate_ari,
        live_fem_nodes,
        live_fem_elements,
        live_critical_members,
        steps,
        overall_ok,
        summary_verdict,
    };
    (report, generated_bim)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};

    #[test]
    fn detects_and_parses_natural_language_design_briefs() {
        let prompt = "Design a 10-story mixed-use tower on 54x42m urban lot, KDB 60%, KLB 5.5, 7.2m bay, composite steel";
        assert!(looks_like_building_design_brief(prompt));
        assert!(!looks_like_building_design_brief("portal 3x2 @6m 2floors"));
        assert!(!looks_like_building_design_brief("/massing"));

        let parsed = parse_design_brief_prompt(prompt);
        assert_eq!(parsed.typology, BuildingTypologyPreset::MixedUseTower);
        assert_eq!(parsed.lot_width_m, Some(54.0));
        assert_eq!(parsed.lot_depth_m, Some(42.0));
        assert_eq!(parsed.target_stories, Some(10));
        assert_eq!(parsed.kdb_pct, Some(60.0));
        assert_eq!(parsed.klb_far, Some(5.5));
        assert_eq!(parsed.bay_m, Some(7.2));
        assert_eq!(parsed.structural_system, MassingStructuralSystem::CompositeSteelCore);
    }

    #[test]
    fn executes_full_six_stage_pipeline_with_bim_and_ifcx() {
        let dummy = BimProject::empty();
        let site = SiteEnvironmentalCoupling::default();
        let (rep, bim_opt) = execute_chained_design_pipeline(
            "Design 8-story courtyard office on 48x36m lot bay 6.0m",
            &dummy,
            &site,
            &SiteZoningConfig::default(),
            &SpatialProgramConfig::default(),
            &MassingStudyConfig::default(),
        );
        assert_eq!(rep.steps.len(), 6);
        assert!(bim_opt.is_some());
        assert!(rep.live_fem_nodes > 0);
        assert!(rep.live_fem_elements > 0);

        let base = IfcxDocument::from_bim_project(&dummy, None, 0.0, None, None, None);
        let composed =
            IfcxStage::compose_with_design_pipeline_overlay(&base, &rep.to_ifcx_overlay());
        assert!(composed
            .data
            .iter()
            .any(|n| n.attributes.contains_key(NS_EG_DESIGN_PIPELINE)));
    }
}
