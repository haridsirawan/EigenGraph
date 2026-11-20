//! features.rs — Centralized Feature Gate & Unlock Roadmap Registry for EigenGraph.
//! Catalogs active engineering capabilities and upcoming Phase 32–35 locked workspace slots
//! across all 15 BIM/Structural domains so layouts stay frozen while capabilities unlock cleanly.

use crate::models::NavigationPillar;

/// Deployment status of an EigenGraph domain capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureStatus {
    /// Fully wired and interactive in the current build.
    Active,
    /// Workspace layout and state contracts are in place, gated until the target unlock phase.
    Locked { target_phase: &'static str },
}

impl FeatureStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    pub fn target_phase(&self) -> Option<&'static str> {
        match self {
            Self::Active => None,
            Self::Locked { target_phase } => Some(target_phase),
        }
    }

    pub fn badge_label(&self) -> String {
        match self {
            Self::Active => "ACTIVE".to_string(),
            Self::Locked { target_phase } => format!("LOCKED ({target_phase})"),
        }
    }
}

/// Canonical identifier for every tracked capability and unlockable slot across the 15 pillars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureId {
    // 1. My Library & Vault
    VaultPortfolioManager,
    GlobalBimFamilyCatalog,
    BrowserIndexedDbPersistence,
    CdeIso19650DocumentHub,
    MultiTypologyProjectTemplates,

    // 2. AI Agent & Virtual Firm
    VirtualEngineeringFirmBoard,
    AutonomousCopilotCalculators,
    LiveGeminiToolOrchestrator,
    AutomatedCodeComplianceAuditor,
    ParetoMultiObjectiveOptimizer,

    // 3. Research & 1D GIS
    SlippyMapWebMercator,
    GeodeticCrsUtmProjection,
    DemTinMeshCutFill,
    LiveRasterMapTileFetcher,
    GeotechBoreholeStratigraphy,
    HydrologyIdfFloodplain,

    // 4. Parametric & 2D Studio
    Planar2DCadAuthoring,
    LiveSectionCutPoche,
    DynamoAlgorithmicStudio,
    ParametricBidirectionalSync,
    AutomatedRcRebarBbsDrafter,
    BuildingElevationsMepRiser,

    // 5. Geometry & 3D BIM
    CsgBrep3DViewport,
    BimMasterRibbonAndBrowser,
    Bcf21CoordinationHub,
    LiveBvhGeometricClashSolver,
    FederatedLayersAxisSlicer,
    PointCloudAsBuiltDeviation,

    // 6. Analysis & Multi-Hazard Simulation
    LinearFemAndSvdAri,
    SeismicModalPushoverNltha,
    MemberCodeDesignSuite,
    ExtremeBlastFireFemaP58,
    LiveBimToFemAutoSync,
    BridgeMovingLoadInfluence,
    RetainingWallSlopeStability,
    FloorVibrationSteelFatigue,

    // 7. Time & 4D Scheduling
    Synchro4DErectionPlayer,
    BimDriven4DCriticalPathSync,
    MonteCarloScheduleRiskPert,
    ResourceLevelingCrewHistogram,

    // 8. Cost & 5D Budgeting
    Parametric5DRabMultiCurrency,
    LiveBimQtoBoqSync,
    EarnedValueEvmProgressClaim,
    ValueEngineeringTenderMatrix,

    // 9. Sustainability & 6D Energy/LCA
    WholeLifeCarbonEn15978,
    BimMaterialLcaAutoSync,
    SolarOttvThermalDaylight,
    CircularMaterialPassportDfd,

    // 10. Facility Management & 7D Digital Twin
    DigitalTwinShmCobie3,
    LiveIotTelemetryIngestion,
    MepIsolationSpaceUtilization,
    DroneDefectCrackMapper,

    // 11. Safety & 8D Site Risk
    TowerCraneFallRiskRadar,
    SitePerimeter3DCraneCollision,
    ScaffoldingFormworkReshoring,
    PermitToWorkEvacuationDrill,

    // 12. Lean Construction & 9D Logistics
    CuttingStock1DAndLastPlanner,
    LiveRebarBbsCuttingSync,
    TaktTimeConcretePourDispatch,
    RfiSubmittalSlaTracker,

    // 13. Industrialized & 10D DfMA / OpenBIM
    OpenBimIfcReaderEditorClashDiff,
    DfmaSupplyChainTracker,
    WasmLocalIfcFileDropPicker,
    PrecastTrailerJointTolerance,
    Concrete3DPrintingGcodeToolpath,

    // 14. Prediction & AI Structural Aging
    FosmReliabilityProgressiveCollapse,
    LiveShmAnomalyForecasting,
    ChlorideCorrosionDurability,
    PostEarthquakeAtc20Tagging,

    // 15. Report & Publishing Suite
    DedDrawingSheetsPdfDxfSvg,
    LivingDocsSheetsSlides,
    AutoProjectedBimDedSheets,
    RegulatoryCalcBookPbgComposer,
    AsBuiltHandoverDossierPackager,
}

/// Metadata record describing one capability inside the centralized Feature Registry.
#[derive(Debug, Clone)]
pub struct FeatureEntry {
    pub id: FeatureId,
    pub pillar: NavigationPillar,
    pub code: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub status: FeatureStatus,
    pub module_hint: &'static str,
    pub prioritized: bool,
}

/// Centralized catalog of active and unlockable capabilities for Settings & inline feature gates.
#[derive(Debug, Clone)]
pub struct FeatureRegistry {
    pub entries: Vec<FeatureEntry>,
}

