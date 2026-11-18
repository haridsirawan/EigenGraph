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

/// Execution planning policy selectable inside the Antigravity AI composer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentExecutionMode {
    #[default]
    Planning,
    Fast,
}

impl AgentExecutionMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Planning => "Planning",
            Self::Fast => "Fast",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Planning => "Agent plans multi-step checks & summarizes engineering verification before responding",
            Self::Fast => "Direct single-pass command execution & immediate lookup",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Planning => Self::Fast,
            Self::Fast => Self::Planning,
        }
    }
}

/// Active reasoning engine selectable inside the Antigravity composer footer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentModelChoice {
    #[default]
    EigenAiProFem,
    Gemini25Flash,
    GeminiLive,
}

impl AgentModelChoice {
    pub fn label(self) -> &'static str {
        match self {
            Self::EigenAiProFem => "EigenAI Pro (FEM+ARI)",
            Self::Gemini25Flash => "Gemini 2.5 Flash",
            Self::GeminiLive => "Gemini Live Stream",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::EigenAiProFem => "EigenAI Pro",
            Self::Gemini25Flash => "Gemini 2.5 Flash",
            Self::GeminiLive => "Gemini Live",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::EigenAiProFem => Self::Gemini25Flash,
            Self::Gemini25Flash => Self::GeminiLive,
            Self::GeminiLive => Self::EigenAiProFem,
        }
    }
}

/// Structured context pill attachable via `+ Add Context` (`@`) inside the Antigravity panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiContextAttachment {
    SelectedElement,
    EarthSite,
    BimGeometry,
    FemRedundancy,
    ScheduleAndCost,
    VaultWorkspace,
    VaultFile {
        name: String,
        category: String,
        rel_path: String,
        size_bytes: usize,
    },
    LocalFile {
        filename: String,
        size_bytes: usize,
        snippet: String,
    },
}

impl AiContextAttachment {
    pub fn dedup_key(&self) -> String {
        match self {
            Self::SelectedElement => "domain:selected".to_string(),
            Self::EarthSite => "domain:earth".to_string(),
            Self::BimGeometry => "domain:bim".to_string(),
            Self::FemRedundancy => "domain:fem".to_string(),
            Self::ScheduleAndCost => "domain:schedule_cost".to_string(),
            Self::VaultWorkspace => "vault:workspace".to_string(),
            Self::VaultFile { rel_path, .. } => format!("vault_file:{rel_path}"),
            Self::LocalFile { filename, .. } => format!("local_file:{filename}"),
        }
    }

    pub fn icon(&self) -> AppIcon {
        match self {
            Self::SelectedElement => AppIcon::Target,
            Self::EarthSite => AppIcon::Globe,
            Self::BimGeometry => AppIcon::Cube3D,
            Self::FemRedundancy => AppIcon::Bolt,
            Self::ScheduleAndCost => AppIcon::Coins,
            Self::VaultWorkspace => AppIcon::Folder,
            Self::VaultFile { .. } | Self::LocalFile { .. } => AppIcon::Document,
        }
    }

    pub fn chip_label(
        &self,
        project: &BimProject,
        selected: Option<SmartSelectionTarget>,
    ) -> String {
        match self {
            Self::SelectedElement => match selected {
                Some(SmartSelectionTarget::Column(id)) => project
                    .columns
                    .iter()
                    .find(|c| c.id == id)
                    .map(|c| format!("@{}", c.name))
                    .unwrap_or_else(|| format!("@Column#{id}")),
                Some(SmartSelectionTarget::Beam(id)) => project
                    .runs
                    .iter()
                    .find(|r| r.id == id)
                    .map(|r| format!("@{}", r.name))
                    .unwrap_or_else(|| format!("@Beam#{id}")),
                Some(SmartSelectionTarget::Wall(id)) => project
                    .walls
                    .iter()
                    .find(|w| w.id == id)
                    .map(|w| format!("@{}", w.name))
                    .unwrap_or_else(|| format!("@Wall#{id}")),
                Some(SmartSelectionTarget::Footing(id)) => project
                    .footings
                    .iter()
                    .find(|f| f.id == id)
                    .map(|f| format!("@{}", f.name))
                    .unwrap_or_else(|| format!("@Footing#{id}")),
                Some(SmartSelectionTarget::EarthSite { lat, lon }) => {
                    format!("@Earth({lat:.2},{lon:.2})")
                }
                Some(SmartSelectionTarget::SketchEntity(id)) => format!("@Sketch#{id}"),
                None => "@Selection".to_string(),
            },
            Self::EarthSite => "@Earth Site".to_string(),
            Self::BimGeometry => "@BIM Model".to_string(),
            Self::FemRedundancy => "@FEM & ARI".to_string(),
            Self::ScheduleAndCost => "@4D/5D Cost".to_string(),
            Self::VaultWorkspace => "@Vault Folder".to_string(),
            Self::VaultFile { name, .. } => format!("@{name}"),
            Self::LocalFile { filename, .. } => format!("@{filename}"),
        }
    }
}

/// Visual Antigravity execution metadata paired to an individual chat message index.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AntigravityTurnMeta {
    pub msg_index: usize,
    pub mode_label: String,
    pub model_label: String,
    pub context_chips: Vec<String>,
    pub trace_steps: Vec<String>,
}

/// Archived Antigravity chat thread allowing `+ New Chat` and History switching.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentThreadArchive {
    pub title: String,
    pub messages: Vec<crate::models::ChatMessage>,
    pub turn_meta: Vec<AntigravityTurnMeta>,
}

/// Persisted Dynamic Island session bundle stored in `Vault/AI_Context/dynamic_island_sessions.json`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DynamicIslandPersistedState {
    pub conversation_title: String,
    pub conversation_archives: Vec<AgentThreadArchive>,
    pub active_messages: Vec<crate::models::ChatMessage>,
    pub turn_metadata: Vec<AntigravityTurnMeta>,
}

/// Action proposed by `Planning` agent mode requiring explicit user confirmation before mutating BIM/Earth.
#[derive(Debug, Clone, PartialEq)]
pub enum PlannedProposalAction {
    ExecuteBimNl(String),
    AutoRepair,
    FlyEarth {
        lat: f64,
        lon: f64,
        zoom: f64,
        label: String,
    },
    AnchorSite {
        lat: f64,
        lon: f64,
    },
    ApplyAriLens,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalStatus {
    Pending,
    Applied,
    Rejected,
}

/// Interactive review card rendered inside Dynamic Island when `AgentExecutionMode::Planning` proposes edits.
#[derive(Debug, Clone)]
pub struct PlannedAgentProposal {
    pub id: u64,
    pub title: String,
    pub rationale: String,
    pub action_preview: String,
    pub actions: Vec<PlannedProposalAction>,
    pub status: ProposalStatus,
}

/// Structured bracket directive optionally emitted by Gemini API responses (`[[FLY:...]]`, `[[BIM:...]]`, `[[REPAIR]]`, `[[ARI_AUDIT]]`).
#[derive(Debug, Clone, PartialEq)]
pub enum GeminiDirective {
    Fly { lat: f64, lon: f64, zoom: f64 },
    Bim(String),
    Repair,
    AriAudit,
}

/// Strips structured `[[...]]` action tags from a Gemini response and returns parsed directives.
pub fn parse_gemini_action_directives(raw: &str) -> (String, Vec<GeminiDirective>) {
    let mut directives = Vec::new();
    let mut clean = String::with_capacity(raw.len());
    let mut cursor = raw;

    while let Some(start) = cursor.find("[[") {
        clean.push_str(&cursor[..start]);
        let after_start = &cursor[start + 2..];
        if let Some(end) = after_start.find("]]") {
            let tag = after_start[..end].trim();
            let upper = tag.to_ascii_uppercase();
            if upper.starts_with("FLY:") {
                let coords_part = tag[4..].trim();
                let nums: Vec<f64> = coords_part
                    .split(',')
                    .filter_map(|s| s.trim().parse::<f64>().ok())
                    .collect();
                if nums.len() >= 2 {
                    let zoom = nums.get(2).copied().unwrap_or(14.5).clamp(1.0, 19.0);
                    directives.push(GeminiDirective::Fly {
                        lat: nums[0].clamp(-85.0, 85.0),
                        lon: nums[1].clamp(-180.0, 180.0),
                        zoom,
                    });
                }
            } else if upper.starts_with("BIM:") {
                let nl = tag[4..].trim();
                if !nl.is_empty() {
                    directives.push(GeminiDirective::Bim(nl.to_string()));
                }
            } else if upper == "REPAIR" {
                directives.push(GeminiDirective::Repair);
            } else if upper == "ARI_AUDIT" || upper == "ARI" {
                directives.push(GeminiDirective::AriAudit);
            }
            cursor = &after_start[end + 2..];
        } else {
            clean.push_str("[[");
            cursor = after_start;
        }
    }
    clean.push_str(cursor);
    (clean.trim().to_string(), directives)
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
    SlashCommandSpec {
        cmd: "/add-folder",
        category: "Vault",
        description: "Add or switch local Folder connected as Engineering Vault",
        icon: AppIcon::Folder,
    },
    SlashCommandSpec {
        cmd: "/context",
        category: "Agent",
        description: "Open @ Add Context picker to attach BIM, FEM, Earth, or Vault files",
        icon: AppIcon::Layers,
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
    pub add_vault_folder: bool,
    pub connect_indexeddb_vault: Option<String>,
    pub connect_vault_path: Option<String>,
    pub create_vault_path_and_name: Option<(String, String)>,
    pub disconnect_vault: bool,
    pub save_active_to_vault: bool,
    pub refresh_vault: bool,
    pub open_vault_project: Option<crate::core::vault::PersonalVaultProjectEntry>,
    pub pick_local_context_file: bool,
    pub anchor_and_build_site: Option<(f64, f64)>,
}

/// Manual CAD/BIM/Earth drawing & measurement commands surfaced directly inside the Dynamic Island.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialManualTool {
    OrbitPan,
    Column,
    Beam,
    Wall,
    Footing,
    Line2D,
    Rect2D,
    Measure,
    Terraform,
}

impl SpatialManualTool {
    pub const ALL: &'static [Self] = &[
        Self::OrbitPan,
        Self::Column,
        Self::Beam,
        Self::Wall,
        Self::Footing,
        Self::Line2D,
        Self::Rect2D,
        Self::Measure,
        Self::Terraform,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::OrbitPan => "Orbit",
            Self::Column => "Column",
            Self::Beam => "Beam",
            Self::Wall => "Wall",
            Self::Footing => "Footing",
            Self::Line2D => "Line",
            Self::Rect2D => "Rect",
            Self::Measure => "Measure",
            Self::Terraform => "Terraform",
        }
    }

    pub fn tooltip(self) -> &'static str {
        match self {
            Self::OrbitPan => "Orbit Earth / Navigate 3D scene & select elements",
            Self::Column => "Place manual structural Column (click ground in 3D BIM)",
            Self::Beam => "Draw manual structural Beam (click start & end in 3D BIM)",
            Self::Wall => "Draw manual shear/partition Wall (click start & end in 3D BIM)",
            Self::Footing => "Place manual isolated Footing on ground origin",
            Self::Line2D => "Draw 2D vector CAD line segment on Earth / Plan",
            Self::Rect2D => "Draw 2D rectangular boundary / slab outline on Earth / Plan",
            Self::Measure => "Measure geodesic distance across Earth / site coordinates",
            Self::Terraform => "Cycle 3D Earth sculpting & megaproject brush (Uplift / Carve / Seawall / Bridge)",
        }
    }

    pub fn icon(self) -> AppIcon {
        match self {
            Self::OrbitPan => AppIcon::CursorPan,
            Self::Column => AppIcon::Column,
            Self::Beam => AppIcon::Beam,
            Self::Wall => AppIcon::Wall,
            Self::Footing => AppIcon::Slab,
            Self::Line2D => AppIcon::LineSeg,
            Self::Rect2D => AppIcon::RectShape,
            Self::Measure => AppIcon::DimensionRuler,
            Self::Terraform => AppIcon::Mountain,
        }
    }

    pub fn is_active(
        self,
        canvas_mode: SpatialCanvasMode,
        universal_canvas: &UniversalCanvasState,
        build_view: &BuildView,
    ) -> bool {
        match self {
            Self::OrbitPan => {
                !universal_canvas.measure_active
                    && build_view.ground_author_tool == Ground3DAuthorTool::Orbit
                    && universal_canvas.active_tool == UniversalCanvasTool::SelectPan
            }
            Self::Column => {
                canvas_mode == SpatialCanvasMode::Bim3D
                    && build_view.ground_author_tool == Ground3DAuthorTool::Column
            }
            Self::Beam => {
                canvas_mode == SpatialCanvasMode::Bim3D
                    && build_view.ground_author_tool == Ground3DAuthorTool::Beam
            }
            Self::Wall => {
                canvas_mode == SpatialCanvasMode::Bim3D
                    && build_view.ground_author_tool == Ground3DAuthorTool::Wall
            }
            Self::Footing => {
                canvas_mode == SpatialCanvasMode::Bim3D
                    && build_view.ground_author_tool == Ground3DAuthorTool::Footing
            }
            Self::Line2D => {
                canvas_mode != SpatialCanvasMode::Bim3D
                    && universal_canvas.active_tool == UniversalCanvasTool::Line
            }
            Self::Rect2D => {
                canvas_mode != SpatialCanvasMode::Bim3D
                    && universal_canvas.active_tool == UniversalCanvasTool::Rectangle
            }
            Self::Measure => universal_canvas.measure_active,
            Self::Terraform => universal_canvas.active_tool.is_earth_tool(),
        }
    }

    pub fn apply(
        self,
        spatial: &mut SpatialWorkspaceState,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();
        universal_canvas.measure_active = false;
        match self {
            Self::OrbitPan => {
                build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                universal_canvas.active_tool = UniversalCanvasTool::SelectPan;
                spatial.push_trace("Navigation & Select mode", true);
            }
            Self::Column => {
                spatial.active_canvas_mode = SpatialCanvasMode::Bim3D;
                build_view.ground_author_tool = Ground3DAuthorTool::Column;
                universal_canvas.ground_origin_anchored = true;
                actions.fit_camera = true;
                spatial.push_trace("Manual Draw: Column (click canvas)", true);
            }
            Self::Beam => {
                spatial.active_canvas_mode = SpatialCanvasMode::Bim3D;
                build_view.ground_author_tool = Ground3DAuthorTool::Beam;
                universal_canvas.ground_origin_anchored = true;
                actions.fit_camera = true;
                spatial.push_trace("Manual Draw: Beam (click 2 pts)", true);
            }
            Self::Wall => {
                spatial.active_canvas_mode = SpatialCanvasMode::Bim3D;
                build_view.ground_author_tool = Ground3DAuthorTool::Wall;
                universal_canvas.ground_origin_anchored = true;
                actions.fit_camera = true;
                spatial.push_trace("Manual Draw: Wall (click 2 pts)", true);
            }
            Self::Footing => {
                spatial.active_canvas_mode = SpatialCanvasMode::Bim3D;
                build_view.ground_author_tool = Ground3DAuthorTool::Footing;
                universal_canvas.ground_origin_anchored = true;
                actions.fit_camera = true;
                spatial.push_trace("Manual Draw: Footing (click canvas)", true);
            }
            Self::Line2D => {
                if spatial.active_canvas_mode == SpatialCanvasMode::Bim3D {
                    spatial.active_canvas_mode = SpatialCanvasMode::EarthPlan2D;
                    universal_canvas.globe_mode = false;
                }
                build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                universal_canvas.active_tool = UniversalCanvasTool::Line;
                spatial.push_trace("Manual Draw: 2D Line segment", true);
            }
            Self::Rect2D => {
                if spatial.active_canvas_mode == SpatialCanvasMode::Bim3D {
                    spatial.active_canvas_mode = SpatialCanvasMode::EarthPlan2D;
                    universal_canvas.globe_mode = false;
                }
                build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                universal_canvas.active_tool = UniversalCanvasTool::Rectangle;
                spatial.push_trace("Manual Draw: 2D Rectangle boundary", true);
            }
            Self::Measure => {
                universal_canvas.measure_active = true;
                build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                universal_canvas.active_tool = UniversalCanvasTool::SelectPan;
                spatial.push_trace("Geodesic Measure active (click points)", true);
            }
            Self::Terraform => {
                spatial.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                universal_canvas.globe_mode = true;
                build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                let next_tool = match universal_canvas.active_tool {
                    UniversalCanvasTool::EarthUplift => UniversalCanvasTool::EarthCarve,
                    UniversalCanvasTool::EarthCarve => UniversalCanvasTool::EarthSeawall,
                    UniversalCanvasTool::EarthSeawall => UniversalCanvasTool::EarthBridge,
                    _ => UniversalCanvasTool::EarthUplift,
                };
                universal_canvas.active_tool = next_tool;
                spatial.push_trace(
                    format!("Earth Brush: {}", next_tool.short_label()),
                    true,
                );
            }
        }
        actions
    }
}

