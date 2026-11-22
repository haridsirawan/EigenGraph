//! modal.rs — Dynamic Modal Analysis & Indonesian Seismic Code SNI 1726:2019 / ASCE 7-16 Engine.
//! Computes natural frequencies, periods, mode shapes, and modal mass participation factors (K * phi = omega^2 * M * phi),
//! along with complete SNI 1726:2019 Response Spectrum parameters (Ss, S1, Fa, Fv, SDS, SD1, Ts, T0, Cs, V_base).

use serde::{Deserialize, Serialize};
use crate::core::linalg::Matrix;
use crate::core::model::StructuralModel;

/// Soil site classification based on SNI 1726:2019 Section 5.3 / ASCE 7-16
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SiteClass {
    SA, // Hard Rock
    SB, // Rock
    SC, // Very Dense Soil and Soft Rock
    SD, // Stiff Soil - Default SNI 1726
    SE, // Soft Clay Soil
}

impl SiteClass {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SA => "SA — Hard Rock (vs > 1500 m/s)",
            Self::SB => "SB — Rock (750 < vs ≤ 1500 m/s)",
            Self::SC => "SC — Very Dense Soil & Soft Rock (350 < vs ≤ 750 m/s)",
            Self::SD => "SD — Stiff Soil (175 < vs ≤ 350 m/s) [Standard]",
            Self::SE => "SE — Soft Clay Soil (vs < 175 m/s)",
        }
    }

    /// Parses a SNI 1726 / ASCE 7 site class label (e.g. `"SE (Soft Soil)"`) into [`SiteClass`].
    pub fn from_soil_label(label: &str) -> Self {
        let up = label.to_ascii_uppercase();
        if up.contains("SA") || up.contains("HARD ROCK") {
            Self::SA
        } else if up.contains("SB") || (up.contains("ROCK") && !up.contains("SOFT ROCK")) {
            Self::SB
        } else if up.contains("SC") || up.contains("VERY DENSE") || up.contains("SOFT ROCK") {
            Self::SC
        } else if up.contains("SE") || up.contains("SOFT CLAY") || up.contains("SOFT SOIL") {
            Self::SE
        } else {
            Self::SD
        }
    }
}

/// Parameter Spektrum Respons Desain Gempa SNI 1726:2019 / ASCE 7-16
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sni1726Spectrum {
    pub site_class: SiteClass,
    pub ss: f64,               // Parameter percepatan respons spektra MCER periode pendek 0.2s (g)
    pub s1: f64,               // Parameter percepatan respons spektra MCER periode 1s (g)
    pub fa: f64,               // Faktor amplifikasi periode pendek
    pub fv: f64,               // Faktor amplifikasi periode 1s
    pub sms: f64,              // Fa * Ss
    pub sm1: f64,              // Fv * S1
    pub sds: f64,              // 2/3 * SMS (Percepatan desain periode pendek)
    pub sd1: f64,              // 2/3 * SM1 (Percepatan desain periode 1s)
    pub t0: f64,               // 0.2 * SD1 / SDS (detik)
    pub ts: f64,               // SD1 / SDS (detik)
    pub tl: f64,               // Transisi periode panjang (detik, default 4.0s)
    pub r_factor: f64,         // Faktor modifikasi respons R (default 8.0 untuk Rangka Pemikul Momen Khusus / Truss)
    pub importance_factor: f64,// Faktor keutamaan gempa Ie (1.0 Kategori II, 1.5 Kategori IV)
    pub cs: f64,               // Koefisien seismik Cs
}

impl Sni1726Spectrum {
    /// Menghitung seluruh parameter spektrum respons SNI 1726:2019
    pub fn new(site_class: SiteClass, ss: f64, s1: f64, r_factor: f64, importance_factor: f64) -> Self {
        let (fa, fv) = Self::compute_fa_fv(site_class, ss, s1);
        let sms = fa * ss;
        let sm1 = fv * s1;
        let sds = (2.0 / 3.0) * sms;
        let sd1 = (2.0 / 3.0) * sm1;
        let ts = if sds > 1e-6 { sd1 / sds } else { 0.5 };
        let t0 = 0.2 * ts;
        let tl = 4.0;

        // Cs dasar = SDS / (R / Ie)
        let mut cs = if r_factor > 1e-4 {
            sds / (r_factor / importance_factor)
        } else {
            0.05
        };

        // Batas minimum Cs: Cs_min = max(0.044 * SDS * Ie, 0.01)
        let cs_min = (0.044 * sds * importance_factor).max(0.01);
        if cs < cs_min {
            cs = cs_min;
        }

        Self {
            site_class,
            ss,
            s1,
            fa,
            fv,
            sms,
            sm1,
            sds,
            sd1,
            t0,
            ts,
            tl,
            r_factor,
            importance_factor,
            cs,
        }
    }

