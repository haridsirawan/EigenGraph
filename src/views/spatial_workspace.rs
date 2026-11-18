//! views/spatial_workspace.rs — Phase 46: Antigravity Spatial Canvas Shell,
//! Unified Command Palette (`/`) & Smart Context Pop-Up Inspector.
//!
//! Provides a zero-chrome full-viewport Earth + BIM/CAD spatial workspace where all 15
//! engineering modules, structural lenses, generative BIM commands, and element-level
//! calculations live behind an Antigravity-style AI Command Bar and context-sensitive
//! floating element pop-ups.

use egui::{
    Align, Align2, Area, Color32, CornerRadius, Frame, Id, Layout, Margin, Order, Rect, RichText,
    ScrollArea, Stroke, Ui, Vec2,
};

use crate::core::{
    compute_site_environmental_coupling, BimCommand, BimElementKind, BimProject,
};
use crate::icons::{self, AppIcon};
use crate::models::{DatabaseComponent, NavigationPillar, ProjectItem, StructuralMember};
use crate::theme::MacTheme;
use crate::views::build::{Ground3DAuthorTool, Render3DMode};
use crate::views::universal_canvas::UniversalCanvasTool;
use crate::views::{AiAssistantView, BuildView, UniversalCanvasState};

/// Active spatial viewport projection rendered across the full screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpatialCanvasMode {
    #[default]
    EarthGlobe,
    EarthPlan2D,
    Bim3D,
}

impl SpatialCanvasMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::EarthGlobe => "Earth 3D",
            Self::EarthPlan2D => "Plan 2D",
            Self::Bim3D => "BIM 3D",
        }
    }

    pub fn icon(self) -> AppIcon {
        match self {
            Self::EarthGlobe => AppIcon::Globe,
            Self::EarthPlan2D => AppIcon::Compass,
            Self::Bim3D => AppIcon::Cube3D,
        }
    }
}

/// On-canvas structural & lifecycle visualization lens invoked via `/lens ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpatialLens {
    #[default]
    Clean,
    DcrHeatmap,
    AriRedundancy,
    AxialForce,
    ShearForce,
    BendingMoment,
    Erection4D,
}

impl SpatialLens {
    pub fn label(self) -> &'static str {
        match self {
            Self::Clean => "Clean",
            Self::DcrHeatmap => "DCR Utilization",
            Self::AriRedundancy => "ARI Redundancy",
            Self::AxialForce => "Axial NFD",
            Self::ShearForce => "Shear SFD",
            Self::BendingMoment => "Moment BMD",
            Self::Erection4D => "4D Erection",
        }
    }
}

/// Tabs inside the context-sensitive Smart Element Pop-Up Inspector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SmartPopupTab {
    #[default]
    Properties,
    Calculations,
    Lifecycle5D6D,
    AskAi,
}

impl SmartPopupTab {
    pub fn label(self) -> &'static str {
        match self {
            Self::Properties => "Props",
            Self::Calculations => "Calc",
            Self::Lifecycle5D6D => "5D/6D",
            Self::AskAi => "Ask AI",
        }
    }
}

/// Element or Earth location targeted by a spatial click on the full canvas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SmartSelectionTarget {
    Column(usize),
    Beam(usize),
    Wall(usize),
    Footing(usize),
    SketchEntity(usize),
    EarthSite { lat: f64, lon: f64 },
}

/// Compact Antigravity-style execution trace chip displayed above the AI Command Bar.
#[derive(Debug, Clone)]
pub struct AgentTraceChip {
    pub label: String,
    pub ok: bool,
}

/// Declarative specification for a `/` command in the unified Command Palette.
#[derive(Debug, Clone, Copy)]
pub struct SlashCommandSpec {
    pub cmd: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub icon: AppIcon,
}

/// Full `/` slash command catalog keeping every EigenGraph capability reachable without chrome.
pub const SLASH_COMMANDS: &[SlashCommandSpec] = &[
    SlashCommandSpec {
        cmd: "/earth",
        category: "Canvas",
        description: "Switch full viewport to 3D Google Earth Globe",
        icon: AppIcon::Globe,
    },
    SlashCommandSpec {
        cmd: "/3d",
        category: "Canvas",
        description: "Switch full viewport to 3D Structural BIM on Earth",
        icon: AppIcon::Cube3D,
    },
    SlashCommandSpec {
        cmd: "/2d",
        category: "Canvas",
        description: "Switch full viewport to 2D Ground Plan & Geodetic Map",
        icon: AppIcon::Compass,
    },
    SlashCommandSpec {
        cmd: "/fit",
        category: "Canvas",
        description: "Fit camera to active BIM building or Earth site",
        icon: AppIcon::ZoomFit,
    },
    SlashCommandSpec {
        cmd: "/sample",
        category: "Model",
        description: "Load integrated 3D BIMCraft building onto active site",
        icon: AppIcon::Building,
    },
    SlashCommandSpec {
        cmd: "/portal 3x2 @6m 2floors",
        category: "Model",
        description: "Synthesize parametric multi-storey structural portal",
        icon: AppIcon::Grid,
    },
    SlashCommandSpec {
        cmd: "/box 8x6",
        category: "Model",
        description: "Synthesize structural box enclosure with roof slab",
        icon: AppIcon::RectShape,
    },
    SlashCommandSpec {
        cmd: "/column",
        category: "Draft",
        description: "Activate 3D Column placement tool (shortcut C)",
        icon: AppIcon::Column,
    },
    SlashCommandSpec {
        cmd: "/beam",
        category: "Draft",
        description: "Activate 3D Beam placement tool (shortcut B)",
        icon: AppIcon::Beam,
    },
    SlashCommandSpec {
        cmd: "/wall",
        category: "Draft",
        description: "Activate 3D Wall placement tool (shortcut W)",
        icon: AppIcon::Wall,
    },
    SlashCommandSpec {
        cmd: "/footing",
        category: "Draft",
        description: "Activate 3D Isolated Footing placement tool (shortcut F)",
        icon: AppIcon::Slab,
    },
    SlashCommandSpec {
        cmd: "/line",
        category: "Draft",
        description: "Activate 2D CAD vector line drafting tool (shortcut L)",
        icon: AppIcon::LineSeg,
    },
    SlashCommandSpec {
        cmd: "/measure",
        category: "Draft",
        description: "Measure geodesic distance or area directly on Earth",
        icon: AppIcon::DimensionRuler,
    },
    SlashCommandSpec {
        cmd: "/lens dcr",
        category: "Lens",
        description: "Colorize structural elements by LRFD utilization DCR%",
        icon: AppIcon::Target,
    },
    SlashCommandSpec {
        cmd: "/lens ari",
        category: "Lens",
        description: "Colorize structural elements by Redundancy Index (ARI)",
        icon: AppIcon::Shield,
    },
    SlashCommandSpec {
        cmd: "/lens moment",
        category: "Lens",
        description: "Display 3D Bending Moment Diagrams (BMD) on frame",
        icon: AppIcon::ChartLine,
    },
    SlashCommandSpec {
        cmd: "/lens axial",
        category: "Lens",
        description: "Display 3D Axial Force Diagrams (NFD) on frame",
        icon: AppIcon::Bolt,
    },
    SlashCommandSpec {
        cmd: "/lens 4d",
        category: "Lens",
        description: "Activate 4D staged construction erection day slider",
        icon: AppIcon::Clock,
    },
    SlashCommandSpec {
        cmd: "/lens clean",
        category: "Lens",
        description: "Restore clean architectural & Earth rendering",
        icon: AppIcon::Sparkles,
    },
    SlashCommandSpec {
        cmd: "/repair",
        category: "AI",
        description: "Run closed-loop AI auto-repair for critical ARI/DCR members",
        icon: AppIcon::Wrench,
    },
    SlashCommandSpec {
        cmd: "/svd",
        category: "AI",
        description: "Run SVD kinematic guard & full redundancy screening",
        icon: AppIcon::Bolt,
    },
    SlashCommandSpec {
        cmd: "/rebar",
        category: "Model",
        description: "Auto-reinforce all structural concrete elements",
        icon: AppIcon::Rebar,
    },
    SlashCommandSpec {
        cmd: "/clear",
        category: "Model",
        description: "Clear active BIM elements to blank ground plane",
        icon: AppIcon::Eraser,
    },
    SlashCommandSpec {
        cmd: "/undo",
        category: "Edit",
        description: "Undo last BIM, CAD, or Earth modification (Ctrl/Cmd+Z)",
        icon: AppIcon::Undo,
    },
    SlashCommandSpec {
        cmd: "/redo",
        category: "Edit",
        description: "Redo last undone modification (Ctrl/Cmd+Y)",
        icon: AppIcon::Redo,
    },
    SlashCommandSpec {
        cmd: "/save",
        category: "File",
        description: "Export unified single-file .eigen.json workspace archive",
        icon: AppIcon::SaveDisk,
    },
    SlashCommandSpec {
        cmd: "/theme",
        category: "View",
        description: "Toggle Pro Obsidian Dark / Engineering Daylight theme",
        icon: AppIcon::Moon,
    },
    SlashCommandSpec {
        cmd: "/vault",
        category: "Module",
        description: "Open Vault & Project Records overlay sheet",
        icon: AppIcon::Folder,
    },
    SlashCommandSpec {
        cmd: "/agent",
        category: "Module",
        description: "Open Autonomous Virtual Engineering Firm sheet",
        icon: AppIcon::Bot,
    },
    SlashCommandSpec {
        cmd: "/planning",
        category: "Module",
        description: "Open Planning, GIS, CRS & Seismic Hazard sheet",
        icon: AppIcon::Compass,
    },
    SlashCommandSpec {
        cmd: "/drafting",
        category: "Module",
        description: "Open 2D Drafting, Rebar BBS & Dynamo Studio sheet",
        icon: AppIcon::RulerSquare,
    },
    SlashCommandSpec {
        cmd: "/geometry",
        category: "Module",
        description: "Open 3D Geometry, Clash Matrix & Point Cloud sheet",
        icon: AppIcon::Cube3D,
    },
    SlashCommandSpec {
        cmd: "/analysis",
        category: "Module",
        description: "Open Geo-Structural FEA, Pushover & Multi-Hazard sheet",
        icon: AppIcon::Bolt,
    },
    SlashCommandSpec {
        cmd: "/schedule",
        category: "Module",
        description: "Open 4D Erection Schedule, CPM & Site Diary sheet",
        icon: AppIcon::Clock,
    },
    SlashCommandSpec {
        cmd: "/cost",
        category: "Module",
        description: "Open 5D BoQ, QTO & Procurement Costing sheet",
        icon: AppIcon::Coins,
    },
    SlashCommandSpec {
        cmd: "/sustainability",
        category: "Module",
        description: "Open 6D Embodied Carbon, Solar OTTV & Circularity sheet",
        icon: AppIcon::Leaf,
    },
    SlashCommandSpec {
        cmd: "/facility",
        category: "Module",
        description: "Open 7D Digital Twin & Live SHM Telemetry sheet",
        icon: AppIcon::Building,
    },
    SlashCommandSpec {
        cmd: "/safety",
        category: "Module",
        description: "Open 8D Crane Sweep, Formwork & PTW Safety sheet",
        icon: AppIcon::Shield,
    },
    SlashCommandSpec {
        cmd: "/construction",
        category: "Module",
        description: "Open 9D Lean Takt, QA/QC Lab & Cutting Stock sheet",
        icon: AppIcon::Recycle,
    },
    SlashCommandSpec {
        cmd: "/automation",
        category: "Module",
        description: "Open 10D DfMA Prefab, 3DCP & OpenBIM IFC sheet",
        icon: AppIcon::Cpu,
    },
    SlashCommandSpec {
        cmd: "/prediction",
        category: "Module",
        description: "Open AI Aging, Chloride Ingress & Seismic Triage sheet",
        icon: AppIcon::Pulse,
    },
    SlashCommandSpec {
        cmd: "/report",
        category: "Module",
        description: "Open PBG Calculation Book, Sheets & Handover Publisher",
        icon: AppIcon::Document,
    },
    SlashCommandSpec {
        cmd: "/settings",
        category: "View",
        description: "Open Workspace Settings & Feature Registry",
        icon: AppIcon::SettingsGear,
    },
    SlashCommandSpec {
        cmd: "/classic",
        category: "View",
        description: "Switch to Classic 15-Pillar Sidebar Workspace layout",
        icon: AppIcon::SlidersPanel,
    },
];