impl Default for FeatureRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FeatureRegistry {
    pub fn new() -> Self {
        use FeatureStatus::Active;
        let entries = vec![
            // =================================================================
            // 1. My Library & Vault (2 Active + 3 Locked = 5)
            // =================================================================
            FeatureEntry {
                id: FeatureId::VaultPortfolioManager,
                pillar: NavigationPillar::MyLibrary,
                code: "LIB-01",
                title: "Portfolio Vault & Audit Certificate",
                description: "Local project archive, SHA-256 dossier fingerprinting, and standalone 3D HTML5 viewer export.",
                status: Active,
                module_hint: "src/core/vault.rs + src/views/my_project.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::GlobalBimFamilyCatalog,
                pillar: NavigationPillar::MyLibrary,
                code: "LIB-02",
                title: "Global BIM Family & Parameter Catalog",
                description: "Revit RFA / buildingSMART IFC parameter browser across MasterFormat & UniFormat families.",
                status: Active,
                module_hint: "src/views/my_project.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::BrowserIndexedDbPersistence,
                pillar: NavigationPillar::MyLibrary,
                code: "LIB-03",
                title: "Browser IndexedDB & LocalStorage Vault Sync",
                description: "Automatic client-side WASM persistence across browser reloads with JSON archive import/export.",
                status: Active,
                module_hint: "src/core/web_bridge.rs + src/core/vault.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::CdeIso19650DocumentHub,
                pillar: NavigationPillar::MyLibrary,
                code: "LIB-04",
                title: "ISO 19650 Common Data Environment (CDE) Hub",
                description: "WIP, Shared, Published, and Archive information containers with suitability codes (S0–A4) and transmittal sign-off.",
                status: Active,
                module_hint: "src/views/my_project.rs + src/core/vault.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::MultiTypologyProjectTemplates,
                pillar: NavigationPillar::MyLibrary,
                code: "LIB-05",
                title: "Multi-Typology Structural Project Templates",
                description: "One-click seed launchers for Long-Span Steel Trusses, Dual-System RC Towers, PT Viaducts, and Precast Logistics.",
                status: Active,
                module_hint: "src/views/my_project.rs + src/core/parametric.rs",
                prioritized: false,
            },

            // =================================================================
            // 2. AI Agent & Virtual Firm (2 Active + 3 Locked = 5)
            // =================================================================
            FeatureEntry {
                id: FeatureId::VirtualEngineeringFirmBoard,
                pillar: NavigationPillar::AiAgent,
                code: "AI-01",
                title: "Multi-Agent Virtual Engineering Firm",
                description: "PM, Senior Drafter, FEA Analyst, and QS Specialist organigram, collaboration log, and telemetry.",
                status: Active,
                module_hint: "src/views/ai_firm.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::AutonomousCopilotCalculators,
                pillar: NavigationPillar::AiAgent,
                code: "AI-02",
                title: "Autonomous Engineering Copilot Studio",
                description: "Deterministic ACI 318-19 Shear Wall, ASCE 7-22 Wind MWFRS, and ISO 14040 Carbon solvers.",
                status: Active,
                module_hint: "src/views/ai_firm.rs + src/models.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::LiveGeminiToolOrchestrator,
                pillar: NavigationPillar::AiAgent,
                code: "AI-03",
                title: "Live LLM Tool-Calling BIM Orchestrator",
                description: "Async Gemini REST integration executing validated BimCommand edits against the live structural model.",
                status: Active,
                module_hint: "src/core/web_bridge.rs + src/views/ai_assistant.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::AutomatedCodeComplianceAuditor,
                pillar: NavigationPillar::AiAgent,
                code: "AI-04",
                title: "Automated Multi-Code Compliance Auditor",
                description: "Clause-by-clause SNI 2847 / ACI 318 / AISC 360 / SNI 1726 sweep checking SCWB joints, drift caps, and detailing rules.",
                status: Active,
                module_hint: "src/views/ai_firm.rs + src/core/fem.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::ParetoMultiObjectiveOptimizer,
                pillar: NavigationPillar::AiAgent,
                code: "AI-05",
                title: "NSGA-II Multi-Objective Pareto Optimizer",
                description: "Simultaneous trade-off optimization across 5D Capex, A1–A3 Embodied Carbon, and global ARI redundancy.",
                status: Active,
                module_hint: "src/views/ai_firm.rs + src/core/parametric.rs",
                prioritized: false,
            },

            // =================================================================
            // 3. Research & 1D GIS (3 Active + 3 Locked = 6)
            // =================================================================
            FeatureEntry {
                id: FeatureId::SlippyMapWebMercator,
                pillar: NavigationPillar::ResearchConcept,
                code: "1D-01",
                title: "EPSG:3857 Slippy Map & Geodetic Pins",
                description: "Interactive Web Mercator quadtree viewport, seismic site pins, and offline vector terrain.",
                status: Active,
                module_hint: "src/core/gis/map_widget.rs + src/views/research.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::GeodeticCrsUtmProjection,
                pillar: NavigationPillar::ResearchConcept,
                code: "1D-02",
                title: "WGS84 / UTM CRS & IfcMapConversion Engine",
                description: "Forward/inverse UTM coordinate transforms, meridian convergence, and IFC4 georeferencing export.",
                status: Active,
                module_hint: "src/core/gis/crs_projection.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::DemTinMeshCutFill,
                pillar: NavigationPillar::ResearchConcept,
                code: "1D-03",
                title: "DEM Topographic TIN Mesh & Cut/Fill Grading",
                description: "10,082-triangle Delaunay terrain, 1m/5m contours, SNI 8460 >30 deg slope hazard shading, and earthwork balance.",
                status: Active,
                module_hint: "src/core/gis/tin_surface.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::LiveRasterMapTileFetcher,
                pillar: NavigationPillar::ResearchConcept,
                code: "1D-04",
                title: "Pure-Rust Vector Earth & OpenBIM IFC Site TIN Importer",
                description: "Zero-raster embedded global vector cartography, IFC 4.3 IfcSite TIN extraction, and offline CSV/XYZ survey ingestion.",
                status: Active,
                module_hint: "src/core/gis/world_vectors.rs + src/core/ifc.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::GeotechBoreholeStratigraphy,
                pillar: NavigationPillar::ResearchConcept,
                code: "1D-05",
                title: "Geotech Borehole Logs, SPT N60 & Vs30 Soil Stratigraphy",
                description: "Subsurface boring log inspector, SNI 8460 liquefaction screening, and end-bearing pile socket recommendations.",
                status: Active,
                module_hint: "src/views/research.rs + src/core/geotech.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::HydrologyIdfFloodplain,
                pillar: NavigationPillar::ResearchConcept,
                code: "1D-06",
                title: "Site Hydrology, IDF Rainfall Curves & Floodplain Datum",
                description: "100-year return-period IDF peak runoff (Q = C*I*A), detention sizing, and minimum finished floor elevation (FFE).",
                status: Active,
                module_hint: "src/views/research.rs + src/core/gis/",
                prioritized: false,
            },

            // =================================================================
            // 4. Parametric & 2D Studio (3 Active + 3 Locked = 6)
            // =================================================================
            FeatureEntry {
                id: FeatureId::Planar2DCadAuthoring,
                pillar: NavigationPillar::ParametricDED,
                code: "2D-01",
                title: "2D Vector CAD Plan & Half-Edge Room Engine",
                description: "Object snaps, compound walls, openings, beams, columns, rebar detailing, and automatic room bounding.",
                status: Active,
                module_hint: "src/views/parametric/canvas_2d.rs + src/core/bim.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::LiveSectionCutPoche,
                pillar: NavigationPillar::ParametricDED,
                code: "2D-02",
                title: "Live Section Cut Poche & Sheet A101 Composer",
                description: "Real-time vertical section slicing, rebar cut dots, and ISO Vector PDF/SVG/IFC4 sheet export.",
                status: Active,
                module_hint: "src/views/parametric/canvas_2d.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::DynamoAlgorithmicStudio,
                pillar: NavigationPillar::ParametricDED,
                code: "2D-03",
                title: "Visual Dynamo Node Graph & 3D Wireframe Preview",
                description: "Draggable parametric presets (Diagrid, Geodesic Dome, Voronoi Canopy) with 3D orbit preview & bake.",
                status: Active,
                module_hint: "src/views/parametric/dynamo_studio.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::ParametricBidirectionalSync,
                pillar: NavigationPillar::ParametricDED,
                code: "2D-04",
                title: "Bidirectional 2D/Dynamo <-> 3D BIM Bus Sync",
                description: "Shares one unified BimCommandBus between Parametric 2D/Dynamo authoring and Geometry 3D viewport.",
                status: Active,
                module_hint: "src/app.rs + src/core/bim.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::AutomatedRcRebarBbsDrafter,
                pillar: NavigationPillar::ParametricDED,
                code: "2D-05",
                title: "Automated RC Beam/Column Rebar Elevation & BBS Drafter",
                description: "Generates ACI 318 / SNI 2847 seismic confinement zones, lap splice zones, and bar-mark shape-code schedules.",
                status: Active,
                module_hint: "src/views/parametric/detailing_2d.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::BuildingElevationsMepRiser,
                pillar: NavigationPillar::ParametricDED,
                code: "2D-06",
                title: "Automated Building Elevations & MEP Riser Diagrams",
                description: "Projects N/E/S/W architectural elevations with storey levels and schematic HVAC/Chilled-Water/Fire riser stacks.",
                status: Active,
                module_hint: "src/views/parametric/detailing_2d.rs",
                prioritized: false,
            },

            // =================================================================
            // 5. Geometry & 3D BIM (3 Active + 3 Locked = 6)
            // =================================================================
            FeatureEntry {
                id: FeatureId::CsgBrep3DViewport,
                pillar: NavigationPillar::GeometryAssembly,
                code: "3D-01",
                title: "3D CSG B-Rep & Analytical Frame Viewport",
                description: "360-deg orbit canvas, NFD/SFD/BMD overlays, interactive section box, ViewCube, and 3D rebar cages.",
                status: Active,
                module_hint: "src/views/build/viewport_3d.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::BimMasterRibbonAndBrowser,
                pillar: NavigationPillar::GeometryAssembly,
                code: "3D-02",
                title: "12-Tab Multi-Discipline Ribbon & Spatial Browser",
                description: "Architecture, Structure, MEP, Civil, Sim, 4D-7D ribbon tools, IFC SBS tree, and Solibri quality audit.",
                status: Active,
                module_hint: "src/views/build/ribbon.rs + src/views/build/sidebar.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::Bcf21CoordinationHub,
                pillar: NavigationPillar::GeometryAssembly,
                code: "3D-03",
                title: "BCF 2.1 Issue Threads & Viewpoint Restorer",
                description: "Camera-captured BCF topics, review comments, priority triage, and BCF 2.1 XML export.",
                status: Active,
                module_hint: "src/views/build/clash_bcf.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::LiveBvhGeometricClashSolver,
                pillar: NavigationPillar::GeometryAssembly,
                code: "3D-04",
                title: "Live Bvh3D Spatial Clash Detection Engine",
                description: "Real-time BVH/AABB collision solver across active walls, columns, beams, MEP runs, and rebar cages.",
                status: Active,
                module_hint: "src/core/csg.rs + src/views/build/clash_bcf.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::FederatedLayersAxisSlicer,
                pillar: NavigationPillar::GeometryAssembly,
                code: "3D-05",
                title: "Federated Discipline Layers & Multi-Axis Section Box",
                description: "Independent Arch/Struct/MEP/Rebar visibility filters paired with orthogonal X/Y/Z clip-plane sliders.",
                status: Active,
                module_hint: "src/views/build/sidebar.rs + src/views/build/viewport_3d.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::PointCloudAsBuiltDeviation,
                pillar: NavigationPillar::GeometryAssembly,
                code: "3D-06",
                title: "E57/LAS LiDAR Point-Cloud vs BIM Deviation Inspector",
                description: "3D ICP scan-to-BIM overlay measuring structural plumbness, slab FF/FL flatness, and millimeter tolerances.",
                status: Active,
                module_hint: "src/views/build/bottom_deck.rs",
                prioritized: false,
            },

            // =================================================================
            // 6. Analysis & Multi-Hazard Simulation (4 Active + 4 Locked = 8)
            // =================================================================
            FeatureEntry {
                id: FeatureId::LinearFemAndSvdAri,
                pillar: NavigationPillar::AnalysisSimulation,
                code: "SIM-01",
                title: "Linear 3D FEA, SVD Kinematic Guard & ARI Matrix",
                description: "Sherman-Morrison/Woodbury N-1 & N-2 redundancy screening, SVD rank verification, and cascading collapse.",
                status: Active,
                module_hint: "src/views/analyze/core_fea.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::SeismicModalPushoverNltha,
                pillar: NavigationPillar::AnalysisSimulation,
                code: "SIM-02",
                title: "SNI 1726 Modal, Pushover CSM, NL-THA & Isolation",
                description: "Lanczos modal spectra, capacity spectrum pushover, Newmark nonlinear time-history, and LRB/FPS isolation.",
                status: Active,
                module_hint: "src/views/analyze/seismic_dynamic.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::MemberCodeDesignSuite,
                pillar: NavigationPillar::AnalysisSimulation,
                code: "SIM-03",
                title: "RC P-M, Steel Connection, Composite, PT & Shear Wall",
                description: "SNI/ACI/AISC code verifications across RC columns, steel joints, composite decks, PT girders, and shear walls.",
                status: Active,
                module_hint: "src/views/analyze/member_design.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::ExtremeBlastFireFemaP58,
                pillar: NavigationPillar::AnalysisSimulation,
                code: "SIM-04",
                title: "FEMA P-58 Loss, UFC 3-340 Blast & AISC Fire",
                description: "Probabilistic REDi loss recovery, Kingery-Bulmash SDOF blast, and coupled thermo-mechanical fire ratings.",
                status: Active,
                module_hint: "src/views/analyze/extreme_hazards.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::LiveBimToFemAutoSync,
                pillar: NavigationPillar::AnalysisSimulation,
                code: "SIM-05",
                title: "Live BimProject -> FemSolver Reactive Sync",
                description: "Automatically converts active 3D BIM geometry & 1D GIS seismic parameters into live FEA runs.",
                status: Active,
                module_hint: "src/core/bim.rs + src/views/analyze/mod.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::BridgeMovingLoadInfluence,
                pillar: NavigationPillar::AnalysisSimulation,
                code: "SIM-06",
                title: "AASHTO HL-93 / SNI 1725 Bridge Moving Load & Influence Lines",
                description: "Computes truck/tandem/lane influence line ordinates, impact factor IM, and live-load bending envelopes.",
                status: Active,
                module_hint: "src/views/analyze/extended_mechanics.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::RetainingWallSlopeStability,
                pillar: NavigationPillar::AnalysisSimulation,
                code: "SIM-07",
                title: "Retaining Wall & Bishop Circular Slope Stability (SNI 8460)",
                description: "Evaluates Mononobe-Okabe seismic earth pressure, overturning/sliding checks, and method-of-slices slip FS.",
                status: Active,
                module_hint: "src/views/analyze/extended_mechanics.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::FloorVibrationSteelFatigue,
                pillar: NavigationPillar::AnalysisSimulation,
                code: "SIM-08",
                title: "AISC DG-11 Floor Vibration & EN 1993-1-9 Steel Fatigue",
                description: "Checks footfall resonance peak acceleration (%g) and cyclic Miner cumulative S-N fatigue life.",
                status: Active,
                module_hint: "src/views/analyze/extended_mechanics.rs",
                prioritized: false,
            },

            // =================================================================
            // 7. Time & 4D Scheduling (1 Active + 3 Locked = 4)
            // =================================================================
            FeatureEntry {
                id: FeatureId::Synchro4DErectionPlayer,
                pillar: NavigationPillar::TimeScheduling,
                code: "4D-01",
                title: "4D Synchro Construction Player & SVD Staging",
                description: "Day 1-360 construction elevation animation, crane progression, CPM tracking, and staged SVD safety.",
                status: Active,
                module_hint: "src/views/planning.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::BimDriven4DCriticalPathSync,
                pillar: NavigationPillar::TimeScheduling,
                code: "4D-02",
                title: "Element-Linked 4D WBS & 3D Erection Coupling",
                description: "Binds WBS tasks directly to active BimProject element IDs to isolate active erection phases in 3D.",
                status: Active,
                module_hint: "src/core/schedule.rs + src/views/planning.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::MonteCarloScheduleRiskPert,
                pillar: NavigationPillar::TimeScheduling,
                code: "4D-03",
                title: "PERT 3-Point & Monte Carlo Schedule Risk Simulation",
                description: "1,000-trial probabilistic completion S-curve (P50/P80/P95) and critical-path sensitivity tornado ranking.",
                status: Active,
                module_hint: "src/views/planning.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::ResourceLevelingCrewHistogram,
                pillar: NavigationPillar::TimeScheduling,
                code: "4D-04",
                title: "Daily Site Crew Histogram & Resource Leveling Engine",
                description: "Smooths ironworker, formwork carpenter, and crane-hour demand spikes within available float windows.",
                status: Active,
                module_hint: "src/views/planning.rs",
                prioritized: false,
            },

            // =================================================================
            // 8. Cost & 5D Budgeting (1 Active + 3 Locked = 4)
            // =================================================================
            FeatureEntry {
                id: FeatureId::Parametric5DRabMultiCurrency,
                pillar: NavigationPillar::CostBudgeting,
                code: "5D-01",
                title: "5D Dynamic RAB, Multi-Currency WBS & Cost-ARI",
                description: "Parametric AHSP pricing across IDR/USD/EUR/SGD, MasterFormat WBS ledger, and Cost-vs-ARI trade-off.",
                status: Active,
                module_hint: "src/views/planning.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::LiveBimQtoBoqSync,
                pillar: NavigationPillar::CostBudgeting,
                code: "5D-02",
                title: "Live BimProject Volume & Rebar QTO Ledger Sync",
                description: "Extracts concrete, structural steel, wall area, and rebar tonnage dynamically from active 3D BIM.",
                status: Active,
                module_hint: "src/core/cost.rs + src/views/planning.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::EarnedValueEvmProgressClaim,
                pillar: NavigationPillar::CostBudgeting,
                code: "5D-03",
                title: "Earned Value Management (EVM) & Progress Claim S-Curve",
                description: "Tracks PV, EV, AC, CPI/SPI variance, Estimate at Completion (EAC), and monthly payment certificates.",
                status: Active,
                module_hint: "src/views/planning.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::ValueEngineeringTenderMatrix,
                pillar: NavigationPillar::CostBudgeting,
                code: "5D-04",
                title: "Value Engineering Trade Study & Tender Bid Leveling",
                description: "Compares Owner Engineer Estimate vs vendor unit rates and quantifies structural system VE savings.",
                status: Active,
                module_hint: "src/views/planning.rs",
                prioritized: false,
            },

            // =================================================================
            // 9. Sustainability & 6D Energy/LCA (1 Active + 3 Locked = 4)
            // =================================================================
            FeatureEntry {
                id: FeatureId::WholeLifeCarbonEn15978,
                pillar: NavigationPillar::SustainabilityEnergy,
                code: "6D-01",
                title: "EN 15978 Whole-Life Carbon & Green Scorecards",
                description: "Modules A1-D lifecycle carbon audit, EAF/H2-DRI routes, and GREENSHIP/LEED/BREEAM credit tracking.",
                status: Active,
                module_hint: "src/views/sustainability.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::BimMaterialLcaAutoSync,
                pillar: NavigationPillar::SustainabilityEnergy,
                code: "6D-02",
                title: "Live BIM Material Embodied Carbon Sync",
                description: "Computes real-time A1-A3 CO2e intensity directly from active BimProject volumes and steel profiles.",
                status: Active,
                module_hint: "src/core/carbon.rs + src/views/sustainability.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::SolarOttvThermalDaylight,
                pillar: NavigationPillar::SustainabilityEnergy,
                code: "6D-03",
                title: "SNI 6389 Solar OTTV & Facade Daylight Simulation",
                description: "Computes N/E/S/W facade Overall Thermal Transfer Value (OTTV W/m²), SHGC glazing, and shading factors.",
                status: Active,
                module_hint: "src/views/sustainability.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::CircularMaterialPassportDfd,
                pillar: NavigationPillar::SustainabilityEnergy,
                code: "6D-04",
                title: "Circular Material Bank Passport & Design-for-Disassembly",
                description: "Tracks reversible bolted steel share, EN 15978 Module D reuse credit, and QR circular material tags.",
                status: Active,
                module_hint: "src/views/sustainability.rs",
                prioritized: false,
            },

            // =================================================================
            // 10. Facility Management & 7D Digital Twin (1 Active + 3 Locked = 4)
            // =================================================================
            FeatureEntry {
                id: FeatureId::DigitalTwinShmCobie3,
                pillar: NavigationPillar::FacilityManagement,
                code: "7D-01",
                title: "7D Digital Twin, SHM Telemetry & COBie 3.0",
                description: "Strain/accelerometer sparklines, buildingSMART COBie spreadsheet inspector, CMMS dispatch, and 30-yr LCC.",
                status: Active,
                module_hint: "src/views/facility.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::LiveIotTelemetryIngestion,
                pillar: NavigationPillar::FacilityManagement,
                code: "7D-02",
                title: "Live WebSocket/MQTT IoT Sensor Stream Ingestion",
                description: "Connects browser WebSocket endpoints for live field strain gauge and accelerometer SHM streaming.",
                status: Active,
                module_hint: "src/core/web_bridge.rs + src/views/facility.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::MepIsolationSpaceUtilization,
                pillar: NavigationPillar::FacilityManagement,
                code: "7D-03",
                title: "MEP Riser Valve Isolation Tree & Space Utilization",
                description: "Maps emergency shutoff valve upstream/downstream dependencies to impacted rooms and gross-to-net KPIs.",
                status: Active,
                module_hint: "src/views/facility.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::DroneDefectCrackMapper,
                pillar: NavigationPillar::FacilityManagement,
                code: "7D-04",
                title: "UAV Orthomosaic Crack Width & Concrete Spalling Mapper",
                description: "Log-pinned facade & bridge pier crack widths vs ACI 224R limits, spalling severities, and repair triggers.",
                status: Active,
                module_hint: "src/views/facility.rs",
                prioritized: false,
            },

            // =================================================================
            // 11. Safety & 8D Site Risk (1 Active + 3 Locked = 4)
            // =================================================================
            FeatureEntry {
                id: FeatureId::TowerCraneFallRiskRadar,
                pillar: NavigationPillar::SafetyHealth,
                code: "8D-01",
                title: "8D Tower Crane Radar & Slab Fall Guard",
                description: "Crane pick load vs radius envelope checks, OSHA 1926.502 edge guards, evacuation routes, and HIRADC matrix.",
                status: Active,
                module_hint: "src/views/safety.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::SitePerimeter3DCraneCollision,
                pillar: NavigationPillar::SafetyHealth,
                code: "8D-02",
                title: "3D Crane Jib vs BIM Perimeter Clearance Check",
                description: "Evaluates 3D swing clearance against active BimProject building envelopes and site boundaries.",
                status: Active,
                module_hint: "src/views/safety.rs + src/core/bim.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::ScaffoldingFormworkReshoring,
                pillar: NavigationPillar::SafetyHealth,
                code: "8D-03",
                title: "ACI 347 Slab Formwork Shoring & Reshoring Load Transfer",
                description: "Checks early-age concrete strength maturity, multi-floor shoring load ratios, and falsework Euler buckling.",
                status: Active,
                module_hint: "src/views/safety.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::PermitToWorkEvacuationDrill,
                pillar: NavigationPillar::SafetyHealth,
                code: "8D-04",
                title: "Digital Permit-to-Work (PTW) & NFPA 101 Muster Telemetry",
                description: "Hot-work/confined-space/height permit ledger, wind lockdown thresholds, and timed evacuation drill logs.",
                status: Active,
                module_hint: "src/views/safety.rs",
                prioritized: false,
            },

            // =================================================================
            // 12. Lean Construction & 9D Logistics (1 Active + 3 Locked = 4)
            // =================================================================
            FeatureEntry {
                id: FeatureId::CuttingStock1DAndLastPlanner,
                pillar: NavigationPillar::LeanConstruction,
                code: "9D-01",
                title: "1D Cutting Stock Optimizer & Last Planner LPS",
                description: "12m rebar waste minimization, 6-week lookahead constraint clearing, PPC chart, and root-cause Pareto.",
                status: Active,
                module_hint: "src/views/lean.rs + src/core/cso.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::LiveRebarBbsCuttingSync,
                pillar: NavigationPillar::LeanConstruction,
                code: "9D-02",
                title: "Live BIM Rebar Schedule -> 1D Cutting Sync",
                description: "Feeds active BimProject rebar bar marks and lengths directly into the 1D cutting stock optimizer.",
                status: Active,
                module_hint: "src/core/cso.rs + src/views/lean.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::TaktTimeConcretePourDispatch,
                pillar: NavigationPillar::LeanConstruction,
                code: "9D-03",
                title: "Zone Takt-Time Wagon Cadence & Ready-Mix Dispatch",
                description: "Balances multi-zone trade trains and monitors ready-mix concrete truck slump & transit intervals.",
                status: Active,
                module_hint: "src/views/lean.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::RfiSubmittalSlaTracker,
                pillar: NavigationPillar::LeanConstruction,
                code: "9D-04",
                title: "BIM-Pinned RFI & Shop-Drawing SLA Turnaround Tracker",
                description: "Links engineering RFIs and shop drawing submittals to lookahead constraints with SLA escalation timers.",
                status: Active,
                module_hint: "src/views/lean.rs",
                prioritized: false,
            },

            // =================================================================
            // 13. Industrialized & 10D DfMA / OpenBIM (2 Active + 3 Locked = 5)
            // =================================================================
            FeatureEntry {
                id: FeatureId::OpenBimIfcReaderEditorClashDiff,
                pillar: NavigationPillar::IndustrializedAutomation,
                code: "10D-01",
                title: "OpenBIM IFC2x3/4/4.3 Reader, Editor, Clash & IDS",
                description: "Full ISO 10303-21 STEP parser/serializer, transaction undo/redo, spatial clash, diff, and IDS audit.",
                status: Active,
                module_hint: "src/views/industrialized/openbim_studio.rs + src/core/ifc.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::DfmaSupplyChainTracker,
                pillar: NavigationPillar::IndustrializedAutomation,
                code: "10D-02",
                title: "10D Hub, Prefab DfMA Pipeline & Robotic CNC",
                description: "6-stage prefab supply chain, QR check-in console, hoist SWL gauges, and DSTV .nc1 robotic welding.",
                status: Active,
                module_hint: "src/views/industrialized/dfma_robotics.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::WasmLocalIfcFileDropPicker,
                pillar: NavigationPillar::IndustrializedAutomation,
                code: "10D-03",
                title: "WASM Browser File Picker & Drag-and-Drop IFC",
                description: "Imports local .ifc, .json, and .e2k files directly in browser via HTML5 file picker and drop events.",
                status: Active,
                module_hint: "src/core/web_bridge.rs + src/views/industrialized/openbim_studio.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::PrecastTrailerJointTolerance,
                pillar: NavigationPillar::IndustrializedAutomation,
                code: "10D-04",
                title: "Precast Trailer Routing Envelope & Grouted Splice Tolerance",
                description: "Checks highway clearance envelopes, trailer axle weights, and millimeter laser-tracker joint alignment.",
                status: Active,
                module_hint: "src/views/industrialized/automation_slots.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::Concrete3DPrintingGcodeToolpath,
                pillar: NavigationPillar::IndustrializedAutomation,
                code: "10D-05",
                title: "3D Concrete Printing (3DCP) Contour Slicer & G-Code",
                description: "Continuous print path preview, extrusion bead velocity sync, open-time cold-joint guard, and G-Code output.",
                status: Active,
                module_hint: "src/views/industrialized/automation_slots.rs",
                prioritized: false,
            },

            // =================================================================
            // 14. Prediction & AI Structural Aging (1 Active + 3 Locked = 4)
            // =================================================================
            FeatureEntry {
                id: FeatureId::FosmReliabilityProgressiveCollapse,
                pillar: NavigationPillar::Prediction,
                code: "PRED-01",
                title: "FOSM Reliability, FEMA Fragility & UFC Collapse",
                description: "Reliability index beta, seismic collapse fragility curves, DoD UFC 4-023-03 APM, and 50-year aging.",
                status: Active,
                module_hint: "src/views/prediction.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::LiveShmAnomalyForecasting,
                pillar: NavigationPillar::Prediction,
                code: "PRED-02",
                title: "Live FEA & SHM Stress Anomaly Forecasting",
                description: "Computes real-time member vulnerability rankings from active FEA forces and 7D SHM sensor trends.",
                status: Active,
                module_hint: "src/core/reliability.rs + src/views/prediction.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::ChlorideCorrosionDurability,
                pillar: NavigationPillar::Prediction,
                code: "PRED-03",
                title: "Fick's 2nd Law Chloride & Carbonation Service-Life Forecaster",
                description: "Models marine chloride penetration, cover depassivation year, and rebar cross-section area decay curves.",
                status: Active,
                module_hint: "src/views/prediction.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::PostEarthquakeAtc20Tagging,
                pillar: NavigationPillar::Prediction,
                code: "PRED-04",
                title: "Post-Earthquake Rapid ATC-20 Placard Triage Matrix",
                description: "Maps peak accelerometer drift & residual strain into automated Inspected / Restricted / Unsafe wing tags.",
                status: Active,
                module_hint: "src/views/prediction.rs",
                prioritized: false,
            },

            // =================================================================
            // 15. Report & Publishing Suite (2 Active + 3 Locked = 5)
            // =================================================================
            FeatureEntry {
                id: FeatureId::DedDrawingSheetsPdfDxfSvg,
                pillar: NavigationPillar::ReportPublishing,
                code: "PUB-01",
                title: "2D Drawing Sheets (DED) & Vector PDF/DXF/SVG",
                description: "ISO A0-A3 & ANSI Arch D multi-viewport drafting sheets, PE digital seal, and layered DXF/PDF exports.",
                status: Active,
                module_hint: "src/views/results.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::LivingDocsSheetsSlides,
                pillar: NavigationPillar::ReportPublishing,
                code: "PUB-02",
                title: "Living Docs, Formula BoQ Sheets & Client Slides",
                description: "Editable calculation memos, live spreadsheet formulas, executive slide decks, and full audit dossiers.",
                status: Active,
                module_hint: "src/views/results.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::AutoProjectedBimDedSheets,
                pillar: NavigationPillar::ReportPublishing,
                code: "PUB-03",
                title: "Auto-Projected DED Sheets from Active BimProject",
                description: "Dynamically projects active plans, section cuts, and rebar/column schedules onto printable sheets.",
                status: Active,
                module_hint: "src/core/bim.rs + src/views/results.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::RegulatoryCalcBookPbgComposer,
                pillar: NavigationPillar::ReportPublishing,
                code: "PUB-04",
                title: "PBG / SIMBG & IBC Regulatory Calculation Book Composer",
                description: "Assembles multi-chapter permitting books combining SNI 1726 spectra, ETABS checks, and PE cover seals.",
                status: Active,
                module_hint: "src/views/results.rs",
                prioritized: false,
            },
            FeatureEntry {
                id: FeatureId::AsBuiltHandoverDossierPackager,
                pillar: NavigationPillar::ReportPublishing,
                code: "PUB-05",
                title: "As-Built BIM Digital Handover & SHA-256 Archive Packager",
                description: "Audits completeness across IFC4, COBie 3.0, record drawings, mill certificates, and O&M warranty manuals.",
                status: Active,
                module_hint: "src/views/results.rs",
                prioritized: false,
            },
        ];

        Self { entries }
    }

