# Changelog

All notable changes to the **EigenGraph** structural analysis and civil engineering platform will be documented in this file.

The project strictly adheres to sequential versioning and strict 1-to-1 phase mapping:
`v0.0.1` (Phase 1) ➔ `v0.0.2` (Phase 2) ➔ `v0.0.3` (Phase 3) ➔ `v0.0.4` (Phase 4) ➔ `v0.0.5` (Phase 5) ➔ `v0.0.6` (Phase 6) ➔ `v0.0.7` (Phase 7) ... (never skip patch numbers).

---

## [v0.5.4] - 2026-11-18 — Phase 54: Essential Restructured Dynamic Island, Structural Health Triage, Enriched Gemini BIM IDs, Wall Openings, 2D Slab Picking & Vault Multi-Folder Notes

### Added & Changed
- **Essential, User-Friendly Restructured Dynamic Island (`src/views/spatial_workspace.rs`, `docs/UI_ARCHITECTURE.md`)**:
  - Simplified the Resting Dock Row 1 so when an Obsidian Vault is connected only `Vault: <name>` appears (moving `Switch` inside the Vault drawer), reducing idle capsule width and visual noise.
  - Reorganized the expanded Hover Extension Deck into four clean, mode-aware layers:
    1. **Structural Health HUD** (when BIM/FEM elements exist): live `C / B • Max DCR % • Min ARI • Crit` metrics with 1-click **`Inspect Worst`** bottleneck focus and **`Auto-Repair`**.
    2. **Core Quick Controls**: `Undo`, `Redo`, `View (Earth 3D / BIM 3D / Plan 2D)`, `Fit`, `Export`, `IFC4`, `Audit .md`, `API Key`, `Settings`, and a collapsible **`More Tools ▼/▲`** toggle (`hover_more_tools_open`).
    3. **Context-Aware Mode Studio Bar**: adapts cleanly to `BIM 3D` (`Iso`, `Top`, `Front`, `Story`, `+ Story`, `+ Slab`, `Clean / DCR% / ARI / Moment / 4D`), `Plan 2D` (`Ortho`, `Grid`, `Freehand`, `Circle`, `Dim`, `Note`, `BoxSel`, `Erase`), or `Earth 3D` (`Map BG`, `Grid`, `Uplift`, `Carve`, `Seawall`).
    4. **Extended Drawer (`More Tools`)**: keeps 11 secondary 2D CAD / Earth / Export utilities tucked behind an opt-in expander for a cleaner everyday workflow.
- **Structural Health Triage & Live Cursor Telemetry (`src/views/spatial_workspace.rs`)**:
  - Added `structural_health_summary` to rank the worst `utilization` / critical `StructuralMember` dynamically and resolve `Inspect Worst` directly to the governing `Column` or `Beam`.
  - Added live cursor world coordinates `@ (x.xx, y.yy) m` (`hover_3d_ground_snap_mm`) and geographic coordinates `@ lat°, lon°` (`cursor_lat_lon`) inside the floating Active Tool Pill.
- **Enriched Gemini AI Payload & Story/Slab Directives (`src/views/spatial_workspace.rs`)**:
  - Added `format_critical_bim_context` to inject exact top-6 critical `Column #id (WxD mm)` and `Beam #id (bxh mm)` identifiers, `DCR%`, `ARI`, and slab counts into `build_enriched_context_payload` (`BimGeometry` / `FemRedundancy`) and the Spatial Gemini system prompt.
  - Added `[[ADD_STORY]]` / `[[ADD_LEVEL]]` and `[[ADD_SLAB]]` directive parsing and undoable proposal execution via `PlannedProposalAction::{AddStory, AddSlab}`.
- **`Wall` Opening Authoring & 2D Plan `Slab` Picking (`src/views/spatial_workspace.rs`, `src/views/universal_canvas.rs`)**:
  - Added 1-click **`+ Door (900x2100)`** (`OpeningKind::SingleDoor`) and **`+ Window (1800x1400)`** (`OpeningKind::CasementWindow`, sill `900 mm`) CSG controls (`BimCommand::AddOpening`) inside the `Wall` Smart Pop-Up Properties tab.
  - Added Stage 5 polygon point-in-polygon hit detection (`BimElementKind::Slab`) inside `UniversalCanvasState::pick_bim_element_at_screen` for 2D Plan selection.
- **Vault Multi-Folder Note Target, `Audit .md` Dossier & `IFC4` Export (`src/core/vault.rs`, `src/views/spatial_workspace.rs`, `src/app.rs`)**:
  - Added `list_vault_subfolders`, `save_markdown_note_in_folder`, `save_markdown_note_in_folder_at`, and `build_engineering_audit_markdown` in `InternalVault`.
  - Added target subfolder selection (`vault_note_folder`) in the Vault `+ Note (.md)` composer plus direct **`Audit .md`** Vault archiving and **`IFC4`** spatial `.ifc` export actions (`SpatialHudActions`).

---

## [v0.5.3] - 2026-11-18 — Phase 53: Earth Site Inside Dynamic Island, Pure Earth Corners, LIFO Undo/Redo, Vault Organizer & Story/Slab Studio

### Added & Changed
- **Unified Earth Site Location Strip Inside Dynamic Island & Watermark-Free Earth Corners (`src/views/spatial_workspace.rs`, `src/core/gis/earth_globe.rs`, `src/core/gis/map_widget.rs`, `docs/UI_ARCHITECTURE.md`)**:
  - Moved `EarthSite` telemetry (`Ground Elevation`, `Seismic Hazard Ss/S1`, `Design Wind Speed`, `Flood Risk Index`, `Soil Class`) and quick site actions (`Anchor Site Here`, `Anchor & Build Here`, `Build 3x2 Portal`, `Load BIMCraft Sample`, `Focus 3D BIM`, `Sync Site Seismic`) directly into the single growing Dynamic Island (`render_earth_site_inspector_strip`), eliminating the separate top-right popup box when clicking Earth locations.
  - Removed the bottom-right `EigenEarth 3D • Tiles © Esri World Imagery` attribution micro-label across both 3D Earth Globe and 2D Slippy Map viewports so all four canvas corners remain completely unobstructed.
- **Keyboard `Cmd/Ctrl+Z` & `Cmd/Ctrl+Shift+Z / Y` + Chronological LIFO Undo/Redo (`src/app.rs`, `src/views/spatial_workspace.rs`)**:
  - Wired global Spatial Mode `Cmd/Ctrl+Z` (Undo) and `Cmd/Ctrl+Shift+Z` / `Cmd/Ctrl+Y` (Redo) shortcuts when keyboard focus is outside text inputs.
  - Added `SpatialUndoDomain` chronological LIFO history (`observe_undo_stacks`, `pop_undo_domain`, `pop_redo_domain`) so Undo/Redo always reverses the most recent BIM 3D or 2D CAD action across domains accurately.
- **Obsidian-Style Vault Subfolder & File Management (`src/core/vault.rs`, `src/views/spatial_workspace.rs`)**:
  - Expanded `InternalVault` directory scanning to discover custom user subfolders alongside structured folders on Native & WASM IndexedDB, and added `create_vault_subfolder`, `rename_vault_file`, and `delete_vault_context_file`.
  - Added Vault search filter, `+ Folder` creator, inline `Ren` / `Del` actions, and `+ Insert Vault Context` backlink helper inside the Dynamic Island Vault drawer.
- **Active Story Selector, `Slab` Authoring/Inspection & 3D Camera Presets (`src/core/bim.rs`, `src/views/build/viewport_3d.rs`, `src/views/spatial_workspace.rs`)**:
  - Added `Story` floor selector, `+ Story` (`AddLevel +3.5m`), and `+ Slab` (`AddSlab`) controls inside the Active Tool Pill and Dynamic Island Hover Extension Strip, plus `Iso`, `Top`, and `Front` 3D camera presets.
  - Added `BimElementKind::Slab` 3D ray picking and `SmartSelectionTarget::Slab(id)` inspector (`Level Elevation`, `Thickness`, `Footprint Area`, `Embodied Carbon`, and undoable delete).
- **Offline `EigenAI Pro` Imperative Routing & Live `Geodesic Measure` Readout (`src/views/universal_canvas.rs`, `src/views/spatial_workspace.rs`)**:
  - Added `geodesic_measure_summary` live distance (`m`/`km`) and polygon area (`m²`/`ha`) display inside the Active Tool Pill, plus offline imperative routing (`open <sheet>`, `add story`, `add slab`, `iso/top/front view`, `focus 3d`, `undo/redo`).

---

## [v0.5.2] - 2026-11-18 — Phase 52: Unified Growing Dynamic Island, Mobile Expander, Esc Orbit Reset, Precision 2D CAD & Gemini 2.5 Pro Sheets

### Added & Changed
- **Unified Single Growing Dynamic Island & Mobile Touch Expander (`src/views/spatial_workspace.rs`, `docs/UI_ARCHITECTURE.md`)**:
  - Consolidated the resting 3-row dock, hover extension deck (`Undo / Redo`, `View`, `API Key`, `Settings`, `Grid`, `Map BG`, `Ortho`, structural lenses, and `More Draw`), and interactive drawers (`Vault`, `@ Context`, `/ Slash`, `History`, `Chat`) into **one single continuous outer `antigravity_glass_frame`** separated cleanly by hairline dividers so hovering expands the capsule itself rather than stacking two boxes.
  - Added a mobile/touch `Deck` toggle (`mobile_deck_pinned`) on viewports `< 640px` so touchscreen users can expand and collapse Dynamic Island without pointer hover.
- **Deterministic `Esc` Priority & Orbit Fallback (`src/app.rs`, `src/views/spatial_workspace.rs`)**:
  - Updated Spatial Mode `Esc` handling to dismiss `api_key_popover_open` and reset active manual BIM authoring, 2D CAD drawing, and `Geodesic Measure` tools back to `Orbit / SelectPan` (`has_active_manual_tool` / `reset_manual_tools_to_orbit`).
- **Upgraded `SketchEntity` Smart Pop-Up & Precision 2D Drafting Controls (`src/views/universal_canvas.rs`, `src/views/spatial_workspace.rs`)**:
  - Upgraded `SmartSelectionTarget::SketchEntity(id)` with computed geometry metrics (`Length`, `Area`, `Radius`), stroke width and RGB color presets (`update_entity_style`), `Duplicate (+500mm)`, and `Delete Entity (Undoable)`.
  - Added parametric numeric drafting inputs (`L / Angle` for `Line`, `W / H` for `Rectangle`, `R` for `Circle`, and `Clear` for `Geodesic Measure`) inside the top Active Tool Pill.
- **3D Grid & Earth Map Backdrop Toggles + `EarthSite` Focus 3D BIM (`src/views/build/mod.rs`, `src/views/build/viewport_3d.rs`)**:
  - Added `Grid` (`show_grid`) and `Map BG` (`show_3d_map_backdrop`) toggles to the Dynamic Island expansion strip and added `Focus 3D BIM` on `EarthSite` Smart Pop-Ups when a BIM model is present.
- **Gemini 2.5 Pro 2400-Token Budget, Native Timeout & `[[OPEN_SHEET:...]]` (`src/core/web_bridge.rs`, `src/views/spatial_workspace.rs`)**:
  - Added `build_gemini_generate_content_payload_for_model` raising `maxOutputTokens` to `2400` for Gemini Pro models (`1400` otherwise) plus a `35s` `--max-time` guard on native `curl` calls.
  - Added `GeminiDirective::OpenSheet(NavigationPillar)` parsed from `[[OPEN_SHEET:...]]` tags and routed through `PlannedProposalAction::OpenSheet`.

---

## [v0.5.1] - 2026-11-18 — Phase 51: Screenshot-Matched 3-Row Dynamic Island, Hover Extended Draw & Undo Deck, 3D BIM-on-Globe & Gemini 2.5 Pro

### Added & Changed
- **Dynamic Island Resting Layout & Hover Extension Deck (`src/views/spatial_workspace.rs`, `src/app.rs`, `docs/UI_ARCHITECTURE.md`)**:
  - Updated the initial resting Dynamic Island to display the clean 3-row floating translucent capsule (`Vault / Switch Vault / + Add Context / Model / Mode / Chat / History`, `Draw: Orbit Column Beam Wall Footing Line Rect Measure Terraform`, and the Composer bar).
  - Moving the cursor over Dynamic Island smoothly expands it upward to reveal the **Hover Extension Deck**: **Undo / Redo**, Canvas View switcher (`Earth 3D / BIM 3D / Plan 2D`), `API Key`, `Settings`, Structural Lenses (`Clean`, `DCR%`, `ARI`, `Moment`, `4D`), `Ortho`, `Fit`, `Export`, and **Extended Drawing Tools** (`Draw`, `Circle`, `Dim`, `Note`, `BoxSel`, `Erase`, `Uplift`, `Carve`, `Seawall`, `Bridge`, `CleanGrid`).
- **Streamlined Settings Menu (`src/views/settings.rs`)**:
  - Removed the `Future Roadmap` tab and legacy lock-queue UI from `Settings` so the menu opens straight into clean `Workspace Settings` and `About EigenGraph` tabs.
- **3D BIM Silhouette Over Google Earth Globe (`src/views/universal_canvas.rs`)**:
  - Projected 3D column heights and floor beams onto `GlobeScreenTransform::project_lat_lon_elev` when `globe_mode` and `ground_origin_anchored` are enabled.
- **Real `Gemini 2.5 Pro` Support, Cancel Thinking & Obsidian Note Upgrades (`src/views/spatial_workspace.rs`)**:
  - Updated `AgentModelChoice` to expose genuine `gemini-2.5-flash` and `gemini-2.5-pro` REST models, added a **Cancel** control on in-flight Gemini requests, added **Save Note** buttons on AI replies, and added inline **Edit / Save** support for Obsidian `KnowledgeBase/*.md` files.
- **Precision Footing/Beam/Wall Placement & Undoable Element Delete (`src/views/spatial_workspace.rs`)**:
  - Added `@(X,Y)` coordinate placement for `Footing`, polar `Length / Angle` span commit for `Beam` and `Wall`, and undoable `BimCommand::DeleteElement` buttons inside Element Smart Pop-Ups.

---

## [v0.5.0] - 2026-11-18 — Phase 50: Pure Spatial Corners, Dynamic Island Settings/View Switcher, Multi-Turn Gemini AI & Live 3D Ghost Preview

### Added & Changed
- **Pure Top-Left Spatial Canvas & Dynamic Island View/Settings Controls (`src/views/spatial_workspace.rs`, `docs/UI_ARCHITECTURE.md`)**:
  - Removed the persistent top-left capsule (`EigenGraph | Earth 3D | /`) so all four canvas corners stay completely unobstructed in idle Spatial Mode.
  - Added **Canvas View switcher (`Earth 3D` / `BIM 3D` / `Plan 2D`)** and a direct **Settings (`AppIcon::SettingsGear`)** button inside the hover-expanded Dynamic Island header, alongside mobile-clamped chat/drawer heights (`< 640px`).
- **Multi-Turn Gemini AI Conversation, Extended Directives & Opt-In Device Key Security (`src/core/web_bridge.rs`, `src/views/spatial_workspace.rs`, `src/views/settings.rs`, `src/app.rs`, `index.html`)**:
  - Upgraded `gemini_send_conversation` (`build_gemini_generate_content_payload`) to transmit recent multi-turn conversation history (`user`/`model` roles) across follow-up engineering queries.
  - Added extended structured directives (`[[RESIZE_COL:id,w,d]]`, `[[RESIZE_BEAM:id,b,h]]`, `[[ADD_COL:x,y,w,d]]`, `[[APPLY_SITE_SEISMIC]]`) routed cleanly through `Planning` interactive confirmation cards or `Fast` execution.
  - Added an **opt-in `"Remember key on this device"`** toggle synced between the Dynamic Island Key popover and Settings (`localStorage` on Web / isolated device config on Native; never serialized into `.eigen` exports).
- **Obsidian Vault Markdown Notes & Firefox Empty-Folder Fix (`index.html`, `src/core/vault.rs`, `src/views/spatial_workspace.rs`)**:
  - Fixed Mozilla Firefox / Safari macOS empty-folder selection in `<input webkitdirectory>` by keeping the native picker overlay open with inline Vault initialization when a selected folder returns `0` files.
  - Added an Obsidian-style **`+ Note (.md)`** creator and inline Markdown previewer in the Dynamic Island Vault drawer for `KnowledgeBase/*.md` engineering design notes.
- **Live 3D Wireframe Ghost Preview & Precision Placement (`src/views/build/viewport_3d.rs`, `src/views/spatial_workspace.rs`)**:
  - Added live scaled 3D wireframe footprint extrusion previews for `Column`, `Footing`, `Beam`, and `Wall` tools matching the active precision dimensions before canvas click, plus direct `@(X, Y)` coordinate placement in the active tool pill and site seismic/wind hazard display on Footing Smart Pop-Ups.

---

## [v0.4.9] - 2026-11-18 — Phase 49: Live Gemini API Dynamic Island, Planning Proposals & Cross-Browser Obsidian Vault

### Added & Changed
- **Cross-Browser Obsidian-Style Vault (Mozilla Firefox / Safari macOS Fix) (`index.html`, `src/core/vault.rs`, `src/core/web_bridge.rs`)**:
  - Added `<input type="file" webkitdirectory directory multiple>` paired with a guaranteed DOM user-activation overlay bar (`#eg_firefox_folder_overlay`) so Mozilla Firefox and Safari users on macOS can open real Finder folders into IndexedDB seamlessly without relying exclusively on Chromium's `window.showDirectoryPicker`.
  - Simplified the Vault layout to an Obsidian-style two-action model (**Open / Add Folder** and **Create Vault**) with automatic structured subfolders (`Projects/`, `KnowledgeBase/`, `Exports/`, `AI_Context/`) and starter documentation (`KnowledgeBase/Welcome.md`, `_VAULT_NOTICE.txt`), plus one-click `.eigen-vault.json` bundle export and drag/drop restore.
- **Deduplicated Unified Vault UI (`src/ui_kit.rs`, `src/views/settings.rs`, `src/app.rs`, `src/views/spatial_workspace.rs`)**:
  - Consolidated legacy Vault UI blocks from the Startup Modal, Settings Vault Card, and Dynamic Island Vault Drawer into a single shared helper (`ui_kit::render_obsidian_vault_panel`), eliminating duplicate platform-specific setup code across the codebase.
- **Live Async Gemini REST Engine & Quick Dynamic Island Key Setup (`src/core/web_bridge.rs`, `src/views/spatial_workspace.rs`, `src/app.rs`)**:
  - Wired `AgentModelChoice::Gemini25Flash` / `GeminiLive` to a non-blocking cross-platform Gemini `generateContent` REST client (`web_sys::Request` via `wasm_bindgen_futures::spawn_local` on WebAssembly; `curl` worker thread on native), bidirectionally synchronized with `Settings` and equipped with an inline Key Setup popover inside Dynamic Island, thinking indicator, automatic offline `EigenAI Pro` fallback, and structured action directive parsing (`[[FLY:...]]`, `[[BIM:...]]`, `[[REPAIR]]`, `[[ARI_AUDIT]]`).
- **Planning Mode Interactive Proposal Review Cards (`src/views/spatial_workspace.rs`)**:
  - Distinguished `Planning` vs `Fast` modes: `Planning` queues interactive `PlannedAgentProposal` cards with **Apply Changes** (transactional `BimCommand`) and **Reject** controls inside Dynamic Island before mutating BIM or Earth models.
- **Precision Manual Authoring & `"Anchor & Build Here"` (`src/views/build/mod.rs`, `src/views/build/viewport_3d.rs`, `src/views/spatial_workspace.rs`, `src/app.rs`)**:
  - Added precision width/depth/thickness/size dimension controls directly inside the active tool HUD pill for `Column`, `Beam`, `Wall`, and `Footing` authoring, plus an **"Anchor & Build Here"** button on clicked Earth Site Smart Pop-Ups to bind the BIM origin directly to any Earth coordinate and jump straight into 3D structural drafting.
- **Dynamic Island AI Session Persistence (`src/core/vault.rs`, `src/models.rs`, `src/views/spatial_workspace.rs`)**:
  - Automatically saves and restores Dynamic Island chat history and archived threads inside `Vault/AI_Context/dynamic_island_sessions.json`.

---

## [v0.4.8] - 2026-11-18 — Phase 48: True Full-Canvas Architecture Lock & Hover-Expandable Floating Dynamic Island

### Added & Changed
- **Removed Docked Right Side Panel — Back to 100% Unobstructed Earth & BIM Canvas (`src/app.rs`, `src/views/spatial_workspace.rs`)**:
  - Removed the docked right-hand sidebar and top-right launcher chrome from Spatial Mode so the 3D Google Earth / BIM viewport occupies 100% of the viewport permanently.
- **Hover-Expandable Floating Dynamic Island (`src/views/spatial_workspace.rs`)**:
  - Upgraded `render_dynamic_island` into an animated hover-responsive capsule (`animate_bool` with hysteresis grace zone) that remains a slim bottom pill when pointer is on the canvas, and automatically expands smoothly upward on hover (or tap/Cmd+K/focus) to surface:
    1. **`+ Add Folder (Vault)` & Vault Explorer Drawer** (OS folder picker / Web File System Access & IndexedDB binding, save-to-Vault, and project file `@` attachment).
    2. **`+ Add Context` (`@`)** searchable picker & removable context chips (`@Selected`, `@Earth`, `@BIM`, `@FEM/ARI`, `@4D/5D Cost`, `@Vault`, and local disk files).
    3. **AI Model & Planning Mode Selector** (`EigenAI Pro`, `Gemini 2.5 Flash`, `Gemini Live Stream` + `Planning` / `Fast` + floating chat thread & conversation history).
    4. **Direct Manual Drawing Commands (`SpatialManualTool`)**: One-click `Orbit`, `Column`, `Beam`, `Wall`, `Footing`, `Line`, `Rect`, `Measure`, and `Terraform` actions that switch tools immediately and fold away popovers as the cursor moves back onto the canvas to draw.
- **Architectural Direction Specification (`docs/UI_ARCHITECTURE.md`)**:
  - Codified the spatial UI contract (100% canvas, single hover Dynamic Island, click-to-inspect Smart Pop-Up, on-demand overlays) to keep future phases aligned.

---

## [v0.4.7] - 2026-11-18 — Phase 47: Google Antigravity AI Assistant Panel UI (`Add Folder -> Vault` & `Add Context`) + Earth Zoom Sensitivity Calibration

### Added & Changed
- **Google Antigravity-Style AI Assistant Side Panel (`src/views/spatial_workspace.rs`, `src/app.rs`)**:
  - Redesigned the spatial AI assistant into a collapsible right-hand **Antigravity Agent Side Panel** on desktop/tablet (`>= 760px`) and adaptive overlay sheet on narrow mobile (`< 760px`), retaining the slim bottom command launcher & top-right toggle capsule when collapsed so the Earth & BIM viewport remains 100% unobstructed on demand.
  - Built an authentic Antigravity composer and turn UX: top session header (`+` New Chat, History thread archive switcher, Classic mode switcher, Collapse toggle), Planning/Fast mode switch (`AgentExecutionMode`), Engine Model selector (`EigenAI Pro • FEM+ARI`, `Gemini 2.5 Flash`, `Gemini Live Stream`), multi-line Enter-to-send / Shift+Enter composer, starter empty-state hero cards, and turn-level `@Context` chips + Thought/Execution step trace pills.
- **Direct `+ Add Folder` Engineering Vault Binding & Vault Explorer (`src/views/spatial_workspace.rs`, `src/core/vault.rs`, `src/core/web_bridge.rs`, `index.html`, `src/app.rs`)**:
  - Embedded a live **Vault Workspace Header** directly inside the AI Assistant panel with a one-click **`+ Add Folder`** action invoking the OS directory picker (`pick_folder_native` + `InternalVault::connect_existing_vault`) on desktop or File System Access (`window.showDirectoryPicker`) / Browser IndexedDB on WebAssembly.
  - Added an inline collapsible **Vault Files Explorer** listing `.eigen` projects and `.json/.md/.txt/.csv/.ifc` Vault documents (`InternalVault::list_vault_context_files`) for 1-click canvas loading, direct save-to-Vault, or `@` context attachment.
- **Interactive `+ Add Context` (`@`) Mention System (`src/views/spatial_workspace.rs`, `src/app.rs`)**:
  - Added searchable `+ Add Context` picker (triggered from button, typing `@`, or `/context`) supporting live workspace domains (`@Selected`, `@Earth Site`, `@BIM Model`, `@FEM & ARI`, `@4D/5D Cost`, `@Vault Folder`), individual Vault files, and local disk file attachments (`trigger_file_picker` -> `LocalFile`), rendered as dismissible context chips above the composer and injected into agent reasoning.

