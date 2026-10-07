//! Folders: open a whole folder of PDFs as one library entry.
//!
//! A folder with many PDFs of the same content (a book split into chapters, a course, scanned
//! issues) shows up in Recent as **one** entry instead of one line per file. Opening it lists its
//! PDFs (subfolders included, natural order: "Chapter 2" before "Chapter 10"); opening a PDF that
//! lives inside a recent folder does not add the PDF to Recent — it bumps the folder and remembers
//! it as the last file read, so the folder card can offer "Continue".

use std::path::Path;

/// How deep subfolders are scanned, and how many PDFs a folder can list.
const MAX_DEPTH: usize = 6;
const MAX_FILES: usize = 5000;
pub const MAX_RECENT_FOLDERS: usize = 12;

/// Reading progress of one PDF: where the reader is, how far they got, and whether they marked it read.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Progress {
    /// Page shown when the file was last open (0-based); reopening the file goes back here.
    pub page: usize,
    /// Furthest page reached (0-based).
    #[serde(default)]
    pub max: usize,
    pub pages: usize,
    /// Marked as read by the user.
    #[serde(default)]
    pub read: bool,
}

impl Progress {
    /// Share of the document read, 0.0–1.0 (1.0 when marked read).
    pub fn fraction(&self) -> f32 {
        if self.read {
            1.0
        } else if self.pages == 0 {
            0.0
        } else {
            ((self.max + 1) as f32 / self.pages as f32).min(1.0)
        }
    }

    /// Record that `page` of a `pages`-page document is on screen. Returns whether anything changed.
    /// Reaching the last page for the first time marks the file read (it can be unmarked by hand).
    pub fn visit(&mut self, page: usize, pages: usize) -> bool {
        let before = self.clone();
        let last = pages.saturating_sub(1);
        self.pages = pages;
        self.page = page.min(last);
        let new_max = self.max.max(self.page);
        if pages > 0 && new_max == last && before.max < last {
            self.read = true;
        }
        self.max = new_max;
        *self != before
    }
}

/// How many of `files` are marked read.
pub fn read_count<'a>(files: impl Iterator<Item = &'a str>, progress: &std::collections::HashMap<String, Progress>) -> usize {
    files.filter(|p| progress.get(*p).is_some_and(|x| x.read)).count()
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RecentFolder {
    pub name: String,
    pub path: String,
    /// PDFs found the last time the folder was opened.
    pub count: usize,
    /// Last PDF opened from this folder.
    #[serde(default)]
    pub last: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FolderEntry {
    pub name: String,
    pub path: String,
    /// Path inside the folder ("Part 1/Chapter 3.pdf"), used for grouping and filtering.
    pub rel: String,
    pub size: u64,
}

#[derive(Clone, Debug, Default)]
pub struct FolderView {
    pub name: String,
    pub path: String,
    pub files: Vec<FolderEntry>,
    pub filter: String,
}

/// Natural ordering: runs of digits compare as numbers, the rest case-insensitively.
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let mut na = String::new();
                while let Some(c) = a.peek().copied().filter(char::is_ascii_digit) {
                    na.push(c);
                    a.next();
                }
                let mut nb = String::new();
                while let Some(c) = b.peek().copied().filter(char::is_ascii_digit) {
                    nb.push(c);
                    b.next();
                }
                let (ta, tb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
                let ord = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb)).then_with(|| na.len().cmp(&nb.len()));
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            (Some(x), Some(y)) => {
                let ord = x.to_lowercase().cmp(y.to_lowercase());
                if ord != Ordering::Equal {
                    return ord;
                }
                a.next();
                b.next();
            }
        }
    }
}

fn is_pdf(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
}

/// List the PDFs of a folder (recursively), in natural order of their path inside the folder.
pub fn scan(root: &Path) -> Vec<FolderEntry> {
    let mut out = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(ft) = e.file_type() else { continue };
            // Hidden entries (".git", "._file.pdf") are skipped.
            if e.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if ft.is_dir() {
                if depth < MAX_DEPTH {
                    stack.push((p, depth + 1));
                }
            } else if is_pdf(&p) && out.len() < MAX_FILES {
                let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().replace('\\', "/");
                out.push(FolderEntry {
                    name: p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                    path: p.to_string_lossy().into_owned(),
                    rel,
                    size: e.metadata().map(|m| m.len()).unwrap_or(0),
                });
            }
        }
    }
    out.sort_by(|a, b| natural_cmp(&a.rel, &b.rel));
    out
}