    pub fn entry(&self, id: FeatureId) -> Option<&FeatureEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn entry_mut(&mut self, id: FeatureId) -> Option<&mut FeatureEntry> {
        self.entries.iter_mut().find(|e| e.id == id)
    }

    pub fn is_active(&self, id: FeatureId) -> bool {
        self.entry(id).is_some_and(|e| e.status.is_active())
    }

    pub fn toggle_prioritized(&mut self, id: FeatureId) {
        if let Some(entry) = self.entry_mut(id) {
            if !entry.status.is_active() {
                entry.prioritized = !entry.prioritized;
            }
        }
    }

    pub fn total_count(&self) -> usize {
        self.entries.len()
    }

    pub fn active_count(&self) -> usize {
        self.entries.iter().filter(|e| e.status.is_active()).count()
    }

    pub fn locked_count(&self) -> usize {
        self.entries.iter().filter(|e| !e.status.is_active()).count()
    }

    pub fn prioritized_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| !e.status.is_active() && e.prioritized)
            .count()
    }

    pub fn locked_by_phase_count(&self, phase: &str) -> usize {
        self.entries
            .iter()
            .filter(|e| {
                e.status
                    .target_phase()
                    .is_some_and(|p| p.eq_ignore_ascii_case(phase))
            })
            .count()
    }

    pub fn distinct_target_phases() -> &'static [&'static str] {
        &["Phase 32", "Phase 33", "Phase 34", "Phase 35"]
    }

    pub fn prioritize_phase(&mut self, phase_prefix: &str) {
        for entry in &mut self.entries {
            if let FeatureStatus::Locked { target_phase } = entry.status {
                if target_phase.eq_ignore_ascii_case(phase_prefix) {
                    entry.prioritized = true;
                }
            }
        }
    }

    pub fn clear_all_priorities(&mut self) {
        for entry in &mut self.entries {
            entry.prioritized = false;
        }
    }

    pub fn pillar_counts(&self, pillar: NavigationPillar) -> (usize, usize) {
        let mut active = 0;
        let mut total = 0;
        for e in &self.entries {
            if e.pillar == pillar {
                total += 1;
                if e.status.is_active() {
                    active += 1;
                }
            }
        }
        (active, total)
    }

    /// Exports a markdown unlock backlog summarizing prioritized locked features for the next sprint.
    pub fn export_unlock_backlog_markdown(&self) -> String {
        let mut out = String::from("# EigenGraph Feature Unlock Queue\n\n");
        out.push_str(&format!(
            "- Total Registered Capabilities: {}\n- Active Capabilities: {}\n- Locked Workspace Slots: {}\n- Prioritized for Next Unlock: {}\n\n",
            self.total_count(),
            self.active_count(),
            self.locked_count(),
            self.prioritized_count()
        ));
        out.push_str("## Locked Capabilities\n\n");
        let locked_items: Vec<_> = self.entries.iter().filter(|e| !e.status.is_active()).collect();
        if locked_items.is_empty() {
            out.push_str("- All 74 registered capabilities are active (`0` locked backlog items).\n");
        }
        for e in locked_items {
            let mark = if e.prioritized { "[PRIORITIZED]" } else { "[QUEUED]" };
            out.push_str(&format!(
                "- {} **{} — {}** (`{}`) | {} | `{}`\n  - {}\n",
                mark,
                e.code,
                e.title,
                e.pillar.clean_name(),
                e.status.badge_label(),
                e.module_hint,
                e.description
            ));
        }
        out
    }

    /// Exports a structured JSON unlock roadmap covering all 74 registered capabilities.
    pub fn export_unlock_roadmap_json(&self) -> String {
        let items: Vec<serde_json::Value> = self
            .entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "code": e.code,
                    "pillar": e.pillar.clean_name(),
                    "title": e.title,
                    "description": e.description,
                    "status": if e.status.is_active() { "ACTIVE" } else { "LOCKED" },
                    "target_phase": e.status.target_phase().unwrap_or("LIVE"),
                    "prioritized": e.prioritized,
                    "module_hint": e.module_hint
                })
            })
            .collect();

        let doc = serde_json::json!({
            "schema": "eigengraph.feature_unlock_roadmap.v2",
            "version": env!("CARGO_PKG_VERSION"),
            "summary": {
                "total": self.total_count(),
                "active": self.active_count(),
                "locked": self.locked_count(),
                "prioritized": self.prioritized_count()
            },
            "capabilities": items
        });

        serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_registry_covers_all_15_pillars() {
        let reg = FeatureRegistry::new();
        for &pillar in NavigationPillar::all() {
            let (active, total) = reg.pillar_counts(pillar);
            assert!(
                total >= 4 && active >= 1,
                "Pillar {:?} must track at least 1 active and 4 total capabilities",
                pillar
            );
        }
        assert_eq!(reg.total_count(), 74);
        assert_eq!(reg.active_count(), 74);
        assert_eq!(reg.locked_count(), 0);
    }

    #[test]
    fn test_feature_registry_phases_and_json_export() {
        let mut reg = FeatureRegistry::new();
        assert_eq!(reg.locked_by_phase_count("Phase 32"), 0);
        assert_eq!(reg.locked_by_phase_count("Phase 33"), 0);
        assert_eq!(reg.locked_by_phase_count("Phase 34"), 0);
        assert_eq!(reg.locked_by_phase_count("Phase 35"), 0);

        let entry = reg
            .entry(FeatureId::ParetoMultiObjectiveOptimizer)
            .expect("entry exists");
        assert!(entry.status.is_active());
        reg.toggle_prioritized(FeatureId::ParetoMultiObjectiveOptimizer);
        assert!(
            !reg.entry(FeatureId::ParetoMultiObjectiveOptimizer)
                .expect("entry exists")
                .prioritized
        );

        let md = reg.export_unlock_backlog_markdown();
        assert!(md.contains("Active Capabilities: 74"));
        assert!(md.contains("Locked Workspace Slots: 0"));

        let json_str = reg.export_unlock_roadmap_json();
        assert!(json_str.contains("AI-05"));
        assert!(json_str.contains("10D-05"));
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("roadmap json parses cleanly");
        assert_eq!(parsed["summary"]["total"], 74);
        assert_eq!(parsed["capabilities"].as_array().map(|a| a.len()), Some(74));
    }
}
