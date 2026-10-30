//! updater.rs — In-App GitHub Releases Auto-Updater for EigenGraph on macOS.
//! Checks GitHub Releases API for new DMG releases, downloads directly, and launches the installer.

use std::sync::{Arc, Mutex};
use std::thread;
use std::process::Command;
use egui::{Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, RichText, Stroke, Vec2};
use crate::theme::MacTheme;

#[derive(Clone, Debug, Default)]
pub struct UpdateReleaseDetails {
    pub tag_name: String,
    pub name: String,
    pub body: String,
    pub published_at: String,
    pub dmg_url: String,
    pub dmg_size_mb: f32,
}

#[derive(Clone, Debug)]
pub struct UpdaterSharedState {
    pub is_checking: bool,
    pub is_downloading: bool,
    pub download_progress: f32,
    pub status_message: Option<String>,
    pub update_available: bool,
    pub release: Option<UpdateReleaseDetails>,
    pub downloaded_path: Option<String>,
    pub error: Option<String>,
}

impl Default for UpdaterSharedState {
    fn default() -> Self {
        Self {
            is_checking: false,
            is_downloading: false,
            download_progress: 0.0,
            status_message: None,
            update_available: false,
            release: None,
            downloaded_path: None,
            error: None,
        }
    }
}

pub struct AppUpdater {
    pub current_version: String,
    pub shared: Arc<Mutex<UpdaterSharedState>>,
    pub show_modal: bool,
}

impl Default for AppUpdater {
    fn default() -> Self {
        Self {
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            shared: Arc::new(Mutex::new(UpdaterSharedState::default())),
            show_modal: false,
        }
    }
}

impl AppUpdater {
    pub fn new() -> Self {
        Self::default()
    }

