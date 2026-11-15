//! models.rs — Central Data Structures for EigenGraph.
//! Contains model entities, navigation pillars, project catalogs, and UI state models.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationPillar {
    MyLibrary,
    AiAgent,
    ResearchConcept,
    ParametricDED,
    GeometryAssembly,
    AnalysisSimulation,
    TimeScheduling,
    CostBudgeting,
    SustainabilityEnergy,
    FacilityManagement,
    SafetyHealth,
    LeanConstruction,
    IndustrializedAutomation,
    Prediction,
    ReportPublishing,
}

impl NavigationPillar {
    pub fn all() -> &'static [NavigationPillar] {
        &[
            NavigationPillar::MyLibrary,
            NavigationPillar::AiAgent,
            NavigationPillar::ResearchConcept,
            NavigationPillar::ParametricDED,
            NavigationPillar::GeometryAssembly,
            NavigationPillar::AnalysisSimulation,
            NavigationPillar::TimeScheduling,
            NavigationPillar::CostBudgeting,
            NavigationPillar::SustainabilityEnergy,
            NavigationPillar::FacilityManagement,
            NavigationPillar::SafetyHealth,
            NavigationPillar::LeanConstruction,
            NavigationPillar::IndustrializedAutomation,
            NavigationPillar::Prediction,
            NavigationPillar::ReportPublishing,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::MyLibrary => "VAULT & PROJECT RECORDS",
            Self::AiAgent => "AGENT & VIRTUAL FIRM",
            Self::ResearchConcept => "1. PLANNING & GIS INCEPTION",
            Self::ParametricDED => "2. DRAFTING & PARAMETRIC STUDIO",
            Self::GeometryAssembly => "3. GEOMETRY & CLEAN BIM ASSEMBLY",
            Self::AnalysisSimulation => "GEO-STRUCTURAL ANALYSIS & MULTI-HAZARD",
            Self::TimeScheduling => "4. SCHEDULING & 4D PHASED ERECTION",
            Self::CostBudgeting => "5. BUDGETING & 5D HEATMAP COSTING",
            Self::SustainabilityEnergy => "6. SUSTAINABILITY & 6D HELIODON LCA",
            Self::FacilityManagement => "7. FACILITY & 7D DIGITAL TWIN",
            Self::SafetyHealth => "8. SAFETY & 8D CRANE SWEEP RISK",
            Self::LeanConstruction => "9. CONSTRUCTION & 9D SITE LOGISTICS",
            Self::IndustrializedAutomation => "10. AUTOMATION & 10D DfMA PREFAB",
            Self::Prediction => "PREDICTION & AI STRUCTURAL AGING",
            Self::ReportPublishing => "REPORT PUBLISHING SUITE (DOCS, SHEETS, SLIDES)",
        }
    }

    pub fn badge(&self) -> &'static str {
        match self {
            Self::MyLibrary => "VAULT",
            Self::AiAgent => "AGENTS",
            Self::ResearchConcept => "1D",
            Self::ParametricDED => "2D",
            Self::GeometryAssembly => "3D",
            Self::AnalysisSimulation => "SIM",
            Self::TimeScheduling => "4D",
            Self::CostBudgeting => "5D",
            Self::SustainabilityEnergy => "6D",
            Self::FacilityManagement => "7D",
            Self::SafetyHealth => "8D",
            Self::LeanConstruction => "9D",
            Self::IndustrializedAutomation => "10D",
            Self::Prediction => "PRED",
            Self::ReportPublishing => "PUB",
        }
    }

    pub fn dimension_tag(&self) -> &'static str {
        self.badge()
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::MyLibrary => "VLT",
            Self::AiAgent => "AI",
            Self::ResearchConcept => "1D",
            Self::ParametricDED => "2D",
            Self::GeometryAssembly => "3D",
            Self::AnalysisSimulation => "FEA",
            Self::TimeScheduling => "4D",
            Self::CostBudgeting => "5D",
            Self::SustainabilityEnergy => "6D",
            Self::FacilityManagement => "7D",
            Self::SafetyHealth => "8D",
            Self::LeanConstruction => "9D",
            Self::IndustrializedAutomation => "10D",
            Self::Prediction => "PRED",
            Self::ReportPublishing => "PUB",
        }
    }

    pub fn app_icon(&self) -> crate::icons::AppIcon {
        use crate::icons::AppIcon;
        match self {
            Self::MyLibrary => AppIcon::Folder,
            Self::AiAgent => AppIcon::Bot,
            Self::ResearchConcept => AppIcon::Compass,
            Self::ParametricDED => AppIcon::RulerSquare,
            Self::GeometryAssembly => AppIcon::Cube3D,
            Self::AnalysisSimulation => AppIcon::Bolt,
            Self::TimeScheduling => AppIcon::Clock,
            Self::CostBudgeting => AppIcon::Coins,
            Self::SustainabilityEnergy => AppIcon::Leaf,
            Self::FacilityManagement => AppIcon::Building,
            Self::SafetyHealth => AppIcon::Shield,
            Self::LeanConstruction => AppIcon::Recycle,
            Self::IndustrializedAutomation => AppIcon::Cpu,
            Self::Prediction => AppIcon::Pulse,
            Self::ReportPublishing => AppIcon::Document,
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            Self::MyLibrary => "Local Obsidian-style Vault: Project records, parametric component database, and SNI/AISC regulatory specifications.",
            Self::AiAgent => "Autonomous Virtual Engineering Firm: Multi-agent organigram (PM, Senior Drafter, Structural Analyst, QS Specialist) & collaboration board.",
            Self::ResearchConcept => "Interactive GIS map container, site geodetic telemetry HUD (WGS84 / UTM), cadastral zoning, and pusGen seismic hazard spectra.",
            Self::ParametricDED => "AutoCAD-style 2D vector drafting canvas with magnetic snaps + live real-time 3D solid extrusion companion and rebar detailer.",
            Self::GeometryAssembly => "Clean BIM spatial assembly workspace: Multi-level Z-planes, structural grid network (A-B-C, 1-2-3), and component stamping.",
            Self::AnalysisSimulation => "3D Geo-Structural sandbox on GIS terrain: X-Ray subsurface ground slider, soil bearing capacity, FEA solver, SVD kinematic guard, and ARI index.",
            Self::TimeScheduling => "4D Phased erection timeline slider, Critical Path Method (CPM) Gantt schedule, and staged construction simulation.",
            Self::CostBudgeting => "5D 3D Cost Heatmap canvas, dynamic Quantity Take-Off (QTO), and parametric Bill of Quantities (BoQ) financial ledger.",
            Self::SustainabilityEnergy => "6D Solar heliodon sun-path dome, cast shadow tracer, operational energy demand, and embodied carbon gradient (A1–A3).",
            Self::FacilityManagement => "7D Digital Twin lifecycle management: Spatial MEP equipment asset tags, IoT telemetry pins, and maintenance lifecycle logs.",
            Self::SafetyHealth => "8D Tower crane working sweep envelope, radius hook capacity checks, open slab edge fall hazard boundary guards.",
            Self::LeanConstruction => "9D Material staging and laydown zones, Just-In-Time (JIT) delivery logistics, and concrete mixer transit routing.",
            Self::IndustrializedAutomation => "10D Design for Manufacture and Assembly (DfMA): Modular precast pods, road freight transport envelope checks, and crane rigging equilibrium.",
            Self::Prediction => "AI structural stress concentration forecasts, non-linear dynamic drift anomalies, progressive collapse risk, and serviceability aging.",
            Self::ReportPublishing => "Living Publishing Suite: Docs (WYSIWYG DED drawings + title block), Sheets (dynamic engineering formulas & BoQ), and Slides (executive deck with 3D viewports).",
        }
    }

    pub fn clean_name(&self) -> &'static str {
        match self {
            Self::MyLibrary => "Vault",
            Self::AiAgent => "Agent",
            Self::ResearchConcept => "Planning",
            Self::ParametricDED => "Drafting",
            Self::GeometryAssembly => "Geometry",
            Self::AnalysisSimulation => "Analysis",
            Self::TimeScheduling => "Scheduling",
            Self::CostBudgeting => "Budgeting",
            Self::SustainabilityEnergy => "Sustainability",
            Self::FacilityManagement => "Facility",
            Self::SafetyHealth => "Safety",
            Self::LeanConstruction => "Construction",
            Self::IndustrializedAutomation => "Automation",
            Self::Prediction => "Prediction",
            Self::ReportPublishing => "Report",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectStatus {
    Designed,
    UnderReview,
    Approved,
    Archived,
    Safe,
    ReviewNeeded,
}

impl ProjectStatus {
    pub fn badge_label(&self) -> &'static str {
        match self {
            Self::Designed => "DESIGNED",
            Self::UnderReview => "UNDER REVIEW",
            Self::Approved => "APPROVED",
            Self::Archived => "ARCHIVED",
            Self::Safe => "SAFE (ARI ≥ 0.70)",
            Self::ReviewNeeded => "REVIEW NEEDED (ARI < 0.70)",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProjectItem {
    pub id: String,
    pub title: String,
    pub category: String,
    pub description: String,
    pub elements_count: usize,
    pub nodes_count: usize,
    pub min_ari: f64,
    pub max_deflection_mm: f64,
    pub status: ProjectStatus,
    pub last_modified: String,
    pub is_favorite: bool,
    pub is_featured: bool,
    pub tag: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParametricVariable {
    pub name: String,
    pub expression: String,
    pub evaluated_value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            Self::Select => ("Select", "Esc"),
            Self::Line => ("Line", "L"),
            Self::Polyline => ("Polyline", "PL"),
            Self::Arc => ("Arc", "A"),
            Self::Circle => ("Circle", "C"),
            Self::Rectangle => ("Rectangle", "REC"),
            Self::Dimension => ("Dimension", "DIM"),
            Self::Trim => ("Trim", "TR"),
            Self::Offset => ("Offset", "O"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuralMember {
    pub id: usize,
    pub name: String,
    pub section: String,
    pub force_kn: f64,
    pub capacity_kn: f64,
    pub utilization: f64,
    pub moment_my_knm: f64,
    pub moment_mz_knm: f64,
    pub is_critical: bool,
    pub is_active: bool,
    pub ari_score: f64,
    pub svd_sensitivity: f64,
    #[serde(default)]
    pub ari: f64,
    #[serde(default)]
    pub dcr_interaction: f64,
    #[serde(default)]
    pub shear_vy_kn: f64,
    #[serde(default)]
    pub svd_margin: f64,
    #[serde(default)]
    pub ci: f64,
    #[serde(default)]
    pub rr: f64,
}

impl Default for StructuralMember {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            section: "W14x90".to_string(),
            force_kn: 0.0,
            capacity_kn: 100.0,
            utilization: 0.0,
            moment_my_knm: 0.0,
            moment_mz_knm: 0.0,
            is_critical: false,
            is_active: true,
            ari_score: 1.0,
            svd_sensitivity: 0.0,
            ari: 1.0,
            dcr_interaction: 0.0,
            shear_vy_kn: 0.0,
            svd_margin: 1.0,
            ci: 0.0,
            rr: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub is_user: bool,
    pub message: String,
    pub time: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarTab {
    Properties,
    ProjectBrowser,
    ClashMatrix,
    ModelAudit,
    BcfIssues,
    Geometry,
    Modify,
    Layers,
    Sections,
    Materials,
    Loads,
    Settings,
}

impl SidebarTab {
    pub fn icon_and_name(&self) -> (&'static str, &'static str) {
        match self {
            Self::Properties => ("PRP", "Properties & IFC Psets"),
            Self::ProjectBrowser => ("BRW", "Project Browser & SBS"),
            Self::ClashMatrix => ("CLS", "Clash Detection Matrix"),
            Self::ModelAudit => ("AUD", "Solibri BIM Audit"),
            Self::BcfIssues => ("BCF", "BCF 3.0 Issues"),
            Self::Geometry => ("GEO", "Geometry & Wizards"),
            Self::Modify => ("MOD", "Modify & Reinforce"),
            Self::Layers => ("LAY", "Layers & Worksets"),
            Self::Sections => ("SEC", "Sections"),
            Self::Materials => ("MAT", "Materials"),
            Self::Loads => ("LOD", "Loads"),
            Self::Settings => ("SET", "Settings"),
        }
    }

    pub fn app_icon(&self) -> crate::icons::AppIcon {
        use crate::icons::AppIcon;
        match self {
            Self::Properties => AppIcon::Clipboard,
            Self::ProjectBrowser => AppIcon::Folder,
            Self::ClashMatrix => AppIcon::AlertTriangle,
            Self::ModelAudit => AppIcon::Shield,
            Self::BcfIssues => AppIcon::CommentNote,
            Self::Geometry => AppIcon::Cube3D,
            Self::Modify => AppIcon::Pencil,
            Self::Layers => AppIcon::Layers,
            Self::Sections => AppIcon::DimensionRuler,
            Self::Materials => AppIcon::Wall,
            Self::Loads => AppIcon::ArrowDown,
            Self::Settings => AppIcon::SettingsGear,
        }
    }
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
    #[serde(default)]
    pub variables: Vec<ParametricVariable>,
}

impl Default for DatabaseComponent {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            category: String::new(),
            description: String::new(),
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
            variables: Vec::new(),
        }
    }
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
            variables: Vec::new(),
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
            variables: Vec::new(),
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
            variables: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 27: Master BIM Superstation Models (Family Catalog, Dynamo Nodes, Sheets)
// ---------------------------------------------------------------------------

/// Standardized BIM Component Family for Global Library (UniFormat & MasterFormat)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BimFamilyItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub classification: String,
    pub ifc_entity: String,
    pub lod: String,
    pub material: String,
    pub dimensions_desc: String,
    pub embodied_carbon_kg: f32,
    pub fire_rating_hr: f32,
    pub type_params: Vec<(String, String)>,
    pub instance_params: Vec<(String, String)>,
}

impl BimFamilyItem {
    pub fn default_catalog() -> Vec<Self> {
        vec![
            Self {
                id: "fam_w14x90".to_string(),
                name: "AISC W14x90 Wide Flange Steel Girder".to_string(),
                category: "Structural Framing".to_string(),
                classification: "MasterFormat 05 12 00 / UniFormat B1010.10".to_string(),
                ifc_entity: "IfcBeam".to_string(),
                lod: "LOD 350".to_string(),
                material: "ASTM A992 Gr 50 Steel".to_string(),
                dimensions_desc: "d=356mm, bf=368mm, tf=18.0mm, tw=11.2mm".to_string(),
                embodied_carbon_kg: 134.2,
                fire_rating_hr: 2.0,
                type_params: vec![
                    ("Section Depth (d)".to_string(), "356 mm".to_string()),
                    ("Flange Width (bf)".to_string(), "368 mm".to_string()),
                    ("Elastic Modulus (Sx)".to_string(), "2340 cm³".to_string()),
                    ("Yield Strength (Fy)".to_string(), "345 MPa".to_string()),
                ],
                instance_params: vec![
                    ("Structural Mark".to_string(), "B-101".to_string()),
                    ("Top Elevation".to_string(), "+4.000 m".to_string()),
                    ("Camber".to_string(), "15 mm".to_string()),
                    ("Phase Created".to_string(), "Phase 1 - Superstructure".to_string()),
                ],
            },
            Self {
                id: "fam_rc_col_600".to_string(),
                name: "Cast-in-Place RC Square Column 600x600".to_string(),
                category: "Columns".to_string(),
                classification: "MasterFormat 03 30 00 / UniFormat B1010.20".to_string(),
                ifc_entity: "IfcColumn".to_string(),
                lod: "LOD 350".to_string(),
                material: "Concrete C40/50 + Rebar D25".to_string(),
                dimensions_desc: "600 mm x 600 mm (12-D25 Main + D10@100 Ties)".to_string(),
                embodied_carbon_kg: 285.6,
                fire_rating_hr: 3.0,
                type_params: vec![
                    ("Width / Depth".to_string(), "600 x 600 mm".to_string()),
                    ("Compressive Strength f'c".to_string(), "40 MPa".to_string()),
                    ("Rebar Longitudinal Ratio".to_string(), "2.45%".to_string()),
                    ("Cover Depth".to_string(), "40 mm".to_string()),
                ],
                instance_params: vec![
                    ("Column Tag".to_string(), "C-1".to_string()),
                    ("Base Level".to_string(), "Level 0 GF".to_string()),
                    ("Top Level".to_string(), "Level 1 (+4.00m)".to_string()),
                    ("Slenderness kl/r".to_string(), "24.2 (Short Column)".to_string()),
                ],
            },
            Self {
                id: "fam_curtain_wall_mullion".to_string(),
                name: "Double-Glazed Unitized Curtain Wall Facade".to_string(),
                category: "Curtain Wall".to_string(),
                classification: "MasterFormat 08 44 00 / UniFormat B2020".to_string(),
                ifc_entity: "IfcCurtainWall".to_string(),
                lod: "LOD 300".to_string(),
                material: "Extruded Aluminium 6063-T6 + Low-E Glass".to_string(),
                dimensions_desc: "1500 mm module width x 4000 mm story height".to_string(),
                embodied_carbon_kg: 88.4,
                fire_rating_hr: 1.0,
                type_params: vec![
                    ("U-Value Thermal".to_string(), "1.25 W/m²K".to_string()),
                    ("SHGC Solar Heat Gain".to_string(), "0.32".to_string()),
                    ("Acoustic Rating (STC)".to_string(), "42 dB".to_string()),
                    ("Mullion Profile Depth".to_string(), "180 mm".to_string()),
                ],
                instance_params: vec![
                    ("Facade Elevation".to_string(), "South Elevation Grid 1-4".to_string()),
                    ("Wind Pressure Design".to_string(), "2.40 kPa (ASCE 7-22)".to_string()),
                    ("Glass Type".to_string(), "6mm Low-E + 12mm Argon + 6mm Clr".to_string()),
                ],
            },
            Self {
                id: "fam_hvac_ahu_rooftop".to_string(),
                name: "Variable Air Volume Rooftop AHU Package".to_string(),
                category: "MEP Equipment".to_string(),
                classification: "MasterFormat 23 74 13 / UniFormat D3030".to_string(),
                ifc_entity: "IfcAirTerminal".to_string(),
                lod: "LOD 350".to_string(),
                material: "Galvanized Steel Casing + EC Motors".to_string(),
                dimensions_desc: "3200 mm L x 1800 mm W x 1650 mm H".to_string(),
                embodied_carbon_kg: 420.0,
                fire_rating_hr: 0.0,
                type_params: vec![
                    ("Nominal Airflow".to_string(), "12,000 CFM (5,660 L/s)".to_string()),
                    ("Cooling Capacity".to_string(), "350 kW (100 Tons)".to_string()),
                    ("External Static Pressure".to_string(), "500 Pa".to_string()),
                    ("Refrigerant".to_string(), "R-454B (Low GWP)".to_string()),
                ],
                instance_params: vec![
                    ("Equipment Tag".to_string(), "AHU-ROOF-01".to_string()),
                    ("Location".to_string(), "Rooftop Plant Deck Grid B-2".to_string()),
                    ("BMS Address".to_string(), "BACnet/IP ID 40012".to_string()),
                ],
            },
            Self {
                id: "fam_pt_i_girder_30m".to_string(),
                name: "PCI Type IV Prestressed I-Girder 30m Span".to_string(),
                category: "Civil Infrastructure".to_string(),
                classification: "MasterFormat 34 71 00 / UniFormat G2010".to_string(),
                ifc_entity: "IfcBeam".to_string(),
                lod: "LOD 350".to_string(),
                material: "Precast Concrete 55 MPa + 0.6\" Strands".to_string(),
                dimensions_desc: "Height 1370mm, Length 30.0m, Web 200mm".to_string(),
                embodied_carbon_kg: 680.0,
                fire_rating_hr: 3.0,
                type_params: vec![
                    ("Span Length".to_string(), "30.0 m".to_string()),
                    ("Girder Depth".to_string(), "1370 mm".to_string()),
                    ("PT Tendons".to_string(), "34 x 15.2mm Low-Relaxation Strands".to_string()),
                    ("Prestress Force Pjack".to_string(), "6,800 kN".to_string()),
                ],
                instance_params: vec![
                    ("Span Mark".to_string(), "GIRDER-SPAN-02-G1".to_string()),
                    ("Casting Date".to_string(), "2026-10-15".to_string()),
                    ("Measured Camber".to_string(), "42 mm".to_string()),
                ],
            },
            Self {
                id: "fam_box_culvert".to_string(),
                name: "Precast Stormwater Box Culvert 3.0x2.5m".to_string(),
                category: "Civil Infrastructure".to_string(),
                classification: "MasterFormat 33 42 16 / UniFormat G3020".to_string(),
                ifc_entity: "IfcDistributionElement".to_string(),
                lod: "LOD 300".to_string(),
                material: "Reinforced Concrete C35/45".to_string(),
                dimensions_desc: "3000 mm span x 2500 mm rise x 2000 mm lay length".to_string(),
                embodied_carbon_kg: 510.0,
                fire_rating_hr: 0.0,
                type_params: vec![
                    ("Hydraulic Capacity".to_string(), "18.5 m³/s (Manning n=0.013)".to_string()),
                    ("Wall Thickness".to_string(), "250 mm".to_string()),
                    ("Live Load Rating".to_string(), "AASHTO HL-93 Highway Loading".to_string()),
                ],
                instance_params: vec![
                    ("Station Invert".to_string(), "Sta 0+450 Invert EL -1.25m".to_string()),
                    ("Bedding Condition".to_string(), "Class B Granular Cradle".to_string()),
                ],
            },
        ]
    }
}

/// Visual Node Programming Category for Dynamo/Grasshopper Studio
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualNodeCategory {
    Input,
    MathLogic,
    GeometryGen,
    BimBake,
}

impl VisualNodeCategory {
    pub fn badge_color_and_name(&self) -> (&'static str, (u8, u8, u8)) {
        match self {
            Self::Input => ("Input Parameter", (52, 199, 89)),
            Self::MathLogic => ("Math & Transform", (255, 149, 0)),
            Self::GeometryGen => ("3D Solid Generator", (10, 132, 255)),
            Self::BimBake => ("BIM Scene Baker", (175, 82, 222)),
        }
    }
}

/// A computational node on the visual programming graph canvas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualGraphNode {
    pub id: usize,
    pub title: String,
    pub category: VisualNodeCategory,
    pub pos: [f32; 2],
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub value_display: String,
    pub is_active: bool,
}

/// Connection wire linking two sockets on the visual programming canvas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualGraphWire {
    pub from_node: usize,
    pub from_socket: usize,
    pub to_node: usize,
    pub to_socket: usize,
}

/// Full Visual Dynamo/Grasshopper computational graph model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualNodeGraph {
    pub nodes: Vec<VisualGraphNode>,
    pub wires: Vec<VisualGraphWire>,
}

impl VisualNodeGraph {
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            wires: Vec::new(),
        }
    }
}

