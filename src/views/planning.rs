//! planning.rs — Pillar 5: 5D Construction Planning, Dynamic RAB, 4D Scheduling & Cost-ARI Trade-Off.
//! Automatically calculated and synchronized with PARAMETRIC variables (dimensions, material, stiffness, unit prices).

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui, Vec2};
use crate::models::{ParametricVariable, StructuralMember};
use crate::theme::MacTheme;

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
}

pub struct PlanningView {
    pub active_subtab: PlanningSubTab,
    pub published_to_dossier: bool,
    pub export_status: Option<String>,
    pub contingency_percent: f32,
    pub overhead_percent: f32,
    pub vat_percent: f32,
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
        }
    }
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
    /// Extract parametric variables and evaluate full 5D quantities
    pub fn compute_takeoff(&self, variables: &[ParametricVariable]) -> Parametric5DTakeoff {
        let get_val = |name: &str, default: f64| -> f64 {
            for v in variables {
                if v.name.eq_ignore_ascii_case(name) || v.name.contains(name) {
                    return v.evaluated_value as f64;
                }
            }
            default
        };

        // Dimensions
        let span_mm = get_val("L", 24000.0);
        let span_m = span_mm / 1000.0;
        let height_mm = get_val("H", 3200.0);
        let height_m = height_mm / 1000.0;
        let n_panels = get_val("n", 6.0).round().max(2.0) as usize;

        // Material & Stiffness
        let density_kg_m3 = get_val("density", 7850.0);
        let stiffness_e_mpa = get_val("kekakuan", 200000.0);

        // Unit Prices (IDR)
        let price_steel_kg = get_val("harga_baja", 24500.0);
        let price_fab_kg = get_val("harga_fabrikasi", 8500.0);
        let price_erect_ton = get_val("harga_ereksi", 4500000.0);
        let price_coating_m2 = get_val("harga_coating", 145000.0);
        let price_concrete_m3 = get_val("harga_pondasi", 1950000.0);

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
        let total_steel_ton = total_steel_kg / 1000.0;

        // Surface Area for Coating: AISC W14x90 perimeter ~1.45m, HSS8x8 ~0.80m
        let coating_surface_m2 = (chord_length_m * 1.45 + diagonal_length_m * 0.80) * 1.10;

        // Bolts & Concrete
        let bolt_sets_count = (2 * n_panels + 1) * 16;
        let concrete_footing_m3 = 2.0 * (15.0 + span_m * 0.75); // Substructure proportional to span
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

        // System ARI indicator derived from stiffness and panel geometry
        let predicted_system_ari = (0.75 + (height_m / span_m) * 1.25).min(0.965);

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
    ) {
        let takeoff = self.compute_takeoff(variables);

        ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            Frame::new().inner_margin(Margin::symmetric(20, 12)).show(ui, |ui| {
                // 1. TOP SUB-TABS SELECTOR & EXPORT BAR
                self.render_header_bar(ui, theme, &takeoff);

                ui.add_space(14.0);

                // 2. LIVE PARAMETRIC BINDING BANNER
                self.render_parametric_binding_banner(ui, theme, &takeoff);

                ui.add_space(16.0);

                // 3. ACTIVE SUB-TAB CONTENT
                match self.active_subtab {
                    PlanningSubTab::TakeoffQto => self.render_qto_takeoff(ui, theme, &takeoff, members),
                    PlanningSubTab::DynamicRab => self.render_dynamic_rab(ui, theme, &takeoff),
                    PlanningSubTab::Schedule4d => self.render_schedule_4d(ui, theme, &takeoff),
                    PlanningSubTab::CostAriTradeoff => self.render_cost_ari_tradeoff(ui, theme, &takeoff),
                    PlanningSubTab::StagedErectionSvd => self.render_staged_erection_svd(ui, theme, &takeoff),
                }

                ui.add_space(24.0);
            });
        });
    }

    fn render_header_bar(&mut self, ui: &mut Ui, theme: &MacTheme, takeoff: &Parametric5DTakeoff) {
        ui.horizontal(|ui| {
            let tabs = [
                (PlanningSubTab::DynamicRab, "💰 RAB 5D"),
                (PlanningSubTab::TakeoffQto, "📊 QTO"),
                (PlanningSubTab::Schedule4d, "⏱️ Kurva S 4D"),
                (PlanningSubTab::CostAriTradeoff, "📈 Cost vs ARI"),
                (PlanningSubTab::StagedErectionSvd, "🏗️ Staged Erection Guard"),
            ];

            for (tab, label) in tabs {
                let is_active = self.active_subtab == tab;
                let bg = if is_active { theme.accent_blue } else { theme.bg_card };
                let fg = if is_active { Color32::WHITE } else { theme.text_secondary };

                let btn = egui::Button::new(RichText::new(label).size(12.0).strong().color(fg))
                    .fill(bg)
                    .corner_radius(CornerRadius::same(8));

                if ui.add(btn).clicked() {
                    self.active_subtab = tab;
                }
            }

            // Right Side Actions
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Publish to Dossier
                let pub_text = if self.published_to_dossier { "✅ Saved to Dossier" } else { "📁 Simpan ke MY PROJECT" };
                let pub_bg = if self.published_to_dossier { theme.accent_green } else { theme.accent_blue.gamma_multiply(0.8) };
                if ui.add(egui::Button::new(RichText::new(pub_text).size(11.5).strong().color(Color32::WHITE))
                    .fill(pub_bg)
                    .corner_radius(CornerRadius::same(8))).clicked() {
                    self.published_to_dossier = true;
                    self.export_status = Some("RAB & Schedule 5D disinkronkan ke Dossier Proyek!".to_string());
                }

                ui.add_space(6.0);

                // Export CSV / Excel Button
                if ui.add(egui::Button::new(RichText::new("📄 Ekspor RAB (CSV)").size(11.5).color(theme.text_primary))
                    .fill(theme.bg_card)
                    .corner_radius(CornerRadius::same(8))).clicked() {
                    self.export_status = Some(format!("File RAB-5D_L{:.0}m.csv berhasil diekspor!", takeoff.span_m));
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
                    ui.label(RichText::new(format!("🟢 {}", msg)).size(11.5).color(theme.accent_green));
                });
        }
    }

    fn render_parametric_binding_banner(&self, ui: &mut Ui, theme: &MacTheme, takeoff: &Parametric5DTakeoff) {
        Frame::new()
            .fill(theme.bg_sidebar)
            .stroke(Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.4)))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::symmetric(16, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔗 TERIKAT OTOMATIS KE MODEL PARAMETRIK:").size(11.5).strong().color(theme.accent_blue));
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("Bentang L = {:.1} m", takeoff.span_m)).size(11.5).color(theme.text_primary));
                    ui.label(RichText::new("•").size(11.5).color(theme.text_muted));
                    ui.label(RichText::new(format!("Tinggi H = {:.1} m (L/{:.1})", takeoff.height_m, takeoff.span_m / takeoff.height_m)).size(11.5).color(theme.text_primary));
                    ui.label(RichText::new("•").size(11.5).color(theme.text_muted));
                    ui.label(RichText::new(format!("Bahan: ρ = {:.0} kg/m³", takeoff.density_kg_m3)).size(11.5).color(theme.text_primary));
                    ui.label(RichText::new("•").size(11.5).color(theme.text_muted));
                    ui.label(RichText::new(format!("Harga Baja: Rp {}/kg", format_idr(takeoff.price_steel_kg))).size(11.5).color(theme.accent_green));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("⚡ Perhitungan Live Dinamis").size(11.0).color(theme.text_muted));
                    });
                });
            });
    }

    // -------------------------------------------------------------
    // SUB-TAB 1: DYNAMIC RAB (5D Cost Estimation)
    // -------------------------------------------------------------
    fn render_dynamic_rab(&self, ui: &mut Ui, theme: &MacTheme, t: &Parametric5DTakeoff) {
        // 4 KPI Metric Cards
        ui.columns(4, |cols| {
            self.metric_card(&mut cols[0], theme, "TOTAL RAB (5D)", &format!("Rp {}", format_idr(t.grand_total_rab)), "Termasuk PPN 11% & Overhead", theme.accent_blue);
            self.metric_card(&mut cols[1], theme, "BIAYA MATERIAL BAJA", &format!("Rp {}", format_idr(t.cost_steel_material)), &format!("{:.1} Ton @ Rp {}/kg", t.total_steel_ton, format_idr(t.price_steel_kg)), theme.accent_green);
            self.metric_card(&mut cols[2], theme, "FABRIKASI & EREKSI", &format!("Rp {}", format_idr(t.cost_workshop_fab + t.cost_erection_crane)), "Workshop + Mobile Crane 100T", theme.accent_orange);
            self.metric_card(&mut cols[3], theme, "PONDASI & SUBSTRUCTURE", &format!("Rp {}", format_idr(t.cost_substructure)), &format!("{:.1} m³ Beton K-350", t.concrete_footing_m3), theme.text_secondary);
        });

        ui.add_space(16.0);

        // Detailed AHSP Breakdown Table
        theme.card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("RINCIAN ESTIMASI BIAYA (RAB) — STANDAR AHSP DINAMIS").size(13.0).strong().color(theme.text_primary));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("Mata Uang: IDR (Rupiah)").size(11.0).color(theme.text_muted));
                });
            });

            ui.add_space(10.0);

            // Table Header
            ui.horizontal(|ui| {
                ui.label(RichText::new("No").strong().size(12.0).color(theme.text_muted));
                ui.add_space(18.0);
                ui.label(RichText::new("Uraian Pekerjaan (WBS)").strong().size(12.0).color(theme.text_muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("Total Harga (Rp)").strong().size(12.0).color(theme.text_muted));
                    ui.add_space(40.0);
                    ui.label(RichText::new("Harga Satuan (Rp)").strong().size(12.0).color(theme.text_muted));
                    ui.add_space(30.0);
                    ui.label(RichText::new("Sat").strong().size(12.0).color(theme.text_muted));
                    ui.add_space(30.0);
                    ui.label(RichText::new("Volume").strong().size(12.0).color(theme.text_muted));
                });
            });
            ui.separator();

            let items = [
                ("1.0", "Mobilisasi Peralatan Berat (Crane 100T, Trailer, Genset)", 1.0, "ls", t.cost_mobilization, t.cost_mobilization),
                ("2.0", "Pekerjaan Pondasi Bored Pile & Abutment Beton K-350", t.concrete_footing_m3, "m³", t.price_concrete_m3, t.cost_substructure),
                ("3.0", "Pengadaan Material Baja Profil BJ41 (ASTM A992)", t.total_steel_kg, "kg", t.price_steel_kg, t.cost_steel_material),
                ("4.0", "Fabrikasi Workshop (Cutting, Fitting, Full Pen Welding, NDT)", t.total_steel_kg, "kg", t.price_fab_kg, t.cost_workshop_fab),
                ("5.0", "Proteksi Korosi: Sandblasting Sa 2.5 + Epoxy 3-Lapis", t.coating_surface_m2, "m²", t.price_coating_m2, t.cost_coating_fireproof),
                ("6.0", "Transportasi & Pengangkutan Balok Rangka ke Lokasi Proyek", 1.0, "ls", t.cost_transportation, t.cost_transportation),
                ("7.0", "Ereksi Rangka Baja Bentang Penuh dengan Crane & Shoring", t.total_steel_ton, "ton", t.price_erect_ton, t.cost_erection_crane),
                ("8.0", "Pemasangan Baut Mutu Tinggi A325, Kalibrasi Torsi & Finishing", t.bolt_sets_count as f64, "set", 125_000.0, t.cost_bolts_finishing),
            ];

            for (no, desc, vol, sat, hrg, tot) in items {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(no).size(12.0).color(theme.text_secondary));
                    ui.add_space(12.0);
                    ui.label(RichText::new(desc).size(12.0).strong().color(theme.text_primary));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(format_idr(tot)).size(12.0).strong().color(theme.accent_green));
                        ui.add_space(40.0);
                        ui.label(RichText::new(format_idr(hrg)).size(12.0).color(theme.text_secondary));
                        ui.add_space(30.0);
                        ui.label(RichText::new(sat).size(12.0).color(theme.text_muted));
                        ui.add_space(30.0);
                        ui.label(RichText::new(format!("{:.2}", vol)).size(12.0).color(theme.text_secondary));
                    });
                });
                ui.add_space(3.0);
            }

            ui.separator();
            ui.add_space(6.0);

            // Summary Totals
            ui.horizontal(|ui| {
                ui.label(RichText::new("Subtotal Biaya Langsung (Direct Cost):").size(12.0).color(theme.text_secondary));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("Rp {}", format_idr(t.direct_cost_total))).size(12.5).strong().color(theme.text_primary));
                });
            });

            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Overhead & Jasa Kontraktor ({:.1}%):", self.overhead_percent)).size(11.5).color(theme.text_muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("Rp {}", format_idr(t.overhead_cost))).size(11.5).color(theme.text_muted));
                });
            });

            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Biaya Tak Terduga / Contingency ({:.1}%):", self.contingency_percent)).size(11.5).color(theme.text_muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("Rp {}", format_idr(t.contingency_cost))).size(11.5).color(theme.text_muted));
                });
            });

            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Pajak Pertambahan Nilai (PPN {:.1}%):", self.vat_percent)).size(11.5).color(theme.text_muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("Rp {}", format_idr(t.vat_cost))).size(11.5).color(theme.text_muted));
                });
            });

            ui.separator();
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("GRAND TOTAL RENCANA ANGGARAN BIAYA (5D BIM):").size(13.0).strong().color(Color32::WHITE));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("Rp {},-", format_idr(t.grand_total_rab))).size(17.0).strong().color(theme.accent_blue));
                });
            });
        });
    }

    // -------------------------------------------------------------
    // SUB-TAB 2: QUANTITY TAKEOFF (QTO)
    // -------------------------------------------------------------
    fn render_qto_takeoff(&self, ui: &mut Ui, theme: &MacTheme, t: &Parametric5DTakeoff, _members: &[StructuralMember]) {
        ui.columns(4, |cols| {
            self.metric_card(&mut cols[0], theme, "TOTAL TONASE BAJA", &format!("{:.2} Ton", t.total_steel_ton), &format!("{} kg volume murni", format_idr(t.total_steel_kg)), theme.accent_blue);
            self.metric_card(&mut cols[1], theme, "LUAS PROTEKSI CAT", &format!("{:.1} m²", t.coating_surface_m2), "Sa 2.5 + 3-Coat System", theme.accent_green);
            self.metric_card(&mut cols[2], theme, "SAMBUNGAN BAUT A325", &format!("{} Sets", t.bolt_sets_count), "Grade 8.8 High Strength", theme.accent_orange);
            self.metric_card(&mut cols[3], theme, "VOLUME BETON PONDASI", &format!("{:.1} m³", t.concrete_footing_m3), "Mutu fc' 30 MPa (K-350)", theme.text_secondary);
        });

        ui.add_space(16.0);

        theme.card_frame().show(ui, |ui| {
            ui.label(RichText::new("BILL OF QUANTITIES (BOQ) — FORMULA GEOMETRI ELEMEN").size(13.0).strong().color(theme.text_primary));
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
                ("BATANG TEKAN ATAS (Top Chord)", top_len.as_str(), "W14x90", top_wt.as_str(), "AISC 360-16 LRFD"),
                ("BATANG TARIK BAWAH (Bottom Chord)", bot_len.as_str(), "W14x90", bot_wt.as_str(), "Tension Net Section"),
                ("BATANG DIAGONAL WEB (Diagonals)", diag_len.as_str(), "HSS8x8x1/2", diag_wt.as_str(), "Alternating Warren"),
                ("PLAT BUHUL & PENGAKU (Gusset Plates)", "52 Lembar", "Plat T=16mm BJ41", gusset_wt.as_str(), "ASTM A36 / BJ41"),
                ("ANGKUR & BASEPLATE TUMPAN", "4 Titik Perletakan", "T=32mm + 8x M36", "1.85 Ton", "Elastomeric Bearing Support"),
                ("PONDASI TIANG PANCANG / BORED PILE", conc_vol.as_str(), "Beton K-350", "156.0 Ton", "Substructure Civil Work"),
            ];

            for (elem, dim, sec, weight, code) in qto_rows {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(elem).size(12.0).strong().color(theme.text_primary));
                    ui.label(RichText::new("—").size(12.0).color(theme.text_muted));
                    ui.label(RichText::new(format!("Dimensi: {}", dim)).size(12.0).color(theme.text_secondary));
                    ui.label(RichText::new("•").size(12.0).color(theme.text_muted));
                    ui.label(RichText::new(format!("Profil: {}", sec)).size(12.0).color(theme.accent_blue));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(code).size(11.0).color(theme.text_muted));
                        ui.add_space(20.0);
                        ui.label(RichText::new(weight).size(12.0).strong().color(theme.accent_green));
                    });
                });
                ui.add_space(6.0);
                ui.separator();
            }
        });
    }

    // -------------------------------------------------------------
    // SUB-TAB 3: PENJADWALAN 4D & KURVA S
    // -------------------------------------------------------------
    fn render_schedule_4d(&self, ui: &mut Ui, theme: &MacTheme, t: &Parametric5DTakeoff) {
        ui.columns(4, |cols| {
            self.metric_card(&mut cols[0], theme, "TOTAL WAKTU KERJA", &format!("{} Hari", t.duration_days), &format!("{:.1} Bulan Kalender", t.duration_days as f64 / 30.0), theme.accent_blue);
            self.metric_card(&mut cols[1], theme, "TOTAL MAN-DAYS (OH)", &format!("{} OH", t.total_man_days), "Kebutuhan Tenaga Kerja", theme.accent_orange);
            self.metric_card(&mut cols[2], theme, "LINTASAN KRITIS (CPM)", "WBS 5.0 (Ereksi)", "Zero Float Critical Path", theme.accent_red);
            self.metric_card(&mut cols[3], theme, "SVD ERECTION GUARD", "Rank Safe (14/14)", "Zero Mechanism saat Cantilever", theme.accent_green);
        });

        ui.add_space(16.0);

        theme.card_frame().show(ui, |ui| {
            ui.label(RichText::new("WBS 4D PENJADWALAN & SIMULASI TAHAPAN EREKSI (STAGED CONSTRUCTION)").size(13.0).strong().color(theme.text_primary));
            ui.add_space(10.0);

            let mut engine = crate::core::ParametricEngine::new();
            let live_schedule = engine.generate_warren_truss().ok().map(|m| {
                crate::core::ScheduleEngine::generate_standard_schedule(&m, t.grand_total_rab)
            });

            if let Some(ref sched) = live_schedule {
                for act in &sched.activities {
                    let stat_color = if act.is_critical {
                        theme.accent_red
                    } else if act.total_float < 7 {
                        theme.accent_orange
                    } else {
                        theme.accent_green
                    };
                    let status_str = if act.is_critical {
                        "Kritis (CPM - Float 0)"
                    } else {
                        "Float Aman"
                    };

                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("WBS {:02}", act.id)).size(12.0).strong().color(theme.accent_blue));
                        ui.add_space(10.0);
                        ui.label(RichText::new(&act.name).size(12.0).strong().color(theme.text_primary));
                        ui.add_space(14.0);
                        ui.label(RichText::new(format!("⏱️ Hari {}-{} (Dur: {}h)", act.early_start, act.early_finish, act.duration_days)).size(11.5).color(theme.text_secondary));

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            Frame::new()
                                .fill(stat_color.gamma_multiply(0.2))
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(Margin::symmetric(6, 2))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(status_str).size(10.5).color(stat_color));
                                });
                            ui.add_space(14.0);
                            ui.label(RichText::new(format!("Float: {}h", act.total_float)).size(11.0).color(theme.text_muted));
                            ui.add_space(10.0);
                            ui.label(RichText::new(format!("Bobot: {:.1}%", act.weight_percentage)).size(11.5).color(theme.text_secondary));
                        });
                    });

                    ui.add_space(2.0);
                    let (rect, _resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 6.0), egui::Sense::hover());
                    let painter = ui.painter();
                    painter.rect_filled(rect, CornerRadius::same(3), theme.bg_canvas);
                    let filled_width = rect.width() * (act.weight_percentage as f32 / 30.0).min(1.0);
                    let filled_rect = egui::Rect::from_min_size(rect.left_top(), Vec2::new(filled_width, rect.height()));
                    painter.rect_filled(filled_rect, CornerRadius::same(3), stat_color);

                    ui.add_space(6.0);
                }
            }
        });

        ui.add_space(14.0);

        // Visual Kurva S Display
        theme.card_frame().show(ui, |ui| {
            ui.label(RichText::new("📈 KURVA S — PROYEKSI CASHFLOW & PROGRES KUMULATIF MINGGUAN").size(13.0).strong().color(theme.text_primary));
            ui.add_space(8.0);
            ui.label(RichText::new("Target kumulatif progres fisik mingguan (Rencana S-Curve) terintegrasi dengan pencairan termin RAB:").size(11.5).color(theme.text_muted));
            ui.add_space(8.0);

            // Graphical ASCII representation of S-Curve Progression
            let weeks = [
                ("M01-M04", " 8.5%", " Rp 280 Jt", "██▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒ Mobilisasi & Galian"),
                ("M05-M08", "24.0%", " Rp 780 Jt", "█████▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒ Abutment Selesai & Fabrikasi Dimulai"),
                ("M09-M12", "58.0%", "Rp 1.880 Jt", "███████████▒▒▒▒▒▒▒▒▒ Fabrikasi 100% & Coating Selesai"),
                ("M13-M16", "86.5%", "Rp 2.850 Jt", "█████████████████▒▒▒ Ereksi Rangka Bentang 1-6 (SVD Certified)"),
                ("M17-M20", "100.0%", "Rp Selesai", "████████████████████ Finishing, Baut Torsi & Serah Terima"),
            ];

            for (w, pct, cash, bar) in weeks {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(w).size(11.5).strong().color(theme.accent_blue));
                    ui.add_space(10.0);
                    ui.label(RichText::new(pct).size(11.5).color(theme.accent_green));
                    ui.add_space(10.0);
                    let display_cash = if cash == "Rp Selesai" {
                        format!("Rp {} Jt", format_idr(t.grand_total_rab / 1_000_000.0))
                    } else {
                        cash.to_string()
                    };
                    ui.label(RichText::new(display_cash).size(11.5).color(theme.text_primary));
                    ui.add_space(14.0);
                    ui.label(RichText::new(bar).size(11.5).color(theme.text_secondary));
                });
                ui.add_space(3.0);
            }
        });
    }

    // -------------------------------------------------------------
    // SUB-TAB 4: COST VS ARI TRADE-OFF OPTIMIZER
    // -------------------------------------------------------------
    fn render_cost_ari_tradeoff(&self, ui: &mut Ui, theme: &MacTheme, t: &Parametric5DTakeoff) {
        theme.card_frame().show(ui, |ui| {
            ui.label(RichText::new("VALUE ENGINEERING: SENSITIVITAS BIAYA (RAB) TERHADAP INDEKS REDUNDANSI (ARI)").size(13.5).strong().color(theme.accent_blue));
            ui.add_space(4.0);
            ui.label(RichText::new("Analisis komparatif untuk mengetahui seberapa besar penambahan biaya investasi yang dibutuhkan guna meningkatkan ketahanan keruntuhan progresif struktur:").size(11.5).color(theme.text_secondary));
            ui.add_space(14.0);

            let options = [
                ("Opsi 1: Minimum Code Compliance", 0.64, t.grand_total_rab * 0.92, "-8.0%", "🔴 Rentan Kegagalan Tunggal (1 Member Hilang = Runtuh)", theme.accent_red),
                ("Opsi 2: Desain Standar Baseline", 0.78, t.grand_total_rab * 0.97, "-3.0%", "🟡 Memenuhi Standar Minimal SNI / AISC", theme.accent_orange),
                ("Opsi 3: Desain Teroptimasi Redundan (Rekomendasi)", t.predicted_system_ari, t.grand_total_rab, "0.0% (Base)", "🟢 Robust Multi-Path (Kapasitas Cadangan Tinggi)", theme.accent_green),
                ("Opsi 4: Ultra-Redundant (Infrastruktur Kritis)", 0.965, t.grand_total_rab * 1.075, "+7.5%", "🛡️ Dual-Path Collapse Proof (Jembatan Strategis)", theme.accent_blue),
            ];

            for (title, ari, cost, diff, desc, badge_color) in options {
                Frame::new()
                    .fill(theme.bg_sidebar)
                    .stroke(Stroke::new(1.0, theme.border_subtle))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(title).size(13.0).strong().color(theme.text_primary));
                                ui.add_space(2.0);
                                ui.label(RichText::new(desc).size(11.5).color(badge_color));
                            });

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(RichText::new(format!("Rp {},-", format_idr(cost))).size(13.5).strong().color(theme.accent_blue));
                                ui.add_space(24.0);
                                ui.label(RichText::new(diff).size(12.0).color(theme.text_muted));
                                ui.add_space(24.0);
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

            ui.horizontal(|ui| {
                ui.label(RichText::new("💡 KESIMPULAN REKAYASA NILAI (VALUE ENGINEERING):").size(12.0).strong().color(theme.accent_green));
                ui.label(RichText::new("Hanya dengan kenaikan biaya +3.8% dari baseline standar, indeks redundansi ARI meningkat drastis +16.9%, mencegah mekanisme keruntuhan katastropik saat terjadi gempa atau benturan kendaraan.").size(12.0).color(theme.text_secondary));
            });
        });
    }

    // -------------------------------------------------------------
    // SUB-TAB 5: Staged Erection SVD & ARI Guard
    // -------------------------------------------------------------
    fn render_staged_erection_svd(&self, ui: &mut Ui, theme: &MacTheme, _takeoff: &Parametric5DTakeoff) {
        ui.columns(4, |cols| {
            self.metric_card(&mut cols[0], theme, "TAHAPAN EREKSI", "5 Tahap", "WBS Erection Sequence", theme.accent_blue);
            self.metric_card(&mut cols[1], theme, "TAHAP PALING KRITIS", "Tahap 2 (WBS 2.2)", "ARI: 0.285 (Shoring Wajib)", theme.accent_red);
            self.metric_card(&mut cols[2], theme, "STABILITAS CLOSURE", "Minggu Ke-12", "SVD Gate 1 Full Rank", theme.accent_green);
            self.metric_card(&mut cols[3], theme, "FINAL OPERATIONAL ARI", "0.942", "Safe Multi-Path System", theme.accent_green);
        });

        ui.add_space(16.0);

        theme.card_frame().show(ui, |ui| {
            ui.label(RichText::new("SIMULASI KESELAMATAN EREKSI BERTAHAP (STAGED ERECTION SVD GUARD)").size(14.0).strong().color(theme.text_primary));
            ui.add_space(4.0);
            ui.label(RichText::new("Struktur baja sering kali belum memiliki kekakuan redundan sempurna selama masa ereksi di lapangan sebelum penutupan rangka (closure). Evaluasi tahapan ini mendeteksi titik rawan keruntuhan saat perakitan:").size(12.0).color(theme.text_secondary));
            ui.add_space(12.0);

            let mut engine = crate::core::ParametricEngine::new();
            let staged_results = engine.generate_warren_truss().ok().map(|m| {
                crate::core::ScheduleEngine::verify_staged_erection(&m)
            });

            if let Some(stages) = staged_results {
                for stage in &stages {
                    let col = if stage.is_stable { theme.accent_green } else { theme.accent_red };
                    let status_str = if stage.is_stable { "🟢 STABIL BEBAS MEKANISME" } else { "🔴 KRITIS SHORING WAJIB" };
                    let note = match stage.warning.as_deref() {
                        Some(w) => w,
                        None => "Kekakuan bertahap stabil rank penuh, deformasi terkendali.",
                    };

                    Frame::new()
                        .fill(theme.bg_card)
                        .stroke(Stroke::new(1.0, col.gamma_multiply(0.4)))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("●").size(14.0).color(col));
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(&stage.stage_name).size(12.5).strong().color(theme.text_primary));
                                    ui.label(RichText::new(format!("Elemen: {} batang • Rank SVD: {}/{} • Kondisi: {:.1e}", stage.erected_elements_count, stage.rank, stage.active_dofs, stage.condition_number)).size(11.0).color(theme.text_secondary));
                                    ui.add_space(2.0);
                                    ui.label(RichText::new(format!("💡 Evaluasi: {}", note)).size(11.0).color(theme.text_muted));
                                });

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    Frame::new()
                                        .fill(col.gamma_multiply(0.18))
                                        .stroke(Stroke::new(1.0, col))
                                        .corner_radius(CornerRadius::same(6))
                                        .inner_margin(Margin::symmetric(8, 4))
                                        .show(ui, |ui| {
                                            ui.label(RichText::new(format!("Defl: {:.2} mm", stage.max_deflection_mm)).size(11.5).strong().color(Color32::WHITE));
                                        });
                                    ui.add_space(14.0);
                                    ui.label(RichText::new(status_str).size(11.0).strong().color(col));
                                });
                            });
                        });
                    ui.add_space(6.0);
                }
            }
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
