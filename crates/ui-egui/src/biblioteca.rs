//! Library (fork feature): the Home "Open" row, collections (virtual folders), the top-bar Open menu
//! with everything already added, moving PDFs between folders, and the back-to-top button.
//!
//! Kept in its own module, with a few calls from `home.rs`, `chrome.rs` and `lib.rs`, so the fork
//! stays easy to merge with upstream.

use egui::{Align2, CornerRadius, Rect, Sense, Stroke, vec2};

use crate::theme::{self, Tokens};
use crate::{PrintCraftApp, folders, icons};

/// Something to do with the app once a menu has finished drawing.
type Acao = Option<Box<dyn FnOnce(&mut PrintCraftApp)>>;

const LARANJA: egui::Color32 = egui::Color32::from_rgb(0xE8, 0xA3, 0x3D);
const ROXO: egui::Color32 = egui::Color32::from_rgb(0x8E, 0x6B, 0xE8);

fn cartao(ui: &mut egui::Ui, t: &Tokens, icone: &str, cor: egui::Color32, titulo: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(170.0, 96.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, titulo));
    ui.painter().rect(
        rect,
        CornerRadius::same(10),
        if resp.hovered() { t.hover } else { t.pasteboard },
        Stroke::new(1.0, t.divider),
        egui::StrokeKind::Inside,
    );
    icons::paint(ui, Rect::from_center_size(rect.center() - vec2(0.0, 14.0), vec2(28.0, 28.0)), icone, 26.0, cor);
    ui.painter().text(rect.center() + vec2(0.0, 22.0), Align2::CENTER_CENTER, titulo, theme::semibold(13.0), t.text);
    resp
}

/// First row of Home: Open file, Open folder, New collection.
pub fn linha_abrir(app: &mut PrintCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
        if cartao(ui, t, "folder-open", t.icon, "Open file").clicked() {
            app.open_dialog();
        }
        #[cfg(not(target_arch = "wasm32"))]
        if cartao(ui, t, "folder", LARANJA, "Open folder").clicked() {
            app.open_folder_dialog();
        }
        if cartao(ui, t, "folder-plus", ROXO, "New collection").clicked() {
            app.criando_colecao = Some(String::new());
        }
    });
    if let Some(nome) = app.criando_colecao.as_mut() {
        let mut fim: Option<bool> = None;
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let r = ui.add(egui::TextEdit::singleline(nome).hint_text("Collection name (e.g. Marketing course)").desired_width(300.0));
            r.request_focus();
            if ui.button("Create").clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                fim = Some(true);
            }
            if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                fim = Some(false);
            }
        });
        match fim {
            Some(true) => {
                let nome = app.criando_colecao.take().unwrap_or_default();
                if !nome.trim().is_empty() {
                    app.create_collection(nome.trim());
                }
            }
            Some(false) => app.criando_colecao = None,
            None => {}
        }
    }
}

/// Drop zone that creates a collection named after what is dropped.
fn zona_nova(app: &mut PrintCraftApp, ui: &mut egui::Ui, t: &Tokens, texto: &str) {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 56.0), Sense::hover());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, texto));
    ui.painter().rect(rect, CornerRadius::same(8), t.pasteboard, Stroke::new(1.0, t.divider), egui::StrokeKind::Inside);
    icons::paint(ui, Rect::from_center_size(rect.left_center() + vec2(26.0, 0.0), vec2(20.0, 20.0)), "folder-plus", 18.0, ROXO);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, texto, theme::medium(13.0), t.text_muted);
    soltar(app, ui, &resp, rect, None);
}

