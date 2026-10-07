//! Home tab: recommended tools, open card, recent files (local only, never another app's list).

use egui::{Align2, CornerRadius, Rect, Sense, Stroke, vec2};
use printcraft_engine::catalog;

use crate::theme::{self, Tokens};
use crate::{LeftPanel, PrintCraftApp, folders, icons, panels::human_size, widgets};

const RECOMMENDED: [&str; 5] = ["organize", "comment", "form", "edit", "protect"];

pub fn show(app: &mut PrintCraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::NONE.inner_margin(egui::Margin { left: 36, right: 36, top: 28, bottom: 28 }).show(ui, |ui| {
            ui.label(egui::RichText::new("Welcome to PrintCraft").font(theme::semibold(24.0)));
            ui.label(
                egui::RichText::new("An open-source PDF workbench — local, private, and scriptable.").color(t.text_muted).font(theme::regular(14.0)),
            );
            ui.add_space(14.0);
            egui::Frame::NONE
                .fill(t.card)
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(egui::Margin::same(14))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        widgets::artcraft_mark(ui, 28.0);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Join the ArtCraft community").font(theme::semibold(15.0)));
                            ui.label(egui::RichText::new("Get help, share feedback and follow development on Discord.").color(t.text_muted));
                        });
                    });
                    ui.add_space(8.0);
                    if let Some(cmd) = widgets::community_links(ui) {
                        app.execute(cmd);
                    }
                });
            ui.add_space(22.0);

            egui::Frame::NONE
                .fill(t.card)
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(egui::Margin::same(18))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(egui::RichText::new("Recommended tools").font(theme::semibold(15.0)));
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
                        for id in RECOMMENDED {
                            let Some(g) = catalog::group(id) else { continue };
                            let (rect, resp) = ui.allocate_exact_size(vec2(190.0, 104.0), Sense::click());
                            resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, g.label));
                            let fill = if resp.hovered() { t.hover } else { t.card };
                            ui.painter().rect(rect, CornerRadius::same(10), fill, Stroke::new(1.0, t.divider), egui::StrokeKind::Inside);
                            let color = egui::Color32::from_rgb(g.hue[0], g.hue[1], g.hue[2]);
                            icons::paint(ui, Rect::from_min_size(rect.min + vec2(14.0, 14.0), vec2(22.0, 22.0)), g.icon, 21.0, color);
                            ui.painter().text(rect.min + vec2(44.0, 25.0), Align2::LEFT_CENTER, g.label, theme::semibold(13.5), t.text);
                            let blurb = g
                                .sections
                                .first()
                                .map(|s| s.items.iter().take(3).map(|i| i.label).collect::<Vec<_>>().join(" · "))
                                .unwrap_or_default();
                            let galley = ui.fonts_mut(|f| f.layout(blurb, theme::regular(11.5), t.text_muted, rect.width() - 28.0));
                            ui.painter().galley(rect.min + vec2(14.0, 46.0), galley, t.text_muted);
                            ui.painter().text(
                                rect.left_bottom() + vec2(14.0, -14.0),
                                Align2::LEFT_CENTER,
                                "Use now",
                                theme::medium(12.0),
                                t.accent_text,
                            );
                            if resp.clicked() {
                                app.left = LeftPanel::Tool(g.id);
                                app.left_open = true;
                            }
                        }
                        let (rect, resp) = ui.allocate_exact_size(vec2(170.0, 104.0), Sense::click());
                        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Open file"));
                        ui.painter().rect(
                            rect,
                            CornerRadius::same(10),
                            if resp.hovered() { t.hover } else { t.pasteboard },
                            Stroke::new(1.0, t.divider),
                            egui::StrokeKind::Inside,
                        );
                        icons::paint(ui, Rect::from_center_size(rect.center() - vec2(0.0, 16.0), vec2(28.0, 28.0)), "folder-open", 26.0, t.icon);
                        ui.painter().text(rect.center() + vec2(0.0, 22.0), Align2::CENTER_CENTER, "Open file", theme::semibold(13.0), t.text);
                        if resp.clicked() {
                            app.open_dialog();
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let (rect, resp) = ui.allocate_exact_size(vec2(170.0, 104.0), Sense::click());
                            resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Open folder"));
                            ui.painter().rect(
                                rect,
                                CornerRadius::same(10),
                                if resp.hovered() { t.hover } else { t.pasteboard },
                                Stroke::new(1.0, t.divider),
                                egui::StrokeKind::Inside,
                            );
                            icons::paint(ui, Rect::from_center_size(rect.center() - vec2(0.0, 16.0), vec2(28.0, 28.0)), "folder", 26.0, t.icon);
                            ui.painter().text(rect.center() + vec2(0.0, 22.0), Align2::CENTER_CENTER, "Open folder", theme::semibold(13.0), t.text);
                            if resp.clicked() {
                                app.open_folder_dialog();
                            }
                        }
                    });
                });

            ui.add_space(26.0);
            if app.folder.is_some() {
                folder_view(app, ui, &t);
                return;
            }
            ui.label(egui::RichText::new("Recent").font(theme::semibold(17.0)));
            ui.add_space(8.0);
            recent_folders(app, ui, &t);
            if app.recent.is_empty() && app.recent_folders.is_empty() {
                ui.label(egui::RichText::new("Files you open in PrintCraft appear here. Drop a PDF anywhere to open it.").color(t.text_muted));
            }
            let mut open = None;
            for r in &app.recent {
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &r.name));
                if resp.hovered() {
                    ui.painter().rect_filled(rect, CornerRadius::same(8), t.hover);
                }
                icons::paint(
                    ui,
                    Rect::from_min_size(rect.min + vec2(10.0, 11.0), vec2(24.0, 24.0)),
                    "file-text",
                    22.0,
                    egui::Color32::from_rgb(0xE0, 0x3E, 0x3E),
                );
                ui.painter().text(rect.min + vec2(46.0, 15.0), Align2::LEFT_CENTER, &r.name, theme::medium(13.5), t.text);
                ui.painter().text(rect.min + vec2(46.0, 32.0), Align2::LEFT_CENTER, &r.path, theme::regular(11.0), t.text_faint);
                ui.painter().text(
                    rect.right_center() - vec2(12.0, 0.0),
                    Align2::RIGHT_CENTER,
                    format!("{} pages  ·  {}", r.pages, human_size(r.size)),
                    theme::regular(12.0),
                    t.text_muted,
                );
                if resp.clicked() {
                    open = Some(r.path.clone());
                }
            }
            if let Some(p) = open {
                if let Some(i) = app.views.iter().position(|v| app.session.get(v.id).and_then(|d| d.path.as_deref()) == Some(p.as_str())) {
                    app.active = Some(i);
                } else {
                    #[cfg(not(target_arch = "wasm32"))]
                    app.open_path(&p);
                }
            }
            ui.add_space(20.0);
            widgets::section_title(ui, "Privacy");
            ui.label(
                egui::RichText::new("PrintCraft works offline. No telemetry, no account, and no cloud processing unless you add a provider.")
                    .color(t.text_muted),
            );
        });
    });
}

