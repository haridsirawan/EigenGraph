//! lean.rs — Pillar 9: 9D BIM Lean Construction.
//! The optimization of the construction process to minimize material waste, streamline workflow, and maximize production efficiency.
//! Features Dynamic 1D Cutting Stock Optimizer (CSO), Bar Bending Schedule (BBS), and Interactive Last Planner System (LPS).

use egui::{
    Align2, Color32, CornerRadius, Frame, Margin, Pos2, Rect, RichText, ScrollArea,
    Stroke, StrokeKind, Ui, Vec2,
};
use crate::core::cso::{CuttingStockOptimizer, CsoReport};
use crate::features::{FeatureId, FeatureRegistry};
use crate::models::{LpsConstraintCategory, LpsLookaheadTask, LpsWeeklyPpcRecord, StructuralMember};
use crate::theme::MacTheme;
use crate::ui_kit;
use crate::views::planning::format_idr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeanSubTab {
    CuttingStock,
    LastPlanner,
    ValueStream,
    JitDelivery,
    TaktConcreteDispatch,
    RfiSubmittalTracker,
    QaQcLabAndCommissioning,
}

pub struct LeanView {
    pub active_subtab: LeanSubTab,
    pub standard_bar_length_m: f32,
    pub saw_kerf_mm: f32,
    pub min_reusable_offcut_m: f32,
    pub steel_price_idr_per_kg: f32,
    pub selected_profile: String,
    pub export_status: Option<String>,
    // Phase 29: Stateful Last Planner System (LPS)
    pub lps_history: Vec<LpsWeeklyPpcRecord>,
    pub lps_tasks: Vec<LpsLookaheadTask>,
    pub lps_week_filter: String,
    pub lps_new_task_name: String,
    pub lps_new_task_week: String,
    // Phase 38: Live BimProject Rebar BBS -> CSO 1D Sync (9D-02)
    pub use_bim_rebar_source: bool,
    // Phase 40 (9D-03): Takt-Time Concrete Pour & Mixer Dispatch
    pub takt_days_per_zone: u32,
    pub takt_transit_min: f32,
    pub takt_mixer_cap_m3: f32,
    pub takt_alert_threshold_min: f32,
    // Phase 41 (9D-04): BIM-Pinned RFI & Submittal SLA Tracker coupled to LPS
    pub rfi_items: Vec<RfiSubmittalItem>,
    pub rfi_sla_days: u32,
    pub rfi_auto_flag_lps: bool,
    // Phase 45: Field QA/QC (Concrete Lab, ITP Hold Points, NCR/Punch List, MEP T&C) + Earth Coupling
    pub qa_qc: crate::core::FieldQualityControlLedger,
    pub site_coupling: crate::core::SiteEnvironmentalCoupling,
    pub new_qc_zone: String,
    pub new_qc_day28_mpa: f32,
    pub new_ncr_desc: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RfiSubmittalItem {
    pub ref_id: String,
    pub subject: String,
    pub owner: String,
    pub age_days: u32,
    pub linked_lps_task: String,
    pub approved: bool,
}

impl RfiSubmittalItem {
    pub fn default_ledger() -> Vec<Self> {
        vec![
            Self {
                ref_id: "RFI-042".to_string(),
                subject: "Shear Wall Core Coupling Rebar Congestion (L07)".to_string(),
                owner: "Lead EOR".to_string(),
                age_days: 7,
                linked_lps_task: "05.08 Hold".to_string(),
                approved: false,
            },
            Self {
                ref_id: "SUB-118".to_string(),
                subject: "W14x90 Bolted Splice Slip-Critical Fab Drawings".to_string(),
                owner: "QA Reviewer".to_string(),
                age_days: 3,
                linked_lps_task: "05.12 Hold".to_string(),
                approved: false,
            },
            Self {
                ref_id: "RFI-045".to_string(),
                subject: "Roof Tuned Mass Damper Pocket Embedded Plate Offset".to_string(),
                owner: "Arch Lead".to_string(),
                age_days: 2,
                linked_lps_task: "08.02 Clear".to_string(),
                approved: false,
            },
            Self {
                ref_id: "SUB-109".to_string(),
                subject: "Base Plate Grout Non-Shrink High-Strength Submittal".to_string(),
                owner: "Structural PE".to_string(),
                age_days: 4,
                linked_lps_task: "02.04 Clear".to_string(),
                approved: true,
            },
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RfiSlaSummary {
    pub open_count: usize,
    pub overdue_count: usize,
    pub approval_pct: f32,
    pub lps_holds_count: usize,
    pub avg_age_days: f32,
}

/// Evaluates open structural RFIs & shop drawing submittals against contractual SLA and LPS constraint holds.
pub fn evaluate_rfi_sla_status(items: &[RfiSubmittalItem], sla_days: u32) -> RfiSlaSummary {
    let sla = sla_days.max(1);
    let mut open_count = 0usize;
    let mut overdue_count = 0usize;
    let mut approved_count = 0usize;
    let mut lps_holds_count = 0usize;
    let mut age_sum = 0u32;

    for r in items {
        if r.approved {
            approved_count += 1;
        } else {
            open_count += 1;
            age_sum += r.age_days;
            if r.age_days > sla {
                overdue_count += 1;
            }
            if r.linked_lps_task.contains("Hold") {
                lps_holds_count += 1;
            }
        }
    }

    let total = items.len().max(1);
    let approval_pct = (approved_count as f32 / total as f32) * 100.0;
    let avg_age_days = if open_count > 0 {
        age_sum as f32 / open_count as f32
    } else {
        0.0
    };

    RfiSlaSummary {
        open_count,
        overdue_count,
        approval_pct,
        lps_holds_count,
        avg_age_days,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaktPourZoneRow {
    pub zone_name: &'static str,
    pub pour_m3: f64,
    pub trucks_needed: usize,
    pub headway_min: f32,
    pub cold_joint_status: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaktConcreteDispatchReport {
    pub total_bim_m3: f64,
    pub shift_pour_m3: f64,
    pub total_trucks_shift: usize,
    pub dispatch_headway_min: f32,
    pub delayed_trucks: usize,
    pub slump_margin_min: f32,
    pub zones: Vec<TaktPourZoneRow>,
}

/// Computes Takt-Time concrete pour cadence and ready-mix truck dispatch intervals from `BimProject` concrete volumes.
pub fn compute_takt_concrete_dispatch(
    bim_project: &crate::core::bim::BimProject,
    takt_days: u32,
    transit_min: f32,
    mixer_cap_m3: f32,
    alert_threshold_min: f32,
) -> TaktConcreteDispatchReport {
    let bim_m3 = bim_project.total_concrete_volume_m3();
    let levels_n = bim_project.levels.len().max(1) as f64;
    let typical_floor_m3 = if bim_m3 > 10.0 {
        (bim_m3 / levels_n).max(90.0)
    } else {
        425.0
    };
    let cap = mixer_cap_m3.clamp(4.0, 12.0) as f64;
    let specs = [
        ("Zone A (West)", 0.28, "Completed"),
        ("Zone B (Center)", 0.48, if transit_min > alert_threshold_min { "Delay Alert" } else { "Nominal" }),
        ("Zone C (East)", 0.24, "Scheduled"),
    ];

    let mut zones = Vec::with_capacity(specs.len());
    let mut active_pour_m3 = 0.0f64;
    let mut active_trucks = 0usize;
    let shift_hours_min = 420.0f32; // 7-hour night pour window

    for (idx, (zname, frac, base_st)) in specs.into_iter().enumerate() {
        let pour_m3 = typical_floor_m3 * frac;
        let trucks = (pour_m3 / cap).ceil().max(1.0) as usize;
        let headway = (shift_hours_min / trucks as f32).clamp(6.0, 35.0);
        if idx == 1 {
            active_pour_m3 = pour_m3;
            active_trucks = trucks;
        }
        zones.push(TaktPourZoneRow {
            zone_name: zname,
            pour_m3,
            trucks_needed: trucks,
            headway_min: headway,
            cold_joint_status: base_st,
        });
    }

    let delayed_trucks = if transit_min > alert_threshold_min {
        ((transit_min - alert_threshold_min) / 8.0).ceil().max(1.0) as usize
    } else {
        0
    };
    let dispatch_headway_min = ((shift_hours_min / active_trucks.max(1) as f32)
        * (5.0 / takt_days.max(1) as f32))
        .clamp(6.0, 35.0);
    let slump_margin_min = 90.0 - transit_min;

    TaktConcreteDispatchReport {
        total_bim_m3: bim_m3,
        shift_pour_m3: active_pour_m3,
        total_trucks_shift: active_trucks,
        dispatch_headway_min,
        delayed_trucks,
        slump_margin_min,
        zones,
    }
}

impl Default for LeanView {
    fn default() -> Self {
        Self {
            active_subtab: LeanSubTab::CuttingStock,
            standard_bar_length_m: 12.0,
            saw_kerf_mm: 5.0,
            min_reusable_offcut_m: 1.5,
            steel_price_idr_per_kg: 17500.0,
            selected_profile: "All Profiles".to_string(),
            export_status: None,
            lps_history: Vec::new(),
            lps_tasks: Vec::new(),
            lps_week_filter: "All Weeks".to_string(),
            lps_new_task_name: String::new(),
            lps_new_task_week: "W-06".to_string(),
            use_bim_rebar_source: false,
            takt_days_per_zone: 5,
            takt_transit_min: 38.0,
            takt_mixer_cap_m3: 7.0,
            takt_alert_threshold_min: 75.0,
            rfi_items: RfiSubmittalItem::default_ledger(),
            rfi_sla_days: 5,
            rfi_auto_flag_lps: true,
            qa_qc: crate::core::FieldQualityControlLedger::default(),
            site_coupling: crate::core::SiteEnvironmentalCoupling::default(),
            new_qc_zone: "L04 Zone-B Core Pour".to_string(),
            new_qc_day28_mpa: 42.5,
            new_ncr_desc: "Concrete cover spacer spacing < 1000mm check".to_string(),
        }
    }
}

impl LeanView {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        members: &[StructuralMember],
        bim_project: &crate::core::bim::BimProject,
        features: &mut FeatureRegistry,
    ) {
        ui.vertical(|ui| {
            Frame::new().inner_margin(Margin::symmetric(0, 8)).show(ui, |ui| {
                let full_w = ui.available_width().max(160.0);
                ui.set_width(full_w);
                ui.set_max_width(full_w);
                // Toast notification if export triggered
                let mut dismiss_toast = false;
                if let Some(ref status) = self.export_status {
                    Frame::new()
                        .fill(theme.bg_card)
                        .corner_radius(CornerRadius::same(8))
                        .stroke(Stroke::new(1.0, theme.accent_green))
                        .inner_margin(Margin::same(10))
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                crate::icons::icon_label(
                                    ui,
                                    crate::icons::AppIcon::CheckCircle,
                                    14.0,
                                    theme.accent_green,
                                    status,
                                    12.0,
                                    theme.text_primary,
                                    false,
                                );
                                if ui.button(RichText::new("Dismiss").size(11.0)).clicked() {
                                    dismiss_toast = true;
                                }
                            });
                        });
                    ui.add_space(8.0);
                }
                if dismiss_toast {
                    self.export_status = None;
                }

                // 1. TOP SUBTABS SELECTOR
                self.render_header_bar(ui, theme);

                ui.add_space(16.0);

                // 2. ACTIVE SUB-TAB CONTENT
                match self.active_subtab {
                    LeanSubTab::CuttingStock => self.render_cutting_stock(ui, theme, members, bim_project, features),
                    LeanSubTab::LastPlanner => self.render_last_planner(ui, theme),
                    LeanSubTab::ValueStream => self.render_value_stream(ui, theme),
                    LeanSubTab::JitDelivery => self.render_jit_delivery(ui, theme),
                    LeanSubTab::TaktConcreteDispatch => {
                        self.render_takt_concrete_dispatch(ui, theme, bim_project, features);
                    }
                    LeanSubTab::RfiSubmittalTracker => {
                        self.render_rfi_submittal_tracker(ui, theme, features);
                    }
                    LeanSubTab::QaQcLabAndCommissioning => {
                        self.render_qa_qc_lab_and_commissioning(ui, theme);
                    }
                }

                ui.add_space(64.0);
            });
        });
    }

    fn render_header_bar(&mut self, ui: &mut Ui, theme: &MacTheme) {
        let (_, _, live_ppc) = LpsLookaheadTask::compute_live_ppc(&self.lps_tasks);
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(6))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                let card_w = ui.available_width().max(140.0);
                ui.set_width(card_w);
                ui.set_max_width(card_w);
                let lps_label = format!("Last Planner System (PPC {:.1}%)", live_ppc);
                let tabs = [
                    (LeanSubTab::CuttingStock, crate::icons::AppIcon::Scissors, "1D/2D Cutting Stock Optimizer".to_string()),
                    (LeanSubTab::LastPlanner, crate::icons::AppIcon::Clipboard, lps_label),
                    (LeanSubTab::ValueStream, crate::icons::AppIcon::ChartLine, "Value Stream & 7 Wastes".to_string()),
                    (LeanSubTab::JitDelivery, crate::icons::AppIcon::Truck, "Just-In-Time (JIT) Delivery Buffer".to_string()),
                    (LeanSubTab::TaktConcreteDispatch, crate::icons::AppIcon::Layers, "Takt Trains & Pour Dispatch".to_string()),
                    (LeanSubTab::RfiSubmittalTracker, crate::icons::AppIcon::Document, "RFI & Submittal SLA".to_string()),
                    (LeanSubTab::QaQcLabAndCommissioning, crate::icons::AppIcon::CheckCircle, "QA/QC Lab, ITP, NCR & T&C".to_string()),
                ];

                ui.horizontal_wrapped(|ui| {
                    for (tab, icon, label) in tabs {
                        let is_active = self.active_subtab == tab;
                        let bg = if is_active { theme.accent_blue } else { Color32::TRANSPARENT };
                        let fg = if is_active { Color32::WHITE } else { theme.text_secondary };

                        if crate::icons::icon_button(
                            ui,
                            icon,
                            label,
                            12.0,
                            fg,
                            bg,
                            egui::Stroke::NONE,
                            CornerRadius::same(7),
                            Vec2::new(0.0, 30.0),
                        )
                        .clicked()
                        {
                            self.active_subtab = tab;
                        }
                    }
                });
            });
    }

    fn render_responsive_cards(&self, ui: &mut Ui, theme: &MacTheme, cards: &[(&str, String, String, Color32)]) {
        ui_kit::responsive_grid_n(ui, cards.len(), 200.0, |ui, idx| {
            let (title, val, sub, col) = &cards[idx];
            self.render_stat_card(ui, theme, title, val, sub, *col);
        });
    }

    fn render_cutting_stock(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        members: &[StructuralMember],
        bim_project: &crate::core::bim::BimProject,
        features: &mut FeatureRegistry,
    ) {
        // 1. Dynamic Extraction from active BimProject Rebar BBS or StructuralMembers
        let items = if self.use_bim_rebar_source && !bim_project.rebars.is_empty() {
            CuttingStockOptimizer::extract_items_from_bim_rebar(bim_project)
        } else {
            CuttingStockOptimizer::extract_items_from_members(members)
        };

        // Get unique profiles list
        let mut profiles = vec!["All Profiles".to_string()];
        for it in &items {
            if !profiles.contains(&it.profile) && !it.profile.is_empty() {
                profiles.push(it.profile.clone());
            }
        }
        if !profiles.contains(&self.selected_profile) {
            self.selected_profile = "All Profiles".to_string();
        }

        let filter_opt = if self.selected_profile == "All Profiles" {
            None
        } else {
            Some(self.selected_profile.as_str())
        };

        // 2. Execute Dynamic 1D Optimization
        let report: CsoReport = CuttingStockOptimizer::optimize_1d(
            &items,
            filter_opt,
            self.standard_bar_length_m as f64,
            self.saw_kerf_mm as f64,
            self.min_reusable_offcut_m as f64,
            self.steel_price_idr_per_kg as f64,
        );

        // 3. Top Master Stat Cards
        let yield_color = if report.material_yield_pct >= 95.0 {
            theme.accent_green
        } else if report.material_yield_pct >= 88.0 {
            theme.accent_blue
        } else {
            Color32::from_rgb(239, 68, 68)
        };

        let cards = [
            (
                "MATERIAL YIELD EFFICIENCY",
                format!("{:.1}%", report.material_yield_pct),
                format!("Scrap Waste: {:.1}% ({:.2}m)", report.scrap_waste_pct, report.net_scrap_waste_m),
                yield_color,
            ),
            (
                "ESTIMATED COST SAVINGS",
                format!("Rp {}", format_idr(report.cost_savings_idr)),
                format!("Avoided Scrap: {:.1} kg Steel", (report.cost_savings_idr / self.steel_price_idr_per_kg as f64).max(0.0)),
                theme.accent_blue,
            ),
            (
                "MASTER BAR STOCK LENGTH",
                format!("{:.2} m", report.stock_length_m),
                format!("Kerf: {:.1} mm • Min Offcut: {:.1} m", report.kerf_mm, report.min_reusable_m),
                theme.text_primary,
            ),
            (
                "TOTAL MASTER BARS REQUIRED",
                format!("{} Bars", report.master_bars_used),
                format!("To Cut {} Structural Elements", report.total_pieces_demanded),
                theme.text_primary,
            ),
        ];
        self.render_responsive_cards(ui, theme, &cards);

        ui.add_space(12.0);

        ui_kit::feature_slot_card(ui, theme, features, FeatureId::LiveRebarBbsCuttingSync, |ui| {
            let rebar_cnt = bim_project.rebars.len();
            let rebar_kg = bim_project.total_rebar_weight_kg();
            ui.horizontal_wrapped(|ui| {
                if ui.selectable_label(!self.use_bim_rebar_source, format!("Source: Analytical Steel/Frame Members ({})", members.len())).clicked() {
                    self.use_bim_rebar_source = false;
                    self.selected_profile = "All Profiles".to_string();
                }
                if ui.selectable_label(self.use_bim_rebar_source, format!("Source: Active BimProject Rebar Marks ({} marks, {:.0} kg)", rebar_cnt, rebar_kg)).clicked() {
                    self.use_bim_rebar_source = true;
                    self.selected_profile = "All Profiles".to_string();
                    if rebar_cnt == 0 {
                        self.export_status = Some("No rebar bars in active BimProject yet. Generate rebar via Parametric -> Rebar BBS or Instantiate a Typology Template first.".to_string());
                    }
                }
                if crate::icons::icon_button(
                    ui,
                    crate::icons::AppIcon::Scissors,
                    "Optimize BIM Rebar Schedule (1D BFD)",
                    12.0,
                    Color32::WHITE,
                    theme.accent_blue,
                    egui::Stroke::NONE,
                    CornerRadius::same(6),
                    Vec2::ZERO,
                ).clicked() {
                    self.use_bim_rebar_source = true;
                    self.selected_profile = "All Profiles".to_string();
                    self.export_status = if rebar_cnt > 0 {
                        Some(format!("Synchronized {} live BimProject rebar marks ({:.1} kg) into 1D Best-Fit Decreasing cutting stock engine.", rebar_cnt, rebar_kg))
                    } else {
                        Some("No concrete rebar bars detected yet in BimProject — open Parametric -> Rebar BBS & Detailing -> Auto-Arm All RC Elements.".to_string())
                    };
                }
            });
        });

        ui.add_space(14.0);

        // 4. Parameter Control & Filter Bar
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(14))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                let card_w = ui.available_width().max(140.0);
                ui.set_width(card_w);
                ui.set_max_width(card_w);
                crate::icons::icon_label(
                    ui,
                    crate::icons::AppIcon::SettingsGear,
                    13.0,
                    theme.accent_blue,
                    "Cutting Optimization Parameters",
                    13.0,
                    theme.text_primary,
                    true,
                );
                ui.add_space(10.0);
                ui_kit::responsive_grid_n(ui, 4, 185.0, |ui, idx| {
                    let cell_w = ui.available_width().max(110.0);
                    ui.spacing_mut().slider_width = (cell_w - 72.0).clamp(56.0, 140.0);
                    match idx {
                        0 => {
                            ui.label(RichText::new("Profile Filter:").size(11.0).color(theme.text_secondary));
                            egui::ComboBox::from_id_salt("cso_profile_filter")
                                .selected_text(RichText::new(&self.selected_profile).size(11.5))
                                .width((cell_w - 8.0).clamp(100.0, 220.0))
                                .show_ui(ui, |ui| {
                                    for p in &profiles {
                                        ui.selectable_value(&mut self.selected_profile, p.clone(), p);
                                    }
                                });
                        }
                        1 => {
                            ui.label(RichText::new("Stock Bar Length:").size(11.0).color(theme.text_secondary));
                            ui.add(egui::Slider::new(&mut self.standard_bar_length_m, 6.0..=14.0).suffix(" m").step_by(0.5));
                        }
                        2 => {
                            ui.label(RichText::new("Saw Kerf:").size(11.0).color(theme.text_secondary));
                            ui.add(egui::Slider::new(&mut self.saw_kerf_mm, 0.0..=10.0).suffix(" mm").step_by(0.5));
                        }
                        _ => {
                            ui.label(RichText::new("Min Reusable:").size(11.0).color(theme.text_secondary));
                            ui.add(egui::Slider::new(&mut self.min_reusable_offcut_m, 0.5..=3.0).suffix(" m").step_by(0.25));
                        }
                    }
                });
            });

        ui.add_space(16.0);

        // 5. Visual Dynamic Cutting Pattern Canvas
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(14))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                let card_w = ui.available_width().max(140.0);
                ui.set_width(card_w);
                ui.set_max_width(card_w);
                if card_w >= 540.0 {
                    ui.horizontal_wrapped(|ui| {
                        crate::icons::icon_label(
                            ui,
                            crate::icons::AppIcon::Scissors,
                            13.5,
                            theme.accent_blue,
                            "Optimized Steel Bar Cutting Patterns (Dynamic BFD)",
                            13.5,
                            theme.text_primary,
                            true,
                        );
                        ui.label(RichText::new(format!("Algorithm: Best-Fit Decreasing • Total Cut Pieces: {} pcs", report.total_pieces_demanded))
                            .size(11.0).color(theme.accent_blue));
                    });
                } else {
                    crate::icons::icon_label(
                        ui,
                        crate::icons::AppIcon::Scissors,
                        13.5,
                        theme.accent_blue,
                        "Optimized Bar Cutting (Dynamic BFD)",
                        13.0,
                        theme.text_primary,
                        true,
                    );
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!(
                                "Algorithm: Best-Fit Decreasing • Total Cut Pieces: {} pcs",
                                report.total_pieces_demanded
                            ))
                            .size(11.0)
                            .color(theme.accent_blue),
                        )
                        .wrap(),
                    );
                }
                ui.add_space(12.0);

                let bar_width = (ui.available_width() - 4.0).max(120.0);
                let bar_height = 24.0;
                let colors = [
                    Color32::from_rgb(16, 185, 129),  // emerald
                    Color32::from_rgb(14, 165, 233),  // sky
                    Color32::from_rgb(99, 102, 241),  // indigo
                    Color32::from_rgb(245, 158, 11),  // amber
                    Color32::from_rgb(168, 85, 247),  // purple
                    Color32::from_rgb(236, 72, 153),  // pink
                ];

                // Render each master bar dynamically
                for (bar_idx, bar) in report.bars.iter().enumerate() {
                    ui.vertical(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            let (status_icon, status_badge) = if bar.remainder_m < 0.05 {
                                (crate::icons::AppIcon::Star, "100% Efficiency (Zero Waste)")
                            } else if bar.is_reusable_offcut {
                                (crate::icons::AppIcon::Recycle, "Reusable Offcut (Gusset/Stiffener Ready)")
                            } else {
                                (crate::icons::AppIcon::AlertTriangle, "Net Scrap Waste")
                            };

                            let badge_color = if bar.remainder_m < 0.05 {
                                theme.accent_green
                            } else if bar.is_reusable_offcut {
                                theme.accent_blue
                            } else {
                                Color32::from_rgb(245, 158, 11)
                            };

                            ui.label(RichText::new(format!(
                                "Master Bar #{:02} • {} ({:.2}m)",
                                bar.bar_index, bar.profile, bar.stock_length_m
                            )).size(12.5).strong().color(theme.text_primary));

                            ui.label(RichText::new(format!("Yield: {:.1}%", bar.yield_pct)).size(11.0).color(theme.text_muted));
                            crate::icons::icon_label(ui, status_icon, 12.0, badge_color, status_badge, 11.5, badge_color, true);
                        });
                        ui.add_space(4.0);

                        // Visual Segmented Bar Diagram
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(bar_width, bar_height), egui::Sense::hover());

                        let total_stock_m = bar.stock_length_m as f32;

                        for (cut_idx, cut) in bar.cuts.iter().enumerate() {
                            let seg_x1 = rect.min.x + (cut.start_m as f32 / total_stock_m) * bar_width;
                            let seg_x2 = rect.min.x + (cut.end_m as f32 / total_stock_m) * bar_width;
                            let seg_w = (seg_x2 - seg_x1 - 1.5).max(1.0);

                            let seg_rect = egui::Rect::from_min_max(
                                egui::Pos2::new(seg_x1, rect.min.y),
                                egui::Pos2::new((seg_x1 + seg_w).min(rect.max.x), rect.max.y),
                            );

                            let color = colors[(bar_idx + cut_idx) % colors.len()];
                            ui.painter().rect_filled(seg_rect, CornerRadius::same(4), color);

                            if seg_w > 45.0 {
                                let label_text = format!("{} ({:.2}m)", cut.label, cut.length_m);
                                ui.painter().text(
                                    seg_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    label_text,
                                    egui::FontId::proportional(10.0),
                                    Color32::WHITE,
                                );
                            }
                        }

                        // Offcut/Scrap at the end
                        if bar.remainder_m > 0.01 {
                            let rem_x1 = rect.min.x + (bar.used_length_m as f32 / total_stock_m) * bar_width;
                            let rem_rect = egui::Rect::from_min_max(
                                egui::Pos2::new(rem_x1.min(rect.max.x), rect.min.y),
                                egui::Pos2::new(rect.max.x, rect.max.y),
                            );
                            let rem_color = if bar.is_reusable_offcut {
                                Color32::from_rgba_premultiplied(14, 165, 233, 100)
                            } else {
                                Color32::from_rgba_premultiplied(239, 68, 68, 120)
                            };
                            ui.painter().rect_filled(rem_rect, CornerRadius::same(4), rem_color);

                            if rem_rect.width() > 50.0 {
                                let rem_text = if bar.is_reusable_offcut {
                                    format!("Offcut {:.2}m", bar.remainder_m)
                                } else {
                                    format!("Scrap {:.2}m", bar.remainder_m)
                                };
                                ui.painter().text(
                                    rem_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    rem_text,
                                    egui::FontId::proportional(9.5),
                                    Color32::WHITE,
                                );
                            }
                        }

                        ui.add_space(4.0);
                        let sub_info = if bar.is_reusable_offcut {
                            format!("Remainder: {:.2}m (Meets reusable threshold ≥ {:.1}m for stiffeners/plates)", bar.remainder_m, report.min_reusable_m)
                        } else if bar.remainder_m > 0.01 {
                            format!("Net Scrap Offcut: {:.2}m (Kerf loss: {:.1}mm)", bar.remainder_m, bar.kerf_loss_m * 1e3)
                        } else {
                            "Fully utilized with zero waste".to_string()
                        };
                        ui.label(RichText::new(sub_info).size(10.5).color(theme.text_muted));
                        ui.add_space(8.0);
                    });
                    ui.separator();
                }

                ui.add_space(10.0);

                // 6. Real Action Export Buttons
                ui.horizontal_wrapped(|ui| {
                    if crate::icons::icon_button(
                        ui,
                        crate::icons::AppIcon::UploadImport,
                        "Export Bar Bending Schedule (BBS) .CSV",
                        12.0,
                        Color32::WHITE,
                        theme.accent_blue,
                        egui::Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::ZERO,
                    ).clicked() {
                        let bbs = if !bim_project.rebars.is_empty() {
                            CuttingStockOptimizer::extract_bbs_from_bim_rebar(bim_project)
                        } else if !members.is_empty() {
                            CuttingStockOptimizer::generate_sample_bbs(members.len())
                        } else {
                            Vec::new()
                        };
                        if bbs.is_empty() {
                            self.export_status = Some("No BIM rebar or structural members loaded yet. Author elements on the canvas before exporting BBS.".to_string());
                        } else {
                            let csv = CuttingStockOptimizer::export_bbs_csv(&bbs);
                            match crate::core::web_bridge::save_text_file("Bar_Bending_Schedule_SNI2847.csv", &csv, "text/csv") {
                                Ok(msg) => self.export_status = Some(format!("BBS exported ({} marks): {}", bbs.len(), msg)),
                                Err(err) => self.export_status = Some(format!("BBS export error: {err}")),
                            }
                        }
                    }

                    if crate::icons::icon_button(
                        ui,
                        crate::icons::AppIcon::Document,
                        "Export Cutting Plan .CSV",
                        12.0,
                        Color32::WHITE,
                        Color32::from_rgb(16, 185, 129),
                        egui::Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::ZERO,
                    ).clicked() {
                        let csv = CuttingStockOptimizer::export_cutting_plan_csv(&report);
                        match crate::core::web_bridge::save_text_file("Cutting_Plan_Optimized.csv", &csv, "text/csv") {
                            Ok(msg) => self.export_status = Some(format!("Cutting plan exported: {}", msg)),
                            Err(err) => self.export_status = Some(format!("Cutting plan export error: {err}")),
                        }
                    }
                });
                ui.add_space(4.0);
                ui.add(
                    egui::Label::new(
                        RichText::new("Standard format interoperable with Voortman CAM, Tekla FabTrol, and CNC Bar Benders.")
                            .size(11.0)
                            .color(theme.text_secondary),
                    )
                    .wrap(),
                );
            });
    }

    fn render_last_planner(&mut self, ui: &mut Ui, theme: &MacTheme) {
        let (committed, completed, live_ppc) = LpsLookaheadTask::compute_live_ppc(&self.lps_tasks);
        let sound_count = self.lps_tasks.iter().filter(|t| t.is_sound).count();
        let sound_pct = if self.lps_tasks.is_empty() {
            0.0
        } else {
            (sound_count as f32 / self.lps_tasks.len() as f32) * 100.0
        };
        let open_holds = self.lps_tasks.iter().filter(|t| !t.is_sound).count();
        let ppc_col = if live_ppc >= 80.0 {
            theme.accent_green
        } else if live_ppc >= 65.0 {
            theme.accent_orange
        } else {
            theme.accent_red
        };

        let cards = [
            (
                "LIVE WEEKLY PPC RELIABILITY",
                format!("{:.1}%", live_ppc),
                format!("Benchmark Target >= 80.0% ({}/{})", completed, committed),
                ppc_col,
            ),
            (
                "6-WEEK MAKE-READY SOUNDNESS",
                format!("{:.0}%", sound_pct),
                format!("{}/{} Activities Constraint-Free", sound_count, self.lps_tasks.len()),
                theme.accent_blue,
            ),
            (
                "ACTIVE CONSTRAINT HOLDS",
                format!("{} Holds", open_holds),
                "Resolve RFI / Material / Crane blockers".to_string(),
                if open_holds == 0 { theme.accent_green } else { theme.accent_orange },
            ),
            (
                "COMMITTED WWP TASKS",
                format!("{} Promises", committed),
                format!("{} Uncommitted Backlog items", self.lps_tasks.len().saturating_sub(committed as usize)),
                theme.text_primary,
            ),
        ];
        self.render_responsive_cards(ui, theme, &cards);
        ui.add_space(14.0);

        // Interactive 6-Week Lookahead Board & Constraint Log
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                let card_w = ui.available_width().max(140.0);
                ui.set_width(card_w);
                ui.set_max_width(card_w);
                ui.horizontal_wrapped(|ui| {
                    crate::icons::icon_label(
                        ui,
                        crate::icons::AppIcon::Clipboard,
                        14.0,
                        theme.accent_blue,
                        "6-Week Lookahead & WWP",
                        14.0,
                        theme.text_primary,
                        true,
                    );
                    Frame::new()
                        .fill(ppc_col.gamma_multiply(0.16))
                        .corner_radius(CornerRadius::same(5))
                        .inner_margin(Margin::symmetric(8, 2))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("LIVE PPC: {:.1}%", live_ppc))
                                    .size(10.5)
                                    .strong()
                                    .color(ppc_col),
                            );
                        });
                });
                ui.add_space(8.0);

                // Week filter pills
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Week Filter:").size(11.0).color(theme.text_secondary));
                    for wk in ["All Weeks", "W-01", "W-02", "W-03", "W-04", "W-05", "W-06"] {
                        if ui.selectable_label(self.lps_week_filter == wk, wk).clicked() {
                            self.lps_week_filter = wk.to_string();
                        }
                    }
                });

                ui.add_space(8.0);

                // Quick Commitment Creator Bar
                ui.horizontal_wrapped(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.lps_new_task_name)
                            .hint_text("Add new lookahead commitment (e.g., Curtain wall gasket torque)...")
                            .desired_width((ui.available_width() * 0.45).clamp(160.0, 360.0)),
                    );
                    egui::ComboBox::from_id_salt("lps_new_wk_combo")
                        .selected_text(RichText::new(&self.lps_new_task_week).size(11.0))
                        .width(75.0)
                        .show_ui(ui, |ui| {
                            for wk in ["W-04", "W-05", "W-06"] {
                                ui.selectable_value(&mut self.lps_new_task_week, wk.to_string(), wk);
                            }
                        });
                    if crate::icons::icon_button(
                        ui,
                        crate::icons::AppIcon::Plus,
                        "Add Work Package",
                        11.0,
                        Color32::WHITE,
                        theme.accent_blue,
                        egui::Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::ZERO,
                    )
                    .clicked()
                        && !self.lps_new_task_name.trim().is_empty()
                    {
                        let next_id = self.lps_tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
                        self.lps_tasks.push(LpsLookaheadTask {
                            id: next_id,
                            wbs_code: format!("05.{:02}.{:02}", next_id, next_id * 2),
                            week_bucket: self.lps_new_task_week.clone(),
                            activity_name: self.lps_new_task_name.trim().to_string(),
                            trade_crew: "Field Erection Crew".to_string(),
                            constraint_category: LpsConstraintCategory::ConstraintFree,
                            constraint_note: "Pre-screened constraint-free".to_string(),
                            is_sound: true,
                            committed_in_wwp: true,
                            completed_on_time: false,
                            variance_reason: "Pending Execution".to_string(),
                        });
                        self.lps_new_task_name.clear();
                    }
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                // Task List
                let mut visible_rows = 0usize;
                for task in &mut self.lps_tasks {
                    if self.lps_week_filter != "All Weeks" && task.week_bucket != self.lps_week_filter {
                        continue;
                    }
                    visible_rows += 1;

                    let row_col = if task.completed_on_time && task.committed_in_wwp {
                        theme.accent_green
                    } else if !task.is_sound {
                        theme.accent_orange
                    } else {
                        theme.accent_blue
                    };

                    Frame::new()
                        .fill(theme.bg_canvas)
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::symmetric(12, 8))
                        .stroke(Stroke::new(0.8, theme.border_subtle))
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                Frame::new()
                                    .fill(theme.accent_blue.gamma_multiply(0.16))
                                    .corner_radius(CornerRadius::same(4))
                                    .inner_margin(Margin::symmetric(6, 2))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(&task.week_bucket)
                                                .size(10.0)
                                                .strong()
                                                .color(theme.accent_blue),
                                        );
                                    });
                                ui.label(
                                    RichText::new(&task.wbs_code)
                                        .size(10.0)
                                        .monospace()
                                        .color(theme.text_muted),
                                );
                                ui.label(
                                    RichText::new(&task.activity_name)
                                        .size(12.0)
                                        .strong()
                                        .color(theme.text_primary),
                                );
                                ui.label(
                                    RichText::new(format!("• {}", task.trade_crew))
                                        .size(10.5)
                                        .color(theme.text_secondary),
                                );
                            });

                            ui.add_space(4.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "{} — {}",
                                        task.constraint_category.label(),
                                        task.constraint_note
                                    ))
                                    .size(10.5)
                                    .color(row_col),
                                );

                                let (sound_icon, sound_lbl, sound_col) = if task.is_sound {
                                    (crate::icons::AppIcon::Unlock, "Sound Ready", theme.accent_green)
                                } else {
                                    (crate::icons::AppIcon::Lock, "Clear Constraint", theme.accent_orange)
                                };
                                if crate::icons::icon_button(
                                    ui,
                                    sound_icon,
                                    sound_lbl,
                                    10.0,
                                    sound_col,
                                    theme.bg_card,
                                    egui::Stroke::NONE,
                                    CornerRadius::same(4),
                                    Vec2::ZERO,
                                )
                                .clicked()
                                {
                                    task.is_sound = !task.is_sound;
                                    if task.is_sound {
                                        task.constraint_category = LpsConstraintCategory::ConstraintFree;
                                        task.constraint_note = "Constraint cleared by site engineer".to_string();
                                    } else {
                                        task.constraint_category = LpsConstraintCategory::MaterialSupply;
                                        task.completed_on_time = false;
                                        if task.variance_reason == "None" {
                                            task.variance_reason = "Material / Access Hold".to_string();
                                        }
                                    }
                                }

                                if ui
                                    .checkbox(&mut task.committed_in_wwp, "Committed (WWP)")
                                    .changed()
                                    && !task.committed_in_wwp
                                {
                                    task.completed_on_time = false;
                                }

                                if ui
                                    .add_enabled(
                                        task.committed_in_wwp,
                                        egui::Checkbox::new(&mut task.completed_on_time, "Completed On-Time"),
                                    )
                                    .changed()
                                    && task.completed_on_time
                                {
                                    task.is_sound = true;
                                    task.variance_reason = "None".to_string();
                                }
                            });
                        });
                    ui.add_space(4.0);
                }

                if visible_rows == 0 {
                    ui.label(
                        RichText::new("No lookahead activities scheduled in selected week filter.")
                            .size(11.0)
                            .color(theme.text_muted),
                    );
                }
            });

        ui.add_space(16.0);

        // PPC Trend Chart + Variance Root-Cause Pareto (Responsive Split)
        let draw_ppc_chart = |ui: &mut Ui| {
            Frame::new()
                .fill(theme.bg_card)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(16))
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .show(ui, |ui| {
                    crate::icons::icon_label(
                        ui,
                        crate::icons::AppIcon::ChartLine,
                        13.0,
                        theme.accent_blue,
                        "Weekly PPC Trend vs 80% Benchmark",
                        13.0,
                        theme.text_primary,
                        true,
                    );
                    ui.label(
                        RichText::new("Historical weekly reliability progression including live WWP simulation")
                            .size(10.5)
                            .color(theme.text_muted),
                    );
                    ui.add_space(10.0);

                    let chart_w = (ui.available_width() - 4.0).max(200.0);
                    let chart_h = 165.0;
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(chart_w, chart_h), egui::Sense::hover());
                    let p = ui.painter();
                    p.rect_filled(rect, CornerRadius::same(6), Color32::from_rgb(16, 20, 28));
                    p.rect_stroke(rect, CornerRadius::same(6), Stroke::new(0.8, theme.border_subtle), StrokeKind::Inside);

                    let pad_x = 26.0;
                    let pad_top = 18.0;
                    let pad_bot = 24.0;
                    let plot_h = (rect.height() - pad_top - pad_bot).max(40.0);
                    let baseline_y = rect.bottom() - pad_bot;

                    // 80% Benchmark Line
                    let target_y = baseline_y - plot_h * 0.80;
                    p.line_segment(
                        [Pos2::new(rect.left() + pad_x, target_y), Pos2::new(rect.right() - 10.0, target_y)],
                        Stroke::new(1.2, theme.accent_orange),
                    );
                    p.text(
                        Pos2::new(rect.left() + pad_x + 4.0, target_y - 10.0),
                        Align2::LEFT_TOP,
                        "80% Target",
                        egui::FontId::monospace(9.0),
                        theme.accent_orange,
                    );

                    // Bars for W-01..W-05 + Live WWP
                    let mut series: Vec<(String, f32, bool)> = self
                        .lps_history
                        .iter()
                        .map(|h| (h.week_label.clone(), h.ppc_pct, false))
                        .collect();
                    series.push(("Live".to_string(), live_ppc, true));

                    let slot_w = (rect.width() - pad_x - 16.0) / (series.len().max(1) as f32);
                    let bar_w = (slot_w * 0.56).clamp(14.0, 38.0);

                    for (idx, (lbl, pct, is_live)) in series.iter().enumerate() {
                        let cx = rect.left() + pad_x + slot_w * (idx as f32 + 0.5);
                        let frac = (*pct / 100.0).clamp(0.0, 1.0);
                        let bh = plot_h * frac;
                        let brect = Rect::from_min_max(
                            Pos2::new(cx - bar_w * 0.5, baseline_y - bh),
                            Pos2::new(cx + bar_w * 0.5, baseline_y),
                        );
                        let bcol = if *is_live {
                            if *pct >= 80.0 { theme.accent_green } else { theme.accent_red }
                        } else if *pct >= 80.0 {
                            theme.accent_blue
                        } else {
                            theme.accent_orange
                        };
                        p.rect_filled(brect, CornerRadius::same(4), bcol);
                        p.text(
                            Pos2::new(cx, brect.top() - 11.0),
                            Align2::CENTER_TOP,
                            format!("{:.0}%", pct),
                            egui::FontId::monospace(9.0),
                            Color32::WHITE,
                        );
                        p.text(
                            Pos2::new(cx, baseline_y + 5.0),
                            Align2::CENTER_TOP,
                            lbl,
                            egui::FontId::monospace(9.0),
                            theme.text_muted,
                        );
                    }
                });
        };

        let draw_pareto_chart = |ui: &mut Ui| {
            let pareto = LpsLookaheadTask::compute_variance_pareto(&self.lps_tasks);
            Frame::new()
                .fill(theme.bg_card)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(16))
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .show(ui, |ui| {
                    crate::icons::icon_label(
                        ui,
                        crate::icons::AppIcon::ChartBar,
                        13.0,
                        theme.accent_blue,
                        "Variance Root-Cause Pareto (Non-Completions)",
                        13.0,
                        theme.text_primary,
                        true,
                    );
                    ui.label(
                        RichText::new("Primary blockers preventing 100% Weekly Work Plan reliability")
                            .size(10.5)
                            .color(theme.text_muted),
                    );
                    ui.add_space(10.0);

                    if pareto.is_empty() {
                        Frame::new()
                            .fill(theme.accent_green.gamma_multiply(0.14))
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::same(14))
                            .show(ui, |ui| {
                                crate::icons::icon_label(
                                    ui,
                                    crate::icons::AppIcon::CheckCircle,
                                    12.0,
                                    theme.accent_green,
                                    "100% Reliability! Zero open variance reasons in current Weekly Work Plan.",
                                    11.5,
                                    theme.accent_green,
                                    true,
                                );
                            });
                    } else {
                        let total_var: u32 = pareto.iter().map(|(_, c)| *c).sum();
                        for (reason, count) in pareto {
                            let share = (count as f32 / total_var.max(1) as f32).clamp(0.0, 1.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(&reason).size(11.0).strong().color(theme.text_primary));
                                ui.label(
                                    RichText::new(format!("• {} holds ({:.0}%)", count, share * 100.0))
                                        .size(10.5)
                                        .strong()
                                        .color(theme.accent_orange),
                                );
                            });
                            let bw = (ui.available_width() - 4.0).max(120.0);
                            let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(bw, 10.0), egui::Sense::hover());
                            ui.painter().rect_filled(bar_rect, CornerRadius::same(4), theme.bg_canvas);
                            let fill_rect = Rect::from_min_size(bar_rect.min, Vec2::new(bw * share, 10.0));
                            ui.painter().rect_filled(fill_rect, CornerRadius::same(4), theme.accent_orange);
                            ui.add_space(6.0);
                        }
                    }
                });
        };

        ui_kit::responsive_two_columns(ui, 800.0, draw_ppc_chart, draw_pareto_chart);
    }

    fn render_value_stream(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                let card_w = ui.available_width().max(140.0);
                ui.set_width(card_w);
                ui.set_max_width(card_w);
                crate::icons::icon_label(
                    ui,
                    crate::icons::AppIcon::ChartLine,
                    13.5,
                    theme.accent_blue,
                    "Value Stream: 7 Construction Wastes",
                    13.5,
                    theme.text_primary,
                    true,
                );
                ui.add_space(10.0);

                let wastes = [
                    ("1. Overproduction", "Fabricate modules strictly aligned with 4D BIM erection sequence", "Eliminates onsite laydown congestion (-100%)"),
                    ("2. Waiting Time", "Crane mobilized Just-In-Time with modular arrivals", "Crane rental cost reduced 35%"),
                    ("3. Transportation", "Direct transport routes from fabrication yard to site", "Reduces haul distance and diesel emissions"),
                    ("4. Overprocessing", "Standardized HTB bolted splices without field welding", "Factory-controlled joint quality verified"),
                    ("5. Inventory", "Laydown buffer restricted to maximum 2 days work", "Clear jobsite footprint for equipment staging"),
                    ("6. Motion", "Laydown organized within crane boom slewing radius", "Pick-and-place cycle duration reduced 25%"),
                    ("7. Defects / Rework", "SVD Kinematic Guard simulation prevents erection instabilities", "Zero mechanism collapse or rework disassembly"),
                ];

                for (name, action, result) in wastes {
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!("{name} — {action} ({result})"))
                                .size(11.5)
                                .color(theme.text_secondary),
                        )
                        .wrap(),
                    );
                    ui.add_space(4.0);
                }
            });
    }

    fn render_jit_delivery(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                let card_w = ui.available_width().max(140.0);
                ui.set_width(card_w);
                ui.set_max_width(card_w);
                crate::icons::icon_label(
                    ui,
                    crate::icons::AppIcon::Truck,
                    13.5,
                    theme.accent_blue,
                    "Just-In-Time (JIT) Delivery Buffer",
                    13.5,
                    theme.text_primary,
                    true,
                );
                ui.add_space(10.0);

                let cards = [
                    ("MAX ONSITE STAGING BUFFER", "Max 2 Days".to_string(), "Prevents Jobsite Congestion".to_string(), theme.accent_green),
                    ("DIRECT RIGGING FROM TRUCK", "70% Components".to_string(), "Rigged Directly from Trailer Bed to Position".to_string(), theme.accent_blue),
                    ("DOUBLE HANDLING REDUCTION", "-80%".to_string(), "Mitigates Structural Impact Risks".to_string(), theme.text_primary),
                ];
                self.render_responsive_cards(ui, theme, &cards);
            });
    }

    fn render_takt_concrete_dispatch(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        bim_project: &crate::core::bim::BimProject,
        features: &mut FeatureRegistry,
    ) {
        let rep = compute_takt_concrete_dispatch(
            bim_project,
            self.takt_days_per_zone,
            self.takt_transit_min,
            self.takt_mixer_cap_m3,
            self.takt_alert_threshold_min,
        );
        ui_kit::feature_slot_card(ui, theme, features, FeatureId::TaktTimeConcretePourDispatch, |ui| {
            let cad_val = format!("{} Days / Zone", self.takt_days_per_zone);
            let cad_sub = format!(
                "Zone A -> B -> C ({} BIM Levels)",
                bim_project.levels.len().max(1)
            );
            let tr_val = format!("{:.0} min Avg Transit", self.takt_transit_min);
            let tr_sub = format!(
                "Slump Margin: {:+.0} min vs 90 min ASTM C94 Cap",
                rep.slump_margin_min
            );
            let tr_col = if rep.slump_margin_min >= 15.0 {
                theme.accent_green
            } else if rep.slump_margin_min >= 0.0 {
                theme.accent_orange
            } else {
                theme.accent_red
            };
            let pour_val = format!("{:.1} m³ / Shift", rep.shift_pour_m3);
            let pour_sub = format!(
                "{} Mixers ({:.1} m³/truck @ {:.1} min headway) • Total BIM {:.1} m³",
                rep.total_trucks_shift,
                self.takt_mixer_cap_m3,
                rep.dispatch_headway_min,
                rep.total_bim_m3
            );
            let risk_val = if rep.delayed_trucks == 0 {
                "0 Cold-Joint Risks".to_string()
            } else {
                format!("{} Truck Warning(s)", rep.delayed_trucks)
            };
            let risk_col = if rep.delayed_trucks == 0 {
                theme.accent_green
            } else {
                theme.accent_orange
            };
            let cards = [
                ("TAKT ZONE CADENCE", cad_val, cad_sub, theme.accent_blue),
                ("READY-MIX TRANSIT WINDOW", tr_val, tr_sub, tr_col),
                ("CONTINUOUS POUR VOLUME", pour_val, pour_sub, theme.accent_blue),
                (
                    "COLD-JOINT RISK MONITOR",
                    risk_val,
                    format!("Alert Threshold = {:.0} min", self.takt_alert_threshold_min),
                    risk_col,
                ),
            ];
            ui_kit::responsive_grid_n(ui, cards.len(), 200.0, |ui, idx| {
                let (t, v, s, c) = &cards[idx];
                self.render_stat_card(ui, theme, t, v, s, *c);
            });

            ui.add_space(10.0);
            ui.spacing_mut().slider_width = (ui.available_width() * 0.45).clamp(72.0, 140.0);
            ui.horizontal_wrapped(|ui| {
                ui_kit::wrap_before(ui, 160.0);
                ui.label(RichText::new("Takt (d/zone):").size(11.0).color(theme.text_secondary));
                ui.add(egui::Slider::new(&mut self.takt_days_per_zone, 2..=10));
                ui_kit::wrap_before(ui, 165.0);
                ui.label(RichText::new("Transit (min):").size(11.0).color(theme.text_secondary));
                ui.add(egui::Slider::new(&mut self.takt_transit_min, 15.0..=105.0).step_by(1.0));
                ui_kit::wrap_before(ui, 160.0);
                ui.label(RichText::new("Mixer (m³):").size(11.0).color(theme.text_secondary));
                ui.add(egui::Slider::new(&mut self.takt_mixer_cap_m3, 5.0..=10.0).step_by(0.5));
                if ui
                    .selectable_label(
                        self.takt_alert_threshold_min <= 75.0,
                        "Alert on Transit > 75 min",
                    )
                    .clicked()
                {
                    self.takt_alert_threshold_min =
                        if self.takt_alert_threshold_min <= 75.0 { 90.0 } else { 75.0 };
                }
                if crate::icons::icon_button(
                    ui,
                    crate::icons::AppIcon::Layers,
                    "Export Takt & Mixer Dispatch (.CSV)",
                    12.0,
                    theme.text_primary,
                    theme.bg_card,
                    egui::Stroke::new(1.0, theme.border_subtle),
                    CornerRadius::same(6),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    let mut csv = String::from("Zone,Pour_m3,TrucksNeeded,Headway_min,ColdJointStatus\n");
                    for z in &rep.zones {
                        csv.push_str(&format!(
                            "{},{:.1},{},{:.1},{}\n",
                            z.zone_name, z.pour_m3, z.trucks_needed, z.headway_min, z.cold_joint_status
                        ));
                    }
                    self.export_status = Some(
                        match crate::core::web_bridge::save_text_file("takt_concrete_dispatch.csv", &csv, "text/csv") {
                            Ok(msg) => format!(
                                "Takt Pour Dispatch ({} trucks, {:.1} min headway) • {msg}",
                                rep.total_trucks_shift, rep.dispatch_headway_min
                            ),
                            Err(e) => format!("Takt CSV export failed: {e}"),
                        },
                    );
                }
            });
            ui.add_space(10.0);

            ScrollArea::horizontal().id_salt("takt_wagon_scroll").show(ui, |ui| {
                egui::Grid::new("takt_wagon_grid")
                    .striped(true)
                    .min_col_width(88.0)
                    .spacing(Vec2::new(14.0, 6.0))
                    .show(ui, |ui| {
                        for h in ["Zone / Level", "W1 Trade", "W2 Trade", "W3 Pour Gate", "Mixer Headway", "Pour Batching"] {
                            ui.label(RichText::new(h).size(10.5).strong().color(theme.accent_blue));
                        }
                        ui.end_row();
                        let trades = [
                            ("Rebar Deck", "MEP Sleeves", "Slab Pour [Done]"),
                            ("Formwork", "Rebar Deck", "Slab Pour [Active]"),
                            ("Shoring", "Formwork", "MEP & Pour Prep"),
                        ];
                        for (idx, z) in rep.zones.iter().enumerate() {
                            let (t1, t2, t3) = trades[idx % trades.len()];
                            let st_col = if z.cold_joint_status.contains("Delay") {
                                theme.accent_orange
                            } else {
                                theme.accent_green
                            };
                            ui.label(RichText::new(z.zone_name).size(10.5).strong().color(theme.text_primary));
                            ui.label(RichText::new(t1).size(10.5).color(theme.text_secondary));
                            ui.label(RichText::new(t2).size(10.5).color(theme.accent_blue));
                            ui.label(RichText::new(t3).size(10.5).color(st_col));
                            ui.label(RichText::new(format!("{:.1} min/truck", z.headway_min)).size(10.5).monospace().color(theme.accent_orange));
                            ui.label(RichText::new(format!("{:.1} m³ ({} mixers)", z.pour_m3, z.trucks_needed)).size(10.5).color(theme.text_muted));
                            ui.end_row();
                        }
                    });
            });
        });
    }

    fn render_rfi_submittal_tracker(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        features: &mut FeatureRegistry,
    ) {
        let sum = evaluate_rfi_sla_status(&self.rfi_items, self.rfi_sla_days);
        let overdue_col = if sum.overdue_count == 0 {
            theme.accent_green
        } else {
            theme.accent_red
        };
        let hold_col = if sum.lps_holds_count == 0 {
            theme.accent_green
        } else {
            theme.accent_orange
        };
        ui_kit::feature_slot_card(ui, theme, features, FeatureId::RfiSubmittalSlaTracker, |ui| {
            let cards = [
                (
                    "OPEN STRUCTURAL RFIs",
                    format!("{} Open RFIs/Sub", sum.open_count),
                    format!("Avg Age = {:.1} Business Days", sum.avg_age_days),
                    theme.accent_blue,
                ),
                (
                    "SLA ESCALATION BREACH",
                    format!("{} Overdue (> {}d)", sum.overdue_count, self.rfi_sla_days),
                    "Contractual EOR / Consultant Turnaround Gate".to_string(),
                    overdue_col,
                ),
                (
                    "SHOP DRAWING APPROVAL",
                    format!("{:.0}% Approved", sum.approval_pct),
                    "Steel Splice Plates & Rebar BBS Packages".to_string(),
                    theme.accent_green,
                ),
                (
                    "LPS CONSTRAINT COUPLING",
                    format!("{} Linked Holds", sum.lps_holds_count),
                    if self.rfi_auto_flag_lps {
                        "Auto-Clears Lookahead Task on Sign-Off".to_string()
                    } else {
                        "Manual Lookahead Unlock Mode".to_string()
                    },
                    hold_col,
                ),
            ];
            ui_kit::responsive_grid_n(ui, cards.len(), 200.0, |ui, idx| {
                let (t, v, s, c) = &cards[idx];
                self.render_stat_card(ui, theme, t, v, s, *c);
            });

            ui.add_space(10.0);
            ui.spacing_mut().slider_width = (ui.available_width() * 0.45).clamp(72.0, 140.0);
            ui.horizontal_wrapped(|ui| {
                ui_kit::wrap_before(ui, 170.0);
                ui.label(RichText::new("Contractual SLA (d):").size(11.0).color(theme.text_secondary));
                ui.add(egui::Slider::new(&mut self.rfi_sla_days, 2..=14).suffix(" d"));
                ui.checkbox(&mut self.rfi_auto_flag_lps, "Auto-Clear LPS Lookahead Holds");
                if crate::icons::icon_button(
                    ui,
                    crate::icons::AppIcon::Play,
                    "Advance +1 Day",
                    11.5,
                    theme.text_primary,
                    theme.bg_card,
                    egui::Stroke::new(1.0, theme.border_subtle),
                    CornerRadius::same(6),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    for item in &mut self.rfi_items {
                        if !item.approved {
                            item.age_days += 1;
                        }
                    }
                    self.export_status = Some("Simulated +1 business day across all open RFIs & Submittals.".to_string());
                }
                if crate::icons::icon_button(
                    ui,
                    crate::icons::AppIcon::Document,
                    "Export RFI SLA Ledger (.csv)",
                    11.5,
                    theme.text_primary,
                    theme.bg_card,
                    egui::Stroke::new(1.0, theme.border_subtle),
                    CornerRadius::same(6),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    let mut csv = String::from("RefID,Subject,Owner,AgeDays,SLA_Days,LinkedLPS,Status\n");
                    for r in &self.rfi_items {
                        let st = if r.approved {
                            "APPROVED"
                        } else if r.age_days > self.rfi_sla_days {
                            "CRITICAL"
                        } else {
                            "OPEN"
                        };
                        csv.push_str(&format!(
                            "{},\"{}\",\"{}\",{},{},\"{}\",{}\n",
                            r.ref_id, r.subject, r.owner, r.age_days, self.rfi_sla_days, r.linked_lps_task, st
                        ));
                    }
                    match crate::core::web_bridge::save_text_file("EigenGraph_RFI_Submittal_SLA.csv", &csv, "text/csv") {
                        Ok(msg) => self.export_status = Some(format!("RFI SLA CSV exported ({msg})")),
                        Err(err) => self.export_status = Some(format!("RFI export error: {err}")),
                    }
                }
            });
            ui.add_space(10.0);

            let mut approve_idx: Option<usize> = None;
            ScrollArea::horizontal().id_salt("rfi_sla_scroll").show(ui, |ui| {
                egui::Grid::new("rfi_sla_grid")
                    .striped(true)
                    .min_col_width(82.0)
                    .spacing(Vec2::new(12.0, 6.0))
                    .show(ui, |ui| {
                        for h in ["Ref ID", "Subject & BIM Element", "Ball-in-Court", "Age / SLA", "Linked LPS Task", "Priority", "Action"] {
                            ui.label(RichText::new(h).size(10.5).strong().color(theme.accent_blue));
                        }
                        ui.end_row();
                        for (idx, r) in self.rfi_items.iter().enumerate() {
                            let (st, col) = if r.approved {
                                ("APPROVED".to_string(), theme.accent_green)
                            } else if r.age_days > self.rfi_sla_days {
                                ("CRITICAL".to_string(), theme.accent_red)
                            } else {
                                ("OPEN".to_string(), theme.accent_orange)
                            };
                            let sla_txt = if r.approved {
                                format!("Closed @ {}d", r.age_days)
                            } else if r.age_days > self.rfi_sla_days {
                                format!("{}d / {}d (+{}d)", r.age_days, self.rfi_sla_days, r.age_days - self.rfi_sla_days)
                            } else {
                                format!("{}d / {}d", r.age_days, self.rfi_sla_days)
                            };
                            ui.label(RichText::new(&r.ref_id).size(10.5).monospace().color(theme.accent_blue));
                            ui.label(RichText::new(&r.subject).size(10.5).strong().color(theme.text_primary));
                            ui.label(RichText::new(&r.owner).size(10.5).color(theme.text_secondary));
                            ui.label(RichText::new(sla_txt).size(10.5).color(col));
                            ui.label(RichText::new(&r.linked_lps_task).size(10.5).color(theme.text_muted));
                            ui.label(RichText::new(st).size(10.5).strong().color(col));
                            if !r.approved {
                                if ui.small_button("Sign-Off & Clear").clicked() {
                                    approve_idx = Some(idx);
                                }
                            } else {
                                ui.label(RichText::new("Verified").size(10.0).color(theme.accent_green));
                            }
                            ui.end_row();
                        }
                    });
            });
            if let Some(i) = approve_idx {
                if let Some(item) = self.rfi_items.get_mut(i) {
                    item.approved = true;
                    if self.rfi_auto_flag_lps && item.linked_lps_task.contains("Hold") {
                        item.linked_lps_task = item.linked_lps_task.replace("Hold", "Clear");
                    }
                    if let Some(first_blocked) = self.lps_tasks.iter_mut().find(|t| !t.is_sound) {
                        first_blocked.is_sound = true;
                    }
                    self.export_status = Some(format!(
                        "Approved {} — Lookahead constraint '{}' cleared in Lean LPS board.",
                        item.ref_id, item.linked_lps_task
                    ));
                }
            }
        });
    }

    fn render_qa_qc_lab_and_commissioning(&mut self, ui: &mut Ui, theme: &MacTheme) {
        let score = self.qa_qc.evaluate_scorecard();
        let site = self.site_coupling.clone();

        theme.card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                crate::icons::icon_label(
                    ui,
                    crate::icons::AppIcon::CheckCircle,
                    14.0,
                    theme.accent_green,
                    "FIELD QA/QC LAB (CYLINDER f'c), ITP HOLD POINTS, NCR PUNCH LIST & MEP T&C",
                    12.5,
                    theme.accent_green,
                    true,
                );
                crate::icons::pill_badge(
                    ui,
                    crate::icons::AppIcon::Globe,
                    format!("Earth Site: {} ({})", site.site_label, site.recommended_soil_class),
                    theme.accent_blue,
                );
            });
            ui.add_space(10.0);

            self.render_responsive_cards(
                ui,
                theme,
                &[
                    (
                        "28-Day Cylinder Pass Rate",
                        format!("{:.1}%", score.fc_pass_rate_pct),
                        format!("{} pour batches tested (SNI/ASTM C39)", self.qa_qc.batches.len()),
                        if score.fc_pass_rate_pct >= 80.0 {
                            theme.accent_green
                        } else {
                            theme.accent_orange
                        },
                    ),
                    (
                        "ITP Hold Points Cleared",
                        format!("{}/{}", score.itp_cleared_count, score.itp_total_count),
                        "Witness / Hold inspection sign-off gate".to_string(),
                        theme.accent_blue,
                    ),
                    (
                        "Open NCR / Punch Items",
                        format!("{}", score.open_ncr_count),
                        "Non-conformance closeout status".to_string(),
                        if score.open_ncr_count == 0 {
                            theme.accent_green
                        } else {
                            theme.accent_red
                        },
                    ),
                    (
                        "MEP T&C Witness Readiness",
                        format!("{:.1}%", score.mep_tc_pass_pct),
                        "Hydrostatic, Megger & TAB balancing".to_string(),
                        if score.mep_tc_pass_pct >= 85.0 {
                            theme.accent_green
                        } else {
                            theme.accent_orange
                        },
                    ),
                ],
            );

            ui.add_space(12.0);
            ui.label(RichText::new("1. CONCRETE CYLINDER LAB & SLUMP LOG").size(11.0).strong().color(theme.accent_blue));
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                let zw = ui_kit::clamped_row_width(ui, 110.0, 210.0, 190.0);
                ui.label(RichText::new("Pour Zone:").size(10.5).color(theme.text_secondary));
                ui.add(egui::TextEdit::singleline(&mut self.new_qc_zone).desired_width(zw));
                ui.label(RichText::new("28d MPa:").size(10.5).color(theme.text_secondary));
                ui.add(egui::DragValue::new(&mut self.new_qc_day28_mpa).range(15.0..=85.0).speed(0.5));
                if crate::icons::icon_button(
                    ui,
                    crate::icons::AppIcon::Plus,
                    "Log Test Batch",
                    10.5,
                    Color32::WHITE,
                    theme.accent_blue,
                    Stroke::NONE,
                    CornerRadius::same(6),
                    Vec2::new(0.0, 24.0),
                )
                .clicked()
                {
                    let n = 2601 + self.qa_qc.batches.len();
                    self.qa_qc.batches.push(crate::core::ConcreteCylinderBatch {
                        batch_id: format!("CY-{n}"),
                        pour_zone: self.new_qc_zone.clone(),
                        target_fc_mpa: 40.0,
                        slump_mm: 125.0,
                        day7_mpa: (self.new_qc_day28_mpa * 0.72),
                        day28_mpa: self.new_qc_day28_mpa,
                    });
                    self.export_status = Some(format!("Logged Concrete Cylinder Batch CY-{n}."));
                }
            });
            ui.add_space(6.0);
            ScrollArea::horizontal().id_salt("qa_qc_lab_scroll").show(ui, |ui| {
                egui::Grid::new("qa_qc_lab_grid")
                    .striped(true)
                    .spacing(Vec2::new(12.0, 5.0))
                    .show(ui, |ui| {
                        for h in ["Batch", "Structural Pour Zone", "Target f'c", "Slump", "7-Day", "28-Day", "Lab Status"] {
                            ui.label(RichText::new(h).size(10.5).strong().color(theme.accent_blue));
                        }
                        ui.end_row();
                        for b in &self.qa_qc.batches {
                            let ok = b.passes_fc();
                            ui.label(RichText::new(&b.batch_id).size(10.5).monospace().color(theme.text_primary));
                            ui.label(RichText::new(&b.pour_zone).size(10.5).color(theme.text_secondary));
                            ui.label(RichText::new(format!("{:.0} MPa", b.target_fc_mpa)).size(10.5).color(theme.text_muted));
                            ui.label(RichText::new(format!("{:.0} mm", b.slump_mm)).size(10.5).color(theme.text_secondary));
                            ui.label(RichText::new(format!("{:.1} MPa", b.day7_mpa)).size(10.5).monospace().color(theme.text_muted));
                            ui.label(RichText::new(format!("{:.1} MPa", b.day28_mpa)).size(10.5).strong().color(if ok { theme.accent_green } else { theme.accent_red }));
                            ui.label(RichText::new(if ok { "PASS" } else { "CORING CHECK" }).size(10.5).strong().color(if ok { theme.accent_green } else { theme.accent_red }));
                            ui.end_row();
                        }
                    });
            });

            ui.add_space(12.0);
            ui.label(RichText::new("2. ITP HOLD POINTS & NCR / PUNCH LIST CLOSEOUT").size(11.0).strong().color(theme.accent_orange));
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                let nw = ui_kit::clamped_row_width(ui, 120.0, 260.0, 170.0);
                ui.add(egui::TextEdit::singleline(&mut self.new_ncr_desc).desired_width(nw));
                if crate::icons::icon_button(
                    ui,
                    crate::icons::AppIcon::Plus,
                    "Raise Field NCR",
                    10.5,
                    Color32::WHITE,
                    theme.accent_orange,
                    Stroke::NONE,
                    CornerRadius::same(6),
                    Vec2::new(0.0, 24.0),
                )
                .clicked()
                {
                    let n = 15 + self.qa_qc.ncrs.len();
                    self.qa_qc.ncrs.push(crate::core::NcrPunchItem {
                        ncr_id: format!("NCR-{n:03}"),
                        discipline: "STR".to_string(),
                        location: "Active Floor Zone".to_string(),
                        description: self.new_ncr_desc.clone(),
                        critical: false,
                        closed: false,
                    });
                }
            });
            ui.add_space(6.0);
            ScrollArea::horizontal().id_salt("qa_qc_itp_ncr_scroll").show(ui, |ui| {
                egui::Grid::new("qa_qc_itp_ncr_grid")
                    .striped(true)
                    .spacing(Vec2::new(12.0, 5.0))
                    .show(ui, |ui| {
                        for h in ["Ref / Type", "Scope / Location", "Inspection / Sign-Off"] {
                            ui.label(RichText::new(h).size(10.5).strong().color(theme.accent_blue));
                        }
                        ui.end_row();
                        for itp in &mut self.qa_qc.itps {
                            ui.label(RichText::new(format!("{} (HOLD)", itp.code)).size(10.5).strong().color(theme.accent_blue));
                            ui.label(RichText::new(&itp.activity).size(10.5).color(theme.text_secondary));
                            if ui.selectable_label(itp.signed_off, if itp.signed_off { "Signed Off" } else { "Sign-Off Hold" }).clicked() {
                                itp.signed_off = !itp.signed_off;
                            }
                            ui.end_row();
                        }
                        for ncr in &mut self.qa_qc.ncrs {
                            let tag_col = if ncr.closed { theme.accent_green } else if ncr.critical { theme.accent_red } else { theme.accent_orange };
                            ui.label(RichText::new(format!("{} [{}]", ncr.ncr_id, ncr.discipline)).size(10.5).strong().color(tag_col));
                            ui.label(RichText::new(format!("{}: {}", ncr.location, ncr.description)).size(10.5).color(theme.text_secondary));
                            if ui.selectable_label(ncr.closed, if ncr.closed { "Closed" } else { "Verify & Close" }).clicked() {
                                ncr.closed = !ncr.closed;
                            }
                            ui.end_row();
                        }
                    });
            });

            ui.add_space(12.0);
            ui.label(RichText::new("3. MEP TESTING & COMMISSIONING (T&C) HANDOVER GATES").size(11.0).strong().color(theme.accent_green));
            ui.add_space(6.0);
            ScrollArea::horizontal().id_salt("qa_qc_mep_tc_scroll").show(ui, |ui| {
                egui::Grid::new("qa_qc_mep_tc_grid")
                    .striped(true)
                    .spacing(Vec2::new(12.0, 5.0))
                    .show(ui, |ui| {
                        for h in ["System", "Test Protocol", "Target Spec", "Measured Value", "Witness Gate"] {
                            ui.label(RichText::new(h).size(10.5).strong().color(theme.accent_blue));
                        }
                        ui.end_row();
                        for t in &mut self.qa_qc.mep_tests {
                            ui.label(RichText::new(&t.system).size(10.5).strong().color(theme.text_primary));
                            ui.label(RichText::new(&t.protocol).size(10.5).color(theme.text_secondary));
                            ui.label(RichText::new(&t.target_spec).size(10.5).color(theme.text_muted));
                            ui.label(RichText::new(&t.measured_val).size(10.5).monospace().color(theme.accent_blue));
                            if ui.selectable_label(t.witnessed_pass, if t.witnessed_pass { "Passed" } else { "Sign T&C" }).clicked() {
                                t.witnessed_pass = !t.witnessed_pass;
                            }
                            ui.end_row();
                        }
                    });
            });
        });
    }

    fn render_stat_card(&self, ui: &mut Ui, theme: &MacTheme, title: &str, val: &str, subtitle: &str, val_color: Color32) {
        ui_kit::stat_card(ui, theme, title, val, subtitle, val_color);
    }
}

