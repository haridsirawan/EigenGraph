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
pub mod ifc;
pub mod gis;
pub mod lifecycle_ops;
pub mod web_bridge;

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
    EtabsE2kExporter, EtabsE2kImporter, HtmlViewerExporter, IfcExporter, IfcImporter, IfcSamExporter,
    LatexExporter, PbgProjectMetadata, PbgReportGenerator, RebarBbsDetailer, RebarBbsSchedule,
    Sap2000Exporter, Sha256Hasher,
};
pub use optimizer::{AriTopologyOptimizer, BracingCandidate, BracingOptimizationReport};
pub use cost::{AhspUnitRates, BoqLineItem, CostEngine, CostEstimationReport, ProfileQuantity};
pub use cascade::{CascadingCollapseReport, CascadingSimulator, CollapseStatus, CollapseStep};
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
    GeotechEngine, GeotechSsiReport, PileElement, PileGroupReport, PunchingShearAudit, SoilLayer,
    SoilType, SsiModalShiftReport, WinklerSpringMatrix,
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
    GroundMotionRecord, HysteresisModel, StoryResponseProfile, TimeHistoryEngine,
    TimeHistoryInput, TimeHistoryReport, TimeHistorySample, TimeIntegrationScheme,
};
pub use fema_p58::{
    ComponentFragility, DamageStateDef, EdpKind, FemaComponentCategory, FemaComplianceItem,
    FemaP58Engine, FemaP58Input, FemaP58Report, REDiResilienceTier, StoryLossSummary,
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
pub use ifc::{
    IfcAuditCheckItem, IfcAuditStatus, IfcBcfTopic, IfcBoundingBox, IfcClashItem,
    IfcClashReport, IfcClashSeverity, IfcDiffKind, IfcEditCommand, IfcEditSession,
    IfcElementCreationSpec, IfcElementDiff, IfcElementSummary, IfcIdsReport,
    IfcIdsRuleCheck, IfcModel, IfcModelDiff, IfcModelHealthReport, IfcPropertySetSummary,
    IfcSchemaVersion, IfcSpatialNode, StepEntity, StepHeader, StepModel, StepParser,
    StepValue,
};
pub use gis::{
    build_tin_from_3d_triangles, ContourPolyline, CutFillVolumeReport, DemTerrainPreset,
    GeoProjectionEngine, IfcMapConversionMatrix, SiteGeodeticPin, SlippyMapState,
    SlippyMapTelemetry, TileSource, TinPoint, TinSurfaceMesh, TinTriangle, UtmZoneInfo,
    VectorEarthPalette, VectorEarthTheme, WebMercator, EARTH_HALF_CIRCUMFERENCE,
    EARTH_RADIUS_METERS, MAX_LATITUDE, MIN_LATITUDE, UTM_FALSE_EASTING,
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


