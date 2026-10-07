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
        app.collections = vec![folders::Collection { name: "Course".into(), files: Vec::new() }];
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
        app.collections = vec![folders::Collection { name: "Course".into(), files: Vec::new() }];
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
