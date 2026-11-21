//! views/industrialized/openbim_studio.rs — Pure-Rust OpenBIM IFCX (IFC 5 JSON ECS) Studio.
//!
//! Provides live composition & inspection of `earth_globe.ifcx`, active georeferenced building
//! layers, AI-captured real-world `.ifcx` layers, non-destructive attribute editing with Undo/Redo,
//! schema health audit, spatial clash detection, layer diffing, and 1-click `.ifcx` export.

use super::*;

impl IndustrializedView {
    pub(super) fn render_openbim_ifcx_studio(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        bim_project: &crate::core::BimProject,
        features: &mut FeatureRegistry,
    ) {
        let (schema_lbl, node_count, elem_count, health_score) =
            if let Some(ref doc) = self.ifcx_doc {
                let hr = self.health_report.as_ref();
                (
                    format!("IFC 5 ({})", doc.header.ifcx_version),
                    doc.data.len(),
                    hr.map(|h| h.building_elements).unwrap_or(0),
                    hr.map(|h| format!("{:.1}%", h.conformance_score_pct))
                        .unwrap_or_else(|| "100.0%".to_string()),
                )
            } else {
                ("No IFCX Loaded".to_string(), 0, 0, "0.0%".to_string())
            };

        let kpis = [
            (
                "SCHEMA SPECIFICATION",
                schema_lbl,
                "100% Pure-Rust IFCX JSON ECS".to_string(),
                theme.accent_blue,
            ),
            (
                "SCENE GRAPH NODES",
                format!("{node_count} Nodes"),
                "Unified /earth/... Hierarchy".to_string(),
                theme.text_primary,
            ),
            (
                "STRUCTURAL ELEMENTS",
                format!("{elem_count} Elements"),
                "bsi::ifc::class + usd::usdgeom::mesh".to_string(),
                theme.accent_green,
            ),
            (
                "CONFORMANCE HEALTH",
                health_score,
                "Zero Legacy STEP Dependencies".to_string(),
                theme.accent_green,
            ),
        ];
        self.render_responsive_cards(ui, &kpis, theme);

        ui.add_space(12.0);

        // Source & Composition Toolbar
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(14))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    crate::icons::icon_label(
                        ui,
                        crate::icons::AppIcon::Folder,
                        12.0,
                        theme.text_secondary,
                        "IFCX LAYER SOURCES:",
                        11.5,
                        theme.text_secondary,
                        true,
                    );

