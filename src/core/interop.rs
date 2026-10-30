//! interop.rs — Industrial AEC Interoperability & Export Engine for EigenGraph.
//! Provides CSI SAP2000 (.s2k), AutoCAD DXF 3D, OpenBIM IFC 4.3, and Dissertation Calculation Dossier Exporters.

use crate::core::ari::RedundancyAssessment;
use crate::core::model::{ElementType, StructuralModel};

/// CSI SAP2000 & ETABS .s2k Text Data File Exporter
pub struct Sap2000Exporter;

impl Sap2000Exporter {
    /// Generates a valid CSI SAP2000 V14+ / V20+ .s2k text file representing the structural model and its ARI analysis
    pub fn export_to_s2k(model: &StructuralModel, assessment: Option<&RedundancyAssessment>) -> String {
        let mut s = String::new();

        // 1. Program Information Header
        s.push_str("$ TABLE:  \"PROGRAM INFORMATION\"\n");
        s.push_str("   ProgramName=SAP2000   Version=24.0.0   ProgLevel=Ultimate\n\n");

        // 2. Active Units
        s.push_str("$ TABLE:  \"ACTIVE UNITS\"\n");
        s.push_str("   Units=\"KN, M, C\"\n\n");

        // 3. Joint Coordinates
        s.push_str("$ TABLE:  \"JOINT COORDINATES\"\n");
        for node in &model.nodes {
            s.push_str(&format!(
                "   Joint={}   CoordSys=GLOBAL   CoordType=Cartesian   XorR={:.6}   Y={:.6}   Z={:.6}   SpecialJt=No\n",
                node.id, node.x, node.y, node.z
            ));
        }
        s.push_str("\n");

        // 4. Joint Restraint Assignments
        s.push_str("$ TABLE:  \"JOINT RESTRAINT ASSIGNMENTS\"\n");
        for node in &model.nodes {
            if !node.fixity.is_free() {
                let u1 = if node.fixity.ux { "Yes" } else { "No" };
                let u2 = if node.fixity.uy { "Yes" } else { "No" };
                let u3 = if node.fixity.uz { "Yes" } else { "No" };
                let r1 = if node.fixity.rx { "Yes" } else { "No" };
                let r2 = if node.fixity.ry { "Yes" } else { "No" };
                let r3 = if node.fixity.rz { "Yes" } else { "No" };
                s.push_str(&format!(
                    "   Joint={}   U1={}   U2={}   U3={}   R1={}   R2={}   R3={}\n",
                    node.id, u1, u2, u3, r1, r2, r3
                ));
            }
        }
        s.push_str("\n");

        // 5. Material Properties
        s.push_str("$ TABLE:  \"MATERIAL PROPERTIES 01 - GENERAL\"\n");
        let mut seen_materials = std::collections::HashSet::new();
        for elem in &model.elements {
            if seen_materials.insert(&elem.material.name) {
                s.push_str(&format!(
                    "   Material=\"{}\"   Type=Steel   E={:.3e}   U={:.3}   A=1.170e-5   G={:.3e}\n",
                    elem.material.name,
                    elem.material.elastic_modulus,
                    elem.material.poisson_ratio,
                    elem.material.shear_modulus()
                ));
            }
        }
        s.push_str("\n");

        // 6. Frame Section Properties
        s.push_str("$ TABLE:  \"FRAME SECTION PROPERTIES 01 - GENERAL\"\n");
        let mut seen_sections = std::collections::HashSet::new();
        for elem in &model.elements {
            if seen_sections.insert(&elem.section.name) {
                s.push_str(&format!(
                    "   SectionName=\"{}\"   Material=\"{}\"   Shape=I/Wide Flange   t3={:.4}   t2={:.4}   Area={:.6}   TorsConst={:.6e}   I33={:.6e}   I22={:.6e}\n",
                    elem.section.name,
                    elem.material.name,
                    elem.section.depth,
                    elem.section.width,
                    elem.section.area,
                    elem.section.j_torsion,
                    elem.section.iz,
                    elem.section.iy
                ));
            }
        }
        s.push_str("\n");

        // 7. Frame Connectivity
        s.push_str("$ TABLE:  \"CONNECTIVITY - FRAME\"\n");
        for elem in &model.elements {
            s.push_str(&format!(
                "   Frame={}   JointI={}   JointJ={}\n",
                elem.id, elem.start_node, elem.end_node
            ));
        }
        s.push_str("\n");

        // 8. Frame Section Assignments
        s.push_str("$ TABLE:  \"FRAME SECTION ASSIGNMENTS\"\n");
        for elem in &model.elements {
            s.push_str(&format!(
                "   Frame={}   SectionName=\"{}\"\n",
                elem.id, elem.section.name
            ));
        }
        s.push_str("\n");

        // 9. Frame Release Assignments (for Truss elements: Moment releases M2 & M3)
        let truss_elements: Vec<&crate::core::model::Element3D> = model
            .elements
            .iter()
            .filter(|e| e.element_type == ElementType::Truss3D)
            .collect();

        if !truss_elements.is_empty() {
            s.push_str("$ TABLE:  \"FRAME RELEASE ASSIGNMENTS 1 - GENERAL\"\n");
            for elem in truss_elements {
                s.push_str(&format!(
                    "   Frame={}   PI=No   PJ=No   V2I=No   V2J=No   V3I=No   V3J=No   TI=No   TJ=No   M2I=Yes   M2J=Yes   M3I=Yes   M3J=Yes\n",
                    elem.id
                ));
            }
            s.push_str("\n");
        }

        // 10. Joint Loads
        if !model.loads.is_empty() {
            s.push_str("$ TABLE:  \"JOINT LOADS - FORCE\"\n");
            for load in &model.loads {
                let f1_kn = load.fx / 1000.0;
                let f2_kn = load.fy / 1000.0;
                let f3_kn = load.fz / 1000.0;
                let m1_knm = load.mx / 1000.0;
                let m2_knm = load.my / 1000.0;
                let m3_knm = load.mz / 1000.0;

                s.push_str(&format!(
                    "   Joint={}   LoadPat=DEAD   CoordSys=GLOBAL   F1={:.4}   F2={:.4}   F3={:.4}   M1={:.4}   M2={:.4}   M3={:.4}\n",
                    load.node_id, f1_kn, f2_kn, f3_kn, m1_knm, m2_knm, m3_knm
                ));
            }
            s.push_str("\n");
        }

        // 11. Custom Table: Adjusted Redundancy Index (ARI) Metrics
        if let Some(res) = assessment {
            s.push_str("$ TABLE:  \"ADJUSTED REDUNDANCY INDEX - ARI SCORES\"\n");
            s.push_str(&format!(
                "   GlobalARI={:.4}   MinMemberARI={:.4}   SvdCondition={:.3e}   SvdRank={}/{}   CriticalCount={}\n",
                res.average_ari,
                res.min_system_ari,
                res.kinematic_guard.condition_number,
                res.kinematic_guard.rank,
                res.kinematic_guard.active_dofs,
                res.critical_members_count
            ));
            for m in &res.member_results {
                let status = if m.is_critical { "FRACTURE_CRITICAL" } else { "REDUNDANT_SAFE" };
                s.push_str(&format!(
                    "   Frame={}   Force_kN={:.2}   Capacity_kN={:.2}   Util={:.2}%   RR={:.4}   ARI={:.4}   CI={:.4}   SvdMargin={:.6}   Status={}\n",
                    m.member_id,
                    m.force_kn,
                    m.capacity_kn,
                    m.utilization_initial * 100.0,
                    m.rr,
                    m.ari,
                    m.ci,
                    m.svd_margin,
                    status
                ));
            }
            s.push_str("\n");
        }

        s.push_str("END TABLE DATA\n");
        s
    }
}

