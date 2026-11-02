# EigenGraph Releases

## Version 2.0.0 (Latest Multi-Platform Release)

EigenGraph v2.0.0 introduces a ground-up transformation of the desktop interface, a high-performance CSG B-Rep geometry kernel, full multi-platform distribution across macOS, Windows, and Linux, and advanced structural mechanics features including $K_g(P)$ geometric stiffness and Euler buckling integration into the linear Adjusted Redundancy Index (ARI).

---

### 🌟 Key Highlights & Major Features

#### 1. 🖥️ Native Full-Height App Store Interface & Minimalist Workspace
- **Full-Height Sidebar Geometry**: The left navigation sidebar extends uninterrupted from $y = 0$ to the bottom of the window, embedding native macOS window controls and top search box without horizontal dividing header bars.
- **Minimalist Canvas**: Completely eliminated secondary toolbar strips and breadcrumbs from the central workspace for a spacious, uncluttered drafting and modeling experience.
- **Adaptive Multi-Platform Decorations**: Native Win32 / Linux window manager decorations on Windows and Linux; borderless fullsize content view on macOS.
- **Bottom Profile & Settings**: Pinned profile row (`👤 Harids Irawan ⚙️`) providing one-click access to System Preferences (⌘,), Vault settings, and `✨ AI Copilot` (⌘K).

#### 2. ⚡ Linear ARI with $K_g(P)$ Geometric Stiffness & Euler Buckling Guard
- **Euler Critical Buckling Capacity**:
  $$P_e = \frac{\pi^2 E \min(I_y, I_z)}{L^2}$$
- **AISC 360-16 $B_1$ Moment Amplification Factor**:
  $$B_1 = \frac{C_m}{1 - P_r / P_e} \ge 1.0$$
- **Stability Modulation**: Modulates post-disruption member redundancy scores to prevent false resilience in slender compression members under redistributive surge:
  $$ARI_e = RR_e \times (1.0 - \eta U_e^*) \times \sqrt{\max\left(0, 1 - \frac{P_r}{P_e}\right)}$$
- **Preserved $O(N^2)$ Complexity**: Maintains exact equivalence and ultra-fast Sherman-Morrison rank-1 perturbation cycles.

#### 3. 📐 2D Sheet Composer & Blueprints Vector Exporter
- **Multi-Format Technical Paper Selector**: Standard architectural paper formats (**A4, A3, A2, A1**) and scaled drawings (**1:20, 1:50, 1:100, 1:200**).
- **Quad-Viewport Drawing Sheet (Sheet A101)**:
  1. *Viewport 1 (Floor Plan)*: Grid lines with bubbles, concrete walls with solid poché, columns, rooms, and section cut lines.
  2. *Viewport 2 (Longitudinal Section A-A)*: Core levels (+7.20m, +3.60m, ±0.00m, -1.80m), structural slabs, cut columns, and footings.
  3. *Viewport 3 (BoQ & Structural Redundancy Schedule)*: Element quantities, estimated rebar mass (kg), and live ARI safety status.
  4. *Viewport 4 (Official Title Block & Seal)*: Engineering metadata, date, status *ISSUED FOR CONSTRUCTION*, and NIST FIPS 180-4 cryptographic seal.
- **Pure Vector Output**: Zero-dependency standalone export to **PDF 1.4** and scalable vector **SVG**.

#### 4. 🌐 Full Multi-Platform Distribution (macOS, Windows, Linux)
- 🍎 **macOS (Universal 2)**: `EigenGraph-macOS.dmg` supporting Apple Silicon (M1/M2/M3/M4) and Intel x86_64 natively.
- 🪟 **Windows 64-bit**: `EigenGraph-Windows-x64.zip` for Windows 10 & 11 with portable executable `eigengraph.exe`.
- 🐧 **Linux x86_64**: `EigenGraph-Linux-x86_64.tar.gz` for Ubuntu 20.04+, Debian, Fedora, Arch with desktop shortcut and automated `install.sh`.
- **Automated Verification**: Automated computation of `SHA256SUMS.txt` on every release build.

#### 5. 🗄️ Obsidian-Compliant Reactive Autosave Internal Vault
- **Zero-Friction Continuous Autosave**: Background change detection eliminates the need for manual save buttons; every CAD command, geometry edit, and structural run is persisted immediately.
- **Direct AI Copilot Integration**: The AI Assistant maintains complete live context of the active model, component catalog, and redundancy diagnostics.

#### 6. 🛡️ Verification & Canonical Benchmarks
- **52/52 Unit Tests Passing (100%)**: Validating Cholesky $\mathbf{L}\mathbf{L}^T$, PCG CSR sparse solvers, Woodbury Rank-2 ($N-2$), Sherman-Morrison exactness, Newmark-$\beta$ dynamic amplification, and CSG boolean geometry.
