//! core/mod.rs — Root Core Engine Module for EigenGraph.
//! Encapsulates Mechanics, Finite Element Assembly, SVD Kinematic Guard,
//! Adjusted Redundancy Index (ARI) Analytics, Parametric CAD Generation,
//! AI-ARI Topology Optimization, BIM 5D QTO Cost Engine, and AEC Interoperability.

pub mod linalg;
pub mod model;
pub mod solver;
pub mod ari;
pub mod parametric;
pub mod interop;
pub mod optimizer;
pub mod cost;
pub mod cascade;
pub mod schedule;
pub mod carbon;
pub mod nonlinear;
pub mod modal;
pub mod reliability;
pub mod connection;
pub mod dynamics;
pub mod vault;
pub mod geom;
pub mod csg;
pub mod bim;
pub mod cso;
pub mod concrete;
pub mod pushover;
pub mod geotech;
pub mod wind;
pub mod composite;
pub mod geometry_engine;
pub mod prestressed_concrete;
pub mod shear_wall;
pub mod isolation;
pub mod time_history;
pub mod fema_p58;
pub mod blast;
pub mod fire;
pub mod ifcx;
pub mod retrofit;
pub mod gis;
pub mod lifecycle_ops;
pub mod portfolio_retrofit;
pub mod ground_motion_replay;
pub mod shm_twin;
pub mod evacuation_routing;
pub mod web_bridge;
pub mod vector_dossier;

