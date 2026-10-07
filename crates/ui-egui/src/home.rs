//! Home tab: recommended tools, open card, recent files (local only, never another app's list).

use egui::{Align2, CornerRadius, Rect, Sense, Stroke, vec2};
use printcraft_engine::catalog;

use crate::theme::{self, Tokens};
use crate::{LeftPanel, PrintCraftApp, folders, icons, panels::human_size, widgets};

const RECOMMENDED: [&str; 5] = ["organize", "comment", "form", "edit", "protect"];

pub fn show(app: &mut PrintCraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    let area = ui.max_rect();
    let mut rolagem = egui::ScrollArea::vertical().auto_shrink([false, false]);
    if std::mem::take(&mut app.home_to_top) {
        rolagem = rolagem.vertical_scroll_offset(0.0);
    }
    let saida = rolagem.show(ui, |ui| {
        egui::Frame::NONE.inner_margin(egui::Margin { left: 36, right: 36, top: 28, bottom: 28 }).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Welcome to Print Labs").font(theme::semibold(24.0)));
            });
            ui.label(
                egui::RichText::new("An open-source PDF workbench — local, private, and scriptable.").color(t.text_muted).font(theme::regular(14.0)),
            );
            ui.add_space(14.0);
            crate::biblioteca::linha_abrir(app, ui, &t);
            ui.add_space(18.0);

            egui::Frame::NONE
                .fill(t.card)
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(egui::Margin::same(18))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let titulo = if app.favorite_tools.is_empty() { "Recommended tools" } else { "My tools" };
                    ui.label(egui::RichText::new(titulo).font(theme::semibold(15.0)));
                    ui.label(
                        egui::RichText::new("Star a tool to keep it here (also from All tools on the left).")
                            .font(theme::regular(11.5))
                            .color(t.text_faint),
                    );
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
                        // Starred tools first (in the order they were starred), then the recommended ones.
                        let mut ids: Vec<String> = app.favorite_tools.clone();
                        ids.extend(RECOMMENDED.iter().map(|r| r.to_string()).filter(|r| !app.favorite_tools.contains(r)));
                        let mut estrela: Option<String> = None;
                        for id in &ids {
                            let Some(g) = catalog::group(id) else { continue };
                            let fav = app.favorite_tools.iter().any(|f| f == id);
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
                            let star = Rect::from_center_size(rect.right_top() + vec2(-18.0, 18.0), vec2(24.0, 24.0));
                            let sr = ui.interact(star, resp.id.with("star"), Sense::click()).on_hover_text(if fav {
                                "Remove from My tools"
                            } else {
                                "Add to My tools"
                            });
                            if fav || resp.hovered() || sr.hovered() {
                                icons::paint(ui, star, "star", 16.0, if fav { egui::Color32::from_rgb(0xF2, 0xB7, 0x05) } else { t.text_faint });
                            }
                            if sr.clicked() {
                                estrela = Some(id.clone());
                            } else if resp.clicked() {
                                app.left = LeftPanel::Tool(g.id);
                                app.left_open = true;
                            }
                        }
                        if let Some(id) = estrela {
                            app.toggle_favorite_tool(&id);
                        }
                    });
                });

            ui.add_space(26.0);
            if app.folder.is_some() {
                folder_view(app, ui, &t);
                return;
            }
            crate::biblioteca::colecoes(app, ui, &t);
            ui.label(egui::RichText::new("Recent").font(theme::semibold(17.0)));
            ui.add_space(8.0);
            recent_folders(app, ui, &t);
            if app.recent.is_empty() && app.recent_folders.is_empty() {
                ui.label(egui::RichText::new("Files you open in Print Labs appear here. Drop a PDF anywhere to open it. Drag a file or folder onto a collection to add it.").color(t.text_muted));
            }
            let mut open = None;
            for r in &app.recent {
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click_and_drag());
                crate::biblioteca::arrastavel(ui, &resp, crate::biblioteca::Arrastado::Arquivo(r.path.clone()));
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
                    app.open_path_async(&p);
                }
            }
            ui.add_space(20.0);
            widgets::section_title(ui, "Privacy");
            ui.label(
                egui::RichText::new("Print Labs works offline. No telemetry, no account, and no cloud processing unless you add a provider.")
                    .color(t.text_muted),
            );
        });
    });
    crate::biblioteca::voltar_ao_topo(app, ui, saida.state.offset.y, area);
}

