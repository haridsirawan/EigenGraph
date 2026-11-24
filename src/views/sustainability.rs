//! sustainability.rs — Pillar 6: 6D BIM Sustainability & Energy.
//! Analysis of energy efficiency, carbon footprint, environmental impact, and green building certification requirements.

use egui::{
    Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, RichText,
    ScrollArea, Sense, Stroke, Ui, Vec2,
};
use crate::features::{FeatureId, FeatureRegistry};
use crate::icons::{self, AppIcon};
use crate::models::{GreenBuildingCredit, LcaStageEmission, StructuralMember};
use crate::theme::MacTheme;
use crate::ui_kit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SustainabilitySubTab {
    EmbodiedCarbon,
    CarbonEigenGateTradeoff,
    GreenCertification,
    EnergyLifecycle,
    SolarOttvDaylight,
    CircularDfdPassport,
}

pub struct SustainabilityView {
    pub active_subtab: SustainabilitySubTab,
    pub steel_production_route: SteelProductionRoute,
    pub target_rating: GreenRatingTarget,
    // Phase 28: Whole-Life LCA & Multi-Standard Green Scorecards
    pub lca_stages: Vec<LcaStageEmission>,
    pub green_credits: Vec<GreenBuildingCredit>,
    pub selected_credit_standard: Option<String>,
    pub lca_sync_status: Option<String>,
    // Phase 40 (6D-03): Live Solar OTTV & Daylight Parameters
    pub ottv_zone_idx: usize,
    pub ottv_sunshade: bool,
    pub ottv_double_low_e: bool,
    pub ottv_u_wall: f64,
    pub ottv_fallback_wwr: f32,
    // Phase 41 (6D-04): Circular Material Passport & DfD Reuse
    pub dfd_bolted_ratio_pct: f64,
    pub dfd_include_madaster_schema: bool,
    pub dfd_concrete_recycle_pct: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CircularMaterialBatch {
    pub batch_id: String,
    pub material_name: String,
    pub mass_ton: f64,
    pub joint_type: String,
    pub pathway: String,
    pub module_d_credit_tco2e: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CircularPassportSummary {
    pub batches: Vec<CircularMaterialBatch>,
    pub total_mass_ton: f64,
    pub reversible_steel_pct: f64,
    pub module_d_offset_tco2e: f64,
    pub mci_index: f64,
    pub batch_count: usize,
}

/// Computes ISO 20887 / EN 15978 Module D circular salvage credits and Ellen MacArthur Material Circularity Indicator (MCI).
pub fn compute_circular_material_passport(
    bim_project: &crate::core::BimProject,
    members: &[StructuralMember],
    steel_route: SteelProductionRoute,
    bolted_ratio_pct: f64,
    concrete_recycle_pct: f64,
) -> CircularPassportSummary {
    let bolted = bolted_ratio_pct.clamp(10.0, 100.0);
    let rec_conc = concrete_recycle_pct.clamp(0.0, 100.0);
    let rebar_ton = (bim_project.total_rebar_weight_kg() / 1000.0).max(12.5);
    let conc_ton = (bim_project.total_concrete_volume_m3() * 2.40).max(145.0);
    let primary_steel_ton = if members.is_empty() {
        (bim_project.runs.len() as f64 * 3.8 + bim_project.columns.len() as f64 * 2.1).max(38.2)
    } else {
        members
            .iter()
            .filter(|m| !m.section.contains('x') || m.section.starts_with('W') || m.section.contains("HSS"))
            .count() as f64
            * 4.2
    }
    .max(28.0);
    let bracing_steel_ton = (primary_steel_ton * 0.27).max(9.6);

    let route_credit_per_ton = match steel_route {
        SteelProductionRoute::ElectricArcFurnace => 1.45,
        SteelProductionRoute::BlastFurnaceBof => 0.85,
        SteelProductionRoute::HydrogenDri => 1.75,
    };

    let primary_credit = -(primary_steel_ton * (bolted / 100.0) * route_credit_per_ton * 0.35);
    let bracing_credit = -(bracing_steel_ton * (bolted / 100.0) * 0.31);
    let rebar_credit = -(rebar_ton * (rec_conc / 100.0) * 0.24);
    let conc_credit = -(conc_ton * (rec_conc / 100.0) * 0.008);

    let batches = vec![
        CircularMaterialBatch {
            batch_id: "MAT-ST-01".to_string(),
            material_name: "Primary Wide-Flange Frame".to_string(),
            mass_ton: primary_steel_ton,
            joint_type: if bolted >= 60.0 {
                "A325 Bolted Splice".to_string()
            } else {
                "Mixed Welded/Bolted".to_string()
            },
            pathway: format!("{:.0}% Direct Structural Reuse", bolted),
            module_d_credit_tco2e: primary_credit,
        },
        CircularMaterialBatch {
            batch_id: "MAT-ST-02".to_string(),
            material_name: "HSS Bracing & Diagonals".to_string(),
            mass_ton: bracing_steel_ton,
            joint_type: "Pinned Gusset Plate".to_string(),
            pathway: format!("{:.0}% Remanufacture / Reuse", (bolted * 0.94).min(100.0)),
            module_d_credit_tco2e: bracing_credit,
        },
        CircularMaterialBatch {
            batch_id: "MAT-RB-03".to_string(),
            material_name: "Deformed Rebar BJTS 420B".to_string(),
            mass_ton: rebar_ton,
            joint_type: "Mechanical Couplers".to_string(),
            pathway: format!("{:.0}% EAF Scrap Remelt", rec_conc),
            module_d_credit_tco2e: rebar_credit,
        },
        CircularMaterialBatch {
            batch_id: "MAT-RC-04".to_string(),
            material_name: "Substructure & Slab Concrete".to_string(),
            mass_ton: conc_ton,
            joint_type: "Cast In-Situ".to_string(),
            pathway: format!("{:.0}% Recycled RCA Aggregate", rec_conc),
            module_d_credit_tco2e: conc_credit,
        },
    ];

    let total_mass_ton = primary_steel_ton + bracing_steel_ton + rebar_ton + conc_ton;
    let module_d_offset_tco2e = primary_credit + bracing_credit + rebar_credit + conc_credit;
    let mci_index = (0.28 + 0.42 * (bolted / 100.0) + 0.22 * (rec_conc / 100.0)).clamp(0.15, 0.95);
    let batch_count = (bim_project.walls.len() + bim_project.columns.len() + bim_project.runs.len()).max(24) * 3;

    CircularPassportSummary {
        batches,
        total_mass_ton,
        reversible_steel_pct: bolted,
        module_d_offset_tco2e,
        mci_index,
        batch_count,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FacadeOttvRow {
    pub orientation: &'static str,
    pub wall_count: usize,
    pub gross_area_m2: f64,
    pub window_area_m2: f64,
    pub wwr: f64,
    pub shgc_eff: f64,
    pub solar_factor_wm2: f64,
    pub ottv_wm2: f64,
    pub pass_sni: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OttvSummary {
    pub zone_label: &'static str,
    pub facade_rows: Vec<FacadeOttvRow>,
    pub total_facade_area_m2: f64,
    pub overall_wwr: f64,
    pub overall_ottv_wm2: f64,
    pub pass_sni_35: bool,
    pub sda_pct: f64,
    pub peak_cooling_kw: f64,
    pub annual_hvac_savings_mwh: f64,
    pub modeled_windows: usize,
}

/// Evaluates SNI 6389 Overall Thermal Transfer Value (OTTV) across cardinal facades from live BIM walls & openings.
pub fn compute_facade_ottv(
    bim_project: &crate::core::BimProject,
    u_wall: f64,
    double_low_e: bool,
    sunshade: bool,
    zone_idx: usize,
    fallback_wwr: f64,
) -> OttvSummary {
    use crate::core::bim::OpeningKind;

    let (zone_label, td_ek, delta_t, sf_scale) = match zone_idx {
        1 => ("Tropical Highland (-6.9° S)", 11.2, 3.5, 0.88),
        2 => ("Equatorial Coastal (0.0°)", 15.6, 5.5, 1.06),
        _ => ("Tropical Marine (-6.2° S)", 14.5, 5.0, 1.00),
    };

    let mut counts = [0usize; 4];
    let mut gross_areas = [0.0f64; 4];
    let mut win_areas = [0.0f64; 4];
    let mut modeled_windows = 0usize;

    for wall in &bim_project.walls {
        let len_m = wall.length_mm() / 1000.0;
        if len_m < 0.1 {
            continue;
        }
        let h_m = (wall.unconnected_height_mm / 1000.0).max(2.6);
        let dx = wall.end.0 - wall.start.0;
        let dy = wall.end.1 - wall.start.1;
        // Outward normal (dy, -dx) maps directly to cardinal quadrants
        let nx = dy;
        let ny = -dx;
        let dir = if ny.abs() >= nx.abs() {
            if ny >= 0.0 { 0 } else { 2 }
        } else if nx >= 0.0 {
            1
        } else {
            3
        };
        counts[dir] += 1;
        gross_areas[dir] += len_m * h_m;

        for op in &wall.openings {
            if matches!(
                op.kind,
                OpeningKind::FixedWindow | OpeningKind::CasementWindow | OpeningKind::SlidingWindow
            ) {
                modeled_windows += 1;
                win_areas[dir] += (op.width_mm / 1000.0) * (op.height_mm / 1000.0);
            }
        }
    }

    // Fallback analytical envelope if the BIM project has no modeled walls yet
    let total_gross_existing: f64 = gross_areas.iter().sum();
    if total_gross_existing < 1.0 {
        let story_h = 3.6 * (bim_project.levels.len().max(1) as f64);
        gross_areas = [42.0 * story_h, 28.0 * story_h, 42.0 * story_h, 28.0 * story_h];
    }

    let shgc_base = if double_low_e { 0.32 } else { 0.72 };
    let shade_mult = if sunshade { 0.68 } else { 1.0 };
    let shgc_eff = shgc_base * shade_mult;
    let sc = shgc_eff / 0.87;
    let u_fen = if double_low_e { 2.8 } else { 5.6 };
    let alpha = 0.52; // Light-medium architectural facade solar absorptance (SNI 6389)
    let base_sf = [130.0, 154.0, 118.0, 211.0];
    let dir_names = ["North", "East", "South", "West"];

    let mut rows = Vec::with_capacity(4);
    let mut area_sum = 0.0f64;
    let mut win_sum = 0.0f64;
    let mut weighted_ottv = 0.0f64;

    for i in 0..4 {
        let g = gross_areas[i];
        let (w_area, wwr) = if g <= 1e-6 {
            (0.0, 0.0)
        } else if modeled_windows > 0 && win_areas[i] > 0.0 {
            let ratio = (win_areas[i] / g).clamp(0.05, 0.85);
            (win_areas[i], ratio)
        } else {
            let ratio = fallback_wwr.clamp(0.10, 0.80);
            (g * ratio, ratio)
        };
        let sf = base_sf[i] * sf_scale;
        let ottv = if g <= 1e-6 {
            0.0
        } else {
            alpha * u_wall * (1.0 - wwr) * td_ek + u_fen * wwr * delta_t + sc * wwr * sf
        };
        if g > 1e-6 {
            area_sum += g;
            win_sum += w_area;
            weighted_ottv += ottv * g;
        }
        rows.push(FacadeOttvRow {
            orientation: dir_names[i],
            wall_count: counts[i],
            gross_area_m2: g,
            window_area_m2: w_area,
            wwr,
            shgc_eff,
            solar_factor_wm2: sf,
            ottv_wm2: ottv,
            pass_sni: ottv <= 35.0,
        });
    }

    let overall_wwr = if area_sum > 1e-6 { win_sum / area_sum } else { 0.0 };
    let overall_ottv_wm2 = if area_sum > 1e-6 {
        weighted_ottv / area_sum
    } else {
        0.0
    };
    let sda_pct = (42.0
        + overall_wwr
            * 68.0
            * if double_low_e { 0.94 } else { 1.05 }
            * if sunshade { 0.95 } else { 1.0 })
        .clamp(25.0, 96.0);
    let delta_wm2 = (48.0 - overall_ottv_wm2).max(0.0);
    let peak_cooling_kw = (delta_wm2 * area_sum) / 1000.0;
    let annual_hvac_savings_mwh = peak_cooling_kw * 1.32;

    OttvSummary {
        zone_label,
        facade_rows: rows,
        total_facade_area_m2: area_sum,
        overall_wwr,
        overall_ottv_wm2,
        pass_sni_35: overall_ottv_wm2 <= 35.0,
        sda_pct,
        peak_cooling_kw,
        annual_hvac_savings_mwh,
        modeled_windows,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SteelProductionRoute {
    ElectricArcFurnace, // EAF: ~1.15 t CO2e/t (High recycled scrap)
    BlastFurnaceBof,    // BF-BOF: ~2.35 t CO2e/t (Virgin ore)
    HydrogenDri,        // H2-DRI: ~0.45 t CO2e/t (Green steel)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GreenRatingTarget {
    GreenshipPlatinum,
    GreenshipGold,
    LeedPlatinum,
    LeedGold,
}

impl Default for SustainabilityView {
    fn default() -> Self {
        Self {
            active_subtab: SustainabilitySubTab::EmbodiedCarbon,
            steel_production_route: SteelProductionRoute::ElectricArcFurnace,
            target_rating: GreenRatingTarget::GreenshipPlatinum,
            lca_stages: LcaStageEmission::default_stages(),
            green_credits: GreenBuildingCredit::default_credits(),
            selected_credit_standard: None,
            lca_sync_status: None,
            ottv_zone_idx: 0,
            ottv_sunshade: true,
            ottv_double_low_e: true,
            ottv_u_wall: 2.1,
            ottv_fallback_wwr: 0.38,
            dfd_bolted_ratio_pct: 78.0,
            dfd_include_madaster_schema: true,
            dfd_concrete_recycle_pct: 65.0,
        }
    }
}

impl SustainabilityView {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        members: &[StructuralMember],
        bim_project: &crate::core::BimProject,
        features: &mut FeatureRegistry,
    ) {
        ui.vertical(|ui| {
            Frame::new().inner_margin(Margin::symmetric(0, 8)).show(ui, |ui| {
                let full_w = ui.available_width().max(160.0);
                ui.set_width(full_w);
                ui.set_max_width(full_w);
                // 1. TOP SUBTABS SELECTOR
                self.render_header_bar(ui, theme);

                ui.add_space(16.0);

                // 2. ACTIVE SUB-TAB CONTENT
                match self.active_subtab {
                    SustainabilitySubTab::EmbodiedCarbon => {
                        self.render_embodied_carbon(ui, theme, members, bim_project, features)
                    }
                    SustainabilitySubTab::CarbonEigenGateTradeoff => {
                        self.render_carbon_eigengate_tradeoff(ui, theme)
                    }
                    SustainabilitySubTab::GreenCertification => {
                        self.render_green_certification(ui, theme)
                    }
                    SustainabilitySubTab::EnergyLifecycle => {
                        self.render_energy_lifecycle(ui, theme)
                    }
                    SustainabilitySubTab::SolarOttvDaylight => {
                        self.render_solar_ottv_daylight(ui, theme, bim_project, features);
                    }
                    SustainabilitySubTab::CircularDfdPassport => {
                        self.render_circular_dfd_passport(ui, theme, members, bim_project, features);
                    }
                }

                ui.add_space(24.0);
            });
        });
    }

    fn render_header_bar(&mut self, ui: &mut Ui, theme: &MacTheme) {
        let tabs = [
            (SustainabilitySubTab::EmbodiedCarbon, "🌱 Embodied Carbon (CO₂e)"),
            (SustainabilitySubTab::CarbonEigenGateTradeoff, "📈 Carbon vs EigenGate Trade-Off"),
            (SustainabilitySubTab::GreenCertification, "🏅 Green Certification (LEED/Greenship)"),
            (SustainabilitySubTab::EnergyLifecycle, "⚡ Thermal & Lifecycle Durability"),
            (SustainabilitySubTab::SolarOttvDaylight, "☀️ Solar OTTV & Daylight"),
            (SustainabilitySubTab::CircularDfdPassport, "♻️ Circular Passport (DfD)"),
        ];
        ui_kit::subtab_bar(ui, theme, "sust_subtab_bar", &mut self.active_subtab, &tabs);
    }

    fn render_embodied_carbon(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        members: &[StructuralMember],
        bim_project: &crate::core::BimProject,
        features: &mut FeatureRegistry,
    ) {
        let concrete_m3 = bim_project.total_concrete_volume_m3();
        let rebar_tons = bim_project.total_rebar_weight_kg() / 1000.0;
        let steel_frame_tons: f64 = members
            .iter()
            .filter(|m| m.is_active && (m.section.starts_with('W') || m.section.contains("HSS") || m.section.contains("WF")))
            .count() as f64
            * 0.42;
        let total_steel_tons = rebar_tons + steel_frame_tons;

        let route_factor = match self.steel_production_route {
            SteelProductionRoute::ElectricArcFurnace => 1.15,
            SteelProductionRoute::BlastFurnaceBof => 2.35,
            SteelProductionRoute::HydrogenDri => 0.45,
        };
        let concrete_co2 = concrete_m3 * 0.285;
        let steel_co2 = total_steel_tons * route_factor;
        let transport_co2 = (concrete_m3 * 0.012) + (total_steel_tons * 0.045);
        let baseline_co2 = concrete_co2 + (total_steel_tons * 2.35) + transport_co2;
        let total_co2 = concrete_co2 + steel_co2 + transport_co2;
        let reduction_pct = if baseline_co2 > 1e-6 {
            ((baseline_co2 - total_co2) / baseline_co2 * 100.0).max(0.0)
        } else {
            0.0
        };
        let avg_eigengate = if members.is_empty() {
            0.0
        } else {
            members.iter().map(|m| if m.eigengate > 0.0 { m.eigengate } else { m.eigengate_score }).sum::<f64>()
                / members.len() as f64
        };
        let eco_eff = if total_co2 > 1e-6 { avg_eigengate / total_co2 } else { 0.0 };
        let floor_area_m2: f64 = bim_project
            .slabs
            .iter()
            .filter(|s| !s.is_roof && s.boundary.len() >= 3)
            .map(|s| {
                let mut sum = 0.0;
                for i in 0..s.boundary.len() {
                    let a = s.boundary[i];
                    let b = s.boundary[(i + 1) % s.boundary.len()];
                    sum += a.0 * b.1 - b.0 * a.1;
                }
                sum.abs() * 0.5 * 1e-6
            })
            .sum::<f64>()
            .max(1.0);

        // Responsive Metric Cards Row
        let cards = [
            (
                "TOTAL EMBODIED CARBON",
                format!("{:.2} t CO₂e", total_co2),
                format!("Concrete: {:.1} m³ | Steel: {:.2} t (A1–A5)", concrete_m3, total_steel_tons),
                theme.accent_green,
            ),
            (
                "CARBON INTENSITY",
                format!("{:.1} kg/m²", (total_co2 * 1000.0) / floor_area_m2),
                format!("Area Basis: {:.1} m²", floor_area_m2),
                theme.accent_blue,
            ),
            (
                "EMISSIONS SAVINGS",
                format!("-{:.1}%", reduction_pct),
                "vs. Baseline BF-BOF".to_string(),
                if reduction_pct > 25.0 {
                    theme.accent_green
                } else {
                    theme.accent_orange
                },
            ),
            (
                "ECO-REDUNDANCY (η)",
                format!("{:.4}", eco_eff),
                "EigenGate / Total tCO₂e".to_string(),
                theme.accent_blue,
            ),
        ];
        ui_kit::responsive_grid_n(ui, cards.len(), 200.0, |ui, idx| {
            let (title, val, sub, col) = &cards[idx];
            self.render_stat_card(ui, theme, title, val, sub, *col);
        });

        ui.add_space(12.0);

        // Live BIM Material Embodied Carbon Sync
        ui_kit::feature_slot_card(ui, theme, features, FeatureId::BimMaterialLcaAutoSync, |ui| {
            ui.vertical(|ui| {
                let slot_w = ui.available_width();
                ui.set_max_width(slot_w);
                ui.add(
                    egui::Label::new(
                        RichText::new(format!(
                            "Active BimProject LCA: {:.2} m³ Concrete ({:.2} tCO₂e) + {:.2} t Steel ({:.2} tCO₂e)",
                            concrete_m3, concrete_co2, total_steel_tons, steel_co2,
                        ))
                        .size(11.0)
                        .strong()
                        .color(theme.text_primary),
                    )
                    .wrap(),
                );
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if icons::icon_button(ui, AppIcon::Recycle, "Recompute Live Model CO₂e Footprint", 11.5, Color32::WHITE, theme.accent_green, Stroke::NONE, CornerRadius::same(6), Vec2::ZERO).clicked() {
                        self.lca_sync_status = Some(format!(
                            "Computed A1–A5 Embodied Carbon from {} BIM elements: {:.2} tCO₂e (Concrete {:.2} t + Steel {:.2} t + Transport {:.2} t).",
                            bim_project.element_count(),
                            total_co2,
                            concrete_co2,
                            steel_co2,
                            transport_co2,
                        ));
                    }
                });
                if let Some(ref st) = self.lca_sync_status {
                    ui.add_space(4.0);
                    ui.add(egui::Label::new(RichText::new(st).size(10.5).color(theme.accent_green)).wrap());
                }
            });
        });

        ui.add_space(14.0);

        // Production Route Selector
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(14))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                let card_w = ui.available_width();
                ui.set_max_width(card_w);
                ui.horizontal_wrapped(|ui| {
                    icons::icon_label(ui, AppIcon::Building, 13.5, theme.accent_blue, "Steel Production Route:", 12.0, theme.text_primary, true);
                    for (route, icon, lbl) in [
                        (SteelProductionRoute::ElectricArcFurnace, AppIcon::Recycle, "EAF Electric Arc (1.15 t CO₂e/t)"),
                        (SteelProductionRoute::BlastFurnaceBof, AppIcon::Wrench, "Conventional BF-BOF (2.35 t CO₂e/t)"),
                        (SteelProductionRoute::HydrogenDri, AppIcon::Leaf, "Green Hydrogen DRI (0.45 t CO₂e/t)"),
                    ] {
                        let is_sel = self.steel_production_route == route;
                        let bg = if is_sel { theme.accent_blue.gamma_multiply(0.18) } else { theme.bg_canvas };
                        let fg = if is_sel { theme.accent_blue } else { theme.text_secondary };
                        let stroke = if is_sel { Stroke::new(1.0, theme.accent_blue) } else { Stroke::new(1.0, theme.border_subtle) };
                        if icons::icon_button(ui, icon, lbl, 11.5, fg, bg, stroke, CornerRadius::same(6), Vec2::ZERO).clicked() {
                            self.steel_production_route = route;
                        }
                    }
                });
            });

        ui.add_space(16.0);

        // ISO 14040 / EN 15978 Whole-Life Carbon Assessment (WBLCA) Stages
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(14))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                let card_inner_w = ui.available_width();
                ui.set_max_width(card_inner_w);
                if card_inner_w >= 680.0 {
                    ui.horizontal(|ui| {
                        icons::icon_label(ui, AppIcon::Globe, 14.5, theme.accent_green, "WHOLE BUILDING LIFE CYCLE ASSESSMENT (EN 15978 / ISO 14040)", 13.0, theme.text_primary, true);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new("RIBA 2030 Climate Challenge Benchmark: Tier A (< 300 kg CO₂e/m²)")
                                    .size(11.0)
                                    .color(theme.accent_green),
                            );
                        });
                    });
                } else {
                    icons::icon_label(ui, AppIcon::Globe, 14.5, theme.accent_green, "WHOLE BUILDING LCA (EN 15978)", 13.0, theme.text_primary, true);
                    ui.add(
                        egui::Label::new(
                            RichText::new("ISO 14040 Lifecycle Assessment • RIBA 2030 Benchmark: Tier A (< 300 kg CO₂e/m²)")
                                .size(11.0)
                                .color(theme.accent_green),
                        )
                        .wrap(),
                    );
                }
                ui.add_space(10.0);

                for stage in &self.lca_stages {
                    let col = Color32::from_rgb(stage.color_rgb.0, stage.color_rgb.1, stage.color_rgb.2);
                    let sign = if stage.emissions_t_co2e < 0.0 { "" } else { "+" };
                    if card_inner_w >= 540.0 {
                        ui.horizontal(|ui| {
                            Frame::new()
                                .fill(col.gamma_multiply(0.2))
                                .stroke(Stroke::new(1.0, col))
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(Margin::symmetric(6, 2))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(&stage.stage_code).size(11.0).strong().color(col));
                                });

                            ui.add_space(8.0);
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(&stage.stage_name)
                                        .size(11.5)
                                        .strong()
                                        .color(theme.text_primary),
                                );
                                ui.label(
                                    RichText::new(&stage.description)
                                        .size(10.5)
                                        .color(theme.text_secondary),
                                );
                            });

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format!("{:.1}%", stage.pct_of_total))
                                        .size(11.5)
                                        .strong()
                                        .color(col),
                                );
                                ui.add_space(12.0);
                                ui.label(
                                    RichText::new(format!("{}{:.1} t CO₂e", sign, stage.emissions_t_co2e))
                                        .size(12.0)
                                        .color(theme.text_primary),
                                );
                            });
                        });
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            Frame::new()
                                .fill(col.gamma_multiply(0.2))
                                .stroke(Stroke::new(1.0, col))
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(Margin::symmetric(6, 2))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(&stage.stage_code).size(11.0).strong().color(col));
                                });
                            ui.label(
                                RichText::new(&stage.stage_name)
                                    .size(11.5)
                                    .strong()
                                    .color(theme.text_primary),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "{}{:.1} t CO₂e ({:.1}%)",
                                    sign, stage.emissions_t_co2e, stage.pct_of_total
                                ))
                                .size(11.0)
                                .strong()
                                .color(col),
                            );
                        });
                        ui.add(
                            egui::Label::new(
                                RichText::new(&stage.description)
                                    .size(10.5)
                                    .color(theme.text_secondary),
                            )
                            .wrap(),
                        );
                    }

                    ui.add_space(2.0);
                    let (b_rect, _resp) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 4.0),
                        egui::Sense::hover(),
                    );
                    let b_painter = ui.painter();
                    b_painter.rect_filled(b_rect, CornerRadius::same(2), theme.bg_canvas);
                    let fill_w =
                        b_rect.width() * (stage.pct_of_total.abs() / 100.0).clamp(0.02, 1.0);
                    b_painter.rect_filled(
                        egui::Rect::from_min_size(b_rect.left_top(), Vec2::new(fill_w, b_rect.height())),
                        CornerRadius::same(2),
                        col,
                    );

                    ui.add_space(6.0);
                }
            });
    }

    fn render_carbon_eigengate_tradeoff(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                icons::icon_label(ui, AppIcon::ChartLine, 14.5, theme.accent_blue, "Pareto Frontier: Structural Redundancy (EigenGate) vs. Embodied Carbon (CO₂e)", 13.5, theme.text_primary, true);
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Finding the optimum trade-off where progressive collapse resilience increases significantly without disproportionate carbon expenditure.")
                        .size(11.5)
                        .color(theme.text_secondary),
                );
                ui.add_space(14.0);

                let scenarios = [
                    ("Vulnerable Design (Non-Redundant)", "0.58", "38.2 Tons", "43.9 t CO₂e", AppIcon::AlertTriangle, theme.accent_red, "Catastrophic Collapse Risk (Reconstruction = +120 t CO₂e)", false),
                    ("Code-Baseline Standard Design", "0.72", "44.0 Tons", "50.6 t CO₂e", AppIcon::Info, theme.accent_orange, "Meets Minimum Code, Lacks Secondary Redundancy", false),
                    ("EigenGraph Optimal Design (Recommended)", "0.94", "48.5 Tons", "55.8 t CO₂e", AppIcon::CheckCircle, theme.accent_green, "SWEET SPOT: EigenGate +30.5%, Carbon Only +5.2 t CO₂e (+10.2%)", true),
                    ("Over-Conservative Design", "0.98", "62.0 Tons", "71.3 t CO₂e", AppIcon::Minus, theme.text_muted, "Inefficient: +27.8% Carbon Surcharge for Negligible EigenGate Gain", false),
                ];

                for (name, eigengate, steel, co2, desc_icon, desc_col, desc, is_opt) in scenarios {
                    let fill = if is_opt {
                        theme.accent_blue.gamma_multiply(0.12)
                    } else {
                        theme.bg_canvas
                    };
                    let stroke = if is_opt {
                        Stroke::new(1.5, theme.accent_blue)
                    } else {
                        Stroke::new(1.0, theme.border_subtle)
                    };

                    Frame::new()
                        .fill(fill)
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(12))
                        .stroke(stroke)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.vertical(|ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(RichText::new(name).size(12.5).strong().color(if is_opt { theme.accent_blue } else { theme.text_primary }));
                                        if is_opt {
                                            icons::pill_badge(ui, AppIcon::Star, "BEST ECO-EFFICIENCY", theme.accent_green);
                                        }
                                    });
                                    ui.add_space(2.0);
                                    icons::icon_label(ui, desc_icon, 12.0, desc_col, desc, 11.0, theme.text_secondary, false);
                                });

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new(format!("EigenGate: {}", eigengate)).size(13.0).strong().color(if is_opt { theme.accent_green } else { theme.text_primary }));
                                        ui.label(RichText::new(format!("{} • {}", steel, co2)).size(11.0).color(theme.text_muted));
                                    });
                                });
                            });
                        });
                    ui.add_space(8.0);
                }
            });
    }

    fn render_green_certification(&mut self, ui: &mut Ui, theme: &MacTheme) {
        let cert_cards = [
            ("GREENSHIP (GBCI)", "PLATINUM", "86 / 100 Credit Points", theme.accent_green),
            ("LEED v4.1 BD+C", "PLATINUM", "84 / 110 Credit Points", theme.accent_blue),
            ("BREEAM INTERNATIONAL", "OUTSTANDING", "88.4% Rating Score", theme.accent_green),
        ];
        ui_kit::responsive_grid_n(ui, cert_cards.len(), 200.0, |ui, idx| {
            let (title, val, sub, col) = cert_cards[idx];
            self.render_stat_card(ui, theme, title, val, sub, col);
        });

        ui.add_space(14.0);

        // Standard Filter Pills
        theme.card_frame().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                icons::icon_label(ui, AppIcon::Star, 13.0, theme.accent_blue, "RATING SYSTEM:", 11.5, theme.accent_blue, true);

                let is_all = self.selected_credit_standard.is_none();
                let all_bg = if is_all { theme.accent_blue } else { theme.bg_canvas };
                let all_fg = if is_all { Color32::WHITE } else { theme.text_secondary };
                if ui
                    .add(
                        egui::Button::new(RichText::new("All Systems").size(11.0).color(all_fg))
                            .fill(all_bg)
                            .corner_radius(CornerRadius::same(6)),
                    )
                    .clicked()
                {
                    self.selected_credit_standard = None;
                }

                let standards = ["GREENSHIP (GBCI)", "LEED v4.1 BD+C", "BREEAM International"];
                for std in standards {
                    let is_sel = self.selected_credit_standard.as_deref() == Some(std);
                    let bg = if is_sel { theme.accent_blue } else { theme.bg_canvas };
                    let fg = if is_sel { Color32::WHITE } else { theme.text_secondary };
                    if ui
                        .add(
                            egui::Button::new(RichText::new(std).size(11.0).color(fg))
                                .fill(bg)
                                .corner_radius(CornerRadius::same(6)),
                        )
                        .clicked()
                    {
                        self.selected_credit_standard = Some(std.to_string());
                    }
                }
            });
        });

        ui.add_space(12.0);

        // Credit Checklist Table
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                icons::icon_label(ui, AppIcon::Clipboard, 14.5, theme.accent_blue, "Green Certification Verification Checklist", 13.0, theme.text_primary, true);
                ui.add_space(10.0);

                for credit in &self.green_credits {
                    if let Some(ref sel_std) = self.selected_credit_standard {
                        if &credit.standard != sel_std {
                            continue;
                        }
                    }

                    ui.horizontal_wrapped(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(&credit.credit_title)
                                    .size(12.0)
                                    .strong()
                                    .color(theme.text_primary),
                            );
                            ui.label(
                                RichText::new(format!("{} • {}", credit.standard, credit.category))
                                    .size(10.5)
                                    .color(theme.text_muted),
                            );
                        });

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let (badge_bg, badge_fg) = if credit.status == "Compliant" {
                                (theme.accent_green.gamma_multiply(0.2), theme.accent_green)
                            } else {
                                (theme.accent_orange.gamma_multiply(0.2), theme.accent_orange)
                            };

                            Frame::new()
                                .fill(badge_bg)
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(Margin::symmetric(6, 2))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(&credit.status)
                                            .size(10.5)
                                            .strong()
                                            .color(badge_fg),
                                    );
                                });

                            ui.add_space(12.0);
                            ui.label(
                                RichText::new(format!(
                                    "{}/{} Pts",
                                    credit.points_achieved, credit.points_max
                                ))
                                .size(12.0)
                                .strong()
                                .color(theme.accent_blue),
                            );
                        });
                    });
                    ui.separator();
                }
            });
    }

    fn render_energy_lifecycle(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                icons::icon_label(ui, AppIcon::Bolt, 14.5, theme.accent_orange, "Thermal Efficiency & Life Cycle Durability Assessment", 13.5, theme.text_primary, true);
                ui.add_space(10.0);

                let energy_cards = [
                    ("DESIGN SERVICE LIFE", "100 Years", "SNI Bridge Serviceability Class A", theme.text_primary),
                    ("COATING DURABILITY", "25+ Years", "C5-High Marine Grade Polyurethane", theme.accent_blue),
                    ("THERMAL EXPANSION", "±14.2 mm", "Elastomeric Bearing Expansion Allowance", theme.accent_green),
                ];
                ui_kit::responsive_grid_n(ui, energy_cards.len(), 200.0, |ui, idx| {
                    let (title, val, sub, col) = energy_cards[idx];
                    self.render_stat_card(ui, theme, title, val, sub, col);
                });
            });
    }

    fn render_solar_ottv_daylight(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        bim_project: &crate::core::BimProject,
        features: &mut FeatureRegistry,
    ) {
        let summary = compute_facade_ottv(
            bim_project,
            self.ottv_u_wall,
            self.ottv_double_low_e,
            self.ottv_sunshade,
            self.ottv_zone_idx,
            self.ottv_fallback_wwr as f64,
        );
        ui_kit::feature_slot_card(ui, theme, features, FeatureId::SolarOttvThermalDaylight, |ui| {
            let ottv_val = format!(
                "{:.1} W/m² ({})",
                summary.overall_ottv_wm2,
                if summary.pass_sni_35 { "PASS" } else { "EXCEED" }
            );
            let ottv_col = if summary.pass_sni_35 {
                theme.accent_green
            } else {
                theme.accent_red
            };
            let shgc_title = format!(
                "{:.2} {}",
                if self.ottv_double_low_e { 0.32 } else { 0.72 }
                    * if self.ottv_sunshade { 0.74 } else { 1.0 },
                if self.ottv_double_low_e { "Double Low-E" } else { "Single Clear" }
            );
            let shgc_sub = format!(
                "WWR = {:.1}% ({} BIM win • {:.0} m²)",
                summary.overall_wwr * 100.0,
                summary.modeled_windows,
                summary.total_facade_area_m2
            );
            let sda_val = format!("{:.1}% Floor Area", summary.sda_pct);
            let sda_col = if summary.sda_pct >= 55.0 {
                theme.accent_green
            } else {
                theme.accent_orange
            };
            let cool_val = format!("-{:.1} kW Peak Chiller", summary.peak_cooling_kw);
            let cool_sub = format!(
                "Annual HVAC Savings ≈ {:.0} MWh/yr",
                summary.annual_hvac_savings_mwh
            );
            let cards = [
                ("OVERALL OTTV (SNI 6389)", ottv_val.as_str(), "Max Allowed Cap = 35.0 W/m²", ottv_col),
                ("EFFECTIVE SHGC & WWR", shgc_title.as_str(), shgc_sub.as_str(), theme.accent_blue),
                ("DAYLIGHT AUTONOMY (sDA)", sda_val.as_str(), "Target >= 55% @ 300 lux Occupied", sda_col),
                ("COOLING LOAD REDUCTION", cool_val.as_str(), cool_sub.as_str(), theme.accent_orange),
            ];
            ui_kit::responsive_grid_n(ui, cards.len(), 185.0, |ui, idx| {
                let (t, v, s, c) = cards[idx];
                self.render_stat_card(ui, theme, t, v, s, c);
            });

            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                for (z_idx, label) in [
                    (0, "Tropical Marine (-6.2° S)"),
                    (1, "Highland (-6.9° S)"),
                    (2, "Equatorial Coastal (0.0°)"),
                ] {
                    if ui.selectable_label(self.ottv_zone_idx == z_idx, label).clicked() {
                        self.ottv_zone_idx = z_idx;
                    }
                }
                ui.checkbox(&mut self.ottv_sunshade, "600mm Horizontal Sunshades");
                ui.checkbox(&mut self.ottv_double_low_e, "Double Low-E Glazing");
            });
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Wall U-Value (W/m²·K):").size(11.0).color(theme.text_secondary));
                ui.add(egui::Slider::new(&mut self.ottv_u_wall, 0.6..=3.2).step_by(0.1));
                ui.label(RichText::new("Baseline WWR:").size(11.0).color(theme.text_secondary));
                ui.add(egui::Slider::new(&mut self.ottv_fallback_wwr, 0.15..=0.75).step_by(0.01));

                if icons::icon_button(
                    ui,
                    AppIcon::Sun,
                    "Export N/E/S/W OTTV Audit (.CSV)",
                    11.5,
                    theme.text_primary,
                    theme.bg_canvas,
                    Stroke::new(1.0, theme.border_subtle),
                    CornerRadius::same(6),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    let mut csv = String::from("Orientation,Walls,Gross_m2,Window_m2,WWR_pct,SHGC_eff,SF_Wm2,OTTV_Wm2,SNI_6389\n");
                    for r in &summary.facade_rows {
                        csv.push_str(&format!(
                            "{},{},{:.1},{:.1},{:.1},{:.2},{:.0},{:.2},{}\n",
                            r.orientation,
                            r.wall_count,
                            r.gross_area_m2,
                            r.window_area_m2,
                            r.wwr * 100.0,
                            r.shgc_eff,
                            r.solar_factor_wm2,
                            r.ottv_wm2,
                            if r.pass_sni { "PASS" } else { "FAIL" }
                        ));
                    }
                    self.lca_sync_status = Some(
                        match crate::core::web_bridge::save_text_file("sni6389_ottv_audit.csv", &csv, "text/csv") {
                            Ok(msg) => format!(
                                "SNI 6389 OTTV ({}): {:.1} W/m² • {}",
                                summary.zone_label, summary.overall_ottv_wm2, msg
                            ),
                            Err(e) => format!("OTTV CSV export failed: {e}"),
                        },
                    );
                }
            });
            if let Some(ref st) = self.lca_sync_status {
                ui.add_space(6.0);
                ui.label(RichText::new(st).size(11.0).color(theme.accent_blue));
            }
            ui.add_space(12.0);

            ui_kit::responsive_two_panes(ui, 640.0, |ui, pane| match pane {
                0 => {
                    ui.label(
                        RichText::new("FACADE ORIENTATION SOLAR FLUX COMPASS (W/m²)")
                            .size(11.0)
                            .strong()
                            .color(theme.accent_blue),
                    );
                    ui.add_space(6.0);
                    let desired = Vec2::new(ui.available_width(), 165.0);
                    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
                    let painter = ui.painter_at(rect);
                    painter.rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(13, 17, 24));
                    let c = rect.center();
                    let r_max = (rect.width().min(rect.height()) * 0.36).max(28.0);
                    painter.circle_stroke(c, r_max, Stroke::new(1.0, theme.border_subtle));
                    painter.circle_stroke(c, r_max * 0.65, Stroke::new(0.8, theme.accent_green.gamma_multiply(0.5)));

                    let vecs = [(0.0_f32, -1.0_f32), (1.0, 0.0), (0.0, 1.0), (-1.0, 0.0)];
                    let mut poly = Vec::with_capacity(4);
                    let mut compass_labels = Vec::with_capacity(4);
                    for (idx, row) in summary.facade_rows.iter().enumerate() {
                        let (dx, dy) = vecs[idx];
                        let frac = ((row.ottv_wm2 / 42.0) as f32).clamp(0.18, 1.0);
                        let col = if !row.pass_sni {
                            theme.accent_red
                        } else if row.ottv_wm2 > 31.5 {
                            theme.accent_orange
                        } else {
                            theme.accent_green
                        };
                        poly.push(c + Vec2::new(dx * r_max * frac, dy * r_max * frac));
                        let short = &row.orientation[..1];
                        compass_labels.push((format!("{}: {:.1}", short, row.ottv_wm2), dx, dy, col));
                    }
                    for i in 0..poly.len() {
                        painter.line_segment(
                            [poly[i], poly[(i + 1) % poly.len()]],
                            Stroke::new(2.0, theme.accent_orange),
                        );
                    }
                    for (lbl, dx, dy, col) in compass_labels {
                        painter.text(
                            c + Vec2::new(dx * (r_max + 24.0), dy * (r_max + 12.0)),
                            Align2::CENTER_CENTER,
                            lbl,
                            egui::FontId::monospace(9.5),
                            col,
                        );
                    }
                }
                _ => {
                    ui.label(
                        RichText::new("FACADE ORIENTATION THERMAL BREAKDOWN")
                            .size(11.0)
                            .strong()
                            .color(theme.accent_blue),
                    );
                    ui.add_space(6.0);
                    ScrollArea::horizontal().id_salt("ottv_facade_scroll").show(ui, |ui| {
                        egui::Grid::new("ottv_facade_grid")
                            .striped(true)
                            .spacing(Vec2::new(12.0, 5.0))
                            .show(ui, |ui| {
                                for h in ["Orientation", "BIM Walls", "Gross (m²)", "WWR", "SHGC eff", "OTTV (W/m²)", "SNI 6389"] {
                                    ui.label(RichText::new(h).size(10.5).strong().color(theme.accent_blue));
                                }
                                ui.end_row();
                                for row in &summary.facade_rows {
                                    let col = if !row.pass_sni {
                                        theme.accent_red
                                    } else if row.ottv_wm2 > 31.5 {
                                        theme.accent_orange
                                    } else {
                                        theme.accent_green
                                    };
                                    let st = if !row.pass_sni {
                                        "EXCEED (>35)"
                                    } else if row.ottv_wm2 > 31.5 {
                                        "PASS (Tight)"
                                    } else {
                                        "PASS"
                                    };
                                    ui.label(RichText::new(row.orientation).size(10.5).strong().color(theme.text_primary));
                                    ui.label(RichText::new(format!("{}", row.wall_count)).size(10.5).monospace().color(theme.text_secondary));
                                    ui.label(RichText::new(format!("{:.1}", row.gross_area_m2)).size(10.5).monospace().color(theme.text_secondary));
                                    ui.label(RichText::new(format!("{:.0}%", row.wwr * 100.0)).size(10.5).color(theme.accent_blue));
                                    ui.label(RichText::new(format!("{:.2}", row.shgc_eff)).size(10.5).color(theme.text_secondary));
                                    ui.label(RichText::new(format!("{:.1}", row.ottv_wm2)).size(10.5).strong().color(col));
                                    ui.label(RichText::new(st).size(10.5).color(col));
                                    ui.end_row();
                                }
                            });
                    });
                }
            });
        });
    }

    fn render_circular_dfd_passport(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        members: &[StructuralMember],
        bim_project: &crate::core::BimProject,
        features: &mut FeatureRegistry,
    ) {
        let rep = compute_circular_material_passport(
            bim_project,
            members,
            self.steel_production_route,
            self.dfd_bolted_ratio_pct,
            self.dfd_concrete_recycle_pct,
        );
        let mci_col = if rep.mci_index >= 0.60 {
            theme.accent_green
        } else {
            theme.accent_orange
        };
        ui_kit::feature_slot_card(ui, theme, features, FeatureId::CircularMaterialPassportDfd, |ui| {
            let cards = [
                (
                    "REVERSIBLE BOLTED STEEL",
                    format!("{:.1}% by Mass", rep.reversible_steel_pct),
                    "ISO 20887 Bolted End-Plates & Pinned Gussets".to_string(),
                    if rep.reversible_steel_pct >= 65.0 {
                        theme.accent_green
                    } else {
                        theme.accent_orange
                    },
                ),
                (
                    "EN 15978 MODULE D OFFSET",
                    format!("{:.1} t CO₂e", rep.module_d_offset_tco2e),
                    "Circular Salvage & Recycling Carbon Credit".to_string(),
                    theme.accent_blue,
                ),
                (
                    "MATERIAL CIRCULARITY (MCI)",
                    format!("{:.2} Index", rep.mci_index),
                    "Target >= 0.60 Urban Mining Certification".to_string(),
                    mci_col,
                ),
                (
                    "PASSPORT INVENTORY TAGS",
                    format!("{} Tagged Batches", rep.batch_count),
                    format!("Total Tracked Mass = {:.1} T", rep.total_mass_ton),
                    theme.text_primary,
                ),
            ];
            ui_kit::responsive_grid_n(ui, cards.len(), 185.0, |ui, idx| {
                let (t, v, s, c) = &cards[idx];
                self.render_stat_card(ui, theme, t, v, s, *c);
            });

            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Bolted Reversible Steel:").size(11.0).color(theme.text_secondary));
                ui.add(egui::Slider::new(&mut self.dfd_bolted_ratio_pct, 15.0..=100.0).suffix("%"));
                ui.add_space(6.0);
                ui.label(RichText::new("RCA Concrete / Rebar Recovery:").size(11.0).color(theme.text_secondary));
                ui.add(egui::Slider::new(&mut self.dfd_concrete_recycle_pct, 10.0..=95.0).suffix("%"));
                ui.checkbox(&mut self.dfd_include_madaster_schema, "EU Madaster / ISO 20887 Schema");
                if icons::icon_button(
                    ui,
                    AppIcon::Recycle,
                    "Export Material Passport (.json)",
                    11.5,
                    theme.text_primary,
                    theme.bg_canvas,
                    Stroke::new(1.0, theme.border_subtle),
                    CornerRadius::same(6),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    let rows_json: Vec<String> = rep
                        .batches
                        .iter()
                        .map(|b| {
                            format!(
                                "    {{\"batch_id\":\"{}\",\"material\":\"{}\",\"mass_ton\":{:.2},\"joint\":\"{}\",\"pathway\":\"{}\",\"module_d_tco2e\":{:.2}}}",
                                b.batch_id, b.material_name, b.mass_ton, b.joint_type, b.pathway, b.module_d_credit_tco2e
                            )
                        })
                        .collect();
                    let json = format!(
                        "{{\n  \"schema\": \"{}\",\n  \"project\": \"{}\",\n  \"reversible_steel_pct\": {:.1},\n  \"module_d_offset_tco2e\": {:.2},\n  \"mci_index\": {:.3},\n  \"total_mass_ton\": {:.2},\n  \"batches\": [\n{}\n  ]\n}}\n",
                        if self.dfd_include_madaster_schema { "ISO-20887-Madaster-v2" } else { "EigenGraph-DfD-v1" },
                        bim_project.project_name.replace('"', "'"),
                        rep.reversible_steel_pct,
                        rep.module_d_offset_tco2e,
                        rep.mci_index,
                        rep.total_mass_ton,
                        rows_json.join(",\n"),
                    );
                    match crate::core::web_bridge::save_text_file("EigenGraph_Circular_Material_Passport.json", &json, "application/json") {
                        Ok(msg) => self.lca_sync_status = Some(format!("Material Passport JSON exported ({msg})")),
                        Err(err) => self.lca_sync_status = Some(format!("Passport JSON error: {err}")),
                    }
                }
            });
            ui.add_space(10.0);

            ScrollArea::horizontal().id_salt("dfd_passport_scroll").show(ui, |ui| {
                egui::Grid::new("dfd_passport_grid")
                    .striped(true)
                    .min_col_width(90.0)
                    .spacing(Vec2::new(14.0, 6.0))
                    .show(ui, |ui| {
                        for h in ["Batch ID", "Structural Material", "Mass (Ton)", "Joint Type", "Reuse Pathway", "Module D Credit"] {
                            ui.label(RichText::new(h).size(10.5).strong().color(theme.accent_blue));
                        }
                        ui.end_row();
                        for b in &rep.batches {
                            ui.label(RichText::new(&b.batch_id).size(10.5).monospace().color(theme.accent_blue));
                            ui.label(RichText::new(&b.material_name).size(10.5).strong().color(theme.text_primary));
                            ui.label(RichText::new(format!("{:.1} T", b.mass_ton)).size(10.5).color(theme.text_secondary));
                            ui.label(RichText::new(&b.joint_type).size(10.5).color(theme.accent_green));
                            ui.label(RichText::new(&b.pathway).size(10.5).color(theme.text_secondary));
                            ui.label(RichText::new(format!("{:.1} t CO₂e", b.module_d_credit_tco2e)).size(10.5).strong().color(theme.accent_green));
                            ui.end_row();
                        }
                    });
            });
        });
    }

    fn render_stat_card(
        &self,
        ui: &mut Ui,
        theme: &MacTheme,
        title: &str,
        val: &str,
        subtitle: &str,
        val_color: Color32,
    ) {
        ui_kit::stat_card(ui, theme, title, val, subtitle, val_color);
    }
}

