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
    EigenGateRedundancy,
    AxialForce,
    ShearForce,
    BendingMoment,
    DeformedShape,
    ModalVibration,
    TimeHistoryReplay,
    ShmDigitalTwin,
    Erection4D,
}

impl SpatialLens {
    pub fn label(self) -> &'static str {
        match self {
            Self::Clean => "Clean",
            Self::DcrHeatmap => "DCR Utilization",
            Self::EigenGateRedundancy => "EigenGate Redundancy",
            Self::AxialForce => "Axial NFD",
            Self::ShearForce => "Shear SFD",
            Self::BendingMoment => "Moment BMD",
            Self::DeformedShape => "Deformed 6-DOF",
            Self::ModalVibration => "Modal Eigenmode",
            Self::TimeHistoryReplay => "EQ Time-History",
            Self::ShmDigitalTwin => "SHM Digital Twin",
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
            Self::EigenAiProFem => "EigenAI Pro (FEM+EigenGate)",
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
            Self::FemRedundancy => "@FEM & EigenGate".to_string(),
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
    ApplyEigenGateLens,
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
    // Phase 74 & 75: Async Multi-Thread / WebWorker StructGNN-CRC, PEER .at2/.csv, Tri-Dir & FEMA P-58
    RunEqReplay(Option<crate::core::time_history::GroundMotionRecord>),
    TogglePlasticHinges,
    ToggleTriDirectional(Option<crate::core::ground_motion_replay::TriDirectionalMode>),
    ToggleAutoMatchSa,
    OpenFragilityTab,
    // Phase 76: Live SHM Digital Twin & Automated Post-Earthquake ATC-20 Placard Triage
    OpenShmDigitalTwin,
    InjectShmFault {
        scenario: String,
        severity_pct: f64,
        story: usize,
    },
    RunAtc20Triage,
    // Phase 77: Autonomous Multi-Hazard Evacuation Routing & Emergency Lifelines
    RunEvacuationRouting,
    OpenEvacuationDrawer,
    // Phase 78: Live WebSocket SHM Telemetry Streaming & Rolling Watchdog
    StartShmWatchdog,
    ConnectShmWs(String),
    TriggerWatchdogSim(String),
    // Phase 79: WebGPU Compute Shaders (`@compute` WGSL) & FEMA P-58 Monte Carlo
    RunGpuMonteCarlo(u32),
    ToggleGpuParticles,
    ExportMcIfcx,
    // Phase 80: Collaborative Multi-Layer IFCX Governance & 3D Risk Field Heatmap
    CreateCheckpoint(String),
    ApproveCheckpoint,
    RunIfcxMerge,
    ToggleRiskHeatmap(Option<crate::core::ifcx_collab::DistrictRiskMetric>),
    // Phase 81: Parametric Trade-Study Matrix, Permit Annex & Peer Sync Relay
    RunTradeStudy(Option<crate::core::trade_study::TradeStudyObjective>),
    ExportPermitAnnex,
    SyncPeerRelay(Option<String>),
    // Phase 82: Field Construction QA/QC, 1D Rebar BBS Cut-List, Sheet S-103 & EigenFlow
    RunBbsOptimize(f64),
    ToggleQaqc3d,
    ExportShopSvg,
    RunEigenFlow,
    // Phase 83: EigenFlow Non-Linear Studio, 3D Woodbury P1->P2, Pushover ADRS & Sheet S-104
    OpenEigenFlowStudio,
    RunPushover(crate::core::pushover::LateralLoadPattern),
    ToggleEigenFlow3d,
    ExportEigenFlowAnnex,
    // Phase 84: Extreme Events Studio (Blast UFC 3-340-02 & Fire ISO 834/EC) & Sheet S-105
    RunBlast { mass_kg: f64, standoff_m: f64 },
    RunFire { minutes: f64, story: Option<usize> },
    ToggleExtreme3d,
    ExportExtremeAnnex,
    // Update v0.8.4.1 (1/10 -> v0.8.5): Seismic Base Isolation & Damper Studio + Sheet S-106
    RunIsolation(crate::core::isolation::IsolatorType),
    ToggleIsolation3d,
    ExportIsolationAnnex,
    // Update v0.8.4.2 (2/10 -> v0.8.5): Wind Aeroelasticity & ISO 10137 Comfort Studio + Sheet S-107
    RunWind {
        speed_ms: Option<f64>,
        exposure: Option<crate::core::wind::ExposureCategory>,
        azimuth_deg: Option<f64>,
    },
    ToggleWind3d,
    ExportWindAnnex,
    // Update v0.8.4.3 (3/10 -> v0.8.5): Special RC Shear Wall, SBE & Diagonal Coupling Beam + Sheet S-108
    RunShearWall {
        tw_mm: Option<f64>,
        fc_mpa: Option<f64>,
        drift_ratio: Option<f64>,
    },
    ToggleShearWall3d,
    ExportShearWallAnnex,
    // Update v0.8.4.4 (4/10 -> v0.8.5): Steel Seismic Connection & Panel-Zone Studio + Sheet S-109
    RunConnection {
        conn_type: Option<crate::core::connection::ConnectionType>,
        bolt_dia_mm: Option<f64>,
        plate_thick_mm: Option<f64>,
    },
    ToggleConnection3d,
    ExportConnectionAnnex,
    // Update v0.8.4.5 (5/10 -> v0.8.5): Post-Tensioned & Prestressed Concrete Studio + Sheet S-110
    RunPrestress {
        profile: Option<crate::core::prestressed_concrete::TendonProfileKind>,
        num_strands: Option<usize>,
        fc_mpa: Option<f64>,
    },
    TogglePrestress3d,
    ExportPrestressAnnex,
    // Update v0.8.4.6 (6/10 -> v0.8.5): Composite Deck, Shear Studs & AISC DG11 Studio + Sheet S-111
    RunComposite {
        studs: Option<usize>,
        orientation: Option<crate::core::composite::DeckOrientation>,
        unshored: Option<bool>,
    },
    ToggleComposite3d,
    ExportCompositeAnnex,
    // Update v0.8.4.7 (7/10 -> v0.8.5): Structural Reliability FORM/SORM & Sensitivity Studio + Sheet S-112
    RunReliability {
        method: Option<crate::core::reliability_studio::ReliabilityMethod>,
        target_beta: Option<f64>,
        limit_state: Option<crate::core::reliability_studio::LimitStateKind>,
    },
    ToggleReliability3d,
    ExportReliabilityAnnex,
    // Update v0.8.4.8 (8/10 -> v0.8.5): Deep Foundation Pile Group, Winkler Springs & Punching Shear + Sheet S-113
    RunFoundation {
        soil_type: Option<crate::core::geotech::SoilType>,
        piles_x: Option<usize>,
        piles_y: Option<usize>,
        pile_length_m: Option<f64>,
    },
    ToggleFoundation3d,
    ExportFoundationAnnex,
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
/// (`[[FLY:...]]`, `[[BIM:...]]`, `[[REPAIR]]`, `[[EIGENGATE_AUDIT]]`, `[[RESIZE_COL:...]]`, `[[RESIZE_BEAM:...]]`, `[[ADD_COL:...]]`, `[[APPLY_SITE_SEISMIC]]`, `[[OPEN_SHEET:...]]`, `[[ADD_STORY]]`, `[[ADD_SLAB]]`).
#[derive(Debug, Clone, PartialEq)]
pub enum GeminiDirective {
    Fly { lat: f64, lon: f64, zoom: f64 },
    Bim(String),
    Repair,
    EigenGateAudit,
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
    // Phase 74 & 75: Async Multi-Thread / WebWorker StructGNN-CRC, PEER .at2/.csv, Tri-Dir & FEMA P-58
    RunEqReplay(Option<crate::core::time_history::GroundMotionRecord>),
    TogglePlasticHinges,
    ToggleTriDirectional(Option<crate::core::ground_motion_replay::TriDirectionalMode>),
    ToggleAutoMatchSa,
    OpenFragilityTab,
    // Phase 76: Live SHM Digital Twin & Automated Post-Earthquake ATC-20 Placard Triage
    OpenShmDigitalTwin,
    InjectShmFault {
        scenario: String,
        severity_pct: f64,
        story: usize,
    },
    RunAtc20Triage,
    // Phase 77: Autonomous Multi-Hazard Evacuation Routing & Emergency Lifelines
    RunEvacuationRouting,
    OpenEvacuationDrawer,
    // Phase 78: Live WebSocket SHM Telemetry Streaming & Rolling Watchdog
    StartShmWatchdog,
    ConnectShmWs(String),
    TriggerWatchdogSim(String),
    // Phase 79: WebGPU Compute Shaders (`@compute` WGSL) & FEMA P-58 Monte Carlo
    RunGpuMonteCarlo(u32),
    ToggleGpuParticles,
    ExportMcIfcx,
    // Phase 80: Collaborative Multi-Layer IFCX Governance & 3D Risk Field Heatmap
    CreateCheckpoint(String),
    ApproveCheckpoint,
    RunIfcxMerge,
    ToggleRiskHeatmap(Option<crate::core::ifcx_collab::DistrictRiskMetric>),
    // Phase 81: Parametric Trade-Study Matrix, Permit Annex & Peer Sync Relay
    RunTradeStudy(Option<crate::core::trade_study::TradeStudyObjective>),
    ExportPermitAnnex,
    SyncPeerRelay(Option<String>),
    // Phase 82: Field Construction QA/QC, 1D Rebar BBS Cut-List, Sheet S-103 & EigenFlow
    RunBbsOptimize(f64),
    ToggleQaqc3d,
    ExportShopSvg,
    RunEigenFlow,
    // Phase 83: EigenFlow Non-Linear Studio, 3D Woodbury P1->P2, Pushover ADRS & Sheet S-104
    OpenEigenFlowStudio,
    RunPushover(crate::core::pushover::LateralLoadPattern),
    ToggleEigenFlow3d,
    ExportEigenFlowAnnex,
    // Phase 84: Extreme Events Studio (Blast UFC 3-340-02 & Fire ISO 834/EC) & Sheet S-105
    RunBlast { mass_kg: f64, standoff_m: f64 },
    RunFire { minutes: f64, story: Option<usize> },
    ToggleExtreme3d,
    ExportExtremeAnnex,
    // Update v0.8.4.1 (1/10 -> v0.8.5): Seismic Base Isolation & Damper Studio + Sheet S-106
    RunIsolation(crate::core::isolation::IsolatorType),
    ToggleIsolation3d,
    ExportIsolationAnnex,
    // Update v0.8.4.2 (2/10 -> v0.8.5): Wind Aeroelasticity & ISO 10137 Comfort Studio + Sheet S-107
    RunWind {
        speed_ms: Option<f64>,
        exposure: Option<crate::core::wind::ExposureCategory>,
        azimuth_deg: Option<f64>,
    },
    ToggleWind3d,
    ExportWindAnnex,
    // Update v0.8.4.3 (3/10 -> v0.8.5): Special RC Shear Wall, SBE & Diagonal Coupling Beam + Sheet S-108
    RunShearWall {
        tw_mm: Option<f64>,
        fc_mpa: Option<f64>,
        drift_ratio: Option<f64>,
    },
    ToggleShearWall3d,
    ExportShearWallAnnex,
    // Update v0.8.4.4 (4/10 -> v0.8.5): Steel Seismic Connection & Panel-Zone Studio + Sheet S-109
    RunConnection {
        conn_type: Option<crate::core::connection::ConnectionType>,
        bolt_dia_mm: Option<f64>,
        plate_thick_mm: Option<f64>,
    },
    ToggleConnection3d,
    ExportConnectionAnnex,
    // Update v0.8.4.5 (5/10 -> v0.8.5): Post-Tensioned & Prestressed Concrete Studio + Sheet S-110
    RunPrestress {
        profile: Option<crate::core::prestressed_concrete::TendonProfileKind>,
        num_strands: Option<usize>,
        fc_mpa: Option<f64>,
    },
    TogglePrestress3d,
    ExportPrestressAnnex,
    // Update v0.8.4.6 (6/10 -> v0.8.5): Composite Deck, Shear Studs & AISC DG11 Studio + Sheet S-111
    RunComposite {
        studs: Option<usize>,
        orientation: Option<crate::core::composite::DeckOrientation>,
        unshored: Option<bool>,
    },
    ToggleComposite3d,
    ExportCompositeAnnex,
    // Update v0.8.4.7 (7/10 -> v0.8.5): Structural Reliability FORM/SORM & Sensitivity Studio + Sheet S-112
    RunReliability {
        method: Option<crate::core::reliability_studio::ReliabilityMethod>,
        target_beta: Option<f64>,
        limit_state: Option<crate::core::reliability_studio::LimitStateKind>,
    },
    ToggleReliability3d,
    ExportReliabilityAnnex,
    // Update v0.8.4.8 (8/10 -> v0.8.5): Deep Foundation Pile Group, Winkler Springs & Punching Shear + Sheet S-113
    RunFoundation {
        soil_type: Option<crate::core::geotech::SoilType>,
        piles_x: Option<usize>,
        piles_y: Option<usize>,
        pile_length_m: Option<f64>,
    },
    ToggleFoundation3d,
    ExportFoundationAnnex,
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
            } else if upper == "EIGENGATE_AUDIT" || upper == "EIGENGATE" || upper == "ARI_AUDIT" || upper == "ARI" {
                directives.push(GeminiDirective::EigenGateAudit);
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
            } else if upper == "TOGGLE_TRIDIR" || upper.starts_with("TOGGLE_TRIDIR:") {
                use crate::core::ground_motion_replay::TriDirectionalMode;
                let mode = upper.strip_prefix("TOGGLE_TRIDIR:").and_then(|s| match s.trim() {
                    "X" | "1D" => Some(TriDirectionalMode::UniaxialX),
                    "XY" | "2D" => Some(TriDirectionalMode::BiaxialXY),
                    "XYZ" | "3D" => Some(TriDirectionalMode::TriDirectionalXYZ),
                    _ => None,
                });
                directives.push(GeminiDirective::ToggleTriDirectional(mode));
            } else if upper == "MATCH_SA_SPECTRUM" {
                directives.push(GeminiDirective::ToggleAutoMatchSa);
            } else if upper == "OPEN_FRAGILITY" {
                directives.push(GeminiDirective::OpenFragilityTab);
            } else if upper == "OPEN_SHM_TWIN" {
                directives.push(GeminiDirective::OpenShmDigitalTwin);
            } else if upper == "RUN_ATC20_TRIAGE" {
                directives.push(GeminiDirective::RunAtc20Triage);
            } else if upper == "RUN_EVACUATION_ROUTING" {
                directives.push(GeminiDirective::RunEvacuationRouting);
            } else if upper == "OPEN_EVAC_DRAWER" {
                directives.push(GeminiDirective::OpenEvacuationDrawer);
            } else if upper == "START_SHM_WATCHDOG" {
                directives.push(GeminiDirective::StartShmWatchdog);
            } else if let Some(rest) = tag.strip_prefix("CONNECT_SHM_WS:")
                .or_else(|| tag.strip_prefix("connect_shm_ws:"))
            {
                let url = rest.trim();
                if !url.is_empty() {
                    directives.push(GeminiDirective::ConnectShmWs(url.to_string()));
                }
            } else if let Some(rest) = upper.strip_prefix("TRIGGER_WATCHDOG_SIM:") {
                directives.push(GeminiDirective::TriggerWatchdogSim(
                    rest.trim().to_ascii_lowercase(),
                ));
            } else if upper == "RUN_GPU_MONTE_CARLO" || upper.starts_with("RUN_GPU_MONTE_CARLO:") {
                let n = upper
                    .strip_prefix("RUN_GPU_MONTE_CARLO:")
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .unwrap_or(4096)
                    .clamp(256, 16_384);
                directives.push(GeminiDirective::RunGpuMonteCarlo(n));
            } else if upper == "TOGGLE_GPU_PARTICLES" {
                directives.push(GeminiDirective::ToggleGpuParticles);
            } else if upper == "EXPORT_MC_IFCX" {
                directives.push(GeminiDirective::ExportMcIfcx);
            } else if upper == "CREATE_CHECKPOINT" || upper.starts_with("CREATE_CHECKPOINT:") {
                let lbl = tag
                    .split_once(':')
                    .map(|(_, r)| r.trim().to_string())
                    .unwrap_or_default();
                directives.push(GeminiDirective::CreateCheckpoint(lbl));
            } else if upper == "APPROVE_CHECKPOINT" {
                directives.push(GeminiDirective::ApproveCheckpoint);
            } else if upper == "TOGGLE_RISK_HEATMAP" || upper.starts_with("TOGGLE_RISK_HEATMAP:") {
                let metric = upper.split_once(':').and_then(|(_, r)| {
                    let k = r.trim();
                    if k.contains("PML") || k.contains("MC") || k.contains("LOSS") {
                        Some(crate::core::ifcx_collab::DistrictRiskMetric::McPml90Loss)
                    } else if k.contains("EIGENGATE") || k.contains("ARI") || k.contains("DCR") {
                        Some(crate::core::ifcx_collab::DistrictRiskMetric::StructuralEigenGate)
                    } else if k.contains("ISO") || k.contains("EVAC") {
                        Some(crate::core::ifcx_collab::DistrictRiskMetric::EvacuationIsolation)
                    } else if k.contains("COMP") {
                        Some(crate::core::ifcx_collab::DistrictRiskMetric::CompositeRisk)
                    } else {
                        None
                    }
                });
                directives.push(GeminiDirective::ToggleRiskHeatmap(metric));
            } else if upper == "RUN_TRADE_STUDY" || upper.starts_with("RUN_TRADE_STUDY:") {
                let obj = upper.split_once(':').and_then(|(_, r)| {
                    let k = r.trim();
                    if k.contains("SAFETY") || k.contains("EIGENGATE") || k.contains("ARI") {
                        Some(crate::core::trade_study::TradeStudyObjective::MaxSafetyEigenGate)
                    } else if k.contains("COST") {
                        Some(crate::core::trade_study::TradeStudyObjective::MinCost)
                    } else if k.contains("CARBON") || k.contains("CO2") {
                        Some(crate::core::trade_study::TradeStudyObjective::MinCarbon)
                    } else if k.contains("BAL") {
                        Some(crate::core::trade_study::TradeStudyObjective::Balanced)
                    } else {
                        None
                    }
                });
                directives.push(GeminiDirective::RunTradeStudy(obj));
            } else if upper == "EXPORT_PERMIT_ANNEX" {
                directives.push(GeminiDirective::ExportPermitAnnex);
            } else if upper == "SYNC_PEER_RELAY" || upper.starts_with("SYNC_PEER_RELAY:") {
                let url = tag
                    .split_once(':')
                    .map(|(_, r)| r.trim().to_string())
                    .filter(|s| !s.is_empty());
                directives.push(GeminiDirective::SyncPeerRelay(url));
            } else if upper == "RUN_BBS_OPTIMIZE" || upper.starts_with("RUN_BBS_OPTIMIZE:") {
                let len_m = upper
                    .split_once(':')
                    .and_then(|(_, r)| r.trim().trim_end_matches('M').parse::<f64>().ok())
                    .map(|v| if (v - 6.0).abs() < 1.5 { 6.0 } else { 12.0 })
                    .unwrap_or(12.0);
                directives.push(GeminiDirective::RunBbsOptimize(len_m));
            } else if upper == "TOGGLE_QAQC_3D" {
                directives.push(GeminiDirective::ToggleQaqc3d);
            } else if upper == "EXPORT_SHOP_SVG" {
                directives.push(GeminiDirective::ExportShopSvg);
            } else if upper == "RUN_EIGENFLOW" || upper == "EIGENFLOW" {
                directives.push(GeminiDirective::RunEigenFlow);
            } else if upper == "OPEN_EIGENFLOW_STUDIO" {
                directives.push(GeminiDirective::OpenEigenFlowStudio);
            } else if upper == "TOGGLE_EIGENFLOW_3D" {
                directives.push(GeminiDirective::ToggleEigenFlow3d);
            } else if upper == "EXPORT_EIGENFLOW_ANNEX" {
                directives.push(GeminiDirective::ExportEigenFlowAnnex);
            } else if upper == "RUN_PUSHOVER" || upper.starts_with("RUN_PUSHOVER:") {
                let arg = upper.split_once(':').map(|(_, r)| r.trim()).unwrap_or("");
                let pat = if arg.contains("UNI") {
                    crate::core::pushover::LateralLoadPattern::UniformMass
                } else if arg.contains("TRI") {
                    crate::core::pushover::LateralLoadPattern::InvertedTriangular
                } else {
                    crate::core::pushover::LateralLoadPattern::FirstModeModal
                };
                directives.push(GeminiDirective::RunPushover(pat));
            } else if upper == "RUN_BLAST" || upper.starts_with("RUN_BLAST:") {
                let mut sp = upper
                    .split_once(':')
                    .map(|(_, r)| r)
                    .unwrap_or("")
                    .split([',', ':'])
                    .map(|s| s.trim());
                let mass_kg = sp
                    .next()
                    .and_then(|s| s.trim_end_matches("KG").parse::<f64>().ok())
                    .unwrap_or(250.0)
                    .clamp(10.0, 5_000.0);
                let standoff_m = sp
                    .next()
                    .and_then(|s| s.trim_end_matches('M').parse::<f64>().ok())
                    .unwrap_or(12.0)
                    .clamp(3.0, 120.0);
                directives.push(GeminiDirective::RunBlast { mass_kg, standoff_m });
            } else if upper == "RUN_FIRE" || upper.starts_with("RUN_FIRE:") {
                let mut sp = upper
                    .split_once(':')
                    .map(|(_, r)| r)
                    .unwrap_or("")
                    .split([',', ':'])
                    .map(|s| s.trim());
                let minutes = sp
                    .next()
                    .and_then(|s| s.trim_end_matches("MIN").parse::<f64>().ok())
                    .unwrap_or(90.0)
                    .clamp(0.0, 180.0);
                let story = sp
                    .next()
                    .and_then(|s| s.trim_start_matches('L').parse::<usize>().ok())
                    .map(|n| n.saturating_sub(1));
                directives.push(GeminiDirective::RunFire { minutes, story });
            } else if upper == "TOGGLE_EXTREME_3D" {
                directives.push(GeminiDirective::ToggleExtreme3d);
            } else if upper == "EXPORT_EXTREME_ANNEX" {
                directives.push(GeminiDirective::ExportExtremeAnnex);
            } else if upper == "RUN_ISOLATION" || upper.starts_with("RUN_ISOLATION:") {
                let arg = upper.split_once(':').map(|(_, r)| r.trim()).unwrap_or("");
                let iso = if arg.contains("FPS") {
                    crate::core::isolation::IsolatorType::FrictionPendulum
                } else if arg.contains("FVD") {
                    crate::core::isolation::IsolatorType::FluidViscousDamper
                } else if arg.contains("HDRB") {
                    crate::core::isolation::IsolatorType::HighDampingRubberBearing
                } else {
                    crate::core::isolation::IsolatorType::LeadRubberBearing
                };
                directives.push(GeminiDirective::RunIsolation(iso));
            } else if upper == "TOGGLE_ISOLATION_3D" {
                directives.push(GeminiDirective::ToggleIsolation3d);
            } else if upper == "EXPORT_ISOLATION_ANNEX" {
                directives.push(GeminiDirective::ExportIsolationAnnex);
            } else if upper == "RUN_WIND" || upper.starts_with("RUN_WIND:") {
                let arg = upper.split_once(':').map(|(_, r)| r.trim()).unwrap_or("");
                let mut speed_ms = None;
                let mut exposure = None;
                let mut azimuth_deg = None;
                for tok in arg.split(':').map(str::trim).filter(|s| !s.is_empty()) {
                    if tok == "B" || tok == "EXPOSUREB" || tok == "EXPOSURE_B" {
                        exposure = Some(crate::core::wind::ExposureCategory::ExposureB);
                    } else if tok == "C" || tok == "EXPOSUREC" || tok == "EXPOSURE_C" {
                        exposure = Some(crate::core::wind::ExposureCategory::ExposureC);
                    } else if tok == "D" || tok == "EXPOSURED" || tok == "EXPOSURE_D" {
                        exposure = Some(crate::core::wind::ExposureCategory::ExposureD);
                    } else if let Ok(v) = tok.parse::<f64>() {
                        if speed_ms.is_none() {
                            speed_ms = Some(v.clamp(20.0, 90.0));
                        } else if azimuth_deg.is_none() {
                            azimuth_deg = Some(v.rem_euclid(360.0));
                        }
                    }
                }
                directives.push(GeminiDirective::RunWind {
                    speed_ms,
                    exposure,
                    azimuth_deg,
                });
            } else if upper == "TOGGLE_WIND_3D" {
                directives.push(GeminiDirective::ToggleWind3d);
            } else if upper == "EXPORT_WIND_ANNEX" {
                directives.push(GeminiDirective::ExportWindAnnex);
            } else if upper == "RUN_SHEAR_WALL" || upper.starts_with("RUN_SHEAR_WALL:") {
                let arg = upper.split_once(':').map(|(_, r)| r.trim()).unwrap_or("");
                let mut sp = arg.split(':').filter(|s| !s.is_empty());
                let tw_mm = sp.next().and_then(|s| s.parse::<f64>().ok()).map(|v| v.clamp(200.0, 800.0));
                let fc_mpa = sp.next().and_then(|s| s.parse::<f64>().ok()).map(|v| v.clamp(25.0, 70.0));
                let drift_ratio = sp.next().and_then(|s| s.parse::<f64>().ok()).map(|v| v.clamp(0.005, 0.030));
                directives.push(GeminiDirective::RunShearWall {
                    tw_mm,
                    fc_mpa,
                    drift_ratio,
                });
            } else if upper == "TOGGLE_SHEAR_WALL_3D" {
                directives.push(GeminiDirective::ToggleShearWall3d);
            } else if upper == "EXPORT_SHEAR_WALL_ANNEX" {
                directives.push(GeminiDirective::ExportShearWallAnnex);
            } else if upper == "RUN_CONNECTION" || upper.starts_with("RUN_CONNECTION:") {
                let arg = upper.split_once(':').map(|(_, r)| r.trim()).unwrap_or("");
                let mut conn_type = None;
                let mut bolt_dia_mm = None;
                let mut plate_thick_mm = None;
                for tok in arg.split(':').map(str::trim).filter(|s| !s.is_empty()) {
                    if tok.contains("SHEAR") || tok == "TAB" {
                        conn_type = Some(crate::core::connection::ConnectionType::ShearTabSinglePlate);
                    } else if tok.contains("ENDPLATE") || tok == "4E" || tok == "8ES" {
                        conn_type = Some(crate::core::connection::ConnectionType::ExtendedEndPlate4E);
                    } else if let Ok(v) = tok.parse::<f64>() {
                        if bolt_dia_mm.is_none() {
                            bolt_dia_mm = Some(v.clamp(16.0, 36.0));
                        } else if plate_thick_mm.is_none() {
                            plate_thick_mm = Some(v.clamp(10.0, 50.0));
                        }
                    }
                }
                directives.push(GeminiDirective::RunConnection {
                    conn_type,
                    bolt_dia_mm,
                    plate_thick_mm,
                });
            } else if upper == "TOGGLE_CONNECTION_3D" {
                directives.push(GeminiDirective::ToggleConnection3d);
            } else if upper == "EXPORT_CONNECTION_ANNEX" {
                directives.push(GeminiDirective::ExportConnectionAnnex);
            } else if upper == "RUN_PRESTRESS" || upper.starts_with("RUN_PRESTRESS:") {
                let arg = upper.split_once(':').map(|(_, r)| r.trim()).unwrap_or("");
                let mut profile = None;
                let mut num_strands = None;
                let mut fc_mpa = None;
                for tok in arg.split(':').map(str::trim).filter(|s| !s.is_empty()) {
                    if tok.contains("HARP") {
                        profile = Some(crate::core::prestressed_concrete::TendonProfileKind::HarpedTwoPoint);
                    } else if tok.contains("STRAIGHT") {
                        profile = Some(crate::core::prestressed_concrete::TendonProfileKind::StraightEccentric);
                    } else if tok.contains("PARA") {
                        profile = Some(crate::core::prestressed_concrete::TendonProfileKind::Parabolic);
                    } else if let Ok(v) = tok.parse::<f64>() {
                        if num_strands.is_none() && v <= 64.0 && v >= 4.0 && (v.fract() == 0.0) && v <= 30.0 {
                            num_strands = Some(v.round() as usize);
                        } else if num_strands.is_none() {
                            num_strands = Some((v.round() as usize).clamp(4, 64));
                        } else if fc_mpa.is_none() {
                            fc_mpa = Some(v.clamp(30.0, 80.0));
                        }
                    }
                }
                directives.push(GeminiDirective::RunPrestress {
                    profile,
                    num_strands,
                    fc_mpa,
                });
            } else if upper == "TOGGLE_PRESTRESS_3D" {
                directives.push(GeminiDirective::TogglePrestress3d);
            } else if upper == "EXPORT_PRESTRESS_ANNEX" {
                directives.push(GeminiDirective::ExportPrestressAnnex);
            } else if upper == "RUN_COMPOSITE" || upper.starts_with("RUN_COMPOSITE:") {
                let arg = upper.split_once(':').map(|(_, r)| r.trim()).unwrap_or("");
                let mut studs = None;
                let mut orientation = None;
                let mut unshored = None;
                for tok in arg.split(':').map(str::trim).filter(|s| !s.is_empty()) {
                    if tok.contains("PARA") {
                        orientation = Some(crate::core::composite::DeckOrientation::Parallel);
                    } else if tok.contains("PERP") {
                        orientation = Some(crate::core::composite::DeckOrientation::Perpendicular);
                    } else if tok == "SHORED" {
                        unshored = Some(false);
                    } else if tok == "UNSHORED" {
                        unshored = Some(true);
                    } else if let Ok(v) = tok.parse::<usize>() {
                        studs = Some(v.clamp(12, 160));
                    }
                }
                directives.push(GeminiDirective::RunComposite {
                    studs,
                    orientation,
                    unshored,
                });
            } else if upper == "TOGGLE_COMPOSITE_3D" {
                directives.push(GeminiDirective::ToggleComposite3d);
            } else if upper == "EXPORT_COMPOSITE_ANNEX" {
                directives.push(GeminiDirective::ExportCompositeAnnex);
            } else if upper == "RUN_RELIABILITY" || upper.starts_with("RUN_RELIABILITY:") {
                let arg = upper.split_once(':').map(|(_, r)| r.trim()).unwrap_or("");
                let mut method = None;
                let mut target_beta = None;
                let mut limit_state = None;
                for tok in arg.split(':').map(str::trim).filter(|s| !s.is_empty()) {
                    if tok.contains("FORM") {
                        method = Some(crate::core::reliability_studio::ReliabilityMethod::FormHasoferLind);
                    } else if tok.contains("SORM") {
                        method = Some(crate::core::reliability_studio::ReliabilityMethod::SormBreitung);
                    } else if tok.contains("FOSM") {
                        method = Some(crate::core::reliability_studio::ReliabilityMethod::FosmLogNormal);
                    } else if tok.contains("FLEX") {
                        limit_state = Some(crate::core::reliability_studio::LimitStateKind::FlexurePlastic);
                    } else if tok.contains("COL") || tok.contains("AXIAL") {
                        limit_state = Some(crate::core::reliability_studio::LimitStateKind::AxialColumn);
                    } else if tok.contains("DRIFT") {
                        limit_state = Some(crate::core::reliability_studio::LimitStateKind::InterstoryDrift);
                    } else if tok.contains("SHEAR") {
                        limit_state = Some(crate::core::reliability_studio::LimitStateKind::ShearBrittle);
                    } else if let Ok(b) = tok.parse::<f64>() {
                        target_beta = Some(b.clamp(2.0, 5.5));
                    }
                }
                directives.push(GeminiDirective::RunReliability {
                    method,
                    target_beta,
                    limit_state,
                });
            } else if upper == "TOGGLE_RELIABILITY_3D" {
                directives.push(GeminiDirective::ToggleReliability3d);
            } else if upper == "EXPORT_RELIABILITY_ANNEX" {
                directives.push(GeminiDirective::ExportReliabilityAnnex);
            } else if upper == "RUN_FOUNDATION" || upper.starts_with("RUN_FOUNDATION:") {
                let arg = upper.split_once(':').map(|(_, r)| r.trim()).unwrap_or("");
                let mut soil_type = None;
                let mut piles_x = None;
                let mut piles_y = None;
                let mut pile_length_m = None;
                for tok in arg.split(':').map(str::trim).filter(|s| !s.is_empty()) {
                    if tok.contains("SOFT") {
                        soil_type = Some(crate::core::geotech::SoilType::SoftClay);
                    } else if tok.contains("STIFF") {
                        soil_type = Some(crate::core::geotech::SoilType::StiffClay);
                    } else if tok.contains("LOOSE") {
                        soil_type = Some(crate::core::geotech::SoilType::LooseSand);
                    } else if tok.contains("DENSE") {
                        soil_type = Some(crate::core::geotech::SoilType::DenseSand);
                    } else if tok.contains("ROCK") || tok.contains("BED") {
                        soil_type = Some(crate::core::geotech::SoilType::Bedrock);
                    } else if let Some((lx, ly)) = tok.split_once('X') {
                        if let (Ok(nx), Ok(ny)) = (lx.parse::<usize>(), ly.parse::<usize>()) {
                            piles_x = Some(nx.clamp(1, 5));
                            piles_y = Some(ny.clamp(1, 5));
                        }
                    } else if let Ok(len_m) = tok.trim_end_matches('M').parse::<f64>() {
                        pile_length_m = Some(len_m.clamp(6.0, 48.0));
                    }
                }
                directives.push(GeminiDirective::RunFoundation {
                    soil_type,
                    piles_x,
                    piles_y,
                    pile_length_m,
                });
            } else if upper == "TOGGLE_FOUNDATION_3D" {
                directives.push(GeminiDirective::ToggleFoundation3d);
            } else if upper == "EXPORT_FOUNDATION_ANNEX" {
                directives.push(GeminiDirective::ExportFoundationAnnex);
            } else if let Some(rest) = upper.strip_prefix("INJECT_SHM_FAULT:") {
                let mut sp = rest.split(':').map(|s| s.trim());
                let scenario = sp.next().unwrap_or("soft_story").to_ascii_lowercase();
                let severity_pct = sp
                    .next()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(75.0)
                    .clamp(0.0, 100.0);
                let story = sp
                    .next()
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(1)
                    .max(1);
                directives.push(GeminiDirective::InjectShmFault {
                    scenario,
                    severity_pct,
                    story,
                });
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
        cmd: "/lens eigengate",
        category: "Lens",
        description: "Colorize structural elements by Redundancy Index (EigenGate)",
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
        description: "Run closed-loop AI auto-repair for critical EigenGate/DCR members",
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
        description: "Toggle 100% Pure WebGPU (WGSL) 3D Vector Earth pass vs. CPU vector fallback",
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
        description: "Stream AI real-building massing .ifcx onto Earth parcel with auto frame inference & EigenGate triage",
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
        description: "Inject 3D Buckling-Restrained Braces (BRB) across low-EigenGate structural bays",
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
        description: "Run Two-Stage EigenGate + StructGNN-CRC conformal screening & 3D progressive collapse cascade",
        icon: AppIcon::Bolt,
    },
    SlashCommandSpec {
        cmd: "/crc",
        category: "AI",
        description: "Evaluate Two-Stage EigenGate + StructGNN-CRC conformal scenario reduction & safety recall",
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
        description: "Toggle 3D WebGPU subsurface geological stratigraphy & groundwater X-Ray cutaway",
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
        description: "Run autonomous multi-building district EigenGate retrofit Pareto optimization",
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
    SlashCommandSpec {
        cmd: "/at2",
        category: "AI",
        description: "Upload PEER NGA (.at2) or (.csv) accelerogram (/at2 sample | clear)",
        icon: AppIcon::Document,
    },
    SlashCommandSpec {
        cmd: "/tridir",
        category: "AI",
        description: "Switch 1D X / 2D XY / 3D XYZ seismic excitation coupling (/tridir xyz)",
        icon: AppIcon::Cube3D,
    },
    SlashCommandSpec {
        cmd: "/match-sa",
        category: "AI",
        description: "Auto-scale ground motion Sa(T1) to SNI 1726 / ASCE 7-22 design spectrum",
        icon: AppIcon::ChartLine,
    },
    SlashCommandSpec {
        cmd: "/fragility",
        category: "AI",
        description: "Open real-time FEMA P-58 fragility curves & 3D per-story loss halo",
        icon: AppIcon::Shield,
    },
    SlashCommandSpec {
        cmd: "/fragility-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::analysis::fema_p58_fragility .ifcx overlay",
        icon: AppIcon::Cube3D,
    },
    SlashCommandSpec {
        cmd: "/shm",
        category: "AI",
        description: "Open Live SHM Digital Twin & Modal Fingerprint Identification (PSD + MAC)",
        icon: AppIcon::ChartLine,
    },
    SlashCommandSpec {
        cmd: "/sensors",
        category: "AI",
        description: "Ingest streamed or sample SHM CSV/JSON sensor logs (/sensors sample | upload | clear)",
        icon: AppIcon::Document,
    },
    SlashCommandSpec {
        cmd: "/placard",
        category: "Safety",
        description: "Run automated ATC-20 post-earthquake Green/Yellow/Red placard & district triage",
        icon: AppIcon::Shield,
    },
    SlashCommandSpec {
        cmd: "/shm-fault",
        category: "AI",
        description: "Inject calibrated structural fault (/shm-fault soft_story 80 1 | column_loss | liquefaction)",
        icon: AppIcon::Bolt,
    },
    SlashCommandSpec {
        cmd: "/shm-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::analysis::shm_digital_twin + atc20_placard .ifcx",
        icon: AppIcon::Cube3D,
    },
    SlashCommandSpec {
        cmd: "/evac",
        category: "Safety",
        description: "Compute A* safe evacuation & emergency responder routes avoiding Red cordons & flood",
        icon: AppIcon::Shield,
    },
    SlashCommandSpec {
        cmd: "/lifelines",
        category: "Safety",
        description: "Open Post-Disaster Road Network Connectivity & Emergency Lifelines Inspector",
        icon: AppIcon::Globe,
    },
    SlashCommandSpec {
        cmd: "/safezone",
        category: "GIS",
        description: "Place a safe assembly Park zone at the site perimeter & recompute evacuation routes",
        icon: AppIcon::Target,
    },
    SlashCommandSpec {
        cmd: "/evac-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::analysis::evacuation_lifelines .ifcx overlay",
        icon: AppIcon::Cube3D,
    },
    SlashCommandSpec {
        cmd: "/watchdog",
        category: "AI",
        description: "Open Live WebSocket SHM Telemetry Stream & Rolling STA/LTA Anomaly Watchdog",
        icon: AppIcon::Waves,
    },
    SlashCommandSpec {
        cmd: "/ws",
        category: "AI",
        description: "Connect live WebSocket SHM telemetry stream (/ws wss://host/ws/telemetry)",
        icon: AppIcon::Bolt,
    },
    SlashCommandSpec {
        cmd: "/ws-stop",
        category: "AI",
        description: "Disconnect active WebSocket SHM telemetry stream and stop Watchdog simulator",
        icon: AppIcon::Close,
    },
    SlashCommandSpec {
        cmd: "/ws-sim",
        category: "AI",
        description: "Run live SHM stream simulator (/ws-sim ambient | aftershock | soft_story | off)",
        icon: AppIcon::ChartLine,
    },
    SlashCommandSpec {
        cmd: "/watchdog-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::analysis::shm_watchdog .ifcx overlay",
        icon: AppIcon::Cube3D,
    },
    SlashCommandSpec {
        cmd: "/gpu-compute",
        category: "AI",
        description: "Open WebGPU @compute Multi-Hazard Particles & FEMA P-58 Monte Carlo drawer",
        icon: AppIcon::Bolt,
    },
    SlashCommandSpec {
        cmd: "/mc",
        category: "AI",
        description: "Run FEMA P-58 probabilistic Monte Carlo fragility (/mc 1024 | 4096 | 16384)",
        icon: AppIcon::ChartLine,
    },
    SlashCommandSpec {
        cmd: "/particles",
        category: "Hazard",
        description: "Toggle 60 FPS WebGPU @compute seismic wave & Venturi wind particles (optional count)",
        icon: AppIcon::Waves,
    },
    SlashCommandSpec {
        cmd: "/mc-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::analysis::gpu_monte_carlo_loss .ifcx overlay",
        icon: AppIcon::Cube3D,
    },
    SlashCommandSpec {
        cmd: "/checkpoint",
        category: "Collab",
        description: "Capture SHA-256 signed multi-layer .ifcx governance checkpoint (/checkpoint <label>)",
        icon: AppIcon::CheckCircle,
    },
    SlashCommandSpec {
        cmd: "/approve",
        category: "Collab",
        description: "Sign off & approve the active .ifcx governance checkpoint",
        icon: AppIcon::Shield,
    },
    SlashCommandSpec {
        cmd: "/merge-ifcx",
        category: "Collab",
        description: "Run deterministic 3-way IFCX merge against peer branch & resolve conflicts",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/risk-map",
        category: "Hazard",
        description: "Toggle 3D Pure WebGPU District Risk Field Heatmap (/risk-map composite|pml90|eigengate|isolation)",
        icon: AppIcon::Flame,
    },
    SlashCommandSpec {
        cmd: "/collab-ifcx",
        category: "Export",
        description: "Export signed .ifcx-bundle.json multi-discipline governance checkpoint chain",
        icon: AppIcon::SaveDisk,
    },
    SlashCommandSpec {
        cmd: "/trade-study",
        category: "AI",
        description: "Run 6-scenario Trade-Study Matrix (/trade-study balanced|safety|cost|carbon)",
        icon: AppIcon::ChartLine,
    },
    SlashCommandSpec {
        cmd: "/permit-check",
        category: "Safety",
        description: "Audit 10-clause SNI 1726 / SNI 2847 / ASCE 7 / UFC Permit Compliance Matrix",
        icon: AppIcon::CheckCircle,
    },
    SlashCommandSpec {
        cmd: "/permit-svg",
        category: "Export",
        description: "Export A3 Sheet S-102 Pure-Vector Permit Compliance & Trade-Study Annex .svg",
        icon: AppIcon::Printer,
    },
    SlashCommandSpec {
        cmd: "/relay",
        category: "Collab",
        description: "Connect or disconnect Live Peer IFCX Sync Relay (/relay wss://... | off)",
        icon: AppIcon::Pulse,
    },
    SlashCommandSpec {
        cmd: "/sync-peer",
        category: "Collab",
        description: "Synchronize incoming peer IFCX delta via Relay & 3-Way Merge",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/eigengate",
        category: "Safety",
        description: "Run Branch 1 EigenGate Linear Pre-Analysis & SVD Kinematic Guard screening",
        icon: AppIcon::Shield,
    },
    SlashCommandSpec {
        cmd: "/eigenflow",
        category: "Safety",
        description: "Run Branch 2 EigenFlow Non-Linear Woodbury P1->P2 screening & open Studio",
        icon: AppIcon::Bolt,
    },
    SlashCommandSpec {
        cmd: "/eigenflow-studio",
        category: "Analysis",
        description: "Open the 2-tab EigenFlow Non-Linear Studio drawer (Woodbury & ADRS)",
        icon: AppIcon::ChartLine,
    },
    SlashCommandSpec {
        cmd: "/pushover",
        category: "Analysis",
        description: "Run P-Delta Pushover analysis (/pushover modal | uniform | triangular)",
        icon: AppIcon::ChartLine,
    },
    SlashCommandSpec {
        cmd: "/adrs",
        category: "Analysis",
        description: "Open ATC-40 / ASCE 41-17 Pushover & ADRS Capacity Spectrum tab",
        icon: AppIcon::Target,
    },
    SlashCommandSpec {
        cmd: "/eigenflow-3d",
        category: "Analysis",
        description: "Toggle 3D Pure WebGPU EigenFlow P1->P2 flow conduits & Pushover hinges",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/eigenflow-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::eigenflow IFCX layer (.ifcx)",
        icon: AppIcon::Document,
    },
    SlashCommandSpec {
        cmd: "/eigenflow-svg",
        category: "Export",
        description: "Export A3 Sheet S-104 Pure-Vector EigenFlow Pushover, ADRS & Woodbury Annex .svg",
        icon: AppIcon::Printer,
    },
    SlashCommandSpec {
        cmd: "/extreme",
        category: "Safety",
        description: "Open Extreme Events Studio (Blast UFC 3-340-02 & Structural Fire ISO 834/EC)",
        icon: AppIcon::AlertTriangle,
    },
    SlashCommandSpec {
        cmd: "/blast",
        category: "Safety",
        description: "Run Kingery-Bulmash Blast & EigenFlow coupling (/blast <kg> <standoff_m>)",
        icon: AppIcon::Target,
    },
    SlashCommandSpec {
        cmd: "/fire",
        category: "Safety",
        description: "Run ISO 834 / Eurocode Structural Fire & thermal EigenFlow (/fire <minutes>)",
        icon: AppIcon::Bolt,
    },
    SlashCommandSpec {
        cmd: "/fire-story",
        category: "Safety",
        description: "Select ignited building story for 3D Structural Fire compartment (/fire-story <1..n>)",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/extreme-3d",
        category: "Safety",
        description: "Toggle 3D Pure WebGPU Blast shockwave rings & Fire thermal plume",
        icon: AppIcon::Shield,
    },
    SlashCommandSpec {
        cmd: "/extreme-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::hazard::extreme_events IFCX layer (.ifcx)",
        icon: AppIcon::Document,
    },
    SlashCommandSpec {
        cmd: "/extreme-svg",
        category: "Export",
        description: "Export A3 Sheet S-105 Pure-Vector Extreme Events Blast & Fire Annex .svg",
        icon: AppIcon::Printer,
    },
    SlashCommandSpec {
        cmd: "/isolation",
        category: "Safety",
        description: "Open Seismic Base Isolation & Damper 3D Studio (ASCE 7-22 Ch. 17-18)",
        icon: AppIcon::Shield,
    },
    SlashCommandSpec {
        cmd: "/lrb",
        category: "Safety",
        description: "Evaluate Lead-Rubber Bearing (LRB) base isolation & moat gap",
        icon: AppIcon::Shield,
    },
    SlashCommandSpec {
        cmd: "/fps",
        category: "Safety",
        description: "Evaluate Friction Pendulum System (FPS) sliding base isolation",
        icon: AppIcon::Target,
    },
    SlashCommandSpec {
        cmd: "/fvd",
        category: "Safety",
        description: "Evaluate Fluid Viscous Damper (FVD) supplementary damping",
        icon: AppIcon::Bolt,
    },
    SlashCommandSpec {
        cmd: "/isolation-3d",
        category: "Safety",
        description: "Toggle 3D Pure WebGPU base isolator pads, moat ring & FVD dampers",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/isolation-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::analysis::seismic_isolation IFCX (.ifcx)",
        icon: AppIcon::Document,
    },
    SlashCommandSpec {
        cmd: "/isolation-svg",
        category: "Export",
        description: "Export A3 Sheet S-106 Pure-Vector Base Isolation & Damper Annex .svg",
        icon: AppIcon::Printer,
    },
    SlashCommandSpec {
        cmd: "/wind-studio",
        category: "Wind",
        description: "Open Wind Aeroelasticity & ISO 10137 Comfort 3D Studio (ASCE 7-22 Ch. 26-30)",
        icon: AppIcon::Wind,
    },
    SlashCommandSpec {
        cmd: "/wind-dir",
        category: "Wind",
        description: "Set basic wind speed (m/s) and/or azimuth deg (/wind-dir 48 270)",
        icon: AppIcon::Wind,
    },
    SlashCommandSpec {
        cmd: "/wind-exposure",
        category: "Wind",
        description: "Set terrain exposure category (/wind-exposure B | C | D)",
        icon: AppIcon::Wind,
    },
    SlashCommandSpec {
        cmd: "/wind-3d",
        category: "Wind",
        description: "Toggle 3D Pure WebGPU facade cladding pressures & wind flow vectors",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/wind-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::analysis::wind_aeroelasticity IFCX (.ifcx)",
        icon: AppIcon::Document,
    },
    SlashCommandSpec {
        cmd: "/wind-svg",
        category: "Export",
        description: "Export A3 Sheet S-107 Pure-Vector Wind & ISO 10137 Comfort Annex .svg",
        icon: AppIcon::Printer,
    },
    SlashCommandSpec {
        cmd: "/shear-wall",
        category: "Safety",
        description: "Open Special RC Shear Wall & Boundary Element (SBE) 3D Studio (ACI 318-19 §18.10)",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/sbe",
        category: "Safety",
        description: "Evaluate Special Boundary Element confinement & thickness (/sbe 350 35)",
        icon: AppIcon::Shield,
    },
    SlashCommandSpec {
        cmd: "/coupling-beam",
        category: "Safety",
        description: "Configure diagonally reinforced X-Cage coupling beam (/coupling-beam 1.6 0.95)",
        icon: AppIcon::Target,
    },
    SlashCommandSpec {
        cmd: "/shear-wall-3d",
        category: "Safety",
        description: "Toggle 3D Pure WebGPU Shear Wall SBE zones & X-Cage coupling beams",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/shear-wall-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::analysis::shear_wall_sbe IFCX (.ifcx)",
        icon: AppIcon::Document,
    },
    SlashCommandSpec {
        cmd: "/shear-wall-svg",
        category: "Export",
        description: "Export A3 Sheet S-108 Pure-Vector Shear Wall, SBE & Coupling Beam Annex .svg",
        icon: AppIcon::Printer,
    },
    SlashCommandSpec {
        cmd: "/connection",
        category: "Safety",
        description: "Open Steel Seismic Connection, Bolts & Panel-Zone 3D Studio (AISC 358/360)",
        icon: AppIcon::Wrench,
    },
    SlashCommandSpec {
        cmd: "/endplate",
        category: "Safety",
        description: "Evaluate 4E Extended End-Plate moment connection (/endplate 24 24)",
        icon: AppIcon::Wrench,
    },
    SlashCommandSpec {
        cmd: "/shear-tab",
        category: "Safety",
        description: "Evaluate Single-Plate Shear Tab pinned connection (/shear-tab 20 12)",
        icon: AppIcon::Wrench,
    },
    SlashCommandSpec {
        cmd: "/connection-3d",
        category: "Safety",
        description: "Toggle 3D Pure WebGPU steel joint plates, bolt pins & panel-zone halos",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/connection-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::analysis::steel_connection IFCX (.ifcx)",
        icon: AppIcon::Document,
    },
    SlashCommandSpec {
        cmd: "/connection-svg",
        category: "Export",
        description: "Export A3 Sheet S-109 Pure-Vector Steel Connection & Panel-Zone Annex .svg",
        icon: AppIcon::Printer,
    },
    SlashCommandSpec {
        cmd: "/prestress",
        category: "Analysis",
        description: "Open Post-Tensioned Tendon Drape, Losses & Camber 3D Studio (ACI 318 Ch. 24)",
        icon: AppIcon::ChartLine,
    },
    SlashCommandSpec {
        cmd: "/tendon",
        category: "Analysis",
        description: "Configure PT tendon drape profile & strands (/tendon parabolic 20)",
        icon: AppIcon::ChartLine,
    },
    SlashCommandSpec {
        cmd: "/pt-losses",
        category: "Analysis",
        description: "Inspect immediate & time-dependent prestress loss waterfall & camber",
        icon: AppIcon::Bolt,
    },
    SlashCommandSpec {
        cmd: "/prestress-3d",
        category: "Analysis",
        description: "Toggle 3D Pure WebGPU draped tendon ducts & anchorage blocks",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/prestress-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::analysis::prestress_tendon IFCX (.ifcx)",
        icon: AppIcon::Document,
    },
    SlashCommandSpec {
        cmd: "/prestress-svg",
        category: "Export",
        description: "Export A3 Sheet S-110 Pure-Vector PT Tendon Drape & Losses Annex .svg",
        icon: AppIcon::Printer,
    },
    SlashCommandSpec {
        cmd: "/composite",
        category: "Analysis",
        description: "Open Composite Steel-Concrete Deck & AISC DG11 Vibration 3D Studio",
        icon: AppIcon::Grid,
    },
    SlashCommandSpec {
        cmd: "/studs",
        category: "Analysis",
        description: "Configure headed shear stud count & rib pattern (/studs 48 perpendicular)",
        icon: AppIcon::Wrench,
    },
    SlashCommandSpec {
        cmd: "/floor-vib",
        category: "Analysis",
        description: "Inspect AISC Design Guide 11 walking vibration & L/360 deflection audit",
        icon: AppIcon::ChartLine,
    },
    SlashCommandSpec {
        cmd: "/composite-3d",
        category: "Analysis",
        description: "Toggle 3D Pure WebGPU composite slab strips & headed shear stud pins",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/composite-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::analysis::composite_deck IFCX (.ifcx)",
        icon: AppIcon::Document,
    },
    SlashCommandSpec {
        cmd: "/composite-svg",
        category: "Export",
        description: "Export A3 Sheet S-111 Pure-Vector Composite Deck & DG11 Vibration Annex .svg",
        icon: AppIcon::Printer,
    },
    SlashCommandSpec {
        cmd: "/reliability",
        category: "Analysis",
        description: "Open Structural Reliability FORM/SORM Beta-Index & Sensitivity 3D Studio",
        icon: AppIcon::Target,
    },
    SlashCommandSpec {
        cmd: "/form",
        category: "Analysis",
        description: "Run Rackwitz-Fiessler Hasofer-Lind FORM reliability index (/form 3.5)",
        icon: AppIcon::ChartLine,
    },
    SlashCommandSpec {
        cmd: "/sorm",
        category: "Analysis",
        description: "Run Breitung curvature-corrected SORM reliability index (/sorm 3.8)",
        icon: AppIcon::ChartLine,
    },
    SlashCommandSpec {
        cmd: "/fragility",
        category: "Analysis",
        description: "Inspect FEMA P-58 multi-state fragility curves & sensitivity alpha^2 tornado",
        icon: AppIcon::ChartLine,
    },
    SlashCommandSpec {
        cmd: "/reliability-3d",
        category: "Analysis",
        description: "Toggle 3D Pure WebGPU structural member beta contours & risk hotspot beacons",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/reliability-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::analysis::reliability_form IFCX (.ifcx)",
        icon: AppIcon::Document,
    },
    SlashCommandSpec {
        cmd: "/reliability-svg",
        category: "Export",
        description: "Export A3 Sheet S-112 Pure-Vector Reliability FORM/SORM & Fragility Annex .svg",
        icon: AppIcon::Printer,
    },
    SlashCommandSpec {
        cmd: "/foundation",
        category: "Geotech",
        description: "Open Deep Foundation Pile Group, Winkler Springs & Punching Shear 3D Studio",
        icon: AppIcon::Hammer,
    },
    SlashCommandSpec {
        cmd: "/pile-group",
        category: "Geotech",
        description: "Size pile group layout & embedment (/pile-group 3x3 24m soft)",
        icon: AppIcon::Hammer,
    },
    SlashCommandSpec {
        cmd: "/punching",
        category: "Geotech",
        description: "Audit ACI 318-19 / SNI 2847 two-way punching shear around column base",
        icon: AppIcon::Shield,
    },
    SlashCommandSpec {
        cmd: "/winkler",
        category: "Geotech",
        description: "Inspect 6-DOF Winkler subgrade springs & Wolf/Veletsos-Meek SSI shift",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/foundation-3d",
        category: "Geotech",
        description: "Toggle 3D Pure WebGPU deep pile caps, pile shafts & Winkler spring nodes",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/foundation-ifcx",
        category: "Export",
        description: "Export non-destructive eigengraph::analysis::deep_foundation IFCX (.ifcx)",
        icon: AppIcon::Document,
    },
    SlashCommandSpec {
        cmd: "/foundation-svg",
        category: "Export",
        description: "Export A3 Sheet S-113 Pure-Vector Deep Foundation & Punching Annex .svg",
        icon: AppIcon::Printer,
    },
    SlashCommandSpec {
        cmd: "/bbs",
        category: "Field",
        description: "Run 1D Rebar BBS Cutting-Stock Optimization (/bbs 12 | 6 m stock bars)",
        icon: AppIcon::Wrench,
    },
    SlashCommandSpec {
        cmd: "/cutlist",
        category: "Field",
        description: "Open Field QA/QC & Rebar BBS Cut-List visualizer drawer",
        icon: AppIcon::Layers,
    },
    SlashCommandSpec {
        cmd: "/qaqc-3d",
        category: "Field",
        description: "Toggle 3D Pure WebGPU Field QA/QC Pour-Strips & NCR Beacon Pins",
        icon: AppIcon::Target,
    },
    SlashCommandSpec {
        cmd: "/waypoint",
        category: "Field",
        description: "Fly 3D Earth camera to next Field QA/QC ITP / NCR spatial waypoint",
        icon: AppIcon::Compass,
    },
    SlashCommandSpec {
        cmd: "/shop-svg",
        category: "Export",
        description: "Export A3 Sheet S-103 Pure-Vector Rebar BBS & Field QA/QC Shop Annex .svg",
        icon: AppIcon::Printer,
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
    pub open_accelerogram_file_picker: bool,
    pub export_fragility_loss_ifcx: bool,
    pub run_shm_digital_twin: bool,
    pub open_shm_telemetry_picker: bool,
    pub export_shm_atc20_ifcx: bool,
    pub run_evacuation_routing: bool,
    pub export_evacuation_ifcx: bool,
    pub connect_shm_ws: Option<String>,
    pub disconnect_shm_ws: bool,
    pub export_watchdog_ifcx: bool,
    pub run_gpu_monte_carlo: Option<u32>,
    pub export_monte_carlo_ifcx: bool,
    pub create_checkpoint_label: Option<String>,
    pub approve_checkpoint: bool,
    pub run_ifcx_three_way_merge: bool,
    pub refresh_district_risk_field: bool,
    pub export_collab_bundle_ifcx: bool,
    pub run_trade_study: Option<crate::core::trade_study::TradeStudyObjective>,
    pub adopt_trade_study_scenario: Option<crate::core::trade_study::TradeScenarioArchetype>,
    pub export_trade_study_ifcx: bool,
    pub run_permit_compliance: bool,
    pub export_permit_annex_svg: bool,
    pub open_collab_bundle_picker: bool,
    pub connect_collab_relay: Option<String>,
    pub disconnect_collab_relay: bool,
    pub sync_peer_relay_delta: bool,
    pub run_field_fabrication: Option<f64>,
    pub toggle_field_qaqc_3d: bool,
    pub fly_to_waypoint_idx: Option<usize>,
    pub clear_waypoint_hold: Option<String>,
    pub log_field_ncr_quick: bool,
    pub export_shop_drawing_svg: bool,
    pub export_bbs_csv: bool,
    pub export_cutlist_csv: bool,
    pub export_field_qaqc_ifcx: bool,
    // Phase 83: EigenFlow Non-Linear Studio
    pub run_eigenflow_studio: bool,
    pub toggle_eigenflow_3d: bool,
    pub export_eigenflow_ifcx: bool,
    pub export_eigenflow_svg: bool,
    pub eigenflow_select_element: Option<usize>,
    pub eigenflow_set_lambda: Option<f64>,
    pub eigenflow_set_pattern: Option<crate::core::pushover::LateralLoadPattern>,
    // Phase 84: Extreme Events Studio (Blast & Structural Fire)
    pub run_extreme_events: bool,
    pub toggle_extreme_events_3d: bool,
    pub export_extreme_events_ifcx: bool,
    pub export_extreme_events_svg: bool,
    pub extreme_send_to_eigenflow: Option<(usize, f64)>,
    // Update v0.8.4.1 (1/10 -> v0.8.5): Base Isolation & Damper Studio
    pub run_isolation_studio: bool,
    pub toggle_isolation_3d: bool,
    pub export_isolation_ifcx: bool,
    pub export_isolation_svg: bool,
    // Update v0.8.4.2 (2/10 -> v0.8.5): Wind Aeroelasticity & ISO 10137 Comfort Studio
    pub run_wind_studio: bool,
    pub toggle_wind_3d: bool,
    pub export_wind_ifcx: bool,
    pub export_wind_svg: bool,
    // Update v0.8.4.3 (3/10 -> v0.8.5): Special RC Shear Wall, SBE & Coupling Beam Studio
    pub run_shear_wall_studio: bool,
    pub toggle_shear_wall_3d: bool,
    pub export_shear_wall_ifcx: bool,
    pub export_shear_wall_svg: bool,
    // Update v0.8.4.4 (4/10 -> v0.8.5): Steel Seismic Connection & Panel-Zone Studio
    pub run_connection_studio: bool,
    pub toggle_connection_3d: bool,
    pub export_connection_ifcx: bool,
    pub export_connection_svg: bool,
    // Update v0.8.4.5 (5/10 -> v0.8.5): Post-Tensioned Tendon Drape & Losses Studio
    pub run_prestress_studio: bool,
    pub toggle_prestress_3d: bool,
    pub export_prestress_ifcx: bool,
    pub export_prestress_svg: bool,
    // Update v0.8.4.6 (6/10 -> v0.8.5): Composite Deck, Studs & AISC DG11 Studio
    pub run_composite_studio: bool,
    pub toggle_composite_3d: bool,
    pub export_composite_ifcx: bool,
    pub export_composite_svg: bool,
    // Update v0.8.4.7 (7/10 -> v0.8.5): Structural Reliability FORM/SORM & Fragility Studio
    pub run_reliability_studio: bool,
    pub toggle_reliability_3d: bool,
    pub export_reliability_ifcx: bool,
    pub export_reliability_svg: bool,
    // Update v0.8.4.8 (8/10 -> v0.8.5): Deep Foundation Pile Group & Punching Studio
    pub run_foundation_studio: bool,
    pub toggle_foundation_3d: bool,
    pub export_foundation_ifcx: bool,
    pub export_foundation_svg: bool,
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
        self.open_accelerogram_file_picker |= other.open_accelerogram_file_picker;
        self.export_fragility_loss_ifcx |= other.export_fragility_loss_ifcx;
        self.run_shm_digital_twin |= other.run_shm_digital_twin;
        self.open_shm_telemetry_picker |= other.open_shm_telemetry_picker;
        self.export_shm_atc20_ifcx |= other.export_shm_atc20_ifcx;
        self.run_evacuation_routing |= other.run_evacuation_routing;
        self.export_evacuation_ifcx |= other.export_evacuation_ifcx;
        if other.connect_shm_ws.is_some() {
            self.connect_shm_ws = other.connect_shm_ws;
        }
        self.disconnect_shm_ws |= other.disconnect_shm_ws;
        self.export_watchdog_ifcx |= other.export_watchdog_ifcx;
        if other.run_gpu_monte_carlo.is_some() {
            self.run_gpu_monte_carlo = other.run_gpu_monte_carlo;
        }
        self.export_monte_carlo_ifcx |= other.export_monte_carlo_ifcx;
        if other.create_checkpoint_label.is_some() {
            self.create_checkpoint_label = other.create_checkpoint_label;
        }
        self.approve_checkpoint |= other.approve_checkpoint;
        self.run_ifcx_three_way_merge |= other.run_ifcx_three_way_merge;
        self.refresh_district_risk_field |= other.refresh_district_risk_field;
        self.export_collab_bundle_ifcx |= other.export_collab_bundle_ifcx;
        if other.run_trade_study.is_some() {
            self.run_trade_study = other.run_trade_study;
        }
        if other.adopt_trade_study_scenario.is_some() {
            self.adopt_trade_study_scenario = other.adopt_trade_study_scenario;
        }
        self.export_trade_study_ifcx |= other.export_trade_study_ifcx;
        self.run_permit_compliance |= other.run_permit_compliance;
        self.export_permit_annex_svg |= other.export_permit_annex_svg;
        self.open_collab_bundle_picker |= other.open_collab_bundle_picker;
        if other.connect_collab_relay.is_some() {
            self.connect_collab_relay = other.connect_collab_relay;
        }
        self.disconnect_collab_relay |= other.disconnect_collab_relay;
        self.sync_peer_relay_delta |= other.sync_peer_relay_delta;
        self.run_eigenflow_studio |= other.run_eigenflow_studio;
        self.toggle_eigenflow_3d |= other.toggle_eigenflow_3d;
        self.export_eigenflow_ifcx |= other.export_eigenflow_ifcx;
        self.export_eigenflow_svg |= other.export_eigenflow_svg;
        if other.eigenflow_select_element.is_some() {
            self.eigenflow_select_element = other.eigenflow_select_element;
        }
        if other.eigenflow_set_lambda.is_some() {
            self.eigenflow_set_lambda = other.eigenflow_set_lambda;
        }
        if other.eigenflow_set_pattern.is_some() {
            self.eigenflow_set_pattern = other.eigenflow_set_pattern;
        }
        self.run_extreme_events |= other.run_extreme_events;
        self.toggle_extreme_events_3d |= other.toggle_extreme_events_3d;
        self.export_extreme_events_ifcx |= other.export_extreme_events_ifcx;
        self.export_extreme_events_svg |= other.export_extreme_events_svg;
        if other.extreme_send_to_eigenflow.is_some() {
            self.extreme_send_to_eigenflow = other.extreme_send_to_eigenflow;
        }
        self.run_isolation_studio |= other.run_isolation_studio;
        self.toggle_isolation_3d |= other.toggle_isolation_3d;
        self.export_isolation_ifcx |= other.export_isolation_ifcx;
        self.export_isolation_svg |= other.export_isolation_svg;
        self.run_wind_studio |= other.run_wind_studio;
        self.toggle_wind_3d |= other.toggle_wind_3d;
        self.export_wind_ifcx |= other.export_wind_ifcx;
        self.export_wind_svg |= other.export_wind_svg;
        self.run_shear_wall_studio |= other.run_shear_wall_studio;
        self.toggle_shear_wall_3d |= other.toggle_shear_wall_3d;
        self.export_shear_wall_ifcx |= other.export_shear_wall_ifcx;
        self.export_shear_wall_svg |= other.export_shear_wall_svg;
        self.run_connection_studio |= other.run_connection_studio;
        self.toggle_connection_3d |= other.toggle_connection_3d;
        self.export_connection_ifcx |= other.export_connection_ifcx;
        self.export_connection_svg |= other.export_connection_svg;
        self.run_prestress_studio |= other.run_prestress_studio;
        self.toggle_prestress_3d |= other.toggle_prestress_3d;
        self.export_prestress_ifcx |= other.export_prestress_ifcx;
        self.export_prestress_svg |= other.export_prestress_svg;
        self.run_composite_studio |= other.run_composite_studio;
        self.toggle_composite_3d |= other.toggle_composite_3d;
        self.export_composite_ifcx |= other.export_composite_ifcx;
        self.export_composite_svg |= other.export_composite_svg;
        self.run_reliability_studio |= other.run_reliability_studio;
        self.toggle_reliability_3d |= other.toggle_reliability_3d;
        self.export_reliability_ifcx |= other.export_reliability_ifcx;
        self.export_reliability_svg |= other.export_reliability_svg;
        self.run_foundation_studio |= other.run_foundation_studio;
        self.toggle_foundation_3d |= other.toggle_foundation_3d;
        self.export_foundation_ifcx |= other.export_foundation_ifcx;
        self.export_foundation_svg |= other.export_foundation_svg;
    }
}
