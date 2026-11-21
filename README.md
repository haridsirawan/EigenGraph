<div align="center">

<img src="assets/logo.png" width="360" alt="EigenGraph Logo" />

# EigenGraph — Adjusted Redundancy Index (ARI) & Spatial Structural Analytics

### Doctoral Dissertation Reference Platform (Rust + WebAssembly / WebGL2)

[![Live Web App](https://img.shields.io/badge/web%20app-eigengraph.net-0A84FF?style=for-the-badge&logo=cloudflare&logoColor=white)](https://eigengraph.net/)
[![Version](https://img.shields.io/badge/version-v0.6.6%20(Phase%2066)-5B92A5?style=for-the-badge)](docs/CHANGELOG.md)
[![Engine](https://img.shields.io/badge/engine-Rust%20%2B%20WebAssembly-orange?style=for-the-badge&logo=rust&logoColor=white)](https://github.com/EigenGraph/EigenGraph)
[![License](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue?style=for-the-badge)](LICENSE)

[**Launch Interactive Web Workstation ↗**](https://eigengraph.net/) • [**Mirror (`www`) ↗**](https://www.eigengraph.net/)

</div>

---

## Doctoral Dissertation Summary

> **Title**: *Development and Validation of the Adjusted Redundancy Index for Pre-Analysis Screening of Element Damage Scenarios in Linear-Elastic Structures*
>
> **Author**: Harids Irawan

### Research Overview
Exhaustive progressive-collapse and element-disruption assessment across large structural portfolios is computationally intensive—even when accelerated by Graph Neural Network (GNN) surrogates. This dissertation introduces and validates the **Adjusted Redundancy Index (ARI)** and **Criticality Index (CI)** as a mechanics-informed, intact-model pre-analysis screening framework for linear-elastic structures.

Rather than relying on purely topological graph metrics, **ARI** combines stiffness- and capacity-weighted alternative load paths (deterministic edge-disjoint path aggregation) with initial utilization adjustments and a **three-channel formulation (`ARI v2`)** integrating kinematic stiffness reserve, cross-section capacity, and deformation control with convex weights $(w_k, w_c, w_d) = (0.25, 0.45, 0.30)$.

### Key Validated Findings
1. **Mathematical & Algorithmic Verification**: The ARI formulation satisfies canonical boundary axioms and achieved **100% pass rate (342/342 automated verification tests)** across deterministic edge-disjoint path searches and direct-stiffness benchmarks.
2. **Epistemic Boundary of Graph Connectivity**: Canonical counterexamples demonstrated that graph connectivity and low topological `CI` alone do *not* guarantee structural stability—removing a zero-force member can still trigger stiffness-matrix rank deficiency (a kinematic mechanism).
3. **Intact-Model Spectral Kinematic Guard**: Coupling ARI with an intact-model spectral/SVD kinematic guard successfully detected and forwarded **100% of mechanism-inducing disruptions** without distorting the underlying path-scoring formula.
4. **Full-Scale Empirical Screening (`S1–S5`)**: Evaluated on an independent benchmark of **300 parent structural models** across five structural families (**7,640 element loss and degradation scenarios**) under strict conformal risk calibration and frozen blind testing, **`ARI v2`** achieved a **68.4% scenario reduction** while maintaining **100.0% safety recall** on all critical collapse scenarios.
5. **Pairwise ARI–GNN Integration & Green AI**: Deploying `ARI v2` as a pre-analysis filter ahead of a trained surrogate Graph Neural Network (**`StructGNN-CRC v1.0`**) reduced net wall-clock computation time by **64.8%** compared to exhaustive GNN inference, substantially lowering computational energy demand and carbon emissions.

**Keywords**: *Adjusted Redundancy Index (ARI)*, *Alternative Load Paths*, *Spectral Kinematic Guard*, *Progressive Collapse Screening*, *Graph Neural Networks (`StructGNN-CRC`)*, *Green AI*, *Structural Redundancy*.

---

## EigenGraph Computational Platform (`v0.6.6`)

**EigenGraph** operationalizes the dissertation's ARI & Spectral Kinematic Guard engines inside a **100% Pure-Rust • 100% Vector • 100% IFCX (IFC 5 JSON ECS) • 0% Raster** WebAssembly (Native WebGL2) spatial engineering platform deployed to [eigengraph.net](https://eigengraph.net/):

- **ARI, Woodbury & Kinematic Guard Core** ([src/core/ari.rs](src/core/ari.rs)): Fast $O(N^2)$ Sherman-Morrison linear ARI, rank-2 Woodbury $N-2$ disruption screening, SVD spectral kinematic guard, and live interactive $N-1$ single-member removal simulation.
- **Site-Coupled 6-DOF FEM & Dynamic Modal Engine** ([src/core/solver.rs](src/core/solver.rs), [src/core/modal.rs](src/core/modal.rs), [src/core/bim.rs](src/core/bim.rs)): 6-DOF 3D frame solver (axial, biaxial bending, shear, P-M-V LRFD envelopes, inter-story drift) coupled to Earth Vector SNI 1726 / ASCE 7 seismic spectra and animated 3D modal eigenmodes (`Mode 1–3`).
- **100% Pure-Rust IFCX (IFC 5) Living Earth & WebGL2 Engine** ([src/core/ifcx.rs](src/core/ifcx.rs), [src/views/webgl_earth.rs](src/views/webgl_earth.rs), [src/core/gis/world_vectors.rs](src/core/gis/world_vectors.rs)): Composable JSON ECS scene graph (`earth_globe.ifcx` + georeferenced sites + AI real-building capture layers + ARI overlays) streaming `usd::usdgeom::mesh` 1:1 into 24-bit Z-buffered WebGL2 (`egui_glow`) with `f64 → f32` Floating-Origin RTC precision, animated procedural oceans, building-base ambient occlusion, and Day/Dusk/Night shaders (0% legacy STEP IFC, 0% raster pixels).

---

## Local Development & Verification

```bash
# Add WebAssembly target and Trunk bundler
rustup target add wasm32-unknown-unknown
cargo install --locked trunk

# Run locally with hot reload
trunk serve --public-url / --open

# Verify Rust tests & WebAssembly compilation
cargo test --lib
cargo check --bin eigengraph --target wasm32-unknown-unknown
trunk build --release --public-url /
```

---

## Documentation & Citation

- [Changelog (`v0.0.1` – `v0.6.6`)](docs/CHANGELOG.md)
- [Earth Vector Architecture Roadmap](docs/Earth%20Vector.md)
- [Spatial UI Architecture Contract](docs/UI_ARCHITECTURE.md)
- [Repository Guidelines](AGENTS.md)

If you reference the **Adjusted Redundancy Index (ARI)** methodology or EigenGraph platform in academic work, please cite:

> Irawan, H. (2026). *Development and Validation of the Adjusted Redundancy Index for Pre-Analysis Screening of Element Damage Scenarios in Linear-Elastic Structures*

---

## License

Copyright © 2026 Harids Irawan.
Dual-licensed under the [MIT License](LICENSE) and [Apache License 2.0](LICENSE).
