//! Updates (LABS-156): the fork's releases, the start-up notice, skipping a version and picking
//! one to download, with stand-in release sources (no network).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use printcraft_ui_egui::PrintCraftApp;
use printcraft_ui_egui::updates::{DOWNLOADS, Install, RELEASES_PAGE, Release, UpdateSource, download_for, is_newer, newer};

const OLD: &str = "v0.2.1-labs.20261007.51eb7a4";
const MID: &str = "v0.2.1-labs.20261009.845a253";
const NEW: &str = "v0.3.0-labs.20261010.abc1234";

fn release(tag: &str) -> Release {
    let asset = |suffix: &str| (format!("printcraft{suffix}"), format!("{DOWNLOADS}{tag}/printcraft{suffix}"));
    Release {
        version: tag.into(),
        url: format!("{RELEASES_PAGE}/tag/{tag}"),
        assets: ["-windows-x64-setup.exe", "-windows-x64-portable.zip", "-linux-x64-portable.tar.gz", "-macos-arm64-portable.tar.gz"]
            .iter()
            .map(|s| asset(s))
            .collect(),
    }
}

fn source(answer: Result<Vec<&'static str>, &'static str>, calls: Arc<AtomicUsize>) -> UpdateSource {
    Arc::new(move || {
        calls.fetch_add(1, Ordering::SeqCst);
        answer.clone().map(|tags| tags.into_iter().map(release).collect()).map_err(str::to_string)
    })
}

fn harness_with(
    answer: Result<Vec<&'static str>, &'static str>,
    installed: &'static str,
    install: Install,
    settings: &'static str,
) -> (Harness<'static, PrintCraftApp>, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    let h = Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_eframe(move |_cc| {
        let mut app = PrintCraftApp::new();
        app.restore(settings);
        app.update_source = Some(source(answer.clone(), counted.clone()));
        app.installed_version = installed.into();
        app.install = install;
        app
    });
    (h, calls)
}

/// Settings that already checked moments ago (no start-up check).
const CHECKED: &str = r#"{"labs_updates": {"at_start": true, "last_check": 99999999999}}"#;