/// Recent folders: one line per folder, with how many PDFs it has and the last one read.
fn recent_folders(app: &mut PrintCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let mut open = None;
    let mut resume = None;
    for f in &app.recent_folders {
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &f.name));
        if resp.hovered() {
            ui.painter().rect_filled(rect, CornerRadius::same(8), t.hover);
        }
        icons::paint(
            ui,
            Rect::from_min_size(rect.min + vec2(10.0, 11.0), vec2(24.0, 24.0)),
            "folder",
            22.0,
            egui::Color32::from_rgb(0xE8, 0xA3, 0x3D),
        );
        ui.painter().text(rect.min + vec2(46.0, 15.0), Align2::LEFT_CENTER, &f.name, theme::medium(13.5), t.text);
        let detail = match &f.last {
            Some(last) => format!(
                "{}  ·  last: {}",
                f.path,
                std::path::Path::new(last).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
            ),
            None => f.path.clone(),
        };
        ui.painter().text(rect.min + vec2(46.0, 32.0), Align2::LEFT_CENTER, detail, theme::regular(11.0), t.text_faint);
        // "Continue" reopens the last PDF read in this folder.
        let right = rect.right_center() - vec2(12.0, 0.0);
        let mut label_left = right.x;
        if let Some(last) = &f.last {
            let r = Rect::from_min_max(egui::pos2(right.x - 84.0, rect.top() + 9.0), egui::pos2(right.x, rect.bottom() - 9.0));
            let cont = ui.interact(r, resp.id.with("continue"), Sense::click());
            ui.painter().rect(
                r,
                CornerRadius::same(6),
                if cont.hovered() { t.accent_soft } else { t.pasteboard },
                Stroke::new(1.0, t.divider),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(r.center(), Align2::CENTER_CENTER, "Continue", theme::medium(12.0), t.accent_text);
            if cont.clicked() {
                resume = Some(last.clone());
            }
            label_left = r.left() - 12.0;
        }
        let lidos = app.progress.iter().filter(|(p, x)| x.read && folders::contains(&f.path, p)).count();
        let contagem = if lidos > 0 { format!("{lidos} of {} read", f.count) } else { format!("{} PDFs", f.count) };
        ui.painter().text(egui::pos2(label_left, right.y), Align2::RIGHT_CENTER, contagem, theme::regular(12.0), t.text_muted);
        if resp.clicked() && resume.is_none() {
            open = Some(f.path.clone());
        }
    }
    if let Some(p) = resume {
        open_file(app, &p);
    } else if let Some(p) = open {
        #[cfg(not(target_arch = "wasm32"))]
        app.open_folder(&p);
        #[cfg(target_arch = "wasm32")]
        let _ = p;
    }
    if !app.recent_folders.is_empty() {
        ui.add_space(6.0);
    }
}