/// Home section: collections (virtual folders) as cards.
pub fn colecoes(app: &mut PrintCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let arrastando = egui::DragAndDrop::has_payload_of_type::<Arrastado>(ui.ctx());
    if app.collections.is_empty() {
        // No collection yet: while something is dragged, offer a zone that creates one.
        if arrastando {
            zona_nova(app, ui, t, "Drop here to create a collection");
            ui.add_space(16.0);
        }
        return;
    }
    ui.label(egui::RichText::new("Collections").font(theme::semibold(17.0)));
    ui.add_space(8.0);
    let mut abrir = None;
    let mut continuar = None;
    let mut alvos = Vec::new();
    for (i, c) in app.collections.iter().enumerate() {
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
        alvos.push((i, resp.clone(), rect));
        let mut direita = rect.right_center().x - 12.0;
        // "Continue" reopens the last PDF read from the collection.
        if let Some(last) = &c.last {
            let r = Rect::from_min_max(egui::pos2(direita - 84.0, rect.top() + 9.0), egui::pos2(direita, rect.bottom() - 9.0));
            let cont = ui.interact(r, resp.id.with("continue"), Sense::click());
            cont.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Continue"));
            ui.painter().rect(
                r,
                CornerRadius::same(6),
                if cont.hovered() { t.accent_soft } else { t.pasteboard },
                Stroke::new(1.0, t.divider),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(r.center(), Align2::CENTER_CENTER, "Continue", theme::medium(12.0), t.accent_text);
            if cont.clicked() {
                continuar = Some((i, last.clone()));
            }
            direita = r.left() - 12.0;
        }
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &c.name));
        if resp.hovered() {
            ui.painter().rect_filled(rect, CornerRadius::same(8), t.hover);
        }
        icons::paint(ui, Rect::from_min_size(rect.min + vec2(10.0, 11.0), vec2(24.0, 24.0)), "book-open", 22.0, ROXO);
        ui.painter().text(rect.min + vec2(46.0, 15.0), Align2::LEFT_CENTER, &c.name, theme::medium(13.5), t.text);
        ui.painter().text(rect.min + vec2(46.0, 32.0), Align2::LEFT_CENTER, "Collection", theme::regular(11.0), t.text_faint);
        let lidos = folders::read_count(c.files.iter().map(String::as_str), &app.progress);
        let detalhe = if lidos > 0 { format!("{lidos} of {} read", c.files.len()) } else { format!("{} PDFs", c.files.len()) };
        ui.painter().text(egui::pos2(direita, rect.center().y), Align2::RIGHT_CENTER, detalhe, theme::regular(12.0), t.text_muted);
        if resp.clicked() && continuar.is_none() {
            abrir = Some(i);
        }
    }
    for (i, resp, rect) in alvos {
        soltar(app, ui, &resp, rect, Some(i));
        menu_colecao(app, &resp, i);
    }
    if let Some((i, p)) = continuar {
        if let Some(c) = app.collections.get_mut(i) {
            c.last = Some(p.clone());
        }
        app.open_path_async(&p);
        return;
    }
    // Dropping on a collection adds to it; dropping here makes another one.
    if arrastando {
        ui.add_space(6.0);
        zona_nova(app, ui, t, "Drop here to create a new collection");
    }
    if let Some(i) = abrir {
        app.open_collection(i);
    }
    ui.add_space(16.0);
}

/// Top bar "Open": a menu with Open file / Open folder and everything already added.
pub fn menu_abrir(app: &mut PrintCraftApp, resp: &egui::Response) {
    let t = Tokens::get(&resp.ctx);
    egui::Popup::menu(resp).align(egui::RectAlign::BOTTOM_START).gap(4.0).show(|ui| {
        ui.set_min_width(320.0);
        let mut acao: Acao = None;
        let linha = |ui: &mut egui::Ui, icone: &str, cor: egui::Color32, texto: &str, detalhe: &str| -> bool {
            let (row, r) = ui.allocate_exact_size(vec2(ui.available_width().max(320.0), 30.0), Sense::click());
            if r.hovered() {
                ui.painter().rect_filled(row, CornerRadius::same(4), t.hover);
            }
            icons::paint(ui, Rect::from_min_size(row.min + vec2(8.0, 7.0), vec2(16.0, 16.0)), icone, 16.0, cor);
            ui.painter().text(row.left_center() + vec2(32.0, 0.0), Align2::LEFT_CENTER, texto, theme::regular(13.0), t.text);
            if !detalhe.is_empty() {
                ui.painter().text(row.right_center() - vec2(8.0, 0.0), Align2::RIGHT_CENTER, detalhe, theme::regular(11.0), t.text_faint);
            }
            r.clicked()
        };
        if linha(ui, "file-text", t.icon, "Open file…", "Ctrl+O") {
            acao = Some(Box::new(|app| app.open_dialog()));
        }
        #[cfg(not(target_arch = "wasm32"))]
        if linha(ui, "folder", LARANJA, "Open folder…", "") {
            acao = Some(Box::new(|app| app.open_folder_dialog()));
        }
        if !app.collections.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new("Collections").font(theme::semibold(11.5)).color(t.text_muted));
            for (i, c) in app.collections.iter().enumerate() {
                if linha(ui, "book-open", ROXO, &c.name, &format!("{} PDFs", c.files.len())) {
                    acao = Some(Box::new(move |app| app.open_collection(i)));
                }
            }
        }
        if !app.recent_folders.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new("Folders").font(theme::semibold(11.5)).color(t.text_muted));
            for f in &app.recent_folders {
                let p = f.path.clone();
                if linha(ui, "folder", LARANJA, &f.name, &format!("{} PDFs", f.count)) {
                    acao = Some(Box::new(move |app| app.open_folder(&p)));
                }
            }
        }
        if !app.recent.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new("Files").font(theme::semibold(11.5)).color(t.text_muted));
            for r in &app.recent {
                let p = r.path.clone();
                if linha(ui, "file-text", egui::Color32::from_rgb(0xE0, 0x3E, 0x3E), &r.name, &format!("{} pages", r.pages)) {
                    acao = Some(Box::new(move |app| app.open_path_async(&p)));
                }
            }
        }
        if let Some(f) = acao {
            f(app);
            ui.close();
        }
    });
}

