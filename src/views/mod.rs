//! views/mod.rs — UI View Modules for EigenGraph (1D to 10D BIM Dimensions).

pub mod my_project;
pub mod parametric;
pub mod build;
pub mod analyze;
pub mod planning;
pub mod settings;
pub mod ai_assistant;
pub mod sustainability;
pub mod facility;
pub mod lean;
pub mod industrialized;

pub use my_project::MyProjectView;
pub use parametric::ParametricView;
pub use build::BuildView;
pub use analyze::AnalyzeView;
pub use planning::PlanningView;
pub use settings::SettingsView;
pub use ai_assistant::AiAssistantView;
pub use sustainability::SustainabilityView;
pub use facility::FacilityView;
pub use lean::LeanView;
pub use industrialized::IndustrializedView;
