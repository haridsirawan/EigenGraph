<div align="center">

<img src="assets/logo.png" width="380" alt="EigenGraph Logo" />

### High-Performance Computational Framework for Adjusted Redundancy Index (ARI), SVD Kinematic Mechanism Guards, and 10D BIM Construction Lifecycle Analytics

[![Release](https://img.shields.io/badge/release-v2.0.0-0A84FF.svg)](https://github.com/haridsirawan/EigenGraph/releases/latest)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-black.svg)](https://github.com/haridsirawan/EigenGraph/releases/latest)
[![License](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-52%20passed%20(100%25)-30D158.svg)](#-canonical-verification--numerical-benchmarks)
[![Engine](https://img.shields.io/badge/engine-Rust%20Analytical%20%26%20GPU%20Accelerated-purple.svg)](#-core-engineering-features--mathematical-formulations)
[![Rust](https://img.shields.io/badge/built%20with-Rust%201.80+-DEA584.svg)](https://www.rust-lang.org/)

*Official high-performance Rust analytical engine and desktop application ported directly from the research repository [Adjusted-Redundancy-Index](https://github.com/haridsirawan/Adjusted-Redundancy-Index).*

</div>

---

### 🌐 The Eigen Platform Suite
| Module | Identity | Architectural Scope |
| :---: | :---: | :--- |
| **EigenGraph** | <img src="assets/logo.png" width="220" alt="EigenGraph" /> | 10D BIM Lifecycle Dimensions, CAD/BIM Triad Dock, IFC 4.3 OpenBIM, and Native Vector Canvas Viewport. |
| **EigenGate** | <img src="assets/eigengate.png" width="220" alt="EigenGate" /> | Sherman-Morrison Rank-1 SVD Kinematic Mechanism Guard, Fracture-Critical Member (FCM) screening, and spectral stability certification. |
| **EigenFlow** | <img src="assets/eigenflow.png" width="220" alt="EigenFlow" /> | Non-linear APM progressive collapse simulation, critical path (CPM) 4D scheduling, and 6D embodied carbon LCA pipelines. |

---

## ⚡ Core Engineering Features & Mathematical Formulations

### 1. Linear ARI with Sherman-Morrison $O(N^2)$ Rank-1 Perturbation
Prior to disruption, structural equilibrium on the unconstrained degrees of freedom is solved via direct Cholesky $\mathbf{L}\mathbf{L}^T$:

$$\mathbf{K}_{ff} \mathbf{u}_f = \mathbf{F}_f$$

When a single member $e$ is disrupted ($\lambda_e = 0$), the modified stiffness matrix undergoes a rank-1 perturbation:

$$\mathbf{K}_{ff}^{(-e)} = \mathbf{K}_{ff} - \mathbf{a}_e k_e \mathbf{a}_e^T$$

Applying the **Sherman-Morrison formula**, the exact inverse of the post-disruption system is computed directly from the intact factorized state in $O(N^2)$ operations without re-factorizing:

$$(\mathbf{K}_{ff}^{(-e)})^{-1} = \mathbf{K}_{ff}^{-1} + \frac{k_e \, (\mathbf{K}_{ff}^{-1} \mathbf{a}_e)(\mathbf{a}_e^T \mathbf{K}_{ff}^{-1})}{1 - h_e}$$

where spectral leverage $h_e$ represents the diagonal projection of member flexibility onto global compliance:

$$h_e = k_e \, \mathbf{a}_e^T \mathbf{K}_{ff}^{-1} \mathbf{a}_e = \sum_{k=1}^{n_{\text{free}}} U_{e,k}^2 \in [0.0, \, 1.0]$$

---

### 2. Geometric Stiffness $K_g(P)$ & Euler Elastic Buckling Guard
Post-disruption redistributive load surges may induce buckling in slender structural members. For every compressive member ($P_r < 0$), the **Euler critical elastic buckling capacity** about the weak principal axis ($I_{\min} = \min(I_y, I_z)$) is evaluated:

$$P_e = \frac{\pi^2 E \min(I_y, I_z)}{L^2}$$

Second-order destabilization is accounted for via the **AISC 360-16 / SNI 1729:2020** $B_1$ moment amplification factor:

$$B_1 = \frac{C_m}{1 - \dfrac{P_r}{P_e}} \ge 1.0$$

The stability margin against member buckling modulates the final redundancy index, penalizing compressive members nearing instability:

$$\text{margin}_{\text{buckle}} = \max\left(0.0, \, 1.0 - \frac{P_r}{P_e}\right)$$

$$\boxed{ARI_e = RR_e \times \left(1.0 - \eta \cdot U_e^*\right) \times \sqrt{\text{margin}_{\text{buckle}}}}$$

---

### 3. Woodbury Rank-2 ($N-2$) Simultaneous Disruption Screening
To screen dual-member simultaneous disruptions $e = \{i, j\}$ across thousands of paired permutations without matrix re-inversion, the **Woodbury Matrix Identity** is deployed with a $2\times 2$ capacitance matrix $\mathbf{C}$:

$$\mathbf{K}_{ff}^{(-i, -j)} = \mathbf{K}_{ff} - \mathbf{U} \mathbf{V}^T, \quad \mathbf{U} = [\mathbf{a}_i \sqrt{k_i}, \ \mathbf{a}_j \sqrt{k_j}]$$

$$\det(\mathbf{I}_2 - \mathbf{C}) = (1 - h_i)(1 - h_j) - h_{ij}^2$$

where the cross-coupling leverage $h_{ij}$ is:

$$h_{ij} = \sqrt{k_i k_j} \, \mathbf{a}_i^T \mathbf{K}_{ff}^{-1} \mathbf{a}_j$$

If $\det(\mathbf{I}_2 - \mathbf{C}) \le \varepsilon_m$, the dual failure mode creates an instantaneous kinematic collapse mechanism.

---

### 4. Gate 1: SVD Kinematic Mechanism Guard & Hard Certificate
Before topological path traversal, structural determinacy is evaluated:

$$m_e = 1.0 - h_e$$

$$\text{If } m_e \le \varepsilon_m \ (10^{-8}) \quad \lor \quad \text{rank}(\mathbf{K}_{ff}^{(-e)}) < n_{\text{free}} \implies \boxed{ARI_e = 0.0, \quad CI_e = 1.0}$$

This guarantees a **zero false-positive rate** for kinematically unstable structures.

---

### 5. Multigraph Edge-Disjoint K-Paths via Menger's Theorem
Redundant load paths are discovered over the structural multigraph $\mathcal{G}^M = (\mathcal{V}, \mathcal{E})$ through **Menger's Theorem**:

$$\mathcal{E}(\pi_k) \cap \mathcal{E}(\pi_m) = \emptyset, \quad \forall k \ne m$$

Node-pair connectivity $\Phi_{ij}^{(K)}$ balances topological length and Castigliano compliance cost $c_e^M$:

$$\Phi_{ij}^{(K)} = \frac{1}{K} \sum_{k=1}^{K_{\text{found}}} \frac{\gamma^{\ell_k - 1}}{\sum_{e \in \pi_k} c_e^M + \varepsilon_d}, \quad c_e^M = \frac{1}{w_e^M + \varepsilon_w}$$

Combined mechanical edge weight $w_e^M$ employs Michaelis-Menten hyperbolic saturation normalization:

$$\bar{k}_e = \frac{k_e}{k_e + s_k}, \quad \bar{R}_e = \frac{R_e}{R_e + s_R}, \quad w_e^M = q_e \left(\alpha_k \bar{k}_e + \alpha_R \bar{R}_e\right)$$

---

### 6. Planar Half-Edge Face Arrangement & 3D Solid CSG B-Rep
- **Planar Half-Edge Topology**: Discovers closed rooms and architectural boundaries automatically from arbitrary intersecting wall layouts conforming to Euler's formula:
  $$\chi = V - E + F = 2 - 2g$$
  with half-edge pointer invariants $\text{twin}(\text{twin}(e)) = e$ and $\text{next}(\text{prev}(e)) = e$.
- **Concave Ear-Clipping Triangulation**: Computes polygon areas using Green's discrete line integral theorem:
  $$A = \frac{1}{2} \sum_{k=0}^{n-1} (x_k y_{k+1} - x_{k+1} y_k)$$
- **Non-Recursive BSP Tree Solid CSG Subtraction**: Robust Boolean void cutting for doors, windows, and floor shafts:
  $$\mathcal{S}_{\text{wall}} = \mathcal{S}_{\text{solid}} \setminus \bigcup_{i=1}^m \mathcal{V}_{\text{opening}, i} = \mathcal{S}_{\text{solid}} \cap \bigcap_{i=1}^m \neg \mathcal{V}_{\text{opening}, i}$$

---

### 7. 2D Technical Sheet Composer & Pure Vector Blueprints
Translates 3D structural and architectural models into calibrated 2D engineering drawings via orthographic projection:

$$\begin{bmatrix} x_{\text{sheet}} \\ y_{\text{sheet}} \end{bmatrix} = s \begin{bmatrix} \cos\theta & -\sin\theta \\ \sin\theta & \cos\theta \end{bmatrix} \begin{bmatrix} x_{\text{world}} - x_0 \\ y_{\text{world}} - y_0 \end{bmatrix} + \begin{bmatrix} t_x \\ t_y \end{bmatrix}$$

- **Technical Formats**: Multi-paper selector (**A4, A3, A2, A1**) and scaled views (**1:20, 1:50, 1:100, 1:200**).
- **Quad-Viewport Blueprints (Sheet A101)**:
  1. *Floor Plan*: Grid bubbles, concrete walls with hatched poché, columns, rooms, and section cut lines.
  2. *Section Cut A-A*: Elevation levels (+7.20m, +3.60m, ±0.00m, -1.80m), slabs, columns, and footings.
  3. *BoQ & Redundancy Schedule*: Material quantities, rebar mass (kg), and live ARI health status.
  4. *Official Title Block*: Project metadata, engineer certification, and NIST FIPS 180-4 cryptographic seal.
- **Zero-Dependency Vector Export**: Generates pristine **PDF 1.4** and **SVG** files directly.

---

### 8. Obsidian-Compliant Reactive Autosave Internal Vault
- **Continuous State Hashing**: Real-time state hashing triggers non-blocking autosave upon model mutations:
  $$\mathcal{H}_t = \text{SHA-256}(\mathcal{M}_t), \quad \Delta \mathcal{M} = \mathcal{M}_t \oplus \mathcal{M}_{t-1}$$
- **Live AI Copilot Context**: The embedded AI Copilot accesses live vault geometry, components, and solver diagnostics directly without requiring manual project saves.

---

## 📊 Canonical Verification & Numerical Benchmarks

### Analytical Verification of Canonical Model M1 (Parallel Bars - Table 25)
Canonical Model M1 consists of two vertical parallel bars pinned between support node $0$ and loaded node $1$ ($L = 1000\text{ mm}$, $E = 200\text{ kN/mm}^2$):
* **Member 1**: $A_1 = 100\text{ mm}^2, \ k_1 = 20\text{ kN/mm}, \ R_1 = 40\text{ kN}$
* **Member 2**: $A_2 = 50\text{ mm}^2, \ k_2 = 10\text{ kN/mm}, \ R_2 = 20\text{ kN}$
* Parameters: $s_k = 15, \ s_R = 30, \ \alpha_k = \alpha_R = 0.50, \ \gamma = 1.0, \ K = 2, \ P = 30\text{ kN}, \ \eta = 0.50$.

Exact rational arithmetic comparison (Benchmark Closed-Form Solution vs. Rust Engine [`src/core/ari.rs`](src/core/ari.rs), verified in unit test `test_m1_parallel_bars_disjoint_paths_exact`):

| Parameter / Metric | Exact Closed-Form Rational Value | Theoretical Decimal | EigenGraph Rust Engine Output | Verification Status |
| :--- | :---: | :---: | :---: | :---: |
| Member 1 Weight ($w_1^M$) | $\frac{4}{7}$ | $0.571429$ | $0.571429$ | ✅ **MATCH (Tol < 1e-6)** |
| Member 2 Weight ($w_2^M$) | $\frac{2}{5}$ | $0.400000$ | $0.400000$ | ✅ **MATCH (Tol < 1e-6)** |
| Intact Graph Score $S(\mathcal{G}^M)$ | $\frac{17}{35}$ | $0.485714$ | $0.485714$ | ✅ **MATCH (Tol < 1e-6)** |
| Member 1 Redundancy Ratio ($RR_1$) | $\frac{7}{17}$ | $0.411765$ | $0.411765$ | ✅ **MATCH (Tol < 1e-6)** |
| Member 1 Pre-Utilization ($U_1^*$) | $\frac{1}{2}$ | $0.500000$ | $0.500000$ | ✅ **MATCH (Tol < 1e-6)** |
| **Adjusted Redundancy Index ($ARI_1$)** | $\mathbf{\frac{21}{68}}$ | $\mathbf{0.308824}$ | $\mathbf{0.308824}$ | ✅ **MATCH (Tol < 1e-6)** |
| **Criticality Index ($CI_1$)** | $\mathbf{\frac{47}{68}}$ | $\mathbf{0.691176}$ | $\mathbf{0.691176}$ | ✅ **MATCH (Tol < 1e-6)** |

---

### ⚡ Comprehensive Computational Engine Modules (`src/core/`)
| Subsystem | Core Module | Mathematical & Computational Implementation |
| :--- | :--- | :--- |
| **Hybrid Linear Algebra** | [`src/core/linalg.rs`](src/core/linalg.rs) | Direct **Cholesky $\mathbf{L}\mathbf{L}^T$** factorization, **CSR Sparse Matrix** storage with Preconditioned Conjugate Gradient (`solve_pcg`), and SVD rank determination. |
| **3D FEM & Frame Mechanics** | [`src/core/solver.rs`](src/core/solver.rs) | $12\times 12$ 3D Euler-Bernoulli beam-column formulation, local-to-global orthogonal triad transformations ($\mathbf{K}_g = \mathbf{T}^T \mathbf{K}_l \mathbf{T}$), and **SNI 1727 / ASCE 7-22** LRFD combinations. |
| **ARI & $N-2$ Woodbury** | [`src/core/ari.rs`](src/core/ari.rs) | **Sherman-Morrison Rank-1**, $K_g(P)$ Euler buckling penalty, and **Woodbury Rank-2** $2\times 2$ determinant screening. |
| **2D/3D Geometry & CSG** | [`src/core/geom.rs`](src/core/geom.rs), [`src/core/csg.rs`](src/core/csg.rs) | Half-edge face arrangements, polygon clipping, ear-clipping triangulation, and BSP tree solid Boolean subtractions. |
| **BIM Entities & 3D Rebar** | [`src/core/bim.rs`](src/core/bim.rs) | Multi-layered walls, columns, beams, automated SNI 2847:2019 reinforcement, transactional command bus, and Sheet A101 vector export. |
| **Transient Dynamics** | [`src/core/dynamics.rs`](src/core/dynamics.rs) | Unconditionally stable implicit **Newmark-$\beta$** integration for Dynamic Amplification Factor ($\text{DAF}$) under sudden member removal. |
| **Non-Linear Pushover** | [`src/core/nonlinear.rs`](src/core/nonlinear.rs) | Geometric stiffness matrix $\mathbf{K}_g(P)$, $P\text{-}\Delta$ moment amplification, and distributed plasticity hinges. |
| **Internal Vault Engine** | [`src/core/vault.rs`](src/core/vault.rs) | Zero-friction reactive continuous autosave, Obsidian Markdown taxonomy routing, and AI Copilot bidirectional sync. |

---

## 📦 Multi-Platform Installation Guide

### Option 1: Pre-Built Multi-Platform Binaries
Download the official release packages from **[GitHub Releases](https://github.com/haridsirawan/EigenGraph/releases/latest)**:

| Platform | Installer Package | Instructions |
| :--- | :--- | :--- |
| 🍎 **macOS** | `EigenGraph-macOS.dmg` | Open the DMG and drag **EigenGraph.app** to your `/Applications` folder. *(Universal 2: Apple Silicon M1-M4 & Intel)* |
| 🪟 **Windows** | `EigenGraph-Windows-x64.zip` | Extract the ZIP archive and launch `eigengraph.exe`. *(Windows 10 / 11 64-bit)* |
| 🐧 **Linux** | `EigenGraph-Linux-x86_64.tar.gz` | Extract the archive and execute `./install.sh` or run `./eigengraph` directly. *(Ubuntu, Debian, Fedora, Arch)* |

### Option 2: Building from Source
Requires the stable Rust toolchain (1.80+):

```bash
# 1. Clone the repository
git clone https://github.com/haridsirawan/EigenGraph.git
cd EigenGraph

# 2. Run the complete test suite (52 unit tests)
cargo test

# 3. Build optimized release binary
cargo build --release

# 4. Launch EigenGraph
cargo run --release
```

---

## 🛡️ License & Acknowledgments

Copyright © 2026 **Harids Irawan**  
Distributed under the dual **MIT License** and **Apache License 2.0**.  
See the [LICENSE](LICENSE) file for complete details.