/// Complete state for the Phase 48 Full-Canvas + Hover Dynamic Island UI.
pub struct SpatialWorkspaceState {
    pub spatial_mode_enabled: bool,
    pub active_canvas_mode: SpatialCanvasMode,
    pub active_lens: SpatialLens,
    pub selected_target: Option<SmartSelectionTarget>,
    pub popup_pinned: bool,
    pub popup_tab: SmartPopupTab,
    pub command_bar_expanded: bool,
    pub dynamic_island_rect: Rect,
    pub island_expanded_prev: bool,
    pub agent_mode: AgentExecutionMode,
    pub agent_model: AgentModelChoice,
    pub attached_contexts: Vec<AiContextAttachment>,
    pub context_picker_open: bool,
    pub context_search_query: String,
    pub vault_drawer_open: bool,
    pub vault_path_popover_open: bool,
    pub vault_inline_path: String,
    pub vault_inline_name: String,
    pub vault_feedback: Option<String>,
    pub history_popover_open: bool,
    pub conversation_title: String,
    pub conversation_archives: Vec<AgentThreadArchive>,
    pub turn_metadata: Vec<AntigravityTurnMeta>,
    pub pending_context_file_pick: bool,
    pub command_input: String,
    pub request_focus_command: bool,
    pub active_overlay_pillar: Option<NavigationPillar>,
    pub recent_trace_chips: Vec<AgentTraceChip>,
    pub last_uc_pick_seq: u64,
    pub last_build_pick_seq: u64,
    pub hud_rects: Vec<Rect>,
    composer_had_focus: bool,
    last_command_len: usize,

    // Phase 49: Live Gemini API, Planning Proposals & Vault AI Session Persistence
    pub gemini_api_key_draft: String,
    pub api_key_popover_open: bool,
    pub gemini_inflight_req_id: Option<u64>,
    pub pending_proposals: Vec<PlannedAgentProposal>,
    next_proposal_id: u64,
    pub sessions_dirty: bool,
    pub vault_sessions_hydrated: bool,

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
            dynamic_island_rect: Rect::NOTHING,
            island_expanded_prev: false,
            agent_mode: AgentExecutionMode::Planning,
            agent_model: AgentModelChoice::EigenAiProFem,
            attached_contexts: Vec::new(),
            context_picker_open: false,
            context_search_query: String::new(),
            vault_drawer_open: false,
            vault_path_popover_open: false,
            vault_inline_path: crate::core::InternalVault::default_vault_dir()
                .to_string_lossy()
                .to_string(),
            vault_inline_name: "EigenGraph_Vault".to_string(),
            vault_feedback: None,
            history_popover_open: false,
            conversation_title: "EigenAI Spatial Agent".to_string(),
            conversation_archives: Vec::new(),
            turn_metadata: Vec::new(),
            pending_context_file_pick: false,
            command_input: String::new(),
            request_focus_command: false,
            active_overlay_pillar: None,
            recent_trace_chips: vec![AgentTraceChip {
                label: "Earth Globe Ready • Hover Dynamic Island for Vault, AI & Draw".to_string(),
                ok: true,
            }],
            last_uc_pick_seq: 0,
            last_build_pick_seq: 0,
            hud_rects: Vec::new(),
            composer_had_focus: false,
            last_command_len: 0,
            gemini_api_key_draft: String::new(),
            api_key_popover_open: false,
            gemini_inflight_req_id: None,
            pending_proposals: Vec::new(),
            next_proposal_id: 1,
            sessions_dirty: false,
            vault_sessions_hydrated: false,
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
    /// Returns true when the Floating Dynamic Island is hovered or has an active composer/popover.
    pub fn is_dynamic_island_expanded(&self, ctx: &egui::Context) -> bool {
        let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
        let hover_margin = if self.island_expanded_prev { 18.0 } else { 4.0 };
        let hovered_island = self.dynamic_island_rect.is_positive()
            && pointer_pos.is_some_and(|p| {
                self.dynamic_island_rect.expand(hover_margin).contains(p)
            });
        let engaged = self.composer_had_focus
            || !self.command_input.trim().is_empty()
            || self.context_picker_open
            || self.vault_drawer_open
            || self.vault_path_popover_open
            || self.history_popover_open
            || self.api_key_popover_open
            || self.gemini_inflight_req_id.is_some()
            || self.command_bar_expanded;
        hovered_island || engaged
    }

    /// Collapses any pinned Dynamic Island drawers/popovers so moving cursor to Earth folds it back.
    pub fn collapse_dynamic_island(&mut self) {
        self.command_bar_expanded = false;
        self.context_picker_open = false;
        self.vault_drawer_open = false;
        self.vault_path_popover_open = false;
        self.history_popover_open = false;
        self.api_key_popover_open = false;
        self.composer_had_focus = false;
    }

    /// Attaches a context source into the AI composer strip without duplicates.
    pub fn attach_context(&mut self, attachment: AiContextAttachment) {
        let key = attachment.dedup_key();
        if let Some(pos) = self
            .attached_contexts
            .iter()
            .position(|a| a.dedup_key() == key)
        {
            self.attached_contexts[pos] = attachment;
        } else {
            self.attached_contexts.push(attachment);
        }
    }

    /// Removes an attached context chip by deduplication key.
    pub fn detach_context(&mut self, key: &str) {
        self.attached_contexts.retain(|a| a.dedup_key() != key);
    }

    /// Attaches a picked or dropped local file directly as AI Context and reveals the panel.
    pub fn attach_local_file_context(&mut self, filename: String, content: &str) {
        let snippet: String = content.chars().take(1600).collect();
        self.attach_context(AiContextAttachment::LocalFile {
            filename: filename.clone(),
            size_bytes: content.len(),
            snippet,
        });
        self.request_focus_command = true;
        self.command_bar_expanded = true;
        self.push_trace(format!("Attached context @{filename}"), true);
    }

    /// Archives the current conversation (if non-empty) and starts a clean Antigravity thread.
    pub fn start_new_conversation(&mut self, ai_assistant: &mut AiAssistantView) {
        if ai_assistant.chat_history.len() > 1 {
            let title = ai_assistant
                .chat_history
                .iter()
                .find(|m| m.is_user)
                .map(|m| {
                    let s: String = m.message.chars().take(34).collect();
                    if m.message.chars().count() > 34 {
                        format!("{s}...")
                    } else {
                        s
                    }
                })
                .unwrap_or_else(|| self.conversation_title.clone());
            self.conversation_archives.push(AgentThreadArchive {
                title,
                messages: ai_assistant.chat_history.clone(),
                turn_meta: self.turn_metadata.clone(),
            });
            if self.conversation_archives.len() > 12 {
                let drop_n = self.conversation_archives.len() - 12;
                self.conversation_archives.drain(0..drop_n);
            }
        }
        ai_assistant.chat_history.clear();
        ai_assistant.chat_history.push(crate::models::ChatMessage {
            is_user: false,
            message: "Ready in Antigravity Agent Studio. Add a Vault folder, attach @ context, or ask me to fly Earth, generate BIM frames, or verify FEM/ARI redundancy.".to_string(),
            time: crate::core::web_bridge::unix_timestamp_label(),
        });
        self.turn_metadata.clear();
        self.conversation_title = "New Spatial Thread".to_string();
        self.context_picker_open = false;
        self.history_popover_open = false;
        self.sessions_dirty = true;
        self.push_trace("Started new Antigravity conversation", true);
    }

    /// Serializes Dynamic Island thread archives and active history for `Vault/AI_Context/`.
    pub fn export_sessions_json(&self, ai_assistant: &AiAssistantView) -> String {
        let bundle = DynamicIslandPersistedState {
            conversation_title: self.conversation_title.clone(),
            conversation_archives: self.conversation_archives.clone(),
            active_messages: ai_assistant.chat_history.clone(),
            turn_metadata: self.turn_metadata.clone(),
        };
        serde_json::to_string_pretty(&bundle).unwrap_or_default()
    }

    /// Restores Dynamic Island thread archives and active history from `Vault/AI_Context/`.
    pub fn restore_sessions_json(&mut self, json: &str, ai_assistant: &mut AiAssistantView) -> bool {
        let Ok(bundle) = serde_json::from_str::<DynamicIslandPersistedState>(json) else {
            return false;
        };
        if !bundle.conversation_title.trim().is_empty() {
            self.conversation_title = bundle.conversation_title;
        }
        self.conversation_archives = bundle.conversation_archives;
        if !bundle.active_messages.is_empty() {
            ai_assistant.chat_history = bundle.active_messages;
        }
        self.turn_metadata = bundle.turn_metadata;
        self.sessions_dirty = false;
        true
    }

    /// Queues an interactive proposal review card in `Planning` mode before applying model changes.
    pub fn queue_planning_proposal(
        &mut self,
        title: impl Into<String>,
        rationale: impl Into<String>,
        action_preview: impl Into<String>,
        actions: Vec<PlannedProposalAction>,
    ) -> u64 {
        let id = self.next_proposal_id;
        self.next_proposal_id += 1;
        self.pending_proposals.push(PlannedAgentProposal {
            id,
            title: title.into(),
            rationale: rationale.into(),
            action_preview: action_preview.into(),
            actions,
            status: ProposalStatus::Pending,
        });
        if self.pending_proposals.len() > 8 {
            let excess = self.pending_proposals.len() - 8;
            self.pending_proposals.drain(0..excess);
        }
        self.command_bar_expanded = true;
        id
    }

    /// Commits a list of planned structural/Earth actions via transactional `BimCommandBus` / canvas state.
    pub fn apply_proposal_actions(
        &mut self,
        planned: &[PlannedProposalAction],
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
    ) -> SpatialHudActions {
        let mut out = SpatialHudActions::default();
        for item in planned {
            match item {
                PlannedProposalAction::ExecuteBimNl(cmd) => {
                    if let Ok(msg) = build_view.bim_bus.execute_natural_language(cmd) {
                        self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                        universal_canvas.ground_origin_anchored = true;
                        out.sync_fem_and_earth = true;
                        out.fit_camera = true;
                        self.push_trace(msg.clone(), true);
                        ai_assistant.chat_history.push(crate::models::ChatMessage {
                            is_user: false,
                            message: format!("[Applied BIM Proposal]: {msg}"),
                            time: crate::core::web_bridge::unix_timestamp_label(),
                        });
                    }
                }
                PlannedProposalAction::AutoRepair => {
                    out.run_auto_repair = true;
                    self.push_trace("Queued Structural Auto-Repair", true);
                }
                PlannedProposalAction::FlyEarth {
                    lat,
                    lon,
                    zoom,
                    label,
                } => {
                    self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                    universal_canvas.globe_mode = true;
                    universal_canvas
                        .earth_world
                        .start_fly_to(&universal_canvas.world_map, *lat, *lon, *zoom as f32, label);
                    universal_canvas.ground_origin_lat_lon = (*lat, *lon);
                    universal_canvas.ground_origin_anchored = true;
                    self.selected_target = Some(SmartSelectionTarget::EarthSite {
                        lat: *lat,
                        lon: *lon,
                    });
                    self.push_trace(format!("Earth Fly-To -> {label}"), true);
                }
                PlannedProposalAction::AnchorSite { lat, lon } => {
                    out.anchor_and_build_site = Some((*lat, *lon));
                    self.push_trace(format!("Anchored BIM Site @ {lat:.3}°, {lon:.3}°"), true);
                }
                PlannedProposalAction::ApplyAriLens => {
                    self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                    self.apply_lens_to_views(
                        SpatialLens::AriRedundancy,
                        build_view,
                        universal_canvas,
                    );
                    out.sync_fem_and_earth = true;
                    self.push_trace("Applied ARI Redundancy Heatmap Lens", true);
                }
            }
        }
        self.sessions_dirty = true;
        out
    }