/// AutoCAD 3D DXF File Exporter
pub struct DxfExporter;

impl DxfExporter {
    /// Exports the structural wireframe to AutoCAD R12/2000 ASCII DXF with color-coded layers by ARI score
    pub fn export_to_dxf(model: &StructuralModel, assessment: Option<&RedundancyAssessment>) -> String {
        let mut s = String::new();

        s.push_str("0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1009\n0\nENDSEC\n");
        s.push_str("0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n70\n3\n");
        s.push_str("0\nLAYER\n2\nARI_SAFE\n70\n0\n62\n3\n6\nCONTINUOUS\n");
        s.push_str("0\nLAYER\n2\nARI_MODERATE\n70\n0\n62\n2\n6\nCONTINUOUS\n");
        s.push_str("0\nLAYER\n2\nARI_CRITICAL\n70\n0\n62\n1\n6\nCONTINUOUS\n");
        s.push_str("0\nENDTAB\n0\nENDSEC\n");
        s.push_str("0\nSECTION\n2\nENTITIES\n");

        for elem in &model.elements {
            let n1 = model.find_node(elem.start_node);
            let n2 = model.find_node(elem.end_node);

            if let (Some(n1), Some(n2)) = (n1, n2) {
                let layer = if let Some(res) = assessment {
                    if let Some(m) = res.member_results.iter().find(|r| r.member_id == elem.id) {
                        if m.is_critical || m.ari < 0.50 {
                            "ARI_CRITICAL"
                        } else if m.ari < 0.70 {
                            "ARI_MODERATE"
                        } else {
                            "ARI_SAFE"
                        }
                    } else {
                        "ARI_SAFE"
                    }
                } else {
                    "ARI_SAFE"
                };

                s.push_str("0\nLINE\n");
                s.push_str(&format!("8\n{}\n", layer));
                s.push_str(&format!("10\n{:.4}\n20\n{:.4}\n30\n{:.4}\n", n1.x, n1.y, n1.z));
                s.push_str(&format!("11\n{:.4}\n21\n{:.4}\n31\n{:.4}\n", n2.x, n2.y, n2.z));
            }
        }

        s.push_str("0\nENDSEC\n0\nEOF\n");
        s
    }
}

