//! views/analyze/header.rs — Grouped Analysis Discipline Header, Interop Bar & Locked BIM->FEM Sync Preview.

use super::*;

impl AnalyzeView {
    pub(super) fn render_subtabs_header(&mut self, ui: &mut Ui, theme: &MacTheme) {
        theme.card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            // Top Row: Suite Title & Interop / Dossier Export Actions (wraps cleanly across phone/tablet/desktop)
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                icons::icon_label(
                    ui,
                    AppIcon::Shield,
                    14.0,
                    theme.accent_blue,
                    "8D Structural Resilience & Multi-Code Verification Suite",
                    12.5,
                    theme.accent_blue,
                    true,
                );
            });
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                let (publish_icon, publish_text) = if self.published_to_dossier {
                    (AppIcon::CheckCircle, "Synced to Vault")
                } else {
                    (AppIcon::UploadImport, "Publish to Vault Dossier")
                };
                let publish_bg = if self.published_to_dossier {
                    theme.accent_green
                } else {
                    theme.accent_blue.gamma_multiply(0.85)
                };

                if icons::icon_button(
                    ui,
                    publish_icon,
                    publish_text,
                    11.5,
                    Color32::WHITE,
                    publish_bg,
                    Stroke::NONE,
                    CornerRadius::same(7),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    self.published_to_dossier = true;
                    self.export_status =
                        Some("Model & EigenGate Analytics synced to Vault Dossier".to_string());
                }

                if icons::icon_button(
                    ui,
                    AppIcon::SaveDisk,
                    "SAP2000 (.s2k)",
                    11.0,
                    Color32::WHITE,
                    theme.bg_canvas,
                    Stroke::new(1.0, theme.accent_blue),
                    CornerRadius::same(7),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    let mut engine = crate::core::ParametricEngine::new();
                    if let Ok(model) = engine.generate_warren_truss() {
                        let eigengate = crate::core::EigenGateEngine::evaluate_redundancy(&model).ok();
                        let s2k =
                            crate::core::Sap2000Exporter::export_to_s2k(&model, eigengate.as_ref());
                        if let Ok(path) = crate::core::DiskExporter::write_export_file(
                            "EigenGraph_SAP2000.s2k",
                            &s2k,
                        ) {
                            self.export_status =
                                Some(format!("CSI SAP2000 .s2k saved to: {}", path));
                        }
                    }
                }

                if icons::icon_button(
                    ui,
                    AppIcon::SaveDisk,
                    "DXF 3D (.dxf)",
                    11.0,
                    Color32::WHITE,
                    theme.bg_canvas,
                    Stroke::new(1.0, theme.accent_green),
                    CornerRadius::same(7),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    let mut engine = crate::core::ParametricEngine::new();
                    if let Ok(model) = engine.generate_warren_truss() {
                        let eigengate = crate::core::EigenGateEngine::evaluate_redundancy(&model).ok();
                        let dxf =
                            crate::core::DxfExporter::export_to_dxf(&model, eigengate.as_ref());
                        if let Ok(path) = crate::core::DiskExporter::write_export_file(
                            "EigenGraph_3D.dxf",
                            &dxf,
                        ) {
                            self.export_status =
                                Some(format!("AutoCAD 3D DXF saved to: {}", path));
                        }
                    }
                }

                if icons::icon_button(
                    ui,
                    AppIcon::Globe,
                    "OpenBIM (.ifcx)",
                    11.0,
                    Color32::WHITE,
                    theme.bg_canvas,
                    Stroke::new(1.0, Color32::from_rgb(180, 100, 255)),
                    CornerRadius::same(7),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    let mut engine = crate::core::ParametricEngine::new();
                    if let Ok(model) = engine.generate_warren_truss() {
                        let eigengate = crate::core::EigenGateEngine::evaluate_redundancy(&model).ok();
                        let ifcx =
                            crate::core::IfcxExporter::export_to_ifcx(&model, eigengate.as_ref());
                        if let Ok(path) = crate::core::DiskExporter::write_export_file(
                            "EigenGraph_OpenBIM.ifcx",
                            &ifcx,
                        ) {
                            self.export_status =
                                Some(format!("OpenBIM IFCX (IFC 5) saved to: {}", path));
                        }
                    }
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Tier 1: Discipline Category Switcher (reduces 19 flat tabs into 4 clean engineering tracks)
            let mut active_category = self.active_subtab.category();
            let prev_category = active_category;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                ui.label(
                    RichText::new("DISCIPLINE:")
                        .size(10.5)
                        .strong()
                        .color(theme.text_muted),
                );
                for &cat in AnalyzeCategory::all() {
                    let is_active = active_category == cat;
                    let bg = if is_active {
                        theme.accent_blue.gamma_multiply(0.24)
                    } else {
                        theme.bg_canvas
                    };
                    let fg = if is_active {
                        Color32::WHITE
                    } else {
                        theme.text_secondary
                    };
                    let stroke = if is_active {
                        Stroke::new(1.2, theme.accent_blue)
                    } else {
                        Stroke::new(1.0, theme.border_subtle)
                    };
                    if icons::smart_button(
                        ui,
                        cat.label(),
                        11.5,
                        fg,
                        bg,
                        stroke,
                        CornerRadius::same(7),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        active_category = cat;
                    }
                }
            });

            if active_category != prev_category {
                self.active_subtab = active_category.default_subtab();
            }

            ui.add_space(6.0);

            // Tier 2: Filtered Sub-Tabs inside the active discipline (responsive wrapping / phone horizontal scroll)
            ui_kit::subtab_bar(
                ui,
                theme,
                "analyze_filtered_subtabs",
                &mut self.active_subtab,
                active_category.subtabs(),
            );

            ui.add_space(6.0);

            // Tier 3: Phase 45 Earth-to-Site Hazard Coupling & Geometry <-> Analysis Bridge
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                icons::icon_label(
                    ui,
                    AppIcon::Globe,
                    12.0,
                    theme.accent_blue,
                    format!(
                        "Earth {}: Elev {:+.1}m | Ss {:.2}g / S1 {:.2}g | Wind {:.0} m/s | {}",
                        self.site_coupling.site_label,
                        self.site_coupling.elevation_m,
                        self.site_coupling.seismic_pga_ss_g,
                        self.site_coupling.seismic_pga_s1_g,
                        self.site_coupling.design_wind_ms,
                        self.site_coupling.recommended_soil_class,
                    ),
                    10.5,
                    theme.text_secondary,
                    false,
                );

                if icons::icon_button(
                    ui,
                    AppIcon::Target,
                    "Sync Earth Hazard -> Seismic & Wind",
                    10.5,
                    Color32::WHITE,
                    theme.accent_green,
                    Stroke::NONE,
                    CornerRadius::same(6),
                    Vec2::new(0.0, 24.0),
                )
                .on_hover_text("Apply active EigenEarth pin Ss/S1 seismic acceleration, design wind speed & soil class to Analysis inputs")
                .clicked()
                {
                    self.ss_val = self.site_coupling.seismic_pga_ss_g;
                    self.s1_val = self.site_coupling.seismic_pga_s1_g;
                    self.wind_v_ms = self.site_coupling.design_wind_ms;
                    if self.site_coupling.recommended_soil_class.starts_with("SE") {
                        self.site_class = crate::core::SiteClass::SE;
                    } else if self.site_coupling.recommended_soil_class.starts_with("SC") {
                        self.site_class = crate::core::SiteClass::SC;
                    } else {
                        self.site_class = crate::core::SiteClass::SD;
                    }
                    self.export_status = Some(format!(
                        "Applied Earth site telemetry ({}) -> Ss={:.2}g, S1={:.2}g, V={:.0} m/s ({:?}).",
                        self.site_coupling.site_label, self.ss_val, self.s1_val, self.wind_v_ms, self.site_class
                    ));
                }

                if icons::icon_button(
                    ui,
                    AppIcon::Cube3D,
                    "Highlight in Geometry 3D ->",
                    10.5,
                    Color32::WHITE,
                    theme.accent_blue,
                    Stroke::NONE,
                    CornerRadius::same(6),
                    Vec2::new(0.0, 24.0),
                )
                .on_hover_text("Jump to Geometry 3D viewport and highlight active critical member with DCR stress & NFD diagram")
                .clicked()
                {
                    let focus_id = (self.woodbury_disrupt_elem_id > 0)
                        .then_some(self.woodbury_disrupt_elem_id);
                    self.requested_geometry_jump = Some((
                        focus_id,
                        Some(crate::views::build::Render3DMode::AxialForceNFD),
                        Some("DCR_Stress".to_string()),
                    ));
                }

                if icons::icon_button(
                    ui,
                    AppIcon::Rebar,
                    "Rebar BBS in Drafting ->",
                    10.5,
                    theme.text_primary,
                    theme.bg_canvas,
                    Stroke::new(1.0, theme.border_subtle),
                    CornerRadius::same(6),
                    Vec2::new(0.0, 24.0),
                )
                .on_hover_text("Jump to Drafting module's below-toolbar Rebar BBS detailing sub-menu")
                .clicked()
                {
                    self.requested_drafting_jump = Some(crate::views::parametric::DraftingSubTab::RebarBbs);
                }

                if let Some(ref note) = self.linked_from_geometry_note {
                    icons::pill_badge(ui, AppIcon::LinkChain, note.clone(), theme.accent_orange);
                }
            });
        });
    }

    pub(super) fn render_bim_sync_gate(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        features: &mut FeatureRegistry,
    ) {
        ui_kit::feature_slot_card(ui, theme, features, FeatureId::LiveBimToFemAutoSync, |ui| {
            ui_kit::responsive_grid_n(ui, 4, 190.0, |ui, idx| match idx {
                0 => {
                    ui.label(RichText::new("Analytical Source Bus").size(10.5).strong().color(theme.text_muted));
                    ui.label(
                        RichText::new(format!(
                            "Active BimProject ({} analytical members synced)",
                            self.synced_bim_element_count
                        ))
                        .size(10.5)
                        .color(theme.text_primary),
                    );
                }
                1 => {
                    ui.label(RichText::new("Mesh & Diaphragm Coupling").size(10.5).strong().color(theme.text_muted));
                    ui.label(
                        RichText::new("Auto Rigid Diaphragm + Nodal Connectivity")
                            .size(10.5)
                            .color(theme.text_secondary),
                    );
                }
                2 => {
                    ui.label(RichText::new("Sync Status").size(10.5).strong().color(theme.text_muted));
                    ui.label(
                        RichText::new(&self.last_bim_sync_label)
                            .size(10.5)
                            .color(theme.accent_green),
                    );
                }
                _ => {
                    ui.label(RichText::new("Reactive Solver Trigger").size(10.5).strong().color(theme.text_muted));
                    if icons::icon_button(
                        ui,
                        AppIcon::Bolt,
                        "Compile & Solve Live BIM -> FEM",
                        11.0,
                        Color32::WHITE,
                        theme.accent_blue,
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        self.request_bim_fem_sync = true;
                    }
                }
            });
        });
    }

    pub(super) fn metric_card(
        &self,
        ui: &mut Ui,
        theme: &MacTheme,
        title: &str,
        value: &str,
        sub: &str,
        color: Color32,
    ) {
        ui_kit::stat_card(ui, theme, title, value, sub, color);
    }

    pub(super) fn render_stat_card(
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