    /// Commits a pending proposal card by ID when the user clicks `Apply Changes`.
    pub fn apply_proposal_by_id(
        &mut self,
        id: u64,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
    ) -> SpatialHudActions {
        let mut to_run: Option<Vec<PlannedProposalAction>> = None;
        if let Some(p) = self.pending_proposals.iter_mut().find(|p| p.id == id) {
            if p.status == ProposalStatus::Pending {
                p.status = ProposalStatus::Applied;
                to_run = Some(p.actions.clone());
            }
        }
        if let Some(actions_list) = to_run {
            self.apply_proposal_actions(&actions_list, universal_canvas, build_view, ai_assistant)
        } else {
            SpatialHudActions::default()
        }
    }

    /// Dismisses a pending proposal card by ID when the user clicks `Reject`.
    pub fn reject_proposal_by_id(&mut self, id: u64, ai_assistant: &mut AiAssistantView) {
        let mut rejected_title: Option<String> = None;
        if let Some(p) = self.pending_proposals.iter_mut().find(|p| p.id == id) {
            if p.status == ProposalStatus::Pending {
                p.status = ProposalStatus::Rejected;
                rejected_title = Some(p.title.clone());
            }
        }
        if let Some(title) = rejected_title {
            self.push_trace(format!("Rejected proposal: {title}"), false);
            ai_assistant.chat_history.push(crate::models::ChatMessage {
                is_user: false,
                message: format!("[Planning Mode]: Proposal '{title}' rejected; model unchanged."),
                time: crate::core::web_bridge::unix_timestamp_label(),
            });
            self.sessions_dirty = true;
        }
    }

    /// Polls asynchronous Gemini REST API replies and routes any structured `[[...]]` directives.
    pub fn poll_gemini_replies(
        &mut self,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
    ) -> SpatialHudActions {
        let replies = crate::core::web_bridge::gemini_poll_responses();
        let mut out = SpatialHudActions::default();
        for reply in replies {
            if self.gemini_inflight_req_id == Some(reply.request_id) {
                self.gemini_inflight_req_id = None;
            }
            if reply.ok {
                let (clean_text, directives) = parse_gemini_action_directives(&reply.text);
                let msg_idx = ai_assistant.chat_history.len();
                let final_msg = if clean_text.is_empty() {
                    format!("[{}] Prepared {} engineering directive(s).", reply.model, directives.len())
                } else {
                    format!("[{}]:\n{}", reply.model, clean_text)
                };
                ai_assistant.chat_history.push(crate::models::ChatMessage {
                    is_user: false,
                    message: final_msg,
                    time: crate::core::web_bridge::unix_timestamp_label(),
                });
                self.turn_metadata.push(AntigravityTurnMeta {
                    msg_index: msg_idx,
                    mode_label: self.agent_mode.label().to_string(),
                    model_label: self.agent_model.short_label().to_string(),
                    context_chips: Vec::new(),
                    trace_steps: vec![format!("Gemini ({}) replied", reply.model)],
                });
                if !directives.is_empty() {
                    let mapped: Vec<PlannedProposalAction> = directives
                        .iter()
                        .map(|d| match d {
                            GeminiDirective::Fly { lat, lon, zoom } => PlannedProposalAction::FlyEarth {
                                lat: *lat,
                                lon: *lon,
                                zoom: *zoom,
                                label: format!("Gemini Target ({lat:.2}°, {lon:.2}°)"),
                            },
                            GeminiDirective::Bim(cmd) => PlannedProposalAction::ExecuteBimNl(cmd.clone()),
                            GeminiDirective::Repair => PlannedProposalAction::AutoRepair,
                            GeminiDirective::AriAudit => PlannedProposalAction::ApplyAriLens,
                        })
                        .collect();
                    let preview: String = directives
                        .iter()
                        .map(|d| match d {
                            GeminiDirective::Fly { lat, lon, .. } => format!("Fly({lat:.2},{lon:.2})"),
                            GeminiDirective::Bim(c) => format!("BIM:{c}"),
                            GeminiDirective::Repair => "AutoRepair".to_string(),
                            GeminiDirective::AriAudit => "ARI_Lens".to_string(),
                        })
                        .collect::<Vec<_>>()
                        .join(" + ");
                    if self.agent_mode == AgentExecutionMode::Planning {
                        self.queue_planning_proposal(
                            "Gemini Structured Action Plan",
                            "Review Gemini-proposed BIM / Earth changes before mutating the workspace.",
                            preview,
                            mapped,
                        );
                    } else {
                        let sub = self.apply_proposal_actions(&mapped, universal_canvas, build_view, ai_assistant);
                        out.merge(sub);
                    }
                }
                self.push_trace(format!("Gemini {} ready", reply.model), true);
                self.sessions_dirty = true;
            } else {
                let err = reply.error.unwrap_or_else(|| "Unknown Gemini API error".to_string());
                ai_assistant.chat_history.push(crate::models::ChatMessage {
                    is_user: false,
                    message: format!(
                        "[Gemini API Notice]: {err}\nClick the Key icon in Dynamic Island to verify your Gemini API Key or switch model to EigenAI Pro (offline FEM engine)."
                    ),
                    time: crate::core::web_bridge::unix_timestamp_label(),
                });
                self.push_trace("Gemini API request failed (check Key)", false);
                self.sessions_dirty = true;
            }
        }
        out
    }