### Fixed
- **3D Earth Globe Zoom & Drag Sensitivity Spike (`src/core/gis/earth_globe.rs`, `src/views/universal_canvas.rs`, `src/views/build/viewport_3d.rs`)**:
  - Replaced the horizon-limb square-root approximation in `GlobeScreenTransform::deg_per_pixel()` with exact tangent-plane perspective scaling `(d_surf / f_px) * (180 / pi)` plus tilt foreshortening compensation (`deg_per_pixel_y()`), eliminating the 25x–340x pan oversensitivity bug when zooming into city and site scales.
  - Added zoom-tapered wheel/pinch zoom (`zoom_delta()` support), two-pass exact ray-sphere cursor anchoring (`apply_globe_zoom_at_screen`) for both zoom-in and zoom-out, and automatic suppression of inertial coasting above street/site zoom (`zoom >= 11.0`).

---

## [v0.4.6] - 2026-11-18 — Phase 46: Antigravity Spatial Canvas Mode, Unified Slash Command Palette (`/`) & Smart Context Pop-Up Inspector

### Added & Changed
- **Zero-Chrome Full-Viewport Spatial Workspace (`src/views/spatial_workspace.rs`, `src/app.rs`, `src/views/universal_canvas.rs`, `src/views/build/viewport_3d.rs`)**:
  - Introduced `SpatialWorkspaceState` (`spatial_mode_enabled: true` default on startup) rendering an unobstructed, edge-to-edge full-viewport **3D Google Earth Globe**, **2D Ground Plan**, or **3D Structural BIM** canvas (`UniversalCanvasState::show_spatial_fullscreen`, `BuildView::show_spatial_fullscreen`) with live canvas interaction unlocked immediately and a clean `/classic` fallback (with `"Antigravity Spatial Mode"` return toggle in the classic sidebar).
  - Eliminated permanent left sidebars, below-viewport toolbars, and startup blocking modals from the spatial canvas, replacing them with a minimal frosted top-left capsule (`EigenGraph` + mode switcher + `/` launcher), contextual top-center active lens/tool pill, and on-demand translucent overlay sheets for all 15 engineering modules.
- **Unified Antigravity AI Assistant & `/` Command Palette (`src/views/spatial_workspace.rs`, `src/app.rs`)**:
  - Unified `Cmd/Ctrl+K` command focus, slash command autocomplete (`/earth`, `/3d`, `/2d`, `/sample`, `/portal`, `/lens dcr|ari|moment|axial|4d`, `/repair`, `/svd`, `/vault`, `/analysis`, `/schedule`, `/cost`, `/sustainability`, `/facility`, `/safety`, `/construction`, `/automation`, `/prediction`, `/report`, `/settings`, `/classic`), `@Element` context mentions, geodetic city fly-to navigation, and natural-language BIM synthesis with execution trace chips.
- **Smart Element Pop-Up Inspector & Transactional BIM Resize (`src/core/bim.rs`, `src/views/universal_canvas.rs`, `src/views/build/viewport_3d.rs`)**:
  - Added deterministic BIM-to-FEM element mapping (`BimProject::to_structural_model_with_map`, `fem_id_for_bim_element`, `bim_ref_for_fem_id`) and live FEM utilization/ARI fusion inside `BuildView::build_model_geometry`.
  - Added screen-space BIM picking on both the Earth/2D canvas (`pick_bim_element_at_screen`) and 3D BIM viewport (`Column`, `Beam/Run`, `Wall`, `Footing`, `SketchEntity`, and open `EarthSite` coordinates) triggering a frosted, responsive **Smart Element Pop-Up** (`Props` with transactional `ResizeColumn`/`ResizeRun`/`ResizeFooting`/`ResizeWall` undoable edits, `Calc` with live `DCR%`, `ARI`, `Pu`, `P-M-V`, `SVD margin` & `Simulate N-1`, `5D/6D` volume/cost/carbon, and `Ask AI` contextual actions).

---

## [v0.4.5] - 2026-11-15 — Phase 45: External Below-Toolbar Detailing & Structural Sub-Menus, Earth-Coupled Lifecycle Ops & Cross-Pillar Bridge

### Added & Changed
- **External Below-Toolbar Sub-Menus in Drafting & Geometry (`src/views/parametric/`, `src/views/build/`)**:
  - Restructured **Drafting** so the upper 2D canvas remains strictly for interactive CAD/BIM authoring (`Floor Plan`, `Section Cut`, `Sheet 2D`, `Dynamo Graph`, plus `Rebar [RF]` tool), while moving **Rebar BBS** and **Elevations & MEP Riser** outside below the toolbar alongside the new **MEP Engineering Sizing** studio inside an expanded-by-default collapsible panel.
  - Restructured **Geometry** by moving 3D structural diagram overlays (`NFD`, `SFD`, `BMD`, `Heat`), FEA/N-1 screening controls, and structural badges out of the upper 3D viewport toolbar into a dedicated below-toolbar **Structural Analysis** sub-menu panel (`3D Diagrams & Heatmap`, `FEA Solver & N-1`, `Critical Check & -> Analysis`) without merging the `Geometry` and `Analysis` pillars.
- **Four Lifecycle Execution & Engineering Additions (`src/core/lifecycle_ops.rs`, `src/views/planning.rs`, `src/views/lean.rs`, `src/views/parametric/detailing_2d.rs`)**:
  - Added interactive **MEP Engineering Sizing** in `Drafting` (HVAC CFM/TR & GI duct velocity/friction, Plumbing WSFU/DN/pump head, and 3-phase IEC kVA/breaker/Cu feeder sizing with 1-click BIM area sync and live BIM pipe/cable-tray run creation).
  - Added **Procurement, Contracts, IPC Claims & VO** (`Budgeting`), **Site Daily Diary & Heavy Fleet Log** (`Scheduling`), and **Field QA/QC Cylinder Lab, ITP Hold Points, NCR Punch List & MEP T&C** (`Construction`), all persisted cleanly in `UnifiedWorkspaceArchive`.
- **Earth-to-Site Environmental Coupling, Cross-Pillar Bridge & Width Parity (`src/core/lifecycle_ops.rs`, `src/views/analyze/`, `src/views/universal_canvas.rs`, `src/ui_kit.rs`, `src/app.rs`)**:
  - Coupled live Earth Globe coordinates and terraforming defenses (`compute_site_environmental_coupling`) to seismic/wind hazard screening (`Analysis`), rain/wind downtime (`Scheduling`), remote logistics factors (`Budgeting`), and geotechnical/concrete QA/QC alerts (`Construction`).
  - Added bidirectional **Geometry ↔ Analysis ↔ Drafting** jump actions, updated the Analysis header `"Vault"` quick-link label, hardened `ui_kit::responsive_grid_n` / `ui_kit::stat_card` against column rounding overflow across `390px / 700px / 1000px` viewports, and verified 198 passing unit tests with zero warnings.

---

## [v0.4.4] - 2026-11-15 — Phase 44: Streamlined Sidebar Module Nomenclature

### Changed
- **Standardized Concise English Sidebar Names (`src/models.rs`, `src/app.rs`, `src/views/my_project.rs`, `src/views/parametric/inspector.rs`, `AGENTS.md`, `README.md`)**:
  - Updated `NavigationPillar::clean_name` (and corresponding titles/navigation labels) across the 15 sidebar modules to crisp single-word English engineering workflow names:
    - `My Library` ➔ `Vault` (`icon`: `"VLT"`)
    - `AI Agent` ➔ `Agent`
    - `Research` ➔ `Planning`
    - `Parametric` ➔ `Drafting`
    - `Time & Scheduling` ➔ `Scheduling`
    - `Cost & Budgeting` ➔ `Budgeting`
    - `Sustainability & Energy` ➔ `Sustainability`
    - `Facility Management` ➔ `Facility`
    - `Safety & Health` ➔ `Safety`
    - `Lean Construction` ➔ `Construction`
    - `Industrialized & Automation` ➔ `Automation`
    - Retained `Geometry`, `Analysis`, `Prediction`, and `Report`.
  - Updated cross-navigation buttons (`"Open 2D Drafting Canvas"`, `"Budgeting (5D BoQ)"`, `"Planning TIN terrain"`) and added unit test coverage verifying all 15 sidebar labels.

---

## [v0.4.3] - 2026-11-15 — Phase 43: Interactive 3D Google Earth Globe Canvas, Planetary Terraforming & Megaproject Sandbox Game

### Added & Changed
- **Default 3D Spherical Earth Engine (`src/core/gis/earth_globe.rs`, `src/core/gis/map_widget.rs`, `src/core/gis/mod.rs`)**:
  - Added a perspective 3D WGS84 spherical Earth viewport (`globe_mode: true` default on startup, toggleable back to `2D Map` via toolbar button or `G` shortcut) featuring a deterministic starfield backdrop, multi-ring Rayleigh atmosphere limb glow, optional solar terminator day/night hemisphere shading, tilted horizon camera (`0°..65°` tilt & `0°..360°` heading, `[` / `]` keys or `Alt+Drag`), smooth spherical orbit with angular momentum inertia, auto-spin, and animated Google Earth Fly-To camera swoops on `"World"`, `"Go"`, `"Ground 1:200"`, and Mission changes.
  - Wrapped live satellite/map raster tiles (`EsriWorldImagery` default) and `WORLD_LANDMASSES` continental polygons onto spherical triangles (`ear_clip_polygon` reuse + `collect_globe_tile_quads`) with `f64` ray-sphere pointer picking (`GlobeScreenTransform::screen_to_lat_lon`).
- **Combined Planetary Terraforming & Megaproject Game Loop (`src/core/gis/earth_globe.rs`, `src/views/universal_canvas.rs`, `src/icons.rs`, `src/app.rs`)**:
  - Added terrain/climate sculpting brushes (`Uplift`, `Carve`, `Reclaim`, `Forest`, `Solar`, `Lake`, `Seawall` — keyboard shortcuts `1..7`, configurable `S`/`M`/`L` brush radii), geodesic megastructures (`Mega-Bridge`, `Mega-Tower`, `Clean Grid` — shortcuts `8..9`), coastal inundation feedback (`Sea Level -50..+50 m`), live planetary telemetry (`Year`, `CO2 ppm`, `Temp ΔT`, `Habitability 0-100`, `Flooded Coast`, `Clean GW`, `Budget $B`), Earth Undo/Redo/Reset, Play/Pause simulation ticks (`+1 Yr`), and four playable missions (`Freeplay Earth`, `Coast +3m Defense`, `Green Nusantara`, `Sahara Solar Oasis`).
  - Integrated external Earth Sandbox dock below the canvas toolbar matching full canvas width (`390px` Mobile, `700px` Tablet, `1000px` Desktop), keeping the 3D Earth viewport clean (Freeze pill only inside the canvas) with coordinate/vitals telemetry bar directly below the viewport, full archive persistence (`UnifiedWorkspaceArchive` v1 SHA-256), and Great-Circle geodesic CAD/BIM projection onto the curved sphere.

---

## [v0.4.2] - 2026-11-14 — Phase 42: Live BIM-FEA Solvers, CAD OSNAP & Marquee Promotion, Reactive 4D/5D/9D Sync, Closed-Loop AI Repair & Lean Width Fix

### Added & Changed
- **Lean Construction Canvas Width Parity & Live BIM Rebar BBS (`src/views/lean.rs`, `src/core/cso.rs`, `src/views/parametric/mod.rs`)**:
  - Fixed horizontal width overflow on `"Cutting Optimization Parameters"` and `"Optimized Steel Bar Cutting Patterns"` by constraining all card containers to canvas width (`ui.set_width` / `ui.set_max_width`), refactoring parameter controls into `ui_kit::responsive_grid_n` with clamped slider/combo widths, wrapping wide tables in horizontal `ScrollArea`, adding `CuttingStockOptimizer::extract_bbs_from_bim_rebar` + CSV downloads, and expanding `test_below_canvas_module_matches_canvas_width_parity` to assert width parity across all 6 `LeanSubTab` views and `PlanningView` at Mobile (`390px`), Tablet (`700px`), and Desktop (`1000px`).
- **Live `BimProject` FEA Coupling & 3D/2D Structural Heatmaps (`src/views/analyze/`, `src/views/build/viewport_3d.rs`, `src/views/parametric/`, `src/core/bim.rs`)**:
  - Connected `AnalyzeView` (`N-2` Woodbury perturbation, cascading collapse, pushover, modal dynamics, geotech SSI, and P-M-V interaction KPIs) directly to active `BimProject::to_structural_model()` with automatic BIM-signature invalidation and Warren Truss fallback when fewer than 2 elements exist.
  - Scaled `BimProject::to_structural_model()` beam and column `CrossSection` stiffness directly from BIM element dimensions (`width_mm`, `depth_mm`) and added interactive 3D heatmap cycle (`Heat: ARI / DCR% / ±Force / Critical`) in `Build` plus `ARI Heat` 2D floor plan overlays in `Parametric`.
- **Precision CAD OSNAP, Marquee Selection & BIM Promotion (`src/views/universal_canvas.rs`, `src/app.rs`)**:
  - Added Object Snap (`OSNAP` `F4` toggle + reticle indicators for `END`, `MID`, `PERP`, and `INT` across sketch entities and BIM walls/beams/columns/grids), Window (left-to-right enclosed) & Crossing (right-to-left intersecting) `BoxSel` (`V`) multi-entity selection across Move/Copy/Offset/Mirror/Rotate/Delete, `GRID` and `ARRAY` generators, and `PROMOTE` (`To BIM`) converting 2D sketch lines and rectangles into native 3D BIM walls, beams, columns, and slabs.
- **Reactive 4D/5D Scheduling & Closed-Loop EigenAI Auto-Repair + Installable Web PWA (`src/views/planning.rs`, `src/views/ai_assistant.rs`, `src/app.rs`, `manifest.webmanifest`, `sw.js`, `index.html`)**:
  - Added reactive `PlanningView::sync_from_bim` & `compute_takeoff_with_bim` updating 4D stages, CSI WBS cost tables, concrete volumes, and rebar quantities whenever the active BIM model changes.
  - Implemented deterministic transactional `BimCommandBus::repair_critical_members` with Undo/Redo support (`repair critical members` prompt & 1-click Copilot `"Auto-Repair ARI"` button) verifying before/after Min ARI & Max DCR deltas via `AriEngine`.
  - Added PWA web app manifest (`manifest.webmanifest`) and WASM-safe offline shell service worker (`sw.js`) wired via Trunk copy links in `index.html`.

---

## [v0.4.1] - 2026-11-14 — Phase 41: Canvas Width Parity, Precision CAD Tools & Full 74/74 Roadmap Unlock

### Added & Changed
- **Below-Canvas Width Parity & Zero Compiler Warnings (`src/app.rs`, `src/ui_kit.rs`, all 12 pillar views, `src/core/gis/map_widget.rs`)**:
  - Unified central workspace column width (`ui.set_width` / `ui.set_max_width`), removed redundant outer horizontal margins across all 12 below-canvas views, aligned docked inspectors and subtab bars directly with the drawing canvas width across Mobile, Tablet, and Desktop breakpoints, and added unit test coverage (`test_below_canvas_module_matches_canvas_width_parity`, `test_world_tile_cache_decode_and_ready_state`) leaving zero warnings across targets.
- **Outside-Viewport CAD Precision Controls & Global Quick-Jump (`src/views/universal_canvas.rs`, `src/views/parametric/`, `src/views/build/viewport_3d.rs`, `src/app.rs`)**:
  - Kept drawing viewports completely uncluttered while expanding bottom toolbars with configurable numeric `Step (mm)`, `Copy/Duplicate`, `Rotate 90°`, `@dx,dy` relative and `@dist<deg` polar CLI expressions, and a `? Shortcuts` / `Ctrl/Cmd+K` / `F1` global CAD shortcut reference and module quick-jump modal.
- **Complete Phase 35 Engineering Capability Suite Unlocked (74/74 Active, 0 Locked across `src/features.rs`)**:
  - `AI-05` (`src/core/parametric.rs`, `src/views/ai_firm.rs`): NSGA-II multi-objective structural Pareto frontier solver (weight, cost, embodied CO2e, ARI) with interactive scatter frontier and one-click `"Apply Selected -> BIM"`.
  - `SIM-08` (`src/views/analyze/extended_mechanics.rs`, `src/views/analyze/mod.rs`): AISC Design Guide 11 floor footfall vibration and EN 1993-1-9 / AISC 360 bilinear S-N Miner cumulative fatigue solver with interactive S-N plot.
  - `3D-06` (`src/views/build/bottom_deck.rs`, `src/views/build/mod.rs`): Point cloud as-built deviation auditor with ACI 117 column plumbness, ASTM E1155 FF/FL calculation, XYZ cloud ingestion, and deviation CSV export.
  - `5D-04` (`src/views/planning.rs`): Normalized bid-leveling tender matrix with front-loading outlier detection, composite deck VE swap, ARI guardrail check, CSV and award memo export.
  - `6D-04` (`src/views/sustainability.rs`): ISO 20887 / EN 15978 Module D circular material passport and MCI calculator tied to live BIM quantities and steel production routes with JSON passport export.
  - `7D-04` (`src/views/facility.rs`): Interactive UAV photogrammetry crack & spalling inspector against ACI 224R limits, CSV ingestion, clickable orthomosaic defect pins, and CMMS work order dispatch.
  - `8D-04` (`src/views/safety.rs`): Digital Permit-to-Work (PTW) workflow with automated crane wind-speed lockdown interlock (`>= 55 km/h`), confined-space gas test gates, and NFPA 101 evacuation drill logger.
  - `9D-04` (`src/views/lean.rs`): BIM-pinned RFI & submittal SLA countdown tracker coupled directly to Lean Last Planner System constraint clearing and CSV export.
  - `10D-05` (`src/views/industrialized/mod.rs`, `src/views/industrialized/automation_slots.rs`): Parametric 3D concrete printing (3DCP) perimeter slicer with thermal cold-joint open-time window validation and `.gcode` toolpath export.
  - `PRED-03` & `PRED-04` (`src/views/prediction.rs`): Fick's 2nd Law `erfc()` chloride ingress & carbonation depassivation service-life curve solver + ATC-20 post-earthquake Green/Yellow/Red wing placard triage coupled to live PGA and 7D SHM strain.
  - `PUB-05` (`src/views/results.rs`): Pure-Rust FIPS 180-4 SHA-256 As-Built handover completeness auditor and tamper-evident custody manifest JSON packager.

## [v0.4.0] - 2026-11-14 — Phase 40: Clean Viewports, Aligned Coordinate Bar, Full CAD Shortcuts & Complete Phase 34 Capability Suite

### Added & Changed
- **Clean Uncluttered Viewports, Aligned Coordinate Bar & Docked Inspector Architecture (`src/views/universal_canvas.rs`, `src/views/parametric/`, `src/views/build/viewport_3d.rs`)**:
  - Removed extra HUD text and floating inspector overlays from inside all three drawing canvases (**Universal Earth Canvas**, **Parametric 2D Drafting**, and **Geometry 3D Earth BIM**), keeping only the geometry/map and the top-left Freeze pill inside each viewport.
  - Standardized the vertical canvas hierarchy across all canvases: **Viewport -> Coordinate & Telemetry Bar -> Notifications -> Rich Bottom Toolbar -> Docked Properties/Inspector Panel**.
- **Full CAD Keyboard Shortcuts & Rich Bottom Modify Toolbars (`src/core/bim.rs`, `src/views/universal_canvas.rs`, `src/views/parametric/`, `src/views/build/viewport_3d.rs`)**:
  - Added guarded CAD shortcut dispatch (`!ctx.egui_wants_keyboard_input()`): `Space` (Freeze toggle), `Esc`, `Del`/`Backspace`, `Ctrl/Cmd+Z` (Undo), `Ctrl/Cmd+Y` or `Ctrl/Cmd+Shift+Z` (Redo), `F8` (Ortho), `F3` (Snap), `Z` (Fit Model), plus drafting/modify hotkeys (`W`/`B`/`C`/`F`/`L`/`R`/`D`/`M`/`O`/`X`).
  - Enriched bottom toolbars with **Redo**, **Ortho**, **Move +1m**, **Offset Wall**, **Mirror**, **Delete**, and **Auto Rooms**, and extended `BimCommandBus` + `BimCommand::MirrorElements` across walls, structural runs, and columns.
- **Completed & Unlocked All 10 Remaining Phase 34 Engineering Modules (`src/features.rs`: 62 Active / 12 Locked)**:
  - **PERT 3-Point & Monte Carlo Schedule Risk (`4D-03`: `src/views/planning.rs`)**: Latin Hypercube Sampling (LHS) 1,000+ trial simulation outputting P50/P80/P95 completion dates, histogram/S-curve canvas, sensitivity drivers, and CSV export.
  - **Daily Site Crew Histogram & Resource Leveling (`4D-04`: `src/views/planning.rs`)**: Float-constrained crew peak smoothing for rebar, formwork, and concrete trades with live daily peak stacked bars and CSV export.
  - **Earned Value Management & Progress Claim S-Curve (`5D-03`: `src/views/planning.rs`)**: Live `PV`/`EV`/`AC`, `CPI`/`SPI`, `EAC`/`VAC`, retention/advance-payment net claim certificates, and CSV export.
  - **SNI 6389 Solar OTTV & Facade Daylight (`6D-03`: `src/views/sustainability.rs`)**: N/E/S/W facade wall & glazing OTTV (`W/m²`), solar flux compass, `sDA %`, cooling load savings, and CSV audit export from live `BimProject` walls/openings.
  - **Live WebSocket/MQTT IoT Sensor Stream Ingestion (`7D-02`: `src/core/web_bridge.rs`, `src/views/facility.rs`)**: Cross-platform `WebSocket` SHM stream client + JSON/CSV telemetry packet parser updating strain/frequency sensors, sparklines, and CSV archives.
  - **MEP Riser Valve Isolation Tree & Space Utilization (`7D-03`: `src/views/facility.rs`, `src/app.rs`)**: Interactive shutoff valve blast-radius analysis and Gross-to-Net space efficiency driven by live `BimProject` slabs and rooms.
  - **ACI 347 Slab Formwork Shoring & Reshoring (`8D-03`: `src/views/safety.rs`)**: ACI 209 concrete maturity vs `70% f'c` strip threshold, multi-level shore load transfer, Ring-Lock prop capacity, Euler buckling FS, and CSV export.
  - **Zone Takt-Time Cadence & Ready-Mix Dispatch (`9D-03`: `src/views/lean.rs`)**: BIM concrete volume pour batching, mixer truck headway intervals, and 90-min ASTM C94 slump cold-joint monitoring.
  - **Precast Trailer Routing Envelope & Grouted Splice Tolerance (`10D-04`: `src/views/industrialized/`)**: GVW axle limits, highway portal clearances, and PCI MNL-135 laser-tracker splice delta verification across `BimProject` runs/columns.
  - **Live FEA & SHM Stress Anomaly Forecasting (`PRED-02`: `src/views/prediction.rs`, `src/app.rs`)**: Fuses active FEM axial forces, BIM element density, and 7D SHM strain/frequency trends into live $\beta$, stiffness ratio, RUL, and vulnerability rankings.
- **Version Bump**: Incremented crate version sequentially from `0.3.9` to `0.4.0` in `Cargo.toml`, `README.md`, `src/views/settings.rs`, and `docs/CHANGELOG.md`.

---

## [v0.3.9] - 2026-11-14 — Phase 39: Viewport-First Canvas Layout, Top-Left Floating Freeze Toggle, Auto-Fit Camera & 8 Phase 34 Engineering Modules

### Added & Changed
- **Viewport-First Canvas Layout & Top-Left Floating Freeze Toggle (`src/views/universal_canvas.rs`, `src/views/parametric/`, `src/views/build/viewport_3d.rs`)**:
  - Inverted the canvas stack order across all three interactive drawing viewports (**Universal Earth Canvas**, **Parametric 2D Drafting**, and **Geometry 3D Earth BIM**) so the main drawing viewport renders **first at the top**, with toolbars, CAD ribbons, telemetry strips, and inspector drawers cleanly docked **below**.
  - Removed the Freeze button from all canvas toolbars and introduced an interactive top-left floating pill (**`Frozen • Unfreeze`** / **`Live • Freeze`**) inside each viewport using shared `UniversalCanvasState::{floating_freeze_pill_rect, interact_floating_freeze_pill, paint_floating_freeze_pill}` helpers with `sidebar_toggle_inset_px` clearance and zero click-through into pan, zoom, orbit, or drawing gestures.
- **Auto-Fit Camera, `Fit Model` Controls & Direct Sheet Vector Exports (`src/core/bim.rs`, `src/views/universal_canvas.rs`, `src/views/parametric/ribbon.rs`, `src/app.rs`)**:
  - Added `BimProject::plan_extents_mm`, `UniversalCanvasState::fit_world_to_bim_extents`, `ParametricView::fit_view_to_project`, and `BuildView::fit_view_to_project`, automatically fitting the active camera when opening Vault projects, converting IFC models to editable `BimProject`, or instantiating structural templates.
  - Added direct **PDF**, **DXF**, **SVG**, and **All Pack** vector export buttons inside the Parametric Sheet A101 ribbon bar (`SheetExportFormat`) and added shared `crate::core::web_bridge::save_text_file` for seamless browser downloads on WebAssembly and Vault `Exports/` output natively.
