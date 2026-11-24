//! research.rs — Pillar 1D: Research, GIS Site Inception & Topographic DEM TIN Studio.
//! Re-ordered modern workspace layout featuring:
//! - Phase 25 (v0.2.5): Digital Elevation Model (DEM) & Topographic TIN Mesh (10,082 Delaunay triangles,
//!   Marching-Triangles 1.0m/5.0m contours, >30° SNI 8460:2017 slope hazard shading, Cut/Fill earthwork)
//! - Phase 23 (v0.2.3): EPSG:3857 Web Mercator Slippy Map Quadtree Engine (`walkers`)
//! - Phase 24 (v0.2.4): Pure-Rust Geodetic CRS & UTM Projection Engine + IFCX IfcMapConversion

use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Frame, Layout, Margin, Pos2, Rect, RichText,
    ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2,
};
use crate::core::gis::{
    ContourPolyline, CutFillVolumeReport, DemTerrainPreset, GeoProjectionEngine, SlippyMapState,
    TileSource, TinSurfaceMesh,
};
use crate::features::{FeatureId, FeatureRegistry};
use crate::icons::{self, AppIcon};
use crate::models::NavigationPillar;
use crate::theme::MacTheme;
use crate::ui_kit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResearchSubTab {
    SurveyingSite,
    GeotechBorehole,
    HydrologyFloodplain,
    ConceptFeasibility,
    RegulatoryCodes,
    EigenGateFormulations,
}

#[derive(Clone, Debug)]
pub struct BoreholeStratum {
    pub depth_top_m: f32,
    pub depth_bot_m: f32,
    pub soil_name: String,
    pub n60: u32,
    pub vs_mps: f32,
    pub strength_desc: String,
    pub allowable_desc: String,
    pub color_rgb: [u8; 3],
}

#[derive(Clone, Debug)]
pub struct GeotechProfileSummary {
    pub vs30_mps: f32,
    pub site_class: &'static str,
    pub liquefaction_fs: f32,
    pub bearing_stratum_depth_m: f32,
    pub rec_foundation: String,
}

pub fn preset_borehole_strata(idx: usize) -> Vec<BoreholeStratum> {
    match idx {
        1 => vec![
            BoreholeStratum {
                depth_top_m: 0.0,
                depth_bot_m: 4.5,
                soil_name: "Soft Marine Silt Fill".to_string(),
                n60: 5,
                vs_mps: 125.0,
                strength_desc: "cu = 22 kPa".to_string(),
                allowable_desc: "Skin 18 kPa".to_string(),
                color_rgb: [120, 108, 96],
            },
            BoreholeStratum {
                depth_top_m: 4.5,
                depth_bot_m: 14.0,
                soil_name: "Loose Saturated Silty Sand".to_string(),
                n60: 9,
                vs_mps: 165.0,
                strength_desc: "phi' = 29 deg".to_string(),
                allowable_desc: "Liquefiable Watch".to_string(),
                color_rgb: [152, 134, 88],
            },
            BoreholeStratum {
                depth_top_m: 14.0,
                depth_bot_m: 24.0,
                soil_name: "Medium-Dense Sandy Clay".to_string(),
                n60: 26,
                vs_mps: 255.0,
                strength_desc: "phi' = 34 deg".to_string(),
                allowable_desc: "Skin 74 kPa".to_string(),
                color_rgb: [94, 114, 128],
            },
            BoreholeStratum {
                depth_top_m: 24.0,
                depth_bot_m: 30.0,
                soil_name: "Dense Cemented Sand".to_string(),
                n60: 48,
                vs_mps: 410.0,
                strength_desc: "phi' = 40 deg".to_string(),
                allowable_desc: "End 1,450 kPa".to_string(),
                color_rgb: [176, 146, 86],
            },
        ],
        2 => vec![
            BoreholeStratum {
                depth_top_m: 0.0,
                depth_bot_m: 5.0,
                soil_name: "Volcanic Tuffaceous Silt".to_string(),
                n60: 18,
                vs_mps: 225.0,
                strength_desc: "cu = 65 kPa".to_string(),
                allowable_desc: "Skin 52 kPa".to_string(),
                color_rgb: [134, 114, 92],
            },
            BoreholeStratum {
                depth_top_m: 5.0,
                depth_bot_m: 16.0,
                soil_name: "Stiff Andesitic Breccia".to_string(),
                n60: 38,
                vs_mps: 380.0,
                strength_desc: "phi' = 38 deg".to_string(),
                allowable_desc: "Skin 110 kPa".to_string(),
                color_rgb: [110, 118, 126],
            },
            BoreholeStratum {
                depth_top_m: 16.0,
                depth_bot_m: 30.0,
                soil_name: "Sound Andesite Bedrock".to_string(),
                n60: 55,
                vs_mps: 690.0,
                strength_desc: "qu = 5.4 MPa".to_string(),
                allowable_desc: "End 2,600 kPa".to_string(),
                color_rgb: [96, 102, 114],
            },
        ],
        _ => vec![
            BoreholeStratum {
                depth_top_m: 0.0,
                depth_bot_m: 6.0,
                soil_name: "Engineered Silty Fill".to_string(),
                n60: 11,
                vs_mps: 185.0,
                strength_desc: "phi' = 28 deg".to_string(),
                allowable_desc: "Skin 34 kPa".to_string(),
                color_rgb: [132, 114, 92],
            },
            BoreholeStratum {
                depth_top_m: 6.0,
                depth_bot_m: 15.0,
                soil_name: "Alluvial Silty Clay".to_string(),
                n60: 6,
                vs_mps: 155.0,
                strength_desc: "cu = 32 kPa".to_string(),
                allowable_desc: "Skin 28 kPa".to_string(),
                color_rgb: [92, 112, 128],
            },
            BoreholeStratum {
                depth_top_m: 15.0,
                depth_bot_m: 24.0,
                soil_name: "Dense Cemented Sand".to_string(),
                n60: 34,
                vs_mps: 295.0,
                strength_desc: "phi' = 37 deg".to_string(),
                allowable_desc: "Skin 96 kPa".to_string(),
                color_rgb: [176, 146, 86],
            },
            BoreholeStratum {
                depth_top_m: 24.0,
                depth_bot_m: 30.0,
                soil_name: "Limestone Bedrock".to_string(),
                n60: 52,
                vs_mps: 540.0,
                strength_desc: "qu = 3.8 MPa".to_string(),
                allowable_desc: "End 1,950 kPa".to_string(),
                color_rgb: [104, 108, 120],
            },
        ],
    }
}

pub fn compute_geotech_profile_summary(
    layers: &[BoreholeStratum],
    gwt_m: f32,
    mw: f32,
    pga_g: f32,
) -> GeotechProfileSummary {
    let mut sum_d = 0.0_f32;
    let mut sum_t = 0.0_f32;
    let mut min_fs = 3.0_f32;
    let mut bearing_depth = 24.0_f32;
    let mut found_bearing = false;

    for s in layers {
        if s.depth_top_m >= 30.0 {
            continue;
        }
        let d = (s.depth_bot_m.min(30.0) - s.depth_top_m).max(0.1);
        sum_d += d;
        sum_t += d / s.vs_mps.max(50.0);
        if s.depth_bot_m > gwt_m && s.soil_name.to_lowercase().contains("sand") {
            let n1_60 = (s.n60 as f32).clamp(2.0, 50.0);
            let crr = (0.048 + 0.011 * n1_60 + (n1_60 / 35.0).powi(3) * 0.15).clamp(0.06, 0.65);
            let z_mid = 0.5 * (s.depth_top_m + s.depth_bot_m);
            let rd = (1.0 - 0.00765 * z_mid).max(0.5);
            let eff_ratio = if z_mid > gwt_m { 1.75 } else { 1.0 };
            let msl = (mw / 7.5).powf(-2.5);
            let csr = (0.65 * pga_g.max(0.05) * eff_ratio * rd).max(0.04);
            let fs = ((crr * msl) / csr).clamp(0.3, 4.0);
            min_fs = min_fs.min(fs);
        }
        if !found_bearing && s.n60 >= 34 {
            bearing_depth = s.depth_top_m.max(8.0);
            found_bearing = true;
        }
    }
    let vs30 = if sum_t > 0.0 { sum_d / sum_t } else { 240.0 };
    let site_class = if vs30 > 1500.0 {
        "Class SA (Hard Rock)"
    } else if vs30 > 750.0 {
        "Class SB (Rock)"
    } else if vs30 >= 350.0 {
        "Class SC (Very Dense Soil)"
    } else if vs30 >= 175.0 {
        "Class SD (Stiff/Medium)"
    } else {
        "Class SE (Soft Soil)"
    };
    let rec_foundation = if bearing_depth <= 6.0 && layers.first().is_some_and(|l| l.n60 >= 18) {
        format!("Shallow Pad Footings @ -{:.1}m", bearing_depth)
    } else {
        format!("-{:.1}m Bored Pile Socket", bearing_depth.max(12.0))
    };

    GeotechProfileSummary {
        vs30_mps: vs30,
        site_class,
        liquefaction_fs: min_fs,
        bearing_stratum_depth_m: bearing_depth,
        rec_foundation,
    }
}

#[derive(Clone, Debug)]
pub struct HydrologyIdfSummary {
    pub return_period_yr: u32,
    pub intensity_mm_hr: f32,
    pub peak_runoff_m3_s: f32,
    pub detention_vol_m3: f32,
    pub recommended_ffe_asl_m: f32,
}

pub fn compute_hydrology_idf_floodplain(
    return_idx: usize,
    area_ha: f32,
    runoff_c: f32,
    tc_min: f32,
    i100_1h_mm: f32,
    flood_wl_asl_m: f32,
) -> HydrologyIdfSummary {
    let (yr, factor) = match return_idx {
        0 => (10, 0.58_f32),
        1 => (25, 0.76_f32),
        _ => (100, 1.00_f32),
    };
    let i_tc = i100_1h_mm.max(20.0) * factor * (60.0 / tc_min.clamp(5.0, 180.0)).powf(0.55);
    let q_peak = 0.00278 * runoff_c.clamp(0.15, 0.98) * i_tc * area_ha.max(0.2);
    let det_ratio = ((runoff_c - 0.35).max(0.20) / runoff_c.max(0.35)).clamp(0.25, 0.85);
    let vol_m3 = q_peak * (tc_min.max(5.0) * 60.0) * det_ratio * 1.15;
    let freeboard = if yr >= 100 { 0.65 } else { 0.50 };
    HydrologyIdfSummary {
        return_period_yr: yr,
        intensity_mm_hr: i_tc,
        peak_runoff_m3_s: q_peak,
        detention_vol_m3: vol_m3,
        recommended_ffe_asl_m: flood_wl_asl_m + freeboard,
    }
}

pub struct ResearchConceptView {
    pub active_subtab: ResearchSubTab,
    pub selected_truss_family: String,
    pub target_design_span_m: f32,

    // Phase 23: Slippy Map Engine
    pub slippy_map: SlippyMapState,