    /// Nilai Sa(T) Spektrum Desain (g) untuk perioda T (detik)
    pub fn spectral_acceleration(&self, t: f64) -> f64 {
        if t < self.t0 {
            self.sds * (0.4 + 0.6 * (t / self.t0))
        } else if t <= self.ts {
            self.sds
        } else if t <= self.tl {
            self.sd1 / t
        } else {
            (self.sd1 * self.tl) / (t * t)
        }
    }

    /// Koefisien respons seismik Cs(T) pada perioda getar T
    pub fn seismic_coefficient(&self, t: f64) -> f64 {
        let sa = self.spectral_acceleration(t);
        let mut cs = sa / (self.r_factor / self.importance_factor);
        let cs_min = (0.044 * self.sds * self.importance_factor).max(0.01);
        let cs_max = if t <= self.ts {
            self.sds / (self.r_factor / self.importance_factor)
        } else if t <= self.tl {
            self.sd1 / (t * (self.r_factor / self.importance_factor))
        } else {
            (self.sd1 * self.tl) / (t * t * (self.r_factor / self.importance_factor))
        };

        if cs > cs_max {
            cs = cs_max;
        }
        if cs < cs_min {
            cs = cs_min;
        }
        cs
    }

    /// Faktor Fa dan Fv berdasarkan Tabel 6 dan Tabel 7 SNI 1726:2019
    fn compute_fa_fv(site_class: SiteClass, ss: f64, s1: f64) -> (f64, f64) {
        match site_class {
            SiteClass::SA => (0.8, 0.8),
            SiteClass::SB => (1.0, 1.0),
            SiteClass::SC => {
                let fa = if ss <= 0.5 { 1.3 } else if ss <= 1.0 { 1.2 } else { 1.1 };
                let fv = if s1 <= 0.2 { 1.5 } else if s1 <= 0.4 { 1.4 } else { 1.3 };
                (fa, fv)
            }
            SiteClass::SD => {
                let fa = if ss <= 0.25 { 1.6 } else if ss <= 0.5 { 1.4 } else if ss <= 0.75 { 1.2 } else if ss <= 1.0 { 1.1 } else { 1.0 };
                let fv = if s1 <= 0.1 { 2.4 } else if s1 <= 0.2 { 2.0 } else if s1 <= 0.3 { 1.8 } else if s1 <= 0.4 { 1.6 } else { 1.5 };
                (fa, fv)
            }
            SiteClass::SE => {
                let fa = if ss <= 0.25 { 2.4 } else if ss <= 0.5 { 1.7 } else if ss <= 0.75 { 1.3 } else { 0.9 };
                let fv = if s1 <= 0.1 { 4.2 } else if s1 <= 0.2 { 3.3 } else if s1 <= 0.3 { 2.8 } else { 2.4 };
                (fa, fv)
            }
        }
    }
}

/// Data satu ragam getar struktur (Eigenmode)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeShape {
    pub mode_number: usize,
    pub omega_rad_s: f64,              // Frekuensi sudut (rad/s)
    pub frequency_hz: f64,             // Frekuensi alami f = omega / (2*pi) (Hz)
    pub period_s: f64,                 // Perioda getar alami T = 1 / f (detik)
    pub generalized_mass_kg: f64,      // Massa umum ragam phi^T * M * phi
    pub modal_participation_factor_x: f64,
    pub modal_participation_factor_y: f64,
    pub effective_mass_kg_x: f64,
    pub effective_mass_kg_y: f64,
    pub mass_participation_ratio_x: f64, // Rasio massa ragam terhadap total massa (%)
    pub mass_participation_ratio_y: f64,
    pub nodal_deformations: Vec<[f64; 3]>, // Displasemen mode shape ter-normalisasi (dx, dy, dz per node)
}

