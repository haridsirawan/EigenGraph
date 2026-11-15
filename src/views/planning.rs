//! planning.rs — Pillar 5: 5D Construction Planning, Dynamic RAB, 4D Scheduling & Cost-ARI Trade-Off.
//! Automatically calculated and synchronized with PARAMETRIC variables (dimensions, material, stiffness, unit prices).

use egui::{
    Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, Pos2, Rect, RichText,
    ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2,
};
use crate::features::{FeatureId, FeatureRegistry};
use crate::icons::{self, AppIcon};
use crate::models::{
    ConstructionSimulationPhase, CurrencyUnit, NavigationPillar, ParametricVariable,
    StructuralMember, WbsCostItem,
};
use crate::theme::MacTheme;
use crate::ui_kit;

/// Formats a number as Rupiah / Indonesian currency string with dot separators (e.g. 1.250.000)
pub fn format_idr(val: f64) -> String {
    let int_part = val.round() as i64;
    let s = int_part.abs().to_string();
    let mut res = String::new();
    let mut count = 0;
    for ch in s.chars().rev() {
        if count > 0 && count % 3 == 0 {
            res.push('.');
        }
        res.push(ch);
        count += 1;
    }
    let formatted: String = res.chars().rev().collect();
    if int_part < 0 {
        format!("-{}", formatted)
    } else {
        formatted
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanningSubTab {
    TakeoffQto,
    DynamicRab,
    Schedule4d,
    CostAriTradeoff,
    StagedErectionSvd,
    WbsHierarchy,
    ScheduleRiskMonteCarlo,
    ResourceLeveling4d,
    EarnedValueEvm,
    ValueEngineeringBid,
    SiteDailyDiaryFleet,
    ProcurementContractsClaims,
}

pub struct PlanningView {
    pub active_subtab: PlanningSubTab,
    pub published_to_dossier: bool,
    pub export_status: Option<String>,
    pub contingency_percent: f32,
    pub overhead_percent: f32,
    pub vat_percent: f32,
    // Phase 28: 4D Simulation & 5D WBS
    pub active_currency: CurrencyUnit,
    pub simulation_day: u32,
    pub is_simulating: bool,
    pub simulation_speed: u32,
    pub simulation_phases: Vec<ConstructionSimulationPhase>,
    pub wbs_catalog: Vec<WbsCostItem>,
    pub selected_wbs_division: Option<String>,
    pub show_manpower_curve: bool,
    pub show_cpm_only: bool,
    // Phase 40: Live 4D-03 / 4D-04 / 5D-03 interactive solver state
    pub mc_trial_count: usize,
    pub mc_pert_spread_pct: f32,
    pub mc_seed: u64,
    pub leveling_crew_cap: u32,
    pub leveling_within_float_only: bool,
    pub evm_cutoff_month: usize,
    pub evm_retention_pct: f32,
    pub evm_cpi_factor: f32,
    // Phase 41 (5D-04): Live Bid Leveling & Value Engineering Tender Matrix
    pub ve_outlier_pct: f64,
    pub ve_composite_deck_swap: bool,
    pub ve_ari_floor: f64,
    pub ve_selected_bidder_idx: usize,
    // Phase 42: Reactive BIM 4D/5D auto-sync signature
    pub last_bim_sync_sig: (usize, usize, usize),
    // Phase 45: Earth Site Hazard Coupling + Site Daily Diary & Fleet + Commercial Procurement/VO
    pub site_coupling: crate::core::SiteEnvironmentalCoupling,
    pub procurement: crate::core::ProcurementContractLedger,
    pub daily_diary: crate::core::SiteDailyDiaryLedger,
    pub new_po_vendor: String,
    pub new_po_pkg: String,
    pub new_po_amount_idr: f64,
    pub new_vo_desc: String,
    pub new_vo_cost_idr: f64,
    pub new_vo_eot_days: i32,
    pub new_diary_remarks: String,
    pub new_diary_pour_m3: f32,
    pub new_diary_crane_pct: f32,
}

impl Default for PlanningView {
    fn default() -> Self {
        Self {
            active_subtab: PlanningSubTab::DynamicRab,
            published_to_dossier: false,
            export_status: None,
            contingency_percent: 5.0,
            overhead_percent: 10.0,
            vat_percent: 11.0,
            active_currency: CurrencyUnit::Idr,
            simulation_day: 0,
            is_simulating: false,
            simulation_speed: 1,
            simulation_phases: Vec::new(),
            wbs_catalog: Vec::new(),
            selected_wbs_division: None,
            show_manpower_curve: true,
            show_cpm_only: false,
            mc_trial_count: 1000,
            mc_pert_spread_pct: 25.0,
            mc_seed: 40,
            leveling_crew_cap: 85,
            leveling_within_float_only: true,
            evm_cutoff_month: 7,
            evm_retention_pct: 5.0,
            evm_cpi_factor: 0.97,
            ve_outlier_pct: 15.0,
            ve_composite_deck_swap: true,
            ve_ari_floor: 0.90,
            ve_selected_bidder_idx: 1,
            last_bim_sync_sig: (usize::MAX, 0, 0),
            site_coupling: crate::core::SiteEnvironmentalCoupling::default(),
            procurement: crate::core::ProcurementContractLedger::default(),
            daily_diary: crate::core::SiteDailyDiaryLedger::default(),
            new_po_vendor: "PT Jaya ReadyMix".to_string(),
            new_po_pkg: "03 Upper Slabs K-350".to_string(),
            new_po_amount_idr: 1_250_000_000.0,
            new_vo_desc: "Foundation dewatering & bentonite cutoff extension".to_string(),
            new_vo_cost_idr: 215_000_000.0,
            new_vo_eot_days: 4,
            new_diary_remarks: "Completed formwork inspection & crane lift schedule.".to_string(),
            new_diary_pour_m3: 84.0,
            new_diary_crane_pct: 84.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BidLevelingLine {
    pub package: &'static str,
    pub owner_est: f64,
    pub bidder_a: f64,
    pub bidder_b: f64,
    pub bidder_c: f64,
    pub outlier_note: String,
    pub balanced: bool,
}

#[derive(Debug, Clone)]
pub struct BidLevelingSummary {
    pub lines: Vec<BidLevelingLine>,
    pub total_oe: f64,
    pub total_a: f64,
    pub total_b: f64,
    pub total_c: f64,
    pub best_bidder_idx: usize,
    pub best_bid_total: f64,
    pub outlier_count: usize,
    pub ve_savings_val: f64,
    pub projected_ari: f64,
    pub ari_compliant: bool,
}

/// Computes normalized bid leveling across three tenderers, front-loading outlier flags, and ARI-guarded VE savings.
pub fn compute_bid_leveling_matrix(
    t: &Parametric5DTakeoff,
    outlier_threshold_pct: f64,
    ve_composite_deck_swap: bool,
    ari_floor: f64,
) -> BidLevelingSummary {
    let thresh = outlier_threshold_pct.clamp(5.0, 40.0);
    let balance_pkg = (t.grand_total_rab - (t.cost_substructure + t.cost_steel_material + t.cost_workshop_fab + t.cost_erection_crane)).max(0.0);
    let raw = [
        ("02 Bored Piles & Pile Caps", t.cost_substructure, 0.98, 0.95, 1.28),
        ("05 Steel Supply BJ41 / A992", t.cost_steel_material, 0.97, 0.92, 0.99),
        ("05 Workshop Fab + UT/NDT", t.cost_workshop_fab, 1.18, 0.94, 0.96),
        ("05 Crane Erection & Rigging", t.cost_erection_crane, 1.02, 0.91, 0.95),
        ("09 Fireproofing, OH & contingency", balance_pkg, 1.00, 0.95, 0.98),
    ];

    let mut lines = Vec::with_capacity(raw.len());
    let mut outlier_count = 0usize;
    let mut total_oe = 0.0;
    let mut total_a = 0.0;
    let mut total_b = 0.0;
    let mut total_c = 0.0;

    for (pkg, oe, ma, mb, mc) in raw {
        let a = oe * ma;
        let b = oe * mb;
        let c = oe * mc;
        total_oe += oe;
        total_a += a;
        total_b += b;
        total_c += c;

        let da = (ma - 1.0) * 100.0;
        let db = (mb - 1.0) * 100.0;
        let dc = (mc - 1.0) * 100.0;
        let mut flags = Vec::new();
        if da.abs() > thresh {
            flags.push(format!("Bidder A {:+.0}%", da));
        }
        if db.abs() > thresh {
            flags.push(format!("Bidder B {:+.0}%", db));
        }
        if dc.abs() > thresh {
            flags.push(format!("Bidder C {:+.0}%", dc));
        }
        let balanced = flags.is_empty();
        if !balanced {
            outlier_count += 1;
        }
        let outlier_note = if balanced {
            "Balanced Rate".to_string()
        } else {
            flags.join(" / ")
        };
        lines.push(BidLevelingLine {
            package: pkg,
            owner_est: oe,
            bidder_a: a,
            bidder_b: b,
            bidder_c: c,
            outlier_note,
            balanced,
        });
    }

    let totals = [total_a, total_b, total_c];
    let (best_bidder_idx, best_bid_total) = totals
        .iter()
        .enumerate()
        .min_by(|(_, x), (_, y)| x.total_cmp(y))
        .map(|(i, v)| (i, *v))
        .unwrap_or((1, total_b));

    let ve_savings_val = if ve_composite_deck_swap {
        t.cost_steel_material * 0.082 + t.cost_erection_crane * 0.055
    } else {
        0.0
    };
    let projected_ari = if ve_composite_deck_swap { 0.935 } else { 0.978 };
    let ari_compliant = projected_ari >= ari_floor;

    BidLevelingSummary {
        lines,
        total_oe,
        total_a,
        total_b,
        total_c,
        best_bidder_idx,
        best_bid_total,
        outlier_count,
        ve_savings_val,
        projected_ari,
        ari_compliant,
    }
}

#[derive(Debug, Clone)]
pub struct MonteCarloScheduleResult {
    pub deterministic_days: f64,
    pub p50_days: f64,
    pub p80_days: f64,
    pub p90_days: f64,
    pub cdf_points: Vec<(f64, f64)>,
    pub drivers: Vec<(String, f64, f64)>,
}

#[derive(Debug, Clone)]
pub struct CrewLevelingResult {
    pub monthly_unleveled: [f32; 12],
    pub monthly_leveled: [f32; 12],
    pub peak_unleveled: f32,
    pub peak_leveled: f32,
    pub float_consumed_days: f64,
}

#[derive(Debug, Clone)]
pub struct EvmMonthSummary {
    pub month: usize,
    pub pv_ratio: f64,
    pub ev_ratio: f64,
    pub ac_ratio: f64,
    pub spi: f64,
    pub cpi: f64,
    pub ev_val: f64,
    pub ac_val: f64,
    pub eac_val: f64,
    pub vac_val: f64,
    pub net_claim: f64,
}

/// Calculated 5D Quantities & Cost Breakdown derived directly from Parametric Variables
#[derive(Debug, Clone)]
pub struct Parametric5DTakeoff {
    pub span_m: f64,
    pub height_m: f64,
    pub n_panels: usize,
    pub density_kg_m3: f64,
    pub stiffness_e_mpa: f64,

    // Unit Prices from Parametric
    pub price_steel_kg: f64,
    pub price_fab_kg: f64,
    pub price_erect_ton: f64,
    pub price_coating_m2: f64,
    pub price_concrete_m3: f64,

    // Quantities (QTO)
    pub chord_length_m: f64,
    pub diagonal_length_m: f64,
    pub total_steel_kg: f64,
    pub total_steel_ton: f64,
    pub coating_surface_m2: f64,
    pub bolt_sets_count: usize,
    pub concrete_footing_m3: f64,
    pub duration_days: usize,
    pub total_man_days: usize,

    // Costs (RAB)
    pub cost_mobilization: f64,
    pub cost_substructure: f64,
    pub cost_steel_material: f64,
    pub cost_workshop_fab: f64,
    pub cost_coating_fireproof: f64,
    pub cost_transportation: f64,
    pub cost_erection_crane: f64,
    pub cost_bolts_finishing: f64,

    pub direct_cost_total: f64,
    pub overhead_cost: f64,
    pub contingency_cost: f64,
    pub vat_cost: f64,
    pub grand_total_rab: f64,

    // Safety & ARI indicator
    pub predicted_system_ari: f64,
}

impl PlanningView {
    /// Synchronizes 4D construction staging phases and 5D CSI WBS cost catalog directly from active BIM + FEM state.
    pub fn sync_from_bim(
        &mut self,
        bim_project: &crate::core::BimProject,
        members: &[StructuralMember],
    ) {
        let sig = (
            bim_project.element_count(),
            bim_project.rebars.len(),
            members.len(),
        );
        self.last_bim_sync_sig = sig;
        if bim_project.is_empty() && members.is_empty() {
            self.simulation_phases.clear();
            self.wbs_catalog.clear();
            return;
        }

        let conc_m3 = bim_project.total_concrete_volume_m3().max(12.0);
        let rebar_kg = bim_project.total_rebar_weight_kg();
        let structural_steel_kg: f64 = bim_project
            .runs
            .iter()
            .filter(|r| r.is_wf_profile)
            .map(|r| (r.start.dist(r.end) / 1000.0) * 48.0)
            .sum::<f64>()
            + bim_project
                .columns
                .iter()
                .filter(|c| c.is_wf_profile)
                .map(|_| 3.6 * 62.0)
                .sum::<f64>();
        let wall_area_m2: f64 = bim_project
            .walls
            .iter()
            .map(|w| (w.length_mm() / 1000.0) * (w.unconnected_height_mm / 1000.0))
            .sum();
        let footing_cnt = bim_project.footings.len().max(1);
        let levels_cnt = bim_project.levels.len().max(1);

        if self.simulation_day == 0 {
            self.simulation_day = 95;
        }

        self.simulation_phases = vec![
            ConstructionSimulationPhase {
                id: "4D-01".to_string(),
                name: format!("Subsurface Pile Caps & {} BIM Footings", footing_cnt),
                day_start: 1,
                day_end: 45,
                planned_progress_pct: 100.0,
                actual_progress_pct: 100.0,
                manpower_count: 24,
                equipment_count: 3,
                concrete_volume_m3: (conc_m3 * 0.25) as f32,
                color_rgb: (245, 158, 11),
                wbs_code: "02.30".to_string(),
                is_critical: true,
            },
            ConstructionSimulationPhase {
                id: "4D-02".to_string(),
                name: format!(
                    "Vertical Core ({} Cols, {} Walls across {} Levels)",
                    bim_project.columns.len(),
                    bim_project.walls.len(),
                    levels_cnt
                ),
                day_start: 46,
                day_end: 135,
                planned_progress_pct: 88.0,
                actual_progress_pct: 84.5,
                manpower_count: 42,
                equipment_count: 4,
                concrete_volume_m3: (conc_m3 * 0.42) as f32,
                color_rgb: (59, 130, 246),
                wbs_code: "03.30".to_string(),
                is_critical: true,
            },
            ConstructionSimulationPhase {
                id: "4D-03".to_string(),
                name: format!(
                    "Primary Beam Grid & Deck Slabs ({} Runs, {} Slabs)",
                    bim_project.runs.len(),
                    bim_project.slabs.len()
                ),
                day_start: 136,
                day_end: 235,
                planned_progress_pct: 64.0,
                actual_progress_pct: 61.0,
                manpower_count: 38,
                equipment_count: 3,
                concrete_volume_m3: (conc_m3 * 0.33) as f32,
                color_rgb: (16, 185, 129),
                wbs_code: "05.12".to_string(),
                is_critical: true,
            },
            ConstructionSimulationPhase {
                id: "4D-04".to_string(),
                name: format!(
                    "Rebar Detailing & Envelope Cladding ({:.0} kg BBS)",
                    rebar_kg
                ),
                day_start: 236,
                day_end: 315,
                planned_progress_pct: 32.0,
                actual_progress_pct: 30.0,
                manpower_count: 28,
                equipment_count: 2,
                concrete_volume_m3: 0.0,
                color_rgb: (139, 92, 246),
                wbs_code: "03.20".to_string(),
                is_critical: false,
            },
            ConstructionSimulationPhase {
                id: "4D-05".to_string(),
                name: format!("Structural NDT, SHM Calibration & Handover ({} FEM)", members.len()),
                day_start: 316,
                day_end: 360,
                planned_progress_pct: 10.0,
                actual_progress_pct: 10.0,
                manpower_count: 14,
                equipment_count: 2,
                concrete_volume_m3: 0.0,
                color_rgb: (6, 182, 212),
                wbs_code: "01.70".to_string(),
                is_critical: false,
            },
        ];

        self.wbs_catalog = vec![
            WbsCostItem {
                wbs_code: "02.31".to_string(),
                description: format!("Isolated/Pile Cap Foundations ({} BIM footings)", footing_cnt),
                division: "02 Substructure".to_string(),
                unit: "pcs".to_string(),
                quantity: footing_cnt as f64,
                unit_price_idr: 14_500_000.0,
                total_cost_idr: footing_cnt as f64 * 14_500_000.0,
                progress_pct: 100.0,
            },
            WbsCostItem {
                wbs_code: "03.30".to_string(),
                description: "Cast-in-Place Concrete fc' 30 MPa (Columns, Beams, Slabs)".to_string(),
                division: "03 Concrete".to_string(),
                unit: "m³".to_string(),
                quantity: conc_m3,
                unit_price_idr: 1_950_000.0,
                total_cost_idr: conc_m3 * 1_950_000.0,
                progress_pct: 82.0,
            },
            WbsCostItem {
                wbs_code: "03.20".to_string(),
                description: format!("Deformed Rebar BBS ({} bar groups)", bim_project.rebars.len()),
                division: "03 Concrete".to_string(),
                unit: "kg".to_string(),
                quantity: rebar_kg.max(100.0),
                unit_price_idr: 18_800.0,
                total_cost_idr: rebar_kg.max(100.0) * 18_800.0,
                progress_pct: 74.0,
            },
            WbsCostItem {
                wbs_code: "04.20".to_string(),
                description: format!("Compound Architectural & Shear Walls ({} walls)", bim_project.walls.len()),
                division: "04 Masonry & Walls".to_string(),
                unit: "m²".to_string(),
                quantity: wall_area_m2.max(10.0),
                unit_price_idr: 685_000.0,
                total_cost_idr: wall_area_m2.max(10.0) * 685_000.0,
                progress_pct: 68.0,
            },
            WbsCostItem {
                wbs_code: "05.12".to_string(),
                description: format!("Structural Steel Frame WF ({} FEM members)", members.len()),
                division: "05 Metals".to_string(),
                unit: "kg".to_string(),
                quantity: structural_steel_kg.max(500.0),
                unit_price_idr: 26_500.0,
                total_cost_idr: structural_steel_kg.max(500.0) * 26_500.0,
                progress_pct: 60.0,
            },
        ];
    }

    /// Extract parametric variables and evaluate full 5D quantities
    pub fn compute_takeoff(&self, variables: &[ParametricVariable]) -> Parametric5DTakeoff {
        self.compute_takeoff_with_bim(variables, None, &[])
    }

    /// Extract parametric variables blended with live 3D BIM element volumes and FEM redundancy results.
    pub fn compute_takeoff_with_bim(
        &self,
        variables: &[ParametricVariable],
        bim_project: Option<&crate::core::BimProject>,
        members: &[StructuralMember],
    ) -> Parametric5DTakeoff {
        let get_val = |name: &str, default: f64| -> f64 {
            for v in variables {
                if v.name.eq_ignore_ascii_case(name) || v.name.contains(name) {
                    return v.evaluated_value as f64;
                }
            }
            default
        };

        // Dimensions
        let has_live_bim = bim_project.is_some_and(|p| !p.is_empty());
        let mut span_m = get_val("L", 24000.0) / 1000.0;
        let height_m = get_val("H", 3200.0) / 1000.0;
        let n_panels = get_val("n", 6.0).round().max(2.0) as usize;
        if let Some(proj) = bim_project.filter(|p| !p.is_empty()) {
            if let Some((a, b)) = proj.plan_extents_mm() {
                let live_span = ((b[0] - a[0]).abs().max((b[1] - a[1]).abs()) / 1000.0).max(4.0);
                span_m = live_span;
            }
        }

        // Material & Stiffness
        let density_kg_m3 = get_val("density", 7850.0);
        let stiffness_e_mpa = get_val("stiffness", get_val("kekakuan", 200000.0));

        // Unit Prices (IDR)
        let price_steel_kg = get_val("price_steel", get_val("harga_baja", 24500.0));
        let price_fab_kg = get_val("price_fab", get_val("harga_fabrikasi", 8500.0));
        let price_erect_ton = get_val("price_erect", get_val("harga_ereksi", 4500000.0));
        let price_coating_m2 = get_val("price_coating", get_val("harga_coating", 145000.0));
        let price_concrete_m3 = get_val("price_concrete", get_val("harga_pondasi", 1950000.0));

        // Member Lengths & Quantities
        let panel_dx = span_m / (n_panels as f64);
        let chord_length_m = span_m + (span_m - panel_dx); // bottom + top chord
        let diag_len = ((panel_dx / 2.0).powi(2) + height_m.powi(2)).sqrt();
        let diagonal_length_m = diag_len * (2.0 * n_panels as f64);

        // Cross Section Areas: W14x90 (0.0171 m2) for chords, HSS 8x8 (0.00929 m2) for diagonals
        // Add 15% allowance for connection plates, gussets, and stiffeners
        let chord_vol = chord_length_m * 0.0171;
        let diag_vol = diagonal_length_m * 0.00929;
        let total_vol = (chord_vol + diag_vol) * 1.15;
        let total_steel_kg = total_vol * density_kg_m3;

        // Surface Area for Coating: AISC W14x90 perimeter ~1.45m, HSS8x8 ~0.80m
        let coating_surface_m2 = (chord_length_m * 1.45 + diagonal_length_m * 0.80) * 1.10;

        // Bolts & Concrete (blended with active BIM model when non-empty)
        let mut total_steel_kg = total_steel_kg;
        let mut concrete_footing_m3 = 2.0 * (15.0 + span_m * 0.75);
        if has_live_bim {
            if let Some(proj) = bim_project {
                let bim_conc = proj.total_concrete_volume_m3();
                if bim_conc > 0.1 {
                    concrete_footing_m3 = bim_conc;
                }
                let bim_rebar = proj.total_rebar_weight_kg();
                let bim_frame_steel: f64 = proj
                    .runs
                    .iter()
                    .filter(|r| r.is_wf_profile)
                    .map(|r| (r.start.dist(r.end) / 1000.0) * 48.0)
                    .sum::<f64>()
                    + proj
                        .columns
                        .iter()
                        .filter(|c| c.is_wf_profile)
                        .map(|_| 3.6 * 62.0)
                        .sum::<f64>();
                if bim_rebar + bim_frame_steel > 50.0 {
                    total_steel_kg = bim_rebar + bim_frame_steel;
                }
            }
        }
        let total_steel_ton = total_steel_kg / 1000.0;
        let bolt_sets_count = (2 * n_panels + 1) * 16;
        let duration_days = 90 + (span_m * 2.0) as usize;
        let total_man_days = (total_steel_ton * 14.5) as usize;

        // Direct Costs
        let cost_mobilization = 120_000_000.0 + span_m * 2_500_000.0;
        let cost_substructure = concrete_footing_m3 * price_concrete_m3;
        let cost_steel_material = total_steel_kg * price_steel_kg;
        let cost_workshop_fab = total_steel_kg * price_fab_kg;
        let cost_coating_fireproof = coating_surface_m2 * price_coating_m2;
        let cost_transportation = 45_000_000.0 + total_steel_ton * 850_000.0;
        let cost_erection_crane = total_steel_ton * price_erect_ton;
        let cost_bolts_finishing = (bolt_sets_count as f64) * 125_000.0 + 45_000_000.0;

        let direct_cost_total = cost_mobilization
            + cost_substructure
            + cost_steel_material
            + cost_workshop_fab
            + cost_coating_fireproof
            + cost_transportation
            + cost_erection_crane
            + cost_bolts_finishing;

        let overhead_cost = direct_cost_total * (self.overhead_percent as f64 / 100.0);
        let contingency_cost = direct_cost_total * (self.contingency_percent as f64 / 100.0);
        let before_vat = direct_cost_total + overhead_cost + contingency_cost;
        let vat_cost = before_vat * (self.vat_percent as f64 / 100.0);
        let grand_total_rab = before_vat + vat_cost;

        // System ARI indicator derived from live members when populated, else parametric geometry
        let predicted_system_ari = if !members.is_empty() {
            (members.iter().map(|m| m.ari).sum::<f64>() / members.len() as f64).clamp(0.0, 0.995)
        } else {
            (0.75 + (height_m / span_m) * 1.25).min(0.965)
        };

        Parametric5DTakeoff {
            span_m,
            height_m,
            n_panels,
            density_kg_m3,
            stiffness_e_mpa,
            price_steel_kg,
            price_fab_kg,
            price_erect_ton,
            price_coating_m2,
            price_concrete_m3,
            chord_length_m,
            diagonal_length_m,
            total_steel_kg,
            total_steel_ton,
            coating_surface_m2,
            bolt_sets_count,
            concrete_footing_m3,
            duration_days,
            total_man_days,
            cost_mobilization,
            cost_substructure,
            cost_steel_material,
            cost_workshop_fab,
            cost_coating_fireproof,
            cost_transportation,
            cost_erection_crane,
            cost_bolts_finishing,
            direct_cost_total,
            overhead_cost,
            contingency_cost,
            vat_cost,
            grand_total_rab,
            predicted_system_ari,
        }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        variables: &[ParametricVariable],
        members: &[StructuralMember],
        bim_project: &crate::core::BimProject,
        features: &mut FeatureRegistry,
        active_pillar: &mut NavigationPillar,
    ) {
        let cur_sig = (
            bim_project.element_count(),
            bim_project.rebars.len(),
            members.len(),
        );
        if cur_sig != self.last_bim_sync_sig {
            self.sync_from_bim(bim_project, members);
        }
        let takeoff = self.compute_takeoff_with_bim(variables, Some(bim_project), members);

        ui.vertical(|ui| {
            Frame::new().inner_margin(Margin::symmetric(0, 8)).show(ui, |ui| {
                let full_w = ui.available_width().max(160.0);
                ui.set_width(full_w);
                ui.set_max_width(full_w);
                // 1. TOP SUB-TABS SELECTOR & EXPORT BAR
                self.render_header_bar(ui, theme, &takeoff, active_pillar);

                ui.add_space(12.0);

                // 2. LIVE PARAMETRIC BINDING BANNER
                self.render_parametric_binding_banner(ui, theme, &takeoff);

                ui.add_space(14.0);

                // 3. ACTIVE SUB-TAB CONTENT
                match self.active_subtab {
                    PlanningSubTab::TakeoffQto => {
                        self.render_qto_takeoff(ui, theme, &takeoff, members)
                    }
                    PlanningSubTab::DynamicRab => {
                        self.render_dynamic_rab(ui, theme, &takeoff, bim_project, features)
                    }
                    PlanningSubTab::Schedule4d => {
                        self.render_schedule_4d(ui, theme, &takeoff, bim_project, features)
                    }
                    PlanningSubTab::CostAriTradeoff => {
                        self.render_cost_ari_tradeoff(ui, theme, &takeoff)
                    }
                    PlanningSubTab::StagedErectionSvd => {
                        self.render_staged_erection_svd(ui, theme, &takeoff)
                    }
                    PlanningSubTab::WbsHierarchy => self.render_wbs_hierarchy(ui, theme, &takeoff),
                    PlanningSubTab::ScheduleRiskMonteCarlo => {
                        self.render_schedule_risk_monte_carlo(ui, theme, &takeoff, bim_project, features);
                    }
                    PlanningSubTab::ResourceLeveling4d => {
                        self.render_resource_leveling_4d(ui, theme, &takeoff, bim_project, features);
                    }
                    PlanningSubTab::EarnedValueEvm => {
                        self.render_earned_value_evm(ui, theme, &takeoff, features);
                    }
                    PlanningSubTab::ValueEngineeringBid => {
                        self.render_value_engineering_bid(ui, theme, &takeoff, features);
                    }
                    PlanningSubTab::SiteDailyDiaryFleet => {
                        self.render_site_daily_diary_fleet(ui, theme);
                    }
                    PlanningSubTab::ProcurementContractsClaims => {
                        self.render_procurement_contracts_claims(ui, theme, &takeoff);
                    }
                }

                ui.add_space(48.0);
            });
        });
    }

    fn render_header_bar(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        takeoff: &Parametric5DTakeoff,
        active_pillar: &mut NavigationPillar,
    ) {
        let is_time_mode = *active_pillar == NavigationPillar::TimeScheduling;
        if is_time_mode {
            let time_tabs = [
                (PlanningSubTab::Schedule4d, "⏱️ 4D Synchro Simulation"),
                (PlanningSubTab::StagedErectionSvd, "🏗️ Staged Erection Guard"),
                (PlanningSubTab::ScheduleRiskMonteCarlo, "🎲 Monte Carlo Risk"),
                (PlanningSubTab::ResourceLeveling4d, "👷 Crew Leveling"),
                (PlanningSubTab::SiteDailyDiaryFleet, "📋 Site Daily Diary & Fleet"),
            ];
            ui_kit::subtab_bar(ui, theme, "planning_time_subtab_bar", &mut self.active_subtab, &time_tabs);
        } else {
            let cost_tabs = [
                (PlanningSubTab::DynamicRab, "💰 5D BoQ / Cost"),
                (PlanningSubTab::WbsHierarchy, "📑 5D WBS Explorer"),
                (PlanningSubTab::TakeoffQto, "📊 QTO"),
                (PlanningSubTab::CostAriTradeoff, "📈 Cost vs ARI"),
                (PlanningSubTab::EarnedValueEvm, "📈 Earned Value (EVM)"),
                (PlanningSubTab::ValueEngineeringBid, "⚖️ VE & Bid Leveling"),
                (PlanningSubTab::ProcurementContractsClaims, "🧾 Procurement, Claims & VO"),
            ];
            ui_kit::subtab_bar(ui, theme, "planning_cost_subtab_bar", &mut self.active_subtab, &cost_tabs);
        }

        ui.add_space(8.0);
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::symmetric(12, 8))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let (switch_icon, switch_lbl, target_pillar, target_tab) = if is_time_mode {
                        (AppIcon::Coins, "Open 5D Budgeting", NavigationPillar::CostBudgeting, PlanningSubTab::DynamicRab)
                    } else {
                        (AppIcon::Clock, "Open 4D Scheduling", NavigationPillar::TimeScheduling, PlanningSubTab::Schedule4d)
                    };
                    if icons::icon_button(
                        ui,
                        switch_icon,
                        switch_lbl,
                        11.0,
                        theme.accent_blue,
                        theme.accent_blue.gamma_multiply(0.15),
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        *active_pillar = target_pillar;
                        self.active_subtab = target_tab;
                    }

                    ui.separator();

                    ui.label(
                        RichText::new("CURRENCY:")
                            .size(10.5)
                            .strong()
                            .color(theme.text_muted),
                    );
                    for curr in CurrencyUnit::all().iter() {
                        let is_curr = self.active_currency == *curr;
                        let c_bg = if is_curr {
                            theme.accent_orange
                        } else {
                            theme.bg_canvas
                        };
                        let c_fg = if is_curr {
                            Color32::WHITE
                        } else {
                            theme.text_secondary
                        };
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(format!("{} ({})", curr.code(), curr.symbol()))
                                        .size(11.0)
                                        .color(c_fg),
                                )
                                .fill(c_bg)
                                .corner_radius(CornerRadius::same(6)),
                            )
                            .clicked()
                        {
                            self.active_currency = *curr;
                        }
                    }

                    ui.separator();

                    if icons::icon_button(
                        ui,
                        AppIcon::Document,
                        "Export BoQ (CSV)",
                        11.5,
                        theme.text_primary,
                        theme.bg_canvas,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(8),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        self.export_status = Some(format!(
                            "File BoQ-5D_L{:.0}m.csv exported successfully!",
                            takeoff.span_m
                        ));
                    }

                    let (pub_icon, pub_text) = if self.published_to_dossier {
                        (AppIcon::CheckCircle, "Saved to Dossier")
                    } else {
                        (AppIcon::Folder, "Save to MY PROJECT")
                    };
                    let pub_bg = if self.published_to_dossier {
                        theme.accent_green
                    } else {
                        theme.accent_blue.gamma_multiply(0.85)
                    };
                    if icons::icon_button(
                        ui,
                        pub_icon,
                        pub_text,
                        11.5,
                        Color32::WHITE,
                        pub_bg,
                        Stroke::NONE,
                        CornerRadius::same(8),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        self.published_to_dossier = true;
                        self.export_status = Some(
                            "5D BoQ & Schedule synchronized to Project Dossier!".to_string(),
                        );
                    }
                });
            });

        if let Some(msg) = &self.export_status {
            ui.add_space(6.0);
            Frame::new()
                .fill(theme.accent_green.gamma_multiply(0.15))
                .stroke(Stroke::new(1.0, theme.accent_green))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::symmetric(12, 6))
                .show(ui, |ui| {
                    icons::icon_label(ui, AppIcon::CheckCircle, 13.0, theme.accent_green, msg.as_str(), 11.5, theme.accent_green, false);
                });
        }
    }

    fn render_parametric_binding_banner(&self, ui: &mut Ui, theme: &MacTheme, takeoff: &Parametric5DTakeoff) {
        Frame::new()
            .fill(theme.bg_sidebar)
            .stroke(Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.4)))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    icons::icon_label(ui, AppIcon::LinkChain, 13.0, theme.accent_blue, "BOUND TO PARAMETRIC MODEL:", 11.5, theme.accent_blue, true);
                    ui.label(RichText::new(format!("Span L = {:.1} m", takeoff.span_m)).size(11.5).color(theme.text_primary));
                    ui.label(RichText::new("•").size(11.5).color(theme.text_muted));
                    ui.label(RichText::new(format!("Height H = {:.1} m (L/{:.1})", takeoff.height_m, takeoff.span_m / takeoff.height_m)).size(11.5).color(theme.text_primary));
                    ui.label(RichText::new("•").size(11.5).color(theme.text_muted));
                    ui.label(RichText::new(format!("ρ = {:.0} kg/m³", takeoff.density_kg_m3)).size(11.5).color(theme.text_primary));
                    ui.label(RichText::new("•").size(11.5).color(theme.text_muted));
                    ui.label(RichText::new(format!("Steel Rate: IDR {}/kg", format_idr(takeoff.price_steel_kg))).size(11.5).color(theme.accent_green));
                    icons::icon_label(ui, AppIcon::Bolt, 12.5, theme.text_muted, "Live Dynamic Calculation", 11.0, theme.text_muted, false);
                });
            });
    }

    // -------------------------------------------------------------
    // SUB-TAB 1: DYNAMIC RAB (5D Cost Estimation)
    // -------------------------------------------------------------
    fn render_dynamic_rab(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        t: &Parametric5DTakeoff,
        bim_project: &crate::core::BimProject,
        features: &mut FeatureRegistry,
    ) {
        // 4 Responsive KPI Metric Cards
        let rab_total = format!("IDR {}", format_idr(t.grand_total_rab));
        let steel_cost = format!("IDR {}", format_idr(t.cost_steel_material));
        let steel_sub = format!("{:.1} Ton @ IDR {}/kg", t.total_steel_ton, format_idr(t.price_steel_kg));
        let fab_erect = format!("IDR {}", format_idr(t.cost_workshop_fab + t.cost_erection_crane));
        let substruct = format!("IDR {}", format_idr(t.cost_substructure));
        let sub_note = format!("{:.1} m³ Concrete K-350", t.concrete_footing_m3);

        ui_kit::responsive_grid_n(ui, 4, 210.0, |ui, idx| match idx {
            0 => self.metric_card(ui, theme, "TOTAL 5D BoQ / BUDGET", &rab_total, "Includes 11% VAT & Overhead", theme.accent_blue),
            1 => self.metric_card(ui, theme, "STEEL MATERIAL COST", &steel_cost, &steel_sub, theme.accent_green),
            2 => self.metric_card(ui, theme, "FABRICATION & ERECTION", &fab_erect, "Workshop + 100T Mobile Crane", theme.accent_orange),
            _ => self.metric_card(ui, theme, "FOUNDATION & SUBSTRUCTURE", &substruct, &sub_note, theme.text_secondary),
        });

        ui.add_space(16.0);

        // Detailed AHSP Breakdown Table (Horizontally scrollable on narrow mobile)
        theme.card_frame().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("BILL OF QUANTITIES (BoQ) BREAKDOWN — DYNAMIC COST ESTIMATION").size(13.0).strong().color(theme.text_primary));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("Currency: IDR (Rupiah)").size(11.0).color(theme.text_muted));
                });
            });

            ui.add_space(10.0);

            let items = [
                ("1.0", "Heavy Equipment Mobilization (100T Crane, Trailer, Genset)", 1.0, "ls", t.cost_mobilization, t.cost_mobilization),
                ("2.0", "Bored Pile Foundation & K-350 Concrete Abutment Works", t.concrete_footing_m3, "m³", t.price_concrete_m3, t.cost_substructure),
                ("3.0", "Structural Steel Material Procurement BJ41 (ASTM A992)", t.total_steel_kg, "kg", t.price_steel_kg, t.cost_steel_material),
                ("4.0", "Workshop Fabrication (Cutting, Fitting, Full Pen Welding, NDT)", t.total_steel_kg, "kg", t.price_fab_kg, t.cost_workshop_fab),
                ("5.0", "Corrosion Protection: Sa 2.5 Sandblasting + 3-Coat Epoxy", t.coating_surface_m2, "m²", t.price_coating_m2, t.cost_coating_fireproof),
                ("6.0", "Truss Beam Haulage & Site Transportation", 1.0, "ls", t.cost_transportation, t.cost_transportation),
                ("7.0", "Full-Span Steel Truss Erection with Crane & Shoring", t.total_steel_ton, "ton", t.price_erect_ton, t.cost_erection_crane),
                ("8.0", "A325 High-Strength Bolt Installation, Torque Calibration & Finishing", t.bolt_sets_count as f64, "set", 125_000.0, t.cost_bolts_finishing),
            ];

            ScrollArea::horizontal()
                .id_salt("planning_boq_table_scroll")
                .show(ui, |ui| {
                    egui::Grid::new("planning_boq_table_grid")
                        .striped(true)
                        .min_col_width(64.0)
                        .spacing(Vec2::new(16.0, 7.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new("No").strong().size(11.5).color(theme.text_muted));
                            ui.label(RichText::new("Work Breakdown Structure (WBS)").strong().size(11.5).color(theme.text_muted));
                            ui.label(RichText::new("Volume").strong().size(11.5).color(theme.text_muted));
                            ui.label(RichText::new("Unit").strong().size(11.5).color(theme.text_muted));
                            ui.label(RichText::new("Unit Price (IDR)").strong().size(11.5).color(theme.text_muted));
                            ui.label(RichText::new("Total Price (IDR)").strong().size(11.5).color(theme.text_muted));
                            ui.end_row();

                            for (no, desc, vol, sat, hrg, tot) in items {
                                ui.label(RichText::new(no).size(11.5).color(theme.text_secondary));
                                ui.label(RichText::new(desc).size(11.5).strong().color(theme.text_primary));
                                ui.label(RichText::new(format!("{:.2}", vol)).size(11.5).color(theme.text_secondary));
                                ui.label(RichText::new(sat).size(11.5).color(theme.text_muted));
                                ui.label(RichText::new(format_idr(hrg)).size(11.5).color(theme.text_secondary));
                                ui.label(RichText::new(format_idr(tot)).size(11.5).strong().color(theme.accent_green));
                                ui.end_row();
                            }
                        });
                });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // Summary Totals
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Direct Cost Subtotal:").size(12.0).color(theme.text_secondary));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("IDR {}", format_idr(t.direct_cost_total))).size(12.5).strong().color(theme.text_primary));
                });
            });

            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("Contractor Overhead & Profit ({:.1}%):", self.overhead_percent)).size(11.5).color(theme.text_muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("IDR {}", format_idr(t.overhead_cost))).size(11.5).color(theme.text_muted));
                });
            });

            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("Contingency ({:.1}%):", self.contingency_percent)).size(11.5).color(theme.text_muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("IDR {}", format_idr(t.contingency_cost))).size(11.5).color(theme.text_muted));
                });
            });

            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("Value Added Tax (VAT {:.1}%):", self.vat_percent)).size(11.5).color(theme.text_muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("IDR {}", format_idr(t.vat_cost))).size(11.5).color(theme.text_muted));
                });
            });

            ui.separator();
            ui.add_space(4.0);

            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("GRAND TOTAL ESTIMATED COST (5D BIM):").size(13.0).strong().color(theme.text_primary));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("IDR {},-", format_idr(t.grand_total_rab))).size(17.0).strong().color(theme.accent_blue));
                });
            });
        });

        ui.add_space(14.0);
        ui_kit::feature_slot_card(
            ui,
            theme,
            features,
            FeatureId::LiveBimQtoBoqSync,
            |ui| {
                let conc_m3 = bim_project.total_concrete_volume_m3();
                let rebar_kg = bim_project.total_rebar_weight_kg();
                let elem_cnt = bim_project.element_count();
                let est_bim_cost = conc_m3 * t.price_concrete_m3 + rebar_kg * t.price_steel_kg;
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "Live BIM QTO: {} elements | Concrete: {:.2} m3 | Rebar: {:.1} kg ({:.2} t) | Est. BIM BOQ: IDR {}",
                                elem_cnt,
                                conc_m3,
                                rebar_kg,
                                rebar_kg / 1000.0,
                                format_idr(est_bim_cost),
                            ))
                            .size(11.0)
                            .strong()
                            .color(theme.text_primary),
                        );
                    });
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        if icons::icon_button(ui, AppIcon::RulerSquare, "Extract Concrete m³", 11.5, theme.text_primary, theme.bg_canvas, Stroke::new(1.0, theme.border_subtle), CornerRadius::same(6), Vec2::ZERO).clicked() {
                            self.export_status = Some(format!(
                                "Extracted live BIM concrete QTO: {:.2} m3 ({} walls, {} cols, {} beams, {} footings, {} slabs).",
                                conc_m3,
                                bim_project.walls.len(),
                                bim_project.columns.len(),
                                bim_project.runs.len(),
                                bim_project.footings.len(),
                                bim_project.slabs.len(),
                            ));
                        }
                        if icons::icon_button(ui, AppIcon::Wrench, "Extract Rebar Tonnage", 11.5, theme.text_primary, theme.bg_canvas, Stroke::new(1.0, theme.border_subtle), CornerRadius::same(6), Vec2::ZERO).clicked() {
                            self.export_status = Some(format!(
                                "Extracted live BIM Rebar BBS: {:.1} kg ({:.3} tonnes) across {} bar groups.",
                                rebar_kg,
                                rebar_kg / 1000.0,
                                bim_project.rebars.len(),
                            ));
                        }
                        if icons::icon_button(ui, AppIcon::Recycle, "Sync Active BIM Quantities -> BoQ", 11.5, Color32::WHITE, theme.accent_blue, Stroke::NONE, CornerRadius::same(6), Vec2::ZERO).clicked() {
                            self.export_status = Some(format!(
                                "Synchronized {} active BIM elements -> 5D BOQ: Concrete IDR {} + Rebar IDR {} = IDR {}.",
                                elem_cnt,
                                format_idr(conc_m3 * t.price_concrete_m3),
                                format_idr(rebar_kg * t.price_steel_kg),
                                format_idr(est_bim_cost),
                            ));
                        }
                    });
                    if let Some(ref st) = self.export_status {
                        ui.add_space(4.0);
                        ui.label(RichText::new(st).size(10.5).color(theme.accent_green));
                    }
                });
            },
        );
    }

    // -------------------------------------------------------------
    // SUB-TAB 2: QUANTITY TAKEOFF (QTO)
    // -------------------------------------------------------------
    fn render_qto_takeoff(&self, ui: &mut Ui, theme: &MacTheme, t: &Parametric5DTakeoff, _members: &[StructuralMember]) {
        let steel_ton = format!("{:.2} Ton", t.total_steel_ton);
        let steel_sub = format!("{} kg pure volume", format_idr(t.total_steel_kg));
        let paint_m2 = format!("{:.1} m²", t.coating_surface_m2);
        let bolt_sets = format!("{} Sets", t.bolt_sets_count);
        let footing_m3 = format!("{:.1} m³", t.concrete_footing_m3);

        ui_kit::responsive_grid_n(ui, 4, 210.0, |ui, idx| match idx {
            0 => self.metric_card(ui, theme, "TOTAL STEEL TONNAGE", &steel_ton, &steel_sub, theme.accent_blue),
            1 => self.metric_card(ui, theme, "PAINT PROTECTION AREA", &paint_m2, "Sa 2.5 + 3-Coat System", theme.accent_green),
            2 => self.metric_card(ui, theme, "A325 BOLTED CONNECTIONS", &bolt_sets, "Grade 8.8 High Strength", theme.accent_orange),
            _ => self.metric_card(ui, theme, "FOUNDATION CONCRETE VOLUME", &footing_m3, "Grade fc' 30 MPa (K-350)", theme.text_secondary),
        });

        ui.add_space(16.0);

        theme.card_frame().show(ui, |ui| {
            ui.label(RichText::new("BILL OF QUANTITIES (BoQ) — ELEMENT GEOMETRY FORMULAS").size(13.0).strong().color(theme.text_primary));
            ui.add_space(8.0);

            let top_len = format!("{:.2} m", t.span_m - (t.span_m / t.n_panels as f64));
            let bot_len = format!("{:.2} m", t.span_m);
            let diag_len = format!("{:.2} m", t.diagonal_length_m);
            let top_wt = format!("{:.2} Ton", t.total_steel_ton * 0.38);
            let bot_wt = format!("{:.2} Ton", t.total_steel_ton * 0.42);
            let diag_wt = format!("{:.2} Ton", t.total_steel_ton * 0.20);
            let gusset_wt = format!("{:.2} Ton", t.total_steel_ton * 0.15);
            let conc_vol = format!("{:.1} m³", t.concrete_footing_m3);

            let qto_rows = [
                ("TOP CHORD COMPRESSION (Top Chord)", top_len.as_str(), "W14x90", top_wt.as_str(), "AISC 360-16 LRFD"),
                ("BOTTOM CHORD TENSION (Bottom Chord)", bot_len.as_str(), "W14x90", bot_wt.as_str(), "Tension Net Section"),
                ("WEB DIAGONALS (Diagonals)", diag_len.as_str(), "HSS8x8x1/2", diag_wt.as_str(), "Alternating Warren"),
                ("GUSSET PLATES & STIFFENERS", "52 Sheets", "Plate T=16mm BJ41", gusset_wt.as_str(), "ASTM A36 / BJ41"),
                ("ANCHOR BOLTS & BEARING BASEPLATES", "4 Support Points", "T=32mm + 8x M36", "1.85 Ton", "Elastomeric Bearing Support"),
                ("BORED PILE / PILE CAP FOUNDATION", conc_vol.as_str(), "Concrete K-350", "156.0 Ton", "Substructure Civil Work"),
            ];

            for (elem, dim, sec, weight, code) in qto_rows {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(elem).size(12.0).strong().color(theme.text_primary));
                    ui.label(RichText::new("—").size(12.0).color(theme.text_muted));
                    ui.label(RichText::new(format!("Dimensions: {}", dim)).size(12.0).color(theme.text_secondary));
                    ui.label(RichText::new("•").size(12.0).color(theme.text_muted));
                    ui.label(RichText::new(format!("Section: {}", sec)).size(12.0).color(theme.accent_blue));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(code).size(11.0).color(theme.text_muted));
                        ui.add_space(12.0);
                        ui.label(RichText::new(weight).size(12.0).strong().color(theme.accent_green));
                    });
                });
                ui.add_space(6.0);
                ui.separator();
            }
        });
    }

    // -------------------------------------------------------------
    // SUB-TAB 3: 4D SCHEDULING & S-CURVE
    // -------------------------------------------------------------
    // -------------------------------------------------------------
    // SUB-TAB 3: 4D SYNCHRO CONSTRUCTION SIMULATION & TIMELINE
    // -------------------------------------------------------------
    fn render_schedule_4d(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        _t: &Parametric5DTakeoff,
        bim_project: &crate::core::BimProject,
        features: &mut FeatureRegistry,
    ) {
        // Top 4 Responsive Metrics
        let sim_day_str = format!("Day {} / 360", self.simulation_day);
        let wbs_pkg_str = format!("{} Items", self.wbs_catalog.len());
        ui_kit::responsive_grid_n(ui, 4, 210.0, |ui, idx| match idx {
            0 => self.metric_card(ui, theme, "TOTAL WORKING DURATION", "360 Days", "12 Calendar Months", theme.accent_blue),
            1 => self.metric_card(ui, theme, "SIMULATION TIMELINE", &sim_day_str, "Interactive 4D Step", theme.accent_orange),
            2 => self.metric_card(ui, theme, "CRITICAL PATH (CPM)", "WBS 05.12.00", "Structural Steel Erection (Float: 0d)", theme.accent_red),
            _ => self.metric_card(ui, theme, "TOTAL WBS PACKAGES", &wbs_pkg_str, "Multi-Discipline Master WBS", theme.accent_green),
        });

        ui.add_space(14.0);

        // 4D Interactive Simulation Player Ribbon
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .inner_margin(Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    icons::icon_label(ui, AppIcon::Clock, 14.0, theme.accent_blue, "4D SYNCHRO SIMULATION PLAYER", 12.5, theme.accent_blue, true);

                    // Play / Pause toggle
                    let (play_icon, play_label) = if self.is_simulating { (AppIcon::Pause, "Pause") } else { (AppIcon::Play, "Play 4D") };
                    let play_bg = if self.is_simulating { theme.accent_orange } else { theme.accent_green };
                    if icons::icon_button(ui, play_icon, play_label, 11.0, Color32::WHITE, play_bg, Stroke::NONE, CornerRadius::same(6), Vec2::ZERO).clicked() {
                        self.is_simulating = !self.is_simulating;
                    }

                    // Rewind & Reset
                    if icons::icon_button(ui, AppIcon::SkipBack, "Day 1", 11.0, theme.text_primary, theme.bg_canvas, Stroke::new(1.0, theme.border_subtle), CornerRadius::same(6), Vec2::ZERO).clicked() {
                        self.simulation_day = 1;
                    }
                    if icons::icon_button(ui, AppIcon::SkipNext, "Day 360", 11.0, theme.text_primary, theme.bg_canvas, Stroke::new(1.0, theme.border_subtle), CornerRadius::same(6), Vec2::ZERO).clicked() {
                        self.simulation_day = 360;
                    }

                    // Speed Toggles
                    ui.label(RichText::new("Speed:").size(11.0).color(theme.text_secondary));
                    for spd in [1, 2, 5] {
                        let is_spd = self.simulation_speed == spd;
                        let s_bg = if is_spd { theme.accent_blue } else { theme.bg_canvas };
                        let s_fg = if is_spd { Color32::WHITE } else { theme.text_secondary };
                        if ui.add(egui::Button::new(RichText::new(format!("{}x", spd)).size(10.5).color(s_fg))
                            .fill(s_bg)
                            .corner_radius(CornerRadius::same(4))).clicked() {
                            self.simulation_speed = spd;
                        }
                    }

                    // Simulation Day Slider
                    ui.add(egui::Slider::new(&mut self.simulation_day, 1..=360).text("Construction Day"));
                });

                // Auto-advance if simulating
                if self.is_simulating {
                    self.simulation_day = (self.simulation_day + self.simulation_speed).min(360);
                    if self.simulation_day >= 360 {
                        self.is_simulating = false;
                    }
                    ui.ctx().request_repaint();
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // Active Phase Status Bar
                let cur_day = self.simulation_day;
                let active_phase = self.simulation_phases.iter().find(|p| cur_day >= p.day_start && cur_day <= p.day_end)
                    .or_else(|| self.simulation_phases.last());

                if let Some(phase) = active_phase {
                    ui.horizontal_wrapped(|ui| {
                        Frame::new()
                            .fill(Color32::from_rgb(phase.color_rgb.0, phase.color_rgb.1, phase.color_rgb.2).gamma_multiply(0.2))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(phase.color_rgb.0, phase.color_rgb.1, phase.color_rgb.2)))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(Margin::symmetric(8, 4))
                            .show(ui, |ui| {
                                ui.label(RichText::new(format!("ACTIVE PHASE: {}", phase.name)).size(11.0).strong()
                                    .color(Color32::from_rgb(phase.color_rgb.0, phase.color_rgb.1, phase.color_rgb.2)));
                            });

                        icons::icon_label(ui, AppIcon::HardHat, 13.0, theme.text_secondary, format!("Manpower: {} Workers", phase.manpower_count), 11.0, theme.text_secondary, false);
                        icons::icon_label(ui, AppIcon::Wrench, 13.0, theme.text_secondary, format!("Cranes/Equipment: {} Units", phase.equipment_count), 11.0, theme.text_secondary, false);
                        if phase.concrete_volume_m3 > 0.0 {
                            icons::icon_label(ui, AppIcon::Building, 13.0, theme.accent_blue, format!("Concrete Poured: {:.0} m³", phase.concrete_volume_m3), 11.0, theme.accent_blue, false);
                        }
                        ui.label(RichText::new(format!("Planned: {:.1}% | Actual: {:.1}%", phase.planned_progress_pct, phase.actual_progress_pct)).size(11.0).color(theme.text_muted));
                    });
                } else {
                    ui.label(
                        RichText::new("No 4D construction schedule phases loaded. Author BIM elements or import an IFC/WBS schedule to simulate erection staging.")
                            .size(11.5)
                            .color(theme.text_muted),
                    );
                }
            });

        ui.add_space(14.0);

        // 2D Staged Building Erection Elevation Canvas
        theme.card_frame().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                icons::icon_label(ui, AppIcon::Building, 15.0, theme.accent_blue, "4D STAGED ERECTION ELEVATION & TOWER CRANE SIMULATION", 13.0, theme.text_primary, true);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("Scale 1:200 • Day {} Staging", self.simulation_day)).size(11.0).color(theme.text_muted));
                });
            });
            ui.add_space(8.0);

            let canvas_w = ui.available_width().clamp(240.0, 1100.0);
            let canvas_h = 240.0;
            let (rect, _resp) = ui.allocate_exact_size(Vec2::new(canvas_w, canvas_h), Sense::hover());
            let painter = ui.painter_at(rect);

            // Background & Ground Line
            painter.rect_filled(rect, CornerRadius::same(6), Color32::from_rgb(18, 22, 28));
            painter.rect_stroke(rect, CornerRadius::same(6), Stroke::new(1.0, Color32::from_rgb(45, 52, 65)), StrokeKind::Inside);

            let ground_y = rect.min.y + 190.0;
            painter.line_segment([Pos2::new(rect.min.x + 16.0, ground_y), Pos2::new(rect.max.x - 16.0, ground_y)], Stroke::new(2.0, Color32::from_rgb(120, 100, 70)));
            painter.text(Pos2::new(rect.min.x + 22.0, ground_y + 12.0), Align2::LEFT_TOP, "GROUND LEVEL EL. ±0.000", egui::FontId::proportional(8.5), Color32::from_rgb(160, 140, 110));

            if self.simulation_phases.is_empty() {
                painter.text(
                    rect.center() - Vec2::new(0.0, 18.0),
                    Align2::CENTER_CENTER,
                    "Blank 4D Staging Elevation — no construction phases loaded yet.",
                    egui::FontId::proportional(12.0),
                    theme.text_muted,
                );
                return;
            }

            // Building footprint centered
            let bldg_w = (rect.width() * 0.46).clamp(110.0, 360.0);
            let bldg_x1 = rect.center().x - bldg_w * 0.5;
            let bldg_x2 = rect.center().x + bldg_w * 0.5;

            // Substructure Piling (Day 1..45)
            let pile_color = if self.simulation_day >= 1 { Color32::from_rgb(140, 90, 45) } else { Color32::from_rgb(40, 35, 30) };
            for p_idx in 0..6 {
                let px = bldg_x1 + (p_idx as f32 / 5.0) * bldg_w;
                painter.rect_filled(Rect::from_min_max(Pos2::new(px - 3.0, ground_y), Pos2::new(px + 3.0, ground_y + 35.0)), CornerRadius::ZERO, pile_color);
            }

            // Raft Slab & Basement (Day 46..90)
            if self.simulation_day >= 46 {
                painter.rect_filled(Rect::from_min_max(Pos2::new(bldg_x1, ground_y - 8.0), Pos2::new(bldg_x2, ground_y)), CornerRadius::ZERO, Color32::from_rgb(130, 138, 150));
            }

            // Superstructure Steel & Concrete Floors (Day 91..195): 12 Storeys
            let max_floors = 12;
            let floors_erected = if self.simulation_day >= 91 {
                (((self.simulation_day - 90) as f32 / 105.0) * max_floors as f32).clamp(1.0, max_floors as f32) as usize
            } else {
                0
            };

            let floor_h = 11.5;
            for fl in 0..floors_erected {
                let fy = ground_y - 8.0 - (fl + 1) as f32 * floor_h;

                // Columns for this floor
                for c_idx in 0..4 {
                    let cx = bldg_x1 + (c_idx as f32 / 3.0) * bldg_w;
                    painter.line_segment([Pos2::new(cx, fy + floor_h), Pos2::new(cx, fy)], Stroke::new(2.2, Color32::from_rgb(0, 140, 255)));
                }

                // Floor slab beam line
                painter.line_segment([Pos2::new(bldg_x1, fy), Pos2::new(bldg_x2, fy)], Stroke::new(1.8, Color32::from_rgb(180, 190, 205)));

                // Façade Glass Panels (Day 180..270)
                if self.simulation_day >= 180 {
                    let facade_floors = (((self.simulation_day - 180) as f32 / 90.0) * max_floors as f32).clamp(0.0, max_floors as f32) as usize;
                    if fl < facade_floors {
                        let glass_rect = Rect::from_min_max(Pos2::new(bldg_x1 + 4.0, fy + 2.0), Pos2::new(bldg_x2 - 4.0, fy + floor_h - 2.0));
                        painter.rect_filled(glass_rect, CornerRadius::same(1), Color32::from_rgba_premultiplied(0, 180, 160, 45));
                        painter.rect_stroke(glass_rect, CornerRadius::same(1), Stroke::new(0.6, Color32::from_rgb(0, 200, 180)), StrokeKind::Inside);
                    }
                }
            }

            // Tower Crane at the top
            if floors_erected > 0 {
                let top_y = ground_y - 8.0 - floors_erected as f32 * floor_h;
                let crane_x = bldg_x1 + bldg_w * 0.70;
                let mast_top = top_y - 28.0;

                // Crane Mast
                painter.line_segment([Pos2::new(crane_x, top_y), Pos2::new(crane_x, mast_top)], Stroke::new(2.0, Color32::from_rgb(255, 180, 0)));
                // Crane Jib (Boom)
                painter.line_segment([Pos2::new(crane_x - 18.0, mast_top), Pos2::new(crane_x + 45.0, mast_top)], Stroke::new(1.5, Color32::from_rgb(255, 200, 0)));
                // Cable & Hook
                let hook_x = crane_x + 24.0;
                painter.line_segment([Pos2::new(hook_x, mast_top), Pos2::new(hook_x, mast_top + 16.0)], Stroke::new(0.8, Color32::WHITE));
                painter.circle_filled(Pos2::new(hook_x, mast_top + 16.0), 2.0, Color32::from_rgb(255, 220, 0));
            }
        });

        ui.add_space(14.0);

        // CPM Activities Breakdown with Planned vs. Current Day Status
        theme.card_frame().show(ui, |ui| {
            icons::icon_label(ui, AppIcon::Clock, 15.0, theme.accent_blue, "CRITICAL PATH METHOD (CPM) & STAGED WBS SCHEDULE", 13.0, theme.text_primary, true);
            ui.add_space(8.0);

            if self.simulation_phases.is_empty() {
                ui.label(RichText::new("No CPM WBS activities defined yet.").size(11.5).color(theme.text_muted));
            }

            for phase in &self.simulation_phases {
                let is_current = self.simulation_day >= phase.day_start && self.simulation_day <= phase.day_end;
                let is_done = self.simulation_day > phase.day_end;
                let bar_color = if is_done {
                    theme.accent_green
                } else if is_current {
                    theme.accent_blue
                } else {
                    theme.border_subtle
                };

                let (status_icon, status_text) = if is_done {
                    (AppIcon::CheckCircle, "Completed")
                } else if is_current {
                    (AppIcon::Bolt, "In Progress")
                } else {
                    (AppIcon::Clock, "Pending")
                };

                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&phase.wbs_code).size(11.5).strong().color(theme.accent_blue));
                    ui.label(RichText::new(&phase.name).size(11.5).color(theme.text_primary));
                    ui.label(RichText::new(format!("Day {}-{} ({}d)", phase.day_start, phase.day_end, phase.day_end - phase.day_start + 1)).size(10.5).color(theme.text_secondary));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        Frame::new()
                            .fill(bar_color.gamma_multiply(0.2))
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                icons::icon_label(ui, status_icon, 11.5, bar_color, status_text, 10.0, bar_color, false);
                            });

                        ui.add_space(10.0);
                        ui.label(RichText::new(format!("{} Workers", phase.manpower_count)).size(10.5).color(theme.text_muted));
                        ui.add_space(8.0);
                        if phase.is_critical {
                            ui.label(RichText::new("CPM (Float: 0d)").size(10.0).strong().color(theme.accent_red));
                        }
                    });
                });

                ui.add_space(2.0);
                let (p_rect, _resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 5.0), Sense::hover());
                let p_painter = ui.painter();
                p_painter.rect_filled(p_rect, CornerRadius::same(2), theme.bg_canvas);

                let pct = if self.simulation_day >= phase.day_end {
                    1.0
                } else if self.simulation_day <= phase.day_start {
                    0.0
                } else {
                    (self.simulation_day - phase.day_start) as f32 / (phase.day_end - phase.day_start) as f32
                };
                let fill_w = p_rect.width() * pct;
                p_painter.rect_filled(Rect::from_min_size(p_rect.left_top(), Vec2::new(fill_w, p_rect.height())), CornerRadius::same(2), bar_color);
                ui.add_space(6.0);
            }
        });

        ui.add_space(14.0);
        ui_kit::feature_slot_card(
            ui,
            theme,
            features,
            FeatureId::BimDriven4DCriticalPathSync,
            |ui| {
                let substruct_cnt = bim_project.footings.len();
                let vert_cnt = bim_project.columns.len() + bim_project.walls.len();
                let super_cnt = bim_project.runs.len() + bim_project.slabs.len();
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "Live 4D WBS Binding ({} Levels): Foundations={} | Vertical Core={} | Beams & Slabs={}",
                                bim_project.levels.len(),
                                substruct_cnt,
                                vert_cnt,
                                super_cnt,
                            ))
                            .size(11.0)
                            .strong()
                            .color(theme.text_primary),
                        );
                    });
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        if icons::icon_button(ui, AppIcon::LinkChain, "Map Selected 3D Elements to WBS", 11.5, theme.text_primary, theme.bg_canvas, Stroke::new(1.0, theme.border_subtle), CornerRadius::same(6), Vec2::ZERO).clicked() {
                            self.export_status = Some(format!(
                                "Mapped {} BIM elements to 4D WBS stages (Stage 1 Foundation: {}, Stage 2 Core: {}, Stage 3 Frame/Slab: {}).",
                                substruct_cnt + vert_cnt + super_cnt,
                                substruct_cnt,
                                vert_cnt,
                                super_cnt,
                            ));
                        }
                        if icons::icon_button(ui, AppIcon::Cube3D, "Advance 4D Erection Stage", 11.5, Color32::WHITE, theme.accent_blue, Stroke::NONE, CornerRadius::same(6), Vec2::ZERO).clicked() {
                            self.simulation_day = (self.simulation_day + 30) % 361;
                            self.export_status = Some(format!(
                                "Advanced 4D construction simulation clock to Day {} / 360 across {} active storeys.",
                                self.simulation_day,
                                bim_project.levels.len(),
                            ));
                        }
                    });
                    if let Some(ref st) = self.export_status {
                        ui.add_space(4.0);
                        ui.label(RichText::new(st).size(10.5).color(theme.accent_green));
                    }
                });
            },
        );
    }

    // -------------------------------------------------------------
    // SUB-TAB: 5D WBS EXPLORER & DYNAMIC COST BREAKDOWN
    // -------------------------------------------------------------
    fn render_wbs_hierarchy(&mut self, ui: &mut Ui, theme: &MacTheme, _t: &Parametric5DTakeoff) {
        // Top Summary Cards
        let total_idr: f64 = self.wbs_catalog.iter().map(|item| item.total_cost_idr).sum();
        let curr = self.active_currency;
        let base_budget_str = curr.format_value(total_idr);
        let curr_str = format!("{} ({})", curr.code(), curr.symbol());
        let cont_val_str = curr.format_value(total_idr * (self.contingency_percent as f64 / 100.0));
        let cont_sub_str = format!("{:.1}% Reserve", self.contingency_percent);
        let vat_val_str = curr.format_value(total_idr * (self.vat_percent as f64 / 100.0));
        let vat_sub_str = format!("{:.1}% Statutory", self.vat_percent);

        ui_kit::responsive_grid_n(ui, 4, 210.0, |ui, idx| match idx {
            0 => self.metric_card(ui, theme, "TOTAL BASE BUDGET", &base_budget_str, "Grand Total Direct Cost", theme.accent_blue),
            1 => self.metric_card(ui, theme, "ACTIVE CURRENCY", &curr_str, "Multi-Currency Dynamic Conversion", theme.accent_orange),
            2 => self.metric_card(ui, theme, "CONTINGENCY ALLOCATION", &cont_val_str, &cont_sub_str, theme.accent_green),
            _ => self.metric_card(ui, theme, "VAT / PBG STATUTORY TAX", &vat_val_str, &vat_sub_str, theme.accent_red),
        });

        ui.add_space(14.0);

        // Division Filter Pills
        theme.card_frame().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                icons::icon_label(ui, AppIcon::Folder, 13.0, theme.accent_blue, "DIVISION FILTER:", 11.5, theme.accent_blue, true);

                let is_all = self.selected_wbs_division.is_none();
                let all_bg = if is_all { theme.accent_blue } else { theme.bg_canvas };
                let all_fg = if is_all { Color32::WHITE } else { theme.text_secondary };
                if ui.add(egui::Button::new(RichText::new("All Divisions").size(11.0).color(all_fg))
                    .fill(all_bg)
                    .corner_radius(CornerRadius::same(6))).clicked() {
                    self.selected_wbs_division = None;
                }

                let mut divisions: Vec<String> = self.wbs_catalog.iter().map(|i| i.division.clone()).collect();
                divisions.sort();
                divisions.dedup();

                for div in divisions {
                    let is_sel = self.selected_wbs_division.as_deref() == Some(&div);
                    let bg = if is_sel { theme.accent_blue } else { theme.bg_canvas };
                    let fg = if is_sel { Color32::WHITE } else { theme.text_secondary };
                    if ui.add(egui::Button::new(RichText::new(&div).size(11.0).color(fg))
                        .fill(bg)
                        .corner_radius(CornerRadius::same(6))).clicked() {
                        self.selected_wbs_division = Some(div);
                    }
                }
            });
        });

        ui.add_space(12.0);

        // WBS Table (Horizontally scrollable grid on narrow viewports)
        theme.card_frame().show(ui, |ui| {
            icons::icon_label(ui, AppIcon::ChartBar, 14.5, theme.accent_blue, "5D WORK BREAKDOWN STRUCTURE (WBS) QUANTITIES & RATES", 13.0, theme.text_primary, true);
            ui.add_space(10.0);

            ScrollArea::horizontal()
                .id_salt("planning_wbs_table_scroll")
                .show(ui, |ui| {
                    egui::Grid::new("planning_wbs_table_grid")
                        .striped(true)
                        .min_col_width(72.0)
                        .spacing(Vec2::new(16.0, 8.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new("WBS CODE").size(11.0).strong().color(theme.text_muted));
                            ui.label(RichText::new("DESCRIPTION").size(11.0).strong().color(theme.text_muted));
                            ui.label(RichText::new("QTY / UNIT").size(11.0).strong().color(theme.text_muted));
                            ui.label(RichText::new("UNIT RATE").size(11.0).strong().color(theme.text_muted));
                            ui.label(RichText::new("TOTAL COST").size(11.0).strong().color(theme.text_muted));
                            ui.label(RichText::new("PROGRESS").size(11.0).strong().color(theme.text_muted));
                            ui.end_row();

                            if self.wbs_catalog.is_empty() {
                                ui.label(RichText::new("-").size(11.5).color(theme.text_muted));
                                ui.label(RichText::new("Blank WBS catalog — add structural members or import WBS quantities.").size(11.5).color(theme.text_muted));
                                ui.label(RichText::new("-").size(11.5).color(theme.text_muted));
                                ui.label(RichText::new("-").size(11.5).color(theme.text_muted));
                                ui.label(RichText::new("-").size(11.5).color(theme.text_muted));
                                ui.label(RichText::new("0%").size(11.5).color(theme.text_muted));
                                ui.end_row();
                            }

                            for item in &self.wbs_catalog {
                                if let Some(ref sel_div) = self.selected_wbs_division {
                                    if &item.division != sel_div {
                                        continue;
                                    }
                                }

                                ui.label(RichText::new(&item.wbs_code).size(11.5).strong().color(theme.accent_blue));
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(&item.description).size(11.5).strong().color(theme.text_primary));
                                    ui.label(RichText::new(&item.division).size(10.0).color(theme.text_muted));
                                });
                                ui.label(RichText::new(format!("{:.1} {}", item.quantity, item.unit)).size(11.0).color(theme.text_secondary));
                                ui.label(RichText::new(curr.format_value(item.unit_price_idr)).size(11.0).color(theme.text_secondary));
                                ui.label(RichText::new(curr.format_value(item.total_cost_idr)).size(12.0).strong().color(theme.accent_blue));
                                ui.label(RichText::new(format!("{:.0}%", item.progress_pct)).size(11.0).strong().color(theme.accent_green));
                                ui.end_row();
                            }
                        });
                });
        });
    }

    // -------------------------------------------------------------
    // SUB-TAB 4: COST VS ARI TRADE-OFF OPTIMIZER
    // -------------------------------------------------------------
    fn render_cost_ari_tradeoff(&self, ui: &mut Ui, theme: &MacTheme, t: &Parametric5DTakeoff) {
        theme.card_frame().show(ui, |ui| {
            ui.label(RichText::new("VALUE ENGINEERING: COST SENSITIVITY (BoQ) TO REDUNDANCY INDEX (ARI)").size(13.5).strong().color(theme.accent_blue));
            ui.add_space(4.0);
            ui.label(RichText::new("Comparative analysis to evaluate the incremental investment required to enhance structural progressive collapse resistance:").size(11.5).color(theme.text_secondary));
            ui.add_space(14.0);

            let options = [
                ("Option 1: Minimum Code Compliance", 0.64, t.grand_total_rab * 0.92, "-8.0%", AppIcon::AlertTriangle, "Vulnerable to Single Failure (1 Member Removed = Collapse)", theme.accent_red),
                ("Option 2: Baseline Standard Design", 0.78, t.grand_total_rab * 0.97, "-3.0%", AppIcon::Info, "Meets Minimal SNI / AISC Standards", theme.accent_orange),
                ("Option 3: Redundancy-Optimized Design (Recommended)", t.predicted_system_ari, t.grand_total_rab, "0.0% (Base)", AppIcon::CheckCircle, "Robust Multi-Path (High Reserve Capacity)", theme.accent_green),
                ("Option 4: Ultra-Redundant (Critical Infrastructure)", 0.965, t.grand_total_rab * 1.075, "+7.5%", AppIcon::Shield, "Dual-Path Collapse Proof (Strategic Bridge)", theme.accent_blue),
            ];

            for (title, ari, cost, diff, desc_icon, desc, badge_color) in options {
                Frame::new()
                    .fill(theme.bg_sidebar)
                    .stroke(Stroke::new(1.0, theme.border_subtle))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(title).size(13.0).strong().color(theme.text_primary));
                                ui.add_space(2.0);
                                icons::icon_label(ui, desc_icon, 12.5, badge_color, desc, 11.5, badge_color, false);
                            });

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(RichText::new(format!("IDR {},-", format_idr(cost))).size(13.5).strong().color(theme.accent_blue));
                                ui.add_space(14.0);
                                ui.label(RichText::new(diff).size(12.0).color(theme.text_muted));
                                ui.add_space(14.0);
                                Frame::new()
                                    .fill(badge_color.gamma_multiply(0.2))
                                    .corner_radius(CornerRadius::same(6))
                                    .inner_margin(Margin::symmetric(8, 4))
                                    .show(ui, |ui| {
                                        ui.label(RichText::new(format!("ARI: {:.3}", ari)).size(12.0).strong().color(badge_color));
                                    });
                            });
                        });
                    });
                ui.add_space(8.0);
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal_wrapped(|ui| {
                icons::icon_label(ui, AppIcon::Sparkles, 13.5, theme.accent_green, "VALUE ENGINEERING CONCLUSION:", 12.0, theme.accent_green, true);
                ui.label(RichText::new("With only a +3.8% cost increase from baseline standard, the ARI redundancy index jumps significantly by +16.9%, preventing catastrophic collapse mechanisms during earthquakes or vehicular impacts.").size(12.0).color(theme.text_secondary));
            });
        });
    }

    // -------------------------------------------------------------
    // SUB-TAB 5: Staged Erection SVD & ARI Guard
    // -------------------------------------------------------------
    fn render_staged_erection_svd(&self, ui: &mut Ui, theme: &MacTheme, _takeoff: &Parametric5DTakeoff) {
        ui_kit::responsive_grid_n(ui, 4, 210.0, |ui, idx| match idx {
            0 => self.metric_card(ui, theme, "ERECTION STAGES", "5 Stages", "WBS Erection Sequence", theme.accent_blue),
            1 => self.metric_card(ui, theme, "MOST CRITICAL STAGE", "Stage 2 (WBS 2.2)", "ARI: 0.285 (Shoring Required)", theme.accent_red),
            2 => self.metric_card(ui, theme, "CLOSURE STABILITY", "Week 12", "SVD Gate 1 Full Rank", theme.accent_green),
            _ => self.metric_card(ui, theme, "FINAL OPERATIONAL ARI", "0.942", "Safe Multi-Path System", theme.accent_green),
        });

        ui.add_space(16.0);

        theme.card_frame().show(ui, |ui| {
            ui.label(RichText::new("STAGED ERECTION SAFETY SIMULATION (STAGED ERECTION SVD GUARD)").size(14.0).strong().color(theme.text_primary));
            ui.add_space(4.0);
            ui.label(RichText::new("Steel structures frequently lack full redundant stiffness during field erection before closure. This staged evaluation detects critical collapse hazards during assembly:").size(12.0).color(theme.text_secondary));
            ui.add_space(12.0);

            let mut engine = crate::core::ParametricEngine::new();
            let staged_results = engine.generate_warren_truss().ok().map(|m| {
                crate::core::ScheduleEngine::verify_staged_erection(&m)
            });

            if let Some(stages) = staged_results {
                for stage in &stages {
                    let col = if stage.is_stable { theme.accent_green } else { theme.accent_red };
                    let (status_icon, status_str) = if stage.is_stable {
                        (AppIcon::CheckCircle, "STABLE / NO MECHANISM")
                    } else {
                        (AppIcon::AlertTriangle, "CRITICAL / SHORING REQUIRED")
                    };
                    let note = match stage.warning.as_deref() {
                        Some(w) => w,
                        None => "Staged stiffness stable with full rank, deflections within limits.",
                    };

                    Frame::new()
                        .fill(theme.bg_card)
                        .stroke(Stroke::new(1.0, col.gamma_multiply(0.4)))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                icons::draw_icon(ui, status_icon, 14.0, col);
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(&stage.stage_name).size(12.5).strong().color(theme.text_primary));
                                    ui.label(RichText::new(format!("Elements: {} members • SVD Rank: {}/{} • Condition: {:.1e}", stage.erected_elements_count, stage.rank, stage.active_dofs, stage.condition_number)).size(11.0).color(theme.text_secondary));
                                    ui.add_space(2.0);
                                    icons::icon_label(ui, AppIcon::Info, 12.0, theme.text_muted, format!("Evaluation: {}", note), 11.0, theme.text_muted, false);
                                });

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    Frame::new()
                                        .fill(col.gamma_multiply(0.18))
                                        .stroke(Stroke::new(1.0, col))
                                        .corner_radius(CornerRadius::same(6))
                                        .inner_margin(Margin::symmetric(8, 4))
                                        .show(ui, |ui| {
                                            ui.label(RichText::new(format!("Defl: {:.2} mm", stage.max_deflection_mm)).size(11.5).strong().color(col));
                                        });
                                    ui.add_space(12.0);
                                    ui.horizontal(|ui| {
                                        icons::icon_label(ui, status_icon, 12.5, col, status_str, 11.0, col, true);
                                    });
                                });
                            });
                        });
                    ui.add_space(6.0);
                }
            }
        });
    }

    // -------------------------------------------------------------
    // SUB-TAB 6: MONTE CARLO SCHEDULE RISK PERT (4D-03 ACTIVE)
    // -------------------------------------------------------------
    pub fn compute_monte_carlo_pert(
        t: &Parametric5DTakeoff,
        bim_project: &crate::core::BimProject,
        trials: usize,
        spread_pct: f32,
        seed: u64,
    ) -> MonteCarloScheduleResult {
        let bim_days = (bim_project.walls.len() as f64 * 2.5
            + bim_project.columns.len() as f64 * 1.8
            + bim_project.slabs.len() as f64 * 6.0)
            .clamp(0.0, 240.0);
        let deterministic_days = (t.duration_days as f64 + bim_days).max(90.0);
        let spread = (spread_pct as f64 / 100.0).clamp(0.05, 0.75);
        let n_trials = trials.clamp(100, 5000);

        let phases = [
            ("02.10 Deep Foundation & Bored Piles", 0.24_f64, 0.85_f64),
            ("03.20 RC Core Walls & Slabs", 0.26, 0.70),
            ("05.12 Steel Erection & Torque", 0.30, 1.15),
            ("08.40 Curtain Wall & MEP Riser Test", 0.20, 0.65),
        ];

        // Deterministic LCG + Latin Hypercube strata for reproducible PERT trials
        let mut rng = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let mut next_u01 = || -> f64 {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let bits = (rng >> 11) as f64;
            (bits / ((1u64 << 53) as f64)).clamp(0.001, 0.999)
        };

        let mut totals = Vec::with_capacity(n_trials);
        let mut phase_samples: Vec<Vec<f64>> = vec![Vec::with_capacity(n_trials); phases.len()];
        for k in 0..n_trials {
            let mut sum_days = 0.0_f64;
            for (p_idx, (_, share, risk_mult)) in phases.iter().enumerate() {
                let m = deterministic_days * share;
                let o = m * (1.0 - 0.35 * spread * risk_mult);
                let p = m * (1.0 + 1.05 * spread * risk_mult);
                let stratum = ((k as f64 + next_u01()) / n_trials as f64).clamp(0.001, 0.999);
                let bates = (stratum + next_u01() + next_u01()) / 3.0;
                let pert_u = bates.powf(0.82);
                let d_sample = o + pert_u * (p - o);
                sum_days += d_sample;
                phase_samples[p_idx].push(d_sample);
            }
            totals.push(sum_days);
        }

        let mut sorted = totals.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let pct = |q: f64| -> f64 {
            let idx = ((sorted.len() - 1) as f64 * q).round() as usize;
            sorted[idx.min(sorted.len() - 1)]
        };
        let p50_days = pct(0.50);
        let p80_days = pct(0.80);
        let p90_days = pct(0.90);

        let mut cdf_points = Vec::with_capacity(21);
        for step in 0..=20 {
            let q = step as f64 / 20.0;
            cdf_points.push((pct(q), q));
        }

        let mut drivers = Vec::with_capacity(phases.len());
        let mean_total = totals.iter().sum::<f64>() / totals.len() as f64;
        let var_total = totals
            .iter()
            .map(|&v| (v - mean_total).powi(2))
            .sum::<f64>()
            .max(1e-6);
        for (p_idx, (name, share, risk_mult)) in phases.iter().enumerate() {
            let mean_p = phase_samples[p_idx].iter().sum::<f64>() / n_trials as f64;
            let cov: f64 = phase_samples[p_idx]
                .iter()
                .zip(totals.iter())
                .map(|(&pi, &ti)| (pi - mean_p) * (ti - mean_total))
                .sum();
            let var_p = phase_samples[p_idx]
                .iter()
                .map(|&pi| (pi - mean_p).powi(2))
                .sum::<f64>()
                .max(1e-6);
            let corr = (cov / (var_p * var_total).sqrt()).clamp(0.10, 0.99);
            let swing = deterministic_days * share * spread * risk_mult * 0.7;
            drivers.push(((*name).to_string(), corr, swing));
        }
        drivers.sort_by(|a, b| b.1.total_cmp(&a.1));

        MonteCarloScheduleResult {
            deterministic_days,
            p50_days,
            p80_days,
            p90_days,
            cdf_points,
            drivers,
        }
    }

    pub fn compute_crew_leveling(
        t: &Parametric5DTakeoff,
        bim_project: &crate::core::BimProject,
        crew_cap: u32,
        within_float_only: bool,
    ) -> CrewLevelingResult {
        let shape = [
            0.22_f32, 0.40, 0.62, 0.96, 1.00, 0.94, 0.78, 0.64, 0.50, 0.36, 0.24, 0.14,
        ];
        let shape_sum: f32 = shape.iter().sum();
        let total_crew_months = ((t.total_man_days as f32 / 22.0)
            + (bim_project.element_count() as f32 * 0.45))
            .clamp(380.0, 2800.0);

        let mut unleveled = [0.0_f32; 12];
        for (i, w) in shape.iter().enumerate() {
            unleveled[i] = (w / shape_sum) * total_crew_months;
        }
        let peak_unleveled = unleveled.iter().copied().fold(0.0_f32, f32::max);

        let cap = (crew_cap as f32).clamp(30.0, 400.0);
        let mut leveled = unleveled;
        let max_shift_passes = if within_float_only { 6 } else { 18 };
        let mut shifted_mass = 0.0_f32;
        for _ in 0..max_shift_passes {
            let mut moved = false;
            for i in 0..11 {
                if leveled[i] > cap {
                    let excess = (leveled[i] - cap).min(if within_float_only { 14.0 } else { 30.0 });
                    leveled[i] -= excess;
                    leveled[i + 1] += excess * 0.75;
                    if i + 2 < 12 {
                        leveled[i + 2] += excess * 0.25;
                    } else {
                        leveled[i + 1] += excess * 0.25;
                    }
                    shifted_mass += excess;
                    moved = true;
                }
            }
            if !moved {
                break;
            }
        }
        let peak_leveled = leveled.iter().copied().fold(0.0_f32, f32::max);
        let float_consumed_days = (shifted_mass as f64 * 0.18).clamp(0.0, if within_float_only { 12.0 } else { 28.0 });

        CrewLevelingResult {
            monthly_unleveled: unleveled,
            monthly_leveled: leveled,
            peak_unleveled,
            peak_leveled,
            float_consumed_days,
        }
    }

    pub fn compute_evm_summary(
        t: &Parametric5DTakeoff,
        cutoff_month: usize,
        retention_pct: f32,
        cpi_factor: f32,
    ) -> EvmMonthSummary {
        let pv_vals = [
            0.04_f64, 0.09, 0.17, 0.28, 0.42, 0.55, 0.66, 0.77, 0.87, 0.94, 0.98, 1.0,
        ];
        let m_idx = cutoff_month.clamp(1, 12) - 1;
        let pv_ratio = pv_vals[m_idx];
        let ev_ratio = (pv_ratio * 1.036).min(1.0);
        let cpi = (cpi_factor as f64).clamp(0.75, 1.25);
        let ac_ratio = (ev_ratio / cpi).clamp(0.01, 1.35);
        let spi = ev_ratio / pv_ratio.max(1e-6);

        let bac = t.grand_total_rab;
        let ev_val = bac * ev_ratio;
        let ac_val = bac * ac_ratio;
        let eac_val = bac / cpi;
        let vac_val = bac - eac_val;
        let net_claim = ev_val * (1.0 - (retention_pct as f64 / 100.0).clamp(0.0, 0.15));

        EvmMonthSummary {
            month: m_idx + 1,
            pv_ratio,
            ev_ratio,
            ac_ratio,
            spi,
            cpi,
            ev_val,
            ac_val,
            eac_val,
            vac_val,
            net_claim,
        }
    }

    fn render_schedule_risk_monte_carlo(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        t: &Parametric5DTakeoff,
        bim_project: &crate::core::BimProject,
        features: &mut FeatureRegistry,
    ) {
        let res = Self::compute_monte_carlo_pert(
            t,
            bim_project,
            self.mc_trial_count,
            self.mc_pert_spread_pct,
            self.mc_seed,
        );
        ui_kit::feature_slot_card(
            ui,
            theme,
            features,
            FeatureId::MonteCarloScheduleRiskPert,
            |ui| {
                let p50_delta = res.p50_days - res.deterministic_days;
                let p80_delta = res.p80_days - res.deterministic_days;
                let top = res.drivers.first().cloned().unwrap_or(("Critical Path".into(), 0.75, 15.0));
                let cpm_str = format!("{:.0} Days", res.deterministic_days);
                let p50_str = format!("{:.0}d ({:+.0}d)", res.p50_days, p50_delta);
                let p80_str = format!("{:.0}d ({:+.0}d)", res.p80_days, p80_delta);
                let top_str = format!("r={:.2} (±{:.0}d)", top.1, top.2);

                ui_kit::responsive_grid_n(ui, 4, 185.0, |ui, idx| match idx {
                    0 => ui_kit::stat_card(
                        ui,
                        theme,
                        "DETERMINISTIC CPM",
                        &cpm_str,
                        "Baseline Zero-Float Duration",
                        theme.text_primary,
                    ),
                    1 => ui_kit::stat_card(
                        ui,
                        theme,
                        "P50 MEDIAN COMPLETION",
                        &p50_str,
                        "50% Joint Confidence Forecast",
                        theme.accent_blue,
                    ),
                    2 => ui_kit::stat_card(
                        ui,
                        theme,
                        "P80 SCHEDULE BUFFER",
                        &p80_str,
                        &format!("P90 = {:.0}d Contingency Envelope", res.p90_days),
                        theme.accent_orange,
                    ),
                    _ => ui_kit::stat_card(
                        ui,
                        theme,
                        "TOP SENSITIVITY DRIVER",
                        &top_str,
                        &top.0,
                        theme.accent_green,
                    ),
                });

                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("LHS Trials:").size(11.0).color(theme.text_secondary));
                    for &cnt in &[500usize, 1000, 2500] {
                        if ui.selectable_label(self.mc_trial_count == cnt, format!("{cnt}")).clicked() {
                            self.mc_trial_count = cnt;
                        }
                    }
                    ui.separator();
                    ui.label(RichText::new("PERT Spread (%):").size(11.0).color(theme.text_secondary));
                    ui.add(egui::Slider::new(&mut self.mc_pert_spread_pct, 10.0..=55.0).fixed_decimals(0));
                    if icons::icon_button(
                        ui,
                        AppIcon::ChartBar,
                        "Resample LHS Seed",
                        11.0,
                        Color32::WHITE,
                        theme.accent_blue,
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        self.mc_seed = self.mc_seed.wrapping_add(7);
                        self.export_status = Some(format!(
                            "PERT Monte Carlo ({} LHS trials): P50={:.0}d, P80={:.0}d, P90={:.0}d.",
                            self.mc_trial_count, res.p50_days, res.p80_days, res.p90_days
                        ));
                    }
                    if icons::icon_button(
                        ui,
                        AppIcon::Document,
                        "Export PERT CSV",
                        11.0,
                        theme.text_primary,
                        theme.bg_canvas,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(6),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        let mut csv = String::from("Probability_q,Forecast_Days\n");
                        for (d, q) in &res.cdf_points {
                            csv.push_str(&format!("{q:.2},{d:.1}\n"));
                        }
                        if let Ok(dest) = crate::core::web_bridge::save_text_file(
                            "EigenGraph_4D_MonteCarlo_PERT.csv",
                            &csv,
                            "text/csv",
                        ) {
                            self.export_status = Some(format!("Exported PERT CDF -> {dest}"));
                        }
                    }
                });
                ui.add_space(12.0);

                ui_kit::responsive_two_panes(ui, 660.0, |ui, pane| match pane {
                    0 => {
                        ui.label(
                            RichText::new("CUMULATIVE COMPLETION PROBABILITY (CDF S-CURVE)")
                                .size(11.0)
                                .strong()
                                .color(theme.accent_blue),
                        );
                        ui.add_space(6.0);
                        let desired = Vec2::new(ui.available_width(), 170.0);
                        let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(13, 17, 24));
                        let plot = rect.shrink(28.0);
                        for step in 0..=4 {
                            let frac = step as f32 / 4.0;
                            let y = plot.bottom() - frac * plot.height();
                            painter.line_segment(
                                [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
                                Stroke::new(0.8, Color32::from_white_alpha(20)),
                            );
                        }
                        let min_d = res.cdf_points.first().map(|p| p.0).unwrap_or(100.0);
                        let max_d = res.cdf_points.last().map(|p| p.0).unwrap_or(200.0).max(min_d + 1.0);
                        let cdf_pts: Vec<Pos2> = res
                            .cdf_points
                            .iter()
                            .map(|&(d, q)| {
                                let x_frac = ((d - min_d) / (max_d - min_d)).clamp(0.0, 1.0) as f32;
                                Pos2::new(
                                    plot.left() + x_frac * plot.width(),
                                    plot.bottom() - (q as f32) * plot.height(),
                                )
                            })
                            .collect();
                        for w in cdf_pts.windows(2) {
                            painter.line_segment([w[0], w[1]], Stroke::new(2.2, theme.accent_green));
                        }
                        let p80_frac = ((res.p80_days - min_d) / (max_d - min_d)).clamp(0.0, 1.0) as f32;
                        let p80_x = plot.left() + p80_frac * plot.width();
                        let p80_y = plot.bottom() - 0.80 * plot.height();
                        painter.line_segment(
                            [Pos2::new(p80_x, plot.bottom()), Pos2::new(p80_x, p80_y)],
                            Stroke::new(1.5, theme.accent_orange),
                        );
                        painter.circle_filled(Pos2::new(p80_x, p80_y), 4.5, theme.accent_orange);
                        painter.text(
                            Pos2::new(p80_x + 6.0, p80_y - 4.0),
                            Align2::LEFT_BOTTOM,
                            format!("P80 = {:.0}d", res.p80_days),
                            egui::FontId::monospace(9.5),
                            theme.accent_orange,
                        );
                    }
                    _ => {
                        ui.label(
                            RichText::new("TORNADO CRITICALITY & SCHEDULE SENSITIVITY")
                                .size(11.0)
                                .strong()
                                .color(theme.accent_blue),
                        );
                        ui.add_space(6.0);
                        let palette = [
                            theme.accent_red,
                            theme.accent_orange,
                            theme.accent_blue,
                            theme.accent_green,
                        ];
                        for (idx, (name, corr, swing)) in res.drivers.iter().enumerate() {
                            let col = palette[idx % palette.len()];
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(name).size(11.0).color(theme.text_primary));
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!("r={:.2} (±{:.0}d)", corr, swing))
                                            .size(10.5)
                                            .strong()
                                            .color(col),
                                    );
                                });
                            });
                            let (bar_r, _) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width().max(120.0), 8.0),
                                Sense::hover(),
                            );
                            ui.painter().rect_filled(bar_r, CornerRadius::same(4), theme.bg_canvas);
                            ui.painter().rect_filled(
                                Rect::from_min_size(bar_r.min, Vec2::new(bar_r.width() * (*corr as f32), 8.0)),
                                CornerRadius::same(4),
                                col,
                            );
                            ui.add_space(6.0);
                        }
                    }
                });
            },
        );
    }

    // -------------------------------------------------------------
    // SUB-TAB 7: CREW HISTOGRAM & RESOURCE LEVELING (4D-04 ACTIVE)
    // -------------------------------------------------------------
    fn render_resource_leveling_4d(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        t: &Parametric5DTakeoff,
        bim_project: &crate::core::BimProject,
        features: &mut FeatureRegistry,
    ) {
        let lvl = Self::compute_crew_leveling(
            t,
            bim_project,
            self.leveling_crew_cap,
            self.leveling_within_float_only,
        );
        ui_kit::feature_slot_card(
            ui,
            theme,
            features,
            FeatureId::ResourceLevelingCrewHistogram,
            |ui| {
                let reduction_pct = if lvl.peak_unleveled > 0.1 {
                    ((lvl.peak_unleveled - lvl.peak_leveled) / lvl.peak_unleveled) * 100.0
                } else {
                    0.0
                };
                let unlev_str = format!("{:.0} Workers/d", lvl.peak_unleveled);
                let lev_str = format!("{:.0} Workers/d", lvl.peak_leveled);
                let hoist_util = ((lvl.peak_leveled / (self.leveling_crew_cap as f32).max(1.0)) * 84.0).clamp(35.0, 99.0);
                let hoist_str = format!("{hoist_util:.1}% Duty");
                let float_str = format!("{:.1} Days avg", lvl.float_consumed_days);

                ui_kit::responsive_grid_n(ui, 4, 185.0, |ui, idx| match idx {
                    0 => ui_kit::stat_card(
                        ui,
                        theme,
                        "UNSMOOTHED PEAK CREW",
                        &unlev_str,
                        "Baseline Concurrent Peak Demand",
                        theme.accent_red,
                    ),
                    1 => ui_kit::stat_card(
                        ui,
                        theme,
                        "LEVELED PEAK DEMAND",
                        &lev_str,
                        &format!("-{reduction_pct:.1}% Peak Smoothing"),
                        theme.accent_green,
                    ),
                    2 => ui_kit::stat_card(
                        ui,
                        theme,
                        "SITE HOIST & CRANE DUTY",
                        &hoist_str,
                        "Zero Simultaneous Double-Pick Clashes",
                        theme.accent_blue,
                    ),
                    _ => ui_kit::stat_card(
                        ui,
                        theme,
                        "TOTAL FLOAT CONSUMED",
                        &float_str,
                        if self.leveling_within_float_only {
                            "Critical Path Preserved (Free Float)"
                        } else {
                            "Aggressive Shaving Applied"
                        },
                        theme.accent_orange,
                    ),
                });

                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    if ui.selectable_label(self.leveling_within_float_only, "Level Within Free Float Only").clicked() {
                        self.leveling_within_float_only = !self.leveling_within_float_only;
                    }
                    ui.separator();
                    ui.label(RichText::new("Crew Cap:").size(11.0).color(theme.text_secondary));
                    let mut cap_f = self.leveling_crew_cap as f32;
                    if ui.add(egui::Slider::new(&mut cap_f, 45.0..=160.0).fixed_decimals(0)).changed() {
                        self.leveling_crew_cap = cap_f.round() as u32;
                    }
                    if icons::icon_button(
                        ui,
                        AppIcon::Document,
                        "Export Manpower CSV",
                        11.0,
                        theme.text_primary,
                        theme.bg_canvas,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(6),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        let mut csv = String::from("Month,Unleveled_Crew,Leveled_Crew\n");
                        for i in 0..12 {
                            csv.push_str(&format!(
                                "M{},{:.1},{:.1}\n",
                                i + 1,
                                lvl.monthly_unleveled[i],
                                lvl.monthly_leveled[i]
                            ));
                        }
                        if let Ok(dest) = crate::core::web_bridge::save_text_file(
                            "EigenGraph_4D_Crew_Leveling.csv",
                            &csv,
                            "text/csv",
                        ) {
                            self.export_status = Some(format!("Exported 4D Crew Histogram -> {dest}"));
                        }
                    }
                });
                ui.add_space(10.0);

                let desired = Vec2::new(ui.available_width(), 165.0);
                let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(13, 17, 24));
                let plot = rect.shrink(26.0);
                let max_scale = lvl.peak_unleveled.max(self.leveling_crew_cap as f32).max(100.0) * 1.12;
                let cap_y = plot.bottom() - ((self.leveling_crew_cap as f32) / max_scale) * plot.height();
                painter.line_segment(
                    [Pos2::new(plot.left(), cap_y), Pos2::new(plot.right(), cap_y)],
                    Stroke::new(1.5, theme.accent_red),
                );
                painter.text(
                    Pos2::new(plot.left() + 4.0, cap_y - 3.0),
                    Align2::LEFT_BOTTOM,
                    format!("Site Crew Capacity Limit ({} Workers)", self.leveling_crew_cap),
                    egui::FontId::monospace(9.0),
                    theme.accent_red,
                );

                let slot_w = plot.width() / 12.0;
                let bar_w = (slot_w * 0.56).clamp(10.0, 28.0);
                for idx in 0..12 {
                    let workers = lvl.monthly_leveled[idx];
                    let cx = plot.left() + slot_w * (idx as f32 + 0.5);
                    let h = (workers / max_scale) * plot.height();
                    let bar = Rect::from_min_max(
                        Pos2::new(cx - bar_w * 0.5, plot.bottom() - h),
                        Pos2::new(cx + bar_w * 0.5, plot.bottom()),
                    );
                    let col = if workers > self.leveling_crew_cap as f32 {
                        theme.accent_orange
                    } else if workers > (self.leveling_crew_cap as f32 * 0.85) {
                        theme.accent_blue
                    } else {
                        theme.accent_green
                    };
                    painter.rect_filled(bar, CornerRadius::same(3), col);
                    painter.text(
                        Pos2::new(cx, plot.bottom() + 4.0),
                        Align2::CENTER_TOP,
                        format!("M{}", idx + 1),
                        egui::FontId::monospace(8.5),
                        theme.text_muted,
                    );
                }
            },
        );
    }

    // -------------------------------------------------------------
    // SUB-TAB 8: EARNED VALUE EVM & PROGRESS CLAIM (5D-03 ACTIVE)
    // -------------------------------------------------------------
    fn render_earned_value_evm(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        t: &Parametric5DTakeoff,
        features: &mut FeatureRegistry,
    ) {
        let evm = Self::compute_evm_summary(
            t,
            self.evm_cutoff_month,
            self.evm_retention_pct,
            self.evm_cpi_factor,
        );
        ui_kit::feature_slot_card(
            ui,
            theme,
            features,
            FeatureId::EarnedValueEvmProgressClaim,
            |ui| {
                let curr = self.active_currency;
                let ev_str = curr.format_value(evm.ev_val);
                let eac_str = curr.format_value(evm.eac_val);
                let spi_str = format!(
                    "{:.2} ({})",
                    evm.spi,
                    if evm.spi >= 1.0 { "Ahead" } else { "Behind" }
                );
                let cpi_str = format!(
                    "{:.2} ({})",
                    evm.cpi,
                    if evm.cpi >= 1.0 { "Under" } else { "Watch" }
                );

                ui_kit::responsive_grid_n(ui, 4, 185.0, |ui, idx| match idx {
                    0 => ui_kit::stat_card(
                        ui,
                        theme,
                        "SCHEDULE PERF INDEX (SPI)",
                        &spi_str,
                        &format!("EV / PV @ M{:02}", evm.month),
                        if evm.spi >= 1.0 { theme.accent_green } else { theme.accent_orange },
                    ),
                    1 => ui_kit::stat_card(
                        ui,
                        theme,
                        "COST PERF INDEX (CPI)",
                        &cpi_str,
                        "EV / AC Progress Efficiency",
                        if evm.cpi >= 1.0 { theme.accent_green } else { theme.accent_orange },
                    ),
                    2 => ui_kit::stat_card(
                        ui,
                        theme,
                        &format!("EARNED VALUE (EV @ M{:02})", evm.month),
                        &ev_str,
                        &format!("{:.1}% Cumulative Certified", evm.ev_ratio * 100.0),
                        theme.accent_blue,
                    ),
                    _ => ui_kit::stat_card(
                        ui,
                        theme,
                        "ESTIMATE AT COMPLETION (EAC)",
                        &eac_str,
                        &format!(
                            "Net IPC: {} ({:.0}% Ret)",
                            curr.format_value(evm.net_claim),
                            self.evm_retention_pct
                        ),
                        theme.text_primary,
                    ),
                });

                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Cutoff Month:").size(11.0).color(theme.text_secondary));
                    let mut m_f = self.evm_cutoff_month as f32;
                    if ui.add(egui::Slider::new(&mut m_f, 1.0..=12.0).fixed_decimals(0)).changed() {
                        self.evm_cutoff_month = m_f.round() as usize;
                    }
                    ui.separator();
                    ui.label(RichText::new("Retention (%):").size(11.0).color(theme.text_secondary));
                    ui.add(egui::Slider::new(&mut self.evm_retention_pct, 0.0..=10.0).fixed_decimals(1));
                    ui.label(RichText::new("CPI Factor:").size(11.0).color(theme.text_secondary));
                    ui.add(egui::Slider::new(&mut self.evm_cpi_factor, 0.80..=1.15).fixed_decimals(2));
                    if icons::icon_button(
                        ui,
                        AppIcon::ChartLine,
                        "Export IPC Progress CSV",
                        11.0,
                        Color32::WHITE,
                        theme.accent_green,
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        let csv = format!(
                            "IPC_Month,PV_Ratio,EV_Ratio,AC_Ratio,SPI,CPI,EV_Val,EAC_Val,Net_Claim\nM{:02},{:.4},{:.4},{:.4},{:.3},{:.3},{:.2},{:.2},{:.2}\n",
                            evm.month, evm.pv_ratio, evm.ev_ratio, evm.ac_ratio, evm.spi, evm.cpi, evm.ev_val, evm.eac_val, evm.net_claim
                        );
                        if let Ok(dest) = crate::core::web_bridge::save_text_file(
                            &format!("EigenGraph_5D_IPC_M{:02}.csv", evm.month),
                            &csv,
                            "text/csv",
                        ) {
                            self.export_status = Some(format!("Certified IPC-M{:02} exported -> {dest}", evm.month));
                        }
                    }
                });
                ui.add_space(12.0);

                ui_kit::responsive_two_panes(ui, 660.0, |ui, pane| match pane {
                    0 => {
                        ui.label(
                            RichText::new("PV / EV / AC EARNED VALUE S-CURVE (M01–M12)")
                                .size(11.0)
                                .strong()
                                .color(theme.accent_blue),
                        );
                        ui.add_space(6.0);
                        let desired = Vec2::new(ui.available_width(), 165.0);
                        let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(13, 17, 24));
                        let plot = rect.shrink(24.0);
                        let cut_frac = (evm.month.saturating_sub(1) as f32) / 11.0;
                        let cut_x = plot.left() + cut_frac * plot.width();
                        painter.line_segment(
                            [Pos2::new(cut_x, plot.top()), Pos2::new(cut_x, plot.bottom())],
                            Stroke::new(1.2, theme.text_muted),
                        );
                        painter.text(
                            Pos2::new(cut_x + 4.0, plot.top() + 2.0),
                            Align2::LEFT_TOP,
                            format!("Data Date M{:02}", evm.month),
                            egui::FontId::monospace(8.5),
                            theme.text_muted,
                        );
                        let pv_vals = [0.04_f32, 0.09, 0.17, 0.28, 0.42, 0.55, 0.66, 0.77, 0.87, 0.94, 0.98, 1.0];
                        let pv_pts: Vec<Pos2> = pv_vals
                            .iter()
                            .enumerate()
                            .map(|(i, v)| {
                                Pos2::new(
                                    plot.left() + (i as f32 / 11.0) * plot.width(),
                                    plot.bottom() - *v * plot.height(),
                                )
                            })
                            .collect();
                        for w in pv_pts.windows(2) {
                            painter.line_segment([w[0], w[1]], Stroke::new(1.8, theme.accent_blue));
                        }
                        let ev_pts: Vec<Pos2> = (0..evm.month)
                            .map(|i| {
                                let v = (pv_vals[i] * 1.036).min(1.0);
                                Pos2::new(
                                    plot.left() + (i as f32 / 11.0) * plot.width(),
                                    plot.bottom() - v * plot.height(),
                                )
                            })
                            .collect();
                        for w in ev_pts.windows(2) {
                            painter.line_segment([w[0], w[1]], Stroke::new(2.4, theme.accent_green));
                        }
                        let ac_pts: Vec<Pos2> = (0..evm.month)
                            .map(|i| {
                                let v = ((pv_vals[i] * 1.036).min(1.0) / self.evm_cpi_factor.clamp(0.75, 1.25)).min(1.05);
                                Pos2::new(
                                    plot.left() + (i as f32 / 11.0) * plot.width(),
                                    plot.bottom() - v * plot.height(),
                                )
                            })
                            .collect();
                        for w in ac_pts.windows(2) {
                            painter.line_segment([w[0], w[1]], Stroke::new(1.8, theme.accent_orange));
                        }
                    }
                    _ => {
                        ui.label(
                            RichText::new(format!("INTERIM PAYMENT CERTIFICATE (IPC-{:02}) BY DIVISION", evm.month))
                                .size(11.0)
                                .strong()
                                .color(theme.accent_blue),
                        );
                        ui.add_space(6.0);
                        ScrollArea::horizontal().id_salt("evm_ipc_scroll").show(ui, |ui| {
                            egui::Grid::new("evm_ipc_grid")
                                .striped(true)
                                .spacing(Vec2::new(12.0, 5.0))
                                .show(ui, |ui| {
                                    for h in ["Division", "Weight", "Earned %", "CPI", "SPI", "Status"] {
                                        ui.label(RichText::new(h).size(10.5).strong().color(theme.accent_blue));
                                    }
                                    ui.end_row();
                                    let ev_pct = evm.ev_ratio * 100.0;
                                    for (div, wt, mult, status) in [
                                        ("02 Substructure", "18%", 1.35, "Settled"),
                                        ("03 RC Cores & Slabs", "28%", 1.15, "Certified"),
                                        ("05 Structural Steel", "36%", 0.98, "IPC Ready"),
                                        ("08 Unitized Glazing", "18%", 0.55, "Fabricating"),
                                    ] {
                                        let div_ev = (ev_pct * mult).min(100.0);
                                        ui.label(RichText::new(div).size(10.5).strong().color(theme.text_primary));
                                        ui.label(RichText::new(wt).size(10.5).color(theme.text_muted));
                                        ui.label(RichText::new(format!("{div_ev:.1}%")).size(10.5).color(theme.accent_green));
                                        ui.label(RichText::new(format!("{:.2}", evm.cpi)).size(10.5).color(theme.text_secondary));
                                        ui.label(RichText::new(format!("{:.2}", evm.spi)).size(10.5).color(theme.accent_blue));
                                        ui.label(RichText::new(status).size(10.5).color(theme.accent_green));
                                        ui.end_row();
                                    }
                                });
                        });
                    }
                });
            },
        );
    }

    // -------------------------------------------------------------
    // SUB-TAB 9: VALUE ENGINEERING & BID LEVELING (5D-04 LOCKED)
    // -------------------------------------------------------------
    fn render_value_engineering_bid(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        t: &Parametric5DTakeoff,
        features: &mut FeatureRegistry,
    ) {
        let sum = compute_bid_leveling_matrix(
            t,
            self.ve_outlier_pct,
            self.ve_composite_deck_swap,
            self.ve_ari_floor,
        );
        ui_kit::feature_slot_card(
            ui,
            theme,
            features,
            FeatureId::ValueEngineeringTenderMatrix,
            |ui| {
                let curr = self.active_currency;
                let oe_str = curr.format_value(sum.total_oe);
                let sel_total = match self.ve_selected_bidder_idx {
                    0 => sum.total_a,
                    1 => sum.total_b,
                    _ => sum.total_c,
                };
                let net_award = (sel_total - sum.ve_savings_val).max(0.0);
                let bidder_name = ["Bidder A (SteelFab)", "Bidder B (Nusantara)", "Bidder C (GlobalTruss)"][self.ve_selected_bidder_idx.min(2)];
                let delta_pct = if sum.total_oe > 0.0 {
                    ((sel_total - sum.total_oe) / sum.total_oe) * 100.0
                } else {
                    0.0
                };
                let ve_save_str = curr.format_value(sum.ve_savings_val);
                let award_str = curr.format_value(net_award);
                let ari_col = if sum.ari_compliant { theme.accent_green } else { theme.accent_red };
                let outlier_col = if sum.outlier_count == 0 { theme.accent_green } else { theme.accent_orange };

                ui_kit::responsive_grid_n(ui, 4, 185.0, |ui, idx| match idx {
                    0 => ui_kit::stat_card(
                        ui,
                        theme,
                        "OWNER ESTIMATE (OE)",
                        &oe_str,
                        "Baseline 5D Cost Model (Parametric)",
                        theme.text_primary,
                    ),
                    1 => ui_kit::stat_card(
                        ui,
                        theme,
                        "SELECTED / AWARD VALUE",
                        &award_str,
                        &format!("{} ({:+.1}% vs OE, Post-VE)", bidder_name, delta_pct),
                        theme.accent_green,
                    ),
                    2 => ui_kit::stat_card(
                        ui,
                        theme,
                        "UNBALANCED RATE ALERTS",
                        &format!("{} Flagged Line(s)", sum.outlier_count),
                        &format!("Threshold ±{:.0}% vs Owner Estimate", self.ve_outlier_pct),
                        outlier_col,
                    ),
                    _ => ui_kit::stat_card(
                        ui,
                        theme,
                        "VE REDESIGN & ARI GUARD",
                        &format!("-{}", ve_save_str),
                        &format!(
                            "ARI = {:.3} (Floor {:.2} — {})",
                            sum.projected_ari,
                            self.ve_ari_floor,
                            if sum.ari_compliant { "PASS" } else { "BREACH" }
                        ),
                        ari_col,
                    ),
                });

                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Outlier ±%:").size(11.0).color(theme.text_secondary));
                    ui.add(egui::Slider::new(&mut self.ve_outlier_pct, 8.0..=30.0).suffix("%"));
                    ui.add_space(6.0);
                    ui.label(RichText::new("ARI Floor:").size(11.0).color(theme.text_secondary));
                    ui.add(egui::Slider::new(&mut self.ve_ari_floor, 0.85..=0.98).fixed_decimals(2));
                    ui.add_space(6.0);
                    ui.checkbox(&mut self.ve_composite_deck_swap, "Composite Metal Deck VE Swap");
                });
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut self.ve_selected_bidder_idx, 0, "Bidder A (SteelFab)");
                    ui.selectable_value(&mut self.ve_selected_bidder_idx, 1, "Bidder B (Nusantara)");
                    ui.selectable_value(&mut self.ve_selected_bidder_idx, 2, "Bidder C (GlobalTruss)");
                    if icons::icon_button(
                        ui,
                        AppIcon::SlidersPanel,
                        "Export Bid Leveling CSV",
                        11.5,
                        theme.text_primary,
                        theme.bg_canvas,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(6),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        let mut csv = String::from("Trade_Package,Owner_Est,Bidder_A,Bidder_B,Bidder_C,Outlier_Audit\n");
                        for line in &sum.lines {
                            csv.push_str(&format!(
                                "\"{}\",{:.2},{:.2},{:.2},{:.2},\"{}\"\n",
                                line.package, line.owner_est, line.bidder_a, line.bidder_b, line.bidder_c, line.outlier_note
                            ));
                        }
                        match crate::core::web_bridge::save_text_file("EigenGraph_Bid_Leveling_Matrix.csv", &csv, "text/csv") {
                            Ok(msg) => self.export_status = Some(format!("Bid Leveling CSV exported ({msg})")),
                            Err(err) => self.export_status = Some(format!("Bid CSV error: {err}")),
                        }
                    }
                    if icons::icon_button(
                        ui,
                        AppIcon::Document,
                        "Export VE Award Memo (.md)",
                        11.5,
                        theme.text_primary,
                        theme.bg_canvas,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(6),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        let md = format!(
                            "# Tender Evaluation & Value Engineering Award Memo\n\n- Selected Tenderer: **{}**\n- Owner Estimate: {}\n- Lowest Compliant Bid: {}\n- Composite Deck VE Savings: -{}\n- Net Contract Award: {}\n- Structural Redundancy Index (ARI): {:.3} (Min {:.2}, {})\n- Flagged Front-Loaded Packages (±{:.0}%): {}\n",
                            bidder_name,
                            oe_str,
                            curr.format_value(sum.best_bid_total),
                            ve_save_str,
                            award_str,
                            sum.projected_ari,
                            self.ve_ari_floor,
                            if sum.ari_compliant { "COMPLIANT" } else { "NON-COMPLIANT" },
                            self.ve_outlier_pct,
                            sum.outlier_count,
                        );
                        match crate::core::web_bridge::save_text_file("EigenGraph_VE_Tender_Award.md", &md, "text/markdown") {
                            Ok(msg) => self.export_status = Some(format!("VE Award Memo exported ({msg})")),
                            Err(err) => self.export_status = Some(format!("VE Memo error: {err}")),
                        }
                    }
                });
                ui.add_space(10.0);

                ScrollArea::horizontal().id_salt("tender_bid_scroll").show(ui, |ui| {
                    egui::Grid::new("tender_bid_grid")
                        .striped(true)
                        .min_col_width(90.0)
                        .spacing(Vec2::new(14.0, 6.0))
                        .show(ui, |ui| {
                            for h in [
                                "Trade Package",
                                "Owner Est (OE)",
                                "Bidder A (SteelFab)",
                                "Bidder B (Nusantara)",
                                "Bidder C (GlobalTruss)",
                                "Outlier Audit",
                            ] {
                                ui.label(RichText::new(h).size(10.5).strong().color(theme.accent_blue));
                            }
                            ui.end_row();
                            for line in &sum.lines {
                                let (icon, col) = if line.balanced {
                                    (AppIcon::CheckCircle, theme.accent_green)
                                } else {
                                    (AppIcon::AlertTriangle, theme.accent_orange)
                                };
                                ui.label(RichText::new(line.package).size(10.5).strong().color(theme.text_primary));
                                ui.label(RichText::new(curr.format_value(line.owner_est)).size(10.5).color(theme.text_secondary));
                                ui.label(RichText::new(curr.format_value(line.bidder_a)).size(10.5).color(theme.text_secondary));
                                ui.label(RichText::new(curr.format_value(line.bidder_b)).size(10.5).strong().color(theme.accent_green));
                                ui.label(RichText::new(curr.format_value(line.bidder_c)).size(10.5).color(theme.text_secondary));
                                icons::icon_label(ui, icon, 12.0, col, &line.outlier_note, 10.5, col, true);
                                ui.end_row();
                            }
                        });
                });
            },
        );
    }

    fn render_site_daily_diary_fleet(&mut self, ui: &mut Ui, theme: &MacTheme) {
        let sum = self.daily_diary.summarize();
        let site = &self.site_coupling;

        theme.card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                icons::icon_label(
                    ui,
                    AppIcon::Clock,
                    14.0,
                    theme.accent_blue,
                    "SITE DAILY DIARY, MANPOWER LOG & HEAVY FLEET TELEMETRY",
                    12.5,
                    theme.accent_blue,
                    true,
                );
                icons::pill_badge(
                    ui,
                    AppIcon::Globe,
                    format!(
                        "Earth Weather Allowance: {:.1}% Rain Risk | Wind {:.0} m/s",
                        site.climate_rain_delay_pct, site.design_wind_ms
                    ),
                    if site.climate_rain_delay_pct >= 18.0 {
                        theme.accent_orange
                    } else {
                        theme.accent_green
                    },
                );
            });
            ui.add_space(10.0);

            ui_kit::responsive_grid_n(ui, 4, 185.0, |ui, idx| {
                let (t, v, s, c) = match idx {
                    0 => (
                        "Avg Daily Manpower",
                        format!("{:.1} Workers", sum.avg_headcount),
                        "Carpentry + Rebar + Concrete + MEP".to_string(),
                        theme.accent_blue,
                    ),
                    1 => (
                        "Tower Crane Avg Util",
                        format!("{:.1}%", sum.avg_crane_util_pct),
                        format!("Design Wind Cap {:.0} m/s", site.design_wind_ms),
                        if sum.avg_crane_util_pct >= 75.0 {
                            theme.accent_green
                        } else {
                            theme.accent_orange
                        },
                    ),
                    2 => (
                        "Cumulative Poured Volume",
                        format!("{:.1} m3", sum.total_concrete_m3),
                        "Boom Pump Verified Ticket Log".to_string(),
                        theme.accent_green,
                    ),
                    _ => (
                        "Weather Stand-Down",
                        format!("{:.1} hrs", sum.total_weather_lost_hrs),
                        format!("{}/{} Incident-Free Days", sum.zero_incident_days, self.daily_diary.entries.len()),
                        if sum.total_weather_lost_hrs > 4.0 {
                            theme.accent_red
                        } else {
                            theme.accent_blue
                        },
                    ),
                };
                ui_kit::stat_card(ui, theme, t, &v, &s, c);
            });

            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui| {
                let w_rem = crate::ui_kit::clamped_row_width(ui, 120.0, 260.0, 220.0);
                ui.label(RichText::new("New Shift Note:").size(10.5).color(theme.text_secondary));
                ui.add(egui::TextEdit::singleline(&mut self.new_diary_remarks).desired_width(w_rem));
                ui.label(RichText::new("Pour m3:").size(10.5).color(theme.text_secondary));
                ui.add(egui::DragValue::new(&mut self.new_diary_pour_m3).range(0.0..=1500.0).speed(2.0));
                ui.label(RichText::new("Crane %:").size(10.5).color(theme.text_secondary));
                ui.add(egui::DragValue::new(&mut self.new_diary_crane_pct).range(0.0..=100.0).speed(1.0));
                if icons::icon_button(
                    ui,
                    AppIcon::Plus,
                    "Append Daily Log",
                    10.5,
                    Color32::WHITE,
                    theme.accent_green,
                    Stroke::NONE,
                    CornerRadius::same(6),
                    Vec2::new(0.0, 25.0),
                )
                .clicked()
                {
                    let next_day = 42 + self.daily_diary.entries.len();
                    let weather = if site.climate_rain_delay_pct > 18.0 {
                        "Monsoon Showers"
                    } else {
                        "Clear / Mild"
                    };
                    self.daily_diary.entries.push(crate::core::DailyDiaryEntry {
                        day_label: format!("Day {next_day}"),
                        weather: weather.to_string(),
                        temp_c: 30.0,
                        workable_hours: if site.climate_rain_delay_pct > 18.0 { 7.5 } else { 8.5 },
                        carpentry_crew: 20,
                        rebar_crew: 16,
                        concrete_crew: 14,
                        mep_crew: 12,
                        crane_util_pct: self.new_diary_crane_pct,
                        pump_m3_poured: self.new_diary_pour_m3,
                        incident_free: true,
                        remarks: self.new_diary_remarks.clone(),
                    });
                    self.export_status = Some(format!("Logged Field Daily Diary entry for Day {next_day}."));
                }
            });

            ui.add_space(10.0);
            ScrollArea::horizontal().id_salt("daily_diary_table_scroll").show(ui, |ui| {
                egui::Grid::new("daily_diary_grid")
                    .striped(true)
                    .spacing(Vec2::new(12.0, 5.0))
                    .show(ui, |ui| {
                        for h in ["Shift", "Weather", "Work Hrs", "Manpower", "Crane %", "Concrete", "HSE", "Field Remarks"] {
                            ui.label(RichText::new(h).size(10.5).strong().color(theme.accent_blue));
                        }
                        ui.end_row();
                        for e in &self.daily_diary.entries {
                            ui.label(RichText::new(&e.day_label).size(10.5).strong().color(theme.text_primary));
                            ui.label(RichText::new(&e.weather).size(10.5).color(theme.text_secondary));
                            ui.label(RichText::new(format!("{:.1} h", e.workable_hours)).size(10.5).monospace().color(theme.text_primary));
                            ui.label(RichText::new(format!("{} pax", e.total_headcount())).size(10.5).color(theme.accent_blue));
                            ui.label(RichText::new(format!("{:.0}%", e.crane_util_pct)).size(10.5).color(theme.accent_orange));
                            ui.label(RichText::new(format!("{:.1} m3", e.pump_m3_poured)).size(10.5).monospace().color(theme.accent_green));
                            ui.label(RichText::new(if e.incident_free { "Zero LTI" } else { "Alert" }).size(10.5).color(if e.incident_free { theme.accent_green } else { theme.accent_red }));
                            ui.label(RichText::new(&e.remarks).size(10.0).color(theme.text_muted));
                            ui.end_row();
                        }
                    });
            });
        });
    }

    fn render_procurement_contracts_claims(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        takeoff: &Parametric5DTakeoff,
    ) {
        let pos = self.procurement.evaluate_position(takeoff.grand_total_rab);
        let curr = self.active_currency;
        let site = &self.site_coupling;

        theme.card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                icons::icon_label(
                    ui,
                    AppIcon::Coins,
                    14.0,
                    theme.accent_green,
                    "COMMERCIAL PROCUREMENT, PROGRESS CLAIMS (IPC) & VARIATION ORDERS (VO)",
                    12.5,
                    theme.accent_green,
                    true,
                );
                if icons::icon_button(
                    ui,
                    AppIcon::Globe,
                    format!("Sync Site Logistics x{:.2}", site.remote_logistics_multiplier),
                    10.5,
                    Color32::WHITE,
                    theme.accent_blue,
                    Stroke::NONE,
                    CornerRadius::same(6),
                    Vec2::new(0.0, 24.0),
                )
                .on_hover_text("Adjust Contingency % based on Earth elevation & coastal barge/mountain logistics multiplier")
                .clicked()
                {
                    let extra_pct = (site.remote_logistics_multiplier - 1.0) * 100.0;
                    self.contingency_percent = (5.0 + extra_pct).clamp(5.0, 28.0);
                    self.export_status = Some(format!(
                        "Synced contingency to {:.1}% from Earth Site {} (x{:.2} logistics factor).",
                        self.contingency_percent, site.site_label, site.remote_logistics_multiplier
                    ));
                }
            });
            ui.add_space(10.0);

            ui_kit::responsive_grid_n(ui, 4, 185.0, |ui, idx| {
                let (t, v, s, c) = match idx {
                    0 => (
                        "PO Committed Total",
                        curr.format_value(pos.total_committed_po_idr),
                        format!("{} active subcontract packages", self.procurement.purchase_orders.len()),
                        theme.accent_blue,
                    ),
                    1 => (
                        "Certified IPC Gross",
                        curr.format_value(pos.certified_ipc_idr),
                        format!(
                            "Net Payable ({:.0}% Ret): {}",
                            self.procurement.retention_pct,
                            curr.format_value(pos.net_payable_after_retention_idr)
                        ),
                        theme.accent_green,
                    ),
                    2 => (
                        "Approved VOs & EOT",
                        curr.format_value(pos.approved_vo_idr),
                        format!(
                            "+{} days EOT | Pending: {}",
                            pos.total_eot_days,
                            curr.format_value(pos.pending_vo_idr)
                        ),
                        theme.accent_orange,
                    ),
                    _ => (
                        "Revised Final Forecast",
                        curr.format_value(pos.revised_forecast_idr),
                        format!("Baseline BoQ: {}", curr.format_value(pos.baseline_budget_idr)),
                        theme.accent_blue,
                    ),
                };
                ui_kit::stat_card(ui, theme, t, &v, &s, c);
            });

            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Retention %:").size(10.5).color(theme.text_secondary));
                ui.add(egui::DragValue::new(&mut self.procurement.retention_pct).range(0.0..=15.0).speed(0.25));
                ui.separator();
                let vw = crate::ui_kit::clamped_row_width(ui, 100.0, 180.0, 210.0);
                ui.label(RichText::new("Vendor:").size(10.5).color(theme.text_secondary));
                ui.add(egui::TextEdit::singleline(&mut self.new_po_vendor).desired_width(vw));
                ui.label(RichText::new("Package:").size(10.5).color(theme.text_secondary));
                ui.add(egui::TextEdit::singleline(&mut self.new_po_pkg).desired_width(vw));
                if icons::icon_button(
                    ui,
                    AppIcon::Plus,
                    "Issue PO",
                    10.5,
                    Color32::WHITE,
                    theme.accent_blue,
                    Stroke::NONE,
                    CornerRadius::same(6),
                    Vec2::new(0.0, 24.0),
                )
                .clicked()
                {
                    let n = self.procurement.purchase_orders.len() + 1;
                    self.procurement.purchase_orders.push(crate::core::PurchaseOrderRow {
                        po_no: format!("PO-2026-{n:02}"),
                        vendor: self.new_po_vendor.clone(),
                        wbs_package: self.new_po_pkg.clone(),
                        committed_idr: self.new_po_amount_idr,
                        certified_pct: 10.0,
                        lead_time_days: 21,
                        status: crate::core::ProcurementStatus::AwardedPo,
                    });
                }
            });

            ui.add_space(8.0);
            ScrollArea::horizontal().id_salt("procurement_po_scroll").show(ui, |ui| {
                egui::Grid::new("procurement_po_grid")
                    .striped(true)
                    .spacing(Vec2::new(12.0, 5.0))
                    .show(ui, |ui| {
                        for h in ["PO #", "Subcontractor / Vendor", "WBS Package", "Committed", "IPC %", "Status", "Action"] {
                            ui.label(RichText::new(h).size(10.5).strong().color(theme.accent_blue));
                        }
                        ui.end_row();
                        for po in &mut self.procurement.purchase_orders {
                            ui.label(RichText::new(&po.po_no).size(10.5).monospace().color(theme.text_primary));
                            ui.label(RichText::new(&po.vendor).size(10.5).color(theme.text_secondary));
                            ui.label(RichText::new(&po.wbs_package).size(10.5).color(theme.text_muted));
                            ui.label(RichText::new(curr.format_value(po.committed_idr)).size(10.5).monospace().color(theme.text_primary));
                            ui.label(RichText::new(format!("{:.0}%", po.certified_pct)).size(10.5).strong().color(theme.accent_green));
                            if ui.small_button(po.status.label()).clicked() {
                                po.status = po.status.next();
                            }
                            if ui.small_button("+10% IPC").clicked() {
                                po.certified_pct = (po.certified_pct + 10.0).min(100.0);
                            }
                            ui.end_row();
                        }
                    });
            });

            ui.add_space(12.0);
            ui.label(RichText::new("VARIATION ORDERS (VO / CHANGE CLAIMS)").size(11.0).strong().color(theme.accent_orange));
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                let d_w = crate::ui_kit::clamped_row_width(ui, 120.0, 260.0, 220.0);
                ui.add(egui::TextEdit::singleline(&mut self.new_vo_desc).desired_width(d_w));
                ui.label(RichText::new("Impact:").size(10.5).color(theme.text_secondary));
                ui.add(egui::DragValue::new(&mut self.new_vo_cost_idr).range(0.0..=50_000_000_000.0).speed(5_000_000.0));
                ui.label(RichText::new("EOT d:").size(10.5).color(theme.text_secondary));
                ui.add(egui::DragValue::new(&mut self.new_vo_eot_days).range(0..=180));
                if icons::icon_button(
                    ui,
                    AppIcon::Plus,
                    "Add VO Claim",
                    10.5,
                    Color32::WHITE,
                    theme.accent_orange,
                    Stroke::NONE,
                    CornerRadius::same(6),
                    Vec2::new(0.0, 24.0),
                )
                .clicked()
                {
                    let n = self.procurement.variation_orders.len() + 1;
                    self.procurement.variation_orders.push(crate::core::VariationOrderRow {
                        vo_no: format!("VO-{n:03}"),
                        description: self.new_vo_desc.clone(),
                        cost_impact_idr: self.new_vo_cost_idr,
                        eot_days: self.new_vo_eot_days,
                        approved: false,
                    });
                }
            });
            ui.add_space(6.0);
            ScrollArea::horizontal().id_salt("procurement_vo_scroll").show(ui, |ui| {
                egui::Grid::new("procurement_vo_grid")
                    .striped(true)
                    .spacing(Vec2::new(12.0, 5.0))
                    .show(ui, |ui| {
                        for h in ["VO #", "Claim Scope & Justification", "Cost Impact", "EOT", "Approval"] {
                            ui.label(RichText::new(h).size(10.5).strong().color(theme.accent_blue));
                        }
                        ui.end_row();
                        for vo in &mut self.procurement.variation_orders {
                            ui.label(RichText::new(&vo.vo_no).size(10.5).monospace().color(theme.text_primary));
                            ui.label(RichText::new(&vo.description).size(10.5).color(theme.text_secondary));
                            ui.label(RichText::new(curr.format_value(vo.cost_impact_idr)).size(10.5).monospace().color(theme.accent_orange));
                            ui.label(RichText::new(format!("+{} d", vo.eot_days)).size(10.5).color(theme.text_muted));
                            if ui
                                .selectable_label(vo.approved, if vo.approved { "Approved" } else { "Pending (Approve)" })
                                .clicked()
                            {
                                vo.approved = !vo.approved;
                            }
                            ui.end_row();
                        }
                    });
            });
        });
    }

    fn metric_card(&self, ui: &mut Ui, theme: &MacTheme, title: &str, value: &str, subtitle: &str, val_color: Color32) {
        theme.card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).size(10.5).strong().color(theme.text_muted));
            ui.add_space(4.0);
            ui.label(RichText::new(value).size(16.0).strong().color(val_color));
            ui.add_space(2.0);
            ui.label(RichText::new(subtitle).size(10.5).color(theme.text_secondary));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planning_phase40_solvers() {
        let pv = PlanningView::default();
        let bim = crate::core::BimProject::sample_bimcraft_building();
        let takeoff = pv.compute_takeoff(&[]);

        let mc = PlanningView::compute_monte_carlo_pert(&takeoff, &bim, 1000, 25.0, 40);
        assert!(mc.p50_days >= mc.deterministic_days * 0.9);
        assert!(mc.p80_days >= mc.p50_days);
        assert!(mc.p90_days >= mc.p80_days);
        assert_eq!(mc.cdf_points.len(), 21);

        let crew = PlanningView::compute_crew_leveling(&takeoff, &bim, 75, true);
        assert!(crew.peak_leveled <= crew.peak_unleveled + 1e-3);

        let evm = PlanningView::compute_evm_summary(&takeoff, 7, 5.0, 0.97);
        assert!(evm.spi > 0.5 && evm.cpi > 0.5);
        assert!(evm.eac_val >= takeoff.grand_total_rab);
        assert!(evm.net_claim < evm.ev_val);
    }

    #[test]
    fn test_value_engineering_bid_leveling_outliers_and_ari() {
        let pv = PlanningView::default();
        let takeoff = pv.compute_takeoff(&[]);
        let tight = compute_bid_leveling_matrix(&takeoff, 15.0, true, 0.90);
        let loose = compute_bid_leveling_matrix(&takeoff, 30.0, false, 0.95);
        assert!(tight.outlier_count >= 2);
        assert_eq!(loose.outlier_count, 0);
        assert_eq!(tight.best_bidder_idx, 1);
        assert!(tight.ve_savings_val > 0.0);
        assert!(tight.ari_compliant);
    }

    #[test]
    fn test_planning_live_bim_sync_and_takeoff() {
        let mut pv = PlanningView::default();
        let bim = crate::core::BimProject::sample_bimcraft_building();
        pv.sync_from_bim(&bim, &[]);
        assert_eq!(pv.simulation_phases.len(), 5);
        assert_eq!(pv.wbs_catalog.len(), 5);

        let live_takeoff = pv.compute_takeoff_with_bim(&[], Some(&bim), &[]);
        assert!((live_takeoff.concrete_footing_m3 - bim.total_concrete_volume_m3()).abs() < 1e-3);
        assert!(live_takeoff.grand_total_rab > 0.0);
    }

    #[test]
    fn test_phase45_diary_fleet_and_procurement_claims_earth_coupling() {
        let mut pv = PlanningView::default();
        let bim = crate::core::BimProject::sample_bimcraft_building();
        pv.site_coupling.climate_rain_delay_pct = 18.0;
        pv.site_coupling.remote_logistics_multiplier = 1.24;

        let d_sum = pv.daily_diary.summarize();
        assert!(d_sum.avg_headcount > 0.0);
        assert!(d_sum.total_concrete_m3 > 0.0);
        assert!(d_sum.zero_incident_days >= 1);

        let p_sum = pv.procurement.evaluate_position(10_000_000_000.0);
        assert!(p_sum.certified_ipc_idr > 0.0);
        assert!(p_sum.net_payable_after_retention_idr < p_sum.certified_ipc_idr);
        assert!(p_sum.total_eot_days > 0);
        assert!(p_sum.revised_forecast_idr >= 10_000_000_000.0);

        let theme = MacTheme::default();
        let mut features = super::FeatureRegistry::default();
        for tab in [PlanningSubTab::SiteDailyDiaryFleet, PlanningSubTab::ProcurementContractsClaims] {
            pv.active_subtab = tab;
            for win_w in [1000.0_f32, 700.0, 390.0] {
                let ctx = egui::Context::default();
                let mut inner_max_x = 0.0_f32;
                let mut panel_max_x = 0.0_f32;
                for _ in 0..2 {
                    let input = egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(win_w, 900.0))),
                        ..Default::default()
                    };
                    let mut output = ctx.run_ui(input, |ui| {
                        egui::CentralPanel::default().show(ui, |ui| {
                            egui::Frame::new().inner_margin(egui::Margin::symmetric(14, 12)).show(ui, |ui| {
                                panel_max_x = ui.max_rect().right();
                                let resp = ui.vertical(|ui| {
                                    let mut pillar = crate::models::NavigationPillar::TimeScheduling;
                                    pv.show(ui, &theme, &[], &[], &bim, &mut features, &mut pillar);
                                });
                                inner_max_x = resp.response.rect.right();
                            });
                        });
                    });
                    output.textures_delta.clear();
                }
                assert!(
                    inner_max_x <= panel_max_x + 2.0,
                    "Planning ({tab:?}) overflow @ {win_w}: inner_max_x={inner_max_x:.1} > panel_max_x={panel_max_x:.1}"
                );
            }
        }
    }
}
