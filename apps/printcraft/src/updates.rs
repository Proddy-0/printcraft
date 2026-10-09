//! Asks GitHub for the Print Labs releases (Help ▸ Check for updates; Proddyt Switch, LABS-156),
//! and tells which release this copy is and how it was installed.

use std::path::Path;
use std::time::Duration;

use printcraft_ui_egui::updates::{DOWNLOADS, Install, LISTED, RELEASES_PAGE, Release, under};

const RELEASES: &str = "https://api.github.com/repos/Proddyt-Labs/print-labs/releases?per_page=10";

/// The latest releases, newest first. The answer is untrusted: its size is capped, only pages
/// under [`RELEASES_PAGE`] and files under [`DOWNLOADS`] are kept, and drafts are skipped.
pub fn releases() -> Result<Vec<Release>, String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .tls_config(ureq::tls::TlsConfig::builder().root_certs(os_roots()?).build())
        .build()
        .new_agent();
    let mut response = agent
        .get(RELEASES)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", concat!("PrintLabs/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|e| format!("couldn't reach GitHub ({e})"))?;
    let body = response.body_mut().with_config().limit(4 << 20).read_to_string().map_err(|e| format!("unreadable answer ({e})"))?;
    parse(&body)
}

/// The certificate authorities the operating system trusts.
fn os_roots() -> Result<ureq::tls::RootCerts, String> {
    let found = rustls_native_certs::load_native_certs();
    let certs: Vec<ureq::tls::Certificate<'static>> = found.certs.iter().map(|c| ureq::tls::Certificate::from_der(c.as_ref()).to_owned()).collect();
    if certs.is_empty() {
        return Err("no trusted certificates found on this system".into());
    }
    Ok(ureq::tls::RootCerts::new_with_certs(&certs))
}

fn parse(body: &str) -> Result<Vec<Release>, String> {
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| format!("unreadable answer ({e})"))?;
    let list = v.as_array().ok_or("no release found")?;
    let releases: Vec<Release> = list
        .iter()
        .filter(|r| !r["draft"].as_bool().unwrap_or(false))
        .filter_map(|r| {
            let version = r["tag_name"].as_str().filter(|t| !t.is_empty() && t.len() <= 64)?.to_string();
            let url = r["html_url"].as_str().filter(|u| under(u, RELEASES_PAGE)).unwrap_or(RELEASES_PAGE).to_string();
            let assets = r["assets"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|f| {
                            let name = f["name"].as_str().filter(|n| !n.is_empty() && n.len() <= 128)?;
                            let link = f["browser_download_url"].as_str().filter(|u| under(u, DOWNLOADS))?;
                            Some((name.to_string(), link.to_string()))
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(Release { version, url, assets })
        })
        .take(LISTED)
        .collect();
    if releases.is_empty() {
        return Err("no release found".into());
    }
    Ok(releases)
}

/// This copy's version and install kind, from the files next to the program: the release workflow
/// writes `VERSION` (the release tag) into both the portable archive and the installed folder, and
/// the Windows installer leaves its uninstaller there.
pub fn installed() -> (String, Install) {
    let dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
    let Some(dir) = dir else { return (env!("CARGO_PKG_VERSION").to_string(), Install::Dev) };
    let tag = std::fs::read_to_string(dir.join("VERSION")).ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty() && t.len() <= 64);
    match tag {
        None => (env!("CARGO_PKG_VERSION").to_string(), Install::Dev),
        Some(tag) if dir.join("unins000.exe").exists() => (tag, Install::Installed),
        Some(tag) => (tag, Install::Portable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_are_read_and_only_our_links_are_kept() {
        let body = r#"[
            {"tag_name":"v0.2.1-labs.20261009.845a253","html_url":"https://github.com/Proddyt-Labs/print-labs/releases/tag/v0.2.1-labs.20261009.845a253",
             "assets":[{"name":"printcraft-windows-x64-setup.exe","browser_download_url":"https://github.com/Proddyt-Labs/print-labs/releases/download/v0.2.1-labs.20261009.845a253/printcraft-windows-x64-setup.exe"},
                       {"name":"evil.exe","browser_download_url":"https://example.com/evil.exe"},
                       {"name":"up.exe","browser_download_url":"https://github.com/Proddyt-Labs/print-labs/releases/download/../../x"}]},
            {"tag_name":"v0.2.0","draft":true,"html_url":"https://github.com/Proddyt-Labs/print-labs/releases/tag/v0.2.0"},
            {"tag_name":"v0.1.0","html_url":"javascript:alert(1)"}
        ]"#;
        let r = parse(body).unwrap();
        assert_eq!(r.len(), 2, "the draft is skipped");
        assert_eq!(r[0].assets.len(), 1, "{:?}", r[0].assets);
        assert_eq!(r[0].assets[0].0, "printcraft-windows-x64-setup.exe");
        assert_eq!(r[1].url, RELEASES_PAGE, "a link elsewhere falls back to the list");
        for elsewhere in [
            "https://example.com/printcraft.exe",
            "https://github.com/Proddyt-Labs/print-labs/releases.evil/x",
            "https://github.com/Proddyt-Labs/print-labs/releases/tag/%2e%2e",
            "https://github.com/Proddyt-Labs/print-labs/releases/tag/a b",
            "javascript:alert(1)",
        ] {
            let r = parse(&format!(r#"[{{"tag_name":"v9.9.9","html_url":"{elsewhere}"}}]"#)).unwrap();
            assert_eq!(r[0].url, RELEASES_PAGE, "{elsewhere}");
        }
        assert!(parse(r#"{"message":"Not Found"}"#).is_err());
        assert!(parse("[]").is_err());
        assert!(parse("<html>").is_err());
    }

    /// Live: asks GitHub over TLS with the OS's roots (`cargo test -p printcraft -- --ignored`).
    #[test]
    #[ignore = "needs network access"]
    fn github_answers_with_the_releases() {
        let r = releases().unwrap();
        assert!(printcraft_ui_egui::updates::is_newer(&r[0].version, "0.0.0"), "{r:?}");
        assert!(r[0].url.starts_with(RELEASES_PAGE), "{r:?}");
    }
}
