use std::path::PathBuf;
use eframe::egui;

pub(crate) fn toolbar(ui: &mut egui::Ui, app: &mut crate::app::FileViewerApp, ctx: &egui::Context, file_to_load: &mut Option<PathBuf>) {
    use egui::{RichText, Color32, Stroke};
    use rfd::FileDialog;

    let accent = ctx.style().visuals.selection.bg_fill;
    let bg = if ctx.style().visuals.dark_mode { Color32::from_rgb(28,32,36) } else { Color32::from_rgb(240,244,248) };

    egui::Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0, Color32::from_rgba_premultiplied(0,0,0,24)))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Left cluster: primary actions
                if ui.button(RichText::new("📂 Open").strong()).clicked()
                    && let Some(path) = FileDialog::new()
                        .add_filter("All Supported", &["txt","rs","py","toml","md","json","js","html","css","png","jpg","jpeg","gif","bmp","webp"])
                        .add_filter("Images", &["png","jpg","jpeg","gif","bmp","webp"])
                        .add_filter("Text/Source", &["txt","rs","py","toml","md","json","js","html","css"])
                        .pick_file()
                {
                    *file_to_load = Some(path);
                }

                ui.menu_button(RichText::new("🕘 Recent"), |ui| {
                    ui.set_min_width(520.0);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    if app.recent_files.is_empty() { ui.label("(empty)"); }
                    for file in app.recent_files.clone().into_iter().rev() {
                        let display = file.to_string_lossy();
                        if ui.button(RichText::new(display.clone()).monospace()).on_hover_text(display).clicked() {
                            *file_to_load = Some(file);
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.button("Clear Recent Files").clicked() { app.recent_files.clear(); ui.close_menu(); }
                });

                ui.separator();

                // Accent chooser
                ui.menu_button(RichText::new("●").color(accent).size(18.0), |ui| {
                    let mut set_accent = |rgb: [u8;3]| {
                        app.accent_rgb = rgb;
                        crate::settings::save_settings_to_disk(app);
                        app.apply_theme(ctx);
                    };
                    let choices: [([u8;3], &str); 6] = [
                        ([93,156,255], "Blue"),
                        ([0,200,150], "Teal"),
                        ([255,120,80], "Coral"),
                        ([180,120,255], "Purple"),
                        ([255,170,0], "Amber"),
                        ([80,200,120], "Mint"),
                    ];
                    for (rgb, name) in choices {
                        let c = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                        if ui.button(RichText::new(name).color(c).strong()).clicked() { set_accent(rgb); ui.close_menu(); }
                    }
                });

                // Toggles
                let prev_dark = app.dark_mode;
                let prev_lines = app.show_line_numbers;
                let dark_lbl = if app.dark_mode { "🌓 Dark" } else { "🌞 Light" };
                if ui.toggle_value(&mut app.dark_mode, dark_lbl).clicked() && app.dark_mode != prev_dark {
                    app.apply_theme(ctx);
                }
                if ui.toggle_value(&mut app.show_line_numbers, "🔢 Lines").clicked() && app.show_line_numbers != prev_lines {
                    crate::settings::save_settings_to_disk(app);
                }

                ui.separator();

                if ui.button("🧹 Clear").clicked() {
                    app.content = None;
                    app.current_path = None;
                    app.error_message = None;
                }

                // Spacer then title
                ui.add_space(8.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("Gemini File Viewer")
                            .size(18.0)
                            .color(accent)
                            .strong()
                    );
                });

                // Contextual image tools
            });

            if matches!(app.content, Some(crate::app::Content::Image(_))) {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let prev_fit = app.image_fit;
                    if let Some(cur) = app.current_path.clone() {
                        if ui.button("⟨ Prev").clicked() {
                            if let Some(prev) = crate::io::neighbor_image(&cur, false) { *file_to_load = Some(prev); }
                        }
                        if ui.button("Next ⟩").clicked() {
                            if let Some(next) = crate::io::neighbor_image(&cur, true) { *file_to_load = Some(next); }
                        }
                    }
                    ui.separator();
                    ui.toggle_value(&mut app.image_fit, "⤢ Fit");
                    if app.image_fit != prev_fit { crate::settings::save_settings_to_disk(app); }
                    if ui.button("−").clicked() { app.image_fit = false; app.image_zoom = (app.image_zoom / 1.10).clamp(0.1, 6.0); }
                    if ui.button("+").clicked() { app.image_fit = false; app.image_zoom = (app.image_zoom * 1.10).clamp(0.1, 6.0); }
                    if ui.button("100% ").clicked() { app.image_fit = false; app.image_zoom = 1.0; }
                });
            }
        });
}

