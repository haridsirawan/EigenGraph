//! views/industrialized/mod.rs — Pillar 13: 10D BIM Industrialized Construction & Automation.
//! Integrates Industry 4.0 workflows: Prefabrication DfMA, Robotics, Autonomous Erection,
//! and Native OpenBIM IFCX (IFC 5 JSON ECS) Composer, Inspector, Clash & Diff Studio.

pub mod automation_slots;
pub mod dfma_robotics;
pub mod lifecycle_10d;
pub mod openbim_studio;

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, Rect, RichText, ScrollArea, Sense, Stroke, Ui, Vec2};
use crate::core::{
    IfcxAuditStatus, IfcxClashReport, IfcxClashSeverity, IfcxDiffKind, IfcxDiffReport,
    IfcxDocument, IfcxEditSession, IfcxHealthReport, IfcxStage, VectorEarthTheme,
};
use crate::features::{FeatureId, FeatureRegistry};
use crate::models::{PrefabModuleUnit, PrefabSupplyStage, StructuralMember};
use crate::theme::MacTheme;
use crate::ui_kit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndustrializedSubTab {
    Lifecycle10DHub,
    ModularDfma,
    RoboticAutomation,
    AutomatedErection,
    OpenBimIfcxStudio,
    PrecastLogisticsTolerance,
    Concrete3DPrinting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IfcxInspectorTab {
    NodeAttributes,
    HealthAudit,
    ClashAndDiff,
    RawJson,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IfcxSourceMode {
    EarthGlobeIfcx,
    SampleVillaIfcx,
    AiRealBuildingIfcx,
    CustomPaste,
    FilePath,
}

pub struct IndustrializedView {
    pub active_subtab: IndustrializedSubTab,
    pub offsite_ratio_pct: f32,
    pub robotic_speed_mm_s: f32,
    pub export_status: Option<String>,

    // Pure-Rust OpenBIM IFCX (IFC 5 JSON ECS) Studio State
    pub ifcx_doc: Option<IfcxDocument>,
    pub baseline_ifcx: Option<IfcxDocument>,
    pub selected_node_path: Option<String>,
    pub inspector_tab: IfcxInspectorTab,
    pub source_mode: IfcxSourceMode,
    pub custom_ifcx_paste: String,
    pub custom_ifcx_path: String,
    pub node_search_query: String,

    // Interactive Non-Destructive Edit Session (Undo / Redo)
    pub edit_session: IfcxEditSession,
    pub edit_name: String,
    pub edit_material: String,
    pub edit_description: String,

    // Spatial Clash & Layer Diff State
    pub clash_tolerance_mm: f32,
    pub clash_report: Option<IfcxClashReport>,
    pub diff_report: Option<IfcxDiffReport>,
    pub health_report: Option<IfcxHealthReport>,

    // Modular Prefab Supply Chain & QR/RFID Scanner Tracker
    pub prefab_units: Vec<PrefabModuleUnit>,
    pub prefab_stage_filter: Option<PrefabSupplyStage>,
    pub scanner_input: String,
    pub selected_module_tag: String,
    pub scanner_log: Vec<String>,
    // One-click IFCX -> Editable BimProject bridge
    pub request_apply_bim_project: Option<crate::core::BimProject>,
    // Precast Trailer Axle & Grout Splice Parameters
    pub precast_axle_limit_ton: f32,
    pub precast_portal_clear_m: f32,
    pub precast_grout_7day_mpa: f32,
    pub precast_night_escort: bool,
    // Concrete 3D Printing Parametric G-Code Slicer
    pub print_nozzle_mm: f32,
    pub print_layer_h_mm: f32,
    pub print_feed_mm_s: f32,
    pub print_wall_height_m: f32,
    pub print_spiralize: bool,
    pub print_cold_joint_cap_min: f32,
}

impl Default for IndustrializedView {
    fn default() -> Self {
        Self {
            active_subtab: IndustrializedSubTab::Lifecycle10DHub,
            offsite_ratio_pct: 82.5,
            robotic_speed_mm_s: 14.5,
            export_status: None,

            ifcx_doc: None,
            baseline_ifcx: None,
            selected_node_path: None,
            inspector_tab: IfcxInspectorTab::NodeAttributes,
            source_mode: IfcxSourceMode::EarthGlobeIfcx,
            custom_ifcx_paste: String::new(),
            custom_ifcx_path: String::new(),
            node_search_query: String::new(),

            edit_session: IfcxEditSession::new(),
            edit_name: String::new(),
            edit_material: String::new(),
            edit_description: String::new(),

            clash_tolerance_mm: 10.0,
            clash_report: None,
            diff_report: None,
            health_report: None,

            prefab_units: Vec::new(),
            prefab_stage_filter: None,
            scanner_input: String::new(),
            selected_module_tag: String::new(),
            scanner_log: Vec::new(),
            request_apply_bim_project: None,
            precast_axle_limit_ton: 64.0,
            precast_portal_clear_m: 4.55,
            precast_grout_7day_mpa: 68.5,
            precast_night_escort: true,
            print_nozzle_mm: 45.0,
            print_layer_h_mm: 15.0,
            print_feed_mm_s: 185.0,
            print_wall_height_m: 1.80,
            print_spiralize: true,
            print_cold_joint_cap_min: 18.0,
        }
    }
}

impl IndustrializedView {
    pub fn load_ifcx_document(&mut self, doc: IfcxDocument, label: &str) {
        self.health_report = Some(doc.audit_health());
        self.selected_node_path = doc.data.first().map(|n| n.path.clone());
        self.sync_edit_buffers_from_doc(&doc);
        self.baseline_ifcx = Some(doc.clone());
        self.clash_report = None;
        self.diff_report = None;
        self.edit_session.clear();
        self.export_status = Some(format!(
            "Loaded IFCX '{}' ({} nodes, schema {}).",
            label,
            doc.data.len(),
            doc.header.ifcx_version
        ));
        self.ifcx_doc = Some(doc);
    }

    pub(super) fn sync_edit_buffers_from_doc(&mut self, doc: &IfcxDocument) {
        if let Some(ref p) = self.selected_node_path {
            if let Some(node) = doc.find_node(p) {
                self.edit_name = node.display_name();
                self.edit_material = node.material_label();
                self.edit_description = node.description_label();
            }
        }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        _members: &[StructuralMember],
        bim_project: &crate::core::BimProject,
        features: &mut FeatureRegistry,
    ) {
        ui.vertical(|ui| {
            Frame::new().inner_margin(Margin::symmetric(0, 8)).show(ui, |ui| {
                let full_w = ui.available_width().max(160.0);
                ui.set_width(full_w);
                ui.set_max_width(full_w);
                self.render_header_bar(ui, theme);

                let mut dismiss_status = false;
                if let Some(ref status_msg) = self.export_status {
                    ui.add_space(10.0);
                    theme.card_frame().show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            crate::icons::icon_label(
                                ui,
                                crate::icons::AppIcon::BoxPackage,
                                12.0,
                                theme.accent_green,
                                "OPENBIM IFCX BUS:",
                                11.5,
                                theme.accent_green,
                                true,
                            );
                            ui.label(RichText::new(status_msg).size(11.5).color(theme.text_primary));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if crate::icons::icon_only_button(
                                    ui,
                                    crate::icons::AppIcon::Close,
                                    theme.text_muted,
                                    theme.bg_card,
                                    Stroke::NONE,
                                    CornerRadius::same(4),
                                    Vec2::splat(20.0),
                                    "Dismiss",
                                )
                                .clicked()
                                {
                                    dismiss_status = true;
                                }
                            });
                        });
                    });
                }
                if dismiss_status {
                    self.export_status = None;
                }

                ui.add_space(16.0);

                match self.active_subtab {
                    IndustrializedSubTab::Lifecycle10DHub => self.render_10d_lifecycle_hub(ui, theme),
                    IndustrializedSubTab::ModularDfma => self.render_modular_dfma(ui, theme),
                    IndustrializedSubTab::RoboticAutomation => self.render_robotic_automation(ui, theme),
                    IndustrializedSubTab::AutomatedErection => self.render_automated_erection(ui, theme),
                    IndustrializedSubTab::OpenBimIfcxStudio => {
                        self.render_openbim_ifcx_studio(ui, theme, bim_project, features)
                    }
                    IndustrializedSubTab::PrecastLogisticsTolerance => {
                        self.render_precast_logistics_tolerance(ui, theme, bim_project, features)
                    }
                    IndustrializedSubTab::Concrete3DPrinting => {
                        self.render_concrete_3d_printing(ui, theme, bim_project, features)
                    }
                }

                ui.add_space(64.0);
            });
        });
    }

    fn render_header_bar(&mut self, ui: &mut Ui, theme: &MacTheme) {
        let tabs = [
            (IndustrializedSubTab::Lifecycle10DHub, "10D BIM Lifecycle Hub"),
            (IndustrializedSubTab::ModularDfma, "Prefabrication & DfMA"),
            (IndustrializedSubTab::RoboticAutomation, "Robotics & Automation"),
            (IndustrializedSubTab::AutomatedErection, "Autonomous Crane & LiDAR"),
            (IndustrializedSubTab::OpenBimIfcxStudio, "OpenBIM IFCX (IFC 5) Studio"),
            (IndustrializedSubTab::PrecastLogisticsTolerance, "Precast Hauling & Splice (10D-04)"),
            (IndustrializedSubTab::Concrete3DPrinting, "3DCP G-Code Slicer (10D-05)"),
        ];
        ui_kit::subtab_bar(ui, theme, "ind_subtab_bar", &mut self.active_subtab, &tabs);
    }

    pub(super) fn render_responsive_cards(
        &self,
        ui: &mut Ui,
        cards: &[(&str, String, String, Color32)],
        theme: &MacTheme,
    ) {
        ui_kit::responsive_grid_n(ui, cards.len(), 210.0, |ui, idx| {
            let (title, val, sub, col) = &cards[idx];
            self.render_stat_card(ui, theme, title, val, sub, *col);
        });
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
