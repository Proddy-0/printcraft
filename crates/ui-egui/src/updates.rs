//! Updates (Proddyt Switch, LABS-156): the fork's releases, a notice when a newer one is out, and a
//! dialog to pick a version and download it.
//!
//! The desktop app supplies how to ask ([`PrintCraftApp::update_source`]) and what is installed
//! ([`PrintCraftApp::installed_version`], [`PrintCraftApp::install`]), so this crate has no
//! network code; without a source (the web build) the command opens the releases page.
//! Nothing is ever installed automatically: the user downloads the installer or the portable
//! archive and runs or unpacks it. A quiet check runs at start at most once a day (it can be turned
//! off in the dialog); a newer release shows a notice that can be dismissed or ignored for good.

use std::sync::Arc;

use egui::{Align, Align2, Layout, vec2};

use crate::{PrintCraftApp, theme, widgets};

/// Where every Print Labs release is listed.
pub const RELEASES_PAGE: &str = "https://github.com/Proddyt-Labs/print-labs/releases";
/// Prefix every downloadable file of a release must have.
pub const DOWNLOADS: &str = "https://github.com/Proddyt-Labs/print-labs/releases/download/";
/// How many releases the dialog lists.
pub const LISTED: usize = 10;
/// Seconds between two start-up checks.
#[cfg(not(target_arch = "wasm32"))]
const DAY: u64 = 24 * 60 * 60;

/// A published release.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Release {
    /// Its version tag, such as `v0.2.1-labs.20261009.845a253`.
    pub version: String,
    /// Its page on [`RELEASES_PAGE`].
    pub url: String,
    /// Its downloadable files, by file name (only links under [`DOWNLOADS`]).
    pub assets: Vec<(String, String)>,
}

/// Asks for the latest releases, newest first (blocking; it runs on its own thread).
pub type UpdateSource = Arc<dyn Fn() -> Result<Vec<Release>, String> + Send + Sync>;

/// How this copy of the app was put on the computer, which decides what an update downloads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Install {
    /// Built from source: updates only point at the releases page.
    #[default]
    Dev,
    /// Unpacked from a `-portable` archive (a `VERSION` file next to the program).
    Portable,
    /// Put there by the Windows installer (its uninstaller sits next to the program).
    Installed,
}

/// Whether release `latest` (a tag such as `v0.2.0`) is newer than version `current` (`0.1.1`).
/// A `-labs.<date>.<commit>` suffix orders builds of the same version by date; other pre-release
/// and build suffixes are ignored. A version that doesn't parse is never newer.
pub fn is_newer(latest: &str, current: &str) -> bool {
    matches!((parse(latest), parse(current)), (Some(l), Some(c)) if l > c)
}

fn parse(v: &str) -> Option<(u64, u64, u64, u64)> {
    let v = v.trim().trim_start_matches(['v', 'V']);
    let mut split = v.splitn(2, ['-', '+']);
    let core = split.next()?;
    let date = split
        .next()
        .and_then(|rest| rest.strip_prefix("labs."))
        .and_then(|rest| rest.split('.').next())
        .and_then(|d| (d.len() == 8).then(|| d.parse::<u64>().ok()).flatten())
        .unwrap_or(0);
    let mut parts = core.split('.');
    let mut next = |required: bool| match parts.next() {
        Some(p) => p.parse::<u64>().ok(),
        None if required => None,
        None => Some(0),
    };
    let version = (next(true)?, next(false)?, next(false)?, date);
    parts.next().is_none().then_some(version)
}

/// The releases newer than `current`, newest first. When `current` is one of the listed tags, the
/// ones published after it are newer (two builds of the same day share a date); otherwise the
/// version numbers and build dates decide (so a build from source, whose version is the bare
/// `0.2.1`, sees every `0.2.1-labs.*` release as newer).
pub fn newer<'a>(releases: &'a [Release], current: &str) -> Vec<&'a Release> {
    let current = current.trim();
    if let Some(i) = releases.iter().position(|r| r.version == current) {
        return releases[..i].iter().collect();
    }
    releases.iter().filter(|r| is_newer(&r.version, current)).collect()
}

/// Whether `url` is `prefix` followed by a plain path: no query, fragment, escapes, `..`, spaces
/// or control characters (release answers are untrusted).
pub fn under(url: &str, prefix: &str) -> bool {
    url.strip_prefix(prefix).is_some_and(|rest| {
        (prefix.ends_with('/') || rest.starts_with('/'))
            && !rest.trim_start_matches('/').is_empty()
            && rest.chars().all(|c| c.is_ascii_graphic() && !matches!(c, '?' | '#' | '\\' | '%'))
            && !rest.contains("..")
    })
}