    /// Formats all attached context pills into prompt metadata, trace step labels, and UI chip strings.
    pub fn build_enriched_context_payload(
        &self,
        project: &BimProject,
        members: &[StructuralMember],
        universal_canvas: &UniversalCanvasState,
    ) -> (Vec<String>, Vec<String>, String) {
        let mut chip_labels = Vec::new();
        let mut step_traces = Vec::new();
        let mut sections = Vec::new();

        for att in &self.attached_contexts {
            chip_labels.push(att.chip_label(project, self.selected_target));
            match att {
                AiContextAttachment::SelectedElement => {
                    if let Some(summary) =
                        self.format_selected_context_summary(project, members, universal_canvas)
                    {
                        step_traces.push("Read @Selected Element".to_string());
                        sections.push(format!("[Selected Element]: {summary}"));
                    }
                }
                AiContextAttachment::EarthSite => {
                    let (lat, lon) = match self.selected_target {
                        Some(SmartSelectionTarget::EarthSite { lat, lon }) => (lat, lon),
                        _ => universal_canvas.world_map.center_lat_lon,
                    };
                    let site = compute_site_environmental_coupling(
                        lat,
                        lon,
                        &universal_canvas.earth_world,
                    );
                    step_traces.push("Sampled Earth Site & Hazards".to_string());
                    sections.push(format!(
                        "[Earth Site ({lat:.3}°, {lon:.3}°) @ zoom {:.1}]: Elev {:.0}m, PGA Ss={:.2}g, Wind {:.0}m/s, Temp {:+.1}°C, CO2 {:.0} ppm",
                        universal_canvas.world_map.zoom_level,
                        site.elevation_m,
                        site.seismic_pga_ss_g,
                        site.design_wind_ms,
                        universal_canvas.earth_world.vitals.temp_anomaly_c,
                        universal_canvas.earth_world.vitals.co2_ppm,
                    ));
                }
                AiContextAttachment::BimGeometry => {
                    step_traces.push("Scanned BIM Model Topology".to_string());
                    sections.push(format!(
                        "[BIM Topology '{}']: {} Columns, {} Beams/Runs, {} Walls, {} Footings, {} Levels",
                        project.project_name,
                        project.columns.len(),
                        project.runs.len(),
                        project.walls.len(),
                        project.footings.len(),
                        project.levels.len(),
                    ));
                }
                AiContextAttachment::FemRedundancy => {
                    let crit = members.iter().filter(|m| m.is_critical || m.ari < 0.70).count();
                    let min_ari = members
                        .iter()
                        .map(|m| m.ari)
                        .fold(1.0_f64, f64::min);
                    let max_dcr = members
                        .iter()
                        .map(|m| m.utilization * 100.0)
                        .fold(0.0_f64, f64::max);
                    step_traces.push("Inspected FEM & ARI Redundancy".to_string());
                    sections.push(format!(
                        "[FEM & ARI Matrix]: {} members, Max DCR={:.1}%, Min ARI={:.3}, Critical (<0.70)={crit}",
                        members.len(),
                        max_dcr,
                        min_ari
                    ));
                }
                AiContextAttachment::ScheduleAndCost => {
                    let total_vol_m3: f64 = project
                        .columns
                        .iter()
                        .map(|c| (c.width_mm / 1000.0) * (c.depth_mm / 1000.0) * 3.5)
                        .sum::<f64>()
                        + project
                            .runs
                            .iter()
                            .map(|r| {
                                (r.width_mm / 1000.0)
                                    * (r.depth_mm / 1000.0)
                                    * (r.length_mm(0.0) / 1000.0)
                            })
                            .sum::<f64>();
                    let est_idr_m = total_vol_m3 * 3.85 + 240.0;
                    step_traces.push("Compiled 4D/5D QTO Metrics".to_string());
                    sections.push(format!(
                        "[4D/5D Summary]: Structural concrete volume ≈ {:.1} m³, Indicative RC budget ≈ Rp {:.1}M IDR",
                        total_vol_m3, est_idr_m
                    ));
                }
                AiContextAttachment::VaultWorkspace => {
                    let status = crate::core::InternalVault::connection_status();
                    let projs = crate::core::InternalVault::list_personal_vault_projects();
                    step_traces.push(format!("Read Vault '{}'", status.vault_name));
                    sections.push(format!(
                        "[Connected Vault '{}']: backend={:?}, projects={}, location={}",
                        status.vault_name, status.backend, projs.len(), status.location_label
                    ));
                }
                AiContextAttachment::VaultFile {
                    name,
                    rel_path,
                    size_bytes,
                    ..
                } => {
                    let snippet =
                        crate::core::InternalVault::read_vault_context_snippet(rel_path, 900)
                            .unwrap_or_default();
                    step_traces.push(format!("Read Vault file '{name}'"));
                    sections.push(format!(
                        "[Vault File '{rel_path}' ({size_bytes} B)]:\n{snippet}"
                    ));
                }
                AiContextAttachment::LocalFile {
                    filename,
                    size_bytes,
                    snippet,
                } => {
                    step_traces.push(format!("Read local file '{filename}'"));
                    sections.push(format!(
                        "[Attached File '{filename}' ({size_bytes} B)]:\n{snippet}"
                    ));
                }
            }
        }

        (chip_labels, step_traces, sections.join("\n"))
    }

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
                "/add-folder" | "/vault-folder" => {
                    actions.add_vault_folder = true;
                    self.vault_drawer_open = true;
                    self.push_trace("Choose local folder as Engineering Vault", true);
                }
                "/context" | "/add-context" => {
                    self.context_picker_open = true;
                    self.push_trace("Opened @ Add Context picker", true);
                }
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
            if self.agent_mode == AgentExecutionMode::Planning {
                let (lat, lon) = universal_canvas.ground_origin_lat_lon;
                let site = compute_site_environmental_coupling(lat, lon, &universal_canvas.earth_world);
                self.queue_planning_proposal(
                    "Synthesize Parametric Structural Frame",
                    format!(
                        "Planning Mode pre-check at Earth Site ({lat:.3}°, {lon:.3}°): Ss={:.2}g, Wind={:.0} m/s. Click Apply Changes to execute BIM synthesis.",
                        site.seismic_pga_ss_g, site.design_wind_ms
                    ),
                    format!("BIM Command: {norm}"),
                    vec![PlannedProposalAction::ExecuteBimNl(norm)],
                );
                ai_assistant.chat_history.push(crate::models::ChatMessage {
                    is_user: true,
                    message: trimmed.to_string(),
                    time: crate::core::web_bridge::unix_timestamp_label(),
                });
                ai_assistant.chat_history.push(crate::models::ChatMessage {
                    is_user: false,
                    message: "[Planning Mode]: Prepared structural frame proposal below. Click Apply Changes to build or Reject to cancel.".to_string(),
                    time: crate::core::web_bridge::unix_timestamp_label(),
                });
                self.sessions_dirty = true;
                return actions;
            }
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
                    self.sessions_dirty = true;
                    return actions;
                }
                Err(_) => {}
            }
        }

        // Collect Antigravity attached context chips + optional selected element context
        let (mut chip_labels, mut step_traces, ctx_markdown) =
            self.build_enriched_context_payload(&build_view.bim_bus.project, members, universal_canvas);

        let selected_extra = self.format_selected_context_summary(
            &build_view.bim_bus.project,
            members,
            universal_canvas,
        );
        let include_selected = selected_extra.is_some()
            && !self
                .attached_contexts
                .contains(&AiContextAttachment::SelectedElement)
            && (trimmed.contains('@')
                || lower.contains("this column")
                || lower.contains("this element")
                || lower.contains("selected"));
        if include_selected {
            if let Some(m) = self.active_context_mention(&build_view.bim_bus.project) {
                chip_labels.push(m);
            }
            step_traces.push("Read @Selected Element".to_string());
        }

        let mut prompt_sections = Vec::new();
        if !ctx_markdown.is_empty() {
            prompt_sections.push(ctx_markdown);
        }
        if include_selected {
            if let Some(ref s) = selected_extra {
                prompt_sections.push(format!("[Active Spatial Selection]: {s}"));
            }
        }
        let enriched_prompt = if prompt_sections.is_empty() {
            trimmed.to_string()
        } else {
            format!("{trimmed}\n\n{}", prompt_sections.join("\n"))
        };

        if self.conversation_title == "EigenAI Spatial Agent"
            || self.conversation_title == "New Spatial Thread"
        {
            let short_title: String = trimmed.chars().take(32).collect();
            self.conversation_title = if trimmed.chars().count() > 32 {
                format!("{short_title}...")
            } else {
                short_title
            };
        }

        // 4. Live Google Gemini REST Client when Gemini model + API Key are configured
        if matches!(
            self.agent_model,
            AgentModelChoice::Gemini25Flash | AgentModelChoice::GeminiLive
        ) && !self.gemini_api_key_draft.trim().is_empty()
        {
            let (lat, lon) = universal_canvas.ground_origin_lat_lon;
            let site = compute_site_environmental_coupling(lat, lon, &universal_canvas.earth_world);
            let sys_prompt = format!(
                "You are EigenAI inside EigenGraph (Geospatial Earth + Structural BIM/FEM & Adjusted Redundancy Index ARI platform). \
                 Respond concisely in professional civil & structural engineering English. \
                 Current Earth Site: ({lat:.3} deg, {lon:.3} deg), Elevation={:.1}m, Seismic Ss={:.2}g, Wind={:.1}m/s. \
                 Active BIM elements={}, FEM members={}. \
                 If the user asks to navigate Earth, synthesize a portal/frame, repair critical ARI members, or run an ARI audit, \
                 you may append bracket directives: [[FLY:lat,lon,zoom]], [[BIM:portal 3x2 @6m 2floors]], [[REPAIR]], [[ARI_AUDIT]].",
                site.elevation_m,
                site.seismic_pga_ss_g,
                site.design_wind_ms,
                build_view.bim_bus.project.element_count(),
                members.len()
            );
            let model_id = "gemini-2.5-flash";
            if let Ok(req_id) = crate::core::web_bridge::gemini_send_prompt(
                &self.gemini_api_key_draft,
                model_id,
                &sys_prompt,
                &enriched_prompt,
            ) {
                let user_idx = ai_assistant.chat_history.len();
                ai_assistant.chat_history.push(crate::models::ChatMessage {
                    is_user: true,
                    message: trimmed.to_string(),
                    time: crate::core::web_bridge::unix_timestamp_label(),
                });
                if !chip_labels.is_empty() || !step_traces.is_empty() {
                    self.turn_metadata.push(AntigravityTurnMeta {
                        msg_index: user_idx,
                        mode_label: self.agent_mode.label().to_string(),
                        model_label: self.agent_model.short_label().to_string(),
                        context_chips: chip_labels,
                        trace_steps: step_traces,
                    });
                }
                self.gemini_inflight_req_id = Some(req_id);
                self.command_bar_expanded = true;
                self.context_picker_open = false;
                self.sessions_dirty = true;
                self.push_trace(format!("Querying {}...", self.agent_model.short_label()), true);
                return actions;
            }
        }

        let user_idx = ai_assistant.chat_history.len();
        self.command_bar_expanded = true;
        self.context_picker_open = false;
        ai_assistant.execute_prompt(
            &enriched_prompt,
            self.active_overlay_pillar
                .unwrap_or(NavigationPillar::GeometryAssembly),
            members,
            projects,
            db_components,
        );
        let asst_idx = ai_assistant.chat_history.len().saturating_sub(1);

        if !chip_labels.is_empty() {
            self.turn_metadata.push(AntigravityTurnMeta {
                msg_index: user_idx,
                mode_label: self.agent_mode.label().to_string(),
                model_label: self.agent_model.short_label().to_string(),
                context_chips: chip_labels,
                trace_steps: Vec::new(),
            });
        }

        if asst_idx > user_idx {
            if self.agent_mode == AgentExecutionMode::Planning {
                step_traces.insert(0, "Planning Check".to_string());
            } else {
                step_traces.insert(0, "Fast Execution".to_string());
            }
            if !prompt_sections.is_empty() {
                if let Some(last_msg) = ai_assistant.chat_history.get_mut(asst_idx) {
                    if !last_msg.message.contains("[Attached") && !prompt_sections.is_empty() {
                        last_msg.message = format!(
                            "{}\n\nContext Applied:\n{}",
                            last_msg.message,
                            prompt_sections.join("\n")
                        );
                    }
                }
            }
            self.turn_metadata.push(AntigravityTurnMeta {
                msg_index: asst_idx,
                mode_label: self.agent_mode.label().to_string(),
                model_label: self.agent_model.short_label().to_string(),
                context_chips: Vec::new(),
                trace_steps: step_traces,
            });
        }

        if ai_assistant.pending_bim_nl_command.as_deref() == Some("__repair_critical__") {
            ai_assistant.pending_bim_nl_command = None;
            if self.agent_mode == AgentExecutionMode::Planning {
                self.queue_planning_proposal(
                    "Auto-Repair Critical ARI Members",
                    "Planning Mode detected repair request. Click Apply Changes to upgrade low-redundancy / overstressed elements cleanly via undoable BimCommand.",
                    "BimCommand::AutoRepairCritical",
                    vec![PlannedProposalAction::AutoRepair],
                );
            } else {
                actions.run_auto_repair = true;
            }
        } else if let Some(cmd) = ai_assistant.pending_bim_nl_command.take() {
            if self.agent_mode == AgentExecutionMode::Planning {
                self.queue_planning_proposal(
                    "Apply Proposed BIM Synthesis",
                    "Review BIM natural-language synthesis before updating the active workspace.",
                    format!("BIM Command: {cmd}"),
                    vec![PlannedProposalAction::ExecuteBimNl(cmd)],
                );
            } else if let Ok(msg) = build_view.bim_bus.execute_natural_language(&cmd) {
                self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                actions.sync_fem_and_earth = true;
                actions.fit_camera = true;
                self.push_trace(msg, true);
            }
        } else {
            self.push_trace(
                format!("{} responded", self.agent_model.short_label()),
                true,
            );
        }
        self.sessions_dirty = true;

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

    /// Renders all Antigravity floating HUD layers (legacy zero-vault wrapper kept compatible for tests).
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
        self.render_hud_overlays_with_vault(
            ctx,
            theme,
            universal_canvas,
            build_view,
            ai_assistant,
            members,
            projects,
            db_components,
            &[],
        )
    }

    /// Renders all Floating HUD layers over the 100% Full-Canvas workspace (ZERO side panels):
    /// 1. Minimal Top-Left Identity & View Mode Capsule
    /// 2. Conditional Top-Center Active Lens / Tool Pill (only when non-Orbit/Clean)
    /// 3. Smart Element Pop-Up Inspector (only when an element or Earth location is clicked)
    /// 4. Hover-Expandable Bottom-Center Dynamic Island (Vault, Context, AI Selector, Manual Draw & Chat)
    #[allow(clippy::too_many_arguments)]
    pub fn render_hud_overlays_with_vault(
        &mut self,
        ctx: &egui::Context,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
        vault_projects: &[crate::core::vault::PersonalVaultProjectEntry],
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();
        self.hud_rects.clear();

        let vp = MacTheme::viewport_size(ctx);
        let is_mobile = vp.x < MacTheme::MOBILE_BREAKPOINT;

        // 1. Top-Left Minimal Capsule (always unobtrusive on the pure canvas)
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

        // 2. Conditional Top-Center Active Lens / Manual Tool Pill (hidden in Clean + Orbit mode)
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
                                match build_view.ground_author_tool {
                                    Ground3DAuthorTool::Column => {
                                        ui.label(RichText::new("W:").size(10.0).color(theme.text_muted));
                                        ui.add(
                                            egui::DragValue::new(&mut build_view.author_col_width_mm)
                                                .range(150.0..=2000.0)
                                                .speed(25.0)
                                                .suffix("mm"),
                                        );
                                        ui.label(RichText::new("D:").size(10.0).color(theme.text_muted));
                                        ui.add(
                                            egui::DragValue::new(&mut build_view.author_col_depth_mm)
                                                .range(150.0..=2000.0)
                                                .speed(25.0)
                                                .suffix("mm"),
                                        );
                                    }
                                    Ground3DAuthorTool::Beam => {
                                        ui.label(RichText::new("b:").size(10.0).color(theme.text_muted));
                                        ui.add(
                                            egui::DragValue::new(&mut build_view.author_beam_width_mm)
                                                .range(120.0..=1500.0)
                                                .speed(25.0)
                                                .suffix("mm"),
                                        );
                                        ui.label(RichText::new("h:").size(10.0).color(theme.text_muted));
                                        ui.add(
                                            egui::DragValue::new(&mut build_view.author_beam_depth_mm)
                                                .range(150.0..=2500.0)
                                                .speed(25.0)
                                                .suffix("mm"),
                                        );
                                    }
                                    Ground3DAuthorTool::Wall => {
                                        ui.label(RichText::new("t:").size(10.0).color(theme.text_muted));
                                        ui.add(
                                            egui::DragValue::new(&mut build_view.author_wall_thickness_mm)
                                                .range(80.0..=1200.0)
                                                .speed(10.0)
                                                .suffix("mm"),
                                        );
                                    }
                                    Ground3DAuthorTool::Footing => {
                                        ui.label(RichText::new("Size:").size(10.0).color(theme.text_muted));
                                        ui.add(
                                            egui::DragValue::new(&mut build_view.author_footing_size_mm)
                                                .range(400.0..=6000.0)
                                                .speed(50.0)
                                                .suffix("mm"),
                                        );
                                    }
                                    Ground3DAuthorTool::Orbit => {}
                                }
                                ui.label(
                                    RichText::new("(Esc: Orbit)")
                                        .size(9.8)
                                        .color(theme.text_muted),
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
                                "Reset active lens & manual tool",
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

        // 3. Smart Element Pop-Up Inspector (ONLY visible when an element or Earth site is clicked)
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

        // 4. Unified Hover-Expandable Floating Dynamic Island
        let island_actions = self.render_dynamic_island(
            ctx,
            theme,
            universal_canvas,
            build_view,
            ai_assistant,
            members,
            projects,
            db_components,
            vault_projects,
            vp,
            is_mobile,
        );
        actions.merge(island_actions);

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
                        AppIcon::Column,
                        "Anchor & Build Here",
                        10.5,
                        Color32::WHITE,
                        theme.accent_green,
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Anchor BIM Origin at this Earth site and open 3D Structural Modeling")
                    .clicked()
                    {
                        universal_canvas.ground_origin_lat_lon = (lat, lon);
                        universal_canvas.ground_origin_anchored = true;
                        self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                        if build_view.bim_bus.project.is_empty() {
                            build_view.ground_author_tool = Ground3DAuthorTool::Column;
                        }
                        actions.anchor_and_build_site = Some((lat, lon));
                        self.push_trace(
                            format!("Anchored & Opened 3D BIM @ {lat:.3}°, {lon:.3}°"),
                            true,
                        );
                    }
                    if icons::icon_button(
                        ui,
                        AppIcon::Grid,
                        "Build 3x2 Portal",
                        10.5,
                        Color32::WHITE,
                        theme.accent_orange,
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
                self.attach_context(AiContextAttachment::SelectedElement);
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
            format!("Add @{elem_name} to Antigravity AI Context"),
            10.5,
            Color32::WHITE,
            theme.accent_blue,
            Stroke::NONE,
            CornerRadius::same(8),
            Vec2::new(ui.available_width(), 26.0),
        )
        .clicked()
        {
            self.attach_context(AiContextAttachment::SelectedElement);
            self.command_bar_expanded = true;
            self.request_focus_command = true;
        }
    }

    /// Shared `/` Slash Command Autocomplete card used in both Side Panel and Collapsed Command Dock.
    #[allow(clippy::too_many_arguments)]
    fn render_slash_palette_box(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
        pal_w: f32,
        max_h: f32,
        salt: &str,
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();
        let q = self.command_input.trim().to_lowercase();
        let mut queued_cmd: Option<String> = None;

        Self::antigravity_glass_frame(theme, 12, Margin::same(9)).show(ui, |ui| {
            ui.set_width(pal_w);
            ui.set_max_width(pal_w);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            ui.horizontal(|ui| {
                icons::icon_label(
                    ui,
                    AppIcon::Sparkles,
                    12.5,
                    theme.accent_blue,
                    "Slash Commands",
                    11.5,
                    theme.text_primary,
                    true,
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new("Enter to run • Esc")
                            .size(9.5)
                            .color(theme.text_muted),
                    );
                });
            });
            ui.add_space(4.0);
            ScrollArea::vertical()
                .id_salt(salt)
                .max_height(max_h)
                .show(ui, |ui| {
                    let btn_w = (pal_w - 24.0).max(120.0);
                    ui.set_width(btn_w);
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
                            Vec2::new(btn_w, 24.0),
                            egui::Button::new(
                                RichText::new(format!(
                                    "{}   •   [{}] {}",
                                    spec.cmd, spec.category, spec.description
                                ))
                                .size(10.5)
                                .color(theme.text_primary),
                            )
                            .wrap_mode(egui::TextWrapMode::Wrap)
                            .fill(theme.bg_card.gamma_multiply(0.88))
                            .stroke(Stroke::new(0.8, theme.border_subtle))
                            .corner_radius(CornerRadius::same(7)),
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
            let sub = self.execute_spatial_input(
                &cmd,
                universal_canvas,
                build_view,
                ai_assistant,
                members,
                projects,
                db_components,
            );
            actions.merge(sub);
        }
        actions
    }

    /// Antigravity Workspace / Vault Header Bar with `+ Add Folder (Vault)` & inline Vault Explorer.
    fn render_vault_workspace_bar(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        vault_projects: &[crate::core::vault::PersonalVaultProjectEntry],
        panel_w: f32,
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();
        let status = crate::core::InternalVault::connection_status();
        let vault_files = crate::core::InternalVault::list_vault_context_files(18);

        let card_bg = if theme.is_dark() {
            Color32::from_rgb(25, 31, 44)
        } else {
            theme.bg_card
        };
        Frame::new()
            .fill(card_bg)
            .corner_radius(CornerRadius::same(10))
            .stroke(Stroke::new(
                1.0,
                if status.connected {
                    theme.accent_green.gamma_multiply(0.35)
                } else {
                    theme.border_card
                },
            ))
            .inner_margin(Margin::symmetric(10, 7))
            .show(ui, |ui| {
                let inner_w = (panel_w - 22.0).max(180.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                ui.horizontal(|ui| {
                    let badge_col = if status.connected {
                        theme.accent_green
                    } else {
                        theme.accent_blue
                    };
                    let title_txt = if status.connected {
                        format!("Vault: {}", status.vault_name)
                    } else {
                        "Engineering Vault".to_string()
                    };
                    icons::icon_label(
                        ui,
                        AppIcon::Folder,
                        13.5,
                        badge_col,
                        &title_txt,
                        11.5,
                        theme.text_primary,
                        true,
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::icon_only_button(
                            ui,
                            AppIcon::SlidersPanel,
                            theme.text_secondary,
                            Color32::TRANSPARENT,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::splat(20.0),
                            "Vault folder options & IndexedDB/Path setup",
                        )
                        .clicked()
                        {
                            self.vault_path_popover_open = !self.vault_path_popover_open;
                        }

                        if status.connected {
                            if icons::icon_only_button(
                                ui,
                                if self.vault_drawer_open {
                                    AppIcon::ChevronUp
                                } else {
                                    AppIcon::ChevronDown
                                },
                                theme.text_secondary,
                                Color32::TRANSPARENT,
                                Stroke::NONE,
                                CornerRadius::same(6),
                                Vec2::splat(20.0),
                                "Toggle Vault Files Explorer",
                            )
                            .clicked()
                            {
                                self.vault_drawer_open = !self.vault_drawer_open;
                            }
                            if icons::icon_only_button(
                                ui,
                                AppIcon::SaveDisk,
                                theme.accent_green,
                                theme.accent_green.gamma_multiply(0.14),
                                Stroke::NONE,
                                CornerRadius::same(6),
                                Vec2::splat(20.0),
                                "Save current workspace into Vault",
                            )
                            .clicked()
                            {
                                actions.save_active_to_vault = true;
                            }
                        }

                        if icons::icon_button(
                            ui,
                            AppIcon::Plus,
                            if status.connected { "Switch" } else { "Add Folder" },
                            10.0,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(7),
                            Vec2::new(0.0, 21.0),
                        )
                        .on_hover_text("Select a local folder to mount as your Engineering Vault")
                        .clicked()
                        {
                            actions.add_vault_folder = true;
                        }
                    });
                });

                ui.add_space(2.0);
                if status.connected {
                    ui.horizontal_wrapped(|ui| {
                        let backend_lbl = match status.backend {
                            crate::core::vault::VaultBackendKind::WebDirectoryPicker => "Browser FS",
                            crate::core::vault::VaultBackendKind::BrowserIndexedDb => "IndexedDB",
                            crate::core::vault::VaultBackendKind::LocalFilesystem => "Local Folder",
                            crate::core::vault::VaultBackendKind::Unbound => "Unbound",
                        };
                        ui.label(
                            RichText::new(format!(
                                "{backend_lbl} • {} project(s) • {} file(s)",
                                vault_projects.len(),
                                vault_files.len()
                            ))
                            .size(10.0)
                            .color(theme.text_muted),
                        );
                        if ui
                            .small_button("+ @Vault")
                            .on_hover_text("Attach connected Vault workspace overview to AI Context")
                            .clicked()
                        {
                            self.attach_context(AiContextAttachment::VaultWorkspace);
                        }
                    });
                } else {
                    ui.label(
                        RichText::new("Connect a local folder to store .eigen models & attach Vault context.")
                            .size(10.0)
                            .color(theme.text_muted),
                    );
                }

                if let Some(ref fb) = self.vault_feedback {
                    ui.add_space(2.0);
                    ui.label(RichText::new(fb).size(9.8).color(theme.accent_orange));
                }

                if self.vault_path_popover_open {
                    ui.add_space(5.0);
                    ui.separator();
                    ui.add_space(5.0);
                    let outcome = crate::ui_kit::render_obsidian_vault_panel(
                        ui,
                        theme,
                        &mut self.vault_inline_path,
                        &mut self.vault_inline_name,
                        true,
                        false,
                        false,
                    );
                    if let Some(fb) = outcome.feedback {
                        self.vault_feedback = Some(fb);
                    }
                    if outcome.state_changed {
                        actions.refresh_vault = true;
                    }
                }

                if status.connected && self.vault_drawer_open {
                    ui.add_space(5.0);
                    ui.separator();
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Vault Files")
                                .size(10.0)
                                .strong()
                                .color(theme.text_secondary),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.small_button("Full Vault").clicked() {
                                self.open_module_overlay(NavigationPillar::MyLibrary);
                            }
                            if ui.small_button("Refresh").clicked() {
                                actions.refresh_vault = true;
                            }
                        });
                    });
                    ScrollArea::vertical()
                        .id_salt("antigravity_vault_explorer_scroll")
                        .max_height(132.0)
                        .show(ui, |ui| {
                            ui.set_max_width((inner_w - 10.0).max(140.0));
                            if vault_projects.is_empty() && vault_files.is_empty() {
                                ui.label(
                                    RichText::new("Empty Vault. Click Save to store the active project.")
                                        .size(10.0)
                                        .color(theme.text_muted),
                                );
                            }
                            for proj in vault_projects.iter().take(10) {
                                ui.horizontal(|ui| {
                                    if ui
                                        .small_button("@")
                                        .on_hover_text("Add project to AI Context")
                                        .clicked()
                                    {
                                        self.attach_context(AiContextAttachment::VaultFile {
                                            name: proj.name.clone(),
                                            category: proj.category.clone(),
                                            rel_path: proj.rel_path.clone(),
                                            size_bytes: proj.size_bytes,
                                        });
                                    }
                                    if ui
                                        .button(
                                            RichText::new(&proj.name)
                                                .size(10.5)
                                                .color(theme.text_primary),
                                        )
                                        .on_hover_text("Load Vault project onto Earth/BIM canvas")
                                        .clicked()
                                    {
                                        actions.open_vault_project = Some(proj.clone());
                                    }
                                });
                            }
                            for vf in vault_files.iter().take(12) {
                                if vault_projects.iter().any(|p| p.rel_path == vf.rel_path) {
                                    continue;
                                }
                                ui.horizontal(|ui| {
                                    if ui
                                        .small_button("+@")
                                        .on_hover_text("Attach file into AI Context")
                                        .clicked()
                                    {
                                        self.attach_context(AiContextAttachment::VaultFile {
                                            name: vf.name.clone(),
                                            category: vf.category.clone(),
                                            rel_path: vf.rel_path.clone(),
                                            size_bytes: vf.size_bytes,
                                        });
                                    }
                                    ui.label(
                                        RichText::new(format!("{} ({})", vf.name, vf.category))
                                            .size(10.0)
                                            .color(theme.text_secondary),
                                    );
                                });
                            }
                        });
                }
            });

        actions
    }

    /// Interactive `@ Add Context` popover allowing live workspace domains, Vault files & disk files.
    fn render_context_picker_popover(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        project: &BimProject,
        vault_projects: &[crate::core::vault::PersonalVaultProjectEntry],
        panel_w: f32,
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();
        let vault_files = crate::core::InternalVault::list_vault_context_files(14);
        let q = self.context_search_query.trim().to_lowercase();

        Frame::new()
            .fill(if theme.is_dark() {
                Color32::from_rgb(24, 30, 42)
            } else {
                theme.bg_card
            })
            .corner_radius(CornerRadius::same(12))
            .stroke(Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.45)))
            .inner_margin(Margin::same(9))
            .show(ui, |ui| {
                let box_w = (panel_w - 22.0).max(160.0);
                ui.set_width(box_w);
                ui.set_max_width(box_w);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                ui.horizontal(|ui| {
                    icons::icon_label(
                        ui,
                        AppIcon::Layers,
                        12.5,
                        theme.accent_blue,
                        "Add Context (@)",
                        11.5,
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
                            Vec2::splat(18.0),
                            "Close context picker",
                        )
                        .clicked()
                        {
                            self.context_picker_open = false;
                        }
                    });
                });
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.context_search_query)
                        .desired_width((box_w - 6.0).max(130.0))
                        .hint_text("Filter domains, Vault files, or attach local file..."),
                );
                ui.add_space(5.0);

                ScrollArea::vertical()
                    .id_salt("antigravity_add_context_scroll")
                    .max_height(185.0)
                    .show(ui, |ui| {
                        let inner_w = (box_w - 12.0).max(140.0);
                        ui.set_max_width(inner_w);

                        let domains = [
                            (
                                AiContextAttachment::SelectedElement,
                                "Current Selection",
                                self.active_context_mention(project)
                                    .unwrap_or_else(|| "Click any column, beam, or Earth site".to_string()),
                            ),
                            (
                                AiContextAttachment::EarthSite,
                                "Earth Viewport & Hazards",
                                "Coordinates, elevation, seismic PGA Ss & climate".to_string(),
                            ),
                            (
                                AiContextAttachment::BimGeometry,
                                "BIM Structural Topology",
                                format!(
                                    "{} cols, {} beams, {} walls",
                                    project.columns.len(),
                                    project.runs.len(),
                                    project.walls.len()
                                ),
                            ),
                            (
                                AiContextAttachment::FemRedundancy,
                                "FEM Stiffness & ARI Matrix",
                                "DCR%, SVD nullspace & critical members".to_string(),
                            ),
                            (
                                AiContextAttachment::ScheduleAndCost,
                                "4D Schedule & 5D BoQ Cost",
                                "Concrete QTO, AHSP budget & erection duration".to_string(),
                            ),
                            (
                                AiContextAttachment::VaultWorkspace,
                                "Connected Vault Workspace",
                                "Vault summary, categories & knowledge memory".to_string(),
                            ),
                        ];

                        for (att, title, sub) in domains {
                            if !q.is_empty()
                                && !title.to_lowercase().contains(&q)
                                && !sub.to_lowercase().contains(&q)
                            {
                                continue;
                            }
                            let key = att.dedup_key();
                            let active = self.attached_contexts.iter().any(|a| a.dedup_key() == key);
                            let btn_fill = if active {
                                theme.accent_blue.gamma_multiply(0.18)
                            } else {
                                theme.bg_card_hover.gamma_multiply(0.45)
                            };
                            if ui
                                .add_sized(
                                    Vec2::new(inner_w, 26.0),
                                    egui::Button::new(
                                        RichText::new(format!(
                                            "{} {} — {}",
                                            if active { "[Attached]" } else { "+" },
                                            title,
                                            sub
                                        ))
                                        .size(10.2)
                                        .color(theme.text_primary),
                                    )
                                    .wrap_mode(egui::TextWrapMode::Wrap)
                                    .fill(btn_fill)
                                    .stroke(Stroke::new(0.8, theme.border_subtle))
                                    .corner_radius(CornerRadius::same(7)),
                                )
                                .clicked()
                            {
                                if active {
                                    self.detach_context(&key);
                                } else {
                                    self.attach_context(att);
                                }
                            }
                            ui.add_space(2.0);
                        }

                        if !vault_projects.is_empty() || !vault_files.is_empty() {
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("VAULT FILES")
                                    .size(9.5)
                                    .strong()
                                    .color(theme.text_muted),
                            );
                            for vp_entry in vault_projects.iter().take(6) {
                                if !q.is_empty() && !vp_entry.name.to_lowercase().contains(&q) {
                                    continue;
                                }
                                let att = AiContextAttachment::VaultFile {
                                    name: vp_entry.name.clone(),
                                    category: vp_entry.category.clone(),
                                    rel_path: vp_entry.rel_path.clone(),
                                    size_bytes: vp_entry.size_bytes,
                                };
                                let key = att.dedup_key();
                                let active =
                                    self.attached_contexts.iter().any(|a| a.dedup_key() == key);
                                if ui
                                    .add_sized(
                                        Vec2::new(inner_w, 24.0),
                                        egui::Button::new(
                                            RichText::new(format!(
                                                "{} {} ({})",
                                                if active { "[Attached]" } else { "+ @" },
                                                vp_entry.name,
                                                vp_entry.category
                                            ))
                                            .size(10.0)
                                            .color(theme.text_primary),
                                        )
                                        .wrap_mode(egui::TextWrapMode::Wrap)
                                        .fill(theme.bg_card)
                                        .stroke(Stroke::new(0.8, theme.border_subtle))
                                        .corner_radius(CornerRadius::same(6)),
                                    )
                                    .clicked()
                                {
                                    if active {
                                        self.detach_context(&key);
                                    } else {
                                        self.attach_context(att);
                                    }
                                }
                                ui.add_space(2.0);
                            }
                            for vf in vault_files.iter().take(6) {
                                if vault_projects.iter().any(|p| p.rel_path == vf.rel_path) {
                                    continue;
                                }
                                if !q.is_empty() && !vf.rel_path.to_lowercase().contains(&q) {
                                    continue;
                                }
                                let att = AiContextAttachment::VaultFile {
                                    name: vf.name.clone(),
                                    category: vf.category.clone(),
                                    rel_path: vf.rel_path.clone(),
                                    size_bytes: vf.size_bytes,
                                };
                                let key = att.dedup_key();
                                let active =
                                    self.attached_contexts.iter().any(|a| a.dedup_key() == key);
                                if ui
                                    .add_sized(
                                        Vec2::new(inner_w, 24.0),
                                        egui::Button::new(
                                            RichText::new(format!(
                                                "{} {}",
                                                if active { "[Attached]" } else { "+ @" },
                                                vf.rel_path
                                            ))
                                            .size(10.0)
                                            .color(theme.text_primary),
                                        )
                                        .wrap_mode(egui::TextWrapMode::Wrap)
                                        .fill(theme.bg_card)
                                        .stroke(Stroke::new(0.8, theme.border_subtle))
                                        .corner_radius(CornerRadius::same(6)),
                                    )
                                    .clicked()
                                {
                                    if active {
                                        self.detach_context(&key);
                                    } else {
                                        self.attach_context(att);
                                    }
                                }
                                ui.add_space(2.0);
                            }
                        }

                        ui.add_space(4.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::UploadImport,
                            "Attach Local File (.ifc, .json, .csv, .md)...",
                            10.5,
                            theme.accent_blue,
                            theme.accent_blue.gamma_multiply(0.12),
                            Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.4)),
                            CornerRadius::same(7),
                            Vec2::new(inner_w, 25.0),
                        )
                        .clicked()
                        {
                            actions.pick_local_context_file = true;
                            self.context_picker_open = false;
                        }
                    });
            });

        actions
    }

    /// Renders the interactive Hover-Expandable Floating Dynamic Island at bottom-center:
    /// - Compact Idle Pill when pointer is away on the Earth/BIM canvas (0 side panels)
    /// - Smooth Hover-Expanded Dynamic Island exposing Vault `+ Add Folder`, `+ Add Context (@)`,
    ///   AI Model & Mode Switcher, Manual Drawing Commands, `/` Palette, Chat Drawer & Composer.
    #[allow(clippy::too_many_arguments)]
    fn render_dynamic_island(
        &mut self,
        ctx: &egui::Context,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
        vault_projects: &[crate::core::vault::PersonalVaultProjectEntry],
        vp: Vec2,
        is_mobile: bool,
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();
        let slash_mode = self.command_input.trim_start().starts_with('/');
        let vault_status = crate::core::InternalVault::connection_status();

        let should_expand = self.is_dynamic_island_expanded(ctx);
        self.island_expanded_prev = should_expand;
        let expand_t = ctx.animate_bool(
            Id::new("eigengraph_dynamic_island_expand"),
            should_expand,
        );

        let compact_w = if is_mobile {
            (vp.x - 24.0).clamp(240.0, 340.0)
        } else {
            390.0_f32.min((vp.x - 40.0).max(260.0))
        };
        let expanded_w = if is_mobile {
            (vp.x - 16.0).clamp(240.0, 420.0)
        } else {
            (vp.x * 0.62).clamp(520.0, 720.0)
        };
        let dock_w = egui::lerp(compact_w..=expanded_w, expand_t);

        let dock_resp = Area::new(Id::new("antigravity_ai_command_dock"))
            .order(Order::Foreground)
            .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -14.0))
            .constrain(true)
            .show(ctx, |ui| {
                ui.set_width(dock_w);
                ui.set_max_width(dock_w);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                ui.vertical(|ui| {
                    // A. Subtle execution trace badge above the Dynamic Island when idle
                    if !should_expand && !self.recent_trace_chips.is_empty() && !slash_mode {
                        ui.horizontal_wrapped(|ui| {
                            for chip in self
                                .recent_trace_chips
                                .iter()
                                .rev()
                                .take(if is_mobile { 1 } else { 2 })
                            {
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

                    if !should_expand {
                        // B1. COMPACT IDLE DYNAMIC ISLAND PILL (automatically grows on cursor hover)
                        let pill_frame = Self::antigravity_glass_frame(
                            theme,
                            24,
                            Margin::symmetric(12, 7),
                        )
                        .show(ui, |ui| {
                            let cap_w = (dock_w - 26.0).max(180.0);
                            ui.set_width(cap_w);
                            ui.set_max_width(cap_w);
                            ui.horizontal(|ui| {
                                icons::icon_label(
                                    ui,
                                    AppIcon::Sparkles,
                                    14.0,
                                    theme.accent_blue,
                                    "EigenAI",
                                    12.0,
                                    theme.text_primary,
                                    true,
                                );

                                let vault_col = if vault_status.connected {
                                    theme.accent_green
                                } else {
                                    theme.text_muted
                                };
                                let vault_badge = if vault_status.connected {
                                    if is_mobile {
                                        "Vault".to_string()
                                    } else {
                                        format!("Vault: {}", vault_status.vault_name)
                                    }
                                } else {
                                    "+ Vault".to_string()
                                };
                                icons::pill_badge(ui, AppIcon::Folder, vault_badge, vault_col);

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

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(if is_mobile { "Tap" } else { "Hover" })
                                            .size(10.0)
                                            .color(theme.text_muted),
                                    );
                                    ui.label(
                                        RichText::new(self.agent_model.short_label())
                                            .size(10.0)
                                            .color(theme.accent_blue),
                                    );
                                });
                            });
                        });
                        if pill_frame.response.interact(egui::Sense::click()).clicked() {
                            self.command_bar_expanded = true;
                            self.request_focus_command = true;
                        }
                        return;
                    }

                    // B2. HOVER-EXPANDED FLOATING DYNAMIC ISLAND
                    // Popover drawers sit cleanly inside top of the expanding island
                    if self.history_popover_open {
                        Frame::new()
                            .fill(theme.bg_card)
                            .corner_radius(CornerRadius::same(12))
                            .stroke(Stroke::new(1.0, theme.border_card))
                            .inner_margin(Margin::same(10))
                            .show(ui, |ui| {
                                let hw = (dock_w - 22.0).max(180.0);
                                ui.set_width(hw);
                                ui.set_max_width(hw);
                                ui.horizontal(|ui| {
                                    icons::icon_label(
                                        ui,
                                        AppIcon::Clock,
                                        12.5,
                                        theme.accent_blue,
                                        "Recent Agent Sessions",
                                        11.0,
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
                                            Vec2::splat(18.0),
                                            "Close history",
                                        )
                                        .clicked()
                                        {
                                            self.history_popover_open = false;
                                        }
                                    });
                                });
                                ui.add_space(4.0);
                                if self.conversation_archives.is_empty() {
                                    ui.label(
                                        RichText::new(
                                            "No archived chats yet. Click + to start a new thread.",
                                        )
                                        .size(10.0)
                                        .color(theme.text_muted),
                                    );
                                } else {
                                    let mut restore_idx = None;
                                    for (idx, thr) in
                                        self.conversation_archives.iter().enumerate().rev()
                                    {
                                        if ui
                                            .button(
                                                RichText::new(format!(
                                                    "{} ({} msgs)",
                                                    thr.title,
                                                    thr.messages.len()
                                                ))
                                                .size(10.5)
                                                .color(theme.text_primary),
                                            )
                                            .clicked()
                                        {
                                            restore_idx = Some(idx);
                                        }
                                    }
                                    if let Some(idx) = restore_idx {
                                        let chosen = self.conversation_archives.remove(idx);
                                        self.conversation_title = chosen.title;
                                        ai_assistant.chat_history = chosen.messages;
                                        self.turn_metadata = chosen.turn_meta;
                                        self.history_popover_open = false;
                                        self.command_bar_expanded = true;
                                    }
                                }
                            });
                        ui.add_space(6.0);
                    }

                    if self.api_key_popover_open {
                        Frame::new()
                            .fill(theme.bg_card)
                            .corner_radius(CornerRadius::same(12))
                            .stroke(Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.55)))
                            .inner_margin(Margin::same(10))
                            .show(ui, |ui| {
                                let kw = (dock_w - 22.0).max(180.0);
                                ui.set_width(kw);
                                ui.set_max_width(kw);
                                ui.horizontal(|ui| {
                                    icons::icon_label(
                                        ui,
                                        AppIcon::Lock,
                                        12.5,
                                        theme.accent_blue,
                                        "Gemini API Key (Quick Setup)",
                                        11.0,
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
                                            Vec2::splat(18.0),
                                            "Close API key setup",
                                        )
                                        .clicked()
                                        {
                                            self.api_key_popover_open = false;
                                        }
                                    });
                                });
                                ui.add_space(4.0);
                                ui.horizontal_wrapped(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.gemini_api_key_draft)
                                            .password(true)
                                            .hint_text("AIza... (synced with Settings)")
                                            .desired_width((kw - 165.0).max(120.0)),
                                    );
                                    if icons::icon_button(
                                        ui,
                                        AppIcon::CheckCircle,
                                        "Use Gemini 2.5",
                                        10.0,
                                        Color32::WHITE,
                                        theme.accent_green,
                                        Stroke::NONE,
                                        CornerRadius::same(7),
                                        Vec2::new(0.0, 22.0),
                                    )
                                    .clicked()
                                    {
                                        if !self.gemini_api_key_draft.trim().is_empty() {
                                            self.agent_model = AgentModelChoice::Gemini25Flash;
                                            self.push_trace("Gemini 2.5 Flash API Key armed", true);
                                        }
                                        self.api_key_popover_open = false;
                                    }
                                    if !self.gemini_api_key_draft.is_empty()
                                        && ui.small_button("Clear").clicked()
                                    {
                                        self.gemini_api_key_draft.clear();
                                        self.agent_model = AgentModelChoice::EigenAiProFem;
                                        self.push_trace("Cleared Gemini API key (Offline EigenAI)", true);
                                    }
                                });
                                ui.label(
                                    RichText::new("Used only for live Google Gemini REST calls from Dynamic Island. Empty key automatically falls back to offline EigenAI Pro.")
                                        .size(9.5)
                                        .color(theme.text_muted),
                                );
                            });
                        ui.add_space(6.0);
                    }

                    if self.vault_drawer_open || self.vault_path_popover_open {
                        let vault_act = self.render_vault_workspace_bar(
                            ui,
                            theme,
                            vault_projects,
                            dock_w - 4.0,
                        );
                        actions.merge(vault_act);
                        ui.add_space(6.0);
                    }

                    if slash_mode {
                        let sub = self.render_slash_palette_box(
                            ui,
                            theme,
                            universal_canvas,
                            build_view,
                            ai_assistant,
                            members,
                            projects,
                            db_components,
                            (dock_w - 24.0).max(200.0),
                            (vp.y * 0.30).clamp(140.0, 230.0),
                            "antigravity_slash_palette_scroll",
                        );
                        actions.merge(sub);
                        ui.add_space(6.0);
                    } else if self.context_picker_open {
                        let sub = self.render_context_picker_popover(
                            ui,
                            theme,
                            &build_view.bim_bus.project,
                            vault_projects,
                            dock_w - 4.0,
                        );
                        actions.merge(sub);
                        ui.add_space(6.0);
                    }

                    if self.command_bar_expanded && !slash_mode {
                        let chat_h = (vp.y * 0.25).clamp(110.0, 210.0);
                        Self::antigravity_glass_frame(theme, 16, Margin::same(10)).show(ui, |ui| {
                            let chat_w = (dock_w - 24.0).max(190.0);
                            ui.set_width(chat_w);
                            ui.set_max_width(chat_w);
                            ui.horizontal(|ui| {
                                icons::icon_label(
                                    ui,
                                    AppIcon::Bot,
                                    14.0,
                                    theme.accent_blue,
                                    &self.conversation_title,
                                    12.0,
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
                                        Vec2::splat(20.0),
                                        "Collapse Chat Drawer",
                                    )
                                    .clicked()
                                    {
                                        self.command_bar_expanded = false;
                                    }
                                    if icons::icon_button(
                                        ui,
                                        AppIcon::Wrench,
                                        "Auto-Repair ARI",
                                        9.8,
                                        Color32::WHITE,
                                        theme.accent_green,
                                        Stroke::NONE,
                                        CornerRadius::same(6),
                                        Vec2::new(0.0, 20.0),
                                    )
                                    .clicked()
                                    {
                                        actions.run_auto_repair = true;
                                        self.push_trace("Auto-Repaired Critical Members", true);
                                    }
                                    if icons::icon_only_button(
                                        ui,
                                        AppIcon::Plus,
                                        theme.text_primary,
                                        theme.bg_card,
                                        Stroke::new(0.8, theme.border_subtle),
                                        CornerRadius::same(6),
                                        Vec2::splat(20.0),
                                        "New Chat",
                                    )
                                    .clicked()
                                    {
                                        self.start_new_conversation(ai_assistant);
                                    }
                                });
                            });
                            ui.add_space(4.0);
                            ScrollArea::vertical()
                                .id_salt("antigravity_chat_scroll")
                                .max_height(chat_h)
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    let inner_w = (chat_w - 14.0).max(160.0);
                                    ui.set_max_width(inner_w);
                                    for (idx, msg) in
                                        ai_assistant.chat_history.iter().enumerate()
                                    {
                                        let meta =
                                            self.turn_metadata.iter().find(|m| m.msg_index == idx);
                                        if msg.is_user {
                                            Frame::new()
                                                .fill(if theme.is_dark() {
                                                    Color32::from_rgb(33, 42, 60)
                                                } else {
                                                    theme.accent_blue.gamma_multiply(0.12)
                                                })
                                                .corner_radius(CornerRadius::same(10))
                                                .stroke(Stroke::new(
                                                    1.0,
                                                    theme.accent_blue.gamma_multiply(0.32),
                                                ))
                                                .inner_margin(Margin::symmetric(10, 6))
                                                .show(ui, |ui| {
                                                    ui.set_width(
                                                        (inner_w - 14.0).max(140.0),
                                                    );
                                                    if let Some(m) = meta {
                                                        if !m.context_chips.is_empty() {
                                                            ui.horizontal_wrapped(|ui| {
                                                                for chip in &m.context_chips {
                                                                    icons::pill_badge(
                                                                        ui,
                                                                        AppIcon::Target,
                                                                        chip.clone(),
                                                                        theme.accent_blue,
                                                                    );
                                                                }
                                                            });
                                                            ui.add_space(3.0);
                                                        }
                                                    }
                                                    ui.label(
                                                        RichText::new(&msg.message)
                                                            .size(11.2)
                                                            .color(theme.text_primary),
                                                    );
                                                });
                                        } else {
                                            Frame::new()
                                                .fill(theme.bg_card)
                                                .corner_radius(CornerRadius::same(10))
                                                .stroke(Stroke::new(1.0, theme.border_subtle))
                                                .inner_margin(Margin::symmetric(10, 6))
                                                .show(ui, |ui| {
                                                    ui.set_width(
                                                        (inner_w - 14.0).max(140.0),
                                                    );
                                                    if let Some(m) = meta {
                                                        if !m.trace_steps.is_empty() {
                                                            ui.horizontal_wrapped(|ui| {
                                                                for st in &m.trace_steps {
                                                                    icons::pill_badge(
                                                                        ui,
                                                                        AppIcon::CheckCircle,
                                                                        st.clone(),
                                                                        theme.accent_green,
                                                                    );
                                                                }
                                                            });
                                                            ui.add_space(3.0);
                                                        }
                                                    }
                                                    ui.label(
                                                        RichText::new(&msg.message)
                                                            .size(11.0)
                                                            .color(theme.text_primary),
                                                    );
                                                });
                                        }
                                        ui.add_space(4.0);
                                    }

                                    if self.gemini_inflight_req_id.is_some() {
                                        Frame::new()
                                            .fill(theme.accent_blue.gamma_multiply(0.12))
                                            .corner_radius(CornerRadius::same(9))
                                            .stroke(Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.4)))
                                            .inner_margin(Margin::symmetric(10, 6))
                                            .show(ui, |ui| {
                                                ui.set_width((inner_w - 14.0).max(140.0));
                                                icons::icon_label(
                                                    ui,
                                                    AppIcon::Sparkles,
                                                    12.0,
                                                    theme.accent_blue,
                                                    format!("{} is thinking...", self.agent_model.short_label()),
                                                    10.8,
                                                    theme.accent_blue,
                                                    true,
                                                );
                                            });
                                        ui.add_space(4.0);
                                    }

                                    let mut apply_id: Option<u64> = None;
                                    let mut reject_id: Option<u64> = None;
                                    for prop in &self.pending_proposals {
                                        let border_col = match prop.status {
                                            ProposalStatus::Pending => theme.accent_orange.gamma_multiply(0.65),
                                            ProposalStatus::Applied => theme.accent_green.gamma_multiply(0.6),
                                            ProposalStatus::Rejected => theme.border_subtle,
                                        };
                                        Frame::new()
                                            .fill(theme.bg_sidebar)
                                            .corner_radius(CornerRadius::same(10))
                                            .stroke(Stroke::new(1.0, border_col))
                                            .inner_margin(Margin::symmetric(10, 7))
                                            .show(ui, |ui| {
                                                ui.set_width((inner_w - 14.0).max(140.0));
                                                ui.horizontal_wrapped(|ui| {
                                                    icons::icon_label(
                                                        ui,
                                                        AppIcon::Wrench,
                                                        12.0,
                                                        theme.accent_orange,
                                                        &prop.title,
                                                        10.8,
                                                        theme.text_primary,
                                                        true,
                                                    );
                                                    match prop.status {
                                                        ProposalStatus::Pending => icons::pill_badge(
                                                            ui,
                                                            AppIcon::Clock,
                                                            "PLANNING REVIEW",
                                                            theme.accent_orange,
                                                        ),
                                                        ProposalStatus::Applied => icons::pill_badge(
                                                            ui,
                                                            AppIcon::CheckCircle,
                                                            "APPLIED",
                                                            theme.accent_green,
                                                        ),
                                                        ProposalStatus::Rejected => icons::pill_badge(
                                                            ui,
                                                            AppIcon::Close,
                                                            "REJECTED",
                                                            theme.text_muted,
                                                        ),
                                                    }
                                                });
                                                ui.label(
                                                    RichText::new(&prop.rationale)
                                                        .size(10.0)
                                                        .color(theme.text_secondary),
                                                );
                                                ui.label(
                                                    RichText::new(&prop.action_preview)
                                                        .size(9.8)
                                                        .monospace()
                                                        .color(theme.accent_blue),
                                                );
                                                if prop.status == ProposalStatus::Pending {
                                                    ui.add_space(4.0);
                                                    ui.horizontal_wrapped(|ui| {
                                                        if icons::icon_button(
                                                            ui,
                                                            AppIcon::CheckCircle,
                                                            "Apply Changes",
                                                            10.0,
                                                            Color32::WHITE,
                                                            theme.accent_green,
                                                            Stroke::NONE,
                                                            CornerRadius::same(7),
                                                            Vec2::new(0.0, 22.0),
                                                        )
                                                        .clicked()
                                                        {
                                                            apply_id = Some(prop.id);
                                                        }
                                                        if icons::icon_button(
                                                            ui,
                                                            AppIcon::Close,
                                                            "Reject",
                                                            10.0,
                                                            theme.text_secondary,
                                                            theme.bg_card,
                                                            Stroke::new(0.9, theme.border_subtle),
                                                            CornerRadius::same(7),
                                                            Vec2::new(0.0, 22.0),
                                                        )
                                                        .clicked()
                                                        {
                                                            reject_id = Some(prop.id);
                                                        }
                                                    });
                                                }
                                            });
                                        ui.add_space(4.0);
                                    }
                                    if let Some(pid) = apply_id {
                                        let sub = self.apply_proposal_by_id(
                                            pid,
                                            universal_canvas,
                                            build_view,
                                            ai_assistant,
                                        );
                                        actions.merge(sub);
                                    }
                                    if let Some(pid) = reject_id {
                                        self.reject_proposal_by_id(pid, ai_assistant);
                                    }
                                });
                        });
                        ui.add_space(6.0);
                    }

                    // C. MAIN EXPANDED DYNAMIC ISLAND COMMAND & FEATURE DECK
                    Self::antigravity_glass_frame(theme, 20, Margin::symmetric(12, 9)).show(
                        ui,
                        |ui| {
                            let cap_w = (dock_w - 28.0).max(200.0);
                            ui.set_width(cap_w);
                            ui.set_max_width(cap_w);

                            // Row 1: Vault (`+ Add Folder`), `+ Add Context`, AI Model/Mode, Chat & Close
                            ui.horizontal_wrapped(|ui| {
                                if vault_status.connected {
                                    if icons::icon_button(
                                        ui,
                                        AppIcon::Folder,
                                        if is_mobile {
                                            "Vault".to_string()
                                        } else {
                                            format!("Vault: {}", vault_status.vault_name)
                                        },
                                        10.0,
                                        theme.accent_green,
                                        theme.accent_green.gamma_multiply(0.14),
                                        Stroke::new(
                                            0.8,
                                            theme.accent_green.gamma_multiply(0.42),
                                        ),
                                        CornerRadius::same(8),
                                        Vec2::new(0.0, 22.0),
                                    )
                                    .on_hover_text("Browse Vault files & folder settings")
                                    .clicked()
                                    {
                                        self.vault_drawer_open = !self.vault_drawer_open;
                                        self.context_picker_open = false;
                                    }
                                }
                                if icons::icon_button(
                                    ui,
                                    AppIcon::Folder,
                                    if vault_status.connected {
                                        "Switch Vault"
                                    } else {
                                        "+ Add Folder (Vault)"
                                    },
                                    10.0,
                                    if vault_status.connected {
                                        theme.text_secondary
                                    } else {
                                        Color32::WHITE
                                    },
                                    if vault_status.connected {
                                        theme.bg_card
                                    } else {
                                        theme.accent_blue
                                    },
                                    Stroke::new(0.8, theme.border_subtle),
                                    CornerRadius::same(8),
                                    Vec2::new(0.0, 22.0),
                                )
                                .on_hover_text("Select local folder to mount as Engineering Vault")
                                .clicked()
                                {
                                    actions.add_vault_folder = true;
                                }

                                if icons::icon_button(
                                    ui,
                                    if self.context_picker_open {
                                        AppIcon::Close
                                    } else {
                                        AppIcon::Plus
                                    },
                                    "Add Context",
                                    10.0,
                                    if self.context_picker_open {
                                        Color32::WHITE
                                    } else {
                                        theme.text_primary
                                    },
                                    if self.context_picker_open {
                                        theme.accent_blue
                                    } else {
                                        theme.bg_card
                                    },
                                    Stroke::new(0.8, theme.border_subtle),
                                    CornerRadius::same(8),
                                    Vec2::new(0.0, 22.0),
                                )
                                .on_hover_text(
                                    "Attach element, Earth site, BIM, FEM, Vault file, or disk file (@)",
                                )
                                .clicked()
                                {
                                    self.context_picker_open = !self.context_picker_open;
                                }

                                if icons::icon_button(
                                    ui,
                                    AppIcon::Bot,
                                    self.agent_model.short_label(),
                                    10.0,
                                    theme.text_primary,
                                    theme.bg_card,
                                    Stroke::new(0.8, theme.border_subtle),
                                    CornerRadius::same(8),
                                    Vec2::new(0.0, 22.0),
                                )
                                .on_hover_text(format!(
                                    "AI Model: {} (click to cycle)",
                                    self.agent_model.label()
                                ))
                                .clicked()
                                {
                                    self.agent_model = self.agent_model.next();
                                }

                                let has_key = !self.gemini_api_key_draft.trim().is_empty();
                                let needs_key = matches!(
                                    self.agent_model,
                                    AgentModelChoice::Gemini25Flash | AgentModelChoice::GeminiLive
                                ) && !has_key;
                                if icons::icon_only_button(
                                    ui,
                                    AppIcon::Lock,
                                    if has_key {
                                        theme.accent_green
                                    } else if needs_key {
                                        theme.accent_orange
                                    } else {
                                        theme.text_secondary
                                    },
                                    if self.api_key_popover_open {
                                        theme.accent_blue.gamma_multiply(0.16)
                                    } else {
                                        theme.bg_card
                                    },
                                    Stroke::new(
                                        0.8,
                                        if needs_key {
                                            theme.accent_orange.gamma_multiply(0.55)
                                        } else {
                                            theme.border_subtle
                                        },
                                    ),
                                    CornerRadius::same(8),
                                    Vec2::splat(22.0),
                                    if has_key {
                                        "Gemini API Key Armed (click to edit)"
                                    } else {
                                        "Configure Google Gemini API Key"
                                    },
                                )
                                .clicked()
                                {
                                    self.api_key_popover_open = !self.api_key_popover_open;
                                    self.history_popover_open = false;
                                }

                                if icons::icon_button(
                                    ui,
                                    AppIcon::ChevronDown,
                                    self.agent_mode.label(),
                                    10.0,
                                    theme.text_secondary,
                                    theme.bg_card,
                                    Stroke::new(0.8, theme.border_subtle),
                                    CornerRadius::same(8),
                                    Vec2::new(0.0, 22.0),
                                )
                                .on_hover_text(self.agent_mode.description())
                                .clicked()
                                {
                                    self.agent_mode = self.agent_mode.next();
                                }

                                if icons::icon_only_button(
                                    ui,
                                    AppIcon::CommentNote,
                                    if self.command_bar_expanded {
                                        theme.accent_blue
                                    } else {
                                        theme.text_secondary
                                    },
                                    theme.bg_card,
                                    Stroke::new(0.8, theme.border_subtle),
                                    CornerRadius::same(8),
                                    Vec2::splat(22.0),
                                    "Toggle AI Chat Drawer",
                                )
                                .clicked()
                                {
                                    self.command_bar_expanded = !self.command_bar_expanded;
                                }

                                if icons::icon_only_button(
                                    ui,
                                    AppIcon::Clock,
                                    if self.history_popover_open {
                                        theme.accent_blue
                                    } else {
                                        theme.text_secondary
                                    },
                                    theme.bg_card,
                                    Stroke::new(0.8, theme.border_subtle),
                                    CornerRadius::same(8),
                                    Vec2::splat(22.0),
                                    "Conversation History",
                                )
                                .clicked()
                                {
                                    self.history_popover_open = !self.history_popover_open;
                                }

                                if is_mobile || self.command_bar_expanded || self.context_picker_open || self.vault_drawer_open {
                                    if icons::icon_only_button(
                                        ui,
                                        AppIcon::Close,
                                        theme.text_secondary,
                                        Color32::TRANSPARENT,
                                        Stroke::NONE,
                                        CornerRadius::same(8),
                                        Vec2::splat(22.0),
                                        "Collapse Dynamic Island",
                                    )
                                    .clicked()
                                    {
                                        self.collapse_dynamic_island();
                                    }
                                }
                            });

                            ui.add_space(5.0);

                            // Row 2: Manual Drawing & Earth Sculpting Commands
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new("Draw:")
                                        .size(9.8)
                                        .strong()
                                        .color(theme.text_muted),
                                );
                                for tool in SpatialManualTool::ALL {
                                    let active = tool.is_active(
                                        self.active_canvas_mode,
                                        universal_canvas,
                                        build_view,
                                    );
                                    if icons::icon_button(
                                        ui,
                                        tool.icon(),
                                        tool.label(),
                                        9.8,
                                        if active {
                                            Color32::WHITE
                                        } else {
                                            theme.text_secondary
                                        },
                                        if active {
                                            if *tool == SpatialManualTool::OrbitPan {
                                                theme.accent_blue
                                            } else {
                                                theme.accent_orange
                                            }
                                        } else {
                                            theme.bg_card
                                        },
                                        Stroke::new(0.8, theme.border_subtle),
                                        CornerRadius::same(8),
                                        Vec2::new(0.0, 21.0),
                                    )
                                    .on_hover_text(tool.tooltip())
                                    .clicked()
                                    {
                                        let sub =
                                            tool.apply(self, universal_canvas, build_view);
                                        actions.merge(sub);
                                        // Fold popovers so moving cursor to Earth lets user draw right away
                                        self.collapse_dynamic_island();
                                    }
                                }
                            });

                            // Row 3: Attached `@Context` Chips (if any)
                            if !self.attached_contexts.is_empty() {
                                ui.add_space(5.0);
                                let mut detach_key: Option<String> = None;
                                ui.horizontal_wrapped(|ui| {
                                    for att in &self.attached_contexts {
                                        let lbl = att.chip_label(
                                            &build_view.bim_bus.project,
                                            self.selected_target,
                                        );
                                        let key = att.dedup_key();
                                        if icons::icon_button(
                                            ui,
                                            att.icon(),
                                            format!("{lbl} ×"),
                                            9.8,
                                            theme.accent_blue,
                                            theme.accent_blue.gamma_multiply(0.14),
                                            Stroke::new(
                                                0.8,
                                                theme.accent_blue.gamma_multiply(0.45),
                                            ),
                                            CornerRadius::same(10),
                                            Vec2::new(0.0, 20.0),
                                        )
                                        .on_hover_text("Click to detach context")
                                        .clicked()
                                        {
                                            detach_key = Some(key);
                                        }
                                    }
                                });
                                if let Some(k) = detach_key {
                                    self.detach_context(&k);
                                }
                            }

                            ui.add_space(5.0);

                            // Row 4: Prompt & `/` Slash Command Composer
                            Frame::new()
                                .fill(theme.bg_card)
                                .corner_radius(CornerRadius::same(12))
                                .stroke(Stroke::new(
                                    1.0,
                                    if self.composer_had_focus {
                                        theme.accent_blue.gamma_multiply(0.68)
                                    } else {
                                        theme.border_subtle
                                    },
                                ))
                                .inner_margin(Margin::symmetric(10, 6))
                                .show(ui, |ui| {
                                    let row_w = (cap_w - 22.0).max(160.0);
                                    ui.set_width(row_w);
                                    ui.horizontal(|ui| {
                                        if let Some(mention) =
                                            self.active_context_mention(&build_view.bim_bus.project)
                                        {
                                            if !is_mobile {
                                                if icons::icon_button(
                                                    ui,
                                                    AppIcon::Target,
                                                    mention,
                                                    9.5,
                                                    theme.accent_blue,
                                                    theme.accent_blue.gamma_multiply(0.14),
                                                    Stroke::NONE,
                                                    CornerRadius::same(8),
                                                    Vec2::new(0.0, 20.0),
                                                )
                                                .on_hover_text("Attach selected canvas item as @Context")
                                                .clicked()
                                                {
                                                    self.attach_context(
                                                        AiContextAttachment::SelectedElement,
                                                    );
                                                }
                                            }
                                        }

                                        let bare_enter = self.composer_had_focus
                                            && !slash_mode
                                            && ui.ctx().input(|i| !i.modifiers.shift)
                                            && ui.ctx().input_mut(|i| {
                                                i.consume_key(
                                                    egui::Modifiers::NONE,
                                                    egui::Key::Enter,
                                                )
                                            });

                                        let trailing_reserve = 66.0;
                                        let input_w =
                                            (ui.available_width() - trailing_reserve).max(60.0);
                                        let edit_resp = ui.add(
                                            egui::TextEdit::singleline(&mut self.command_input)
                                                .desired_width(input_w)
                                                .frame(Frame::NONE)
                                                .hint_text(
                                                    "Ask EigenAI, @ context, or / commands...",
                                                ),
                                        );
                                        self.composer_had_focus = edit_resp.has_focus();
                                        if std::mem::take(&mut self.request_focus_command) {
                                            edit_resp.request_focus();
                                        }

                                        let cur_len = self.command_input.len();
                                        if cur_len > self.last_command_len
                                            && self.command_input.ends_with('@')
                                        {
                                            self.context_picker_open = true;
                                        }
                                        self.last_command_len = cur_len;

                                        let enter_singleline = edit_resp.lost_focus()
                                            && ui.input(|i| i.key_pressed(egui::Key::Enter));

                                        if ui
                                            .add_sized(
                                                Vec2::new(24.0, 23.0),
                                                egui::Button::new(
                                                    RichText::new("/")
                                                        .size(11.5)
                                                        .strong()
                                                        .color(theme.text_secondary),
                                                )
                                                .fill(theme.bg_sidebar)
                                                .stroke(Stroke::new(0.8, theme.border_subtle))
                                                .corner_radius(CornerRadius::same(7)),
                                            )
                                            .on_hover_text("Browse slash commands (/earth, /3d, /vault...)")
                                            .clicked()
                                        {
                                            if self.command_input.trim_start().starts_with('/') {
                                                self.command_input.clear();
                                            } else {
                                                self.command_input = "/".to_string();
                                                self.context_picker_open = false;
                                                self.request_focus_command = true;
                                            }
                                        }

                                        let has_text = !self.command_input.trim().is_empty();
                                        let send_clicked = icons::icon_only_button(
                                            ui,
                                            AppIcon::ArrowRight,
                                            Color32::WHITE,
                                            if has_text {
                                                theme.accent_blue
                                            } else {
                                                theme.accent_blue.gamma_multiply(0.45)
                                            },
                                            Stroke::NONE,
                                            CornerRadius::same(9),
                                            Vec2::splat(24.0),
                                            "Execute Command or AI Prompt",
                                        )
                                        .clicked();

                                        if (bare_enter || enter_singleline || send_clicked)
                                            && has_text
                                        {
                                            let submitted = std::mem::take(&mut self.command_input);
                                            self.last_command_len = 0;
                                            let sub = self.execute_spatial_input(
                                                &submitted,
                                                universal_canvas,
                                                build_view,
                                                ai_assistant,
                                                members,
                                                projects,
                                                db_components,
                                            );
                                            actions.merge(sub);
                                        }
                                    });
                                });
                        },
                    );
                });
            });

        self.dynamic_island_rect = dock_resp.response.rect;
        self.hud_rects.push(dock_resp.response.rect);

        actions
    }
}

