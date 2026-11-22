//! model.rs — Structural Graph Topology & Finite Element Domain Models.

use serde::{Deserialize, Serialize};

/// Boundary restraint conditions (Degrees of Freedom fixities) at a Node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundaryFixity {
    pub ux: bool,
    pub uy: bool,
    pub uz: bool,
    pub rx: bool,
    pub ry: bool,
    pub rz: bool,
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

    pub fn roller_x() -> Self {
        Self { ux: false, uy: true, uz: true, rx: false, ry: false, rz: false }
    }

    pub fn is_pinned(&self) -> bool {
        self.ux && self.uy && self.uz && !self.rx && !self.ry && !self.rz
    }

    pub fn is_fixed(&self) -> bool {
        self.ux && self.uy && self.uz && self.rx && self.ry && self.rz
    }

    pub fn is_free(&self) -> bool {
        !self.ux && !self.uy && !self.uz && !self.rx && !self.ry && !self.rz
    }

    pub fn is_roller(&self) -> bool {
        !self.is_fixed() && !self.is_pinned() && !self.is_free()
    }
}

/// A 3D Spatial Node in the Structural Graph
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node3D {
    pub id: usize,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub fixity: BoundaryFixity,
    /// Optional 6-DOF elastic Winkler / SSI nodal foundation springs `[kx, ky, kz, krx, kry, krz]`
    /// in SI units (`N/m` for translational DOFs, `N*m/rad` for rotational DOFs).
    #[serde(default)]
    pub spring_stiffness: [f64; 6],
}

impl Node3D {
    pub fn new(id: usize, x: f64, y: f64, z: f64) -> Self {
        Self {
            id,
            x,
            y,
            z,
            fixity: BoundaryFixity::free(),
            spring_stiffness: [0.0; 6],
        }
    }

    pub fn with_fixity(mut self, fixity: BoundaryFixity) -> Self {
        self.fixity = fixity;
        self
    }

    pub fn with_springs(mut self, spring_stiffness: [f64; 6]) -> Self {
        self.spring_stiffness = spring_stiffness;
        self
    }

    pub fn has_springs(&self) -> bool {
        self.spring_stiffness
            .iter()
            .any(|&k| k.is_finite() && k > 0.0)
    }

    pub fn distance_to(&self, other: &Node3D) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Structural Element Type Classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementType {
    Truss3D,
    Frame3D,
}

/// Isotropic Elastic Material Properties
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub name: String,
    pub elastic_modulus: f64, // E (Pascals)
    pub poisson_ratio: f64,   // nu
    pub yield_strength: f64,  // Fy (Pascals)
    pub density: f64,         // kg/m^3
}

impl Default for Material {
    fn default() -> Self {
        Self::steel_q355()
    }
}

impl Material {
    pub fn steel_q355() -> Self {
        Self {
            name: "Steel Q355 / BJ 55".to_string(),
            elastic_modulus: 205.0e9, // 205 GPa
            poisson_ratio: 0.30,
            yield_strength: 355.0e6, // 355 MPa
            density: 7850.0,         // kg/m^3
        }
    }

    pub fn structural_steel() -> Self {
        Self::steel_q355()
    }

    pub fn steel_a36() -> Self {
        Self {
            name: "Steel A36 / BJ 37".to_string(),
            elastic_modulus: 200.0e9, // 200 GPa
            poisson_ratio: 0.26,
            yield_strength: 250.0e6, // 250 MPa
            density: 7850.0,
        }
    }

    /// Standard structural reinforced concrete (f'c = 30 MPa, Ec = 4700*sqrt(30) ≈ 25.74 GPa).
    pub fn reinforced_concrete_fc30() -> Self {
        Self {
            name: "Concrete fc' 30 MPa".to_string(),
            elastic_modulus: 25.74e9, // 25.74 GPa (SNI 2847 / ACI 318)
            poisson_ratio: 0.20,
            yield_strength: 30.0e6, // 30 MPa nominal design strength
            density: 2400.0,        // kg/m^3
        }
    }

