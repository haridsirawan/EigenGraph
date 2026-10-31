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

pub use linalg::{CsrMatrix, Matrix};
pub use model::{
    BoundaryFixity, CrossSection, Element3D, ElementType, LoadCaseType, LoadCombination, Material,
    NodalLoad, Node3D, StructuralModel,
};
pub use solver::{AnalysisResult, EnvelopeAnalysisResult, FemSolver, MemberEnvelopeResult};
pub use ari::{
    AriEngine, KinematicGuardReport, MemberRedundancyResult, N2PairRedundancyResult,
    N2WoodburyScreeningReport, RedundancyAssessment,
};
pub use parametric::{ParametricEngine, ParametricFormula};
pub use interop::{
    DiskExporter, DossierExporter, DstvExporter, DstvPiece, DxfExporter, EigenProjectArchive,
    HtmlViewerExporter, IfcExporter, IfcImporter, LatexExporter, Sap2000Exporter, Sha256Hasher,
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
pub use modal::{
    ModalAnalysisReport, ModalEngine, ModeShape, SiteClass, Sni1726Spectrum,
};
pub use reliability::{
    FragilityCurvePoint, MemberReliabilityResult, ReliabilityAndClashReport, ReliabilityEngine,
    SpatialClash, standard_normal_cdf,
};
pub use connection::{
    BoltGrade, ConnectionAuditReport, ConnectionEngine, MemberConnectionDesign,
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
    BimColumn, BimCommand, BimCommandBus, BimDimension, BimFooting, BimGrid, BimLevel, BimOpening,
    BimProject, BimRoom, BimRun, BimSlab, BimWall, CompoundLayer, LayerFunction, OpeningKind,
    RebarBar, RebarPreset, RunCategory, RunJustify, SectionCutProjection, SnapHit, SnapKind,
    ViewRange,
};
