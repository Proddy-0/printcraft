//! Library (fork): dragging Recent files and folders onto collections, and the top bars on Home.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use printcraft_ui_egui::{PrintCraftApp, RecentFile, folders};

fn harness(setup: impl FnOnce(&mut PrintCraftApp) + 'static) -> Harness<'static, PrintCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 1400.0)).build_eframe(move |_cc| {
        let mut app = PrintCraftApp::new();
        setup(&mut app);
        app
    });
    h.run_steps(4);
    h
}

fn recent(name: &str, path: &str) -> RecentFile {
    RecentFile { name: name.into(), path: path.into(), pages: 3, size: 1000 }
}

/// Press on `from`, move in steps to `to` and release there.
fn drag(h: &mut Harness<'static, PrintCraftApp>, from: egui::Pos2, to: egui::Pos2) {
    h.hover_at(from);
    h.run_steps(1);
    h.drag_at(from);
    h.run_steps(1);
    for k in 1..=8 {
        h.hover_at(from + (to - from) * (k as f32 / 8.0));
        h.run_steps(1);
    }
    h.drop_at(to);
    h.run_steps(3);
}

#[test]
fn dragging_a_recent_file_onto_a_collection_adds_it() {
    let mut h = harness(|app| {
        app.recent = vec![recent("chapter1.pdf", "/books/chapter1.pdf")];
        app.collections = vec![folders::Collection { name: "Course".into(), ..Default::default() }];
    });
    let (from, to) = (h.get_by_label("chapter1.pdf").rect().center(), h.get_by_label("Course").rect().center());
    drag(&mut h, from, to);
    assert_eq!(h.state().collections[0].files, vec!["/books/chapter1.pdf".to_string()]);
    assert!(h.state().active.is_none(), "dragging does not open the file");
}