    /// Detects whether a section profile label refers to a hot-rolled or hollow steel section.
    pub fn is_steel_profile(profile: &str) -> bool {
        let up = profile.trim().to_ascii_uppercase();
        up.starts_with("WF")
            || up.starts_with("H-")
            || up.starts_with("H ")
            || up.starts_with("IPE")
            || up.starts_with("HEA")
            || up.starts_with("HEB")
            || up.starts_with("UB")
            || up.starts_with("UC")
            || up.starts_with("W1")
            || up.starts_with("W2")
            || up.starts_with("W3")
            || up.starts_with("BOX")
            || up.starts_with("SHS")
            || up.starts_with("RHS")
            || up.starts_with("BRB")
            || up.contains("BRB")
            || up.contains("STEEL")
    }

    /// Resolves structural steel vs reinforced concrete material from a BIM section profile label.
    pub fn from_profile_name(profile: &str) -> Self {
        if Self::is_steel_profile(profile) {
            Self::steel_q355()
        } else {
            Self::reinforced_concrete_fc30()
        }
    }

    pub fn shear_modulus(&self) -> f64 {
        self.elastic_modulus / (2.0 * (1.0 + self.poisson_ratio))
    }
}

/// Geometric Cross-Section Profile
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossSection {
    pub name: String,
    pub area: f64,      // A (m^2)
    pub iy: f64,        // Moment of inertia about local y-axis (m^4)
    pub iz: f64,        // Moment of inertia about local z-axis (m^4)
    pub j_torsion: f64, // St. Venant torsional constant (m^4)
    pub depth: f64,     // Section height (m)
    pub width: f64,     // Section width (m)
}

impl Default for CrossSection {
    fn default() -> Self {
        Self::wf_300x150()
    }
}

impl CrossSection {
    /// Constructs a solid rectangular reinforced-concrete cross-section with ACI 318 / SNI 2847
    /// effective cracked moment-of-inertia modifier (`0.70` columns, `0.35` beams).
    pub fn rectangular_rc(
        name: impl Into<String>,
        width_m: f64,
        depth_m: f64,
        cracked_i_factor: f64,
    ) -> Self {
        let b = width_m.clamp(0.08, 4.0);
        let h = depth_m.clamp(0.08, 4.0);
        let k_cr = cracked_i_factor.clamp(0.25, 1.0);
        let area = b * h;
        let iz = k_cr * (b * h.powi(3)) / 12.0;
        let iy = k_cr * (h * b.powi(3)) / 12.0;
        let short_dim = b.min(h);
        let long_dim = b.max(h);
        let j_torsion = k_cr * 0.141 * short_dim.powi(3) * long_dim;
        Self {
            name: name.into(),
            area,
            iy,
            iz,
            j_torsion,
            depth: h,
            width: b,
        }
    }
    pub fn wf_300x150() -> Self {
        Self {
            name: "WF 300x150x6.5x9".to_string(),
            area: 46.78e-4,       // 46.78 cm^2
            iy: 5.08e-6,          // 508 cm^4
            iz: 72.1e-6,          // 7210 cm^4
            j_torsion: 13.1e-8,   // 13.1 cm^4
            depth: 0.300,
            width: 0.150,
        }
    }

    pub fn wf_400x200() -> Self {
        Self {
            name: "WF 400x200x8x13".to_string(),
            area: 84.1e-4,        // 84.1 cm^2
            iy: 17.4e-6,          // 1740 cm^4
            iz: 237.0e-6,         // 23700 cm^4
            j_torsion: 42.6e-8,
            depth: 0.400,
            width: 0.200,
        }
    }

    pub fn w14x90() -> Self {
        Self::wf_400x200()
    }

    pub fn box_200x200x8() -> Self {
        Self {
            name: "HSS Box 200x200x8".to_string(),
            area: 59.5e-4,
            iy: 36.5e-6,
            iz: 36.5e-6,
            j_torsion: 57.0e-6,
            depth: 0.200,
            width: 0.200,
        }
    }

    pub fn hss8x8() -> Self {
        Self::box_200x200x8()
    }

