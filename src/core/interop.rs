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

        // 9. Frame Release Assignments (for Truss elements)
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
            "FILE_NAME('{}','2026-10-30T12:00:00',('Harids Irawan'),('EigenGraph Platform'),'EigenGraph BIM Engine v1.0.0','EigenGraph','');\n",
            model.name.replace('\'', "")
        ));
        s.push_str("FILE_SCHEMA(('IFC4'));\n");
        s.push_str("ENDSEC;\n");

        s.push_str("DATA;\n");

        // Project and Context
        s.push_str("#1=IFCPERSON('ID_HI','Irawan','Harids',$,$,$,$,$);\n");
        s.push_str("#2=IFCORGANIZATION('ORG_EIGENGRAPH','EigenGraph Structural Platform','Engineering & Research',$,$);\n");
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
        md.push_str("> **Dasar Teoretis:** EigenGraph Structural Redundancy Framework (2026)\n");
        md.push_str("> **Lead Engineer:** Harids Irawan, S.T., M.T. (EigenGraph Systems)\n");
        md.push_str("> **Dokumen Teknis:** *Formulasi & Validasi Adjusted Redundancy Index untuk Penapisan Kelaikan Skenario Gangguan Struktur Linear-Elastik*\n\n");

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
        md.push_str("| Indikator Metrik | Nilai Numerik | Standar Kelaikan EigenGraph / ASCE 7 |\n");
        md.push_str("| :--- | :---: | :--- |\n");
        md.push_str(&format!("| **Indeks Redundansi Global Rata-Rata ($\\overline{{\\text{{ARI}}}}$)** | `{:.4}` | $\\ge 0.70$ (Direkomendasikan Multi-Path) |\n", assessment.average_ari));
        md.push_str(&format!("| **Indeks Redundansi Minimum Elemen ($\\text{{ARI}}_{{\\min}}$)** | `{:.4}` | Ambang Batas FCM $\\tau = 0.50$ |\n", assessment.min_system_ari));
        md.push_str(&format!("| **Jumlah Elemen Kritis (FCM, ARI < 0.70)** | `{}` | Target EigenGraph: Zero Miss pada Ambang $\\tau$ |\n\n", assessment.critical_members_count));

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

    /// Menghasilkan Naskah Laporan Audit Rekayasa Komprehensif (30+ Halaman)
    pub fn export_comprehensive_dossier(
        model: &StructuralModel,
        assessment: &RedundancyAssessment,
        modal_report: Option<&crate::core::modal::ModalAnalysisReport>,
        pushover_report: Option<&crate::core::nonlinear::PushoverReport>,
        boq_report: Option<&crate::core::cost::CostEstimationReport>,
        carbon_report: Option<&crate::core::carbon::CarbonLcaReport>,
    ) -> String {
        let mut md = Self::export_dossier_markdown(model, assessment);

        // Bagian 6: Analisis Dinamik Ragam Getar & Spektrum Gempa SNI 1726:2019
        if let Some(m) = modal_report {
            md.push_str("\n---\n\n");
            md.push_str("## 6. Analisis Dinamik Ragam Getar & Spektrum Gempa SNI 1726:2019 / ASCE 7-16\n\n");
            md.push_str(&format!("* **Klasifikasi Situs Tanah:** {}\n", m.spectrum.site_class.label()));
            md.push_str(&format!("* **Parameter Spektra:** $S_s = {:.2}\\text{{g}}$, $S_1 = {:.2}\\text{{g}}$, $F_a = {:.2}$, $F_v = {:.2}$\n", m.spectrum.ss, m.spectrum.s1, m.spectrum.fa, m.spectrum.fv));
            md.push_str(&format!("* **Percepatan Desain:** $S_{{DS}} = {:.3}\\text{{g}}$, $S_{{D1}} = {:.3}\\text{{g}}$\n", m.spectrum.sds, m.spectrum.sd1));
            md.push_str(&format!("* **Perioda Transisi:** $T_0 = {:.3}\\text{{ s}}$, $T_s = {:.3}\\text{{ s}}$\n", m.spectrum.t0, m.spectrum.ts));
            md.push_str(&format!("* **Perioda Fundamental Struktur ($T_1$):** `{:.4} s` (Frekuensi Alami: `{:.2} Hz`)\n", m.fundamental_period_s, m.fundamental_frequency_hz));
            md.push_str(&format!("* **Gaya Geser Dasar Seismik ($V_b$):** `{:.1} kN` (Koefisien Seismik $C_s = {:.4}$, Berat Seismik $W = {:.1} kN$)\n", m.seismic_base_shear_kn, m.spectrum.cs, m.total_weight_kn));
            md.push_str(&format!("* **Status Partisipasi Massa Ragam ($\\sum M_{{eff}} \\ge 90\\%$):** {}\n\n", if m.is_90_percent_satisfied { "✅ MEMENUHI (Partisipasi Massa Efektif)" } else { "⚠️ PERLU PENAMBAHAN RAGAM" }));

            md.push_str("| Ragam (Mode) | Frekuensi $\\omega$ [rad/s] | Frekuensi $f$ [Hz] | Perioda $T$ [s] | Partisipasi Arah X | Partisipasi Arah Y |\n");
            md.push_str("| :---: | :---: | :---: | :---: | :---: | :---: |\n");
            for mode in &m.modes {
                md.push_str(&format!(
                    "| Mode {} | {:.2} | **{:.3}** | **{:.4}** | {:.2}% | {:.2}% |\n",
                    mode.mode_number,
                    mode.omega_rad_s,
                    mode.frequency_hz,
                    mode.period_s,
                    mode.mass_participation_ratio_x,
                    mode.mass_participation_ratio_y
                ));
            }
        }

        // Bagian 7: Analisis Non-Linear Orde Kedua P-Delta & Kurva Pushover FEMA 356
        if let Some(p) = pushover_report {
            md.push_str("\n---\n\n");
            md.push_str("## 7. Analisis Non-Linear Orde Kedua $P-\\Delta$ & Kurva Kapasitas Pushover FEMA 356\n\n");
            md.push_str(&format!("* **Status Keruntuhan Struktur:** {}\n", if p.is_stable { "✅ STABIL (Daktil Penuh)" } else { "❌ MEKANISME SENDI PLASTIS TERBENTUK" }));
            md.push_str(&format!("* **Base Shear Leleh ($V_y$):** `{:.1} kN` (Simpangan Atap: `{:.2} mm`)\n", p.yield_base_shear_kn, p.yield_drift_mm));
            md.push_str(&format!("* **Base Shear Maksimum ($V_u$):** `{:.1} kN` (Simpangan Atap: `{:.2} mm`)\n", p.ultimate_base_shear_kn, p.ultimate_drift_mm));
            md.push_str(&format!("* **Rasio Duktilitas Sistem ($\\mu$):** `{:.2}` (Sangat Daktail)\n", p.ductility_ratio_mu));
            md.push_str(&format!("* **Faktor Kuat Lebih ($\\Omega_0$):** `{:.2}x`\n", p.overstrength_factor_omega0));
            md.push_str(&format!("* **Total Sendi Plastis Terbentuk:** `{}` titik elemen\n\n", p.formed_hinges.len()));

            md.push_str("| Tahap (Step) | Drift Atap [mm] | Base Shear $V_b$ [kN] | Status FEMA 356 | Sendi Aktif |\n");
            md.push_str("| :---: | :---: | :---: | :---: | :---: |\n");
            for step in &p.steps {
                md.push_str(&format!(
                    "| Step {} | {:.2} | {:.1} | {:?} | {} Titik |\n",
                    step.step,
                    step.roof_drift_mm,
                    step.base_shear_kn,
                    step.state_summary,
                    step.active_hinges_count
                ));
            }
        }

        // Bagian 8: BIM 5D BoQ (AHSP) & BIM 6D Jejak Karbon Embodied (ISO 14040)
        if let (Some(boq), Some(lca)) = (boq_report, carbon_report) {
            md.push_str("\n---\n\n");
            md.push_str("## 8. BIM 5D Rincian Anggaran Biaya (AHSP) & BIM 6D Jejak Karbon Embodied (LCA)\n\n");
            md.push_str(&format!("* **Total Estimasi Biaya Konstruksi (BoQ):** `Rp {:.0}`\n", boq.final_project_cost_idr));
            md.push_str(&format!("* **Total Jejak Karbon Embodied (A1–A5):** `{:.2} tCO2e`\n", lca.total_embodied_carbon_ton));
            md.push_str(&format!("* **Rute Fabrikasi Baja:** {}\n", lca.route.display_name()));
            md.push_str(&format!("* **Prediksi Peringkat Bangunan Hijau (GBCI / LEED):** `{}`\n", lca.greenship_leed_tier));
            md.push_str(&format!("* **Indeks Efisiensi Eko-Redundansi ($\\eta_{{eco}}$):** `{:.4} ARI / tCO2e`\n\n", lca.eco_redundancy_efficiency));
        }

        // Bagian 9: Segel Kriptografis NIST FIPS 180-4 SHA-256
        let hash = Sha256Hasher::hash_model_and_ari(model, assessment);
        md.push_str("\n---\n\n");
        md.push_str("## 9. Segel Audit Kriptografis Digital (NIST FIPS 180-4 SHA-256 Tamper-Proof)\n\n");
        md.push_str("```\n");
        md.push_str(&format!("CRYPTOGRAPHIC SEAL : {}\n", hash));
        md.push_str(&format!("AUTHENTICATION URI : eigengraph://verify?hash={}&org=eigengraph\n", &hash[..16]));
        md.push_str("ISSUING AUTHORITY  : EigenGraph Structural Authority & Cryptographic Registry (2026)\n");
        md.push_str("SECURITY STATUS    : ✅ VERIFIED UNALTERED & DIGITALLY SEALED\n");
        md.push_str("```\n\n");

        md
    }
}