/// Right-click menu of a PDF in a folder or collection: add to a collection, move it on disk.
pub fn menu_arquivo(app: &mut PrintCraftApp, resp: &egui::Response, path: &str) {
    let path = path.to_string();
    let colecoes: Vec<(usize, String)> = app.collections.iter().enumerate().map(|(i, c)| (i, c.name.clone())).collect();
    let mut acao: Acao = None;
    resp.context_menu(|ui| {
        ui.menu_button("Add to collection", |ui| {
            for (i, nome) in &colecoes {
                if ui.button(nome).clicked() {
                    let (i, p) = (*i, path.clone());
                    acao = Some(Box::new(move |app| app.add_to_collection(i, &[p])));
                    ui.close();
                }
            }
            if ui.button("New collection…").clicked() {
                let p = path.clone();
                acao = Some(Box::new(move |app| {
                    let nome = std::path::Path::new(&p)
                        .parent()
                        .and_then(|d| d.file_name())
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "New collection".into());
                    let i = app.create_collection(&nome);
                    app.add_to_collection(i, &[p]);
                }));
                ui.close();
            }
        });
        #[cfg(not(target_arch = "wasm32"))]
        if ui.button("Move to folder…").clicked() {
            let p = path.clone();
            acao = Some(Box::new(move |app| {
                if let Some(dest) = rfd::FileDialog::new().set_title("Move the PDF to…").pick_folder() {
                    app.move_file(&p, &dest.to_string_lossy());
                }
            }));
            ui.close();
        }
        match app.folder.as_ref().map(|v| (v.collection, v.path.clone())) {
            // In a collection: move between its sections.
            Some((Some(i), _)) => {
                let secoes = app.collections.get(i).map(|c| c.sections.clone()).unwrap_or_default();
                let atual = app.collections.get(i).and_then(|c| c.section_of.get(&path).cloned());
                ui.menu_button("Move to section", |ui| {
                    for s in &secoes {
                        if atual.as_deref() != Some(s.as_str()) && ui.button(s).clicked() {
                            let (p, s) = (path.clone(), s.clone());
                            acao = Some(Box::new(move |app| app.set_section(i, &p, Some(&s))));
                            ui.close();
                        }
                    }
                    if atual.is_some() && ui.button("No section").clicked() {
                        let p = path.clone();
                        acao = Some(Box::new(move |app| app.set_section(i, &p, None)));
                        ui.close();
                    }
                    ui.separator();
                    ui.label(egui::RichText::new("New section").small());
                    if let Some(nome) = campo_nome(ui, egui::Id::new(("nova-secao", &path)), "") {
                        let p = path.clone();
                        acao = Some(Box::new(move |app| app.set_section(i, &p, Some(&nome))));
                        ui.close();
                    }
                });
                if ui.button("Remove from this collection").clicked() {
                    let p = path.clone();
                    acao = Some(Box::new(move |app| app.remove_from_collection(i, &p)));
                    ui.close();
                }
            }
            // In a folder on disk: move the file into one of its subfolders (or back to the top).
            #[cfg(not(target_arch = "wasm32"))]
            Some((None, raiz)) if !raiz.is_empty() => {
                let mut subs: Vec<String> = app
                    .folder
                    .as_ref()
                    .map(|v| v.files.iter().filter_map(|f| f.rel.rsplit_once('/').map(|(d, _)| d.to_string())).collect())
                    .unwrap_or_default();
                subs.sort_by(|a, b| folders::natural_cmp(a, b));
                subs.dedup();
                let aqui = std::path::Path::new(&path).parent().map(|d| d.to_path_buf());
                ui.menu_button("Move to subfolder", |ui| {
                    let mut destinos: Vec<(String, std::path::PathBuf)> = vec![("(top of the folder)".into(), std::path::PathBuf::from(&raiz))];
                    destinos.extend(subs.iter().map(|s| (s.clone(), std::path::Path::new(&raiz).join(s))));
                    for (rotulo, dir) in destinos {
                        if Some(&dir) != aqui.as_ref() && ui.button(rotulo).clicked() {
                            let p = path.clone();
                            acao = Some(Box::new(move |app| app.move_file(&p, &dir.to_string_lossy())));
                            ui.close();
                        }
                    }
                });
            }
            _ => {}
        }
    });
    if let Some(f) = acao {
        f(app);
    }
}