    pub fn tube_chs_300x10() -> Self {
        Self {
            name: "CHS Tube 300x10".to_string(),
            area: 91.1e-4,
            iy: 98.2e-6,
            iz: 98.2e-6,
            j_torsion: 196.4e-6,
            depth: 0.300,
            width: 0.300,
        }
    }

    pub fn pipe_chs_150x6() -> Self {
        Self {
            name: "CHS Pipe 150x6".to_string(),
            area: 27.1e-4,
            iy: 7.21e-6,
            iz: 7.21e-6,
            j_torsion: 14.42e-6,
            depth: 0.150,
            width: 0.150,
        }
    }

    pub fn wf_150x75() -> Self {
        Self {
            name: "WF 150x75x5x7".to_string(),
            area: 17.85e-4,
            iy: 0.495e-6,
            iz: 6.66e-6,
            j_torsion: 2.31e-8,
            depth: 0.150,
            width: 0.075,
        }
    }

    pub fn wf_200x100() -> Self {
        Self {
            name: "WF 200x100x5.5x8".to_string(),
            area: 27.16e-4,
            iy: 1.34e-6,
            iz: 18.4e-6,
            j_torsion: 4.68e-8,
            depth: 0.200,
            width: 0.100,
        }
    }

    pub fn wf_250x125() -> Self {
        Self {
            name: "WF 250x125x6x9".to_string(),
            area: 37.66e-4,
            iy: 2.94e-6,
            iz: 40.5e-6,
            j_torsion: 8.15e-8,
            depth: 0.250,
            width: 0.125,
        }
    }

    pub fn h_200x200() -> Self {
        Self {
            name: "H-Beam 200x200x8x12".to_string(),
            area: 63.53e-4,
            iy: 16.0e-6,
            iz: 47.2e-6,
            j_torsion: 28.5e-8,
            depth: 0.200,
            width: 0.200,
        }
    }

    pub fn h_300x300() -> Self {
        Self {
            name: "H-Beam 300x300x10x15".to_string(),
            area: 119.8e-4,
            iy: 67.5e-6,
            iz: 204.0e-6,
            j_torsion: 81.2e-8,
            depth: 0.300,
            width: 0.300,
        }
    }

    pub fn unp_150x75() -> Self {
        Self {
            name: "UNP 150x75x6.5x10".to_string(),
            area: 23.71e-4,
            iy: 1.14e-6,
            iz: 8.64e-6,
            j_torsion: 5.80e-8,
            depth: 0.150,
            width: 0.075,
        }
    }

    /// Returns the complete integrated JIS G3192 / SNI / AISC Standard Steel Profile Catalog
    pub fn catalog_jis_aisc() -> Vec<Self> {
        vec![
            Self::wf_150x75(),
            Self::wf_200x100(),
            Self::wf_250x125(),
            Self::wf_300x150(),
            Self::wf_400x200(),
            Self::h_200x200(),
            Self::h_300x300(),
            Self::box_200x200x8(),
            Self::tube_chs_300x10(),
            Self::pipe_chs_150x6(),
            Self::unp_150x75(),
        ]
    }

    /// Radius of gyration about minor axis y: r_y = sqrt(I_y / A)
    #[inline]
    pub fn radius_of_gyration_y(&self) -> f64 {
        (self.iy / self.area.max(1e-12)).sqrt()
    }

    /// Radius of gyration about major axis z: r_z = sqrt(I_z / A)
    #[inline]
    pub fn radius_of_gyration_z(&self) -> f64 {
        (self.iz / self.area.max(1e-12)).sqrt()
    }
}

/// Primary Structural Load Case Categories (SNI 1727:2020 / ASCE 7-22)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoadCaseType {
    Dead,     // D: Beban Mati
    Live,     // L: Beban Hidup
    RoofLive, // Lr: Beban Hidup Atap
    Wind,     // W: Beban Angin
    Seismic,  // E: Beban Gempa (SNI 1726:2019)
}

