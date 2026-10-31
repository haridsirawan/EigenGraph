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