/// Side-effect requests produced by the Antigravity Spatial HUD for `EigenGraphApp` to commit.
#[derive(Debug, Default)]
pub struct SpatialHudActions {
    pub sync_fem_and_earth: bool,
    pub fit_camera: bool,
    pub load_sample_building: bool,
    pub export_archive: bool,
    pub toggle_theme: bool,
    pub open_settings: bool,
    pub switch_to_classic_ui: bool,
    pub toggle_n1_member: Option<usize>,
    pub run_auto_repair: bool,
}

/// Complete state for the Phase 46 Antigravity Spatial Workspace UI.
pub struct SpatialWorkspaceState {
    pub spatial_mode_enabled: bool,
    pub active_canvas_mode: SpatialCanvasMode,
    pub active_lens: SpatialLens,
    pub selected_target: Option<SmartSelectionTarget>,
    pub popup_pinned: bool,
    pub popup_tab: SmartPopupTab,
    pub command_bar_expanded: bool,
    pub command_input: String,
    pub request_focus_command: bool,
    pub active_overlay_pillar: Option<NavigationPillar>,
    pub recent_trace_chips: Vec<AgentTraceChip>,
    pub last_uc_pick_seq: u64,
    pub last_build_pick_seq: u64,
    pub hud_rects: Vec<Rect>,

    // Editable element inspector draft state synced lazily when selection changes
    last_loaded_target: Option<SmartSelectionTarget>,
    pub edit_width_mm: f64,
    pub edit_depth_mm: f64,
    pub edit_length_mm: f64,
    pub edit_thickness_mm: f64,
    pub edit_rot_deg: f64,
    pub edit_is_structural: bool,
    pub edit_profile_name: String,
    pub active_4d_day: u32,
}

impl Default for SpatialWorkspaceState {
    fn default() -> Self {
        Self {
            spatial_mode_enabled: true,
            active_canvas_mode: SpatialCanvasMode::EarthGlobe,
            active_lens: SpatialLens::Clean,
            selected_target: None,
            popup_pinned: false,
            popup_tab: SmartPopupTab::Properties,
            command_bar_expanded: false,
            command_input: String::new(),
            request_focus_command: false,
            active_overlay_pillar: None,
            recent_trace_chips: vec![AgentTraceChip {
                label: "Earth Globe Ready • Type / or click any element".to_string(),
                ok: true,
            }],
            last_uc_pick_seq: 0,
            last_build_pick_seq: 0,
            hud_rects: Vec::new(),
            last_loaded_target: None,
            edit_width_mm: 400.0,
            edit_depth_mm: 400.0,
            edit_length_mm: 1400.0,
            edit_thickness_mm: 450.0,
            edit_rot_deg: 0.0,
            edit_is_structural: true,
            edit_profile_name: "RC 400x400".to_string(),
            active_4d_day: 90,
        }
    }
}

impl SpatialWorkspaceState {
    /// Pushes a concise Antigravity execution trace badge, capping history to 4 chips.
    pub fn push_trace(&mut self, label: impl Into<String>, ok: bool) {
        self.recent_trace_chips.push(AgentTraceChip {
            label: label.into(),
            ok,
        });
        if self.recent_trace_chips.len() > 4 {
            let drain = self.recent_trace_chips.len() - 4;
            self.recent_trace_chips.drain(0..drain);
        }
    }

    /// Returns true if the pointer currently hovers any floating Antigravity HUD panel.
    pub fn is_pointer_over_hud(&self, ctx: &egui::Context) -> bool {
        if self.active_overlay_pillar.is_some() {
            return true;
        }
        ctx.input(|i| i.pointer.hover_pos())
            .is_some_and(|p| self.hud_rects.iter().any(|r| r.contains(p)))
    }

    /// Pulls fresh pick events from either `UniversalCanvasState` or `BuildView` into `selected_target`.
    pub fn sync_pick_events(
        &mut self,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
    ) {
        if universal_canvas.spatial_pick_seq != self.last_uc_pick_seq {
            self.last_uc_pick_seq = universal_canvas.spatial_pick_seq;
            if let Some((kind, id)) = universal_canvas.last_picked_bim_element {
                self.selected_target = Some(Self::target_from_bim(kind, id));
                self.popup_tab = SmartPopupTab::Properties;
            } else if let Some(eid) = universal_canvas.selected_entity_id {
                self.selected_target = Some(SmartSelectionTarget::SketchEntity(eid));
            } else if let Some((lat, lon)) = universal_canvas.last_picked_earth_lat_lon {
                self.selected_target = Some(SmartSelectionTarget::EarthSite { lat, lon });
            } else if !self.popup_pinned {
                self.selected_target = None;
            }
        }

        if build_view.spatial_pick_seq != self.last_build_pick_seq {
            self.last_build_pick_seq = build_view.spatial_pick_seq;
            if let Some((kind, id)) = build_view.last_spatial_bim_pick {
                let target = Self::target_from_bim(kind, id);
                self.selected_target = Some(target);
                universal_canvas.highlighted_bim_element = Some((kind, id));
                self.popup_tab = SmartPopupTab::Properties;
            } else if !self.popup_pinned {
                self.selected_target = None;
                universal_canvas.highlighted_bim_element = None;
            }
        }
    }

    fn target_from_bim(kind: BimElementKind, id: usize) -> SmartSelectionTarget {
        match kind {
            BimElementKind::Column => SmartSelectionTarget::Column(id),
            BimElementKind::Run => SmartSelectionTarget::Beam(id),
            BimElementKind::Wall => SmartSelectionTarget::Wall(id),
            BimElementKind::Footing => SmartSelectionTarget::Footing(id),
        }
    }

    /// Returns a contextual `@Mention` label when an element or Earth location is selected.
    pub fn active_context_mention(&self, project: &BimProject) -> Option<String> {
        match self.selected_target? {
            SmartSelectionTarget::Column(id) => project
                .columns
                .iter()
                .find(|c| c.id == id)
                .map(|c| format!("@{}", c.name)),
            SmartSelectionTarget::Beam(id) => project
                .runs
                .iter()
                .find(|r| r.id == id)
                .map(|r| format!("@{}", r.name)),
            SmartSelectionTarget::Wall(id) => project
                .walls
                .iter()
                .find(|w| w.id == id)
                .map(|w| format!("@{}", w.name)),
            SmartSelectionTarget::Footing(id) => project
                .footings
                .iter()
                .find(|f| f.id == id)
                .map(|f| format!("@{}", f.name)),
            SmartSelectionTarget::SketchEntity(id) => Some(format!("@Entity#{id}")),
            SmartSelectionTarget::EarthSite { lat, lon } => {
                Some(format!("@Site({lat:.2},{lon:.2})"))
            }
        }
    }

    /// Maps the active spatial lens to `NavigationPillar` + `BuildView` diagram/color modes.
    pub fn apply_lens_to_views(
        &mut self,
        lens: SpatialLens,
        build_view: &mut BuildView,
        universal_canvas: &mut UniversalCanvasState,
    ) {
        self.active_lens = lens;
        match lens {
            SpatialLens::Clean => {
                build_view.render_mode = Render3DMode::CsgSolidBrep;
                build_view.highlight_4d_erection = false;
                universal_canvas.show_domain_overlay = false;
            }
            SpatialLens::DcrHeatmap => {
                build_view.render_mode = Render3DMode::SolidShaded;
                build_view.color_mode = "DCR%".to_string();
                build_view.highlight_4d_erection = false;
                universal_canvas.show_domain_overlay = true;
            }
            SpatialLens::AriRedundancy => {
                build_view.render_mode = Render3DMode::SolidShaded;
                build_view.color_mode = "ARI".to_string();
                build_view.highlight_4d_erection = false;
                universal_canvas.show_domain_overlay = true;
            }
            SpatialLens::AxialForce => {
                build_view.render_mode = Render3DMode::AxialForceNFD;
                build_view.highlight_4d_erection = false;
            }
            SpatialLens::ShearForce => {
                build_view.render_mode = Render3DMode::ShearForceSFD;
                build_view.highlight_4d_erection = false;
            }
            SpatialLens::BendingMoment => {
                build_view.render_mode = Render3DMode::BendingMomentBMD;
                build_view.highlight_4d_erection = false;
            }
            SpatialLens::Erection4D => {
                build_view.render_mode = Render3DMode::CsgSolidBrep;
                build_view.highlight_4d_erection = true;
                build_view.simulation_4d_day = self.active_4d_day as usize;
                universal_canvas.show_domain_overlay = true;
            }
        }
    }

    /// Executes any `/` slash command, `@Element` directive, or natural-language AI query.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_spatial_input(
        &mut self,
        raw_input: &str,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();
        let trimmed = raw_input.trim();
        if trimmed.is_empty() {
            return actions;
        }
        let lower = trimmed.to_lowercase();

