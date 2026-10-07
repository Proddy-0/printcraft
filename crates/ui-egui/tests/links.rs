//! Community links (fork: no Discord): Help menu, About dialog and CLI open the ArtCraft and
//! PrintCraft pages; there is no Discord button or community card anywhere.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use printcraft_engine::links;
use printcraft_ui_egui::{Dialog, PrintCraftApp};

fn harness(setup: impl FnOnce(&mut PrintCraftApp) + 'static) -> Harness<'static, PrintCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
        let mut app = PrintCraftApp::new();
        setup(&mut app);
        app
    });
    h.run_steps(4);
    h
}

#[test]
fn no_discord_anywhere() {
    let h = harness(|_| {});
    assert!(h.query_by_label_contains("Discord").is_none(), "no Discord button on the home screen");
    assert!(h.query_by_label_contains("ArtCraft community").is_none(), "no community card");
    assert!(links::LINKS.iter().all(|l| !l.url.contains("discord")));
    assert!(printcraft_engine::commands::command("help.discord").is_none());
}

#[test]
fn about_dialog_shows_the_brand_and_links() {
    let pdf = b"%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >> endobj\ntrailer << /Root 1 0 R >>\n%%EOF";
    let mut h = harness(move |app| {
        app.open_bytes("one.pdf", None, pdf.to_vec()).unwrap();
        app.dialog = Some(Dialog::About);
    });
    assert!(h.query_all_by_label("ArtCraft").count() >= 2, "the mark and the wordmark (alt text)");
    h.get_by_label("Print Labs");
    h.get_by_label("PrintCraft on GitHub").click();
    h.run_steps(2);
    assert_eq!(h.state().last_opened_url.as_deref(), Some(links::GITHUB));
}

#[test]
fn help_commands_open_each_link() {
    for l in links::LINKS {
        let mut h = harness(|_| {});
        assert!(h.state_mut().execute(l.command), "{}", l.command);
        assert_eq!(h.state().last_opened_url.as_deref(), Some(l.url));
        assert_eq!(printcraft_engine::commands::command(l.command).unwrap().menu, Some("Help"));
    }
}