pub use linalg::{CsrMatrix, Matrix};
pub use model::{
    BoundaryFixity, CrossSection, Element3D, ElementType, LoadCaseType, LoadCombination, Material,
    NodalLoad, Node3D, ShellElement4N, StructuralModel,
};
pub use solver::{
    AnalysisResult, EnvelopeAnalysisResult, FemSolver, MemberEnvelopeResult,
    SecondOrderStabilityResult, SecondOrderStabilitySolver,
};
pub use ari::{
    AriEngine, KinematicGuardReport, MemberRedundancyResult, N1SimulationReport,
    N2PairRedundancyResult, N2WoodburyScreeningReport, RedundancyAssessment,
};
pub use parametric::{ParametricEngine, ParametricFormula};
pub use interop::{
    DiskExporter, DossierExporter, DstvExporter, DstvPiece, DxfExporter, EigenProjectArchive,
    EtabsE2kExporter, EtabsE2kImporter, HtmlViewerExporter, IfcxExporter, IfcxImporter,
    IfcxSamExporter, LatexExporter, PbgProjectMetadata, PbgReportGenerator, RebarBbsDetailer,
    RebarBbsSchedule, Sap2000Exporter, Sha256Hasher,
};
pub use optimizer::{AriTopologyOptimizer, BracingCandidate, BracingOptimizationReport};
pub use cost::{AhspUnitRates, BoqLineItem, CostEngine, CostEstimationReport, ProfileQuantity};
pub use cascade::{
    run_two_stage_crc_and_cascade_for_bim, CascadingCollapseReport, CascadingSimulator,
    CollapseStatus, CollapseStep, CrcRiskTier, StructGnnCrcScenario, TwoStageCrcCascadeReport,
    NS_EG_CASCADE_CRC,
};
pub use schedule::{
    ConstructionActivity, SCurvePoint, Schedule4DReport, ScheduleEngine, StageStabilityResult,
};
pub use carbon::{CarbonEngine, CarbonLcaReport, CarbonLcaStage, CarbonSteelRoute};
pub use nonlinear::{
    NonlinearEngine, PlasticHinge, PlasticHingeState, PushoverReport, PushoverStep,
};
pub use pushover::{
    AdrsPoint, CapacitySpectrum, InelasticDemandSpectrum, LateralLoadPattern, PerformancePoint,
    PushoverCsmReport, PushoverEngine, SeismicPerformanceLevel,
};
pub use geotech::{
    evaluate_coupled_subsurface_ssi_for_bim, BoreholePreset, FootingSsiCouplingResult,
    GeotechEngine, GeotechSsiReport, PileElement, PileGroupReport, PunchingShearAudit, SoilLayer,
    SoilType, SsiModalShiftReport, StratigraphyStratum, SubsurfaceSsiAnalysisReport,
    SubsurfaceSsiConfig, WinklerSpringMatrix, NS_EG_GEOTECH_SSI, NS_EG_GEOTECH_STRATIGRAPHY,
};
pub use modal::{
    ModalAnalysisReport, ModalEngine, ModeShape, SiteClass, Sni1726Spectrum,
};
pub use reliability::{
    FragilityCurvePoint, MemberReliabilityResult, ReliabilityAndClashReport, ReliabilityEngine,
    SpatialClash, standard_normal_cdf,
};
pub use connection::{
    BoltGrade, ConnectionAuditReport, ConnectionEngine, ConnectionReport, ConnectionType,
    LimitStateCheck, MemberConnectionDesign, PanelZoneAudit,
};
pub use dynamics::{
    DynamicsEngine, SuddenRemovalTransientReport, TimeHistoryStep,
};
pub use vault::{
    AriKnowledgeMemory, InternalVault, MultiLodConvergenceReport, MultiLodStageMetrics,
};
pub use geom::{
    area as polygon_area, centroid as polygon_centroid, clip as polygon_clip, contains as polygon_contains,
    faces as planar_faces, grow as polygon_grow, line_intersection, perimeter as polygon_perimeter,
    region_at, region_in, straight_skeleton_roof, triangulate as polygon_triangulate, P2, Region,
    StraightSkeletonRoof,
};
pub use csg::{
    csg_subtract, cuboid_mesh, extrude_box_run_mesh, extrude_ring_mesh, extrude_tube_mesh,
    extrude_wf_section_mesh, mesh_bounds, mesh_surface_area, mesh_volume, slice_mesh_with_plane,
    Aabb3D, Bvh3D, CsgMeter, P3,
};
pub use bim::{
    BimColumn, BimCommand, BimCommandBus, BimDimension, BimElementKind, BimFooting, BimGrid,
    BimLevel, BimOpening, BimProject, BimRoom, BimRun, BimSlab, BimWall, CompoundLayer,
    LayerFunction, OpeningKind, RebarBar, RebarPreset, RunCategory, RunJustify,
    SectionCutProjection, SiteLoadSummary, SnapHit, SnapKind, ViewRange,
};
pub use cso::{
    BbsScheduleItem, CsoReport, CuttingItem, CuttingStockOptimizer, MasterStockBar, PlacedCut,
};
pub use concrete::{
    CardinalPoints, ConcreteEngine, ConcretePmReport, ConcreteSection, DemandAudit,
    InteractionPoint, RebarLayer,
};
pub use wind::{
    EnclosureClassification, ExposureCategory, RiskCategory, StoryWindLevel, WindAnalysisReport,
    WindComfortAudit, WindEngine,
};
pub use composite::{
    CompositeBeamEngine, CompositeBeamReport, CompositeLimitState, CompositeSlabDeck,
    DeckOrientation, PnaLocation, ShearStud,
};
pub use geometry_engine::{
    MeshTopologyEngine, ParametricSpatialWizard, PolygonPoint2D, SectionIntegrator,
    SectionPropertiesReport,
};
pub use prestressed_concrete::{
    AciTensionClass, FiberStressCheck, GirderSectionArchetype, PrestressLossBreakdown,
    PrestressedConcreteEngine, PrestressedGirderReport, PrestressingMethod, StrandGrade,
    TendonPointCoordinates, TendonProfileKind,
};
pub use shear_wall::{
    BoundaryElementApproach, BoundaryElementReport, CouplingBeamReport, CouplingBeamType,
    InPlaneShearReport, ShearWallEngine, ShearWallSystemReport, WallFactoredDemands,
    WallGeometry, WallMaterials, WallSeismicClassification,
};
pub use isolation::{
    DamperConfiguration, FluidViscousDamperParams, FrictionPendulumParams, HysteresisPoint,
    IsolatorType, LeadRubberBearingParams, RegulatoryCheckItem, SeismicIsolationEngine,
    SeismicIsolationInput, SeismicIsolationReport,
};
pub use time_history::{
    compute_elastic_response_spectrum_5pct, looks_like_accelerogram_payload, parse_accelerogram_csv,
    parse_custom_accelerogram, parse_peer_at2_accelerogram, sample_peer_at2_text,
    AccelerogramFormat, CustomAccelerogram, GroundMotionRecord, HysteresisModel,
    ResponseSpectrumPoint, StoryResponseProfile, TimeHistoryEngine, TimeHistoryInput,
    TimeHistoryReport, TimeHistorySample, TimeIntegrationScheme,
};
pub use fema_p58::{
    ComponentFragility, DamageStateDef, EdpKind, FemaComponentCategory, FemaComplianceItem,
    FemaP58Engine, FemaP58Input, FemaP58Report, REDiResilienceTier, ReplayFemaP58Synthesis,
    ReplayFragilityCurvePoint, StoryFragilityLoss, StoryLossSummary,
};
pub use blast::{
    BlastAnalysisInput, BlastAnalysisReport, BlastComponentKind, BlastComplianceItem, BlastEngine,
    BlastResponseRegime, BlastWaveParameters, BurstType, ExplosiveType, PiCurvePoint,
    SdofSystemParameters, SdofTimeStep, SupportCondition, UfcDamageLevel,
};
pub use fire::{
    ConcreteAggregateKind, FireAnalysisInput, FireAnalysisReport, FireComplianceItem, FireCurveKind,
    FireEngine, FireMemberKind, FireProtectionKind, FireTimeStep, ThermalRestraintKind,
};
pub use ifcx::{
    AiBuildingFrameInferenceReport, IfcxAriOverlay, IfcxAuditItem, IfcxAuditStatus,
    IfcxClashItem, IfcxClashReport, IfcxClashSeverity, IfcxDiffKind, IfcxDiffLayerFilter,
    IfcxDiffReport, IfcxDocument, IfcxEditSession, IfcxGeoWgs84, IfcxHeader, IfcxHealthReport,
    IfcxImport, IfcxNode, IfcxNodeDiff, IfcxStage, IfcxUnits, IfcxVisualDiffEntry,
    IfcxVisualDiffReport, UsdGeomMesh, IFCX_DATA_VERSION, IFCX_SCHEMA_VERSION,
};
pub use ifcx::ns::EG_IFCX_DIFF as NS_EG_IFCX_DIFF;
pub use retrofit::{
    synthesize_structural_retrofit, RetrofitActionKind, RetrofitElementAction, RetrofitStrategy,
    StructuralRetrofitReport, NS_EG_RETROFIT,
};
pub use gis::{
    build_tin_from_3d_triangles, evaluate_district_flood, evaluate_district_multi_hazard,
    evaluate_seismic_wave_displacement_m, to_ifcx_hazard_node, ContourPolyline, CutFillVolumeReport,
    DemTerrainPreset, DistrictFloodAssessment, DistrictHazardConfig, DistrictMultiHazardReport,
    GeoProjectionEngine, IfcMapConversionMatrix, IfcxCityScene, IfcxGradedPad, IfcxRoadKind,
    IfcxRoadSegment, IfcxSpatialZone, IfcxZoneKind, RoadParcelSnapHit, SiteGeodeticPin,
    SlippyMapState, SlippyMapTelemetry, TileSource, TinPoint, TinSurfaceMesh, TinTriangle,
    UtmZoneInfo, VectorEarthPalette, VectorEarthTheme, VenturiStreamlineSegment, WebMercator,
    EARTH_HALF_CIRCUMFERENCE, EARTH_RADIUS_METERS, MAX_LATITUDE, MIN_LATITUDE, UTM_FALSE_EASTING,
    UTM_FALSE_NORTHING_SOUTH, UTM_SCALE_FACTOR_CENTRAL,
};
pub use lifecycle_ops::{
    compute_mep_engineering_sizing, compute_site_environmental_coupling,
    CommercialPositionSummary, ConcreteCylinderBatch, DailyDiaryEntry, DailyDiarySummary,
    FieldQualityControlLedger, ItpInspectionItem, LifecycleOpsState, MepSizingInput,
    MepSizingResult, MepTestingItem, NcrPunchItem, ProcurementContractLedger, ProcurementStatus,
    PurchaseOrderRow, QualityScorecardSummary, SiteDailyDiaryLedger, SiteEnvironmentalCoupling,
    VariationOrderRow,
};
pub use vector_dossier::{
    build_pure_vector_engineering_dossier_svg, VectorDossierExport, VectorDossierInput,
};
pub use portfolio_retrofit::{
    optimize_district_portfolio_retrofit, DistrictPortfolioRetrofitReport,
    PortfolioBuildingDecision, PortfolioOptimizationObjective, PortfolioParetoPoint,
    PortfolioRetrofitConfig, NS_EG_PORTFOLIO_RETROFIT,
};
pub use ground_motion_replay::{
    run_ground_motion_replay_for_bim, AsyncCrcReplayTelemetry, AsyncReplayJobManager,
    AsyncReplayJobPhase, GroundMotionReplayConfig, GroundMotionReplayFrame,
    GroundMotionReplayReport, PlasticHingeJointSnapshot, TriDirectionalMode,
    NS_EG_FRAGILITY_LOSS, NS_EG_TIME_HISTORY_REPLAY,
};
pub use shm_twin::{
    evaluate_shm_digital_twin_for_bim, looks_like_shm_telemetry_payload, parse_shm_telemetry_log,
    sample_shm_telemetry_csv, Atc20ChecklistCriterion, Atc20Placard, CustomShmTelemetryLog,
    DistrictBuildingPlacard, ShmDigitalTwinConfig, ShmDigitalTwinReport, ShmFaultScenario,
    ShmPsdPoint, ShmSensorKind, ShmSensorNodeSnapshot, ShmTelemetryRecord, StoryShmHealth,
    NS_EG_ATC20_PLACARD, NS_EG_SHM_DIGITAL_TWIN,
};
pub use evacuation_routing::{
    evaluate_evacuation_and_lifelines, AssemblySafePoint, DebrisCordonZone,
    EvacuationLifelineReport, EvacuationMode, EvacuationRoutePath, EvacuationRoutingConfig,
    LifelineGraphEdge, LifelineGraphNode, RoadEdgeStatus, NS_EG_EVACUATION_LIFELINES,
};


