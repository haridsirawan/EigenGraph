//! models.rs — Domain models and state definitions for EigenGraph.

use serde::{Deserialize, Serialize};

/// The 10-Dimensional BIM & Lifecycle Navigation Pillars of EigenGraph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NavigationPillar {
    ResearchConcept,         // 1D: Research & Concept
    DraftingDrawings,        // 2D: Drafting & Drawings
    ModelingGeometry,        // 3D: Modeling & Geometry
    TimeScheduling,          // 4D: Time & Scheduling
    CostBudgeting,           // 5D: Cost & Budgeting
    SustainabilityEnergy,    // 6D: Sustainability & Energy
    FacilityManagement,      // 7D: Facility Management
    SafetyHealth,            // 8D: Safety/Health & Safety
    LeanConstruction,        // 9D: Lean Construction
    IndustrializedAutomation,// 10D: Industrialized Construction & Automation
}

impl NavigationPillar {
    pub fn title(&self) -> &'static str {
        match self {
            Self::ResearchConcept => "RESEARCH & CONCEPT",
            Self::DraftingDrawings => "DRAFTING & DRAWINGS",
            Self::ModelingGeometry => "MODELING & GEOMETRY",
            Self::TimeScheduling => "TIME & SCHEDULING",
            Self::CostBudgeting => "COST & BUDGETING",
            Self::SustainabilityEnergy => "SUSTAINABILITY & ENERGY",
            Self::FacilityManagement => "FACILITY MANAGEMENT",
            Self::SafetyHealth => "SAFETY / HEALTH & SAFETY",
            Self::LeanConstruction => "LEAN CONSTRUCTION",
            Self::IndustrializedAutomation => "INDUSTRIALIZED & AUTOMATION",
        }
    }

    pub fn clean_name(&self) -> &'static str {
        match self {
            Self::ResearchConcept => "Research & Concept",
            Self::DraftingDrawings => "Drafting & Drawings",
            Self::ModelingGeometry => "Modeling & Geometry",
            Self::TimeScheduling => "Time & Scheduling",
            Self::CostBudgeting => "Cost & Budgeting",
            Self::SustainabilityEnergy => "Sustainability & Energy",
            Self::FacilityManagement => "Facility Management",
            Self::SafetyHealth => "Safety/Health & Safety",
            Self::LeanConstruction => "Lean Construction",
            Self::IndustrializedAutomation => "Industrialized & Automation",
        }
    }

    pub fn dimension_tag(&self) -> &'static str {
        match self {
            Self::ResearchConcept => "1D",
            Self::DraftingDrawings => "2D",
            Self::ModelingGeometry => "3D",
            Self::TimeScheduling => "4D",
            Self::CostBudgeting => "5D",
            Self::SustainabilityEnergy => "6D",
            Self::FacilityManagement => "7D",
            Self::SafetyHealth => "8D",
            Self::LeanConstruction => "9D",
            Self::IndustrializedAutomation => "10D",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::ResearchConcept => "💡",
            Self::DraftingDrawings => "📐",
            Self::ModelingGeometry => "🏗️",
            Self::TimeScheduling => "⏱️",
            Self::CostBudgeting => "💰",
            Self::SustainabilityEnergy => "🌱",
            Self::FacilityManagement => "🏢",
            Self::SafetyHealth => "🛡️",
            Self::LeanConstruction => "♻️",
            Self::IndustrializedAutomation => "🤖",
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            Self::ResearchConcept => "The foundational stage involving initial ideas, feasibility studies, regulatory checks, surveying data, and project guidelines.",
            Self::DraftingDrawings => "Traditional two-dimensional technical documentation showing X and Y axes, such as floor plans, elevations, sections, and blueprints.",
            Self::ModelingGeometry => "The creation of a digital three-dimensional model (length, width, height) allowing for spatial visualization and clash detection.",
            Self::TimeScheduling => "The integration of project timelines and construction sequences into the 3D model to simulate the building process over time.",
            Self::CostBudgeting => "The tracking of cost, dynamic quantity take-offs (QTO), and financial estimates directly linked to the digital model.",
            Self::SustainabilityEnergy => "The analysis of energy efficiency, carbon footprint, environmental impact, and green building certification requirements.",
            Self::FacilityManagement => "The lifecycle operation and maintenance data used by building owners to manage assets, repairs, and components after construction is complete.",
            Self::SafetyHealth => "The integration of accident prevention strategies, site risk assessments, and emergency planning into the construction design.",
            Self::LeanConstruction => "The optimization of the construction process to minimize material waste, streamline workflow, and maximize production efficiency.",
            Self::IndustrializedAutomation => "The integration of Industry 4.0 technologies, including prefabrication, robotics, 3D printing, and automated site processes.",
        }
    }
}