- **Activated 8 Additional Phase 34 Engineering Modules (`src/features.rs`: 52 Active / 22 Locked)**:
  - **Automated Multi-Code Compliance Auditor (`AI-04`: `src/views/ai_firm.rs`)**: Evaluates live `BimProject` and FEM `StructuralMember[]` compliance against SNI 1726/2847/1729 and ACI/AISC clauses (`drift`, `axial utilization`, `ARI redundancy`, `confinement hoops`, `column-footing continuity`) with Markdown report export.
  - **Retaining Wall & Bishop Simplified Slope Stability (`SIM-07`: `src/views/analyze/extended_mechanics.rs`, `src/views/analyze/mod.rs`)**: Computes Rankine `Ka`, Mononobe-Okabe dynamic `Kae`, overturning/sliding factors of safety with shear-key passive resistance, toe bearing pressure, and iterative Bishop Simplified static/seismic circular slip stability.
  - **Geotech Borehole Vs30 & Hydrology IDF/Rational Runoff (`1D-05`, `1D-06`: `src/views/research.rs`)**: Interactive borehole stratigraphy (`BH-01`, `BH-02`, `CPT-04`) with editable SPT `N60` layers, `Vs30` seismic site classification, Seed-Idriss liquefaction screening, CSV export, plus Mononobe IDF curves, Rational peak runoff (`Q = 0.278 C i A`), and minimum flood-safe finished floor elevation (`FFE`).
  - **Automated N/E/S/W Orthographic Elevations & Vertical MEP Riser Schedule (`2D-06`: `src/views/parametric/detailing_2d.rs`)**: Projects North/East/South/West orthographic facades and datum levels from live `BimProject` elements with SVG export and storey-scaled HVAC/CHW/Fire Standpipe riser schedules.
  - **3D Tower Crane Perimeter & Clearance Radar (`8D-02`: `src/views/safety.rs`, `src/app.rs`)**: Evaluates tower crane jib sweep, mast height vs live building roof elevation, and horizontal buffer clearance on an interactive plan radar.
  - **PBG/SIMBG & IBC HTML Calculation Book Composer (`PUB-04`: `src/views/results.rs`)**: Generates jurisdiction-aware HTML calculation books directly from live `BimProject` quantities, concrete/rebar totals, and FEM member utilization envelopes.
  - **ISO 19650 CDE Stateful Container Workflow (`LIB-04`: `src/views/my_project.rs`)**: Interactive `WIP -> Shared -> Published -> Archive` container promotion, filtering, package addition, and `.json` transmittal manifest export.
- **Version Bump**: Incremented crate version sequentially from `0.3.8` to `0.3.9` in `Cargo.toml`, `README.md`, `src/views/settings.rs`, and `docs/CHANGELOG.md`.

---

## [v0.3.8] - 2026-11-14 — Phase 38: Rounded Drawing Canvas Chrome, Parametric Width Overflow Fix, Phase 33 Unlocks, Vector DED Downloads & IFC-to-BimProject

### Added & Changed
- **Rounded Canvas Chrome & Responsive Parametric Overflow Fix (`src/ui_kit.rs`, `src/icons.rs`, `src/views/universal_canvas.rs`, `src/views/parametric/`, `src/views/build/`)**:
  - Updated `CornerRadius::same(12)` outer canvas frames, `CornerRadius::same(8)` inner top toolbars and bottom footers, and added viewport border spandrel painting (`UniversalCanvasState::paint_rounded_viewport_border`) so drawing canvases have rounded corners matching their sub-tab controls.
  - Resolved `egui 0.36.2` `horizontal_wrapped` pre-wrap overflow (`available_size_before_wrap().x`) via `ui_kit::wrap_before` / `clamped_row_width` and bounded Frame inner widths across Desktop (`1000px`), Tablet (`700px`), and Mobile (`390px`) with zero horizontal clipping.
- **Activated All 5 Remaining Phase 33 Modules (`src/features.rs`)**:
  - **Multi-Typology Project Templates (`LIB-05`: `src/views/my_project.rs`, `src/app.rs`)**: Added on-demand structural templates (`Steel Bridge Viaduct`, `Dual-System RC Tower`, `Post-Tensioned Box Girder Flyover`, `Precast Industrial Logistics Hub`) generating live `BimProject`, analytical FEM members, and Earth sync strictly on user selection (startup workspace remains blank).
  - **Automated RC Rebar BBS Drafter (`2D-05`: `src/views/parametric/detailing_2d.rs`, `src/views/parametric/mod.rs`)**: Connected live `BimProject` concrete column/beam/footing selection, `"Auto-Arm All RC Elements"` rebar synthesis, scaled section/elevation rendering, confinement zones (`Lo`), and live ISO 3766 Bar Bending Schedule cut tables.
  - **Federated 3D Discipline Isolation, 3-Axis Section Box & 4D Erection Highlight (`3D-05`: `src/views/build/mod.rs`, `src/views/build/sidebar.rs`, `src/views/build/viewport_3d.rs`, `src/app.rs`)**: Implemented live `ARC`/`STR`/`MEP`/`GEO` visibility filtering, full 3-axis `(X, Y, Elev Z)` Section Box triangle & wireframe clipping, footing 3D mesh rendering, and live 4D Erection Stage viewport coloring synchronized with `Time & Scheduling`.
  - **Bridge Moving Load & Influence Line Analyzer (`SIM-06`: `src/views/analyze/extended_mechanics.rs`, `src/views/analyze/mod.rs`)**: Implemented closed-form 1-span simple and 2-span continuous influence-line and moving-truck solver (`AASHTO HL-93` and `SNI 1725:2016 BGT/BTR`), dynamic impact allowance (`IM`), interactive axle stepping, and live `+Mmax` / `-Mpier` / `Vmax` plotting.
  - **Live BIM Rebar BBS to 1D Cutting Stock Sync (`9D-02`: `src/core/cso.rs`, `src/views/lean.rs`, `src/app.rs`)**: Added `CuttingStockOptimizer::extract_items_from_bim_rebar` and live source toggle in `Lean Construction` to optimize active `BimProject` rebar bar marks (`D10–D32`) into 12 m commercial stock cutting patterns.
- **Real Vector DED Downloads & IFC-to-Editable `BimProject` Converter (`src/core/bim.rs`, `src/views/results.rs`, `src/core/ifc.rs`, `src/views/industrialized/`)**:
  - Added `BimProject::export_sheet_dxf` (ASCII DXF `AC1015` with organized CAD layers `S-GRID`, `S-WALL`, `S-COLS`, `S-BEAMS`, `S-SLAB`, `S-ANNO-DIMS`) and wired PDF (ISO A1), DXF, and SVG exports in `Report -> Drawing Sheets` to trigger browser downloads on WebAssembly or write to the Vault `Exports/` directory natively.
  - Implemented `IfcModel::to_bim_project` and `"Convert IFC -> Editable BimProject"` in OpenBIM IFC Studio to convert parsed IFC2x3/IFC4 elements into editable native BIM objects across Geometry, Parametric, and Analysis.
- **Version Bump**: Incremented crate version sequentially from `0.3.7` to `0.3.8` in `Cargo.toml`, `README.md`, `src/views/settings.rs`, and `docs/CHANGELOG.md`.

---

## [v0.3.7] - 2026-11-14 — Phase 37: Full 1-File Workspace Save/Load, 3D Earth BIM Authoring, Reactive BIM-to-FEM & Clash Sync, Browser File Drop & Text-to-BIM

### Added & Changed
- **Full 1-File Workspace Archive & Auto-Save Sync (`LIB-03`: `index.html`, `src/core/web_bridge.rs`, `src/core/vault.rs`, `src/views/my_project.rs`, `src/app.rs`)**:
  - Upgraded Personal Vault save/load in `My Library` to store and restore complete `UnifiedWorkspaceArchive` `.eigen` files (all 15-module `UniversalCanvasState` drawings, `BimProject`, `StructuralMember[]`, `ParametricVariable[]`, Earth map origin/camera/zoom, and `WorkspaceCanvasSettings`) via File System Access API, IndexedDB, and native files.
  - Added browser IndexedDB + localStorage + native disk auto-save hydration and throttled background snapshot persistence (`autosave_workspace_json`, `load_autosaved_workspace_json`).
- **Interactive 3D Earth Ground BIM Authoring, Location Search & WGS84 Measurement (`1D-04`, `2D-04`: `src/core/gis/map_widget.rs`, `src/views/universal_canvas.rs`, `src/views/parametric/canvas_2d.rs`, `src/views/build/mod.rs`, `src/views/build/viewport_3d.rs`, `src/views/research.rs`)**:
  - Added `Ground3DAuthorTool` (`Orbit`, `+Col`, `+Wall`, `+Beam`, `+Foot`) inside `Geometry` with active datum plane raycasting, magnetic 450mm structural node / 500mm grid snapping, cyan targeting reticle, and orange 2-click rubber-band preview.
  - Added offline global city / `"lat, lon"` location search and geodesic Distance (`m`/`km`) & Area (`m^2`/`ha`) measurement tools across Universal Canvas, Parametric 2D, and Geometry 3D Earth viewports.
  - Synchronized Earth camera coordinates, zoom levels, and basemap tile sources bidirectionally between `Research` and all ground drawing canvases, plus `.csv`/`.xyz` survey point cloud TIN ingestion.
- **Reactive BIM-to-FEM/ARI Sync & Real `Aabb3D` Spatial Clash Engine (`SIM-05`, `3D-04`: `src/core/csg.rs`, `src/views/build/clash_bcf.rs`, `src/views/analyze/header.rs`, `src/views/analyze/mod.rs`)**:
  - Wired automatic bidirectional 2D `<->` 3D `BimProject` synchronization and live `BimProject -> StructuralMember[]` + `AriEngine::evaluate_redundancy` updates alongside the `Compile & Solve Live BIM -> FEM` trigger in `Analysis`.
  - Replaced clash placeholder actions with real 3D bounding-box interference computation (`Aabb3D::overlap_with_tolerance`) across active walls, columns, beams, MEP runs, and footings, mapped accurately to 3D wireframe markers.
- **Browser Local File Picker & Global Drag-and-Drop Importer (`10D-03`: `index.html`, `src/core/web_bridge.rs`, `src/views/industrialized/openbim_studio.rs`, `src/app.rs`)**:
  - Added HTML5 file input & global `dragover`/`drop` listeners plus native OS file dialogs for importing `.ifc` OpenBIM models into OpenBIM Studio, `.eigen`/`.json` archives into the active workspace, and `.csv`/`.xyz` survey clouds into Research TIN.
- **Text-to-BIM Command Synthesizer & Live Multi-Pillar QTO/LCA/DED Sync (`AI-03`, `4D-02`, `5D-02`, `6D-02`, `PUB-03`: `src/core/bim.rs`, `src/views/ai_assistant.rs`, `src/views/ai_firm.rs`, `src/views/planning.rs`, `src/views/sustainability.rs`, `src/views/results.rs`)**:
  - Implemented deterministic `BimCommandBus::execute_natural_language` (`portal 3x2 @6m 2floors`, `box 8x6`, `grid 3x2 @5m`, `wall`, `col`, `beam`, `footing`, `level`, `rebar`, `clear`) wired into `BIM Cmd:`, Dynamic Island AI Copilot, and AI Firm.
  - Connected `Time & Scheduling`, `Cost & Budgeting`, `Sustainability & Energy`, and `Report` drawing sheets directly to live active `BimProject` quantities, rebar BBS, A1-A5 CO2e footprints, and framing plan / schedule projections with zero synthetic demo fallbacks.
- **Version Bump**: Incremented crate version sequentially from `0.3.6` to `0.3.7` in `Cargo.toml`, `README.md`, `src/views/settings.rs`, and `docs/CHANGELOG.md`.

---

## [v0.3.6] - 2026-11-14 — Phase 36: Personal Vault Onboarding, Web FS/IndexedDB Bridge, Top-Toolbar Inspectors & Geometry 3D Earth Map

### Added & Changed
- **Startup Personal Vault Onboarding Pop-Up (`src/app.rs`, `src/core/vault.rs`, `src/core/web_bridge.rs`, `index.html`)**:
  - Displayed a modal prompt on launch whenever no personal Vault is connected (`!vault_config.has_connected_vault() && !vault_config.guest_explore_mode`) with three clear choices: **Connect Existing Vault**, **Create New Vault**, and **Explore Workspace First (Guest Mode)**.
  - Added browser persistence in `index.html` (`window.EigenGraphVaultBridge`) and `src/core/web_bridge.rs` (`wasm_bindgen`) combining the HTML5 **File System Access API** (`showDirectoryPicker`, `readwrite` handles persisted in IndexedDB) with an automatic **IndexedDB** virtual Vault fallback for Safari/Firefox/restricted contexts, plus live project index scanning (`Projects/`, `.eigen`, `.json`) and synchronous `localStorage` snapshot caching.
  - Added native desktop folder-picker support (`pick_folder_native` via macOS/Linux/Windows OS dialogs) and clean Vault lifecycle methods (`connect_existing_vault`, `create_new_vault`, `disconnect_vault`, `enter_guest_explore_mode`, `list_personal_vault_projects`, `load_personal_vault_project_model`).
- **Settings-Consolidated Vault Management & Focused `My Library` (`src/views/settings.rs`, `src/views/my_project.rs`)**:
  - Consolidated all Vault path configuration, Browse Folder, Connect Existing, Create New Vault, Browser Local Folder / IndexedDB connection controls, and Disconnect actions inside **Settings → Workspace & Units** (`render_vault_settings_card`), widening the workspace canvas height slider to `0.45..=1.15` (`0.82` default).
  - Refocused `My Library` to display Vault connection status (with one-click link to Settings) and index/open/save `.eigen` / `.eigen.json` projects stored inside the connected personal Vault.
- **Sidebar Cleanup (`src/app.rs`)**:
  - Removed numeric feature count badges (`{active}/{total}`) from sidebar navigation module rows for a cleaner engineering workspace.
- **Parametric & Geometry Top-Toolbar Dockable Inspectors & Taller Canvas (`src/views/parametric/`, `src/views/build/`, `src/views/universal_canvas.rs`)**:
  - Removed floating HUD overlays from inside the Parametric 2D and Geometry 3D canvases and organized **Levels** (`+ Lvl`), **Properties**, **Project Browser**, **CAD / BIM Command Line**, **Scale**, **Detail (LOD)**, visual toggles, and **Data Deck** neatly inside multi-row collapsible Top Toolbars inside each canvas container.
  - Raised `UniversalCanvasState` default `canvas_height_ratio` to `0.82` (`380px/540px .. 960px` clamp) for generous drawing space under richer top toolbars.
- **Geometry 3D Earth World Map Fix (`src/views/build/viewport_3d.rs`, `src/core/gis/map_widget.rs`)**:
  - Fixed 3D camera depth sign (`depth = -(dy * sin_p + z1 * cos_p)`) and added near-plane frustum clipping (`near_clip_cam_poly`, `near_clip_segment`, `near_clip_uv_poly`) so horizon sky, ocean fills, ear-clipped vector `WORLD_LANDMASSES` continents & coastlines across `[-360°, 0°, +360°]`, 3D lat/lon graticules, and `4x4`-subdivided live Web Mercator raster tiles (`TileGeoQuad`) render directly on the `y = 0.0` ground plane in both `Top` plan and 3D oblique/perspective views with coupled globe pan and scroll-wheel zoom.
- **Version Bump**: Incremented crate version sequentially from `0.3.5` to `0.3.6` in `Cargo.toml`, `README.md`, `src/views/settings.rs`, and `docs/CHANGELOG.md`.

---

## [v0.3.5] - 2026-11-14 — Phase 35: Default-Frozen World Map Ground Canvas & Zero-Demo Blank Workspace

### Added & Changed
- **Default-Frozen Canvas Across All Viewports (`src/ui_kit.rs`, `src/views/settings.rs`, `src/views/universal_canvas.rs`, `src/views/parametric/`, `src/views/build/`)**:
  - Enabled canvas freeze by default (`is_frozen: true`, `freeze_by_default: true`, `is_canvas_frozen` defaulting to `true`) across Universal Canvas, Parametric 2D, Geometry 3D, and Research GIS viewports so vertical page scrolling glides smoothly until the user explicitly clicks **Unfreeze**.
  - Enforced freeze guards on zoom, pan, 3D orbit, node dragging, sketch drawing, and BIM element placement (`render_with_freeze`, `canvas_2d.rs`, `viewport_3d.rs`, `universal_canvas.rs`).
- **World Map Ground Canvas (`src/core/gis/map_widget.rs`, `src/views/universal_canvas.rs`, `src/views/parametric/`, `src/views/build/`)**:
  - Upgraded `SlippyMapState` to launch at a global World Map overview (`center: (15.0, 15.0)`, `zoom: 1.85`) featuring an ear-clipped offline vector world basemap (`triangulate_polygon_2d` with `-360°/0°/+360°` wrapping), live async OSM / Esri Satellite / OpenTopoMap / CartoDB tile streaming (`WorldTileCache` on both native and `wasm32`), metric ground surveying grids (`zoom >= 11.5`), and bidirectional `world_mm <-> lat_lon <-> screen` geodetic transforms.
  - Integrated shared Earth basemap switching (`OSM`, `Satellite`, `Topo`, `Dark`, `Offline Vector`), `World`, `Ground 1:200`, and `Set Origin` toolbar controls (`render_world_map_toolbar_controls`) across Universal Canvas, Parametric 2D floor plan/drafting, and Geometry 3D viewport (with a 3D Earth Ground Terrain Plane at `y = 0.0` and cardinal North/East markers).
- **Zero-Demo Blank Workspace (`src/core/vault.rs`, `src/core/bim.rs`, `src/models.rs`, `src/views/`)**:
  - Cleared all starter demo structures, hardcoded site pins/presets, sample IFC buttons, demo node graphs, and seeded Vault datasets (`BimProject::empty()`, `VisualNodeGraph::empty()`, `InternalVault` blank defaults) so every module opens on a clean empty state ready for user authoring, with isolated temporary directories for Vault unit tests.
  - Added defensive empty-state UI panels and bounds checks in `my_project.rs`, `ai_assistant.rs`, `research.rs`, `planning.rs`, `lean.rs`, `industrialized/`, and `results.rs`.
- **Version Bump**: Incremented crate version sequentially from `0.3.4` to `0.3.5` in `Cargo.toml`, `README.md`, `src/views/settings.rs`, and `docs/CHANGELOG.md`.

---

## [v0.3.4] - 2026-11-14 — Phase 34: Universal Canvas Parity & Floating In-Canvas HUD Panels for Parametric and Geometry

### Added & Changed
- **Shared Universal Canvas UI & Ink Sketch Primitives (`src/views/universal_canvas.rs`)**:
  - Extracted and exposed shared viewport helpers on `UniversalCanvasState`: `canvas_bg`, `preferred_canvas_height`, `floating_panel_frame`, `render_toast_bar`, `render_freeze_button`, `paint_frozen_badge`, `render_sketch_tools_bar`, `render_color_swatches`, `handle_world_sketch_interaction`, `paint_live_preview_world`, `paint_entities_world`, and `render_canvas_sync_controls`.
  - Refactored Universal Canvas toolbar, viewport interactions, and floating inspector to call these shared helpers with zero duplicated UI/drawing logic.
- **Parametric 2D Studio Aligned to Universal Canvas (`src/views/parametric/`)**:
  - Replaced external docked sidebars and fixed bottom CAD Command Line box with full-width Universal Canvas chrome, shared toast notifications, unified canvas height calculation, edge-to-edge drawing viewport, and a Universal-style bottom telemetry status strip.
  - Added translucent floating HUD panels inside the drawing canvas for **Project & Storey Levels**, **Properties / Driving Parameters / Canvas Sync**, and **CAD Command Line (`CLI`)**, with `last_floating_rects` pointer/zoom/pan exclusion and responsive stacking below the footer on viewports `< 720px`.
  - Retained all Parametric domain capabilities (BIMCraft wall/beam/column/door/window/rebar tools, Object Snaps `SE..SG`, ORTHO, Live Section Cut, Sheet A101 composer, Dynamo Visual Node Graph, Rebar BBS, Elevations/MEP, Component Database, and `Save 1-File` / `Import` / `PDF/SVG/IFC4`). Added an interactive **Sketch** mode toggle inside the canvas for synced world-coordinate vector annotations.
- **Geometry 3D BIM Aligned to Universal Canvas (`src/views/build/`)**:
  - Replaced the fixed external sidebar split and always-open stacked toolbars with Universal-Canvas-style HUD chrome (Row 1 quick controls + collapsible 12-discipline `BIM Tools` ribbon + optional ground-plane `Sketch` strip), full-bleed 3D canvas (`preferred_canvas_height`), and a compact bottom viewport control/telemetry footer.
  - Floated **Project & Model Tree**, **Properties / Clash Matrix / Model Audit / BCF Issues / Geometry Wizards / Modify / Layers / Canvas Sync**, and the **Data Deck** (`QTO`, `4D`, `Clashes`, `BIM Console`, `LiDAR Point Cloud`) inside the 3D viewport (`last_floating_rects` orbit/pan/zoom/click guard, XYZ triad & HUD label clearance, and clean mobile stacking `< 720px`).
  - Added 3D ground-plane inverse projection (`screen_to_ground_mm`) and `y = 0.02m` synced vector ink overlay across all 17 structural archetypes, `CSG B-Rep`/`Solid`/`Wireframe`/`NFD`/`SFD`/`BMD`, ViewCube presets, Freeze Canvas guard, and `Save 1-File` / `Import`.
- **Version Bump**: Incremented crate version sequentially from `0.3.3` to `0.3.4` in `Cargo.toml`, `README.md`, and `docs/CHANGELOG.md`.

---

## [v0.3.3] - 2026-11-14 — Phase 33: Single-Canvas Parametric/Geometry Views & Vector Icon Overhaul

### Added & Changed
- **Single Unified Canvas in Parametric & Geometry (`src/views/parametric/`, `src/views/build/`)**:
  - Consolidated `ParametricDED` and `GeometryAssembly` to render a single synchronized workspace canvas each, eliminating duplicate canvas panes while retaining `Freeze Canvas` / `Frozen (Scroll Page)`, `Save 1-File` / `Import` (`.eigen.json`), and shared BIM synchronization.
  - Respects `sidebar_toggle_inset_px` clearance so floating sidebar controls never collide with module action ribbons.
- **Painted Vector Icon Overhaul (`src/icons.rs`, `src/ui_kit.rs`, `src/app.rs`, `src/views/`)**:
  - Replaced raw Unicode emojis, colored circles, pictographs, and bitmap glyph workarounds across sidebar items, subtab bars, tool ribbons, cards, badges, inspectors, and canvas overlays with geometric `egui::Painter` icons (`crate::icons::{self, AppIcon}`).
  - Expanded `prefix_map` and `strip_pictographs` in `src/icons.rs` to map all MEP/infrastructure/schedule/cost/sustainability ribbon tools cleanly while preserving engineering and mathematical typography (`Δ`, `σ`, `ε`, `μ`, `λ`, `θ`, `φ`, `·`, `±`, `°`, `≥`, `≤`, subscripts/superscripts).
- **Version Bump**: Incremented crate version from `0.3.2` to `0.3.3` in `Cargo.toml` and `README.md`.

---

## [v0.3.2] - 2026-11-14 — Phase 32: Canvas-First Unified Workspace & Settings Consolidation

### Added & Changed
- **Top Toolbar Removed (`src/app.rs`)**:
  - Removed the `CentralPanel` top header banner, active-module title bar, status/roadmap badges, and `WORKFLOW:` shortcut ribbon so every module opens directly onto the workspace.
  - Added a floating top-left `☰` button when the sidebar is collapsed or closed so users can reopen the navigation drawer effortlessly.
- **Consolidated Workspace Settings & Future Roadmap (`src/views/settings.rs`, `src/theme.rs`)**:
  - Merged Workspace Settings and Future Roadmap (`SettingsSubTab::WorkspaceSettings`, `SettingsSubTab::FutureRoadmap`, `SettingsSubTab::About`) inside the single `⚙️ Settings` dialog (`Cmd+,` or sidebar footer), keeping existing Phase 32 locked feature slots frozen until UI stabilization is complete.
  - Added `ThemeMode::ProDark` / `ThemeMode::EngineeringLight` theme switching, Floating AI Copilot visibility toggle (`src/views/ai_assistant.rs`), unified canvas defaults (height ratio, Freeze by Default, Grid/Snap/Spacing, BIM underlay, Domain telemetry HUD, Floating Inspector, and Sync All 15 Modules), alongside Engineering Units, Design Standards, Solver Safeguards, and single-file sync actions.
