//! models.rs — Central Data Structures for EigenGraph.
//! Contains model entities, navigation pillars, project catalogs, and UI state models.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationPillar {
    MyLibrary,
    ResearchConcept,
    DraftingDrawings,
    ModelingGeometry,
    TimeScheduling,
    CostBudgeting,
    SustainabilityEnergy,
    FacilityManagement,
    SafetyHealth,
    LeanConstruction,
    IndustrializedAutomation,
}

impl NavigationPillar {
    pub fn all() -> &'static [NavigationPillar] {
        &[
            NavigationPillar::MyLibrary,
            NavigationPillar::ResearchConcept,
            NavigationPillar::DraftingDrawings,
            NavigationPillar::ModelingGeometry,
            NavigationPillar::TimeScheduling,
            NavigationPillar::CostBudgeting,
            NavigationPillar::SustainabilityEnergy,
            NavigationPillar::FacilityManagement,
            NavigationPillar::SafetyHealth,
            NavigationPillar::LeanConstruction,
            NavigationPillar::IndustrializedAutomation,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::MyLibrary => "MY LIBRARY & CATALOG",
            Self::ResearchConcept => "1. RESEARCH & CONCEPT",
            Self::DraftingDrawings => "2. DRAFTING & 2D BLUEPRINTS",
            Self::ModelingGeometry => "3. 3D MODELING & GEOMETRY",
            Self::TimeScheduling => "4. TIME & 4D SCHEDULING",
            Self::CostBudgeting => "5. COST & 5D BUDGETING",
            Self::SustainabilityEnergy => "6. SUSTAINABILITY & 6D LCA",
            Self::FacilityManagement => "7. FACILITY & 7D MANAGEMENT",
            Self::SafetyHealth => "8. SAFETY & 8D RISK",
            Self::LeanConstruction => "9. LEAN & 9D WASTE-OPTIMIZATION",
            Self::IndustrializedAutomation => "10. AUTOMATION & 10D DfMA",
        }
    }

    pub fn badge(&self) -> &'static str {
        match self {
            Self::MyLibrary => "CATALOG",
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
            Self::MyLibrary => "📁",
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
            Self::MyLibrary => "Project Catalog, Parametric Component Database & EigenGraph Audit Dossier.",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectStatus {
    Designed,
    UnderReview,
    Approved,
    Archived,
}

impl ProjectStatus {
    pub fn badge_label(&self) -> &'static str {
        match self {
            Self::Designed => "DESIGNED",
            Self::UnderReview => "UNDER REVIEW",
            Self::Approved => "APPROVED",
            Self::Archived => "ARCHIVED",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProjectItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub last_modified: String,
    pub status: ProjectStatus,
    pub elements_count: usize,
    pub nodes_count: usize,
    pub max_deflection_mm: f64,
    pub min_ari: f64,
}

#[derive(Debug, Clone)]
pub struct StructuralMember {
    pub id: usize,
    pub name: String,
    pub section: String,
    pub material: String,
    pub length: f64,
    pub axial_force_kn: f64,
    pub capacity_kn: f64,
    pub utilization: f64,
    pub ari_score: f64,
    pub criticality_index: f64,
    pub is_critical: bool,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub is_user: bool,
    pub message: String,
    pub time: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarTab {
    Geometry,
    Sections,
    Materials,
    Loads,
    Settings,
}

/// A stored parametric structural component in the local database library
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseComponent {
    pub id: String,
    pub name: String,
    pub category: String, // e.g. "Truss", "Portal Frame", "Space Frame"
    pub description: String,
    pub span_m: f64,
    pub height_m: f64,
    pub divisions: usize,
    pub default_section: String,
    pub driving_formula: String, // e.g. "H = L / 7.5"
    pub elements_count: usize,
    pub nodes_count: usize,
    pub est_weight_ton: f64,
    pub min_system_ari: f64,
    pub created_at: String,
}

impl DatabaseComponent {
    pub fn warren_truss_default() -> Self {
        Self {
            id: "comp_warren_24m".to_string(),
            name: "Standard Warren Girder (24m)".to_string(),
            category: "Steel Truss".to_string(),
            description: "24m modular Warren steel bridge truss with diagonal braces and top chord loads.".to_string(),
            span_m: 24.0,
            height_m: 3.2,
            divisions: 6,
            default_section: "W14x90".to_string(),
            driving_formula: "H = L / 7.5".to_string(),
            elements_count: 25,
            nodes_count: 14,
            est_weight_ton: 12.8,
            min_system_ari: 0.942,
            created_at: "2026-10-30".to_string(),
        }
    }

    pub fn pratt_truss_default() -> Self {
        Self {
            id: "comp_pratt_30m".to_string(),
            name: "Industrial Pratt Roof Truss (30m)".to_string(),
            category: "Roof Structure".to_string(),
            description: "30m heavy industrial pitched roof truss with vertical tension and diagonal compression ties.".to_string(),
            span_m: 30.0,
            height_m: 4.5,
            divisions: 8,
            default_section: "W12x65".to_string(),
            driving_formula: "H = L / 6.67".to_string(),
            elements_count: 33,
            nodes_count: 18,
            est_weight_ton: 18.4,
            min_system_ari: 0.915,
            created_at: "2026-10-30".to_string(),
        }
    }

    pub fn diagrid_module_default() -> Self {
        Self {
            id: "comp_diagrid_48m".to_string(),
            name: "High-Rise Diagrid Perimeter Module".to_string(),
            category: "Space Frame".to_string(),
            description: "Modular triangular interlocking diagrid system for tall building wind/seismic shear resistance.".to_string(),
            span_m: 18.0,
            height_m: 12.0,
            divisions: 4,
            default_section: "HSS16x16x0.5".to_string(),
            driving_formula: "θ = 65° (Optimum Diagrid Angle)".to_string(),
            elements_count: 42,
            nodes_count: 20,
            est_weight_ton: 26.5,
            min_system_ari: 0.968,
            created_at: "2026-10-30".to_string(),
        }
    }
}
