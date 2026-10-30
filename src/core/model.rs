//! model.rs — Structural Graph Topology & Finite Element Domain Models.

use serde::{Deserialize, Serialize};

/// Boundary restraint conditions (Degrees of Freedom fixities) at a Node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundaryFixity {
    pub ux: bool, // Translational restraint in X
    pub uy: bool, // Translational restraint in Y
    pub uz: bool, // Translational restraint in Z
    pub rx: bool, // Rotational restraint about X
    pub ry: bool, // Rotational restraint about Y
    pub rz: bool, // Rotational restraint about Z
}

impl Default for BoundaryFixity {
    fn default() -> Self {
        Self::free()
    }
}

impl BoundaryFixity {
    pub fn free() -> Self {
        Self { ux: false, uy: false, uz: false, rx: false, ry: false, rz: false }
    }

    pub fn pinned() -> Self {
        Self { ux: true, uy: true, uz: true, rx: false, ry: false, rz: false }
    }

    pub fn fixed() -> Self {
        Self { ux: true, uy: true, uz: true, rx: true, ry: true, rz: true }
    }

    pub fn roller_y() -> Self {
        Self { ux: false, uy: true, uz: false, rx: false, ry: false, rz: false }
    }

    pub fn is_free(&self) -> bool {
        !self.ux && !self.uy && !self.uz && !self.rx && !self.ry && !self.rz
    }
}

/// A 3D Spatial Node in the Structural Graph
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node3D {
    pub id: usize,
    pub x: f64, // Position in meters
    pub y: f64,
    pub z: f64,
    pub fixity: BoundaryFixity,
}

impl Node3D {
    pub fn new(id: usize, x: f64, y: f64, z: f64) -> Self {
        Self {
            id,
            x,
            y,
            z,
            fixity: BoundaryFixity::free(),
        }
    }

    pub fn with_fixity(mut self, fixity: BoundaryFixity) -> Self {
        self.fixity = fixity;
        self
    }

    pub fn distance_to(&self, other: &Node3D) -> f64 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        let dz = other.z - self.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Structural Element Behavior Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementType {
    /// 3D Truss (Axial-only, 3 translational DOFs per node)
    Truss3D,
    /// 3D Frame / Beam (Axial, Shear, Bending, Torsion, 6 DOFs per node)
    Frame3D,
}

/// Material properties for linear elastic mechanics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub name: String,
    pub elastic_modulus: f64, // Young's Modulus E (Pascals, e.g. 200e9 for Structural Steel)
    pub poisson_ratio: f64,   // nu (e.g. 0.3)
    pub yield_strength: f64,  // Fy (Pascals, e.g. 250e6 for BJ37 / A36)
    pub density: f64,         // Density rho (kg/m^3, e.g. 7850 for steel)
}

impl Default for Material {
    fn default() -> Self {
        Self::structural_steel()
    }
}

impl Material {
    pub fn structural_steel() -> Self {
        Self {
            name: "Structural Steel (BJ37 / A36)".to_string(),
            elastic_modulus: 200.0e9,
            poisson_ratio: 0.3,
            yield_strength: 250.0e6,
            density: 7850.0,
        }
    }

    pub fn shear_modulus(&self) -> f64 {
        self.elastic_modulus / (2.0 * (1.0 + self.poisson_ratio))
    }
}

/// Geometric Cross-Section Properties
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossSection {
    pub name: String,
    pub area: f64,      // Cross-sectional area A in m^2
    pub iy: f64,        // Second moment of area about weak axis Iy in m^4
    pub iz: f64,        // Second moment of area about strong axis Iz in m^4
    pub j_torsion: f64, // Polar/torsional constant J in m^4
    pub depth: f64,     // Section height in meters
    pub width: f64,     // Flange width in meters
}

impl Default for CrossSection {
    fn default() -> Self {
        Self::w14x90()
    }
}

impl CrossSection {
    pub fn w14x90() -> Self {
        // Standard AISC W14x90 profile
        // Area = 26.5 in^2 = 0.0171 m^2
        // Iz = 999 in^4 = 4.158e-4 m^4
        // Iy = 362 in^4 = 1.507e-4 m^4
        // J = 4.06 in^4 = 1.69e-6 m^4
        Self {
            name: "W14x90".to_string(),
            area: 0.0171,
            iy: 1.507e-4,
            iz: 4.158e-4,
            j_torsion: 1.69e-6,
            depth: 0.356,
            width: 0.369,
        }
    }

    pub fn hss8x8() -> Self {
        // HSS 8x8x1/2 Box Section
        Self {
            name: "HSS8x8x1/2".to_string(),
            area: 0.00929,
            iy: 9.32e-5,
            iz: 9.32e-5,
            j_torsion: 1.45e-4,
            depth: 0.203,
            width: 0.203,
        }
    }
}

/// A 3D Structural Member / Element
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Element3D {
    pub id: usize,
    pub name: String,
    pub start_node: usize,
    pub end_node: usize,
    pub element_type: ElementType,
    pub section: CrossSection,
    pub material: Material,
}

impl Element3D {
    pub fn new(id: usize, name: impl Into<String>, start_node: usize, end_node: usize) -> Self {
        Self {
            id,
            name: name.into(),
            start_node,
            end_node,
            element_type: ElementType::Frame3D,
            section: CrossSection::default(),
            material: Material::default(),
        }
    }

    pub fn with_type(mut self, elem_type: ElementType) -> Self {
        self.element_type = elem_type;
        self
    }

    pub fn with_section(mut self, section: CrossSection) -> Self {
        self.section = section;
        self
    }
}

/// Point load applied at a specific Node
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodalLoad {
    pub node_id: usize,
    pub fx: f64, // Force in Newtons
    pub fy: f64,
    pub fz: f64,
    pub mx: f64, // Moment in Newton-meters
    pub my: f64,
    pub mz: f64,
}

impl NodalLoad {
    pub fn force_y(node_id: usize, fy_newtons: f64) -> Self {
        Self {
            node_id,
            fx: 0.0,
            fy: fy_newtons,
            fz: 0.0,
            mx: 0.0,
            my: 0.0,
            mz: 0.0,
        }
    }
}

/// The Complete Structural Graph Model
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StructuralModel {
    pub name: String,
    pub nodes: Vec<Node3D>,
    pub elements: Vec<Element3D>,
    pub loads: Vec<NodalLoad>,
}

impl StructuralModel {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: Vec::new(),
            elements: Vec::new(),
            loads: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: Node3D) -> usize {
        let id = node.id;
        self.nodes.push(node);
        id
    }

    pub fn add_element(&mut self, element: Element3D) -> usize {
        let id = element.id;
        self.elements.push(element);
        id
    }

    pub fn add_load(&mut self, load: NodalLoad) {
        self.loads.push(load);
    }

    pub fn find_node(&self, id: usize) -> Option<&Node3D> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn member_length(&self, elem: &Element3D) -> Option<f64> {
        let n1 = self.find_node(elem.start_node)?;
        let n2 = self.find_node(elem.end_node)?;
        Some(n1.distance_to(n2))
    }

    /// Computes the total mass of the structural steel members in Kilograms
    pub fn total_steel_mass_kg(&self) -> f64 {
        let mut total = 0.0;
        for elem in &self.elements {
            if let Some(l) = self.member_length(elem) {
                let volume = elem.section.area * l;
                total += volume * elem.material.density;
            }
        }
        total
    }

    /// Computes total steel weight in Metric Tonnes
    pub fn total_steel_tonnage(&self) -> f64 {
        self.total_steel_mass_kg() / 1000.0
    }
}
