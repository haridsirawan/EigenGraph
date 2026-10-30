//! views/mod.rs — UI View Modules for EigenGraph.

pub mod my_project;
pub mod parametric;
pub mod build;
pub mod analyze;
pub mod settings;
pub mod ai_assistant;

pub use my_project::MyProjectView;
pub use parametric::ParametricView;
pub use build::BuildView;
pub use analyze::AnalyzeView;
pub use settings::SettingsView;
pub use ai_assistant::AiAssistantView;