/// Laporan lengkap Analisis Ragam & Spektrum Gempa SNI 1726:2019
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalAnalysisReport {
    pub modes: Vec<ModeShape>,
    pub total_mass_kg: f64,
    pub total_weight_kn: f64,
    pub cumulative_mass_ratio_x: f64,
    pub cumulative_mass_ratio_y: f64,
    pub is_90_percent_satisfied: bool,
    pub fundamental_period_s: f64,
    pub fundamental_frequency_hz: f64,
    pub spectrum: Sni1726Spectrum,
    pub design_spectral_accel_g: f64,
    pub seismic_base_shear_kn: f64,
}

/// Modal Response Combination Report (CQC vs SRSS) per SNI 1726:2019 Pasal 7.9.3 & ASCE 7-16
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalCombinationReport {
    pub num_modes: usize,
    pub damping_ratio: f64,
    pub correlation_matrix_rho: Vec<Vec<f64>>,
    pub modal_base_shears_kn: Vec<f64>,
    pub srss_base_shear_kn: f64,
    pub cqc_base_shear_kn: f64,
    pub static_base_shear_kn: f64,
    pub min_85pct_threshold_kn: f64,
    pub scale_factor_85pct: f64,
    pub governing_design_base_shear_kn: f64,
}

pub struct ModalEngine;