/// The file of `release` to download for this computer, or `None` (then its page is offered).
pub fn download_for(release: &Release, install: Install) -> Option<&str> {
    let suffix = match (install, std::env::consts::OS) {
        (Install::Dev, _) => return None,
        (Install::Installed, "windows") => "-windows-x64-setup.exe",
        (_, "windows") => "-windows-x64-portable.zip",
        (_, "linux") => "-linux-x64-portable.tar.gz",
        (_, "macos") => "-macos-arm64-portable.tar.gz",
        _ => return None,
    };
    release.assets.iter().find(|(name, _)| name.ends_with(suffix)).map(|(_, url)| url.as_str())
}

/// Where a check is.
#[derive(Default)]
pub(crate) enum Check {
    #[default]
    Idle,
    #[cfg(not(target_arch = "wasm32"))]
    Running(std::sync::mpsc::Receiver<Result<Vec<Release>, String>>),
    Done(Result<Vec<Release>, String>),
}

// The web build never checks by itself, so some fields go unread there.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) struct Updates {
    pub(crate) check: Check,
    /// The Updates dialog is showing.
    pub(crate) open: bool,
    /// The running check was started at launch: report only a newer release, as a notice.
    quiet: bool,
    /// The start-up check was considered (once per run).
    started: bool,
    /// The "new version" notice is showing.
    pub(crate) notice: bool,
    /// The release picked in the dialog (its tag).
    picked: Option<String>,
    /// Check once a day at start (saved).
    pub(crate) at_start: bool,
    /// When the last check ran, in seconds since 1970 (saved).
    pub(crate) last_check: u64,
    /// A release the user chose to ignore (saved): no notice for it.
    pub(crate) ignored: Option<String>,
}

impl Default for Updates {
    fn default() -> Self {
        Self {
            check: Check::Idle,
            open: false,
            quiet: false,
            started: false,
            notice: false,
            picked: None,
            at_start: true,
            last_check: 0,
            ignored: None,
        }
    }
}

impl Updates {
    pub(crate) fn persist(&self) -> serde_json::Value {
        serde_json::json!({ "at_start": self.at_start, "last_check": self.last_check, "ignored": self.ignored })
    }

    pub(crate) fn restore(&mut self, v: &serde_json::Value) {
        if let Some(on) = v["at_start"].as_bool() {
            self.at_start = on;
        }
        if let Some(t) = v["last_check"].as_u64() {
            self.last_check = t;
        }
        self.ignored = v["ignored"].as_str().filter(|t| !t.is_empty() && t.len() <= 64).map(str::to_string);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl PrintCraftApp {
    /// Help ▸ Check for updates: ask for the releases and show the dialog.
    pub fn check_for_updates(&mut self) {
        self.updates.notice = false;
        if self.update_source.is_none() {
            self.open_url(RELEASES_PAGE);
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.updates.open = true;
            self.updates.quiet = false;
            self.start_check();
        }
        #[cfg(target_arch = "wasm32")]
        self.open_url(RELEASES_PAGE);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn start_check(&mut self) {
        let Some(source) = self.update_source.clone() else { return };
        if running(&self.updates.check) {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            // The receiver may be gone (the app quit): nothing to report to then.
            let _ = tx.send(source());
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
        self.updates.check = Check::Running(rx);
    }

    /// Start the daily check once, and pick up a finished check (each frame).
    pub(crate) fn poll_updates(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if !self.updates.started && self.update_source.is_some() {
                self.updates.started = true;
                let since = now().checked_sub(self.updates.last_check);
                if self.updates.at_start && since.is_none_or(|s| s >= DAY) {
                    self.updates.quiet = true;
                    self.start_check();
                }
            }
            if let Check::Running(rx) = &self.updates.check {
                let result = match rx.try_recv() {
                    Ok(r) => r,
                    Err(std::sync::mpsc::TryRecvError::Empty) => return,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => Err("the update check stopped unexpectedly".into()),
                };
                if std::mem::take(&mut self.updates.quiet)
                    && let Ok(list) = &result
                    && let Some(newest) = newer(list, &self.installed_version).first()
                    && self.updates.ignored.as_deref() != Some(newest.version.as_str())
                {
                    self.updates.notice = true;
                }
                if result.is_ok() {
                    self.updates.last_check = now();
                }
                self.updates.picked = None;
                self.updates.check = Check::Done(result);
            }
        }
    }
}

fn running(check: &Check) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    if matches!(check, Check::Running(_)) {
        return true;
    }
    let _ = check;
    false
}

fn short(tag: &str) -> &str {
    tag.trim_start_matches(['v', 'V'])
}

/// The notice in the top-right corner after a start-up check found a newer release.
fn notice(app: &mut PrintCraftApp, ctx: &egui::Context) {
    let Check::Done(Ok(list)) = &app.updates.check else { return };
    let Some(newest) = newer(list, &app.installed_version).first().map(|r| (*r).clone()) else { return };
    let t = theme::Tokens::get(ctx);
    let (mut view, mut ignore, mut close) = (false, false, false);
    egui::Area::new(egui::Id::new("update-notice"))
        .order(egui::Order::Foreground)
        .pivot(Align2::RIGHT_TOP)
        .fixed_pos(ctx.content_rect().right_top() + vec2(-16.0, 56.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
                ui.set_max_width(320.0);
                ui.horizontal(|ui| {
                    ui.add(crate::icons::image("cloud", 18.0, t.accent));
                    ui.label(egui::RichText::new(format!("Print Labs {} is available", short(&newest.version))).strong());
                });
                ui.label(egui::RichText::new(format!("You have {}.", short(&app.installed_version))).color(t.text_muted));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    view = widgets::pill_button(ui, "See update", true).clicked();
                    ignore = widgets::pill_button(ui, "Skip this version", false).clicked();
                    close = widgets::pill_button(ui, "Later", false).clicked();
                });
            });
        });
    if ignore {
        app.updates.ignored = Some(newest.version);
    }
    if view {
        app.updates.open = true;
    }
    if view || ignore || close {
        app.updates.notice = false;
    }
}