#[cfg(test)]
mod tests {
    use super::{compute_takt_concrete_dispatch, evaluate_rfi_sla_status, RfiSubmittalItem};
    use crate::core::bim::BimProject;

    #[test]
    fn test_takt_concrete_dispatch_flags_excess_transit_delay() {
        let proj = BimProject::default();
        let on_time = compute_takt_concrete_dispatch(&proj, 5, 38.0, 7.0, 75.0);
        let delayed = compute_takt_concrete_dispatch(&proj, 5, 84.0, 7.0, 75.0);
        assert_eq!(on_time.delayed_trucks, 0);
        assert!(delayed.delayed_trucks >= 1);
        assert!(on_time.total_trucks_shift >= 1);
    }

    #[test]
    fn test_rfi_sla_tracker_overdue_escalation_and_approval() {
        let mut ledger = RfiSubmittalItem::default_ledger();
        let init = evaluate_rfi_sla_status(&ledger, 5);
        assert_eq!(init.open_count, 3);
        assert_eq!(init.overdue_count, 1);
        assert_eq!(init.lps_holds_count, 2);

        ledger[0].approved = true;
        let cleared = evaluate_rfi_sla_status(&ledger, 5);
        assert_eq!(cleared.overdue_count, 0);
        assert!(cleared.approval_pct > init.approval_pct);
    }