pub(crate) fn search_bar(ui: &mut egui::Ui, app: &mut crate::app::FileViewerApp) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Find:");
        let prev = app.search_query.clone();
        let resp = ui.text_edit_singleline(&mut app.search_query);
        if app.search_active {
            resp.request_focus();
            app.search_active = false;
        }
        // Enter / Shift+Enter navigate matches
        let (enter, shift) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.modifiers.shift));
        if enter && app.search_count > 0 {
            if shift {
                if app.search_current == 0 { app.search_current = app.search_count.saturating_sub(1); } else { app.search_current -= 1; }
            } else {
                app.search_current = (app.search_current + 1) % app.search_count;
            }
        }

        if resp.changed() || (prev.is_empty() && !app.search_query.is_empty()) {
            app.search_count = 0;
            app.search_current = 0;
            if let Some(crate::app::Content::Text(ref text)) = app.content {
                if !app.search_query.is_empty() && text.len() <= crate::app::HIGHLIGHT_CHAR_THRESHOLD {
                    app.search_count = crate::search::recompute_count(&app.search_query, text);
                }
            }
        }
        if !app.search_query.is_empty() {
            ui.label(format!("{} match(es)", app.search_count));
            ui.add_space(8.0);
            if ui.button("Prev").clicked() && app.search_count > 0 {
                if app.search_current == 0 { app.search_current = app.search_count.saturating_sub(1); } else { app.search_current -= 1; }
            }
            if ui.button("Next").clicked() && app.search_count > 0 {
                app.search_current = (app.search_current + 1) % app.search_count;
            }
            if app.search_count > 0 {
                ui.label(format!("{}/{}", app.search_current + 1, app.search_count));
            }
        }
    });
}

pub(crate) fn status_bar(ui: &mut egui::Ui, app: &mut crate::app::FileViewerApp) {
    use std::fs;
    egui::Frame::new()
        .fill(ui.visuals().panel_fill)
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(0,0,0,24)))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(12, 6))
        .show(ui, |ui| {
        ui.horizontal(|ui| {
            if let Some(path) = &app.current_path {
                ui.monospace(path.to_string_lossy());
                if let Ok(metadata) = fs::metadata(path) {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("({:.1} KB)", metadata.len() as f64 / 1024.0));
                    });
                }
                if ui.button("Copy Path").on_hover_text("Copy path to clipboard").clicked() {
                    ui.ctx().copy_text(path.to_string_lossy().into());
                }
                if ui.button("Open Folder").clicked() {
                    #[cfg(target_os = "windows")]
                    { let _ = std::process::Command::new("explorer").arg(path).spawn(); }
                    #[cfg(target_os = "macos")]
                    { let _ = std::process::Command::new("open").arg("-R").arg(path).spawn(); }
                    #[cfg(all(unix, not(target_os = "macos")))]
                    { if let Some(parent) = path.parent() { let _ = std::process::Command::new("xdg-open").arg(parent).spawn(); } }
                }
            } else {
                ui.label("No file selected.");
            }
        });
    });
}

pub(crate) fn status_extra(ui: &mut egui::Ui, app: &mut crate::app::FileViewerApp) {
    egui::Frame::new()
        .fill(ui.visuals().panel_fill)
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(12, 6))
        .show(ui, |ui| {
    ui.horizontal(|ui| {
        match &app.content {
            Some(crate::app::Content::Image(texture)) => {
                let size = texture.size();
                ui.label(format!("Image: {}x{} px", size[0], size[1]));
                let eff = if app.image_fit { None } else { Some(app.image_zoom) };
                if let Some(z) = eff { ui.label(format!("Zoom: {:.0}%", z * 100.0)); }
                let est = (size[0] as usize).saturating_mul(size[1] as usize).saturating_mul(4);
                ui.label(format!("Texture ~{:.1} MB", est as f64 / (1024.0 * 1024.0)));
                if app.image_fit { ui.label("Fit: on"); }
            }
            Some(crate::app::Content::Text(_)) => {
                ui.label(format!("Lines: {}", app.text_line_count));
                ui.label(format!("Zoom: {:.0}%", app.text_zoom * 100.0));
                if app.text_is_big { ui.label("Large file: reduced features"); }
                if app.text_is_lossy { ui.label("UTF-8 (lossy)"); }
            }
            _ => {}
        }
    });
    });
}

