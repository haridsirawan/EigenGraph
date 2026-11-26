//! Dynamic Island exclusive drawers: Slash Palette, Obsidian/IFCX Vault Explorer, and @Context Picker.

use super::*;
use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui, Vec2};

use crate::core::BimProject;
use crate::icons::{self, AppIcon};
use crate::models::{DatabaseComponent, NavigationPillar, ProjectItem, StructuralMember};
use crate::theme::MacTheme;
use crate::views::{AiAssistantView, BuildView, UniversalCanvasState};

impl SpatialWorkspaceState {
    /// Shared `/` Slash Command Autocomplete card used in both Side Panel and Collapsed Command Dock.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_slash_palette_box(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
        pal_w: f32,
        max_h: f32,
        salt: &str,
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();
        let q = self.command_input.trim().to_lowercase();
        let mut queued_cmd: Option<String> = None;

        Frame::new()
            .fill(Color32::TRANSPARENT)
            .inner_margin(Margin::symmetric(2, 4))
            .show(ui, |ui| {
                ui.set_width(pal_w);
                ui.set_max_width(pal_w);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            ui.horizontal(|ui| {
                icons::icon_label(
                    ui,
                    AppIcon::Sparkles,
                    12.5,
                    theme.accent_blue,
                    "Slash Commands",
                    11.5,
                    theme.text_primary,
                    true,
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new("Enter to run • Esc")
                            .size(9.5)
                            .color(theme.text_muted),
                    );
                });
            });
            ui.add_space(4.0);
            ScrollArea::vertical()
                .id_salt(salt)
                .max_height(max_h)
                .show(ui, |ui| {
                    let btn_w = (pal_w - 24.0).max(120.0);
                    ui.set_width(btn_w);
                    ui.set_max_width(btn_w);
                    let needle = q.strip_prefix('/').unwrap_or(&q);
                    for spec in SLASH_COMMANDS {
                        let matches = q == "/"
                            || spec.cmd.contains(&q)
                            || spec.category.to_lowercase().contains(needle)
                            || spec.description.to_lowercase().contains(needle);
                        if !matches {
                            continue;
                        }
                        let resp = ui.add_sized(
                            Vec2::new(btn_w, 24.0),
                            egui::Button::new(
                                RichText::new(format!(
                                    "{}   •   [{}] {}",
                                    spec.cmd, spec.category, spec.description
                                ))
                                .size(10.5)
                                .color(theme.text_primary),
                            )
                            .wrap_mode(egui::TextWrapMode::Wrap)
                            .fill(theme.bg_card.gamma_multiply(0.88))
                            .stroke(Stroke::new(0.8, theme.border_subtle))
                            .corner_radius(CornerRadius::same(7)),
                        );
                        if resp.clicked() {
                            queued_cmd = Some(spec.cmd.to_string());
                        }
                        ui.add_space(2.0);
                    }
                });
        });

        if let Some(cmd) = queued_cmd {
            self.command_input.clear();
            let sub = self.execute_spatial_input(
                &cmd,
                universal_canvas,
                build_view,
                ai_assistant,
                members,
                projects,
                db_components,
            );
            actions.merge(sub);
        }
        actions
    }

    /// Antigravity Workspace / Vault Header Bar with `+ Add Folder (Vault)` & inline Vault Explorer.
    pub(super) fn render_vault_workspace_bar(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        vault_projects: &[crate::core::vault::PersonalVaultProjectEntry],
        universal_canvas: &mut UniversalCanvasState,
        panel_w: f32,
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();
        let status = crate::core::InternalVault::connection_status();
        let vault_files = crate::core::InternalVault::list_vault_context_files(128);

        Frame::new()
            .fill(Color32::TRANSPARENT)
            .inner_margin(Margin::symmetric(2, 4))
            .show(ui, |ui| {
                let inner_w = (panel_w - 10.0).max(160.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                ui.horizontal(|ui| {
                    let badge_col = if status.connected {
                        theme.accent_green
                    } else {
                        theme.accent_blue
                    };
                    let title_txt = if status.connected {
                        format!("Vault: {}", status.vault_name)
                    } else {
                        "Engineering Vault".to_string()
                    };
                    icons::icon_label(
                        ui,
                        AppIcon::Folder,
                        13.5,
                        badge_col,
                        &title_txt,
                        11.5,
                        theme.text_primary,
                        true,
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::icon_only_button(
                            ui,
                            AppIcon::SlidersPanel,
                            theme.text_secondary,
                            Color32::TRANSPARENT,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::splat(20.0),
                            "Vault folder options & IndexedDB/Path setup",
                        )
                        .clicked()
                        {
                            self.vault_path_popover_open = !self.vault_path_popover_open;
                        }

                        if status.connected {
                            if icons::icon_only_button(
                                ui,
                                if self.vault_drawer_open {
                                    AppIcon::ChevronUp
                                } else {
                                    AppIcon::ChevronDown
                                },
                                theme.text_secondary,
                                Color32::TRANSPARENT,
                                Stroke::NONE,
                                CornerRadius::same(6),
                                Vec2::splat(20.0),
                                "Toggle Vault Files Explorer",
                            )
                            .clicked()
                            {
                                self.vault_drawer_open = !self.vault_drawer_open;
                            }
                            if icons::icon_only_button(
                                ui,
                                AppIcon::SaveDisk,
                                theme.accent_green,
                                theme.accent_green.gamma_multiply(0.14),
                                Stroke::NONE,
                                CornerRadius::same(6),
                                Vec2::splat(20.0),
                                "Save current workspace into Vault",
                            )
                            .clicked()
                            {
                                actions.save_active_to_vault = true;
                            }
                        }

                        if icons::icon_button(
                            ui,
                            AppIcon::Plus,
                            if status.connected { "Switch" } else { "Add Folder" },
                            10.0,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(7),
                            Vec2::new(0.0, 21.0),
                        )
                        .on_hover_text("Select a local folder to mount as your Engineering Vault")
                        .clicked()
                        {
                            actions.add_vault_folder = true;
                        }
                    });
                });

                ui.add_space(2.0);
                if status.connected {
                    ui.horizontal_wrapped(|ui| {
                        let backend_lbl = match status.backend {
                            crate::core::vault::VaultBackendKind::WebDirectoryPicker => "Browser FS",
                            crate::core::vault::VaultBackendKind::BrowserIndexedDb => "IndexedDB",
                            crate::core::vault::VaultBackendKind::LocalFilesystem => "Local Folder",
                            crate::core::vault::VaultBackendKind::Unbound => "Unbound",
                        };
                        ui.label(
                            RichText::new(format!(
                                "{backend_lbl} • {} project(s) • {} file(s)",
                                vault_projects.len(),
                                vault_files.len()
                            ))
                            .size(10.0)
                            .color(theme.text_muted),
                        );
                        if ui
                            .small_button("+ @Vault")
                            .on_hover_text("Attach connected Vault workspace overview to AI Context")
                            .clicked()
                        {
                            self.attach_context(AiContextAttachment::VaultWorkspace);
                        }
                    });
                } else {
                    ui.label(
                        RichText::new("Connect a local folder to store .eigen models & attach Vault context.")
                            .size(10.0)
                            .color(theme.text_muted),
                    );
                }

                if let Some(ref fb) = self.vault_feedback {
                    ui.add_space(2.0);
                    ui.label(RichText::new(fb).size(9.8).color(theme.accent_orange));
                }

                if self.vault_path_popover_open {
                    ui.add_space(5.0);
                    ui.separator();
                    ui.add_space(5.0);
                    let outcome = crate::ui_kit::render_obsidian_vault_panel(
                        ui,
                        theme,
                        &mut self.vault_inline_path,
                        &mut self.vault_inline_name,
                        true,
                        false,
                        false,
                    );
                    if let Some(fb) = outcome.feedback {
                        self.vault_feedback = Some(fb);
                    }
                    if outcome.state_changed {
                        actions.refresh_vault = true;
                    }
                }

                if status.connected && self.vault_drawer_open {
                    ui.add_space(5.0);
                    ui.separator();
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Vault Files")
                                .size(10.0)
                                .strong()
                                .color(theme.text_secondary),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.small_button("Full Vault").clicked() {
                                self.open_module_overlay(NavigationPillar::MyLibrary);
                            }
                            if ui.small_button("Refresh").clicked() {
                                actions.refresh_vault = true;
                            }
                            if ui
                                .small_button("Audit .md")
                                .on_hover_text("Generate structural & Earth audit Markdown note into Vault")
                                .clicked()
                            {
                                actions.save_audit_note = true;
                            }
                            if ui
                                .small_button("IFCX")
                                .on_hover_text("Export active BIM model to pure IFCX (IFC 5 JSON ECS)")
                                .clicked()
                            {
                                actions.export_ifcx = true;
                            }
                            if ui
                                .small_button(if self.vault_note_composer_open {
                                    "Close Note"
                                } else {
                                    "+ Note (.md)"
                                })
                                .on_hover_text("Create a new Obsidian Markdown engineering note inside selected folder")
                                .clicked()
                            {
                                self.vault_note_composer_open = !self.vault_note_composer_open;
                            }
                        });
                    });
                    ui.add_space(3.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.vault_filter_query)
                                .desired_width((inner_w * 0.44).max(95.0))
                                .hint_text("Filter vault files..."),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.vault_new_folder_name)
                                .desired_width((inner_w * 0.34).max(80.0))
                                .hint_text("New subfolder..."),
                        );
                        if ui
                            .small_button("+ Folder")
                            .on_hover_text("Create organized subfolder inside active Vault")
                            .clicked()
                            && !self.vault_new_folder_name.trim().is_empty()
                        {
                            match crate::core::InternalVault::create_vault_subfolder(
                                &self.vault_new_folder_name,
                            ) {
                                Ok(rel) => {
                                    self.vault_feedback = Some(format!("Created subfolder note '{rel}'."));
                                    self.vault_new_folder_name.clear();
                                    actions.refresh_vault = true;
                                    self.push_trace(format!("Created folder -> {rel}"), true);
                                }
                                Err(err) => {
                                    self.vault_feedback = Some(err);
                                }
                            }
                        }
                    });
                    if self.vault_note_composer_open {
                        ui.add_space(4.0);
                        Frame::new()
                            .fill(theme.bg_sidebar)
                            .corner_radius(CornerRadius::same(8))
                            .stroke(Stroke::new(0.8, theme.accent_blue.gamma_multiply(0.45)))
                            .inner_margin(Margin::same(6))
                            .show(ui, |ui| {
                                ui.set_width((inner_w - 16.0).max(140.0));
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new("Folder:").size(9.5).color(theme.text_muted));
                                    for folder_opt in crate::core::InternalVault::list_vault_subfolders() {
                                        let sel_f = self.vault_note_folder == folder_opt;
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new(&folder_opt).size(9.2).color(if sel_f {
                                                        Color32::WHITE
                                                    } else {
                                                        theme.text_secondary
                                                    }),
                                                )
                                                .min_size(Vec2::new(0.0, 19.0))
                                                .fill(if sel_f {
                                                    theme.accent_blue
                                                } else {
                                                    theme.bg_card
                                                })
                                                .corner_radius(CornerRadius::same(6)),
                                            )
                                            .clicked()
                                        {
                                            self.vault_note_folder = folder_opt;
                                        }
                                    }
                                });
                                ui.add_space(3.0);
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.vault_note_title_input)
                                        .hint_text("Note title (e.g. Seismic_Design_Criteria)")
                                        .desired_width(ui.available_width()),
                                );
                                ui.add_space(3.0);
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.vault_note_body_input)
                                        .desired_rows(2)
                                        .hint_text("# Notes...\n- Concrete fc' = 35 MPa...")
                                        .desired_width(ui.available_width()),
                                );
                                ui.add_space(3.0);
                                if icons::icon_button(
                                    ui,
                                    AppIcon::SaveDisk,
                                    format!("Save to {}/*.md", self.vault_note_folder),
                                    9.8,
                                    Color32::WHITE,
                                    theme.accent_green,
                                    Stroke::NONE,
                                    CornerRadius::same(6),
                                    Vec2::new(0.0, 20.0),
                                )
                                .clicked()
                                {
                                    let title = if self.vault_note_title_input.trim().is_empty() {
                                        "Design_Note"
                                    } else {
                                        self.vault_note_title_input.trim()
                                    };
                                    match crate::core::InternalVault::save_markdown_note_in_folder(
                                        &self.vault_note_folder,
                                        title,
                                        &self.vault_note_body_input,
                                    ) {
                                        Ok(rel) => {
                                            self.vault_feedback = Some(format!("Saved note '{rel}'."));
                                            self.vault_note_composer_open = false;
                                            self.vault_note_title_input.clear();
                                            self.vault_note_body_input.clear();
                                            actions.refresh_vault = true;
                                            self.push_trace(format!("Saved {rel}"), true);
                                        }
                                        Err(err) => {
                                            self.vault_feedback = Some(err);
                                        }
                                    }
                                }
                            });
                    }
                    if let Some((note_path, mut note_body)) = self.previewed_vault_note.clone() {
                        ui.add_space(4.0);
                        Frame::new()
                            .fill(theme.bg_sidebar)
                            .corner_radius(CornerRadius::same(8))
                            .stroke(Stroke::new(0.8, theme.border_subtle))
                            .inner_margin(Margin::same(6))
                            .show(ui, |ui| {
                                ui.set_width((inner_w - 16.0).max(140.0));
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(&note_path)
                                            .size(9.8)
                                            .strong()
                                            .color(theme.accent_blue),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui.small_button("Close").clicked() {
                                            self.previewed_vault_note = None;
                                            self.preview_note_editing = false;
                                        }
                                        if note_path.ends_with(".md")
                                            && ui
                                                .small_button(if self.preview_note_editing {
                                                    "Save Note"
                                                } else {
                                                    "Edit"
                                                })
                                                .clicked()
                                        {
                                            if self.preview_note_editing {
                                                let stem = note_path
                                                    .rsplit('/')
                                                    .next()
                                                    .unwrap_or(&note_path);
                                                if crate::core::InternalVault::save_markdown_note(
                                                    stem,
                                                    &note_body,
                                                )
                                                .is_ok()
                                                {
                                                    self.vault_feedback = Some(format!(
                                                        "Updated Obsidian note '{note_path}'."
                                                    ));
                                                    actions.refresh_vault = true;
                                                }
                                                self.preview_note_editing = false;
                                            } else {
                                                self.preview_note_editing = true;
                                            }
                                        }
                                    });
                                });
                                if self.preview_note_editing {
                                    ui.add_space(2.0);
                                    if ui.small_button("+ Insert Vault Context").clicked() {
                                        note_body.push_str("\n\n- Backlink: [[Engineering Design Reference]]\n");
                                    }
                                    ui.add(
                                        egui::TextEdit::multiline(&mut note_body)
                                            .desired_rows(3)
                                            .desired_width(ui.available_width()),
                                    );
                                    self.previewed_vault_note = Some((note_path, note_body));
                                } else {
                                    ui.label(
                                        RichText::new(&note_body)
                                            .size(9.6)
                                            .color(theme.text_secondary),
                                    );
                                }
                            });
                    }
                    if let Some((old_rel, mut rename_draft)) = self.vault_rename_target.clone() {
                        ui.add_space(3.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Rename:").size(9.5).color(theme.text_muted));
                            ui.add(
                                egui::TextEdit::singleline(&mut rename_draft)
                                    .desired_width((inner_w * 0.5).max(110.0)),
                            );
                            if ui.small_button("Apply").clicked() {
                                match crate::core::InternalVault::rename_vault_file(
                                    &old_rel,
                                    &rename_draft,
                                ) {
                                    Ok(new_rel) => {
                                        self.vault_feedback = Some(format!("Renamed -> {new_rel}"));
                                        self.vault_rename_target = None;
                                        actions.refresh_vault = true;
                                    }
                                    Err(err) => {
                                        self.vault_feedback = Some(err);
                                    }
                                }
                            } else if ui.small_button("Cancel").clicked() {
                                self.vault_rename_target = None;
                            } else {
                                self.vault_rename_target = Some((old_rel, rename_draft));
                            }
                        });
                    }
                    let fq = self.vault_filter_query.trim().to_lowercase();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Show:").size(9.5).color(theme.text_muted));
                        for pf in [
                            crate::core::gis::WorldPortfolioFilter::All,
                            crate::core::gis::WorldPortfolioFilter::UnlocatedStaging,
                            crate::core::gis::WorldPortfolioFilter::CriticalRisk,
                            crate::core::gis::WorldPortfolioFilter::VerifiedOrPinned,
                        ] {
                            let active = self.world_portfolio_filter == pf;
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(pf.short_label())
                                            .size(9.2)
                                            .color(if active { Color32::WHITE } else { theme.text_secondary }),
                                    )
                                    .min_size(Vec2::new(0.0, 18.0))
                                    .fill(if active { theme.accent_blue } else { theme.bg_card })
                                    .corner_radius(CornerRadius::same(6)),
                                )
                                .clicked()
                            {
                                self.world_portfolio_filter = pf;
                            }
                        }
                    });
                    if universal_canvas.world_index.unlocated_count() > 0 {
                        ui.add_space(2.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Batch Pin:").size(9.5).color(theme.text_muted));
                            ui.add(
                                egui::TextEdit::singleline(&mut self.batch_pin_query)
                                    .hint_text("Folder/name")
                                    .desired_width(82.0),
                            );
                            let cities = crate::core::gis::GLOBAL_CITY_CATALOG;
                            let sel_idx = self.batch_pin_city_idx.min(cities.len().saturating_sub(1));
                            let (sel_city, sel_lat, sel_lon, _) = cities[sel_idx];
                            egui::ComboBox::from_id_salt("vault_batch_city_combo")
                                .width(102.0)
                                .selected_text(sel_city)
                                .show_ui(ui, |ui| {
                                    for (i, &(cname, _, _, _)) in cities.iter().enumerate() {
                                        ui.selectable_value(&mut self.batch_pin_city_idx, i, cname);
                                    }
                                });
                            if ui
                                .small_button("Assign City")
                                .on_hover_text("Relocate all Staging models matching keyword onto the selected city grid")
                                .clicked()
                            {
                                let q = if self.batch_pin_query.trim().is_empty() {
                                    self.vault_filter_query.trim().to_string()
                                } else {
                                    self.batch_pin_query.trim().to_string()
                                };
                                let q_eff = if q.is_empty() { sel_city.to_string() } else { q };
                                let moved = universal_canvas
                                    .world_index
                                    .relocate_city_batch(&q_eff, sel_lat, sel_lon, sel_city);
                                if moved > 0 {
                                    crate::core::InternalVault::save_world_spatial_index(&universal_canvas.world_index);
                                    self.vault_feedback = Some(format!(
                                        "Batch-assigned {moved} Staging model(s) matching '{q_eff}' to {sel_city}."
                                    ));
                                    universal_canvas.earth_world.start_fly_to(
                                        &universal_canvas.world_map,
                                        sel_lat,
                                        sel_lon,
                                        14.5,
                                        sel_city,
                                    );
                                } else {
                                    self.vault_feedback = Some(format!(
                                        "No Staging models matched '{q_eff}'."
                                    ));
                                }
                            }
                        });
                    }
                    ui.add_space(2.0);
                    ScrollArea::vertical()
                        .id_salt("antigravity_vault_explorer_scroll")
                        .max_height(132.0)
                        .show(ui, |ui| {
                            ui.set_max_width((inner_w - 10.0).max(140.0));
                            if vault_projects.is_empty() && vault_files.is_empty() {
                                ui.label(
                                    RichText::new("Empty Vault. Click Save to store the active project.")
                                        .size(10.0)
                                        .color(theme.text_muted),
                                );
                            }
                            let portfolio_filter = self.world_portfolio_filter;
                            for proj in vault_projects
                                .iter()
                                .filter(|proj| {
                                    let proxy_opt = universal_canvas.world_index.get(&proj.rel_path);
                                    let matches_pf = match portfolio_filter {
                                        crate::core::gis::WorldPortfolioFilter::All => true,
                                        crate::core::gis::WorldPortfolioFilter::UnlocatedStaging => {
                                            proxy_opt.map_or(true, |b| b.is_unlocated())
                                        }
                                        crate::core::gis::WorldPortfolioFilter::CriticalRisk => {
                                            proxy_opt.is_some_and(|b| b.is_critical())
                                        }
                                        crate::core::gis::WorldPortfolioFilter::VerifiedOrPinned => {
                                            proxy_opt.is_some_and(|b| !b.is_unlocated())
                                        }
                                    };
                                    let matches_q = fq.is_empty()
                                        || proj.name.to_lowercase().contains(&fq)
                                        || proj.rel_path.to_lowercase().contains(&fq)
                                        || proxy_opt.is_some_and(|b| b.address_hint.to_lowercase().contains(&fq));
                                    matches_pf && matches_q
                                })
                                .take(80)
                            {
                                let proxy_opt = universal_canvas.world_index.get(&proj.rel_path).cloned();
                                let geo_tag = proxy_opt
                                    .as_ref()
                                    .map(|b| match b.source {
                                        crate::core::gis::GeoPlacementSource::FileNativeGeo => "GPS",
                                        crate::core::gis::GeoPlacementSource::AddressOrFolderHint => "City",
                                        crate::core::gis::GeoPlacementSource::UserPinnedOnEarth => "Pin",
                                        crate::core::gis::GeoPlacementSource::StagingCampusAuto => "Stage",
                                    })
                                    .unwrap_or("Stage");
                                ui.horizontal(|ui| {
                                    if ui
                                        .small_button("@")
                                        .on_hover_text("Add project to AI Context")
                                        .clicked()
                                    {
                                        self.attach_context(AiContextAttachment::VaultFile {
                                            name: proj.name.clone(),
                                            category: proj.category.clone(),
                                            rel_path: proj.rel_path.clone(),
                                            size_bytes: proj.size_bytes,
                                        });
                                    }
                                    if ui
                                        .button(
                                            RichText::new(format!("[{geo_tag}] {}", proj.name))
                                                .size(10.5)
                                                .color(theme.text_primary),
                                        )
                                        .on_hover_text("Load Vault project onto Earth 3D (auto-flies to building)")
                                        .clicked()
                                    {
                                        actions.open_vault_project = Some(proj.clone());
                                    }
                                    if ui
                                        .small_button("Place")
                                        .on_hover_text("Place or relocate this model anywhere on Earth 3D interactively")
                                        .clicked()
                                    {
                                        universal_canvas.globe_mode = true;
                                        universal_canvas.placing_unlocated_rel_path = Some(proj.rel_path.clone());
                                        universal_canvas.placing_heading_deg = proxy_opt
                                            .as_ref()
                                            .map(|b| b.heading_deg)
                                            .unwrap_or(0.0);
                                        self.vault_drawer_open = false;
                                        self.push_trace(format!("Click Earth 3D to place '{}'", proj.name), true);
                                    }
                                    if ui.small_button("Ren").on_hover_text("Rename file").clicked() {
                                        self.vault_rename_target = Some((proj.rel_path.clone(), proj.name.clone()));
                                    }
                                    if ui.small_button("Del").on_hover_text("Delete Vault file").clicked()
                                        && crate::core::InternalVault::delete_vault_context_file(&proj.rel_path).is_ok()
                                    {
                                        self.vault_feedback = Some(format!("Deleted {}", proj.name));
                                        actions.refresh_vault = true;
                                    }
                                });
                            }
                            for vf in vault_files.iter().take(64) {
                                if vault_projects.iter().any(|p| p.rel_path == vf.rel_path) {
                                    continue;
                                }
                                if !fq.is_empty()
                                    && !vf.name.to_lowercase().contains(&fq)
                                    && !vf.category.to_lowercase().contains(&fq)
                                {
                                    continue;
                                }
                                let is_ifcx = vf.name.to_ascii_lowercase().ends_with(".ifcx");
                                ui.horizontal(|ui| {
                                    if ui
                                        .small_button("+@")
                                        .on_hover_text("Attach file into AI Context")
                                        .clicked()
                                    {
                                        self.attach_context(AiContextAttachment::VaultFile {
                                            name: vf.name.clone(),
                                            category: vf.category.clone(),
                                            rel_path: vf.rel_path.clone(),
                                            size_bytes: vf.size_bytes,
                                        });
                                    }
                                    if ui
                                        .button(
                                            RichText::new(format!("{} ({})", vf.name, vf.category))
                                                .size(10.0)
                                                .color(theme.text_secondary),
                                        )
                                        .on_hover_text(if is_ifcx {
                                            "Load this OpenBIM IFCX (IFC 5) layer directly onto Earth 3D"
                                        } else {
                                            "Click to preview Obsidian Vault note / file"
                                        })
                                        .clicked()
                                    {
                                        if is_ifcx {
                                            actions.open_vault_project = Some(
                                                crate::core::vault::PersonalVaultProjectEntry {
                                                    name: vf.name.clone(),
                                                    filename: vf.name.clone(),
                                                    rel_path: vf.rel_path.clone(),
                                                    abs_path: crate::core::InternalVault::vault_root_dir()
                                                        .join(&vf.rel_path)
                                                        .to_string_lossy()
                                                        .to_string(),
                                                    category: vf.category.clone(),
                                                    node_count: 0,
                                                    member_count: 0,
                                                    size_bytes: vf.size_bytes,
                                                    updated_label: "IFC".to_string(),
                                                },
                                            );
                                        } else {
                                            let preview =
                                                crate::core::InternalVault::read_vault_context_snippet(
                                                    &vf.rel_path,
                                                    600,
                                                )
                                                .unwrap_or_else(|| "(Binary or empty file)".to_string());
                                            self.previewed_vault_note = Some((vf.rel_path.clone(), preview));
                                        }
                                    }
                                    if ui.small_button("Ren").on_hover_text("Rename Vault file").clicked() {
                                        self.vault_rename_target = Some((vf.rel_path.clone(), vf.name.clone()));
                                    }
                                    if ui.small_button("Del").on_hover_text("Delete Vault file").clicked()
                                        && crate::core::InternalVault::delete_vault_context_file(&vf.rel_path).is_ok()
                                    {
                                        self.vault_feedback = Some(format!("Deleted {}", vf.name));
                                        actions.refresh_vault = true;
                                    }
                                });
                            }
                        });
                }
            });

        actions
    }

    /// Interactive `@ Add Context` popover allowing live workspace domains, Vault files & disk files.
    pub(super) fn render_context_picker_popover(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        project: &BimProject,
        vault_projects: &[crate::core::vault::PersonalVaultProjectEntry],
        panel_w: f32,
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();
        let vault_files = crate::core::InternalVault::list_vault_context_files(14);
        let q = self.context_search_query.trim().to_lowercase();

        Frame::new()
            .fill(Color32::TRANSPARENT)
            .inner_margin(Margin::symmetric(2, 4))
            .show(ui, |ui| {
                let box_w = (panel_w - 10.0).max(160.0);
                ui.set_width(box_w);
                ui.set_max_width(box_w);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                ui.horizontal(|ui| {
                    icons::icon_label(
                        ui,
                        AppIcon::Layers,
                        12.5,
                        theme.accent_blue,
                        "Add Context (@)",
                        11.5,
                        theme.text_primary,
                        true,
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::icon_only_button(
                            ui,
                            AppIcon::Close,
                            theme.text_secondary,
                            Color32::TRANSPARENT,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::splat(18.0),
                            "Close context picker",
                        )
                        .clicked()
                        {
                            self.context_picker_open = false;
                        }
                    });
                });
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.context_search_query)
                        .desired_width((box_w - 6.0).max(130.0))
                        .hint_text("Filter domains, Vault files, or attach local file..."),
                );
                ui.add_space(5.0);

                ScrollArea::vertical()
                    .id_salt("antigravity_add_context_scroll")
                    .max_height(185.0)
                    .show(ui, |ui| {
                        let inner_w = (box_w - 12.0).max(140.0);
                        ui.set_max_width(inner_w);

                        let domains = [
                            (
                                AiContextAttachment::SelectedElement,
                                "Current Selection",
                                self.active_context_mention(project)
                                    .unwrap_or_else(|| "Click any column, beam, or Earth site".to_string()),
                            ),
                            (
                                AiContextAttachment::EarthSite,
                                "Earth Viewport & Hazards",
                                "Coordinates, elevation, seismic PGA Ss & climate".to_string(),
                            ),
                            (
                                AiContextAttachment::WorldPortfolio,
                                "Planetary Vault Portfolio",
                                "Global 10,000+ project index, staging & critical counts".to_string(),
                            ),
                            (
                                AiContextAttachment::BimGeometry,
                                "BIM Structural Topology",
                                format!(
                                    "{} cols, {} beams, {} walls",
                                    project.columns.len(),
                                    project.runs.len(),
                                    project.walls.len()
                                ),
                            ),
                            (
                                AiContextAttachment::FemRedundancy,
                                "FEM Stiffness & EigenGate Matrix",
                                "DCR%, SVD nullspace & critical members".to_string(),
                            ),
                            (
                                AiContextAttachment::ScheduleAndCost,
                                "4D Schedule & 5D BoQ Cost",
                                "Concrete QTO, AHSP budget & erection duration".to_string(),
                            ),
                            (
                                AiContextAttachment::VaultWorkspace,
                                "Connected Vault Workspace",
                                "Vault summary, categories & knowledge memory".to_string(),
                            ),
                        ];

                        for (att, title, sub) in domains {
                            if !q.is_empty()
                                && !title.to_lowercase().contains(&q)
                                && !sub.to_lowercase().contains(&q)
                            {
                                continue;
                            }
                            let key = att.dedup_key();
                            let active = self.attached_contexts.iter().any(|a| a.dedup_key() == key);
                            let btn_fill = if active {
                                theme.accent_blue.gamma_multiply(0.18)
                            } else {
                                theme.bg_card_hover.gamma_multiply(0.45)
                            };
                            if ui
                                .add_sized(
                                    Vec2::new(inner_w, 26.0),
                                    egui::Button::new(
                                        RichText::new(format!(
                                            "{} {} — {}",
                                            if active { "[Attached]" } else { "+" },
                                            title,
                                            sub
                                        ))
                                        .size(10.2)
                                        .color(theme.text_primary),
                                    )
                                    .wrap_mode(egui::TextWrapMode::Wrap)
                                    .fill(btn_fill)
                                    .stroke(Stroke::new(0.8, theme.border_subtle))
                                    .corner_radius(CornerRadius::same(7)),
                                )
                                .clicked()
                            {
                                if active {
                                    self.detach_context(&key);
                                } else {
                                    self.attach_context(att);
                                }
                            }
                            ui.add_space(2.0);
                        }

                        if !vault_projects.is_empty() || !vault_files.is_empty() {
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("VAULT FILES")
                                    .size(9.5)
                                    .strong()
                                    .color(theme.text_muted),
                            );
                            for vp_entry in vault_projects.iter().take(6) {
                                if !q.is_empty() && !vp_entry.name.to_lowercase().contains(&q) {
                                    continue;
                                }
                                let att = AiContextAttachment::VaultFile {
                                    name: vp_entry.name.clone(),
                                    category: vp_entry.category.clone(),
                                    rel_path: vp_entry.rel_path.clone(),
                                    size_bytes: vp_entry.size_bytes,
                                };
                                let key = att.dedup_key();
                                let active =
                                    self.attached_contexts.iter().any(|a| a.dedup_key() == key);
                                if ui
                                    .add_sized(
                                        Vec2::new(inner_w, 24.0),
                                        egui::Button::new(
                                            RichText::new(format!(
                                                "{} {} ({})",
                                                if active { "[Attached]" } else { "+ @" },
                                                vp_entry.name,
                                                vp_entry.category
                                            ))
                                            .size(10.0)
                                            .color(theme.text_primary),
                                        )
                                        .wrap_mode(egui::TextWrapMode::Wrap)
                                        .fill(theme.bg_card)
                                        .stroke(Stroke::new(0.8, theme.border_subtle))
                                        .corner_radius(CornerRadius::same(6)),
                                    )
                                    .clicked()
                                {
                                    if active {
                                        self.detach_context(&key);
                                    } else {
                                        self.attach_context(att);
                                    }
                                }
                                ui.add_space(2.0);
                            }
                            for vf in vault_files.iter().take(6) {
                                if vault_projects.iter().any(|p| p.rel_path == vf.rel_path) {
                                    continue;
                                }
                                if !q.is_empty() && !vf.rel_path.to_lowercase().contains(&q) {
                                    continue;
                                }
                                let att = AiContextAttachment::VaultFile {
                                    name: vf.name.clone(),
                                    category: vf.category.clone(),
                                    rel_path: vf.rel_path.clone(),
                                    size_bytes: vf.size_bytes,
                                };
                                let key = att.dedup_key();
                                let active =
                                    self.attached_contexts.iter().any(|a| a.dedup_key() == key);
                                if ui
                                    .add_sized(
                                        Vec2::new(inner_w, 24.0),
                                        egui::Button::new(
                                            RichText::new(format!(
                                                "{} {}",
                                                if active { "[Attached]" } else { "+ @" },
                                                vf.rel_path
                                            ))
                                            .size(10.0)
                                            .color(theme.text_primary),
                                        )
                                        .wrap_mode(egui::TextWrapMode::Wrap)
                                        .fill(theme.bg_card)
                                        .stroke(Stroke::new(0.8, theme.border_subtle))
                                        .corner_radius(CornerRadius::same(6)),
                                    )
                                    .clicked()
                                {
                                    if active {
                                        self.detach_context(&key);
                                    } else {
                                        self.attach_context(att);
                                    }
                                }
                                ui.add_space(2.0);
                            }
                        }

                        ui.add_space(4.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::UploadImport,
                            "Attach Local File (.ifcx, .json, .csv, .md)...",
                            10.5,
                            theme.accent_blue,
                            theme.accent_blue.gamma_multiply(0.12),
                            Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.4)),
                            CornerRadius::same(7),
                            Vec2::new(inner_w, 25.0),
                        )
                        .clicked()
                        {
                        actions.pick_local_context_file = true;
                            self.context_picker_open = false;
                        }
                    });
            });

        actions
    }

    /// Phase 80 (`v0.8.0`): Width-first 2-tab drawer inside the Single Dynamic Island hosting:
    /// - Tab 0 (`Checkpoints & Merge`): SHA-256 chained IFCX checkpoints, formal discipline sign-off,
    ///   `.ifcx-bundle.json` export, and deterministic Three-Way Peer Merge with conflict resolution.
    /// - Tab 1 (`District Risk Field`): 3D Pure WebGPU IDW District Risk Heatmap controls, pure-vector
    ///   risk distribution chart, and clickable top hotspot parcels.
    pub(super) fn render_collab_and_risk_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        use crate::core::ifcx_collab::{
            apply_conflict_resolution, ConflictResolution, DistrictRiskMetric,
        };

        let deck_w = (cap_w - 8.0).min(ui.available_width()).max(200.0);
        let two_col = !is_mobile && deck_w >= 680.0;

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(deck_w);
            icons::pill_badge(
                ui,
                AppIcon::Shield,
                "IFCX Governance, Risk & Permit",
                theme.accent_blue,
            );
            for (tab_idx, tab_lbl) in [
                (0_usize, "Checkpoints & Relay"),
                (1_usize, "District Risk Field"),
                (2_usize, "Trade-Study & Permit"),
            ] {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    tab_lbl,
                    self.collab_drawer_tab == tab_idx,
                    theme.accent_blue,
                    "",
                )
                .clicked()
                {
                    self.collab_drawer_tab = tab_idx;
                    if tab_idx == 1 && universal_canvas.last_risk_field_report.is_none() {
                        actions.refresh_district_risk_field = true;
                    } else if tab_idx == 2 {
                        if universal_canvas.last_trade_study_report.is_none() {
                            actions.run_trade_study =
                                Some(universal_canvas.trade_study_config.objective);
                        }
                        if universal_canvas.last_permit_report.is_none() {
                            actions.run_permit_compliance = true;
                        }
                    }
                }
            }
            let (chain_ok, bad_links) = universal_canvas.collab_state.verify_chain();
            icons::pill_badge(
                ui,
                AppIcon::CheckCircle,
                if universal_canvas.collab_state.checkpoints.is_empty() {
                    "Chain Ready (0 CP)".to_string()
                } else if chain_ok {
                    format!(
                        "SHA-256 Chain OK ({})",
                        universal_canvas.collab_state.checkpoints.len()
                    )
                } else {
                    format!("TAMPER ALERT ({bad_links})")
                },
                if chain_ok {
                    theme.accent_green
                } else {
                    theme.accent_red
                },
            );
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close IFCX Governance & Risk drawer",
            )
            .clicked()
            {
                self.collab_drawer_open = false;
            }
        });

        ui.add_space(4.0);
        let tab = self.collab_drawer_tab;
        Self::width_first_pair(ui, deck_w, two_col, |ui, is_left, col_w| {
            theme.glass_subpill_frame(9, Margin::symmetric(8, 6)).show(ui, |ui| {
                let inner_w = (col_w - 20.0).max(150.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.vertical(|ui| {
                    ui.set_max_width(inner_w);
                if tab == 0 {
                    if is_left {
                        ui.label(
                            RichText::new("SHA-256 Signed Checkpoints & Handover")
                                .size(10.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.horizontal_wrapped(|ui| {
                            ui.set_max_width(inner_w);
                            ui.label(RichText::new("Label:").size(9.2).color(theme.text_muted));
                            ui.add(
                                egui::TextEdit::singleline(
                                    &mut universal_canvas.collab_state.checkpoint_label_draft,
                                )
                                .desired_width((inner_w * 0.42).clamp(80.0, 160.0)),
                            );
                            let disc = universal_canvas.collab_state.config.active_discipline;
                            let [dr, dg, db] = disc.badge_color_rgb();
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                None,
                                disc.short_label(),
                                true,
                                Color32::from_rgb(dr, dg, db),
                                "Cycle discipline owner for next IFCX checkpoint",
                            )
                            .clicked()
                            {
                                universal_canvas.collab_state.config.active_discipline = disc.next();
                            }
                        });
                        ui.add_space(3.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.set_max_width(inner_w);
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                Some(AppIcon::Plus),
                                "+ Checkpoint",
                                false,
                                theme.accent_blue,
                                "Freeze current layered .ifcx graph + KPIs into tamper-proof SHA-256 chain",
                            )
                            .clicked()
                            {
                                let lbl = universal_canvas.collab_state.checkpoint_label_draft.clone();
                                actions.create_checkpoint_label = Some(lbl);
                            }
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                Some(AppIcon::Shield),
                                "Approve",
                                false,
                                theme.accent_green,
                                "Mark selected checkpoint APPROVED and re-seal SHA-256 chain",
                            )
                            .clicked()
                            {
                                actions.approve_checkpoint = true;
                            }
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                Some(AppIcon::SaveDisk),
                                ".ifcx-bundle",
                                false,
                                theme.accent_blue,
                                "Export signed multi-checkpoint .ifcx-bundle.json handover package",
                            )
                            .clicked()
                            {
                                actions.export_collab_bundle_ifcx = true;
                            }
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                Some(AppIcon::UploadImport),
                                "Import Bundle",
                                false,
                                theme.accent_blue,
                                "Load & verify a signed .ifcx-bundle.json handover file from disk",
                            )
                            .clicked()
                            {
                                actions.open_collab_bundle_picker = true;
                            }
                        });
                        ui.add_space(4.0);
                        ui.separator();
                        ui.label(
                            RichText::new("Checkpoint Lineage Timeline:")
                                .size(9.5)
                                .strong()
                                .color(theme.text_secondary),
                        );
                        if universal_canvas.collab_state.checkpoints.is_empty() {
                            ui.label(
                                RichText::new("No checkpoints recorded yet. Click '+ Checkpoint' or run /checkpoint.")
                                    .size(9.0)
                                    .color(theme.text_muted),
                            );
                        } else {
                            let active_i = universal_canvas.collab_state.active_checkpoint_idx;
                            let mut pick_idx: Option<usize> = None;
                            ui.horizontal_wrapped(|ui| {
                                ui.set_max_width(inner_w);
                                for (idx, cp) in universal_canvas.collab_state.checkpoints.iter().enumerate() {
                                    let [sr, sg, sb] = cp.approval_status.badge_color_rgb();
                                    let sel = active_i == Some(idx);
                                    if Self::glass_pill_button(
                                        ui,
                                        theme,
                                        None,
                                        format!(
                                            "{} [{}] • {} • EigenGate {:.2}",
                                            cp.checkpoint_id,
                                            cp.approval_status.short_label(),
                                            cp.short_sha256(),
                                            cp.kpi.min_eigengate
                                        ),
                                        sel,
                                        Color32::from_rgb(sr, sg, sb),
                                        &format!("{} | Author: {} | Nodes: {}", cp.label, cp.author, cp.kpi.node_count),
                                    )
                                    .clicked()
                                    {
                                        pick_idx = Some(idx);
                                    }
                                }
                            });
                            if let Some(idx) = pick_idx {
                                universal_canvas.collab_state.active_checkpoint_idx = Some(idx);
                            }
                        }
                    } else {
                        ui.label(
                            RichText::new("Deterministic 3-Way Peer IFCX Merge & Live Sync Relay")
                                .size(10.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.horizontal_wrapped(|ui| {
                            ui.set_max_width(inner_w);
                            ui.label(RichText::new("Relay:").size(9.0).color(theme.text_muted));
                            ui.add(
                                egui::TextEdit::singleline(
                                    &mut universal_canvas.collab_state.relay_url_draft,
                                )
                                .desired_width((inner_w * 0.44).clamp(80.0, 170.0)),
                            );
                            let r_conn = universal_canvas.collab_state.relay_connected;
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                Some(AppIcon::Pulse),
                                if r_conn { "Disconnect" } else { "Connect" },
                                r_conn,
                                if r_conn { theme.accent_green } else { theme.accent_blue },
                                "Connect/disconnect WebSocket IFCX Delta Sync Relay",
                            )
                            .clicked()
                            {
                                if r_conn {
                                    actions.disconnect_collab_relay = true;
                                } else {
                                    actions.connect_collab_relay =
                                        Some(universal_canvas.collab_state.relay_url_draft.clone());
                                }
                            }
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                None,
                                "Sync Peer",
                                false,
                                theme.accent_green,
                                "Fetch or simulate incoming peer delta over Sync Relay & run 3-way merge",
                            )
                            .clicked()
                            {
                                actions.sync_peer_relay_delta = true;
                            }
                        });
                        ui.label(
                            RichText::new(format!(
                                "Relay: {} (synced: {})",
                                universal_canvas.collab_state.relay_status,
                                universal_canvas.collab_state.relay_synced_count
                            ))
                            .size(8.6)
                            .color(theme.text_muted),
                        );
                        ui.add_space(2.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.set_max_width(inner_w);
                            let pdisc = universal_canvas.collab_state.peer_discipline;
                            let [pr, pg, pb] = pdisc.badge_color_rgb();
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                None,
                                format!("Peer: {}", pdisc.short_label()),
                                false,
                                Color32::from_rgb(pr, pg, pb),
                                "Cycle incoming peer branch discipline",
                            )
                            .clicked()
                            {
                                universal_canvas.collab_state.peer_discipline = pdisc.next();
                            }
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                Some(AppIcon::Layers),
                                "Peer 3-Way Merge",
                                true,
                                theme.accent_orange,
                                "Run Base + Local + Peer 3-way node/attribute merge with conflict resolver",
                            )
                            .clicked()
                            {
                                actions.run_ifcx_three_way_merge = true;
                            }
                        });
                        ui.add_space(3.0);
                        if let Some(ref mut merge) = universal_canvas.collab_state.last_merge {
                            ui.label(
                                RichText::new(&merge.summary)
                                    .size(9.0)
                                    .monospace()
                                    .color(theme.accent_green),
                            );
                            let mut pending_res: Option<(usize, ConflictResolution)> = None;
                            for (ci, c) in merge.conflicts.iter().enumerate() {
                                let short_path = c.path.rsplit('/').next().unwrap_or(&c.path);
                                ui.horizontal_wrapped(|ui| {
                                    ui.set_max_width(inner_w);
                                    ui.label(
                                        RichText::new(format!("{short_path} [{}]", c.attribute_key))
                                            .size(8.8)
                                            .monospace()
                                            .color(theme.text_primary),
                                    );
                                    for res in [
                                        ConflictResolution::KeepLocal,
                                        ConflictResolution::KeepIncoming,
                                        ConflictResolution::KeepBothLayered,
                                    ] {
                                        if Self::glass_pill_button(
                                            ui,
                                            theme,
                                            None,
                                            res.short_label(),
                                            c.resolution == res,
                                            theme.accent_blue,
                                            res.label(),
                                        )
                                        .clicked()
                                        {
                                            pending_res = Some((ci, res));
                                        }
                                    }
                                });
                            }
                            if let Some((ci, res)) = pending_res {
                                apply_conflict_resolution(merge, ci, res);
                            }
                        } else {
                            ui.label(
                                RichText::new("Click 'Peer 3-Way Merge' or run /merge-ifcx to reconcile concurrent discipline layers.")
                                    .size(9.0)
                                    .color(theme.text_muted),
                            );
                        }
                    }
                } else if tab == 1 && is_left {
                    ui.label(
                        RichText::new("3D WebGPU District Risk Field Heatmap")
                            .size(10.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    ui.horizontal_wrapped(|ui| {
                        ui.set_max_width(inner_w);
                        let vis = universal_canvas.collab_state.config.risk_heatmap_visible;
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            Some(AppIcon::Flame),
                            if vis { "Risk Heatmap: ON" } else { "Risk Heatmap: OFF" },
                            vis,
                            theme.accent_red,
                            "Toggle 3D Pure WebGPU iso-contour Risk Field draped over Earth site terrain",
                        )
                        .clicked()
                        {
                            universal_canvas.collab_state.config.risk_heatmap_visible = !vis;
                            if universal_canvas.collab_state.config.risk_heatmap_visible {
                                actions.refresh_district_risk_field = true;
                            }
                        }
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            Some(AppIcon::Bolt),
                            "Recompute",
                            false,
                            theme.accent_blue,
                            "Re-evaluate IDW Spatial Risk Field uniting EigenGate, ATC-20, PML-90, SSI & Evacuation",
                        )
                        .clicked()
                        {
                            universal_canvas.collab_state.config.risk_heatmap_visible = true;
                            actions.refresh_district_risk_field = true;
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.set_max_width(inner_w);
                        ui.label(RichText::new("Metric:").size(9.2).color(theme.text_muted));
                        for &m in DistrictRiskMetric::all() {
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                None,
                                m.short_label(),
                                universal_canvas.collab_state.config.risk_metric == m,
                                theme.accent_orange,
                                m.label(),
                            )
                            .clicked()
                            {
                                universal_canvas.collab_state.config.risk_metric = m;
                                universal_canvas.collab_state.config.risk_heatmap_visible = true;
                                actions.refresh_district_risk_field = true;
                            }
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.set_max_width(inner_w);
                        ui.label(RichText::new("Opacity:").size(9.2).color(theme.text_muted));
                        ui.spacing_mut().slider_width = (inner_w * 0.42).clamp(64.0, 130.0);
                        ui.add(
                            egui::Slider::new(
                                &mut universal_canvas.collab_state.config.risk_opacity,
                                0.25..=0.90,
                            )
                            .fixed_decimals(2),
                        );
                    });
                    if let Some(ref rep) = universal_canvas.last_risk_field_report {
                        ui.label(
                            RichText::new(&rep.summary)
                                .size(9.0)
                                .monospace()
                                .color(theme.text_primary),
                        );
                        // Pure-vector risk tier distribution bar chart
                        let chart_w = inner_w.clamp(130.0, 300.0);
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(chart_w, 26.0), egui::Sense::hover());
                        let painter = ui.painter_at(rect);
                        let total = (rep.low_count + rep.moderate_count + rep.high_count + rep.critical_count).max(1) as f32;
                        let tiers = [
                            (rep.low_count, Color32::from_rgb(16, 185, 129), "Low"),
                            (rep.moderate_count, Color32::from_rgb(6, 182, 212), "Mod"),
                            (rep.high_count, Color32::from_rgb(245, 158, 11), "High"),
                            (rep.critical_count, Color32::from_rgb(239, 68, 68), "Crit"),
                        ];
                        let mut x0 = rect.left();
                        for (cnt, col, _) in tiers {
                            let w = rect.width() * (cnt as f32 / total);
                            if w >= 1.0 {
                                painter.rect_filled(
                                    Rect::from_min_size(egui::Pos2::new(x0, rect.top() + 2.0), Vec2::new(w, 12.0)),
                                    CornerRadius::same(2),
                                    col,
                                );
                                x0 += w;
                            }
                        }
                        painter.text(
                            egui::Pos2::new(rect.left(), rect.top() + 16.0),
                            egui::Align2::LEFT_TOP,
                            format!(
                                "Low:{} Mod:{} High:{} Crit:{}",
                                rep.low_count, rep.moderate_count, rep.high_count, rep.critical_count
                            ),
                            egui::FontId::monospace(8.5),
                            theme.text_secondary,
                        );
                    }
                } else if tab == 1 {
                    ui.label(
                        RichText::new("Top District Risk Hotspot Parcels")
                            .size(10.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    if let Some(ref rep) = universal_canvas.last_risk_field_report {
                        let unit = rep.metric.unit_label();
                        let mut fly_target: Option<(String, f64, f64)> = None;
                        for h in rep.hotspots.iter().take(5) {
                            let col = if h.risk_score >= 0.70 {
                                theme.accent_red
                            } else if h.risk_score >= 0.45 {
                                theme.accent_orange
                            } else {
                                theme.accent_green
                            };
                            ui.horizontal_wrapped(|ui| {
                                ui.set_max_width(inner_w);
                                ui.colored_label(
                                    col,
                                    RichText::new(format!("{:.0}%", h.risk_score * 100.0))
                                        .size(9.2)
                                        .strong()
                                        .monospace(),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "{} ({}) • EigenGate {:.2} • {:.1} {unit}",
                                        h.name, h.atc20_tag, h.min_eigengate, h.metric_value
                                    ))
                                    .size(8.8)
                                    .color(theme.text_secondary),
                                );
                                if Self::glass_pill_button(
                                    ui,
                                    theme,
                                    Some(AppIcon::Target),
                                    "Fly",
                                    false,
                                    theme.accent_blue,
                                    "Fly Earth camera to hotspot parcel",
                                )
                                .clicked()
                                {
                                    fly_target = Some((h.name.clone(), h.lat, h.lon));
                                }
                            });
                        }
                        if let Some((name, lat, lon)) = fly_target {
                            self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                            universal_canvas.globe_mode = true;
                            universal_canvas.earth_world.start_fly_to(
                                &universal_canvas.world_map,
                                lat,
                                lon,
                                16.4,
                                &name,
                            );
                            self.push_trace(format!("Flew Earth -> Hotspot '{name}'"), true);
                        }
                    } else {
                        ui.label(
                            RichText::new("Click 'Recompute' or run /risk-map to populate spatial hotspots.")
                                .size(9.0)
                                .color(theme.text_muted),
                        );
                    }
                } else if is_left {
                    use crate::core::trade_study::TradeStudyObjective;

                    ui.label(
                        RichText::new("Parametric 6-Scenario Trade-Study Matrix")
                            .size(10.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    ui.horizontal_wrapped(|ui| {
                        ui.set_max_width(inner_w);
                        ui.label(RichText::new("Goal:").size(9.0).color(theme.text_muted));
                        for &obj in TradeStudyObjective::all() {
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                None,
                                obj.short_label(),
                                universal_canvas.trade_study_config.objective == obj,
                                theme.accent_blue,
                                obj.label(),
                            )
                            .clicked()
                            {
                                universal_canvas.trade_study_config.objective = obj;
                                actions.run_trade_study = Some(obj);
                            }
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.set_max_width(inner_w);
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            Some(AppIcon::Bolt),
                            "Run Trade-Study",
                            true,
                            theme.accent_blue,
                            "Evaluate all 6 structural alternatives across EigenGate, DCR%, Cost & Carbon",
                        )
                        .clicked()
                        {
                            actions.run_trade_study = Some(universal_canvas.trade_study_config.objective);
                        }
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            Some(AppIcon::Cube3D),
                            "Trade-Study .ifcx",
                            false,
                            theme.accent_green,
                            "Export non-destructive eigengraph::analysis::trade_study_matrix .ifcx overlay",
                        )
                        .clicked()
                        {
                            actions.export_trade_study_ifcx = true;
                        }
                    });
                    if let Some(ref ts) = universal_canvas.last_trade_study_report {
                        ui.label(
                            RichText::new(&ts.summary)
                                .size(8.8)
                                .monospace()
                                .color(theme.accent_green),
                        );
                        // Pure-vector Trade-Study utility comparison bar chart
                        let chart_w = inner_w.clamp(140.0, 320.0);
                        let row_h = 13.0_f32;
                        let chart_h = (ts.scenarios.len().min(6) as f32) * row_h + 4.0;
                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(chart_w, chart_h),
                            egui::Sense::hover(),
                        );
                        let painter = ui.painter_at(rect);
                        for (i, s) in ts.scenarios.iter().take(6).enumerate() {
                            let y = rect.top() + 2.0 + (i as f32) * row_h;
                            let bw = ((s.weighted_utility_score / 100.0).clamp(0.04, 1.0) as f32)
                                * (rect.width() - 90.0).max(40.0);
                            let col = if s.is_recommended {
                                Color32::from_rgb(16, 185, 129)
                            } else {
                                Color32::from_rgb(56, 189, 248)
                            };
                            painter.text(
                                egui::Pos2::new(rect.left(), y + 5.5),
                                egui::Align2::LEFT_CENTER,
                                s.archetype.short_label(),
                                egui::FontId::monospace(8.0),
                                theme.text_secondary,
                            );
                            painter.rect_filled(
                                Rect::from_min_size(
                                    egui::Pos2::new(rect.left() + 82.0, y + 1.5),
                                    Vec2::new(bw, 8.5),
                                ),
                                CornerRadius::same(2),
                                col,
                            );
                        }
                        ui.add_space(2.0);
                        for s in ts.scenarios.iter().take(6) {
                            ui.horizontal_wrapped(|ui| {
                                ui.set_max_width(inner_w);
                                let lbl_col = if s.is_recommended {
                                    theme.accent_green
                                } else {
                                    theme.text_primary
                                };
                                ui.label(
                                    RichText::new(format!(
                                        "{} | EigenGate {:.2} | DCR {:.0}% | ${:.0}k | {:.0}t",
                                        s.archetype.short_label(),
                                        s.min_eigengate,
                                        s.max_dcr_pct,
                                        s.estimated_cost_usd * 0.001,
                                        s.embodied_carbon_tco2e,
                                    ))
                                    .size(8.6)
                                    .monospace()
                                    .color(lbl_col),
                                );
                                if Self::glass_pill_button(
                                    ui,
                                    theme,
                                    None,
                                    if s.is_recommended { "Adopt *" } else { "Adopt" },
                                    s.is_recommended,
                                    theme.accent_green,
                                    &s.archetype.description(),
                                )
                                .clicked()
                                {
                                    actions.adopt_trade_study_scenario = Some(s.archetype);
                                }
                            });
                        }
                    } else {
                        ui.label(
                            RichText::new("Click 'Run Trade-Study' or /trade-study to compare 6 design options.")
                                .size(9.0)
                                .color(theme.text_muted),
                        );
                    }
                } else {
                    use crate::core::trade_study::PermitClauseStatus;

                    ui.label(
                        RichText::new("Automated SNI / ASCE / UFC Permit Compliance")
                            .size(10.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    ui.horizontal_wrapped(|ui| {
                        ui.set_max_width(inner_w);
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            Some(AppIcon::CheckCircle),
                            "Audit Permit",
                            false,
                            theme.accent_blue,
                            "Evaluate 10 regulatory clauses (SNI 1726/2847/8460, EigenGate, ATC-20, FEMA P-58, IFCX Seal)",
                        )
                        .clicked()
                        {
                            actions.run_permit_compliance = true;
                        }
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            Some(AppIcon::Printer),
                            "Permit Annex .svg (S-102)",
                            false,
                            theme.accent_green,
                            "Export A3 Pure-Vector Permit Compliance & Trade-Study Annex SVG (Sheet S-102)",
                        )
                        .clicked()
                        {
                            actions.export_permit_annex_svg = true;
                        }
                    });
                    if let Some(ref permit) = universal_canvas.last_permit_report {
                        ui.label(
                            RichText::new(&permit.summary)
                                .size(9.0)
                                .monospace()
                                .color(if permit.overall_ready {
                                    theme.accent_green
                                } else {
                                    theme.accent_orange
                                }),
                        );
                        for cl in &permit.clauses {
                            let [cr, cg, cb] = cl.status.badge_color_rgb();
                            ui.horizontal_wrapped(|ui| {
                                ui.set_max_width(inner_w);
                                ui.colored_label(
                                    Color32::from_rgb(cr, cg, cb),
                                    RichText::new(format!("[{}]", cl.status.short_label()))
                                        .size(8.6)
                                        .strong()
                                        .monospace(),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "{}: {} (Lim {})",
                                        cl.code_ref, cl.demand_value, cl.limit_value
                                    ))
                                    .size(8.6)
                                    .color(if cl.status == PermitClauseStatus::Fail {
                                        theme.accent_red
                                    } else {
                                        theme.text_secondary
                                    }),
                                );
                            });
                        }
                    } else {
                        ui.label(
                            RichText::new("Click 'Audit Permit' or /permit-check to run the 10-clause code check.")
                                .size(9.0)
                                .color(theme.text_muted),
                        );
                    }
                }
                });
            });
        });
    }

    /// Phase 82 (`v0.8.2`): Width-first Dynamic Island drawer for Field Construction QA/QC
    /// Spatial Waypoints, 1D Rebar BBS Cutting-Stock Optimization & Sheet `S-103` Fabrication Annex.
    pub(super) fn render_field_qaqc_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        let deck_w = (cap_w - 8.0).max(180.0);
        let two_col = !is_mobile && deck_w >= 540.0;
        ui.set_width(deck_w);
        ui.set_max_width(deck_w);

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(deck_w);
            icons::icon_label(
                ui,
                AppIcon::Wrench,
                13.0,
                theme.accent_blue,
                "Field QA/QC & Rebar BBS",
                11.0,
                theme.text_primary,
                true,
            );
            for (idx, lbl) in [(0usize, "Rebar BBS & Cut-List"), (1usize, "Field Waypoints & ITP/NCR")] {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    lbl,
                    self.field_qaqc_drawer_tab == idx,
                    theme.accent_blue,
                    "",
                )
                .clicked()
                {
                    self.field_qaqc_drawer_tab = idx;
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close Field QA/QC & Rebar BBS drawer",
            )
            .clicked()
            {
                self.field_qaqc_drawer_open = false;
            }
        });

        ui.add_space(4.0);
        let tab = self.field_qaqc_drawer_tab;
        Self::width_first_pair(ui, deck_w, two_col, |ui, is_left, col_w| {
            theme.glass_subpill_frame(9, Margin::symmetric(8, 6)).show(ui, |ui| {
                let inner_w = (col_w - 20.0).max(150.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.vertical(|ui| {
                    ui.set_max_width(inner_w);
                    if tab == 0 {
                        if is_left {
                            ui.label(
                                RichText::new("1D Master Stock Bar Cutting-Stock Optimizer")
                                    .size(10.0)
                                    .strong()
                                    .color(theme.text_primary),
                            );
                            ui.horizontal_wrapped(|ui| {
                                ui.set_max_width(inner_w);
                                ui.label(RichText::new("Stock:").size(9.0).color(theme.text_muted));
                                for stock_m in [12.0_f64, 6.0_f64] {
                                    let active = (universal_canvas.field_fab_config.stock_length_m - stock_m).abs() < 0.2;
                                    if Self::glass_pill_button(
                                        ui,
                                        theme,
                                        None,
                                        format!("{stock_m:.0}.0 m"),
                                        active,
                                        theme.accent_blue,
                                        "Select commercial master rebar stock bar length",
                                    )
                                    .clicked()
                                    {
                                        universal_canvas.field_fab_config.stock_length_m = stock_m;
                                        actions.run_field_fabrication = Some(stock_m);
                                    }
                                }
                                if Self::glass_pill_button(
                                    ui,
                                    theme,
                                    Some(AppIcon::Bolt),
                                    "Optimize BBS",
                                    false,
                                    theme.accent_green,
                                    "Compute SNI 2847 Bar Bending Schedule & 1D BFD cut-list",
                                )
                                .clicked()
                                {
                                    actions.run_field_fabrication =
                                        Some(universal_canvas.field_fab_config.stock_length_m);
                                }
                            });

                            if let Some(ref rep) = universal_canvas.last_field_fab_report {
                                ui.add_space(2.0);
                                ui.label(
                                    RichText::new(format!(
                                        "Yield: {:.1}%  |  Scrap: {:.1}%  |  Bars: {} x {:.0}m",
                                        rep.cso_report.material_yield_pct,
                                        rep.cso_report.scrap_waste_pct,
                                        rep.cso_report.master_bars_used,
                                        rep.cso_report.stock_length_m,
                                    ))
                                    .size(9.2)
                                    .strong()
                                    .monospace()
                                    .color(theme.accent_green),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "Rebar Mass: {:.0} kg  |  Reusable Offcut: {:.1}m  |  Save ~${:.0}",
                                        rep.total_rebar_weight_kg,
                                        rep.cso_report.reusable_offcut_m,
                                        rep.estimated_savings_usd,
                                    ))
                                    .size(8.8)
                                    .monospace()
                                    .color(theme.text_secondary),
                                );
                            } else {
                                ui.label(
                                    RichText::new("Click 'Optimize BBS' or /bbs to compute yield & cut patterns.")
                                        .size(8.8)
                                        .color(theme.text_muted),
                                );
                            }

                            ui.add_space(3.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.set_max_width(inner_w);
                                if Self::glass_pill_button(
                                    ui,
                                    theme,
                                    Some(AppIcon::Printer),
                                    "Sheet S-103 .svg",
                                    false,
                                    theme.accent_green,
                                    "Export A3 Pure-Vector Rebar BBS & Field QA/QC Shop Annex (Sheet S-103)",
                                )
                                .clicked()
                                {
                                    actions.export_shop_drawing_svg = true;
                                }
                                if Self::glass_pill_button(
                                    ui,
                                    theme,
                                    Some(AppIcon::SaveDisk),
                                    "BBS .csv",
                                    false,
                                    theme.accent_blue,
                                    "Export Bar Bending Schedule CSV",
                                )
                                .clicked()
                                {
                                    actions.export_bbs_csv = true;
                                }
                                if Self::glass_pill_button(
                                    ui,
                                    theme,
                                    Some(AppIcon::SaveDisk),
                                    "Cut-List .csv",
                                    false,
                                    theme.accent_blue,
                                    "Export 1D CAM Cutting Pattern CSV",
                                )
                                .clicked()
                                {
                                    actions.export_cutlist_csv = true;
                                }
                            });
                        } else {
                            ui.label(
                                RichText::new("Pure-Vector Stock Bar Cutting Patterns")
                                    .size(10.0)
                                    .strong()
                                    .color(theme.text_primary),
                            );
                            if let Some(ref rep) = universal_canvas.last_field_fab_report {
                                let bar_count = rep.cso_report.bars.len().min(5);
                                if bar_count > 0 {
                                    let chart_h = (bar_count as f32) * 18.0 + 8.0;
                                    let (rect, _) = ui.allocate_exact_size(
                                        Vec2::new(inner_w, chart_h),
                                        egui::Sense::hover(),
                                    );
                                    let p = ui.painter_at(rect);
                                    let stock_m = rep.cso_report.stock_length_m.max(1.0);
                                    let bar_x0 = rect.left() + 42.0;
                                    let bar_w = (rect.width() - 48.0).max(40.0);
                                    for (idx, bar) in rep.cso_report.bars.iter().take(5).enumerate() {
                                        let y = rect.top() + 4.0 + (idx as f32) * 18.0;
                                        p.text(
                                            egui::pos2(rect.left() + 2.0, y + 6.5),
                                            egui::Align2::LEFT_CENTER,
                                            format!("#{}", bar.bar_index),
                                            egui::FontId::monospace(8.5),
                                            theme.text_secondary,
                                        );
                                        let track = egui::Rect::from_min_size(
                                            egui::pos2(bar_x0, y),
                                            Vec2::new(bar_w, 13.0),
                                        );
                                        p.rect_filled(track, 2.0, theme.bg_card.gamma_multiply(0.75));
                                        for (ci, cut) in bar.cuts.iter().enumerate() {
                                            let sx = bar_x0 + ((cut.start_m / stock_m) as f32) * bar_w;
                                            let cw = (((cut.length_m / stock_m) as f32) * bar_w).max(2.0);
                                            let col = if ci % 2 == 0 {
                                                theme.accent_blue
                                            } else {
                                                theme.accent_green
                                            };
                                            p.rect_filled(
                                                egui::Rect::from_min_size(
                                                    egui::pos2(sx, y + 1.0),
                                                    Vec2::new(cw, 11.0),
                                                ),
                                                1.5,
                                                col,
                                            );
                                        }
                                        if bar.remainder_m > 0.02 {
                                            let rx = bar_x0 + ((bar.used_length_m / stock_m) as f32) * bar_w;
                                            let rw = (((bar.remainder_m / stock_m) as f32) * bar_w).max(2.0);
                                            let rcol = if bar.is_reusable_offcut {
                                                Color32::from_rgb(34, 211, 238)
                                            } else {
                                                theme.accent_orange
                                            };
                                            p.rect_filled(
                                                egui::Rect::from_min_size(
                                                    egui::pos2(rx, y + 1.0),
                                                    Vec2::new(rw, 11.0),
                                                ),
                                                1.5,
                                                rcol,
                                            );
                                        }
                                    }
                                }
                                for item in rep.bbs_items.iter().take(3) {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} • D{:.0} x{} @ {:.0}mm ({:.1}kg)",
                                            item.mark,
                                            item.bar_size_mm,
                                            item.count,
                                            item.cut_length_mm,
                                            item.total_weight_kg,
                                        ))
                                        .size(8.6)
                                        .monospace()
                                        .color(theme.text_secondary),
                                    );
                                }
                            } else {
                                ui.label(
                                    RichText::new("Run 'Optimize BBS' to preview cutting-stock bars & BBS marks.")
                                        .size(8.8)
                                        .color(theme.text_muted),
                                );
                            }
                        }
                    } else if is_left {
                        ui.label(
                            RichText::new("Field QA/QC Scorecard & 3D Pour-Strip Overlay")
                                .size(10.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                        let sc = universal_canvas.field_qaqc_ledger.evaluate_scorecard();
                        ui.label(
                            RichText::new(format!(
                                "f'c Pass: {:.0}%  |  ITP: {}/{}  |  Open NCR: {}  |  MEP: {:.0}%",
                                sc.fc_pass_rate_pct,
                                sc.itp_cleared_count,
                                sc.itp_total_count,
                                sc.open_ncr_count,
                                sc.mep_tc_pass_pct,
                            ))
                            .size(9.0)
                            .monospace()
                            .color(if sc.open_ncr_count == 0 {
                                theme.accent_green
                            } else {
                                theme.accent_orange
                            }),
                        );
                        ui.horizontal_wrapped(|ui| {
                            ui.set_max_width(inner_w);
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                Some(AppIcon::Target),
                                if universal_canvas.field_qaqc_overlay_visible {
                                    "3D QA/QC ON"
                                } else {
                                    "3D QA/QC Overlay"
                                },
                                universal_canvas.field_qaqc_overlay_visible,
                                theme.accent_green,
                                "Toggle 3D Pure WebGPU Pour-Strip & NCR Waypoint Pin overlay",
                            )
                            .clicked()
                            {
                                actions.toggle_field_qaqc_3d = true;
                            }
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                Some(AppIcon::CheckCircle),
                                "Clear Hold",
                                false,
                                theme.accent_blue,
                                "Sign off the next pending ITP hold point or close open NCR",
                            )
                            .clicked()
                            {
                                actions.clear_waypoint_hold = Some(String::new());
                            }
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                Some(AppIcon::Plus),
                                "Log NCR",
                                false,
                                theme.accent_orange,
                                "Log a field NCR inspection waypoint onto active structural bay",
                            )
                            .clicked()
                            {
                                actions.log_field_ncr_quick = true;
                            }
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                Some(AppIcon::SaveDisk),
                                "QA/QC .ifcx",
                                false,
                                theme.accent_blue,
                                "Export eigengraph::construction::field_qaqc_bbs IFCX overlay",
                            )
                            .clicked()
                            {
                                actions.export_field_qaqc_ifcx = true;
                            }
                        });
                    } else {
                        ui.label(
                            RichText::new("Spatial Field Inspection Waypoints (ENU)")
                                .size(10.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                        if let Some(ref rep) = universal_canvas.last_field_fab_report {
                            for (idx, wp) in rep.waypoints.iter().take(4).enumerate() {
                                let [r, g, b] = wp.status.badge_color_rgb();
                                ui.horizontal_wrapped(|ui| {
                                    ui.set_max_width(inner_w);
                                    ui.colored_label(
                                        Color32::from_rgb(r, g, b),
                                        RichText::new(format!("[{}]", wp.status.short_label()))
                                            .size(8.4)
                                            .strong()
                                            .monospace(),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "{} @ {} ({:.1},{:.1},{:.1})m",
                                            wp.code,
                                            wp.element_ref,
                                            wp.local_x_m,
                                            wp.local_y_m,
                                            wp.local_z_m,
                                        ))
                                        .size(8.5)
                                        .monospace()
                                        .color(theme.text_secondary),
                                    );
                                    if Self::glass_pill_button(
                                        ui,
                                        theme,
                                        Some(AppIcon::Compass),
                                        "Fly",
                                        universal_canvas.active_waypoint_idx == idx
                                            && universal_canvas.field_qaqc_overlay_visible,
                                        theme.accent_blue,
                                        "Fly 3D Earth camera to this field inspection waypoint",
                                    )
                                    .clicked()
                                    {
                                        actions.fly_to_waypoint_idx = Some(idx);
                                    }
                                });
                            }
                        } else {
                            ui.label(
                                RichText::new("Toggle '3D QA/QC Overlay' or run /waypoint to anchor ENU pins.")
                                    .size(8.8)
                                    .color(theme.text_muted),
                            );
                        }
                    }
                });
            });
        });
    }

    /// Phase 83 (`v0.8.3`): Width-first Dynamic Island drawer for Branch 2 **EigenFlow** Non-Linear Studio
    /// (Woodbury `P1 -> P2` Load-Path Redistribution, Pushover Capacity Curve & ATC-40 ADRS Spectrum).
    pub(super) fn render_eigenflow_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        use crate::core::pushover::LateralLoadPattern;

        let deck_w = (cap_w - 8.0).max(180.0);
        let two_col = !is_mobile && deck_w >= 540.0;
        ui.set_width(deck_w);
        ui.set_max_width(deck_w);

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(deck_w);
            icons::icon_label(
                ui,
                AppIcon::Bolt,
                13.0,
                theme.accent_blue,
                "EigenFlow Non-Linear Studio",
                11.0,
                theme.text_primary,
                true,
            );
            for (idx, lbl) in [(0usize, "Woodbury P1->P2"), (1usize, "Pushover & ADRS")] {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    lbl,
                    self.eigenflow_drawer_tab == idx,
                    theme.accent_blue,
                    "",
                )
                .clicked()
                {
                    self.eigenflow_drawer_tab = idx;
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close EigenFlow Non-Linear Studio drawer",
            )
            .clicked()
            {
                self.eigenflow_drawer_open = false;
            }
        });

        ui.add_space(4.0);
        let tab = self.eigenflow_drawer_tab;
        // Top responsive control strip
        theme.glass_subpill_frame(8, Margin::symmetric(8, 5)).show(ui, |ui| {
            let strip_w = (deck_w - 18.0).max(150.0);
            ui.set_max_width(strip_w);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(strip_w);
                if tab == 0 {
                    ui.label(RichText::new("Damage lambda:").size(9.0).color(theme.text_muted));
                    for (lam, lbl) in [(0.0_f64, "0.00 Cut"), (0.25, "0.25 Severe"), (0.50, "0.50 Yield")] {
                        let active = (universal_canvas.eigenflow_config.stiffness_retention_lambda - lam).abs() < 0.05;
                        if Self::glass_pill_button(ui, theme, None, lbl, active, theme.accent_orange, "Set P1 residual stiffness lambda").clicked() {
                            actions.eigenflow_set_lambda = Some(lam);
                        }
                    }
                    if let Some(ref rep) = universal_canvas.last_eigenflow_report {
                        ui.label(RichText::new("P1:").size(9.0).color(theme.text_muted));
                        for (eid, ename) in rep.woodbury.available_elements.iter().take(4) {
                            let active = rep.woodbury.disrupted_element_id == *eid;
                            let short = format!("#{eid} {}", ename.chars().take(6).collect::<String>());
                            if Self::glass_pill_button(ui, theme, None, &short, active, theme.accent_red, &format!("Disrupt Element #{eid} ({ename})")).clicked() {
                                actions.eigenflow_select_element = Some(*eid);
                            }
                        }
                    }
                } else {
                    ui.label(RichText::new("Pattern:").size(9.0).color(theme.text_muted));
                    for (pat, lbl) in [
                        (LateralLoadPattern::FirstModeModal, "FirstMode"),
                        (LateralLoadPattern::UniformMass, "Uniform"),
                        (LateralLoadPattern::InvertedTriangular, "Triangular"),
                    ] {
                        let active = universal_canvas.eigenflow_config.load_pattern == pat;
                        if Self::glass_pill_button(ui, theme, None, lbl, active, theme.accent_blue, pat.label()).clicked() {
                            actions.eigenflow_set_pattern = Some(pat);
                        }
                    }
                    let n_steps = universal_canvas
                        .last_eigenflow_report
                        .as_ref()
                        .map(|r| r.pushover_csm.raw_pushover.steps.len().max(1))
                        .unwrap_or(12);
                    if Self::glass_pill_button(ui, theme, None, "Prev", false, theme.accent_blue, "Previous pushover step").clicked() {
                        universal_canvas.eigenflow_pushover_playing = false;
                        universal_canvas.eigenflow_config.active_step_idx =
                            universal_canvas.eigenflow_config.active_step_idx.saturating_sub(1);
                    }
                    let play_lbl = if universal_canvas.eigenflow_pushover_playing { "Pause" } else { "Play" };
                    if Self::glass_pill_button(ui, theme, None, play_lbl, universal_canvas.eigenflow_pushover_playing, theme.accent_green, "Animate Pushover steps & 3D hinges").clicked() {
                        universal_canvas.eigenflow_pushover_playing = !universal_canvas.eigenflow_pushover_playing;
                        if universal_canvas.eigenflow_pushover_playing {
                            universal_canvas.eigenflow_overlay_visible = true;
                        }
                    }
                    if Self::glass_pill_button(ui, theme, None, "Next", false, theme.accent_blue, "Next pushover step").clicked() {
                        universal_canvas.eigenflow_pushover_playing = false;
                        universal_canvas.eigenflow_config.active_step_idx =
                            (universal_canvas.eigenflow_config.active_step_idx + 1).min(n_steps.saturating_sub(1));
                    }
                    ui.label(
                        RichText::new(format!("Step {}/{}", universal_canvas.eigenflow_config.active_step_idx + 1, n_steps))
                            .size(8.8)
                            .monospace()
                            .color(theme.text_primary),
                    );
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Target),
                    if universal_canvas.eigenflow_overlay_visible { "EigenFlow 3D: ON" } else { "EigenFlow 3D" },
                    universal_canvas.eigenflow_overlay_visible,
                    theme.accent_blue,
                    "Toggle 3D Pure WebGPU P1->P2 conduits & Pushover hinges",
                ).clicked() {
                    actions.toggle_eigenflow_3d = true;
                }
                if Self::glass_pill_button(ui, theme, Some(AppIcon::Document), ".ifcx", false, theme.accent_blue, "Export eigengraph::eigenflow IFCX layer").clicked() {
                    actions.export_eigenflow_ifcx = true;
                }
                if Self::glass_pill_button(ui, theme, Some(AppIcon::Printer), "Sheet S-104 .svg", false, theme.accent_green, "Export A3 Sheet S-104 Pure-Vector EigenFlow Annex").clicked() {
                    actions.export_eigenflow_svg = true;
                }
            });
        });

        ui.add_space(4.0);
        Self::width_first_pair(ui, deck_w, two_col, |ui, is_left, col_w| {
            theme.glass_subpill_frame(9, Margin::symmetric(8, 6)).show(ui, |ui| {
                let inner_w = (col_w - 20.0).max(150.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.vertical(|ui| {
                    ui.set_max_width(inner_w);
                    let Some(ref rep) = universal_canvas.last_eigenflow_report else {
                        ui.label(RichText::new("Evaluating EigenFlow Non-Linear Studio...").size(9.5).color(theme.text_muted));
                        return;
                    };
                    if tab == 0 {
                        let wb = &rep.woodbury;
                        if is_left {
                            ui.label(RichText::new("EigenGate (Linear) vs EigenFlow (Non-Linear)").size(10.0).strong().color(theme.text_primary));
                            ui.label(
                                RichText::new(format!(
                                    "EigenGate Lin: {:.3}  |  EigenFlow NL: {:.3} ({:+.3})",
                                    rep.linear_eigengate_system,
                                    rep.non_linear_eigenflow_system,
                                    rep.delta_score_nl_minus_lin
                                ))
                                .size(9.2)
                                .monospace()
                                .strong()
                                .color(theme.accent_blue),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "P1 #{} ({}) | h_e={:.3} | m_s={:.3} | RRR={:.3} | mu={:.2} | Shed {:.1} kN",
                                    wb.disrupted_element_id,
                                    wb.disrupted_element_name,
                                    wb.svd_leverage_he,
                                    wb.stability_margin_ms,
                                    wb.residual_reserve_ratio_rrr,
                                    wb.max_ductility_mu,
                                    wb.total_shed_force_kn
                                ))
                                .size(8.8)
                                .monospace()
                                .color(theme.text_secondary),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(&wb.prescriptive_recommendation)
                                    .size(8.8)
                                    .color(if wb.is_mechanism { theme.accent_red } else { theme.text_primary }),
                            );
                        } else {
                            ui.label(RichText::new("Woodbury P1 -> P2 Surviving Shed Forces").size(10.0).strong().color(theme.text_primary));
                            let rows = wb.redistributed_p2.len().clamp(1, 6);
                            let chart_h = (rows as f32) * 20.0 + 10.0;
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(inner_w, chart_h), egui::Sense::hover());
                            let p = ui.painter_at(rect);
                            let max_dn = wb.redistributed_p2.iter().map(|m| m.delta_n_kn).fold(10.0_f64, f64::max) as f32;
                            let bar_x0 = rect.left() + 58.0;
                            let bar_w = (rect.width() - 66.0).max(36.0);
                            for (idx, m) in wb.redistributed_p2.iter().take(6).enumerate() {
                                let y = rect.top() + 4.0 + (idx as f32) * 20.0;
                                p.text(
                                    egui::pos2(rect.left() + 2.0, y + 7.0),
                                    egui::Align2::LEFT_CENTER,
                                    format!("#{}", m.element_id),
                                    egui::FontId::monospace(8.5),
                                    theme.text_secondary,
                                );
                                let track = egui::Rect::from_min_size(egui::pos2(bar_x0, y), Vec2::new(bar_w, 14.0));
                                p.rect_filled(track, 3.0, Color32::from_rgba_unmultiplied(128, 128, 128, 36));
                                let frac = (m.delta_n_kn as f32 / max_dn.max(1.0)).clamp(0.04, 1.0);
                                let fill_col = if m.is_critical_overload {
                                    theme.accent_red
                                } else if m.post_yield_dcr > 0.72 {
                                    theme.accent_orange
                                } else {
                                    theme.accent_blue
                                };
                                p.rect_filled(egui::Rect::from_min_size(egui::pos2(bar_x0, y), Vec2::new(bar_w * frac, 14.0)), 3.0, fill_col);
                                p.text(
                                    egui::pos2(bar_x0 + 4.0, y + 7.0),
                                    egui::Align2::LEFT_CENTER,
                                    format!("{:.1} kN ({:.0}% | DCR {:.2})", m.delta_n_kn, m.share_pct, m.post_yield_dcr),
                                    egui::FontId::monospace(8.0),
                                    Color32::WHITE,
                                );
                            }
                        }
                    } else if is_left {
                        let raw = &rep.pushover_csm.raw_pushover;
                        let perf = &rep.pushover_csm.performance_point;
                        ui.label(
                            RichText::new(format!("Pushover Curve (Vy={:.0} kN • Vu={:.0} kN)", raw.yield_base_shear_kn, raw.ultimate_base_shear_kn))
                                .size(9.6)
                                .strong()
                                .color(theme.text_primary),
                        );
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(inner_w, 118.0), egui::Sense::hover());
                        let p = ui.painter_at(rect);
                        p.rect_filled(rect, 5.0, Color32::from_rgba_unmultiplied(15, 23, 42, 42));
                        let max_d = raw.steps.iter().map(|s| s.roof_drift_mm).fold(perf.target_displacement_dt_mm, f64::max).max(5.0) * 1.1;
                        let max_v = raw.steps.iter().map(|s| s.base_shear_kn).fold(perf.target_base_shear_vt_kn, f64::max).max(10.0) * 1.15;
                        let to_pt = |d: f64, v: f64| -> egui::Pos2 {
                            egui::pos2(
                                rect.left() + 8.0 + ((d / max_d).clamp(0.0, 1.0) as f32) * (rect.width() - 16.0),
                                rect.bottom() - 8.0 - ((v / max_v).clamp(0.0, 1.0) as f32) * (rect.height() - 18.0),
                            )
                        };
                        let mut pts = vec![to_pt(0.0, 0.0)];
                        for st in &raw.steps {
                            pts.push(to_pt(st.roof_drift_mm, st.base_shear_kn));
                        }
                        if pts.len() >= 2 {
                            p.add(egui::Shape::line(pts, Stroke::new(2.0, theme.accent_blue)));
                        }
                        if let Some(st) = raw.steps.get(universal_canvas.eigenflow_config.active_step_idx) {
                            p.circle_filled(to_pt(st.roof_drift_mm, st.base_shear_kn), 4.5, theme.accent_green);
                        }
                    } else {
                        let perf = &rep.pushover_csm.performance_point;
                        ui.label(
                            RichText::new(format!("ADRS Spectrum (Sd*={:.1} mm • Sa*={:.3}g • Teff={:.2}s)", perf.sd_star_m * 1000.0, perf.sa_star_g, perf.effective_period_teff_s))
                                .size(9.5)
                                .strong()
                                .color(theme.text_primary),
                        );
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(inner_w, 118.0), egui::Sense::hover());
                        let p = ui.painter_at(rect);
                        p.rect_filled(rect, 5.0, Color32::from_rgba_unmultiplied(15, 23, 42, 42));
                        let cap_pts = &rep.pushover_csm.capacity_spectrum.points;
                        let red_pts = &rep.pushover_csm.demand_spectrum.reduced_inelastic_points;
                        let max_sd = cap_pts.iter().chain(red_pts.iter()).map(|pt| pt.sd_mm()).fold(perf.sd_star_m * 1000.0, f64::max).max(20.0) * 1.1;
                        let max_sa = cap_pts.iter().chain(red_pts.iter()).map(|pt| pt.sa_g).fold(perf.sa_star_g, f64::max).max(0.25) * 1.15;
                        let to_adrs = |sd_mm: f64, sa_g: f64| -> egui::Pos2 {
                            egui::pos2(
                                rect.left() + 8.0 + ((sd_mm / max_sd).clamp(0.0, 1.0) as f32) * (rect.width() - 16.0),
                                rect.bottom() - 8.0 - ((sa_g / max_sa).clamp(0.0, 1.0) as f32) * (rect.height() - 18.0),
                            )
                        };
                        let red_line: Vec<_> = red_pts.iter().map(|pt| to_adrs(pt.sd_mm(), pt.sa_g)).collect();
                        if red_line.len() >= 2 {
                            p.add(egui::Shape::line(red_line, Stroke::new(1.6, theme.accent_red)));
                        }
                        let cap_line: Vec<_> = cap_pts.iter().map(|pt| to_adrs(pt.sd_mm(), pt.sa_g)).collect();
                        if cap_line.len() >= 2 {
                            p.add(egui::Shape::line(cap_line, Stroke::new(2.0, theme.accent_blue)));
                        }
                        p.circle_filled(to_adrs(perf.sd_star_m * 1000.0, perf.sa_star_g), 5.0, theme.accent_green);
                        ui.label(RichText::new(perf.performance_level.label()).size(8.6).color(theme.accent_green));
                    }
                });
            });
        });
    }

    /// Phase 84 (`v0.8.4`): Width-first Dynamic Island drawer for Extreme Events Studio
    /// (Blast `UFC 3-340-02` Kingery-Bulmash + Structural Fire `ISO 834/EC` + EigenFlow Coupling).
    pub(super) fn render_extreme_events_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        use crate::core::blast::{BurstType, ExplosiveType};
        use crate::core::extreme_events::ExtremeEventMode;
        use crate::core::fire::{FireCurveKind, FireProtectionKind};

        let deck_w = (cap_w - 8.0).max(180.0);
        let two_col = !is_mobile && deck_w >= 540.0;
        ui.set_width(deck_w);
        ui.set_max_width(deck_w);

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(deck_w);
            icons::icon_label(
                ui,
                AppIcon::AlertTriangle,
                13.0,
                theme.accent_orange,
                "Extreme Events (Blast & Fire)",
                11.0,
                theme.text_primary,
                true,
            );
            for (idx, lbl, mode) in [
                (0usize, "Blast UFC 3-340-02", ExtremeEventMode::Blast),
                (1usize, "Structural Fire ISO 834", ExtremeEventMode::StructuralFire),
            ] {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    lbl,
                    self.extreme_events_drawer_tab == idx,
                    theme.accent_orange,
                    "",
                )
                .clicked()
                {
                    self.extreme_events_drawer_tab = idx;
                    if universal_canvas.extreme_events_config.mode != ExtremeEventMode::Combined {
                        universal_canvas.extreme_events_config.mode = mode;
                    }
                    actions.run_extreme_events = true;
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close Extreme Events drawer",
            )
            .clicked()
            {
                self.extreme_events_drawer_open = false;
            }
        });

        ui.add_space(4.0);
        let tab = self.extreme_events_drawer_tab;
        theme.glass_subpill_frame(8, Margin::symmetric(8, 5)).show(ui, |ui| {
            let strip_w = (deck_w - 18.0).max(150.0);
            ui.set_max_width(strip_w);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(strip_w);
                if tab == 0 {
                    for (ex, lbl) in [
                        (ExplosiveType::Tnt, "TNT"),
                        (ExplosiveType::Anfo, "ANFO"),
                        (ExplosiveType::C4, "C-4"),
                    ] {
                        let active = universal_canvas.extreme_events_config.explosive_type == ex;
                        if Self::glass_pill_button(ui, theme, None, lbl, active, theme.accent_red, ex.label()).clicked() {
                            universal_canvas.extreme_events_config.explosive_type = ex;
                            universal_canvas.extreme_events_config.mode = ExtremeEventMode::Blast;
                            actions.run_extreme_events = true;
                        }
                    }
                    for (kg, lbl) in [(100.0_f64, "100kg"), (250.0, "250kg"), (500.0, "500kg")] {
                        let active = (universal_canvas.extreme_events_config.explosive_mass_kg - kg).abs() < 10.0;
                        if Self::glass_pill_button(ui, theme, None, lbl, active, theme.accent_orange, "Charge mass W").clicked() {
                            universal_canvas.extreme_events_config.explosive_mass_kg = kg;
                            actions.run_extreme_events = true;
                        }
                    }
                    for (r_m, lbl) in [(8.0_f64, "R 8m"), (12.0, "R 12m"), (20.0, "R 20m")] {
                        let active = (universal_canvas.extreme_events_config.standoff_distance_m - r_m).abs() < 1.0;
                        if Self::glass_pill_button(ui, theme, None, lbl, active, theme.accent_blue, "Standoff distance R").clicked() {
                            universal_canvas.extreme_events_config.standoff_distance_m = r_m;
                            actions.run_extreme_events = true;
                        }
                    }
                    let burst_lbl = match universal_canvas.extreme_events_config.burst_type {
                        BurstType::SurfaceBurst => "Surface",
                        BurstType::FreeAirBurst => "FreeAir",
                    };
                    if Self::glass_pill_button(ui, theme, None, burst_lbl, false, theme.accent_blue, "Toggle Surface / FreeAir burst").clicked() {
                        universal_canvas.extreme_events_config.burst_type = match universal_canvas.extreme_events_config.burst_type {
                            BurstType::SurfaceBurst => BurstType::FreeAirBurst,
                            BurstType::FreeAirBurst => BurstType::SurfaceBurst,
                        };
                        actions.run_extreme_events = true;
                    }
                } else {
                    for (cv, lbl) in [
                        (FireCurveKind::Iso834Standard, "ISO 834"),
                        (FireCurveKind::AstmE119, "ASTM E119"),
                        (FireCurveKind::EurocodeHydrocarbon, "HC Rapid"),
                    ] {
                        let active = universal_canvas.extreme_events_config.fire_curve == cv;
                        if Self::glass_pill_button(ui, theme, None, lbl, active, theme.accent_red, cv.label()).clicked() {
                            universal_canvas.extreme_events_config.fire_curve = cv;
                            universal_canvas.extreme_events_config.mode = ExtremeEventMode::StructuralFire;
                            actions.run_extreme_events = true;
                        }
                    }
                    for (pr, lbl) in [
                        (FireProtectionKind::Unprotected, "Bare"),
                        (FireProtectionKind::SprayedMineralFiber, "SFRM"),
                        (FireProtectionKind::IntumescentCoating, "Intum"),
                    ] {
                        let active = universal_canvas.extreme_events_config.protection_kind == pr;
                        if Self::glass_pill_button(ui, theme, None, lbl, active, theme.accent_orange, pr.label()).clicked() {
                            universal_canvas.extreme_events_config.protection_kind = pr;
                            actions.run_extreme_events = true;
                        }
                    }
                    for st_idx in 0..3usize {
                        let active = universal_canvas.extreme_events_config.fire_story_idx == st_idx;
                        if Self::glass_pill_button(ui, theme, None, &format!("L{}", st_idx + 1), active, theme.accent_blue, "Select ignited story").clicked() {
                            universal_canvas.extreme_events_config.fire_story_idx = st_idx;
                            actions.run_extreme_events = true;
                        }
                    }
                    if Self::glass_pill_button(ui, theme, None, "-15m", false, theme.accent_blue, "Step fire exposure -15 min").clicked() {
                        universal_canvas.fire_sim_playing = false;
                        universal_canvas.extreme_events_config.exposure_minutes =
                            (universal_canvas.extreme_events_config.exposure_minutes - 15.0).max(0.0);
                        actions.run_extreme_events = true;
                    }
                    let fplay = if universal_canvas.fire_sim_playing { "Pause" } else { "Play" };
                    if Self::glass_pill_button(ui, theme, None, fplay, universal_canvas.fire_sim_playing, theme.accent_green, "Animate 0..180 min fire curve").clicked() {
                        universal_canvas.fire_sim_playing = !universal_canvas.fire_sim_playing;
                        if universal_canvas.fire_sim_playing {
                            universal_canvas.extreme_events_overlay_visible = true;
                            universal_canvas.extreme_events_config.mode = ExtremeEventMode::StructuralFire;
                        }
                    }
                    if Self::glass_pill_button(ui, theme, None, "+15m", false, theme.accent_blue, "Step fire exposure +15 min").clicked() {
                        universal_canvas.fire_sim_playing = false;
                        universal_canvas.extreme_events_config.exposure_minutes =
                            (universal_canvas.extreme_events_config.exposure_minutes + 15.0).min(180.0);
                        actions.run_extreme_events = true;
                    }
                    ui.label(
                        RichText::new(format!("{:.0} min", universal_canvas.extreme_events_config.exposure_minutes))
                            .size(8.8)
                            .monospace()
                            .color(theme.text_primary),
                    );
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Target),
                    if universal_canvas.extreme_events_overlay_visible { "Extreme 3D: ON" } else { "Extreme 3D" },
                    universal_canvas.extreme_events_overlay_visible,
                    theme.accent_blue,
                    "Toggle 3D Pure WebGPU Blast shockwave & Fire compartment plume",
                )
                .clicked()
                {
                    actions.toggle_extreme_events_3d = true;
                }
                if Self::glass_pill_button(ui, theme, Some(AppIcon::Document), ".ifcx", false, theme.accent_blue, "Export eigengraph::hazard::extreme_events IFCX").clicked() {
                    actions.export_extreme_events_ifcx = true;
                }
                if Self::glass_pill_button(ui, theme, Some(AppIcon::Printer), "Sheet S-105 .svg", false, theme.accent_green, "Export A3 Sheet S-105 Extreme Events Annex SVG").clicked() {
                    actions.export_extreme_events_svg = true;
                }
            });
        });

        ui.add_space(4.0);
        Self::width_first_pair(ui, deck_w, two_col, |ui, is_left, col_w| {
            theme.glass_subpill_frame(9, Margin::symmetric(8, 6)).show(ui, |ui| {
                let inner_w = (col_w - 20.0).max(150.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.vertical(|ui| {
                    ui.set_max_width(inner_w);
                    let Some(ref rep) = universal_canvas.last_extreme_events_report else {
                        ui.label(RichText::new("Evaluating Extreme Events...").size(9.5).color(theme.text_muted));
                        return;
                    };
                    if tab == 0 {
                        let bw = &rep.blast_report.blast_wave;
                        let be = &rep.blast_eigenflow;
                        if is_left {
                            ui.label(RichText::new("Blast Wave & Closed-Loop EigenFlow Audit").size(10.0).strong().color(theme.text_primary));
                            ui.label(
                                RichText::new(format!(
                                    "Z={:.2} | Pso={:.0} kPa | Pr={:.0} kPa | ir={:.0} kPa-ms | mu={:.2}",
                                    bw.scaled_distance_z,
                                    bw.peak_incident_overpressure_kpa,
                                    bw.peak_reflected_overpressure_kpa,
                                    bw.reflected_impulse_kpa_ms,
                                    rep.blast_report.dynamic_ductility_ratio
                                ))
                                .size(9.0)
                                .monospace()
                                .color(theme.accent_orange),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "Damage: {} | P1 #{} (lam={:.2}) -> Woodbury m_s={:.3}, EigenFlow {:.3}",
                                    rep.worst_blast_damage.label(),
                                    be.critical_p1_element_id,
                                    be.derived_lambda,
                                    be.woodbury_ms,
                                    be.eigenflow_score
                                ))
                                .size(8.8)
                                .monospace()
                                .color(if be.is_progressive_collapse_risk { theme.accent_red } else { theme.accent_green }),
                            );
                            ui.add_space(4.0);
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                Some(AppIcon::Bolt),
                                "Send P1 to EigenFlow Studio",
                                false,
                                theme.accent_blue,
                                "Open EigenFlow Woodbury P1->P2 with blast-damaged stiffness ratio lambda",
                            )
                            .clicked()
                            {
                                actions.extreme_send_to_eigenflow =
                                    Some((be.critical_p1_element_id, be.derived_lambda));
                            }
                        } else {
                            ui.label(RichText::new("Blast Pressure P(t) & Midspan SDOF Response y(t)").size(9.6).strong().color(theme.text_primary));
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(inner_w, 118.0), egui::Sense::hover());
                            let p = ui.painter_at(rect);
                            p.rect_filled(rect, 5.0, Color32::from_rgba_unmultiplied(15, 23, 42, 42));
                            let th = &rep.blast_report.time_history_samples;
                            let max_t = th.iter().map(|s| s.time_ms).fold(1.0_f64, f64::max).max(1.0);
                            let max_p = th.iter().map(|s| s.blast_pressure_kpa.max(0.0)).fold(bw.peak_reflected_overpressure_kpa, f64::max).max(10.0);
                            let max_y = th.iter().map(|s| s.displacement_mm.abs()).fold(1.0_f64, f64::max);
                            let to_p = |t: f64, v: f64, vmax: f64| -> egui::Pos2 {
                                egui::pos2(
                                    rect.left() + 8.0 + ((t / max_t).clamp(0.0, 1.0) as f32) * (rect.width() - 16.0),
                                    rect.bottom() - 8.0 - ((v / vmax).clamp(0.0, 1.0) as f32) * (rect.height() - 18.0),
                                )
                            };
                            let p_pts: Vec<_> = th.iter().map(|s| to_p(s.time_ms, s.blast_pressure_kpa.max(0.0), max_p)).collect();
                            if p_pts.len() >= 2 {
                                p.add(egui::Shape::line(p_pts, Stroke::new(2.0, theme.accent_red)));
                            }
                            let y_pts: Vec<_> = th.iter().map(|s| to_p(s.time_ms, s.displacement_mm.abs(), max_y)).collect();
                            if y_pts.len() >= 2 {
                                p.add(egui::Shape::line(y_pts, Stroke::new(1.8, theme.accent_blue)));
                            }
                        }
                    } else {
                        let fc = &rep.fire_compartment;
                        let fe = &rep.fire_eigenflow;
                        if is_left {
                            ui.label(RichText::new("Thermal Fire Endurance & EigenFlow Stability").size(10.0).strong().color(theme.text_primary));
                            ui.label(
                                RichText::new(format!(
                                    "{} @ {:.0} min: Tg={:.0}C | Tm={:.0}C | kE={:.2} | ky={:.2} | FRR={:.0} min",
                                    fc.story_label,
                                    rep.config.exposure_minutes,
                                    fc.gas_temp_c,
                                    fc.member_temp_c,
                                    fc.ke_factor,
                                    fc.ky_factor,
                                    fc.frr_minutes
                                ))
                                .size(9.0)
                                .monospace()
                                .color(theme.accent_orange),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "Thermal P1 #{} (lam=kE={:.2}) -> m_s={:.3}, EigenFlow {:.3}",
                                    fe.critical_p1_element_id,
                                    fe.derived_lambda,
                                    fe.woodbury_ms,
                                    fe.eigenflow_score
                                ))
                                .size(8.8)
                                .monospace()
                                .color(if fe.is_progressive_collapse_risk { theme.accent_red } else { theme.accent_green }),
                            );
                            ui.add_space(4.0);
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                Some(AppIcon::Bolt),
                                "Check Thermal EigenFlow",
                                false,
                                theme.accent_blue,
                                "Open EigenFlow Studio with thermal stiffness retention lambda = kE(T)",
                            )
                            .clicked()
                            {
                                actions.extreme_send_to_eigenflow =
                                    Some((fe.critical_p1_element_id, fe.derived_lambda));
                            }
                        } else {
                            ui.label(RichText::new("Gas/Core Temp Tg(t)/Tm(t) & Stiffness kE(T)").size(9.6).strong().color(theme.text_primary));
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(inner_w, 118.0), egui::Sense::hover());
                            let p = ui.painter_at(rect);
                            p.rect_filled(rect, 5.0, Color32::from_rgba_unmultiplied(15, 23, 42, 42));
                            let fs = &rep.fire_report.time_history_samples;
                            let max_t = fs.iter().map(|s| s.time_minutes).fold(180.0_f64, f64::max).max(30.0);
                            let max_temp = fs.iter().map(|s| s.gas_temp_c).fold(1100.0_f64, f64::max).max(600.0);
                            let to_ft = |t: f64, val: f64, vmax: f64| -> egui::Pos2 {
                                egui::pos2(
                                    rect.left() + 8.0 + ((t / max_t).clamp(0.0, 1.0) as f32) * (rect.width() - 16.0),
                                    rect.bottom() - 8.0 - ((val / vmax).clamp(0.0, 1.0) as f32) * (rect.height() - 18.0),
                                )
                            };
                            let tg_line: Vec<_> = fs.iter().map(|s| to_ft(s.time_minutes, s.gas_temp_c, max_temp)).collect();
                            if tg_line.len() >= 2 {
                                p.add(egui::Shape::line(tg_line, Stroke::new(1.8, theme.accent_red)));
                            }
                            let tm_line: Vec<_> = fs.iter().map(|s| to_ft(s.time_minutes, s.member_temp_c, max_temp)).collect();
                            if tm_line.len() >= 2 {
                                p.add(egui::Shape::line(tm_line, Stroke::new(1.8, theme.accent_orange)));
                            }
                            let ke_line: Vec<_> = fs.iter().map(|s| to_ft(s.time_minutes, s.ke_factor, 1.0)).collect();
                            if ke_line.len() >= 2 {
                                p.add(egui::Shape::line(ke_line, Stroke::new(1.6, theme.accent_blue)));
                            }
                            let mx = rect.left() + 8.0 + ((rep.config.exposure_minutes / max_t).clamp(0.0, 1.0) as f32) * (rect.width() - 16.0);
                            p.line_segment([egui::pos2(mx, rect.top() + 4.0), egui::pos2(mx, rect.bottom() - 4.0)], Stroke::new(1.3, theme.accent_green));
                        }
                    }
                });
            });
        });
    }

    /// Update `v0.8.4.1` (`1/10` -> `v0.8.5`): Width-first Dynamic Island drawer for Seismic Base Isolation & Damper Studio.
    pub(super) fn render_isolation_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        use crate::core::isolation::IsolatorType;

        let deck_w = (cap_w - 8.0).max(180.0);
        let two_col = !is_mobile && deck_w >= 540.0;
        ui.set_width(deck_w);
        ui.set_max_width(deck_w);

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(deck_w);
            icons::icon_label(
                ui,
                AppIcon::Shield,
                13.0,
                theme.accent_blue,
                "Seismic Base Isolation & Damper Studio",
                11.0,
                theme.text_primary,
                true,
            );
            for (idx, lbl) in [(0usize, "Isolator Sizing & Moat"), (1usize, "Hysteresis & ASCE/SNI Audit")] {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    lbl,
                    self.isolation_drawer_tab == idx,
                    theme.accent_blue,
                    "",
                )
                .clicked()
                {
                    self.isolation_drawer_tab = idx;
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close Base Isolation Studio drawer",
            )
            .clicked()
            {
                self.isolation_drawer_open = false;
            }
        });

        ui.add_space(4.0);
        theme.glass_subpill_frame(8, Margin::symmetric(8, 5)).show(ui, |ui| {
            let strip_w = (deck_w - 18.0).max(150.0);
            ui.set_max_width(strip_w);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(strip_w);
                for (iso, lbl) in [
                    (IsolatorType::LeadRubberBearing, "LRB"),
                    (IsolatorType::HighDampingRubberBearing, "HDRB"),
                    (IsolatorType::FrictionPendulum, "FPS"),
                    (IsolatorType::FluidViscousDamper, "FVD"),
                ] {
                    let active = universal_canvas.isolation_studio_config.isolator_type == iso;
                    if Self::glass_pill_button(ui, theme, None, lbl, active, theme.accent_blue, "Select isolator / damper system").clicked() {
                        universal_canvas.isolation_studio_config.isolator_type = iso;
                        actions.run_isolation_studio = true;
                    }
                }
                for (d_mm, lbl) in [(700.0_f64, "D 700mm"), (850.0, "D 850mm"), (1050.0, "D 1050mm")] {
                    let active = (universal_canvas.isolation_studio_config.lrb_outer_diameter_mm - d_mm).abs() < 40.0;
                    if Self::glass_pill_button(ui, theme, None, lbl, active, theme.accent_orange, "Bearing outer diameter").clicked() {
                        universal_canvas.isolation_studio_config.lrb_outer_diameter_mm = d_mm;
                        actions.run_isolation_studio = true;
                    }
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Target),
                    if universal_canvas.isolation_overlay_visible { "Isolation 3D: ON" } else { "Isolation 3D" },
                    universal_canvas.isolation_overlay_visible,
                    theme.accent_blue,
                    "Toggle 3D Pure WebGPU isolator pads, moat ring & FVD dampers",
                )
                .clicked()
                {
                    actions.toggle_isolation_3d = true;
                }
                if Self::glass_pill_button(ui, theme, Some(AppIcon::Document), ".ifcx", false, theme.accent_blue, "Export eigengraph::analysis::seismic_isolation IFCX").clicked() {
                    actions.export_isolation_ifcx = true;
                }
                if Self::glass_pill_button(ui, theme, Some(AppIcon::Printer), "Sheet S-106 .svg", false, theme.accent_green, "Export A3 Sheet S-106 Base Isolation Annex SVG").clicked() {
                    actions.export_isolation_svg = true;
                }
            });
        });

        ui.add_space(4.0);
        let tab = self.isolation_drawer_tab;
        Self::width_first_pair(ui, deck_w, two_col, |ui, is_left, col_w| {
            theme.glass_subpill_frame(9, Margin::symmetric(8, 6)).show(ui, |ui| {
                let inner_w = (col_w - 20.0).max(150.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.vertical(|ui| {
                    ui.set_max_width(inner_w);
                    let Some(ref studio) = universal_canvas.last_isolation_report else {
                        ui.label(RichText::new("Evaluating Base Isolation Studio...").size(9.5).color(theme.text_muted));
                        return;
                    };
                    let r = &studio.report;
                    if tab == 0 {
                        if is_left {
                            ui.label(RichText::new("Fixed-Base vs Base-Isolated Performance").size(10.0).strong().color(theme.text_primary));
                            ui.label(
                                RichText::new(format!(
                                    "T_fix {:.2}s -> T_M {:.2}s (x{:.2}) | Shear -{:.0}% ({:.0}->{:.0} kN)",
                                    studio.input_used.fixed_base_period_s,
                                    r.effective_period_tm_s,
                                    studio.period_shift_ratio,
                                    studio.shear_reduction_pct,
                                    studio.fixed_base_shear_kn,
                                    r.superstructure_design_base_shear_vs_kn,
                                ))
                                .size(9.0)
                                .monospace()
                                .color(theme.accent_blue),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "DM={:.0} mm | DTM={:.0} mm | Moat={:.0} mm | Beta={:.1}% | Buckling SF={:.2}",
                                    r.max_displacement_dm_mm,
                                    r.total_max_displacement_dtm_mm,
                                    r.required_moat_gap_mm,
                                    r.combined_total_damping * 100.0,
                                    r.buckling_safety_factor
                                ))
                                .size(8.8)
                                .monospace()
                                .color(if r.is_design_acceptable { theme.accent_green } else { theme.accent_red }),
                            );
                        } else {
                            ui.label(RichText::new("Isolator Bearing & Seismic Moat Schedule").size(10.0).strong().color(theme.text_primary));
                            ui.label(
                                RichText::new(format!(
                                    "{} Base Pads ({:.0} kN/pad) • {} Perimeter FVD Struts • Moat {:.2} m",
                                    studio.pads_3d.len(),
                                    studio.input_used.axial_load_per_bearing_kn,
                                    studio.dampers_3d.len(),
                                    studio.moat_3d.moat_gap_m,
                                ))
                                .size(8.8)
                                .monospace()
                                .color(theme.text_secondary),
                            );
                            for pad in studio.pads_3d.iter().take(3) {
                                ui.label(
                                    RichText::new(format!(
                                        "B{} {} ({:.1},{:.1})m • D={:.2}m • SF={:.2} [{}]",
                                        pad.id,
                                        pad.label,
                                        pad.pos_enu_m[0],
                                        pad.pos_enu_m[1],
                                        pad.diameter_m,
                                        pad.buckling_sf,
                                        if pad.is_safe { "SAFE" } else { "CHECK" },
                                    ))
                                    .size(8.5)
                                    .monospace()
                                    .color(if pad.is_safe { theme.accent_green } else { theme.accent_red }),
                                );
                            }
                        }
                    } else if is_left {
                        ui.label(
                            RichText::new(format!("Bilinear Hysteresis Loop (ED={:.1} kJ/cycle)", r.energy_dissipated_per_cycle_kj))
                                .size(9.6)
                                .strong()
                                .color(theme.text_primary),
                        );
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(inner_w, 118.0), egui::Sense::hover());
                        let p = ui.painter_at(rect);
                        p.rect_filled(rect, 5.0, Color32::from_rgba_unmultiplied(15, 23, 42, 42));
                        let max_d = r.hysteresis_loop.iter().map(|pt| pt.displacement_mm.abs()).fold(r.total_max_displacement_dtm_mm, f64::max).max(40.0) * 1.1;
                        let max_f = r.hysteresis_loop.iter().map(|pt| pt.lateral_force_kn.abs()).fold(100.0_f64, f64::max) * 1.15;
                        let pts: Vec<_> = r
                            .hysteresis_loop
                            .iter()
                            .map(|pt| {
                                egui::pos2(
                                    rect.left() + 8.0 + (((pt.displacement_mm / max_d) * 0.5 + 0.5).clamp(0.0, 1.0) as f32) * (rect.width() - 16.0),
                                    rect.bottom() - 8.0 - (((pt.lateral_force_kn / max_f) * 0.5 + 0.5).clamp(0.0, 1.0) as f32) * (rect.height() - 16.0),
                                )
                            })
                            .collect();
                        if pts.len() >= 2 {
                            p.add(egui::Shape::line(pts, Stroke::new(2.0, theme.accent_blue)));
                        }
                    } else {
                        ui.label(
                            RichText::new(format!("ASCE 7-22 / SNI 1726 Compliance (D/C {:.2})", r.governing_demand_capacity_ratio))
                                .size(9.6)
                                .strong()
                                .color(theme.text_primary),
                        );
                        for chk in r.regulatory_checks.iter().take(4) {
                            ui.label(
                                RichText::new(format!(
                                    "[{}] {} ({:.2}/{:.2} {})",
                                    if chk.is_passed { "PASS" } else { "FAIL" },
                                    chk.code_clause,
                                    chk.demand_val,
                                    chk.capacity_val,
                                    chk.unit,
                                ))
                                .size(8.5)
                                .monospace()
                                .color(if chk.is_passed { theme.accent_green } else { theme.accent_red }),
                            );
                        }
                    }
                });
            });
        });
    }

    /// Update `v0.8.4.2` (`2/10` -> `v0.8.5`): Width-first Dynamic Island drawer for Wind Aeroelasticity & ISO 10137 Comfort Studio.
    pub(super) fn render_wind_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        use crate::core::wind::{EnclosureClassification, ExposureCategory};

        let deck_w = (cap_w - 8.0).max(180.0);
        let two_col = !is_mobile && deck_w >= 540.0;
        ui.set_width(deck_w);
        ui.set_max_width(deck_w);

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(deck_w);
            icons::icon_label(
                ui,
                AppIcon::Wind,
                13.0,
                theme.accent_blue,
                "Wind Aeroelasticity & Comfort",
                11.0,
                theme.text_primary,
                true,
            );
            for (idx, lbl) in [
                (0usize, "Wind Profile & Cladding"),
                (1usize, "ISO 10137 Comfort & Envelope"),
            ] {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    lbl,
                    self.wind_drawer_tab == idx,
                    theme.accent_blue,
                    "",
                )
                .clicked()
                {
                    self.wind_drawer_tab = idx;
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close Wind Aeroelasticity Studio drawer",
            )
            .clicked()
            {
                self.wind_drawer_open = false;
            }
        });

        ui.add_space(4.0);
        theme.glass_subpill_frame(8, Margin::symmetric(8, 5)).show(ui, |ui| {
            let strip_w = (deck_w - 18.0).max(150.0);
            ui.set_max_width(strip_w);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(strip_w);
                for (exp, lbl) in [
                    (ExposureCategory::ExposureB, "Exp B"),
                    (ExposureCategory::ExposureC, "Exp C"),
                    (ExposureCategory::ExposureD, "Exp D"),
                ] {
                    let active = universal_canvas.wind_studio_config.exposure == exp;
                    if Self::glass_pill_button(ui, theme, None, lbl, active, theme.accent_blue, "ASCE 7-22 Terrain Exposure Category").clicked() {
                        universal_canvas.wind_studio_config.exposure = exp;
                        actions.run_wind_studio = true;
                    }
                }
                for (enc, lbl) in [
                    (EnclosureClassification::Enclosed, "Enclosed"),
                    (EnclosureClassification::PartiallyEnclosed, "Partial"),
                ] {
                    let active = universal_canvas.wind_studio_config.enclosure == enc;
                    if Self::glass_pill_button(ui, theme, None, lbl, active, theme.accent_orange, "Building enclosure classification").clicked() {
                        universal_canvas.wind_studio_config.enclosure = enc;
                        actions.run_wind_studio = true;
                    }
                }
                let occ_lbl = if universal_canvas.wind_studio_config.occupancy_residential {
                    "Residential (15mg)"
                } else {
                    "Office (20mg)"
                };
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    occ_lbl,
                    universal_canvas.wind_studio_config.occupancy_residential,
                    theme.accent_blue,
                    "Toggle ISO 10137 occupant comfort limit (15 mg Residential vs 20 mg Office)",
                )
                .clicked()
                {
                    universal_canvas.wind_studio_config.occupancy_residential =
                        !universal_canvas.wind_studio_config.occupancy_residential;
                    actions.run_wind_studio = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Layers),
                    if universal_canvas.wind_overlay_visible { "Wind 3D: ON" } else { "Wind 3D" },
                    universal_canvas.wind_overlay_visible,
                    theme.accent_blue,
                    "Toggle 3D Pure WebGPU facade cladding pressure & wind flow vectors",
                )
                .clicked()
                {
                    actions.toggle_wind_3d = true;
                }
                if Self::glass_pill_button(ui, theme, Some(AppIcon::Document), ".ifcx", false, theme.accent_blue, "Export eigengraph::analysis::wind_aeroelasticity IFCX").clicked() {
                    actions.export_wind_ifcx = true;
                }
                if Self::glass_pill_button(ui, theme, Some(AppIcon::Printer), "Sheet S-107 .svg", false, theme.accent_green, "Export A3 Sheet S-107 Wind & ISO 10137 Annex SVG").clicked() {
                    actions.export_wind_svg = true;
                }
            });
            ui.add_space(3.0);
            ui.spacing_mut().slider_width = (strip_w * 0.28).clamp(58.0, 96.0);
            let render_sliders = |ui: &mut Ui| {
                ui.horizontal(|ui| {
                    let mut spd = universal_canvas.wind_studio_config.basic_wind_speed_ms;
                    ui.label(RichText::new("Speed V (m/s)").size(9.0).color(theme.text_secondary));
                    if ui.add(egui::Slider::new(&mut spd, 20.0..=90.0).max_decimals(0)).changed() {
                        universal_canvas.wind_studio_config.basic_wind_speed_ms = spd;
                        actions.run_wind_studio = true;
                    }
                });
                ui.horizontal(|ui| {
                    let mut az = universal_canvas.wind_studio_config.wind_azimuth_deg;
                    ui.label(RichText::new("Azimuth (deg)").size(9.0).color(theme.text_secondary));
                    if ui.add(egui::Slider::new(&mut az, 0.0..=360.0).max_decimals(0)).changed() {
                        universal_canvas.wind_studio_config.wind_azimuth_deg = az;
                        actions.run_wind_studio = true;
                    }
                });
            };
            if strip_w >= 640.0 {
                ui.horizontal(render_sliders);
            } else {
                ui.vertical(render_sliders);
            }
        });

        ui.add_space(4.0);
        let tab = self.wind_drawer_tab;
        Self::width_first_pair(ui, deck_w, two_col, |ui, is_left, col_w| {
            theme.glass_subpill_frame(9, Margin::symmetric(8, 6)).show(ui, |ui| {
                let inner_w = (col_w - 20.0).max(150.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.vertical(|ui| {
                    ui.set_max_width(inner_w);
                    let Some(ref studio) = universal_canvas.last_wind_report else {
                        ui.label(RichText::new("Evaluating Wind Aeroelasticity Studio...").size(9.5).color(theme.text_muted));
                        return;
                    };
                    let r = &studio.report;
                    let c = &r.comfort_audit;
                    if tab == 0 {
                        if is_left {
                            ui.label(RichText::new("Velocity Pressure qz(z) & Net MWFRS Profile").size(9.8).strong().color(theme.text_primary));
                            ui.label(
                                RichText::new(format!(
                                    "V={:.1} m/s ({:.0} deg) | qh={:.0} Pa | Gf={:.3} | Vw={:.1} kN",
                                    r.basic_wind_speed_v_ms,
                                    studio.config.wind_azimuth_deg,
                                    r.qh_mean_roof_velocity_pressure_pa,
                                    c.gust_effect_factor_g,
                                    r.total_base_shear_wind_kn,
                                ))
                                .size(8.8)
                                .monospace()
                                .color(theme.accent_blue),
                            );
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(inner_w, 108.0), egui::Sense::hover());
                            let p = ui.painter_at(rect);
                            p.rect_filled(rect, 5.0, Color32::from_rgba_unmultiplied(15, 23, 42, 42));
                            let max_p = r.story_levels.iter().map(|l| l.net_mwfrs_pressure_pa.max(l.qz_velocity_pressure_pa)).fold(200.0_f64, f64::max) * 1.1;
                            let n = r.story_levels.len().max(1);
                            let row_h = ((rect.height() - 12.0) / n as f32).clamp(4.0, 20.0);
                            for (idx, lvl) in r.story_levels.iter().take(12).enumerate() {
                                let y = rect.bottom() - 6.0 - (idx as f32 + 1.0) * row_h;
                                let bar_w = ((lvl.net_mwfrs_pressure_pa / max_p).clamp(0.0, 1.0) as f32) * (rect.width() - 18.0);
                                p.rect_filled(
                                    Rect::from_min_size(egui::pos2(rect.left() + 8.0, y + 1.0), Vec2::new(bar_w, (row_h - 2.0).max(2.0))),
                                    2.0,
                                    theme.accent_blue,
                                );
                            }
                        } else {
                            ui.label(RichText::new("Facade Cladding Pressures (Top Story)").size(9.8).strong().color(theme.text_primary));
                            let top_story = r.story_levels.last().map(|l| l.story_num).unwrap_or(1);
                            for pan in studio.panels_3d.iter().filter(|p| p.story_num == top_story).take(4) {
                                ui.label(
                                    RichText::new(format!(
                                        "{}: {:+.0} Pa ({:.1}x{:.1} m)",
                                        pan.face_label, pan.pressure_pa, pan.width_m, pan.height_m
                                    ))
                                    .size(8.6)
                                    .monospace()
                                    .color(if pan.is_windward { theme.accent_blue } else { theme.accent_orange }),
                                );
                            }
                        }
                    } else if is_left {
                        ui.label(RichText::new("ISO 10137 Occupant Comfort Acceleration").size(9.8).strong().color(theme.text_primary));
                        ui.label(
                            RichText::new(format!(
                                "f1={:.3} Hz (T1={:.2} s) | Along={:.2} mg | Cross={:.2} mg",
                                c.fundamental_frequency_hz,
                                c.fundamental_period_t1_s,
                                c.peak_acceleration_along_wind_mg,
                                c.peak_acceleration_cross_wind_mg,
                            ))
                            .size(8.8)
                            .monospace()
                            .color(theme.accent_blue),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Resultant a_res = {:.2} mg vs Limit {:.1} mg [{}]",
                                c.resultant_acceleration_mg,
                                studio.comfort_limit_mg,
                                if studio.comfort_acceptable { "COMPLIANT" } else { "REVIEW" },
                            ))
                            .size(8.8)
                            .monospace()
                            .color(if studio.comfort_acceptable { theme.accent_green } else { theme.accent_red }),
                        );
                    } else {
                        ui.label(RichText::new("Wind vs Seismic Governing Lateral Envelope").size(9.8).strong().color(theme.text_primary));
                        ui.label(
                            RichText::new(format!(
                                "Wind Vw={:.0} kN vs Seismic Vs={:.0} kN -> {}",
                                r.total_base_shear_wind_kn,
                                r.seismic_base_shear_ref_kn,
                                if r.wind_governs_lateral_system { "WIND GOVERNS" } else { "SEISMIC GOVERNS" },
                            ))
                            .size(8.8)
                            .monospace()
                            .color(if r.wind_governs_lateral_system { theme.accent_orange } else { theme.accent_green }),
                        );
                        for lvl in r.story_levels.iter().take(4) {
                            ui.label(
                                RichText::new(format!(
                                    "L{} z={:.1}m | qz={:.0} Pa | Fx={:.1} kN | Vx={:.0} kN",
                                    lvl.story_num,
                                    lvl.height_z_m,
                                    lvl.qz_velocity_pressure_pa,
                                    lvl.tributary_force_fx_kn,
                                    lvl.cumulative_shear_vx_kn,
                                ))
                                .size(8.4)
                                .monospace()
                                .color(theme.text_secondary),
                            );
                        }
                    }
                });
            });
        });
    }

    /// Update `v0.8.4.3` (`3/10` -> `v0.8.5`): Width-first Dynamic Island drawer for Special RC Shear Wall, SBE & Coupling Beam Studio.
    pub(super) fn render_shear_wall_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        let deck_w = (cap_w - 8.0).max(180.0);
        let two_col = !is_mobile && deck_w >= 540.0;
        ui.set_width(deck_w);
        ui.set_max_width(deck_w);

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(deck_w);
            icons::icon_label(
                ui,
                AppIcon::Layers,
                13.0,
                theme.accent_blue,
                "RC Shear Wall & SBE Studio",
                11.0,
                theme.text_primary,
                true,
            );
            for (idx, lbl) in [
                (0usize, "Wall Pier & SBE Confinement"),
                (1usize, "Coupling Beam & ACI/SNI Audit"),
            ] {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    lbl,
                    self.shear_wall_drawer_tab == idx,
                    theme.accent_blue,
                    "",
                )
                .clicked()
                {
                    self.shear_wall_drawer_tab = idx;
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close Shear Wall & SBE Studio drawer",
            )
            .clicked()
            {
                self.shear_wall_drawer_open = false;
            }
        });

        ui.add_space(4.0);
        theme.glass_subpill_frame(8, Margin::symmetric(8, 5)).show(ui, |ui| {
            let strip_w = (deck_w - 18.0).max(150.0);
            ui.set_max_width(strip_w);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(strip_w);
                for (fc, lbl) in [(30.0_f64, "fc 30"), (35.0, "fc 35"), (45.0, "fc 45 MPa")] {
                    let active = (universal_canvas.shear_wall_studio_config.fc_mpa - fc).abs() < 1.5;
                    if Self::glass_pill_button(ui, theme, None, lbl, active, theme.accent_blue, "Concrete cylinder strength f'c").clicked() {
                        universal_canvas.shear_wall_studio_config.fc_mpa = fc;
                        actions.run_shear_wall_studio = true;
                    }
                }
                for (dia, lbl) in [(22.0_f64, "X-D22"), (25.0, "X-D25"), (29.0, "X-D29")] {
                    let active = (universal_canvas.shear_wall_studio_config.cb_diag_bar_dia_mm - dia).abs() < 1.0;
                    if Self::glass_pill_button(ui, theme, None, lbl, active, theme.accent_orange, "Coupling beam diagonal bar diameter").clicked() {
                        universal_canvas.shear_wall_studio_config.cb_diag_bar_dia_mm = dia;
                        actions.run_shear_wall_studio = true;
                    }
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Layers),
                    if universal_canvas.shear_wall_overlay_visible { "SBE 3D: ON" } else { "SBE 3D" },
                    universal_canvas.shear_wall_overlay_visible,
                    theme.accent_blue,
                    "Toggle 3D Pure WebGPU Shear Wall SBE zones & X-Cage coupling beams",
                )
                .clicked()
                {
                    actions.toggle_shear_wall_3d = true;
                }
                if Self::glass_pill_button(ui, theme, Some(AppIcon::Document), ".ifcx", false, theme.accent_blue, "Export eigengraph::analysis::shear_wall_sbe IFCX").clicked() {
                    actions.export_shear_wall_ifcx = true;
                }
                if Self::glass_pill_button(ui, theme, Some(AppIcon::Printer), "Sheet S-108 .svg", false, theme.accent_green, "Export A3 Sheet S-108 Shear Wall & SBE Annex SVG").clicked() {
                    actions.export_shear_wall_svg = true;
                }
            });
            ui.add_space(3.0);
            ui.spacing_mut().slider_width = (strip_w * 0.28).clamp(58.0, 96.0);
            let render_sliders = |ui: &mut Ui| {
                ui.horizontal(|ui| {
                    let mut tw = universal_canvas.shear_wall_studio_config.wall_thickness_mm;
                    ui.label(RichText::new("Web tw (mm)").size(9.0).color(theme.text_secondary));
                    if ui.add(egui::Slider::new(&mut tw, 250.0..=600.0).max_decimals(0)).changed() {
                        universal_canvas.shear_wall_studio_config.wall_thickness_mm = tw;
                        actions.run_shear_wall_studio = true;
                    }
                });
                ui.horizontal(|ui| {
                    let mut drift_pct = universal_canvas.shear_wall_studio_config.design_drift_ratio * 100.0;
                    ui.label(RichText::new("Drift (%)").size(9.0).color(theme.text_secondary));
                    if ui.add(egui::Slider::new(&mut drift_pct, 0.5..=2.5).max_decimals(2)).changed() {
                        universal_canvas.shear_wall_studio_config.design_drift_ratio = drift_pct * 0.01;
                        actions.run_shear_wall_studio = true;
                    }
                });
                ui.horizontal(|ui| {
                    let mut ln = universal_canvas.shear_wall_studio_config.cb_clear_span_m;
                    ui.label(RichText::new("CB ln (m)").size(9.0).color(theme.text_secondary));
                    if ui.add(egui::Slider::new(&mut ln, 1.0..=3.2).max_decimals(2)).changed() {
                        universal_canvas.shear_wall_studio_config.cb_clear_span_m = ln;
                        actions.run_shear_wall_studio = true;
                    }
                });
            };
            if strip_w >= 640.0 {
                ui.horizontal(render_sliders);
            } else {
                ui.vertical(render_sliders);
            }
        });

        ui.add_space(4.0);
        let tab = self.shear_wall_drawer_tab;
        Self::width_first_pair(ui, deck_w, two_col, |ui, is_left, col_w| {
            theme.glass_subpill_frame(9, Margin::symmetric(8, 6)).show(ui, |ui| {
                let inner_w = (col_w - 20.0).max(150.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.vertical(|ui| {
                    ui.set_max_width(inner_w);
                    let Some(ref studio) = universal_canvas.last_shear_wall_report else {
                        ui.label(RichText::new("Evaluating Special RC Shear Wall Studio...").size(9.5).color(theme.text_muted));
                        return;
                    };
                    let r = &studio.report;
                    let be = &r.boundary_element;
                    let sh = &r.in_plane_shear;
                    let cb = &r.coupling_beam;
                    if tab == 0 {
                        if is_left {
                            ui.label(RichText::new("Special Boundary Element (SBE) Section Schematic").size(9.8).strong().color(theme.text_primary));
                            ui.label(
                                RichText::new(format!(
                                    "SBE {} | c={:.0} vs c_lim={:.0} mm | bE={:.0} mm @ s={:.0} mm",
                                    if be.is_sbe_required { "REQUIRED" } else { "OPTIONAL" },
                                    be.neutral_axis_c_mm,
                                    be.c_limit_mm,
                                    be.provided_length_be_mm,
                                    be.provided_tie_spacing_s_mm,
                                ))
                                .size(8.8)
                                .monospace()
                                .color(if be.is_confinement_adequate { theme.accent_green } else { theme.accent_red }),
                            );
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(inner_w, 72.0), egui::Sense::hover());
                            let p = ui.painter_at(rect);
                            p.rect_filled(rect, 5.0, Color32::from_rgba_unmultiplied(15, 23, 42, 42));
                            let w_rect = Rect::from_min_size(
                                egui::pos2(rect.left() + 14.0, rect.top() + 16.0),
                                Vec2::new((rect.width() - 28.0).max(40.0), 40.0),
                            );
                            p.rect_filled(w_rect, 2.0, Color32::from_rgb(71, 85, 105));
                            let sbe_w = (w_rect.width()
                                * ((be.provided_length_be_mm / r.geometry.length_lw_mm.max(500.0)).clamp(0.12, 0.34) as f32))
                                .max(12.0);
                            let sbe_c = if be.is_confinement_adequate { theme.accent_green } else { theme.accent_red };
                            p.rect_filled(Rect::from_min_size(w_rect.min, Vec2::new(sbe_w, 40.0)), 2.0, sbe_c);
                            p.rect_filled(
                                Rect::from_min_size(egui::pos2(w_rect.right() - sbe_w, w_rect.top()), Vec2::new(sbe_w, 40.0)),
                                2.0,
                                sbe_c,
                            );
                        } else {
                            ui.label(RichText::new("Pier Flexure & In-Plane Shear Demand").size(9.8).strong().color(theme.text_primary));
                            ui.label(
                                RichText::new(format!(
                                    "Flexure Mu={:.0}/{:.0} kNm (DCR={:.2}) | Double Curtain: {}",
                                    r.demands.mu_knm,
                                    r.flexure_phi_mn_knm,
                                    r.flexure_dcr,
                                    if sh.is_double_curtain_required { "YES" } else { "NO" },
                                ))
                                .size(8.8)
                                .monospace()
                                .color(theme.accent_blue),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "In-Plane Shear Vu={:.0}/{:.0} kN (DCR={:.2}) | Ash/s={:.2}/{:.2}",
                                    sh.factored_shear_vu_kn,
                                    sh.phi_vn_kn,
                                    sh.shear_dcr,
                                    be.provided_ash_over_s_mm2_per_mm,
                                    be.req_ash_over_s_mm2_per_mm,
                                ))
                                .size(8.6)
                                .monospace()
                                .color(if sh.is_shear_adequate { theme.accent_green } else { theme.accent_red }),
                            );
                        }
                    } else if is_left {
                        ui.label(RichText::new("Coupling Beam X-Cage Geometry & Capacity").size(9.8).strong().color(theme.text_primary));
                        ui.label(
                            RichText::new(format!(
                                "ln/h={:.2} | alpha={:.1} deg | {} D{:.0}mm | DCR={:.2}",
                                cb.aspect_ratio_ln_over_h,
                                cb.diagonal_angle_deg,
                                cb.num_bars_per_diagonal,
                                cb.bar_dia_mm,
                                cb.coupling_dcr,
                            ))
                            .size(8.8)
                            .monospace()
                            .color(if cb.is_coupling_adequate { theme.accent_blue } else { theme.accent_red }),
                        );
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(inner_w, 72.0), egui::Sense::hover());
                        let p = ui.painter_at(rect);
                        p.rect_filled(rect, 5.0, Color32::from_rgba_unmultiplied(15, 23, 42, 42));
                        let bx = rect.left() + 18.0;
                        let by = rect.top() + 12.0;
                        let bw = (rect.width() - 36.0).max(40.0);
                        let bh = 48.0;
                        p.rect_stroke(Rect::from_min_size(egui::pos2(bx, by), Vec2::new(bw, bh)), 2.0, Stroke::new(1.5, theme.text_secondary), egui::StrokeKind::Inside);
                        p.line_segment([egui::pos2(bx, by), egui::pos2(bx + bw, by + bh)], Stroke::new(2.4, theme.accent_blue));
                        p.line_segment([egui::pos2(bx, by + bh), egui::pos2(bx + bw, by)], Stroke::new(2.4, theme.accent_green));
                    } else {
                        ui.label(RichText::new("ACI 318-19 §18.10 / SNI 2847 Compliance").size(9.8).strong().color(theme.text_primary));
                        for (nm, ok, dcr) in [
                            ("Flexure Mu / phi*Mn", r.is_flexure_adequate, r.flexure_dcr),
                            ("Web Shear Vu / phi*Vn", sh.is_shear_adequate, sh.shear_dcr),
                            ("SBE Confinement Ash/s", be.is_confinement_adequate, be.dcr_confinement),
                            ("Coupling Beam X-Cage", cb.is_coupling_adequate, cb.coupling_dcr),
                        ] {
                            ui.label(
                                RichText::new(format!(
                                    "[{}] {} (DCR {:.2})",
                                    if ok { "PASS" } else { "FAIL" },
                                    nm,
                                    dcr
                                ))
                                .size(8.6)
                                .monospace()
                                .color(if ok { theme.accent_green } else { theme.accent_red }),
                            );
                        }
                    }
                });
            });
        });
    }

    /// Update `v0.8.4.4` (`4/10` -> `v0.8.5`): Width-first Dynamic Island drawer for Steel Seismic Connection & Panel-Zone Studio.
    pub(super) fn render_connection_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        use crate::core::connection::{BoltGrade, ConnectionType};

        let deck_w = (cap_w - 8.0).max(180.0);
        let two_col = !is_mobile && deck_w >= 540.0;
        ui.set_width(deck_w);
        ui.set_max_width(deck_w);

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(deck_w);
            icons::icon_label(
                ui,
                AppIcon::Wrench,
                13.0,
                theme.accent_blue,
                "Steel Connection & Panel-Zone",
                11.0,
                theme.text_primary,
                true,
            );
            for (idx, lbl) in [
                (0usize, "Joint Sizing & Bolt Pattern"),
                (1usize, "Limit States & Panel Zone"),
            ] {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    lbl,
                    self.connection_drawer_tab == idx,
                    theme.accent_blue,
                    "",
                )
                .clicked()
                {
                    self.connection_drawer_tab = idx;
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close Steel Connection Studio drawer",
            )
            .clicked()
            {
                self.connection_drawer_open = false;
            }
        });

        ui.add_space(4.0);
        theme.glass_subpill_frame(8, Margin::symmetric(8, 5)).show(ui, |ui| {
            let strip_w = (deck_w - 18.0).max(150.0);
            ui.set_max_width(strip_w);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(strip_w);
                for (ct, lbl) in [
                    (ConnectionType::ExtendedEndPlate4E, "4E End-Plate"),
                    (ConnectionType::ShearTabSinglePlate, "Shear Tab"),
                ] {
                    let active = universal_canvas.connection_studio_config.connection_type == ct;
                    if Self::glass_pill_button(ui, theme, None, lbl, active, theme.accent_blue, "Select connection archetype").clicked() {
                        universal_canvas.connection_studio_config.connection_type = ct;
                        actions.run_connection_studio = true;
                    }
                }
                for (bg, lbl) in [
                    (BoltGrade::A325X, "A325-X"),
                    (BoltGrade::A490X, "A490-X"),
                    (BoltGrade::Grade10_9, "Class 10.9"),
                ] {
                    let active = universal_canvas.connection_studio_config.bolt_grade == bg;
                    if Self::glass_pill_button(ui, theme, None, lbl, active, theme.accent_orange, "High-strength structural bolt grade").clicked() {
                        universal_canvas.connection_studio_config.bolt_grade = bg;
                        actions.run_connection_studio = true;
                    }
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Layers),
                    if universal_canvas.connection_overlay_visible { "Joints 3D: ON" } else { "Joints 3D" },
                    universal_canvas.connection_overlay_visible,
                    theme.accent_blue,
                    "Toggle 3D Pure WebGPU steel end-plates, bolts & panel zones",
                )
                .clicked()
                {
                    actions.toggle_connection_3d = true;
                }
                if Self::glass_pill_button(ui, theme, Some(AppIcon::Document), ".ifcx", false, theme.accent_blue, "Export eigengraph::analysis::steel_connection IFCX").clicked() {
                    actions.export_connection_ifcx = true;
                }
                if Self::glass_pill_button(ui, theme, Some(AppIcon::Printer), "Sheet S-109 .svg", false, theme.accent_green, "Export A3 Sheet S-109 Steel Connection Annex SVG").clicked() {
                    actions.export_connection_svg = true;
                }
            });
            ui.add_space(3.0);
            ui.spacing_mut().slider_width = (strip_w * 0.28).clamp(58.0, 96.0);
            let render_sliders = |ui: &mut Ui| {
                ui.horizontal(|ui| {
                    let mut db = universal_canvas.connection_studio_config.bolt_diameter_mm;
                    ui.label(RichText::new("Bolt M (mm)").size(9.0).color(theme.text_secondary));
                    if ui.add(egui::Slider::new(&mut db, 16.0..=36.0).max_decimals(0)).changed() {
                        universal_canvas.connection_studio_config.bolt_diameter_mm = db;
                        actions.run_connection_studio = true;
                    }
                });
                ui.horizontal(|ui| {
                    let mut tp = universal_canvas.connection_studio_config.plate_thickness_mm;
                    ui.label(RichText::new("Plate tp (mm)").size(9.0).color(theme.text_secondary));
                    if ui.add(egui::Slider::new(&mut tp, 10.0..=50.0).max_decimals(0)).changed() {
                        universal_canvas.connection_studio_config.plate_thickness_mm = tp;
                        actions.run_connection_studio = true;
                    }
                });
                ui.horizontal(|ui| {
                    let mut bd = universal_canvas.connection_studio_config.beam_depth_mm;
                    ui.label(RichText::new("Beam d (mm)").size(9.0).color(theme.text_secondary));
                    if ui.add(egui::Slider::new(&mut bd, 250.0..=800.0).max_decimals(0)).changed() {
                        universal_canvas.connection_studio_config.beam_depth_mm = bd;
                        actions.run_connection_studio = true;
                    }
                });
            };
            if strip_w >= 640.0 {
                ui.horizontal(render_sliders);
            } else {
                ui.vertical(render_sliders);
            }
        });

        ui.add_space(4.0);
        let tab = self.connection_drawer_tab;
        Self::width_first_pair(ui, deck_w, two_col, |ui, is_left, col_w| {
            theme.glass_subpill_frame(9, Margin::symmetric(8, 6)).show(ui, |ui| {
                let inner_w = (col_w - 20.0).max(150.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.vertical(|ui| {
                    ui.set_max_width(inner_w);
                    let Some(ref studio) = universal_canvas.last_connection_report else {
                        ui.label(RichText::new("Evaluating Steel Connection Studio...").size(9.5).color(theme.text_muted));
                        return;
                    };
                    let r = &studio.report;
                    if tab == 0 {
                        if is_left {
                            ui.label(RichText::new("Connection Plate & High-Strength Bolt Grid").size(9.8).strong().color(theme.text_primary));
                            ui.label(
                                RichText::new(format!(
                                    "{}x M{:.0} {} | tp={:.0} mm | Weld a={:.0} mm | Gov DCR={:.2}",
                                    r.bolt_count_n,
                                    r.bolt_diameter_mm,
                                    r.bolt_grade.label(),
                                    r.plate_thickness_mm,
                                    r.weld_size_mm,
                                    r.governing_dcr,
                                ))
                                .size(8.6)
                                .monospace()
                                .color(if r.is_compliant { theme.accent_blue } else { theme.accent_red }),
                            );
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(inner_w, 72.0), egui::Sense::hover());
                            let p = ui.painter_at(rect);
                            p.rect_filled(rect, 5.0, Color32::from_rgba_unmultiplied(15, 23, 42, 42));
                            let pl = Rect::from_min_size(
                                egui::pos2(rect.center().x - 28.0, rect.top() + 8.0),
                                Vec2::new(56.0, 56.0),
                            );
                            p.rect_stroke(pl, 3.0, Stroke::new(1.5, theme.accent_blue), egui::StrokeKind::Inside);
                            let rows = r.bolt_rows.clamp(2, 6);
                            let cols = r.bolt_cols.clamp(1, 2);
                            for ri in 0..rows {
                                let by = pl.top() + 8.0 + (ri as f32) * (40.0 / (rows.saturating_sub(1).max(1) as f32));
                                for ci in 0..cols {
                                    let bx = if cols == 1 { pl.center().x } else { pl.left() + 16.0 + (ci as f32) * 24.0 };
                                    p.circle_filled(egui::pos2(bx, by), 3.0, theme.accent_orange);
                                }
                            }
                        } else {
                            ui.label(RichText::new("Joint Demands & Building Bolt Quantity").size(9.8).strong().color(theme.text_primary));
                            ui.label(
                                RichText::new(format!(
                                    "Mu={:.0} kNm | Vu={:.0} kN | {} Joints ({} Total Bolts)",
                                    r.applied_moment_mu_knm,
                                    r.applied_shear_vu_kn,
                                    studio.joints_3d.len(),
                                    studio.total_bolts_building,
                                ))
                                .size(8.8)
                                .monospace()
                                .color(theme.accent_blue),
                            );
                            for j in studio.joints_3d.iter().take(3) {
                                ui.label(
                                    RichText::new(format!(
                                        "{} ({:.1},{:.1},{:.1})m | DCR {:.2} | {}",
                                        j.joint_id,
                                        j.pos_enu_m[0],
                                        j.pos_enu_m[1],
                                        j.pos_enu_m[2],
                                        j.governing_dcr,
                                        if j.doubler_required { "DOUBLER REQ" } else { "OK" },
                                    ))
                                    .size(8.5)
                                    .monospace()
                                    .color(if j.is_compliant { theme.accent_green } else { theme.accent_red }),
                                );
                            }
                        }
                    } else if is_left {
                        ui.label(RichText::new("AISC 358/360 Limit States Audit").size(9.8).strong().color(theme.text_primary));
                        for ls in r.limit_states.iter().take(5) {
                            ui.label(
                                RichText::new(format!(
                                    "[{}] {} ({:.2})",
                                    if ls.is_safe { "PASS" } else { "FAIL" },
                                    ls.limit_state_name,
                                    ls.demand_capacity_ratio
                                ))
                                .size(8.5)
                                .monospace()
                                .color(if ls.is_safe { theme.accent_green } else { theme.accent_red }),
                            );
                        }
                    } else {
                        ui.label(RichText::new("Column Panel Zone & Continuity Plates").size(9.8).strong().color(theme.text_primary));
                        if let Some(ref pz) = r.panel_zone {
                            ui.label(
                                RichText::new(format!(
                                    "Vpz={:.0}/{:.0} kN (DCR={:.2}) | Doubler: {}",
                                    pz.applied_shear_vpz_kn,
                                    pz.design_shear_capacity_phi_vn_kn,
                                    pz.panel_zone_dcr,
                                    if pz.doubler_plate_required {
                                        format!("{:.1} mm", pz.recommended_doubler_thickness_mm)
                                    } else {
                                        "NO".to_string()
                                    },
                                ))
                                .size(8.7)
                                .monospace()
                                .color(if pz.is_panel_zone_safe { theme.accent_green } else { theme.accent_orange }),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "Continuity: {} | Web Yield {:.0} kN | Crippling {:.0} kN",
                                    if pz.continuity_plates_required { "YES" } else { "NO" },
                                    pz.web_local_yielding_phi_rn_kn,
                                    pz.web_crippling_phi_rn_kn,
                                ))
                                .size(8.5)
                                .monospace()
                                .color(theme.text_secondary),
                            );
                        } else {
                            ui.label(
                                RichText::new("Pinned Shear Tab mode — column panel zone moment shear is zero.")
                                    .size(8.8)
                                    .color(theme.text_muted),
                            );
                        }
                    }
                });
            });
        });
    }

    /// Update `v0.8.4.5` (`5/10` -> `v0.8.5`): Width-first Dynamic Island drawer for Post-Tensioned Tendon Drape & Losses Studio.
    pub(super) fn render_prestress_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        use crate::core::prestressed_concrete::{PrestressingMethod, TendonProfileKind};

        let deck_w = (cap_w - 8.0).max(180.0);
        let two_col = !is_mobile && deck_w >= 540.0;
        ui.set_width(deck_w);
        ui.set_max_width(deck_w);

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(deck_w);
            icons::icon_label(
                ui,
                AppIcon::ChartLine,
                13.0,
                theme.accent_blue,
                "PT & Prestressed Tendon Studio",
                11.0,
                theme.text_primary,
                true,
            );
            for (idx, lbl) in [
                (0usize, "Tendon Profile & Losses"),
                (1usize, "Fiber Stress, Shear & Camber"),
            ] {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    lbl,
                    self.prestress_drawer_tab == idx,
                    theme.accent_blue,
                    "",
                )
                .clicked()
                {
                    self.prestress_drawer_tab = idx;
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close Prestressed / PT Studio drawer",
            )
            .clicked()
            {
                self.prestress_drawer_open = false;
            }
        });

        ui.add_space(4.0);
        theme.glass_subpill_frame(8, Margin::symmetric(8, 5)).show(ui, |ui| {
            let strip_w = (deck_w - 18.0).max(150.0);
            ui.set_max_width(strip_w);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(strip_w);
                for (prof, lbl) in [
                    (TendonProfileKind::Parabolic, "Parabolic"),
                    (TendonProfileKind::HarpedTwoPoint, "Harped 2-Pt"),
                    (TendonProfileKind::StraightEccentric, "Straight"),
                ] {
                    let active = universal_canvas.prestress_studio_config.profile_kind == prof;
                    if Self::glass_pill_button(ui, theme, None, lbl, active, theme.accent_blue, "Tendon longitudinal drape profile").clicked() {
                        universal_canvas.prestress_studio_config.profile_kind = prof;
                        actions.run_prestress_studio = true;
                    }
                }
                for (meth, lbl) in [
                    (PrestressingMethod::PostTensioned, "Post-Tension"),
                    (PrestressingMethod::PreTensioned, "Pre-Tension"),
                ] {
                    let active = universal_canvas.prestress_studio_config.method == meth;
                    if Self::glass_pill_button(ui, theme, None, lbl, active, theme.accent_orange, "Prestressing sequence").clicked() {
                        universal_canvas.prestress_studio_config.method = meth;
                        actions.run_prestress_studio = true;
                    }
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Layers),
                    if universal_canvas.prestress_overlay_visible { "Tendons 3D: ON" } else { "Tendons 3D" },
                    universal_canvas.prestress_overlay_visible,
                    theme.accent_blue,
                    "Toggle 3D Pure WebGPU draped tendon ducts & anchorage zones",
                )
                .clicked()
                {
                    actions.toggle_prestress_3d = true;
                }
                if Self::glass_pill_button(ui, theme, Some(AppIcon::Document), ".ifcx", false, theme.accent_blue, "Export eigengraph::analysis::prestress_tendon IFCX").clicked() {
                    actions.export_prestress_ifcx = true;
                }
                if Self::glass_pill_button(ui, theme, Some(AppIcon::Printer), "Sheet S-110 .svg", false, theme.accent_green, "Export A3 Sheet S-110 PT Tendon Drape Annex SVG").clicked() {
                    actions.export_prestress_svg = true;
                }
            });
            ui.add_space(3.0);
            ui.spacing_mut().slider_width = (strip_w * 0.28).clamp(58.0, 96.0);
            let render_sliders = |ui: &mut Ui| {
                ui.horizontal(|ui| {
                    let mut strands = universal_canvas.prestress_studio_config.num_strands as f64;
                    ui.label(RichText::new("Strands").size(9.0).color(theme.text_secondary));
                    if ui.add(egui::Slider::new(&mut strands, 4.0..=48.0).max_decimals(0)).changed() {
                        universal_canvas.prestress_studio_config.num_strands = strands.round() as usize;
                        actions.run_prestress_studio = true;
                    }
                });
                ui.horizontal(|ui| {
                    let mut e_mid = universal_canvas.prestress_studio_config.eccentricity_mid_mm;
                    ui.label(RichText::new("e_mid (mm)").size(9.0).color(theme.text_secondary));
                    if ui.add(egui::Slider::new(&mut e_mid, 100.0..=650.0).max_decimals(0)).changed() {
                        universal_canvas.prestress_studio_config.eccentricity_mid_mm = e_mid;
                        actions.run_prestress_studio = true;
                    }
                });
                ui.horizontal(|ui| {
                    let mut fc = universal_canvas.prestress_studio_config.fc_prime_mpa;
                    ui.label(RichText::new("f'c (MPa)").size(9.0).color(theme.text_secondary));
                    if ui.add(egui::Slider::new(&mut fc, 30.0..=75.0).max_decimals(0)).changed() {
                        universal_canvas.prestress_studio_config.fc_prime_mpa = fc;
                        actions.run_prestress_studio = true;
                    }
                });
            };
            if strip_w >= 640.0 {
                ui.horizontal(render_sliders);
            } else {
                ui.vertical(render_sliders);
            }
        });

        ui.add_space(4.0);
        let tab = self.prestress_drawer_tab;
        Self::width_first_pair(ui, deck_w, two_col, |ui, is_left, col_w| {
            theme.glass_subpill_frame(9, Margin::symmetric(8, 6)).show(ui, |ui| {
                let inner_w = (col_w - 20.0).max(150.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.vertical(|ui| {
                    ui.set_max_width(inner_w);
                    let Some(ref studio) = universal_canvas.last_prestress_report else {
                        ui.label(RichText::new("Evaluating Prestressed / PT Studio...").size(9.5).color(theme.text_muted));
                        return;
                    };
                    let r = &studio.report;
                    let ls = &r.losses;
                    if tab == 0 {
                        if is_left {
                            ui.label(RichText::new("Draped Tendon Profile (L-Elevation)").size(9.8).strong().color(theme.text_primary));
                            ui.label(
                                RichText::new(format!(
                                    "L={:.1}m | {}x{:.1}mm (Aps={:.0} mm2) | e_mid={:.0}mm",
                                    r.span_length_m,
                                    r.num_strands,
                                    r.strand_dia_mm,
                                    r.total_aps_mm2,
                                    studio.config.eccentricity_mid_mm,
                                ))
                                .size(8.8)
                                .monospace()
                                .color(theme.accent_blue),
                            );
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(inner_w, 76.0), egui::Sense::hover());
                            let p = ui.painter_at(rect);
                            p.rect_filled(rect, 5.0, Color32::from_rgba_unmultiplied(15, 23, 42, 42));
                            let span_l = r.span_length_m.max(1.0);
                            let max_e = (r.y_bot_mm.max(r.y_top_mm)).max(200.0);
                            let pts: Vec<_> = r
                                .tendon_profile_points
                                .iter()
                                .map(|pt| {
                                    egui::pos2(
                                        rect.left() + 10.0 + ((pt.x_m / span_l).clamp(0.0, 1.0) as f32) * (rect.width() - 20.0),
                                        rect.center().y + ((pt.e_mm / max_e).clamp(-0.45, 0.45) as f32) * (rect.height() - 16.0),
                                    )
                                })
                                .collect();
                            if pts.len() >= 2 {
                                p.add(egui::Shape::line(pts, Stroke::new(2.6, theme.accent_orange)));
                            }
                        } else {
                            ui.label(RichText::new("Immediate & Long-Term Prestress Losses").size(9.8).strong().color(theme.text_primary));
                            ui.label(
                                RichText::new(format!(
                                    "Pj={:.0} -> Pi={:.0} -> Pe={:.0} kN (-{:.1}% Total Loss)",
                                    ls.p_jack_kn, ls.p_transfer_kn, ls.p_effective_kn, ls.loss_percentage
                                ))
                                .size(8.8)
                                .monospace()
                                .color(theme.accent_blue),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "Imm: Fric {:.0} + Slip {:.0} + ES {:.0} = {:.0} MPa",
                                    ls.friction_loss_mpa,
                                    ls.anchorage_slip_mpa,
                                    ls.elastic_shortening_mpa,
                                    ls.total_immediate_loss_mpa,
                                ))
                                .size(8.5)
                                .monospace()
                                .color(theme.text_secondary),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "Time: Creep {:.0} + Shr {:.0} + Rel {:.0} = {:.0} MPa",
                                    ls.concrete_creep_mpa,
                                    ls.concrete_shrinkage_mpa,
                                    ls.steel_relaxation_mpa,
                                    ls.total_long_term_loss_mpa,
                                ))
                                .size(8.5)
                                .monospace()
                                .color(theme.text_secondary),
                            );
                        }
                    } else if is_left {
                        ui.label(RichText::new("Transfer & Service Fiber Stresses").size(9.8).strong().color(theme.text_primary));
                        ui.label(
                            RichText::new(format!(
                                "ACI Tension Class: {}",
                                r.aci_class.label(),
                            ))
                            .size(8.6)
                            .color(theme.accent_blue),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Transfer: top {:+.2} / bot {:+.2} MPa (DCR {:.2}/{:.2})",
                                r.transfer_stress.sigma_top_mpa,
                                r.transfer_stress.sigma_bot_mpa,
                                r.transfer_stress.top_dcr,
                                r.transfer_stress.bot_dcr,
                            ))
                            .size(8.5)
                            .monospace()
                            .color(if r.transfer_stress.is_acceptable { theme.accent_green } else { theme.accent_red }),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Service: top {:+.2} / bot {:+.2} MPa (DCR {:.2}/{:.2})",
                                r.service_stress.sigma_top_mpa,
                                r.service_stress.sigma_bot_mpa,
                                r.service_stress.top_dcr,
                                r.service_stress.bot_dcr,
                            ))
                            .size(8.5)
                            .monospace()
                            .color(if r.service_stress.is_acceptable { theme.accent_green } else { theme.accent_red }),
                        );
                    } else {
                        ui.label(RichText::new("Ultimate Strength & Camber Summary").size(9.8).strong().color(theme.text_primary));
                        ui.label(
                            RichText::new(format!(
                                "Flexure Mu={:.0}/{:.0} kNm (DCR={:.2}) | Shear Vu={:.0}/{:.0} kN (DCR={:.2})",
                                r.m_ultimate_knm, r.phi_mn_knm, r.flexure_dcr, r.v_ultimate_kn, r.phi_vn_kn, r.shear_dcr
                            ))
                            .size(8.6)
                            .monospace()
                            .color(if r.is_code_compliant { theme.accent_green } else { theme.accent_red }),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Camber: Transfer {:+.1} mm | Long-Term {:+.1} mm | 1.2*Mcr={:.0} kNm",
                                r.net_camber_transfer_mm, r.long_term_camber_mm, r.min_flexure_req_knm
                            ))
                            .size(8.5)
                            .monospace()
                            .color(theme.text_secondary),
                        );
                    }
                });
            });
        });
    }

    /// Update `v0.8.4.6` (`6/10` -> `v0.8.5`): Width-first Dynamic Island drawer for Composite Deck, Shear Studs & AISC DG11 Vibration Studio.
    pub(super) fn render_composite_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        use crate::core::composite::DeckOrientation;

        let deck_w = (cap_w - 8.0).max(180.0);
        let two_col = !is_mobile && deck_w >= 540.0;
        ui.set_width(deck_w);
        ui.set_max_width(deck_w);

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(deck_w);
            icons::icon_label(
                ui,
                AppIcon::Grid,
                13.0,
                theme.accent_blue,
                "Composite Deck & DG11 Studio",
                11.0,
                theme.text_primary,
                true,
            );
            for (idx, lbl) in [
                (0usize, "Deck & Shear Studs"),
                (1usize, "DG11 Vibration & Deflection"),
            ] {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    lbl,
                    self.composite_drawer_tab == idx,
                    theme.accent_blue,
                    "",
                )
                .clicked()
                {
                    self.composite_drawer_tab = idx;
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close Composite Deck Studio drawer",
            )
            .clicked()
            {
                self.composite_drawer_open = false;
            }
        });

        ui.add_space(4.0);
        theme.glass_subpill_frame(8, Margin::symmetric(8, 5)).show(ui, |ui| {
            let strip_w = (deck_w - 18.0).max(150.0);
            ui.set_max_width(strip_w);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(strip_w);
                for (or, lbl) in [
                    (DeckOrientation::Perpendicular, "Perp Ribs"),
                    (DeckOrientation::Parallel, "Parallel Ribs"),
                ] {
                    let active = universal_canvas.composite_studio_config.deck_orientation == or;
                    if Self::glass_pill_button(ui, theme, None, lbl, active, theme.accent_blue, "Metal deck rib orientation").clicked() {
                        universal_canvas.composite_studio_config.deck_orientation = or;
                        actions.run_composite_studio = true;
                    }
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    if universal_canvas.composite_studio_config.unshored { "Unshored" } else { "Shored" },
                    universal_canvas.composite_studio_config.unshored,
                    theme.accent_orange,
                    "Toggle unshored vs shored wet concrete construction check",
                )
                .clicked()
                {
                    universal_canvas.composite_studio_config.unshored =
                        !universal_canvas.composite_studio_config.unshored;
                    actions.run_composite_studio = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Layers),
                    if universal_canvas.composite_overlay_visible { "Deck 3D: ON" } else { "Deck 3D" },
                    universal_canvas.composite_overlay_visible,
                    theme.accent_blue,
                    "Toggle 3D Pure WebGPU composite floor strips & shear stud pins",
                )
                .clicked()
                {
                    actions.toggle_composite_3d = true;
                }
                if Self::glass_pill_button(ui, theme, Some(AppIcon::Document), ".ifcx", false, theme.accent_blue, "Export eigengraph::analysis::composite_deck IFCX").clicked() {
                    actions.export_composite_ifcx = true;
                }
                if Self::glass_pill_button(ui, theme, Some(AppIcon::Printer), "Sheet S-111 .svg", false, theme.accent_green, "Export A3 Sheet S-111 Composite Deck Annex SVG").clicked() {
                    actions.export_composite_svg = true;
                }
            });
            ui.add_space(3.0);
            ui.spacing_mut().slider_width = (strip_w * 0.28).clamp(58.0, 96.0);
            let render_sliders = |ui: &mut Ui| {
                ui.horizontal(|ui| {
                    let mut studs = universal_canvas.composite_studio_config.studs_provided as f64;
                    ui.label(RichText::new("Studs/Bay").size(9.0).color(theme.text_secondary));
                    if ui.add(egui::Slider::new(&mut studs, 12.0..=120.0).max_decimals(0)).changed() {
                        universal_canvas.composite_studio_config.studs_provided = studs.round() as usize;
                        actions.run_composite_studio = true;
                    }
                });
                ui.horizontal(|ui| {
                    let mut slab_h = universal_canvas.composite_studio_config.total_slab_h_mm;
                    ui.label(RichText::new("Slab h (mm)").size(9.0).color(theme.text_secondary));
                    if ui.add(egui::Slider::new(&mut slab_h, 110.0..=240.0).max_decimals(0)).changed() {
                        universal_canvas.composite_studio_config.total_slab_h_mm = slab_h;
                        actions.run_composite_studio = true;
                    }
                });
                ui.horizontal(|ui| {
                    let mut beta = universal_canvas.composite_studio_config.floor_damping_beta * 100.0;
                    ui.label(RichText::new("Damping (%)").size(9.0).color(theme.text_secondary));
                    if ui.add(egui::Slider::new(&mut beta, 1.5..=6.0).max_decimals(1)).changed() {
                        universal_canvas.composite_studio_config.floor_damping_beta = beta * 0.01;
                        actions.run_composite_studio = true;
                    }
                });
            };
            if strip_w >= 640.0 {
                ui.horizontal(render_sliders);
            } else {
                ui.vertical(render_sliders);
            }
        });

        ui.add_space(4.0);
        let tab = self.composite_drawer_tab;
        Self::width_first_pair(ui, deck_w, two_col, |ui, is_left, col_w| {
            theme.glass_subpill_frame(9, Margin::symmetric(8, 6)).show(ui, |ui| {
                let inner_w = (col_w - 20.0).max(150.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.vertical(|ui| {
                    ui.set_max_width(inner_w);
                    let Some(ref studio) = universal_canvas.last_composite_report else {
                        ui.label(RichText::new("Evaluating Composite Deck Studio...").size(9.5).color(theme.text_muted));
                        return;
                    };
                    let r = &studio.report;
                    let v = &studio.vibration;
                    if tab == 0 {
                        if is_left {
                            ui.label(RichText::new("Composite Cross-Section & PNA Location").size(9.8).strong().color(theme.text_primary));
                            ui.label(
                                RichText::new(format!(
                                    "{} | Action={:.0}% | Qn={:.1} kN | {}",
                                    r.steel_beam_name,
                                    r.degree_of_composite_action_pct,
                                    r.stud_single_shear_capacity_qn_kn,
                                    r.pna_location.label(),
                                ))
                                .size(8.6)
                                .monospace()
                                .color(if r.is_composite_action_adequate { theme.accent_blue } else { theme.accent_red }),
                            );
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(inner_w, 70.0), egui::Sense::hover());
                            let p = ui.painter_at(rect);
                            p.rect_filled(rect, 5.0, Color32::from_rgba_unmultiplied(15, 23, 42, 42));
                            p.rect_filled(
                                Rect::from_min_size(egui::pos2(rect.left() + 18.0, rect.top() + 10.0), Vec2::new((rect.width() - 36.0).max(40.0), 18.0)),
                                2.0,
                                Color32::from_rgb(148, 163, 184),
                            );
                            p.rect_filled(
                                Rect::from_min_size(egui::pos2(rect.center().x - 16.0, rect.top() + 28.0), Vec2::new(32.0, 32.0)),
                                2.0,
                                theme.accent_blue,
                            );
                        } else {
                            ui.label(RichText::new("Flexure, Wet Concrete & Stud Quantity").size(9.8).strong().color(theme.text_primary));
                            ui.label(
                                RichText::new(format!(
                                    "Mu={:.0}/{:.0} kNm (DCR={:.2}) | Const DCR={:.2}",
                                    r.applied_factored_moment_mu_knm,
                                    r.design_flexural_strength_phi_mn_knm,
                                    r.flexure_dcr,
                                    r.construction_stage_dcr,
                                ))
                                .size(8.8)
                                .monospace()
                                .color(if r.is_overall_compliant { theme.accent_green } else { theme.accent_red }),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "{} Studs/Bay ({} Total Studs) • Rg={:.2} Rp={:.2}",
                                    r.total_studs_provided,
                                    studio.total_studs_building,
                                    r.rg_factor,
                                    r.rp_factor,
                                ))
                                .size(8.6)
                                .monospace()
                                .color(theme.text_secondary),
                            );
                        }
                    } else if is_left {
                        ui.label(RichText::new("AISC Design Guide 11 Walking Floor Vibration").size(9.8).strong().color(theme.text_primary));
                        ui.label(
                            RichText::new(format!(
                                "fn = {:.2} Hz | Peak Accel = {:.3}% g (Limit {:.2}% g)",
                                v.fundamental_freq_fn_hz,
                                v.peak_accel_pct_g,
                                v.tolerance_limit_pct_g,
                            ))
                            .size(8.8)
                            .monospace()
                            .color(if v.is_vibration_ok { theme.accent_green } else { theme.accent_red }),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Floor Damping beta={:.1}% • Status: {}",
                                v.damping_ratio * 100.0,
                                if v.is_vibration_ok { "DG11 COMFORT COMPLIANT" } else { "STIFFEN BAY / ADD DAMPING" }
                            ))
                            .size(8.5)
                            .monospace()
                            .color(theme.text_secondary),
                        );
                    } else {
                        ui.label(RichText::new("Serviceability Deflections & Composite Ieff").size(9.8).strong().color(theme.text_primary));
                        ui.label(
                            RichText::new(format!(
                                "LL Deflection: {:.2} / {:.2} mm (L/360) [{}]",
                                r.live_load_deflection_delta_ll_mm,
                                r.live_load_deflection_limit_mm,
                                if r.is_deflection_ll_ok { "PASS" } else { "FAIL" },
                            ))
                            .size(8.6)
                            .monospace()
                            .color(if r.is_deflection_ll_ok { theme.accent_green } else { theme.accent_red }),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Creep Total: {:.2} / {:.2} mm (L/240) | Ieff={:.0} cm4",
                                r.total_creep_deflection_delta_total_mm,
                                r.total_deflection_limit_mm,
                                r.lower_bound_effective_inertia_ieff_cm4,
                            ))
                            .size(8.5)
                            .monospace()
                            .color(if r.is_deflection_total_ok { theme.accent_green } else { theme.accent_red }),
                        );
                    }
                });
            });
        });
    }

    /// Update `v0.8.4.7` (`7/10` -> `v0.8.5`): Width-first Dynamic Island drawer for Structural Reliability `FORM/SORM` & FEMA P-58 Fragility Studio.
    pub(super) fn render_reliability_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        use crate::core::reliability_studio::{LimitStateKind, ReliabilityMethod};

        let deck_w = (cap_w - 8.0).max(180.0);
        let two_col = !is_mobile && deck_w >= 540.0;
        ui.set_width(deck_w);
        ui.set_max_width(deck_w);

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(deck_w);
            icons::icon_label(
                ui,
                AppIcon::Target,
                13.0,
                theme.accent_orange,
                "Reliability FORM/SORM Studio",
                11.0,
                theme.text_primary,
                true,
            );
            for (idx, lbl) in [
                (0usize, "FORM/SORM & Sensitivity"),
                (1usize, "FEMA P-58 Fragility & Audit"),
            ] {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    lbl,
                    self.reliability_drawer_tab == idx,
                    theme.accent_orange,
                    "",
                )
                .clicked()
                {
                    self.reliability_drawer_tab = idx;
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close Structural Reliability Studio drawer",
            )
            .clicked()
            {
                self.reliability_drawer_open = false;
            }
        });

        ui.add_space(4.0);
        theme.glass_subpill_frame(8, Margin::symmetric(8, 5)).show(ui, |ui| {
            let strip_w = (deck_w - 18.0).max(150.0);
            ui.set_max_width(strip_w);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(strip_w);
                for (m, lbl) in [
                    (ReliabilityMethod::FormHasoferLind, "FORM"),
                    (ReliabilityMethod::SormBreitung, "SORM"),
                    (ReliabilityMethod::FosmLogNormal, "FOSM"),
                ] {
                    let active = universal_canvas.reliability_studio_config.method == m;
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        lbl,
                        active,
                        theme.accent_orange,
                        "Select probabilistic reliability solver",
                    )
                    .clicked()
                    {
                        universal_canvas.reliability_studio_config.method = m;
                        actions.run_reliability_studio = true;
                    }
                }
                for (ls, lbl) in [
                    (LimitStateKind::FlexurePlastic, "Flexure"),
                    (LimitStateKind::AxialColumn, "Column"),
                    (LimitStateKind::InterstoryDrift, "Drift"),
                    (LimitStateKind::ShearBrittle, "Shear"),
                ] {
                    let active = universal_canvas.reliability_studio_config.limit_state == ls;
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        lbl,
                        active,
                        theme.accent_blue,
                        "Select limit-state formulation",
                    )
                    .clicked()
                    {
                        universal_canvas.reliability_studio_config.limit_state = ls;
                        actions.run_reliability_studio = true;
                    }
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Bolt),
                    "Run FORM/SORM",
                    true,
                    theme.accent_green,
                    "Evaluate Rackwitz-Fiessler FORM, Breitung SORM & sensitivities",
                )
                .clicked()
                {
                    actions.run_reliability_studio = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Layers),
                    if universal_canvas.reliability_overlay_visible {
                        "3D Risk: ON"
                    } else {
                        "3D Risk: OFF"
                    },
                    universal_canvas.reliability_overlay_visible,
                    theme.accent_blue,
                    "Toggle 3D Pure WebGPU beta contour members & risk beacons",
                )
                .clicked()
                {
                    actions.toggle_reliability_3d = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Document),
                    "IFCX",
                    false,
                    theme.accent_orange,
                    "Export non-destructive eigengraph::analysis::reliability_form IFCX",
                )
                .clicked()
                {
                    actions.export_reliability_ifcx = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Printer),
                    "Sheet S-112",
                    false,
                    theme.accent_blue,
                    "Export A3 Pure-Vector Sheet S-112 Reliability Annex SVG",
                )
                .clicked()
                {
                    actions.export_reliability_svg = true;
                }
            });
        });

        ui.add_space(4.0);
        let Some(ref studio) = universal_canvas.last_reliability_report else {
            ui.label(
                RichText::new("Click 'Run FORM/SORM' or toggle '3D Risk' to evaluate member beta indices & sensitivity factors.")
                    .size(9.4)
                    .color(theme.text_muted),
            );
            return;
        };

        let tab = self.reliability_drawer_tab;
        Self::width_first_pair(ui, deck_w, two_col, |ui, is_left, col_w| {
            theme.glass_subpill_frame(8, Margin::symmetric(8, 6)).show(ui, |ui| {
                let inner_w = (col_w - 20.0).max(150.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.vertical(|ui| {
                    ui.set_max_width(inner_w);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    if is_left {
                        ui.label(RichText::new("Random Variables & Target Beta Parameters").size(9.8).strong().color(theme.text_primary));
                        let cfg = &mut universal_canvas.reliability_studio_config;
                        let mut changed = false;
                        ui.horizontal_wrapped(|ui| {
                            ui.set_max_width(inner_w);
                            ui.label(RichText::new("Target beta:").size(8.8).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.target_beta).range(2.0..=5.5).speed(0.05)).changed();
                            ui.label(RichText::new("Mean DCR:").size(8.8).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.demand_to_capacity_ratio).range(0.25..=1.25).speed(0.02)).changed();
                            ui.label(RichText::new("V_R:").size(8.8).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.resistance_cov).range(0.05..=0.30).speed(0.01)).changed();
                            ui.label(RichText::new("V_L:").size(8.8).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.live_load_cov).range(0.10..=0.45).speed(0.01)).changed();
                            ui.label(RichText::new("V_E:").size(8.8).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.seismic_load_cov).range(0.15..=0.65).speed(0.02)).changed();
                            ui.label(RichText::new("Seismic %:").size(8.8).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.seismic_fraction).range(0.0..=0.80).speed(0.02)).changed();
                        });
                        if changed {
                            actions.run_reliability_studio = true;
                        }
                        if tab == 0 {
                            ui.label(
                                RichText::new(format!(
                                    "Active beta={:.2} (Pf={:.2e}) | kappa={:+.4} ({} iter)",
                                    studio.active_beta,
                                    studio.active_pf,
                                    studio.curvature_kappa,
                                    studio.iterations,
                                ))
                                .size(8.6)
                                .monospace()
                                .color(if studio.is_overall_compliant { theme.accent_green } else { theme.accent_red }),
                            );
                        } else {
                            let mid_frag = studio
                                .fragility_curve
                                .get(studio.fragility_curve.len() / 2);
                            if let Some(pt) = mid_frag {
                                ui.label(
                                    RichText::new(format!(
                                        "@ {:.2}g: Slight={:.0}% Mod={:.0}% Ext={:.0}% Col={:.0}%",
                                        pt.pga_g,
                                        pt.prob_slight * 100.0,
                                        pt.prob_moderate * 100.0,
                                        pt.prob_extensive * 100.0,
                                        pt.prob_complete_collapse * 100.0,
                                    ))
                                    .size(8.5)
                                    .monospace()
                                    .color(theme.accent_orange),
                                );
                            }
                        }
                    } else if tab == 0 {
                        ui.label(RichText::new("FORM/SORM Comparison & Sensitivity (alpha^2 %)").size(9.8).strong().color(theme.text_primary));
                        ui.label(
                            RichText::new(format!(
                                "FOSM={:.2} (Pf {:.2e}) | FORM={:.2} | SORM={:.2}",
                                studio.fosm_beta, studio.fosm_pf, studio.form_beta, studio.sorm_beta
                            ))
                            .size(8.7)
                            .monospace()
                            .color(if studio.is_overall_compliant { theme.accent_green } else { theme.accent_red }),
                        );
                        for s in &studio.sensitivities {
                            ui.label(
                                RichText::new(format!(
                                    "{} a={:+.3} | {:.1}% | u*={:+.2}",
                                    s.variable_name, s.alpha, s.importance_pct, s.design_point_u
                                ))
                                .size(8.3)
                                .monospace()
                                .color(theme.text_secondary),
                            );
                        }
                    } else {
                        ui.label(RichText::new("3D Member Beta & Code Compliance Audit").size(9.8).strong().color(theme.text_primary));
                        ui.label(
                            RichText::new(format!(
                                "Min Member beta={:.2} | Below Target={}/{} [{}]",
                                studio.min_member_beta,
                                studio.below_target_count,
                                studio.members_3d.len(),
                                if studio.is_overall_compliant { "PASS" } else { "REVIEW" }
                            ))
                            .size(8.8)
                            .monospace()
                            .color(if studio.is_overall_compliant { theme.accent_green } else { theme.accent_red }),
                        );
                        for hs in studio.hotspots_3d.iter().take(3) {
                            ui.label(
                                RichText::new(format!(
                                    "• {} ({})",
                                    hs.label,
                                    if hs.is_critical { "CRITICAL" } else { "OK" }
                                ))
                                .size(8.4)
                                .monospace()
                                .color(if hs.is_critical { theme.accent_red } else { theme.accent_green }),
                            );
                        }
                    }
                });
            });
        });
    }

    /// Update `v0.8.4.8` (`8/10` -> `v0.8.5`): Width-first Dynamic Island drawer for Deep Foundation Pile Group, Winkler Springs & Punching Shear Studio.
    pub(super) fn render_foundation_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        use crate::core::geotech::SoilType;

        let deck_w = (cap_w - 8.0).max(180.0);
        let two_col = !is_mobile && deck_w >= 540.0;
        ui.set_width(deck_w);
        ui.set_max_width(deck_w);

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(deck_w);
            icons::icon_label(
                ui,
                AppIcon::Hammer,
                13.0,
                theme.accent_green,
                "Deep Foundation & Winkler Studio",
                11.0,
                theme.text_primary,
                true,
            );
            for (idx, lbl) in [
                (0usize, "Pile Group & Cap"),
                (1usize, "Winkler SSI & Punching"),
            ] {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    lbl,
                    self.foundation_drawer_tab == idx,
                    theme.accent_green,
                    "",
                )
                .clicked()
                {
                    self.foundation_drawer_tab = idx;
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close Deep Foundation Studio drawer",
            )
            .clicked()
            {
                self.foundation_drawer_open = false;
            }
        });

        ui.add_space(4.0);
        theme.glass_subpill_frame(8, Margin::symmetric(8, 5)).show(ui, |ui| {
            let strip_w = (deck_w - 18.0).max(150.0);
            ui.set_max_width(strip_w);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(strip_w);
                for (st, lbl) in [
                    (SoilType::SoftClay, "Soft Clay"),
                    (SoilType::MediumSand, "Med Sand"),
                    (SoilType::DenseSand, "Dense Sand"),
                    (SoilType::Bedrock, "Bedrock"),
                ] {
                    let active = universal_canvas.foundation_studio_config.soil_type == st;
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        lbl,
                        active,
                        theme.accent_orange,
                        "Select bearing soil stratum",
                    )
                    .clicked()
                    {
                        universal_canvas.foundation_studio_config.soil_type = st;
                        actions.run_foundation_studio = true;
                    }
                }
                for (nx, ny, lbl) in [(2usize, 2usize, "2x2"), (3, 3, "3x3"), (4, 4, "4x4")] {
                    let cfg = &universal_canvas.foundation_studio_config;
                    let active = cfg.piles_x == nx && cfg.piles_y == ny;
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        lbl,
                        active,
                        theme.accent_blue,
                        "Select pile group grid",
                    )
                    .clicked()
                    {
                        universal_canvas.foundation_studio_config.piles_x = nx;
                        universal_canvas.foundation_studio_config.piles_y = ny;
                        actions.run_foundation_studio = true;
                    }
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Bolt),
                    "Run Piles",
                    true,
                    theme.accent_green,
                    "Evaluate Converse-Labarre pile group, Winkler springs & punching shear",
                )
                .clicked()
                {
                    actions.run_foundation_studio = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Layers),
                    if universal_canvas.foundation_overlay_visible {
                        "3D Piles: ON"
                    } else {
                        "3D Piles: OFF"
                    },
                    universal_canvas.foundation_overlay_visible,
                    theme.accent_blue,
                    "Toggle 3D Pure WebGPU deep pile caps, shafts & Winkler springs",
                )
                .clicked()
                {
                    actions.toggle_foundation_3d = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Document),
                    "IFCX",
                    false,
                    theme.accent_orange,
                    "Export non-destructive eigengraph::analysis::deep_foundation IFCX",
                )
                .clicked()
                {
                    actions.export_foundation_ifcx = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Printer),
                    "Sheet S-113",
                    false,
                    theme.accent_blue,
                    "Export A3 Pure-Vector Sheet S-113 Deep Foundation Annex SVG",
                )
                .clicked()
                {
                    actions.export_foundation_svg = true;
                }
            });
        });

        ui.add_space(4.0);
        let Some(ref studio) = universal_canvas.last_foundation_report else {
            ui.label(
                RichText::new("Click 'Run Piles' or toggle '3D Piles' to compute Converse-Labarre group efficiency, 6-DOF Winkler springs & punching shear.")
                    .size(9.4)
                    .color(theme.text_muted),
            );
            return;
        };

        let tab = self.foundation_drawer_tab;
        Self::width_first_pair(ui, deck_w, two_col, |ui, is_left, col_w| {
            theme.glass_subpill_frame(8, Margin::symmetric(8, 6)).show(ui, |ui| {
                let inner_w = (col_w - 20.0).max(150.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.vertical(|ui| {
                    ui.set_max_width(inner_w);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    if is_left {
                        ui.label(RichText::new("Pile Geometry, Cap & Column Loads").size(9.8).strong().color(theme.text_primary));
                        let cfg = &mut universal_canvas.foundation_studio_config;
                        let mut changed = false;
                        ui.horizontal_wrapped(|ui| {
                            ui.set_max_width(inner_w);
                            ui.label(RichText::new("Dia (m):").size(8.8).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.pile_diameter_m).range(0.35..=1.80).speed(0.05)).changed();
                            ui.label(RichText::new("Spacing (m):").size(8.8).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.pile_spacing_m).range(1.20..=5.00).speed(0.1)).changed();
                            ui.label(RichText::new("Length (m):").size(8.8).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.pile_length_m).range(6.0..=48.0).speed(0.5)).changed();
                            ui.label(RichText::new("Cap h (m):").size(8.8).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.cap_thickness_m).range(0.60..=3.20).speed(0.05)).changed();
                            ui.label(RichText::new("Pu (kN):").size(8.8).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.column_pu_kn).range(400.0..=24000.0).speed(50.0)).changed();
                            ui.label(RichText::new("Mux (kNm):").size(8.8).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.column_mux_knm).range(0.0..=6000.0).speed(25.0)).changed();
                        });
                        if changed {
                            actions.run_foundation_studio = true;
                        }
                        if tab == 0 {
                            ui.label(
                                RichText::new(format!(
                                    "Converse-Labarre Eg={:.3} | {}x{} Cap={:.2}x{:.2}x{:.2}m",
                                    studio.pile_group.converse_labarre_efficiency,
                                    studio.pile_group.piles_count_x,
                                    studio.pile_group.piles_count_y,
                                    studio.pile_group.cap_width_m,
                                    studio.pile_group.cap_length_m,
                                    studio.pile_group.cap_thickness_m,
                                ))
                                .size(8.6)
                                .monospace()
                                .color(if studio.pile_group.is_group_safe { theme.accent_green } else { theme.accent_red }),
                            );
                        } else {
                            ui.label(
                                RichText::new(format!(
                                    "Winkler Kz={:.1} MN/m | Kx={:.1} MN/m | Kry={:.1} MNm/rad",
                                    studio.winkler_springs.kz_trans_kn_m / 1000.0,
                                    studio.winkler_springs.kx_trans_kn_m / 1000.0,
                                    studio.winkler_springs.k_theta_y_knm_rad / 1000.0,
                                ))
                                .size(8.5)
                                .monospace()
                                .color(theme.accent_blue),
                            );
                        }
                    } else if tab == 0 {
                        let pg = &studio.pile_group;
                        ui.label(RichText::new("Pile Group Axial Distribution & Take-Off").size(9.8).strong().color(theme.text_primary));
                        ui.label(
                            RichText::new(format!(
                                "Max Pile={:.0} kN | Min Pile={:.0} kN | Gov DCR={:.3}",
                                pg.max_pile_axial_load_kn, pg.min_pile_axial_load_kn, pg.governing_dcr
                            ))
                            .size(8.7)
                            .monospace()
                            .color(if pg.is_group_safe { theme.accent_green } else { theme.accent_red }),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Building: {} Caps, {} Piles | Concrete={:.1} m3 | {}",
                                studio.caps_3d.len(),
                                studio.total_piles_building,
                                studio.total_concrete_m3,
                                if studio.is_overall_compliant { "PASS" } else { "REVIEW" }
                            ))
                            .size(8.5)
                            .monospace()
                            .color(theme.text_secondary),
                        );
                    } else {
                        let ps = &studio.punching_shear;
                        let ssi = &studio.ssi_modal_shift;
                        ui.label(RichText::new("ACI 318 Punching Shear & Wolf/Veletsos-Meek SSI").size(9.8).strong().color(theme.text_primary));
                        ui.label(
                            RichText::new(format!(
                                "bo={:.0} mm | vu={:.3} / phi*Vc={:.3} MPa (DCR={:.3})",
                                ps.critical_perimeter_bo_mm,
                                ps.punching_stress_vu_mpa,
                                ps.concrete_capacity_phi_vc_mpa,
                                ps.punching_dcr,
                            ))
                            .size(8.6)
                            .monospace()
                            .color(if ps.is_punching_safe { theme.accent_green } else { theme.accent_red }),
                        );
                        ui.label(
                            RichText::new(format!(
                                "SSI T={:.2}s -> T~={:.2}s ({:.2}x) | V_base={:.0}% | Drift={:.2}x",
                                ssi.fixed_base_period_t_s,
                                ssi.flexible_ssi_period_t_tilde_s,
                                ssi.period_lengthening_ratio,
                                ssi.base_shear_reduction_ratio * 100.0,
                                ssi.lateral_drift_amplification_ratio,
                            ))
                            .size(8.5)
                            .monospace()
                            .color(theme.text_secondary),
                        );
                    }
                });
            });
        });
    }

    /// Update `v0.8.4.9` (`9/10` -> `v0.8.5`): Width-first Dynamic Island drawer for 4D CPM Sequence, 5D Cost S-Curve & 6D Carbon LCA Studio.
    pub(super) fn render_construction_lca_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        use crate::core::construction_lca_studio::{ConcreteMixGrade, SteelSmeltingRoute};

        let deck_w = (cap_w - 8.0).max(180.0);
        let two_col = !is_mobile && deck_w >= 540.0;
        ui.set_width(deck_w);
        ui.set_max_width(deck_w);

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(deck_w);
            icons::icon_label(
                ui,
                AppIcon::Clock,
                13.0,
                theme.accent_blue,
                "4D/5D/6D Sequence & Carbon LCA",
                11.0,
                theme.text_primary,
                true,
            );
            for (idx, lbl) in [
                (0usize, "4D Sequence & 5D S-Curve"),
                (1usize, "6D Carbon LCA & QTO"),
            ] {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    lbl,
                    self.construction_lca_drawer_tab == idx,
                    theme.accent_blue,
                    "",
                )
                .clicked()
                {
                    self.construction_lca_drawer_tab = idx;
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close 4D/5D/6D Studio drawer",
            )
            .clicked()
            {
                self.construction_lca_drawer_open = false;
            }
        });

        ui.add_space(4.0);
        theme.glass_subpill_frame(8, Margin::symmetric(8, 5)).show(ui, |ui| {
            let strip_w = (deck_w - 18.0).max(150.0);
            ui.set_max_width(strip_w);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(strip_w);
                for (sr, lbl) in [
                    (SteelSmeltingRoute::BfBofStandard, "BF-BOF"),
                    (SteelSmeltingRoute::EafRecycled, "EAF"),
                    (SteelSmeltingRoute::GreenHydrogenDri, "DRI-H2"),
                ] {
                    let active = universal_canvas.construction_lca_config.steel_route == sr;
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        lbl,
                        active,
                        theme.accent_green,
                        "Select structural steel smelting LCA route",
                    )
                    .clicked()
                    {
                        universal_canvas.construction_lca_config.steel_route = sr;
                        actions.run_construction_lca_studio = true;
                    }
                }
                for (cm, lbl) in [
                    (ConcreteMixGrade::OpcStandard, "OPC"),
                    (ConcreteMixGrade::Ggbs30LowCarbon, "GGBS 30%"),
                    (ConcreteMixGrade::GeopolymerGreen, "Geopolymer"),
                ] {
                    let active = universal_canvas.construction_lca_config.concrete_mix == cm;
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        lbl,
                        active,
                        theme.accent_orange,
                        "Select ready-mix concrete binder LCA blend",
                    )
                    .clicked()
                    {
                        universal_canvas.construction_lca_config.concrete_mix = cm;
                        actions.run_construction_lca_studio = true;
                    }
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Bolt),
                    "Run 4D/5D/6D",
                    true,
                    theme.accent_green,
                    "Evaluate CPM schedule, AHSP cost S-curve & EN 15978 carbon LCA",
                )
                .clicked()
                {
                    actions.run_construction_lca_studio = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Layers),
                    if universal_canvas.construction_lca_overlay_visible {
                        "3D 4D-Seq: ON"
                    } else {
                        "3D 4D-Seq: OFF"
                    },
                    universal_canvas.construction_lca_overlay_visible,
                    theme.accent_blue,
                    "Toggle 3D Pure WebGPU story sequence prisms & slewing crane",
                )
                .clicked()
                {
                    actions.toggle_construction_3d = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Document),
                    "IFCX",
                    false,
                    theme.accent_orange,
                    "Export non-destructive eigengraph::analysis::construction_4d5d6d IFCX",
                )
                .clicked()
                {
                    actions.export_construction_ifcx = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Printer),
                    "Sheet S-114",
                    false,
                    theme.accent_blue,
                    "Export A3 Pure-Vector Sheet S-114 4D/5D/6D & Carbon LCA Annex SVG",
                )
                .clicked()
                {
                    actions.export_construction_svg = true;
                }
            });
        });

        ui.add_space(4.0);
        let Some(ref studio) = universal_canvas.last_construction_lca_report else {
            ui.label(
                RichText::new("Click 'Run 4D/5D/6D' or toggle '3D 4D-Seq' to simulate CPM sequence, cost S-curve & EN 15978 carbon LCA.")
                    .size(9.4)
                    .color(theme.text_muted),
            );
            return;
        };

        let tab = self.construction_lca_drawer_tab;
        let max_day = studio.total_duration_days.max(120);
        Self::width_first_pair(ui, deck_w, two_col, |ui, is_left, col_w| {
            theme.glass_subpill_frame(8, Margin::symmetric(8, 6)).show(ui, |ui| {
                let inner_w = (col_w - 20.0).max(150.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.vertical(|ui| {
                    ui.set_max_width(inner_w);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    if is_left {
                        ui.label(RichText::new("4D Timeline, Crews & Logistics Parameters").size(9.8).strong().color(theme.text_primary));
                        let cfg = &mut universal_canvas.construction_lca_config;
                        let mut changed = false;
                        ui.horizontal_wrapped(|ui| {
                            ui.set_max_width(inner_w);
                            ui.spacing_mut().slider_width = (inner_w * 0.38).clamp(58.0, 105.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Day:").size(8.8).color(theme.text_muted));
                                changed |= ui.add(egui::Slider::new(&mut cfg.active_day, 0..=max_day)).changed();
                            });
                            ui.label(RichText::new("Crews:").size(8.8).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.crane_crews).range(1..=6).speed(1.0)).changed();
                            ui.label(RichText::new("Haul (km):").size(8.8).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.transport_distance_km).range(5.0..=1500.0).speed(5.0)).changed();
                            changed |= ui.checkbox(&mut cfg.include_rain_delay, "Site Rain Delay").changed();
                        });
                        if changed {
                            actions.run_construction_lca_studio = true;
                        }
                        if tab == 0 {
                            ui.label(
                                RichText::new(format!(
                                    "Day {}/{} ({:.1}% Physical Progress) • {} Critical Tasks",
                                    studio.config.active_day,
                                    studio.total_duration_days,
                                    studio.physical_progress_pct,
                                    studio.critical_path_count,
                                ))
                                .size(8.6)
                                .monospace()
                                .color(theme.accent_blue),
                            );
                        } else {
                            ui.label(
                                RichText::new(format!(
                                    "Total Carbon: {:.1} tCO2e (-{:.1}% vs BOF/OPC) • {:.1} kg/m2",
                                    studio.total_embodied_carbon_ton,
                                    studio.carbon_reduction_pct,
                                    studio.carbon_intensity_kg_m2,
                                ))
                                .size(8.6)
                                .monospace()
                                .color(theme.accent_green),
                            );
                        }
                    } else if tab == 0 {
                        ui.label(RichText::new("5D AHSP Budget & S-Curve Cashflow").size(9.8).strong().color(theme.text_primary));
                        ui.label(
                            RichText::new(format!(
                                "Total Budget: {} ({} wks)",
                                studio.total_budget_label,
                                studio.scurve.len(),
                            ))
                            .size(8.7)
                            .monospace()
                            .color(theme.accent_green),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Earned Value to Day {}: IDR {:.0}M | QTO: {:.1}t steel, {:.0} m3 concrete",
                                studio.config.active_day,
                                studio.spent_to_date_idr / 1e6,
                                studio.steel_tonnage,
                                studio.concrete_m3,
                            ))
                            .size(8.5)
                            .monospace()
                            .color(theme.text_secondary),
                        );
                    } else {
                        ui.label(RichText::new("6D EN 15978 A1-A5 Stages & Certification").size(9.8).strong().color(theme.text_primary));
                        ui.label(
                            RichText::new(format!(
                                "A1-A3={:.1}t | A4={:.1}t | A5={:.1}t | Baseline={:.1}t",
                                studio.a1_a3_carbon_ton,
                                studio.a4_carbon_ton,
                                studio.a5_carbon_ton,
                                studio.baseline_carbon_ton,
                            ))
                            .size(8.6)
                            .monospace()
                            .color(theme.accent_orange),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Tier: {} • Crane Jib={:.1}m @ {:.0} deg",
                                studio.greenship_leed_tier,
                                studio.crane_3d.jib_length_m,
                                studio.crane_3d.slew_deg,
                            ))
                            .size(8.4)
                            .monospace()
                            .color(theme.accent_green),
                        );
                    }
                });
            });
        });
    }

    /// Update `v0.8.5.6` (`6/10` -> `v0.8.6`): Width-first Dynamic Island drawer for Site Zoning & 3D Buildable Envelope (`KDB/KLB/GSB` + Sun Path).
    pub(super) fn render_zoning_envelope_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        use crate::core::zoning_agent::ZoningDistrictPreset;

        let deck_w = (cap_w - 8.0).max(180.0);
        let two_col = !is_mobile && deck_w >= 540.0;
        ui.set_width(deck_w);
        ui.set_max_width(deck_w);

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(deck_w);
            icons::icon_label(
                ui,
                AppIcon::Compass,
                13.0,
                theme.accent_blue,
                "Site Zoning & 3D Buildable Envelope (KDB / KLB / GSB / Sun Path)",
                11.0,
                theme.text_primary,
                true,
            );
            for (idx, lbl) in [
                (0usize, "Zoning & Envelope"),
                (1usize, "Solar Path & Compliance"),
            ] {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    lbl,
                    self.zoning_drawer_tab == idx,
                    theme.accent_blue,
                    "",
                )
                .clicked()
                {
                    self.zoning_drawer_tab = idx;
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close Site Zoning & Buildable Envelope drawer",
            )
            .clicked()
            {
                self.zoning_drawer_open = false;
            }
        });

        ui.add_space(4.0);
        theme.glass_subpill_frame(8, Margin::symmetric(8, 5)).show(ui, |ui| {
            let strip_w = (deck_w - 18.0).max(150.0);
            ui.set_max_width(strip_w);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(strip_w);
                for preset in ZoningDistrictPreset::ALL {
                    let active = universal_canvas.zoning_config.preset == preset;
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        preset.short_label(),
                        active,
                        theme.accent_green,
                        preset.label(),
                    )
                    .clicked()
                    {
                        universal_canvas.zoning_config.apply_preset(preset);
                        actions.run_zoning_agent = true;
                    }
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Bolt),
                    "Evaluate",
                    true,
                    theme.accent_green,
                    "Evaluate KDB/KLB/GSB limits, buildable envelope & Sun Path",
                )
                .clicked()
                {
                    actions.run_zoning_agent = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Cube3D),
                    if universal_canvas.zoning_overlay_visible {
                        "3D Envelope: ON"
                    } else {
                        "3D Envelope: OFF"
                    },
                    universal_canvas.zoning_overlay_visible,
                    theme.accent_blue,
                    "Toggle 3D Pure WebGPU buildable envelope prisms, setback cage & Sun Path",
                )
                .clicked()
                {
                    actions.toggle_zoning_3d = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Document),
                    "IFCX",
                    false,
                    theme.accent_orange,
                    "Export non-destructive eigengraph::design::zoning_envelope IFCX (.ifcx)",
                )
                .clicked()
                {
                    actions.export_zoning_ifcx = true;
                }
            });
        });

        ui.add_space(4.0);
        let Some(ref rep) = universal_canvas.last_zoning_report.clone() else {
            ui.label(
                RichText::new("Click 'Evaluate' or toggle '3D Envelope' to generate KDB/KLB/GSB limits and Sun Path.")
                    .size(9.4)
                    .color(theme.text_muted),
            );
            return;
        };

        let tab = self.zoning_drawer_tab;
        Self::width_first_pair(ui, deck_w, two_col, |ui, is_left, col_w| {
            theme.glass_subpill_frame(8, Margin::symmetric(8, 6)).show(ui, |ui| {
                let inner_w = (col_w - 20.0).max(150.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.vertical(|ui| {
                    ui.set_max_width(inner_w);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    if is_left {
                        let cfg = &mut universal_canvas.zoning_config;
                        let mut changed = false;
                        if tab == 0 {
                            ui.label(RichText::new("Lot Geometry, KDB/KLB & Setbacks").size(9.8).strong().color(theme.text_primary));
                            ui.horizontal_wrapped(|ui| {
                                ui.set_max_width(inner_w);
                                ui.label(RichText::new("Lot W:").size(8.8).color(theme.text_muted));
                                changed |= ui.add(egui::DragValue::new(&mut cfg.lot_width_m).range(10.0..=400.0).speed(1.0).suffix("m")).changed();
                                ui.label(RichText::new("D:").size(8.8).color(theme.text_muted));
                                changed |= ui.add(egui::DragValue::new(&mut cfg.lot_depth_m).range(10.0..=400.0).speed(1.0).suffix("m")).changed();
                                ui.label(RichText::new("KDB:").size(8.8).color(theme.text_muted));
                                changed |= ui.add(egui::DragValue::new(&mut cfg.kdb_bcr_pct).range(10.0..=90.0).speed(1.0).suffix("%")).changed();
                                ui.label(RichText::new("KLB:").size(8.8).color(theme.text_muted));
                                changed |= ui.add(egui::DragValue::new(&mut cfg.klb_far).range(0.4..=24.0).speed(0.1)).changed();
                                ui.label(RichText::new("KDH:").size(8.8).color(theme.text_muted));
                                changed |= ui.add(egui::DragValue::new(&mut cfg.kdh_green_pct).range(5.0..=70.0).speed(1.0).suffix("%")).changed();
                            });
                            ui.horizontal_wrapped(|ui| {
                                ui.set_max_width(inner_w);
                                ui.label(RichText::new("GSB F/S/R:").size(8.8).color(theme.text_muted));
                                changed |= ui.add(egui::DragValue::new(&mut cfg.setback_front_m).range(0.0..=30.0).speed(0.5).suffix("m")).changed();
                                changed |= ui.add(egui::DragValue::new(&mut cfg.setback_side_m).range(0.0..=25.0).speed(0.5).suffix("m")).changed();
                                changed |= ui.add(egui::DragValue::new(&mut cfg.setback_rear_m).range(0.0..=25.0).speed(0.5).suffix("m")).changed();
                                ui.label(RichText::new("Max H:").size(8.8).color(theme.text_muted));
                                changed |= ui.add(egui::DragValue::new(&mut cfg.max_height_m).range(4.0..=350.0).speed(1.0).suffix("m")).changed();
                                ui.label(RichText::new("Podium:").size(8.8).color(theme.text_muted));
                                changed |= ui.add(egui::DragValue::new(&mut cfg.podium_stories).range(0..=12).speed(1.0).suffix("st")).changed();
                                ui.label(RichText::new("Stepback:").size(8.8).color(theme.text_muted));
                                changed |= ui.add(egui::DragValue::new(&mut cfg.tower_stepback_m).range(0.0..=16.0).speed(0.5).suffix("m")).changed();
                            });
                        } else {
                            ui.label(RichText::new("Analytical Solar Path & Daylight Parameters").size(9.8).strong().color(theme.text_primary));
                            ui.horizontal_wrapped(|ui| {
                                ui.set_max_width(inner_w);
                                ui.spacing_mut().slider_width = (inner_w * 0.36).clamp(56.0, 100.0);
                                ui.label(RichText::new("Month:").size(8.8).color(theme.text_muted));
                                changed |= ui.add(egui::DragValue::new(&mut cfg.solar_month).range(1..=12).speed(1.0)).changed();
                                ui.label(RichText::new("Hour:").size(8.8).color(theme.text_muted));
                                changed |= ui.add(egui::Slider::new(&mut cfg.solar_hour_local, 6.0..=18.0).suffix("h")).changed();
                                changed |= ui.checkbox(&mut cfg.show_sun_path, "Show 3D Sun Path").changed();
                            });
                            ui.label(
                                RichText::new(format!(
                                    "Site: {} ({:.3}°, {:.3}°) • Alt {:.1}° • Az {:.1}° • Shadow {:.2}x",
                                    rep.site_label,
                                    rep.lat_deg,
                                    rep.lon_deg,
                                    rep.active_sun_altitude_deg,
                                    rep.active_sun_azimuth_deg,
                                    rep.shadow_length_factor,
                                ))
                                .size(8.6)
                                .monospace()
                                .color(theme.accent_orange),
                            );
                        }
                        if changed {
                            actions.run_zoning_agent = true;
                        }
                        ui.label(
                            RichText::new(format!(
                                "Lot {:.0} m2 • Pad {:.1}x{:.1}m ({:.0} m2) • Max Footprint {:.0} m2",
                                rep.lot_area_m2,
                                rep.buildable_pad_width_m,
                                rep.buildable_pad_depth_m,
                                rep.buildable_pad_area_m2,
                                rep.effective_footprint_limit_m2,
                            ))
                            .size(8.6)
                            .monospace()
                            .color(theme.accent_blue),
                        );
                    } else if tab == 0 {
                        ui.label(RichText::new("Buildable GFA Capacity & BIM Envelope Audit").size(9.8).strong().color(theme.text_primary));
                        ui.label(
                            RichText::new(format!(
                                "Max GFA (KLB {:.1}): {:.0} m2 | Existing BIM: {:.0} m2 | Remaining: {:.0} m2",
                                rep.config.klb_far,
                                rep.max_gfa_klb_m2,
                                rep.existing_bim_gfa_m2,
                                rep.remaining_gfa_capacity_m2,
                            ))
                            .size(8.6)
                            .monospace()
                            .color(theme.accent_green),
                        );
                        ui.label(
                            RichText::new(format!(
                                "KDB {:.1}%/{:.0}% • Height {:.1}/{:.0}m • Stories {}/{} • Open Space {:.0} m2",
                                rep.actual_kdb_pct,
                                rep.config.kdb_bcr_pct,
                                rep.existing_bim_height_m,
                                rep.config.max_height_m,
                                rep.existing_bim_stories,
                                rep.max_allowable_stories,
                                rep.required_green_kdh_m2,
                            ))
                            .size(8.5)
                            .monospace()
                            .color(if rep.overall_compliant { theme.text_secondary } else { theme.accent_red }),
                        );
                    } else {
                        ui.label(RichText::new("5-Clause Zoning & Solar Exposure Audit").size(9.8).strong().color(theme.text_primary));
                        for item in &rep.compliance_checks {
                            let col = if item.compliant { theme.accent_green } else { theme.accent_red };
                            let tag = if item.compliant { "PASS" } else { "FAIL" };
                            ui.label(
                                RichText::new(format!(
                                    "[{tag}] {}: {:.1}/{:.1}{} — {}",
                                    item.code, item.actual_val, item.limit_val, item.unit, item.detail
                                ))
                                .size(8.4)
                                .monospace()
                                .color(col),
                            );
                        }
                        ui.label(
                            RichText::new(format!(
                                "Daylight Exposure Score: {:.0}%",
                                rep.solar_exposure_score_pct
                            ))
                            .size(8.6)
                            .monospace()
                            .color(theme.accent_blue),
                        );
                    }
                });
            });
        });
    }

    /// Update `v0.8.5.7` (`7/10` -> `v0.8.6`): Width-first Dynamic Island drawer for Spatial Programming, Adjacency & 3D Stack/Pack Studio.
    pub(super) fn render_spatial_program_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        use crate::core::program_agent::{
            BuildingTypologyPreset, DepartmentKind, ProgramSpaceEntry,
        };

        let deck_w = (cap_w - 8.0).max(180.0);
        let two_col = !is_mobile && deck_w >= 540.0;
        ui.set_width(deck_w);
        ui.set_max_width(deck_w);

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(deck_w);
            icons::icon_label(
                ui,
                AppIcon::BoxPackage,
                13.0,
                theme.accent_green,
                "Spatial Program, Adjacency & 3D Stack/Pack Studio",
                11.0,
                theme.text_primary,
                true,
            );
            for (idx, lbl) in [
                (0usize, "Program Table & Stack"),
                (1usize, "Adjacency & Floor-Plate"),
            ] {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    lbl,
                    self.program_drawer_tab == idx,
                    theme.accent_blue,
                    "",
                )
                .clicked()
                {
                    self.program_drawer_tab = idx;
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close Spatial Program & Stack/Pack Studio drawer",
            )
            .clicked()
            {
                self.program_drawer_open = false;
            }
        });

        ui.add_space(4.0);
        theme.glass_subpill_frame(8, Margin::symmetric(8, 5)).show(ui, |ui| {
            let strip_w = (deck_w - 18.0).max(150.0);
            ui.set_max_width(strip_w);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(strip_w);
                for typ in BuildingTypologyPreset::ALL {
                    let active = universal_canvas.program_config.typology == typ;
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        typ.short_label(),
                        active,
                        theme.accent_green,
                        typ.label(),
                    )
                    .clicked()
                    {
                        universal_canvas.program_config.apply_typology(typ);
                        actions.run_spatial_program = true;
                    }
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Bolt),
                    "Stack & Pack",
                    true,
                    theme.accent_green,
                    "Run vertical stacking, 2D floor-plate packing & adjacency check",
                )
                .clicked()
                {
                    actions.run_spatial_program = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Cube3D),
                    if universal_canvas.program_overlay_visible {
                        "3D Program: ON"
                    } else {
                        "3D Program: OFF"
                    },
                    universal_canvas.program_overlay_visible,
                    theme.accent_blue,
                    "Toggle 3D Pure WebGPU stacked & packed department blocks",
                )
                .clicked()
                {
                    actions.toggle_program_3d = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Document),
                    "IFCX",
                    false,
                    theme.accent_orange,
                    "Export non-destructive eigengraph::design::spatial_program IFCX (.ifcx)",
                )
                .clicked()
                {
                    actions.export_program_ifcx = true;
                }
            });
        });

        ui.add_space(4.0);
        let Some(rep) = universal_canvas.last_program_report.clone() else {
            ui.label(
                RichText::new("Click 'Stack & Pack' or toggle '3D Program' to generate stacked floor plates & 2D room packing.")
                    .size(9.4)
                    .color(theme.text_muted),
            );
            return;
        };

        let tab = self.program_drawer_tab;
        Self::width_first_pair(ui, deck_w, two_col, |ui, is_left, col_w| {
            theme.glass_subpill_frame(8, Margin::symmetric(8, 6)).show(ui, |ui| {
                let inner_w = (col_w - 20.0).max(150.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.vertical(|ui| {
                    ui.set_max_width(inner_w);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    if is_left && tab == 0 {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Space Program Schedule (2-Way)").size(9.8).strong().color(theme.text_primary));
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                Some(AppIcon::Plus),
                                "Add Space",
                                false,
                                theme.accent_green,
                                "Append a new programmable space entry",
                            )
                            .clicked()
                            {
                                let next_id = universal_canvas
                                    .program_config
                                    .spaces
                                    .iter()
                                    .map(|s| s.id)
                                    .max()
                                    .unwrap_or(0)
                                    + 1;
                                universal_canvas.program_config.spaces.push(ProgramSpaceEntry {
                                    id: next_id,
                                    name: format!("Custom Zone {next_id}"),
                                    department: DepartmentKind::Office,
                                    unit_count: 4,
                                    area_per_unit_m2: 90.0,
                                    preferred_floor_band: 1,
                                    adjacency_group: 2,
                                });
                                actions.run_spatial_program = true;
                            }
                        });
                        let cfg = &mut universal_canvas.program_config;
                        let mut changed = false;
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Circ%:").size(8.8).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.circulation_factor_pct).range(5.0..=40.0).speed(0.5).suffix("%")).changed();
                            ui.label(RichText::new("Core%:").size(8.8).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.core_ratio_pct).range(6.0..=30.0).speed(0.5).suffix("%")).changed();
                        });
                        let mut remove_idx = None;
                        ScrollArea::vertical()
                            .id_salt("spatial_program_table_scroll")
                            .max_width(inner_w)
                            .max_height(132.0)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for (idx, sp) in cfg.spaces.iter_mut().enumerate() {
                                    ui.horizontal_wrapped(|ui| {
                                        let c = sp.department.color_rgba();
                                        let dept_col = Color32::from_rgb(
                                            (c[0] * 255.0) as u8,
                                            (c[1] * 255.0) as u8,
                                            (c[2] * 255.0) as u8,
                                        );
                                        if Self::glass_pill_button(
                                            ui,
                                            theme,
                                            None,
                                            sp.department.short_label(),
                                            true,
                                            dept_col,
                                            "Click to cycle department",
                                        )
                                        .clicked()
                                        {
                                            sp.department = sp.department.next();
                                            changed = true;
                                        }
                                        ui.label(RichText::new(&sp.name).size(8.8).color(theme.text_primary));
                                        changed |= ui.add(egui::DragValue::new(&mut sp.unit_count).range(1..=200).speed(1.0).suffix("u")).changed();
                                        changed |= ui.add(egui::DragValue::new(&mut sp.area_per_unit_m2).range(6.0..=2000.0).speed(2.0).suffix("m2")).changed();
                                        ui.label(RichText::new(format!("={:.0}", sp.total_area_m2())).size(8.4).monospace().color(theme.text_muted));
                                        if cfg.spaces.len() > 1 && ui.small_button("x").clicked() {
                                            remove_idx = Some(idx);
                                        }
                                    });
                                }
                            });
                        if let Some(idx) = remove_idx {
                            cfg.spaces.remove(idx);
                            changed = true;
                        }
                        if changed {
                            actions.run_spatial_program = true;
                        }
                    } else if !is_left && tab == 0 {
                        ui.label(RichText::new("Stacking Metrics & Story Filter").size(9.8).strong().color(theme.text_primary));
                        ui.label(
                            RichText::new(format!(
                                "NFA {:.0} m2 • GFA {:.0} / {:.0} m2 ({:.0}% KLB) • Eff {:.1}% • {} Units",
                                rep.total_nfa_m2,
                                rep.total_gfa_m2,
                                rep.zoning_max_gfa_m2,
                                rep.gfa_utilization_pct,
                                rep.overall_efficiency_pct,
                                rep.total_units,
                            ))
                            .size(8.6)
                            .monospace()
                            .color(if rep.fits_zoning_envelope { theme.accent_green } else { theme.accent_red }),
                        );
                        ui.horizontal_wrapped(|ui| {
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                None,
                                "All Stories",
                                universal_canvas.program_config.selected_story_filter.is_none(),
                                theme.accent_blue,
                                "Show all stacked stories at full opacity",
                            )
                            .clicked()
                            {
                                universal_canvas.program_config.selected_story_filter = None;
                            }
                            for st in rep.story_summaries.iter().take(12) {
                                let act = universal_canvas.program_config.selected_story_filter
                                    == Some(st.story_index);
                                if Self::glass_pill_button(
                                    ui,
                                    theme,
                                    None,
                                    format!("L{} {}", st.story_index + 1, st.primary_department),
                                    act,
                                    theme.accent_green,
                                    &format!(
                                        "Floor L{}: Net {:.0} m2 / Gross {:.0} m2 ({:.0}% eff)",
                                        st.story_index + 1,
                                        st.net_program_area_m2,
                                        st.gross_plate_area_m2,
                                        st.efficiency_pct
                                    ),
                                )
                                .clicked()
                                {
                                    universal_canvas.program_config.selected_story_filter =
                                        Some(st.story_index);
                                }
                            }
                        });
                    } else if is_left && tab == 1 {
                        let focus_story = universal_canvas
                            .program_config
                            .selected_story_filter
                            .unwrap_or(0)
                            .min(rep.stacked_stories.saturating_sub(1));
                        ui.label(
                            RichText::new(format!("2D Packed Floor Plate — Story L{}", focus_story + 1))
                                .size(9.8)
                                .strong()
                                .color(theme.text_primary),
                        );
                        let plan_w = inner_w.min(360.0);
                        let plan_h = 122.0_f32;
                        let (resp, painter) =
                            ui.allocate_painter(Vec2::new(plan_w, plan_h), egui::Sense::hover());
                        let r = resp.rect;
                        painter.rect_filled(r, CornerRadius::same(6), theme.bg_card.gamma_multiply(0.55));
                        let blocks: Vec<_> = rep
                            .packed_blocks_3d
                            .iter()
                            .filter(|b| b.story_index == focus_story)
                            .collect();
                        if !blocks.is_empty() {
                            let min_x = blocks.iter().map(|b| b.min_enu_m[0]).fold(f64::INFINITY, f64::min);
                            let max_x = blocks.iter().map(|b| b.max_enu_m[0]).fold(f64::NEG_INFINITY, f64::max);
                            let min_y = blocks.iter().map(|b| b.min_enu_m[1]).fold(f64::INFINITY, f64::min);
                            let max_y = blocks.iter().map(|b| b.max_enu_m[1]).fold(f64::NEG_INFINITY, f64::max);
                            let span_x = (max_x - min_x).max(1.0);
                            let span_y = (max_y - min_y).max(1.0);
                            let pad = 8.0_f32;
                            let avail_w = (r.width() - 2.0 * pad).max(20.0);
                            let avail_h = (r.height() - 2.0 * pad).max(20.0);
                            let sc = (avail_w as f64 / span_x).min(avail_h as f64 / span_y);
                            let ox = r.center().x - ((span_x * sc) as f32) * 0.5;
                            let oy = r.center().y + ((span_y * sc) as f32) * 0.5;
                            for b in blocks {
                                let x0 = ox + ((b.min_enu_m[0] - min_x) * sc) as f32;
                                let x1 = ox + ((b.max_enu_m[0] - min_x) * sc) as f32;
                                let y0 = oy - ((b.max_enu_m[1] - min_y) * sc) as f32;
                                let y1 = oy - ((b.min_enu_m[1] - min_y) * sc) as f32;
                                let br = egui::Rect::from_min_max(
                                    egui::Pos2::new(x0, y0),
                                    egui::Pos2::new(x1, y1),
                                );
                                let c = b.color_rgba;
                                let col = Color32::from_rgb(
                                    (c[0] * 255.0) as u8,
                                    (c[1] * 255.0) as u8,
                                    (c[2] * 255.0) as u8,
                                );
                                painter.rect_filled(br, CornerRadius::same(2), col.gamma_multiply(0.32));
                                painter.rect_stroke(br, CornerRadius::same(2), Stroke::new(1.0, col), egui::StrokeKind::Middle);
                                if br.width() > 36.0 && br.height() > 14.0 {
                                    painter.text(
                                        br.center(),
                                        Align2::CENTER_CENTER,
                                        b.department.short_label(),
                                        egui::FontId::proportional(8.5),
                                        theme.text_primary,
                                    );
                                }
                            }
                        }
                    } else {
                        ui.label(
                            RichText::new(format!("Adjacency Matrix Score: {:.0}%", rep.adjacency_score_pct))
                                .size(9.8)
                                .strong()
                                .color(if rep.adjacency_score_pct >= 80.0 {
                                    theme.accent_green
                                } else {
                                    theme.accent_orange
                                }),
                        );
                        for ap in &rep.adjacency_pairs {
                            let col = if ap.satisfied { theme.accent_green } else { theme.accent_orange };
                            let tag = if ap.satisfied { "PASS" } else { "WARN" };
                            ui.label(
                                RichText::new(format!(
                                    "[{tag}] {} <-> {} (dL={}) — {}",
                                    ap.from_dept, ap.to_dept, ap.story_delta, ap.rationale
                                ))
                                .size(8.4)
                                .monospace()
                                .color(col),
                            );
                        }
                    }
                });
            });
        });
    }

    /// Update `v0.8.5.8` (`8/10` -> `v0.8.6`): Multi-Option Massing Explorer & 1-Click Mass-to-Engineered-BIM Studio Drawer.
    pub(super) fn render_massing_study_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        use crate::core::massing_agent::{MassingArchetype, MassingStructuralSystem};

        let deck_w = (cap_w - 8.0).max(180.0);
        let two_col = !is_mobile && deck_w >= 540.0;
        ui.set_width(deck_w);
        ui.set_max_width(deck_w);

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(deck_w);
            icons::icon_label(
                ui,
                AppIcon::Building,
                13.0,
                theme.accent_blue,
                "Multi-Option Massing Explorer & 1-Click Mass-to-BIM",
                11.0,
                theme.text_primary,
                true,
            );
            for (idx, lbl) in [
                (0usize, "Options & Mass-to-BIM"),
                (1usize, "Grid, Framing & Trade-Off"),
            ] {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    lbl,
                    self.massing_drawer_tab == idx,
                    theme.accent_blue,
                    "",
                )
                .clicked()
                {
                    self.massing_drawer_tab = idx;
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close Multi-Option Massing Explorer drawer",
            )
            .clicked()
            {
                self.massing_drawer_open = false;
            }
        });

        ui.add_space(4.0);
        theme.glass_subpill_frame(8, Margin::symmetric(8, 5)).show(ui, |ui| {
            let strip_w = (deck_w - 18.0).max(150.0);
            ui.set_max_width(strip_w);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(strip_w);
                for arch in MassingArchetype::ALL {
                    let active = universal_canvas.massing_config.selected_archetype == arch;
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        arch.short_label(),
                        active,
                        theme.accent_blue,
                        arch.label(),
                    )
                    .clicked()
                    {
                        universal_canvas.massing_config.selected_archetype = arch;
                        universal_canvas.massing_overlay_visible = true;
                        actions.run_massing_study = true;
                    }
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Recycle),
                    "Evaluate",
                    false,
                    theme.accent_blue,
                    "Re-evaluate all 4 massing archetypes against zoning, program & Earth hazards",
                )
                .clicked()
                {
                    actions.run_massing_study = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Cube3D),
                    if universal_canvas.massing_overlay_visible {
                        "3D Mass: ON"
                    } else {
                        "3D Mass: OFF"
                    },
                    universal_canvas.massing_overlay_visible,
                    theme.accent_blue,
                    "Toggle 3D Pure WebGPU Multi-Option Massing Explorer overlay",
                )
                .clicked()
                {
                    actions.toggle_massing_3d = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Document),
                    "IFCX",
                    false,
                    theme.accent_orange,
                    "Export non-destructive eigengraph::design::massing_study IFCX (.ifcx)",
                )
                .clicked()
                {
                    actions.export_massing_ifcx = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Document),
                    "Sheet S-115",
                    false,
                    theme.accent_green,
                    "Export Archival A3 Sheet S-115 Zoning, Spatial Program & Feasibility Brief (.svg)",
                )
                .clicked()
                {
                    actions.export_feasibility_s115 = true;
                }
            });
        });

        ui.add_space(4.0);
        let Some(rep) = universal_canvas.last_massing_report.clone() else {
            ui.label(
                RichText::new("Click 'Evaluate' or pick a massing option above to compare GFA, EigenGate ARI, DCR%, Cost & CO2.")
                    .size(9.4)
                    .color(theme.text_muted),
            );
            return;
        };

        let tab = self.massing_drawer_tab;
        Self::width_first_pair(ui, deck_w, two_col, |ui, is_left, col_w| {
            theme.glass_subpill_frame(8, Margin::symmetric(8, 6)).show(ui, |ui| {
                let inner_w = (col_w - 20.0).max(150.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.vertical(|ui| {
                    ui.set_max_width(inner_w);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    if is_left && tab == 0 {
                        ui.label(
                            RichText::new("4-Option Architectural Massing Cards")
                                .size(9.8)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.add_space(2.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.set_max_width(inner_w);
                            for (idx, opt) in rep.options.iter().enumerate() {
                                let selected = idx == rep.selected_index;
                                let is_rec = idx == rep.recommended_index;
                                let c = opt.archetype.accent_rgba();
                                let accent = Color32::from_rgb(
                                    (c[0] * 255.0) as u8,
                                    (c[1] * 255.0) as u8,
                                    (c[2] * 255.0) as u8,
                                );
                                let badge = if is_rec { " [REC]" } else { "" };
                                let btn_lbl = format!(
                                    "{}{badge} • {}F • {:.0}m² • ARI {:.2}",
                                    opt.archetype.short_label(),
                                    opt.stories,
                                    opt.gfa_m2,
                                    opt.estimated_eigengate_ari,
                                );
                                if Self::glass_pill_button(
                                    ui,
                                    theme,
                                    None,
                                    btn_lbl,
                                    selected,
                                    accent,
                                    &opt.rationale,
                                )
                                .clicked()
                                {
                                    universal_canvas.massing_config.selected_archetype = opt.archetype;
                                    universal_canvas.massing_overlay_visible = true;
                                    actions.run_massing_study = true;
                                }
                            }
                        });
                        ui.add_space(4.0);
                        if let Some(active) = rep.active_option() {
                            ui.label(
                                RichText::new(&active.rationale)
                                    .size(8.6)
                                    .color(theme.text_secondary),
                            );
                            ui.add_space(4.0);
                        }
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            Some(AppIcon::CheckCircle),
                            "1-Click Convert Selected Mass -> Engineered BIM + EigenGate",
                            true,
                            theme.accent_green,
                            "Synthesize full parametric levels, grids, columns, beams, shear-wall core, compound walls with windows/doors, slabs & footings",
                        )
                        .clicked()
                        {
                            actions.adopt_massing_to_bim = Some(rep.selected_index);
                        }
                    } else if !is_left && tab == 0 {
                        ui.label(
                            RichText::new("Multi-Option Engineering & Cost/Carbon Matrix")
                                .size(9.8)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ScrollArea::vertical()
                            .id_salt("massing_matrix_scroll")
                            .max_width(inner_w)
                            .max_height(155.0)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for (idx, opt) in rep.options.iter().enumerate() {
                                    let is_sel = idx == rep.selected_index;
                                    let col = if is_sel {
                                        theme.accent_blue
                                    } else if opt.zoning_compliant {
                                        theme.text_primary
                                    } else {
                                        theme.accent_red
                                    };
                                    let pass_tag = if opt.zoning_compliant { "OK" } else { "LIM" };
                                    ui.label(
                                        RichText::new(format!(
                                            "{} [{}] {}F ({:.1}m) | GFA {:.0} | KLB {:.2} | ARI {:.2} | DCR {:.0}% | ${:.1}M / Rp{:.1}B | {:.0}tCO2 | {:.0}pts",
                                            if is_sel { ">" } else { " " },
                                            pass_tag,
                                            opt.archetype.short_label(),
                                            opt.stories,
                                            opt.total_height_m,
                                            opt.gfa_m2,
                                            opt.klb_far,
                                            opt.estimated_eigengate_ari,
                                            opt.estimated_max_dcr_pct,
                                            opt.estimated_cost_usd / 1_000_000.0,
                                            opt.estimated_cost_idr_billion,
                                            opt.embodied_carbon_tco2e,
                                            opt.composite_score,
                                        ))
                                        .size(8.3)
                                        .monospace()
                                        .color(col),
                                    );
                                }
                            });
                        ui.label(
                            RichText::new(&rep.summary_verdict)
                                .size(8.4)
                                .monospace()
                                .color(theme.accent_green),
                        );
                    } else if is_left && tab == 1 {
                        ui.label(
                            RichText::new("Structural Framing & Facade Grid Parameters")
                                .size(9.8)
                                .strong()
                                .color(theme.text_primary),
                        );
                        let cfg = &mut universal_canvas.massing_config;
                        let mut changed = false;
                        ui.horizontal_wrapped(|ui| {
                            for sys in MassingStructuralSystem::ALL {
                                if Self::glass_pill_button(
                                    ui,
                                    theme,
                                    None,
                                    sys.short_label(),
                                    cfg.structural_system == sys,
                                    theme.accent_blue,
                                    sys.label(),
                                )
                                .clicked()
                                {
                                    cfg.structural_system = sys;
                                    changed = true;
                                }
                            }
                        });
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("BayX:").size(8.6).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.bay_x_m).range(4.8..=10.8).speed(0.2).suffix("m")).changed();
                            ui.label(RichText::new("BayY:").size(8.6).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.bay_y_m).range(4.8..=10.8).speed(0.2).suffix("m")).changed();
                        });
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Core:").size(8.6).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.core_wall_thick_mm).range(200.0..=500.0).speed(10.0).suffix("mm")).changed();
                            ui.label(RichText::new("WWR:").size(8.6).color(theme.text_muted));
                            changed |= ui.add(egui::DragValue::new(&mut cfg.facade_wwr_pct).range(15.0..=75.0).speed(1.0).suffix("%")).changed();
                        });
                        ui.horizontal_wrapped(|ui| {
                            changed |= ui.checkbox(&mut cfg.include_basement_footings, "Pad Footings").changed();
                            changed |= ui.checkbox(&mut cfg.show_structural_grid_preview, "3D Grid Preview").changed();
                        });
                        if changed {
                            actions.run_massing_study = true;
                        }
                    } else {
                        ui.label(
                            RichText::new("Trade-Off Vector Chart (Composite Score & ARI)")
                                .size(9.8)
                                .strong()
                                .color(theme.text_primary),
                        );
                        let plot_h = 112.0_f32;
                        let (plot_rect, _) = ui.allocate_exact_size(
                            Vec2::new(inner_w, plot_h),
                            egui::Sense::hover(),
                        );
                        let painter = ui.painter_at(plot_rect);
                        painter.rect_filled(plot_rect, CornerRadius::same(6), theme.card_bg_subtle);
                        let row_h = (plot_h - 14.0) / 4.0;
                        for (idx, opt) in rep.options.iter().enumerate() {
                            let y = plot_rect.top() + 8.0 + (idx as f32) * row_h;
                            let bar_left = plot_rect.left() + 84.0;
                            let bar_max_w = (plot_rect.width() - 130.0).max(20.0);
                            let frac = (opt.composite_score / 100.0).clamp(0.05, 1.0) as f32;
                            let c = opt.archetype.accent_rgba();
                            let col = Color32::from_rgb(
                                (c[0] * 255.0) as u8,
                                (c[1] * 255.0) as u8,
                                (c[2] * 255.0) as u8,
                            );
                            painter.text(
                                egui::Pos2::new(plot_rect.left() + 6.0, y + 6.0),
                                Align2::LEFT_CENTER,
                                opt.archetype.short_label(),
                                egui::FontId::proportional(8.4),
                                if idx == rep.selected_index {
                                    theme.text_primary
                                } else {
                                    theme.text_secondary
                                },
                            );
                            let bar_r = egui::Rect::from_min_size(
                                egui::Pos2::new(bar_left, y + 2.0),
                                Vec2::new(bar_max_w * frac, 10.0),
                            );
                            painter.rect_filled(bar_r, CornerRadius::same(3), col);
                            painter.text(
                                egui::Pos2::new(bar_r.right() + 6.0, y + 7.0),
                                Align2::LEFT_CENTER,
                                format!("{:.0}", opt.composite_score),
                                egui::FontId::monospace(8.2),
                                col,
                            );
                        }
                    }
                });
            });
        });
    }

    /// Update `v0.8.5.9` (`9/10` -> `v0.8.6`): Chained 6-Stage AI Building Design Agent & Feasibility Brief Drawer.
    pub(super) fn render_design_agent_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        let deck_w = (cap_w - 8.0).max(180.0);
        let two_col = !is_mobile && deck_w >= 540.0;
        ui.set_width(deck_w);
        ui.set_max_width(deck_w);

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(deck_w);
            icons::icon_label(
                ui,
                AppIcon::Sparkles,
                13.0,
                theme.accent_blue,
                "AI Building Design Agent (6-Stage Concept-to-BIM)",
                11.0,
                theme.text_primary,
                true,
            );
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Bolt),
                "Run 6-Stage Pipeline",
                true,
                theme.accent_green,
                "Execute Brief -> Zoning -> Program -> Massing -> Mass-to-BIM -> Live EigenGate Audit",
            )
            .clicked()
            {
                actions.run_design_pipeline = Some(self.design_agent_brief_draft.clone());
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Document),
                "Sheet S-115",
                false,
                theme.accent_blue,
                "Export Archival A3 Sheet S-115 Zoning, Spatial Program & Feasibility Brief (.svg)",
            )
            .clicked()
            {
                actions.export_feasibility_s115 = true;
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Document),
                "IFCX",
                false,
                theme.accent_orange,
                "Export non-destructive eigengraph::design::concept_pipeline IFCX (.ifcx)",
            )
            .clicked()
            {
                actions.export_design_ifcx = true;
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close AI Building Design Agent drawer",
            )
            .clicked()
            {
                self.design_agent_drawer_open = false;
            }
        });

        ui.add_space(4.0);
        Self::width_first_pair(ui, deck_w, two_col, |ui, is_left, col_w| {
            theme.glass_subpill_frame(8, Margin::symmetric(8, 6)).show(ui, |ui| {
                let inner_w = (col_w - 20.0).max(150.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.vertical(|ui| {
                    ui.set_max_width(inner_w);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    if is_left {
                        ui.label(
                            RichText::new("Architectural Brief / RFP Input & Quick Presets")
                                .size(9.8)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.add_space(2.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.set_max_width(inner_w);
                            for (lbl, brief_txt) in [
                                (
                                    "CBD Mixed 14F",
                                    "Design 14-story CBD mixed-use podium tower on 60x45m lot, KDB 65%, KLB 7.5, bay 7.8m composite steel",
                                ),
                                (
                                    "Office HQ 10F",
                                    "Design 10-story corporate office courtyard on 54x42m lot, KDB 60%, KLB 5.0, bay 7.2m",
                                ),
                                (
                                    "Hospital 8F",
                                    "Design 8-story clinical hospital linear slab on 64x48m lot, KDB 50%, bay 8.4m",
                                ),
                                (
                                    "Campus 6F",
                                    "Design 6-story university campus stepped terrace on 56x40m lot, KDB 45%, bay 7.2m",
                                ),
                            ] {
                                if Self::glass_pill_button(
                                    ui,
                                    theme,
                                    None,
                                    lbl,
                                    false,
                                    theme.accent_blue,
                                    brief_txt,
                                )
                                .clicked()
                                {
                                    self.design_agent_brief_draft = brief_txt.to_string();
                                    actions.run_design_pipeline = Some(self.design_agent_brief_draft.clone());
                                }
                            }
                        });
                        ui.add_space(4.0);
                        let resp = ui.add(
                            egui::TextEdit::multiline(&mut self.design_agent_brief_draft)
                                .desired_rows(3)
                                .desired_width(inner_w),
                        );
                        if resp.has_focus()
                            && ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter))
                        {
                            actions.run_design_pipeline = Some(self.design_agent_brief_draft.clone());
                        }
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                Some(AppIcon::Compass),
                                "Zoning Studio",
                                false,
                                theme.accent_green,
                                "Inspect Stage 2 Site Zoning & Sun Path",
                            )
                            .clicked()
                            {
                                self.open_zoning_drawer(0);
                            }
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                Some(AppIcon::BoxPackage),
                                "Program Studio",
                                false,
                                theme.accent_green,
                                "Inspect Stage 3 Spatial Program & 2D Floor-Plate Packing",
                            )
                            .clicked()
                            {
                                self.open_program_drawer(0);
                            }
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                Some(AppIcon::Building),
                                "Massing Explorer",
                                false,
                                theme.accent_blue,
                                "Inspect Stage 4 Multi-Option Massing Trade-Off",
                            )
                            .clicked()
                            {
                                self.open_massing_drawer(0);
                            }
                        });
                    } else {
                        ui.label(
                            RichText::new("6-Stage Pipeline Execution Trace & Live Audit")
                                .size(9.8)
                                .strong()
                                .color(theme.text_primary),
                        );
                        if let Some(ref rep) = universal_canvas.last_design_pipeline_report {
                            for st in &rep.steps {
                                let col = if st.status_ok {
                                    theme.accent_green
                                } else {
                                    theme.accent_orange
                                };
                                let tag = if st.status_ok { "PASS" } else { "WARN" };
                                ui.label(
                                    RichText::new(format!(
                                        "[{tag}] {}. {} — {}",
                                        st.stage_index, st.stage_name, st.metric_summary
                                    ))
                                    .size(8.4)
                                    .monospace()
                                    .color(col),
                                );
                            }
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(&rep.summary_verdict)
                                    .size(8.5)
                                    .monospace()
                                    .color(if rep.overall_ok {
                                        theme.accent_green
                                    } else {
                                        theme.accent_orange
                                    }),
                            );
                        } else {
                            ui.label(
                                RichText::new("Click 'Run 6-Stage Pipeline' or pick a quick RFP preset to chain Zoning -> Program -> Massing -> Mass-to-BIM -> Live EigenGate Audit.")
                                    .size(8.8)
                                    .color(theme.text_muted),
                            );
                        }
                    }
                });
            });
        });
    }
}