    // Phase 24: Geodetic CRS & UTM Projection Engine
    pub projection_engine: GeoProjectionEngine,
    pub crs_input_lat: String,
    pub crs_input_lon: String,
    pub crs_input_easting: String,
    pub crs_input_northing: String,
    pub crs_selected_zone: u8,
    pub crs_is_south: bool,
    pub crs_conversion_log: Vec<String>,

    // Phase 25: Digital Elevation Model (DEM) & Topographic TIN Mesh
    pub tin_preset: DemTerrainPreset,
    pub tin_mesh: TinSurfaceMesh,
    pub tin_contours: Vec<ContourPolyline>,
    pub tin_cut_fill_report: CutFillVolumeReport,
    pub tin_datum_elevation_m: f32,
    pub tin_minor_contour_m: f32,
    pub tin_major_contour_m: f32,
    pub tin_vertical_exaggeration: f32,
    pub tin_slope_hazard_deg: f32,
    pub tin_3d_isometric: bool,
    pub tin_show_wireframe: bool,
    pub tin_show_contours: bool,
    pub tin_show_slope_hazard: bool,
    pub tin_show_datum_plane: bool,
    pub tin_yaw_deg: f32,
    pub tin_pitch_deg: f32,
    // Phase 39: Geotech Borehole (1D-05) & Site Hydrology IDF (1D-06)
    pub geotech_borehole_idx: usize,
    pub geotech_gwt_depth_m: f32,
    pub geotech_eq_mw: f32,
    pub geotech_pga_g: f32,
    pub geotech_layers: Vec<BoreholeStratum>,
    pub hydro_return_idx: usize,
    pub hydro_catchment_ha: f32,
    pub hydro_runoff_c: f32,
    pub hydro_tc_min: f32,
    pub hydro_i100_1h_mm: f32,
    pub hydro_flood_wl_asl_m: f32,
}

impl Default for ResearchConceptView {
    fn default() -> Self {
        let preset = DemTerrainPreset::VolcanicFoothillsRidge;
        let tin_mesh = TinSurfaceMesh::triangulate_bowyer_watson(Vec::new());
        let datum = 0.0;
        let tin_contours = Vec::new();
        let tin_cut_fill_report = tin_mesh.compute_cut_fill_report(datum, 30.0);

        Self {
            // Re-ordered default sub-tab to immediately surface the interactive GIS & TIN Canvas
            active_subtab: ResearchSubTab::SurveyingSite,
            selected_truss_family: "Warren with Verticals (Family S1)".to_string(),
            target_design_span_m: 24.0,
            slippy_map: SlippyMapState::default(),
            projection_engine: GeoProjectionEngine::new(48, true),
            crs_input_lat: String::new(),
            crs_input_lon: String::new(),
            crs_input_easting: String::new(),
            crs_input_northing: String::new(),
            crs_selected_zone: 48,
            crs_is_south: true,
            crs_conversion_log: Vec::new(),

            tin_preset: preset,
            tin_mesh,
            tin_contours,
            tin_cut_fill_report,
            tin_datum_elevation_m: datum,
            tin_minor_contour_m: 1.0,
            tin_major_contour_m: 5.0,
            tin_vertical_exaggeration: 1.0,
            tin_slope_hazard_deg: 30.0,
            tin_3d_isometric: true,
            tin_show_wireframe: true,
            tin_show_contours: true,
            tin_show_slope_hazard: true,
            tin_show_datum_plane: true,
            tin_yaw_deg: 38.0,
            tin_pitch_deg: 54.0,
            geotech_borehole_idx: 0,
            geotech_gwt_depth_m: 3.2,
            geotech_eq_mw: 7.8,
            geotech_pga_g: 0.36,
            geotech_layers: preset_borehole_strata(0),
            hydro_return_idx: 2,
            hydro_catchment_ha: 6.2,
            hydro_runoff_c: 0.75,
            hydro_tc_min: 22.0,
            hydro_i100_1h_mm: 88.0,
            hydro_flood_wl_asl_m: 1.50,
        }
    }
}