/// The notice and the Updates dialog.
pub(crate) fn dialog(app: &mut PrintCraftApp, ctx: &egui::Context) {
    if app.updates.notice && !app.updates.open {
        notice(app, ctx);
    }
    if !app.updates.open {
        return;
    }
    let t = theme::Tokens::get(ctx);
    let current = app.installed_version.clone();
    let install = app.install;
    let mut close = false;
    let mut get: Option<String> = None;
    let mut recheck = false;
    let modal = egui::Modal::new(egui::Id::new("updates")).show(ctx, |ui| {
        ui.set_width(460.0);
        ui.horizontal(|ui| {
            ui.add(crate::icons::image("cloud", 22.0, t.accent));
            ui.label(egui::RichText::new("Updates").font(theme::semibold(16.0)));
        });
        ui.add_space(8.0);
        match &app.updates.check {
            Check::Idle => {
                ui.label("No check has run yet.");
            }
            #[cfg(not(target_arch = "wasm32"))]
            Check::Running(_) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Checking for a newer version…");
                });
            }
            Check::Done(Err(e)) => {
                ui.label(format!("Couldn't check for updates: {e}"));
                ui.label(
                    egui::RichText::new(format!("You have {}. All releases are listed at {RELEASES_PAGE}.", short(&current))).color(t.text_muted),
                );
            }
            Check::Done(Ok(list)) => {
                let fresh = newer(list, &current);
                if let Some(newest) = fresh.first() {
                    ui.label(egui::RichText::new(format!("Print Labs {} is available.", short(&newest.version))).strong());
                } else {
                    ui.label(format!("Print Labs {} is up to date.", short(&current)));
                }
                ui.label(egui::RichText::new(format!("You have {}.", short(&current))).color(t.text_muted));
                ui.add_space(8.0);
                let picked =
                    app.updates.picked.get_or_insert_with(|| fresh.first().copied().or(list.first()).map(|r| r.version.clone()).unwrap_or_default());
                egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                    for r in list.iter().take(LISTED) {
                        let mut label = short(&r.version).to_string();
                        if r.version == current.trim() {
                            label.push_str("  (this one)");
                        } else if fresh.iter().any(|f| f.version == r.version) {
                            label.push_str("  (newer)");
                        }
                        ui.radio_value(picked, r.version.clone(), label);
                    }
                });
                let chosen = list.iter().find(|r| &r.version == picked).or(list.first());
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(match install {
                        Install::Installed => "Downloads the installer: close Print Labs and run it to update.",
                        Install::Portable => {
                            "This is the portable copy: the new archive is downloaded; unpack it over this folder (or run atualizar-portables.ps1)."
                        }
                        Install::Dev => "Built from source: the release page is opened.",
                    })
                    .color(t.text_muted)
                    .small(),
                );
                if let Some(r) = chosen {
                    get = Some(download_for(r, install).unwrap_or(r.url.as_str()).to_string());
                }
                ui.add_space(4.0);
                if let Some(newest) = fresh.first() {
                    let skipped = app.updates.ignored.as_deref() == Some(newest.version.as_str());
                    let mut skip = skipped;
                    if ui.checkbox(&mut skip, format!("Don't remind me about {}", short(&newest.version))).changed() {
                        app.updates.ignored = skip.then(|| newest.version.clone());
                    }
                }
            }
        }
        ui.checkbox(&mut app.updates.at_start, "Check at start (once a day)");
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Asks GitHub for the releases. Nothing is installed automatically.").color(t.text_muted).small());
        ui.add_space(12.0);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if let Some(url) = get.take() {
                let download = widgets::pill_button(ui, "Download", true).clicked();
                close |= widgets::pill_button(ui, "Later", false).clicked();
                if download {
                    close = true;
                    get = Some(url);
                }
            } else if widgets::pill_button(ui, "Close", true).clicked() {
                close = true;
            }
            recheck = !running(&app.updates.check) && widgets::pill_button(ui, "Check again", false).clicked();
        });
    });
    if modal.should_close() {
        close = true;
        get = None;
    }
    if recheck {
        app.check_for_updates();
        return;
    }
    if close {
        app.updates.open = false;
        if let Some(url) = get {
            app.open_url(&url);
        }
    }
}