        // 1. Slash Command Dispatch
        if lower.starts_with('/') {
            let mut parts = trimmed.splitn(2, char::is_whitespace);
            let cmd = parts.next().unwrap_or("").to_lowercase();
            let arg = parts.next().unwrap_or("").trim();

            match cmd.as_str() {
                "" | "/" => {
                    self.command_input = "/".to_string();
                    self.request_focus_command = true;
                }
                "/earth" | "/globe" => {
                    self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                    universal_canvas.globe_mode = true;
                    universal_canvas.active_tool = UniversalCanvasTool::SelectPan;
                    self.push_trace("Switched to 3D Earth Globe", true);
                }
                "/2d" | "/plan" | "/map" => {
                    self.active_canvas_mode = SpatialCanvasMode::EarthPlan2D;
                    universal_canvas.globe_mode = false;
                    universal_canvas.active_tool = UniversalCanvasTool::SelectPan;
                    actions.fit_camera = true;
                    self.push_trace("Switched to 2D Plan & Earth Map", true);
                }
                "/3d" | "/bim" => {
                    self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                    build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                    actions.fit_camera = true;
                    self.push_trace("Switched to 3D Structural BIM", true);
                }
                "/fit" | "/zoom" => {
                    actions.fit_camera = true;
                    self.push_trace("Fitted camera to active model", true);
                }
                "/classic" | "/sidebar" => {
                    actions.switch_to_classic_ui = true;
                    self.push_trace("Switched to Classic 15-Pillar UI", true);
                }
                "/sample" | "/demo" => {
                    actions.load_sample_building = true;
                    self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                    self.push_trace("Loaded BIMCraft 3D Building on Earth", true);
                }
                "/portal" | "/frame" | "/box" | "/grid" | "/col" | "/rebar" | "/clear" => {
                    let nl_cmd = trimmed.trim_start_matches('/');
                    match build_view.bim_bus.execute_natural_language(nl_cmd) {
                        Ok(msg) => {
                            if nl_cmd == "clear" {
                                self.selected_target = None;
                                universal_canvas.highlighted_bim_element = None;
                            } else {
                                self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                                universal_canvas.ground_origin_anchored = true;
                            }
                            actions.sync_fem_and_earth = true;
                            actions.fit_camera = true;
                            self.push_trace(msg.clone(), true);
                            ai_assistant.chat_history.push(crate::models::ChatMessage {
                                is_user: false,
                                message: format!("[Spatial Command]: {msg}"),
                                time: crate::core::web_bridge::unix_timestamp_label(),
                            });
                        }
                        Err(err) => {
                            self.push_trace(err.clone(), false);
                        }
                    }
                }
                "/column" => {
                    self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                    build_view.ground_author_tool = Ground3DAuthorTool::Column;
                    self.push_trace("Column placement tool active (click ground)", true);
                }
                "/beam" => {
                    self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                    build_view.ground_author_tool = Ground3DAuthorTool::Beam;
                    self.push_trace("Beam placement tool active (click 2 pts)", true);
                }
                "/wall" => {
                    self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                    build_view.ground_author_tool = Ground3DAuthorTool::Wall;
                    self.push_trace("Wall placement tool active (click 2 pts)", true);
                }
                "/footing" => {
                    self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                    build_view.ground_author_tool = Ground3DAuthorTool::Footing;
                    self.push_trace("Footing placement tool active (click ground)", true);
                }
                "/line" => {
                    if self.active_canvas_mode == SpatialCanvasMode::Bim3D {
                        self.active_canvas_mode = SpatialCanvasMode::EarthPlan2D;
                        universal_canvas.globe_mode = false;
                    }
                    universal_canvas.active_tool = UniversalCanvasTool::Line;
                    self.push_trace("2D CAD Line tool active", true);
                }
                "/measure" => {
                    universal_canvas.measure_active = !universal_canvas.measure_active;
                    if !universal_canvas.measure_active {
                        universal_canvas.measure_points_lat_lon.clear();
                    }
                    self.push_trace(
                        if universal_canvas.measure_active {
                            "Geodesic Earth Measure ON"
                        } else {
                            "Geodesic Earth Measure OFF"
                        },
                        true,
                    );
                }
                "/lens" => {
                    let arg_l = arg.to_lowercase();
                    let next_lens = if arg_l.contains("dcr") || arg_l.contains("util") {
                        SpatialLens::DcrHeatmap
                    } else if arg_l.contains("ari") || arg_l.contains("red") {
                        SpatialLens::AriRedundancy
                    } else if arg_l.contains("mom") || arg_l.contains("bmd") {
                        SpatialLens::BendingMoment
                    } else if arg_l.contains("shear") || arg_l.contains("sfd") {
                        SpatialLens::ShearForce
                    } else if arg_l.contains("ax") || arg_l.contains("nfd") {
                        SpatialLens::AxialForce
                    } else if arg_l.contains("4d") || arg_l.contains("time") {
                        SpatialLens::Erection4D
                    } else {
                        SpatialLens::Clean
                    };
                    self.apply_lens_to_views(next_lens, build_view, universal_canvas);
                    if next_lens != SpatialLens::Clean && self.active_canvas_mode == SpatialCanvasMode::EarthGlobe {
                        self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                    }
                    self.push_trace(format!("Lens: {}", next_lens.label()), true);
                }
                "/fly" | "/go" | "/loc" => {
                    if let Some((label, lat, lon, zoom)) =
                        crate::core::gis::SlippyMapState::resolve_location_query(arg)
                    {
                        self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                        universal_canvas.globe_mode = true;
                        universal_canvas
                            .earth_world
                            .start_fly_to(&universal_canvas.world_map, lat, lon, zoom, &label);
                        universal_canvas.ground_origin_lat_lon = (lat, lon);
                        universal_canvas.ground_origin_anchored = true;
                        self.selected_target = Some(SmartSelectionTarget::EarthSite { lat, lon });
                        self.push_trace(format!("Flying Earth -> {label}"), true);
                    } else {
                        self.push_trace("Usage: /fly <city or lat, lon>", false);
                    }
                }
                "/repair" => {
                    actions.run_auto_repair = true;
                    self.command_bar_expanded = true;
                    self.push_trace("Executed Closed-Loop AI Repair", true);
                }
                "/svd" | "/ari" => {
                    self.command_bar_expanded = true;
                    ai_assistant.execute_prompt(
                        "check svd mechanism and screen ari",
                        NavigationPillar::AnalysisSimulation,
                        members,
                        projects,
                        db_components,
                    );
                    self.push_trace("Ran SVD Kinematic Guard & ARI Audit", true);
                }
                "/undo" => {
                    if let Some(msg) = build_view.bim_bus.undo() {
                        actions.sync_fem_and_earth = true;
                        self.push_trace(msg, true);
                    } else if universal_canvas.undo() {
                        self.push_trace("Undid canvas stroke", true);
                    } else {
                        self.push_trace("Nothing to undo", false);
                    }
                }
                "/redo" => {
                    if let Some(msg) = build_view.bim_bus.redo() {
                        actions.sync_fem_and_earth = true;
                        self.push_trace(msg, true);
                    } else if universal_canvas.redo() {
                        self.push_trace("Redid canvas stroke", true);
                    } else {
                        self.push_trace("Nothing to redo", false);
                    }
                }
                "/save" | "/export" => {
                    actions.export_archive = true;
                    self.push_trace("Exported .eigen.json workspace", true);
                }
                "/theme" => {
                    actions.toggle_theme = true;
                    self.push_trace("Toggled Pro Dark / Engineering Daylight", true);
                }
                "/settings" => {
                    actions.open_settings = true;
                    self.push_trace("Opened Settings", true);
                }
                "/vault" => self.open_module_overlay(NavigationPillar::MyLibrary),
                "/agent" => self.open_module_overlay(NavigationPillar::AiAgent),
                "/planning" => self.open_module_overlay(NavigationPillar::ResearchConcept),
                "/drafting" => self.open_module_overlay(NavigationPillar::ParametricDED),
                "/geometry" => self.open_module_overlay(NavigationPillar::GeometryAssembly),
                "/analysis" => self.open_module_overlay(NavigationPillar::AnalysisSimulation),
                "/schedule" => self.open_module_overlay(NavigationPillar::TimeScheduling),
                "/cost" | "/boq" | "/rab" => self.open_module_overlay(NavigationPillar::CostBudgeting),
                "/sustainability" | "/carbon" => {
                    self.open_module_overlay(NavigationPillar::SustainabilityEnergy)
                }
                "/facility" | "/shm" => self.open_module_overlay(NavigationPillar::FacilityManagement),
                "/safety" => self.open_module_overlay(NavigationPillar::SafetyHealth),
                "/construction" | "/lean" | "/qaqc" => {
                    self.open_module_overlay(NavigationPillar::LeanConstruction)
                }
                "/automation" | "/ifc" | "/dfma" => {
                    self.open_module_overlay(NavigationPillar::IndustrializedAutomation)
                }
                "/prediction" => self.open_module_overlay(NavigationPillar::Prediction),
                "/report" | "/pbg" => self.open_module_overlay(NavigationPillar::ReportPublishing),
                _ => {
                    // Fallback: treat unknown `/...` as natural language query
                    self.command_bar_expanded = true;
                    ai_assistant.execute_prompt(
                        trimmed.trim_start_matches('/'),
                        NavigationPillar::GeometryAssembly,
                        members,
                        projects,
                        db_components,
                    );
                    self.push_trace(format!("Ran AI prompt '{cmd}'"), true);
                }
            }
            return actions;
        }

        // 2. Direct Earth Place Navigation if query matches a city or lat,lon cleanly
        if !lower.contains("portal")
            && !lower.contains("frame")
            && !lower.contains("col")
            && !lower.contains("beam")
            && !lower.contains("repair")
            && !lower.contains("ari")
            && !lower.contains("svd")
            && !lower.contains("cost")
        {
            if let Some((label, lat, lon, zoom)) =
                crate::core::gis::SlippyMapState::resolve_location_query(trimmed)
            {
                self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                universal_canvas.globe_mode = true;
                universal_canvas
                    .earth_world
                    .start_fly_to(&universal_canvas.world_map, lat, lon, zoom, &label);
                universal_canvas.ground_origin_lat_lon = (lat, lon);
                universal_canvas.ground_origin_anchored = true;
                self.selected_target = Some(SmartSelectionTarget::EarthSite { lat, lon });
                self.push_trace(format!("Earth Fly-To -> {label}"), true);
                return actions;
            }
        }

        // 3. Natural Language BIM Synthesis or Selected-Element Context Prompt
        if lower.contains("build portal")
            || lower.contains("create portal")
            || lower.starts_with("portal ")
            || lower.starts_with("frame ")
            || lower.starts_with("box ")
        {
            let norm = trimmed
                .replace("build ", "")
                .replace("create ", "")
                .replace("stories", "floors")
                .replace("story", "floors");
            match build_view.bim_bus.execute_natural_language(&norm) {
                Ok(msg) => {
                    self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                    universal_canvas.ground_origin_anchored = true;
                    actions.sync_fem_and_earth = true;
                    actions.fit_camera = true;
                    self.push_trace(msg.clone(), true);
                    ai_assistant.chat_history.push(crate::models::ChatMessage {
                        is_user: true,
                        message: trimmed.to_string(),
                        time: crate::core::web_bridge::unix_timestamp_label(),
                    });
                    ai_assistant.chat_history.push(crate::models::ChatMessage {
                        is_user: false,
                        message: format!("[Antigravity BIM Agent]: {msg}"),
                        time: crate::core::web_bridge::unix_timestamp_label(),
                    });
                    return actions;
                }
                Err(_) => {}
            }
        }

        // Inject live selected element context when user references `@` or asks about selected column
        let enriched_prompt = if let Some(ctx_block) =
            self.format_selected_context_summary(&build_view.bim_bus.project, members, universal_canvas)
        {
            if trimmed.contains('@') || lower.contains("this column") || lower.contains("this element") || lower.contains("selected") {
                format!("{trimmed}\n\n[Active Spatial Context: {ctx_block}]")
            } else {
                trimmed.to_string()
            }
        } else {
            trimmed.to_string()
        };

        self.command_bar_expanded = true;
        ai_assistant.execute_prompt(
            &enriched_prompt,
            self.active_overlay_pillar
                .unwrap_or(NavigationPillar::GeometryAssembly),
            members,
            projects,
            db_components,
        );
        if ai_assistant.pending_bim_nl_command.as_deref() == Some("__repair_critical__") {
            ai_assistant.pending_bim_nl_command = None;
            actions.run_auto_repair = true;
        } else if let Some(cmd) = ai_assistant.pending_bim_nl_command.take() {
            if let Ok(msg) = build_view.bim_bus.execute_natural_language(&cmd) {
                self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                actions.sync_fem_and_earth = true;
                actions.fit_camera = true;
                self.push_trace(msg, true);
            }
        } else {
            self.push_trace("EigenAI responded", true);
        }