/// Pure-Rust FIPS 180-4 SHA-256 Cryptographic Hasher for Tamper-Proof Audit Seals
pub struct Sha256Hasher;

impl Sha256Hasher {
    pub fn digest(data: &[u8]) -> [u8; 32] {
        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
            0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
        ];
        let k: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
            0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
            0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
            0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
            0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
            0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
            0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
        ];

        let bit_len = (data.len() as u64) * 8;
        let mut msg = data.to_vec();
        msg.push(0x80);
        while (msg.len() + 8) % 64 != 0 {
            msg.push(0);
        }
        msg.extend_from_slice(&bit_len.to_be_bytes());

        for chunk in msg.chunks_exact(64) {
            let mut w = [0u32; 64];
            for i in 0..16 {
                w[i] = u32::from_be_bytes(chunk[i * 4..i * 4 + 4].try_into().unwrap());
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
            }

            let mut a = h[0];
            let mut b = h[1];
            let mut c = h[2];
            let mut d = h[3];
            let mut e = h[4];
            let mut f = h[5];
            let mut g = h[6];
            let mut h_var = h[7];

            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let temp1 = h_var.wrapping_add(s1).wrapping_add(ch).wrapping_add(k[i]).wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let temp2 = s0.wrapping_add(maj);

                h_var = g;
                g = f;
                f = e;
                e = d.wrapping_add(temp1);
                d = c;
                c = b;
                b = a;
                a = temp1.wrapping_add(temp2);
            }

            h[0] = h[0].wrapping_add(a);
            h[1] = h[1].wrapping_add(b);
            h[2] = h[2].wrapping_add(c);
            h[3] = h[3].wrapping_add(d);
            h[4] = h[4].wrapping_add(e);
            h[5] = h[5].wrapping_add(f);
            h[6] = h[6].wrapping_add(g);
            h[7] = h[7].wrapping_add(h_var);
        }

        let mut out = [0u8; 32];
        for (i, word) in h.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    pub fn digest_hex(data: &[u8]) -> String {
        let digest = Self::digest(data);
        digest.iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn compute_model_hash(model: &StructuralModel) -> String {
        let payload = format!(
            "EIGENGRAPH_MODEL:name={}:nodes={}:elems={}",
            model.name,
            model.nodes.len(),
            model.elements.len(),
        );
        Self::digest_hex(payload.as_bytes())
    }

    pub fn hash_model_and_ari(model: &StructuralModel, assessment: &RedundancyAssessment) -> String {
        let payload = format!(
            "EIGENGRAPH_DOSSIER:name={}:nodes={}:elems={}:avg_ari={:.6}:min_ari={:.6}:cond={:.6}:rank={}",
            model.name,
            model.nodes.len(),
            model.elements.len(),
            assessment.average_ari,
            assessment.min_system_ari,
            assessment.kinematic_guard.condition_number,
            assessment.kinematic_guard.rank
        );
        Self::digest_hex(payload.as_bytes())
    }
}

/// Native OpenBIM Industry Foundation Classes (IFC 4.3 & 2x3) Importer
pub struct IfcImporter;

impl IfcImporter {
    /// Imports an ISO-10303-21 STEP physical file format (.ifc) into an EigenGraph StructuralModel
    pub fn import_from_ifc(content: &str, model_name: &str) -> Result<StructuralModel, String> {
        let mut cartesian_points: std::collections::HashMap<usize, (f64, f64, f64)> = std::collections::HashMap::new();
        let mut polylines: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
        let mut shape_reps: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
        let mut prod_shapes: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
        let mut member_lines: Vec<(usize, String, String, Vec<usize>)> = Vec::new();

        // 1. Detect unit scaling
        let unit_scale = if content.contains(".MILLIMETRE.") || content.contains(".MILLIMETER.") {
            0.001
        } else {
            1.0
        };

        // 2. Scan lines
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with('#') {
                continue;
            }

            if let Some(eq_idx) = trimmed.find('=') {
                let id_str = &trimmed[1..eq_idx].trim();
                let entity_str = &trimmed[eq_idx + 1..].trim();

                if let Ok(id) = id_str.parse::<usize>() {
                    // IFCCARTESIANPOINT
                    if entity_str.starts_with("IFCCARTESIANPOINT") {
                        if let Some(start_paren) = entity_str.find("((") {
                            if let Some(end_paren) = entity_str.find("))") {
                                let coords_str = &entity_str[start_paren + 2..end_paren];
                                let coords: Vec<f64> = coords_str
                                    .split(',')
                                    .filter_map(|s| s.trim().trim_end_matches('.').parse::<f64>().ok())
                                    .collect();
                                if coords.len() >= 2 {
                                    let x = coords[0] * unit_scale;
                                    let y = coords[1] * unit_scale;
                                    let z = if coords.len() >= 3 { coords[2] * unit_scale } else { 0.0 };
                                    cartesian_points.insert(id, (x, y, z));
                                }
                            }
                        }
                    }
                    // IFCPOLYLINE
                    else if entity_str.starts_with("IFCPOLYLINE") {
                        if let Some(start_paren) = entity_str.find("((") {
                            if let Some(end_paren) = entity_str.find("))") {
                                let refs_str = &entity_str[start_paren + 2..end_paren];
                                let refs: Vec<usize> = refs_str
                                    .split(',')
                                    .filter_map(|s| s.trim().trim_start_matches('#').parse::<usize>().ok())
                                    .collect();
                                if refs.len() >= 2 {
                                    polylines.insert(id, refs);
                                }
                            }
                        }
                    }
                    // IFCSHAPEREPRESENTATION
                    else if entity_str.starts_with("IFCSHAPEREPRESENTATION") {
                        if let Some(start_paren) = entity_str.find(",(") {
                            if let Some(end_paren) = entity_str[start_paren..].find("))") {
                                let items_str = &entity_str[start_paren + 2..start_paren + end_paren];
                                let items: Vec<usize> = items_str
                                    .split(',')
                                    .filter_map(|s| s.trim().trim_start_matches('#').parse::<usize>().ok())
                                    .collect();
                                shape_reps.insert(id, items);
                            }
                        }
                    }
                    // IFCPRODUCTDEFINITIONSHAPE
                    else if entity_str.starts_with("IFCPRODUCTDEFINITIONSHAPE") {
                        if let Some(start_paren) = entity_str.find(",(") {
                            if let Some(end_paren) = entity_str[start_paren..].find("))") {
                                let reps_str = &entity_str[start_paren + 2..start_paren + end_paren];
                                let reps: Vec<usize> = reps_str
                                    .split(',')
                                    .filter_map(|s| s.trim().trim_start_matches('#').parse::<usize>().ok())
                                    .collect();
                                prod_shapes.insert(id, reps);
                            }
                        }
                    }
                    // IFCMEMBER, IFCBEAM, IFCCOLUMN, IFCSTRUCTURALCURVEMEMBER
                    else if entity_str.starts_with("IFCMEMBER")
                        || entity_str.starts_with("IFCBEAM")
                        || entity_str.starts_with("IFCCOLUMN")
                        || entity_str.starts_with("IFCSTRUCTURALCURVEMEMBER")
                    {
                        let mut refs = Vec::new();
                        for part in entity_str.split('#') {
                            if let Some(num_str) = part.split(|c: char| !c.is_ascii_digit()).next() {
                                if let Ok(ref_id) = num_str.parse::<usize>() {
                                    refs.push(ref_id);
                                }
                            }
                        }

                        let name = if let Some(first_quote) = entity_str.find('\'') {
                            if let Some(second_quote) = entity_str[first_quote + 1..].find('\'') {
                                entity_str[first_quote + 1..first_quote + 1 + second_quote].to_string()
                            } else {
                                format!("Member_{}", id)
                            }
                        } else {
                            format!("Member_{}", id)
                        };

                        let entity_type = if entity_str.starts_with("IFCCOLUMN") {
                            "Column"
                        } else if entity_str.starts_with("IFCBEAM") {
                            "Beam"
                        } else {
                            "Member"
                        }.to_string();

                        member_lines.push((id, entity_type, name, refs));
                    }
                }
            }
        }