/// CAD Drawing Tools in PARAMETRIC Free Drawing Studio
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CadTool {
    Select,
    Line,
    Polyline,
    Arc,
    Circle,
    Rectangle,
    Dimension,
    Trim,
    Offset,
}

impl CadTool {
    pub fn label_and_shortcut(&self) -> (&'static str, &'static str) {
        match self {
            Self::Select => ("👆 Select", "ESC"),
            Self::Line => ("📏 Line", "L"),
            Self::Polyline => ("〰️ Polyline", "PL"),
            Self::Arc => ("⭕ Arc", "A"),
            Self::Circle => ("⚪ Circle", "C"),
            Self::Rectangle => ("▭ Rectangle", "REC"),
            Self::Dimension => ("📐 Dimension", "DIM"),
            Self::Trim => ("✂️ Trim", "TR"),
            Self::Offset => ("↔️ Offset", "O"),
        }
    }
}

/// Parametric Driving Variable (e.g. L = 24.0m, H = L / 7.5, n = 6)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametricVariable {
    pub name: String,
    pub expression: String,
    pub evaluated_value: f32,
    pub unit: String,
}

/// A Drawn Parametric Component saved into the Database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseComponent {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub variables: Vec<ParametricVariable>,
    pub elements_count: usize,
    pub created_at: String,
}

/// Unified Left Sidebar (Activity Bar) Tabs in BUILD Workspace (CAD/BIM Modeling & Inspection)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SidebarTab {
    Properties,
    ProjectBrowser,
    Modify,
    Layers,
}

impl SidebarTab {
    pub fn icon_and_name(&self) -> (&'static str, &'static str) {
        match self {
            Self::Properties => ("⚙️", "Properties Inspector"),
            Self::ProjectBrowser => ("📁", "Project Browser"),
            Self::Modify => ("🛠️", "Modify Tools"),
            Self::Layers => ("📑", "Display Layers"),
        }
    }
}

/// Project Item in Mac App Store style Catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectItem {
    pub id: String,
    pub title: String,
    pub category: String,
    pub description: String,
    pub elements_count: usize,
    pub nodes_count: usize,
    pub min_ari: f32,
    pub max_deflection_mm: f32,
    pub status: ProjectStatus,
    pub last_modified: String,
    pub is_favorite: bool,
    pub is_featured: bool,
    pub tag: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectStatus {
    Safe,
    ReviewNeeded,
    CriticalMechanism,
}

impl ProjectStatus {
    pub fn badge_label(&self) -> &'static str {
        match self {
            Self::Safe => "🟢 ARI Safe",
            Self::ReviewNeeded => "🟡 Review Needed",
            Self::CriticalMechanism => "🔴 Mechanism Warning",
        }
    }
}

/// Structural Element for the Build and Analyze views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralMember {
    pub id: usize,
    pub name: String,
    pub section: String,
    pub force_kn: f32,
    pub capacity_kn: f32,
    pub utilization: f32,
    pub moment_my_knm: f32,
    pub moment_mz_knm: f32,
    pub shear_vy_kn: f32,
    pub dcr_interaction: f32,
    pub rr: f32,
    pub ari: f32,
    pub ci: f32,
    pub svd_margin: f32,
    pub is_critical: bool,
}

/// Chat message with EigenAI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub is_user: bool,
    pub message: String,
    pub time: String,
}