        actions
    }

    fn open_module_overlay(&mut self, pillar: NavigationPillar) {
        self.active_overlay_pillar = Some(pillar);
        self.command_bar_expanded = false;
        self.push_trace(format!("Opened {} sheet", pillar.clean_name()), true);
    }

    fn format_selected_context_summary(
        &self,
        project: &BimProject,
        members: &[StructuralMember],
        universal_canvas: &UniversalCanvasState,
    ) -> Option<String> {
        match self.selected_target? {
            SmartSelectionTarget::Column(id) => {
                let col = project.columns.iter().find(|c| c.id == id)?;
                let fem_m = project
                    .fem_id_for_bim_element(BimElementKind::Column, id)
                    .and_then(|fid| members.iter().find(|m| m.id == fid));
                let dcr = fem_m.map(|m| m.utilization * 100.0).unwrap_or(42.0);
                let ari = fem_m.map(|m| m.ari).unwrap_or(0.89);
                Some(format!(
                    "Column {} ({:.0}x{:.0}mm, {}), DCR={:.1}%, ARI={:.3}",
                    col.name, col.width_mm, col.depth_mm, col.profile_name, dcr, ari
                ))
            }
            SmartSelectionTarget::Beam(id) => {
                let r = project.runs.iter().find(|r| r.id == id)?;
                let fem_m = project
                    .fem_id_for_bim_element(BimElementKind::Run, id)
                    .and_then(|fid| members.iter().find(|m| m.id == fid));
                let dcr = fem_m.map(|m| m.utilization * 100.0).unwrap_or(45.0);
                let ari = fem_m.map(|m| m.ari).unwrap_or(0.87);
                Some(format!(
                    "Beam {} ({:.0}x{:.0}mm), DCR={:.1}%, ARI={:.3}",
                    r.name, r.width_mm, r.depth_mm, dcr, ari
                ))
            }
            SmartSelectionTarget::Wall(id) => {
                let w = project.walls.iter().find(|w| w.id == id)?;
                Some(format!(
                    "Wall {} (L={:.0}mm, t={:.0}mm, structural={})",
                    w.name,
                    w.length_mm(),
                    w.thickness_mm,
                    w.is_structural
                ))
            }
            SmartSelectionTarget::Footing(id) => {
                let f = project.footings.iter().find(|f| f.id == id)?;
                Some(format!(
                    "Footing {} ({:.0}x{:.0}x{:.0}mm)",
                    f.name, f.width_mm, f.length_mm, f.thickness_mm
                ))
            }
            SmartSelectionTarget::EarthSite { lat, lon } => {
                let site = compute_site_environmental_coupling(lat, lon, &universal_canvas.earth_world);
                Some(format!(
                    "Earth Site ({:.3}°, {:.3}°), Elev {:.0}m, PGA Ss={:.2}g, Wind {:.0}m/s",
                    lat, lon, site.elevation_m, site.seismic_pga_ss_g, site.design_wind_ms
                ))
            }
            SmartSelectionTarget::SketchEntity(id) => Some(format!("CAD Entity #{id}")),
        }
    }

    /// Antigravity frosted translucent surface frame used for all spatial HUD overlays.
    fn antigravity_glass_frame(theme: &MacTheme, corner_r: u8, pad: Margin) -> Frame {
        let fill = if theme.is_dark() {
            Color32::from_rgba_premultiplied(15, 19, 28, 240)
        } else {
            Color32::from_rgba_premultiplied(252, 253, 255, 245)
        };
        Frame::new()
            .fill(fill)
            .corner_radius(CornerRadius::same(corner_r))
            .stroke(Stroke::new(1.1, theme.accent_blue.gamma_multiply(0.42)))
            .inner_margin(pad)
    }

    /// Renders all Antigravity floating HUD layers:
    /// 1. Minimal Top-Left Identity & Mode Capsule (zero permanent sidebars/ribbons)
    /// 2. Contextual Top-Center Lens / 4D Day Pill (only when active)
    /// 3. Smart Context Pop-Up Inspector (only when an element or Earth location is clicked)
    /// 4. Bottom-Center Antigravity AI Assistant & `/` Command Bar
    #[allow(clippy::too_many_arguments)]
    pub fn render_hud_overlays(
        &mut self,
        ctx: &egui::Context,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();
        self.hud_rects.clear();

        let vp = MacTheme::viewport_size(ctx);
        let is_mobile = vp.x < MacTheme::MOBILE_BREAKPOINT;

        // 1. Top-Left Minimal Antigravity Capsule
        let top_left_resp = Area::new(Id::new("antigravity_top_left_capsule"))
            .order(Order::Foreground)
            .anchor(Align2::LEFT_TOP, Vec2::new(12.0, 12.0))
            .show(ctx, |ui| {
                Self::antigravity_glass_frame(theme, 20, Margin::symmetric(12, 6)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        icons::icon_label(
                            ui,
                            AppIcon::Sparkles,
                            14.0,
                            theme.accent_blue,
                            "EigenGraph",
                            12.5,
                            theme.text_primary,
                            true,
                        );
                        ui.add_space(4.0);

                        // Cycle Earth 3D / BIM 3D / Plan 2D with 1 click
                        let mode_lbl = self.active_canvas_mode.label();
                        if icons::icon_button(
                            ui,
                            self.active_canvas_mode.icon(),
                            mode_lbl,
                            10.5,
                            theme.accent_blue,
                            theme.accent_blue.gamma_multiply(0.14),
                            Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.35)),
                            CornerRadius::same(12),
                            Vec2::new(0.0, 22.0),
                        )
                        .on_hover_text("Switch spatial view (Earth 3D -> BIM 3D -> Plan 2D)")
                        .clicked()
                        {
                            self.active_canvas_mode = match self.active_canvas_mode {
                                SpatialCanvasMode::EarthGlobe => SpatialCanvasMode::Bim3D,
                                SpatialCanvasMode::Bim3D => SpatialCanvasMode::EarthPlan2D,
                                SpatialCanvasMode::EarthPlan2D => SpatialCanvasMode::EarthGlobe,
                            };
                            universal_canvas.globe_mode =
                                self.active_canvas_mode == SpatialCanvasMode::EarthGlobe;
                            if self.active_canvas_mode != SpatialCanvasMode::EarthGlobe {
                                actions.fit_camera = true;
                            }
                        }

                        if icons::icon_button(
                            ui,
                            AppIcon::SlidersPanel,
                            "/",
                            11.0,
                            theme.text_primary,
                            theme.bg_card,
                            Stroke::new(1.0, theme.border_subtle),
                            CornerRadius::same(12),
                            Vec2::new(24.0, 22.0),
                        )
                        .on_hover_text("Open Slash Command Palette (Cmd/Ctrl+K)")
                        .clicked()
                        {
                            self.command_input = "/".to_string();
                            self.command_bar_expanded = true;
                            self.request_focus_command = true;
                        }
                    });
                });
            });
        self.hud_rects.push(top_left_resp.response.rect);

        // 2. Conditional Top-Center Active Lens / 4D Timeline Pill (hidden in Clean mode)
        if self.active_lens != SpatialLens::Clean
            || build_view.ground_author_tool != Ground3DAuthorTool::Orbit
            || universal_canvas.active_tool != UniversalCanvasTool::SelectPan
            || universal_canvas.measure_active
        {
            let top_center_offset = if is_mobile {
                Vec2::new(0.0, 52.0)
            } else {
                Vec2::new(0.0, 12.0)
            };
            let pill_resp = Area::new(Id::new("antigravity_active_lens_pill"))
                .order(Order::Foreground)
                .anchor(Align2::CENTER_TOP, top_center_offset)
                .show(ctx, |ui| {
                    Self::antigravity_glass_frame(theme, 16, Margin::symmetric(12, 5)).show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            if self.active_lens != SpatialLens::Clean {
                                icons::pill_badge(
                                    ui,
                                    AppIcon::Target,
                                    format!("Lens: {}", self.active_lens.label()),
                                    theme.accent_blue,
                                );
                                if self.active_lens == SpatialLens::Erection4D {
                                    ui.add(
                                        egui::Slider::new(&mut self.active_4d_day, 1..=180)
                                            .text("Day"),
                                    );
                                    build_view.simulation_4d_day = self.active_4d_day as usize;
                                }
                            }
                            if build_view.ground_author_tool != Ground3DAuthorTool::Orbit {
                                icons::pill_badge(
                                    ui,
                                    AppIcon::Wrench,
                                    format!("Tool: {}", build_view.ground_author_tool.label()),
                                    theme.accent_orange,
                                );
                            } else if universal_canvas.active_tool != UniversalCanvasTool::SelectPan {
                                icons::pill_badge(
                                    ui,
                                    universal_canvas.active_tool.app_icon(),
                                    format!("Tool: {}", universal_canvas.active_tool.short_label()),
                                    theme.accent_orange,
                                );
                            }
                            if universal_canvas.measure_active {
                                icons::pill_badge(
                                    ui,
                                    AppIcon::DimensionRuler,
                                    "Geodesic Measure",
                                    theme.accent_green,
                                );
                            }
                            if icons::icon_only_button(
                                ui,
                                AppIcon::Close,
                                theme.text_secondary,
                                Color32::TRANSPARENT,
                                Stroke::NONE,
                                CornerRadius::same(8),
                                Vec2::splat(20.0),
                                "Reset active lens & tool",
                            )
                            .clicked()
                            {
                                self.apply_lens_to_views(
                                    SpatialLens::Clean,
                                    build_view,
                                    universal_canvas,
                                );
                                build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                                universal_canvas.active_tool = UniversalCanvasTool::SelectPan;
                                universal_canvas.measure_active = false;
                            }
                        });
                    });
                });
            self.hud_rects.push(pill_resp.response.rect);
        }

        // 3. Smart Element Pop-Up Inspector (ONLY visible when user clicked a Column, Beam, Wall, Footing, or Earth Site)
        if let Some(target) = self.selected_target {
            let popup_actions = self.render_smart_element_popup(
                ctx,
                theme,
                target,
                universal_canvas,
                build_view,
                ai_assistant,
                members,
                projects,
                db_components,
                vp,
                is_mobile,
            );
            actions.merge(popup_actions);
        }

        // 4. Bottom-Center Antigravity AI Assistant & `/` Command Bar
        let bar_actions = self.render_antigravity_command_dock(
            ctx,
            theme,
            universal_canvas,
            build_view,
            ai_assistant,
            members,
            projects,
            db_components,
            vp,
            is_mobile,
        );
        actions.merge(bar_actions);

        actions
    }

    #[allow(clippy::too_many_arguments)]
    fn render_smart_element_popup(
        &mut self,
        ctx: &egui::Context,
        theme: &MacTheme,
        target: SmartSelectionTarget,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
        vp: Vec2,
        is_mobile: bool,
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();

        // Sync editable draft inputs lazily when selection changes
        if self.last_loaded_target != Some(target) {
            self.sync_edit_draft_from_target(target, &build_view.bim_bus.project);
            self.last_loaded_target = Some(target);
        }

        let card_w = if is_mobile {
            (vp.x - 18.0).clamp(240.0, 380.0)
        } else {
            (vp.x * 0.34).clamp(310.0, 385.0)
        };
        let inner_w = (card_w - 28.0).max(200.0);
        let max_h = if is_mobile {
            (vp.y * 0.44).clamp(190.0, 340.0)
        } else {
            (vp.y - 125.0).clamp(260.0, 520.0)
        };
        let (anchor, offset) = if is_mobile {
            (Align2::CENTER_TOP, Vec2::new(0.0, 54.0))
        } else {
            (Align2::RIGHT_TOP, Vec2::new(-14.0, 14.0))
        };

        let mut close_popup = false;
        let area_resp = Area::new(Id::new("antigravity_smart_element_popup"))
            .order(Order::Foreground)
            .anchor(anchor, offset)
            .constrain(true)
            .show(ctx, |ui| {
                Self::antigravity_glass_frame(theme, 14, Margin::same(12)).show(ui, |ui| {
                    ui.set_width(inner_w);
                    ui.set_max_width(inner_w);
                    ui.set_max_height(max_h);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                    // Header bar with element identity, Pin toggle, and Close button
                    let (title, subtitle, elem_icon) =
                        Self::describe_target(target, &build_view.bim_bus.project);
                    ui.horizontal(|ui| {
                        icons::icon_label(
                            ui,
                            elem_icon,
                            15.0,
                            theme.accent_blue,
                            &title,
                            13.0,
                            theme.text_primary,
                            true,
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if icons::icon_only_button(
                                ui,
                                AppIcon::Close,
                                theme.text_secondary,
                                Color32::TRANSPARENT,
                                Stroke::NONE,
                                CornerRadius::same(6),
                                Vec2::splat(22.0),
                                "Close pop-up (Esc)",
                            )
                            .clicked()
                            {
                                close_popup = true;
                            }
                            let pin_bg = if self.popup_pinned {
                                theme.accent_blue.gamma_multiply(0.22)
                            } else {
                                Color32::TRANSPARENT
                            };
                            if icons::icon_button(
                                ui,
                                AppIcon::MapPin,
                                if self.popup_pinned { "Pinned" } else { "Pin" },
                                10.0,
                                if self.popup_pinned {
                                    theme.accent_blue
                                } else {
                                    theme.text_muted
                                },
                                pin_bg,
                                Stroke::NONE,
                                CornerRadius::same(6),
                                Vec2::new(0.0, 22.0),
                            )
                            .on_hover_text("Pin inspector open across canvas clicks")
                            .clicked()
                            {
                                self.popup_pinned = !self.popup_pinned;
                            }
                        });
                    });
                    ui.label(RichText::new(subtitle).size(10.5).color(theme.text_secondary));
                    ui.add_space(6.0);

                    // Context Tab Switcher (Properties | Calc | 5D/6D | Ask AI)
                    if !matches!(target, SmartSelectionTarget::EarthSite { .. } | SmartSelectionTarget::SketchEntity(_)) {
                        ui.horizontal_wrapped(|ui| {
                            for tab in [
                                SmartPopupTab::Properties,
                                SmartPopupTab::Calculations,
                                SmartPopupTab::Lifecycle5D6D,
                                SmartPopupTab::AskAi,
                            ] {
                                let active = self.popup_tab == tab;
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(tab.label()).size(10.5).color(if active {
                                                Color32::WHITE
                                            } else {
                                                theme.text_secondary
                                            }),
                                        )
                                        .fill(if active {
                                            theme.accent_blue
                                        } else {
                                            theme.bg_card
                                        })
                                        .corner_radius(CornerRadius::same(8))
                                        .min_size(Vec2::new(56.0, 22.0)),
                                    )
                                    .clicked()
                                {
                                    self.popup_tab = tab;
                                }
                            }
                        });
                        ui.add_space(6.0);
                    }

                    ui.separator();
                    ui.add_space(4.0);

                    ScrollArea::vertical()
                        .id_salt("smart_popup_scroll")
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            let body_actions = self.render_smart_popup_body(
                                ui,
                                theme,
                                target,
                                universal_canvas,
                                build_view,
                                ai_assistant,
                                members,
                                projects,
                                db_components,
                            );
                            actions.merge(body_actions);
                        });
                });
            });
        self.hud_rects.push(area_resp.response.rect);

        if close_popup {
            self.selected_target = None;
            self.popup_pinned = false;
            universal_canvas.highlighted_bim_element = None;
            universal_canvas.last_picked_earth_lat_lon = None;
        }

        actions
    }

    fn sync_edit_draft_from_target(&mut self, target: SmartSelectionTarget, project: &BimProject) {
        match target {
            SmartSelectionTarget::Column(id) => {
                if let Some(c) = project.columns.iter().find(|c| c.id == id) {
                    self.edit_width_mm = c.width_mm;
                    self.edit_depth_mm = c.depth_mm;
                    self.edit_rot_deg = c.rotation_deg;
                    self.edit_profile_name = c.profile_name.clone();
                }
            }
            SmartSelectionTarget::Beam(id) => {
                if let Some(r) = project.runs.iter().find(|r| r.id == id) {
                    self.edit_width_mm = r.width_mm;
                    self.edit_depth_mm = r.depth_mm;
                    self.edit_profile_name = r.profile_name.clone();
                }
            }
            SmartSelectionTarget::Footing(id) => {
                if let Some(f) = project.footings.iter().find(|f| f.id == id) {
                    self.edit_width_mm = f.width_mm;
                    self.edit_length_mm = f.length_mm;
                    self.edit_thickness_mm = f.thickness_mm;
                }
            }
            SmartSelectionTarget::Wall(id) => {
                if let Some(w) = project.walls.iter().find(|w| w.id == id) {
                    self.edit_thickness_mm = w.thickness_mm;
                    self.edit_is_structural = w.is_structural;
                }
            }
            _ => {}
        }
    }

    fn describe_target(
        target: SmartSelectionTarget,
        project: &BimProject,
    ) -> (String, String, AppIcon) {
        match target {
            SmartSelectionTarget::Column(id) => {
                if let Some(c) = project.columns.iter().find(|c| c.id == id) {
                    let z0 = project.level_elevation(c.base_level_id) / 1000.0;
                    let z1 = project.level_elevation(c.top_level_id) / 1000.0;
                    (
                        c.name.clone(),
                        format!(
                            "Column • {:.0}x{:.0} mm • Elev {:.1}m -> {:.1}m",
                            c.width_mm, c.depth_mm, z0, z1
                        ),
                        AppIcon::Column,
                    )
                } else {
                    (format!("Column #{id}"), "Removed or missing".to_string(), AppIcon::Column)
                }
            }
            SmartSelectionTarget::Beam(id) => {
                if let Some(r) = project.runs.iter().find(|r| r.id == id) {
                    (
                        r.name.clone(),
                        format!(
                            "Beam/Run • L={:.2}m • {:.0}x{:.0} mm",
                            r.start.dist(r.end) * 0.001,
                            r.width_mm,
                            r.depth_mm
                        ),
                        AppIcon::Beam,
                    )
                } else {
                    (format!("Beam #{id}"), "Removed or missing".to_string(), AppIcon::Beam)
                }
            }
            SmartSelectionTarget::Wall(id) => {
                if let Some(w) = project.walls.iter().find(|w| w.id == id) {
                    (
                        w.name.clone(),
                        format!(
                            "Wall • L={:.2}m • t={:.0} mm • {} opening(s)",
                            w.length_mm() * 0.001,
                            w.thickness_mm,
                            w.openings.len()
                        ),
                        AppIcon::Wall,
                    )
                } else {
                    (format!("Wall #{id}"), "Removed or missing".to_string(), AppIcon::Wall)
                }
            }
            SmartSelectionTarget::Footing(id) => {
                if let Some(f) = project.footings.iter().find(|f| f.id == id) {
                    (
                        f.name.clone(),
                        format!(
                            "Isolated Footing • {:.0}x{:.0}x{:.0} mm",
                            f.width_mm, f.length_mm, f.thickness_mm
                        ),
                        AppIcon::Slab,
                    )
                } else {
                    (format!("Footing #{id}"), "Removed or missing".to_string(), AppIcon::Slab)
                }
            }
            SmartSelectionTarget::SketchEntity(id) => (
                format!("CAD Sketch #{id}"),
                "2D Vector Annotation Entity".to_string(),
                AppIcon::Pencil,
            ),
            SmartSelectionTarget::EarthSite { lat, lon } => (
                "Earth Site Location".to_string(),
                format!("WGS84 Lat {:.4}°, Lon {:.4}°", lat, lon),
                AppIcon::Globe,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_smart_popup_body(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        target: SmartSelectionTarget,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();
        let site = universal_canvas.active_site_coupling();

        match target {
            SmartSelectionTarget::EarthSite { lat, lon } => {
                let picked_site =
                    compute_site_environmental_coupling(lat, lon, &universal_canvas.earth_world);
                Self::render_kv_row(
                    ui,
                    theme,
                    "Ground Elevation",
                    &format!("{:+.1} m ASL", picked_site.elevation_m),
                    theme.text_primary,
                );
                Self::render_kv_row(
                    ui,
                    theme,
                    "Seismic Hazard (Ss / S1)",
                    &format!(
                        "{:.2}g / {:.2}g",
                        picked_site.seismic_pga_ss_g, picked_site.seismic_pga_s1_g
                    ),
                    theme.accent_orange,
                );
                Self::render_kv_row(
                    ui,
                    theme,
                    "Design Wind Speed",
                    &format!("{:.1} m/s", picked_site.design_wind_ms),
                    theme.accent_blue,
                );
                Self::render_kv_row(
                    ui,
                    theme,
                    "Flood Risk Index",
                    &format!("{:.0}%", picked_site.flood_risk_index * 100.0),
                    if picked_site.flood_risk_index > 0.55 {
                        theme.accent_red
                    } else {
                        theme.accent_green
                    },
                );
                Self::render_kv_row(
                    ui,
                    theme,
                    "Soil / Foundation",
                    &picked_site.recommended_soil_class,
                    theme.text_secondary,
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(picked_site.foundation_advisory)
                        .size(10.0)
                        .color(theme.text_muted),
                );
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    if icons::icon_button(
                        ui,
                        AppIcon::MapPin,
                        "Anchor Site Here",
                        10.5,
                        Color32::WHITE,
                        theme.accent_blue,
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::new(0.0, 26.0),
                    )
                    .clicked()
                    {
                        universal_canvas.ground_origin_lat_lon = (lat, lon);
                        universal_canvas.ground_origin_anchored = true;
                        universal_canvas
                            .earth_world
                            .start_fly_to(&universal_canvas.world_map, lat, lon, 16.5, "Anchored Site");
                        self.push_trace(format!("Site anchored @ {lat:.3}°, {lon:.3}°"), true);
                    }
                    if icons::icon_button(
                        ui,
                        AppIcon::Grid,
                        "Build 3x2 Portal",
                        10.5,
                        Color32::WHITE,
                        theme.accent_green,
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::new(0.0, 26.0),
                    )
                    .clicked()
                    {
                        universal_canvas.ground_origin_lat_lon = (lat, lon);
                        universal_canvas.ground_origin_anchored = true;
                        if let Ok(msg) = build_view
                            .bim_bus
                            .execute_natural_language("portal 3x2 @6m 2floors")
                        {
                            self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                            actions.sync_fem_and_earth = true;
                            actions.fit_camera = true;
                            self.push_trace(msg, true);
                        }
                    }
                    if icons::icon_button(
                        ui,
                        AppIcon::Building,
                        "Load BIMCraft Sample",
                        10.5,
                        theme.text_primary,
                        theme.bg_card,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(7),
                        Vec2::new(0.0, 26.0),
                    )
                    .clicked()
                    {
                        universal_canvas.ground_origin_lat_lon = (lat, lon);
                        universal_canvas.ground_origin_anchored = true;
                        actions.load_sample_building = true;
                        self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                    }
                });
            }
            SmartSelectionTarget::SketchEntity(id) => {
                Self::render_kv_row(
                    ui,
                    theme,
                    "Entity ID",
                    &format!("#{id}"),
                    theme.accent_blue,
                );
                ui.add_space(6.0);
                if icons::icon_button(
                    ui,
                    AppIcon::Building,
                    "Promote to 3D BIM",
                    10.5,
                    Color32::WHITE,
                    theme.accent_blue,
                    Stroke::NONE,
                    CornerRadius::same(7),
                    Vec2::new(0.0, 26.0),
                )
                .clicked()
                {
                    let (w, r, s) =
                        universal_canvas.promote_selection_to_bim(&mut build_view.bim_bus.project);
                    if w + r + s > 0 {
                        actions.sync_fem_and_earth = true;
                        self.push_trace(format!("Promoted +{w}W +{r}B +{s}S to BIM"), true);
                    }
                }
            }
            SmartSelectionTarget::Column(col_id) => {
                let col_opt = build_view
                    .bim_bus
                    .project
                    .columns
                    .iter()
                    .find(|c| c.id == col_id)
                    .cloned();
                let Some(col) = col_opt else {
                    ui.label("Column not found in active model.");
                    return actions;
                };
                let fem_id = build_view
                    .bim_bus
                    .project
                    .fem_id_for_bim_element(BimElementKind::Column, col_id);
                let fem_member = fem_id.and_then(|fid| members.iter().find(|m| m.id == fid));
                let height_m = ((build_view.bim_bus.project.level_elevation(col.top_level_id)
                    - build_view.bim_bus.project.level_elevation(col.base_level_id))
                .abs()
                .max(3000.0))
                    * 0.001;

                self.render_structural_kpi_strip(ui, theme, fem_member);
                ui.add_space(6.0);

                match self.popup_tab {
                    SmartPopupTab::Properties => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Width (mm):").size(11.0).color(theme.text_secondary));
                            ui.add(
                                egui::DragValue::new(&mut self.edit_width_mm)
                                    .range(150.0..=2000.0)
                                    .speed(10.0),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Depth (mm):").size(11.0).color(theme.text_secondary));
                            ui.add(
                                egui::DragValue::new(&mut self.edit_depth_mm)
                                    .range(150.0..=2000.0)
                                    .speed(10.0),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Rotation (°):").size(11.0).color(theme.text_secondary));
                            ui.add(
                                egui::DragValue::new(&mut self.edit_rot_deg)
                                    .range(-180.0..=180.0)
                                    .speed(5.0),
                            );
                        });
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            for (lbl, w, d) in [
                                ("350x350", 350.0, 350.0),
                                ("400x400", 400.0, 400.0),
                                ("500x500", 500.0, 500.0),
                                ("600x600", 600.0, 600.0),
                            ] {
                                if ui
                                    .add(
                                        egui::Button::new(RichText::new(lbl).size(10.0))
                                            .corner_radius(CornerRadius::same(6)),
                                    )
                                    .clicked()
                                {
                                    self.edit_width_mm = w;
                                    self.edit_depth_mm = d;
                                    self.edit_profile_name = format!("RC {w:.0}x{d:.0}");
                                }
                            }
                        });
                        ui.add_space(6.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::CheckCircle,
                            "Apply Column Resize",
                            11.0,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 28.0),
                        )
                        .clicked()
                        {
                            let new_prof = format!("RC {:.0}x{:.0}", self.edit_width_mm, self.edit_depth_mm);
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::ResizeColumn {
                                column_id: col_id,
                                width_mm: self.edit_width_mm,
                                depth_mm: self.edit_depth_mm,
                                rotation_deg: self.edit_rot_deg,
                                profile_name: new_prof,
                            }) {
                                actions.sync_fem_and_earth = true;
                                self.push_trace(msg, true);
                            }
                        }
                    }
                    SmartPopupTab::Calculations => {
                        let force_kn = fem_member.map(|m| m.force_kn).unwrap_or(-420.0);
                        let cap_kn = fem_member.map(|m| m.capacity_kn).unwrap_or(1450.0);
                        let util = fem_member.map(|m| m.utilization).unwrap_or(0.29);
                        let dcr_pmv = fem_member.map(|m| m.dcr_interaction).unwrap_or(util);
                        let ari = fem_member.map(|m| m.ari).unwrap_or(0.90);
                        let ci = fem_member.map(|m| m.ci).unwrap_or(0.10);
                        let svd_m = fem_member.map(|m| m.svd_margin).unwrap_or(0.95);
                        let r_min_mm = (col.width_mm.min(col.depth_mm) * 0.289).max(1.0);
                        let slenderness = (height_m * 1000.0) / r_min_mm;

                        Self::render_kv_row(
                            ui,
                            theme,
                            "Axial Load (Pu)",
                            &format!("{:.1} kN", force_kn),
                            theme.accent_blue,
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Nominal Capacity (ϕPn)",
                            &format!("{:.1} kN", cap_kn),
                            theme.text_primary,
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "LRFD Utilization (DCR)",
                            &format!("{:.1}%", util * 100.0),
                            if util > 0.85 { theme.accent_red } else { theme.accent_green },
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "P-M-V Second-Order DCR",
                            &format!("{:.3}", dcr_pmv),
                            theme.text_secondary,
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Redundancy Index (ARI / CI)",
                            &format!("{:.3} / {:.3}", ari, ci),
                            if ari >= 0.70 { theme.accent_green } else { theme.accent_orange },
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "SVD Kinematic Margin",
                            &format!("{:.3}", svd_m),
                            theme.text_secondary,
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Slenderness (kLu/r)",
                            &format!("{:.1} (Site Ss={:.2}g)", slenderness, site.seismic_pga_ss_g),
                            theme.text_muted,
                        );
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            if let Some(fid) = fem_id {
                                let elim = build_view.simulated_eliminated_ids.contains(&fid);
                                if icons::icon_button(
                                    ui,
                                    AppIcon::Bolt,
                                    if elim { "Restore N-1" } else { "Simulate N-1" },
                                    10.0,
                                    Color32::WHITE,
                                    if elim { theme.accent_green } else { theme.accent_red },
                                    Stroke::NONE,
                                    CornerRadius::same(7),
                                    Vec2::new(0.0, 24.0),
                                )
                                .clicked()
                                {
                                    actions.toggle_n1_member = Some(fid);
                                    self.push_trace(
                                        format!("Toggled N-1 on {}", col.name),
                                        true,
                                    );
                                }
                            }
                            if icons::icon_button(
                                ui,
                                AppIcon::Plus,
                                "Upsize +50mm",
                                10.0,
                                Color32::WHITE,
                                theme.accent_blue,
                                Stroke::NONE,
                                CornerRadius::same(7),
                                Vec2::new(0.0, 24.0),
                            )
                            .clicked()
                            {
                                let nw = col.width_mm + 50.0;
                                let nd = col.depth_mm + 50.0;
                                self.edit_width_mm = nw;
                                self.edit_depth_mm = nd;
                                if let Ok(msg) = build_view.bim_bus.execute(BimCommand::ResizeColumn {
                                    column_id: col_id,
                                    width_mm: nw,
                                    depth_mm: nd,
                                    rotation_deg: col.rotation_deg,
                                    profile_name: format!("RC {nw:.0}x{nd:.0}"),
                                }) {
                                    actions.sync_fem_and_earth = true;
                                    self.push_trace(msg, true);
                                }
                            }
                            if icons::icon_button(
                                ui,
                                AppIcon::Rebar,
                                "Auto-Rebar",
                                10.0,
                                theme.text_primary,
                                theme.bg_card,
                                Stroke::new(1.0, theme.border_subtle),
                                CornerRadius::same(7),
                                Vec2::new(0.0, 24.0),
                            )
                            .clicked()
                            {
                                if let Ok(msg) = build_view.bim_bus.execute(BimCommand::ReinforceHost {
                                    host_id: col_id,
                                    preset: Default::default(),
                                }) {
                                    self.push_trace(msg, true);
                                }
                            }
                        });
                    }
                    SmartPopupTab::Lifecycle5D6D => {
                        let vol_m3 = (col.width_mm * 0.001) * (col.depth_mm * 0.001) * height_m;
                        let rebar_kg = vol_m3 * 145.0;
                        let co2_kg = vol_m3 * 340.0 + rebar_kg * 1.15;
                        let cost_idr = (vol_m3 * 1_650_000.0 + rebar_kg * 18_500.0)
                            * site.remote_logistics_multiplier as f64;
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Concrete Volume",
                            &format!("{vol_m3:.3} m³"),
                            theme.text_primary,
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Est. Reinforcement",
                            &format!("{rebar_kg:.1} kg (ρ≈1.8%)"),
                            theme.accent_blue,
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Embodied Carbon (A1–A3)",
                            &format!("{co2_kg:.1} kgCO₂e"),
                            theme.accent_green,
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Est. Element Cost",
                            &format!("Rp {}", crate::core::cost::CostEngine::format_idr(cost_idr)),
                            theme.accent_orange,
                        );
                    }
                    SmartPopupTab::AskAi => {
                        self.render_popup_ai_actions(
                            ui,
                            theme,
                            &col.name,
                            ai_assistant,
                            members,
                            projects,
                            db_components,
                        );
                    }
                }
            }
            SmartSelectionTarget::Beam(run_id) => {
                let run_opt = build_view
                    .bim_bus
                    .project
                    .runs
                    .iter()
                    .find(|r| r.id == run_id)
                    .cloned();
                let Some(run) = run_opt else {
                    ui.label("Beam/Run not found in active model.");
                    return actions;
                };
                let fem_id = build_view
                    .bim_bus
                    .project
                    .fem_id_for_bim_element(BimElementKind::Run, run_id);
                let fem_member = fem_id.and_then(|fid| members.iter().find(|m| m.id == fid));
                let len_m = run.start.dist(run.end) * 0.001;

                self.render_structural_kpi_strip(ui, theme, fem_member);
                ui.add_space(6.0);

                match self.popup_tab {
                    SmartPopupTab::Properties => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Width (mm):").size(11.0).color(theme.text_secondary));
                            ui.add(
                                egui::DragValue::new(&mut self.edit_width_mm)
                                    .range(100.0..=1500.0)
                                    .speed(10.0),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Depth (mm):").size(11.0).color(theme.text_secondary));
                            ui.add(
                                egui::DragValue::new(&mut self.edit_depth_mm)
                                    .range(150.0..=2500.0)
                                    .speed(10.0),
                            );
                        });
                        ui.add_space(6.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::CheckCircle,
                            "Apply Beam Resize",
                            11.0,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 28.0),
                        )
                        .clicked()
                        {
                            let new_prof = format!("RC {:.0}x{:.0}", self.edit_width_mm, self.edit_depth_mm);
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::ResizeRun {
                                run_id,
                                width_mm: self.edit_width_mm,
                                depth_mm: self.edit_depth_mm,
                                profile_name: new_prof,
                            }) {
                                actions.sync_fem_and_earth = true;
                                self.push_trace(msg, true);
                            }
                        }
                    }
                    SmartPopupTab::Calculations => {
                        let force_kn = fem_member.map(|m| m.force_kn).unwrap_or(185.0);
                        let cap_kn = fem_member.map(|m| m.capacity_kn).unwrap_or(520.0);
                        let util = fem_member.map(|m| m.utilization).unwrap_or(0.36);
                        let ari = fem_member.map(|m| m.ari).unwrap_or(0.88);
                        let est_moment_knm = (force_kn.abs() * len_m * 0.125).max(12.0);
                        let span_depth = (len_m * 1000.0) / run.depth_mm.max(100.0);

                        Self::render_kv_row(ui, theme, "Clear Span (L)", &format!("{len_m:.2} m"), theme.text_primary);
                        Self::render_kv_row(ui, theme, "Span/Depth Ratio (L/h)", &format!("{span_depth:.1}"), theme.text_secondary);
                        Self::render_kv_row(ui, theme, "Est. Moment (Mu)", &format!("{est_moment_knm:.1} kNm"), theme.accent_blue);
                        Self::render_kv_row(ui, theme, "Axial/Chord Force", &format!("{force_kn:.1} / {cap_kn:.1} kN"), theme.text_secondary);
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Utilization (DCR) / ARI",
                            &format!("{:.1}% / {:.3}", util * 100.0, ari),
                            if util > 0.85 { theme.accent_red } else { theme.accent_green },
                        );
                    }
                    SmartPopupTab::Lifecycle5D6D => {
                        let vol_m3 = len_m * (run.width_mm * 0.001) * (run.depth_mm * 0.001);
                        let rebar_kg = vol_m3 * 125.0;
                        let co2_kg = vol_m3 * 335.0 + rebar_kg * 1.15;
                        let cost_idr = (vol_m3 * 1_620_000.0 + rebar_kg * 18_500.0)
                            * site.remote_logistics_multiplier as f64;
                        Self::render_kv_row(ui, theme, "Concrete Volume", &format!("{vol_m3:.3} m³"), theme.text_primary);
                        Self::render_kv_row(ui, theme, "Est. Reinforcement", &format!("{rebar_kg:.1} kg"), theme.accent_blue);
                        Self::render_kv_row(ui, theme, "Embodied Carbon", &format!("{co2_kg:.1} kgCO₂e"), theme.accent_green);
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Est. Element Cost",
                            &format!("Rp {}", crate::core::cost::CostEngine::format_idr(cost_idr)),
                            theme.accent_orange,
                        );
                    }
                    SmartPopupTab::AskAi => {
                        self.render_popup_ai_actions(
                            ui,
                            theme,
                            &run.name,
                            ai_assistant,
                            members,
                            projects,
                            db_components,
                        );
                    }
                }
            }
            SmartSelectionTarget::Wall(wall_id) => {
                let wall_opt = build_view
                    .bim_bus
                    .project
                    .walls
                    .iter()
                    .find(|w| w.id == wall_id)
                    .cloned();
                let Some(wall) = wall_opt else {
                    ui.label("Wall not found.");
                    return actions;
                };
                let len_m = wall.length_mm() * 0.001;
                let h_m = wall.unconnected_height_mm * 0.001;
                let vol_m3 = len_m * h_m * (wall.thickness_mm * 0.001);

                match self.popup_tab {
                    SmartPopupTab::Properties => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Thickness (mm):").size(11.0).color(theme.text_secondary));
                            ui.add(
                                egui::DragValue::new(&mut self.edit_thickness_mm)
                                    .range(80.0..=1000.0)
                                    .speed(10.0),
                            );
                        });
                        ui.checkbox(&mut self.edit_is_structural, "Structural Shear Wall");
                        ui.add_space(6.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::CheckCircle,
                            "Apply Wall Update",
                            11.0,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 28.0),
                        )
                        .clicked()
                        {
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::ResizeWall {
                                wall_id,
                                thickness_mm: self.edit_thickness_mm,
                                is_structural: self.edit_is_structural,
                            }) {
                                actions.sync_fem_and_earth = true;
                                self.push_trace(msg, true);
                            }
                        }
                    }
                    SmartPopupTab::Calculations => {
                        Self::render_kv_row(ui, theme, "Length x Height", &format!("{len_m:.2}m x {h_m:.2}m"), theme.text_primary);
                        Self::render_kv_row(ui, theme, "Structural Shear Role", if wall.is_structural { "Active Shear Wall" } else { "Architectural Partition" }, theme.accent_blue);
                        Self::render_kv_row(ui, theme, "Compound Layers", &format!("{} plies", wall.layers.len()), theme.text_secondary);
                    }
                    SmartPopupTab::Lifecycle5D6D => {
                        let co2_kg = vol_m3 * 290.0;
                        Self::render_kv_row(ui, theme, "Wall Volume", &format!("{vol_m3:.2} m³"), theme.text_primary);
                        Self::render_kv_row(ui, theme, "Embodied Carbon", &format!("{co2_kg:.1} kgCO₂e"), theme.accent_green);
                    }
                    SmartPopupTab::AskAi => {
                        self.render_popup_ai_actions(
                            ui,
                            theme,
                            &wall.name,
                            ai_assistant,
                            members,
                            projects,
                            db_components,
                        );
                    }
                }
            }
            SmartSelectionTarget::Footing(foot_id) => {
                let ft_opt = build_view
                    .bim_bus
                    .project
                    .footings
                    .iter()
                    .find(|f| f.id == foot_id)
                    .cloned();
                let Some(ft) = ft_opt else {
                    ui.label("Footing not found.");
                    return actions;
                };
                let area_m2 = (ft.width_mm * 0.001) * (ft.length_mm * 0.001);
                let vol_m3 = area_m2 * (ft.thickness_mm * 0.001);
                let qa_kpa = if site.recommended_soil_class.contains("SC") {
                    240.0
                } else if site.recommended_soil_class.contains("SD") {
                    175.0
                } else {
                    125.0
                };
                let est_bearing_kpa = 640.0 / area_m2.max(0.25);

                match self.popup_tab {
                    SmartPopupTab::Properties => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Width B (mm):").size(11.0).color(theme.text_secondary));
                            ui.add(egui::DragValue::new(&mut self.edit_width_mm).range(600.0..=6000.0).speed(50.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Length L (mm):").size(11.0).color(theme.text_secondary));
                            ui.add(egui::DragValue::new(&mut self.edit_length_mm).range(600.0..=6000.0).speed(50.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Thick h (mm):").size(11.0).color(theme.text_secondary));
                            ui.add(egui::DragValue::new(&mut self.edit_thickness_mm).range(250.0..=2000.0).speed(25.0));
                        });
                        ui.add_space(6.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::CheckCircle,
                            "Apply Footing Resize",
                            11.0,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 28.0),
                        )
                        .clicked()
                        {
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::ResizeFooting {
                                footing_id: foot_id,
                                width_mm: self.edit_width_mm,
                                length_mm: self.edit_length_mm,
                                thickness_mm: self.edit_thickness_mm,
                            }) {
                                actions.sync_fem_and_earth = true;
                                self.push_trace(msg, true);
                            }
                        }
                    }
                    SmartPopupTab::Calculations => {
                        Self::render_kv_row(ui, theme, "Bearing Area", &format!("{area_m2:.2} m²"), theme.text_primary);
                        Self::render_kv_row(ui, theme, "Site Allowable qa", &format!("{qa_kpa:.0} kPa ({})", site.recommended_soil_class), theme.accent_blue);
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Est. Soil Pressure",
                            &format!("{est_bearing_kpa:.0} kPa ({:.0}%)", (est_bearing_kpa / qa_kpa) * 100.0),
                            if est_bearing_kpa > qa_kpa { theme.accent_red } else { theme.accent_green },
                        );
                    }
                    SmartPopupTab::Lifecycle5D6D => {
                        let rebar_kg = vol_m3 * 105.0;
                        let co2_kg = vol_m3 * 315.0 + rebar_kg * 1.15;
                        Self::render_kv_row(ui, theme, "Concrete Volume", &format!("{vol_m3:.3} m³"), theme.text_primary);
                        Self::render_kv_row(ui, theme, "Est. Rebar", &format!("{rebar_kg:.1} kg"), theme.accent_blue);
                        Self::render_kv_row(ui, theme, "Embodied Carbon", &format!("{co2_kg:.1} kgCO₂e"), theme.accent_green);
                    }
                    SmartPopupTab::AskAi => {
                        self.render_popup_ai_actions(
                            ui,
                            theme,
                            &ft.name,
                            ai_assistant,
                            members,
                            projects,
                            db_components,
                        );
                    }
                }
            }
        }

        actions
    }

    fn render_structural_kpi_strip(
        &self,
        ui: &mut Ui,
        theme: &MacTheme,
        fem_member: Option<&StructuralMember>,
    ) {
        let dcr = fem_member.map(|m| m.utilization * 100.0).unwrap_or(42.0);
        let ari = fem_member.map(|m| m.ari).unwrap_or(0.89);
        let force = fem_member.map(|m| m.force_kn).unwrap_or(-420.0);
        let dcr_col = if dcr > 85.0 {
            theme.accent_red
        } else if dcr > 65.0 {
            theme.accent_orange
        } else {
            theme.accent_green
        };
        let ari_col = if ari >= 0.70 {
            theme.accent_green
        } else {
            theme.accent_orange
        };

        ui.columns(3, |cols| {
            Self::kpi_chip(&mut cols[0], theme, "DCR", &format!("{dcr:.1}%"), dcr_col);
            Self::kpi_chip(&mut cols[1], theme, "ARI", &format!("{ari:.3}"), ari_col);
            Self::kpi_chip(&mut cols[2], theme, "FORCE", &format!("{force:.0}kN"), theme.accent_blue);
        });
    }

    fn kpi_chip(ui: &mut Ui, theme: &MacTheme, label: &str, val: &str, col: Color32) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .inner_margin(Margin::symmetric(8, 5))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(label).size(9.0).color(theme.text_muted));
                    ui.label(RichText::new(val).size(12.0).strong().color(col));
                });
            });
    }

    fn render_kv_row(ui: &mut Ui, theme: &MacTheme, k: &str, v: &str, col: Color32) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(k).size(10.8).color(theme.text_secondary));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(v).size(10.8).strong().color(col));
            });
        });
        ui.add_space(2.0);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_popup_ai_actions(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        elem_name: &str,
        ai_assistant: &mut AiAssistantView,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
    ) {
        let prompts = [
            (
                format!("Audit @{elem_name} DCR & ARI"),
                format!("screen ari and p-m-v interaction for @{elem_name}"),
            ),
            (
                format!("Check N-1 Collapse Risk"),
                format!("n-2 cascading disruption check around @{elem_name}"),
            ),
            (
                format!("Recommend Bracing / Repair"),
                "recommend ari bracing".to_string(),
            ),
        ];
        for (btn_lbl, query) in prompts {
            if icons::icon_button(
                ui,
                AppIcon::Sparkles,
                btn_lbl,
                10.5,
                theme.text_primary,
                theme.bg_card,
                Stroke::new(1.0, theme.border_subtle),
                CornerRadius::same(8),
                Vec2::new(ui.available_width(), 26.0),
            )
            .clicked()
            {
                self.command_bar_expanded = true;
                ai_assistant.execute_prompt(
                    &query,
                    NavigationPillar::AnalysisSimulation,
                    members,
                    projects,
                    db_components,
                );
                self.push_trace(format!("EigenAI audited @{elem_name}"), true);
            }
            ui.add_space(4.0);
        }
        if icons::icon_button(
            ui,
            AppIcon::Bot,
            format!("Mention @{elem_name} in Command Bar"),
            10.5,
            Color32::WHITE,
            theme.accent_blue,
            Stroke::NONE,
            CornerRadius::same(8),
            Vec2::new(ui.available_width(), 26.0),
        )
        .clicked()
        {
            self.command_input = format!("@{elem_name} ");
            self.command_bar_expanded = true;
            self.request_focus_command = true;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_antigravity_command_dock(
        &mut self,
        ctx: &egui::Context,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
        vp: Vec2,
        is_mobile: bool,
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();
        let dock_w = if is_mobile {
            (vp.x - 20.0).clamp(240.0, 420.0)
        } else {
            (vp.x - 40.0).clamp(360.0, 680.0)
        };
        let slash_mode = self.command_input.trim_start().starts_with('/');

        let dock_resp = Area::new(Id::new("antigravity_ai_command_dock"))
            .order(Order::Foreground)
            .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -14.0))
            .constrain(true)
            .show(ctx, |ui| {
                ui.set_width(dock_w);
                ui.set_max_width(dock_w);
                ui.vertical(|ui| {
                    // A. Compact Antigravity Execution Trace Chips above the dock
                    if !self.recent_trace_chips.is_empty() && !self.command_bar_expanded && !slash_mode {
                        ui.horizontal_wrapped(|ui| {
                            for chip in self.recent_trace_chips.iter().rev().take(if is_mobile { 1 } else { 2 }) {
                                let col = if chip.ok {
                                    theme.accent_green
                                } else {
                                    theme.accent_orange
                                };
                                Frame::new()
                                    .fill(if theme.is_dark() {
                                        Color32::from_rgba_premultiplied(15, 19, 28, 225)
                                    } else {
                                        Color32::from_rgba_premultiplied(250, 252, 255, 235)
                                    })
                                    .corner_radius(CornerRadius::same(12))
                                    .stroke(Stroke::new(1.0, col.gamma_multiply(0.45)))
                                    .inner_margin(Margin::symmetric(10, 3))
                                    .show(ui, |ui| {
                                        icons::icon_label(
                                            ui,
                                            if chip.ok {
                                                AppIcon::CheckCircle
                                            } else {
                                                AppIcon::AlertTriangle
                                            },
                                            11.0,
                                            col,
                                            &chip.label,
                                            10.0,
                                            theme.text_secondary,
                                            false,
                                        );
                                    });
                            }
                        });
                        ui.add_space(5.0);
                    }

                    // B. Slash Command Autocomplete Palette (shown whenever user types `/`)
                    if slash_mode {
                        let q = self.command_input.trim().to_lowercase();
                        let mut queued_cmd: Option<String> = None;
                        Self::antigravity_glass_frame(theme, 14, Margin::same(10)).show(ui, |ui| {
                            let pal_w = (dock_w - 24.0).max(200.0);
                            ui.set_width(pal_w);
                            ui.set_max_width(pal_w);
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                            ui.horizontal(|ui| {
                                icons::icon_label(
                                    ui,
                                    AppIcon::Sparkles,
                                    13.0,
                                    theme.accent_blue,
                                    "Command Palette",
                                    11.5,
                                    theme.text_primary,
                                    true,
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new("Enter to run • Esc to close")
                                            .size(9.5)
                                            .color(theme.text_muted),
                                    );
                                });
                            });
                            ui.add_space(4.0);
                            ScrollArea::vertical()
                                .id_salt("antigravity_slash_palette_scroll")
                                .max_height((vp.y * 0.34).clamp(150.0, 260.0))
                                .show(ui, |ui| {
                                    let btn_w = (pal_w - 14.0).max(160.0);
                                    ui.set_max_width(btn_w);
                                    let needle = q.strip_prefix('/').unwrap_or(&q);
                                    for spec in SLASH_COMMANDS {
                                        let matches = q == "/"
                                            || spec.cmd.contains(&q)
                                            || spec.category.to_lowercase().contains(needle)
                                            || spec.description.to_lowercase().contains(needle);
                                        if !matches {
                                            continue;
                                        }
                                        let resp = ui.add_sized(
                                            Vec2::new(btn_w, 26.0),
                                            egui::Button::new(
                                                RichText::new(format!(
                                                    "{}   •   [{}] {}",
                                                    spec.cmd, spec.category, spec.description
                                                ))
                                                .size(11.0)
                                                .color(theme.text_primary),
                                            )
                                            .wrap_mode(egui::TextWrapMode::Wrap)
                                            .fill(theme.bg_card.gamma_multiply(0.85))
                                            .stroke(Stroke::new(0.8, theme.border_subtle))
                                            .corner_radius(CornerRadius::same(8)),
                                        );
                                        if resp.clicked() {
                                            queued_cmd = Some(spec.cmd.to_string());
                                        }
                                        ui.add_space(2.0);
                                    }
                                });
                        });
                        if let Some(cmd) = queued_cmd {
                            self.command_input.clear();
                            let sub_actions = self.execute_spatial_input(
                                &cmd,
                                universal_canvas,
                                build_view,
                                ai_assistant,
                                members,
                                projects,
                                db_components,
                            );
                            actions.merge(sub_actions);
                        }
                        ui.add_space(6.0);
                    }

                    // C. Expanded Antigravity Chat Thread
                    if self.command_bar_expanded && !slash_mode {
                        Self::antigravity_glass_frame(theme, 16, Margin::same(12)).show(ui, |ui| {
                            let chat_w = (dock_w - 28.0).max(200.0);
                            ui.set_width(chat_w);
                            ui.set_max_width(chat_w);
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                            ui.horizontal(|ui| {
                                icons::icon_label(
                                    ui,
                                    AppIcon::Bot,
                                    15.0,
                                    theme.accent_blue,
                                    "EigenAI Spatial Agent",
                                    13.0,
                                    theme.text_primary,
                                    true,
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if icons::icon_only_button(
                                        ui,
                                        AppIcon::Close,
                                        theme.text_secondary,
                                        Color32::TRANSPARENT,
                                        Stroke::NONE,
                                        CornerRadius::same(6),
                                        Vec2::splat(22.0),
                                        "Collapse Agent Console",
                                    )
                                    .clicked()
                                    {
                                        self.command_bar_expanded = false;
                                    }
                                    if icons::icon_button(
                                        ui,
                                        AppIcon::Wrench,
                                        "Auto-Repair ARI",
                                        10.0,
                                        Color32::WHITE,
                                        theme.accent_green,
                                        Stroke::NONE,
                                        CornerRadius::same(6),
                                        Vec2::new(0.0, 22.0),
                                    )
                                    .clicked()
                                    {
                                        actions.run_auto_repair = true;
                                        self.push_trace("Auto-Repaired Critical Members", true);
                                    }
                                });
                            });
                            ui.add_space(4.0);
                            ScrollArea::vertical()
                                .id_salt("antigravity_chat_scroll")
                                .max_height((vp.y * 0.28).clamp(110.0, 210.0))
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    ui.set_max_width((chat_w - 14.0).max(160.0));
                                    for msg in ai_assistant.chat_history.iter().rev().take(12).rev() {
                                        let bg = if msg.is_user {
                                            theme.accent_blue
                                        } else {
                                            theme.bg_card
                                        };
                                        let fg = if msg.is_user {
                                            Color32::WHITE
                                        } else {
                                            theme.text_primary
                                        };
                                        Frame::new()
                                            .fill(bg)
                                            .corner_radius(CornerRadius::same(10))
                                            .stroke(if msg.is_user {
                                                Stroke::NONE
                                            } else {
                                                Stroke::new(1.0, theme.border_subtle)
                                            })
                                            .inner_margin(Margin::symmetric(10, 6))
                                            .show(ui, |ui| {
                                                ui.label(RichText::new(&msg.message).size(11.5).color(fg));
                                            });
                                        ui.add_space(4.0);
                                    }
                                });
                        });
                        ui.add_space(6.0);
                    }

                    // D. Main Antigravity Command & AI Input Capsule
                    Self::antigravity_glass_frame(theme, 24, Margin::symmetric(12, 7)).show(ui, |ui| {
                        let cap_w = (dock_w - 28.0).max(200.0);
                        ui.set_width(cap_w);
                        ui.set_max_width(cap_w);
                        ui.horizontal(|ui| {
                            if icons::icon_only_button(
                                ui,
                                AppIcon::Sparkles,
                                theme.accent_blue,
                                theme.accent_blue.gamma_multiply(0.14),
                                Stroke::NONE,
                                CornerRadius::same(10),
                                Vec2::splat(26.0),
                                "Toggle Agent Chat Console",
                            )
                            .clicked()
                            {
                                self.command_bar_expanded = !self.command_bar_expanded;
                            }

                            if let Some(mention) =
                                self.active_context_mention(&build_view.bim_bus.project)
                            {
                                if !is_mobile {
                                    icons::pill_badge(
                                        ui,
                                        AppIcon::Target,
                                        mention,
                                        theme.accent_blue,
                                    );
                                }
                            }

                            let trailing_reserve = if is_mobile { 70.0 } else { 76.0 };
                            let input_w = (ui.available_width() - trailing_reserve).max(90.0);
                            let edit_resp = ui.add(
                                egui::TextEdit::singleline(&mut self.command_input)
                                    .desired_width(input_w)
                                    .frame(Frame::NONE)
                                    .hint_text("Ask EigenAI, fly to a city, or type / for commands..."),
                            );
                            if std::mem::take(&mut self.request_focus_command) {
                                edit_resp.request_focus();
                            }

                            let enter_submit = edit_resp.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter));

                            if icons::icon_button(
                                ui,
                                AppIcon::Menu,
                                "/",
                                11.0,
                                theme.text_secondary,
                                theme.bg_card,
                                Stroke::new(1.0, theme.border_subtle),
                                CornerRadius::same(8),
                                Vec2::new(24.0, 24.0),
                            )
                            .on_hover_text("Browse slash commands (/earth, /3d, /cost, /analysis...)")
                            .clicked()
                            {
                                if self.command_input.trim_start().starts_with('/') {
                                    self.command_input.clear();
                                } else {
                                    self.command_input = "/".to_string();
                                    self.request_focus_command = true;
                                }
                            }

                            let send_clicked = icons::icon_only_button(
                                ui,
                                AppIcon::ArrowRight,
                                Color32::WHITE,
                                theme.accent_blue,
                                Stroke::NONE,
                                CornerRadius::same(10),
                                Vec2::splat(26.0),
                                "Execute Command or AI Prompt",
                            )
                            .clicked();

                            if (enter_submit || send_clicked) && !self.command_input.trim().is_empty()
                            {
                                let submitted = std::mem::take(&mut self.command_input);
                                let sub_actions = self.execute_spatial_input(
                                    &submitted,
                                    universal_canvas,
                                    build_view,
                                    ai_assistant,
                                    members,
                                    projects,
                                    db_components,
                                );
                                actions.merge(sub_actions);
                            }
                        });
                    });
                });
            });
        self.hud_rects.push(dock_resp.response.rect);

        actions
    }
}