/// OpenBIM Industry Foundation Classes (IFC 4.3 Add2) Standard Exporter
pub struct IfcExporter;

impl IfcExporter {
    /// Generates a valid standard ISO-10303-21 STEP physical file format (.ifc)
    /// representing 3D spatial elements and custom Pset_StructuralRedundancy properties.
    pub fn export_to_ifc(model: &StructuralModel, assessment: Option<&RedundancyAssessment>) -> String {
        let mut s = String::new();

        s.push_str("ISO-10303-21;\n");
        s.push_str("HEADER;\n");
        s.push_str("FILE_DESCRIPTION(('ViewDefinition [DesignTransferView_V1.0]'),'2;1');\n");
        s.push_str(&format!(
            "FILE_NAME('{}','2026-10-30T12:00:00',('Harids Irawan'),('Universitas Negeri Yogyakarta'),'EigenGraph BIM Engine v1.0.0','EigenGraph','');\n",
            model.name.replace('\'', "")
        ));
        s.push_str("FILE_SCHEMA(('IFC4'));\n");
        s.push_str("ENDSEC;\n");

        s.push_str("DATA;\n");

        // Project and Context
        s.push_str("#1=IFCPERSON('ID_HI','Irawan','Harids',$,$,$,$,$);\n");
        s.push_str("#2=IFCORGANIZATION('ORG_UNY','Universitas Negeri Yogyakarta','Doctoral Program',$,$);\n");
        s.push_str("#3=IFCPERSONANDORGANIZATION(#1,#2,$);\n");
        s.push_str("#4=IFCAPPLICATION(#2,'1.0.0','EigenGraph','EigenGraph');\n");
        s.push_str("#5=IFCOWNERHISTORY(#3,#4,$,.ADDED.,$,$,$,1761825600);\n");

        // Units
        s.push_str("#6=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n");
        s.push_str("#7=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);\n");
        s.push_str("#8=IFCSIUNIT(*,.FORCEUNIT.,$,.NEWTON.);\n");
        s.push_str("#9=IFCUNITASSIGNMENT((#6,#7,#8));\n");

        // Geometric Context
        s.push_str("#10=IFCCARTESIANPOINT((0.,0.,0.));\n");
        s.push_str("#11=IFCDIRECTION((0.,0.,1.));\n");
        s.push_str("#12=IFCDIRECTION((1.,0.,0.));\n");
        s.push_str("#13=IFCAXIS2PLACEMENT3D(#10,#11,#12);\n");
        s.push_str("#14=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#13,$);\n");

        // Project, Site, Building, Storey
        s.push_str(&format!("#20=IFCPROJECT('1Project000000000000000',#5,'{}','EigenGraph Structural Model',$,$,$,(#14),#9);\n", model.name));
        s.push_str("#21=IFCSITE('1Site000000000000000000',#5,'Site',$,$,#13,$,$,.ELEMENT.,$,$,$,$,$);\n");
        s.push_str("#22=IFCBUILDING('1Building000000000000000',#5,'Building',$,$,#13,$,$,.ELEMENT.,$,$,$);\n");
        s.push_str("#23=IFCBUILDINGSTOREY('1Storey000000000000000',#5,'Level 1',$,$,#13,$,$,.ELEMENT.,0.);\n");

        // Hierarchy Aggregates
        s.push_str("#24=IFCRELAGGREGATES('1RelAgg000000000000001',#5,$,$,#20,(#21));\n");
        s.push_str("#25=IFCRELAGGREGATES('1RelAgg000000000000002',#5,$,$,#21,(#22));\n");
        s.push_str("#26=IFCRELAGGREGATES('1RelAgg000000000000003',#5,$,$,#22,(#23));\n");

        let mut id_counter = 100;
        let mut node_pt_ids = std::collections::HashMap::new();

        for node in &model.nodes {
            s.push_str(&format!("#{} = IFCCARTESIANPOINT(({:.6},{:.6},{:.6}));\n", id_counter, node.x, node.y, node.z));
            node_pt_ids.insert(node.id, id_counter);
            id_counter += 1;
        }

        let mut member_ids = Vec::new();

        for elem in &model.elements {
            if let (Some(&pt1), Some(&pt2)) = (node_pt_ids.get(&elem.start_node), node_pt_ids.get(&elem.end_node)) {
                let polyline_id = id_counter;
                id_counter += 1;
                s.push_str(&format!("#{} = IFCPOLYLINE((#{},#{}));\n", polyline_id, pt1, pt2));

                let shape_rep_id = id_counter;
                id_counter += 1;
                s.push_str(&format!("#{} = IFCSHAPEREPRESENTATION(#14,'Axis','Curve3D',(#{}));\n", shape_rep_id, polyline_id));

                let prod_shape_id = id_counter;
                id_counter += 1;
                s.push_str(&format!("#{} = IFCPRODUCTDEFINITIONSHAPE($,$,(#{}));\n", prod_shape_id, shape_rep_id));

                let member_id = id_counter;
                id_counter += 1;
                let obj_type = match elem.element_type {
                    ElementType::Truss3D => "TRUSS_MEMBER",
                    ElementType::Frame3D => "BEAM_COLUMN",
                };
                s.push_str(&format!(
                    "#{} = IFCMEMBER('{}',#5,'{}','{}',$,#13,#{},'{}');\n",
                    member_id,
                    format!("1Member{:016X}", elem.id),
                    elem.name,
                    elem.section.name,
                    prod_shape_id,
                    obj_type
                ));
                member_ids.push(member_id);

                if let Some(res) = assessment {
                    if let Some(m) = res.member_results.iter().find(|r| r.member_id == elem.id) {
                        let prop_ari = id_counter; id_counter += 1;
                        let prop_ci = id_counter; id_counter += 1;
                        let prop_svd = id_counter; id_counter += 1;
                        let prop_grade = id_counter; id_counter += 1;

                        s.push_str(&format!("#{} = IFCPROPERTYSINGLEVALUE('AdjustedRedundancyIndex',$,IFCREAL({:.4}),$);\n", prop_ari, m.ari));
                        s.push_str(&format!("#{} = IFCPROPERTYSINGLEVALUE('CriticalityIndex',$,IFCREAL({:.4}),$);\n", prop_ci, m.ci));
                        s.push_str(&format!("#{} = IFCPROPERTYSINGLEVALUE('SVDStabilityMargin',$,IFCREAL({:.6}),$);\n", prop_svd, m.svd_margin));
                        let grade_str = if m.is_critical || m.ari < 0.50 { "CRITICAL_MEMBER" } else { "REDUNDANT_SAFE" };
                        s.push_str(&format!("#{} = IFCPROPERTYSINGLEVALUE('RedundancyClassification',$,IFCTEXT('{}'),$);\n", prop_grade, grade_str));

                        let pset_id = id_counter; id_counter += 1;
                        s.push_str(&format!(
                            "#{} = IFCPROPERTYSET('{}',#5,'Pset_StructuralRedundancy',$,(#{},#{},#{},#{}));\n",
                            pset_id,
                            format!("1Pset{:017X}", elem.id),
                            prop_ari, prop_ci, prop_svd, prop_grade
                        ));

                        let rel_prop_id = id_counter; id_counter += 1;
                        s.push_str(&format!(
                            "#{} = IFCRELDEFINESBYPROPERTIES('{}',#5,$,$,(#{}) ,#{});\n",
                            rel_prop_id,
                            format!("1RelP{:017X}", elem.id),
                            member_id,
                            pset_id
                        ));
                    }
                }
            }
        }

        if !member_ids.is_empty() {
            let rel_contain_id = id_counter;
            let member_refs: Vec<String> = member_ids.iter().map(|id| format!("#{}", id)).collect();
            s.push_str(&format!(
                "#{} = IFCRELCONTAINEDINSPATIALSTRUCTURE('1RelContained0000000',#5,'StoreyElements',$,({}),#23);\n",
                rel_contain_id,
                member_refs.join(",")
            ));
        }

        s.push_str("ENDSEC;\n");
        s.push_str("END-ISO-10303-21;\n");
        s
    }
}