- **One Synchronized Canvas-First Drawing Workspace Across All 15 Modules (`src/views/universal_canvas.rs`, `src/ui_kit.rs`, `src/app.rs`)**:
  - Implemented `UniversalCanvasState` rendered at the top of every `NavigationPillar` module inside a single vertical `ScrollArea`, with an interactive 8-tool CAD/sketch toolbar, synchronized multi-layer BIM underlay, domain telemetry overlays, floating collapsible inspector, and `"⬇ <Module> Workbench"` jump button.
  - Added the **`❄️ Frozen (Scroll Page)` / `🔓 Freeze Canvas`** guard (`ui_kit::set_canvas_frozen` / `is_canvas_frozen`) shared across the unified canvas and existing domain viewports (`viewport_3d.rs`, `canvas_2d.rs`, `dynamo_studio.rs`, `map_widget.rs`) so vertical scrolling glides cleanly past the canvas without unintended zoom or pan.
  - Removed conflicting outer full-page vertical scroll containers across module workbenches (`my_project.rs`, `ai_firm.rs`, `research.rs`, `analyze/mod.rs`, `planning.rs`, `sustainability.rs`, `facility.rs`, `safety.rs`, `lean.rs`, `industrialized/mod.rs`, `prediction.rs`, `results.rs`, `build/mod.rs`, `parametric/mod.rs`).
- **Unified Single-File `.eigen.json` Sync (`UnifiedWorkspaceArchive`)**:
  - Sealed BIM entities, `StructuralMember`s, `ParametricVariable`s, all 15-module canvas drawings, and `WorkspaceCanvasSettings` into a single SHA-256 verified `.eigen.json` archive (with optional SVG export and explicit per-module split export when needed).

---

## [v0.3.1] - 2026-11-13 — Phase 31: Full-Suite 15-Pillar Workspace Layout & 74-Capability Unlock Architecture

### Added
- **Expanded Centralized Feature Registry (`src/features.rs`)**:
  - Expanded `FeatureId` and `FeatureRegistry` from 43 to **74 tracked engineering capabilities** (**28 Active + 46 Locked preview slots**) distributed across all **15 Navigation Pillars** and organized cleanly into target unlock milestones (**Phase 32**: 6 slots, **Phase 33**: 10 slots, **Phase 34**: 18 slots, **Phase 35**: 12 slots).
  - Added `FeatureRegistry::locked_by_phase_count()`, `distinct_target_phases()`, `export_unlock_roadmap_json()`, and updated unit tests verifying full 15-pillar coverage and JSON/Markdown roadmap serialization.
- **Frozen Full-Suite Locked Feature Preview Slots Across All 15 Domains (`src/views/`)**:
  - **1. My Library (`my_project.rs`)**: Added `CdeIso19650DocumentHub` (`LIB-04`) WIP/Shared/Published container studio & `MultiTypologyProjectTemplates` (`LIB-05`) quick project seed launcher.
  - **2. AI Agent (`ai_firm.rs`)**: Added `AutomatedCodeComplianceAuditor` (`AI-04`) clause-by-clause sweep panel & `ParetoMultiObjectiveOptimizer` (`AI-05`) 5D Capex vs. Carbon vs. ARI scatter envelope.
  - **3. Research & 1D GIS (`research.rs`)**: Added `GeotechBoreholeStratigraphy` (`1D-05`) SPT N60 / Vs30 boring log viewer & `HydrologyIdfFloodplain` (`1D-06`) 100-year IDF runoff & FFE datum dashboard.
  - **4. Parametric & 2D Studio (`parametric/detailing_2d.rs`)**: Added `AutomatedRcRebarBbsDrafter` (`2D-05`) ACI 318 seismic confinement elevation & shape-code BBS preview, plus `BuildingElevationsMepRiser` (`2D-06`).
  - **5. Geometry & 3D BIM (`build/sidebar.rs`, `build/bottom_deck.rs`)**: Added `FederatedLayersAxisSlicer` (`3D-05`) discipline layer & X/Y/Z section box controls and `PointCloudAsBuiltDeviation` (`3D-06`) E57/LAS deviation inspector.
  - **6. Analysis & Multi-Hazard Simulation (`analyze/extended_mechanics.rs`)**: Added `BridgeMovingLoadInfluence` (`SIM-06`), `RetainingWallSlopeStability` (`SIM-07`), and `FloorVibrationSteelFatigue` (`SIM-08`).
  - **7 & 8. Time 4D & Cost 5D (`planning.rs`, `app.rs`)**: Separated 4D Scheduling tabs (4 tabs) from 5D Budgeting tabs (6 tabs) with one-click cross-pillar switching, and added `MonteCarloScheduleRiskPert` (`4D-03`), `ResourceLevelingCrewHistogram` (`4D-04`), `EarnedValueEvmProgressClaim` (`5D-03`), and `ValueEngineeringTenderMatrix` (`5D-04`).
  - **9. Sustainability 6D (`sustainability.rs`)**: Added `SolarOttvThermalDaylight` (`6D-03`) N/E/S/W facade compass + SNI 6389 OTTV grid and `CircularMaterialPassportDfd` (`6D-04`).
  - **10. Facility Management 7D (`facility.rs`)**: Added `MepIsolationSpaceUtilization` (`7D-03`) riser isolation matrix and `DroneDefectCrackMapper` (`7D-04`) UAV crack overlay & ACI 224R ledger.
  - **11. Safety & Health 8D (`safety.rs`)**: Added `ScaffoldingFormworkReshoring` (`8D-03`) ACI 347 shoring load transfer and `PermitToWorkEvacuationDrill` (`8D-04`).
  - **12. Lean Construction 9D (`lean.rs`)**: Added `TaktTimeConcretePourDispatch` (`9D-03`) and `RfiSubmittalSlaTracker` (`9D-04`).
  - **13. Industrialized & 10D OpenBIM (`industrialized/automation_slots.rs`)**: Added `PrecastTrailerJointTolerance` (`10D-04`) and `Concrete3DPrintingGcodeToolpath` (`10D-05`) 3DCP contour preview.
  - **14. Prediction & AI Aging (`prediction.rs`)**: Added `ChlorideCorrosionDurability` (`PRED-03`) Fick's 2nd Law diffusion plotter and `PostEarthquakeAtc20Tagging` (`PRED-04`) wing placard triage matrix.
  - **15. Report & Publishing (`results.rs`)**: Added `RegulatoryCalcBookPbgComposer` (`PUB-04`) SIMBG/PBG/IBC cover & PE stamp preview and `AsBuiltHandoverDossierPackager` (`PUB-05`) SHA-256 package verifier.
- **Roadmap Dashboard & Navigation Telemetry (`src/views/settings.rs`, `src/app.rs`)**:
  - Added per-pillar capability counters (`Active/Total`) in the sidebar navigation list, active+locked counts in the workspace header pill, Phase 32–35 filter chips in Settings -> Feature Registry, Phase 32/33 batch queue triggers, and structured `EigenGraph_Unlock_Roadmap.json` export.

---

## [v0.3.0] - 2026-11-13 — Phase 30: Frozen Responsive 15-Pillar Master Workspace Architecture & Centralized Feature Unlock Registry

### Added
- **Centralized Feature Gate & Unlock Roadmap Registry (`src/features.rs`, `src/lib.rs`, `src/app.rs`, `src/views/settings.rs`)**:
  - Added `FeatureId` (44 capability identifiers), `FeatureStatus` (`Active` / `Locked { target_phase }`), and `FeatureRegistry` covering every active capability and upcoming Phase 31–33 slot across all 15 engineering navigation pillars.
  - Upgraded **Settings Modal (`src/views/settings.rs`)** into a 3-tab hub featuring a live **Feature Registry & Unlock Roadmap** dashboard (`All`, `Locked Only`, `Prioritized`, `Active Only`, pillar readiness cards with one-click domain jump, batch Phase 31 prioritization, per-feature priority queueing, and `EigenGraph_Unlock_Backlog.md` export).
  - Added a clickable header pill (`🔓 Active/Total • 📋 Roadmap`) in `src/app.rs` opening the Feature Registry directly from any workspace screen.
- **Shared Responsive UI Primitive Kit (`src/ui_kit.rs`)**:
  - Replaced rigid multi-column layouts across all 15 views with shared viewport-aware primitives (`responsive_grid_n`, `responsive_two_columns`, `responsive_two_panes`, `split_workspace`, `split_workspace_panes`, `subtab_bar`, `stat_card`, `feature_status_badge`, `locked_feature_slot`, `feature_slot_card`).
- **Inline Locked Feature Slots Across All 15 Engineering Domains**:
  - Wired standardized locked preview slots (`BrowserIndexedDbPersistence`, `LiveGeminiToolOrchestrator`, `LiveRasterMapTileFetcher`, `ParametricBidirectionalSync`, `LiveBvhGeometricClashSolver`, `LiveBimToFemAutoSync`, `BimDriven4DCriticalPathSync`, `LiveBimQtoBoqSync`, `BimMaterialLcaAutoSync`, `LiveIotTelemetryIngestion`, `SitePerimeter3DCraneCollision`, `LiveRebarBbsCuttingSync`, `WasmLocalIfcFileDropPicker`, `LiveShmAnomalyForecasting`, and `AutoProjectedBimDedSheets`) inline within their destination view layouts so future logic unlocks require zero layout restructuring.
- **Modular View Architecture Cleanup (`src/views/`)**:
  - Split oversized monolithic views into clean submodule directories: `src/views/analyze/` (`mod.rs`, `header.rs`, `core_fea.rs`, `seismic_dynamic.rs`, `member_design.rs`, `extreme_hazards.rs`), `src/views/build/` (`mod.rs`, `ribbon.rs`, `sidebar.rs`, `clash_bcf.rs`, `bottom_deck.rs`, `viewport_3d.rs`), `src/views/parametric/` (`mod.rs`, `ribbon.rs`, `inspector.rs`, `canvas_2d.rs`, `dynamo_studio.rs`), and `src/views/industrialized/` (`mod.rs`, `openbim_studio.rs`, `ifc_inspector.rs`, `dfma_robotics.rs`, `lifecycle_10d.rs`).

### Fixed
- **Viewport-Safe Grids, Scrollable Matrix Tables & Clamped Canvas Inputs Across All Views**:
  - Eliminated remaining `ui.columns(...)` calls across views in favor of `ui_kit` adaptive cards and panes, wrapped wide member/QTO/DED matrices in horizontal `ScrollArea` containers, prevented title-block/schematic overlaps on narrow viewports in `src/views/results.rs`, removed dead helper helpers, and clamped unrestricted `TextEdit` / `ComboBox` widths across mobile, tablet, and desktop breakpoints.

---

## [v0.2.9] - 2026-11-12 — Phase 29: Stateful 3D Clash & BCF 2.1 Coordination Hub, Autonomous AI Copilot Studio, Parametric Algorithmic Dynamo Preview & Lean/DfMA Supply Chain Trackers

### Added
- **Multi-Disciplinary Clash Detection & BCF 2.1 Coordination Hub (`src/views/build.rs`, `src/models.rs`)**:
  - Added stateful clash models (`ClashDetectionItem`, `ClashSeverity`, `ClashStatus`) and BCF 2.1 issue records (`BcfTopicRecord`, `BcfComment`, `BcfPriority`, `BcfTopicStatus`) with deterministic tolerance filtering (`is_triggered_at_tolerance`) and volumetric clash estimation (`clash_volume_m3`).
  - Upgraded the left Clash Manager tab, Bottom Deck Clash Manager, and 3D Viewport with tolerance-driven clash solvers, discipline/severity filters, clickable status workflows (`Active -> In Review -> Resolved`), `👁️ Focus 3D` / `💬 Log BCF` actions, camera-viewpoint-capturing BCF issue creation, comment threads, viewpoint restoration (`📷 Restore Viewpoint`), and `BCF 2.1 XML` export via `DiskExporter`.
  - Projected 3D wireframe clash bounding boxes, center halo markers, and selected highlight rings directly into the 3D orbit canvas with a viewport `⚔️ Clashes` overlay toggle.