impl SpatialHudActions {
    fn merge(&mut self, other: Self) {
        self.sync_fem_and_earth |= other.sync_fem_and_earth;
        self.fit_camera |= other.fit_camera;
        self.load_sample_building |= other.load_sample_building;
        self.export_archive |= other.export_archive;
        self.toggle_theme |= other.toggle_theme;
        self.open_settings |= other.open_settings;
        self.switch_to_classic_ui |= other.switch_to_classic_ui;
        if other.toggle_n1_member.is_some() {
            self.toggle_n1_member = other.toggle_n1_member;
        }
        self.run_auto_repair |= other.run_auto_repair;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase46_slash_commands_cover_all_15_pillars_and_spatial_lenses() {
        let required = [
            "/earth",
            "/3d",
            "/2d",
            "/sample",
            "/lens dcr",
            "/lens ari",
            "/lens moment",
            "/lens 4d",
            "/vault",
            "/agent",
            "/planning",
            "/drafting",
            "/geometry",
            "/analysis",
            "/schedule",
            "/cost",
            "/sustainability",
            "/facility",
            "/safety",
            "/construction",
            "/automation",
            "/prediction",
            "/report",
            "/settings",
            "/classic",
        ];
        for cmd in required {
            assert!(
                SLASH_COMMANDS.iter().any(|s| s.cmd == cmd),
                "Missing required slash command {cmd}"
            );
        }
    }

    #[test]
    fn test_phase46_column_click_popup_and_transactional_resize() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus = crate::core::BimCommandBus::from_project(
            BimProject::sample_bimcraft_building(),
        );
        let first_col_id = build.bim_bus.project.columns[0].id;
        let fem_id = build
            .bim_bus
            .project
            .fem_id_for_bim_element(BimElementKind::Column, first_col_id)
            .expect("column maps deterministically to FEM element");
        assert!(fem_id >= 1);

        // Simulate 3D pick event
        build.last_spatial_bim_pick = Some((BimElementKind::Column, first_col_id));
        build.spatial_pick_seq = 1;
        spatial.sync_pick_events(&mut canvas, &mut build);
        assert_eq!(
            spatial.selected_target,
            Some(SmartSelectionTarget::Column(first_col_id))
        );
        assert!(spatial
            .active_context_mention(&build.bim_bus.project)
            .unwrap()
            .starts_with('@'));

        // Verify transactional ResizeColumn with Undo
        let orig_w = build.bim_bus.project.columns[0].width_mm;
        build
            .bim_bus
            .execute(BimCommand::ResizeColumn {
                column_id: first_col_id,
                width_mm: 550.0,
                depth_mm: 550.0,
                rotation_deg: 15.0,
                profile_name: "RC 550x550".to_string(),
            })
            .expect("resize column");
        assert!((build.bim_bus.project.columns[0].width_mm - 550.0).abs() < 1e-6);
        build.bim_bus.undo().expect("undo resize");
        assert!((build.bim_bus.project.columns[0].width_mm - orig_w).abs() < 1e-6);
    }