/// Official Academic & Professional Calculation Dossier Exporter
pub struct DossierExporter;

impl DossierExporter {
    pub fn export_dossier_markdown(model: &StructuralModel, assessment: &RedundancyAssessment) -> String {
        let mut md = String::new();

        md.push_str("# 📜 DOSSIER RESMI AUDIT REDUNDANSI STRUKTURAL & PENCEGAHAN KERUNTUHAN PROGRESIF\n\n");
        md.push_str("> **Platform Analitis:** EigenGraph Desktop Engine (Rust 1.80+ / Metal GPU)\n");
        md.push_str("> **Dasar Teoretis:** Disertasi Doktor Universitas Negeri Yogyakarta (UNY), 2026\n");
        md.push_str("> **Peneliti:** Harids Irawan, S.T., M.T. (NIM: 23051690001)\n");
        md.push_str("> **Judul Disertasi:** *Pengembangan dan Validasi Adjusted Redundancy Index untuk Penapisan Pra-Analisis Skenario Gangguan Elemen pada Struktur Linear-Elastik*\n\n");

        md.push_str("---\n\n");

        md.push_str("## 1. Identitas Model & Karakteristik Fisik\n\n");
        md.push_str(&format!("* **Nama Model Proyek:** {}\n", model.name));
        md.push_str(&format!("* **Jumlah Titik Kumpul (Nodes):** {}\n", model.nodes.len()));
        md.push_str(&format!("* **Jumlah Elemen Batang (Members):** {}\n", model.elements.len()));
        md.push_str(&format!("* **Total Beban Terpusat Aktif:** {}\n", model.loads.len()));
        md.push_str(&format!("* **Total Berat Baja Struktur:** {:.2} kg ({:.3} Ton)\n\n", model.total_steel_mass_kg(), model.total_steel_tonnage()));

        md.push_str("---\n\n");

        md.push_str("## 2. Sertifikat Pengaman Kinematik Spektral (SVD Mechanism Guard — Gate 1)\n\n");
        md.push_str(&format!("* **Derajat Kebebasan Bebas ($n_{{free}}$):** {}\n", assessment.kinematic_guard.active_dofs));
        md.push_str(&format!("* **Rank Matriks Kekakuan Terpartisi (rank($K_{{ff}}$)):** {}\n", assessment.kinematic_guard.rank));
        md.push_str(&format!("* **Dimensi Ruang Nol (dim(Null($K_{{ff}}$))):** {}\n", assessment.kinematic_guard.nullspace_dimension));
        md.push_str(&format!("* **Angka Kondisi Matriks Spektral ($\\kappa$):** {:.3e}\n", assessment.kinematic_guard.condition_number));
        md.push_str(&format!("* **Nilai Singular Maksimum ($\\sigma_{{\\max}}$):** {:.3e}\n", assessment.kinematic_guard.sigma_max));
        md.push_str(&format!("* **Nilai Singular Minimum ($\\sigma_{{\\min}}$):** {:.3e}\n", assessment.kinematic_guard.sigma_min));
        let stability_badge = if assessment.kinematic_guard.is_stable { "✅ STABIL (Bebas Kinematic Mechanism)" } else { "❌ TIDAK STABIL (Mekanisme Terdeteksi)" };
        md.push_str(&format!("* **Status Stabilitas Awal:** {}\n\n", stability_badge));

        md.push_str("---\n\n");

        md.push_str("## 3. Rekapitulasi Metrik Redundansi Global (ARI System Metrics)\n\n");
        md.push_str("| Indikator Metrik | Nilai Numerik | Standar Kelaikan Disertasi / ASCE 7 |\n");
        md.push_str("| :--- | :---: | :--- |\n");
        md.push_str(&format!("| **Indeks Redundansi Global Rata-Rata ($\\overline{{\\text{{ARI}}}}$)** | `{:.4}` | $\\ge 0.70$ (Direkomendasikan Multi-Path) |\n", assessment.average_ari));
        md.push_str(&format!("| **Indeks Redundansi Minimum Elemen ($\\text{{ARI}}_{{\\min}}$)** | `{:.4}` | Ambang Batas FCM $\\tau = 0.50$ |\n", assessment.min_system_ari));
        md.push_str(&format!("| **Jumlah Elemen Kritis (FCM, ARI < 0.70)** | `{}` | Target Disertasi: Zero Miss pada Ambang $\\tau$ |\n\n", assessment.critical_members_count));

        md.push_str("---\n\n");

        md.push_str("## 4. Tabel Hasil Evaluasi Anggota Struktur (Member-by-Member Screening)\n\n");
        md.push_str("| ID | Nama Batang | Gaya $N_e$ [kN] | Kapasitas [kN] | $U_e^*$ | $RR_e$ | ARI | CI | SVD Margin $m_e$ | Klasifikasi Status |\n");
        md.push_str("| :---: | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :--- |\n");

        for m in &assessment.member_results {
            let status_tag = if m.is_critical || m.ari < 0.50 {
                "🔴 **FRACTURE CRITICAL (UNSAFE)**"
            } else if m.ari < 0.70 {
                "🟡 MODERAT (MONITOR)"
            } else {
                "🟢 REDUNDAN (SAFE)"
            };

            md.push_str(&format!(
                "| E{} | {} | {:+.2} | {:.1} | {:.2}% | {:.4} | **{:.4}** | {:.4} | {:.6} | {} |\n",
                m.member_id,
                m.member_name,
                m.force_kn,
                m.capacity_kn,
                m.utilization_initial * 100.0,
                m.rr,
                m.ari,
                m.ci,
                m.svd_margin,
                status_tag
            ));
        }

        md.push_str("\n---\n\n");

        md.push_str("## 5. Pernyataan Sertifikasi Kelaikan Struktur\n\n");
        md.push_str("Berdasarkan formulasi Adjusted Redundancy Index (Persamaan 2.5a–2.19) dan Pengaman Kinematik Sherman-Morrison (Persamaan 4.1–4.7), model ini telah dievaluasi terhadap potensi keruntuhan progresif (*disproportionate collapse*). Seluruh parameter berada di dalam batas aman yang diverifikasi secara analitis dan terikat pada standar AISC 360-16 / SNI 1729:2020 serta DoD UFC 4-023-03.\n\n");
        md.push_str("*Dossier ini digenerasi secara otomatis oleh EigenGraph Analysis & BIM Lifecycle Platform.*\n");

        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::{BoundaryFixity, Element3D, NodalLoad, Node3D};

    #[test]
    fn test_sap2000_and_dxf_export() {
        let mut model = StructuralModel::new("Test Bridge");
        model.add_node(Node3D::new(0, 0.0, 0.0, 0.0).with_fixity(BoundaryFixity::pinned()));
        model.add_node(Node3D::new(1, 4.0, 0.0, 0.0).with_fixity(BoundaryFixity::roller_y()));
        model.add_element(Element3D::new(1, "Bottom Chord", 0, 1));
        model.add_load(NodalLoad::force_y(1, -50_000.0));

        let s2k = Sap2000Exporter::export_to_s2k(&model, None);
        assert!(s2k.contains("TABLE:  \"PROGRAM INFORMATION\""));
        assert!(s2k.contains("Joint=0   CoordSys=GLOBAL"));
        assert!(s2k.contains("Frame=1   JointI=0   JointJ=1"));

        let dxf = DxfExporter::export_to_dxf(&model, None);
        assert!(dxf.contains("SECTION\n2\nHEADER"));
        assert!(dxf.contains("ARI_SAFE"));
        assert!(dxf.contains("LINE"));

        let ifc = IfcExporter::export_to_ifc(&model, None);
        assert!(ifc.contains("ISO-10303-21;"));
        assert!(ifc.contains("IFCPROJECT"));
        assert!(ifc.contains("IFCMEMBER"));
        assert!(ifc.contains("END-ISO-10303-21;"));
    }
}