/// Factored Load Combination per SNI 1727:2020 / ASCE 7-22 Section 2.3.1 (LRFD)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadCombination {
    pub name: String,
    pub factor_dead: f64,
    pub factor_live: f64,
    pub factor_roof_live: f64,
    pub factor_wind: f64,
    pub factor_seismic: f64,
}

impl LoadCombination {
    /// Generates the 6 canonical LRFD Load Combinations per SNI 1727:2020 / ASCE 7-22
    pub fn standard_sni_1727_lrfd() -> Vec<Self> {
        vec![
            Self {
                name: "LC1: 1.4D".to_string(),
                factor_dead: 1.4,
                factor_live: 0.0,
                factor_roof_live: 0.0,
                factor_wind: 0.0,
                factor_seismic: 0.0,
            },
            Self {
                name: "LC2: 1.2D + 1.6L + 0.5Lr".to_string(),
                factor_dead: 1.2,
                factor_live: 1.6,
                factor_roof_live: 0.5,
                factor_wind: 0.0,
                factor_seismic: 0.0,
            },
            Self {
                name: "LC3: 1.2D + 1.0W + 1.0L + 0.5Lr".to_string(),
                factor_dead: 1.2,
                factor_live: 1.0,
                factor_roof_live: 0.5,
                factor_wind: 1.0,
                factor_seismic: 0.0,
            },
            Self {
                name: "LC4: 1.2D + 1.0E + 1.0L".to_string(),
                factor_dead: 1.2,
                factor_live: 1.0,
                factor_roof_live: 0.0,
                factor_wind: 0.0,
                factor_seismic: 1.0,
            },
            Self {
                name: "LC5: 0.9D + 1.0W".to_string(),
                factor_dead: 0.9,
                factor_live: 0.0,
                factor_roof_live: 0.0,
                factor_wind: 1.0,
                factor_seismic: 0.0,
            },
            Self {
                name: "LC6: 0.9D + 1.0E".to_string(),
                factor_dead: 0.9,
                factor_live: 0.0,
                factor_roof_live: 0.0,
                factor_wind: 0.0,
                factor_seismic: 1.0,
            },
        ]
    }

    /// Computes the combined scalar multiplier when applied to a nominal design load vector
    /// assuming typical structural breakdown (65% Dead, 25% Live, 10% Lateral/Environmental)
    pub fn effective_load_multiplier(&self) -> f64 {
        0.65 * self.factor_dead
            + 0.25 * self.factor_live
            + 0.05 * self.factor_roof_live
            + 0.15 * self.factor_wind.max(self.factor_seismic)
    }
}

/// A 3D Structural Member (Element) connecting two Nodes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Element3D {
    pub id: usize,
    pub name: String,
    pub start_node: usize,
    pub end_node: usize,
    pub element_type: ElementType,
    pub material: Material,
    pub section: CrossSection,
}

impl Element3D {
    pub fn new(id: usize, name: impl Into<String>, start_node: usize, end_node: usize) -> Self {
        Self {
            id,
            name: name.into(),
            start_node,
            end_node,
            element_type: ElementType::Truss3D,
            material: Material::default(),
            section: CrossSection::default(),
        }
    }

    pub fn with_type(mut self, element_type: ElementType) -> Self {
        self.element_type = element_type;
        self
    }

    pub fn with_section(mut self, section: CrossSection) -> Self {
        self.section = section;
        self
    }

    pub fn with_material(mut self, material: Material) -> Self {
        self.material = material;
        self
    }
}

/// Point load applied at a specific Node
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodalLoad {
    pub node_id: usize,
    pub fx: f64,
    pub fy: f64,
    pub fz: f64,
    pub mx: f64,
    pub my: f64,
    pub mz: f64,
}

impl NodalLoad {
    pub fn new(node_id: usize, fx: f64, fy: f64, fz: f64, mx: f64, my: f64, mz: f64) -> Self {
        Self {
            node_id,
            fx,
            fy,
            fz,
            mx,
            my,
            mz,
        }
    }

