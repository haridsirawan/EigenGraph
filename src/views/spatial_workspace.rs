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

/// Domain provenance used to maintain chronological LIFO Undo/Redo across 3D BIM & 2D Canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialUndoDomain {
    Bim3D,
    Canvas2D,
}

/// Element or Earth location targeted by a spatial click on the full canvas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SmartSelectionTarget {
    Column(usize),
    Beam(usize),
    Wall(usize),
    Footing(usize),
    Slab(usize),
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
    Gemini25Pro,
}

impl AgentModelChoice {
    pub fn label(self) -> &'static str {
        match self {
            Self::EigenAiProFem => "EigenAI Pro (FEM+ARI)",
            Self::Gemini25Flash => "Gemini 2.5 Flash",
            Self::Gemini25Pro => "Gemini 2.5 Pro",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::EigenAiProFem => "EigenAI Pro",
            Self::Gemini25Flash => "Gemini 2.5 Flash",
            Self::Gemini25Pro => "Gemini 2.5 Pro",
        }
    }

    pub fn gemini_model_id(self) -> &'static str {
        match self {
            Self::Gemini25Flash => "gemini-2.5-flash",
            Self::Gemini25Pro => "gemini-2.5-pro",
            Self::EigenAiProFem => "",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::EigenAiProFem => Self::Gemini25Flash,
            Self::Gemini25Flash => Self::Gemini25Pro,
            Self::Gemini25Pro => Self::EigenAiProFem,
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
                Some(SmartSelectionTarget::Slab(id)) => project
                    .slabs
                    .iter()
                    .find(|s| s.id == id)
                    .map(|s| format!("@{}", s.name))
                    .unwrap_or_else(|| format!("@Slab#{id}")),
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
    ResizeColumn {
        column_id: usize,
        width_mm: f64,
        depth_mm: f64,
    },
    ResizeBeam {
        run_id: usize,
        width_mm: f64,
        depth_mm: f64,
    },
    AddColumnAt {
        x_mm: f64,
        y_mm: f64,
        width_mm: f64,
        depth_mm: f64,
    },
    ApplySiteSeismic,
    OpenSheet(NavigationPillar),
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

/// Structured bracket directive optionally emitted by Gemini API responses
/// (`[[FLY:...]]`, `[[BIM:...]]`, `[[REPAIR]]`, `[[ARI_AUDIT]]`, `[[RESIZE_COL:...]]`, `[[RESIZE_BEAM:...]]`, `[[ADD_COL:...]]`, `[[APPLY_SITE_SEISMIC]]`, `[[OPEN_SHEET:...]]`).
#[derive(Debug, Clone, PartialEq)]
pub enum GeminiDirective {
    Fly { lat: f64, lon: f64, zoom: f64 },
    Bim(String),
    Repair,
    AriAudit,
    ResizeColumn { id: usize, w_mm: f64, d_mm: f64 },
    ResizeBeam { id: usize, b_mm: f64, h_mm: f64 },
    AddColumn { x_mm: f64, y_mm: f64, w_mm: f64, d_mm: f64 },
    ApplySiteSeismic,
    OpenSheet(NavigationPillar),
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
            } else if upper.starts_with("RESIZE_COL:") {
                let nums: Vec<f64> = tag[11..]
                    .split(',')
                    .filter_map(|s| s.trim().parse::<f64>().ok())
                    .collect();
                if nums.len() >= 3 {
                    directives.push(GeminiDirective::ResizeColumn {
                        id: nums[0].max(1.0) as usize,
                        w_mm: nums[1].clamp(150.0, 2500.0),
                        d_mm: nums[2].clamp(150.0, 2500.0),
                    });
                }
            } else if upper.starts_with("RESIZE_BEAM:") {
                let nums: Vec<f64> = tag[12..]
                    .split(',')
                    .filter_map(|s| s.trim().parse::<f64>().ok())
                    .collect();
                if nums.len() >= 3 {
                    directives.push(GeminiDirective::ResizeBeam {
                        id: nums[0].max(1.0) as usize,
                        b_mm: nums[1].clamp(120.0, 1800.0),
                        h_mm: nums[2].clamp(150.0, 3000.0),
                    });
                }
            } else if upper.starts_with("ADD_COL:") {
                let nums: Vec<f64> = tag[8..]
                    .split(',')
                    .filter_map(|s| s.trim().parse::<f64>().ok())
                    .collect();
                if nums.len() >= 2 {
                    let w_mm = nums.get(2).copied().unwrap_or(400.0).clamp(150.0, 2000.0);
                    let d_mm = nums.get(3).copied().unwrap_or(w_mm).clamp(150.0, 2000.0);
                    directives.push(GeminiDirective::AddColumn {
                        x_mm: nums[0],
                        y_mm: nums[1],
                        w_mm,
                        d_mm,
                    });
                }
            } else if upper == "APPLY_SITE_SEISMIC" || upper == "SEISMIC_SYNC" {
                directives.push(GeminiDirective::ApplySiteSeismic);
            } else if upper == "REPAIR" {
                directives.push(GeminiDirective::Repair);
            } else if upper == "ARI_AUDIT" || upper == "ARI" {
                directives.push(GeminiDirective::AriAudit);
            } else if upper.starts_with("OPEN_SHEET:") {
                let sheet_key = upper[11..].trim();
                let pillar = if sheet_key.contains("VAULT") || sheet_key.contains("LIBRARY") {
                    Some(NavigationPillar::MyLibrary)
                } else if sheet_key.contains("AGENT") || sheet_key.contains("FIRM") {
                    Some(NavigationPillar::AiAgent)
                } else if sheet_key.contains("PLAN") || sheet_key.contains("1D") || sheet_key.contains("GIS") {
                    Some(NavigationPillar::ResearchConcept)
                } else if sheet_key.contains("DRAFT") || sheet_key.contains("2D") || sheet_key.contains("DED") {
                    Some(NavigationPillar::ParametricDED)
                } else if sheet_key.contains("GEOM") || sheet_key.contains("BIM") || sheet_key.contains("3D") {
                    Some(NavigationPillar::GeometryAssembly)
                } else if sheet_key.contains("ANALY") || sheet_key.contains("FEM") || sheet_key.contains("SIM") {
                    Some(NavigationPillar::AnalysisSimulation)
                } else if sheet_key.contains("SCHED") || sheet_key.contains("4D") {
                    Some(NavigationPillar::TimeScheduling)
                } else if sheet_key.contains("BUDGET") || sheet_key.contains("COST") || sheet_key.contains("5D") {
                    Some(NavigationPillar::CostBudgeting)
                } else if sheet_key.contains("SUST") || sheet_key.contains("CARBON") || sheet_key.contains("6D") {
                    Some(NavigationPillar::SustainabilityEnergy)
                } else if sheet_key.contains("FACIL") || sheet_key.contains("TWIN") || sheet_key.contains("7D") {
                    Some(NavigationPillar::FacilityManagement)
                } else if sheet_key.contains("SAFE") || sheet_key.contains("RISK") || sheet_key.contains("8D") {
                    Some(NavigationPillar::SafetyHealth)
                } else if sheet_key.contains("CONST") || sheet_key.contains("LEAN") || sheet_key.contains("9D") {
                    Some(NavigationPillar::LeanConstruction)
                } else if sheet_key.contains("AUTO") || sheet_key.contains("DFMA") || sheet_key.contains("10D") {
                    Some(NavigationPillar::IndustrializedAutomation)
                } else if sheet_key.contains("PRED") || sheet_key.contains("AGING") {
                    Some(NavigationPillar::Prediction)
                } else if sheet_key.contains("REP") || sheet_key.contains("PUB") {
                    Some(NavigationPillar::ReportPublishing)
                } else {
                    None
                };
                if let Some(p) = pillar {
                    directives.push(GeminiDirective::OpenSheet(p));
                }
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
    pub apply_site_seismic: bool,
    pub undo: bool,
    pub redo: bool,
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

    // Phase 49 & 50: Live Gemini API, Planning Proposals, Obsidian Notes & Precision Placement
    pub gemini_api_key_draft: String,
    pub remember_gemini_key: bool,
    pub api_key_popover_open: bool,
    pub gemini_inflight_req_id: Option<u64>,
    pub pending_proposals: Vec<PlannedAgentProposal>,
    next_proposal_id: u64,
    pub sessions_dirty: bool,
    pub vault_sessions_hydrated: bool,
    pub vault_note_composer_open: bool,
    pub vault_note_title_input: String,
    pub vault_note_body_input: String,
    pub previewed_vault_note: Option<(String, String)>,
    pub preview_note_editing: bool,
    pub manual_place_x_mm: f64,
    pub manual_place_y_mm: f64,
    pub manual_polar_len_m: f64,
    pub manual_polar_deg: f64,

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
    // Phase 52: Mobile/touch island pin toggle, precision 2D draw drafts, & 2D entity style editor
    pub mobile_deck_pinned: bool,
    pub draw_2d_len_m: f64,
    pub draw_2d_deg: f64,
    pub draw_2d_rect_w_m: f64,
    pub draw_2d_rect_h_m: f64,
    pub draw_2d_radius_m: f64,
    pub sketch_edit_color: [u8; 3],
    pub sketch_edit_width: f32,
    // Phase 53: Chronological mixed Undo/Redo, Story/Slab authoring, & Obsidian Vault organizer
    pub undo_timeline: Vec<SpatialUndoDomain>,
    pub redo_timeline: Vec<SpatialUndoDomain>,
    last_bim_undo_len: usize,
    last_uc_undo_len: usize,
    pub active_story_idx: usize,
    pub vault_filter_query: String,
    pub vault_new_folder_name: String,
    pub vault_rename_target: Option<(String, String)>,
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
            remember_gemini_key: false,
            api_key_popover_open: false,
            gemini_inflight_req_id: None,
            pending_proposals: Vec::new(),
            next_proposal_id: 1,
            sessions_dirty: false,
            vault_sessions_hydrated: false,
            vault_note_composer_open: false,
            vault_note_title_input: String::new(),
            vault_note_body_input: String::new(),
            previewed_vault_note: None,
            preview_note_editing: false,
            manual_place_x_mm: 0.0,
            manual_place_y_mm: 0.0,
            manual_polar_len_m: 6.0,
            manual_polar_deg: 0.0,
            last_loaded_target: None,
            edit_width_mm: 400.0,
            edit_depth_mm: 400.0,
            edit_length_mm: 1400.0,
            edit_thickness_mm: 450.0,
            edit_rot_deg: 0.0,
            edit_is_structural: true,
            edit_profile_name: "RC 400x400".to_string(),
            active_4d_day: 90,
            mobile_deck_pinned: false,
            draw_2d_len_m: 6.0,
            draw_2d_deg: 0.0,
            draw_2d_rect_w_m: 8.0,
            draw_2d_rect_h_m: 5.0,
            draw_2d_radius_m: 3.0,
            sketch_edit_color: [59, 130, 246],
            sketch_edit_width: 2.2,
            undo_timeline: Vec::new(),
            redo_timeline: Vec::new(),
            last_bim_undo_len: 0,
            last_uc_undo_len: 0,
            active_story_idx: 0,
            vault_filter_query: String::new(),
            vault_new_folder_name: String::new(),
            vault_rename_target: None,
        }
    }
}