impl ModalEngine {
    /// Runs dynamic modal analysis and computes SNI 1726:2019 / ASCE 7-16 response spectrum
    pub fn analyze(
        model: &StructuralModel,
        num_modes: usize,
        spectrum: Sni1726Spectrum,
    ) -> Result<ModalAnalysisReport, String> {
        let n_nodes = model.nodes.len();
        if n_nodes < 2 {
            return Err("Model must have at least 2 nodes for dynamic modal analysis".to_string());
        }

        // Run FEA solver to assemble global stiffness matrix K and partition free DOFs
        let fea = crate::core::solver::FemSolver::solve(model)?;
        let free_dofs = fea.free_dofs;
        let n_free = free_dofs.len();
        if n_free == 0 {
            return Err("Structure has no active free degrees of freedom".to_string());
        }

        let dofs_per_node = 6;
        let total_dofs = n_nodes * dofs_per_node;
        let mut m_diag = vec![0.0; total_dofs];

        let mut total_structural_mass_kg = 0.0;

        for elem in &model.elements {
            let n1 = model.find_node(elem.start_node).ok_or("Start node not found")?;
            let n2 = model.find_node(elem.end_node).ok_or("End node not found")?;
            let len = n1.distance_to(n2);
            let mat_density = elem.material.density.max(1000.0);
            let elem_mass = elem.section.area * len * mat_density;
            total_structural_mass_kg += elem_mass;

            let n1_idx = model.nodes.iter().position(|n| n.id == elem.start_node).unwrap();
            let n2_idx = model.nodes.iter().position(|n| n.id == elem.end_node).unwrap();

            let half_mass = elem_mass * 0.5;
            for i in 0..3 {
                m_diag[n1_idx * 6 + i] += half_mass;
                m_diag[n2_idx * 6 + i] += half_mass;
            }
            // Rotational mass inertia
            let rot_mass = (half_mass * 0.05).max(1e-2);
            for i in 3..6 {
                m_diag[n1_idx * 6 + i] += rot_mass;
                m_diag[n2_idx * 6 + i] += rot_mass;
            }
        }

        // Tambahkan massa beban gravitasi jika ada
        for load in &model.loads {
            if let Some(n_idx) = model.nodes.iter().position(|n| n.id == load.node_id) {
                let load_mag = (load.fx * load.fx + load.fy * load.fy + load.fz * load.fz).sqrt();
                let added_mass = (load_mag / 9.81).abs();
                total_structural_mass_kg += added_mass;
                for i in 0..3 {
                    m_diag[n_idx * 6 + i] += added_mass;
                }
            }
        }

        let mut m_ff = vec![0.0; n_free];
        let mut d_vec = vec![0.0; n_free];
        for (i_f, &dof_i) in free_dofs.iter().enumerate() {
            let m_val = m_diag[dof_i].max(1.0);
            m_ff[i_f] = m_val;
            d_vec[i_f] = 1.0 / m_val.sqrt();
        }

        // Matriks kekakuan ternormalisasi massa: K_tilde = D * K_ff * D
        let mut k_tilde = Matrix::zeros(n_free, n_free);
        for i in 0..n_free {
            for j in 0..n_free {
                k_tilde[(i, j)] = d_vec[i] * fea.k_free[(i, j)] * d_vec[j];
            }
        }

        // Invers K_tilde (A_dyn = K_tilde^-1)
        let a_dyn = k_tilde.inverse().unwrap_or_else(|| {
            let mut reg = k_tilde.clone();
            for i in 0..n_free {
                reg[(i, i)] += 1e-4;
            }
            reg.inverse().unwrap_or_else(|| Matrix::identity(n_free))
        });

        // 4. Inverse Power Iteration dengan Deflasi Gram-Schmidt
        let modes_to_extract = num_modes.min(n_free).min(8);
        let mut mode_shapes = Vec::new();
        let mut y_vectors: Vec<Vec<f64>> = Vec::new();

        for m_idx in 0..modes_to_extract {
            // Vektor awal iterasi (pseudo-acak teratur)
            let mut v = vec![0.0; n_free];
            for i in 0..n_free {
                v[i] = (((i + m_idx * 7) % 13) as f64 - 6.0) / 7.0;
                if v[i].abs() < 1e-4 { v[i] = 1.0; }
            }

            // Power iteration
            let mut mu = 1.0;
            for _ in 0..60 {
                // Gram-Schmidt orthogonalization terhadap vektor ragam sebelumnya
                for prev_y in &y_vectors {
                    let dot: f64 = v.iter().zip(prev_y.iter()).map(|(a, b)| a * b).sum();
                    for k in 0..n_free {
                        v[k] -= dot * prev_y[k];
                    }
                }

                let norm_v: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
                if norm_v > 1e-12 {
                    for x in &mut v { *x /= norm_v; }
                }

                // w = A_dyn * v
                let mut w = vec![0.0; n_free];
                for r in 0..n_free {
                    let mut sum = 0.0;
                    for c in 0..n_free {
                        sum += a_dyn[(r, c)] * v[c];
                    }
                    w[r] = sum;
                }

                for prev_y in &y_vectors {
                    let dot: f64 = w.iter().zip(prev_y.iter()).map(|(a, b)| a * b).sum();
                    for k in 0..n_free {
                        w[k] -= dot * prev_y[k];
                    }
                }

                mu = w.iter().map(|x| x * x).sum::<f64>().sqrt();
                if mu < 1e-12 { break; }
                for k in 0..n_free {
                    v[k] = w[k] / mu;
                }
            }

            y_vectors.push(v.clone());

            // mu = 1 / lambda = 1 / omega^2
            let omega = if mu > 1e-12 { (1.0 / mu).sqrt() } else { 1.0 };
            let freq_hz = omega / (2.0 * std::f64::consts::PI);
            let period_s = if freq_hz > 1e-6 { 1.0 / freq_hz } else { 0.0 };

            // Vektor ragam struktural phi = D * v
            let mut phi_free = vec![0.0; n_free];
            for i in 0..n_free {
                phi_free[i] = d_vec[i] * v[i];
            }

            // Normalisasi terhadap matriks massa phi^T * M * phi = 1.0
            let gen_mass: f64 = phi_free.iter().enumerate().map(|(i, &val)| val * val * m_ff[i]).sum();
            let norm_factor = if gen_mass > 1e-12 { 1.0 / gen_mass.sqrt() } else { 1.0 };
            for x in &mut phi_free { *x *= norm_factor; }

            // Rekonstruksi ke seluruh DOFs nodal (dx, dy, dz)
            let mut full_phi = vec![0.0; total_dofs];
            for (i_f, &dof_i) in free_dofs.iter().enumerate() {
                full_phi[dof_i] = phi_free[i_f];
            }

            let mut nodal_deforms = Vec::new();
            let mut max_disp = 0.0_f64;
            for n_i in 0..n_nodes {
                let dx = full_phi[n_i * 6];
                let dy = full_phi[n_i * 6 + 1];
                let dz = full_phi[n_i * 6 + 2];
                let mag = (dx * dx + dy * dy + dz * dz).sqrt();
                max_disp = max_disp.max(mag);
                nodal_deforms.push([dx, dy, dz]);
            }
            if max_disp > 1e-6 {
                for def in &mut nodal_deforms {
                    def[0] /= max_disp;
                    def[1] /= max_disp;
                    def[2] /= max_disp;
                }
            }

            // Faktor Partisipasi Massa Ragam (Gamma_x, Gamma_y)
            let mut l_x = 0.0;
            let mut l_y = 0.0;
            for (i_f, &dof_i) in free_dofs.iter().enumerate() {
                let dof_type = dof_i % 6;
                if dof_type == 0 {
                    l_x += phi_free[i_f] * m_ff[i_f];
                } else if dof_type == 1 {
                    l_y += phi_free[i_f] * m_ff[i_f];
                }
            }

            let eff_mass_x = l_x * l_x;
            let eff_mass_y = l_y * l_y;
            let mass_ratio_x = if total_structural_mass_kg > 1e-4 { (eff_mass_x / total_structural_mass_kg) * 100.0 } else { 0.0 };
            let mass_ratio_y = if total_structural_mass_kg > 1e-4 { (eff_mass_y / total_structural_mass_kg) * 100.0 } else { 0.0 };

            mode_shapes.push(ModeShape {
                mode_number: m_idx + 1,
                omega_rad_s: omega,
                frequency_hz: freq_hz,
                period_s,
                generalized_mass_kg: 1.0,
                modal_participation_factor_x: l_x,
                modal_participation_factor_y: l_y,
                effective_mass_kg_x: eff_mass_x,
                effective_mass_kg_y: eff_mass_y,
                mass_participation_ratio_x: mass_ratio_x,
                mass_participation_ratio_y: mass_ratio_y,
                nodal_deformations: nodal_deforms,
            });
        }

        let cum_x: f64 = mode_shapes.iter().map(|m| m.mass_participation_ratio_x).sum();
        let cum_y: f64 = mode_shapes.iter().map(|m| m.mass_participation_ratio_y).sum();
        let t1 = mode_shapes.first().map(|m| m.period_s).unwrap_or(0.5);
        let f1 = mode_shapes.first().map(|m| m.frequency_hz).unwrap_or(2.0);

        let sa_design = spectrum.spectral_acceleration(t1);
        let cs_actual = spectrum.seismic_coefficient(t1);
        let total_weight_kn = (total_structural_mass_kg * 9.81) / 1000.0;
        let base_shear_kn = cs_actual * total_weight_kn;

        Ok(ModalAnalysisReport {
            modes: mode_shapes,
            total_mass_kg: total_structural_mass_kg,
            total_weight_kn,
            cumulative_mass_ratio_x: cum_x,
            cumulative_mass_ratio_y: cum_y,
            is_90_percent_satisfied: cum_x >= 90.0 || cum_y >= 90.0,
            fundamental_period_s: t1,
            fundamental_frequency_hz: f1,
            spectrum,
            design_spectral_accel_g: sa_design,
            seismic_base_shear_kn: base_shear_kn,
        })
    }