    pub fn force_z(node_id: usize, fz_newtons: f64) -> Self {
        Self {
            node_id,
            fx: 0.0,
            fy: 0.0,
            fz: fz_newtons,
            mx: 0.0,
            my: 0.0,
            mz: 0.0,
        }
    }

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

/// 2D 4-Node Quadrilateral Flat Shell Element (Mindlin-Reissner Plate Bending + Bilinear Membrane)
/// Suitable for Floor Slabs, Shear Walls, Core Walls, and Foundation Rafts
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShellElement4N {
    pub id: usize,
    pub name: String,
    pub nodes: [usize; 4], // 4 corner node indices in counter-clockwise order
    pub thickness_m: f64,  // Shell thickness t (meters)
    pub material: Material,
    pub is_shear_wall: bool,
}

impl ShellElement4N {
    pub fn new(id: usize, name: impl Into<String>, nodes: [usize; 4], thickness_m: f64) -> Self {
        Self {
            id,
            name: name.into(),
            nodes,
            thickness_m,
            material: Material::default(),
            is_shear_wall: false,
        }
    }

    pub fn with_material(mut self, material: Material) -> Self {
        self.material = material;
        self
    }

    pub fn as_shear_wall(mut self) -> Self {
        self.is_shear_wall = true;
        self
    }
}

/// The Complete Structural Graph Model
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StructuralModel {
    pub name: String,
    pub nodes: Vec<Node3D>,
    pub elements: Vec<Element3D>,
    pub shells: Vec<ShellElement4N>,
    pub loads: Vec<NodalLoad>,
}

impl StructuralModel {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: Vec::new(),
            elements: Vec::new(),
            shells: Vec::new(),
            loads: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: Node3D) {
        self.nodes.push(node);
    }

    pub fn add_element(&mut self, element: Element3D) {
        self.elements.push(element);
    }

    pub fn add_shell(&mut self, shell: ShellElement4N) {
        self.shells.push(shell);
    }

    pub fn add_load(&mut self, load: NodalLoad) {
        self.loads.push(load);
    }

    pub fn find_node(&self, id: usize) -> Option<&Node3D> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn find_element(&self, id: usize) -> Option<&Element3D> {
        self.elements.iter().find(|e| e.id == id)
    }

    pub fn find_shell(&self, id: usize) -> Option<&ShellElement4N> {
        self.shells.iter().find(|s| s.id == id)
    }

    pub fn member_length(&self, elem: &Element3D) -> Option<f64> {
        let n1 = self.find_node(elem.start_node)?;
        let n2 = self.find_node(elem.end_node)?;
        Some(n1.distance_to(n2))
    }

    pub fn shell_surface_area(&self, shell: &ShellElement4N) -> Option<f64> {
        let n0 = self.find_node(shell.nodes[0])?;
        let n1 = self.find_node(shell.nodes[1])?;
        let n2 = self.find_node(shell.nodes[2])?;
        let n3 = self.find_node(shell.nodes[3])?;

        let tri_area = |a: &Node3D, b: &Node3D, c: &Node3D| -> f64 {
            let ab = [b.x - a.x, b.y - a.y, b.z - a.z];
            let ac = [c.x - a.x, c.y - a.y, c.z - a.z];
            let cross = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            0.5 * (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt()
        };

        Some(tri_area(n0, n1, n2) + tri_area(n0, n2, n3))
    }

    /// Computes total steel mass in kg
    pub fn total_steel_mass_kg(&self) -> f64 {
        let mut total_mass = 0.0;
        for elem in &self.elements {
            if let (Some(n1), Some(n2)) = (self.find_node(elem.start_node), self.find_node(elem.end_node)) {
                let dx = n2.x - n1.x;
                let dy = n2.y - n1.y;
                let dz = n2.z - n1.z;
                let length = (dx * dx + dy * dy + dz * dz).sqrt();
                let volume = elem.section.area * length;
                total_mass += volume * elem.material.density;
            }
        }
        total_mass
    }

    /// Computes total steel mass in metric tonnes
    pub fn total_steel_tonnage(&self) -> f64 {
        self.total_steel_mass_kg() / 1000.0
    }
}