    #[test]
    fn test_phase46_spatial_hud_responsive_zero_overflow() {
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        for win_w in [1000.0_f32, 700.0, 390.0] {
            let mut spatial = SpatialWorkspaceState::default();
            let mut canvas = UniversalCanvasState::default();
            canvas.is_frozen = false;
            let mut build = BuildView::default();
            build.bim_bus = crate::core::BimCommandBus::from_project(
                BimProject::sample_bimcraft_building(),
            );
            let col_id = build.bim_bus.project.columns[0].id;
            spatial.selected_target = Some(SmartSelectionTarget::Column(col_id));
            spatial.command_input = "/".to_string();

            let mut ai = AiAssistantView::default();
            let members = vec![StructuralMember::default()];
            let projects = Vec::new();
            let mut db_components = Vec::new();
            let ctx = egui::Context::default();

            for _ in 0..2 {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 840.0))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(input, |ui| {
                    egui::CentralPanel::default()
                        .frame(Frame::new().fill(theme.bg_canvas))
                        .show(ui, |ui| {
                            canvas.show_spatial_fullscreen(
                                ui,
                                &theme,
                                NavigationPillar::GeometryAssembly,
                                &mut build.bim_bus.project,
                                &members,
                                false,
                            );
                        });
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }

            for r in &spatial.hud_rects {
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "HUD rect {r:?} overflowed viewport width {win_w}"
                );
            }
        }
    }

    #[test]
    fn test_phase46_command_dispatch_fly_lens_overlay_and_classic_toggle() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members = vec![StructuralMember::default()];
        let projects = Vec::new();
        let mut db_components = Vec::new();

        // 1. /fly jakarta switches to EarthGlobe and anchors site
        let _ = spatial.execute_spatial_input(
            "/fly jakarta",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(spatial.active_canvas_mode, SpatialCanvasMode::EarthGlobe);
        assert!(canvas.ground_origin_anchored);
        assert!(matches!(
            spatial.selected_target,
            Some(SmartSelectionTarget::EarthSite { .. })
        ));

        // 2. /lens dcr switches to Bim3D with DCR% heatmap mode
        let _ = spatial.execute_spatial_input(
            "/lens dcr",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(spatial.active_lens, SpatialLens::DcrHeatmap);
        assert_eq!(spatial.active_canvas_mode, SpatialCanvasMode::Bim3D);
        assert_eq!(build.color_mode, "DCR%");

        // 3. /cost summons CostBudgeting as an on-demand overlay sheet
        let _ = spatial.execute_spatial_input(
            "/cost",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(
            spatial.active_overlay_pillar,
            Some(NavigationPillar::CostBudgeting)
        );

        // 4. /classic flags switch back to classic 15-pillar sidebar mode
        let classic_act = spatial.execute_spatial_input(
            "/classic",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(classic_act.switch_to_classic_ui);
    }
}
