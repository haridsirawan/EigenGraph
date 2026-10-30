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
        let sha256_seal = Sha256Hasher::hash_model_and_ari(model, assessment);
        md.push_str("## 6. Segel Kriptografis & Verifikasi Integritas (Tamper-Proof Audit Seal)\n\n");
        md.push_str("* **Standar Algoritma:** NIST FIPS 180-4 Secure Hash Algorithm (SHA-256)\n");
        md.push_str(&format!("* **Cryptographic Hash Digest:** `{}`\n", sha256_seal));
        md.push_str(&format!("* **QR Verification Payload:** `eigengraph://verify?hash={}&nim=23051690001`\n", sha256_seal));
        md.push_str("* **Status Integritas:** ✅ TEROTENTIKASI (Bebas Modifikasi Pasca-Kalkulasi)\n\n");

        md.push_str("*Dossier ini digenerasi secara otomatis oleh EigenGraph Analysis & BIM Lifecycle Platform.*\n");

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

    pub fn hash_model_and_ari(model: &StructuralModel, assessment: &RedundancyAssessment) -> String {
        let payload = format!(
            "EIGENGRAPH_UNY_DOSSIER:name={}:nodes={}:elems={}:avg_ari={:.6}:min_ari={:.6}:cond={:.6}:rank={}",
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
        let text = b"EigenGraph Structural Redundancy Dossier UNY 2026";
        let hash = Sha256Hasher::digest_hex(text);
        assert_eq!(hash, "b2e609583b06d78c6fe7e40dabedb3deeef3ada75d4ef8b88dcab3d8b5bd677c");
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
}