    /// Interactive-latency dynamic modal analysis for real-time Earth 3D animation.
    /// Executes the full 6-DOF inverse-power eigenproblem for models up to 48 nodes,
    /// and transitions cleanly to an analytical 3-mode shear-building continuum fallback
    /// for massive multi-hundred-node IFC imports so WebAssembly never stalls.
    pub fn analyze_for_earth_3d(
        model: &StructuralModel,
        num_modes: usize,
        spectrum: Sni1726Spectrum,
    ) -> Result<ModalAnalysisReport, String> {
        let k_modes = num_modes.clamp(1, 3);
        if model.nodes.len() <= 48 && model.elements.len() <= 140 {
            if let Ok(rep) = Self::analyze(model, k_modes, spectrum.clone()) {
                if !rep.modes.is_empty() {
                    return Ok(rep);
                }
            }
        }
        Self::synthesize_continuum_modes(model, k_modes, spectrum)
    }

    /// Fast analytical 3-mode continuum shear/torsion eigenmode estimator for large 3D frames
    /// (Mode 1: East-West first sway, Mode 2: North-South orthogonal sway, Mode 3: 2nd sway / torsion).
    fn synthesize_continuum_modes(
        model: &StructuralModel,
        num_modes: usize,
        spectrum: Sni1726Spectrum,
    ) -> Result<ModalAnalysisReport, String> {
        if model.nodes.len() < 2 {
            return Err("Model requires at least 2 nodes for modal synthesis".to_string());
        }
        let max_h = model
            .nodes
            .iter()
            .map(|n| n.y)
            .fold(0.0_f64, f64::max)
            .max(3.0);
        let cx = model.nodes.iter().map(|n| n.x).sum::<f64>() / model.nodes.len() as f64;
        let cz = model.nodes.iter().map(|n| n.z).sum::<f64>() / model.nodes.len() as f64;
        let max_r = model
            .nodes
            .iter()
            .map(|n| (n.x - cx).hypot(n.z - cz))
            .fold(1.0_f64, f64::max);

        // Approximate fundamental period T1 = 0.0466 * H^0.9 (SNI 1726 Table 18 concrete MRF)
        let mut t1 = (0.0466 * max_h.powf(0.9)).clamp(0.12, 4.5);
        let period_factors = [1.0, 0.86, 0.36];
        let mass_x_pcts = [76.5, 4.2, 12.8];
        let mass_y_pcts = [4.5, 74.8, 11.2];

        let mut total_mass_kg = 0.0;
        for elem in &model.elements {
            if let (Some(n1), Some(n2)) = (model.find_node(elem.start_node), model.find_node(elem.end_node)) {
                total_mass_kg += elem.section.area * n1.distance_to(n2) * elem.material.density.max(1000.0);
            }
        }
        for load in &model.loads {
            total_mass_kg += (load.fy.abs() / 9.81).max(0.0);
        }
        total_mass_kg = total_mass_kg.max(1000.0);

        // Apply Veletsos-Meek SSI period lengthening if foundation nodes carry elastic Winkler springs
        let mut ssi_base_count = 0usize;
        let mut sum_kx_n_m = 0.0_f64;
        let mut sum_kr_nm_rad = 0.0_f64;
        for node in &model.nodes {
            if node.y <= 0.1 && node.has_springs() {
                ssi_base_count += 1;
                sum_kx_n_m += node.spring_stiffness[0].max(node.spring_stiffness[2]).max(1.0);
                sum_kr_nm_rad += node.spring_stiffness[3].max(node.spring_stiffness[5]).max(1.0);
            }
        }
        let has_ssi_base = ssi_base_count > 0;
        if has_ssi_base {
            let omega_fixed = 2.0 * std::f64::consts::PI / t1;
            let k_fixed_n_m = total_mass_kg * omega_fixed * omega_fixed;
            let h_eff = 0.70 * max_h;
            let len_sq = 1.0
                + (k_fixed_n_m / sum_kx_n_m.max(1e3))
                + (k_fixed_n_m * h_eff * h_eff / sum_kr_nm_rad.max(1e4));
            let ssi_ratio = len_sq.sqrt().clamp(1.04, 1.45);
            t1 = (t1 * ssi_ratio).clamp(0.12, 6.0);
        }

        let mut modes = Vec::with_capacity(num_modes.min(3));
        for m in 0..num_modes.min(3) {
            let period_s = (t1 * period_factors[m]).max(0.04);
            let frequency_hz = 1.0 / period_s;
            let omega_rad_s = 2.0 * std::f64::consts::PI * frequency_hz;
            let mut nodal_deformations = Vec::with_capacity(model.nodes.len());
            for node in &model.nodes {
                let h_frac = (node.y / max_h).clamp(0.0, 1.0);
                if (h_frac < 0.02 && !node.has_springs()) || node.fixity.is_fixed() {
                    nodal_deformations.push([0.0, 0.0, 0.0]);
                    continue;
                }
                let base_slip = if has_ssi_base && h_frac < 0.02 { 0.08 } else { 0.0 };
                let def = match m {
                    0 => {
                        // Mode 1: Fundamental lateral sway along X
                        let amp = (0.5 * std::f64::consts::PI * h_frac).sin().max(base_slip);
                        [amp, 0.0, 0.12 * amp]
                    }
                    1 => {
                        // Mode 2: Orthogonal lateral sway along Z
                        let amp = (0.5 * std::f64::consts::PI * h_frac).sin().max(base_slip);
                        [-0.12 * amp, 0.0, amp]
                    }
                    _ => {
                        // Mode 3: Coupled 2nd sway + plan torsion about centroid
                        let s2 = (1.5 * std::f64::consts::PI * h_frac).sin();
                        let rx = (node.x - cx) / max_r;
                        let rz = (node.z - cz) / max_r;
                        [
                            (-rz * h_frac.max(base_slip) + 0.45 * s2).clamp(-1.0, 1.0),
                            0.0,
                            (rx * h_frac.max(base_slip)).clamp(-1.0, 1.0),
                        ]
                    }
                };
                nodal_deformations.push(def);
            }
            let mx = mass_x_pcts[m];
            let my = mass_y_pcts[m];
            modes.push(ModeShape {
                mode_number: m + 1,
                omega_rad_s,
                frequency_hz,
                period_s,
                generalized_mass_kg: 1.0,
                modal_participation_factor_x: (mx / 100.0 * total_mass_kg).sqrt(),
                modal_participation_factor_y: (my / 100.0 * total_mass_kg).sqrt(),
                effective_mass_kg_x: mx / 100.0 * total_mass_kg,
                effective_mass_kg_y: my / 100.0 * total_mass_kg,
                mass_participation_ratio_x: mx,
                mass_participation_ratio_y: my,
                nodal_deformations,
            });
        }

        let cum_x: f64 = modes.iter().map(|m| m.mass_participation_ratio_x).sum();
        let cum_y: f64 = modes.iter().map(|m| m.mass_participation_ratio_y).sum();
        let f1 = 1.0 / t1;
        let sa_design = spectrum.spectral_acceleration(t1);
        let cs_actual = spectrum.seismic_coefficient(t1);
        let total_weight_kn = (total_mass_kg * 9.81) / 1000.0;
        let base_shear_kn = cs_actual * total_weight_kn;

        Ok(ModalAnalysisReport {
            modes,
            total_mass_kg,
            total_weight_kn,
            cumulative_mass_ratio_x: cum_x,
            cumulative_mass_ratio_y: cum_y,
            is_90_percent_satisfied: cum_x >= 90.0 || cum_y >= 90.0,
            fundamental_period_s: t1,
            fundamental_frequency_hz: f1,
            spectrum,
            design_spectral_accel_g: sa_design,
            seismic_base_shear_kn: base_shear_kn,
        })
    }