                    if ui.button("Living Earth Globe").clicked() {
                        self.source_mode = IfcxSourceMode::EarthGlobeIfcx;
                        let earth = IfcxStage::build_earth_globe_layer(VectorEarthTheme::CoastalBlueprint);
                        self.load_ifcx_document(earth, "earth_globe.ifcx");
                    }
                    if ui.button("Compose Earth + Active BIM").clicked() {
                        let composed = IfcxStage::compose_earth_with_site(
                            VectorEarthTheme::CoastalBlueprint,
                            bim_project,
                            None,
                            0.0,
                            None,
                            None,
                            None,
                        );
                        self.load_ifcx_document(composed, "living_earth_composed.ifcx");
                    }
                    if ui.button("Sample Duplex Villa").clicked() {
                        self.source_mode = IfcxSourceMode::SampleVillaIfcx;
                        self.load_ifcx_document(IfcxDocument::sample_villa_ifcx(), "duplex_villa.ifcx");
                    }
                    if ui.button("AI Real-Building Capture").clicked() {
                        self.source_mode = IfcxSourceMode::AiRealBuildingIfcx;
                        self.load_ifcx_document(
                            IfcxDocument::sample_ai_real_building_capture_ifcx(),
                            "ai_capture_bandung.ifcx",
                        );
                    }
                    if ui.button("Paste IFCX JSON").clicked() {
                        self.source_mode = IfcxSourceMode::CustomPaste;
                    }
                    if ui.button("Open Local .ifcx").clicked() {
                        self.source_mode = IfcxSourceMode::FilePath;
                    }
                });

                if self.source_mode == IfcxSourceMode::CustomPaste {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Paste IFCX (IFC 5) JSON document:")
                            .size(10.5)
                            .color(theme.text_muted),
                    );
                    ui.add(
                        egui::TextEdit::multiline(&mut self.custom_ifcx_paste)
                            .desired_rows(5)
                            .desired_width(f32::INFINITY)
                            .code_editor(),
                    );
                    ui.add_space(6.0);
                    if ui.button("Parse & Load IFCX").clicked() {
                        match IfcxDocument::parse(&self.custom_ifcx_paste) {
                            Ok(doc) => self.load_ifcx_document(doc, "pasted.ifcx"),
                            Err(err) => self.export_status = Some(format!("IFCX parse error: {err}")),
                        }
                    }
                } else if self.source_mode == IfcxSourceMode::FilePath {
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.custom_ifcx_path)
                                .hint_text("/path/to/building.ifcx")
                                .desired_width(320.0),
                        );
                        #[cfg(not(target_arch = "wasm32"))]
                        if ui.button("Load File").clicked() {
                            match std::fs::read_to_string(self.custom_ifcx_path.trim()) {
                                Ok(raw) => match IfcxDocument::parse(&raw) {
                                    Ok(doc) => {
                                        let path_lbl = self.custom_ifcx_path.clone();
                                        self.load_ifcx_document(doc, &path_lbl);
                                    }
                                    Err(err) => {
                                        self.export_status = Some(format!("IFCX parse error: {err}"));
                                    }
                                },
                                Err(err) => {
                                    self.export_status = Some(format!("File read error: {err}"));
                                }
                            }
                        }
                        if ui.button("Browse (.ifcx)").clicked() {
                            crate::core::web_bridge::trigger_file_picker(".ifcx,.json");
                        }
                    });
                }
            });

        ui.add_space(12.0);

        // Action Bar (Undo/Redo, Apply to BIM, Export .ifcx)
        if self.ifcx_doc.is_some() {
            ui.horizontal_wrapped(|ui| {
                let can_undo = self.edit_session.can_undo();
                let can_redo = self.edit_session.can_redo();
                if ui.add_enabled(can_undo, egui::Button::new("Undo")).clicked() {
                    if let Some(ref mut doc) = self.ifcx_doc {
                        if self.edit_session.undo(doc) {
                            self.health_report = Some(doc.audit_health());
                            let snap = doc.clone();
                            self.sync_edit_buffers_from_doc(&snap);
                        }
                    }
                }
                if ui.add_enabled(can_redo, egui::Button::new("Redo")).clicked() {
                    if let Some(ref mut doc) = self.ifcx_doc {
                        if self.edit_session.redo(doc) {
                            self.health_report = Some(doc.audit_health());
                            let snap = doc.clone();
                            self.sync_edit_buffers_from_doc(&snap);
                        }
                    }
                }
                if ui.button("Load into 3D Earth BIM").clicked() {
                    if let Some(ref doc) = self.ifcx_doc {
                        match doc.to_bim_project("IFCX_Import") {
                            Ok((proj, msg)) => {
                                self.request_apply_bim_project = Some(proj);
                                self.export_status = Some(msg);
                            }
                            Err(err) => {
                                self.export_status = Some(err);
                            }
                        }
                    }
                }
                if ui.button("Export .ifcx").clicked() {
                    if let Some(ref doc) = self.ifcx_doc {
                        let json_out = doc.to_json_pretty();
                        let fname = format!("{}.ifcx", doc.header.id.replace(' ', "_"));
                        match crate::core::web_bridge::save_text_file(
                            &fname,
                            &json_out,
                            "application/json;charset=utf-8",
                        ) {
                            Ok(_) => {
                                self.export_status = Some(format!("Exported IFCX layer '{fname}'."));
                            }
                            Err(e) => {
                                self.export_status = Some(e);
                            }
                        }
                    }
                }
            });
            ui.add_space(10.0);
        }

        // Main Split Workbench (Node Tree + Inspector)
        if self.ifcx_doc.is_none() {
            theme.card_frame().show(ui, |ui| {
                ui.label(
                    RichText::new("Select 'Living Earth Globe', 'Sample Duplex Villa', 'AI Real-Building Capture', or import any .ifcx layer above to inspect and edit.")
                        .size(12.0)
                        .color(theme.text_muted),
                );
            });
            return;
        }

        let wide = ui.available_width() >= 760.0;
        let _ = (features, FeatureId::OpenBimIfcReaderEditorClashDiff);

        if wide {
            ui.columns(2, |cols| {
                self.render_node_browser(&mut cols[0], theme);
                self.render_inspector_panel(&mut cols[1], theme);
            });
        } else {
            self.render_node_browser(ui, theme);
            ui.add_space(10.0);
            self.render_inspector_panel(ui, theme);
        }
    }

    fn render_node_browser(&mut self, ui: &mut Ui, theme: &MacTheme) {
        theme.card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("IFCX SCENE GRAPH (/earth/...)").strong().size(11.5).color(theme.text_secondary));
            });
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.node_search_query)
                    .hint_text("Filter path or class (e.g. IfcColumn, ocean, land)...")
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(6.0);

            let q = self.node_search_query.trim().to_lowercase();
            let mut newly_selected: Option<String> = None;

            if let Some(ref doc) = self.ifcx_doc {
                ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                    for node in &doc.data {
                        let cls = node.ifc_class().unwrap_or("Node");
                        let dn = node.display_name();
                        if !q.is_empty()
                            && !node.path.to_lowercase().contains(&q)
                            && !cls.to_lowercase().contains(&q)
                            && !dn.to_lowercase().contains(&q)
                        {
                            continue;
                        }
                        let is_sel = self.selected_node_path.as_deref() == Some(&node.path);
                        let lbl = format!("[{cls}] {}  ({})", dn, node.path);
                        if ui.selectable_label(is_sel, RichText::new(lbl).size(11.0)).clicked() {
                            newly_selected = Some(node.path.clone());
                        }
                    }
                });
            }

            if let Some(path) = newly_selected {
                self.selected_node_path = Some(path);
                if let Some(doc) = self.ifcx_doc.clone() {
                    self.sync_edit_buffers_from_doc(&doc);
                }
            }
        });
    }

    fn render_inspector_panel(&mut self, ui: &mut Ui, theme: &MacTheme) {
        theme.card_frame().show(ui, |ui| {
            let tabs = [
                (IfcxInspectorTab::NodeAttributes, "Attributes & Edit"),
                (IfcxInspectorTab::HealthAudit, "Health Audit"),
                (IfcxInspectorTab::ClashAndDiff, "Clash & Layer Diff"),
                (IfcxInspectorTab::RawJson, "Node JSON"),
            ];
            ui_kit::subtab_bar(ui, theme, "ifcx_inspector_tabs", &mut self.inspector_tab, &tabs);
            ui.add_space(8.0);

            match self.inspector_tab {
                IfcxInspectorTab::NodeAttributes => {
                    let sel_path = self.selected_node_path.clone();
                    if let (Some(path), Some(doc)) = (sel_path, self.ifcx_doc.as_ref()) {
                        if let Some(node) = doc.find_node(&path) {
                            ui.label(RichText::new(&node.path).strong().size(12.0).color(theme.accent_blue));
                            ui.label(
                                RichText::new(format!(
                                    "Class: {} | Children: {} | Namespaces: {}",
                                    node.ifc_class().unwrap_or("Unclassified"),
                                    node.children.len(),
                                    node.attributes.len()
                                ))
                                .size(11.0)
                                .color(theme.text_muted),
                            );
                            if let Some(mesh) = node.mesh() {
                                ui.label(
                                    RichText::new(format!(
                                        "usd::usdgeom::mesh: {} vertices, {} triangles",
                                        mesh.points.len(),
                                        mesh.triangulated_indices().len()
                                    ))
                                    .size(11.0)
                                    .color(theme.accent_green),
                                );
                            }
                            if let Some(geo) = node.geo_wgs84() {
                                ui.label(
                                    RichText::new(format!(
                                        "eigengraph::geo::wgs84: {:.5}°, {:.5}° (Elev {:.1} m, UTM {}{})",
                                        geo.lat,
                                        geo.lon,
                                        geo.elev_m,
                                        geo.utm_zone,
                                        if geo.south_hemisphere { "S" } else { "N" }
                                    ))
                                    .size(11.0)
                                    .color(theme.accent_blue),
                                );
                            }
                            ui.add_space(8.0);
                            ui.label(RichText::new("Name (bsi::ifc::prop):").size(10.5));
                            ui.text_edit_singleline(&mut self.edit_name);
                            ui.label(RichText::new("Material:").size(10.5));
                            ui.text_edit_singleline(&mut self.edit_material);
                            ui.label(RichText::new("Description:").size(10.5));
                            ui.text_edit_singleline(&mut self.edit_description);
                            ui.add_space(6.0);
                            if ui.button("Apply Attribute Edit").clicked() {
                                if let Some(ref mut mut_doc) = self.ifcx_doc {
                                    self.edit_session.push_snapshot(mut_doc.clone());
                                    if let Some(n) = mut_doc.find_node_mut(&path) {
                                        n.set_prop(
                                            &self.edit_name,
                                            &self.edit_material,
                                            &self.edit_description,
                                        );
                                    }
                                    self.health_report = Some(mut_doc.audit_health());
                                    self.export_status = Some(format!("Updated attributes on '{path}'."));
                                }
                            }
                        }
                    } else {
                        ui.label("Select a node in the IFCX Scene Graph to inspect.");
                    }
                }
                IfcxInspectorTab::HealthAudit => {
                    if let Some(ref hr) = self.health_report {
                        ui.label(
                            RichText::new(format!(
                                "{} — {:.1}% ({} nodes, {} meshes)",
                                hr.verdict, hr.conformance_score_pct, hr.total_nodes, hr.mesh_nodes
                            ))
                            .strong()
                            .size(12.0)
                            .color(theme.accent_green),
                        );
                        ui.add_space(6.0);
                        for chk in &hr.checks {
                            let col = match chk.status {
                                IfcxAuditStatus::Pass => theme.accent_green,
                                IfcxAuditStatus::Warning => Color32::from_rgb(245, 158, 11),
                                IfcxAuditStatus::Fail => Color32::from_rgb(239, 68, 68),
                            };
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(&chk.code).strong().color(col).size(11.0));
                                ui.label(RichText::new(&chk.title).strong().size(11.0));
                            });
                            ui.label(RichText::new(&chk.detail).size(10.5).color(theme.text_muted));
                            ui.add_space(4.0);
                        }
                    }
                }
                IfcxInspectorTab::ClashAndDiff => {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Tolerance (mm):");
                        ui.add(egui::DragValue::new(&mut self.clash_tolerance_mm).range(0.0..=200.0));
                        if ui.button("Run Spatial Clash").clicked() {
                            if let Some(ref doc) = self.ifcx_doc {
                                self.clash_report = Some(doc.detect_clashes(self.clash_tolerance_mm as f64));
                            }
                        }
                        if ui.button("Run Layer Diff").clicked() {
                            if let (Some(ref doc), Some(ref base)) = (&self.ifcx_doc, &self.baseline_ifcx) {
                                self.diff_report = Some(doc.diff_against(base));
                            }
                        }
                    });
                    ui.add_space(6.0);
                    if let Some(ref cr) = self.clash_report {
                        ui.label(
                            RichText::new(format!(
                                "Clash Check: {} pairs evaluated, {} clashes detected.",
                                cr.checked_pairs,
                                cr.clashes.len()
                            ))
                            .strong()
                            .size(11.0),
                        );
                        for c in cr.clashes.iter().take(20) {
                            let col = match c.severity {
                                IfcxClashSeverity::Critical => Color32::from_rgb(239, 68, 68),
                                IfcxClashSeverity::Major => Color32::from_rgb(249, 115, 22),
                                IfcxClashSeverity::Clearance => Color32::from_rgb(234, 179, 8),
                            };
                            ui.label(
                                RichText::new(format!(
                                    "{} <-> {} ({:.1} mm overlap)",
                                    c.path_a, c.path_b, c.overlap_mm
                                ))
                                .size(10.5)
                                .color(col),
                            );
                        }
                    }
                    if let Some(ref dr) = self.diff_report {
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(format!(
                                "Layer Diff vs Baseline: +{} added, *{} modified, -{} deleted, ={} unchanged",
                                dr.added, dr.modified, dr.deleted, dr.unchanged
                            ))
                            .strong()
                            .size(11.0),
                        );
                        for d in dr.items.iter().filter(|i| i.kind != IfcxDiffKind::Unchanged).take(25) {
                            let (col, tag) = match d.kind {
                                IfcxDiffKind::Added => (theme.accent_green, "+"),
                                IfcxDiffKind::Modified => (Color32::from_rgb(249, 115, 22), "*"),
                                IfcxDiffKind::Deleted => (Color32::from_rgb(239, 68, 68), "-"),
                                IfcxDiffKind::Unchanged => (theme.text_muted, "="),
                            };
                            ui.label(
                                RichText::new(format!("{tag} [{}] {} — {}", d.class_code, d.path, d.detail))
                                    .size(10.5)
                                    .color(col),
                            );
                        }
                    }
                }
                IfcxInspectorTab::RawJson => {
                    if let (Some(ref path), Some(ref doc)) = (&self.selected_node_path, &self.ifcx_doc) {
                        if let Some(node) = doc.find_node(path) {
                            let mut txt = serde_json::to_string_pretty(node).unwrap_or_default();
                            if txt.len() > 16_000 {
                                txt.truncate(16_000);
                                txt.push_str("\n... (truncated for preview; use Export .ifcx for full payload)");
                            }
                            ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut txt)
                                        .code_editor()
                                        .desired_width(f32::INFINITY),
                                );
                            });
                        }
                    }
                }
            }
        });
    }
}