- **Autonomous AI Civil Engineering Copilot Studio (`src/views/ai_firm.rs`, `src/models.rs`)**:
  - Added `AiFirmSubTab::CopilotStudio` with interactive engineering parameter sliders, deterministic structural calculation solvers, full variable substitution breakdowns, and one-click `.md` Calculation Memo downloads:
    - **ACI 318-19 Chapter 18 / Eurocode 2 Shear Wall Designer** (`solve_aci318_shear_wall`): Aspect coefficient $\alpha_c$, web shear ratio $\rho_t$, $\phi V_n$, bar curtain selection, and special boundary element compressive stress check ($0.2 f'_c$).
    - **ASCE 7-22 Chapter 27 MWFRS Wind Load Solver** (`solve_asce7_wind`): Velocity pressure coefficient $K_z$, velocity pressure $q_z$, windward/leeward net facade design pressure $p_{net}$, and curtain-wall DCR check.
    - **ISO 14040 / EN 15978 A1–A3 Embodied Carbon Minimizer** (`solve_iso14040_carbon`): SCM slag/fly-ash concrete blend, EAF recycled steel fraction, and mass-timber hybrid deck optimization.
  - Added `"📢 Push Summary to Firm Board"` integration and responsive wrapping/stacking across all `AI Firm` sub-tabs.
- **Parametric Dynamo / Grasshopper Algorithmic Studio (`src/views/parametric.rs`, `src/models.rs`)**:
  - Added `ParametricGeneratorPreset` (`DiagridTower`, `GeodesicDome`, `VoronoiCanopy`) paired with 4 live geometric parameter sliders and dynamic `VisualNodeGraph::from_preset(...)` rebuilding.
  - Added an orbitable **Real-Time 3D Wireframe Mesh Preview** alongside the interactive draggable Dynamo node canvas, plus a real `"⚡ Bake to BIM Scene"` executor (`BimCommand::AddColumn` & `AddBeamRun` via `BimCommandBus`) with full Undo support and live ARI evaluation.
- **Lean Last Planner System (LPS) & Modular Prefab Supply Chain Tracker (`src/views/lean.rs`, `src/views/industrialized.rs`, `src/models.rs`)**:
  - Upgraded **Lean Construction (`lean.rs`)**: Interactive 6-week Make-Ready lookahead & Weekly Work Plan board (`LpsLookaheadTask`), constraint clearing, live Percent Plan Complete (PPC %) header indicator, painted Weekly PPC Trend chart vs. 80% reliability benchmark, dynamic Variance Root-Cause Pareto (`compute_variance_pareto`), responsive card grids, and Indonesian Rupiah (`Rp`) currency formatting fix.
  - Upgraded **10D Modular DfMA (`industrialized.rs`)**: 6-stage prefabricated supply chain pipeline (`PrefabSupplyStage`, `PrefabModuleUnit`), simulated QR/RFID Scanner & Gate Check-In Console, crane hoist Safe Working Load (`SWL %`) utilization gauges, stage advancement workflows, responsive mobile/tablet card & pane stacking, and browser-safe `wasm32` guards on local IFC file reads.
- **Automated Unit Verification (`src/models.rs`)**:
  - Added `mod phase29_tests` covering clash tolerance filtering & BCF 2.1 XML export, deterministic AI Copilot calculators (shear wall, wind, carbon), live PPC & variance Pareto aggregation, and parametric preset / crane SWL bounds.

### Fixed
- **Planning 5D WBS Sub-Tab Navigation Regression (`src/app.rs`)**: Preserved `PlanningSubTab::WbsHierarchy` alongside `QuantityTakeoff5D` and `EarnedValueEvm` when activating `NavigationPillar::CostBudgeting`.
- **Responsive Wrapping Across Engineering Workbenches (`src/views/build.rs`, `src/views/ai_firm.rs`, `src/views/lean.rs`, `src/views/industrialized.rs`, `src/views/parametric.rs`)**: Replaced fixed horizontal toolbars and unstacked split containers with `horizontal_wrapped` controls and viewport-width-aware vertical stacking below tablet breakpoints.

---

## [v0.2.8] - 2026-11-12 — Phase 28: 4D-7D BIM Superstation (4D Synchro Simulation, 5D Dynamic WBS & Multi-Currency Explorer, 6D ISO 14040 Whole-Life Carbon Assessment & 7D COBie 3.0 Digital Twin Telemetry)

### Added
- **4D Synchro Construction Simulation & Staged Erection Studio (`src/views/planning.rs`, `PlanningSubTab::Schedule4d`, `ConstructionSimulationPhase`)**:
  - Interactive 4D timeline simulation player with Play/Pause toggles, speed multipliers (1x, 2x, 5x), Day scrubber slider (Day 1 to Day 360), and calendar date conversion.
  - Active construction phase indicator linking WBS codes, planned vs. actual progress %, daily manpower demands, active cranes/equipment, and cumulative concrete pour volume ($m^3$).
  - 2D Staged Building Erection Elevation Canvas:
    - Dynamic physical simulation of a 15-storey structure: deep bored piling, raft slab and basement walls, columns and primary girders, composite floor slabs, and unitized curtain wall glass panels reacting to current timeline day.
    - Animated tower crane at the current roof elevation with realistic mast, jib, trolley, and hook geometry.
  - Critical Path Method (CPM) zero-float activity tracking and S-Curve cashflow progress projection.
- **5D Automated Quantity Takeoff (QTO) & Dynamic WBS Explorer (`src/views/planning.rs`, `PlanningSubTab::WbsHierarchy`, `WbsCostItem`, `CurrencyUnit`)**:
  - Multi-Currency Dynamic Switcher with live conversion across Indonesian Rupiah (`IDR / Rp`), US Dollars (`USD / $`), Euros (`EUR / €`), and Singapore Dollars (`SGD / S$`).
  - Hierarchical MasterFormat & UniFormat WBS Explorer with division filtering pills (`02 Substructure`, `03 Concrete`, `05 Structural Metals`, `08 Openings & Glazing`, `23 Mechanical`).
  - Interactive itemized cost breakdown with unit rates, total prices, contingency reserves, statutory PBG tax estimation, and physical completion progress bars.
- **6D Whole Building Life Cycle Carbon Assessment & Green Building Scorecard (`src/views/sustainability.rs`, `LcaStageEmission`, `GreenBuildingCredit`)**:
  - Full EN 15978 / ISO 14040 cradle-to-grave WBLCA breakdown across stages:
    - $A1–A3$ Product Manufacturing (Raw materials & steel smelting)
    - $A4–A5$ Transport & Construction Process
    - $B1–B7$ 50-Year Operational Energy & Building Use
    - $C1–C4$ Deconstruction & End-of-Life Waste Processing
    - Module $D$ Beyond Lifecycle Circular Reuse & Scrap Recycling Offset
  - Benchmark tracking against RIBA 2030 Climate Challenge and carbon intensity ($kg\text{ CO}_2e / m^2$).
  - Multi-Standard Green Building Certification Engine with real-time rating status for **GREENSHIP (GBCI Platinum: 86 Pts)**, **LEED v4.1 (BD+C Platinum: 84 Pts)**, and **BREEAM International (Outstanding: 88.4%)**.
- **7D Digital Twin Asset Registry & COBie 3.0 Inspector (`src/views/facility.rs`, `ShmSensorTelemetry`, `CobieAssetRecord`)**:
  - Live IoT Structural Health Monitoring (SHM) telemetry with multi-sensor sparkline history graphs:
    - Fiber-Optic Strain Gauges ($\mu\varepsilon$) with elastic limit guards.
    - Triaxial Accelerometers ($Hz$) for natural modal frequency resonance tracking.
    - Laser displacement crack meters ($mm$) and electrolytic tiltmeters ($^\circ$).
  - Full buildingSMART COBie 3.0 / 2.4 Spreadsheet Data Inspector with subtabs (`Facility`, `Floor`, `Space`, `Component`, `Type`, `Document`), search filter, condition indices, and one-click COBie workbook export.
  - Computerized Maintenance Management (CMMS) dispatch panel with automated work order generation and inspector routing.

---

## [v0.2.7] - 2026-11-12 — Phase 27: All-Inclusive World-Class Master BIM Superstation (2D Drawing Sheets & DED Engine, Visual Dynamo/Grasshopper Node Graph Studio, 10D BIM Lifecycle Master Hub & Global Family Catalog)

### Added
- **Interactive 2D Drawing Sheets & DED Production Engine (`src/views/results.rs`, `ResultsSubTab::DrawingSheets`, `DrawingSheetItem`)**:
  - Full-featured ISO architectural & structural sheet layout workstation:
    - Standard paper sizes: ISO A0 (1189×841 mm), ISO A1 (841×594 mm), ISO A2 (594×420 mm), ISO A3 (420×297 mm), and ANSI Arch D (36"×24").
    - Professional engineering title block: Project metadata ("Menara Sudirman Structural Complex"), Client attribution, Sheet titles, Sheet numbers (`S-101`, `S-201`, `A-101`, `M-301`), Drawing scale (`1:100`), Revision markers (`REV-0`, `REV-B`), North Arrow compass gizmo, and graphic scale bar.
    - Verified Professional Engineer (PE) Digital Seal stamp with circular golden certification boundary (`PE #49120-ST`).
    - Multi-viewport layout engine:
      - Viewport 1 (Main Framing Plan): Structural grids A–D and 1–4, grid callout bubbles, column placements with crosshatching, primary double-line girders (`W14x90 [G1]`), and automatic dimension chains with tick marks.
      - Viewport 2 (Schedule Table): Real-time column & footing schedules with marks (`C1`, `C2`, `F1`), dimensions, rebar arrangements, and concrete strengths.
      - Viewport 3 (Connection Detail): 1:10 scale moment connection callout with column/beam profiles, flange continuity plates, and 4-bolt shear tabs.
    - One-click production exports: Vector PDF (ISO A1, 300 DPI), AutoCAD 2024 DXF with layered geometry (`S-GRID`, `S-COLS`, `S-BEAMS`, `S-ANNO-DIMS`), and SVG vector graphics.
- **Visual Dynamo / Grasshopper Computational Node Graph Studio (`src/views/parametric.rs`, `DraftingViewMode::VisualNodeGraphDynamo`, `VisualNodeGraph`)**:
  - Visual node-based computational programming canvas built alongside the 2D CAD studio:
    - Infinite dark slate canvas with interactive pan, smooth zoom scaling, and subtle coordinate dot lattice.
    - Cubic Bézier wire connectors linking output sockets to input sockets with smooth gradients and green pulsing data-flow packet indicators.
    - Four core computational node categories (`VisualNodeCategory`):
      - **Input Nodes**: Parameter sliders (Span length, bay width, storey height, loads).
      - **Math & Logic Nodes**: Procedural formula evaluators, trigonometry functions, ratio calculators ($H = L / 8$).
      - **3D Solid Generators**: Parametric space truss generators, box meshes, beam grids, curtain wall mullion arrays.
      - **BIM Scene Bakers**: Direct scene synchronization baking procedural computational elements into the live BIM model.
    - Real-time "Run Graph" execution button with instant feedback toast logging element generation count and compute latency.
    - Procedural node insertion tools and pre-configured algorithmic templates.
- **10D BIM Multi-Discipline Lifecycle Master Hub (`src/views/industrialized.rs`, `IndustrializedSubTab::Lifecycle10DHub`)**:
  - Unified Integrated Project Delivery (IPD) command dashboard encompassing all 10 engineering lifecycle dimensions:
    - **1D Scope**: Project Charter, specifications, and baseline tracking.
    - **2D Working Drawings**: DED drawing sheets, schedules, and code annotations.
    - **3D Spatial**: Coordinated CSG B-Rep solid geometry and parametric elements.
    - **4D Time & Scheduling**: Synchro-style 180-day construction timeline ($SPI = 1.04$).
    - **5D Cost & Earned Value (EVM)**: Live Earned Value Management metrics calculating Planned Value ($PV$), Earned Value ($EV$), Actual Cost ($AC$), Cost Variance ($CV = +\$295,000$), Schedule Variance ($SV = +\$290,000$), $CPI = 1.061$, and $SPI = 1.059$.
    - **6D Sustainability & LCA**: Cradle-to-gate embodied carbon tracking ($-28.4\%\text{ CO}_2\text{e}$) and LEED Platinum 84-point scorecard.
    - **7D Digital Twin & IoT Telemetry**: Real-time sensor stream readouts for strain gauges ($\mu\epsilon$), accelerometers ($f_1 = 1.42\text{ Hz}$), tiltmeters, and thermocouples.
    - **8D Site Safety & Logistics**: Dynamic tower crane hook radius (48m / 55m jib envelope), Safe Working Load (SWL 70%), and perimeter slab-edge fall arrest net detection.
    - **9D Lean Construction**: Last Planner System (LPS) Kanban board with live status pills (Done, In Progress, Committed, Backlog) tracking Percent Plan Complete ($PPC = 88\%$).
    - **10D Industrialized DfMA**: Off-site prefabrication modularity metrics ($82.5\%$ factory pre-assembly) and highway transport module clearance validation.
- **Global BIM Component Family Catalog & Parameter Inspector (`src/views/my_project.rs`, `BimFamilyItem`, `LibraryViewMode`)**:
  - Top library mode switcher between "Project Portfolios & Vault" and "Global BIM Families & Parameter Inspector (Revit RFA / IFC Style)".
  - Searchable multi-discipline catalog of parametric component families categorized by MasterFormat 2024 and UniFormat II.
  - Multi-discipline families pre-loaded: AISC W14x90 Steel Girder, Cast-in-Place RC 600x600 Column, Unitized Curtain Wall Facade, VAV Rooftop AHU Package, PCI Type IV 30m Prestressed I-Girder, Precast Stormwater Box Culvert.
  - Deep parameter inspector panel exposing Type Parameters, Instance Parameters, and buildingSMART IFC Property Sets (`Pset_BeamCommon`, `Pset_StructuralAnalysis`, `Pset_EnvironmentalMetrics`).
  - One-click "Insert into Active 3D Scene", "Duplicate Type", and "Export IFC Spec" action controls.

---

## [v0.2.6] - 2026-11-12 — Phase 26: Master World-Class BIM Suite Overhaul (Architecture, Structure, MEP, Civil, 4D/5D, Solibri Audit & BCF 3.0)

### Added
- **Master Multi-Discipline BIM Ribbon Toolbar (`src/views/build.rs`, `BimRibbonTab`)**:
  - Two-tier responsive engineering ribbon with horizontal fluid scrolling for phone, tablet, and desktop viewports:
    - **🏛️ Architecture (ARCH)**: Compound layered walls (`BimWall`), parametric curtain walls, hosted single & double doors (`BimOpening`), casement & ribbon windows, architectural columns, floor slabs, straight-skeleton hip/gable roofs, circulation stairs & safety railings, room polygon boundary tags (`BimRoom`), shaft voids, structural grid axes (`BimGrid`), and level elevation datums (`BimLevel`).
    - **🏗️ Structure (STRUCT)**: Primary girders, secondary joists, 3D space trusses (MERO octet), chevron lateral braces, isolated spread footings, strip footings, raft mat foundations, bored piles & pile caps, ductile RC shear core walls, 3D parametric rebar cages with stirrup ties, post-tensioned parabolic tendons, steel base plates with anchor bolts, moment connections, and analytical boundary restraints (Fixed, Pinned, Roller).
    - **⚡ MEP Systems (MEP / Fire)**: Rectangular supply & return air ducts, ceiling diffusers, rooftop air handling units (AHU), variable air volume (VAV) boxes, domestic water piping, sanitary DWV wastewater stacks, water pressure booster pumps, electrical cable ladder trays, branch circuit conduits, main switchboards (MSB), 600x600 LED troffers, automatic pendant fire sprinklers, and wet fire standpipes.
    - **🌍 Civil & Infrastructure (CIVIL)**: 3D horizontal road alignment splines, profile cross-section corridors, digital elevation model (DEM) TIN terrain surfaces, cut & fill volumetric grading meshes, prestressed segmental concrete box girders, bridge pier caps & columns, abutments, and stormwater box culverts.
    - **🔬 FEA Analysis & Simulation (SIM)**: Dead & live gravity load applications, ASCE 7-22 wind pressure distributions, SNI 1726 seismic response spectra, ISO 834 fire curves, UFC 3-340 blast dynamics, ASCE LRFD load combinations, linear static space frame solver, second-order P-Delta solver, Lanczos modal eigenvalue solver, non-linear pushover capacity spectrum analysis, progressive collapse N-1 disruption solver, Von Mises stress contours, 3D normal force diagrams (NFD), shear force diagrams (SFD), and bending moment diagrams (BMD).
    - **⏱️ 4D Time & Scheduling (4D BIM)**: Real-time staged construction phase playback, 1-to-180 day scrubber timeline, active phase status banners, critical path method (CPM) Gantt schedule view, and task-to-spatial object automatic WBS linkers.
    - **💰 5D Cost & Budgeting (5D BIM)**: Dynamic Quantity Take-Off (QTO), unit-rate Bill of Quantities (BoQ / RAB) ledger, 3D financial cost heatmap, S-curve expenditure forecasting, and multi-currency exchange support (USD, EUR, IDR).
    - **🌱 6D Sustainability & LCA (6D BIM)**: Cradle-to-gate (A1–A3) embodied carbon calculators, operational energy use intensity (EUI), 365-day solar heliodon sun-path dome, cast shadow perimeter analysis, and LEED v4.1 credit scorecards.
    - **🏢 7D Digital Twin & FM (7D BIM)**: COBie equipment asset tags, real-time IoT accelerometer & strain gauge telemetry streaming, preventive maintenance work orders, and filter lifecycle trackers.
    - **🛡️ Coordination & Clash Detection (NAV / Solibri)**: Spatial collision interference detector, hard clash vs soft clearance identification, automated building quality audits, and BCF 3.0 issue topic generation.
    - **🌐 Collaborate & OpenBIM (IFC / Speckle)**: ISO 10303-21 STEP physical file import/export, buildingSMART certified IFC4 Reference View, ifcJSON-4 serializers, Speckle stream cloud synchronization, Wavefront OBJ export, and AutoCAD 3D DXF export.
    - **📑 Documentation & Working Drawings (DED)**: Standard A0/A1/A2/A3 drawing sheet generators, drag-and-drop viewport placements, title block metadata stamps, smart associative dimensions, and bar bending schedules (BBS).
- **Comprehensive Multi-Tab BIM Inspector & Coordination Dock (`SidebarTab`)**:
  - Added new dedicated activity bar tabs:
    - **⚔️ Spatial Clash Matrix (`SidebarTab::ClashMatrix`)**: Hard collision and clearance violation inspector with tolerance sliders, severity badges, and one-click 3D viewport isolation.
    - **🛡️ Solibri-Style Model Quality & Code Audit (`SidebarTab::ModelAudit`)**: Automated rule-based building compliance checker computing global BIM Health Score (98.4%) covering room enclosure, column continuity, egress headroom, fire rating integrity, and slenderness limits.
    - **💬 BCF 3.0 Collaboration Issues (`SidebarTab::BcfIssues`)**: Standardized issue topic cards with priority tags, assignees, due dates, status markers, and camera viewpoint restoration in 3D.
  - Enhanced **Properties Inspector (`SidebarTab::Properties`)** with deep buildingSMART IFC Common Property Sets (`Pset_BeamCommon`, `Pset_ColumnCommon`, `Pset_StructuralAnalysis`, `Pset_EnvironmentalMetrics`).
  - Enhanced **Project Browser (`SidebarTab::ProjectBrowser`)** with two-tier Views Hierarchy (Floor plans with section box height coupling, 3D perspectives, cardinal elevations, schedules, sheets) and buildingSMART Spatial Breakdown Structure (`IfcProject` ➔ `IfcSite` ➔ `IfcBuilding` ➔ `IfcBuildingStorey`) with element visibility toggles.
- **3D Interactive BIM Viewport Overlays & Navigational Gizmos**:
  - **3D Level Elevation Datums**: Horizontal datum planes with circular blue elevation heads and level tags (Level 0 GF, Level 1, Level 2, Level 3 Roof).
  - **3D Structural Grid Network**: Grid axes 1–4 and A–D rendered on the foundation plane with numbered circular bubbles.
  - **Interactive 3D Section Box**: Cyan bounding box cage with 12 wireframe clipping edges and multi-axis sliders for real-time room/storey cutaways.
  - **ViewCube & Compass Navigation Gizmo**: Corner orientation gizmo for snapping camera to Top, Front, Right, Back, Left, and Isometric views.
  - **Revit-Style Viewport Status Bar**: Scale selector (1:50, 1:100, 1:200), Level of Development selector (LOD 100 to 400), Sun/Shadow toggle, Section Box toggle, Datums toggle, and loaded elements counter.
- **Expandable Bottom BIM Data Deck (`BottomDeckTab`)**:
  - **Live Schedules & BoQ (QTO)**: Interactive spreadsheet table with search filtering, category selectors, element quantities, unit rates, costs, and CSV export.
  - **4D Construction Timeline**: Play/pause/step simulation bar with day scrubber and 5 phased construction milestone badges.
  - **Clash & Coordination Quick Deck**: Mini collision cards with one-click 3D camera isolation.
  - **BIM Automation Terminal**: Interactive command line interface with instant feedback and command macros.

---

## [v0.2.0] - 2026-11-08 — Phase 20: OpenBIM IFC Spatial Clash Detection, Model Diff, IDS Validation & Multi-Format Exporter

### Added
- **OpenBIM IFC Spatial Clash Detection Engine (`src/core/ifc.rs`)**:
  - **3D Axis-Aligned Bounding Box (AABB) Collision Solver (`intersects`, `intersection_volume`, `distance_to`, `overlap_ratio`)**:
    - High-performance spatial intersection calculations with configurable tolerance ($\Delta_{\text{tol}} \ge 0.0$ mm).
    - True 3D volumetric interference evaluation ($V_{\text{intersect}} = \Delta x \times \Delta y \times \Delta z$).
    - Overlap ratio metric relative to minimum participating element bounding volume.
    - Proximity clearance separation distance calculation across all three spatial axes.
  - **Multi-Tier Clash Classification (`IfcClashSeverity`, `IfcClashItem`, `IfcClashReport`)**:
    - `Duplicate`: Coincident identical centroids and dimensions within spatial tolerance threshold.
    - `HardClash`: Direct physical volumetric interpenetration between distinct structural components.
    - `ClearanceViolation`: Proximity clearance breach when distance between elements falls below clearance buffer.
    - Detailed reporting with element step IDs, names, classes, overlap volumes, clearance distances, and 3D clash midpoint coordinates.
- **BIM Differential & Revision Tracking Engine (`diff`, `IfcDiffKind`, `IfcElementDiff`, `IfcModelDiff`)**:
  - Bi-model comparison between baseline and revised models with element tracking via 22-char GlobalId GUID and Step ID.
  - Categorization of changes into `Added`, `Deleted`, `Modified`, and `Unchanged`.
  - Detailed delta audit tracking attribute edits (name, description, object type), storey transfers, material changes, 3D translation vectors $[dX, dY, dZ]$, and volume differences ($\Delta V$).
  - Revision cryptographic fingerprinting using FIPS 180-4 SHA-256 for model provenance.
- **buildingSMART Information Delivery Specification (IDS) Validation Engine (`validate_ids`, `IfcIdsReport`, `IfcIdsRuleCheck`)**:
  - Automated rule-based compliance evaluation against buildingSMART OpenBIM standards:
    - Element naming non-empty check.
    - Standard 22-character Base64 alphanumeric IFC GlobalId validation.
    - Material association requirement verification.
    - Spatial storey containment check.
    - Positive bounding box extents and volume validation.
  - Granular compliance scoring with pass/fail counts and itemized non-compliance details.
- **Multi-Format OpenBIM Exporters & Interoperability**:
  - **buildingSMART BCF (BIM Collaboration Format) 2.1 XML Generator (`export_bcf_xml`)**:
    - Automatically converts detected spatial clashes into standardized BCF topics with unique UUIDs.
    - Computes 3D camera viewpoints and target vectors pointing directly at collision midpoints.
    - Generates ISO compliant XML topic and viewpoint structures ready for BIM coordinators.
  - **buildingSMART ifcJSON-4 Serializer (`to_ifc_json`)**:
    - Serializes complete in-memory IFC graph into standardized JSON notation.
    - Exports schema version, project metadata, products, types, bounding extents, centroids, and properties.
  - **Live Analytical FEM Synchronization (`to_structural_model`)**:
    - Direct transformation from BIM geometrical representation into discrete finite element nodes and frame elements.
- **OpenBIM Studio Interactive Clash, Diff & IDS Workbench (`src/views/industrialized.rs`)**:
  - Added 4th inspector tab: `⚔️ Clash, Diff & IDS` with sub-navigation:
    - **⚔️ Spatial Clash Matrix**: Tolerance sliders, clearance buffer controls, execution button, severity summary pill badges, and detailed clash item tables.
    - **🔀 BIM Differential**: Model comparison summary, change counters (Added, Modified, Deleted, Unchanged), and delta inspection table with translation and volumetric differences.
    - **📋 IDS & BCF Collaboration**: Rule compliance scorecard (Pass/Total, Compliance %), detailed rule audit table, and one-click BCF XML topic export.
  - Added bottom toolbar action buttons:
    - `📋 Export ifcJSON-4 (.json)`: Instant JSON export of the current model.
    - `📬 Export BCF Topics (.xml)`: Instant BCF topic package generation from active clashes.
- **Comprehensive Unit Testing Suite**:
  - 5 rigorous new unit tests (`test_ifc_spatial_clash_detection_hard_and_clearance`, `test_ifc_model_diff_added_deleted_modified`, `test_ifc_ids_specification_validation`, `test_ifc_bcf_topic_generation`, `test_ifc_json_serializer`), expanding test suite to **107/107 tests (100% pass)**.

---

## [v0.1.9] - 2026-11-08 — Phase 19: OpenBIM IFC Geometric Topology, Swept Solid Extrusion & Element Synthesis Engine

### Added
- **OpenBIM IFC Geometric Topology, Swept Solid Extrusion & Element Synthesis Engine (`src/core/ifc.rs`)**:
  - **In-Place Swept Solid Extrusion Mutator (`modify_element_extrusion`)**:
    - Direct parameter mutation of `IfcExtrudedAreaSolid.Depth` (parameter 3) across structural framing entities (`IfcColumn`, `IfcBeam`, `IfcMember`).
    - Recalculates in-memory wireframe segments, 3D Axis-Aligned Bounding Box (AABB), and centroid coordinates while preserving local coordinate placement.
    - Synchronizes STEP parameter arrays directly for instant export without loss of topological accuracy.
  - **3D Cartesian Spatial Translation Mutator (`modify_element_placement`)**:
    - Discovers and shifts underlying `IfcCartesianPoint` coordinates associated with `IfcLocalPlacement` and `IfcAxis2Placement3D`.
    - Applies 3D displacement vectors $[dX, dY, dZ]$ across all wireframe vertices, spatial extents, and centroid positions.
  - **2D Parametric Cross-Section Resizer (`modify_element_profile`)**:
    - In-place mutation of `IfcRectangleProfileDef` dimensions (`XDim`, `YDim`) with real-time bounding box expansion/contraction.
  - **Complete Structural Element Synthesizer (`create_structural_element`, `IfcElementCreationSpec`)**:
    - Synthesizes valid buildingSMART IFC4 / IFC2x3 compliant entities into the STEP graph:
      1. Generates 22-character cryptographic IFC GlobalId (GUID).
      2. Creates `IfcCartesianPoint` coordinates and `IfcDirection` vectors.
      3. Builds `IfcAxis2Placement3D` and `IfcLocalPlacement` hierarchy.
      4. Synthesizes `IfcRectangleProfileDef` and `IfcExtrudedAreaSolid` representation items.
      5. Generates `IfcShapeRepresentation` (SweptSolid) and `IfcProductDefinitionShape`.
      6. Instantiates physical structural product (`IfcColumn`, `IfcBeam`, `IfcMember`).
      7. Establishes relational associations: `IfcRelContainedInSpatialStructure` and `IfcRelAssociatesMaterial`.
    - Computes analytical wireframe, bounding box, quantities takeoff (`Length`, `GrossVolume`), and updates schema conformance audits.
  - **Safe Element Deletion & Restoration Engine (`delete_structural_element`, `restore_structural_element`)**:
    - Safely excises structural elements and cleans up spatial containment, material association, and property set references.
    - Preserves deleted entity states and spatial links for zero-loss bidirectional transaction rollback.
  - **Expanded Transactional Undo/Redo Engine (`IfcEditCommand`, `IfcEditSession`)**:
    - Added first-class geometric command variants: `ModifyExtrusionDepth`, `TranslateElement`, `ModifyProfileDimensions`, `CreateElement`, `DeleteElementById`, and `RestoreElement`.
    - Full bidirectional inversion (`cmd.inverse()`) and human-readable engineering action descriptions.
  - **Comprehensive Unit Testing Suite**:
    - 5 rigorous new geometric & synthesis unit tests (`test_ifc_modify_element_extrusion_depth`, `test_ifc_modify_element_placement_translation`, `test_ifc_modify_element_profile_dimensions`, `test_ifc_create_and_delete_structural_element`, `test_ifc_geometric_transaction_undo_redo`), bringing test suite to **102/102 tests (100% pass)**.
- **OpenBIM Studio Interactive 3D Geometry Workbench (`src/views/industrialized.rs`)**:
  - Unified **3D Geometry & Bounding Box** interactive panel with live transaction stack (`↩ Undo`, `↷ Redo`).
  - **Swept Solid Extrusion Depth Controller**: Real-time slider and depth value mutator for element length/height.
  - **3D Cartesian Translation Controls**: Independent $[dX, dY, dZ]$ input controls for spatial relocation.
  - **Parametric Profile Resizer**: Independent Width $X$ and Depth $Y$ drag inputs for cross-sectional geometry updates.
  - **Safe Element Deletion**: One-click deletion button with full undo/redo transaction safety.
  - **Physical Structural Element Synthesis Form**:
    - Interactive dialog supporting `IfcColumn`, `IfcBeam`, and `IfcMember` instantiation.
    - Parametric length, profile dimensions, material designation, and target storey binding.
    - One-click synthesis directly into the live IFC model graph and spatial hierarchy.

---

## [v0.1.8] - 2026-11-08 — Phase 18: OpenBIM IFC Interactive Editing & Property/Metadata Mutation Engine

### Added
- **OpenBIM IFC Interactive Editing & Property/Metadata Mutation Engine (`src/core/ifc.rs`)**:
  - **Entity Identity & Classification Mutation**:
    - Real-time in-place editing of fundamental IFC product identity attributes: `Name`, `Description`, `ObjectType`, and `PredefinedType`.
    - Bi-directional synchronization: updates in-memory `IfcElementSummary` and synchronizes underlying ISO 10303-21 `StepEntity` parameter arrays.
  - **Dynamic Property Set (Pset) Management**:
    - Direct property key-value insertion, modification, and deletion within existing or custom property sets.
    - Full support for adding and deleting entire `IfcPropertySet` containers (`IfcPropertySingleValue`).
  - **Material Reassignment Engine**:
    - Dynamic reassignment of structural element materials (`IfcMaterial`).
    - Automatically updates or creates `IFCMATERIAL` entities and maintains `IFCRELASSOCIATESMATERIAL` relationships in the underlying STEP entity model.
  - **Spatial Storey Relocation Solver**:
    - Dynamic relocation of BIM elements across building storeys (`IfcBuildingStorey`).
    - Rebalances hierarchical containment within `IfcSpatialNode` and synchronizes `IFCRELCONTAINEDINSPATIALSTRUCTURE` spatial assignment links.
  - **Quantities Takeoff (Qto) Mutator**:
    - Updates custom dimensional parameters, volumes, and geometric takeoff attributes on elements.
  - **ISO 10303-21 STEP Physical File Serializer (`StepModel::to_step_string()`)**:
    - Full ISO 10303-21 STEP serializer capable of formatting modified header metadata, entity statements (`#id = TYPE(params);`), and complete STEP physical files ready for export and sharing with Revit, ArchiCAD, and Tekla.
  - **Transactional Command & Undo/Redo Session Engine (`IfcEditCommand`, `IfcEditSession`)**:
    - Command pattern implementation with bidirectional inversion (`inverse()`) for all mutation operations.
    - Full unlimited Undo/Redo stack with history cap, dirty state detection, and friendly engineering operation descriptions.
  - **Automatic Health Audit Recomputation (`recompute_health_audit()`)**:
    - Re-evaluates schema conformance, GUID uniqueness, dangling pointer checks, and storey containment metrics following every mutation.
  - **Unit Testing Suite**:
    - 5 comprehensive new unit tests (`test_ifc_update_element_attributes`, `test_ifc_property_set_mutation_and_deletion`, `test_ifc_material_reassignment`, `test_ifc_storey_reassignment`, `test_ifc_transaction_undo_redo_rollback`), expanding the verified suite to **97/97 tests (100% pass)**.
- **OpenBIM Studio Interactive UI (`src/views/industrialized.rs`)**:
  - Dedicated **Interactive Edit Mode** toggle (`✏️ Edit Entity` / `👁️ View Mode`) in the Property Inspector.
  - Transaction control bar featuring `↶ Undo` and `↷ Redo` buttons with action descriptions and `● MODIFIED` dirty state indicator.
  - Inline attribute modification form (Name, Description, ObjectType).
  - Material reassignment input bar with one-click application.
  - One-click building storey reassignment buttons.
  - Interactive Property Set manager with individual property deletion (`✕`), property addition, and entire Pset deletion (`🗑️ Delete Pset`).
  - Export mutated STEP physical file button directly from the inspector workbench.

---

## [v0.1.7] - 2026-11-07 — Phase 17: OpenBIM IFC Fundamental Reader, Spatial Hierarchy & Inspection Engine (ISO 10303-21 / ISO 16739)

### Added
- **OpenBIM Industry Foundation Classes (IFC) Core Engine (`src/core/ifc.rs`)**:
  - Full compliance with **ISO 10303-21:2016 (STEP Physical File format / SPF)**, **ISO 16739-1:2024 / buildingSMART IFC4.3 Add2**, **ISO 16739-1:2018 / IFC4 Reference View**, and **ISO 16739:2013 / IFC2x3 Coordination View 2.0**.
  - **Robust STEP ISO-10303-21 Tokenizer & Lexer**:
    - Complete multi-line STEP syntax parser handling block comments `/* ... */`, string escaping `''`, enum literals `.ENUM.`, entity references `#id`, and nested parameter lists `((...),(...))`.
    - Handles all standard line endings (`\r\n`, `\n`, `\r`) without data corruption.
  - **Spatial Hierarchy Decomposition Engine**:
    - Automatic spatial tree reconstruction: `IfcProject` ➔ `IfcSite` ➔ `IfcBuilding` ➔ `IfcBuildingStorey` ➔ `IfcSpace` / `IfcProduct`.
    - Resolves `IfcRelAggregates` and `IfcRelContainedInSpatialStructure` to map every building element to its host storey and elevation.
  - **BIM Metadata & Property Sets (Psets) Extractor**:
    - Extracts `GlobalId` (22-character GUID), `Name`, `Description`, `ObjectType`, and `PredefinedType`.
    - Resolves `IfcRelDefinesByProperties` linking elements to `IfcPropertySet` (`IfcPropertySingleValue`) and `IfcElementQuantity` (Qto: GrossVolume, NetVolume, Length, Width, Height).
  - **Material Association Extractor**:
    - Resolves `IfcRelAssociatesMaterial` for single `IfcMaterial`, layered `IfcMaterialLayerSet`, and profiled `IfcMaterialProfileSet`.
  - **Geometric Placement & 3D Bounding Box Solver**:
    - Recursively resolves `IfcLocalPlacement` matrices up to the world coordinate origin.
    - Evaluates swept solid bodies (`IfcExtrudedAreaSolid`), profile definitions (`IfcRectangleProfileDef`, `IfcIShapeProfileDef`), and 3D axis polylines (`IfcPolyline`).
    - Computes 3D Bounding Box $[X_{\min}, X_{\max}] \times [Y_{\min}, Y_{\max}] \times [Z_{\min}, Z_{\max}]$, centroid, dimensions, and wireframe visualization lines.
  - **IFC Model Health & Conformance Quality Assurance Audit**:
    - Automated model checking verifying schema conformance, duplicate `GlobalId` collisions, orphaned/uncontained products, and dangling pointer references.
    - Computes overall model Conformance Score (%) and actionable check reports.
  - **Bi-directional Bridge to StructuralModel**:
    - Direct converter `to_structural_model()` mapping IFC beams, columns, and members into analytical frame elements with node snapping and ground fixities for live SVD & ARI analysis.
  - **Embedded Reference Models**:
    - Built-in reference models: `sample_villa_ifc4()`, `sample_portal_frame_ifc4()`, and `sample_diagrid_tower_ifc2x3()`.
  - **Unit Testing Suite**: 6 verified test cases (`test_step_tokenizer_and_syntax_parser`, `test_ifc_spatial_hierarchy_decomposition`, `test_ifc_property_set_and_material_extraction`, `test_ifc_bounding_box_and_geometry_extractor`, `test_ifc_model_health_and_conformance_audit`, `test_ifc_to_structural_model_conversion`), expanding verified suite to **92/92 tests (100%)**.
- **Industrialized Construction Studio (`src/views/industrialized.rs`)**:
  - Added dedicated `"🌐 OpenBIM IFC Reader & Inspector"` workbench.
  - 4 Executive KPI cards: Schema Specification, Total STEP Entities, Contained BIM Products, Conformance Health Score.
  - Model loader toolbar with one-click presets (Sample Villa, Steel Portal Frame, Diagrid Skyscraper), Paste STEP SPF code, and Local File Path reader.
  - Interactive two-pane interface:
    - Left Pane: Collapsible Spatial Hierarchy Tree with element filter search.
    - Right Pane: Multi-tab Inspector featuring Entity Property Inspector, Conformance Health Audit, and 3D Bounding Box & Coordinates.
  - Action buttons to sync parsed IFC model directly to the 3D viewport and analytical solver.
- **3D Modeling & Geometry Viewport (`src/views/build.rs`)**:
  - Added archetype `ModelArchetype::OpenBimIfcModel3D` ("🌐 OpenBIM Duplex Villa (IFC4)").
  - Generates 3D coordinates and wireframe frames directly from parsed IFC STEP geometry.

---

## [v0.1.6] - 2026-11-07 — Phase 16: Structural Fire Engineering & Coupled Thermo-Mechanical Response Engine (AISC 360-16 / EN 1993-1-2)

### Added
- **Structural Fire Engineering & Coupled Thermo-Mechanical Engine (`src/core/fire.rs`)**:
  - Full compliance with **AISC 360-16 Appendix 4 (Structural Design for Fire Conditions)**, **Eurocode 1 (EN 1991-1-2)**, **Eurocode 2 (EN 1992-1-2)**, **Eurocode 3 (EN 1993-1-2)**, **ASTM E119 / UL 263**, **ISO 834-1**, and **ACI 216.1-14 / TMS 0216**.
  - **Standard & Parametric Design Fire Curves**:
    - **ISO 834-1 / EN 1991-1-2**: $T_g(t) = 20 + 345 \log_{10}(8t + 1)$ ($t$ in min, $T_g$ in $^\circ\text{C}$).
    - **ASTM E119 / UL 263**: Closed-form approximation modeling furnace heating.
    - **Eurocode Hydrocarbon (HC)**: Rapid rise curve modeling fuel spills reaching $1100^\circ\text{C}$ within 30 minutes.
    - **External Exposure & Parametric Natural Fire Curves**: Natural fire load decay modeling.
  - **Transient Cross-Section Heat Conduction**:
    - Unprotected structural members with combined convection ($h_c = 25\text{ W/m}^2\text{K}$) and Stefan-Boltzmann radiation ($\epsilon_{\text{res}} = 0.70, \sigma = 5.67 \times 10^{-8}\text{ W/m}^2\text{K}^4$).
    - Passive fire protection modeling for Spray-Applied Fire Resistive Material (SFRM / Gypsum), Thin-Film Intumescent Coatings, Board Encasement, and Solid Concrete Encasement.
    - Sizing tool for required insulation coating thickness $d_p$ (mm) to achieve target Fire Resistance Rating (FRR).
    - Section factor computation ($A_m / V$ in $\text{m}^{-1}$) for 3-sided beam and 4-sided column exposures.
  - **High-Temperature Material Constitutive Degradation**:
    - Structural steel yield strength retention factor $k_y(T) = f_{y,T} / f_y$ and elastic modulus factor $k_E(T) = E_T / E$ per AISC 360-16 Table A-4.2.1 and Eurocode 3 §3.2.
    - Concrete compressive strength degradation $k_c(T) = f_{c,T}' / f_c'$ for Siliceous vs Calcareous aggregates per Eurocode 2 §3.2.2.
    - Structural steel nonlinear thermal expansion strain: $\epsilon_{th}(T)$ modeling phase transformation plateaus.
    - Thermal restraint axial force generation: $P_{th} = \alpha_{\text{restr}} \cdot k_E(T) E A \cdot \epsilon_{th}$ accounting for multi-bay cold boundary continuity.
  - **Fire Endurance Rating (FRR) & Critical Temperature Evaluation**:
    - Automated evaluation of achieved fire rating period (e.g., R60, R90, R120, R180) tracking plastic capacity reduction $M_{n,T}(t) = Z_x f_{y,T}$ and column elevated buckling.
    - Identification of critical steel core runaway temperature $T_{\text{crit}} \approx 500^\circ\text{C} \sim 550^\circ\text{C}$.
  - **3D Fire Compartment Structural Frame Archetype (`generate_fire_compartment_archetype`)**:
    - Synthesizes an 18-node, 21-element structural steel frame isolating localized corner fire compartments with intumescent-coated W18x50 beams and W14x90 columns.
  - **Automated Calculation Dossier Generator (`generate_markdown_dossier`)**:
    - Professional calculation report detailing furnace exposure curves, member heat conduction, material degradation factors, and regulatory compliance matrix.
  - **Unit Testing Suite**: 4 verified test cases (`test_fire_gas_temperature_curves`, `test_high_temperature_material_degradation`, `test_structural_fire_assessment_pipeline`, `test_fire_compartment_archetype_generation`), expanding verified suite to **86/86 tests (100%)**.
- **Deep Analysis Studio (`src/views/analyze.rs`)**:
  - Added subtab `AnalyzeSubTab::StructuralFireEngineering` ("🔥 Structural Fire (AISC/EC3)").
  - 4 Executive KPI cards: Fire Exposure & Gas Temp ($T_g, T_s, A_m/V$), Fire Resistance Rating ($R_{\min}$ achieved vs target), Degraded Elevated Capacity ($D/C$ ratio, cold vs hot capacity), and Passive Insulation Sizing ($d_p$ applied vs required).
  - Interactive parameter controls: Fire curve combo, duration slider, target FRR slider, thermal restraint combo, section geometry dimensions (depth, width, web/flange thk, cover), passive protection systems, and material strengths.
  - 2D Vector CAD Canvas: Left side Gas Temp $T_g(t)$ vs Member Core Temp $T_s(t)$ with $T_{\text{crit}}$ threshold line; Right side elevated moment capacity $M_{n,T}(t)$ degradation curve with demand line and failure marker.
  - Complete AISC 360-16 & Eurocode 1/2/3 compliance audit matrix.
- **3D Modeling & CAD Viewport (`src/views/build.rs`)**:
  - Added `ModelArchetype::FireCompartmentFrame3D` ("🔥 Fire Compartment Steel Frame (AISC/EC3)").
  - Integrated into model selectors and dedicated camera preset (yaw 0.58, pitch 0.32, zoom 0.95).
  - 3D rendering of structural frame highlighting fire zone elements in orange/red with cold frame boundary members.
- **Results & Engineering Dossier View (`src/views/results.rs`)**:
  - Added action button `"🔥 Structural Fire Dossier (.md)"` saving `EigenGraph_Structural_Fire_AISC360_Dossier.md`.

## [v0.1.5] - 2026-11-07 — Phase 15: Blast Resistant Design & Explosive Impulse Dynamics Engine (UFC 3-340-02 / ASCE 59-11)

### Added
- **Blast Resistant Design & Explosive Dynamics Core Engine (`src/core/blast.rs`)**:
  - Full compliance with **DoD UFC 3-340-02 (Structures to Resist the Effects of Accidental Explosions)**, **ASCE 59-11 (Blast Protection of Buildings)**, **USACE PDC-TR-06-08 (SDOF Structural Response Limits)**, **GSA PBS-P100 / ISC Physical Security Standards**, and **ACI 318-19 / AISC 360-16**.
  - **Kingery-Bulmash Empirical Blast Wave Characterization**:
    - TNT equivalence conversion across high explosives (TNT: 1.00, ANFO: 0.82, Composition C-4: 1.30, PETN: 1.27, Commercial Dynamite: 0.92).
    - Surface burst ground reflection enhancement ($\alpha = 1.80$) and free-air spherical burst modeling.
    - Scaled distance formulation: $Z = \frac{R}{W_{\text{effective}}^{1/3}}$ ($\text{m/kg}^{1/3}$).
    - High-order Kingery-Bulmash polynomials for peak incident overpressure $P_{so}$ (kPa) and normally reflected overpressure $P_r$:
      $$P_r = 2 P_{so} \left(\frac{7 P_0 + 4 P_{so}}{7 P_0 + P_{so}}\right)$$
    - Shock front velocity $U_s$, arrival time $t_a$, positive phase duration $t_d$, and reflected specific impulse $i_r$.
    - Modified Friedlander waveform decay calibration with numerical Newton-Raphson solver for $b$:
      $$P(t) = P_r \left(1 - \frac{t}{t_d}\right) e^{-b \cdot t / t_d}$$
    - Target clearing relief effects on finite facade panels: clearing time $t_c = \frac{3 S}{C_r}$ reducing effective impulse.
  - **Single-Degree-of-Freedom (SDOF) Dynamic Transformation (Biggs Method)**:
    - Load-mass transformation factors $K_M, K_L, K_{LM}$ for simply supported, fixed-fixed, propped cantilever, and cantilever boundary conditions.
    - Dynamic Increase Factors (DIF) for high strain rates: $\text{DIF}_c = 1.25$ for concrete, $\text{DIF}_y = 1.20$ with $\text{SIF} = 1.10$ for steel reinforcement and structural steel.
    - Cracked section stiffness $k_E$ and dynamic ultimate plastic resistance $R_u$ derived from yield line and collapse mechanisms.
    - Response regime classification: Impulsive ($t_d / T_n < 0.10$), Dynamic ($0.10 \le t_d / T_n \le 2.0$), and Quasi-Static ($t_d / T_n > 2.0$).
  - **Central Difference Elastoplastic Numerical Time-History Integration**:
    - Step-by-step integration of $M_e \ddot{y} + C \dot{y} + R(y) = F_e(t)$ tracking displacement $y(t)$, velocity $\dot{y}(t)$, dynamic resistance $R(t)$, and support reaction forces $V(t)$.
    - Dynamic ductility ratio $\mu = \frac{y_{\max}}{y_e}$ and support rotation angle $\theta = \arctan\left(\frac{y_{\max}}{L/2}\right)$ (degrees).
    - Permanent plastic offset $y_{\text{perm}}$ and dynamic direct shear capacity checks ($V_u \le \phi V_{c,\text{dyn}}$).
  - **UFC 3-340-02 / PDC-TR-06-08 Damage Classification**:
    - Rigorous categorization into **B1: Superficial** ($\mu \le 1.0, \theta \le 0.5^\circ$), **B2: Low Damage** ($\mu \le 3.0, \theta \le 1.0^\circ$), **B3: Medium Damage** ($\mu \le 6.0, \theta \le 2.0^\circ$), **B4: High Damage** ($\mu \le 10.0, \theta \le 6.0^\circ$), and **B5: Hazardous Failure**.
  - **Pressure-Impulse (P-I) Iso-Damage Contours**:
    - Asymptotic analytical generation of P-I curves across impulsive and quasi-static regimes for all UFC damage tiers.
  - **3D High-Security Protective Blast Bunker Archetype (`generate_blast_bunker_archetype`)**:
    - Synthesizes a reinforced command bunker with 450mm shear walls, heavy roof diaphragm girders, and perimeter blast deflector revetments at standoff distance.
  - **Automated Calculation Dossier Generator (`generate_markdown_dossier`)**:
    - Markdown calculation report with Kingery-Bulmash parameters, SDOF dynamics, damage state audit, and regulatory compliance tables.
  - **Unit Testing Suite**: 4 verified test cases (`test_kingery_bulmash_blast_wave_parameters`, `test_sdof_system_and_plastic_resistance`, `test_blast_assessment_pipeline`, `test_blast_bunker_archetype_generation`), expanding verified suite to **82/82 tests (100%)**.
- **Deep Analysis Studio (`src/views/analyze.rs`)**:
  - Added subtab `AnalyzeSubTab::BlastResistantDesign` ("💣 Blast & Explosive Dynamics (UFC/ASCE)").
  - 4 Executive KPI cards: Threat Intensity ($W, R, Z, P_{so}$), Reflected Shock & Impulse ($P_r, t_d, i_r, U_s$), Dynamic SDOF Demands ($\mu, y_{\max}, y_e, \theta$), and UFC Damage Tier (B1-B5).
  - Interactive parameter controls: Charge mass, explosive type, burst type, standoff distance, component archetype, support condition, geometry dimensions, concrete $f_c'$, steel $f_y$, rebar ratio, and target damage level.
  - 2D Vector CAD Canvas: Left side Friedlander blast pressure waveform $P(t)$ with shaded impulse; Right side dynamic SDOF displacement history $y(t)$ with elastic yield limit $y_e$ and peak deflection marker.
  - Complete UFC 3-340-02 & ASCE 59-11 compliance audit matrix.
- **3D Modeling & CAD Viewport (`src/views/build.rs`)**:
  - Added `ModelArchetype::BlastResistantBunker3D` ("🛡️ Blast Protective Bunker (UFC 3-340-02)").
  - Integrated into model selectors and dedicated camera preset (yaw 0.68, pitch 0.36, zoom 1.05).
  - 3D rendering of blast bunker frame, roof diaphragm, and perimeter standoff deflector revetment.
- **Results & Engineering Dossier View (`src/views/results.rs`)**:
  - Added action button `"💣 Blast Dynamics Dossier (.md)"` saving `EigenGraph_Blast_Dynamics_UFC340_Dossier.md`.

## [v0.1.4] - 2026-11-07 — Phase 14: FEMA P-58 Seismic Performance Assessment & Probabilistic Loss Engine (FEMA P-58 / REDi)

### Added
- **FEMA P-58 Next-Generation Seismic Performance & Probabilistic Loss Core Engine (`src/core/fema_p58.rs`)**:
  - Full compliance with **FEMA P-58-1 / ATC-58 (Seismic Performance Assessment of Buildings)**, **Arup REDi™ Rating System**, **ASCE 7-22 / ASCE 41-17**, and **ISO 14040/14044 & EN 15978 (Embodied Carbon CO2e)**.
  - **Component Vulnerability & Lognormal Fragility Functions**:
    - Rigorous lognormal fragility evaluation: $P(DS \ge ds | EDP) = \Phi\left(\frac{\ln(EDP/\theta)}{\beta}\right)$ with median demand capacity $\theta$ and composite dispersion $\beta$.
    - Standard FEMA P-58 component library covering drift-sensitive structural frames (RC special moment joints), drift-sensitive non-structural enclosures (drywall partitions, structural silicone glazed curtain walls), acceleration-sensitive MEP systems (suspended acoustic ceiling grids, seismically braced chilled water piping), and building contents.
  - **Sequential Damage State & Consequence Integration**:
    - Exact damage state probability partitioning: $P(DS = i) = P(DS \ge ds_i) - P(DS \ge ds_{i+1})$.
    - Consequence calculation for Direct Repair Costs ($ USD), Repair Labor (Person-Days), and Embodied Carbon ($kg\text{ CO}_2e$).
  - **Downtime & Business Interruption (BI) Timeline (REDi™ Methodology)**:
    - Critical-path labor synthesis modeling parallel vs serial labor allocation per story:
      $$T_{\text{downtime}} = \max\left(\frac{\text{Labor}}{\text{Workers}_{\text{story}}}\right) \cdot 1.35 + T_{\text{permitting/inspection}}$$
    - Business interruption financial exposure: $\text{Loss}_{\text{BI}} = T_{\text{downtime}} \cdot \text{Revenue}_{\text{daily}}$.
  - **Residual Drift Demolition & Global Collapse Risk (§7.3)**:
    - Demolition probability due to excessive residual drift ($RDR > 0.010\text{ rad}$) per Ramirez & Miranda.
    - Global dynamic collapse fragility under Maximum Considered Earthquake (MCE).
  - **REDi™ Resilience Tier Classification**:
    - Automatic classification into **REDi™ Platinum** ($\le 7$ days downtime, $\le 5\%$ loss), **REDi™ Gold** ($\le 30$ days, $\le 10\%$ loss), **REDi™ Silver** ($\le 60$ days, $\le 20\%$ loss), or **Baseline Code**.
  - **10-Story FEMA P-58 Hospital & Resilience Archetype Generator (`generate_fema_resilience_archetype`)**:
    - Synthesizes a 10-story medical center (132 nodes, 280 frame members, perimeter curtain wall ties).
  - **Markdown Calculation Dossier Generator (`generate_markdown_dossier`)**:
    - Exports comprehensive calculation report detailing REDi compliance, story-by-story loss breakdowns, labor Gantt paths, and carbon footprint.
  - **Unit Testing Suite**: 4 verified test cases (`test_normal_cdf_approximation`, `test_fragility_evaluation`, `test_fema_p58_loss_evaluation`, `test_fema_resilience_archetype`).
- **Deep Analysis Studio (`src/views/analyze.rs`)**:
  - Added subtab `AnalyzeSubTab::FemaP58LossAssessment` ("🏛️ FEMA P-58 Loss & REDi").
  - 4 KPI real-time telemetry cards (Direct Repair Loss, Recovery Downtime, Embodied Carbon, REDi Rating Tier).
  - Multi-parameter interactive controls: Asset replacement value, floor area, story count, story height, site PGA, spectral acceleration $S_a(T_1)$, daily revenue, target REDi tier, and labor limits.
  - 2D Vector CAD canvas: Component fragility curves plot ($P(DS \ge ds | EDP)$ vs EDP) and story repair cost breakdown & labor days horizontal bar chart.
  - Complete FEMA P-58 and REDi performance compliance audit table.
- **3D Modeling & CAD Viewport (`src/views/build.rs`)**:
  - Added `ModelArchetype::FemaP58HospitalArchetype` ("🏥 FEMA P-58 Hospital (10-Story)").
  - Integrated into both model dropdown menus and camera presets (yaw 0.62, pitch 0.34, zoom 0.94).
  - 3D viewport rendering of 10-story hospital frame with highlighted structural elements and perimeter facade ties.
- **Results & Engineering Dossier View (`src/views/results.rs`)**:
  - Added button `"🏛️ FEMA P-58 Dossier (.md)"` saving `EigenGraph_FEMA_P58_Seismic_Loss_Dossier.md`.

---

## [v0.1.3] - 2026-11-07 — Phase 13: Nonlinear Dynamic Direct Time-History Analysis (NL-THA) Engine (ASCE 7-22 / SNI 1726:2019)

### Added
- **Nonlinear Dynamic Direct Time-History Analysis (NL-THA) Core Engine (`src/core/time_history.rs`)**:
  - Full compliance with **ASCE 7-22 Chapter 16 (Nonlinear Response History Analysis)**, **SNI 1726:2019 Pasal 11 (Prosedur Riwayat Respon Non-Linier)**, **NIST GCR 17-917-46 / ATC-114**, and **PEER/TBI Guidelines (2017)**.
  - **Ground Motion Suite & Stochastic Kanai-Tajimi Scaling (§16.2)**:
    - Built-in canonical earthquake records: 1994 Northridge (Sylmar Station, PGA 0.84g), 1995 Kobe (Takatori Station, PGA 0.67g), 1940 El Centro (Array #9, PGA 0.35g), 1999 Chi-Chi Taiwan (TCU068, PGA 0.51g), and 2004 Sumatra Mega-thrust Interface (PGA 0.42g).
    - Synthetic non-stationary acceleration wave generation using Gamma-envelope modulated filtered stochastic processes.
    - Amplitude scale factor $SF$ per ASCE 7-22 §16.2.3 over period range $[0.2 T_1, 1.5 T_1]$.
  - **Hilber-Hughes-Taylor ($\text{HHT}-\alpha$) Implicit Numerical Solver**:
    - Unconditionally stable integration scheme with numerical dissipation controlled by parameter $\alpha \in [-1/3, 0]$ (nominal $\alpha = -0.10$, $\beta = \frac{(1-\alpha)^2}{4} = 0.3025$, $\gamma = \frac{1 - 2\alpha}{2} = 0.60$).
    - Eliminates spurious high-frequency mode chatter while preserving accurate low-frequency structural dynamic response.
  - **Bouc-Wen Differential Hysteresis Restoring Force Model**:
    - Smooth inelastic transitions governed by:
      $$\dot{z} = \dot{u} \left[ A - (\beta_w \operatorname{sgn}(\dot{u}) |z|^n + \gamma_w |z|^n) \right]$$
    - Tangent stiffness matrix Newton-Raphson nonlinear equilibrium solver with convergence tolerance $\epsilon_r \le 10^{-4}$.
  - **Instantaneous Dynamic Energy Balance Integration (ATC-114)**:
    - Simultaneous time integration of Kinetic ($E_K$), Viscous Damping ($E_D$), Elastic Strain ($E_S$), and Inelastic Hysteretic ($E_H$) energy:
      $$E_I(t) = E_K(t) + E_D(t) + E_S(t) + E_H(t)$$
    - Continuous tracking of instantaneous energy balance residual error:
      $$\epsilon_E(t) = \frac{|E_I(t) - (E_K + E_D + E_S + E_H)|}{\max(E_I(t), 1.0)} \le 2.5\%$$
      strictly satisfying NIST GCR 17-917-46 numerical stability bounds.
  - **Multi-Story Dynamic Response Profile Envelopes (§16.4)**:
    - Story-by-story peak lateral displacement, inter-story drift ratio $\theta_i$, story shear $V_{\text{story}}$, and overturning moment $M_{\text{OT}}$ with higher-mode whiplash amplification.
    - Mean inter-story drift check $\theta_{\max} \le 0.020$ per ASCE 7-22 §16.4.1.
    - Base shear lower bound verification $V_b \ge 0.85 V_{\text{ELF}}$ per ASCE 7-22 §16.4.2.
  - **16-Story NL-THA Test Tower Archetype Generator (`generate_nl_tha_tower_archetype`)**:
    - Synthesizes a 60m 16-story tall building (204 nodes, 432 frame members, 8 inelastic hysteretic link elements across lower stories).
  - **Markdown Calculation Dossier Generator (`generate_markdown_dossier`)**:
    - Exports comprehensive professional calculation dossier with code compliance audit matrix, multi-story drift tables, and energy conservation ledger.
  - **Unit Test Suite**: 4 verified test cases (`test_ground_motion_generation`, `test_nonlinear_time_history_evaluation`, `test_energy_balance_conservation`, `test_nl_tha_tower_archetype_generation`).
- **Deep Analysis Studio (`src/views/analyze.rs`)**:
  - Added subtab `AnalyzeSubTab::TimeHistoryAnalysis` ("🌊 NL-THA Time-History").
  - 4 KPI real-time telemetry cards (Peak Roof Disp & Drift, Peak Base Shear, Inelastic Energy Dissipation, Energy Balance Error).
  - Multi-parameter interactive controls: Ground motion record selection, scale factor, time step $\Delta t$, duration, tower geometry, seismic weight, period $T_1$, Rayleigh damping $\xi$, yield coefficient $C_y$, and strain hardening $\alpha_p$.
  - 2D Vector CAD canvas rendering dynamic response traces ($a_g(t)$, $u_{\text{roof}}(t)$) and height vs inter-story drift ratio envelope profile with 2.0% ASCE 7-22 limit line.
  - Complete ASCE 7-22 Chapter 16 and SNI 1726:2019 Pasal 11 regulatory compliance audit table.
- **3D Modeling & CAD Viewport (`src/views/build.rs`)**:
  - Added `ModelArchetype::TimeHistoryTower3D` ("🌊 NL-THA Tower (16-Story)").
  - Integrated into both model dropdown menus and camera presets (yaw 0.55, pitch 0.30, zoom 0.88).
  - 3D viewport rendering of 16-story tower frame with highlighted Bouc-Wen inelastic yielding links.
- **Results & Engineering Dossier View (`src/views/results.rs`)**:
  - Added button `"🌊 NL-THA Dossier (.md)"` saving `EigenGraph_NL_THA_Time_History_Dossier.md`.

---

## [v0.1.2] - 2026-11-07 — Phase 12: Seismic Base Isolation & Energy Dissipation Engine (ASCE 7-22 Ch 17 & 18 / SNI 1726:2019 Pasal 12 & 13)

### Added
- **Seismic Base Isolation & Supplemental Energy Dissipation Core Engine (`src/core/isolation.rs`)**:
  - Full compliance with **ASCE 7-22 Chapter 17 (Seismic Design of Seismically Isolated Structures)**, **ASCE 7-22 Chapter 18 (Seismic Design of Structures with Damping Systems)**, **SNI 1726:2019 Pasal 12 & 13**, and **FEMA P-1051 Chapter 12**.
  - **Lead-Rubber Bearings (LRB) Bilinear Hysteretic Formulation (§17.5)**:
    - Characteristic yield strength: $Q_d = \sigma_{pb} \cdot A_{pb}$ where $\sigma_{pb} = 10.2\text{ MPa}$ shear yield stress of high-purity lead.
    - Post-yield stiffness: $k_d = \frac{G \cdot A_{bonded}}{T_r}$ where $T_r = n_{layers} \cdot t_{layer}$ total bonded rubber thickness.
    - Elastic initial stiffness: $k_u = 10 \cdot k_d$ with yield displacement $D_y = \frac{Q_d}{k_u - k_d}$.
    - Primary shape factor $S_1 = \frac{D_o - d_{pb}}{4 t_r}$ and secondary shape factor $S_2 = \frac{D_o}{T_r}$.
    - Compression modulus $E_c = 6 G S_1^2$ and effective compression modulus $E_c' = \frac{E_c K_{bulk}}{E_c + K_{bulk}}$ ($K_{bulk} \approx 2000\text{ MPa}$).
    - Haringx critical buckling load at zero displacement: $P_{cr0} = \sqrt{\frac{1}{3} P_E P_S}$.
    - Reduced critical buckling capacity at design displacement $D_M$: $P_{cr}(D_M) = P_{cr0} \cdot \left(1 - \frac{D_M}{D_o}\right)$ with safety factor $SF \ge 2.0$.
  - **Friction Pendulum System (FPS) Formulation (§17.5)**:
    - Spherical concave surface kinematics with effective pendulum radius $R_{eff} = R - h_{slider}$.
    - Velocity-dependent sliding friction $\mu = f_{\max} - (f_{\max} - f_{\min}) e^{-a \dot{u}}$.
    - Restoring pendulum stiffness $k_r = \frac{W}{R_{eff}}$ and post-sliding stiffness $K_{eff} = \frac{W}{R_{eff}} + \frac{\mu W}{D}$.
    - Pendulum natural isolation period: $T_M = 2\pi \sqrt{\frac{R_{eff}}{g (1 + \mu \frac{R_{eff}}{D_M})}}$.
  - **Fluid Viscous Dampers (FVD) Supplementary Damping (§18.2 - §18.4)**:
    - Constitutive power law: $F_D = C \cdot \operatorname{sgn}(\dot{u}) |\dot{u}|^\alpha$ with velocity exponent $0.30 \le \alpha \le 1.0$.
    - Energy dissipated per cycle: $E_D = 4 \lambda_m C (\omega D)^\alpha D$ where $\lambda_m = \frac{\sqrt{\pi} \Gamma(1 + \alpha/2)}{\Gamma(1.5 + \alpha/2)}$.
    - Supplemental viscous damping ratio: $\beta_V = \frac{\sum E_{D,j}}{4 \pi E_S}$ where $E_S = \frac{1}{2} M \omega^2 D^2$.
    - Total combined damping: $\beta_{total} = \beta_{inherent} + \beta_V$ with damping coefficient $B_M$ (ASCE 7-22 Table 17.5-1).
  - **MCE_R Iterative Convergence & Accidental Torsion Amplification**:
    - Newton-Raphson / secant iterative convergence for maximum displacement $D_M = \frac{g S_{M1} T_M}{4 \pi^2 B_M}$.
    - Total maximum lateral displacement including accidental torsion per ASCE 7-22 Eq. 17.5-3:
      $$D_{TM} = D_M \left[ 1 + y \left(\frac{12 e}{b^2 + d^2}\right) \right]$$
    - Minimum required seismic moat clear gap: $\delta_{\text{moat}} \ge 1.10 D_{TM}$.
  - **Superstructure Base Shear Reduction (§17.5.4)**:
    - Superstructure factored design base shear: $V_s = \frac{K_M D_M}{R_I}$ (where $R_I \approx 1.5 - 2.0$), yielding over 60-70% base shear reduction compared to conventional fixed-base design.
  - **3D Base-Isolated Hospital Model Archetype Generator (`generate_base_isolated_hospital_archetype`)**:
    - Synthesizes an 8-story essential facility (120 nodes, 12 base isolator elements, spatial frame grid 24m x 16m x 28.8m).
  - **Markdown Calculation Dossier Generator (`generate_markdown_dossier`)**:
    - Exports comprehensive design verification report with KaTeX mathematical formulas, code citations, and limit state verification matrix.
- **Deep Analysis Studio (`src/views/analyze.rs`)**:
  - Added subtab `AnalyzeSubTab::BaseIsolationDesign` ("🛡️ Base Isolation & Dampers").
  - 4 Executive KPI cards: Effective Period $T_M$, Total Displacement $D_{TM}$, System Damping $\beta_{eff}$, and Base Shear Reduction vs Fixed Base.
  - Parameter Studio with interactive controls for isolator technology (LRB, FPS, FVD), seismic demands ($W, S_{M1}, S_{MS}, T_1, R_I$), plan dimensions, and device properties.
  - **2D Vector CAD Blueprint & Dynamic Hysteresis Canvas**:
    - Device cross-section details: LRB steel laminates, rubber layers, and central lead plug; FPS concave dish and articulated slider puck; FVD cylinder, piston rod, and clevis joints.
    - Live force-displacement ($F$ vs $D$) hysteretic dissipation loop with shaded energy area $E_D$ and backbone yield limits.
  - ASCE 7-22 / SNI 1726:2019 regulatory compliance audit table with 6 pass/fail limit checks.
- **3D Modeling & Geometry Studio (`src/views/build.rs`)**:
  - Added `ModelArchetype::BaseIsolatedHospital3D` ("🛡️ Base-Isolated Hospital (8-Story)").
  - Integrated into 3D turntable viewport, camera presets, and element generator.
- **Results Studio (`src/views/results.rs`)**:
  - Added export action `"🛡️ Base Isolation Dossier (.md)"` generating `EigenGraph_Seismic_Base_Isolation_Damper_Dossier.md`.

---

## [v0.1.1] - 2026-11-07 — Phase 11: Special Structural Concrete Shear Walls & Diagonally Reinforced Coupling Beams Engine (ACI 318-19 / SNI 2847:2019)

### Added
- **RC Special Structural Shear Walls & Coupling Beams Engine (`src/core/shear_wall.rs`)**:
  - Full compliance with **ACI 318-19 Chapter 18 (Section 18.10)**, **SNI 2847:2019 Pasal 18.10**, and **NIST GCR 11-917-11 / NEHRP Seismic Design Brief No. 6**.
  - **In-Plane Nominal Shear Strength ($V_n$) (§18.10.4)**:
    - Aspect ratio factor $\alpha_c$ calculation:
      $$\alpha_c = \begin{cases} 0.25, & \frac{h_w}{l_w} \le 1.5 \\ 0.25 - 0.16\left(\frac{h_w}{l_w} - 1.5\right), & 1.5 < \frac{h_w}{l_w} < 2.0 \\ 0.17, & \frac{h_w}{l_w} \ge 2.0 \end{cases}$$
    - Nominal in-plane shear strength: $V_n = A_{cv} \left( \alpha_c \lambda \sqrt{f'_c} + \rho_t f_y \right) \le 0.83 A_{cv} \sqrt{f'_c}$ (with $\phi = 0.75$).
    - Automated enforcement of double curtain web reinforcement when $V_u > 0.17 A_{cv} \lambda \sqrt{f'_c}$ or $h_w / l_w \ge 2.0$.
  - **Dual Boundary Element Trigger Analysis (§18.10.6)**:
    - **Displacement-Based Trigger (§18.10.6.2)**: Evaluates $c \ge \frac{l_w}{600 (\delta_u / h_w)}$ with mandatory minimum design drift ratio $\delta_u / h_w \ge 0.005$.
    - **Stress-Based Trigger (§18.10.6.3)**: Normal compressive stress check $\sigma_{\text{comp}} = \frac{P_u}{A_g} + \frac{M_u (l_w/2)}{I_g} \ge 0.20 f'_c$.
  - **Special Boundary Element (SBE) Geometric & Confinement Detailing**:
    - Horizontal boundary extension length: $b_E \ge \max\left(c - 0.1 l_w, \; \frac{c}{2}\right)$.
    - Out-of-plane lateral buckling thickness: $b \ge \max\left(t_w, \; \frac{h_u}{16}\right)$.
    - Transverse confinement reinforcement ratio per Table 18.10.6.4(f):
      $$\frac{A_{sh}}{s \cdot b_c} \ge \max\left( 0.3 \left(\frac{A_g}{A_{ch}} - 1\right) \frac{f'_c}{f_{yt}}, \; 0.09 \frac{f'_c}{f_{yt}} \right)$$
    - Maximum vertical tie pitch: $s_{\max} = \min\left( \frac{b}{3}, \; 6 d_{b,\text{long}}, \; 150\text{ mm} \right)$.
  - **Diagonally Reinforced Coupling Beams (§18.10.7)**:
    - Classification across aspect ratios: Diagonally Reinforced ($l_n / h < 2.0$ and $V_u \ge 0.33 \lambda \sqrt{f'_c} b_w d$), Conventional Flexural ($l_n / h \ge 4.0$), and Transition ($2.0 \le l_n / h < 4.0$).
    - Diagonal bar inclination angle: $\alpha = \arctan\left(\frac{h - 2 d'}{l_n}\right)$.
    - Nominal shear strength: $V_n = 2 A_{vd} f_y \sin\alpha \le 0.83 \sqrt{f'_c} b_w d$.
    - Confinement hoop cages around individual diagonal bar groups: $A_{sh} \ge 0.09 s b_c \frac{f'_c}{f_{yt}}$.
  - **Wall Pier Axial-Flexural Capacity**:
    - Sliced Whitney stress block ($a = \beta_1 c$) with boundary rebar couple and axial force $P_u$ for design moment $\phi M_n$.
  - **Coupled Core Tower 3D Archetype Generator (`generate_coupled_core_archetype`)**:
    - Synthesizes 12-story 3D structural model with two C-shaped wall piers, central door openings, and diagonally reinforced coupling beams.
  - **Engineering Markdown Dossier Generator (`generate_markdown_dossier`)**:
    - Generates complete calculation report with KaTeX mathematical formulas, code citations, and limit state verification matrix.
- **Deep Analysis Studio (`src/views/analyze.rs`)**:
  - Added subtab `AnalyzeSubTab::ShearWallDesign` ("🏢 RC Shear Walls & Coupling Beams (ACI 318 / SNI 2847)").
  - 4 Executive metric cards: Overall Compliance & Governing DCR, Boundary Element (SBE) Trigger, In-Plane Web Shear $\phi V_n$, and Coupling Beam $\phi V_n$.
  - Parameter Studio with interactive controls for wall geometry, material strengths, applied demands, and coupling beam parameters.
  - 2D Vector CAD Canvas displaying wall plan section with SBE boundary zones, confining tie cages, longitudinal rebar dots, double curtain web mesh, neutral axis indicator, and coupling beam elevation with diagonal X-reinforcement.
  - Comprehensive ACI 318-19 / SNI 2847:2019 regulatory compliance audit table.
- **CAD Viewport Modeling Studio (`src/views/build.rs`)**:
  - Added `ModelArchetype::CoupledShearWallCore3D` ("🏢 Coupled Shear Wall Core (12-Story)").
  - 3D Viewport renders 12-story coupled shear wall tower with vertical wall piers, floor spandrels, and central coupling beams.
- **Results View Disk Exporters (`src/views/results.rs`)**:
  - Added export action `🏢 RC Shear Wall Dossier (.md)` generating `EigenGraph_RC_Shear_Wall_Coupling_Beam_Dossier.md`.

---

## [v0.1.0] - 2026-11-07 — Phase 10: Prestressed & Post-Tensioned Concrete Engine (ACI 318-19 / SNI 2847:2019)

### Added
- **Prestressed & Post-Tensioned Computational Engine (`src/core/prestressed_concrete.rs`)**:
  - Full compliance with **ACI 318-19 Chapter 24**, **SNI 2847:2019 Pasal 24**, **AASHTO LRFD Section 5**, and **PCI Design Handbook (8th Edition)**.
  - **Parametric Tendon Profiling**:
    - Parabolic draped continuous profile with midspan and end eccentricities: $e(x) = e_{mid} - 4(e_{mid} - e_{end})(x/L - 0.5)^2$.
    - Harped 2-point depressed profile with adjustable hold-down stations.
    - Straight eccentric profile with constant eccentricity.
    - Equivalent upward balancing load calculations: $w_{bal} = \frac{8 P_{eff} (e_{mid} - e_{end})}{L^2}$.
  - **Prestress Losses Engine**:
    - **Immediate Losses**:
      - Curvature & wobble friction loss: $\Delta f_{pF}(x) = f_{pj} [1 - \exp(-(\mu \alpha + K x))]$.
      - Anchorage seating draw-in slip: $\Delta f_{pA} = \frac{\Delta_{anc} E_p}{L_{set}}$.
      - Concrete elastic shortening: $\Delta f_{pES} = \frac{N-1}{2N} \frac{E_p}{E_{ci}} f_{cgp}$ (post-tensioned) or $\frac{E_p}{E_{ci}} f_{cgp}$ (pre-tensioned).
    - **Long-Term Time-Dependent Losses**:
      - Concrete creep loss: $\Delta f_{pCR} = K_{cr} \frac{E_p}{E_c} (f_{cgp} - f_{cds})$.
      - Concrete shrinkage loss: $\Delta f_{pSH} = 8.2 \times 10^{-6} K_{sh} E_p (1 - 0.06 V/S)(100 - RH)$.
      - Steel strand relaxation: $\Delta f_{pRE} = [K_{re} - J(\Delta f_{pSH} + \Delta f_{pCR} + \Delta f_{pES})] C$.
      - Total effective prestress: $P_{eff} = A_{ps} f_{se}$.
  - **Concrete Fiber Stresses & Serviceability Verification**:
    - Initial transfer stage stresses ($\sigma_{top,i}, \sigma_{bot,i}$) vs allowable compression ($0.60 f'_{ci}$) and tension ($0.25 \sqrt{f'_{ci}}$).
    - Service limit state stresses ($\sigma_{top,s}, \sigma_{bot,s}$) vs sustained compression ($0.45 f'_c$) and tensile limits.
    - ACI 318-19 Table 24.5.2.1 Classification: Class U (Uncracked), Class T (Transition), Class C (Cracked).
    - Cracking moment: $M_{cr} = S_b [f_r + P_{eff}/A + P_{eff} e / S_b]$.
  - **Ultimate Flexural & Shear Capacities**:
    - Prestressing steel stress $f_{ps}$ at nominal flexural strength per ACI 318-19 Eq. 20.3.2.3.1.
    - Equivalent rectangular Whitney compression block depth $a$ and neutral axis $c$.
    - Flexural capacity $\phi M_n$ and minimum flexure requirement ($\phi M_n \ge 1.2 M_{cr}$).
    - Concrete shear resistance: flexure-shear cracking $V_{ci}$ and web-shear cracking $V_{cw}$, with stirrup reinforcement capacity $V_s$ and design strength $\phi V_n$.
  - **Camber & Deflection Analytics**:
    - Upward prestress camber $\Delta_{p,i}$, self-weight downward deflection $\Delta_{sw}$, net transfer camber, and long-term camber via PCI Martin Multipliers.
  - **Bridge Archetype Generator (`generate_bridge_archetype_elements`)**:
    - Generates complete 3D structural model with precast AASHTO girders, transverse RC diaphragms, and 3D parabolic tendon runs.
  - **Markdown Design Dossier Generator (`generate_markdown_dossier`)**:
    - Formats comprehensive engineering calculation reports with KaTeX math, tables, and regulatory compliance status.
- **Deep Analysis Studio (`src/views/analyze.rs`)**:
  - Added `AnalyzeSubTab::PrestressedDesign` ("🪢 PT Concrete (ACI 318)").
  - 4 Metric cards: Overall Compliance & Governing DCR, Effective Prestress & Total Losses, Flexural Strength $\phi M_n$, and Service Class & Camber.
  - Real-time 2D Vector Canvas rendering girder elevation, neutral axis, and parabolic tendon trajectory with anchor plates.
  - Prestress losses breakdown, fiber stress checks table, and code compliance audit.
- **CAD Viewport Modeling Studio (`src/views/build.rs`)**:
  - Added `ModelArchetype::PrestressedGirderBridge3D` ("🌉 Prestressed Bridge (30m PT Girder)").
  - 3D Viewport renders precast I-girders, transverse diaphragms, and 3D internal parabolic prestressing cables.
- **Results View Disk Exporters (`src/views/results.rs`)**:
  - Added export action `🪢 Prestressed PT Dossier (.md)` generating `EigenGraph_Prestressed_Concrete_Dossier.md`.

---

## [v0.0.9] - 2026-11-06 — Phase 9: Modeling & Advanced Computational Geometry Engine

### Added
- **Green's Theorem Arbitrary Cross-Section Property Integrator (`src/core/geometry_engine.rs`)**:
  - Exact boundary line integrals for arbitrary 2D closed planar polygons (solid, hollow, asymmetric):
    - Area: $A = \frac{1}{2} \sum (x_i y_{i+1} - x_{i+1} y_i)$
    - Centroids: $\bar{x} = \frac{1}{6A} \sum (x_i + x_{i+1}) (x_i y_{i+1} - x_{i+1} y_i), \quad \bar{y} = \frac{1}{6A} \sum (y_i + y_{i+1}) (x_i y_{i+1} - x_{i+1} y_i)$
    - Moments of Inertia: $I_{xx0} = \frac{1}{12} \sum (y_i^2 + y_i y_{i+1} + y_{i+1}^2) (x_i y_{i+1} - x_{i+1} y_i), \quad I_{yy0} = \frac{1}{12} \sum (x_i^2 + x_i x_{i+1} + x_{i+1}^2) (x_i y_{i+1} - x_{i+1} y_i)$
    - Product of Inertia: $I_{xy0} = \frac{1}{24} \sum (x_i y_{i+1} + 2 x_i y_i + 2 x_{i+1} y_{i+1} + x_{i+1} y_i) (x_i y_{i+1} - x_{i+1} y_i)$
    - Parallel axis theorem centroidal transfer: $I_{xx} = I_{xx0} - A \bar{y}^2, \quad I_{yy} = I_{yy0} - A \bar{x}^2, \quad I_{xy} = I_{xy0} - A \bar{x} \bar{y}$
    - Principal Moments of Inertia ($I_1, I_2$) and Principal Orientation Angle $\theta_p$:
      $$\theta_p = \frac{1}{2} \operatorname{atan2}(-2 I_{xy}, \; I_{xx} - I_{yy})$$
    - Radii of gyration ($r_x, r_y$), elastic section moduli ($S_x, S_y$), sliced numerical plastic section moduli ($Z_x, Z_y$), polar moment ($J$), and Saint-Venant torsional constant ($J_t$).
  - Standard parametric polygon generators for Wide-Flange (I-Shape), Channel (UNP), Angle (L-Shape), and T-Shape profiles.
- **Parametric Spatial Structure Generator Wizard (`ParametricSpatialWizard`)**:
  - **3D Double-Layer Space Truss (MERO Octet Lattice)**: Parametric orthogonal two-way top/bottom chord grid with diagonal pyramidal web bracing and perimeter support restraints.
  - **3D Geodesic Lattice Dome (Schwedler Dome)**: Spherical geometry subdivision generating hoop rings, meridional ribs, and diagonal triangulating ties with crown apex convergence.
  - **Hyperbolic Paraboloid Ruled Shell (Hypar Saddle / Inverted Umbrella)**: Doubly-curved ruled surface generation ($z = 4c \cdot \xi \cdot \eta$) with orthogonal straight-line structural framing and 4-node quad shell panel meshing.
  - **Multi-Story Building Frame with Core Walls**: Parametric multi-story 3D building tower with columns, floor beams, and central concrete shear wall shell panels.
- **Topological Mesh Operations & Cleaners (`MeshTopologyEngine`)**:
  - Distance-based node consolidation: merges duplicate vertices within $\varepsilon_{\text{tol}}$ tolerance and remaps 1D beam and 2D shell element connectivity.
  - Certified Wavefront 3D OBJ mesh exporter (`export_to_obj`) and AutoCAD 3D DXF exporter (`export_to_dxf_3d`).
  - Automated computational geometry markdown dossier generator (`generate_markdown_dossier`).
- **Interactive 3D Viewport Modeling Studio in Build View (`src/views/build.rs`)**:
  - Added `SidebarTab::Geometry` ("📐", "Geometry & Wizard") to the CAD activity bar.
  - Integrated Parametric Generator Wizard with interactive sliders for span dimensions, divisions, radius, rings, and stories.
  - Integrated Green's Theorem Section Integrator Studio with 2D vector cross-section canvas preview, centroid crosshair reticle, and live structural section properties table.
  - Added 4 new spatial archetypes to 3D Viewport Top Ribbon and camera presets.
- **Results View Disk Exporters (`src/views/results.rs`)**:
  - Added export action `📐 Spatial Geometry Dossier (.md)` generating `EigenGraph_Computational_Geometry_Dossier.md`.
  - Added export action `🧊 Wavefront 3D (.obj)` generating `EigenGraph_Spatial_Truss.obj`.

---

## [v0.0.8] - 2026-11-06 — Phase 8: Composite Steel-Concrete Beam & Slab Design (AISC 360-16 / SNI 1729:2020)

### Added
- **AISC 360-16 Chapter I & SNI 1729:2020 Pasal I Composite Beam Design Engine (`src/core/composite.rs`)**:
  - **Headed Shear Stud Nominal Strength ($Q_n$)**:
    $$Q_n = \min\left(0.5 A_{sa} \sqrt{f'_c E_c}, \; R_g R_p A_{sa} F_u\right)$$
    Accounting for deck geometry reduction factors $R_g$ and $R_p$ across perpendicular and parallel ribs per AISC Eq. I8-1.
  - **Effective Slab Flange Width ($b_{\text{eff}}$)**:
    $$b_{\text{eff}} = \min\left(\frac{L}{4}, \; S\right)$$
    Per AISC 360-16 Section I3.1.1 for interior beams with span $L$ and beam center-to-center spacing $S$.
  - **Plastic Neutral Axis (PNA) Tracking & Plastic Moment Strength ($M_n$)**:
    - Complete plastic stress distribution analysis across concrete compression zone $C_c = 0.85 f'_c b_{\text{eff}} a$, shear stud force $\sum Q_n$, and steel tensile/compressive zones.
    - Automated detection and equilibrium calculation across 3 distinct cases:
      - **Case 1 (PNA in concrete slab)**: $a \le t_c$, entire steel section in pure tension.
      - **Case 2 (PNA in top steel flange)**: Concrete in compression, portion of top flange in compression, rest in tension.
      - **Case 3 (PNA in steel web)**: Concrete and top flange in compression, web depth in tension/compression equilibrium.
    - Design flexural capacity $\phi M_n$ with LRFD resistance factor $\phi_b = 0.90$.
  - **Degree of Composite Action Check**:
    $$\text{Composite Action} = \frac{\sum Q_n}{C_{\text{max}}} \times 100\% \ge 25\%$$
    Enforces AISC 360-16 minimum threshold of 25% composite interaction.
  - **Unshored Construction Stage Wet Concrete Stability**:
    - Evaluates wet concrete dead load and construction live load prior to concrete curing.
    - Verifies bare steel section flexural resistance $\phi M_n \ge M_{u,\text{const}}$ to prevent lateral-torsional buckling or plastic failure during pouring.
  - **Lower-Bound Effective Moment of Inertia ($I_{\text{eff}}$) & Serviceability Deflections**:
    $$I_{\text{eff}} = I_s + \sqrt{\frac{\sum Q_n}{C_{\text{max}}}} \left(I_{\text{tr}} - I_s\right)$$
    - Live load deflection check: $\Delta_{\text{LL}} \le L/360$.
    - Long-term creep deflection check (accounting for sustained slab dead load): $\Delta_{\text{total}} \le L/240$.
- **Interactive AISC Composite Beam Studio in Analyze View (`src/views/analyze.rs`)**:
  - Added new sub-tab `AnalyzeSubTab::CompositeDesign` (`🥞 AISC Composite`).
  - 4 Executive Metric Cards: Design Capacity ($\phi M_n$), DCR Flexure ratio, Composite Action percentage, and Live Load Deflection ($\Delta_{\text{LL}}$ vs. $L/360$).
  - Control Toolbar: Interactive configuration for span length $L$, beam spacing $S$, wide-flange steel dimensions ($d, b_f, t_f, t_w, F_y$), concrete slab thickness $t_c$, deck rib height $h_r$, $f'_c$, shear stud diameter/quantity, unshored construction toggle, and applied moment/live loads.
  - Dual Vector Visualizer:
    1. *Left*: 2D CAD cross-section detailing showing hatched concrete slab, trapezoidal metal deck ribs, headed shear studs, W-beam profile, and colored dashed line tracking the exact PNA elevation.
    2. *Right*: Plastic Stress Distribution block diagram and Deflection Gauge comparing $\Delta_{\text{LL}}$ against $L/360$.
  - Comprehensive AISC limit states verification table auditing flexure, shear stud capacity, minimum composite action, unshored construction stage, and serviceability creep deflection.
- **AISC 360-16 Composite Beam Markdown Dossier Export (`generate_markdown_dossier`)**:
  - Added one-click export button `🥞 AISC Composite Dossier (.md)` in Results View generating `AISC360_Composite_Beam_Audit.md`.

---

## [v0.0.7] - 2026-11-06 — Phase 7: Wind Engineering & Computational Aero-Dynamics per ASCE 7-22 / SNI 1727:2020

### Added
- **ASCE 7-22 & SNI 1727:2020 Computational Wind Engineering Engine (`src/core/wind.rs`)**:
  - **Exposure Categories & Terrain Profiles**: Implemented Exposure B (urban/suburban), Exposure C (open terrain), and Exposure D (unobstructed coastal) with terrain power-law constants ($\alpha$, $z_g$, $c_{\text{turb}}$) per Table 26.11-1.
  - **Velocity Pressure Exposure Coefficient ($K_z$) & Mean Roof Pressure ($q_h$)**:
    $$K_z = 2.01 \left( \frac{\max(z, 4.5)}{z_g} \right)^{2/\alpha}, \quad q_z = 0.613 K_z K_{zt} K_d K_e V^2 \; (\text{Pa})$$
    Formulated velocity pressure profiles factoring ground elevation $K_e$, wind directionality $K_d = 0.85$, and topographic factor $K_{zt}$.
  - **Dynamic Gust Effect Factor ($G_f$) for Flexible Structures**:
    - Formulated background factor $Q$ and resonant factor $R$ accounting for turbulent integral length scale $L_{\bar{z}}$, damping $\beta$, and aerodynamic admittance $R_h, R_b$.
    - Unified rigid ($G = 0.85$ for $f_1 \ge 1\text{ Hz}$) and flexible structure formulation ($f_1 < 1\text{ Hz}$) per ASCE 7-22 Section 26.11.
  - **Directional MWFRS Design Lateral Pressures & Story Shear Envelopes**:
    - Windward pressure: $p_{\text{windward}} = q_z G C_p$ ($C_p = 0.80$).
    - Leeward pressure: $p_{\text{leeward}} = q_h G C_p$ ($C_p = -0.50$ to $-0.30$ based on $L/B$).
    - Internal pressure variations with $GC_{pi} = \pm 0.18$ (Enclosed) and $\pm 0.55$ (Partially Enclosed).
    - Multi-story tributary shear aggregation $V_w(z) = \sum p_{\text{net}} A_{\text{trib}}$ and base overturning moment $M_{\text{OTM}} = \sum F_i z_i$.
  - **Dynamic Occupant Comfort & Vortex Shedding Acceleration (ISO 10137 / ASCE 7-22 CC)**:
    - Peak along-wind dynamic acceleration: $a_{\text{along}} = g \cdot \frac{3 \sigma_x \cdot (2 \ln(3600 f_1))^{1/2}}{M_{\text{gen}}}$ (milli-g).
    - Cross-wind vortex shedding acceleration incorporating reduced velocity $V_r = \frac{V}{f_1 B}$.
    - Resultant acceleration vs. standard occupant comfort limit ($20\text{ mg}$ for 10-year MRI wind per ISO 10137).
  - **Lateral Governing Demand Comparison**: Automated comparison of total wind base shear $V_w$ against seismic base shear $V_s$ to identify governing lateral resistance mechanism.
- **Interactive ASCE 7 Wind Engineering Dashboard in Analyze View (`src/views/analyze.rs`)**:
  - Added new sub-tab `AnalyzeSubTab::WindEngineering` (`🌪️ ASCE 7 Wind`).
  - 4 Executive Metric Cards: Basic Wind Speed ($V$), Mean Roof Pressure ($q_h$), Wind Base Shear ($V_w$ and $M_{\text{OTM}}$), and Occupant Comfort Acceleration ($a_{\text{res}}$ vs. ISO 10137 limit).
  - Control Toolbar: Real-time adjustments for wind speed (m/s), exposure category, building enclosure, building dimensions ($H \times B \times L$), number of stories, and fundamental period $T_1$.
  - Dual Vector Visualizer:
    1. *Left*: Elevation Velocity Pressure Profile $q_z(z)$ with CAD building silhouette and windward pressure arrows scaled by height.
    2. *Right*: Occupant Acceleration Bar Chart (Along-wind, Cross-wind, Resultant vs ISO 10137 limit) and Wind ($V_w$) vs. Seismic ($V_s$) base shear envelope comparison.
  - Story-by-story wind force schedule table displaying $K_z$, $q_z$, windward/leeward pressures, tributary story forces, and cumulative shear $V_x(z)$.
- **Official Wind Engineering Dossier Export (`generate_markdown_dossier`)**:
  - Added one-click export button `🌪️ ASCE 7 Wind Dossier (.md)` in Results View generating `ASCE7_Wind_Engineering_Dossier.md`.

---

## [v0.0.6] - 2026-11-06 — Phase 6: Steel Connection Design & AISC 360-16 / SNI 1729:2020 Limit States Engine

### Added
- **AISC 360-16 & SNI 1729:2020 Steel Connection Limit States Engine (`src/core/connection.rs`)**:
  - **Bolt Group Shear Rupture**: Formulated nominal bolt shear capacities ($F_{nv} A_b$) for ASTM A325-N/X, A490-N/X, and ISO Class 8.8 / 10.9 fasteners per AISC 360-16 Table J3.2.
  - **Bolt Bearing and Tearout**: Evaluated clear distance $l_c$ across edge and interior pitch spaces, calculating governing bearing resistance $R_n = \min(1.2 l_c t F_u, 2.4 d_b t F_u)$ per Section J3.10.
  - **Block Shear Rupture (AISC 360 Section J4.3 / SNI 1729 Section J4.3)**:
    $$R_n = \min \left( 0.60 F_u A_{nv} + U_{bs} F_u A_{nt}, \; 0.60 F_y A_{gv} + U_{bs} F_u A_{nt} \right) \quad (\phi = 0.75, \; U_{bs} = 1.0)$$
  - **Plate Gross Shear Yielding & Net Shear Rupture**: Sized connection plates for shear yield ($0.60 F_y A_g$) and net shear fracture ($0.60 F_u A_{nv}$).
  - **Double Fillet Weld Shear Resistance**: Formulated E70XX/E80XX weld metal shear capacity across effective throat $t_e = 0.707 w$ ($R_n = 0.60 F_{EXX} A_{we}$).
- **AISC 358-16 / SNI 7860:2020 4E Extended End-Plate Moment Connection (`evaluate_extended_end_plate`)**:
  - Implemented yield-line mechanism parameter $Y_p$ for 4E unstiffened extended end-plates:
    $$Y_p = \frac{b_p}{2} \left[ h_1 \left(\frac{1}{p_{fi}} + \frac{1}{s}\right) + h_0 \left(\frac{1}{p_{fo}}\right) \right] + \frac{2}{g} \left[ h_1 (p_{fi} + s) + h_0 p_{fo} \right]$$
  - Flexural design moment capacity: $\phi M_n = \phi_b F_{yp} t_p^2 Y_p$ ($\phi_b = 0.90$).
  - Evaluated bolt tension demands in tension flange group and gravity shear in compression zone bolts.
- **Column Panel Zone Shear & Web Stiffener Audit (AISC 360 Section J10)**:
  - Formulated column panel zone shear strength:
    $$R_n = 0.60 F_y d_c t_w \left[ 1 + \frac{3 b_{cf} t_{cf}^2}{d_b d_c t_w} \right] \quad (\phi = 0.90)$$
  - Automated calculation of required column web doubler plate thickness ($t_{\text{doubler}} = \frac{V_{pz} - \phi R_n}{0.60 \phi F_y d_c}$).
  - Evaluated Column Web Local Yielding ($\phi R_n = 1.0 F_{yw} t_w (5k + l_b)$) and Web Local Crippling, automatically assessing transverse continuity plate requirements.
- **Interactive Steel Connection CAD Blueprint & DCR Dashboard in Analyze View (`src/views/analyze.rs`)**:
  - Added new sub-tab `AnalyzeSubTab::SteelConnection` (`🔩 Steel Connections`).
  - Integrated 4 executive summary cards: Governing DCR, Critical Limit State, Fastener Configuration, and Panel Zone Audit.
  - Controls toolbar for connection typology, bolt grades, bolt diameter, plate thickness, and factored forces ($V_u, M_u$).
  - Dual Vector Visualizer:
    1. *Left*: 2D Elevation Blueprint showing column profile, beam flanges/web, end-plate/shear tab, tension/compression bolts, and weld beads.
    2. *Right*: Limit States DCR Utilization horizontal bar chart with color-coded utilization thresholds.
  - Detailed Limit States Verification Table displaying nominal capacity, $\phi$, design capacity, and safety status.
- **Official Steel Connection Dossier Export (`generate_markdown_dossier`)**:
  - Added one-click export button `🔩 AISC 360 Connection Dossier (.md)` in Results View generating `AISC360_Steel_Connection_Audit.md`.

---

## [v0.0.5] - 2026-11-06 — Phase 5: Geotechnical Engineering, Deep Foundations & Soil-Structure Interaction (SSI)

### Added
- **Winkler Subgrade Reaction & 6-DOF Foundation Spring Matrix (`WinklerSpringMatrix`, `SoilLayer`, `SoilType`)**:
  - Implemented the Bowles / Vesic elastic subgrade reaction formulation in `src/core/geotech.rs` for spread and mat foundations.
  - Formulated equivalent 6-DOF foundation stiffness matrix:
    - Translational vertical stiffness: $K_z = k_s \cdot B \cdot L$ (kN/m).
    - Translational horizontal shear stiffness: $K_x = K_y = 0.82 \cdot K_z$ (kN/m).
    - Rocking rotational stiffness about primary axes: $K_{\theta x} = k_s \cdot \frac{B \cdot L^3}{12}$, $K_{\theta y} = k_s \cdot \frac{L \cdot B^3}{12}$ (kNm/rad).
    - Torsional rotational stiffness: $K_{\theta z} = 0.65 \cdot k_s \cdot (I_x + I_y)$ (kNm/rad).
  - Built stratigraphy models for 7 standard soil layers: Dense Sand, Medium Dense Sand, Loose Sand, Stiff Cohesive Clay, Medium Stiff Clay, Soft Clay, and Competent Bedrock ($\gamma, c_u, \phi, k_s, E_s, \nu$).
- **Deep Foundation Pile Group Converse-Labarre Efficiency & Biaxial Distribution (`evaluate_pile_group`, `PileElement`, `PileGroupReport`)**:
  - Implemented the Converse-Labarre pile group efficiency formulation:
    $$E_g = 1 - \theta \cdot \frac{(n_x - 1)n_y + (n_y - 1)n_x}{90 \cdot n_x \cdot n_y}, \quad \text{where } \theta = \arctan(d / s)$$
  - Formulated single pile ultimate compressive and uplift capacities ($q_{\text{end}} + q_{\text{skin}}$) per Meyerhof / Tomlinson standards.
  - Evaluated rigorous 2D grid biaxial eccentric load distribution:
    $$P_i = \frac{P_u}{N} \pm \frac{M_{uy} \cdot x_i}{\sum x^2} \pm \frac{M_{ux} \cdot y_i}{\sum y^2}$$
  - Computed lateral subgrade resistance and lateral pile head deflection $\delta_{\text{lat}}$ under tributary horizontal shear.
- **Two-Way Concrete Punching Shear Audit per SNI 2847:2019 / ACI 318-19 Section 22.6 (`audit_punching_shear`, `PunchingShearAudit`)**:
  - Established critical shear perimeter $b_o = 2(c_1 + d) + 2(c_2 + d)$ at $d/2$ from column perimeter.
  - Evaluated governing nominal concrete shear capacity per SNI 2847:2019 Eq. 22.6.5.2:
    $$V_c = \min \left\{ 0.33 \sqrt{f'_c}, \; 0.17 \left(1 + \frac{2}{\beta}\right) \sqrt{f'_c}, \; 0.083 \left(2 + \frac{\alpha_s d}{b_o}\right) \sqrt{f'_c} \right\} \cdot b_o d$$
  - Factored punching shear stress with direct shear and unbalanced eccentric moment transfer:
    $$\nu_u = \frac{V_u}{b_o d} + \frac{\gamma_v M_{unb} c}{J_c}$$
  - Automated safety verdict, demand-capacity ratio ($DCR_{\text{punch}}$), and prescriptive shear stud reinforcement advisories.
- **Veletsos & Meek Dynamic Soil-Structure Interaction (SSI) Modal Period Lengthening (`evaluate_ssi_modal_shift`, `SsiModalShiftReport`)**:
  - Formulated Veletsos & Meek dynamic SSI period lengthening:
    $$\left(\frac{\tilde{T}}{T}\right)^2 = 1 + \frac{K_{\text{fixed}}}{K_h} + \frac{K_{\text{fixed}} h^2}{K_r}$$
  - Evaluated ASCE 7-22 Chapter 19 foundation radiation and hysteretic damping $\beta_{\text{ssi}}$.
  - Derived design base shear reduction allowance $(1 / (\tilde{T}/T)^{0.85})$ and lateral drift amplification.
- **Interactive Geotechnical & SSI Dashboard in Analyze View (`src/views/analyze.rs`)**:
  - Added new sub-tab `AnalyzeSubTab::GeotechSsi` (`🌱 Geotech & SSI`).
  - Integrated 4 executive metric cards: `SUBGRADE MODULUS (ks)`, `CONVERSE-LABARRE (Eg)`, `PUNCHING SHEAR (SNI 22.6)`, and `SSI DYNAMIC SHIFT (T̃/T)`.
  - Added interactive dual visualizer:
    1. 2D Pile Group Plan Layout with cap geometry, column base, and individual pile axial load status colors.
    2. Critical Punching Shear Perimeter $b_o$ with column cross-section, $d/2$ reticles, and stress callouts.
  - Added 6-DOF Winkler Spring Matrix summary and individual pile biaxial load/deflection schedule table.
- **Exportable Geotechnical & SSI Calculation Dossier (`generate_markdown_dossier`)**:
  - Produces publication-ready Markdown calculation sheets with LaTeX equations, spring matrices, pile schedules, and SNI 2847 verification tables.
  - Added one-click export button `🌱 Geotech & SSI Dossier (.md)` in Results View (`src/views/results.rs`).

---

## [v0.0.4] - 2026-11-06 — Phase 4: Advanced Seismic & Pushover Analysis (ASCE 41-17 / ATC-40)

### Added
- **Capacity Spectrum Method Engine (`PushoverEngine`, `CapacitySpectrum`, `InelasticDemandSpectrum`)**:
  - Implemented the complete ATC-40 / ASCE 41-17 Capacity Spectrum Method in `src/core/pushover.rs`.
  - Formulated conversion of Base Shear vs Roof Displacement $(V_b, D_{\text{roof}})$ into Acceleration-Displacement Response Spectrum ($ADRS$) format $(S_a, S_d)$ using modal participation factor $\Gamma_1$ and modal mass coefficient $\alpha_1$:
    $$S_d = \frac{D_{\text{roof}}}{\Gamma_1 \cdot \phi_{\text{roof}, 1}}, \quad S_a = \frac{V_b / W}{\alpha_1}$$
  - Synthesized 5% damped elastic design response spectrum per SNI 1726:2019 / ASCE 7-22 ($S_s, S_1, F_a, F_v, S_{DS}, S_{D1}, T_0, T_s$).
  - Derived equivalent hysteretic viscous damping $\beta_{\text{eff}} = \beta_0 + \kappa \cdot \frac{2}{\pi} \frac{A_y D_p - D_y A_p}{A_p D_p}$ and spectral damping reduction factors $SR_A$ and $SR_V$.
  - Implemented ATC-40 Procedure A iterative solver converging to the **Performance Point** $(S_d^*, S_a^*)$, target displacement $D_t$, and target base shear $V_t$.
- **Performance-Based Earthquake Engineering (PBEE) Classification (`SeismicPerformanceLevel`)**:
  - Automated structural audit against ASCE 41-17 Table 9-6 / FEMA 356 performance levels:
    - `ImmediateOccupancy` (IO): Minor micro-yielding, no residual drift, structure safe for immediate post-earthquake occupancy.
    - `LifeSafety` (LS): Controlled inelastic plastification, wide collapse margin, main structural framing stable under Design Basis Earthquake (DBE).
    - `CollapsePrevention` (CP): Severe structural damage, minimal residual margin, prevents global collapse under Maximum Considered Earthquake (MCE).
    - `CollapseExceeded`: Target displacement exceeds ultimate capacity; structure requires retrofitting or shear wall stiffening.
  - Comprehensive plastic hinge distribution tracking at the Performance Point across Elastic, IO, LS, CP, and Mechanism states.
- **Configurable Lateral Load Patterns (`LateralLoadPattern`)**:
  - Added support for `FirstModeModal` (proportional to fundamental mode shape), `UniformMass` (proportional to floor mass acceleration), and `InvertedTriangular` (ASCE 7 $k$-exponent height distribution).
- **Interactive Dual Vector Visualizer in Analyze View (`src/views/analyze.rs`)**:
  - Integrated side-by-side interactive vector plots:
    1. Base Shear vs Roof Drift capacity curve with target displacement $D_t$ reticle indicator.
    2. ADRS Capacity-Demand Spectrum showing Elastic Demand (red dashed), Inelastic Reduced Demand (amber), Capacity Spectrum (emerald green), and intersecting Performance Point $P^*(S_d^*, S_a^*)$.
  - Added live toolbar controls for Lateral Load Pattern, Soil Site Class (SB, SC, SD, SE), and spectral parameters $S_s, S_1$.
- **Official Exportable ASCE 41 Pushover Calculation Dossier**:
  - Added `generate_markdown_dossier` producing publication-ready Markdown calculation sheets with LaTeX equations, performance metrics, and hinge distribution matrices.
  - Added one-click export button `📉 ASCE 41 Pushover Dossier (.md)` in Results View (`src/views/results.rs`).

---

## [v0.0.3] - 2026-11-06 — Phase 3: Core Non-Linear & Geometry Upgrade (Shell FEA, P-Delta & IFC4 SAM)

### Added
- **2D Mindlin-Reissner Flat Shell / Plate Finite Elements (`ShellElement4N`)**:
  - Implemented 4-node isoparametric quadrilateral flat shell elements in `src/core/model.rs` for reinforced concrete shear walls (*dinding geser*) and floor diaphragms.
  - Formulated combined membrane stiffness ($u, v$ in-plane behavior) and Mindlin-Reissner plate bending ($w, \theta_x, \theta_y$ with transverse shear deformation) plus drilling degree of freedom $\theta_z$, providing 6 degrees of freedom per node (24 DOFs per quad element).
  - Evaluated stiffness and internal stresses using $2 \times 2$ Gauss-Legendre numerical quadrature.
  - Added geometric attribute helpers for shear wall classification (`as_shear_wall`) and exact shell surface area integration.
  - Integrated `pub shells: Vec<ShellElement4N>` into `StructuralModel`.
- **Second-Order Geometric Non-Linearity $P-\Delta$ & $P-\delta$ Solver (`SecondOrderStabilitySolver`)**:
  - Implemented geometric stiffness matrix $\mathbf{K}_g(\mathbf{P})$ in `src/core/solver.rs` accounting for axial load softening/stiffening effects.
  - Formulated tangent stiffness matrix $\mathbf{K}_t = \mathbf{K}_e - \mathbf{K}_g(\mathbf{P})$.
  - Evaluated Euler elastic critical buckling load $P_{\text{cr}} = \frac{\pi^2 E I}{(K L)^2}$ for slender compression columns.
  - Evaluated AISC 360-16 / 360-22 $B_1$ curvature amplification factor:
    $$B_1 = \frac{C_m}{1 - P_u / P_{e1}} \ge 1.0$$
  - Evaluated ASCE 7-22 Section 12.8.7 story drift stability coefficient:
    $$\theta = \frac{P_x \Delta I_e}{V_x h_{sx} C_d} \le \theta_{\max} = \frac{0.50}{\beta C_d} \le 0.25$$
  - Added automated stability classification (`StableLinear`, `PDeltaAmplified`, `CriticalBucklingRisk`, `DynamicPDeltaInstability`).
- **OpenBIM Industry Foundation Classes (IFC4 / IFC4.3) Structural Analysis Model Exporter (`IfcSamExporter`)**:
  - Implemented ISO 16739-1 compliant `.ifc` step-physical text exporter in `src/core/interop.rs`.
  - Serializes `IFCSTRUCTURALANALYSISMODEL`, `IFCSTRUCTURALCURVEMEMBER` (beams and columns with coordinate axes), `IFCSTRUCTURALSURFACEMEMBER` (4-node shear walls and slabs), `IFCSTRUCTURALPOINTCONNECTION` (nodes and boundary conditions), and `IFCMATERIAL` (structural concrete and structural steel).
  - Validated compatibility with Autodesk Revit Structure, Tekla Structures, ArchiCAD, and BlenderBIM.
- **UI Integration in Results View (`src/views/results.rs`)**:
  - Added export trigger button `🏛️ IFC4 SAM (.ifc)` alongside ETABS and SIMBG exports.

---

## [v0.0.2] - 2026-11-06 — Phase 2: Quick Wins (ETABS Interop, PBG/SIMBG & Rebar BBS)

### Added
- **CSI ETABS `.e2k` Native Importer & Exporter (`EtabsE2kExporter`, `EtabsE2kImporter`)**:
  - Implemented complete bidirectional exchange in `src/core/interop.rs` for CSI ETABS text definition files (`.e2k` / `.$et`).
  - Added parser and serializer blocks for `PROGRAM INFORMATION`, `STORIES`, `GRIDS`, `POINT COORDINATES`, `FRAME SECTIONS`, `LINE CONNECTIVITY` (`LINE OBJECTS`), and point/line structural loads.
  - Enables zero-loss structural migration between CSI ETABS v18–v21 and EigenGraph.
- **Indonesian Municipal PBG/SIMBG Technical Calculation Dossier Generator (`PbgReportGenerator`, `PbgProjectMetadata`)**:
  - Implemented automated structural calculation report generator in `src/core/interop.rs` tailored for the Indonesian Ministry of Public Works and Public Housing (PUPR) SIMBG portal under Government Regulation PP No. 16 Tahun 2021.
  - Encoded regulatory compliance audits:
    - **SNI 1726:2019** (Earthquake Resistant Building Design): Inelastic drift limit $\Delta_a = 0.020 h_{sx} / \rho$, P-Delta stability coefficients, modal mass participation ratio $\ge 90\%$.
    - **SNI 2847:2019** (Structural Concrete Requirements): Strength reduction factors ($\phi = 0.90$ flexure, $\phi = 0.75$ shear/torsion, $\phi = 0.65$ tied axial compression), and Strong-Column Weak-Beam flexural capacity ratio $\sum M_{nc} / \sum M_{nb} \ge 1.20$.
    - **SNI 1727:2020** (Minimum Design Loads for Buildings).
  - Generates comprehensive technical dossiers formatted in publication-ready Markdown/PDF with engineering stamp seals, SKA/SKK Utama verification, and project registration metadata.
- **Automated Bar Bending Schedule (BBS) & Rebar Detailing (`RebarBbsDetailer`, `RebarBbsSchedule`)**:
  - Implemented automated rebar cutting schedule and detailing engine in `src/core/interop.rs` in accordance with SNI 2847:2019 Pasal 25.3 / ACI 318-19 Chapter 25.
  - Standardized deformed rebar grades (BJTS 420B: D10, D13, D16, D19, D22, D25) with exact unit weights per meter ($w = 0.006165 \times d_b^2 \text{ kg/m}$).
  - Automatic calculation of development length ($l_d$), seismic standard hooks ($135^\circ$ with $6d_b \ge 75\text{ mm}$ extension), lap splice lengths ($1.3 l_d$ Class B), and total rebar tonnage including scrap allowance.
  - Produces tabular CSV output for procurement, cost estimation (BoQ), and fabrication shops.
- **UI Action Triggers in Results View (`src/views/results.rs`)**:
  - Added dedicated one-click export buttons:
    - `📄 SIMBG Dossier (.md)`
    - `📊 Bar Schedule (.csv)`
    - `🔄 Export ETABS (.e2k)`

---

## [v0.0.1] - 2026-11-06 — Phase 1: Structural Mechanics & Non-Linear Core Engine

### Added
- **Non-Linear Sherman-Morrison & Woodbury Plastic Redistribution Engine (`src/core/nonlinear.rs`)**:
  - Rank-1 matrix perturbation solver computing exact redistribution vector $\Delta \mathbf{N}$ without full stiffness inversion $\mathcal{O}(N^3) \to \mathcal{O}(N^2)$.
  - Redundancy Reserve Ratio ($RRR(\lambda)$) tracking progressive plastic hinge formation.
- **3D Biaxial P-M-M Concrete Interaction Surface (`src/core/concrete.rs`)**:
  - Bresler reciprocal load method and PCA load contour formulation per SNI 2847:2019 and ACI 318-19.
  - Capacity ratio checks across axial-flexural limit states.
- **CQC and SRSS Modal Dynamic Response Combination (`src/core/modal.rs`)**:
  - Complete Quadratic Combination with Der Kiureghian cross-modal correlation coefficients $\rho_{ij}$ accounting for closely spaced modal frequencies.
  - 85% static base shear minimum scaling per SNI 1726:2019 Pasal 7.9.4.1.
- **High-Performance Linear Equation Solvers**:
  - Sparse Cholesky $\mathbf{L}\mathbf{L}^T$ and Preconditioned Conjugate Gradient (PCG) in Compressed Sparse Row (CSR) format.
- **Vector Blueprint Exporter**:
  - Direct CAD-to-Vector generator exporting architectural and structural sheets to SVG and vector PDF.