#[test]
fn dragging_a_recent_folder_adds_its_pdfs() {
    let dir = std::env::temp_dir().join(format!("printlabs-dnd-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    for f in ["a.pdf", "sub/b.pdf", "notes.txt"] {
        std::fs::write(dir.join(f), b"%PDF-1.7").unwrap();
    }
    let path = dir.to_string_lossy().into_owned();
    let p = path.clone();
    let mut h = harness(move |app| {
        app.recent_folders = vec![folders::RecentFolder { name: "Livros".into(), path: p, count: 2, last: None }];
        app.collections = vec![folders::Collection { name: "Course".into(), ..Default::default() }];
    });
    let (from, to) = (h.get_by_label("Livros").rect().center(), h.get_by_label("Course").rect().center());
    drag(&mut h, from, to);
    let files = h.state().collections[0].files.clone();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(files.len(), 2, "{files:?}");
    assert!(files.iter().all(|f| f.ends_with(".pdf")));
    assert!(h.state().folder.is_none(), "dragging does not open the folder");
}

#[test]
fn home_has_no_document_buttons() {
    let h = harness(|_| {});
    for label in ["Print (⌘P)", "Save (M4)", "Document properties (⌘D)"] {
        assert!(h.query_by_label(label).is_none(), "{label} is hidden on Home");
    }
}

#[test]
fn dropping_a_second_folder_below_the_list_makes_another_collection() {
    let dir = std::env::temp_dir().join(format!("printlabs-dnd2-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("c.pdf"), b"%PDF-1.7").unwrap();
    let p = dir.to_string_lossy().into_owned();
    let mut h = harness(move |app| {
        app.recent_folders = vec![folders::RecentFolder { name: "Segunda".into(), path: p, count: 1, last: None }];
        app.collections = vec![folders::Collection { name: "Primeira".into(), files: vec!["/x/a.pdf".into()], ..Default::default() }];
    });
    let from = h.get_by_label("Segunda").rect().center();
    h.hover_at(from);
    h.run_steps(1);
    h.drag_at(from);
    h.run_steps(1);
    h.hover_at(from + egui::vec2(0.0, -20.0));
    h.run_steps(2);
    // The zone only exists while dragging.
    let to = h.get_by_label("Drop here to create a new collection").rect().center();
    for k in 1..=6 {
        h.hover_at(from + (to - from) * (k as f32 / 6.0));
        h.run_steps(1);
    }
    h.drop_at(to);
    h.run_steps(3);
    let cols: Vec<(String, usize)> = h.state().collections.iter().map(|c| (c.name.clone(), c.files.len())).collect();
    let pasta = dir.file_name().unwrap().to_string_lossy().into_owned();
    let _ = std::fs::remove_dir_all(&dir);
    // The new collection is named after the folder on disk.
    assert_eq!(cols, vec![("Primeira".to_string(), 1), (pasta, 1)]);
}

#[test]
fn recent_can_be_cleared() {
    let mut h = harness(|app| {
        app.recent = vec![recent("a.pdf", "/a.pdf")];
        app.recent_folders = vec![folders::RecentFolder { name: "Livros".into(), path: "/livros".into(), count: 2, last: None }];
    });
    h.get_by_label("Clear recent").click();
    h.run_steps(2);
    h.get_by_label("Clear all").click();
    h.run_steps(2);
    assert!(h.state().recent.is_empty() && h.state().recent_folders.is_empty());
}

#[test]
fn menu_opens_as_a_side_panel() {
    let mut h = harness(|_| {});
    h.get_by_label("Menu").click();
    h.run_steps(3);
    assert!(h.state().left_open && h.state().left == printcraft_ui_egui::LeftPanel::Menu);
    h.get_by_label("File");
    h.get_by_label("Close panel").click();
    h.run_steps(2);
    assert!(!h.state().left_open);
}

#[test]
fn rail_panels_open_in_read_mode() {
    let pdf = b"%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >> endobj\ntrailer << /Root 1 0 R >>\n%%EOF";
    let mut h = harness(move |app| {
        app.open_bytes("one.pdf", None, pdf.to_vec()).unwrap();
        app.mode = printcraft_ui_egui::Mode::Read;
    });
    h.get_by_label("Bookmarks").click();
    h.run_steps(3);
    h.get_by_label("This document has no bookmarks.");
}

#[test]
fn new_sticky_notes_are_dots() {
    assert_eq!(printcraft_engine::NoteIcon::from_name("Circle"), Some(printcraft_engine::NoteIcon::Circle));
}

#[test]
fn collection_sections_group_fold_and_move() {
    let mut c =
        folders::Collection { name: "Curso".into(), files: vec!["/a/1.pdf".into(), "/a/2.pdf".into(), "/b/3.pdf".into()], ..Default::default() };
    c.set_section("/a/1.pdf", Some("Módulo 1"));
    c.set_section("/b/3.pdf", Some("Módulo 2"));
    let rels: Vec<String> = c.entries().into_iter().map(|e| e.rel).collect();
    assert_eq!(rels, vec!["2.pdf", "Módulo 1/1.pdf", "Módulo 2/3.pdf"], "no section first, then sections in order");
    c.rename_section("Módulo 2", "Módulo 1");
    assert_eq!(c.sections, vec!["Módulo 1"], "renaming onto an existing section merges them");
    c.remove_section("Módulo 1");
    assert!(c.section_of.is_empty() && c.files.len() == 3, "removing a section keeps its PDFs");
    c.set_section("/a/2.pdf", Some("X"));
    c.renamed("/a/2.pdf", "/z/2.pdf");
    assert_eq!(c.section_of.get("/z/2.pdf").map(String::as_str), Some("X"), "a moved file keeps its section");

    // On screen: headers fold their PDFs away.
    let col = c.clone();
    let mut h = harness(move |app| {
        app.collections = vec![col];
        app.open_collection(0);
    });
    h.get_by_label("X");
    assert!(h.query_by_label("2.pdf").is_some());
    h.get_by_label("X").click();
    h.run_steps(3);
    assert!(h.query_by_label("2.pdf").is_none(), "folded");
}

#[test]
fn a_folder_with_subfolders_becomes_sections() {
    let dir = std::env::temp_dir().join(format!("printlabs-secoes-{}", std::process::id()));
    for f in ["livros 1/p1.pdf", "livros 1/p2.pdf", "livros 2/p3.pdf"] {
        std::fs::create_dir_all(dir.join(f).parent().unwrap()).unwrap();
        std::fs::write(dir.join(f), b"%PDF-1.7").unwrap();
    }
    let p = dir.to_string_lossy().into_owned();
    let mut h = harness(move |app| {
        app.recent_folders = vec![folders::RecentFolder { name: "Pasta main".into(), path: p, count: 3, last: None }];
        app.collections = vec![folders::Collection { name: "Estudos".into(), ..Default::default() }];
    });
    let (from, to) = (h.get_by_label("Pasta main").rect().center(), h.get_by_label("Estudos").rect().center());
    drag(&mut h, from, to);
    let c = h.state().collections[0].clone();
    let pasta = dir.file_name().unwrap().to_string_lossy().into_owned();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(c.files.len(), 3);
    assert_eq!(c.sections, vec![format!("{pasta}/livros 1"), format!("{pasta}/livros 2")]);
}

#[test]
fn a_collection_can_be_deleted_and_continued_from_home() {
    let mut h = harness(|app| {
        app.collections = vec![
            folders::Collection { name: "Velha".into(), ..Default::default() },
            folders::Collection { name: "Curso".into(), files: vec!["/c/a.pdf".into()], last: Some("/c/a.pdf".into()), ..Default::default() },
        ];
    });
    h.get_by_label("Continue");
    h.get_by_label("Velha").click_secondary();
    h.run_steps(2);
    h.get_by_label("Delete collection").click();
    h.run_steps(2);
    let nomes: Vec<String> = h.state().collections.iter().map(|c| c.name.clone()).collect();
    assert_eq!(nomes, vec!["Curso"]);
}
