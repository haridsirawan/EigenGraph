//! models.rs — Domain models and state definitions for EigenGraph.

use serde::{Deserialize, Serialize};

/// The 5 Top-Level Navigation Pillars of EigenGraph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NavigationPillar {
    MyProject,
    Parametric,
    Build,
    Analyze,
    Settings,
}

impl NavigationPillar {
    pub fn title(&self) -> &'static str {
        match self {
            Self::MyProject => "📑 MY PROJECT",
            Self::Parametric => "📐 PARAMETRIC",
            Self::Build => "🏗️ BUILD",
            Self::Analyze => "⚡ ANALYZE",
            Self::Settings => "⚙️ SETTINGS",
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            Self::MyProject => "Portfolio & Client Presentation",
            Self::Parametric => "AutoCAD Free Drawing & Parametric Database",
            Self::Build => "3D Assembly Canvas & Modeling Workspace",
            Self::Analyze => "Structural Health & SVD Guard",
            Self::Settings => "Preferences & Integrations",
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