impl ResearchConceptView {
    /// Parses CSV / XYZ survey point clouds (`x, y, z` or whitespace-delimited) into the TIN surface mesh.
    pub fn import_survey_points_csv_xyz(&mut self, content: &str) -> Result<usize, String> {
        let mut raw_pts: Vec<(f32, f32, f32)> = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let cleaned = trimmed.replace([',', ';', '\t'], " ");
            let nums: Vec<f32> = cleaned
                .split_whitespace()
                .filter_map(|t| t.parse::<f32>().ok())
                .collect();
            if nums.len() >= 3 {
                raw_pts.push((nums[0], nums[1], nums[2]));
            }
        }
        if raw_pts.len() < 3 {
            return Err("CSV/XYZ survey file requires at least 3 numeric X, Y, Z rows.".to_string());
        }
        let min_x = raw_pts.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
        let min_y = raw_pts.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
        let max_pts = 900;
        let step = (raw_pts.len() / max_pts).max(1);
        let pts: Vec<crate::core::gis::TinPoint> = raw_pts
            .into_iter()
            .step_by(step)
            .enumerate()
            .map(|(id, (x, y, z))| crate::core::gis::TinPoint {
                id,
                x: x - min_x,
                y: y - min_y,
                z: z * self.tin_vertical_exaggeration,
            })
            .collect();
        let count = pts.len();
        self.tin_mesh = TinSurfaceMesh::triangulate_bowyer_watson(pts);
        self.tin_datum_elevation_m = self.tin_datum_elevation_m.clamp(
            self.tin_mesh.min_elevation,
            self.tin_mesh.max_elevation,
        );
        self.tin_contours = self
            .tin_mesh
            .extract_contours(self.tin_minor_contour_m, self.tin_major_contour_m);
        self.tin_cut_fill_report = self
            .tin_mesh
            .compute_cut_fill_report(self.tin_datum_elevation_m, self.tin_slope_hazard_deg);
        Ok(count)
    }

    fn rebuild_tin_mesh(&mut self) {
        self.tin_mesh = TinSurfaceMesh::generate_from_dem_preset(
            self.tin_preset,
            500.0,
            500.0,
            72,
            72,
            self.tin_vertical_exaggeration,
        );
        self.tin_datum_elevation_m = self.tin_datum_elevation_m.clamp(
            self.tin_mesh.min_elevation,
            self.tin_mesh.max_elevation,
        );
        self.tin_contours = self
            .tin_mesh
            .extract_contours(self.tin_minor_contour_m, self.tin_major_contour_m);
        self.tin_cut_fill_report = self
            .tin_mesh
            .compute_cut_fill_report(self.tin_datum_elevation_m, self.tin_slope_hazard_deg);
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        active_pillar: &mut NavigationPillar,
        features: &mut FeatureRegistry,
    ) {
        ui.vertical(|ui| {
            Frame::new().inner_margin(Margin::symmetric(0, 8)).show(ui, |ui| {
                let full_w = ui.available_width().max(160.0);
                ui.set_width(full_w);
                ui.set_max_width(full_w);
                // 1. TOP SUBTABS SELECTOR (Re-Ordered: GIS & DEM TIN Studio First)
                self.render_header_bar(ui, theme);

                ui.add_space(14.0);

                // 2. ACTIVE SUB-TAB CONTENT
                match self.active_subtab {
                    ResearchSubTab::SurveyingSite => self.render_surveying_site(ui, theme, features),
                    ResearchSubTab::GeotechBorehole => self.render_geotech_borehole(ui, theme, features),
                    ResearchSubTab::HydrologyFloodplain => {
                        self.render_hydrology_floodplain(ui, theme, features)
                    }
                    ResearchSubTab::ConceptFeasibility => {
                        self.render_concept_feasibility(ui, theme, active_pillar)
                    }
                    ResearchSubTab::RegulatoryCodes => self.render_regulatory_codes(ui, theme),
                    ResearchSubTab::EigenGateFormulations => self.render_eigengate_formulations(ui, theme),
                }

                ui.add_space(24.0);
            });
        });
    }

    fn render_header_bar(&mut self, ui: &mut Ui, theme: &MacTheme) {
        let tabs = [
            (
                ResearchSubTab::SurveyingSite,
                "⛰️ GIS, DEM TIN Mesh & Geodetic Survey",
            ),
            (
                ResearchSubTab::GeotechBorehole,
                "🕳️ Geotech Borehole & SPT (1D-05)",
            ),
            (
                ResearchSubTab::HydrologyFloodplain,
                "🌊 Hydrology IDF & Floodplain (1D-06)",
            ),
            (
                ResearchSubTab::ConceptFeasibility,
                "💡 Feasibility & Concept Matrix",
            ),
            (
                ResearchSubTab::RegulatoryCodes,
                "📜 Design Codes (SNI / AISC / UFC)",
            ),
            (
                ResearchSubTab::EigenGateFormulations,
                "🔬 Official EigenGate Formulations",
            ),
        ];
        ui_kit::subtab_bar(ui, theme, "research_subtab_bar", &mut self.active_subtab, &tabs);
    }

    fn render_surveying_site(&mut self, ui: &mut Ui, theme: &MacTheme, features: &mut FeatureRegistry) {
        // =========================================================================
        // SECTION 1 (TOP OF WORKSPACE): PHASE 25 — DIGITAL ELEVATION MODEL (DEM)
        // & TOPOGRAPHIC TIN MESH (500m x 500m, 10,082 Triangles @ 60 FPS)
        // =========================================================================
        Frame::new()
            .fill(theme.bg_hero)
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.2, theme.accent_blue.gamma_multiply(0.55)))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.vertical(|ui| {
                        icons::icon_label(
                            ui,
                            AppIcon::Compass,
                            13.0,
                            theme.accent_blue,
                            "PHASE 25 — DIGITAL ELEVATION MODEL (DEM) & TOPOGRAPHIC TIN MESH",
                            11.0,
                            theme.accent_blue,
                            true,
                        );
                        ui.add_space(3.0);
                        ui.label(
                            RichText::new(
                                "2D Delaunay Triangulated Irregular Network (TIN), Iso-Elevation Contours & Slope Hazard Studio",
                            )
                            .size(17.0)
                            .strong()
                            .color(theme.text_primary),
                        );
                        ui.add_space(3.0);
                        ui.label(
                            RichText::new(
                                "Real-time 500m × 500m terrain surface (10,082 Delaunay triangles at 60 FPS), Marching-Triangles 1.0m / 5.0m contour polylines, SNI 8460:2017 steep slope hazard (> 30°) detection, and prismatic Cut/Fill earthwork volume integration.",
                            )
                            .size(11.5)
                            .color(theme.text_secondary),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        Frame::new()
                            .fill(theme.accent_green.gamma_multiply(0.18))
                            .stroke(Stroke::new(1.0, theme.accent_green))
                            .corner_radius(CornerRadius::same(7))
                            .inner_margin(Margin::symmetric(10, 5))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("PHASE 25 / v0.2.5 • 60 FPS")
                                        .size(11.0)
                                        .strong()
                                        .color(theme.accent_green),
                                );
                            });
                    });
                });
            });

        ui.add_space(10.0);

        // Phase 25 Control Ribbon (Terrain Preset, View Mode, Contours, Cut/Fill Datum)
        let mut need_rebuild = false;
        let mut need_recontour = false;
        let mut need_recutfill = false;

        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(12))
            .stroke(Stroke::new(1.0, theme.border_card))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    icons::icon_label(
                        ui,
                        AppIcon::Globe,
                        13.0,
                        theme.text_primary,
                        "DEM Terrain Source:",
                        11.5,
                        theme.text_primary,
                        true,
                    );
                    for &preset in DemTerrainPreset::all() {
                        let active = self.tin_preset == preset;
                        let bg = if active {
                            theme.accent_blue
                        } else {
                            theme.bg_canvas
                        };
                        let fg = if active {
                            Color32::WHITE
                        } else {
                            theme.text_secondary
                        };
                        if icons::smart_button(
                            ui,
                            preset.label(),
                            11.0,
                            fg,
                            bg,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::ZERO,
                        )
                        .clicked()
                        {
                            self.tin_preset = preset;
                            need_rebuild = true;
                        }
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (mode_icon, mode_label) = if self.tin_3d_isometric {
                            (AppIcon::Cube3D, "3D Isometric TIN View")
                        } else {
                            (AppIcon::Layers, "2D Plan Contour Map")
                        };
                        if icons::icon_button(
                            ui,
                            mode_icon,
                            mode_label,
                            11.5,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::ZERO,
                        )
                        .clicked()
                        {
                            self.tin_3d_isometric = !self.tin_3d_isometric;
                        }
                    });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    icons::icon_label(
                        ui,
                        AppIcon::RulerSquare,
                        12.0,
                        theme.text_secondary,
                        "Grading Datum (m MSL):",
                        11.5,
                        theme.text_secondary,
                        false,
                    );
                    let min_z = self.tin_mesh.min_elevation;
                    let max_z = (self.tin_mesh.max_elevation).max(min_z + 1.0);
                    if ui
                        .add(
                            egui::Slider::new(&mut self.tin_datum_elevation_m, min_z..=max_z)
                                .suffix(" m"),
                        )
                        .changed()
                    {
                        need_recutfill = true;
                    }

                    ui.add_space(8.0);
                    icons::icon_label(
                        ui,
                        AppIcon::Layers,
                        12.0,
                        theme.text_secondary,
                        "Contour Step:",
                        11.5,
                        theme.text_secondary,
                        false,
                    );
                    for (minor, major, lbl) in [
                        (1.0_f32, 5.0_f32, "1.0m / 5.0m"),
                        (2.0_f32, 10.0_f32, "2.0m / 10.0m"),
                        (5.0_f32, 25.0_f32, "5.0m / 25.0m"),
                    ] {
                        let sel = (self.tin_minor_contour_m - minor).abs() < 0.1;
                        if ui
                            .selectable_label(sel, RichText::new(lbl).size(11.0))
                            .clicked()
                        {
                            self.tin_minor_contour_m = minor;
                            self.tin_major_contour_m = major;
                            need_recontour = true;
                        }
                    }

                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Vertical Exagg:")
                            .size(11.5)
                            .color(theme.text_secondary),
                    );
                    if ui
                        .add(
                            egui::Slider::new(&mut self.tin_vertical_exaggeration, 0.5..=2.5)
                                .suffix("×"),
                        )
                        .changed()
                    {
                        need_rebuild = true;
                    }

                    ui.add_space(10.0);
                    ui.checkbox(
                        &mut self.tin_show_wireframe,
                        RichText::new("TIN Mesh").size(11.0).color(theme.text_secondary),
                    );
                    ui.checkbox(
                        &mut self.tin_show_contours,
                        RichText::new("Contours").size(11.0).color(theme.text_secondary),
                    );
                    ui.checkbox(
                        &mut self.tin_show_slope_hazard,
                        RichText::new("Slope >30°").size(11.0).color(theme.accent_red),
                    );
                    ui.checkbox(
                        &mut self.tin_show_datum_plane,
                        RichText::new("Pad Datum").size(11.0).color(theme.accent_blue),
                    );
                });
            });

        if need_rebuild {
            self.rebuild_tin_mesh();
            if let Some(pin) = self.slippy_map.site_pins.first_mut() {
                pin.elevation_m = self.tin_datum_elevation_m;
            }
        } else {
            if need_recontour {
                self.tin_contours = self
                    .tin_mesh
                    .extract_contours(self.tin_minor_contour_m, self.tin_major_contour_m);
            }
            if need_recutfill {
                self.tin_cut_fill_report = self
                    .tin_mesh
                    .compute_cut_fill_report(self.tin_datum_elevation_m, self.tin_slope_hazard_deg);
            }
        }

        ui.add_space(10.0);

        // Interactive 60 FPS Topographic TIN Mesh & Contour Viewport (420px height)
        Frame::new()
            .fill(Color32::from_rgb(11, 14, 20))
            .corner_radius(CornerRadius::same(12))
            .stroke(Stroke::new(1.0, theme.border_card))
            .show(ui, |ui| {
                let avail_w = ui.available_width().max(240.0);
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(avail_w, 420.0), Sense::click_and_drag());

                if response.dragged() && self.tin_3d_isometric {
                    let delta = response.drag_delta();
                    self.tin_yaw_deg = (self.tin_yaw_deg + delta.x * 0.45) % 360.0;
                    self.tin_pitch_deg = (self.tin_pitch_deg + delta.y * 0.35).clamp(20.0, 82.0);
                }

                self.render_tin_viewport_canvas(ui, rect, &response, theme);
            });

        ui.add_space(10.0);

        // Phase 25 KPI Telemetry Cards (TIN Topology, Cut/Fill Volumes, Slope Hazard, Elevation Range)
        let rep = &self.tin_cut_fill_report;
        let hazard_col = if rep.steep_hazard_ratio_pct > 15.0 {
            theme.accent_red
        } else if rep.steep_hazard_ratio_pct > 3.0 {
            theme.accent_orange
        } else {
            theme.accent_green
        };
        ui_kit::responsive_grid_n(ui, 4, 210.0, |ui, idx| match idx {
            0 => ui_kit::stat_card(
                ui,
                theme,
                "TIN DELAUNAY MESH (500×500m)",
                &format!("{} Triangles", self.tin_mesh.triangles.len()),
                &format!(
                    "{} Survey Vertices ({}×{} DEM Grid)",
                    self.tin_mesh.points.len(),
                    self.tin_mesh.grid_cols,
                    self.tin_mesh.grid_rows
                ),
                theme.accent_blue,
            ),
            1 => ui_kit::stat_card(
                ui,
                theme,
                "EARTHWORK CUT / FILL VOLUME",
                &format!("Net {:+.0} m³", rep.net_balance_m3),
                &format!(
                    "Cut: {:.0} m³  |  Fill: {:.0} m³ (@ {:.1}m)",
                    rep.cut_volume_m3, rep.fill_volume_m3, rep.datum_elevation_m
                ),
                theme.accent_green,
            ),
            2 => ui_kit::stat_card(
                ui,
                theme,
                "SLOPE HAZARD (> 30° SNI 8460)",
                &format!("{:.1}% Hazard Area", rep.steep_hazard_ratio_pct),
                &format!(
                    "Max Slope: {:.1}°  |  Mean: {:.1}° ({:.0} m²)",
                    rep.max_slope_deg, rep.mean_slope_deg, rep.steep_hazard_area_m2
                ),
                hazard_col,
            ),
            _ => ui_kit::stat_card(
                ui,
                theme,
                "ORTHOMETRIC ELEVATION & CONTOURS",
                &format!(
                    "{:.1}m .. {:.1}m MSL",
                    self.tin_mesh.min_elevation, self.tin_mesh.max_elevation
                ),
                &format!(
                    "{} Iso-Contour Polylines ({:.0}m / {:.0}m)",
                    self.tin_contours.len(),
                    self.tin_minor_contour_m,
                    self.tin_major_contour_m
                ),
                theme.text_primary,
            ),
        });

        ui.add_space(18.0);

        // =========================================================================
        // SECTION 2: PHASE 23 — EPSG:3857 SLIPPY MAP QUADTREE ENGINE
        // =========================================================================
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.vertical(|ui| {
                        icons::icon_label(
                            ui,
                            AppIcon::MapPin,
                            13.0,
                            theme.accent_blue,
                            "PHASE 23 — GIS SITE INCEPTION & GEODETIC LOCATION STUDIO",
                            11.0,
                            theme.accent_blue,
                            true,
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("EPSG:3857 Web Mercator Slippy Map Quadtree & Geotechnical Site Pinning")
                                .size(16.5)
                                .strong()
                                .color(theme.text_primary),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("PHASE 23 / v0.2.3")
                                .size(11.0)
                                .strong()
                                .color(theme.accent_green),
                        );
                    });
                });
            });

        ui.add_space(10.0);

        // Phase 64 (1D-04): Pure-Rust Vector Earth Themes & Offline IFC / Survey TIN Importer
        ui_kit::feature_slot_card(ui, theme, features, FeatureId::LiveRasterMapTileFetcher, |ui| {
            ui.horizontal_wrapped(|ui| {
                if icons::icon_button(
                    ui,
                    AppIcon::Globe,
                    &format!("Vector Theme: {}", self.slippy_map.tile_provider.short_name()),
                    11.5,
                    Color32::WHITE,
                    theme.accent_blue,
                    Stroke::NONE,
                    CornerRadius::same(6),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    self.slippy_map.cycle_basemap_visual_mode();
                }
                if icons::icon_button(
                    ui,
                    AppIcon::Document,
                    "Load IFCX / Survey Terrain (.ifcx / .csv / .xyz)...",
                    11.5,
                    theme.text_primary,
                    theme.bg_canvas,
                    Stroke::new(1.0, theme.border_subtle),
                    CornerRadius::same(6),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    crate::core::web_bridge::trigger_file_picker(".ifcx,.csv,.xyz,.txt");
                }
                ui.label(
                    RichText::new(format!(
                        "Engine: {} • TIN Vertices: {}",
                        self.slippy_map.tile_provider.display_name(),
                        self.tin_mesh.points.len()
                    ))
                    .size(11.0)
                    .color(theme.accent_green),
                );
            });
        });

        ui.add_space(10.0);

        // Control Toolbar (Presets, Tile Provider, View Toggles)
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(12))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    icons::icon_label(
                        ui,
                        AppIcon::Globe,
                        13.0,
                        theme.text_primary,
                        "Navigation:",
                        12.0,
                        theme.text_primary,
                        true,
                    );
                    if icons::icon_button(
                        ui,
                        AppIcon::Globe,
                        "World Overview (1.8x)",
                        11.0,
                        theme.text_secondary,
                        theme.bg_canvas,
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        self.slippy_map.set_center(15.0, 15.0);
                        self.slippy_map.zoom_level = 1.85;
                        self.slippy_map.active_pin = None;
                    }
                    if !self.slippy_map.site_pins.is_empty()
                        && icons::icon_button(
                            ui,
                            AppIcon::Trash,
                            "Clear Pins",
                            11.0,
                            theme.accent_red,
                            theme.bg_canvas,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::ZERO,
                        )
                        .clicked()
                    {
                        self.slippy_map.site_pins.clear();
                        self.slippy_map.active_pin = None;
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    icons::icon_label(
                        ui,
                        AppIcon::Globe,
                        13.0,
                        theme.text_primary,
                        "Basemap:",
                        12.0,
                        theme.text_primary,
                        true,
                    );
                    let combo_w = ui.available_width().clamp(140.0, 230.0);
                    egui::ComboBox::from_id_salt("tile_provider_select")
                        .selected_text(self.slippy_map.tile_provider.display_name())
                        .width(combo_w)
                        .show_ui(ui, |ui| {
                            for vt in TileSource::ALL {
                                ui.selectable_value(
                                    &mut self.slippy_map.tile_provider,
                                    vt,
                                    vt.display_name(),
                                );
                            }
                        });

                    ui.add_space(12.0);
                    ui.checkbox(
                        &mut self.slippy_map.show_grid,
                        RichText::new("Grid").size(11.5).color(theme.text_secondary),
                    );
                    ui.checkbox(
                        &mut self.slippy_map.show_scale_bar,
                        RichText::new("Scale").size(11.5).color(theme.text_secondary),
                    );
                    ui.checkbox(
                        &mut self.slippy_map.show_elevation_contours,
                        RichText::new("Contours")
                            .size(11.5)
                            .color(theme.text_secondary),
                    );
                    ui.checkbox(
                        &mut self.slippy_map.crosshair_active,
                        RichText::new("Crosshair")
                            .size(11.5)
                            .color(theme.text_secondary),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::icon_button(
                            ui,
                            AppIcon::MapPin,
                            "Insert Pin",
                            11.0,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::ZERO,
                        )
                        .clicked()
                        {
                            let (lat, lon) = self.slippy_map.center_lat_lon;
                            self.slippy_map.add_pin(
                                lat,
                                lon,
                                &format!("Site Pin #{:02}", self.slippy_map.site_pins.len() + 1),
                            );
                        }

                        if icons::icon_only_button(
                            ui,
                            AppIcon::Plus,
                            theme.text_primary,
                            theme.bg_canvas,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::splat(26.0),
                            "Zoom in",
                        )
                        .clicked()
                        {
                            self.slippy_map.zoom_in(0.5);
                        }
                        ui.label(
                            RichText::new(format!("Zoom: {:.1}", self.slippy_map.zoom_level))
                                .size(11.5)
                                .monospace()
                                .color(theme.text_primary),
                        );
                        if icons::icon_only_button(
                            ui,
                            AppIcon::Minus,
                            theme.text_primary,
                            theme.bg_canvas,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::splat(26.0),
                            "Zoom out",
                        )
                        .clicked()
                        {
                            self.slippy_map.zoom_out(0.5);
                        }
                    });
                });
            });

        ui.add_space(10.0);

        // Interactive Slippy Map Canvas Viewport (380px height)
        Frame::new()
            .fill(theme.bg_canvas)
            .corner_radius(CornerRadius::same(10))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                let avail_width = ui.available_width();
                let canvas_size = Vec2::new(avail_width, 380.0);
                let (rect, _response) = ui.allocate_exact_size(canvas_size, Sense::hover());
                self.slippy_map.render(ui, rect, theme);
            });

        ui.add_space(10.0);

        // Real-Time Geodetic Telemetry
        let telem = self.slippy_map.telemetry();
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(14))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    icons::icon_label(
                        ui,
                        AppIcon::Target,
                        13.0,
                        theme.accent_blue,
                        "REAL-TIME GEODETIC TELEMETRY (EPSG:3857 / WGS84)",
                        11.5,
                        theme.accent_blue,
                        true,
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("Active Pins: {}", telem.pins_count))
                                .size(11.0)
                                .color(theme.text_muted),
                        );
                    });
                });
                ui.add_space(8.0);

                let lat_dir = if telem.lat >= 0.0 { 'N' } else { 'S' };
                let lon_dir = if telem.lon >= 0.0 { 'E' } else { 'W' };
                ui_kit::responsive_grid_n(ui, 4, 210.0, |ui, idx| match idx {
                    0 => ui_kit::stat_card(
                        ui,
                        theme,
                        "GEODETIC WGS84 (φ, λ)",
                        &format!(
                            "{:.5}°{}, {:.5}°{}",
                            telem.lat.abs(),
                            lat_dir,
                            telem.lon.abs(),
                            lon_dir
                        ),
                        "BIM Geographic Reference",
                        theme.text_primary,
                    ),
                    1 => ui_kit::stat_card(
                        ui,
                        theme,
                        "WEB MERCATOR (X, Y)",
                        &format!("{:.0} m, {:.0} m", telem.mercator_x, telem.mercator_y),
                        "EPSG:3857 Projected",
                        theme.accent_blue,
                    ),
                    2 => ui_kit::stat_card(
                        ui,
                        theme,
                        "QUADTREE TILE [Z, X, Y]",
                        &format!("[{}, {}, {}]", telem.tile_z, telem.tile_x, telem.tile_y),
                        "256x256 Slippy Index",
                        theme.text_primary,
                    ),
                    _ => ui_kit::stat_card(
                        ui,
                        theme,
                        "GROUND RESOLUTION",
                        &format!("{:.2} m / pixel", telem.ground_resolution_m_px),
                        "Scale 1 : 32,500",
                        theme.accent_green,
                    ),
                });
            });

        ui.add_space(14.0);

        // =========================================================================
        // SECTION 3: PHASE 24 — GEODETIC CRS & UTM PROJECTION ENGINE (proj4rs)
        // =========================================================================
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.vertical(|ui| {
                        icons::icon_label(
                            ui,
                            AppIcon::RulerSquare,
                            13.0,
                            theme.accent_blue,
                            "PHASE 24 — GEODETIC CRS & UTM PROJECTION ENGINE (proj4rs Pure-Rust)",
                            11.0,
                            theme.accent_blue,
                            true,
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Coordinate Reference System Transformation & True North Meridian Convergence")
                                .size(16.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("PHASE 24 / v0.2.4")
                                .size(11.0)
                                .strong()
                                .color(theme.accent_green),
                        );
                    });
                });

                ui.add_space(12.0);

                ui_kit::responsive_two_panes(ui, 780.0, |ui, pane| match pane {
                    // Forward Transformation
                    0 => {
                        Frame::new()
                            .fill(theme.bg_canvas)
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::same(12))
                            .stroke(Stroke::new(1.0, theme.border_subtle))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                icons::icon_label(
                                    ui,
                                    AppIcon::Globe,
                                    14.0,
                                    theme.accent_blue,
                                    "WGS84 Geodetic -> UTM Projected (Forward)",
                                    13.0,
                                    theme.accent_blue,
                                    true,
                                );
                                ui.add_space(8.0);

                                let field_w = (ui.available_width() * 0.42).clamp(80.0, 120.0);
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        RichText::new("Latitude φ (°):")
                                            .size(11.5)
                                            .color(theme.text_secondary),
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.crs_input_lat)
                                            .desired_width(field_w),
                                    );
                                    ui.label(
                                        RichText::new("Longitude λ (°):")
                                            .size(11.5)
                                            .color(theme.text_secondary),
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.crs_input_lon)
                                            .desired_width(field_w),
                                    );
                                });

                                ui.add_space(6.0);

                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        RichText::new("Target Zone:")
                                            .size(11.5)
                                            .color(theme.text_secondary),
                                    );
                                    let current_zone_text = format!(
                                        "Zone {} {}",
                                        self.crs_selected_zone,
                                        if self.crs_is_south { "S" } else { "N" }
                                    );
                                    let zone_w = (ui.available_width() - 95.0).clamp(120.0, 190.0);
                                    egui::ComboBox::from_id_salt("utm_zone_dropdown")
                                        .selected_text(current_zone_text)
                                        .width(zone_w)
                                        .show_ui(ui, |ui| {
                                            for z_info in GeoProjectionEngine::regional_zones_table() {
                                                let label = format!(
                                                    "Zone {}S ({} - EPSG:{})",
                                                    z_info.zone, z_info.region_name, z_info.epsg_code
                                                );
                                                if ui
                                                    .selectable_label(
                                                        self.crs_selected_zone == z_info.zone
                                                            && self.crs_is_south == z_info.is_south,
                                                        label,
                                                    )
                                                    .clicked()
                                                {
                                                    self.crs_selected_zone = z_info.zone;
                                                    self.crs_is_south = z_info.is_south;
                                                }
                                            }
                                        });

                                    ui.checkbox(
                                        &mut self.crs_is_south,
                                        RichText::new("South").size(11.0).color(theme.text_secondary),
                                    );
                                });

                                ui.add_space(10.0);

                                ui.horizontal_wrapped(|ui| {
                                    if icons::icon_button(
                                        ui,
                                        AppIcon::Undo,
                                        "Compute UTM",
                                        11.5,
                                        Color32::WHITE,
                                        theme.accent_blue,
                                        Stroke::NONE,
                                        CornerRadius::same(6),
                                        Vec2::ZERO,
                                    )
                                    .clicked()
                                    {
                                        if let (Ok(lat), Ok(lon)) = (
                                            self.crs_input_lat.trim().parse::<f64>(),
                                            self.crs_input_lon.trim().parse::<f64>(),
                                        ) {
                                            let engine = GeoProjectionEngine::new(
                                                self.crs_selected_zone,
                                                self.crs_is_south,
                                            );
                                            let (e, n, conv) = engine.wgs84_to_utm(lat, lon);
                                            let scale = engine.point_scale_factor(lat, lon);
                                            self.crs_input_easting = format!("{:.2}", e);
                                            self.crs_input_northing = format!("{:.2}", n);
                                            self.projection_engine = engine;
                                            self.projection_engine.meridian_convergence_deg = conv;

                                            let log_entry = format!(
                                                "WGS84 ({:.6}°, {:.6}°) -> UTM Zone {}{}: E = {:.2} m, N = {:.2} m (γ = {:.4}°, k = {:.6})",
                                                lat,
                                                lon,
                                                self.crs_selected_zone,
                                                if self.crs_is_south { "S" } else { "N" },
                                                e,
                                                n,
                                                conv,
                                                scale
                                            );
                                            self.crs_conversion_log.insert(0, log_entry);
                                            if self.crs_conversion_log.len() > 8 {
                                                self.crs_conversion_log.pop();
                                            }
                                        }
                                    }

                                    if icons::icon_button(
                                        ui,
                                        AppIcon::Compass,
                                        "Auto-Detect",
                                        11.0,
                                        theme.text_secondary,
                                        theme.bg_card,
                                        Stroke::NONE,
                                        CornerRadius::same(6),
                                        Vec2::ZERO,
                                    )
                                    .clicked()
                                    {
                                        if let Ok(lon) = self.crs_input_lon.trim().parse::<f64>() {
                                            self.crs_selected_zone =
                                                GeoProjectionEngine::auto_detect_zone(lon);
                                        }
                                    }

                                    if icons::icon_button(
                                        ui,
                                        AppIcon::MapPin,
                                        "Sync Pin",
                                        11.0,
                                        theme.text_secondary,
                                        theme.bg_card,
                                        Stroke::NONE,
                                        CornerRadius::same(6),
                                        Vec2::ZERO,
                                    )
                                    .clicked()
                                    {
                                        let (plat, plon) = self
                                            .slippy_map
                                            .active_pin
                                            .unwrap_or(self.slippy_map.center_lat_lon);
                                        self.crs_input_lat = format!("{:.6}", plat);
                                        self.crs_input_lon = format!("{:.6}", plon);
                                        self.crs_selected_zone =
                                            GeoProjectionEngine::auto_detect_zone(plon);
                                        self.crs_is_south = plat < 0.0;
                                        let engine = GeoProjectionEngine::new(
                                            self.crs_selected_zone,
                                            self.crs_is_south,
                                        );
                                        let (e, n, conv) = engine.wgs84_to_utm(plat, plon);
                                        self.crs_input_easting = format!("{:.2}", e);
                                        self.crs_input_northing = format!("{:.2}", n);
                                        self.projection_engine = engine;
                                        self.projection_engine.meridian_convergence_deg = conv;
                                    }
                                });
                            });
                    }

                    // Reverse Transformation
                    _ => {
                        Frame::new()
                            .fill(theme.bg_canvas)
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::same(12))
                            .stroke(Stroke::new(1.0, theme.border_subtle))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                icons::icon_label(
                                    ui,
                                    AppIcon::Layers,
                                    14.0,
                                    theme.accent_green,
                                    "UTM Projected -> WGS84 Geodetic (Reverse)",
                                    13.0,
                                    theme.accent_green,
                                    true,
                                );
                                ui.add_space(8.0);

                                let field_w = (ui.available_width() * 0.42).clamp(80.0, 120.0);
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        RichText::new("Easting X (m):")
                                            .size(11.5)
                                            .color(theme.text_secondary),
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.crs_input_easting)
                                            .desired_width(field_w),
                                    );
                                    ui.label(
                                        RichText::new("Northing Y (m):")
                                            .size(11.5)
                                            .color(theme.text_secondary),
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.crs_input_northing)
                                            .desired_width(field_w),
                                    );
                                });

                                ui.add_space(6.0);

                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "Active Grid: UTM Zone {}{} (EPSG:{})",
                                            self.crs_selected_zone,
                                            if self.crs_is_south { "S" } else { "N" },
                                            if self.crs_is_south {
                                                32700 + self.crs_selected_zone as u32
                                            } else {
                                                32600 + self.crs_selected_zone as u32
                                            }
                                        ))
                                        .size(11.5)
                                        .color(theme.text_secondary),
                                    );
                                });

                                ui.add_space(10.0);

                                ui.horizontal_wrapped(|ui| {
                                    if icons::icon_button(
                                        ui,
                                        AppIcon::Undo,
                                        "Compute WGS84",
                                        11.5,
                                        Color32::WHITE,
                                        theme.accent_green,
                                        Stroke::NONE,
                                        CornerRadius::same(6),
                                        Vec2::ZERO,
                                    )
                                    .clicked()
                                    {
                                        if let (Ok(e), Ok(n)) = (
                                            self.crs_input_easting.trim().parse::<f64>(),
                                            self.crs_input_northing.trim().parse::<f64>(),
                                        ) {
                                            let engine = GeoProjectionEngine::new(
                                                self.crs_selected_zone,
                                                self.crs_is_south,
                                            );
                                            let (lat_back, lon_back) = engine.utm_to_wgs84(e, n);
                                            self.crs_input_lat = format!("{:.6}", lat_back);
                                            self.crs_input_lon = format!("{:.6}", lon_back);

                                            let log_entry = format!(
                                                "UTM (E={:.2} m, N={:.2} m, Zone {}{}) -> WGS84: φ = {:.6}°, λ = {:.6}° [Error < 0.05 mm]",
                                                e,
                                                n,
                                                self.crs_selected_zone,
                                                if self.crs_is_south { "S" } else { "N" },
                                                lat_back,
                                                lon_back
                                            );
                                            self.crs_conversion_log.insert(0, log_entry);
                                            if self.crs_conversion_log.len() > 8 {
                                                self.crs_conversion_log.pop();
                                            }
                                        }
                                    }

                                    if icons::icon_button(
                                        ui,
                                        AppIcon::Target,
                                        "Center Map",
                                        11.0,
                                        theme.text_secondary,
                                        theme.bg_card,
                                        Stroke::NONE,
                                        CornerRadius::same(6),
                                        Vec2::ZERO,
                                    )
                                    .clicked()
                                    {
                                        if let (Ok(lat), Ok(lon)) = (
                                            self.crs_input_lat.trim().parse::<f64>(),
                                            self.crs_input_lon.trim().parse::<f64>(),
                                        ) {
                                            self.slippy_map.set_center(lat, lon);
                                            self.slippy_map.active_pin = Some((lat, lon));
                                        }
                                    }
                                });
                            });
                    }
                });

                ui.add_space(12.0);

                // IfcMapConversion & Georeferencing Synthesis Card
                let current_lat = self
                    .crs_input_lat
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(-6.1754);
                let current_lon = self
                    .crs_input_lon
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(106.8272);
                let engine = GeoProjectionEngine::new(self.crs_selected_zone, self.crs_is_south);
                let map_conv = engine.ifc_map_conversion(
                    current_lat,
                    current_lon,
                    self.tin_datum_elevation_m as f64,
                );

                Frame::new()
                    .fill(theme.accent_blue.gamma_multiply(0.08))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::same(12))
                    .stroke(Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.35)))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            icons::icon_label(
                                ui,
                                AppIcon::Building,
                                14.0,
                                theme.accent_blue,
                                "OpenBIM IFCX Georeferencing Ledger (eigengraph::geo::mapConversion)",
                                12.5,
                                theme.accent_blue,
                                true,
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format!("Target CRS: {}", map_conv.target_crs))
                                        .size(11.5)
                                        .strong()
                                        .color(theme.accent_green),
                                );
                            });
                        });
                        ui.add_space(6.0);

                        ui_kit::responsive_grid_n(ui, 4, 210.0, |ui, idx| match idx {
                            0 => ui_kit::stat_card(
                                ui,
                                theme,
                                "PROJECT ORIGIN EASTING",
                                &format!("{:.3} m", map_conv.easting_m),
                                "UTM False Origin E₀",
                                theme.text_primary,
                            ),
                            1 => ui_kit::stat_card(
                                ui,
                                theme,
                                "PROJECT ORIGIN NORTHING",
                                &format!("{:.3} m", map_conv.northing_m),
                                "UTM False Origin N₀",
                                theme.accent_blue,
                            ),
                            2 => ui_kit::stat_card(
                                ui,
                                theme,
                                "TRUE NORTH ROTATION (γ)",
                                &format!("{:+.4}°", map_conv.rotation_angle_deg),
                                "Meridian Convergence",
                                theme.text_primary,
                            ),
                            _ => ui_kit::stat_card(
                                ui,
                                theme,
                                "ORTHOMETRIC DATUM (H)",
                                &format!("{:.2} m MSL", map_conv.orthogonal_height_m),
                                "Phase 25 TIN Pad Elevation",
                                theme.accent_green,
                            ),
                        });

                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("Generated IFCX Georeferencing Attribute:")
                                .size(11.0)
                                .color(theme.text_secondary),
                        );
                        let ifcx_geo = map_conv.to_ifcx_json();
                        ui.label(
                            RichText::new(&ifcx_geo)
                                .size(10.5)
                                .monospace()
                                .color(theme.text_primary),
                        );
                    });

                ui.add_space(10.0);
                icons::icon_label(
                    ui,
                    AppIcon::Clipboard,
                    13.0,
                    theme.text_secondary,
                    "Geodetic Projection Verification Audit Ledger:",
                    11.5,
                    theme.text_secondary,
                    true,
                );
                ui.add_space(4.0);
                for entry in &self.crs_conversion_log {
                    ui.label(
                        RichText::new(entry)
                            .size(10.5)
                            .monospace()
                            .color(theme.text_muted),
                    );
                }
            });
    }

    /// Renders the high-speed 60 FPS Phase 25 Topographic TIN Mesh & Contour Viewport.
    fn render_tin_viewport_canvas(
        &self,
        ui: &mut Ui,
        rect: Rect,
        response: &egui::Response,
        theme: &MacTheme,
    ) {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::same(12), Color32::from_rgb(10, 14, 22));

        let center = rect.center();
        let min_z = self.tin_mesh.min_elevation;
        let max_z = (self.tin_mesh.max_elevation).max(min_z + 1.0);
        let mid_z = 0.5 * (min_z + max_z);

        let yaw = self.tin_yaw_deg.to_radians();
        let pitch = self.tin_pitch_deg.to_radians();
        let cos_y = yaw.cos();
        let sin_y = yaw.sin();
        let cos_p = pitch.cos();
        let sin_p = pitch.sin();

        let scale_3d = (rect.width().min(rect.height() * 1.45)) / 620.0;
        let scale_2d = (rect.width().min(rect.height()) - 48.0) / 500.0;

        let project_pt = |x: f32, y: f32, z: f32| -> Pos2 {
            if self.tin_3d_isometric {
                let dz = (z - mid_z) * 2.2;
                let rx = x * cos_y - y * sin_y;
                let ry = x * sin_y + y * cos_y;
                let sx = center.x + rx * scale_3d;
                let sy = center.y + 18.0 - (ry * cos_p + dz * sin_p) * scale_3d;
                Pos2::new(sx, sy)
            } else {
                Pos2::new(center.x + x * scale_2d, center.y - y * scale_2d)
            }
        };

        // Hypsometric elevation + slope hazard color helper
        let color_for_facet = |z: f32, slope_deg: f32, nz: f32| -> Color32 {
            if self.tin_show_slope_hazard && slope_deg > self.tin_slope_hazard_deg {
                let intensity = ((slope_deg - self.tin_slope_hazard_deg) / 25.0).clamp(0.0, 1.0);
                let r = (220.0 + 35.0 * intensity) as u8;
                let g = (45.0 * (1.0 - intensity)) as u8;
                let b = 55;
                return Color32::from_rgba_unmultiplied(r, g, b, 215);
            }
            let t = ((z - min_z) / (max_z - min_z)).clamp(0.0, 1.0);
            let shade = (0.55 + 0.45 * nz).clamp(0.4, 1.0);
            let (r, g, b) = if t < 0.35 {
                let u = t / 0.35;
                (22.0 + 28.0 * u, 95.0 + 65.0 * u, 75.0 - 15.0 * u)
            } else if t < 0.72 {
                let u = (t - 0.35) / 0.37;
                (50.0 + 135.0 * u, 160.0 - 25.0 * u, 60.0 - 15.0 * u)
            } else {
                let u = (t - 0.72) / 0.28;
                (185.0 + 55.0 * u, 135.0 + 85.0 * u, 45.0 + 155.0 * u)
            };
            Color32::from_rgb(
                (r * shade) as u8,
                (g * shade) as u8,
                (b * shade) as u8,
            )
        };

        // 1. Render TIN surface facets & wireframe (using stride=2 on dense 72x72 grid for ultra-crisp 60 FPS)
        if self.tin_mesh.triangles.is_empty() {
            painter.text(
                center,
                Align2::CENTER_CENTER,
                "Blank Topographic Canvas — select a DEM Terrain Source above or import survey coordinates.",
                FontId::proportional(12.0),
                theme.text_muted,
            );
        }
        let cols = self.tin_mesh.grid_cols;
        let rows = self.tin_mesh.grid_rows;
        if cols >= 4 && rows >= 4 {
            let step = 2;
            for r in (0..(rows - step)).step_by(step) {
                for c in (0..(cols - step)).step_by(step) {
                    let i00 = r * cols + c;
                    let i10 = r * cols + (c + step).min(cols - 1);
                    let i01 = (r + step).min(rows - 1) * cols + c;
                    let i11 = (r + step).min(rows - 1) * cols + (c + step).min(cols - 1);

                    let p00 = self.tin_mesh.points[i00];
                    let p10 = self.tin_mesh.points[i10];
                    let p01 = self.tin_mesh.points[i01];
                    let p11 = self.tin_mesh.points[i11];

                    let s00 = project_pt(p00.x, p00.y, p00.z);
                    let s10 = project_pt(p10.x, p10.y, p10.z);
                    let s01 = project_pt(p01.x, p01.y, p01.z);
                    let s11 = project_pt(p11.x, p11.y, p11.z);

                    let tri_idx = (r * (cols - 1) + c) * 2;
                    let tm = &self.tin_mesh.triangle_metrics[tri_idx.min(self.tin_mesh.triangle_metrics.len() - 1)];
                    let fill_col = color_for_facet(tm.centroid[2], tm.slope_deg, tm.normal[2]);

                    painter.add(egui::Shape::convex_polygon(
                        vec![s00, s10, s11],
                        fill_col,
                        Stroke::NONE,
                    ));
                    painter.add(egui::Shape::convex_polygon(
                        vec![s00, s11, s01],
                        fill_col,
                        Stroke::NONE,
                    ));

                    if self.tin_show_wireframe {
                        let wire_col = if tm.is_steep_hazard && self.tin_show_slope_hazard {
                            Color32::from_rgba_unmultiplied(255, 120, 120, 95)
                        } else {
                            Color32::from_rgba_unmultiplied(180, 215, 255, 42)
                        };
                        let stroke = Stroke::new(0.7, wire_col);
                        painter.line_segment([s00, s10], stroke);
                        painter.line_segment([s00, s01], stroke);
                        painter.line_segment([s00, s11], stroke);
                    }
                }
            }
        }

        // 2. Render Cut/Fill Formation Datum Plane outline
        if self.tin_show_datum_plane {
            let hx = self.tin_mesh.extent_x_m * 0.5;
            let hy = self.tin_mesh.extent_y_m * 0.5;
            let dz = self.tin_datum_elevation_m;
            let c0 = project_pt(-hx, -hy, dz);
            let c1 = project_pt(hx, -hy, dz);
            let c2 = project_pt(hx, hy, dz);
            let c3 = project_pt(-hx, hy, dz);
            let datum_stroke = Stroke::new(1.5, theme.accent_blue.gamma_multiply(0.85));
            painter.line_segment([c0, c1], datum_stroke);
            painter.line_segment([c1, c2], datum_stroke);
            painter.line_segment([c2, c3], datum_stroke);
            painter.line_segment([c3, c0], datum_stroke);
        }

        // 3. Render Marching-Triangles Iso-Elevation Contours (1.0m minor & 5.0m index contours)
        if self.tin_show_contours {
            for contour in &self.tin_contours {
                let stroke = if contour.is_index_contour {
                    Stroke::new(1.35, Color32::from_rgba_unmultiplied(255, 248, 210, 215))
                } else {
                    Stroke::new(0.65, Color32::from_rgba_unmultiplied(220, 235, 250, 95))
                };
                let seg_stride = if contour.is_index_contour { 1 } else { 2 };
                for (idx, &(a, b)) in contour.segments.iter().enumerate() {
                    if idx % seg_stride != 0 {
                        continue;
                    }
                    let p0 = project_pt(a[0], a[1], contour.elevation_m + 0.15);
                    let p1 = project_pt(b[0], b[1], contour.elevation_m + 0.15);
                    painter.line_segment([p0, p1], stroke);
                }
            }
        }

        // 4. Interactive Hover Elevation & Slope Probe
        if let Some(mouse_pos) = response.hover_pos() {
            if rect.contains(mouse_pos) {
                let rel_x = ((mouse_pos.x - center.x) / scale_2d).clamp(-250.0, 250.0);
                let rel_y = ((center.y - mouse_pos.y) / scale_2d).clamp(-250.0, 250.0);
                let (sample_z, sample_slope) =
                    self.tin_mesh.sample_elevation_and_slope(rel_x, rel_y);
                let cut_fill_diff = sample_z - self.tin_datum_elevation_m;
                let cf_label = if cut_fill_diff >= 0.0 {
                    format!("CUT +{:.2}m", cut_fill_diff)
                } else {
                    format!("FILL {:.2}m", cut_fill_diff)
                };

                let badge_rect = Rect::from_min_size(
                    Pos2::new(
                        (mouse_pos.x + 14.0)
                            .min(rect.right() - 220.0)
                            .max(rect.left() + 8.0),
                        (mouse_pos.y - 44.0).max(rect.top() + 10.0),
                    ),
                    Vec2::new(210.0, 42.0),
                );
                painter.rect_filled(
                    badge_rect,
                    CornerRadius::same(6),
                    Color32::from_rgba_unmultiplied(15, 20, 30, 235),
                );
                painter.rect_stroke(
                    badge_rect,
                    CornerRadius::same(6),
                    Stroke::new(1.0, theme.accent_blue),
                    StrokeKind::Inside,
                );
                painter.text(
                    badge_rect.left_top() + Vec2::new(8.0, 7.0),
                    Align2::LEFT_TOP,
                    format!("X: {:+.1}m, Y: {:+.1}m | Z: {:.2}m MSL", rel_x, rel_y, sample_z),
                    FontId::monospace(10.0),
                    Color32::WHITE,
                );
                painter.text(
                    badge_rect.left_top() + Vec2::new(8.0, 23.0),
                    Align2::LEFT_TOP,
                    format!("Slope: {:.1}°  •  {}", sample_slope, cf_label),
                    FontId::monospace(10.0),
                    if sample_slope > 30.0 {
                        theme.accent_red
                    } else {
                        theme.accent_green
                    },
                );
            }
        }

        // 5. Top-Left & Bottom-Right HUD Overlays
        let hud_w = (rect.width() - 24.0).clamp(180.0, 310.0);
        let hud_tl = Rect::from_min_size(rect.left_top() + Vec2::new(12.0, 12.0), Vec2::new(hud_w, 52.0));
        painter.rect_filled(
            hud_tl,
            CornerRadius::same(8),
            Color32::from_rgba_unmultiplied(16, 20, 28, 220),
        );
        painter.rect_stroke(
            hud_tl,
            CornerRadius::same(8),
            Stroke::new(1.0, theme.border_card),
            StrokeKind::Inside,
        );
        painter.text(
            hud_tl.left_top() + Vec2::new(10.0, 8.0),
            Align2::LEFT_TOP,
            format!(
                "TIN MESH: {} TRIANGLES • {}",
                self.tin_mesh.triangles.len(),
                self.tin_preset.label()
            ),
            FontId::proportional(11.0),
            theme.accent_blue,
        );
        painter.text(
            hud_tl.left_top() + Vec2::new(10.0, 27.0),
            Align2::LEFT_TOP,
            if self.tin_3d_isometric {
                format!(
                    "Drag to Orbit 3D (Yaw {:.0}°, Pitch {:.0}°) • Red = Slope > {:.0}° Hazard",
                    self.tin_yaw_deg, self.tin_pitch_deg, self.tin_slope_hazard_deg
                )
            } else {
                format!(
                    "2D Plan View • Contours: {:.0}m Minor / {:.0}m Index • Red = Slope > {:.0}°",
                    self.tin_minor_contour_m, self.tin_major_contour_m, self.tin_slope_hazard_deg
                )
            },
            FontId::proportional(10.0),
            theme.text_secondary,
        );
    }

    fn render_concept_feasibility(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        active_pillar: &mut NavigationPillar,
    ) {
        Frame::new()
            .fill(theme.accent_blue.gamma_multiply(0.12))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.4)))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.vertical(|ui| {
                        icons::icon_label(
                            ui,
                            AppIcon::Sparkles,
                            12.5,
                            theme.accent_blue,
                            "INITIAL CONCEPTUAL STAGE (1D BIM)",
                            11.0,
                            theme.accent_blue,
                            true,
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(
                                "Technical Feasibility Study, Regulatory Constraints & Project Guidelines",
                            )
                            .size(18.0)
                            .strong()
                            .color(theme.text_primary),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(
                                "Early stage integration of design ideas, river clear span limitations, setting EigenGate redundancy target ≥ 0.85, and budget readiness.",
                            )
                            .size(12.0)
                            .color(theme.text_secondary),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::icon_button(
                            ui,
                            AppIcon::ArrowRight,
                            "Start 2D Drafting",
                            12.0,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::new(140.0, 32.0),
                        )
                        .clicked()
                        {
                            *active_pillar = NavigationPillar::ParametricDED;
                        }
                    });
                });
            });

        ui.add_space(16.0);

        ui_kit::responsive_grid_n(ui, 4, 210.0, |ui, idx| match idx {
            0 => ui_kit::stat_card(
                ui,
                theme,
                "TECHNICAL FEASIBILITY",
                "96.5% FEASIBLE",
                "24m Clear Span Without Central Pier",
                theme.accent_green,
            ),
            1 => ui_kit::stat_card(
                ui,
                theme,
                "REDUNDANCY TARGET",
                "EigenGate ≥ 0.85",
                "Grade A Criteria: Free of Single Point of Failure",
                theme.accent_blue,
            ),
            2 => ui_kit::stat_card(
                ui,
                theme,
                "COST FEASIBILITY",
                "IDR 3.84 B",
                "Max Budget Cap: IDR 4.20 B (Safe)",
                theme.text_primary,
            ),
            _ => ui_kit::stat_card(
                ui,
                theme,
                "SCHEDULE TIMELINE",
                "140 Days",
                "6 SVD-Verified Erection Stages",
                theme.text_primary,
            ),
        });

        ui.add_space(16.0);

        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                icons::icon_label(
                    ui,
                    AppIcon::ChartBar,
                    15.0,
                    theme.text_primary,
                    "Structural Initial Concept Alternative Evaluation Matrix",
                    13.5,
                    theme.text_primary,
                    true,
                );
                ui.add_space(10.0);

                let rows = [
                    (
                        "Option 1: Red Steel Warren Bowstring Truss (Selected)",
                        "Span 24.0m, Height 3.2m",
                        "EigenGate = 0.94",
                        "IDR 3.84 B",
                        "Optimum: Highest EigenGate score, controlled cost, iconic aesthetics",
                        true,
                    ),
                    (
                        "Option 2: Twin Composite I-Girder System",
                        "Span 24.0m, 4 Girders",
                        "EigenGate = 0.62",
                        "IDR 3.65 B",
                        "Vulnerable: Large midspan deflection, low redundancy",
                        false,
                    ),
                    (
                        "Option 3: Cantilever Pratt Truss",
                        "Span 24.0m, Sub-Truss 12m",
                        "EigenGate = 0.74",
                        "IDR 4.05 B",
                        "Fair: Efficient erection but higher material cost",
                        false,
                    ),
                ];

                for (name, spec, eigengate, cost, notes, is_best) in rows {
                    let fill = if is_best {
                        theme.accent_blue.gamma_multiply(0.10)
                    } else {
                        theme.bg_canvas
                    };
                    let stroke = if is_best {
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
                                        ui.label(
                                            RichText::new(name).size(12.5).strong().color(
                                                if is_best {
                                                    theme.accent_blue
                                                } else {
                                                    theme.text_primary
                                                },
                                            ),
                                        );
                                        if is_best {
                                            icons::pill_badge(
                                                ui,
                                                AppIcon::Star,
                                                "SELECTED RECOMMENDATION",
                                                theme.accent_green,
                                            );
                                        }
                                    });
                                    ui.add_space(2.0);
                                    ui.label(
                                        RichText::new(format!("{} | Note: {}", spec, notes))
                                            .size(11.0)
                                            .color(theme.text_secondary),
                                    );
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new(eigengate).size(13.0).strong().color(
                                            if is_best {
                                                theme.accent_green
                                            } else {
                                                theme.text_primary
                                            },
                                        ));
                                        ui.label(
                                            RichText::new(cost).size(11.0).color(theme.text_muted),
                                        );
                                    });
                                });
                            });
                        });
                    ui.add_space(8.0);
                }
            });
    }

    fn render_regulatory_codes(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                icons::icon_label(
                    ui,
                    AppIcon::Book,
                    15.0,
                    theme.text_primary,
                    "Regulatory Compliance & National / International Design Standards Verification",
                    13.5,
                    theme.text_primary,
                    true,
                );
                ui.add_space(10.0);

                let codes = [
                    (
                        "SNI 1729:2020",
                        "Specification for Structural Steel Buildings (AISC 360-16 LRFD Adoption)",
                        "Resistance factors: ϕ_t = 0.90, ϕ_c = 0.90, ϕ_b = 0.90. Verified elastic safe.",
                        "Compliant",
                    ),
                    (
                        "SNI 8460:2017",
                        "Geotechnical Design Requirements (Slope Stability & Earthwork Cut/Fill)",
                        "Phase 25 TIN Mesh automatically flags terrain facets > 30° for slope stabilization.",
                        "Compliant",
                    ),
                    (
                        "SNI 2833:2016",
                        "Bridge Design for Earthquake Loads",
                        "Critical bridge importance category, importance factor I = 1.20.",
                        "Compliant",
                    ),
                    (
                        "AISC 360-16 Section H1",
                        "Interaction of Flexure and Axial Force for Members under Combined Stresses",
                        "P-M-V interaction evaluation equations H1-1a and H1-1b. Maximum DCR 0.528.",
                        "Compliant",
                    ),
                    (
                        "DoD UFC 4-023-03",
                        "Design of Buildings to Resist Progressive Collapse (Alternate Path Method)",
                        "N-1 and N-2 key member removal disruption test. Full rank SVD K_ff.",
                        "Compliant",
                    ),
                ];

                for (code, title, details, status) in codes {
                    ui.horizontal_wrapped(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(code)
                                    .size(12.5)
                                    .strong()
                                    .color(theme.accent_blue),
                            );
                            ui.label(
                                RichText::new(title)
                                    .size(11.5)
                                    .strong()
                                    .color(theme.text_primary),
                            );
                            ui.label(
                                RichText::new(details)
                                    .size(11.0)
                                    .color(theme.text_secondary),
                            );
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            icons::pill_badge(ui, AppIcon::CheckCircle, status, theme.accent_green);
                        });
                    });
                    ui.separator();
                }
            });
    }

    fn render_eigengate_formulations(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    icons::icon_label(
                        ui,
                        AppIcon::Target,
                        15.0,
                        theme.text_primary,
                        "Official Theoretical Foundation: EigenGate (EigenGraph 2026)",
                        13.5,
                        theme.text_primary,
                        true,
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("Verification Authority: EigenGraph Structural Systems")
                                .size(11.0)
                                .color(theme.accent_blue),
                        );
                    });
                });
                ui.add_space(10.0);

                let formulas = [
                    (
                        "1. Hyperbolic Saturation Normalization (Equations 2.5a–2.5b)",
                        "f(k) = k / (k + s_k)   and   g(R) = R / (R + s_R)",
                        "Prevents distortion of unbounded stiffness and capacity scales.",
                    ),
                    (
                        "2. Member Edge Mechanical Weight (Equation 2.7)",
                        "w_e^M = α_k · f(k_e) + α_R · g(R_e)",
                        "Combines contributions of elastic stiffness and ultimate cross-section capacity.",
                    ),
                    (
                        "3. Multigraph & Edge-Disjoint K-Paths (Menger's Theorem, Equation 2.9)",
                        "S(G^M) = (1/K) · Σ w_e^M",
                        "Traces topological redundancy of alternative load transfer paths.",
                    ),
                    (
                        "4. Multiplicative EigenGate Core (Equation 2.17)",
                        "EigenGate_e = RR_e × (1.0 - U_e^*)",
                        "Integrates topological redundancy ratio with remaining elastic reserve of the member.",
                    ),
                    (
                        "5. SVD Kinematic Mechanism Guard (Gate 1, Equation 4.1)",
                        "K_ff = U · Σ · V^T   ==>   detect σ_min = 0 (Nullspace = mechanism)",
                        "Guarantees the structure is free from unstable kinematic collapse mechanisms.",
                    ),
                ];

                for (title, eq, desc) in formulas {
                    Frame::new()
                        .fill(theme.bg_canvas)
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(12))
                        .stroke(Stroke::new(1.0, theme.border_subtle))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(
                                RichText::new(title)
                                    .size(12.0)
                                    .strong()
                                    .color(theme.text_primary),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(eq)
                                    .size(13.0)
                                    .strong()
                                    .color(theme.accent_blue),
                            );
                            ui.add_space(2.0);
                            ui.label(RichText::new(desc).size(11.0).color(theme.text_secondary));
                        });
                    ui.add_space(8.0);
                }
            });
    }

    fn render_geotech_borehole(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        features: &mut FeatureRegistry,
    ) {
        ui_kit::feature_slot_card(ui, theme, features, FeatureId::GeotechBoreholeStratigraphy, |ui| {
            let summary = compute_geotech_profile_summary(
                &self.geotech_layers,
                self.geotech_gwt_depth_m,
                self.geotech_eq_mw,
                self.geotech_pga_g,
            );
            let vs_sub = format!("Vs30 = {:.0} m/s Weighted Avg", summary.vs30_mps);
            let gwt_val = format!("-{:.2} m BGS", self.geotech_gwt_depth_m);
            let liq_val = format!(
                "Fs = {:.2} ({})",
                summary.liquefaction_fs,
                if summary.liquefaction_fs >= 1.25 { "SAFE" } else { "LIQ RISK" }
            );
            let liq_sub = format!("Seed-Idriss CRR/CSR @ Mw {:.1}", self.geotech_eq_mw);
            let stats = [
                ("SITE CLASS (SNI 1726)", summary.site_class, vs_sub.as_str(), theme.accent_blue),
                ("GROUNDWATER TABLE", gwt_val.as_str(), "Interactive Piezometer Datum", theme.accent_orange),
                ("LIQUEFACTION SCREEN", liq_val.as_str(), liq_sub.as_str(), if summary.liquefaction_fs >= 1.25 { theme.accent_green } else { theme.accent_red }),
                ("BEARING STRATUM TOE", summary.rec_foundation.as_str(), "Socket & Pile End Bearing", theme.text_primary),
            ];
            ui_kit::responsive_grid_n(ui, stats.len(), 175.0, |ui, idx| {
                let (title, val, sub, col) = stats[idx];
                ui_kit::stat_card(ui, theme, title, val, sub, col);
            });

            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                for (idx, lbl) in [
                    (0, "BH-01 Alluvial Sand"),
                    (1, "BH-02 Soft Coastal Fill"),
                    (2, "CPT-04 Stiff Volcanic"),
                ] {
                    if ui.selectable_label(self.geotech_borehole_idx == idx, lbl).clicked() {
                        self.geotech_borehole_idx = idx;
                        self.geotech_layers = preset_borehole_strata(idx);
                    }
                }
                ui.label(RichText::new("GWT (m):").size(10.5).color(theme.text_muted));
                ui.add(egui::DragValue::new(&mut self.geotech_gwt_depth_m).range(0.5..=20.0).speed(0.1).max_decimals(1));
                ui.label(RichText::new("Mw:").size(10.5).color(theme.text_muted));
                ui.add(egui::DragValue::new(&mut self.geotech_eq_mw).range(5.5..=9.0).speed(0.1).max_decimals(1));
                ui.label(RichText::new("PGA (g):").size(10.5).color(theme.text_muted));
                ui.add(egui::DragValue::new(&mut self.geotech_pga_g).range(0.05..=0.80).speed(0.01).max_decimals(2));
                if icons::icon_button(
                    ui,
                    AppIcon::SaveDisk,
                    "Export AGS / CSV Log",
                    10.5,
                    theme.text_primary,
                    theme.bg_canvas,
                    Stroke::new(1.0, theme.border_subtle),
                    CornerRadius::same(6),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    let mut csv = String::from("depth_top_m,depth_bot_m,soil_name,N60,Vs_mps,strength,allowable\n");
                    for s in &self.geotech_layers {
                        csv.push_str(&format!(
                            "{:.1},{:.1},{},{},{:.0},{},{}\n",
                            s.depth_top_m, s.depth_bot_m, s.soil_name, s.n60, s.vs_mps, s.strength_desc, s.allowable_desc
                        ));
                    }
                    let _ = crate::core::web_bridge::save_text_file("Borehole_Stratigraphy.csv", &csv, "text/csv");
                }
            });
            ui.add_space(10.0);

            ui_kit::responsive_two_panes(ui, 620.0, |ui, pane| match pane {
                0 => {
                    ui.label(RichText::new("SUBSURFACE STRATIGRAPHY PROFILE (0m – 30m)").size(11.0).strong().color(theme.accent_blue));
                    ui.add_space(6.0);
                    let desired = Vec2::new(ui.available_width(), 170.0);
                    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
                    let painter = ui.painter_at(rect);
                    painter.rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(13, 17, 24));

                    let col_x = rect.left() + 18.0;
                    let col_w = 56.0;
                    for s in &self.geotech_layers {
                        let t0 = (s.depth_top_m / 30.0).clamp(0.0, 1.0);
                        let t1 = (s.depth_bot_m / 30.0).clamp(0.0, 1.0);
                        let band = Rect::from_min_max(
                            Pos2::new(col_x, rect.top() + 10.0 + t0 * (rect.height() - 20.0)),
                            Pos2::new(col_x + col_w, rect.top() + 10.0 + t1 * (rect.height() - 20.0)),
                        );
                        let col = Color32::from_rgb(s.color_rgb[0], s.color_rgb[1], s.color_rgb[2]);
                        painter.rect_filled(band, CornerRadius::ZERO, col);
                        painter.rect_stroke(band, CornerRadius::ZERO, Stroke::new(0.8, Color32::BLACK), StrokeKind::Inside);
                        painter.text(
                            Pos2::new(col_x + col_w + 12.0, band.center().y),
                            Align2::LEFT_CENTER,
                            format!("{:.0}–{:.0}m {} (N={}, Vs={:.0})", s.depth_top_m, s.depth_bot_m, s.soil_name, s.n60, s.vs_mps),
                            FontId::proportional(10.0),
                            theme.text_primary,
                        );
                    }
                    let gwt_y = rect.top() + 10.0 + (self.geotech_gwt_depth_m / 30.0).clamp(0.0, 1.0) * (rect.height() - 20.0);
                    painter.line_segment(
                        [Pos2::new(col_x - 6.0, gwt_y), Pos2::new(col_x + col_w + 6.0, gwt_y)],
                        Stroke::new(2.0, theme.accent_blue),
                    );
                }
                _ => {
                    ui.label(RichText::new("SPT BLOW COUNT (N60) & INTERACTIVE PILE DESIGN").size(11.0).strong().color(theme.accent_blue));
                    ui.add_space(6.0);
                    ScrollArea::horizontal().id_salt("geotech_spt_table").show(ui, |ui| {
                        egui::Grid::new("geotech_spt_grid")
                            .striped(true)
                            .spacing(Vec2::new(12.0, 6.0))
                            .show(ui, |ui| {
                                for h in ["Depth (m)", "Soil Description", "N60 (Edit)", "Vs (m/s)", "Capacity"] {
                                    ui.label(RichText::new(h).size(10.5).strong().color(theme.accent_blue));
                                }
                                ui.end_row();
                                for s in &mut self.geotech_layers {
                                    ui.label(RichText::new(format!("{:.1}–{:.1}m", s.depth_top_m, s.depth_bot_m)).size(10.5).monospace().color(theme.text_primary));
                                    ui.label(RichText::new(&s.soil_name).size(10.5).color(theme.text_secondary));
                                    let mut n_i = s.n60 as f32;
                                    if ui.add(egui::DragValue::new(&mut n_i).range(1.0..=60.0).speed(1.0).max_decimals(0)).changed() {
                                        s.n60 = n_i.round() as u32;
                                        s.vs_mps = (80.0 * s.n60.max(1) as f32).powf(0.35).max(110.0) * 1.15;
                                    }
                                    ui.label(RichText::new(format!("{:.0}", s.vs_mps)).size(10.5).monospace().color(theme.text_muted));
                                    ui.label(RichText::new(&s.allowable_desc).size(10.5).strong().color(theme.accent_green));
                                    ui.end_row();
                                }
                            });
                    });
                }
            });
        });
    }

    fn render_hydrology_floodplain(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        features: &mut FeatureRegistry,
    ) {
        ui_kit::feature_slot_card(ui, theme, features, FeatureId::HydrologyIdfFloodplain, |ui| {
            let hyd = compute_hydrology_idf_floodplain(
                self.hydro_return_idx,
                self.hydro_catchment_ha,
                self.hydro_runoff_c,
                self.hydro_tc_min,
                self.hydro_i100_1h_mm,
                self.hydro_flood_wl_asl_m,
            );
            let storm_hdr = format!("DESIGN STORM (T = {} YR)", hyd.return_period_yr);
            let i_val = format!("I = {:.1} mm/hr", hyd.intensity_mm_hr);
            let tc_sub = format!("tc = {:.0} min • Mononobe IDF", self.hydro_tc_min);
            let q_val = format!("{:.2} m³/s", hyd.peak_runoff_m3_s);
            let q_sub = format!("Catchment {:.1} ha • C = {:.2}", self.hydro_catchment_ha, self.hydro_runoff_c);
            let v_val = format!("{:.0} m³ Required", hyd.detention_vol_m3);
            let ffe_val = format!("+{:.2} m ASL", hyd.recommended_ffe_asl_m);
            let ffe_sub = format!("Flood WL +{:.2}m + Freeboard", self.hydro_flood_wl_asl_m);
            let cards = [
                (storm_hdr.as_str(), i_val.as_str(), tc_sub.as_str(), theme.accent_blue),
                ("PEAK RUNOFF (Q = C·I·A)", q_val.as_str(), q_sub.as_str(), theme.accent_orange),
                ("DETENTION BASIN VOLUME", v_val.as_str(), "Zero-Runoff Delta Retention", theme.accent_green),
                ("RECOMMENDED MIN FFE", ffe_val.as_str(), ffe_sub.as_str(), theme.accent_green),
            ];
            ui_kit::responsive_grid_n(ui, cards.len(), 175.0, |ui, idx| {
                let (title, val, sub, col) = cards[idx];
                ui_kit::stat_card(ui, theme, title, val, sub, col);
            });

            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                for (idx, lbl) in [(0, "10-Yr Storm"), (1, "25-Yr Storm"), (2, "100-Yr Critical Storm")] {
                    if ui.selectable_label(self.hydro_return_idx == idx, lbl).clicked() {
                        self.hydro_return_idx = idx;
                    }
                }
                ui.label(RichText::new("Area (ha):").size(10.5).color(theme.text_muted));
                ui.add(egui::DragValue::new(&mut self.hydro_catchment_ha).range(0.2..=250.0).speed(0.2).max_decimals(1));
                ui.label(RichText::new("C:").size(10.5).color(theme.text_muted));
                ui.add(egui::DragValue::new(&mut self.hydro_runoff_c).range(0.15..=0.95).speed(0.02).max_decimals(2));
                ui.label(RichText::new("tc (min):").size(10.5).color(theme.text_muted));
                ui.add(egui::DragValue::new(&mut self.hydro_tc_min).range(5.0..=120.0).speed(1.0).max_decimals(0));
                ui.label(RichText::new("Flood WL (m):").size(10.5).color(theme.text_muted));
                ui.add(egui::DragValue::new(&mut self.hydro_flood_wl_asl_m).range(-2.0..=30.0).speed(0.05).max_decimals(2));
            });
            ui.add_space(10.0);

            let desired = Vec2::new(ui.available_width(), 145.0);
            let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(13, 17, 24));
            let plot = rect.shrink(28.0);
            painter.line_segment(
                [Pos2::new(plot.left(), plot.bottom()), Pos2::new(plot.right(), plot.bottom())],
                Stroke::new(1.0, theme.text_muted),
            );
            painter.line_segment(
                [Pos2::new(plot.left(), plot.bottom()), Pos2::new(plot.left(), plot.top())],
                Stroke::new(1.0, theme.text_muted),
            );
            let max_i = (self.hydro_i100_1h_mm * (60.0_f32 / 5.0).powf(0.55)).max(120.0);
            for (r_idx, scale, col, lbl) in [
                (2, 1.00_f32, theme.accent_red, "T = 100 yr"),
                (1, 0.76, theme.accent_orange, "T = 25 yr"),
                (0, 0.58, theme.accent_blue, "T = 10 yr"),
            ] {
                let mut pts = Vec::with_capacity(20);
                for i in 0..20 {
                    let frac = i as f32 / 19.0;
                    let tc_pt = 5.0 + frac * 175.0;
                    let i_val = self.hydro_i100_1h_mm * scale * (60.0 / tc_pt).powf(0.55);
                    let y_norm = (i_val / max_i).clamp(0.04, 0.98);
                    pts.push(Pos2::new(
                        plot.left() + frac * plot.width(),
                        plot.bottom() - y_norm * plot.height(),
                    ));
                }
                let stroke_w = if r_idx == self.hydro_return_idx { 2.8 } else { 1.5 };
                for w in pts.windows(2) {
                    painter.line_segment([w[0], w[1]], Stroke::new(stroke_w, col));
                }
                painter.text(
                    pts[2],
                    Align2::LEFT_BOTTOM,
                    lbl,
                    FontId::proportional(9.5),
                    col,
                );
            }
            let op_frac = ((self.hydro_tc_min - 5.0) / 175.0).clamp(0.0, 1.0);
            let op_x = plot.left() + op_frac * plot.width();
            let op_y = plot.bottom() - (hyd.intensity_mm_hr / max_i).clamp(0.04, 0.98) * plot.height();
            painter.circle_filled(Pos2::new(op_x, op_y), 4.5, Color32::WHITE);
            painter.text(
                Pos2::new(plot.center().x, rect.bottom() - 8.0),
                Align2::CENTER_CENTER,
                format!("IDF Duration tc = {:.0} min  •  I = {:.1} mm/hr  •  Min FFE = +{:.2} m ASL", self.hydro_tc_min, hyd.intensity_mm_hr, hyd.recommended_ffe_asl_m),
                FontId::proportional(9.5),
                theme.text_muted,
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geotech_vs30_and_hydrology_rational_solvers() {
        let bh0 = preset_borehole_strata(0);
        let sum0 = compute_geotech_profile_summary(&bh0, 3.2, 7.8, 0.36);
        assert!(sum0.vs30_mps > 175.0 && sum0.vs30_mps < 350.0);
        assert_eq!(sum0.site_class, "Class SD (Stiff/Medium)");

        let bh1 = preset_borehole_strata(1);
        let sum1 = compute_geotech_profile_summary(&bh1, 1.5, 7.8, 0.42);
        assert!(sum1.liquefaction_fs < sum0.liquefaction_fs);

        let h10 = compute_hydrology_idf_floodplain(0, 6.2, 0.75, 22.0, 88.0, 1.50);
        let h100 = compute_hydrology_idf_floodplain(2, 6.2, 0.75, 22.0, 88.0, 1.50);
        assert!(h100.peak_runoff_m3_s > h10.peak_runoff_m3_s);
        assert!((h100.recommended_ffe_asl_m - 2.15).abs() < 1e-3);
    }
}
