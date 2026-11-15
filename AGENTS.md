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

## ⚡ Continuous Deployment & Verification Protocol
- Validate UI modifications, `cargo test --lib`, `cargo check --bin eigengraph --target wasm32-unknown-unknown`, and `trunk build` before publishing.
- Every push to `main` automatically deploys `./dist` to Cloudflare Workers (`eigengraph.net` and `www.eigengraph.net`).

---

## 🔢 STRICT SEMANTIC VERSIONING PROGRESSION RULE

To maintain predictable engineering cadence and transparent release lineage, the project follows a **mandatory three-tier numerical progression and 1-to-1 Phase mapping rule**:

1. **Patch Level Progression (Initial Releases):**
   - Versioning begins at initial baseline `v0.0.1` and advances one patch digit at a time (`v0.0.1` → `v0.0.2` → ... → `v0.0.9`).
   - Skipping intermediate patch versions is prohibited.

2. **Minor Level Progression (After v0.0.9):**
   - Advances to `v0.1.0` → `v0.1.1` → ... → `v0.2.5` (Phase 25) → ... → `v0.9.9`.

3. **Major Level Progression (Production Milestone v1.0.0):**
   - Only after `v0.9.9` completes does the version advance to `v1.0.0`.
