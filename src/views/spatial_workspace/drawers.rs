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
}