/// Floating "back to top" button over the Home tab once it is scrolled down.
pub fn voltar_ao_topo(app: &mut PrintCraftApp, ui: &egui::Ui, offset: f32, area: Rect) {
    if offset < 240.0 {
        return;
    }
    let t = Tokens::get(ui.ctx());
    let centro = area.right_bottom() - vec2(40.0, 40.0);
    egui::Area::new(egui::Id::new("home-voltar-topo")).fixed_pos(centro - vec2(20.0, 20.0)).order(egui::Order::Foreground).show(ui.ctx(), |ui| {
        let (rect, resp) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::click());
        let resp = resp.on_hover_text("Back to top");
        ui.painter().circle(rect.center(), 20.0, if resp.hovered() { t.accent_soft } else { t.card }, Stroke::new(1.0, t.border));
        icons::paint(ui, rect, "chevron-up", 20.0, t.accent_text);
        if resp.clicked() {
            app.home_to_top = true;
        }
    });
}

/// What is being dragged from Recent onto a collection.
#[derive(Clone, Debug)]
pub enum Arrastado {
    Arquivo(String),
    Pasta(String),
}

impl Arrastado {
    fn nome(&self) -> String {
        let p = match self {
            Arrastado::Arquivo(p) | Arrastado::Pasta(p) => p,
        };
        std::path::Path::new(p).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.clone())
    }

    /// The PDFs it stands for, each with the subfolder it sits in (a folder's subfolders
    /// become sections of the collection).
    fn arquivos(&self) -> Vec<(String, Option<String>)> {
        match self {
            Arrastado::Arquivo(p) => vec![(p.clone(), None)],
            #[cfg(not(target_arch = "wasm32"))]
            Arrastado::Pasta(p) => folders::scan(std::path::Path::new(p))
                .into_iter()
                .map(|e| {
                    let sub = e.rel.rsplit_once('/').map(|(d, _)| d.to_string());
                    (e.path, sub)
                })
                .collect(),
            #[cfg(target_arch = "wasm32")]
            Arrastado::Pasta(_) => Vec::new(),
        }
    }
}

/// Lets a Recent row be dragged onto a collection (the row must sense drags).
pub fn arrastavel(ui: &egui::Ui, resp: &egui::Response, item: Arrastado) {
    if !resp.dragged() {
        return;
    }
    let t = Tokens::get(ui.ctx());
    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    if let Some(pos) = ui.ctx().pointer_interact_pos() {
        let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("arrastando")));
        let texto = item.nome();
        let galley = painter.layout_no_wrap(texto, theme::medium(12.5), t.text);
        let rect = Rect::from_min_size(pos + vec2(14.0, 10.0), galley.size() + vec2(16.0, 10.0));
        painter.rect(rect, CornerRadius::same(6), t.card, Stroke::new(1.0, ROXO), egui::StrokeKind::Inside);
        painter.galley(rect.min + vec2(8.0, 5.0), galley, t.text);
    }
    resp.dnd_set_drag_payload(item);
}