/// Switch to the tab of an already-open file, or open it.
fn open_file(app: &mut PrintCraftApp, path: &str) {
    if let Some(i) = app.views.iter().position(|v| app.session.get(v.id).and_then(|d| d.path.as_deref()) == Some(path)) {
        app.active = Some(i);
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        app.open_path(path);
    }
}

/// The PDFs of the open folder, grouped by subfolder, with a filter box.
fn folder_view(app: &mut PrintCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let Some(view) = app.folder.as_mut() else { return };
    let mut back = false;
    ui.horizontal(|ui| {
        if icons::button(ui, "chevron-left", 30.0, false, "Back to Home").clicked() {
            back = true;
        }
        icons::paint(
            ui,
            Rect::from_min_size(ui.cursor().min + vec2(0.0, 3.0), vec2(24.0, 24.0)),
            "folder-open",
            22.0,
            egui::Color32::from_rgb(0xE8, 0xA3, 0x3D),
        );
        ui.add_space(28.0);
        ui.label(egui::RichText::new(&view.name).font(theme::semibold(17.0)));
        let lidos = folders::read_count(view.files.iter().map(|f| f.path.as_str()), &app.progress);
        ui.label(egui::RichText::new(format!("{} PDFs  ·  {lidos} read", view.files.len())).color(t.text_muted));
    });
    ui.label(egui::RichText::new(&view.path).font(theme::regular(11.0)).color(t.text_faint));
    ui.add_space(8.0);
    ui.add(egui::TextEdit::singleline(&mut view.filter).hint_text("Filter by name or subfolder…").desired_width(320.0));
    ui.add_space(8.0);
    let last = app.recent_folders.iter().find(|f| f.path == view.path).and_then(|f| f.last.clone());
    let progress = &app.progress;
    let mut toggle_read: Option<(String, bool)> = None;
    let mut open = None;
    let mut group: Option<String> = None;
    let shown = view.files.iter().filter(|f| folders::matches(f, &view.filter)).count();
    if shown == 0 {
        ui.label(
            egui::RichText::new(if view.files.is_empty() { "No PDFs in this folder." } else { "No PDFs match the filter." }).color(t.text_muted),
        );
    }
    for f in view.files.iter().filter(|f| folders::matches(f, &view.filter)) {
        let parent = f.rel.rsplit_once('/').map(|(d, _)| d.to_string());
        if parent != group {
            group = parent.clone();
            if let Some(g) = &parent {
                ui.add_space(6.0);
                ui.label(egui::RichText::new(g).font(theme::semibold(13.0)).color(t.text_muted));
            }
        }
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &f.name));
        let is_last = last.as_deref() == Some(f.path.as_str());
        let prog = progress.get(&f.path);
        let read = prog.is_some_and(|p| p.read);
        if resp.hovered() {
            ui.painter().rect_filled(rect, CornerRadius::same(8), t.hover);
        } else if is_last {
            ui.painter().rect_filled(rect, CornerRadius::same(8), t.accent_soft);
        }
        let file_tint = if read { t.text_faint } else { egui::Color32::from_rgb(0xE0, 0x3E, 0x3E) };
        icons::paint(ui, Rect::from_min_size(rect.min + vec2(10.0, 9.0), vec2(22.0, 22.0)), "file-text", 20.0, file_tint);
        ui.painter().text(rect.min + vec2(42.0, 16.0), Align2::LEFT_CENTER, &f.name, theme::medium(13.0), if read { t.text_muted } else { t.text });
        // Read toggle (right edge), then where the reader is.
        let toggle = Rect::from_center_size(rect.right_center() - vec2(22.0, 0.0), vec2(28.0, 28.0));
        let tr = ui.interact(toggle, resp.id.with("read"), Sense::click()).on_hover_text(if read { "Mark as not read" } else { "Mark as read" });
        if tr.hovered() {
            ui.painter().rect_filled(toggle, CornerRadius::same(6), t.pressed);
        }
        icons::paint(
            ui,
            toggle,
            if read { "circle-check" } else { "circle" },
            18.0,
            if read { egui::Color32::from_rgb(0x2E, 0xA0, 0x5A) } else { t.text_faint },
        );
        if tr.clicked() {
            toggle_read = Some((f.path.clone(), !read));
        }
        let size = human_size(usize::try_from(f.size).unwrap_or(usize::MAX));
        let status = match prog {
            Some(p) if p.read => format!("read  ·  {size}"),
            Some(p) if p.pages > 0 => format!("p. {} of {}{}  ·  {size}", p.page + 1, p.pages, if is_last { "  ·  last read" } else { "" }),
            _ => size,
        };
        ui.painter().text(toggle.left_center() - vec2(10.0, 0.0), Align2::RIGHT_CENTER, status, theme::regular(12.0), t.text_muted);
        // Progress bar under the name: furthest page reached.
        if let Some(p) = prog.filter(|p| p.pages > 0) {
            let bar = Rect::from_min_size(rect.min + vec2(42.0, 30.0), vec2((rect.width() * 0.35).clamp(80.0, 260.0), 4.0));
            ui.painter().rect_filled(bar, CornerRadius::same(2), t.divider);
            let done = Rect::from_min_size(bar.min, vec2(bar.width() * p.fraction(), bar.height()));
            let color = if p.read { egui::Color32::from_rgb(0x2E, 0xA0, 0x5A) } else { egui::Color32::from_rgb(0xE8, 0xA3, 0x3D) };
            ui.painter().rect_filled(done, CornerRadius::same(2), color);
        }
        if resp.clicked() && !tr.clicked() {
            open = Some(f.path.clone());
        }
    }
    if let Some((p, read)) = toggle_read {
        app.set_read(&p, read);
        return;
    }
    if back {
        app.folder = None;
    } else if let Some(p) = open {
        open_file(app, &p);
    }
}