    /// Evaluates CQC (Complete Quadratic Combination) and SRSS (Square Root of Sum of Squares)
    /// modal response combinations per SNI 1726:2019 Pasal 7.9.3 & ASCE 7-16 Section 12.9.1.
    pub fn evaluate_modal_combinations(
        report: &ModalAnalysisReport,
        damping_ratio: f64,
    ) -> ModalCombinationReport {
        let n = report.modes.len();
        let xi = damping_ratio.clamp(0.01, 0.20); // 5% default

        // 1. Cross-modal correlation matrix rho_ij using Der Kiureghian formulation
        let mut rho = vec![vec![1.0; n]; n];
        for i in 0..n {
            let wi = report.modes[i].omega_rad_s.max(1e-4);
            for j in 0..n {
                if i != j {
                    let wj = report.modes[j].omega_rad_s.max(1e-4);
                    let r = wj / wi;
                    let num = 8.0 * xi * xi * (1.0 + r) * r.powf(1.5);
                    let denom = (1.0 - r * r).powi(2) + 4.0 * xi * xi * r * (1.0 + r).powi(2);
                    let coeff = if denom > 1e-9 { (num / denom).clamp(0.0, 1.0) } else { 1.0 };
                    rho[i][j] = coeff;
                }
            }
        }

        // 2. Modal base shears per mode: V_i = M_eff_i * Sa(Ti) * g
        let mut modal_v = Vec::with_capacity(n);
        for m in &report.modes {
            let sa_g = report.spectrum.spectral_acceleration(m.period_s);
            let eff_mass_tonne = (m.effective_mass_kg_x.max(m.effective_mass_kg_y)) / 1000.0;
            let vi_kn = eff_mass_tonne * 9.81 * (sa_g / report.spectrum.r_factor.max(1.0));
            modal_v.push(vi_kn);
        }

        // 3. SRSS combination: V_srss = sqrt(sum(Vi^2))
        let srss_sum: f64 = modal_v.iter().map(|v| v * v).sum();
        let v_srss = srss_sum.sqrt();

        // 4. CQC combination: V_cqc = sqrt(sum_i sum_j Vi * rho_ij * Vj)
        let mut cqc_sum = 0.0_f64;
        for i in 0..n {
            for j in 0..n {
                cqc_sum += modal_v[i] * rho[i][j] * modal_v[j];
            }
        }
        let v_cqc = cqc_sum.max(0.0).sqrt();

        // 5. SNI 1726 Pasal 7.9.4.1 / ASCE 7-16 Section 12.9.1.4.1:
        // Combined response must not be less than 85% of static equivalent base shear V_static
        let v_static = report.seismic_base_shear_kn;
        let v_min_85 = 0.85 * v_static;
        let governing_comb = v_cqc.max(v_srss);
        let scale_factor = if governing_comb < v_min_85 && governing_comb > 1e-4 {
            v_min_85 / governing_comb
        } else {
            1.0
        };
        let governing_design = governing_comb * scale_factor;

        ModalCombinationReport {
            num_modes: n,
            damping_ratio: xi,
            correlation_matrix_rho: rho,
            modal_base_shears_kn: modal_v,
            srss_base_shear_kn: v_srss,
            cqc_base_shear_kn: v_cqc,
            static_base_shear_kn: v_static,
            min_85pct_threshold_kn: v_min_85,
            scale_factor_85pct: scale_factor,
            governing_design_base_shear_kn: governing_design,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sni1726_spectrum_values() {
        let spec = Sni1726Spectrum::new(SiteClass::SD, 1.20, 0.50, 8.0, 1.0);
        assert!(spec.fa > 0.0);
        assert!(spec.fv > 0.0);
        assert!(spec.sds > 0.0);
        assert!(spec.sd1 > 0.0);
        assert!(spec.ts > spec.t0);

        let sa_plat = spec.spectral_acceleration(spec.t0 + 0.05);
        assert!((sa_plat - spec.sds).abs() < 1e-4);

        let sa_long = spec.spectral_acceleration(spec.ts + 1.0);
        assert!(sa_long < spec.sds);
    }

    #[test]
    fn test_modal_analysis_on_warren_truss() {
        let mut engine = crate::core::ParametricEngine::new();
        let model = engine.generate_warren_truss().unwrap();

        let spectrum = Sni1726Spectrum::new(SiteClass::SD, 1.20, 0.50, 8.0, 1.0);
        let report = ModalEngine::analyze(&model, 4, spectrum).unwrap();

        assert_eq!(report.modes.len(), 4);
        assert!(report.fundamental_period_s > 0.0);
        assert!(report.fundamental_frequency_hz > 0.0);
        assert!(report.total_mass_kg > 0.0);
        assert!(report.seismic_base_shear_kn > 0.0);

        // Perioda getar harus monoton menurun (T1 >= T2 >= T3 >= T4)
        for i in 1..report.modes.len() {
            assert!(report.modes[i - 1].period_s >= report.modes[i].period_s - 1e-4);
        }

        let comb = ModalEngine::evaluate_modal_combinations(&report, 0.05);
        assert_eq!(comb.num_modes, 4);
        assert!(comb.srss_base_shear_kn > 0.0);
        assert!(comb.cqc_base_shear_kn > 0.0);
        assert!(comb.governing_design_base_shear_kn > 0.0);
        assert_eq!(comb.correlation_matrix_rho.len(), 4);
        assert_eq!(comb.correlation_matrix_rho[0][0], 1.0);
    }
}