impl Default for VisualNodeGraph {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
impl VisualNodeGraph {
    #[allow(dead_code)]
    pub fn sample_default_graph() -> Self {
        Self {
            nodes: vec![
                VisualGraphNode {
                    id: 1,
                    title: "Slider: Span L (m)".to_string(),
                    category: VisualNodeCategory::Input,
                    pos: [40.0, 60.0],
                    inputs: vec![],
                    outputs: vec!["Length (m)".to_string()],
                    value_display: "24.0".to_string(),
                    is_active: true,
                },
                VisualGraphNode {
                    id: 2,
                    title: "Slider: Bay Width B (m)".to_string(),
                    category: VisualNodeCategory::Input,
                    pos: [40.0, 190.0],
                    inputs: vec![],
                    outputs: vec!["Width (m)".to_string()],
                    value_display: "6.0".to_string(),
                    is_active: true,
                },
                VisualGraphNode {
                    id: 3,
                    title: "Math: Formula H = L / 8".to_string(),
                    category: VisualNodeCategory::MathLogic,
                    pos: [290.0, 60.0],
                    inputs: vec!["L".to_string()],
                    outputs: vec!["Depth H".to_string()],
                    value_display: "3.00 m".to_string(),
                    is_active: true,
                },
                VisualGraphNode {
                    id: 4,
                    title: "Truss Octet Generator".to_string(),
                    category: VisualNodeCategory::GeometryGen,
                    pos: [520.0, 90.0],
                    inputs: vec!["L".to_string(), "B".to_string(), "H".to_string()],
                    outputs: vec!["Mesh 3D".to_string(), "Elem Count".to_string()],
                    value_display: "48 Elements".to_string(),
                    is_active: true,
                },
                VisualGraphNode {
                    id: 5,
                    title: "Bake to BIM 3D Scene".to_string(),
                    category: VisualNodeCategory::BimBake,
                    pos: [790.0, 110.0],
                    inputs: vec!["Geometry".to_string()],
                    outputs: vec!["IfcExport".to_string()],
                    value_display: "Live Synced".to_string(),
                    is_active: true,
                },
            ],
            wires: vec![
                VisualGraphWire { from_node: 1, from_socket: 0, to_node: 3, to_socket: 0 },
                VisualGraphWire { from_node: 1, from_socket: 0, to_node: 4, to_socket: 0 },
                VisualGraphWire { from_node: 2, from_socket: 0, to_node: 4, to_socket: 1 },
                VisualGraphWire { from_node: 3, from_socket: 0, to_node: 4, to_socket: 2 },
                VisualGraphWire { from_node: 4, from_socket: 0, to_node: 5, to_socket: 0 },
            ],
        }
    }
}

/// A Drawing Sheet Item for Production Working Drawings (DED)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawingSheetItem {
    pub id: String,
    pub sheet_number: String,
    pub sheet_title: String,
    pub paper_size: String,
    pub scale: String,
    pub discipline: String,
    pub engineer_stamp: String,
    pub revision: String,
    pub viewports: Vec<String>,
}