/// Whether `file` lives inside `folder` (any depth). Compares path components, not raw strings,
/// so "/books2/a.pdf" is not inside "/books".
pub fn contains(folder: &str, file: &str) -> bool {
    let (f, p) = (Path::new(folder), Path::new(file));
    p != f && p.starts_with(f)
}

/// Put `folder` first in the recent folders list (keeping what was known about it).
pub fn bump(list: &mut Vec<RecentFolder>, folder: RecentFolder) {
    let previous = list.iter().position(|r| r.path == folder.path).map(|i| list.remove(i));
    let last = folder.last.clone().or_else(|| previous.and_then(|p| p.last));
    list.insert(0, RecentFolder { last, ..folder });
    list.truncate(MAX_RECENT_FOLDERS);
}

/// Matches the folder filter against the file's path inside the folder (all words must appear).
pub fn matches(entry: &FolderEntry, filter: &str) -> bool {
    let hay = entry.rel.to_lowercase();
    filter.split_whitespace().all(|w| hay.contains(&w.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_tracks_furthest_page_and_read_flag() {
        let mut p = Progress::default();
        assert!(p.visit(4, 10));
        assert!(!p.visit(4, 10)); // same page: nothing to save
        p.visit(2, 10); // going back keeps the furthest page
        assert_eq!((p.page, p.max), (2, 4));
        assert!((p.fraction() - 0.5).abs() < 1e-6);
        assert!(!p.read);
        p.visit(99, 10); // past the end clamps to the last page — and reaching it marks the file read
        assert_eq!(p.max, 9);
        assert!(p.read);
        p.read = false; // unmarked by hand: staying on the last page does not mark it again
        p.visit(9, 10);
        assert!(!p.read);
        p.read = true;
        assert!((p.fraction() - 1.0).abs() < 1e-6);
        let mut m = std::collections::HashMap::new();
        m.insert("a.pdf".to_string(), p);
        m.insert("b.pdf".to_string(), Progress::default());
        assert_eq!(read_count(["a.pdf", "b.pdf", "c.pdf"].into_iter(), &m), 1);
    }

    #[test]
    fn natural_order_puts_2_before_10() {
        let mut v = vec!["Cap 10.pdf", "cap 2.pdf", "Cap 1.pdf", "Anexo.pdf", "Cap 02b.pdf"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, ["Anexo.pdf", "Cap 1.pdf", "cap 2.pdf", "Cap 02b.pdf", "Cap 10.pdf"]);
    }

    #[test]
    fn contains_compares_path_components() {
        let (books, books2) = (std::path::Path::new("/books"), std::path::Path::new("/books2"));
        let inside = books.join("a").join("x.pdf");
        assert!(contains(&books.to_string_lossy(), &inside.to_string_lossy()));
        assert!(!contains(&books.to_string_lossy(), &books2.join("x.pdf").to_string_lossy()));
        assert!(!contains(&books.to_string_lossy(), &books.to_string_lossy()));
    }

    #[test]
    fn bump_moves_to_front_and_keeps_last() {
        let f = |p: &str, last: Option<&str>| RecentFolder { name: p.into(), path: p.into(), count: 1, last: last.map(Into::into) };
        let mut list = vec![f("a", None), f("b", Some("b/1.pdf"))];
        bump(&mut list, f("b", None));
        assert_eq!(list[0].path, "b");
        assert_eq!(list[0].last.as_deref(), Some("b/1.pdf"));
        assert_eq!(list.len(), 2);
        for i in 0..20 {
            bump(&mut list, f(&format!("x{i}"), None));
        }
        assert_eq!(list.len(), MAX_RECENT_FOLDERS);
    }

    #[test]
    fn scan_finds_pdfs_recursively_in_natural_order() {
        let root = std::env::temp_dir().join(format!("printcraft-folders-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Parte 2")).unwrap();
        std::fs::create_dir_all(root.join(".escondida")).unwrap();
        for f in ["Cap 10.pdf", "Cap 2.PDF", "notas.txt", "Parte 2/Cap 1.pdf", ".escondida/x.pdf", "._lixo.pdf"] {
            std::fs::write(root.join(f), b"%PDF-1.4").unwrap();
        }
        let files = scan(&root);
        let rels: Vec<_> = files.iter().map(|f| f.rel.as_str()).collect();
        assert_eq!(rels, ["Cap 2.PDF", "Cap 10.pdf", "Parte 2/Cap 1.pdf"]);
        assert!(matches(&files[2], "parte cap"));
        assert!(!matches(&files[0], "parte"));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
