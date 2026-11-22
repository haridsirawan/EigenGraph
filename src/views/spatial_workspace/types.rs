//! Types, slash command catalog, Gemini directive parser, and action flags for Spatial Workspace.

use super::SpatialWorkspaceState;
use crate::core::BimProject;
use crate::icons::AppIcon;
use crate::models::NavigationPillar;
use crate::views::build::Ground3DAuthorTool;
use crate::views::universal_canvas::UniversalCanvasTool;
use crate::views::{BuildView, UniversalCanvasState};

/// Active spatial viewport projection rendered across the full screen (100% Unified Earth 3D Canvas).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpatialCanvasMode {
    #[default]
    EarthGlobe,
}

impl SpatialCanvasMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::EarthGlobe => "Earth 3D",
        }
    }

    pub fn icon(self) -> AppIcon {
        match self {
            Self::EarthGlobe => AppIcon::Globe,
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
    DeformedShape,
    ModalVibration,
    TimeHistoryReplay,
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
            Self::DeformedShape => "Deformed 6-DOF",
            Self::ModalVibration => "Modal Eigenmode",
            Self::TimeHistoryReplay => "EQ Time-History",
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

/// Active category tab inside the Dynamic Island Hover Studio Deck (Phase 61).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DynamicIslandStudioTab {
    #[default]
    ViewAndLens,
    BimAndStory,
    GisAndSite,
    CadAndExport,
}

impl DynamicIslandStudioTab {
    pub const ALL: [Self; 4] = [
        Self::ViewAndLens,
        Self::BimAndStory,
        Self::GisAndSite,
        Self::CadAndExport,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::ViewAndLens => "View & Lens",
            Self::BimAndStory => "BIM & Story",
            Self::GisAndSite => "GIS & Site",
            Self::CadAndExport => "CAD & Export",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::ViewAndLens => "View",
            Self::BimAndStory => "BIM",
            Self::GisAndSite => "GIS",
            Self::CadAndExport => "CAD",
        }
    }

    pub fn icon(self) -> AppIcon {
        match self {
            Self::ViewAndLens => AppIcon::Cube3D,
            Self::BimAndStory => AppIcon::Layers,
            Self::GisAndSite => AppIcon::Globe,
            Self::CadAndExport => AppIcon::RulerSquare,
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
    WorldPortfolio,
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
            Self::WorldPortfolio => "domain:world_portfolio".to_string(),
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
            Self::WorldPortfolio => AppIcon::Layers,
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
            Self::WorldPortfolio => "@World Portfolio".to_string(),
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
    AddStory,
    AddSlab,
    // Phase 59: Live AI Multi-Action Tool-Calling for World Portfolio, Vector Earth, DEM & Neighborhood Coupling
    BatchPinCity {
        query: String,
        city_name: String,
    },
    PinActiveBuilding {
        lat: f64,
        lon: f64,
        heading_deg: f64,
    },
    AuditNeighborhood,
    EnableTerrainContours,
    SwitchPureVector,
    // Phase 63: True 6-DOF Deformed Shape, Modal Eigenmodes & Interactive N-1 Disruption
    ApplyDeformedLens,
    ApplyModalLens(usize),
    SimulateN1(usize),
    RestoreN1,
    // Phase 74: Async Multi-Thread / WebWorker StructGNN-CRC & Ground-Motion Replay
    RunEqReplay(Option<crate::core::time_history::GroundMotionRecord>),
    TogglePlasticHinges,
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
/// (`[[FLY:...]]`, `[[BIM:...]]`, `[[REPAIR]]`, `[[ARI_AUDIT]]`, `[[RESIZE_COL:...]]`, `[[RESIZE_BEAM:...]]`, `[[ADD_COL:...]]`, `[[APPLY_SITE_SEISMIC]]`, `[[OPEN_SHEET:...]]`, `[[ADD_STORY]]`, `[[ADD_SLAB]]`).
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
    AddStory,
    AddSlab,
    // Phase 59: Live AI Multi-Action Tool-Calling for World Portfolio, Vector Earth, DEM & Neighborhood Coupling
    BatchPinCity {
        query: String,
        city_name: String,
    },
    PinActiveBuilding {
        lat: f64,
        lon: f64,
        heading_deg: f64,
    },
    AuditNeighborhood,
    EnableTerrainContours,
    SwitchPureVector,
    // Phase 63: True 6-DOF Deformed Shape, Modal Eigenmodes & Interactive N-1 Disruption
    ApplyDeformedLens,
    ApplyModalLens(usize),
    SimulateN1(usize),
    RestoreN1,
    // Phase 74: Async Multi-Thread / WebWorker StructGNN-CRC & Ground-Motion Replay
    RunEqReplay(Option<crate::core::time_history::GroundMotionRecord>),
    TogglePlasticHinges,
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
            } else if upper == "ADD_STORY" || upper == "ADD_LEVEL" {
                directives.push(GeminiDirective::AddStory);
            } else if upper == "ADD_SLAB" || upper == "ADD_FLOOR_SLAB" {
                directives.push(GeminiDirective::AddSlab);
            } else if upper.starts_with("BATCH_PIN_CITY:") {
                let args: Vec<&str> = tag[15..].split(',').map(|s| s.trim()).collect();
                if args.len() >= 2 && !args[0].is_empty() && !args[1].is_empty() {
                    directives.push(GeminiDirective::BatchPinCity {
                        query: args[0].to_string(),
                        city_name: args[1].to_string(),
                    });
                }
            } else if upper.starts_with("PIN_ACTIVE:") {
                let nums: Vec<f64> = tag[11..]
                    .split(',')
                    .filter_map(|s| s.trim().parse::<f64>().ok())
                    .collect();
                if nums.len() >= 2 {
                    directives.push(GeminiDirective::PinActiveBuilding {
                        lat: nums[0].clamp(-85.0, 85.0),
                        lon: nums[1].clamp(-180.0, 180.0),
                        heading_deg: nums.get(2).copied().unwrap_or(0.0).rem_euclid(360.0),
                    });
                }
            } else if upper == "AUDIT_NEIGHBORHOOD" || upper == "AUDIT_POUNDING" {
                directives.push(GeminiDirective::AuditNeighborhood);
            } else if upper == "TOGGLE_TOPO" || upper == "ENABLE_TOPO" {
                directives.push(GeminiDirective::EnableTerrainContours);
            } else if upper == "TOGGLE_VECTOR" || upper == "PURE_VECTOR" {
                directives.push(GeminiDirective::SwitchPureVector);
            } else if upper == "LENS_DEFORM" || upper == "DEFORMED" {
                directives.push(GeminiDirective::ApplyDeformedLens);
            } else if upper == "LENS_MODAL" || upper.starts_with("LENS_MODAL:") {
                let m = upper
                    .strip_prefix("LENS_MODAL:")
                    .and_then(|s| s.trim().parse::<usize>().ok())
                    .unwrap_or(1)
                    .saturating_sub(1)
                    .min(2);
                directives.push(GeminiDirective::ApplyModalLens(m));
            } else if upper.starts_with("SIMULATE_N1:") {
                if let Ok(fid) = tag[12..].trim().parse::<usize>() {
                    directives.push(GeminiDirective::SimulateN1(fid));
                }
            } else if upper == "RESTORE_N1" {
                directives.push(GeminiDirective::RestoreN1);
            } else if upper == "RUN_EQ_REPLAY" || upper.starts_with("RUN_EQ_REPLAY:") {
                let rec = upper
                    .strip_prefix("RUN_EQ_REPLAY:")
                    .and_then(crate::core::ground_motion_replay::GroundMotionReplayConfig::record_from_arg);
                directives.push(GeminiDirective::RunEqReplay(rec));
            } else if upper == "TOGGLE_HINGES" {
                directives.push(GeminiDirective::TogglePlasticHinges);
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
        description: "Reset focus on 3D Google Earth Globe",
        icon: AppIcon::Globe,
    },
    SlashCommandSpec {
        cmd: "/iso",
        category: "Canvas",
        description: "Set Earth 3D camera to Isometric site tilt (42°)",
        icon: AppIcon::Cube3D,
    },
    SlashCommandSpec {
        cmd: "/top",
        category: "Canvas",
        description: "Set Earth 3D camera to Nadir top-down plan view (0°)",
        icon: AppIcon::Compass,
    },
    SlashCommandSpec {
        cmd: "/fit",
        category: "Canvas",
        description: "Fit Earth 3D camera to active BIM building or site",
        icon: AppIcon::ZoomFit,
    },
    SlashCommandSpec {
        cmd: "/portal 3x2 @6m 2floors",
        category: "Model",
        description: "Synthesize parametric multi-storey structural portal on Earth 3D",
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
        cmd: "/section",
        category: "Lens",
        description: "Toggle 3D horizontal section cut plane (e.g. /section 4.5)",
        icon: AppIcon::RulerSquare,
    },
    SlashCommandSpec {
        cmd: "/clonestory",
        category: "Model",
        description: "Clone active floor columns, beams, walls & slabs one story upward (+3.5m)",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/footings",
        category: "Model",
        description: "Auto-generate foundation footings / pile caps beneath ground columns",
        icon: AppIcon::MapPin,
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
    SlashCommandSpec {
        cmd: "/world-portfolio",
        category: "World",
        description: "Jump Earth 3D to next unlocated Staging or critical Risk building across Vault",
        icon: AppIcon::Globe,
    },
    SlashCommandSpec {
        cmd: "/deform",
        category: "Lens",
        description: "Toggle live 6-DOF elastic deformed shape & inter-story drift lens on Earth 3D",
        icon: AppIcon::Bolt,
    },
    SlashCommandSpec {
        cmd: "/modal",
        category: "Lens",
        description: "Animate 3D dynamic modal eigenmodes (Mode 1–3, natural period T & frequency f)",
        icon: AppIcon::Pulse,
    },
    SlashCommandSpec {
        cmd: "/n1",
        category: "Analysis",
        description: "Simulate N-1 single-member removal on selected or worst critical element",
        icon: AppIcon::Shield,
    },
    SlashCommandSpec {
        cmd: "/restore",
        category: "Analysis",
        description: "Restore intact structure and clear any active N-1 disruption overlay",
        icon: AppIcon::CheckCircle,
    },
    SlashCommandSpec {
        cmd: "/day",
        category: "Earth",
        description: "Set Earth Vector solar lighting to crisp afternoon Daylight (13:30)",
        icon: AppIcon::Sun,
    },
    SlashCommandSpec {
        cmd: "/dusk",
        category: "Earth",
        description: "Set Earth Vector solar lighting to Golden Hour Dusk (17:30)",
        icon: AppIcon::Sun,
    },
    SlashCommandSpec {
        cmd: "/night",
        category: "Earth",
        description: "Set Earth Vector solar lighting to Night Cyber-Twin with glowing facades (22:00)",
        icon: AppIcon::Moon,
    },
    SlashCommandSpec {
        cmd: "/gpu",
        category: "Earth",
        description: "Toggle WebGL2 hardware-accelerated 3D Vector Earth pass vs. CPU vector fallback",
        icon: AppIcon::Globe,
    },
    SlashCommandSpec {
        cmd: "/ifcx",
        category: "Export",
        description: "Export active georeferenced site & BIM model as pure IFCX (IFC 5 JSON ECS)",
        icon: AppIcon::Cube3D,
    },
    SlashCommandSpec {
        cmd: "/earth-ifcx",
        category: "Export",
        description: "Export composed Living Earth Globe (earth_globe.ifcx) + active site layer",
        icon: AppIcon::Globe,
    },
    SlashCommandSpec {
        cmd: "/road",
        category: "City",
        description: "Author pure-vector IFCX IfcRoad multi-lane corridor on Earth 3D (2 clicks)",
        icon: AppIcon::MapPin,
    },
    SlashCommandSpec {
        cmd: "/bridge",
        category: "City",
        description: "Author elevated IFCX IfcBridge deck & piers on Earth 3D (2 clicks)",
        icon: AppIcon::BridgeSpan,
    },
    SlashCommandSpec {
        cmd: "/zone",
        category: "City",
        description: "Stamp IFCX IfcSpatialZone urban parcel with context towers & trees (1 click)",
        icon: AppIcon::Building,
    },
    SlashCommandSpec {
        cmd: "/gradepad",
        category: "City",
        description: "Auto-grade flat foundation pad onto IfcSite TIN terrain & compute Cut/Fill m3",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/seismic",
        category: "Hazard",
        description: "Toggle animated SNI 1726 / ASCE 7 traveling seismic wavefield & pounding simulation",
        icon: AppIcon::Pulse,
    },
    SlashCommandSpec {
        cmd: "/wind",
        category: "Hazard",
        description: "Toggle Venturi urban wind channeling streamlines & speed-up audit",
        icon: AppIcon::Wind,
    },
    SlashCommandSpec {
        cmd: "/flood",
        category: "Hazard",
        description: "Toggle 3D flood / sea-level plane against IfcSite TIN contours (optional meters)",
        icon: AppIcon::Waves,
    },
    SlashCommandSpec {
        cmd: "/hazard",
        category: "Hazard",
        description: "Run coupled District Multi-Hazard audit (seismic + pounding + Venturi + flood)",
        icon: AppIcon::AlertTriangle,
    },
    SlashCommandSpec {
        cmd: "/ai-capture",
        category: "IFCX",
        description: "Stream AI real-building massing .ifcx onto Earth parcel with auto frame inference & ARI triage",
        icon: AppIcon::Sparkles,
    },
    SlashCommandSpec {
        cmd: "/retrofit",
        category: "AI",
        description: "Synthesize automated structural retrofit (BRB, RC jacketing, shear wall) & .ifcx delta",
        icon: AppIcon::Shield,
    },
    SlashCommandSpec {
        cmd: "/brb",
        category: "AI",
        description: "Inject 3D Buckling-Restrained Braces (BRB) across low-ARI structural bays",
        icon: AppIcon::Bolt,
    },
    SlashCommandSpec {
        cmd: "/jacket",
        category: "AI",
        description: "Apply RC column/beam jacketing & foundation footing enlargement",
        icon: AppIcon::Wrench,
    },
    SlashCommandSpec {
        cmd: "/retrofit-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::bim::retrofit_delta .ifcx strengthening layer",
        icon: AppIcon::Cube3D,
    },
    SlashCommandSpec {
        cmd: "/cascade",
        category: "Analysis",
        description: "Run Two-Stage ARI v2 + StructGNN-CRC conformal screening & 3D progressive collapse cascade",
        icon: AppIcon::Bolt,
    },
    SlashCommandSpec {
        cmd: "/crc",
        category: "AI",
        description: "Evaluate Two-Stage ARI v2 + StructGNN-CRC conformal scenario reduction & safety recall",
        icon: AppIcon::Shield,
    },
    SlashCommandSpec {
        cmd: "/cascade-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::analysis::crc_cascade .ifcx conformal overlay",
        icon: AppIcon::Cube3D,
    },
    SlashCommandSpec {
        cmd: "/ssi",
        category: "Geotech",
        description: "Toggle 6-DOF Winkler Soil-Structure Interaction & subsurface borehole audit",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/borehole",
        category: "Geotech",
        description: "Switch borehole stratigraphy preset (soft|liquefy|stiff|rock)",
        icon: AppIcon::Database,
    },
    SlashCommandSpec {
        cmd: "/liquefaction",
        category: "Hazard",
        description: "Toggle Seed-Idriss seismic liquefaction trigger & p-y spring degradation",
        icon: AppIcon::Waves,
    },
    SlashCommandSpec {
        cmd: "/subsurface",
        category: "View",
        description: "Toggle 3D WebGL2 subsurface geological stratigraphy & groundwater X-Ray cutaway",
        icon: AppIcon::SectionCut,
    },
    SlashCommandSpec {
        cmd: "/ssi-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::geotech::stratigraphy & ssi_springs .ifcx overlay",
        icon: AppIcon::Cube3D,
    },
    SlashCommandSpec {
        cmd: "/snapshot",
        category: "IFCX",
        description: "Capture baseline .ifcx snapshot for multi-layer visual diffing",
        icon: AppIcon::Camera,
    },
    SlashCommandSpec {
        cmd: "/diff",
        category: "IFCX",
        description: "Open Multi-Layer IFCX visual diff inspector & 3D color overlay",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/diff-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::ifcx::diff_manifest .ifcx layer",
        icon: AppIcon::Cube3D,
    },
    SlashCommandSpec {
        cmd: "/dossier",
        category: "Export",
        description: "Export publication-grade 100% Pure-Vector SVG engineering calculation dossier",
        icon: AppIcon::Printer,
    },
    SlashCommandSpec {
        cmd: "/portfolio",
        category: "AI",
        description: "Run autonomous multi-building district ARI v2 retrofit Pareto optimization",
        icon: AppIcon::ChartLine,
    },
    SlashCommandSpec {
        cmd: "/pareto",
        category: "AI",
        description: "Open District Portfolio Budget/Carbon Pareto Frontier drawer inside Dynamic Island",
        icon: AppIcon::ChartBar,
    },
    SlashCommandSpec {
        cmd: "/budget",
        category: "AI",
        description: "Set district retrofit budget cap in USD (e.g. /budget 300k)",
        icon: AppIcon::Coins,
    },
    SlashCommandSpec {
        cmd: "/portfolio-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::portfolio::retrofit_plan .ifcx district overlay",
        icon: AppIcon::Cube3D,
    },
    SlashCommandSpec {
        cmd: "/replay",
        category: "AI",
        description: "Run background async StructGNN-CRC & live 60 FPS earthquake ground-motion replay",
        icon: AppIcon::Bolt,
    },
    SlashCommandSpec {
        cmd: "/record",
        category: "AI",
        description: "Switch benchmark earthquake record (northridge, kobe, elcentro, chichi, sumatra)",
        icon: AppIcon::ChartLine,
    },
    SlashCommandSpec {
        cmd: "/hinges",
        category: "View",
        description: "Toggle 3D ASCE 41-17 plastic hinge beacons and jump to peak seismic drift",
        icon: AppIcon::Target,
    },
    SlashCommandSpec {
        cmd: "/replay-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::analysis::time_history_replay .ifcx overlay",
        icon: AppIcon::Cube3D,
    },
];

/// Side-effect requests produced by the Antigravity Spatial HUD for `EigenGraphApp` to commit.
#[derive(Debug, Default)]
pub struct SpatialHudActions {
    pub sync_fem_and_earth: bool,
    pub fit_camera: bool,
    pub export_archive: bool,
    pub export_ifcx: bool,
    pub export_earth_ifcx: bool,
    pub save_audit_note: bool,
    pub toggle_theme: bool,
    pub open_settings: bool,
    pub switch_to_classic_ui: bool,
    pub toggle_n1_member: Option<usize>,
    pub restore_n1_simulation: bool,
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
    pub fly_to_next_world_building: Option<crate::core::gis::WorldPortfolioFilter>,
    pub rotate_active_world_heading_deg: Option<f64>,
    pub undo: bool,
    pub redo: bool,
    pub run_district_hazard_audit: bool,
    pub ingest_ai_real_building_capture: bool,
    pub run_structural_retrofit: Option<crate::core::retrofit::RetrofitStrategy>,
    pub toggle_retrofit_before_after: bool,
    pub export_retrofit_delta_ifcx: bool,
    pub run_crc_cascade_screening: Option<f64>,
    pub export_crc_cascade_ifcx: bool,
    pub run_subsurface_ssi_sync: bool,
    pub export_subsurface_ssi_ifcx: bool,
    pub capture_ifcx_baseline: bool,
    pub run_ifcx_visual_diff: bool,
    pub export_ifcx_diff_manifest: bool,
    pub export_vector_dossier_svg: bool,
    pub run_portfolio_retrofit: bool,
    pub export_portfolio_retrofit_ifcx: bool,
    pub run_async_ground_motion_replay: bool,
    pub export_ground_motion_replay_ifcx: bool,
}

/// Manual CAD/BIM/Earth drawing & measurement commands surfaced directly inside the Dynamic Island.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialManualTool {
    OrbitPan,
    Column,
    Beam,
    Wall,
    Footing,
    Slab,
    Door,
    Window,
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
        Self::Slab,
        Self::Door,
        Self::Window,
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
            Self::Slab => "Slab",
            Self::Door => "Door",
            Self::Window => "Window",
            Self::Line2D => "Line",
            Self::Rect2D => "Rect",
            Self::Measure => "Measure",
            Self::Terraform => "Terraform",
        }
    }

    /// Compact single-line button label used inside the Dynamic Island Quick-Draw Strip.
    pub fn short_label(self) -> &'static str {
        match self {
            Self::OrbitPan => "Orbit",
            Self::Column => "Col",
            Self::Beam => "Beam",
            Self::Wall => "Wall",
            Self::Footing => "Foot",
            Self::Slab => "Slab",
            Self::Door => "Door",
            Self::Window => "Win",
            Self::Line2D => "Line",
            Self::Rect2D => "Rect",
            Self::Measure => "Meas",
            Self::Terraform => "Earth",
        }
    }

    pub fn tooltip(self) -> &'static str {
        match self {
            Self::OrbitPan => "Orbit Earth / Navigate 3D scene & select elements",
            Self::Column => "Place manual structural Column directly on Earth 3D",
            Self::Beam => "Draw manual structural Beam (click start & end on Earth 3D)",
            Self::Wall => "Draw manual shear/partition Wall (click start & end on Earth 3D)",
            Self::Footing => "Place manual isolated Footing on ground origin",
            Self::Slab => "Draw rectangular floor/roof Slab on active Story plane (2 corner clicks)",
            Self::Door => "Insert CSG Single Door (900x2100mm) by clicking any 3D Wall",
            Self::Window => "Insert CSG Window (1800x1400mm) by clicking any 3D Wall",
            Self::Line2D => "Draw 2D vector CAD line segment on Earth 3D",
            Self::Rect2D => "Draw 2D rectangular boundary / slab outline on Earth 3D",
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
            Self::Slab => AppIcon::Layers,
            Self::Door => AppIcon::Wrench,
            Self::Window => AppIcon::Grid,
            Self::Line2D => AppIcon::LineSeg,
            Self::Rect2D => AppIcon::RectShape,
            Self::Measure => AppIcon::DimensionRuler,
            Self::Terraform => AppIcon::Mountain,
        }
    }

    pub fn is_active(
        self,
        _canvas_mode: SpatialCanvasMode,
        universal_canvas: &UniversalCanvasState,
        build_view: &BuildView,
    ) -> bool {
        use crate::views::universal_canvas::EarthBimTool;
        match self {
            Self::OrbitPan => {
                !universal_canvas.measure_active
                    && universal_canvas.earth_bim_author.active_tool == EarthBimTool::None
                    && build_view.ground_author_tool == Ground3DAuthorTool::Orbit
                    && universal_canvas.active_tool == UniversalCanvasTool::SelectPan
            }
            Self::Column => {
                universal_canvas.earth_bim_author.active_tool == EarthBimTool::Column
                    || build_view.ground_author_tool == Ground3DAuthorTool::Column
            }
            Self::Beam => {
                universal_canvas.earth_bim_author.active_tool == EarthBimTool::Beam
                    || build_view.ground_author_tool == Ground3DAuthorTool::Beam
            }
            Self::Wall => {
                universal_canvas.earth_bim_author.active_tool == EarthBimTool::Wall
                    || build_view.ground_author_tool == Ground3DAuthorTool::Wall
            }
            Self::Footing => {
                universal_canvas.earth_bim_author.active_tool == EarthBimTool::Footing
                    || build_view.ground_author_tool == Ground3DAuthorTool::Footing
            }
            Self::Slab => universal_canvas.earth_bim_author.active_tool == EarthBimTool::Slab,
            Self::Door => universal_canvas.earth_bim_author.active_tool == EarthBimTool::Door,
            Self::Window => universal_canvas.earth_bim_author.active_tool == EarthBimTool::Window,
            Self::Line2D => {
                universal_canvas.earth_bim_author.active_tool == EarthBimTool::None
                    && universal_canvas.active_tool == UniversalCanvasTool::Line
            }
            Self::Rect2D => {
                universal_canvas.earth_bim_author.active_tool == EarthBimTool::None
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
        use crate::views::universal_canvas::EarthBimTool;
        let actions = SpatialHudActions::default();
        spatial.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
        universal_canvas.globe_mode = true;
        universal_canvas.measure_active = false;
        let zoom_in_if_high_altitude = |uc: &mut UniversalCanvasState| {
            if uc.world_map.zoom_level < 17.0 {
                let target = if uc.ground_origin_anchored {
                    uc.ground_origin_lat_lon
                } else {
                    uc.world_map.center_lat_lon
                };
                uc.ground_origin_lat_lon = target;
                uc.ground_origin_anchored = true;
                uc.earth_world.camera.tilt_deg = uc.earth_world.camera.tilt_deg.clamp(12.0, 48.0);
                uc.earth_world.start_fly_to(&uc.world_map, target.0, target.1, 18.2, "Earth 3D Draw");
            }
        };
        match self {
            Self::OrbitPan => {
                build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                universal_canvas.earth_bim_author.active_tool = EarthBimTool::None;
                universal_canvas.earth_bim_author.pending_start_mm = None;
                universal_canvas.active_tool = UniversalCanvasTool::SelectPan;
                spatial.push_trace("Earth 3D Orbit & Select mode", true);
            }
            Self::Column => {
                build_view.ground_author_tool = Ground3DAuthorTool::Column;
                universal_canvas.earth_bim_author.active_tool = EarthBimTool::Column;
                universal_canvas.earth_bim_author.pending_start_mm = None;
                universal_canvas.active_tool = UniversalCanvasTool::SelectPan;
                universal_canvas.ground_origin_anchored = true;
                zoom_in_if_high_altitude(universal_canvas);
                spatial.push_trace("Draw Column directly on Earth 3D (click point)", true);
            }
            Self::Beam => {
                build_view.ground_author_tool = Ground3DAuthorTool::Beam;
                universal_canvas.earth_bim_author.active_tool = EarthBimTool::Beam;
                universal_canvas.earth_bim_author.pending_start_mm = None;
                universal_canvas.active_tool = UniversalCanvasTool::SelectPan;
                universal_canvas.ground_origin_anchored = true;
                zoom_in_if_high_altitude(universal_canvas);
                spatial.push_trace("Draw Beam directly on Earth 3D (click 2 pts)", true);
            }
            Self::Wall => {
                build_view.ground_author_tool = Ground3DAuthorTool::Wall;
                universal_canvas.earth_bim_author.active_tool = EarthBimTool::Wall;
                universal_canvas.earth_bim_author.pending_start_mm = None;
                universal_canvas.active_tool = UniversalCanvasTool::SelectPan;
                universal_canvas.ground_origin_anchored = true;
                zoom_in_if_high_altitude(universal_canvas);
                spatial.push_trace("Draw Wall directly on Earth 3D (click 2 pts)", true);
            }
            Self::Footing => {
                build_view.ground_author_tool = Ground3DAuthorTool::Footing;
                universal_canvas.earth_bim_author.active_tool = EarthBimTool::Footing;
                universal_canvas.earth_bim_author.pending_start_mm = None;
                universal_canvas.active_tool = UniversalCanvasTool::SelectPan;
                universal_canvas.ground_origin_anchored = true;
                zoom_in_if_high_altitude(universal_canvas);
                spatial.push_trace("Draw Footing directly on Earth 3D (click point)", true);
            }
            Self::Slab => {
                build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                universal_canvas.earth_bim_author.active_tool = EarthBimTool::Slab;
                universal_canvas.earth_bim_author.pending_start_mm = None;
                universal_canvas.active_tool = UniversalCanvasTool::SelectPan;
                universal_canvas.ground_origin_anchored = true;
                zoom_in_if_high_altitude(universal_canvas);
                spatial.push_trace("Draw Slab on Earth 3D (click 2 opposite corners)", true);
            }
            Self::Door => {
                build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                universal_canvas.earth_bim_author.active_tool = EarthBimTool::Door;
                universal_canvas.earth_bim_author.pending_start_mm = None;
                universal_canvas.active_tool = UniversalCanvasTool::SelectPan;
                universal_canvas.ground_origin_anchored = true;
                zoom_in_if_high_altitude(universal_canvas);
                spatial.push_trace("Insert Door: click any 3D Wall on Earth 3D", true);
            }
            Self::Window => {
                build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                universal_canvas.earth_bim_author.active_tool = EarthBimTool::Window;
                universal_canvas.earth_bim_author.pending_start_mm = None;
                universal_canvas.active_tool = UniversalCanvasTool::SelectPan;
                universal_canvas.ground_origin_anchored = true;
                zoom_in_if_high_altitude(universal_canvas);
                spatial.push_trace("Insert Window: click any 3D Wall on Earth 3D", true);
            }
            Self::Line2D => {
                build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                universal_canvas.earth_bim_author.active_tool = EarthBimTool::None;
                universal_canvas.active_tool = UniversalCanvasTool::Line;
                zoom_in_if_high_altitude(universal_canvas);
                spatial.push_trace("Draw Vector Line directly on Earth 3D", true);
            }
            Self::Rect2D => {
                build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                universal_canvas.earth_bim_author.active_tool = EarthBimTool::None;
                universal_canvas.active_tool = UniversalCanvasTool::Rectangle;
                zoom_in_if_high_altitude(universal_canvas);
                spatial.push_trace("Draw Rectangle boundary directly on Earth 3D", true);
            }
            Self::Measure => {
                universal_canvas.measure_active = true;
                build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                universal_canvas.earth_bim_author.active_tool = EarthBimTool::None;
                universal_canvas.active_tool = UniversalCanvasTool::SelectPan;
                spatial.push_trace("Geodesic Measure active on Earth 3D (click points)", true);
            }
            Self::Terraform => {
                build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                universal_canvas.earth_bim_author.active_tool = EarthBimTool::None;
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


impl SpatialHudActions {
    pub fn merge(&mut self, other: Self) {
        self.sync_fem_and_earth |= other.sync_fem_and_earth;
        self.fit_camera |= other.fit_camera;
        self.export_archive |= other.export_archive;
        self.export_ifcx |= other.export_ifcx;
        self.export_earth_ifcx |= other.export_earth_ifcx;
        self.save_audit_note |= other.save_audit_note;
        self.toggle_theme |= other.toggle_theme;
        self.open_settings |= other.open_settings;
        self.switch_to_classic_ui |= other.switch_to_classic_ui;
        if other.toggle_n1_member.is_some() {
            self.toggle_n1_member = other.toggle_n1_member;
        }
        self.restore_n1_simulation |= other.restore_n1_simulation;
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
        if other.fly_to_next_world_building.is_some() {
            self.fly_to_next_world_building = other.fly_to_next_world_building;
        }
        if other.rotate_active_world_heading_deg.is_some() {
            self.rotate_active_world_heading_deg = other.rotate_active_world_heading_deg;
        }
        self.undo |= other.undo;
        self.redo |= other.redo;
        self.run_district_hazard_audit |= other.run_district_hazard_audit;
        self.ingest_ai_real_building_capture |= other.ingest_ai_real_building_capture;
        if other.run_structural_retrofit.is_some() {
            self.run_structural_retrofit = other.run_structural_retrofit;
        }
        self.toggle_retrofit_before_after |= other.toggle_retrofit_before_after;
        self.export_retrofit_delta_ifcx |= other.export_retrofit_delta_ifcx;
        if other.run_crc_cascade_screening.is_some() {
            self.run_crc_cascade_screening = other.run_crc_cascade_screening;
        }
        self.export_crc_cascade_ifcx |= other.export_crc_cascade_ifcx;
        self.run_subsurface_ssi_sync |= other.run_subsurface_ssi_sync;
        self.export_subsurface_ssi_ifcx |= other.export_subsurface_ssi_ifcx;
        self.capture_ifcx_baseline |= other.capture_ifcx_baseline;
        self.run_ifcx_visual_diff |= other.run_ifcx_visual_diff;
        self.export_ifcx_diff_manifest |= other.export_ifcx_diff_manifest;
        self.export_vector_dossier_svg |= other.export_vector_dossier_svg;
        self.run_portfolio_retrofit |= other.run_portfolio_retrofit;
        self.export_portfolio_retrofit_ifcx |= other.export_portfolio_retrofit_ifcx;
        self.run_async_ground_motion_replay |= other.run_async_ground_motion_replay;
        self.export_ground_motion_replay_ifcx |= other.export_ground_motion_replay_ifcx;
    }
}