impl SpatialWorkspaceState {
    /// Returns true when the Floating Dynamic Island is hovered, has an active composer/popover,
    /// or displays the active Earth Site Location card.
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
            || self.mobile_deck_pinned
            || matches!(self.selected_target, Some(SmartSelectionTarget::EarthSite { .. }))
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
        self.mobile_deck_pinned = false;
        self.composer_had_focus = false;
        if matches!(self.selected_target, Some(SmartSelectionTarget::EarthSite { .. })) {
            self.selected_target = None;
        }
    }

    /// Records new undo push events from either BIM or 2D Canvas in strict chronological LIFO order.
    pub fn observe_undo_stacks(&mut self, bim_undo_len: usize, uc_undo_len: usize) {
        if bim_undo_len > self.last_bim_undo_len {
            let delta = bim_undo_len - self.last_bim_undo_len;
            for _ in 0..delta {
                self.undo_timeline.push(SpatialUndoDomain::Bim3D);
            }
            self.redo_timeline.clear();
        }
        if uc_undo_len > self.last_uc_undo_len {
            let delta = uc_undo_len - self.last_uc_undo_len;
            for _ in 0..delta {
                self.undo_timeline.push(SpatialUndoDomain::Canvas2D);
            }
            self.redo_timeline.clear();
        }
        self.last_bim_undo_len = bim_undo_len;
        self.last_uc_undo_len = uc_undo_len;
    }

    /// Synchronizes undo baseline counters after commit/undo/redo so no spurious growth is logged.
    pub fn sync_undo_baseline(&mut self, bim_undo_len: usize, uc_undo_len: usize) {
        self.last_bim_undo_len = bim_undo_len;
        self.last_uc_undo_len = uc_undo_len;
    }

    /// Resolves the most recent domain to undo chronologically (LIFO) across BIM & 2D Canvas.
    pub fn pop_undo_domain(
        &mut self,
        bim_undo_len: usize,
        uc_undo_len: usize,
    ) -> Option<SpatialUndoDomain> {
        while let Some(dom) = self.undo_timeline.pop() {
            match dom {
                SpatialUndoDomain::Bim3D if bim_undo_len > 0 => {
                    self.redo_timeline.push(SpatialUndoDomain::Bim3D);
                    return Some(SpatialUndoDomain::Bim3D);
                }
                SpatialUndoDomain::Canvas2D if uc_undo_len > 0 => {
                    self.redo_timeline.push(SpatialUndoDomain::Canvas2D);
                    return Some(SpatialUndoDomain::Canvas2D);
                }
                _ => {}
            }
        }
        if bim_undo_len > 0 {
            self.redo_timeline.push(SpatialUndoDomain::Bim3D);
            Some(SpatialUndoDomain::Bim3D)
        } else if uc_undo_len > 0 {
            self.redo_timeline.push(SpatialUndoDomain::Canvas2D);
            Some(SpatialUndoDomain::Canvas2D)
        } else {
            None
        }
    }

    /// Resolves the next domain to redo chronologically (LIFO) across BIM & 2D Canvas.
    pub fn pop_redo_domain(
        &mut self,
        bim_redo_len: usize,
        uc_redo_len: usize,
    ) -> Option<SpatialUndoDomain> {
        while let Some(dom) = self.redo_timeline.pop() {
            match dom {
                SpatialUndoDomain::Bim3D if bim_redo_len > 0 => {
                    self.undo_timeline.push(SpatialUndoDomain::Bim3D);
                    return Some(SpatialUndoDomain::Bim3D);
                }
                SpatialUndoDomain::Canvas2D if uc_redo_len > 0 => {
                    self.undo_timeline.push(SpatialUndoDomain::Canvas2D);
                    return Some(SpatialUndoDomain::Canvas2D);
                }
                _ => {}
            }
        }
        if bim_redo_len > 0 {
            self.undo_timeline.push(SpatialUndoDomain::Bim3D);
            Some(SpatialUndoDomain::Bim3D)
        } else if uc_redo_len > 0 {
            self.undo_timeline.push(SpatialUndoDomain::Canvas2D);
            Some(SpatialUndoDomain::Canvas2D)
        } else {
            None
        }
    }

    /// Resolves sorted active story `(base_level_id, top_level_id, label, count)` for manual BIM placement.
    pub fn active_base_and_top_levels(&self, project: &BimProject) -> (usize, usize, String, usize) {
        let mut lvls = project.levels.clone();
        lvls.sort_by(|a, b| a.elevation_mm.total_cmp(&b.elevation_mm));
        if lvls.is_empty() {
            return (1, 1, "Level 1".to_string(), 1);
        }
        let idx = self.active_story_idx.min(lvls.len().saturating_sub(1));
        let base = &lvls[idx];
        let top = lvls.get(idx + 1).unwrap_or(base);
        (
            base.id,
            top.id,
            format!("{} ({:.1}m)", base.name, base.elevation_mm * 0.001),
            lvls.len(),
        )
    }

    /// Adds a new floor story `3500 mm` above the highest existing BIM level and selects it.
    pub fn add_story_above_top(&mut self, build_view: &mut BuildView) -> Result<String, String> {
        let mut lvls = build_view.bim_bus.project.levels.clone();
        lvls.sort_by(|a, b| a.elevation_mm.total_cmp(&b.elevation_mm));
        let max_elev = lvls.last().map(|l| l.elevation_mm).unwrap_or(0.0);
        let next_num = lvls.len() + 1;
        let msg = build_view.bim_bus.execute(BimCommand::AddLevel {
            name: format!("Level {next_num}"),
            elevation_mm: max_elev + 3500.0,
        })?;
        self.active_story_idx = lvls.len().saturating_sub(1);
        self.push_trace(msg.clone(), true);
        Ok(msg)
    }

    /// Creates a structural concrete slab on the target elevated story spanning project extents.
    pub fn add_slab_at_active_story(&mut self, build_view: &mut BuildView) -> Result<String, String> {
        let mut lvls = build_view.bim_bus.project.levels.clone();
        lvls.sort_by(|a, b| a.elevation_mm.total_cmp(&b.elevation_mm));
        let level_id = if lvls.is_empty() {
            1
        } else {
            let idx = (self.active_story_idx + 1).min(lvls.len() - 1);
            lvls[idx].id
        };
        let (min_x, max_x, min_y, max_y) =
            if let Some((pmin, pmax)) = build_view.bim_bus.project.plan_extents_mm() {
                if (pmax[0] - pmin[0]).abs() < 800.0 || (pmax[1] - pmin[1]).abs() < 800.0 {
                    (-4000.0, 4000.0, -3000.0, 3000.0)
                } else {
                    (pmin[0], pmax[0], pmin[1], pmax[1])
                }
            } else {
                (-4000.0, 4000.0, -3000.0, 3000.0)
            };
        let msg = build_view.bim_bus.execute(BimCommand::AddSlab {
            level_id,
            boundary: vec![
                crate::core::P2(min_x, min_y),
                crate::core::P2(max_x, min_y),
                crate::core::P2(max_x, max_y),
                crate::core::P2(min_x, max_y),
            ],
            openings: Vec::new(),
            thickness_mm: 180.0,
            is_roof: false,
            roof_slope_deg: 0.0,
        })?;
        self.active_canvas_mode = SpatialCanvasMode::Bim3D;
        self.push_trace(msg.clone(), true);
        Ok(msg)
    }

    /// Applies an instant 3D BIM camera orientation preset (`iso`, `top`, `front`).
    pub fn apply_camera_preset(&mut self, build_view: &mut BuildView, preset: &str) {
        self.active_canvas_mode = SpatialCanvasMode::Bim3D;
        match preset {
            "top" => {
                build_view.cam_yaw = 0.0;
                build_view.cam_pitch = 1.48;
                build_view.perspective = false;
                self.push_trace("Camera Preset: Plan Top", true);
            }
            "front" => {
                build_view.cam_yaw = 0.0;
                build_view.cam_pitch = 0.04;
                build_view.perspective = false;
                self.push_trace("Camera Preset: Front Elevation", true);
            }
            _ => {
                build_view.cam_yaw = 0.68;
                build_view.cam_pitch = 0.44;
                build_view.perspective = true;
                self.push_trace("Camera Preset: Isometric 3D", true);
            }
        }
    }

    /// Returns true if any manual 3D authoring, 2D CAD drawing, or geodesic measure tool is currently active.
    pub fn has_active_manual_tool(
        &self,
        build_view: &BuildView,
        universal_canvas: &UniversalCanvasState,
    ) -> bool {
        build_view.ground_author_tool != Ground3DAuthorTool::Orbit
            || build_view.pending_3d_start_mm.is_some()
            || universal_canvas.active_tool != UniversalCanvasTool::SelectPan
            || universal_canvas.measure_active
    }

    /// Resets any active manual BIM/2D CAD/Measure tool cleanly back to pointer Orbit / Select-Pan.
    pub fn reset_manual_tools_to_orbit(
        &mut self,
        build_view: &mut BuildView,
        universal_canvas: &mut UniversalCanvasState,
    ) {
        build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
        build_view.pending_3d_start_mm = None;
        universal_canvas.active_tool = UniversalCanvasTool::SelectPan;
        universal_canvas.measure_active = false;
        self.push_trace("Returned to pointer Orbit / Select", true);
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
                PlannedProposalAction::ResizeColumn {
                    column_id,
                    width_mm,
                    depth_mm,
                } => {
                    let rot = build_view
                        .bim_bus
                        .project
                        .columns
                        .iter()
                        .find(|c| c.id == *column_id)
                        .map(|c| c.rotation_deg)
                        .unwrap_or(0.0);
                    if let Ok(msg) = build_view.bim_bus.execute(BimCommand::ResizeColumn {
                        column_id: *column_id,
                        width_mm: *width_mm,
                        depth_mm: *depth_mm,
                        rotation_deg: rot,
                        profile_name: format!("RC {:.0}x{:.0}", width_mm, depth_mm),
                    }) {
                        self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                        out.sync_fem_and_earth = true;
                        self.push_trace(msg.clone(), true);
                        ai_assistant.chat_history.push(crate::models::ChatMessage {
                            is_user: false,
                            message: format!("[Applied Column Resize]: {msg}"),
                            time: crate::core::web_bridge::unix_timestamp_label(),
                        });
                    }
                }
                PlannedProposalAction::ResizeBeam {
                    run_id,
                    width_mm,
                    depth_mm,
                } => {
                    if let Ok(msg) = build_view.bim_bus.execute(BimCommand::ResizeRun {
                        run_id: *run_id,
                        width_mm: *width_mm,
                        depth_mm: *depth_mm,
                        profile_name: format!("RC {:.0}x{:.0}", width_mm, depth_mm),
                    }) {
                        self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                        out.sync_fem_and_earth = true;
                        self.push_trace(msg.clone(), true);
                        ai_assistant.chat_history.push(crate::models::ChatMessage {
                            is_user: false,
                            message: format!("[Applied Beam Resize]: {msg}"),
                            time: crate::core::web_bridge::unix_timestamp_label(),
                        });
                    }
                }
                PlannedProposalAction::AddColumnAt {
                    x_mm,
                    y_mm,
                    width_mm,
                    depth_mm,
                } => {
                    let mut lvls = build_view.bim_bus.project.levels.clone();
                    lvls.sort_by(|a, b| a.elevation_mm.total_cmp(&b.elevation_mm));
                    let base_level_id = lvls.first().map(|l| l.id).unwrap_or(1);
                    let top_level_id = lvls.get(1).map(|l| l.id).unwrap_or(base_level_id);
                    if let Ok(msg) = build_view.bim_bus.execute(BimCommand::AddColumn {
                        base_level_id,
                        top_level_id,
                        pos: crate::core::P2(*x_mm, *y_mm),
                        width_mm: *width_mm,
                        depth_mm: *depth_mm,
                        is_round: false,
                        profile_name: format!("C{:.0}x{:.0}", width_mm, depth_mm),
                    }) {
                        self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                        universal_canvas.ground_origin_anchored = true;
                        out.sync_fem_and_earth = true;
                        out.fit_camera = true;
                        self.push_trace(msg.clone(), true);
                        ai_assistant.chat_history.push(crate::models::ChatMessage {
                            is_user: false,
                            message: format!("[Added Column]: {msg}"),
                            time: crate::core::web_bridge::unix_timestamp_label(),
                        });
                    }
                }
                PlannedProposalAction::ApplySiteSeismic => {
                    out.apply_site_seismic = true;
                    out.sync_fem_and_earth = true;
                    self.push_trace("Coupled Earth Site Seismic & Wind Loads to FEM", true);
                }
                PlannedProposalAction::OpenSheet(pillar) => {
                    self.open_module_overlay(*pillar);
                    self.push_trace(format!("Opened {} Sheet", pillar.clean_name()), true);
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
                            GeminiDirective::ResizeColumn { id, w_mm, d_mm } => {
                                PlannedProposalAction::ResizeColumn {
                                    column_id: *id,
                                    width_mm: *w_mm,
                                    depth_mm: *d_mm,
                                }
                            }
                            GeminiDirective::ResizeBeam { id, b_mm, h_mm } => {
                                PlannedProposalAction::ResizeBeam {
                                    run_id: *id,
                                    width_mm: *b_mm,
                                    depth_mm: *h_mm,
                                }
                            }
                            GeminiDirective::AddColumn { x_mm, y_mm, w_mm, d_mm } => {
                                PlannedProposalAction::AddColumnAt {
                                    x_mm: *x_mm,
                                    y_mm: *y_mm,
                                    width_mm: *w_mm,
                                    depth_mm: *d_mm,
                                }
                            }
                            GeminiDirective::ApplySiteSeismic => PlannedProposalAction::ApplySiteSeismic,
                            GeminiDirective::OpenSheet(pillar) => {
                                PlannedProposalAction::OpenSheet(*pillar)
                            }
                        })
                        .collect();
                    let preview: String = directives
                        .iter()
                        .map(|d| match d {
                            GeminiDirective::Fly { lat, lon, .. } => format!("Fly({lat:.2},{lon:.2})"),
                            GeminiDirective::Bim(c) => format!("BIM:{c}"),
                            GeminiDirective::Repair => "AutoRepair".to_string(),
                            GeminiDirective::AriAudit => "ARI_Lens".to_string(),
                            GeminiDirective::ResizeColumn { id, w_mm, d_mm } => {
                                format!("ResizeCol#{id}({w_mm:.0}x{d_mm:.0})")
                            }
                            GeminiDirective::ResizeBeam { id, b_mm, h_mm } => {
                                format!("ResizeBeam#{id}({b_mm:.0}x{h_mm:.0})")
                            }
                            GeminiDirective::AddColumn { x_mm, y_mm, w_mm, d_mm } => {
                                format!("AddCol({x_mm:.0},{y_mm:.0},{w_mm:.0}x{d_mm:.0})")
                            }
                            GeminiDirective::ApplySiteSeismic => "SyncSiteSeismic".to_string(),
                            GeminiDirective::OpenSheet(pillar) => {
                                format!("OpenSheet({})", pillar.clean_name())
                            }
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
            BimElementKind::Slab => SmartSelectionTarget::Slab(id),
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
            SmartSelectionTarget::Slab(id) => project
                .slabs
                .iter()
                .find(|s| s.id == id)
                .map(|s| format!("@{}", s.name)),
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

        // 1b. Smart Local Imperative Routing when offline or explicit workspace imperatives are requested
        if lower == "undo" {
            actions.undo = true;
            return actions;
        } else if lower == "redo" {
            actions.redo = true;
            return actions;
        } else if lower == "focus 3d" || lower == "open 3d" || lower == "bim 3d" {
            self.active_canvas_mode = SpatialCanvasMode::Bim3D;
            actions.fit_camera = true;
            self.push_trace("Switched to 3D BIM Viewport", true);
            return actions;
        } else if lower == "iso view" || lower == "isometric view" {
            self.apply_camera_preset(build_view, "iso");
            actions.fit_camera = true;
            return actions;
        } else if lower == "top view" || lower == "plan view" {
            self.apply_camera_preset(build_view, "top");
            actions.fit_camera = true;
            return actions;
        } else if lower == "front view" || lower == "elevation view" {
            self.apply_camera_preset(build_view, "front");
            actions.fit_camera = true;
            return actions;
        } else if lower == "add story" || lower == "add level" || lower == "new story" {
            if self.add_story_above_top(build_view).is_ok() {
                actions.sync_fem_and_earth = true;
                actions.fit_camera = true;
            }
            return actions;
        } else if lower == "add slab" || lower == "create slab" {
            if self.add_slab_at_active_story(build_view).is_ok() {
                actions.sync_fem_and_earth = true;
                actions.fit_camera = true;
            }
            return actions;
        } else if let Some(target_sheet) = lower.strip_prefix("open ") {
            let pillar_opt = if target_sheet.contains("analysis") || target_sheet.contains("fem") {
                Some(NavigationPillar::AnalysisSimulation)
            } else if target_sheet.contains("sched") || target_sheet.contains("4d") {
                Some(NavigationPillar::TimeScheduling)
            } else if target_sheet.contains("budget") || target_sheet.contains("cost") || target_sheet.contains("boq") {
                Some(NavigationPillar::CostBudgeting)
            } else if target_sheet.contains("sustain") || target_sheet.contains("carbon") || target_sheet.contains("6d") {
                Some(NavigationPillar::SustainabilityEnergy)
            } else if target_sheet.contains("safety") || target_sheet.contains("8d") {
                Some(NavigationPillar::SafetyHealth)
            } else if target_sheet.contains("report") || target_sheet.contains("pbg") {
                Some(NavigationPillar::ReportPublishing)
            } else if target_sheet.contains("vault") {
                Some(NavigationPillar::MyLibrary)
            } else {
                None
            };
            if let Some(p) = pillar_opt {
                if self.agent_mode == AgentExecutionMode::Planning && self.agent_model != AgentModelChoice::EigenAiProFem {
                    self.queue_planning_proposal(
                        format!("Open {} Sheet", p.clean_name()),
                        "Review sheet navigation request before opening floating panel.",
                        format!("OPEN_SHEET({})", p.clean_name()),
                        vec![PlannedProposalAction::OpenSheet(p)],
                    );
                } else {
                    self.open_module_overlay(p);
                }
                return actions;
            }
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
            AgentModelChoice::Gemini25Flash | AgentModelChoice::Gemini25Pro
        ) && !self.gemini_api_key_draft.trim().is_empty()
        {
            let (lat, lon) = universal_canvas.ground_origin_lat_lon;
            let site = compute_site_environmental_coupling(lat, lon, &universal_canvas.earth_world);
            let sys_prompt = format!(
                "You are EigenAI inside EigenGraph (Geospatial Earth + Structural BIM/FEM & Adjusted Redundancy Index ARI platform). \
                 Respond concisely in professional civil & structural engineering English. \
                 Current Earth Site: ({lat:.3} deg, {lon:.3} deg), Elevation={:.1}m, Seismic Ss={:.2}g, Wind={:.1}m/s. \
                 Active BIM elements={}, FEM members={}. \
                 To propose workspace actions, you may append bracket directives: \
                 [[FLY:lat,lon,zoom]], [[BIM:portal 3x2 @6m 2floors]], [[RESIZE_COL:id,w_mm,d_mm]], [[RESIZE_BEAM:id,b_mm,h_mm]], [[ADD_COL:x_mm,y_mm,w_mm,d_mm]], [[APPLY_SITE_SEISMIC]], [[REPAIR]], [[ARI_AUDIT]], [[OPEN_SHEET:Analysis|Scheduling|Budgeting|Sustainability|Safety|Report]].",
                site.elevation_m,
                site.seismic_pga_ss_g,
                site.design_wind_ms,
                build_view.bim_bus.project.element_count(),
                members.len()
            );
            let mut history_turns: Vec<(bool, String)> = ai_assistant
                .chat_history
                .iter()
                .rev()
                .take(8)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(|m| (m.is_user, m.message.clone()))
                .collect();
            history_turns.push((true, enriched_prompt.clone()));

            let model_id = self.agent_model.gemini_model_id();
            if let Ok(req_id) = crate::core::web_bridge::gemini_send_conversation(
                &self.gemini_api_key_draft,
                model_id,
                &sys_prompt,
                &history_turns,
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
            SmartSelectionTarget::Slab(id) => {
                let s = project.slabs.iter().find(|s| s.id == id)?;
                Some(format!(
                    "Slab {} (t={:.0}mm, level=#{}, openings={})",
                    s.name, s.thickness_mm, s.level_id, s.openings.len()
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

    /// Renders all Floating HUD layers over the 100% Full-Canvas workspace (ZERO side panels or corner chrome):
    /// 1. Conditional Top-Center Active Lens / Tool Pill (only when non-Orbit/Clean)
    /// 2. Smart Element Pop-Up Inspector (only when an element or Earth location is clicked)
    /// 3. Dynamic Island Bottom-Center Studio (3-row resting dock expanding upward on cursor hover)
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

        // 1. Conditional Top-Center Active Lens / Manual Tool Pill (top-left corner stays 100% pure canvas)
        if self.active_lens != SpatialLens::Clean
            || build_view.ground_author_tool != Ground3DAuthorTool::Orbit
            || universal_canvas.active_tool != UniversalCanvasTool::SelectPan
            || universal_canvas.measure_active
        {
            let top_center_offset = Vec2::new(0.0, 12.0);
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
                                        ui.label(RichText::new("X:").size(10.0).color(theme.text_muted));
                                        ui.add(
                                            egui::DragValue::new(&mut self.manual_place_x_mm)
                                                .speed(100.0)
                                                .suffix("mm"),
                                        );
                                        ui.label(RichText::new("Y:").size(10.0).color(theme.text_muted));
                                        ui.add(
                                            egui::DragValue::new(&mut self.manual_place_y_mm)
                                                .speed(100.0)
                                                .suffix("mm"),
                                        );
                                        if ui.small_button("Place @(X,Y)").clicked() {
                                            let (base_id, top_id, _, _) =
                                                self.active_base_and_top_levels(&build_view.bim_bus.project);
                                            let w = build_view.author_col_width_mm.clamp(150.0, 2000.0);
                                            let d = build_view.author_col_depth_mm.clamp(150.0, 2000.0);
                                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::AddColumn {
                                                base_level_id: base_id,
                                                top_level_id: top_id,
                                                pos: crate::core::P2(self.manual_place_x_mm, self.manual_place_y_mm),
                                                width_mm: w,
                                                depth_mm: d,
                                                is_round: false,
                                                profile_name: format!("C{:.0}x{:.0}", w, d),
                                            }) {
                                                universal_canvas.ground_origin_anchored = true;
                                                actions.sync_fem_and_earth = true;
                                                self.push_trace(msg, true);
                                            }
                                        }
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
                                        if let Some(start_mm) = build_view.pending_3d_start_mm {
                                            ui.label(RichText::new("L:").size(10.0).color(theme.text_muted));
                                            ui.add(
                                                egui::DragValue::new(&mut self.manual_polar_len_m)
                                                    .range(0.5..=60.0)
                                                    .speed(0.25)
                                                    .suffix("m"),
                                            );
                                            ui.label(RichText::new("∠:").size(10.0).color(theme.text_muted));
                                            ui.add(
                                                egui::DragValue::new(&mut self.manual_polar_deg)
                                                    .range(-180.0..=180.0)
                                                    .speed(5.0)
                                                    .suffix("°"),
                                            );
                                            if ui.small_button("Commit Span").clicked() {
                                                let rad = self.manual_polar_deg.to_radians();
                                                let end_x = start_mm[0] + self.manual_polar_len_m * 1000.0 * rad.cos();
                                                let end_y = start_mm[1] + self.manual_polar_len_m * 1000.0 * rad.sin();
                                                let (base_id, _, _, _) =
                                                    self.active_base_and_top_levels(&build_view.bim_bus.project);
                                                let bw = build_view.author_beam_width_mm.clamp(120.0, 1500.0);
                                                let bd = build_view.author_beam_depth_mm.clamp(150.0, 2500.0);
                                                if let Ok(msg) = build_view.bim_bus.execute(crate::core::BimCommand::AddBeamRun {
                                                    level_id: base_id,
                                                    category: crate::core::RunCategory::StructuralBeam,
                                                    start: crate::core::P2(start_mm[0], start_mm[1]),
                                                    end: crate::core::P2(end_x, end_y),
                                                    width_mm: bw,
                                                    depth_mm: bd,
                                                    justify: crate::core::RunJustify::Top,
                                                    profile_name: format!("B{bw:.0}x{bd:.0}"),
                                                }) {
                                                    build_view.pending_3d_start_mm = None;
                                                    actions.sync_fem_and_earth = true;
                                                    self.push_trace(msg, true);
                                                }
                                            }
                                        }
                                    }
                                    Ground3DAuthorTool::Wall => {
                                        ui.label(RichText::new("t:").size(10.0).color(theme.text_muted));
                                        ui.add(
                                            egui::DragValue::new(&mut build_view.author_wall_thickness_mm)
                                                .range(80.0..=1200.0)
                                                .speed(10.0)
                                                .suffix("mm"),
                                        );
                                        if let Some(start_mm) = build_view.pending_3d_start_mm {
                                            ui.label(RichText::new("L:").size(10.0).color(theme.text_muted));
                                            ui.add(
                                                egui::DragValue::new(&mut self.manual_polar_len_m)
                                                    .range(0.5..=60.0)
                                                    .speed(0.25)
                                                    .suffix("m"),
                                            );
                                            ui.label(RichText::new("∠:").size(10.0).color(theme.text_muted));
                                            ui.add(
                                                egui::DragValue::new(&mut self.manual_polar_deg)
                                                    .range(-180.0..=180.0)
                                                    .speed(5.0)
                                                    .suffix("°"),
                                            );
                                            if ui.small_button("Commit Wall").clicked() {
                                                let rad = self.manual_polar_deg.to_radians();
                                                let end_x = start_mm[0] + self.manual_polar_len_m * 1000.0 * rad.cos();
                                                let end_y = start_mm[1] + self.manual_polar_len_m * 1000.0 * rad.sin();
                                                let (base_id, _, _, _) =
                                                    self.active_base_and_top_levels(&build_view.bim_bus.project);
                                                let wt = build_view.author_wall_thickness_mm.clamp(80.0, 1200.0);
                                                if let Ok(msg) = build_view.bim_bus.execute(crate::core::BimCommand::AddWall {
                                                    base_level_id: base_id,
                                                    start: crate::core::P2(start_mm[0], start_mm[1]),
                                                    end: crate::core::P2(end_x, end_y),
                                                    thickness_mm: wt,
                                                    is_structural: true,
                                                }) {
                                                    build_view.pending_3d_start_mm = None;
                                                    actions.sync_fem_and_earth = true;
                                                    self.push_trace(msg, true);
                                                }
                                            }
                                        }
                                    }
                                    Ground3DAuthorTool::Footing => {
                                        ui.label(RichText::new("Size:").size(10.0).color(theme.text_muted));
                                        ui.add(
                                            egui::DragValue::new(&mut build_view.author_footing_size_mm)
                                                .range(400.0..=6000.0)
                                                .speed(50.0)
                                                .suffix("mm"),
                                        );
                                        ui.label(RichText::new("X:").size(10.0).color(theme.text_muted));
                                        ui.add(
                                            egui::DragValue::new(&mut self.manual_place_x_mm)
                                                .speed(100.0)
                                                .suffix("mm"),
                                        );
                                        ui.label(RichText::new("Y:").size(10.0).color(theme.text_muted));
                                        ui.add(
                                            egui::DragValue::new(&mut self.manual_place_y_mm)
                                                .speed(100.0)
                                                .suffix("mm"),
                                        );
                                        if ui.small_button("Place @(X,Y)").clicked() {
                                            let (base_id, _, _, _) =
                                                self.active_base_and_top_levels(&build_view.bim_bus.project);
                                            let fs = build_view.author_footing_size_mm.clamp(400.0, 6000.0);
                                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::AddFooting {
                                                level_id: base_id,
                                                pos: crate::core::P2(self.manual_place_x_mm, self.manual_place_y_mm),
                                                width_mm: fs,
                                                length_mm: fs,
                                                thickness_mm: 450.0,
                                            }) {
                                                universal_canvas.ground_origin_anchored = true;
                                                actions.sync_fem_and_earth = true;
                                                self.push_trace(msg, true);
                                            }
                                        }
                                    }
                                    Ground3DAuthorTool::Orbit => {}
                                }
                                let (_, _, story_lbl, lvl_cnt) =
                                    self.active_base_and_top_levels(&build_view.bim_bus.project);
                                if ui
                                    .small_button(format!("Story: {story_lbl}"))
                                    .on_hover_text("Cycle active base level for placement")
                                    .clicked()
                                {
                                    self.active_story_idx = (self.active_story_idx + 1) % lvl_cnt.max(1);
                                }
                                if ui
                                    .small_button("+ Story")
                                    .on_hover_text("Add upper floor story (+3.5m) via undoable BimCommand")
                                    .clicked()
                                    && self.add_story_above_top(build_view).is_ok()
                                {
                                    actions.sync_fem_and_earth = true;
                                    actions.fit_camera = true;
                                }
                                if ui
                                    .small_button("+ Slab")
                                    .on_hover_text("Add structural concrete floor slab on active upper level")
                                    .clicked()
                                    && self.add_slab_at_active_story(build_view).is_ok()
                                {
                                    actions.sync_fem_and_earth = true;
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
                                match universal_canvas.active_tool {
                                    UniversalCanvasTool::Line => {
                                        ui.label(RichText::new("L:").size(9.8).color(theme.text_muted));
                                        ui.add(
                                            egui::DragValue::new(&mut self.draw_2d_len_m)
                                                .range(0.1..=250.0)
                                                .speed(0.25)
                                                .suffix("m"),
                                        );
                                        ui.label(RichText::new("@").size(9.8).color(theme.text_muted));
                                        ui.add(
                                            egui::DragValue::new(&mut self.draw_2d_deg)
                                                .range(-180.0..=180.0)
                                                .speed(1.0)
                                                .suffix("°"),
                                        );
                                        if ui.small_button("Add Line").clicked() {
                                            let eid = universal_canvas.commit_parametric_2d_line(
                                                self.draw_2d_len_m,
                                                self.draw_2d_deg,
                                                "2D",
                                                "Drafting",
                                            );
                                            self.selected_target = Some(SmartSelectionTarget::SketchEntity(eid));
                                            self.push_trace(
                                                format!("Added 2D Line {:.2}m @ {:.0}°", self.draw_2d_len_m, self.draw_2d_deg),
                                                true,
                                            );
                                        }
                                    }
                                    UniversalCanvasTool::Rectangle => {
                                        ui.label(RichText::new("W:").size(9.8).color(theme.text_muted));
                                        ui.add(
                                            egui::DragValue::new(&mut self.draw_2d_rect_w_m)
                                                .range(0.2..=250.0)
                                                .speed(0.25)
                                                .suffix("m"),
                                        );
                                        ui.label(RichText::new("H:").size(9.8).color(theme.text_muted));
                                        ui.add(
                                            egui::DragValue::new(&mut self.draw_2d_rect_h_m)
                                                .range(0.2..=250.0)
                                                .speed(0.25)
                                                .suffix("m"),
                                        );
                                        if ui.small_button("Add Rect").clicked() {
                                            let eid = universal_canvas.commit_parametric_2d_rect(
                                                self.draw_2d_rect_w_m,
                                                self.draw_2d_rect_h_m,
                                                "2D",
                                                "Drafting",
                                            );
                                            self.selected_target = Some(SmartSelectionTarget::SketchEntity(eid));
                                            self.push_trace(
                                                format!("Added 2D Rect {:.2}x{:.2}m", self.draw_2d_rect_w_m, self.draw_2d_rect_h_m),
                                                true,
                                            );
                                        }
                                    }
                                    UniversalCanvasTool::Circle => {
                                        ui.label(RichText::new("R:").size(9.8).color(theme.text_muted));
                                        ui.add(
                                            egui::DragValue::new(&mut self.draw_2d_radius_m)
                                                .range(0.1..=150.0)
                                                .speed(0.2)
                                                .suffix("m"),
                                        );
                                        if ui.small_button("Add Circle").clicked() {
                                            let eid = universal_canvas.commit_parametric_2d_circle(
                                                self.draw_2d_radius_m,
                                                "2D",
                                                "Drafting",
                                            );
                                            self.selected_target = Some(SmartSelectionTarget::SketchEntity(eid));
                                            self.push_trace(
                                                format!("Added 2D Circle R={:.2}m", self.draw_2d_radius_m),
                                                true,
                                            );
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            if universal_canvas.measure_active {
                                let (dist_m, area_opt, pts_n) =
                                    universal_canvas.geodesic_measure_summary();
                                let measure_lbl = if let Some(area_m2) = area_opt {
                                    format!(
                                        "Measure: {:.0} m² ({:.2} ha) • {:.1} m",
                                        area_m2,
                                        area_m2 / 10_000.0,
                                        dist_m
                                    )
                                } else if pts_n >= 2 {
                                    if dist_m >= 1000.0 {
                                        format!("Measure: {:.2} km ({pts_n} pts)", dist_m / 1000.0)
                                    } else {
                                        format!("Measure: {dist_m:.1} m ({pts_n} pts)")
                                    }
                                } else {
                                    "Geodesic Measure".to_string()
                                };
                                icons::pill_badge(
                                    ui,
                                    AppIcon::DimensionRuler,
                                    measure_lbl,
                                    theme.accent_green,
                                );
                                if !universal_canvas.measure_points_lat_lon.is_empty()
                                    && ui.small_button("Clear").clicked()
                                {
                                    universal_canvas.clear_measure();
                                    self.push_trace("Cleared geodesic measurement points", true);
                                }
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

        // 3. Smart Element Pop-Up Inspector (for structural elements & 2D CAD; EarthSite now lives in Dynamic Island)
        if let Some(target) = self.selected_target {
            if !matches!(target, SmartSelectionTarget::EarthSite { .. }) {
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
            if let SmartSelectionTarget::SketchEntity(eid) = target {
                if let Some((_, _, _, _, _, col, sw)) = universal_canvas.entity_summary(eid) {
                    self.sketch_edit_color = col;
                    self.sketch_edit_width = sw;
                }
            }
            self.last_loaded_target = Some(target);
        }

        let card_w = if is_mobile {
            (vp.x - 16.0).clamp(200.0, 380.0)
        } else {
            (vp.x * 0.34).clamp(310.0, 385.0)
        };
        let inner_w = (card_w - 28.0).max(170.0);
        let max_h = if is_mobile {
            (vp.y * 0.44).clamp(190.0, 340.0)
        } else {
            (vp.y - 125.0).clamp(260.0, 520.0)
        };
        let (anchor, offset) = if is_mobile {
            (Align2::CENTER_TOP, Vec2::new(0.0, 12.0))
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
            SmartSelectionTarget::Slab(id) => {
                if let Some(s) = project.slabs.iter().find(|s| s.id == id) {
                    (
                        s.name.clone(),
                        format!(
                            "Structural {} • t={:.0} mm • {} opening(s)",
                            if s.is_roof { "Roof" } else { "Slab" },
                            s.thickness_mm,
                            s.openings.len()
                        ),
                        AppIcon::Slab,
                    )
                } else {
                    (format!("Slab #{id}"), "Removed or missing".to_string(), AppIcon::Slab)
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

    /// Renders the interactive Earth Site Location inspector & build actions cleanly inside Dynamic Island.
    #[allow(clippy::too_many_arguments)]
    fn render_earth_site_inspector_strip(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        lat: f64,
        lon: f64,
        panel_w: f32,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();
        let picked_site =
            compute_site_environmental_coupling(lat, lon, &universal_canvas.earth_world);

        Frame::new()
            .fill(Color32::TRANSPARENT)
            .inner_margin(Margin::symmetric(2, 4))
            .show(ui, |ui| {
                let sw = (panel_w - 10.0).max(160.0);
                ui.set_width(sw);
                ui.set_max_width(sw);
                ui.horizontal(|ui| {
                    icons::icon_label(
                        ui,
                        AppIcon::Globe,
                        13.0,
                        theme.accent_blue,
                        format!("Earth Site Location ({lat:.4}°, {lon:.4}°)"),
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
                            "Dismiss Earth Site Location",
                        )
                        .clicked()
                        {
                            self.selected_target = None;
                            universal_canvas.last_picked_earth_lat_lon = None;
                        }
                    });
                });
                ui.add_space(3.0);
                ui.horizontal_wrapped(|ui| {
                    icons::pill_badge(
                        ui,
                        AppIcon::MapPin,
                        format!("Elev {:+.1} m", picked_site.elevation_m),
                        theme.accent_blue,
                    );
                    icons::pill_badge(
                        ui,
                        AppIcon::Bolt,
                        format!(
                            "Ss {:.2}g / S1 {:.2}g",
                            picked_site.seismic_pga_ss_g, picked_site.seismic_pga_s1_g
                        ),
                        theme.accent_orange,
                    );
                    icons::pill_badge(
                        ui,
                        AppIcon::Globe,
                        format!("Wind {:.1} m/s", picked_site.design_wind_ms),
                        theme.accent_blue,
                    );
                    icons::pill_badge(
                        ui,
                        AppIcon::AlertTriangle,
                        format!("Flood {:.0}%", picked_site.flood_risk_index * 100.0),
                        if picked_site.flood_risk_index > 0.55 {
                            theme.accent_red
                        } else {
                            theme.accent_green
                        },
                    );
                    ui.label(
                        RichText::new(&picked_site.recommended_soil_class)
                            .size(9.8)
                            .color(theme.text_secondary),
                    );
                });
                ui.label(
                    RichText::new(picked_site.foundation_advisory)
                        .size(9.6)
                        .color(theme.text_muted),
                );
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if icons::icon_button(
                        ui,
                        AppIcon::MapPin,
                        "Anchor Site Here",
                        9.8,
                        Color32::WHITE,
                        theme.accent_blue,
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::new(0.0, 22.0),
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
                        9.8,
                        Color32::WHITE,
                        theme.accent_green,
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::new(0.0, 22.0),
                    )
                    .clicked()
                    {
                        universal_canvas.ground_origin_lat_lon = (lat, lon);
                        universal_canvas.ground_origin_anchored = true;
                        self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                        if build_view.bim_bus.project.is_empty() {
                            build_view.ground_author_tool = Ground3DAuthorTool::Column;
                        }
                        actions.anchor_and_build_site = Some((lat, lon));
                        self.push_trace(format!("Anchored & Opened 3D BIM @ {lat:.3}°, {lon:.3}°"), true);
                    }
                    if icons::icon_button(
                        ui,
                        AppIcon::Grid,
                        "Build 3x2 Portal",
                        9.8,
                        Color32::WHITE,
                        theme.accent_orange,
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::new(0.0, 22.0),
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
                        9.8,
                        theme.text_primary,
                        theme.bg_card,
                        Stroke::new(0.9, theme.border_subtle),
                        CornerRadius::same(7),
                        Vec2::new(0.0, 22.0),
                    )
                    .clicked()
                    {
                        universal_canvas.ground_origin_lat_lon = (lat, lon);
                        universal_canvas.ground_origin_anchored = true;
                        actions.load_sample_building = true;
                        self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                    }
                    if !build_view.bim_bus.project.is_empty()
                        && icons::icon_button(
                            ui,
                            AppIcon::Building,
                            "Focus 3D BIM",
                            9.8,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(7),
                            Vec2::new(0.0, 22.0),
                        )
                        .clicked()
                    {
                        self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                        actions.fit_camera = true;
                        self.push_trace("Focused 3D BIM Viewport", true);
                    }
                    if icons::icon_button(
                        ui,
                        AppIcon::Bolt,
                        "Sync Site Seismic",
                        9.8,
                        theme.accent_orange,
                        theme.bg_card,
                        Stroke::new(0.9, theme.accent_orange.gamma_multiply(0.45)),
                        CornerRadius::same(7),
                        Vec2::new(0.0, 22.0),
                    )
                    .clicked()
                    {
                        universal_canvas.ground_origin_lat_lon = (lat, lon);
                        universal_canvas.ground_origin_anchored = true;
                        actions.apply_site_seismic = true;
                        self.push_trace(
                            format!("Synced Earth Site Ss={:.2}g, Wind={:.0} m/s", picked_site.seismic_pga_ss_g, picked_site.design_wind_ms),
                            true,
                        );
                    }
                });
            });
        actions
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
                    if !build_view.bim_bus.project.is_empty()
                        && icons::icon_button(
                            ui,
                            AppIcon::Building,
                            "Focus 3D BIM",
                            10.5,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(7),
                            Vec2::new(0.0, 26.0),
                        )
                        .on_hover_text("Switch directly to interactive 3D BIM viewport centered on active model")
                        .clicked()
                    {
                        self.active_canvas_mode = SpatialCanvasMode::Bim3D;
                        actions.fit_camera = true;
                        self.push_trace("Focused 3D BIM Viewport", true);
                    }
                    if icons::icon_button(
                        ui,
                        AppIcon::Bolt,
                        "Sync Site Seismic",
                        10.5,
                        theme.accent_orange,
                        theme.bg_card,
                        Stroke::new(1.0, theme.accent_orange.gamma_multiply(0.45)),
                        CornerRadius::same(7),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Apply local seismic Ss/S1 & design wind speed from this coordinate to FEM design")
                    .clicked()
                    {
                        universal_canvas.ground_origin_lat_lon = (lat, lon);
                        universal_canvas.ground_origin_anchored = true;
                        actions.apply_site_seismic = true;
                        self.push_trace(
                            format!("Synced Earth Site Ss={:.2}g, Wind={:.0} m/s", picked_site.seismic_pga_ss_g, picked_site.design_wind_ms),
                            true,
                        );
                    }
                });
            }
            SmartSelectionTarget::SketchEntity(id) => {
                if let Some((kind_lbl, len_m, area_m2, rad_m, detail, _, _)) =
                    universal_canvas.entity_summary(id)
                {
                    Self::render_kv_row(ui, theme, "Entity Type", kind_lbl, theme.accent_blue);
                    Self::render_kv_row(ui, theme, "Geometry", &detail, theme.text_primary);
                    if len_m > 0.001 {
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Length / Perimeter",
                            &format!("{len_m:.3} m"),
                            theme.accent_green,
                        );
                    }
                    if area_m2 > 0.0001 {
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Enclosed Area",
                            &format!("{area_m2:.2} m²"),
                            theme.accent_orange,
                        );
                    }
                    if rad_m > 0.0001 {
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Radius",
                            &format!("{rad_m:.3} m"),
                            theme.text_secondary,
                        );
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Stroke (px):").size(10.5).color(theme.text_secondary));
                        ui.add(
                            egui::DragValue::new(&mut self.sketch_edit_width)
                                .range(0.8..=12.0)
                                .speed(0.1),
                        );
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Color:").size(10.5).color(theme.text_secondary));
                        for (lbl, rgb) in [
                            ("Blue", [59, 130, 246]),
                            ("Green", [16, 185, 129]),
                            ("Orange", [249, 115, 22]),
                            ("Violet", [168, 85, 247]),
                        ] {
                            let active_c = self.sketch_edit_color == rgb;
                            let sw_col = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(lbl).size(9.5).color(Color32::WHITE),
                                    )
                                    .min_size(Vec2::new(0.0, 21.0))
                                    .fill(sw_col)
                                    .stroke(if active_c {
                                        Stroke::new(1.5, Color32::WHITE)
                                    } else {
                                        Stroke::NONE
                                    })
                                    .corner_radius(CornerRadius::same(6)),
                                )
                                .clicked()
                            {
                                self.sketch_edit_color = rgb;
                            }
                        }
                    });
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        if icons::icon_button(
                            ui,
                            AppIcon::CheckCircle,
                            "Apply Style",
                            10.0,
                            Color32::WHITE,
                            theme.accent_green,
                            Stroke::NONE,
                            CornerRadius::same(7),
                            Vec2::new(0.0, 24.0),
                        )
                        .clicked()
                        && universal_canvas.update_entity_style(
                            id,
                            self.sketch_edit_color,
                            self.sketch_edit_width,
                        )
                        {
                            self.push_trace(format!("Updated 2D CAD #{id} style"), true);
                        }
                        if icons::icon_button(
                            ui,
                            AppIcon::Layers,
                            "Duplicate (+500mm)",
                            10.0,
                            theme.text_primary,
                            theme.bg_card,
                            Stroke::new(0.9, theme.border_subtle),
                            CornerRadius::same(7),
                            Vec2::new(0.0, 24.0),
                        )
                        .clicked()
                        {
                            if let Some(new_id) =
                                universal_canvas.duplicate_offset_entity(id, 500.0, 500.0)
                            {
                                self.selected_target = Some(SmartSelectionTarget::SketchEntity(new_id));
                                self.push_trace(format!("Duplicated CAD #{id} -> #{new_id}"), true);
                            }
                        }
                        if icons::icon_button(
                            ui,
                            AppIcon::Building,
                            "Promote to 3D BIM",
                            10.0,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(7),
                            Vec2::new(0.0, 24.0),
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
                    });
                    ui.add_space(4.0);
                    if icons::icon_button(
                        ui,
                        AppIcon::Trash,
                        "Delete Entity (Undoable)",
                        10.0,
                        theme.accent_red,
                        theme.accent_red.gamma_multiply(0.12),
                        Stroke::new(0.9, theme.accent_red.gamma_multiply(0.45)),
                        CornerRadius::same(7),
                        Vec2::new(ui.available_width(), 24.0),
                    )
                    .clicked()
                    && universal_canvas.delete_entity_by_id(id)
                    {
                        self.selected_target = None;
                        self.push_trace(format!("Deleted 2D CAD entity #{id}"), true);
                    }
                } else {
                    Self::render_kv_row(
                        ui,
                        theme,
                        "Entity ID",
                        &format!("#{id} (Removed)"),
                        theme.text_muted,
                    );
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
                        ui.add_space(4.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::Trash,
                            "Delete Column (Undoable)",
                            10.5,
                            theme.accent_red,
                            theme.accent_red.gamma_multiply(0.12),
                            Stroke::new(0.9, theme.accent_red.gamma_multiply(0.45)),
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 26.0),
                        )
                        .clicked()
                        {
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::DeleteElement {
                                element_id: col_id,
                            }) {
                                actions.sync_fem_and_earth = true;
                                self.selected_target = None;
                                universal_canvas.highlighted_bim_element = None;
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
                        ui.add_space(4.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::Trash,
                            "Delete Beam (Undoable)",
                            10.5,
                            theme.accent_red,
                            theme.accent_red.gamma_multiply(0.12),
                            Stroke::new(0.9, theme.accent_red.gamma_multiply(0.45)),
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 26.0),
                        )
                        .clicked()
                        {
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::DeleteElement {
                                element_id: run_id,
                            }) {
                                actions.sync_fem_and_earth = true;
                                self.selected_target = None;
                                universal_canvas.highlighted_bim_element = None;
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
                        ui.add_space(4.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::Trash,
                            "Delete Wall (Undoable)",
                            10.5,
                            theme.accent_red,
                            theme.accent_red.gamma_multiply(0.12),
                            Stroke::new(0.9, theme.accent_red.gamma_multiply(0.45)),
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 26.0),
                        )
                        .clicked()
                        {
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::DeleteElement {
                                element_id: wall_id,
                            }) {
                                actions.sync_fem_and_earth = true;
                                self.selected_target = None;
                                universal_canvas.highlighted_bim_element = None;
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
                        ui.add_space(4.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::Trash,
                            "Delete Footing (Undoable)",
                            10.5,
                            theme.accent_red,
                            theme.accent_red.gamma_multiply(0.12),
                            Stroke::new(0.9, theme.accent_red.gamma_multiply(0.45)),
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 26.0),
                        )
                        .clicked()
                        {
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::DeleteElement {
                                element_id: foot_id,
                            }) {
                                actions.sync_fem_and_earth = true;
                                self.selected_target = None;
                                universal_canvas.highlighted_bim_element = None;
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
                            "Site Hazard (Ss / Wind)",
                            &format!("{:.2}g / {:.0} m/s", site.seismic_pga_ss_g, site.design_wind_ms),
                            theme.accent_orange,
                        );
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
            SmartSelectionTarget::Slab(slab_id) => {
                let slab_opt = build_view
                    .bim_bus
                    .project
                    .slabs
                    .iter()
                    .find(|s| s.id == slab_id)
                    .cloned();
                let Some(slab) = slab_opt else {
                    ui.label("Slab/Roof not found in active model.");
                    return actions;
                };
                let area_mm2 = {
                    let n = slab.boundary.len();
                    let mut sum = 0.0_f64;
                    for i in 0..n {
                        let a = slab.boundary[i];
                        let b = slab.boundary[(i + 1) % n];
                        sum += a.0 * b.1 - b.0 * a.1;
                    }
                    0.5 * sum.abs()
                };
                let area_m2 = area_mm2 * 1e-6;
                let vol_m3 = area_m2 * (slab.thickness_mm * 0.001);
                let elev_m = build_view.bim_bus.project.level_elevation(slab.level_id) * 0.001;
                match self.popup_tab {
                    SmartPopupTab::Properties => {
                        Self::render_kv_row(ui, theme, "Level Elevation", &format!("{elev_m:.2} m"), theme.text_primary);
                        Self::render_kv_row(ui, theme, "Thickness", &format!("{:.0} mm", slab.thickness_mm), theme.accent_blue);
                        Self::render_kv_row(ui, theme, "Footprint Area", &format!("{area_m2:.2} m²"), theme.accent_green);
                        Self::render_kv_row(ui, theme, "Shaft Openings", &format!("{}", slab.openings.len()), theme.text_secondary);
                        ui.add_space(6.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::Trash,
                            "Delete Slab (Undoable)",
                            10.5,
                            theme.accent_red,
                            theme.accent_red.gamma_multiply(0.12),
                            Stroke::new(0.9, theme.accent_red.gamma_multiply(0.45)),
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 26.0),
                        )
                        .clicked()
                        {
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::DeleteElement {
                                element_id: slab_id,
                            }) {
                                actions.sync_fem_and_earth = true;
                                self.selected_target = None;
                                universal_canvas.highlighted_bim_element = None;
                                self.push_trace(msg, true);
                            }
                        }
                    }
                    SmartPopupTab::Calculations => {
                        let sw_kpa = 24.0 * (slab.thickness_mm * 0.001);
                        let est_factored_kpa = 1.2 * (sw_kpa + 1.5) + 1.6 * 2.5;
                        Self::render_kv_row(ui, theme, "Self-Weight (DL)", &format!("{sw_kpa:.2} kPa"), theme.text_primary);
                        Self::render_kv_row(ui, theme, "Factored Qu (1.2D+1.6L)", &format!("{est_factored_kpa:.2} kPa"), theme.accent_orange);
                    }
                    SmartPopupTab::Lifecycle5D6D => {
                        let co2_kg = vol_m3 * 305.0;
                        Self::render_kv_row(ui, theme, "Concrete Volume", &format!("{vol_m3:.2} m³"), theme.text_primary);
                        Self::render_kv_row(ui, theme, "Embodied Carbon", &format!("{co2_kg:.1} kgCO₂e"), theme.accent_green);
                    }
                    SmartPopupTab::AskAi => {
                        self.render_popup_ai_actions(
                            ui,
                            theme,
                            &slab.name,
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

        Frame::new()
            .fill(Color32::TRANSPARENT)
            .inner_margin(Margin::symmetric(2, 4))
            .show(ui, |ui| {
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

        Frame::new()
            .fill(Color32::TRANSPARENT)
            .inner_margin(Margin::symmetric(2, 4))
            .show(ui, |ui| {
                let inner_w = (panel_w - 10.0).max(160.0);
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
                            if ui
                                .small_button(if self.vault_note_composer_open {
                                    "Close Note"
                                } else {
                                    "+ Note (.md)"
                                })
                                .on_hover_text("Create a new Obsidian Markdown engineering note inside KnowledgeBase/")
                                .clicked()
                            {
                                self.vault_note_composer_open = !self.vault_note_composer_open;
                            }
                        });
                    });
                    ui.add_space(3.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.vault_filter_query)
                                .desired_width((inner_w * 0.44).max(95.0))
                                .hint_text("Filter vault files..."),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.vault_new_folder_name)
                                .desired_width((inner_w * 0.34).max(80.0))
                                .hint_text("New subfolder..."),
                        );
                        if ui
                            .small_button("+ Folder")
                            .on_hover_text("Create organized subfolder inside active Vault")
                            .clicked()
                            && !self.vault_new_folder_name.trim().is_empty()
                        {
                            match crate::core::InternalVault::create_vault_subfolder(
                                &self.vault_new_folder_name,
                            ) {
                                Ok(rel) => {
                                    self.vault_feedback = Some(format!("Created subfolder note '{rel}'."));
                                    self.vault_new_folder_name.clear();
                                    actions.refresh_vault = true;
                                    self.push_trace(format!("Created folder -> {rel}"), true);
                                }
                                Err(err) => {
                                    self.vault_feedback = Some(err);
                                }
                            }
                        }
                    });
                    if self.vault_note_composer_open {
                        ui.add_space(4.0);
                        Frame::new()
                            .fill(theme.bg_sidebar)
                            .corner_radius(CornerRadius::same(8))
                            .stroke(Stroke::new(0.8, theme.accent_blue.gamma_multiply(0.45)))
                            .inner_margin(Margin::same(6))
                            .show(ui, |ui| {
                                ui.set_width((inner_w - 16.0).max(140.0));
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.vault_note_title_input)
                                        .hint_text("Note title (e.g. Seismic_Design_Criteria)")
                                        .desired_width(ui.available_width()),
                                );
                                ui.add_space(3.0);
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.vault_note_body_input)
                                        .desired_rows(2)
                                        .hint_text("# Notes...\n- Concrete fc' = 35 MPa...")
                                        .desired_width(ui.available_width()),
                                );
                                ui.add_space(3.0);
                                if icons::icon_button(
                                    ui,
                                    AppIcon::SaveDisk,
                                    "Save to KnowledgeBase/*.md",
                                    9.8,
                                    Color32::WHITE,
                                    theme.accent_green,
                                    Stroke::NONE,
                                    CornerRadius::same(6),
                                    Vec2::new(0.0, 20.0),
                                )
                                .clicked()
                                {
                                    let title = if self.vault_note_title_input.trim().is_empty() {
                                        "Design_Note"
                                    } else {
                                        self.vault_note_title_input.trim()
                                    };
                                    match crate::core::InternalVault::save_markdown_note(
                                        title,
                                        &self.vault_note_body_input,
                                    ) {
                                        Ok(rel) => {
                                            self.vault_feedback = Some(format!("Saved note '{rel}'."));
                                            self.vault_note_composer_open = false;
                                            self.vault_note_title_input.clear();
                                            self.vault_note_body_input.clear();
                                            actions.refresh_vault = true;
                                            self.push_trace(format!("Saved {rel}"), true);
                                        }
                                        Err(err) => {
                                            self.vault_feedback = Some(err);
                                        }
                                    }
                                }
                            });
                    }
                    if let Some((note_path, mut note_body)) = self.previewed_vault_note.clone() {
                        ui.add_space(4.0);
                        Frame::new()
                            .fill(theme.bg_sidebar)
                            .corner_radius(CornerRadius::same(8))
                            .stroke(Stroke::new(0.8, theme.border_subtle))
                            .inner_margin(Margin::same(6))
                            .show(ui, |ui| {
                                ui.set_width((inner_w - 16.0).max(140.0));
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(&note_path)
                                            .size(9.8)
                                            .strong()
                                            .color(theme.accent_blue),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui.small_button("Close").clicked() {
                                            self.previewed_vault_note = None;
                                            self.preview_note_editing = false;
                                        }
                                        if note_path.ends_with(".md")
                                            && ui
                                                .small_button(if self.preview_note_editing {
                                                    "Save Note"
                                                } else {
                                                    "Edit"
                                                })
                                                .clicked()
                                        {
                                            if self.preview_note_editing {
                                                let stem = note_path
                                                    .rsplit('/')
                                                    .next()
                                                    .unwrap_or(&note_path);
                                                if crate::core::InternalVault::save_markdown_note(
                                                    stem,
                                                    &note_body,
                                                )
                                                .is_ok()
                                                {
                                                    self.vault_feedback = Some(format!(
                                                        "Updated Obsidian note '{note_path}'."
                                                    ));
                                                    actions.refresh_vault = true;
                                                }
                                                self.preview_note_editing = false;
                                            } else {
                                                self.preview_note_editing = true;
                                            }
                                        }
                                    });
                                });
                                if self.preview_note_editing {
                                    ui.add_space(2.0);
                                    if ui.small_button("+ Insert Vault Context").clicked() {
                                        note_body.push_str("\n\n- Backlink: [[Engineering Design Reference]]\n");
                                    }
                                    ui.add(
                                        egui::TextEdit::multiline(&mut note_body)
                                            .desired_rows(3)
                                            .desired_width(ui.available_width()),
                                    );
                                    self.previewed_vault_note = Some((note_path, note_body));
                                } else {
                                    ui.label(
                                        RichText::new(&note_body)
                                            .size(9.6)
                                            .color(theme.text_secondary),
                                    );
                                }
                            });
                    }
                    if let Some((old_rel, mut rename_draft)) = self.vault_rename_target.clone() {
                        ui.add_space(3.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Rename:").size(9.5).color(theme.text_muted));
                            ui.add(
                                egui::TextEdit::singleline(&mut rename_draft)
                                    .desired_width((inner_w * 0.5).max(110.0)),
                            );
                            if ui.small_button("Apply").clicked() {
                                match crate::core::InternalVault::rename_vault_file(
                                    &old_rel,
                                    &rename_draft,
                                ) {
                                    Ok(new_rel) => {
                                        self.vault_feedback = Some(format!("Renamed -> {new_rel}"));
                                        self.vault_rename_target = None;
                                        actions.refresh_vault = true;
                                    }
                                    Err(err) => {
                                        self.vault_feedback = Some(err);
                                    }
                                }
                            } else if ui.small_button("Cancel").clicked() {
                                self.vault_rename_target = None;
                            } else {
                                self.vault_rename_target = Some((old_rel, rename_draft));
                            }
                        });
                    }
                    let fq = self.vault_filter_query.trim().to_lowercase();
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
                                if !fq.is_empty()
                                    && !proj.name.to_lowercase().contains(&fq)
                                    && !proj.rel_path.to_lowercase().contains(&fq)
                                {
                                    continue;
                                }
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
                                    if ui.small_button("Ren").on_hover_text("Rename file").clicked() {
                                        self.vault_rename_target = Some((proj.rel_path.clone(), proj.name.clone()));
                                    }
                                    if ui.small_button("Del").on_hover_text("Delete Vault file").clicked()
                                        && crate::core::InternalVault::delete_vault_context_file(&proj.rel_path).is_ok()
                                    {
                                        self.vault_feedback = Some(format!("Deleted {}", proj.name));
                                        actions.refresh_vault = true;
                                    }
                                });
                            }
                            for vf in vault_files.iter().take(16) {
                                if vault_projects.iter().any(|p| p.rel_path == vf.rel_path) {
                                    continue;
                                }
                                if !fq.is_empty()
                                    && !vf.name.to_lowercase().contains(&fq)
                                    && !vf.category.to_lowercase().contains(&fq)
                                {
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
                                    if ui
                                        .button(
                                            RichText::new(format!("{} ({})", vf.name, vf.category))
                                                .size(10.0)
                                                .color(theme.text_secondary),
                                        )
                                        .on_hover_text("Click to preview Obsidian Vault note / file")
                                        .clicked()
                                    {
                                        let preview =
                                            crate::core::InternalVault::read_vault_context_snippet(
                                                &vf.rel_path,
                                                600,
                                            )
                                            .unwrap_or_else(|| "(Binary or empty file)".to_string());
                                        self.previewed_vault_note = Some((vf.rel_path.clone(), preview));
                                    }
                                    if ui.small_button("Ren").on_hover_text("Rename Vault file").clicked() {
                                        self.vault_rename_target = Some((vf.rel_path.clone(), vf.name.clone()));
                                    }
                                    if ui.small_button("Del").on_hover_text("Delete Vault file").clicked()
                                        && crate::core::InternalVault::delete_vault_context_file(&vf.rel_path).is_ok()
                                    {
                                        self.vault_feedback = Some(format!("Deleted {}", vf.name));
                                        actions.refresh_vault = true;
                                    }
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
            .fill(Color32::TRANSPARENT)
            .inner_margin(Margin::symmetric(2, 4))
            .show(ui, |ui| {
                let box_w = (panel_w - 10.0).max(160.0);
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
            (vp.x - 16.0).clamp(220.0, 420.0)
        } else {
            (vp.x * 0.62).clamp(520.0, 740.0)
        };
        let expanded_w = if is_mobile {
            (vp.x - 12.0).clamp(220.0, 440.0)
        } else {
            (vp.x * 0.72).clamp(580.0, 840.0)
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
                    // A. Subtle execution trace badge above the resting Dynamic Island
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

                    // B. ONE UNIFIED GROWING DYNAMIC ISLAND (morphs seamlessly upward when hovered or expanded)
                    Self::antigravity_glass_frame(theme, 20, Margin::symmetric(12, 9)).show(
                        ui,
                        |ui| {
                            let cap_w = (dock_w - 28.0).max(200.0);
                            ui.set_width(cap_w);
                            ui.set_max_width(cap_w);

                    if should_expand {
                    ScrollArea::vertical()
                        .id_salt("dynamic_island_expand_scroll")
                        .max_width(cap_w)
                        .max_height((vp.y * 0.54).clamp(170.0, 440.0))
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                    ui.set_max_width(cap_w);
                    if let Some(SmartSelectionTarget::EarthSite { lat, lon }) = self.selected_target {
                        let site_actions = self.render_earth_site_inspector_strip(
                            ui,
                            theme,
                            lat,
                            lon,
                            cap_w,
                            universal_canvas,
                            build_view,
                        );
                        actions.merge(site_actions);
                        ui.separator();
                    }
                    if self.history_popover_open {
                        Frame::new()
                            .fill(Color32::TRANSPARENT)
                            .inner_margin(Margin::symmetric(2, 4))
                            .show(ui, |ui| {
                                let hw = (cap_w - 10.0).max(160.0);
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
                        ui.separator();
                    }

                    if self.api_key_popover_open {
                        Frame::new()
                            .fill(Color32::TRANSPARENT)
                            .inner_margin(Margin::symmetric(2, 4))
                            .show(ui, |ui| {
                                let kw = (cap_w - 10.0).max(160.0);
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
                                    let key_edit = ui.add(
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
                                        if self.remember_gemini_key {
                                            crate::core::web_bridge::gemini_key_save_device_opt_in(
                                                true,
                                                &self.gemini_api_key_draft,
                                            );
                                        }
                                        self.api_key_popover_open = false;
                                    }
                                    if !self.gemini_api_key_draft.is_empty()
                                        && ui.small_button("Clear").clicked()
                                    {
                                        self.gemini_api_key_draft.clear();
                                        self.agent_model = AgentModelChoice::EigenAiProFem;
                                        crate::core::web_bridge::gemini_key_save_device_opt_in(
                                            false,
                                            "",
                                        );
                                        self.push_trace("Cleared Gemini API key (Offline EigenAI)", true);
                                    }
                                    if self.remember_gemini_key && key_edit.changed() {
                                        crate::core::web_bridge::gemini_key_save_device_opt_in(
                                            true,
                                            &self.gemini_api_key_draft,
                                        );
                                    }
                                });
                                let rem_cb = ui.checkbox(
                                    &mut self.remember_gemini_key,
                                    "Remember key on this device (local storage only, never exported in .eigen)",
                                );
                                if rem_cb.changed() {
                                    crate::core::web_bridge::gemini_key_save_device_opt_in(
                                        self.remember_gemini_key,
                                        &self.gemini_api_key_draft,
                                    );
                                }
                                ui.label(
                                    RichText::new("Used only for live Google Gemini REST calls from Dynamic Island. Empty key automatically falls back to offline EigenAI Pro.")
                                        .size(9.5)
                                        .color(theme.text_muted),
                                );
                            });
                        ui.separator();
                    }

                    if self.vault_drawer_open || self.vault_path_popover_open {
                        let vault_act = self.render_vault_workspace_bar(
                            ui,
                            theme,
                            vault_projects,
                            cap_w,
                        );
                        actions.merge(vault_act);
                        ui.separator();
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
                            (cap_w - 4.0).max(170.0),
                            (vp.y * 0.30).clamp(140.0, 230.0),
                            "antigravity_slash_palette_scroll",
                        );
                        actions.merge(sub);
                        ui.separator();
                    } else if self.context_picker_open {
                        let sub = self.render_context_picker_popover(
                            ui,
                            theme,
                            &build_view.bim_bus.project,
                            vault_projects,
                            cap_w,
                        );
                        actions.merge(sub);
                        ui.separator();
                    }

                    if self.command_bar_expanded && !slash_mode {
                        let chat_h = (vp.y * if is_mobile { 0.22 } else { 0.25 }).clamp(90.0, 210.0);
                        Frame::new()
                            .fill(Color32::TRANSPARENT)
                            .inner_margin(Margin::symmetric(2, 4))
                            .show(ui, |ui| {
                            let chat_w = (cap_w - 10.0).max(170.0);
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
                                                    if idx > 0 {
                                                        ui.horizontal(|ui| {
                                                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                                if ui
                                                                    .small_button("Save Note")
                                                                    .on_hover_text("Save response to Obsidian Vault KnowledgeBase/*.md")
                                                                    .clicked()
                                                                {
                                                                    let title = format!("AI_Audit_{idx}");
                                                                    if let Ok(rel) = crate::core::InternalVault::save_markdown_note(&title, &msg.message) {
                                                                        self.vault_feedback = Some(format!("Saved AI reply -> {rel}"));
                                                                        actions.refresh_vault = true;
                                                                    }
                                                                }
                                                            });
                                                        });
                                                    }
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
                                                ui.horizontal(|ui| {
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
                                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                        if ui.small_button("Cancel").clicked() {
                                                            self.gemini_inflight_req_id = None;
                                                            self.push_trace("Cancelled Gemini request", false);
                                                        }
                                                    });
                                                });
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
                        ui.separator();
                    }

                    // Hover-Only Workspace Extension Strip inside the SAME growing Dynamic Island
                            Frame::new()
                                .fill(Color32::TRANSPARENT)
                                .inner_margin(Margin::symmetric(2, 4))
                                .show(
                                ui,
                                |ui| {
                            let deck_w = (cap_w - 10.0).max(170.0);
                            ui.set_width(deck_w);
                            ui.set_max_width(deck_w);

                            let can_undo = !build_view.bim_bus.undo_stack.is_empty()
                                || !universal_canvas.undo_stack.is_empty();
                            let can_redo = !build_view.bim_bus.redo_stack.is_empty()
                                || !universal_canvas.redo_stack.is_empty();

                            ui.horizontal_wrapped(|ui| {
                                if icons::icon_button(
                                    ui,
                                    AppIcon::Undo,
                                    "Undo",
                                    9.8,
                                    if can_undo {
                                        theme.text_primary
                                    } else {
                                        theme.text_muted
                                    },
                                    theme.bg_card,
                                    Stroke::new(0.8, theme.border_subtle),
                                    CornerRadius::same(7),
                                    Vec2::new(0.0, 21.0),
                                )
                                .on_hover_text("Undo last BIM or 2D canvas action (Cmd/Ctrl+Z)")
                                .clicked()
                                    && can_undo
                                {
                                    actions.undo = true;
                                }
                                if icons::icon_button(
                                    ui,
                                    AppIcon::Redo,
                                    "Redo",
                                    9.8,
                                    if can_redo {
                                        theme.text_primary
                                    } else {
                                        theme.text_muted
                                    },
                                    theme.bg_card,
                                    Stroke::new(0.8, theme.border_subtle),
                                    CornerRadius::same(7),
                                    Vec2::new(0.0, 21.0),
                                )
                                .on_hover_text("Redo undone BIM or 2D canvas action (Cmd/Ctrl+Y)")
                                .clicked()
                                    && can_redo
                                {
                                    actions.redo = true;
                                }

                                let view_lbl = self.active_canvas_mode.label();
                                if icons::icon_button(
                                    ui,
                                    self.active_canvas_mode.icon(),
                                    view_lbl,
                                    9.8,
                                    theme.accent_blue,
                                    theme.accent_blue.gamma_multiply(0.14),
                                    Stroke::new(0.8, theme.accent_blue.gamma_multiply(0.4)),
                                    CornerRadius::same(7),
                                    Vec2::new(0.0, 21.0),
                                )
                                .on_hover_text("Switch Canvas View: Earth 3D -> BIM 3D -> Plan 2D")
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
                                    self.push_trace(
                                        format!("View: {}", self.active_canvas_mode.label()),
                                        true,
                                    );
                                }

                                let has_key = !self.gemini_api_key_draft.trim().is_empty();
                                let needs_key = matches!(
                                    self.agent_model,
                                    AgentModelChoice::Gemini25Flash | AgentModelChoice::Gemini25Pro
                                ) && !has_key;
                                if icons::icon_button(
                                    ui,
                                    AppIcon::Lock,
                                    if has_key { "Key OK" } else { "API Key" },
                                    9.8,
                                    if has_key {
                                        theme.accent_green
                                    } else if needs_key {
                                        theme.accent_orange
                                    } else {
                                        theme.text_secondary
                                    },
                                    theme.bg_card,
                                    Stroke::new(0.8, theme.border_subtle),
                                    CornerRadius::same(7),
                                    Vec2::new(0.0, 21.0),
                                )
                                .clicked()
                                {
                                    self.api_key_popover_open = !self.api_key_popover_open;
                                    self.history_popover_open = false;
                                }

                                if icons::icon_button(
                                    ui,
                                    AppIcon::SettingsGear,
                                    "Settings",
                                    9.8,
                                    theme.text_primary,
                                    theme.bg_card,
                                    Stroke::new(0.8, theme.border_subtle),
                                    CornerRadius::same(7),
                                    Vec2::new(0.0, 21.0),
                                )
                                .on_hover_text("Open Workspace Settings")
                                .clicked()
                                {
                                    actions.open_settings = true;
                                }

                                if icons::icon_button(
                                    ui,
                                    AppIcon::MagnetSnap,
                                    if universal_canvas.ortho_mode { "Ortho ON" } else { "Ortho" },
                                    9.8,
                                    if universal_canvas.ortho_mode {
                                        Color32::WHITE
                                    } else {
                                        theme.text_secondary
                                    },
                                    if universal_canvas.ortho_mode {
                                        theme.accent_blue
                                    } else {
                                        theme.bg_card
                                    },
                                    Stroke::new(0.8, theme.border_subtle),
                                    CornerRadius::same(7),
                                    Vec2::new(0.0, 21.0),
                                )
                                .clicked()
                                {
                                    universal_canvas.ortho_mode = !universal_canvas.ortho_mode;
                                }

                                let grid_on = build_view.show_grid;
                                if icons::icon_button(
                                    ui,
                                    AppIcon::Grid,
                                    if grid_on { "Grid ON" } else { "Grid OFF" },
                                    9.8,
                                    if grid_on {
                                        Color32::WHITE
                                    } else {
                                        theme.text_secondary
                                    },
                                    if grid_on {
                                        theme.accent_blue
                                    } else {
                                        theme.bg_card
                                    },
                                    Stroke::new(0.8, theme.border_subtle),
                                    CornerRadius::same(7),
                                    Vec2::new(0.0, 21.0),
                                )
                                .on_hover_text("Toggle structural & 2D CAD grid lines")
                                .clicked()
                                {
                                    let next_grid = !build_view.show_grid;
                                    build_view.show_grid = next_grid;
                                    universal_canvas.show_grid = next_grid;
                                }

                                let map_on = build_view.show_3d_map_backdrop;
                                if icons::icon_button(
                                    ui,
                                    AppIcon::Globe,
                                    if map_on { "Map BG ON" } else { "Map BG OFF" },
                                    9.8,
                                    if map_on {
                                        Color32::WHITE
                                    } else {
                                        theme.text_secondary
                                    },
                                    if map_on {
                                        theme.accent_green
                                    } else {
                                        theme.bg_card
                                    },
                                    Stroke::new(0.8, theme.border_subtle),
                                    CornerRadius::same(7),
                                    Vec2::new(0.0, 21.0),
                                )
                                .on_hover_text("Toggle geodetic Earth backdrop beneath 3D BIM model")
                                .clicked()
                                {
                                    build_view.show_3d_map_backdrop = !build_view.show_3d_map_backdrop;
                                }

                                for (lens, lbl) in [
                                    (SpatialLens::Clean, "Clean"),
                                    (SpatialLens::DcrHeatmap, "DCR%"),
                                    (SpatialLens::AriRedundancy, "ARI"),
                                    (SpatialLens::BendingMoment, "Moment"),
                                    (SpatialLens::Erection4D, "4D"),
                                ] {
                                    let active_l = self.active_lens == lens;
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(lbl).size(9.8).color(if active_l {
                                                    Color32::WHITE
                                                } else {
                                                    theme.text_secondary
                                                }),
                                            )
                                            .min_size(Vec2::new(0.0, 21.0))
                                            .fill(if active_l {
                                                theme.accent_blue
                                            } else {
                                                theme.bg_card
                                            })
                                            .stroke(Stroke::new(0.8, theme.border_subtle))
                                            .corner_radius(CornerRadius::same(7)),
                                        )
                                        .clicked()
                                    {
                                        self.apply_lens_to_views(lens, build_view, universal_canvas);
                                    }
                                }

                                if ui.small_button("Iso").on_hover_text("3D Isometric Camera").clicked() {
                                    self.apply_camera_preset(build_view, "iso");
                                    actions.fit_camera = true;
                                }
                                if ui.small_button("Top").on_hover_text("3D Plan Top Camera").clicked() {
                                    self.apply_camera_preset(build_view, "top");
                                    actions.fit_camera = true;
                                }
                                if ui.small_button("Front").on_hover_text("3D Elevation Camera").clicked() {
                                    self.apply_camera_preset(build_view, "front");
                                    actions.fit_camera = true;
                                }
                                let (_, _, story_lbl, lvl_cnt) =
                                    self.active_base_and_top_levels(&build_view.bim_bus.project);
                                if ui
                                    .small_button(format!("Story: {story_lbl}"))
                                    .on_hover_text("Cycle active placement floor level")
                                    .clicked()
                                {
                                    self.active_story_idx = (self.active_story_idx + 1) % lvl_cnt.max(1);
                                }
                                if ui
                                    .small_button("+ Story")
                                    .on_hover_text("Add upper floor story (+3.5m)")
                                    .clicked()
                                    && self.add_story_above_top(build_view).is_ok()
                                {
                                    actions.sync_fem_and_earth = true;
                                    actions.fit_camera = true;
                                }
                                if ui
                                    .small_button("+ Slab")
                                    .on_hover_text("Add structural concrete floor slab")
                                    .clicked()
                                    && self.add_slab_at_active_story(build_view).is_ok()
                                {
                                    actions.sync_fem_and_earth = true;
                                }
                                if ui.small_button("Fit").clicked() {
                                    actions.fit_camera = true;
                                }
                                if ui.small_button("Export").clicked() {
                                    actions.export_archive = true;
                                }
                            });

                            ui.add_space(4.0);
                            // Extended Drawing & Earth Terraforming Tools ("Alat Gambar")
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new("More Draw:")
                                        .size(9.5)
                                        .strong()
                                        .color(theme.text_muted),
                                );
                                for ext_tool in [
                                    UniversalCanvasTool::Freehand,
                                    UniversalCanvasTool::Circle,
                                    UniversalCanvasTool::Dimension,
                                    UniversalCanvasTool::Callout,
                                    UniversalCanvasTool::BoxSelect,
                                    UniversalCanvasTool::Eraser,
                                    UniversalCanvasTool::EarthUplift,
                                    UniversalCanvasTool::EarthCarve,
                                    UniversalCanvasTool::EarthSeawall,
                                    UniversalCanvasTool::EarthBridge,
                                    UniversalCanvasTool::EarthCorridor,
                                ] {
                                    let is_act = build_view.ground_author_tool == Ground3DAuthorTool::Orbit
                                        && universal_canvas.active_tool == ext_tool;
                                    if icons::icon_button(
                                        ui,
                                        ext_tool.app_icon(),
                                        ext_tool.short_label(),
                                        9.5,
                                        if is_act { Color32::WHITE } else { theme.text_secondary },
                                        if is_act {
                                            theme.accent_orange
                                        } else {
                                            theme.bg_card
                                        },
                                        Stroke::new(0.7, theme.border_subtle),
                                        CornerRadius::same(7),
                                        Vec2::new(0.0, 20.0),
                                    )
                                    .on_hover_text(ext_tool.tooltip())
                                    .clicked()
                                    {
                                        if ext_tool.is_earth_tool() {
                                            self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                                            universal_canvas.globe_mode = true;
                                        } else if self.active_canvas_mode == SpatialCanvasMode::Bim3D {
                                            self.active_canvas_mode = SpatialCanvasMode::EarthPlan2D;
                                            universal_canvas.globe_mode = false;
                                        }
                                        build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                                        universal_canvas.active_tool = ext_tool;
                                        self.push_trace(
                                            format!("Tool: {}", ext_tool.short_label()),
                                            true,
                                        );
                                    }
                                }
                            });
                        },
                    );
                        });
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                    }

                            // Resting 3-Row Dynamic Island Base Dock (seamlessly inside ONE growing capsule)
                            // Row 1: Vault (`+ Add Folder`), `+ Add Context`, AI Model/Mode, Chat & Mobile Deck Toggle
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

                                if is_mobile && !should_expand {
                                    if icons::icon_button(
                                        ui,
                                        AppIcon::Layers,
                                        "Deck",
                                        9.8,
                                        theme.accent_blue,
                                        theme.accent_blue.gamma_multiply(0.14),
                                        Stroke::new(0.8, theme.accent_blue.gamma_multiply(0.42)),
                                        CornerRadius::same(8),
                                        Vec2::new(0.0, 22.0),
                                    )
                                    .on_hover_text("Expand Dynamic Island controls for touch devices")
                                    .clicked()
                                    {
                                        self.mobile_deck_pinned = true;
                                    }
                                }

                                if should_expand
                                    && (is_mobile
                                        || self.command_bar_expanded
                                        || self.context_picker_open
                                        || self.vault_drawer_open
                                        || self.mobile_deck_pinned)
                                {
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
        self.apply_site_seismic |= other.apply_site_seismic;
        self.undo |= other.undo;
        self.redo |= other.redo;
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

    #[test]
    fn test_phase50_dynamic_island_no_top_left_capsule_and_zero_overflow() {
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        for win_w in [1100.0_f32, 390.0] {
            let mut spatial = SpatialWorkspaceState::default();
            spatial.command_bar_expanded = true;
            spatial.api_key_popover_open = true;
            let mut canvas = UniversalCanvasState::default();
            let mut build = BuildView::default();
            let mut ai = AiAssistantView::default();
            let members = Vec::new();
            let projects = Vec::new();
            let mut db_components = Vec::new();
            let ctx = egui::Context::default();

            for _ in 0..6 {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 800.0))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(input, |ui| {
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

            // Top-left corner has NO capsule in Clean + Orbit mode; only Dynamic Island rect is emitted
            assert_eq!(spatial.hud_rects.len(), 1);
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0 && r.bottom() <= 801.0,
                "Dynamic Island {r:?} overflowed viewport width {win_w}"
            );
        }
    }

    #[test]
    fn test_phase50_gemini_multiturn_and_extended_directives() {
        let payload = crate::core::web_bridge::build_gemini_generate_content_payload(
            "System context",
            &[
                (true, "Audit column 1".to_string()),
                (false, "Column 1 DCR is 88%".to_string()),
                (true, "Resize column 1 to 500x500".to_string()),
            ],
        );
        let contents = payload["contents"].as_array().expect("contents array");
        assert_eq!(contents.len(), 3);
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[1]["role"], "model");
        assert_eq!(contents[2]["role"], "user");

        let raw = "Upsizing column C1 to 500x500 mm and adding seismic sync. [[RESIZE_COL:1,500,500]] [[ADD_COL:6000,3000,450,450]] [[APPLY_SITE_SEISMIC]]";
        let (clean, dirs) = parse_gemini_action_directives(raw);
        assert_eq!(
            clean,
            "Upsizing column C1 to 500x500 mm and adding seismic sync."
        );
        assert_eq!(dirs.len(), 3);
        assert_eq!(
            dirs[0],
            GeminiDirective::ResizeColumn {
                id: 1,
                w_mm: 500.0,
                d_mm: 500.0
            }
        );
        assert_eq!(
            dirs[1],
            GeminiDirective::AddColumn {
                x_mm: 6000.0,
                y_mm: 3000.0,
                w_mm: 450.0,
                d_mm: 450.0
            }
        );
        assert_eq!(dirs[2], GeminiDirective::ApplySiteSeismic);
    }

    #[test]
    fn test_phase50_vault_markdown_note_and_optin_key_safety() {
        let temp_vault = std::env::temp_dir().join(format!(
            "eigengraph_phase50_vault_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_vault);
        let rel = crate::core::InternalVault::save_markdown_note_at(
            &temp_vault,
            "Seismic Criteria",
            "# Jakarta Zone\n- Ss = 0.85 g\n",
        )
        .expect("Markdown note should save inside KnowledgeBase/");
        assert_eq!(rel, "KnowledgeBase/Seismic_Criteria.md");
        let saved_md = std::fs::read_to_string(temp_vault.join(&rel)).expect("Note readable");
        assert!(saved_md.contains("Jakarta Zone"));
        let _ = std::fs::remove_dir_all(&temp_vault);

        let key_path = std::env::temp_dir().join(format!(
            "eigengraph_phase50_optin_{}.json",
            std::process::id()
        ));
        crate::core::web_bridge::gemini_key_save_device_opt_in_at(
            &key_path,
            true,
            "AIzaSecretLocalOnly",
        );
        let (rem, loaded) = crate::core::web_bridge::gemini_key_load_device_opt_in_at(&key_path);
        assert!(rem);
        assert_eq!(loaded, "AIzaSecretLocalOnly");

        // Opt-out deletes file cleanly
        crate::core::web_bridge::gemini_key_save_device_opt_in_at(&key_path, false, "");
        let (rem2, loaded2) = crate::core::web_bridge::gemini_key_load_device_opt_in_at(&key_path);
        assert!(!rem2);
        assert!(loaded2.is_empty());

        // Verify UnifiedWorkspaceArchive / WorkspaceCanvasSettings never serialize any API key
        let mut settings = crate::views::settings::SettingsView::default();
        settings.gemini_api_key = "AIzaSecretLocalOnly".to_string();
        let canvas = UniversalCanvasState::default();
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ws = settings.workspace_settings_snapshot(&theme, &canvas, false);
        let ws_json = serde_json::to_string(&ws).unwrap_or_default();
        assert!(!ws_json.contains("gemini_api_key"));
        assert!(!ws_json.contains("AIzaSecretLocalOnly"));
    }

    #[test]
    fn test_phase51_gemini_model_ids_undo_redo_and_hover_deck_expansion() {
        assert_eq!(
            AgentModelChoice::EigenAiProFem.gemini_model_id(),
            ""
        );
        assert_eq!(
            AgentModelChoice::Gemini25Flash.gemini_model_id(),
            "gemini-2.5-flash"
        );
        assert_eq!(
            AgentModelChoice::Gemini25Pro.gemini_model_id(),
            "gemini-2.5-pro"
        );
        assert_eq!(AgentModelChoice::Gemini25Flash.next(), AgentModelChoice::Gemini25Pro);
        assert_eq!(AgentModelChoice::Gemini25Pro.next(), AgentModelChoice::EigenAiProFem);

        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();

        // Resting screenshot state renders 3-row dock at bottom-center without hover extension
        for _ in 0..4 {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1100.0, 820.0))),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
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
        assert!(!spatial.island_expanded_prev);
        let rest_rect = spatial.dynamic_island_rect;

        // Hover over resting Dynamic Island -> expands upward to expose Hover Extension Deck + Extended Draw
        let mut hover_rect = Rect::NOTHING;
        for _ in 0..6 {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1100.0, 820.0))),
                events: vec![egui::Event::PointerMoved(rest_rect.center())],
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
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
            hover_rect = spatial.dynamic_island_rect;
        }
        assert!(spatial.island_expanded_prev);
        assert!(hover_rect.height() > rest_rect.height());
    }

    #[test]
    fn test_phase52_single_dynamic_island_frame_and_mobile_toggle() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();

        // Mobile viewport (< 640px): starts compact, expands smoothly in ONE single outer capsule when `mobile_deck_pinned` is toggled
        let mut rest_mobile = Rect::NOTHING;
        for _ in 0..4 {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(390.0, 844.0))),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
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
            rest_mobile = spatial.dynamic_island_rect;
        }
        assert!(!spatial.island_expanded_prev);
        assert!(rest_mobile.width() <= 386.0);

        spatial.mobile_deck_pinned = true;
        assert!(spatial.is_dynamic_island_expanded(&ctx));
        let mut exp_mobile = Rect::NOTHING;
        for _ in 0..5 {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(390.0, 844.0))),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
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
            exp_mobile = spatial.dynamic_island_rect;
        }
        assert!(spatial.island_expanded_prev);
        assert!(exp_mobile.height() > rest_mobile.height());
        assert!(exp_mobile.width() <= 386.0);

        spatial.collapse_dynamic_island();
        assert!(!spatial.mobile_deck_pinned);
    }

    #[test]
    fn test_phase52_sketch_entity_edit_delete_and_open_sheet_directive() {
        let mut canvas = UniversalCanvasState::default();
        let initial_cnt = canvas.entities.len();
        let line_id = canvas.commit_parametric_2d_line(6.0, 0.0, "2D", "Drafting");
        assert_eq!(canvas.entities.len(), initial_cnt + 1);
        let (_, len_m, _, _, _, _, _) = canvas.entity_summary(line_id).expect("line summary");
        assert!((len_m - 6.0).abs() < 1e-6);

        let rect_id = canvas.commit_parametric_2d_rect(8.0, 5.0, "2D", "Drafting");
        let (_, _, area_m2, _, _, _, _) = canvas.entity_summary(rect_id).expect("rect summary");
        assert!((area_m2 - 40.0).abs() < 1e-6);

        let circ_id = canvas.commit_parametric_2d_circle(3.0, "2D", "Drafting");
        let (_, _, _, rad_m, _, _, _) = canvas.entity_summary(circ_id).expect("circle summary");
        assert!((rad_m - 3.0).abs() < 1e-6);

        assert!(canvas.update_entity_style(line_id, [16, 185, 129], 4.5));
        let (_, _, _, _, _, col, sw) = canvas.entity_summary(line_id).unwrap();
        assert_eq!(col, [16, 185, 129]);
        assert!((sw - 4.5).abs() < 1e-4);

        let dup_id = canvas.duplicate_offset_entity(line_id, 500.0, 500.0).expect("duplicate");
        assert_ne!(dup_id, line_id);
        assert!(canvas.delete_entity_by_id(dup_id));
        assert!(canvas.entity_summary(dup_id).is_none());
        canvas.undo();
        assert!(canvas.entity_summary(dup_id).is_some());

        let (clean, directives) = parse_gemini_action_directives(
            "Open the phased schedule and FEM checks. [[OPEN_SHEET:Scheduling]] [[OPEN_SHEET:Analysis]]",
        );
        assert_eq!(clean, "Open the phased schedule and FEM checks.");
        assert_eq!(
            directives,
            vec![
                GeminiDirective::OpenSheet(NavigationPillar::TimeScheduling),
                GeminiDirective::OpenSheet(NavigationPillar::AnalysisSimulation),
            ]
        );

        let mut spatial = SpatialWorkspaceState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let _ = spatial.apply_proposal_actions(
            &[PlannedProposalAction::OpenSheet(NavigationPillar::TimeScheduling)],
            &mut canvas,
            &mut build,
            &mut ai,
        );
        assert_eq!(spatial.active_overlay_pillar, Some(NavigationPillar::TimeScheduling));
    }

    #[test]
    fn test_phase52_gemini_pro_payload_and_escape_reset() {
        let history = vec![(true, "Inspect buckling safety factor.".to_string())];
        let pro_payload = crate::core::web_bridge::build_gemini_generate_content_payload_for_model(
            "Sys",
            &history,
            "gemini-2.5-pro",
        );
        assert_eq!(pro_payload["generationConfig"]["maxOutputTokens"], 2400);

        let flash_payload =
            crate::core::web_bridge::build_gemini_generate_content_payload_for_model(
                "Sys",
                &history,
                "gemini-2.5-flash",
            );
        assert_eq!(flash_payload["generationConfig"]["maxOutputTokens"], 1400);

        let mut spatial = SpatialWorkspaceState::default();
        let mut build = BuildView::default();
        let mut canvas = UniversalCanvasState::default();
        assert!(build.show_3d_map_backdrop);

        build.ground_author_tool = Ground3DAuthorTool::Beam;
        build.pending_3d_start_mm = Some([1000.0, 2000.0]);
        canvas.active_tool = UniversalCanvasTool::Line;
        canvas.measure_active = true;
        canvas.measure_points_lat_lon.push((-6.2, 106.8));

        assert!(spatial.has_active_manual_tool(&build, &canvas));
        spatial.reset_manual_tools_to_orbit(&mut build, &mut canvas);
        assert!(!spatial.has_active_manual_tool(&build, &canvas));
        assert_eq!(build.ground_author_tool, Ground3DAuthorTool::Orbit);
        assert!(build.pending_3d_start_mm.is_none());
        assert_eq!(canvas.active_tool, UniversalCanvasTool::SelectPan);
        assert!(!canvas.measure_active);
        canvas.clear_measure();
        assert!(canvas.measure_points_lat_lon.is_empty());
    }

    #[test]
    fn test_phase53_earth_site_in_dynamic_island_no_top_right_popup_and_zero_overflow() {
        let mut spatial = SpatialWorkspaceState::default();
        spatial.selected_target = Some(SmartSelectionTarget::EarthSite {
            lat: -6.2088,
            lon: 106.8456,
        });
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);

        for win_w in [1100.0_f32, 390.0] {
            let ctx = egui::Context::default();
            for _ in 0..5 {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 820.0))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(input, |ui| {
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
            // EarthSite expands inside the ONE Dynamic Island (NO separate top-right Smart Pop-Up rect)
            assert_eq!(spatial.hud_rects.len(), 1);
            assert!(spatial.is_dynamic_island_expanded(&ctx));
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0,
                "Dynamic Island with Earth Site Location {r:?} overflowed viewport {win_w}"
            );
        }

        // Collapsing dismisses EarthSite smoothly back to idle pill
        spatial.collapse_dynamic_island();
        assert!(spatial.selected_target.is_none());
    }

    #[test]
    fn test_phase53_lifo_undo_story_slab_camera_and_measure_summary() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut build = BuildView::default();
        let mut canvas = UniversalCanvasState::default();

        // 1. Chronological LIFO Undo/Redo across BIM 3D & 2D Canvas
        spatial.observe_undo_stacks(build.bim_bus.undo_stack.len(), canvas.undo_stack.len());
        let _ = build
            .bim_bus
            .execute_natural_language("portal 2x1 @6m 1floors")
            .unwrap();
        spatial.observe_undo_stacks(build.bim_bus.undo_stack.len(), canvas.undo_stack.len());
        let eid = canvas.commit_parametric_2d_line(6.0, 0.0, "2D", "Drafting");
        spatial.observe_undo_stacks(build.bim_bus.undo_stack.len(), canvas.undo_stack.len());

        assert_eq!(
            spatial.pop_undo_domain(build.bim_bus.undo_stack.len(), canvas.undo_stack.len()),
            Some(SpatialUndoDomain::Canvas2D)
        );
        assert!(canvas.undo());
        assert!(canvas.entity_summary(eid).is_none());
        assert_eq!(
            spatial.pop_undo_domain(build.bim_bus.undo_stack.len(), canvas.undo_stack.len()),
            Some(SpatialUndoDomain::Bim3D)
        );

        // 2. Story increment, Slab authoring, Slab selection mention, & Camera presets
        assert!(spatial.add_story_above_top(&mut build).is_ok());
        assert!(spatial.add_slab_at_active_story(&mut build).is_ok());
        let slab_id = build.bim_bus.project.slabs.last().unwrap().id;
        spatial.selected_target = Some(SmartSelectionTarget::Slab(slab_id));
        assert!(spatial
            .active_context_mention(&build.bim_bus.project)
            .unwrap()
            .contains("Slab"));

        spatial.apply_camera_preset(&mut build, "top");
        assert!(!build.perspective);
        spatial.apply_camera_preset(&mut build, "iso");
        assert!(build.perspective);

        // 3. Geodesic measure summary & local action routing
        canvas.measure_active = true;
        canvas.measure_points_lat_lon.push((-6.20, 106.84));
        canvas.measure_points_lat_lon.push((-6.21, 106.85));
        let (dist_m, _, pts_n) = canvas.geodesic_measure_summary();
        assert_eq!(pts_n, 2);
        assert!(dist_m > 1000.0);

        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let _ = spatial.execute_spatial_input(
            "open scheduling",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(
            spatial.active_overlay_pillar == Some(NavigationPillar::TimeScheduling)
                || !spatial.pending_proposals.is_empty()
        );
    }

    #[test]
    fn test_phase53_vault_subfolder_rename_and_delete() {
        let _guard = crate::core::vault::TEST_VAULT_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let temp_vault = std::env::temp_dir().join(format!(
            "eigengraph_phase53_vault_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_vault);
        crate::core::InternalVault::set_runtime_vault_dir(temp_vault.clone());

        let sub_rel = crate::core::InternalVault::create_vault_subfolder_at(&temp_vault, "Site_Data")
            .expect("Subfolder should create starter note");
        assert_eq!(sub_rel, "Site_Data/_index.md");
        assert!(temp_vault.join("Site_Data/_index.md").exists());

        let renamed = crate::core::InternalVault::rename_vault_file_at(
            &temp_vault,
            "Site_Data/_index.md",
            "Site_Data/Geotech_Overview.md",
        )
        .expect("Should rename file cleanly");
        assert_eq!(renamed, "Site_Data/Geotech_Overview.md");
        assert!(temp_vault.join("Site_Data/Geotech_Overview.md").exists());

        crate::core::InternalVault::delete_vault_context_file_at(
            &temp_vault,
            "Site_Data/Geotech_Overview.md",
        )
        .expect("Should delete file cleanly");
        assert!(!temp_vault.join("Site_Data/Geotech_Overview.md").exists());

        crate::core::InternalVault::reset_runtime_vault_dir();
        let _ = std::fs::remove_dir_all(&temp_vault);
    }
}