/// Recent folders: one line per folder, with how many PDFs it has and the last one read.
fn recent_folders(app: &mut PrintCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let mut open = None;
    let mut resume = None;
    for f in &app.recent_folders {
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click_and_drag());
        crate::biblioteca::arrastavel(ui, &resp, crate::biblioteca::Arrastado::Pasta(f.path.clone()));
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

/// Switch to the tab of an already-open file, or open it (read in the background).
fn open_file(app: &mut PrintCraftApp, path: &str) {
    app.open_path_async(path);
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
    let pasta = view.path.clone();
    let colecao = view.collection;
    let mut clear_all = false;
    let mut add_pdfs = false;
    let mut apagar_colecao = false;
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut view.filter).hint_text("Filter by name or subfolder…").desired_width(320.0));
        if ui
            .add(egui::Button::new(egui::RichText::new("Clear progress").font(theme::regular(12.0))))
            .on_hover_text("Forget page, progress and read marks of every PDF in this folder")
            .clicked()
        {
            clear_all = true;
        }
        if colecao.is_some() {
            if ui.button("Add PDFs…").clicked() {
                add_pdfs = true;
            }
            if ui.button("Delete collection").on_hover_text("Deletes only the collection; the PDFs stay where they are").clicked() {
                apagar_colecao = true;
            }
        }
    });
    ui.add_space(8.0);
    let last = app.recent_folders.iter().find(|f| f.path == view.path).and_then(|f| f.last.clone());
    let progress = &app.progress;
    let mut toggle_read: Option<(String, bool)> = None;
    let mut clear_one: Option<String> = None;
    let mut menu_de: Vec<(egui::Response, String)> = Vec::new();
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
        // Clear this file's progress (only when there is some).
        let limpar = Rect::from_center_size(toggle.center() - vec2(30.0, 0.0), vec2(28.0, 28.0));
        if prog.is_some() {
            let lr = ui.interact(limpar, resp.id.with("clear"), Sense::click()).on_hover_text("Clear reading progress");
            if lr.hovered() {
                ui.painter().rect_filled(limpar, CornerRadius::same(6), t.pressed);
            }
            if resp.hovered() || lr.hovered() {
                icons::paint(ui, limpar, "eraser", 16.0, t.text_muted);
            }
            if lr.clicked() {
                clear_one = Some(f.path.clone());
            }
        }
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
        ui.painter().text(limpar.left_center() - vec2(6.0, 0.0), Align2::RIGHT_CENTER, status, theme::regular(12.0), t.text_muted);
        // Progress bar under the name: furthest page reached.
        if let Some(p) = prog.filter(|p| p.pages > 0) {
            let bar = Rect::from_min_size(rect.min + vec2(42.0, 30.0), vec2((rect.width() * 0.35).clamp(80.0, 260.0), 4.0));
            ui.painter().rect_filled(bar, CornerRadius::same(2), t.divider);
            let done = Rect::from_min_size(bar.min, vec2(bar.width() * p.fraction(), bar.height()));
            let color = if p.read { egui::Color32::from_rgb(0x2E, 0xA0, 0x5A) } else { egui::Color32::from_rgb(0xE8, 0xA3, 0x3D) };
            ui.painter().rect_filled(done, CornerRadius::same(2), color);
        }
        menu_de.push((resp.clone(), f.path.clone()));
        if resp.clicked() && !tr.clicked() && clear_one.is_none() {
            open = Some(f.path.clone());
        }
    }
    if let Some((p, read)) = toggle_read {
        app.set_read(&p, read);
        return;
    }
    for (r, p) in menu_de {
        crate::biblioteca::menu_arquivo(app, &r, &p);
    }
    if let Some(p) = clear_one {
        app.clear_progress(&p);
        return;
    }
    if clear_all {
        match colecao.and_then(|i| app.collections.get(i)).map(|c| c.files.clone()) {
            Some(files) => files.iter().for_each(|p| app.clear_progress(p)),
            None => app.clear_folder_progress(&pasta),
        }
        return;
    }
    if let Some(i) = colecao {
        if add_pdfs {
            app.add_files_dialog(i);
            return;
        }
        if apagar_colecao {
            app.delete_collection(i);
            return;
        }
    }
    if back {
        app.folder = None;
    } else if let Some(p) = open {
        open_file(app, &p);
    }
}