#[cfg(test)]
mod tests {
    use super::{compute_circular_material_passport, compute_facade_ottv, SteelProductionRoute};
    use crate::core::BimProject;

    #[test]
    fn test_solar_ottv_sunshade_and_low_e_reduce_transfer() {
        let proj = BimProject::default();
        let high_solar = compute_facade_ottv(&proj, 2.6, false, false, 0, 0.45);
        let low_e_shaded = compute_facade_ottv(&proj, 2.1, true, true, 0, 0.45);

        assert_eq!(low_e_shaded.facade_rows.len(), 4);
        assert!(low_e_shaded.overall_ottv_wm2 < high_solar.overall_ottv_wm2);
        assert!(low_e_shaded.pass_sni_35);
        assert!(low_e_shaded.peak_cooling_kw > 0.0);
    }

    #[test]
    fn test_circular_dfd_passport_bolted_ratio_improves_mci() {
        let proj = BimProject::sample_bimcraft_building();
        let welded = compute_circular_material_passport(
            &proj,
            &[],
            SteelProductionRoute::BlastFurnaceBof,
            25.0,
            20.0,
        );
        let bolted_eaf = compute_circular_material_passport(
            &proj,
            &[],
            SteelProductionRoute::ElectricArcFurnace,
            88.0,
            75.0,
        );
        assert!(bolted_eaf.mci_index > welded.mci_index);
        assert!(bolted_eaf.module_d_offset_tco2e < welded.module_d_offset_tco2e);
        assert_eq!(bolted_eaf.batches.len(), 4);
    }
}