impl DrawingSheetItem {
    pub fn default_set() -> Vec<Self> {
        vec![
            Self {
                id: "sheet_s101".to_string(),
                sheet_number: "S-101".to_string(),
                sheet_title: "FOUNDATION & GROUND FLOOR FRAMING PLAN".to_string(),
                paper_size: "ISO A1 (841 x 594 mm)".to_string(),
                scale: "1 : 100".to_string(),
                discipline: "Structural (STR)".to_string(),
                engineer_stamp: "PE Structural License #49120-ST (Digital Sealed)".to_string(),
                revision: "REV-0 (Issued for Construction)".to_string(),
                viewports: vec![
                    "Main Framing Grid A-D & 1-4 (Scale 1:100)".to_string(),
                    "Isolated Footing Schedule Table".to_string(),
                    "Typical RC Column Detail Callout 1:20".to_string(),
                ],
            },
            Self {
                id: "sheet_s201".to_string(),
                sheet_number: "S-201".to_string(),
                sheet_title: "LEVEL 2 TYPICAL FLOOR BEAM & SLAB REBAR".to_string(),
                paper_size: "ISO A1 (841 x 594 mm)".to_string(),
                scale: "1 : 100".to_string(),
                discipline: "Structural (STR)".to_string(),
                engineer_stamp: "PE Structural License #49120-ST (Digital Sealed)".to_string(),
                revision: "REV-B (Coordination Approval)".to_string(),
                viewports: vec![
                    "Floor Slab Two-Way Mesh Detailing".to_string(),
                    "W14x90 Moment Frame Connection Detail 1:10".to_string(),
                    "Bar Bending Schedule (BBS) Table".to_string(),
                ],
            },
            Self {
                id: "sheet_a101".to_string(),
                sheet_number: "A-101".to_string(),
                sheet_title: "ARCHITECTURAL FLOOR PLAN & ROOM SCHEDULE".to_string(),
                paper_size: "ISO A1 (841 x 594 mm)".to_string(),
                scale: "1 : 100".to_string(),
                discipline: "Architectural (ARC)".to_string(),
                engineer_stamp: "Licensed Architect #IAI-88241".to_string(),
                revision: "REV-1 (Client Signoff)".to_string(),
                viewports: vec![
                    "Ground Floor Partition & Door Layout".to_string(),
                    "Room Boundary Area Tag Matrix".to_string(),
                    "Curtain Wall Mullion Elevation 1:50".to_string(),
                ],
            },
            Self {
                id: "sheet_m301".to_string(),
                sheet_number: "M-301".to_string(),
                sheet_title: "HVAC DUCTING & FIRE SPRINKLER DISTRIBUTION".to_string(),
                paper_size: "ISO A1 (841 x 594 mm)".to_string(),
                scale: "1 : 100".to_string(),
                discipline: "Mechanical & Fire (MEP)".to_string(),
                engineer_stamp: "Professional Mechanical Engineer #MEP-3310".to_string(),
                revision: "REV-0 (Issued for Construction)".to_string(),
                viewports: vec![
                    "Level 1 Supply/Return Air Run (Scale 1:100)".to_string(),
                    "VAV Terminal Box Wiring & Riser Diagram".to_string(),
                    "Automatic Sprinkler Hydraulic Calculation Summary".to_string(),
                ],
            },
        ]
    }
}

// ============================================================================
// Phase 28: 4D-7D BIM Superstation Data Models
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstructionSimulationPhase {
    pub id: String,
    pub name: String,
    pub day_start: u32,
    pub day_end: u32,
    pub planned_progress_pct: f32,
    pub actual_progress_pct: f32,
    pub manpower_count: u32,
    pub equipment_count: u32,
    pub concrete_volume_m3: f32,
    pub color_rgb: (u8, u8, u8),
    pub wbs_code: String,
    pub is_critical: bool,
}