/// A collection row (or the empty drop zone) taking a dragged file or folder.
fn soltar(app: &mut PrintCraftApp, ui: &egui::Ui, resp: &egui::Response, rect: Rect, destino: Option<usize>) {
    if resp.dnd_hover_payload::<Arrastado>().is_some() {
        ui.painter().rect_stroke(rect, CornerRadius::same(8), Stroke::new(2.0, ROXO), egui::StrokeKind::Inside);
    }
    if let Some(item) = resp.dnd_release_payload::<Arrastado>() {
        let arquivos = item.arquivos();
        if arquivos.is_empty() {
            app.notify("No PDFs to add".to_string());
            return;
        }
        // Into a new collection, the folder's subfolders are the sections; into an existing one,
        // the folder itself is a section (with its subfolders under it).
        let pasta = matches!(*item, Arrastado::Pasta(_)).then(|| item.nome());
        let arquivos: Vec<(String, Option<String>)> = match (destino, pasta) {
            (Some(_), Some(nome)) => arquivos
                .into_iter()
                .map(|(f, sub)| {
                    let s = match sub {
                        Some(sub) => format!("{nome}/{sub}"),
                        None => nome.clone(),
                    };
                    (f, Some(s))
                })
                .collect(),
            _ => arquivos,
        };
        let i = destino.unwrap_or_else(|| app.create_collection(&item.nome()));
        app.add_to_collection_in(i, &arquivos);
    }
}

/// A one-line name field with an OK button, kept between frames under `id`. Returns the name
/// when confirmed (OK or Enter).
pub fn campo_nome(ui: &mut egui::Ui, id: egui::Id, inicial: &str) -> Option<String> {
    let mut texto: String = ui.data_mut(|d| d.get_temp(id)).unwrap_or_else(|| inicial.to_string());
    let mut ok = false;
    ui.horizontal(|ui| {
        let r = ui.add(egui::TextEdit::singleline(&mut texto).desired_width(180.0).hint_text("Name"));
        ok = ui.button("OK").clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
    });
    if ok && !texto.trim().is_empty() {
        ui.data_mut(|d| d.remove::<String>(id));
        return Some(texto.trim().to_string());
    }
    ui.data_mut(|d| d.insert_temp(id, texto));
    None
}

/// Right-click menu of a collection on Home: open, rename, delete.
fn menu_colecao(app: &mut PrintCraftApp, resp: &egui::Response, i: usize) {
    let nome = app.collections.get(i).map(|c| c.name.clone()).unwrap_or_default();
    let mut acao: Acao = None;
    resp.context_menu(|ui| {
        if ui.button("Open").clicked() {
            acao = Some(Box::new(move |app| app.open_collection(i)));
            ui.close();
        }
        ui.menu_button("Rename", |ui| {
            if let Some(novo) = campo_nome(ui, egui::Id::new(("renomear-colecao", i)), &nome) {
                acao = Some(Box::new(move |app| app.rename_collection(i, &novo)));
                ui.close();
            }
        });
        ui.separator();
        if ui.button("Delete collection").on_hover_text("Deletes only the collection; the PDFs stay where they are").clicked() {
            acao = Some(Box::new(move |app| app.delete_collection(i)));
            ui.close();
        }
    });
    if let Some(f) = acao {
        f(app);
    }
}

/// Right-click menu of a section header in a collection: rename it or remove it.
pub fn menu_secao(app: &mut PrintCraftApp, resp: &egui::Response, i: usize, secao: &str) {
    let secao = secao.to_string();
    let mut acao: Acao = None;
    resp.context_menu(|ui| {
        ui.menu_button("Rename section", |ui| {
            if let Some(novo) = campo_nome(ui, egui::Id::new(("renomear-secao", i, &secao)), &secao) {
                let s = secao.clone();
                acao = Some(Box::new(move |app| app.rename_section(i, &s, &novo)));
                ui.close();
            }
        });
        if ui.button("Remove section").on_hover_text("The PDFs stay in the collection, without a section").clicked() {
            let s = secao.clone();
            acao = Some(Box::new(move |app| app.remove_section(i, &s)));
            ui.close();
        }
    });
    if let Some(f) = acao {
        f(app);
    }
}
