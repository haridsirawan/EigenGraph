# EigenGraph Web Agent Guidelines & Architectural Rules

## 🌐 MANDATORY ENGLISH CONSISTENCY FOR ALL GITHUB INPUTS & CONTRIBUTIONS

- **Strict English-Only Policy**:
  - All content committed or pushed to GitHub for `EigenGraph` **MUST BE EXCLUSIVELY WRITTEN IN PROFESSIONAL CIVIL & STRUCTURAL ENGINEERING ENGLISH**.
- **Automatic Language Conversion**:
  - If a user prompt, issue, task description, or feedback is submitted in Indonesian (or any language other than English), agents **must automatically translate and adapt** all resulting code, comments, documentation, UI labels, and commit messages into standard English before staging, committing, or pushing to GitHub.
- **Enforced Scope**:
  1. *Git Metadata*: Commit messages, branch names, pull request titles/descriptions, and release notes.
  2. *User Interfaces & Web*: All buttons, ribbons, HUD inspectors, dialog modals, tooltips, cards, and web pages (`index.html`, Rust `egui` views).
  3. *Source Code & Frontend Scripts*: All code comments, docstrings (`///`), variable names, error messages, and console logs.
  4. *Technical Artifacts*: All 3D visualizations, calculation summaries, and exported structural documentation.

---

## 📱 RESPONSIVE WEB-FIRST UI & FLEXIBLE NAVIGATION ARCHITECTURE

### 1. Flexible Layout Across Phone, Tablet, and Desktop:
- **No Rigid Sidebar or Chrome Locks**: `src/app.rs` and `src/views/` are **NOT** locked to a fixed sidebar width, permanent desktop sidebar, or macOS window chrome.
- **Responsive Viewport Adaptation**: All navigation, toolbars, panels, floating overlays, and modals must adapt smoothly across **Mobile (`< 640px`)**, **Tablet (`640px – 960px`)**, and **Desktop (`>= 960px`)** screens:
  - Sidebars must be **collapsible/toggleable** and behave as an **overlay drawer** (or compact navigation) on phones and narrow tablets so the main workspace retains full usable width.
  - Multi-column inspectors and canvas pairs must stack vertically or wrap cleanly on narrow viewports rather than forcing fixed pixel minimums that clip horizontally.
  - Action bars, ribbons, and badges must wrap (`horizontal_wrapped`) or scroll cleanly without overlapping text.
  - Modals and floating overlays (`egui::Window`, `Area`, AI Copilot) must clamp their dimensions to `ctx.content_rect()` using shared helpers in `src/theme.rs`.

### 2. Core Domain Navigation Pillars (`NavigationPillar`):
Keep the 15 engineering workflow modules accessible in navigation (sidebar/drawer/quick switcher), while allowing visual presentation, grouping, icons, and responsive controls to evolve freely for web and touch usability:
`Vault`, `Agent`, `Planning`, `Drafting`, `Geometry`, `Analysis`, `Scheduling`, `Budgeting`, `Sustainability`, `Facility`, `Safety`, `Construction`, `Automation`, `Prediction`, and `Report`.

### 3. 🌐 WebAssembly & Cross-Platform Compatibility:
- All core mechanics and UI code must compile cleanly on both native targets and `wasm32-unknown-unknown`.
- Never invoke blocking native OS APIs (`std::process::Command`, `std::time::SystemTime::now()`, `std::time::Instant::now()`, `std::thread::spawn`, or unconditional filesystem writes) without `#[cfg(not(target_arch = "wasm32"))]` guards or browser-safe alternatives in `src/core/web_bridge.rs`.

---

## 📁 REPOSITORY STRUCTURE & ORGANIZATION

- `src/` — Rust application (`app.rs`, `theme.rs`, `models.rs`, `core/`, `views/`, `bin/`).
- `assets/` — Web icons/logos (`assets/*.png`).
- `docs/` — Versioned project documentation and changelog (`CHANGELOG.md`).
- `.github/workflows/deploy.yml` — Automated Rust unit tests, WASM verification, Trunk release build, and Cloudflare Workers deployment.
- Keep the repository root clean; place new design documents, specifications, and guides under `docs/`.

---

## ⚡ Continuous Deployment, Release Cadence & Local Cleanup Protocol
- **10-Update Patch Release & Push Rule**:
  - Patch versions advance by `+0.0.1` once every **10 completed updates** (starting after `v0.8.4`, so Updates `1/10` through `10/10` accumulate toward `v0.8.5`, then the next 10 toward `v0.8.6`, and so on):
    1. **Intermediate Updates (`1/10` – `9/10`)**:
       - Keep the current patch version in `Cargo.toml` unchanged and log each milestone in `docs/CHANGELOG.md` under `## [Unreleased — Next: vX.Y.Z (k/10)]`.
       - Verify with `cargo test --lib` and `cargo check --bin eigengraph --target wasm32-unknown-unknown`.
       - Stage and **commit locally only** (`git commit`); **do NOT push to `origin main`** until the 10th update is reached (unless explicitly requested by the user).
    2. **Milestone Release Update (`10/10`)**:
       - Bump the patch version by `+0.0.1` in `Cargo.toml`, `Settings -> About EigenGraph`, `docs/CHANGELOG.md`, and `docs/IFCX Earth.md`.
       - Run full verification (`cargo test --lib`, `cargo check --bin eigengraph --target wasm32-unknown-unknown`, and `cargo build --bin eigengraph --target wasm32-unknown-unknown --release` or `trunk build`).
       - **Automatically stage, commit, and push all changes to `origin main`** so `./dist` deploys to Cloudflare Workers (`eigengraph.net` and `www.eigengraph.net`).
       - Clean up local build cache (`cargo clean`) and stray OS metadata files (`find . -name ".DS_Store" -not -path "./.git/*" -delete`) after pushing to keep local storage lean.

---

## 🔢 STRICT SEMANTIC VERSIONING PROGRESSION RULE

To maintain predictable engineering cadence and transparent release lineage, the project follows a **mandatory three-tier numerical progression rule** (at **10 updates per patch version** starting from `v0.8.4`):

1. **Patch Level Progression:**
   - Initial phases progressed `v0.0.1` → ... → `v0.8.4`; from `v0.8.4` onward, every **10 completed updates** advance the patch digit by one (`v0.8.4` → `v0.8.5` → ... → `v0.8.9`).
   - Skipping intermediate patch versions is prohibited.

2. **Minor Level Progression (After vX.Y.9):**
   - Advances sequentially (`v0.8.9` → `v0.9.0` → ... → `v0.9.9`).

3. **Major Level Progression (Production Milestone v1.0.0):**
   - Only after `v0.9.9` completes does the version advance to `v1.0.0`.