impl ConstructionSimulationPhase {
    pub fn default_phases() -> Vec<Self> {
        vec![
            Self {
                id: "ph_sub".to_string(),
                name: "1. Substructure Piling & Deep Foundation".to_string(),
                day_start: 1,
                day_end: 45,
                planned_progress_pct: 100.0,
                actual_progress_pct: 100.0,
                manpower_count: 42,
                equipment_count: 6,
                concrete_volume_m3: 1450.0,
                color_rgb: (160, 100, 45),
                wbs_code: "02.40.00".to_string(),
                is_critical: true,
            },
            Self {
                id: "ph_raft".to_string(),
                name: "2. Raft Slab & Perimeter Retaining Walls".to_string(),
                day_start: 46,
                day_end: 90,
                planned_progress_pct: 100.0,
                actual_progress_pct: 96.0,
                manpower_count: 58,
                equipment_count: 8,
                concrete_volume_m3: 2200.0,
                color_rgb: (120, 130, 145),
                wbs_code: "03.30.00".to_string(),
                is_critical: true,
            },
            Self {
                id: "ph_steel".to_string(),
                name: "3. Structural Steel Erection L1-L15 & Composite Decks".to_string(),
                day_start: 91,
                day_end: 195,
                planned_progress_pct: 78.0,
                actual_progress_pct: 82.5,
                manpower_count: 85,
                equipment_count: 12,
                concrete_volume_m3: 1850.0,
                color_rgb: (0, 122, 255),
                wbs_code: "05.12.00".to_string(),
                is_critical: true,
            },
            Self {
                id: "ph_facade".to_string(),
                name: "4. Unitized Curtain Wall & Building Envelope".to_string(),
                day_start: 180,
                day_end: 270,
                planned_progress_pct: 45.0,
                actual_progress_pct: 42.0,
                manpower_count: 64,
                equipment_count: 5,
                concrete_volume_m3: 0.0,
                color_rgb: (52, 199, 89),
                wbs_code: "08.44.00".to_string(),
                is_critical: false,
            },
            Self {
                id: "ph_mep".to_string(),
                name: "5. MEP Core Risers, HVAC & Fire Suppression".to_string(),
                day_start: 210,
                day_end: 320,
                planned_progress_pct: 30.0,
                actual_progress_pct: 28.0,
                manpower_count: 72,
                equipment_count: 4,
                concrete_volume_m3: 0.0,
                color_rgb: (255, 149, 0),
                wbs_code: "23.00.00".to_string(),
                is_critical: true,
            },
            Self {
                id: "ph_com".to_string(),
                name: "6. Integrated Testing, Commissioning & Handover".to_string(),
                day_start: 321,
                day_end: 360,
                planned_progress_pct: 5.0,
                actual_progress_pct: 0.0,
                manpower_count: 24,
                equipment_count: 2,
                concrete_volume_m3: 0.0,
                color_rgb: (175, 82, 222),
                wbs_code: "01.91.00".to_string(),
                is_critical: true,
            },
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurrencyUnit {
    Idr,
    Usd,
    Eur,
    Sgd,
}

impl CurrencyUnit {
    pub fn all() -> &'static [CurrencyUnit] {
        &[CurrencyUnit::Idr, CurrencyUnit::Usd, CurrencyUnit::Eur, CurrencyUnit::Sgd]
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Idr => "IDR",
            Self::Usd => "USD",
            Self::Eur => "EUR",
            Self::Sgd => "SGD",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Idr => "Rp",
            Self::Usd => "$",
            Self::Eur => "€",
            Self::Sgd => "S$",
        }
    }

    pub fn rate_from_idr(&self) -> f64 {
        match self {
            Self::Idr => 1.0,
            Self::Usd => 1.0 / 16_200.0,
            Self::Eur => 1.0 / 17_400.0,
            Self::Sgd => 1.0 / 12_100.0,
        }
    }

    pub fn format_value(&self, idr_val: f64) -> String {
        let converted = idr_val * self.rate_from_idr();
        match self {
            Self::Idr => {
                let rounded = idr_val.round() as i64;
                let s = rounded.abs().to_string();
                let mut res = String::new();
                let mut c = 0;
                for ch in s.chars().rev() {
                    if c > 0 && c % 3 == 0 { res.push('.'); }
                    res.push(ch);
                    c += 1;
                }
                let rev: String = res.chars().rev().collect();
                if rounded < 0 { format!("-Rp {}", rev) } else { format!("Rp {}", rev) }
            }
            Self::Usd => {
                let int_part = converted.round() as i64;
                let s = int_part.abs().to_string();
                let mut res = String::new();
                let mut c = 0;
                for ch in s.chars().rev() {
                    if c > 0 && c % 3 == 0 { res.push(','); }
                    res.push(ch);
                    c += 1;
                }
                let rev: String = res.chars().rev().collect();
                if int_part < 0 { format!("-${}", rev) } else { format!("${}", rev) }
            }
            Self::Eur => {
                let int_part = converted.round() as i64;
                let s = int_part.abs().to_string();
                let mut res = String::new();
                let mut c = 0;
                for ch in s.chars().rev() {
                    if c > 0 && c % 3 == 0 { res.push('.'); }
                    res.push(ch);
                    c += 1;
                }
                let rev: String = res.chars().rev().collect();
                if int_part < 0 { format!("-€{}", rev) } else { format!("€{}", rev) }
            }
            Self::Sgd => {
                let int_part = converted.round() as i64;
                let s = int_part.abs().to_string();
                let mut res = String::new();
                let mut c = 0;
                for ch in s.chars().rev() {
                    if c > 0 && c % 3 == 0 { res.push(','); }
                    res.push(ch);
                    c += 1;
                }
                let rev: String = res.chars().rev().collect();
                if int_part < 0 { format!("-S${}", rev) } else { format!("S${}", rev) }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WbsCostItem {
    pub wbs_code: String,
    pub description: String,
    pub division: String,
    pub unit: String,
    pub quantity: f64,
    pub unit_price_idr: f64,
    pub total_cost_idr: f64,
    pub progress_pct: f32,
}

impl WbsCostItem {
    pub fn default_catalog() -> Vec<Self> {
        vec![
            Self {
                wbs_code: "02.41.16".to_string(),
                description: "Deep Bored Piling D=1000mm (L=32m)".to_string(),
                division: "02 Substructure".to_string(),
                unit: "m".to_string(),
                quantity: 4800.0,
                unit_price_idr: 2_850_000.0,
                total_cost_idr: 13_680_000_000.0,
                progress_pct: 100.0,
            },
            Self {
                wbs_code: "03.31.00".to_string(),
                description: "Structural Concrete fc' 40 MPa (Foundation & Columns)".to_string(),
                division: "03 Cast-in-Place Concrete".to_string(),
                unit: "m3".to_string(),
                quantity: 5500.0,
                unit_price_idr: 1_450_000.0,
                total_cost_idr: 7_975_000_000.0,
                progress_pct: 88.0,
            },
            Self {
                wbs_code: "03.21.00".to_string(),
                description: "Deformed High-Strength Rebar BJTS 420B".to_string(),
                division: "03 Reinforcing Steel".to_string(),
                unit: "kg".to_string(),
                quantity: 680_000.0,
                unit_price_idr: 14_800.0,
                total_cost_idr: 10_064_000_000.0,
                progress_pct: 85.0,
            },
            Self {
                wbs_code: "05.12.23".to_string(),
                description: "Structural Steel Framing ASTM A992 (W-Shapes & Trusses)".to_string(),
                division: "05 Structural Metals".to_string(),
                unit: "ton".to_string(),
                quantity: 940.0,
                unit_price_idr: 28_500_000.0,
                total_cost_idr: 26_790_000_000.0,
                progress_pct: 76.0,
            },
            Self {
                wbs_code: "08.44.13".to_string(),
                description: "Glazed Aluminum Curtain Wall System (Double Low-E)".to_string(),
                division: "08 Openings & Glazing".to_string(),
                unit: "m2".to_string(),
                quantity: 7200.0,
                unit_price_idr: 2_600_000.0,
                total_cost_idr: 18_720_000_000.0,
                progress_pct: 42.0,
            },
            Self {
                wbs_code: "23.05.00".to_string(),
                description: "Common Work Results for HVAC & Central Chillers".to_string(),
                division: "23 Mechanical & HVAC".to_string(),
                unit: "lot".to_string(),
                quantity: 1.0,
                unit_price_idr: 15_200_000_000.0,
                total_cost_idr: 15_200_000_000.0,
                progress_pct: 25.0,
            },
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LcaStageEmission {
    pub stage_code: String,
    pub stage_name: String,
    pub emissions_t_co2e: f64,
    pub pct_of_total: f32,
    pub description: String,
    pub color_rgb: (u8, u8, u8),
}

impl LcaStageEmission {
    pub fn default_stages() -> Vec<Self> {
        vec![
            Self {
                stage_code: "A1-A3".to_string(),
                stage_name: "Raw Material & Product Manufacturing".to_string(),
                emissions_t_co2e: 1420.5,
                pct_of_total: 54.2,
                description: "Cradle-to-gate extraction of iron ore, blast furnace smelting, Portland cement clinker.".to_string(),
                color_rgb: (235, 75, 55),
            },
            Self {
                stage_code: "A4-A5".to_string(),
                stage_name: "Transport & Construction Process".to_string(),
                emissions_t_co2e: 312.4,
                pct_of_total: 11.9,
                description: "Fleet freight logistics, crane diesel consumption, batch plant site energy.".to_string(),
                color_rgb: (255, 149, 0),
            },
            Self {
                stage_code: "B1-B7".to_string(),
                stage_name: "Operational Energy & Building Use".to_string(),
                emissions_t_co2e: 648.0,
                pct_of_total: 24.7,
                description: "50-year structural serviceability, cathodic protection power, maintenance coating.".to_string(),
                color_rgb: (0, 122, 255),
            },
            Self {
                stage_code: "C1-C4".to_string(),
                stage_name: "Deconstruction & Waste Processing".to_string(),
                emissions_t_co2e: 140.2,
                pct_of_total: 5.3,
                description: "Controlled demolition, concrete crushing sorting, landfill transport.".to_string(),
                color_rgb: (142, 142, 147),
            },
            Self {
                stage_code: "Module D".to_string(),
                stage_name: "Beyond Lifecycle Circular Reuse Offset".to_string(),
                emissions_t_co2e: -480.0,
                pct_of_total: 3.9,
                description: "Net carbon credit from 95% structural steel electric arc furnace remelting recycling.".to_string(),
                color_rgb: (52, 199, 89),
            },
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GreenBuildingCredit {
    pub standard: String,
    pub category: String,
    pub credit_title: String,
    pub points_achieved: u32,
    pub points_max: u32,
    pub status: String,
}

impl GreenBuildingCredit {
    pub fn default_credits() -> Vec<Self> {
        vec![
            Self {
                standard: "GREENSHIP (GBCI)".to_string(),
                category: "Material Resource & Cycle (MRC)".to_string(),
                credit_title: "MRC 1: Recycled Content in Structural Steel (EAF > 75%)".to_string(),
                points_achieved: 3,
                points_max: 3,
                status: "Compliant".to_string(),
            },
            Self {
                standard: "GREENSHIP (GBCI)".to_string(),
                category: "Energy Efficiency & Conservation (EEC)".to_string(),
                credit_title: "EEC 1: Overall Thermal Transfer Value (OTTV < 35 W/m2)".to_string(),
                points_achieved: 14,
                points_max: 15,
                status: "Compliant".to_string(),
            },
            Self {
                standard: "LEED v4.1 BD+C".to_string(),
                category: "Materials and Resources (MR)".to_string(),
                credit_title: "Building Life-Cycle Impact Reduction (Option 4 Whole-Building LCA)".to_string(),
                points_achieved: 4,
                points_max: 4,
                status: "Compliant".to_string(),
            },
            Self {
                standard: "LEED v4.1 BD+C".to_string(),
                category: "Energy and Atmosphere (EA)".to_string(),
                credit_title: "Optimize Energy Performance (32% Improvement over ASHRAE 90.1)".to_string(),
                points_achieved: 16,
                points_max: 18,
                status: "Under Review".to_string(),
            },
            Self {
                standard: "BREEAM International".to_string(),
                category: "Mat 01 Life Cycle Impacts".to_string(),
                credit_title: "Environmental Product Declaration (ISO 14025 Type III EPD Verified)".to_string(),
                points_achieved: 5,
                points_max: 6,
                status: "Compliant".to_string(),
            },
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShmSensorTelemetry {
    pub sensor_id: String,
    pub sensor_name: String,
    pub location: String,
    pub sensor_type: String,
    pub current_value: f32,
    pub unit: String,
    pub threshold_warning: f32,
    pub threshold_critical: f32,
    pub status_level: String,
    pub sparkline_history: [f32; 8],
}

impl ShmSensorTelemetry {
    pub fn default_sensors() -> Vec<Self> {
        vec![
            Self {
                sensor_id: "SG-L01-C03".to_string(),
                sensor_name: "Fiber-Optic Strain Gauge SG-01".to_string(),
                location: "Grid B-2 Ground Column Base".to_string(),
                sensor_type: "Fiber Bragg Grating (FBG)".to_string(),
                current_value: 342.5,
                unit: "με".to_string(),
                threshold_warning: 600.0,
                threshold_critical: 950.0,
                status_level: "Optimal".to_string(),
                sparkline_history: [330.0, 335.0, 340.0, 338.0, 345.0, 341.0, 343.0, 342.5],
            },
            Self {
                sensor_id: "ACC-ROOF-01".to_string(),
                sensor_name: "Triaxial Accelerometer ACC-01".to_string(),
                location: "Level 15 Roof Core Top".to_string(),
                sensor_type: "MEMS Seismic Sensor".to_string(),
                current_value: 1.48,
                unit: "Hz".to_string(),
                threshold_warning: 1.20,
                threshold_critical: 0.95,
                status_level: "Optimal".to_string(),
                sparkline_history: [1.52, 1.51, 1.50, 1.49, 1.48, 1.49, 1.48, 1.48],
            },
            Self {
                sensor_id: "CM-BASE-04".to_string(),
                sensor_name: "Laser Displacement Crack Meter CM-04".to_string(),
                location: "Basement 2 South Retaining Wall".to_string(),
                sensor_type: "Potentiometric LVDT".to_string(),
                current_value: 0.12,
                unit: "mm".to_string(),
                threshold_warning: 0.30,
                threshold_critical: 0.50,
                status_level: "Optimal".to_string(),
                sparkline_history: [0.08, 0.09, 0.10, 0.10, 0.11, 0.11, 0.12, 0.12],
            },
            Self {
                sensor_id: "TM-CORE-02".to_string(),
                sensor_name: "Biaxial Inclinometer Tiltmeter TM-02".to_string(),
                location: "Level 8 Mid-Tower Shear Core".to_string(),
                sensor_type: "Electrolytic Tilt Sensor".to_string(),
                current_value: 0.018,
                unit: "deg".to_string(),
                threshold_warning: 0.050,
                threshold_critical: 0.100,
                status_level: "Optimal".to_string(),
                sparkline_history: [0.015, 0.016, 0.017, 0.017, 0.018, 0.018, 0.018, 0.018],
            },
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CobieAssetRecord {
    pub sheet_tab: String,
    pub name: String,
    pub created_by: String,
    pub category: String,
    pub space_or_location: String,
    pub serial_or_doc_no: String,
    pub warranty_or_date: String,
    pub condition_index: f32,
}

impl CobieAssetRecord {
    pub fn default_records() -> Vec<Self> {
        vec![
            Self {
                sheet_tab: "Facility".to_string(),
                name: "Menara Sudirman Commercial Tower".to_string(),
                created_by: "EigenGraph BIM Team".to_string(),
                category: "High-Rise Office Complex".to_string(),
                space_or_location: "Kav. 52-53 Sudirman, Jakarta".to_string(),
                serial_or_doc_no: "FAC-JKT-2026-001".to_string(),
                warranty_or_date: "10-Year Structural Defect Liability".to_string(),
                condition_index: 99.2,
            },
            Self {
                sheet_tab: "Floor".to_string(),
                name: "Level 01 Ground Lobby".to_string(),
                created_by: "Architectural Lead".to_string(),
                category: "Commercial Public Realm".to_string(),
                space_or_location: "Elevation +0.000m".to_string(),
                serial_or_doc_no: "FLR-L01".to_string(),
                warranty_or_date: "Commissioned 2026-11-12".to_string(),
                condition_index: 100.0,
            },
            Self {
                sheet_tab: "Component".to_string(),
                name: "W14x132 Transfer Girder TG-01".to_string(),
                created_by: "Structural Engineer of Record".to_string(),
                category: "ASTM A992 Heavy Section".to_string(),
                space_or_location: "Grid C/2-3 Level 2 Ceiling".to_string(),
                serial_or_doc_no: "HEAT-#88392-AISC".to_string(),
                warranty_or_date: "50-Year Design Life".to_string(),
                condition_index: 98.6,
            },
            Self {
                sheet_tab: "Type".to_string(),
                name: "Square RC Column 600x600 C1".to_string(),
                created_by: "BIM Modeler".to_string(),
                category: "C40/50 Self-Consolidating Concrete".to_string(),
                space_or_location: "Ground through L15 Columns".to_string(),
                serial_or_doc_no: "TYP-COL-600".to_string(),
                warranty_or_date: "Tested to SNI 2847:2019".to_string(),
                condition_index: 99.0,
            },
            Self {
                sheet_tab: "Document".to_string(),
                name: "As-Built Structural Certification Dossier".to_string(),
                created_by: "Chief Structural Engineer".to_string(),
                category: "Statutory Approval PBG".to_string(),
                space_or_location: "Electronic Vault / SIMBG".to_string(),
                serial_or_doc_no: "DOC-PBG-2026-881".to_string(),
                warranty_or_date: "Certified 2026-11-12".to_string(),
                condition_index: 100.0,
            },
        ]
    }
}

// ============================================================================
// Phase 29: Coordination, AI Studio, Parametric Presets & Lean/DfMA Models
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClashSeverity {
    Hard,
    Soft,
    CodeTolerance,
}

impl ClashSeverity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Hard => "Hard Clash",
            Self::Soft => "Soft Clearance",
            Self::CodeTolerance => "Code Tolerance",
        }
    }

    pub fn badge(&self) -> &'static str {
        match self {
            Self::Hard => "[HARD]",
            Self::Soft => "[SOFT]",
            Self::CodeTolerance => "[CODE]",
        }
    }

    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            Self::Hard => (239, 68, 68),
            Self::Soft => (245, 158, 11),
            Self::CodeTolerance => (249, 115, 22),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClashStatus {
    Active,
    InReview,
    Resolved,
}

impl ClashStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::InReview => "In Review",
            Self::Resolved => "Resolved",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Active => Self::InReview,
            Self::InReview => Self::Resolved,
            Self::Resolved => Self::Active,
        }
    }

    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            Self::Active => (239, 68, 68),
            Self::InReview => (59, 130, 246),
            Self::Resolved => (16, 185, 129),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClashDetectionItem {
    pub id: usize,
    pub tag: String,
    pub discipline_pair: String,
    pub severity: ClashSeverity,
    pub status: ClashStatus,
    pub element_a: String,
    pub element_b: String,
    /// Negative for physical intersection depth (mm), positive for gap (mm).
    pub intersection_mm: f64,
    pub required_clearance_mm: f64,
    pub point_3d: [f32; 3],
    pub box_half_extents: [f32; 3],
    pub note: String,
}

impl ClashDetectionItem {
    pub fn is_triggered_at_tolerance(&self, tol_mm: f64) -> bool {
        match self.severity {
            ClashSeverity::Hard => (-self.intersection_mm) >= tol_mm,
            ClashSeverity::Soft | ClashSeverity::CodeTolerance => {
                let shortfall = (self.required_clearance_mm - self.intersection_mm).max(0.0);
                shortfall >= tol_mm * 0.5
            }
        }
    }

    pub fn metric_summary(&self) -> String {
        match self.severity {
            ClashSeverity::Hard => format!("Penetration: {:.0} mm", -self.intersection_mm),
            ClashSeverity::Soft => format!(
                "Gap: {:.0} mm (< {:.0} mm req.)",
                self.intersection_mm, self.required_clearance_mm
            ),
            ClashSeverity::CodeTolerance => format!(
                "Clearance: {:.0} mm (< {:.0} mm code min.)",
                self.intersection_mm, self.required_clearance_mm
            ),
        }
    }

    pub fn clash_volume_m3(&self) -> f64 {
        let sx = (self.box_half_extents[0] * 2.0) as f64;
        let sy = (self.box_half_extents[1] * 2.0) as f64;
        let sz = (self.box_half_extents[2] * 2.0) as f64;
        (sx * sy * sz * 0.08).max(0.004)
    }

    pub fn default_clashes() -> Vec<Self> {
        vec![
            Self {
                id: 1,
                tag: "CLASH-001".to_string(),
                discipline_pair: "Structure vs HVAC Duct".to_string(),
                severity: ClashSeverity::Hard,
                status: ClashStatus::Active,
                element_a: "HVAC Supply Duct Ø350".to_string(),
                element_b: "Primary Girder W18x50 #4".to_string(),
                intersection_mm: -42.0,
                required_clearance_mm: 25.0,
                point_3d: [4.2, 3.6, 2.8],
                box_half_extents: [0.45, 0.35, 0.45],
                note: "Mechanical soffit drops 42 mm into top flange zone at Grid B-2.".to_string(),
            },
            Self {
                id: 2,
                tag: "CLASH-002".to_string(),
                discipline_pair: "Structure vs Piping".to_string(),
                severity: ClashSeverity::Hard,
                status: ClashStatus::Active,
                element_a: "Chilled Water Pipe DN100".to_string(),
                element_b: "Special Shear Wall SW-01".to_string(),
                intersection_mm: -110.0,
                required_clearance_mm: 50.0,
                point_3d: [7.8, 2.4, 5.2],
                box_half_extents: [0.40, 0.40, 0.40],
                note: "Missing cast-in structural steel sleeve in boundary zone.".to_string(),
            },
            Self {
                id: 3,
                tag: "CLASH-003".to_string(),
                discipline_pair: "Structure vs HVAC Duct".to_string(),
                severity: ClashSeverity::Soft,
                status: ClashStatus::InReview,
                element_a: "Electrical Cable Ladder Tray".to_string(),
                element_b: "Fire Sprinkler Main DN80".to_string(),
                intersection_mm: 85.0,
                required_clearance_mm: 150.0,
                point_3d: [2.6, 3.4, 6.4],
                box_half_extents: [0.50, 0.30, 0.40],
                note: "Violates NFPA 13 minimum 150 mm hanger maintenance clearance.".to_string(),
            },
            Self {
                id: 4,
                tag: "CLASH-004".to_string(),
                discipline_pair: "Framing vs Rebar Clearances".to_string(),
                severity: ClashSeverity::Hard,
                status: ClashStatus::Active,
                element_a: "Column C2 Confining Stirrup".to_string(),
                element_b: "Hold-Down Anchor Bolt M27".to_string(),
                intersection_mm: -18.0,
                required_clearance_mm: 20.0,
                point_3d: [9.0, 0.4, 1.5],
                box_half_extents: [0.35, 0.35, 0.35],
                note: "Anchor bolt washer intersects top stirrup leg at plinth pedestal.".to_string(),
            },
            Self {
                id: 5,
                tag: "CLASH-005".to_string(),
                discipline_pair: "Architecture vs Structure".to_string(),
                severity: ClashSeverity::CodeTolerance,
                status: ClashStatus::Active,
                element_a: "Stair Landing Slab Soffit".to_string(),
                element_b: "Emergency Conduits & Exit Signage".to_string(),
                intersection_mm: 2050.0,
                required_clearance_mm: 2100.0,
                point_3d: [6.0, 2.05, 8.2],
                box_half_extents: [0.55, 0.45, 0.45],
                note: "Headroom 2,050 mm is 50 mm below IBC egress minimum (2,100 mm).".to_string(),
            },
            Self {
                id: 6,
                tag: "CLASH-006".to_string(),
                discipline_pair: "Structure vs Piping".to_string(),
                severity: ClashSeverity::Soft,
                status: ClashStatus::Resolved,
                element_a: "Level 2 Gravity Riser Sleeve".to_string(),
                element_b: "Secondary Composite Beam B-12".to_string(),
                intersection_mm: 22.0,
                required_clearance_mm: 50.0,
                point_3d: [3.2, 6.8, 4.1],
                box_half_extents: [0.38, 0.30, 0.38],
                note: "Offset riser by 120 mm west of beam web centerline.".to_string(),
            },
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BcfComment {
    pub id: usize,
    pub author: String,
    pub timestamp: String,
    pub comment: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BcfPriority {
    Critical,
    High,
    Medium,
    Low,
}

impl BcfPriority {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Critical => "Critical",
            Self::High => "High",
            Self::Medium => "Medium",
            Self::Low => "Low",
        }
    }

    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            Self::Critical => (239, 68, 68),
            Self::High => (249, 115, 22),
            Self::Medium => (245, 158, 11),
            Self::Low => (16, 185, 129),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BcfTopicStatus {
    Open,
    InProgress,
    Resolved,
}

impl BcfTopicStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::InProgress => "In Progress",
            Self::Resolved => "Resolved",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Open => Self::InProgress,
            Self::InProgress => Self::Resolved,
            Self::Resolved => Self::Open,
        }
    }

    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            Self::Open => (239, 68, 68),
            Self::InProgress => (59, 130, 246),
            Self::Resolved => (16, 185, 129),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BcfTopicRecord {
    pub id: usize,
    pub guid: String,
    pub tag: String,
    pub title: String,
    pub description: String,
    pub priority: BcfPriority,
    pub status: BcfTopicStatus,
    pub assignee: String,
    pub due_date: String,
    pub linked_clash_id: Option<usize>,
    pub cam_yaw: f32,
    pub cam_pitch: f32,
    pub cam_zoom: f32,
    pub cam_target: [f32; 3],
    pub comments: Vec<BcfComment>,
}

impl BcfTopicRecord {
    pub fn default_topics() -> Vec<Self> {
        vec![
            Self {
                id: 1,
                guid: "8a9e4f12-21b0-4e8f-b901-bcf210000101".to_string(),
                tag: "BCF-101".to_string(),
                title: "Relocate HVAC Duct D-104 below W18x50 flange".to_string(),
                description: "Duct elevation violates girder bottom chord cover near Grid B-2."
                    .to_string(),
                priority: BcfPriority::Critical,
                status: BcfTopicStatus::InProgress,
                assignee: "Senior MEP Lead".to_string(),
                due_date: "2026-11-20".to_string(),
                linked_clash_id: Some(1),
                cam_yaw: 0.72,
                cam_pitch: 0.42,
                cam_zoom: 1.85,
                cam_target: [4.2, 3.6, 2.8],
                comments: vec![
                    BcfComment {
                        id: 1,
                        author: "BIM Coordinator".to_string(),
                        timestamp: "2026-11-11 09:15".to_string(),
                        comment: "Flagged 42 mm web interference during federation run.".to_string(),
                    },
                    BcfComment {
                        id: 2,
                        author: "Senior MEP Lead".to_string(),
                        timestamp: "2026-11-12 11:40".to_string(),
                        comment: "Transitioning duct profile from Ø350 to 450x220 flat oval."
                            .to_string(),
                    },
                ],
            },
            Self {
                id: 2,
                guid: "8a9e4f12-21b0-4e8f-b901-bcf210000102".to_string(),
                tag: "BCF-102".to_string(),
                title: "Add cast-in pipe sleeve at Shear Wall SW-01".to_string(),
                description: "Provide Sch 80 steel sleeve and boundary confinement hairpin bars."
                    .to_string(),
                priority: BcfPriority::High,
                status: BcfTopicStatus::Open,
                assignee: "Structural Lead".to_string(),
                due_date: "2026-11-18".to_string(),
                linked_clash_id: Some(2),
                cam_yaw: 1.15,
                cam_pitch: 0.34,
                cam_zoom: 2.05,
                cam_target: [7.8, 2.4, 5.2],
                comments: vec![BcfComment {
                    id: 1,
                    author: "Structural Lead".to_string(),
                    timestamp: "2026-11-12 08:20".to_string(),
                    comment: "Checking ACI 318-19 Ch. 18 opening reinforcement details."
                        .to_string(),
                }],
            },
            Self {
                id: 3,
                guid: "8a9e4f12-21b0-4e8f-b901-bcf210000103".to_string(),
                tag: "BCF-103".to_string(),
                title: "Stair landing headroom verification (min 2.10 m)".to_string(),
                description: "Adjust emergency conduit routing out of stair flight envelope."
                    .to_string(),
                priority: BcfPriority::Medium,
                status: BcfTopicStatus::Resolved,
                assignee: "Project Architect".to_string(),
                due_date: "2026-11-15".to_string(),
                linked_clash_id: Some(5),
                cam_yaw: 0.45,
                cam_pitch: 0.28,
                cam_zoom: 1.60,
                cam_target: [6.0, 2.05, 8.2],
                comments: vec![BcfComment {
                    id: 1,
                    author: "Project Architect".to_string(),
                    timestamp: "2026-11-12 14:10".to_string(),
                    comment: "Conduit recessed into dry-lined riser chase; headroom now 2.18 m."
                        .to_string(),
                }],
            },
        ]
    }

    pub fn export_bcf_xml(topics: &[Self], project_name: &str) -> String {
        let escape = |s: &str| -> String {
            s.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
        };

        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str(&format!(
            "<BcfArchive Version=\"2.1\" ProjectName=\"{}\">\n",
            escape(project_name)
        ));
        for t in topics {
            xml.push_str(&format!(
                "  <Markup>\n    <Topic Guid=\"{}\" TopicType=\"Clash\" TopicStatus=\"{}\">\n",
                escape(&t.guid),
                escape(t.status.label())
            ));
            xml.push_str(&format!("      <ReferenceLink>{}</ReferenceLink>\n", escape(&t.tag)));
            xml.push_str(&format!("      <Title>{}</Title>\n", escape(&t.title)));
            xml.push_str(&format!("      <Priority>{}</Priority>\n", escape(t.priority.label())));
            xml.push_str(&format!("      <AssignedTo>{}</AssignedTo>\n", escape(&t.assignee)));
            xml.push_str(&format!("      <DueDate>{}</DueDate>\n", escape(&t.due_date)));
            xml.push_str(&format!("      <Description>{}</Description>\n", escape(&t.description)));
            xml.push_str("    </Topic>\n");
            for c in &t.comments {
                xml.push_str(&format!(
                    "    <Comment Guid=\"{}-c{}\">\n      <Date>{}</Date>\n      <Author>{}</Author>\n      <Comment>{}</Comment>\n    </Comment>\n",
                    escape(&t.guid),
                    c.id,
                    escape(&c.timestamp),
                    escape(&c.author),
                    escape(&c.comment)
                ));
            }
            xml.push_str("    <VisualizationInfo>\n      <PerspectiveCamera>\n");
            xml.push_str(&format!(
                "        <CameraViewPoint><X>{:.3}</X><Y>{:.3}</Y><Z>{:.3}</Z></CameraViewPoint>\n",
                t.cam_target[0], t.cam_target[1], t.cam_target[2]
            ));
            xml.push_str(&format!(
                "        <EigenGraphOrbit yaw_rad=\"{:.4}\" pitch_rad=\"{:.4}\" zoom=\"{:.3}\" />\n",
                t.cam_yaw, t.cam_pitch, t.cam_zoom
            ));
            xml.push_str("      </PerspectiveCamera>\n    </VisualizationInfo>\n  </Markup>\n");
        }
        xml.push_str("</BcfArchive>\n");
        xml
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CopilotWorkflowKind {
    ShearWallAci318,
    WindLoadAsce7,
    CarbonMinimizerIso14040,
}

impl CopilotWorkflowKind {
    pub fn all() -> &'static [Self] {
        &[
            Self::ShearWallAci318,
            Self::WindLoadAsce7,
            Self::CarbonMinimizerIso14040,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::ShearWallAci318 => "ACI 318-19 / EC2 Shear Wall Reinforcement",
            Self::WindLoadAsce7 => "ASCE 7-22 Lateral Wind Pressure",
            Self::CarbonMinimizerIso14040 => "ISO 14040 / EN 15978 Carbon Minimizer",
        }
    }

    pub fn short_tag(&self) -> &'static str {
        match self {
            Self::ShearWallAci318 => "ACI 318 / EC2",
            Self::WindLoadAsce7 => "ASCE 7-22",
            Self::CarbonMinimizerIso14040 => "ISO 14040",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CopilotShearWallParams {
    pub wall_id: String,
    pub length_m: f64,
    pub thickness_mm: f64,
    pub height_m: f64,
    pub fc_mpa: f64,
    pub fy_mpa: f64,
    pub vu_kn: f64,
    pub pu_kn: f64,
    pub mu_knm: f64,
}

impl Default for CopilotShearWallParams {
    fn default() -> Self {
        Self {
            wall_id: "SW-CORE-01".to_string(),
            length_m: 4.8,
            thickness_mm: 350.0,
            height_m: 12.0,
            fc_mpa: 35.0,
            fy_mpa: 420.0,
            vu_kn: 3250.0,
            pu_kn: 4800.0,
            mu_knm: 14500.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CopilotWindParams {
    pub project_ref: String,
    pub basic_wind_speed_ms: f64,
    pub building_height_m: f64,
    pub exposure: String,
    pub topographic_kzt: f64,
    pub gust_factor_g: f64,
    pub cp_windward: f64,
    pub cp_leeward: f64,
}

impl Default for CopilotWindParams {
    fn default() -> Self {
        Self {
            project_ref: "Menara Sudirman Tower".to_string(),
            basic_wind_speed_ms: 42.0,
            building_height_m: 64.0,
            exposure: "C".to_string(),
            topographic_kzt: 1.0,
            gust_factor_g: 0.85,
            cp_windward: 0.80,
            cp_leeward: -0.50,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CopilotCarbonParams {
    pub concrete_vol_m3: f64,
    pub steel_mass_tons: f64,
    pub scm_replacement_pct: f64,
    pub eaf_recycled_steel_pct: f64,
    pub timber_hybrid_offset_pct: f64,
}

impl Default for CopilotCarbonParams {
    fn default() -> Self {
        Self {
            concrete_vol_m3: 5500.0,
            steel_mass_tons: 940.0,
            scm_replacement_pct: 35.0,
            eaf_recycled_steel_pct: 80.0,
            timber_hybrid_offset_pct: 12.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalculationMemoResult {
    pub title: String,
    pub standard_ref: String,
    pub summary_headline: String,
    pub dcr_ratio: f64,
    pub is_compliant: bool,
    pub equations: Vec<String>,
    pub substitutions: Vec<(String, String, String)>,
    pub recommendation: String,
    pub markdown_memo: String,
}

/// Deterministic ACI 318-19 Chapter 18 / Eurocode 2 Special Structural Shear Wall Designer
pub fn solve_aci318_shear_wall(p: &CopilotShearWallParams) -> CalculationMemoResult {
    let lw_mm = (p.length_m * 1000.0).max(500.0);
    let tw_mm = p.thickness_mm.max(150.0);
    let hw_mm = (p.height_m * 1000.0).max(1000.0);
    let fc = p.fc_mpa.max(20.0);
    let fy = p.fy_mpa.max(280.0);

    let acv_mm2 = lw_mm * tw_mm;
    let hw_lw = hw_mm / lw_mm;
    let alpha_c = if hw_lw <= 1.5 {
        0.25
    } else if hw_lw >= 2.0 {
        0.17
    } else {
        0.25 - (hw_lw - 1.5) * (0.08 / 0.5)
    };

    let vc_n = alpha_c * fc.sqrt() * acv_mm2;
    let vc_kn = vc_n / 1000.0;
    let phi_v = 0.75;
    let vu_req_kn = p.vu_kn / phi_v;
    let rho_t_calc = ((vu_req_kn * 1000.0 / acv_mm2) - alpha_c * fc.sqrt()) / fy;
    let rho_t_prov = rho_t_calc.max(0.0025);
    let vn_kn = (acv_mm2 * (alpha_c * fc.sqrt() + rho_t_prov * fy)) / 1000.0;
    let vn_max_kn = (0.83 * fc.sqrt() * acv_mm2) / 1000.0;
    let phi_vn_kn = phi_v * vn_kn.min(vn_max_kn);
    let dcr_shear = p.vu_kn / phi_vn_kn.max(1.0);

    // Boundary element extreme fiber compressive stress check (ACI 318-19 18.10.6.3)
    let ag_m2 = (lw_mm / 1000.0) * (tw_mm / 1000.0);
    let ig_m4 = (tw_mm / 1000.0) * p.length_m.powi(3) / 12.0;
    let sigma_max_mpa = ((p.pu_kn / ag_m2) + (p.mu_knm * (p.length_m * 0.5) / ig_m4)) / 1000.0;
    let special_be_required = sigma_max_mpa >= 0.2 * fc;

    // Bar selection across 2 curtains
    let as_per_m_req = rho_t_prov * tw_mm * 1000.0; // mm2 / m total
    let (bar_label, spacing_mm) = if as_per_m_req <= 1300.0 {
        ("2x D13 (2 Curtains)", 200)
    } else if as_per_m_req <= 2010.0 {
        ("2x D16 (2 Curtains)", 200)
    } else if as_per_m_req <= 2680.0 {
        ("2x D16 (2 Curtains)", 150)
    } else {
        ("2x D19 (2 Curtains)", 150)
    };

    let shear_cap_ok = vn_kn <= vn_max_kn && dcr_shear <= 1.0;
    let rec = if special_be_required {
        format!(
            "Provide {} @ {} mm horiz/vert + Special Boundary Elements (SBE length {:.2} m, D13 hoops @ 100 mm).",
            bar_label,
            spacing_mm,
            (p.length_m * 0.15).max(0.45)
        )
    } else {
        format!(
            "Provide {} @ {} mm horiz/vert with Ordinary Boundary transverse ties @ 150 mm.",
            bar_label, spacing_mm
        )
    };

    let md = format!(
        "# Engineering Calculation Memo: Structural Shear Wall ({})\n\
        **Design Code:** ACI 318-19 Chapter 18 / Eurocode 2\n\n\
        ## 1. Geometry & Applied Demands\n\
        - Wall Length $l_w = {:.2}\\text{{ m}}$, Thickness $t_w = {:.0}\\text{{ mm}}$, Height $h_w = {:.2}\\text{{ m}}$\n\
        - Concrete $f'_c = {:.1}\\text{{ MPa}}$, Steel $f_y = {:.1}\\text{{ MPa}}$\n\
        - Factored Demands: $V_u = {:.1}\\text{{ kN}}$, $P_u = {:.1}\\text{{ kN}}$, $M_u = {:.1}\\text{{ kNm}}$\n\n\
        ## 2. In-Plane Shear Strength & Reinforcement Ratio\n\
        - Aspect ratio $h_w / l_w = {:.2} \\Rightarrow \\alpha_c = {:.3}$\n\
        - Concrete shear contribution $V_c = {:.1}\\text{{ kN}}$\n\
        - Required/Provided web ratio $\\rho_t = {:.4}$ (min 0.0025)\n\
        - Design Shear Capacity $\\phi V_n = {:.1}\\text{{ kN}}$ vs $V_{{n,\\max}} = {:.1}\\text{{ kN}}$ (DCR = **{:.3}**)\n\n\
        ## 3. Extreme Fiber Compression & Boundary Element Trigger\n\
        - Extreme fiber stress $\\sigma_{{max}} = {:.2}\\text{{ MPa}}$ vs trigger $0.2 f'_c = {:.2}\\text{{ MPa}}$ -> **{}**\n\n\
        ## 4. Reinforcement Recommendation\n\
        - {}\n",
        p.wall_id,
        p.length_m,
        p.thickness_mm,
        p.height_m,
        fc,
        fy,
        p.vu_kn,
        p.pu_kn,
        p.mu_knm,
        hw_lw,
        alpha_c,
        vc_kn,
        rho_t_prov,
        phi_vn_kn,
        vn_max_kn,
        dcr_shear,
        sigma_max_mpa,
        0.2 * fc,
        if special_be_required { "SPECIAL BOUNDARY ELEMENT REQUIRED" } else { "ORDINARY BOUNDARY" },
        rec
    );

    CalculationMemoResult {
        title: format!("Shear Wall Reinforcement Design — {}", p.wall_id),
        standard_ref: "ACI 318-19 §18.10 / EN 1992-1-1".to_string(),
        summary_headline: format!(
            "{} @ {} mm • φVn = {:.0} kN • DCR = {:.2}",
            bar_label, spacing_mm, phi_vn_kn, dcr_shear
        ),
        dcr_ratio: dcr_shear,
        is_compliant: shear_cap_ok,
        equations: vec![
            "Vn = Acv * (alpha_c * sqrt(fc') + rho_t * fy) <= 0.83 * sqrt(fc') * Acv".to_string(),
            "sigma_max = Pu / Ag + Mu * (lw / 2) / Ig  vs  0.20 * fc'".to_string(),
        ],
        substitutions: vec![
            (
                "Gross Web Shear Area (Acv)".to_string(),
                format!("{:.0} mm * {:.0} mm", lw_mm, tw_mm),
                format!("{:.2} m²", acv_mm2 / 1e6),
            ),
            (
                "Aspect Coefficient (αc)".to_string(),
                format!("hw/lw = {:.2}", hw_lw),
                format!("{:.3} (Vc = {:.1} kN)", alpha_c, vc_kn),
            ),
            (
                "Web Reinforcement Ratio (ρt)".to_string(),
                format!("max(0.0025, req {:.4})", rho_t_calc.max(0.0)),
                format!("{:.4} ({:.0} mm²/m)", rho_t_prov, as_per_m_req),
            ),
            (
                "Shear Utilization (Vu / φVn)".to_string(),
                format!("{:.1} / {:.1} kN", p.vu_kn, phi_vn_kn),
                format!(
                    "{:.3} ({})",
                    dcr_shear,
                    if shear_cap_ok { "PASS" } else { "EXCEEDS LIMIT" }
                ),
            ),
            (
                "Boundary Stress (σmax vs 0.2fc')".to_string(),
                format!("{:.2} MPa vs {:.2} MPa", sigma_max_mpa, 0.2 * fc),
                if special_be_required {
                    "Special BE Triggered".to_string()
                } else {
                    "Ordinary Detailing OK".to_string()
                },
            ),
        ],
        recommendation: rec,
        markdown_memo: md,
    }
}

/// Deterministic ASCE 7-22 Chapter 27 MWFRS Wind Pressure Solver
pub fn solve_asce7_wind(p: &CopilotWindParams) -> CalculationMemoResult {
    let z_m = p.building_height_m.max(4.6);
    let (zg_m, alpha) = match p.exposure.as_str() {
        "B" => (365.76, 7.0),
        "D" => (213.36, 11.5),
        _ => (274.32, 9.5), // Exposure C
    };
    let kz = 2.01 * (z_m / zg_m).min(1.0).powf(2.0 / alpha);
    let kd = 0.85;
    let ke = 1.00;
    let v_ms = p.basic_wind_speed_ms.max(20.0);
    let qz_kpa = 0.613 * kz * p.topographic_kzt * kd * ke * v_ms * v_ms / 1000.0;
    let p_windward_kpa = qz_kpa * p.gust_factor_g * p.cp_windward;
    let p_leeward_kpa = qz_kpa * p.gust_factor_g * p.cp_leeward;
    let p_net_kpa = p_windward_kpa + p_leeward_kpa.abs();
    let dcr_curtain = p_net_kpa / 3.20; // Typical 3.20 kPa ultimate curtain wall capacity benchmark
    let is_ok = dcr_curtain <= 1.0;

    let rec = format!(
        "Net roof-level MWFRS wind pressure {:.2} kPa (Windward +{:.2} kPa, Leeward {:.2} kPa). Specify facade mullions for >= {:.2} kPa.",
        p_net_kpa,
        p_windward_kpa,
        p_leeward_kpa,
        p_net_kpa * 1.1
    );

    let md = format!(
        "# Engineering Calculation Memo: Lateral Wind Load Analysis ({})\n\
        **Design Code:** ASCE 7-22 Chapter 27 (Directional Procedure)\n\n\
        ## 1. Site Wind Parameters\n\
        - Basic Wind Speed $V = {:.1}\\text{{ m/s}}$ | Exposure Category **{}** | Height $z = {:.1}\\text{{ m}}$\n\
        - Factors: $K_d = {:.2}$, $K_{{zt}} = {:.2}$, $K_e = {:.2}$, $G = {:.2}$\n\n\
        ## 2. Velocity Pressure & Design Net Pressure\n\
        - Exposure Coefficient $K_z = 2.01 (z / z_g)^{{2/\\alpha}} = {:.3}$\n\
        - Velocity Pressure $q_z = 0.613 K_z K_{{zt}} K_d K_e V^2 = {:.3}\\text{{ kPa}}$\n\
        - Windward Pressure $p_w = +{:.3}\\text{{ kPa}}$ | Leeward Suction $p_l = {:.3}\\text{{ kPa}}$\n\
        - Net Lateral Design Pressure $p_{{net}} = {:.3}\\text{{ kPa}}$\n",
        p.project_ref, v_ms, p.exposure, z_m, kd, p.topographic_kzt, ke, p.gust_factor_g, kz, qz_kpa, p_windward_kpa, p_leeward_kpa, p_net_kpa
    );

    CalculationMemoResult {
        title: format!("ASCE 7-22 Wind Pressure Sheet — {}", p.project_ref),
        standard_ref: "ASCE/SEI 7-22 Ch. 26 & 27".to_string(),
        summary_headline: format!(
            "qz = {:.2} kPa • p_net = {:.2} kPa (Kz = {:.3})",
            qz_kpa, p_net_kpa, kz
        ),
        dcr_ratio: dcr_curtain,
        is_compliant: is_ok,
        equations: vec![
            "qz = 0.613 * Kz * Kzt * Kd * Ke * V^2 (N/m2)".to_string(),
            "p_net = qz * G * Cp_windward - qh * G * Cp_leeward".to_string(),
        ],
        substitutions: vec![
            (
                "Velocity Pressure Coeff (Kz)".to_string(),
                format!("Exp {} @ z={:.1}m (zg={:.1}m, α={:.1})", p.exposure, z_m, zg_m, alpha),
                format!("{:.3}", kz),
            ),
            (
                "Velocity Pressure (qz)".to_string(),
                format!("0.613*{:.3}*{:.2}*0.85*{:.1}²", kz, p.topographic_kzt, v_ms),
                format!("{:.3} kPa", qz_kpa),
            ),
            (
                "Windward Face Pressure (pw)".to_string(),
                format!("{:.3} * {:.2} * {:.2}", qz_kpa, p.gust_factor_g, p.cp_windward),
                format!("+{:.3} kPa", p_windward_kpa),
            ),
            (
                "Leeward Suction (pl)".to_string(),
                format!("{:.3} * {:.2} * ({:.2})", qz_kpa, p.gust_factor_g, p.cp_leeward),
                format!("{:.3} kPa", p_leeward_kpa),
            ),
            (
                "Net Combined Lateral (p_net)".to_string(),
                "pw + |pl| vs 3.20 kPa facade benchmark".to_string(),
                format!("{:.3} kPa (DCR {:.2})", p_net_kpa, dcr_curtain),
            ),
        ],
        recommendation: rec,
        markdown_memo: md,
    }
}

/// Deterministic ISO 14040 / EN 15978 Structural Embodied Carbon Minimizer
pub fn solve_iso14040_carbon(p: &CopilotCarbonParams) -> CalculationMemoResult {
    // Baseline: Ordinary Portland Cement CEM I (340 kgCO2e/m3) + BOF Steel (2350 kgCO2e/ton)
    let base_conc_t = p.concrete_vol_m3 * 0.340;
    let base_steel_t = p.steel_mass_tons * 2.350;
    let baseline_total_t = (base_conc_t + base_steel_t).max(1.0);

    // Optimized via SCM replacement (slag/fly-ash reduces clinker carbon up to ~62% of replacement share),
    // EAF recycled steel fraction, and Mass Timber hybrid substitution credit.
    let conc_factor = 1.0 - 0.62 * (p.scm_replacement_pct.clamp(0.0, 65.0) / 100.0);
    let steel_ef_t_per_t = 2.350 - 1.750 * (p.eaf_recycled_steel_pct.clamp(0.0, 100.0) / 100.0);
    let opt_conc_t = base_conc_t * conc_factor * (1.0 - 0.45 * p.timber_hybrid_offset_pct / 100.0);
    let opt_steel_t = p.steel_mass_tons * steel_ef_t_per_t;
    let opt_total_t = opt_conc_t + opt_steel_t;

    let reduction_pct = ((baseline_total_t - opt_total_t) / baseline_total_t) * 100.0;
    let target_ratio = opt_total_t / (baseline_total_t * 0.65); // Benchmark against 35% reduction target (RIBA 2030 path)
    let is_ok = reduction_pct >= 30.0;

    let rec = format!(
        "Achieves {:.1}% embodied carbon reduction ({:.0} tCO2e avoided) using {:.0}% SCM concrete & {:.0}% EAF scrap steel.",
        reduction_pct,
        baseline_total_t - opt_total_t,
        p.scm_replacement_pct,
        p.eaf_recycled_steel_pct
    );

    let md = format!(
        "# Engineering Calculation Memo: Whole-Building A1-A3 Carbon Optimization\n\
        **Standard:** ISO 14040 / EN 15978 Modules A1–A3\n\n\
        - Concrete Volume: {:.0} m³ | Structural Steel: {:.0} tons\n\
        - Baseline Emissions: **{:.1} tCO2e** (Concrete {:.1} t + BOF Steel {:.1} t)\n\
        - Optimized Emissions: **{:.1} tCO2e** (Net Avoidance **{:.1}%**)\n\
        - Mix Spec: {:.0}% SCM (GGBS/Fly Ash), {:.0}% EAF Recycled Steel, {:.0}% Hybrid Timber Deck\n",
        p.concrete_vol_m3,
        p.steel_mass_tons,
        baseline_total_t,
        base_conc_t,
        base_steel_t,
        opt_total_t,
        reduction_pct,
        p.scm_replacement_pct,
        p.eaf_recycled_steel_pct,
        p.timber_hybrid_offset_pct
    );

    CalculationMemoResult {
        title: "Embodied Carbon Minimization Audit (A1–A3)".to_string(),
        standard_ref: "ISO 14040 / EN 15978 / RIBA 2030".to_string(),
        summary_headline: format!(
            "{:.1} tCO2e (-{:.1}% vs CEM I / BOF Baseline)",
            opt_total_t, reduction_pct
        ),
        dcr_ratio: target_ratio,
        is_compliant: is_ok,
        equations: vec![
            "GWP_A1_A3 = Sum(Qty_i * EF_i * (1 - Decarbonization_i))".to_string(),
            "Reduction (%) = (GWP_base - GWP_opt) / GWP_base * 100".to_string(),
        ],
        substitutions: vec![
            (
                "Baseline A1-A3 Total".to_string(),
                format!("{:.0} m³*0.34 + {:.0} t*2.35", p.concrete_vol_m3, p.steel_mass_tons),
                format!("{:.1} tCO2e", baseline_total_t),
            ),
            (
                "Low-Carbon Concrete (SCM)".to_string(),
                format!("{:.0}% GGBS/Fly-Ash blend", p.scm_replacement_pct),
                format!("{:.1} tCO2e", opt_conc_t),
            ),
            (
                "Electric Arc Furnace Steel".to_string(),
                format!("{:.0}% recycled scrap ({:.2} tCO2/t)", p.eaf_recycled_steel_pct, steel_ef_t_per_t),
                format!("{:.1} tCO2e", opt_steel_t),
            ),
            (
                "Net Carbon Savings".to_string(),
                format!("{:.1} - {:.1} tCO2e avoided", baseline_total_t, opt_total_t),
                format!("-{:.1}% ({:.1} tCO2e saved)", reduction_pct, baseline_total_t - opt_total_t),
            ),
        ],
        recommendation: rec,
        markdown_memo: md,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParametricGeneratorPreset {
    DiagridTower,
    GeodesicDome,
    VoronoiCanopy,
}

impl ParametricGeneratorPreset {
    pub fn all() -> &'static [Self] {
        &[Self::DiagridTower, Self::GeodesicDome, Self::VoronoiCanopy]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::DiagridTower => "Twisting Diagrid Tower",
            Self::GeodesicDome => "Geodesic Space Dome",
            Self::VoronoiCanopy => "Freeform Canopy Shell",
        }
    }

    pub fn param_labels(&self) -> [&'static str; 4] {
        match self {
            Self::DiagridTower => [
                "Tower Height (m)",
                "Base Radius (m)",
                "Helical Twist (°)",
                "Radial Sides",
            ],
            Self::GeodesicDome => [
                "Dome Span Dia. (m)",
                "Apex Rise (m)",
                "Frequency Rings",
                "Meridian Sectors",
            ],
            Self::VoronoiCanopy => [
                "Canopy Span L (m)",
                "Arch Depth B (m)",
                "Edge Lift Camber (m)",
                "Grid Divisions",
            ],
        }
    }

    pub fn default_params(&self) -> [f32; 4] {
        match self {
            Self::DiagridTower => [48.0, 11.0, 45.0, 10.0],
            Self::GeodesicDome => [28.0, 11.0, 4.0, 14.0],
            Self::VoronoiCanopy => [26.0, 18.0, 3.8, 8.0],
        }
    }
}

impl VisualNodeGraph {
    pub fn from_preset(
        preset: ParametricGeneratorPreset,
        p1: f32,
        p2: f32,
        p3: f32,
        p4: f32,
    ) -> Self {
        let labels = preset.param_labels();
        let divisions = (p4.round() as usize).max(3);
        let elem_count = match preset {
            ParametricGeneratorPreset::DiagridTower => divisions * 3 * 3 + divisions * 3,
            ParametricGeneratorPreset::GeodesicDome => {
                let rings = (p3.round() as usize).max(2);
                rings * divisions * 3
            }
            ParametricGeneratorPreset::VoronoiCanopy => divisions * divisions * 3,
        };
        let total_steel_ton = match preset {
            ParametricGeneratorPreset::DiagridTower => (p1 * p2 * 0.42) + (p3 * 0.15),
            ParametricGeneratorPreset::GeodesicDome => (p1 * p2 * 0.18) + (divisions as f32 * 0.9),
            ParametricGeneratorPreset::VoronoiCanopy => (p1 * p2 * 0.09) + (p3 * 2.4),
        };

        Self {
            nodes: vec![
                VisualGraphNode {
                    id: 1,
                    title: format!("Input: {}", labels[0]),
                    category: VisualNodeCategory::Input,
                    pos: [30.0, 45.0],
                    inputs: vec![],
                    outputs: vec!["Val A".to_string()],
                    value_display: format!("{:.1}", p1),
                    is_active: true,
                },
                VisualGraphNode {
                    id: 2,
                    title: format!("Input: {}", labels[1]),
                    category: VisualNodeCategory::Input,
                    pos: [30.0, 175.0],
                    inputs: vec![],
                    outputs: vec!["Val B".to_string()],
                    value_display: format!("{:.1}", p2),
                    is_active: true,
                },
                VisualGraphNode {
                    id: 3,
                    title: format!("Transform: {}", labels[2]),
                    category: VisualNodeCategory::MathLogic,
                    pos: [275.0, 45.0],
                    inputs: vec!["A".to_string()],
                    outputs: vec!["Param K".to_string()],
                    value_display: format!("{:.1} ({} seg)", p3, divisions),
                    is_active: true,
                },
                VisualGraphNode {
                    id: 4,
                    title: format!("Gen: {}", preset.label()),
                    category: VisualNodeCategory::GeometryGen,
                    pos: [520.0, 95.0],
                    inputs: vec!["K".to_string(), "B".to_string()],
                    outputs: vec!["3D Mesh".to_string(), "Mass".to_string()],
                    value_display: format!("{} Struts ({:.1}t)", elem_count, total_steel_ton),
                    is_active: true,
                },
                VisualGraphNode {
                    id: 5,
                    title: "Bake to BIM 3D Scene".to_string(),
                    category: VisualNodeCategory::BimBake,
                    pos: [770.0, 105.0],
                    inputs: vec!["Geometry".to_string()],
                    outputs: vec!["IfcExport".to_string()],
                    value_display: "Live Preview Sync".to_string(),
                    is_active: true,
                },
            ],
            wires: vec![
                VisualGraphWire { from_node: 1, from_socket: 0, to_node: 3, to_socket: 0 },
                VisualGraphWire { from_node: 3, from_socket: 0, to_node: 4, to_socket: 0 },
                VisualGraphWire { from_node: 2, from_socket: 0, to_node: 4, to_socket: 1 },
                VisualGraphWire { from_node: 4, from_socket: 0, to_node: 5, to_socket: 0 },
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LpsWeeklyPpcRecord {
    pub week_label: String,
    pub planned_commitments: u32,
    pub completed_on_time: u32,
    pub ppc_pct: f32,
    pub primary_variance_reason: String,
}

impl LpsWeeklyPpcRecord {
    pub fn default_history() -> Vec<Self> {
        vec![
            Self {
                week_label: "W-01".to_string(),
                planned_commitments: 12,
                completed_on_time: 10,
                ppc_pct: 83.3,
                primary_variance_reason: "Weather Rain Delay".to_string(),
            },
            Self {
                week_label: "W-02".to_string(),
                planned_commitments: 14,
                completed_on_time: 11,
                ppc_pct: 78.6,
                primary_variance_reason: "Rebar Delivery Late".to_string(),
            },
            Self {
                week_label: "W-03".to_string(),
                planned_commitments: 15,
                completed_on_time: 13,
                ppc_pct: 86.7,
                primary_variance_reason: "RFI Design Change".to_string(),
            },
            Self {
                week_label: "W-04".to_string(),
                planned_commitments: 16,
                completed_on_time: 14,
                ppc_pct: 87.5,
                primary_variance_reason: "Crane Hook Conflict".to_string(),
            },
            Self {
                week_label: "W-05".to_string(),
                planned_commitments: 15,
                completed_on_time: 14,
                ppc_pct: 93.3,
                primary_variance_reason: "Minor Grout Curing".to_string(),
            },
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LpsConstraintCategory {
    ConstraintFree,
    EngineeringRfi,
    MaterialSupply,
    CraneLogistics,
    CrewAvailability,
}

impl LpsConstraintCategory {
    pub fn all() -> &'static [Self] {
        &[
            Self::ConstraintFree,
            Self::EngineeringRfi,
            Self::MaterialSupply,
            Self::CraneLogistics,
            Self::CrewAvailability,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::ConstraintFree => "Sound (Constraint-Free)",
            Self::EngineeringRfi => "Engineering / RFI Hold",
            Self::MaterialSupply => "Material Delivery Pending",
            Self::CraneLogistics => "Crane Hook / Rigging Access",
            Self::CrewAvailability => "Crew Calibration / Certification",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LpsLookaheadTask {
    pub id: usize,
    pub wbs_code: String,
    pub week_bucket: String,
    pub activity_name: String,
    pub trade_crew: String,
    pub constraint_category: LpsConstraintCategory,
    pub constraint_note: String,
    pub is_sound: bool,
    pub committed_in_wwp: bool,
    pub completed_on_time: bool,
    pub variance_reason: String,
}

impl LpsLookaheadTask {
    pub fn default_tasks() -> Vec<Self> {
        vec![
            Self {
                id: 1,
                wbs_code: "05.12.01".to_string(),
                week_bucket: "W-01".to_string(),
                activity_name: "Warren Truss Modules 1–2 Pre-Assembly".to_string(),
                trade_crew: "Structural Steel Crew A".to_string(),
                constraint_category: LpsConstraintCategory::ConstraintFree,
                constraint_note: "Shop drawings IFC stamped & jigs cleared".to_string(),
                is_sound: true,
                committed_in_wwp: true,
                completed_on_time: true,
                variance_reason: "None".to_string(),
            },
            Self {
                id: 2,
                wbs_code: "03.31.02".to_string(),
                week_bucket: "W-02".to_string(),
                activity_name: "Abutment Pile Cap & Embed Plates Cast".to_string(),
                trade_crew: "Concrete Team C1".to_string(),
                constraint_category: LpsConstraintCategory::ConstraintFree,
                constraint_note: "Survey coordinates verified by total station".to_string(),
                is_sound: true,
                committed_in_wwp: true,
                completed_on_time: true,
                variance_reason: "None".to_string(),
            },
            Self {
                id: 3,
                wbs_code: "05.12.04".to_string(),
                week_bucket: "W-03".to_string(),
                activity_name: "50T Crawler Hoisting of 12 m Span Segments".to_string(),
                trade_crew: "Rigging & Crane Team".to_string(),
                constraint_category: LpsConstraintCategory::ConstraintFree,
                constraint_note: "Wind window < 12 m/s & SVD stable".to_string(),
                is_sound: true,
                committed_in_wwp: true,
                completed_on_time: true,
                variance_reason: "None".to_string(),
            },
            Self {
                id: 4,
                wbs_code: "05.05.23".to_string(),
                week_bucket: "W-04".to_string(),
                activity_name: "ASTM F3125 A325 HTB Torque Calibration".to_string(),
                trade_crew: "Bolting Specialists B2".to_string(),
                constraint_category: LpsConstraintCategory::CrewAvailability,
                constraint_note: "Skidmore-Wilhelm tension calibrator check".to_string(),
                is_sound: false,
                committed_in_wwp: true,
                completed_on_time: false,
                variance_reason: "Tool Calibration Hold".to_string(),
            },
            Self {
                id: 5,
                wbs_code: "09.97.13".to_string(),
                week_bucket: "W-04".to_string(),
                activity_name: "C5-High Intumescent Coating Touch-Up".to_string(),
                trade_crew: "Protective Coating Crew".to_string(),
                constraint_category: LpsConstraintCategory::ConstraintFree,
                constraint_note: "Dew point & DFT gauge check approved".to_string(),
                is_sound: true,
                committed_in_wwp: true,
                completed_on_time: true,
                variance_reason: "None".to_string(),
            },
            Self {
                id: 6,
                wbs_code: "08.44.13".to_string(),
                week_bucket: "W-05".to_string(),
                activity_name: "Unitized Curtain Wall Pod Installation L4–L6".to_string(),
                trade_crew: "Facade Glazing Unit".to_string(),
                constraint_category: LpsConstraintCategory::MaterialSupply,
                constraint_note: "Awaiting Low-E unit batch #14 port release".to_string(),
                is_sound: false,
                committed_in_wwp: true,
                completed_on_time: false,
                variance_reason: "Material Delivery Late".to_string(),
            },
            Self {
                id: 7,
                wbs_code: "23.31.13".to_string(),
                week_bucket: "W-05".to_string(),
                activity_name: "Level 3 Overhead Chilled Water & Duct Riser".to_string(),
                trade_crew: "MEP Coordination Crew".to_string(),
                constraint_category: LpsConstraintCategory::EngineeringRfi,
                constraint_note: "Pending BCF-101 flat oval duct approval".to_string(),
                is_sound: false,
                committed_in_wwp: false,
                completed_on_time: false,
                variance_reason: "RFI / Clash Resolution".to_string(),
            },
            Self {
                id: 8,
                wbs_code: "03.41.13".to_string(),
                week_bucket: "W-06".to_string(),
                activity_name: "Precast Stair Core Module Flight 6 Placement".to_string(),
                trade_crew: "Precast Erection Crew".to_string(),
                constraint_category: LpsConstraintCategory::CraneLogistics,
                constraint_note: "Requires TC-1 hook clear window 13:00–16:00".to_string(),
                is_sound: true,
                committed_in_wwp: true,
                completed_on_time: true,
                variance_reason: "None".to_string(),
            },
        ]
    }

    pub fn compute_live_ppc(tasks: &[Self]) -> (u32, u32, f32) {
        let committed: u32 = tasks.iter().filter(|t| t.committed_in_wwp).count() as u32;
        if committed == 0 {
            return (0, 0, 0.0);
        }
        let completed: u32 = tasks
            .iter()
            .filter(|t| t.committed_in_wwp && t.completed_on_time)
            .count() as u32;
        let pct = (completed as f32 / committed as f32) * 100.0;
        (committed, completed, pct)
    }

    pub fn compute_variance_pareto(tasks: &[Self]) -> Vec<(String, u32)> {
        let mut counts: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
        for t in tasks {
            if t.committed_in_wwp && !t.completed_on_time {
                let reason = if t.variance_reason.trim().is_empty() || t.variance_reason == "None" {
                    t.constraint_category.label().to_string()
                } else {
                    t.variance_reason.clone()
                };
                *counts.entry(reason).or_insert(0) += 1;
            }
        }
        let mut list: Vec<(String, u32)> = counts.into_iter().collect();
        list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrefabSupplyStage {
    ShopDrawingApproved,
    FactoryCastingWeld,
    QaNdtInspection,
    InTransitFleet,
    SiteLaydownStaging,
    ErectedAndTorqued,
}

impl PrefabSupplyStage {
    pub fn all() -> &'static [Self] {
        &[
            Self::ShopDrawingApproved,
            Self::FactoryCastingWeld,
            Self::QaNdtInspection,
            Self::InTransitFleet,
            Self::SiteLaydownStaging,
            Self::ErectedAndTorqued,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::ShopDrawingApproved => "1. Shop Drawing",
            Self::FactoryCastingWeld => "2. Factory Fab/Cast",
            Self::QaNdtInspection => "3. QA / NDT Check",
            Self::InTransitFleet => "4. In-Transit Fleet",
            Self::SiteLaydownStaging => "5. Site Laydown",
            Self::ErectedAndTorqued => "6. Erected & Torqued",
        }
    }

    pub fn step_index(&self) -> usize {
        match self {
            Self::ShopDrawingApproved => 0,
            Self::FactoryCastingWeld => 1,
            Self::QaNdtInspection => 2,
            Self::InTransitFleet => 3,
            Self::SiteLaydownStaging => 4,
            Self::ErectedAndTorqued => 5,
        }
    }

    pub fn next(&self) -> Option<Self> {
        match self {
            Self::ShopDrawingApproved => Some(Self::FactoryCastingWeld),
            Self::FactoryCastingWeld => Some(Self::QaNdtInspection),
            Self::QaNdtInspection => Some(Self::InTransitFleet),
            Self::InTransitFleet => Some(Self::SiteLaydownStaging),
            Self::SiteLaydownStaging => Some(Self::ErectedAndTorqued),
            Self::ErectedAndTorqued => None,
        }
    }

    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            Self::ShopDrawingApproved => (148, 163, 184),
            Self::FactoryCastingWeld => (59, 130, 246),
            Self::QaNdtInspection => (168, 85, 247),
            Self::InTransitFleet => (245, 158, 11),
            Self::SiteLaydownStaging => (14, 165, 233),
            Self::ErectedAndTorqued => (16, 185, 129),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrefabModuleType {
    PrecastCorePanel,
    VolumetricSteelPod,
    CompositeTrussSegment,
    UnitizedCurtainPanel,
}

impl PrefabModuleType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::PrecastCorePanel => "Precast Core Panel",
            Self::VolumetricSteelPod => "Volumetric Steel Pod",
            Self::CompositeTrussSegment => "Composite Truss Segment",
            Self::UnitizedCurtainPanel => "Unitized Curtain Wall",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrefabModuleUnit {
    pub tag_id: String,
    pub rfid_code: String,
    pub name: String,
    pub module_type: PrefabModuleType,
    pub dimensions_m: [f32; 3],
    pub mass_tons: f32,
    pub crane_swl_tons: f32,
    pub target_zone: String,
    pub stage: PrefabSupplyStage,
    pub qa_passed: bool,
    pub last_scan_timestamp: String,
}

impl PrefabModuleUnit {
    pub fn crane_utilization_pct(&self) -> f32 {
        (self.mass_tons / self.crane_swl_tons.max(0.5)) * 100.0
    }

    pub fn default_units() -> Vec<Self> {
        vec![
            Self {
                tag_id: "MOD-TR-014".to_string(),
                rfid_code: "RFID-EG-88412".to_string(),
                name: "24 m Warren Truss Half-Segment T14".to_string(),
                module_type: PrefabModuleType::CompositeTrussSegment,
                dimensions_m: [12.0, 2.4, 2.2],
                mass_tons: 8.4,
                crane_swl_tons: 12.0,
                target_zone: "Grid B/1-3 Level 12".to_string(),
                stage: PrefabSupplyStage::InTransitFleet,
                qa_passed: true,
                last_scan_timestamp: "GPS Gate +08:42 UTC".to_string(),
            },
            Self {
                tag_id: "MOD-PC-208".to_string(),
                rfid_code: "RFID-EG-71904".to_string(),
                name: "Precast Shear Core Spandrel Wall PC-208".to_string(),
                module_type: PrefabModuleType::PrecastCorePanel,
                dimensions_m: [6.2, 0.35, 3.6],
                mass_tons: 10.2,
                crane_swl_tons: 12.0,
                target_zone: "Core South L06".to_string(),
                stage: PrefabSupplyStage::QaNdtInspection,
                qa_passed: true,
                last_scan_timestamp: "Ultrasonic NDT Pass".to_string(),
            },
            Self {
                tag_id: "MOD-VP-042".to_string(),
                rfid_code: "RFID-EG-55219".to_string(),
                name: "Volumetric MEP Core Toilet Pod VP-42".to_string(),
                module_type: PrefabModuleType::VolumetricSteelPod,
                dimensions_m: [4.8, 2.4, 3.1],
                mass_tons: 6.1,
                crane_swl_tons: 12.0,
                target_zone: "Bay C-3 Level 04".to_string(),
                stage: PrefabSupplyStage::SiteLaydownStaging,
                qa_passed: true,
                last_scan_timestamp: "Yard Bay #3 Staged".to_string(),
            },
            Self {
                tag_id: "MOD-CW-311".to_string(),
                rfid_code: "RFID-EG-92044".to_string(),
                name: "Double Low-E Curtain Wall Pod CW-311".to_string(),
                module_type: PrefabModuleType::UnitizedCurtainPanel,
                dimensions_m: [3.2, 0.25, 4.0],
                mass_tons: 1.85,
                crane_swl_tons: 5.0,
                target_zone: "East Facade L08".to_string(),
                stage: PrefabSupplyStage::ErectedAndTorqued,
                qa_passed: true,
                last_scan_timestamp: "As-Built Scan Verified".to_string(),
            },
            Self {
                tag_id: "MOD-TR-015".to_string(),
                rfid_code: "RFID-EG-88413".to_string(),
                name: "24 m Warren Truss Half-Segment T15".to_string(),
                module_type: PrefabModuleType::CompositeTrussSegment,
                dimensions_m: [12.0, 2.4, 2.2],
                mass_tons: 8.6,
                crane_swl_tons: 12.0,
                target_zone: "Grid B/3-5 Level 12".to_string(),
                stage: PrefabSupplyStage::FactoryCastingWeld,
                qa_passed: false,
                last_scan_timestamp: "Robotic Cell #2 Welding".to_string(),
            },
            Self {
                tag_id: "MOD-PC-212".to_string(),
                rfid_code: "RFID-EG-71910".to_string(),
                name: "Precast Twin-Tee Hollowcore Slab PC-212".to_string(),
                module_type: PrefabModuleType::PrecastCorePanel,
                dimensions_m: [9.0, 2.4, 0.4],
                mass_tons: 7.5,
                crane_swl_tons: 12.0,
                target_zone: "Podium Roof Bay D".to_string(),
                stage: PrefabSupplyStage::ShopDrawingApproved,
                qa_passed: true,
                last_scan_timestamp: "Tekla NC1 Released".to_string(),
            },
        ]
    }
}

#[cfg(test)]
mod phase29_tests {
    use super::*;

    #[test]
    fn test_clash_tolerance_filtering_and_bcf_xml_export() {
        let clashes = ClashDetectionItem::default_clashes();
        // CLASH-004 has -18 mm overlap: active at 15 mm, ignored above 18 mm
        let triggered_15 = clashes.iter().filter(|c| c.is_triggered_at_tolerance(15.0)).count();
        let triggered_30 = clashes.iter().filter(|c| c.is_triggered_at_tolerance(30.0)).count();
        assert!(triggered_15 > triggered_30);

        let topics = BcfTopicRecord::default_topics();
        let xml = BcfTopicRecord::export_bcf_xml(&topics, "EigenGraph Test Tower");
        assert!(xml.contains("<BcfArchive Version=\"2.1\""));
        assert!(xml.contains("BCF-101"));
        assert!(xml.contains("<EigenGraphOrbit"));
    }

    #[test]
    fn test_ai_copilot_deterministic_solvers() {
        let sw = solve_aci318_shear_wall(&CopilotShearWallParams::default());
        assert!(sw.dcr_ratio > 0.0 && sw.markdown_memo.contains("ACI 318-19"));

        let wind = solve_asce7_wind(&CopilotWindParams::default());
        assert!(wind.summary_headline.contains("kPa"));
        assert!(wind.markdown_memo.contains("ASCE 7-22"));

        let carbon = solve_iso14040_carbon(&CopilotCarbonParams::default());
        assert!(carbon.is_compliant);
        assert!(carbon.markdown_memo.contains("ISO 14040"));
    }

    #[test]
    fn test_lps_ppc_calculation_and_prefab_stages() {
        let mut tasks = LpsLookaheadTask::default_tasks();
        let (committed, completed, pct) = LpsLookaheadTask::compute_live_ppc(&tasks);
        assert_eq!(committed, 7);
        assert_eq!(completed, 5);
        assert!((pct - (500.0 / 7.0)).abs() < 0.01);

        // Removing constraint & marking completed raises PPC
        tasks[3].is_sound = true;
        tasks[3].completed_on_time = true;
        let (_, completed2, pct2) = LpsLookaheadTask::compute_live_ppc(&tasks);
        assert_eq!(completed2, 6);
        assert!(pct2 > pct);

        let units = PrefabModuleUnit::default_units();
        assert_eq!(
            PrefabSupplyStage::ShopDrawingApproved.next(),
            Some(PrefabSupplyStage::FactoryCastingWeld)
        );
        assert!(units[0].crane_utilization_pct() < 100.0);
    }

    #[test]
    fn test_parametric_preset_graph_generation() {
        let graph = VisualNodeGraph::from_preset(
            ParametricGeneratorPreset::DiagridTower,
            60.0,
            12.0,
            60.0,
            12.0,
        );
        assert_eq!(graph.nodes.len(), 5);
        assert_eq!(graph.wires.len(), 4);
        assert!(graph.nodes[3].value_display.contains("Struts"));
    }

    #[test]
    fn test_phase44_navigation_pillar_concise_english_names() {
        let names: Vec<&str> = NavigationPillar::all()
            .iter()
            .map(NavigationPillar::clean_name)
            .collect();
        assert_eq!(
            names,
            vec![
                "Vault",
                "Agent",
                "Planning",
                "Drafting",
                "Geometry",
                "Analysis",
                "Scheduling",
                "Budgeting",
                "Sustainability",
                "Facility",
                "Safety",
                "Construction",
                "Automation",
                "Prediction",
                "Report",
            ]
        );
        assert_eq!(NavigationPillar::MyLibrary.icon(), "VLT");
    }
}