    #[test]
    fn test_phase45_qa_qc_lab_itp_ncr_and_mep_tc_ledger() {
        let mut lv = super::LeanView::default();
        lv.active_subtab = super::LeanSubTab::QaQcLabAndCommissioning;
        let before = lv.qa_qc.evaluate_scorecard();
        assert!(before.fc_pass_rate_pct > 0.0);
        assert!(before.open_ncr_count >= 1);

        for ncr in &mut lv.qa_qc.ncrs {
            ncr.closed = true;
        }
        let after = lv.qa_qc.evaluate_scorecard();
        assert_eq!(after.open_ncr_count, 0);
        assert!(after.itp_total_count >= after.itp_cleared_count);

        let theme = crate::theme::MacTheme::default();
        let bim = BimProject::sample_bimcraft_building();
        let mut features = super::FeatureRegistry::default();
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
                                lv.show(ui, &theme, &[], &bim, &mut features);
                            });
                            inner_max_x = resp.response.rect.right();
                        });
                    });
                });
                output.textures_delta.clear();
            }
            assert!(
                inner_max_x <= panel_max_x + 2.0,
                "Lean QA/QC overflow @ {win_w}: inner_max_x={inner_max_x:.1} > panel_max_x={panel_max_x:.1}"
            );
        }
    }
}
