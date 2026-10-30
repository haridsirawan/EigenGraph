//! core/mod.rs — Root Core Engine Module for EigenGraph.
//! Encapsulates Mechanics, Finite Element Assembly, SVD Kinematic Guard,
//! Adjusted Redundancy Index (ARI) Analytics, and Parametric CAD Generation.

pub mod linalg;
pub mod model;
pub mod solver;
pub mod ari;
pub mod parametric;

pub use linalg::Matrix;
pub use model::{
    BoundaryFixity, CrossSection, Element3D, ElementType, Material, NodalLoad, Node3D,
    StructuralModel,
};
pub use solver::{AnalysisResult, FemSolver};
pub use ari::{AriEngine, KinematicGuardReport, MemberRedundancyResult, RedundancyAssessment};
pub use parametric::{ParametricEngine, ParametricFormula};