    /// Triggers an asynchronous check for updates against the GitHub Releases API.
    pub fn check_for_updates(&mut self) {
        let current_ver = self.current_version.clone();
        let shared = Arc::clone(&self.shared);

        {
            let mut state = shared.lock().unwrap();
            state.is_checking = true;
            state.error = None;
            state.status_message = Some("Memeriksa pembaruan ke GitHub Releases...".to_string());
        }

        self.show_modal = true;

        thread::spawn(move || {
            let api_url = "https://api.github.com/repos/haridsirawan/EigenGraph/releases/latest";
            
            // Query GitHub Releases using native macOS curl (supports system proxy & keychain)
            let output = Command::new("curl")
                .arg("-s")
                .arg("-L")
                .arg("--max-time")
                .arg("10")
                .arg("-H")
                .arg("User-Agent: EigenGraph-Desktop-App")
                .arg("-H")
                .arg("Accept: application/vnd.github.v3+json")
                .arg(api_url)
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    let response_text = String::from_utf8_lossy(&out.stdout);
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
                        if let Some(tag_name) = json.get("tag_name").and_then(|v| v.as_str()) {
                            let clean_tag = tag_name.trim_start_matches('v');
                            let clean_current = current_ver.trim_start_matches('v');
                            
                            let is_newer = is_version_newer(clean_tag, clean_current);
                            let title = json.get("name").and_then(|v| v.as_str()).unwrap_or(tag_name).to_string();
                            let body = json.get("body").and_then(|v| v.as_str()).unwrap_or("Pembaruan dan penyempurnaan sistem EigenGraph.").to_string();
                            let published_at = json.get("published_at").and_then(|v| v.as_str()).unwrap_or("").to_string();

                            // Search for DMG asset in release assets array
                            let mut dmg_download_url = String::new();
                            let mut dmg_size_mb = 4.2;

                            if let Some(assets) = json.get("assets").and_then(|v| v.as_array()) {
                                for asset in assets {
                                    if let Some(name) = asset.get("name").and_then(|v| v.as_str()) {
                                        if name.ends_with(".dmg") {
                                            if let Some(url) = asset.get("browser_download_url").and_then(|v| v.as_str()) {
                                                dmg_download_url = url.to_string();
                                            }
                                            if let Some(size) = asset.get("size").and_then(|v| v.as_f64()) {
                                                dmg_size_mb = (size / 1_048_576.0) as f32;
                                            }
                                            break;
                                        }
                                    }
                                }
                            }

                            // Fallback direct download link if assets array empty
                            if dmg_download_url.is_empty() {
                                dmg_download_url = format!("https://github.com/haridsirawan/EigenGraph/releases/download/{}/EigenGraph-{}.dmg", tag_name, tag_name);
                            }

                            let mut state = shared.lock().unwrap();
                            state.is_checking = false;
                            state.update_available = is_newer;
                            state.release = Some(UpdateReleaseDetails {
                                tag_name: tag_name.to_string(),
                                name: title,
                                body,
                                published_at,
                                dmg_url: dmg_download_url,
                                dmg_size_mb,
                            });

                            if is_newer {
                                state.status_message = Some(format!("Versi baru {} tersedia!", tag_name));
                            } else {
                                state.status_message = Some(format!("EigenGraph sudah menggunakan versi terbaru (v{}).", clean_current));
                            }
                        } else {
                            // Repo might be private or release not created yet
                            let mut state = shared.lock().unwrap();
                            state.is_checking = false;
                            state.status_message = Some(format!("EigenGraph v{} adalah rilis aktif saat ini.", current_ver));
                        }
                    } else {
                        let mut state = shared.lock().unwrap();
                        state.is_checking = false;
                        state.error = Some("Respon rilis GitHub tidak valid.".to_string());
                    }
                }
                Ok(out) => {
                    let mut state = shared.lock().unwrap();
                    state.is_checking = false;
                    state.error = Some(format!("Gagal menghubungkan ke GitHub (Exit code {}).", out.status.code().unwrap_or(-1)));
                }
                Err(e) => {
                    let mut state = shared.lock().unwrap();
                    state.is_checking = false;
                    state.error = Some(format!("Koneksi jaringan error: {}", e));
                }
            }
        });
    }

    /// Downloads the DMG from GitHub Releases directly to ~/Downloads/ and executes `open` to mount installer.
    pub fn start_download_and_install(&mut self) {
        let shared = Arc::clone(&self.shared);
        let dmg_url = {
            let state = shared.lock().unwrap();
            state.release.as_ref().map(|r| (r.dmg_url.clone(), r.tag_name.clone()))
        };

        if let Some((url, tag)) = dmg_url {
            {
                let mut state = shared.lock().unwrap();
                state.is_downloading = true;
                state.download_progress = 0.10;
                state.status_message = Some("Mengunduh rilis DMG dari GitHub...".to_string());
            }

            thread::spawn(move || {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                let download_target = format!("{}/Downloads/EigenGraph-{}.dmg", home, tag);

                let res = Command::new("curl")
                    .arg("-L")
                    .arg("-o")
                    .arg(&download_target)
                    .arg(&url)
                    .output();

                match res {
                    Ok(out) if out.status.success() => {
                        {
                            let mut state = shared.lock().unwrap();
                            state.is_downloading = false;
                            state.download_progress = 1.0;
                            state.downloaded_path = Some(download_target.clone());
                            state.status_message = Some("Unduhan selesai! Membuka installer disk image...".to_string());
                        }

                        // Automatically mount the downloaded DMG on macOS
                        let _ = Command::new("open")
                            .arg(&download_target)
                            .spawn();
                    }
                    Ok(_) | Err(_) => {
                        let mut state = shared.lock().unwrap();
                        state.is_downloading = false;
                        state.error = Some("Gagal mengunduh file DMG dari GitHub. Periksa koneksi internet Anda.".to_string());
                    }
                }
            });
        }
    }

    /// Renders the macOS native modal for Software Updates.
    pub fn render_modal(&mut self, ctx: &egui::Context, theme: &MacTheme) {
        if !self.show_modal {
            return;
        }

        let mut close_modal = false;
        let mut trigger_download = false;
        let mut trigger_recheck = false;

        let state = {
            let s = self.shared.lock().unwrap();
            s.clone()
        };

        egui::Window::new("Pembaruan Perangkat Lunak EigenGraph")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, 0.0))
            .fixed_size(Vec2::new(460.0, 320.0))
            .frame(Frame::new()
                .fill(theme.bg_card)
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(Margin::same(20)))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🚀").size(32.0));
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new("EigenGraph Auto-Updater").size(16.0).strong().color(theme.text_primary));
                            ui.label(RichText::new(format!("Versi Terpasang Saat Ini: v{}", self.current_version)).size(12.0).color(theme.text_secondary));
                        });
                    });

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(12.0);

                    if state.is_checking {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.add_space(6.0);
                            ui.label(RichText::new("Sedang memeriksa server rilis GitHub...").size(12.5).color(theme.text_secondary));
                        });
                    } else if state.is_downloading {
                        ui.label(RichText::new("Mengunduh installer rilis terbaru dari GitHub...").size(12.5).strong().color(theme.accent_blue));
                        ui.add_space(6.0);
                        let progress_bar = egui::ProgressBar::new(state.download_progress)
                            .show_percentage()
                            .animate(true);
                        ui.add(progress_bar);
                        ui.add_space(4.0);
                        ui.label(RichText::new("Setelah unduhan selesai, disk image DMG akan dibuka otomatis.").size(11.0).color(theme.text_muted));
                    } else if let Some(ref release) = state.release {
                        if state.update_available {
                            Frame::new()
                                .fill(theme.accent_green.gamma_multiply(0.12))
                                .stroke(Stroke::new(1.0, theme.accent_green))
                                .corner_radius(CornerRadius::same(8))
                                .inner_margin(Margin::same(10))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("✨").size(16.0));
                                        ui.vertical(|ui| {
                                            ui.label(RichText::new(format!("Versi Baru Tersedia: {}", release.tag_name)).size(13.5).strong().color(theme.accent_green));
                                            ui.label(RichText::new(format!("Ukuran: {:.1} MB • Rilis: {}", release.dmg_size_mb, release.published_at.chars().take(10).collect::<String>())).size(11.0).color(theme.text_secondary));
                                        });
                                    });
                                });

                            ui.add_space(8.0);
                            ui.label(RichText::new("Catatan Rilis / Changelog:").size(11.5).strong().color(theme.text_secondary));
                            egui::ScrollArea::vertical().max_height(80.0).show(ui, |ui| {
                                ui.label(RichText::new(&release.body).size(11.0).color(theme.text_muted));
                            });
                        } else {
                            Frame::new()
                                .fill(theme.bg_canvas)
                                .stroke(Stroke::new(1.0, theme.border_subtle))
                                .corner_radius(CornerRadius::same(8))
                                .inner_margin(Margin::same(12))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("✅").size(20.0));
                                        ui.vertical(|ui| {
                                            ui.label(RichText::new("EigenGraph Sudah Versi Terbaru!").size(13.0).strong().color(theme.accent_green));
                                            ui.label(RichText::new(format!("Anda sudah menjalankan rilis resmi terbaru (v{}). Tidak ada pembaruan yang diperlukan.", self.current_version)).size(11.5).color(theme.text_secondary));
                                        });
                                    });
                                });
                        }
                    } else if let Some(ref err) = state.error {
                        ui.label(RichText::new(format!("⚠️ {}", err)).size(12.0).color(theme.accent_red));
                    } else if let Some(ref msg) = state.status_message {
                        ui.label(RichText::new(msg).size(12.0).color(theme.text_secondary));
                    }

                    if let Some(ref path) = state.downloaded_path {
                        ui.add_space(8.0);
                        ui.label(RichText::new(format!("✅ File DMG tersimpan di: {}", path)).size(10.5).color(theme.accent_green));
                    }

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Modal Action Buttons Row
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if state.update_available && !state.is_downloading {
                            if ui.add(egui::Button::new(RichText::new("🚀 Unduh & Pasang Sekarang").size(12.0).strong().color(Color32::WHITE))
                                .fill(theme.accent_blue)
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(170.0, 30.0))).clicked() {
                                trigger_download = true;
                            }
                            ui.add_space(8.0);
                        }

                        if ui.add(egui::Button::new(RichText::new("Tutup").size(12.0).color(theme.text_secondary))
                            .fill(theme.bg_canvas)
                            .corner_radius(CornerRadius::same(6))
                            .min_size(Vec2::new(75.0, 30.0))).clicked() {
                            close_modal = true;
                        }

                        if !state.is_checking && !state.is_downloading {
                            ui.add_space(8.0);
                            if ui.add(egui::Button::new(RichText::new("🔄 Periksa Ulang").size(11.5).color(theme.text_primary))
                                .fill(theme.bg_card)
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(100.0, 30.0))).clicked() {
                                trigger_recheck = true;
                            }
                        }
                    });
                });
            });

        if close_modal {
            self.show_modal = false;
        }
        if trigger_download {
            self.start_download_and_install();
        }
        if trigger_recheck {
            self.check_for_updates();
        }
    }
}

/// Helper function to compare semver strings like "1.0.1" and "1.0.0"
fn is_version_newer(remote: &str, local: &str) -> bool {
    let parse = |s: &str| -> (u32, u32, u32) {
        let parts: Vec<u32> = s.split('.')
            .filter_map(|p| p.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse::<u32>().ok())
            .collect();
        (
            *parts.get(0).unwrap_or(&0),
            *parts.get(1).unwrap_or(&0),
            *parts.get(2).unwrap_or(&0),
        )
    };

    let (r_maj, r_min, r_pat) = parse(remote);
    let (l_maj, l_min, l_pat) = parse(local);

    if r_maj > l_maj {
        return true;
    }
    if r_maj == l_maj && r_min > l_min {
        return true;
    }
    if r_maj == l_maj && r_min == l_min && r_pat > l_pat {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_version_newer("1.0.1", "1.0.0"));
        assert!(is_version_newer("1.1.0", "1.0.9"));
        assert!(is_version_newer("2.0.0", "1.9.9"));
        assert!(!is_version_newer("1.0.0", "1.0.0"));
        assert!(!is_version_newer("1.0.0", "1.0.1"));
    }
}