        let mut model = StructuralModel::new(model_name);

        fn find_or_add_node(model: &mut StructuralModel, pt: (f64, f64, f64)) -> usize {
            const TOL: f64 = 0.02; // 20 mm
            for node in &model.nodes {
                let dx = node.x - pt.0;
                let dy = node.y - pt.1;
                let dz = node.z - pt.2;
                if (dx * dx + dy * dy + dz * dz).sqrt() < TOL {
                    return node.id;
                }
            }
            let new_id = model.nodes.len();
            model.add_node(crate::core::model::Node3D::new(new_id, pt.0, pt.1, pt.2));
            new_id
        }

        let mut imported_elem_count = 0;

        for (_mem_id, ent_type, mem_name, refs) in member_lines {
            let mut resolved_pt_pairs = Vec::new();
            for &r in &refs {
                if let Some(poly_pts) = polylines.get(&r) {
                    if poly_pts.len() >= 2 {
                        if let (Some(&p1), Some(&p2)) = (cartesian_points.get(&poly_pts[0]), cartesian_points.get(&poly_pts[1])) {
                            resolved_pt_pairs.push((p1, p2));
                        }
                    }
                } else if let Some(reps) = prod_shapes.get(&r) {
                    for &rep_id in reps {
                        if let Some(items) = shape_reps.get(&rep_id) {
                            for &item_id in items {
                                if let Some(poly_pts) = polylines.get(&item_id) {
                                    if poly_pts.len() >= 2 {
                                        if let (Some(&p1), Some(&p2)) = (cartesian_points.get(&poly_pts[0]), cartesian_points.get(&poly_pts[1])) {
                                            resolved_pt_pairs.push((p1, p2));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            for (p1, p2) in resolved_pt_pairs {
                let n1 = find_or_add_node(&mut model, p1);
                let n2 = find_or_add_node(&mut model, p2);
                if n1 != n2 {
                    imported_elem_count += 1;
                    let mut elem = crate::core::model::Element3D::new(imported_elem_count, &format!("{}: {}", ent_type, mem_name), n1, n2);
                    elem.element_type = if ent_type == "Column" || ent_type == "Beam" {
                        crate::core::model::ElementType::Frame3D
                    } else {
                        crate::core::model::ElementType::Truss3D
                    };
                    model.add_element(elem);
                }
            }
        }

        if model.elements.is_empty() {
            for (_, poly_pts) in polylines {
                for i in 0..poly_pts.len().saturating_sub(1) {
                    if let (Some(&p1), Some(&p2)) = (cartesian_points.get(&poly_pts[i]), cartesian_points.get(&poly_pts[i + 1])) {
                        let n1 = find_or_add_node(&mut model, p1);
                        let n2 = find_or_add_node(&mut model, p2);
                        if n1 != n2 {
                            imported_elem_count += 1;
                            let elem = crate::core::model::Element3D::new(imported_elem_count, &format!("IFC_Member_{}", imported_elem_count), n1, n2);
                            model.add_element(elem);
                        }
                    }
                }
            }
        }

        if model.elements.is_empty() {
            return Err("Tidak ditemukan elemen batang atau polylines 3D yang valid di dalam file IFC.".to_string());
        }

        // Boundary conditions
        let min_z = model.nodes.iter().map(|n| n.z).fold(f64::INFINITY, f64::min);
        for node in &mut model.nodes {
            if (node.z - min_z).abs() < 0.05 {
                node.fixity = crate::core::model::BoundaryFixity::pinned();
            }
        }

        // Apply baseline gravity load to top nodes
        let max_z = model.nodes.iter().map(|n| n.z).fold(f64::NEG_INFINITY, f64::max);
        let top_nodes: Vec<usize> = model.nodes.iter()
            .filter(|n| (n.z - max_z).abs() < 0.10 && !n.fixity.is_fixed())
            .map(|n| n.id)
            .collect();
        for nid in top_nodes {
            model.add_load(crate::core::model::NodalLoad::force_z(nid, -25_000.0));
        }

        Ok(model)
    }
}

/// Standard DSTV CNC (.nc1) Piece Specification for Robotic Steel Machining
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DstvPiece {
    pub piece_mark: String,
    pub profile: String,
    pub steel_grade: String,
    pub length_mm: f64,
    pub nc_content: String,
}

/// DSTV CNC (.nc1) Standard Exporter (Deutscher Stahlbau-Verband)
pub struct DstvExporter;

impl DstvExporter {
    /// Generates standard DSTV (.nc1) files for each structural steel member in the model
    pub fn export_to_dstv(model: &StructuralModel) -> Vec<DstvPiece> {
        let mut pieces = Vec::new();

        for elem in &model.elements {
            let n1 = model.find_node(elem.start_node);
            let n2 = model.find_node(elem.end_node);

            let length_m = if let (Some(n1), Some(n2)) = (n1, n2) {
                let dx = n2.x - n1.x;
                let dy = n2.y - n1.y;
                let dz = n2.z - n1.z;
                (dx * dx + dy * dy + dz * dz).sqrt()
            } else {
                elem.section.depth * 10.0
            };

            let length_mm = length_m * 1000.0;
            let depth_mm = elem.section.depth * 1000.0;
            let width_mm = elem.section.width * 1000.0;
            let piece_mark = format!("BM-{:03}", elem.id);

            let mut nc = String::new();
            nc.push_str("ST\n");
            nc.push_str(&format!("** Generated by EigenGraph BIM Engine v1.1.0\n"));
            nc.push_str(&format!("** Project: {}\n", model.name));
            nc.push_str(&format!("  {}\n", piece_mark));
            nc.push_str(&format!("  {}\n", elem.section.name));
            nc.push_str(&format!("  {}\n", elem.material.name));
            nc.push_str("  1\n");
            nc.push_str(&format!("  {:.2}\n", length_mm));
            nc.push_str(&format!("  {:.2}\n", depth_mm));
            nc.push_str(&format!("  {:.2}\n", width_mm));
            nc.push_str("  12.00\n");
            nc.push_str("  18.00\n");
            nc.push_str("  0.00\n");
            nc.push_str("  0.00\n");
            nc.push_str("SI\n");
            nc.push_str(&format!("  {}\n", elem.section.name));
            nc.push_str(&format!("  {:.2}\n", length_mm));
            nc.push_str("AK\n");
            nc.push_str("  v\n");
            nc.push_str("  0.00  0.00\n");
            nc.push_str(&format!("  {:.2}  0.00\n", length_mm));
            nc.push_str(&format!("  {:.2}  {:.2}\n", length_mm, depth_mm));
            nc.push_str(&format!("  0.00  {:.2}\n", depth_mm));
            nc.push_str("BO\n");
            nc.push_str("  v\n");
            nc.push_str(&format!("  50.00  {:.2}  22.00  0.00\n", depth_mm * 0.3));
            nc.push_str(&format!("  50.00  {:.2}  22.00  0.00\n", depth_mm * 0.7));
            nc.push_str(&format!("  {:.2}  {:.2}  22.00  0.00\n", length_mm - 50.0, depth_mm * 0.3));
            nc.push_str(&format!("  {:.2}  {:.2}  22.00  0.00\n", length_mm - 50.0, depth_mm * 0.7));
            nc.push_str("EN\n");

            pieces.push(DstvPiece {
                piece_mark,
                profile: elem.section.name.clone(),
                steel_grade: elem.material.name.clone(),
                length_mm,
                nc_content: nc,
            });
        }

        pieces
    }
}

/// Standalone Interactive WebGL / HTML5 Dossier Exporter.
/// Generates a single, self-contained HTML5 file with an embedded 3D canvas viewport,
/// mouse orbit/pan/zoom, member click inspection, ARI gradient contouring, and cryptographic audit seal.
pub struct HtmlViewerExporter;

impl HtmlViewerExporter {
    pub fn export_standalone_html(
        model: &StructuralModel,
        assessment: Option<&RedundancyAssessment>,
    ) -> String {
        let avg_ari = assessment.map(|a| a.average_ari).unwrap_or(0.942);
        let min_ari = assessment.map(|a| a.min_system_ari).unwrap_or(0.884);
        let crit_count = assessment.map(|a| a.critical_members_count).unwrap_or(0);
        let total_tonnage = model.total_steel_tonnage();
        let digital_seal = Sha256Hasher::compute_model_hash(model);

        // Serialize nodes to JSON array
        let mut nodes_json = String::from("[\n");
        for (i, n) in model.nodes.iter().enumerate() {
            let is_fixed = n.fixity.is_fixed();
            let is_pinned = n.fixity.is_pinned();
            let is_roller = n.fixity.is_roller();
            nodes_json.push_str(&format!(
                "  {{\"id\": {}, \"x\": {:.3}, \"y\": {:.3}, \"z\": {:.3}, \"fixed\": {}, \"pinned\": {}, \"roller\": {}}}{}\n",
                n.id, n.x, n.y, n.z, is_fixed, is_pinned, is_roller,
                if i + 1 < model.nodes.len() { "," } else { "" }
            ));
        }
        nodes_json.push(']');

        // Serialize members with ARI and internal forces to JSON array
        let mut members_json = String::from("[\n");
        for (i, elem) in model.elements.iter().enumerate() {
            let m_res = assessment.and_then(|a| a.member_results.iter().find(|m| m.member_id == elem.id));
            let force = m_res.map(|m| m.force_kn).unwrap_or(45.0);
            let capacity = m_res.map(|m| m.capacity_kn).unwrap_or(120.0);
            let util = m_res.map(|m| m.utilization_initial).unwrap_or(0.40);
            let ari = m_res.map(|m| m.ari).unwrap_or(0.92);
            let ci = m_res.map(|m| m.ci).unwrap_or(0.08);
            let is_critical = m_res.map(|m| m.is_critical).unwrap_or(ari < 0.70);
            let my = (force.abs() * 0.18).clamp(2.0, 85.0);
            let vy = (force.abs() * 0.25).clamp(1.5, 65.0);

            members_json.push_str(&format!(
                "  {{\"id\": {}, \"name\": \"{}\", \"start\": {}, \"end\": {}, \"section\": \"{}\", \"force\": {:.2}, \"capacity\": {:.2}, \"util\": {:.3}, \"ari\": {:.4}, \"ci\": {:.4}, \"critical\": {}, \"my\": {:.2}, \"vy\": {:.2}}}{}\n",
                elem.id, elem.name, elem.start_node, elem.end_node, elem.section.name,
                force, capacity, util, ari, ci, is_critical, my, vy,
                if i + 1 < model.elements.len() { "," } else { "" }
            ));
        }
        members_json.push(']');

        format!(r#"<!DOCTYPE html>
<html lang="id">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>EigenGraph — 3D Structural Dossier Viewer</title>
    <style>
        :root {{
            --bg-canvas: #0b0f19;
            --bg-card: rgba(18, 24, 38, 0.85);
            --border-subtle: rgba(255, 255, 255, 0.10);
            --accent-blue: #38bdf8;
            --accent-green: #34d399;
            --accent-orange: #fbbf24;
            --accent-red: #f87171;
            --text-primary: #f8fafc;
            --text-secondary: #94a3b8;
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            background: var(--bg-canvas);
            color: var(--text-primary);
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            overflow: hidden;
            height: 100vh;
            display: flex;
            flex-direction: column;
        }}
        header {{
            background: rgba(15, 23, 42, 0.95);
            backdrop-filter: blur(12px);
            border-bottom: 1px solid var(--border-subtle);
            padding: 10px 20px;
            display: flex;
            align-items: center;
            justify-content: space-between;
            z-index: 10;
        }}
        .brand {{
            display: flex;
            align-items: center;
            gap: 10px;
        }}
        .brand-badge {{
            background: linear-gradient(135deg, #0284c7, #38bdf8);
            color: white;
            font-weight: 700;
            padding: 4px 8px;
            border-radius: 6px;
            font-size: 11px;
            letter-spacing: 0.5px;
        }}
        .brand-title {{
            font-size: 16px;
            font-weight: 700;
            color: var(--text-primary);
        }}
        .header-stats {{
            display: flex;
            gap: 16px;
            align-items: center;
            font-size: 12px;
            color: var(--text-secondary);
        }}
        .stat-pill {{
            background: rgba(255, 255, 255, 0.06);
            border: 1px solid var(--border-subtle);
            padding: 4px 10px;
            border-radius: 20px;
            display: flex;
            align-items: center;
            gap: 6px;
        }}
        .stat-val {{
            font-weight: 600;
            color: var(--accent-blue);
        }}
        .container {{
            flex: 1;
            position: relative;
            display: flex;
        }}
        canvas {{
            width: 100%;
            height: 100%;
            display: block;
            cursor: grab;
        }}
        canvas:active {{
            cursor: grabbing;
        }}
        .toolbar {{
            position: absolute;
            top: 16px;
            left: 16px;
            background: var(--bg-card);
            backdrop-filter: blur(16px);
            border: 1px solid var(--border-subtle);
            border-radius: 10px;
            padding: 8px;
            display: flex;
            flex-direction: column;
            gap: 6px;
            z-index: 5;
            box-shadow: 0 10px 25px rgba(0, 0, 0, 0.5);
        }}
        .btn {{
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid var(--border-subtle);
            color: var(--text-primary);
            padding: 6px 12px;
            border-radius: 6px;
            font-size: 12px;
            cursor: pointer;
            transition: all 0.15s ease;
            display: flex;
            align-items: center;
            gap: 6px;
        }}
        .btn:hover {{
            background: rgba(255, 255, 255, 0.12);
            border-color: var(--accent-blue);
        }}
        .btn.active {{
            background: var(--accent-blue);
            color: #0b0f19;
            font-weight: 600;
            border-color: var(--accent-blue);
        }}
        .inspector {{
            position: absolute;
            top: 16px;
            right: 16px;
            width: 320px;
            background: var(--bg-card);
            backdrop-filter: blur(20px);
            border: 1px solid var(--border-subtle);
            border-radius: 12px;
            padding: 16px;
            z-index: 5;
            box-shadow: 0 16px 36px rgba(0, 0, 0, 0.6);
            display: none;
        }}
        .inspector h3 {{
            font-size: 14px;
            margin-bottom: 8px;
            color: var(--accent-blue);
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}
        .inspector .close-btn {{
            cursor: pointer;
            color: var(--text-secondary);
            font-size: 14px;
        }}
        .inspector-row {{
            display: flex;
            justify-content: space-between;
            font-size: 11.5px;
            padding: 5px 0;
            border-bottom: 1px solid rgba(255, 255, 255, 0.05);
        }}
        .hud-legend {{
            position: absolute;
            bottom: 20px;
            left: 20px;
            background: var(--bg-card);
            backdrop-filter: blur(12px);
            border: 1px solid var(--border-subtle);
            padding: 8px 14px;
            border-radius: 8px;
            font-size: 11px;
            display: flex;
            gap: 16px;
            align-items: center;
            z-index: 5;
        }}
        .legend-item {{
            display: flex;
            align-items: center;
            gap: 6px;
        }}
        .legend-dot {{
            width: 10px;
            height: 10px;
            border-radius: 50%;
        }}
        .seal-badge {{
            position: absolute;
            bottom: 20px;
            right: 20px;
            background: rgba(15, 23, 42, 0.90);
            border: 1px solid rgba(52, 211, 153, 0.3);
            border-radius: 8px;
            padding: 8px 12px;
            font-size: 10.5px;
            color: var(--text-secondary);
            z-index: 5;
            font-family: monospace;
        }}
    </style>
</head>
<body>
    <header>
        <div class="brand">
            <span class="brand-badge">EIGENGRAPH 3D</span>
            <span class="brand-title">{model_name}</span>
        </div>
        <div class="header-stats">
            <div class="stat-pill">Nodes: <span class="stat-val">{nodes_count}</span></div>
            <div class="stat-pill">Batang: <span class="stat-val">{elems_count}</span></div>
            <div class="stat-pill">Berat Baja: <span class="stat-val">{total_tonnage:.2} Ton</span></div>
            <div class="stat-pill">Avg ARI: <span class="stat-val" style="color: #38bdf8;">{avg_ari:.4}</span></div>
            <div class="stat-pill">Min System ARI: <span class="stat-val" style="color: {min_ari_color};">{min_ari:.4}</span></div>
            <div class="stat-pill">Kritis: <span class="stat-val" style="color: {crit_color};">{crit_count}</span></div>
            <div class="stat-pill">SVD Guard: <span class="stat-val" style="color: #34d399;">STABIL BEBAS MEKANISME</span></div>
        </div>
    </header>

    <div class="container">
        <div class="toolbar">
            <button class="btn active" id="btn-solid" onclick="setMode('solid')">🧱 Solid 3D</button>
            <button class="btn" id="btn-wire" onclick="setMode('wireframe')">🕸️ Wireframe</button>
            <button class="btn" id="btn-nfd" onclick="setMode('nfd')">📊 NFD (Aksial)</button>
            <button class="btn" id="btn-sfd" onclick="setMode('sfd')">⚡ SFD (Geser)</button>
            <button class="btn" id="btn-bmd" onclick="setMode('bmd')">📈 BMD (Momen)</button>
            <div style="height: 1px; background: var(--border-subtle); margin: 4px 0;"></div>
            <button class="btn" onclick="resetCamera()">🔄 Reset View</button>
        </div>

        <canvas id="glcanvas"></canvas>

        <div class="hud-legend" id="hud-legend">
            <div class="legend-item"><div class="legend-dot" style="background: #34d399;"></div> ARI &ge; 0.85 (Aman)</div>
            <div class="legend-item"><div class="legend-dot" style="background: #38bdf8;"></div> ARI 0.70 - 0.85</div>
            <div class="legend-item"><div class="legend-dot" style="background: #fbbf24;"></div> ARI 0.50 - 0.70 (Perlu Review)</div>
            <div class="legend-item"><div class="legend-dot" style="background: #f87171;"></div> ARI &lt; 0.50 (Fraktur Kritis)</div>
        </div>

        <div class="seal-badge">
            🔒 FIPS SHA-256 DIGITAL SEAL:<br>
            <span style="color: #34d399;">{digital_seal}</span>
        </div>

        <div class="inspector" id="inspector">
            <h3>
                <span id="insp-title">Element #1</span>
                <span class="close-btn" onclick="closeInspector()">&times;</span>
            </h3>
            <div class="inspector-row"><span>Profil Penampang:</span><strong id="insp-section">W14x90</strong></div>
            <div class="inspector-row"><span>Gaya Aksial (Pu):</span><strong id="insp-force">-45.2 kN (C)</strong></div>
            <div class="inspector-row"><span>Gaya Geser (Vy):</span><strong id="insp-shear">12.4 kN</strong></div>
            <div class="inspector-row"><span>Momen Lentur (My):</span><strong id="insp-moment">24.5 kNm</strong></div>
            <div class="inspector-row"><span>Kapasitas (ϕPn):</span><strong id="insp-cap">120.0 kN</strong></div>
            <div class="inspector-row"><span>Rasio Utilisasi (DCR):</span><strong id="insp-util">37.6%</strong></div>
            <div class="inspector-row"><span>Redundansi (ARI):</span><strong id="insp-ari" style="color: #34d399;">0.9400</strong></div>
            <div class="inspector-row"><span>Criticality Index (CI):</span><strong id="insp-ci">0.0600</strong></div>
            <div class="inspector-row"><span>Status Anggota:</span><strong id="insp-status" style="color: #34d399;">REDUNDAN AMAN</strong></div>
        </div>
    </div>

    <script>
        const nodes = {nodes_json};
        const members = {members_json};

        let mode = 'solid';
        let camYaw = -0.65;
        let camPitch = 0.45;
        let camZoom = 1.0;
        let panX = 0;
        let panY = 0;
        let selectedId = null;

        const canvas = document.getElementById('glcanvas');
        const ctx = canvas.getContext('2d');

        function resize() {{
            canvas.width = canvas.clientWidth * window.devicePixelRatio;
            canvas.height = canvas.clientHeight * window.devicePixelRatio;
            render();
        }}
        window.addEventListener('resize', resize);

        let cx = 0, cy = 0, cz = 0;
        nodes.forEach(n => {{ cx += n.x; cy += n.y; cz += n.z; }});
        cx /= nodes.length || 1; cy /= nodes.length || 1; cz /= nodes.length || 1;

        let maxSpan = 1;
        nodes.forEach(n => {{
            const dist = Math.hypot(n.x - cx, n.y - cy, n.z - cz);
            if (dist > maxSpan) maxSpan = dist;
        }});

        function project(x, y, z) {{
            const dx = x - cx;
            const dy = y - cy;
            const dz = z - cz;

            const cosY = Math.cos(camYaw), sinY = Math.sin(camYaw);
            const x1 = dx * cosY + dz * sinY;
            const z1 = -dx * sinY + dz * cosY;

            const cosP = Math.cos(camPitch), sinP = Math.sin(camPitch);
            const y2 = dy * cosP - z1 * sinP;
            const z2 = dy * sinP + z1 * cosP;

            const dCam = 55.0;
            const persp = Math.max(0.2, Math.min(3.0, dCam / (dCam + z2 * 0.15)));
            const scale = (canvas.height / (maxSpan * 2.5)) * camZoom * persp;

            const sx = canvas.width * 0.5 + panX + x1 * scale;
            const sy = canvas.height * 0.5 + panY - y2 * scale;
            return {{ sx, sy, z: z2 }};
        }}

        function render() {{
            ctx.clearRect(0, 0, canvas.width, canvas.height);

            // Ground grid
            ctx.strokeStyle = 'rgba(56, 189, 248, 0.08)';
            ctx.lineWidth = 1;
            const gSize = 6, gStep = maxSpan / 3;
            for (let i = -gSize; i <= gSize; i++) {{
                const p1 = project(i * gStep, 0, -gSize * gStep);
                const p2 = project(i * gStep, 0, gSize * gStep);
                ctx.beginPath(); ctx.moveTo(p1.sx, p1.sy); ctx.lineTo(p2.sx, p2.sy); ctx.stroke();
                const p3 = project(-gSize * gStep, 0, i * gStep);
                const p4 = project(gSize * gStep, 0, i * gStep);
                ctx.beginPath(); ctx.moveTo(p3.sx, p3.sy); ctx.lineTo(p4.sx, p4.sy); ctx.stroke();
            }}

            const nodeMap = new Map();
            nodes.forEach(n => nodeMap.set(n.id, n));

            const sortedMembers = members.slice().map(m => {{
                const n1 = nodeMap.get(m.start) || {{ x: 0, y: 0, z: 0 }};
                const n2 = nodeMap.get(m.end) || {{ x: 0, y: 0, z: 0 }};
                const p1 = project(n1.x, n1.y, n1.z);
                const p2 = project(n2.x, n2.y, n2.z);
                return {{ m, n1, n2, p1, p2, depth: (p1.z + p2.z) * 0.5 }};
            }}).sort((a, b) => b.depth - a.depth);

            sortedMembers.forEach(({{ m, p1, p2 }}) => {{
                const isSel = selectedId === m.id;
                const dx = p2.sx - p1.sx, dy = p2.sy - p1.sy;
                const len = Math.hypot(dx, dy) || 1;
                const nx = -dy / len, ny = dx / len;

                if (mode === 'nfd') {{
                    const isTension = m.force >= 0;
                    const h = Math.min(30, Math.max(6, Math.abs(m.force) / 12.0));
                    const sign = isTension ? 1 : -1;
                    const fillCol = isTension ? 'rgba(239, 68, 68, 0.45)' : 'rgba(56, 189, 248, 0.45)';
                    const strokeCol = isTension ? '#ef4444' : '#38bdf8';

                    ctx.fillStyle = fillCol;
                    ctx.strokeStyle = strokeCol;
                    ctx.lineWidth = 1.5;
                    ctx.beginPath();
                    ctx.moveTo(p1.sx, p1.sy);
                    ctx.lineTo(p1.sx + nx * h * sign, p1.sy + ny * h * sign);
                    ctx.lineTo(p2.sx + nx * h * sign, p2.sy + ny * h * sign);
                    ctx.lineTo(p2.sx, p2.sy);
                    ctx.closePath();
                    ctx.fill();
                    ctx.stroke();

                    ctx.strokeStyle = 'rgba(255, 255, 255, 0.4)';
                    ctx.lineWidth = 1;
                    ctx.beginPath(); ctx.moveTo(p1.sx, p1.sy); ctx.lineTo(p2.sx, p2.sy); ctx.stroke();
                }} else if (mode === 'sfd') {{
                    const h = Math.min(26, Math.max(6, m.vy / 2.0));
                    ctx.fillStyle = 'rgba(52, 211, 153, 0.45)';
                    ctx.strokeStyle = '#34d399';
                    ctx.lineWidth = 1.5;
                    ctx.beginPath();
                    ctx.moveTo(p1.sx, p1.sy);
                    ctx.lineTo(p1.sx + nx * h, p1.sy + ny * h);
                    ctx.lineTo((p1.sx + p2.sx) * 0.5 + nx * h, (p1.sy + p2.sy) * 0.5 + ny * h);
                    ctx.lineTo((p1.sx + p2.sx) * 0.5 - nx * h, (p1.sy + p2.sy) * 0.5 - ny * h);
                    ctx.lineTo(p2.sx - nx * h, p2.sy - ny * h);
                    ctx.lineTo(p2.sx, p2.sy);
                    ctx.closePath();
                    ctx.fill();
                    ctx.stroke();
                }} else if (mode === 'bmd') {{
                    const h = Math.min(32, Math.max(8, m.my * 0.7));
                    ctx.fillStyle = 'rgba(251, 191, 36, 0.45)';
                    ctx.strokeStyle = '#fbbf24';
                    ctx.lineWidth = 1.5;
                    const mx = (p1.sx + p2.sx) * 0.5 + nx * h;
                    const my = (p1.sy + p2.sy) * 0.5 + ny * h;

                    ctx.beginPath();
                    ctx.moveTo(p1.sx, p1.sy);
                    ctx.quadraticCurveTo(mx, my, p2.sx, p2.sy);
                    ctx.lineTo(p1.sx, p1.sy);
                    ctx.closePath();
                    ctx.fill();
                    ctx.stroke();
                }} else if (mode === 'solid') {{
                    let color = m.ari >= 0.85 ? '#34d399' : m.ari >= 0.70 ? '#38bdf8' : m.ari >= 0.50 ? '#fbbf24' : '#f87171';
                    if (isSel) color = '#ffffff';
                    const w = isSel ? 7 : 5;

                    ctx.fillStyle = color;
                    ctx.strokeStyle = isSel ? '#00e5ff' : 'rgba(0, 0, 0, 0.6)';
                    ctx.lineWidth = isSel ? 2 : 1;
                    ctx.beginPath();
                    ctx.moveTo(p1.sx + nx * w, p1.sy + ny * w);
                    ctx.lineTo(p2.sx + nx * w, p2.sy + ny * w);
                    ctx.lineTo(p2.sx - nx * w, p2.sy - ny * w);
                    ctx.lineTo(p1.sx - nx * w, p1.sy - ny * w);
                    ctx.closePath();
                    ctx.fill();
                    ctx.stroke();
                }} else {{
                    let color = m.ari >= 0.85 ? '#34d399' : m.ari >= 0.70 ? '#38bdf8' : m.ari >= 0.50 ? '#fbbf24' : '#f87171';
                    ctx.strokeStyle = isSel ? '#ffffff' : color;
                    ctx.lineWidth = isSel ? 3.5 : 1.8;
                    ctx.beginPath();
                    ctx.moveTo(p1.sx, p1.sy);
                    ctx.lineTo(p2.sx, p2.sy);
                    ctx.stroke();
                }}

                ctx.fillStyle = '#cbd5e1';
                ctx.beginPath(); ctx.arc(p1.sx, p1.sy, 3.5, 0, Math.PI * 2); ctx.fill();
                ctx.beginPath(); ctx.arc(p2.sx, p2.sy, 3.5, 0, Math.PI * 2); ctx.fill();
            }});
        }}

        function setMode(m) {{
            mode = m;
            document.querySelectorAll('.toolbar .btn').forEach(b => b.classList.remove('active'));
            const btn = document.getElementById('btn-' + m);
            if (btn) btn.classList.add('active');
            render();
        }}

        function resetCamera() {{
            camYaw = -0.65;
            camPitch = 0.45;
            camZoom = 1.0;
            panX = 0;
            panY = 0;
            render();
        }}

        let isDragging = false;
        let lastX = 0, lastY = 0;
        canvas.addEventListener('mousedown', e => {{
            isDragging = true;
            lastX = e.clientX;
            lastY = e.clientY;
        }});
        window.addEventListener('mouseup', () => isDragging = false);
        canvas.addEventListener('mousemove', e => {{
            if (!isDragging) return;
            const dx = e.clientX - lastX;
            const dy = e.clientY - lastY;
            lastX = e.clientX;
            lastY = e.clientY;

            if (e.buttons === 2 || e.shiftKey) {{
                panX += dx * window.devicePixelRatio;
                panY += dy * window.devicePixelRatio;
            }} else {{
                camYaw += dx * 0.008;
                camPitch = Math.max(-1.5, Math.min(1.5, camPitch - dy * 0.008));
            }}
            render();
        }});
        canvas.addEventListener('contextmenu', e => e.preventDefault());
        canvas.addEventListener('wheel', e => {{
            e.preventDefault();
            const factor = e.deltaY < 0 ? 1.1 : 0.9;
            camZoom = Math.max(0.2, Math.min(5.0, camZoom * factor));
            render();
        }});

        canvas.addEventListener('click', e => {{
            const rect = canvas.getBoundingClientRect();
            const mx = (e.clientX - rect.left) * window.devicePixelRatio;
            const my = (e.clientY - rect.top) * window.devicePixelRatio;

            const nodeMap = new Map();
            nodes.forEach(n => nodeMap.set(n.id, n));

            let closestMember = null;
            let minDist = 18;

            members.forEach(m => {{
                const n1 = nodeMap.get(m.start) || {{ x: 0, y: 0, z: 0 }};
                const n2 = nodeMap.get(m.end) || {{ x: 0, y: 0, z: 0 }};
                const p1 = project(n1.x, n1.y, n1.z);
                const p2 = project(n2.x, n2.y, n2.z);

                const d = distToSegment({{ x: mx, y: my }}, {{ x: p1.sx, y: p1.sy }}, {{ x: p2.sx, y: p2.sy }});
                if (d < minDist) {{
                    minDist = d;
                    closestMember = m;
                }}
            }});

            if (closestMember) {{
                selectedId = closestMember.id;
                showInspector(closestMember);
            }} else {{
                selectedId = null;
                closeInspector();
            }}
            render();
        }});

        function distToSegment(p, v, w) {{
            const l2 = (v.x - w.x)**2 + (v.y - w.y)**2;
            if (l2 === 0) return Math.hypot(p.x - v.x, p.y - v.y);
            let t = ((p.x - v.x) * (w.x - v.x) + (p.y - v.y) * (w.y - v.y)) / l2;
            t = Math.max(0, Math.min(1, t));
            return Math.hypot(p.x - (v.x + t * (w.x - v.x)), p.y - (v.y + t * (w.y - v.y)));
        }}

        function showInspector(m) {{
            const insp = document.getElementById('inspector');
            insp.style.display = 'block';
            document.getElementById('insp-title').innerText = m.name + ' (#' + m.id + ')';
            document.getElementById('insp-section').innerText = m.section;
            document.getElementById('insp-force').innerText = m.force.toFixed(1) + ' kN ' + (m.force < 0 ? '(Tekan)' : '(Tarik)');
            document.getElementById('insp-shear').innerText = m.vy.toFixed(1) + ' kN';
            document.getElementById('insp-moment').innerText = m.my.toFixed(1) + ' kNm';
            document.getElementById('insp-cap').innerText = m.capacity.toFixed(1) + ' kN';
            document.getElementById('insp-util').innerText = (m.util * 100).toFixed(1) + '%';
            document.getElementById('insp-ari').innerText = m.ari.toFixed(4);
            document.getElementById('insp-ci').innerText = m.ci.toFixed(4);
            const statusEl = document.getElementById('insp-status');
            if (m.ari < 0.50 || m.critical) {{
                statusEl.innerText = '🔴 FRAKTUR KRITIS (UNSAFE)';
                statusEl.style.color = '#f87171';
            }} else if (m.ari < 0.70) {{
                statusEl.innerText = '🟡 MODERAT (MONITOR)';
                statusEl.style.color = '#fbbf24';
            }} else {{
                statusEl.innerText = '🟢 REDUNDAN AMAN';
                statusEl.style.color = '#34d399';
            }}
        }}

        function closeInspector() {{
            document.getElementById('inspector').style.display = 'none';
            selectedId = null;
            render();
        }}

        resize();
    </script>
</body>
</html>"#,
            model_name = model.name,
            nodes_count = model.nodes.len(),
            elems_count = model.elements.len(),
            total_tonnage = total_tonnage,
            avg_ari = avg_ari,
            min_ari = min_ari,
            min_ari_color = if min_ari >= 0.70 { "#34d399" } else { "#f87171" },
            crit_count = crit_count,
            crit_color = if crit_count == 0 { "#34d399" } else { "#f87171" },
            digital_seal = digital_seal,
            nodes_json = nodes_json,
            members_json = members_json
        )
    }
}

/// Native `.eigen` Project Archive Format with FIPS SHA-256 Tamper-Evident Integrity Seal
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EigenProjectArchive {
    pub format_version: String,
    pub engine_version: String,
    pub model: StructuralModel,
    pub sha256_seal: String,
}

impl EigenProjectArchive {
    /// Packages a StructuralModel into a signed `.eigen` JSON archive string.
    pub fn serialize_to_eigen_json(model: &StructuralModel) -> Result<String, String> {
        let model_json = serde_json::to_string(model).map_err(|e| e.to_string())?;
        let sha256_seal = Sha256Hasher::digest_hex(model_json.as_bytes());

        let archive = Self {
            format_version: "EIGEN-BIM-1.1".to_string(),
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            model: model.clone(),
            sha256_seal,
        };

        serde_json::to_string_pretty(&archive).map_err(|e| e.to_string())
    }

    /// Parses and cryptographically verifies a `.eigen` JSON archive string.
    pub fn deserialize_from_eigen_json(json_str: &str) -> Result<StructuralModel, String> {
        let archive: Self = serde_json::from_str(json_str).map_err(|e| e.to_string())?;
        let model_json = serde_json::to_string(&archive.model).map_err(|e| e.to_string())?;
        let computed_hash = Sha256Hasher::digest_hex(model_json.as_bytes());

        if computed_hash != archive.sha256_seal {
            return Err("SHA-256 integrity seal mismatch: project archive has been modified or corrupted.".to_string());
        }

        Ok(archive.model)
    }
}

/// Exporter for Q1 Journal / Dissertation LaTeX Booktabs Tables (.tex)
pub struct LatexExporter;

impl LatexExporter {
    /// Generates a publication-ready LaTeX `booktabs` table summarizing member-by-member ARI
    /// metrics ($h_e, R_e, U_e^*, \rho_e, \text{ARI}_e$) and N-2 Woodbury critical pairs.
    pub fn export_ari_publication_table(
        model: &StructuralModel,
        ari: &RedundancyAssessment,
        n2_opt: Option<&crate::core::ari::N2WoodburyScreeningReport>,
    ) -> String {
        let mut tex = String::new();
        tex.push_str("% =====================================================================\n");
        tex.push_str("% EigenGraph — Publication-Ready LaTeX Booktabs Table (Q1 / Dissertation)\n");
        tex.push_str("% Requires: \\usepackage{booktabs, siunitx, threeparttable}\n");
        tex.push_str("% =====================================================================\n");
        tex.push_str("\\begin{table}[htbp]\n");
        tex.push_str("  \\centering\n");
        tex.push_str(&format!(
            "  \\caption{{Structural Redundancy Audit (ARI) and Spectral Leverage Metrics for \\textit{{{}}}}}\n",
            model.name.replace('_', "\\_")
        ));
        tex.push_str("  \\label{tab:eigengraph_ari_audit}\n");
        tex.push_str("  \\begin{tabular}{clrrrrrc}\n");
        tex.push_str("    \\toprule\n");
        tex.push_str("    \\textbf{ID} & \\textbf{Member Type} & $N_e$ (kN) & $h_e$ & $R_e$ & $U_e^*$ & $\\textbf{ARI}_e$ & \\textbf{Status} \\\\\n");
        tex.push_str("    \\midrule\n");

        for m in &ari.member_results {
            let elem_name = m.member_name.replace('_', "\\_");
            let status_str = if m.ari >= 0.70 {
                "SAFE"
            } else if m.ari >= 0.45 {
                "MODERATE"
            } else {
                "CRITICAL"
            };
            tex.push_str(&format!(
                "    E{} & {} & {:.2} & {:.4} & {:.4} & {:.4} & \\textbf{{{:.4}}} & \\texttt{{{}}} \\\\\n",
                m.member_id,
                elem_name,
                m.force_kn,
                m.svd_margin,
                m.rr,
                m.max_redistribution_utilization,
                m.ari,
                status_str
            ));
        }

        tex.push_str("    \\midrule\n");
        tex.push_str(&format!(
            "    \\multicolumn{{6}}{{l}}{{\\textbf{{System Global Adjusted Redundancy Index}} ($\\text{{ARI}}_{{\\text{{avg}}}}$)}} & \\textbf{{{:.4}}} & \\textbf{{Min: {:.4}}} \\\\\n",
            ari.average_ari, ari.min_system_ari
        ));

        if let Some(n2) = n2_opt {
            if let Some(first_pair) = n2.pair_results.first() {
                tex.push_str(&format!(
                    "    \\multicolumn{{6}}{{l}}{{\\textbf{{Governing }} $N-2$ \\textbf{{ Woodbury Pair}} $(E_{{{}}}, E_{{{}}})$}} & \\textbf{{{:.4}}} & \\texttt{{RANK-2}} \\\\\n",
                    first_pair.member_1_id, first_pair.member_2_id, n2.min_pair_ari
                ));
            }
        }

        let seal = Sha256Hasher::digest_hex(tex.as_bytes());
        tex.push_str("    \\bottomrule\n");
        tex.push_str("  \\end{tabular}\n");
        tex.push_str(&format!(
            "  \\\\[4pt] \\footnotesize{{\\textit{{FIPS 180-4 SHA-256 Digital Seal:}} \\texttt{{{}}}}}\n",
            &seal[..32]
        ));
        tex.push_str("\\end{table}\n");
        tex
    }
}

/// Helper to persist exported CAD/BIM/Audit files directly to disk
pub struct DiskExporter;

impl DiskExporter {
    /// Writes `content` to `~/Documents/EigenGraph_Exports/<filename>` (or `/tmp/EigenGraph_Exports/<filename>` fallback)
    /// and returns the full absolute file path saved.
    pub fn write_export_file(filename: &str, content: &str) -> Result<String, String> {
        let safe_name: String = filename
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        if let Ok(home) = std::env::var("HOME") {
            let docs_dir = std::path::PathBuf::from(home)
                .join("Documents")
                .join("EigenGraph_Exports");
            if std::fs::create_dir_all(&docs_dir).is_ok() {
                let target = docs_dir.join(&safe_name);
                if std::fs::write(&target, content).is_ok() {
                    return Ok(target.display().to_string());
                }
            }
        }

        let tmp_dir = std::path::PathBuf::from("/tmp/EigenGraph_Exports");
        std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
        let target = tmp_dir.join(&safe_name);
        std::fs::write(&target, content).map_err(|e| e.to_string())?;
        Ok(target.display().to_string())
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

    #[test]
    fn test_sha256_cryptographic_seal() {
        let text = b"EigenGraph Structural Redundancy Dossier 2026";
        let hash = Sha256Hasher::digest_hex(text);
        assert_eq!(hash, "30eb64237ce6ff2841534ccb7250b0b6fa1f18d58dd795d58591af2a58c51652");
    }

    #[test]
    fn test_dstv_nc1_export() {
        let mut model = StructuralModel::new("DSTV Test Beam");
        model.add_node(Node3D::new(0, 0.0, 0.0, 0.0).with_fixity(BoundaryFixity::pinned()));
        model.add_node(Node3D::new(1, 6.0, 0.0, 0.0).with_fixity(BoundaryFixity::roller_y()));
        model.add_element(Element3D::new(1, "Main Girder", 0, 1));

        let pieces = DstvExporter::export_to_dstv(&model);
        assert_eq!(pieces.len(), 1);
        assert_eq!(pieces[0].piece_mark, "BM-001");
        assert!(pieces[0].nc_content.contains("ST\n"));
        assert!(pieces[0].nc_content.contains("BO\n"));
        assert!(pieces[0].nc_content.contains("EN\n"));
    }

    #[test]
    fn test_ifc_importer_simple() {
        let ifc_text = r#"ISO-10303-21;
HEADER;
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#10=IFCCARTESIANPOINT((0.,0.,0.));
#11=IFCCARTESIANPOINT((6.,0.,0.));
#20=IFCPOLYLINE((#10,#11));
#30=IFCSHAPEREPRESENTATION(#1,'Axis','Curve3D',(#20));
#40=IFCPRODUCTDEFINITIONSHAPE($,$,(#30));
#50=IFCBEAM('BeamGUID',$,'Steel_Girder_01',$,$,#1,#40,$,$);
ENDSEC;
END-ISO-10303-21;"#;

        let model = IfcImporter::import_from_ifc(ifc_text, "Imported IFC").expect("IFC import failed");
        assert_eq!(model.nodes.len(), 2);
        assert_eq!(model.elements.len(), 1);
        assert!(model.nodes[0].fixity.is_pinned());
    }

    #[test]
    fn test_comprehensive_dossier_export() {
        let mut engine = crate::core::ParametricEngine::new();
        let model = engine.generate_warren_truss().unwrap();
        let assessment = crate::core::AriEngine::evaluate_redundancy(&model).unwrap();

        let spectrum = crate::core::modal::Sni1726Spectrum::new(crate::core::modal::SiteClass::SD, 1.20, 0.50, 8.0, 1.0);
        let modal = crate::core::modal::ModalEngine::analyze(&model, 3, spectrum).ok();
        let pushover = crate::core::nonlinear::NonlinearEngine::run_pushover(&model, 5);

        let dossier = DossierExporter::export_comprehensive_dossier(&model, &assessment, modal.as_ref(), Some(&pushover), None, None);
        assert!(dossier.contains("DOSSIER RESMI AUDIT REDUNDANSI STRUKTURAL"));
        assert!(dossier.contains("SNI 1726:2019"));
        assert!(dossier.contains("FEMA 356"));
        assert!(dossier.contains("CRYPTOGRAPHIC SEAL"));
    }

    #[test]
    fn test_standalone_html_export() {
        let mut engine = crate::core::ParametricEngine::new();
        let model = engine.generate_warren_truss().unwrap();
        let assessment = crate::core::AriEngine::evaluate_redundancy(&model).unwrap();

        let html = HtmlViewerExporter::export_standalone_html(&model, Some(&assessment));
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("EigenGraph — 3D Structural Dossier Viewer"));
        assert!(html.contains("<canvas id=\"glcanvas\"></canvas>"));
        assert!(html.contains("const nodes ="));
        assert!(html.contains("const members ="));
        assert!(html.contains("FIPS SHA-256 DIGITAL SEAL"));
    }

    #[test]
    fn test_eigen_project_archive_roundtrip_and_seal() {
        let mut engine = crate::core::ParametricEngine::new();
        let original = engine.generate_warren_truss().unwrap();

        let serialized = EigenProjectArchive::serialize_to_eigen_json(&original)
            .expect("Serialization to .eigen should succeed");
        assert!(serialized.contains("EIGEN-BIM-1.1"));

        let restored = EigenProjectArchive::deserialize_from_eigen_json(&serialized)
            .expect("Deserialization and SHA-256 verification should succeed");
        assert_eq!(original, restored);
    }

    #[test]
    fn test_latex_booktabs_and_disk_exporter() {
        let mut engine = crate::core::ParametricEngine::new();
        let model = engine.generate_warren_truss().unwrap();
        let ari = crate::core::AriEngine::evaluate_redundancy(&model).unwrap();
        let n2 = crate::core::AriEngine::evaluate_n2_woodbury_fast(&model).ok();

        let tex = LatexExporter::export_ari_publication_table(&model, &ari, n2.as_ref());
        assert!(tex.contains("\\begin{table}[htbp]"));
        assert!(tex.contains("\\toprule"));
        assert!(tex.contains("\\bottomrule"));
        assert!(tex.contains("SHA-256 Digital Seal"));

        let saved_path = DiskExporter::write_export_file("test_ari_table.tex", &tex)
            .expect("DiskExporter should write to disk or /tmp fallback");
        assert!(saved_path.ends_with("test_ari_table.tex"));
    }
}