impl SpatialHudActions {
    pub fn merge(&mut self, other: Self) {
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
        self.add_vault_folder |= other.add_vault_folder;
        if other.connect_indexeddb_vault.is_some() {
            self.connect_indexeddb_vault = other.connect_indexeddb_vault;
        }
        if other.connect_vault_path.is_some() {
            self.connect_vault_path = other.connect_vault_path;
        }
        if other.create_vault_path_and_name.is_some() {
            self.create_vault_path_and_name = other.create_vault_path_and_name;
        }
        self.disconnect_vault |= other.disconnect_vault;
        self.save_active_to_vault |= other.save_active_to_vault;
        self.refresh_vault |= other.refresh_vault;
        if other.open_vault_project.is_some() {
            self.open_vault_project = other.open_vault_project;
        }
        self.pick_local_context_file |= other.pick_local_context_file;
        if other.anchor_and_build_site.is_some() {
            self.anchor_and_build_site = other.anchor_and_build_site;
        }
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

    #[test]
    fn test_phase47_antigravity_add_context_and_vault_attach() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus = crate::core::BimCommandBus::from_project(
            BimProject::sample_bimcraft_building(),
        );
        let col_id = build.bim_bus.project.columns[0].id;
        spatial.selected_target = Some(SmartSelectionTarget::Column(col_id));

        let mut ai = AiAssistantView::default();
        let members = vec![StructuralMember::default()];
        let projects = Vec::new();
        let mut db_components = Vec::new();

        spatial.attach_context(AiContextAttachment::SelectedElement);
        spatial.attach_context(AiContextAttachment::SelectedElement);
        spatial.attach_context(AiContextAttachment::FemRedundancy);
        spatial.attach_local_file_context(
            "spec_notes.md".to_string(),
            "# ACI 318 Seismic Column Note\nUse confinement hoops @ 100mm.".into(),
        );
        assert_eq!(spatial.attached_contexts.len(), 3);

        let (chips, traces, markdown) = spatial.build_enriched_context_payload(
            &build.bim_bus.project,
            &members,
            &canvas,
        );
        assert_eq!(chips.len(), 3);
        assert!(traces.iter().any(|t| t.contains("Selected Element")));
        assert!(markdown.contains("spec_notes.md"));
        assert!(markdown.contains("FEM & ARI"));

        let _ = spatial.execute_spatial_input(
            "audit attached column and spec",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(spatial.turn_metadata.len() >= 2);
        assert!(ai.chat_history.last().unwrap().message.contains("Context Applied"));

        spatial.detach_context("domain:fem");
        assert_eq!(spatial.attached_contexts.len(), 2);
    }

    #[test]
    fn test_phase48_dynamic_island_hover_expand_and_responsive_zero_overflow() {
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        for win_w in [1280.0_f32, 840.0, 700.0, 390.0] {
            let mut spatial = SpatialWorkspaceState::default();
            let mut canvas = UniversalCanvasState::default();
            canvas.is_frozen = false;
            let mut build = BuildView::default();
            build.bim_bus = crate::core::BimCommandBus::from_project(
                BimProject::sample_bimcraft_building(),
            );
            let mut ai = AiAssistantView::default();
            let members = vec![StructuralMember::default()];
            let projects = Vec::new();
            let mut db_components = Vec::new();
            let ctx = egui::Context::default();

            let mut run_frame = |pointer: Option<egui::Pos2>,
                             spatial: &mut SpatialWorkspaceState,
                             canvas: &mut UniversalCanvasState,
                             build: &mut BuildView,
                             ai: &mut AiAssistantView| {
                let mut input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 840.0))),
                    ..Default::default()
                };
                if let Some(pos) = pointer {
                    input.events.push(egui::Event::PointerMoved(pos));
                }
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
                    let _ = spatial.render_hud_overlays_with_vault(
                        ui.ctx(),
                        &theme,
                        canvas,
                        build,
                        ai,
                        &members,
                        &projects,
                        &mut db_components,
                        &[],
                    );
                });
                out.textures_delta.clear();
            };

            // 1. Pointer away on canvas -> compact idle Dynamic Island pill
            run_frame(
                Some(egui::Pos2::new(100.0, 100.0)),
                &mut spatial,
                &mut canvas,
                &mut build,
                &mut ai,
            );
            assert!(!spatial.island_expanded_prev);
            let idle_rect = spatial.dynamic_island_rect;
            assert!(idle_rect.is_positive());

            // 2. Hover onto Dynamic Island -> automatically expands to show Vault, Context, AI & Draw controls
            run_frame(
                Some(idle_rect.center()),
                &mut spatial,
                &mut canvas,
                &mut build,
                &mut ai,
            );
            assert!(
                spatial.island_expanded_prev,
                "Dynamic Island should expand immediately on cursor hover at width {win_w}"
            );
            let hovered_rect = spatial.dynamic_island_rect;
            assert!(
                hovered_rect.height() >= idle_rect.height(),
                "Expanded Dynamic Island must grow upward cleanly"
            );

            // 3. Pin chat + context + Vault drawers open across multiple frames until full animation width and confirm zero overflow
            spatial.command_bar_expanded = true;
            spatial.context_picker_open = true;
            spatial.vault_drawer_open = true;
            spatial.attach_context(AiContextAttachment::BimGeometry);
            for _ in 0..8 {
                run_frame(
                    Some(hovered_rect.center()),
                    &mut spatial,
                    &mut canvas,
                    &mut build,
                    &mut ai,
                );
            }
            for r in &spatial.hud_rects {
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Dynamic Island HUD rect {r:?} overflowed viewport width {win_w}"
                );
            }

            // 4. Unpin and move cursor back to canvas -> Dynamic Island folds back to compact pill
            spatial.collapse_dynamic_island();
            run_frame(
                Some(egui::Pos2::new(60.0, 60.0)),
                &mut spatial,
                &mut canvas,
                &mut build,
                &mut ai,
            );
            assert!(!spatial.island_expanded_prev);
        }
    }

    #[test]
    fn test_phase48_manual_draw_commands_from_dynamic_island() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();

        let _ = SpatialManualTool::Column.apply(&mut spatial, &mut canvas, &mut build);
        assert_eq!(spatial.active_canvas_mode, SpatialCanvasMode::Bim3D);
        assert_eq!(build.ground_author_tool, Ground3DAuthorTool::Column);
        assert!(SpatialManualTool::Column.is_active(
            spatial.active_canvas_mode,
            &canvas,
            &build
        ));

        let _ = SpatialManualTool::Line2D.apply(&mut spatial, &mut canvas, &mut build);
        assert_eq!(spatial.active_canvas_mode, SpatialCanvasMode::EarthPlan2D);
        assert_eq!(canvas.active_tool, UniversalCanvasTool::Line);
        assert!(!canvas.globe_mode);

        let _ = SpatialManualTool::Measure.apply(&mut spatial, &mut canvas, &mut build);
        assert!(canvas.measure_active);

        let _ = SpatialManualTool::Terraform.apply(&mut spatial, &mut canvas, &mut build);
        assert_eq!(spatial.active_canvas_mode, SpatialCanvasMode::EarthGlobe);
        assert!(canvas.globe_mode);
        assert!(canvas.active_tool.is_earth_tool());

        let _ = SpatialManualTool::OrbitPan.apply(&mut spatial, &mut canvas, &mut build);
        assert!(SpatialManualTool::OrbitPan.is_active(
            spatial.active_canvas_mode,
            &canvas,
            &build
        ));
    }

    #[test]
    fn test_phase47_antigravity_mode_model_and_new_chat_history() {
        let mut spatial = SpatialWorkspaceState::default();
        assert_eq!(spatial.agent_mode, AgentExecutionMode::Planning);
        assert_eq!(spatial.agent_mode.next(), AgentExecutionMode::Fast);
        assert_eq!(spatial.agent_model.short_label(), "EigenAI Pro");
        assert_eq!(spatial.agent_model.next(), AgentModelChoice::Gemini25Flash);

        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members = vec![StructuralMember::default()];
        let projects = Vec::new();
        let mut db_components = Vec::new();

        let add_folder_act = spatial.execute_spatial_input(
            "/add-folder",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(add_folder_act.add_vault_folder);

        let ctx_act = spatial.execute_spatial_input(
            "/context",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(spatial.context_picker_open);
        assert!(!ctx_act.add_vault_folder);

        let _ = spatial.execute_spatial_input(
            "check redundancy",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(ai.chat_history.len() > 1);
        spatial.start_new_conversation(&mut ai);
        assert_eq!(spatial.conversation_archives.len(), 1);
        assert_eq!(ai.chat_history.len(), 1);
    }

    #[test]
    fn test_phase49_planning_proposal_apply_and_reject() {
        let mut spatial = SpatialWorkspaceState::default();
        assert_eq!(spatial.agent_mode, AgentExecutionMode::Planning);
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        // In Planning mode, "build portal" queues a proposal card instead of mutating BIM immediately
        let _ = spatial.execute_spatial_input(
            "build portal 3x2 @6m 2floors",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(spatial.pending_proposals.len(), 1);
        assert_eq!(spatial.pending_proposals[0].status, ProposalStatus::Pending);
        assert!(build.bim_bus.project.is_empty());

        let prop_id = spatial.pending_proposals[0].id;
        let apply_act = spatial.apply_proposal_by_id(prop_id, &mut canvas, &mut build, &mut ai);
        assert_eq!(spatial.pending_proposals[0].status, ProposalStatus::Applied);
        assert!(!build.bim_bus.project.is_empty());
        assert!(apply_act.sync_fem_and_earth);
        assert!(apply_act.fit_camera);

        // Second proposal rejected leaves BIM count unchanged
        let before_count = build.bim_bus.project.element_count();
        let id2 = spatial.queue_planning_proposal(
            "Add Repair",
            "Test rationale",
            "BimCommand::Repair",
            vec![PlannedProposalAction::AutoRepair],
        );
        spatial.reject_proposal_by_id(id2, &mut ai);
        assert_eq!(spatial.pending_proposals[1].status, ProposalStatus::Rejected);
        assert_eq!(build.bim_bus.project.element_count(), before_count);
    }

    #[test]
    fn test_phase49_gemini_response_and_directive_parsing() {
        let raw = "Recommended 2-bay ductile moment frame aligned to seismic zone. [[FLY:-6.2088,106.8456,15.5]] [[BIM:portal 2x2 @6m 2floors]] [[ARI_AUDIT]]";
        let (clean, dirs) = parse_gemini_action_directives(raw);
        assert_eq!(
            clean,
            "Recommended 2-bay ductile moment frame aligned to seismic zone."
        );
        assert_eq!(dirs.len(), 3);
        assert_eq!(
            dirs[0],
            GeminiDirective::Fly {
                lat: -6.2088,
                lon: 106.8456,
                zoom: 15.5
            }
        );
        assert_eq!(
            dirs[1],
            GeminiDirective::Bim("portal 2x2 @6m 2floors".to_string())
        );
        assert_eq!(dirs[2], GeminiDirective::AriAudit);

        let api_json = r#"{
            "candidates": [{
                "content": {
                    "parts": [{"text": "Column C1 utilization is 48%."}]
                }
            }]
        }"#;
        let parsed = crate::core::web_bridge::parse_gemini_generate_content_response(api_json)
            .expect("Gemini candidate should parse");
        assert!(parsed.contains("48%"));
    }

    #[test]
    fn test_phase49_obsidian_vault_sessions_roundtrip_and_earth_build_here() {
        let temp_vault = std::env::temp_dir().join(format!(
            "eigengraph_phase49_vault_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_vault);
        let mut spatial = SpatialWorkspaceState::default();
        spatial.conversation_title = "Jakarta Site Audit".to_string();
        let mut ai = AiAssistantView::default();
        ai.chat_history.push(crate::models::ChatMessage {
            is_user: true,
            message: "Audit Jakarta seismic columns".to_string(),
            time: "Now".to_string(),
        });
        let json = spatial.export_sessions_json(&ai);
        let saved_path = crate::core::InternalVault::save_ai_sessions_json_at(&temp_vault, &json)
            .expect("AI sessions JSON should save to AI_Context/");
        assert!(saved_path.ends_with("AI_Context/dynamic_island_sessions.json"));

        let loaded_json =
            crate::core::InternalVault::load_ai_sessions_json_at(&temp_vault).expect("Should reload");
        let mut spatial_restored = SpatialWorkspaceState::default();
        let mut ai_restored = AiAssistantView::default();
        assert!(spatial_restored.restore_sessions_json(&loaded_json, &mut ai_restored));
        assert_eq!(spatial_restored.conversation_title, "Jakarta Site Audit");
        assert_eq!(ai_restored.chat_history.len(), ai.chat_history.len());
        let _ = std::fs::remove_dir_all(&temp_vault);
    }
}