/// Run frames until the background check has reported (or give up).
fn settle(h: &mut Harness<'static, PrintCraftApp>) {
    for _ in 0..200 {
        h.run_steps(2);
        if h.query_by_label_contains("Checking for a newer version").is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    for _ in 0..20 {
        h.run_steps(2);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[test]
fn versions_compare_by_number_then_build_date() {
    assert!(is_newer("v0.2.0", "0.1.1"));
    assert!(is_newer("v0.1.10", "0.1.9"));
    assert!(is_newer("1", "0.9.9"));
    assert!(!is_newer("v0.1.1", "0.1.1"));
    assert!(!is_newer("v0.1.0", "0.1.1"));
    assert!(!is_newer("v0.1.1-beta.2", "0.1.1"), "other suffixes are ignored");
    assert!(!is_newer("nightly", "0.1.1"), "a tag that isn't a version is never newer");
    assert!(!is_newer("v1.2.3.4", "0.1.1"));
    assert!(!is_newer("v99999999999999999999.0.0", "0.1.1"), "out of range");
    assert!(is_newer(MID, OLD), "a later build of the same version");
    assert!(is_newer(MID, "0.2.1"), "a labs build is newer than the bare version");
    assert!(is_newer(NEW, MID));
    assert!(!is_newer(OLD, MID));
}

#[test]
fn newer_releases_are_the_ones_listed_before_ours() {
    let list: Vec<Release> = [NEW, MID, OLD].map(release).into();
    assert_eq!(newer(&list, OLD).len(), 2);
    assert_eq!(newer(&list, MID).len(), 1);
    assert!(newer(&list, NEW).is_empty());
    // Two builds of the same day: the list order decides.
    let same_day: Vec<Release> = ["v0.2.1-labs.20261009.bbbbbbb", "v0.2.1-labs.20261009.aaaaaaa"].map(release).into();
    assert_eq!(newer(&same_day, "v0.2.1-labs.20261009.aaaaaaa").len(), 1);
    // A copy that isn't listed (a dev build) compares by number.
    assert_eq!(newer(&list, "0.2.1").len(), 3);
    assert!(newer(&list, "9.0.0").is_empty());
}

#[test]
fn the_download_matches_how_the_app_was_installed() {
    let r = release(NEW);
    assert_eq!(download_for(&r, Install::Dev), None, "a dev build gets the release page");
    let portable = download_for(&r, Install::Portable).unwrap();
    assert!(portable.contains("-portable."), "{portable}");
    let installed = download_for(&r, Install::Installed).unwrap();
    if cfg!(windows) {
        assert!(installed.ends_with("-windows-x64-setup.exe"), "{installed}");
    } else {
        assert!(installed.contains("-portable."), "no installer off Windows: {installed}");
    }
    assert_eq!(download_for(&Release { assets: vec![], ..release(NEW) }, Install::Portable), None);
}

#[test]
fn a_start_up_check_shows_a_notice_that_opens_the_dialog() {
    let (mut h, calls) = harness_with(Ok(vec![NEW, MID, OLD]), MID, Install::Portable, "{}");
    settle(&mut h);
    assert_eq!(calls.load(Ordering::SeqCst), 1, "one check at start");
    h.get_by_label_contains("Print Labs 0.3.0-labs.20261010.abc1234 is available");
    h.get_by_label("See update").click();
    settle(&mut h);
    h.get_by_label_contains("This is the portable copy");
    h.get_by_label_contains("(this one)");
    h.get_by_label("Download").click();
    h.run_steps(3);
    let url = h.state().last_opened_url.clone().unwrap();
    assert!(url.starts_with(&format!("{DOWNLOADS}{NEW}/")) && url.contains("-portable."), "{url}");
}

#[test]
fn skipping_a_version_silences_its_notice() {
    let (mut h, _) = harness_with(Ok(vec![NEW, MID]), MID, Install::Installed, "{}");
    settle(&mut h);
    h.get_by_label("Skip this version").click();
    h.run_steps(3);
    assert!(h.query_by_label_contains("is available").is_none());
    let saved = h.state().persist();
    assert!(saved.contains(NEW), "the skipped version is saved: {saved}");

    // Next start: the same release stays quiet.
    let settings: &'static str = Box::leak(saved.replace("\"last_check\":", "\"was\":").into_boxed_str());
    let (mut h, calls) = harness_with(Ok(vec![NEW, MID]), MID, Install::Installed, settings);
    settle(&mut h);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(h.query_by_label_contains("is available").is_none(), "no notice for a skipped version");
}

#[test]
fn no_start_up_check_when_turned_off_or_done_today() {
    let (mut h, calls) = harness_with(Ok(vec![NEW]), MID, Install::Portable, r#"{"labs_updates": {"at_start": false}}"#);
    settle(&mut h);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let (mut h, calls) = harness_with(Ok(vec![NEW]), MID, Install::Portable, CHECKED);
    settle(&mut h);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    // The menu still asks.
    h.state_mut().execute("help.check_updates");
    settle(&mut h);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    h.get_by_label_contains("Print Labs 0.3.0-labs.20261010.abc1234 is available.");
}

#[test]
fn an_older_version_can_be_picked() {
    let (mut h, _) = harness_with(Ok(vec![NEW, MID, OLD]), MID, Install::Portable, CHECKED);
    h.state_mut().execute("help.check_updates");
    settle(&mut h);
    h.get_by_label_contains("0.2.1-labs.20261007.51eb7a4").click();
    h.run_steps(3);
    h.get_by_label("Download").click();
    h.run_steps(3);
    let url = h.state().last_opened_url.clone().unwrap();
    assert!(url.starts_with(&format!("{DOWNLOADS}{OLD}/")), "{url}");
}

#[test]
fn an_up_to_date_or_failed_check_says_so() {
    let (mut h, _) = harness_with(Ok(vec![MID, OLD]), MID, Install::Portable, CHECKED);
    h.state_mut().execute("help.check_updates");
    settle(&mut h);
    h.get_by_label_contains("is up to date");

    let (mut h, _) = harness_with(Err("couldn't reach GitHub"), MID, Install::Portable, CHECKED);
    h.state_mut().execute("help.check_updates");
    settle(&mut h);
    h.get_by_label_contains("Couldn't check for updates: couldn't reach GitHub");
    assert!(h.query_by_label("Download").is_none());
}
