use eframe::egui;
use std::cmp::Ordering as CmpOrdering;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

/// A single file in the scan results
#[derive(Clone)]
pub struct FileEntry {
    pub name: String,
    /// Absolute path of the file, used to open or copy it
    pub path: String,
    /// Path shown in the table and the exported list: relative to the scan root,
    /// the root itself written as "/", always using "/" as separator
    pub rel_path: String,
    pub size: u64,
    /// Last modification time in seconds since the Unix epoch (0 when unknown)
    pub modified: u64,
}

/// Background scan state: the worker thread writes results into shared memory
pub struct ScanState {
    entries: Arc<Mutex<Vec<FileEntry>>>,
    done: Arc<AtomicBool>,
}

impl ScanState {
    /// Whether the scan has finished (call take_result afterwards to collect results)
    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::Relaxed)
    }

    /// Number of files found so far (used as progress hint)
    pub fn current_len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    /// Take the scan results (only call after is_done returns true)
    pub fn take_result(&self) -> Vec<FileEntry> {
        std::mem::take(&mut *self.entries.lock().unwrap())
    }
}

/// Spawn a background thread that recursively scans a directory and return a
/// pollable state handle. The scan runs on its own thread and does not block the
/// UI; when finished it requests a repaint through `ctx`.
pub fn start_scan(dir: PathBuf, exts: HashSet<String>, ctx: &egui::Context) -> ScanState {
    let entries = Arc::new(Mutex::new(Vec::new()));
    let done = Arc::new(AtomicBool::new(false));

    let e = Arc::clone(&entries);
    let d = Arc::clone(&done);
    let ctx = ctx.clone();

    // Match on the whole file name suffix so that compound extensions such as
    // "tar.gz" are supported as well
    let suffixes: Vec<String> = exts.iter().map(|x| format!(".{}", x.to_lowercase())).collect();

    thread::spawn(move || {
        for entry in WalkDir::new(&dir)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if !suffixes.iter().any(|s| name.ends_with(s.as_str())) {
                continue;
            }
            let path = entry.path();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let modified = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let path_str = path.to_string_lossy().to_string();
            let fe = FileEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                rel_path: display_path(&path_str, &dir),
                path: path_str,
                size,
                modified,
            };
            e.lock().unwrap().push(fe);
        }
        d.store(true, Ordering::Relaxed);
        ctx.request_repaint();
    });

    ScanState { entries, done }
}

/// Sort the results the way a directory tree is read: inside every directory the
/// files come first (A-Z) and only then the sub-directories (A-Z, again with
/// their own files first). Files sitting directly in the scan root therefore
/// lead the list, followed by the sub-directories level by level.
pub fn sort_entries(entries: &mut Vec<FileEntry>, scan_root: &Path) {
    let mut keyed: Vec<(Vec<String>, FileEntry)> = entries
        .drain(..)
        .map(|e| (relative_components(&e.path, scan_root), e))
        .collect();
    keyed.sort_by(|a, b| compare_components(&a.0, &b.0));
    *entries = keyed.into_iter().map(|(_, e)| e).collect();
}

/// Path of a file as it is shown in the table and the exported list: relative to
/// the scan root, with the root itself rendered as "/" and every level joined by
/// "/" — "report.pdf" in the scan root becomes "/report.pdf" and a file inside
/// the sub-directory "docs" becomes "/docs/report.pdf". Files that are not below
/// the scan root keep their full path (again with "/" as separator).
pub fn display_path(path: &str, scan_root: &Path) -> String {
    let p = PathBuf::from(path);
    let Some(rel) = p.strip_prefix(scan_root).ok() else {
        return path.replace('\\', "/");
    };
    let mut out = String::from("/");
    for c in rel.components() {
        if let Component::Normal(s) = c {
            if out.len() > 1 {
                out.push('/');
            }
            out.push_str(&s.to_string_lossy());
        }
    }
    out
}

/// Components of a file path relative to the scan root, lowercased so the sort
/// is A-Z regardless of case. Falls back to the full path when the file is not
/// below the root.
fn relative_components(path: &str, scan_root: &Path) -> Vec<String> {
    let p = PathBuf::from(path);
    let rel = p.strip_prefix(scan_root).unwrap_or(p.as_path());
    rel.components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().to_lowercase()),
            _ => None,
        })
        .collect()
}

/// Compare two relative paths: at every level a file wins over a sub-directory,
/// otherwise the names are compared A-Z.
fn compare_components(a: &[String], b: &[String]) -> CmpOrdering {
    let common = a.len().min(b.len());
    for i in 0..common {
        let a_last = i + 1 == a.len();
        let b_last = i + 1 == b.len();
        if a_last && b_last {
            return a[i].cmp(&b[i]);
        }
        if a_last {
            return CmpOrdering::Less;
        }
        if b_last {
            return CmpOrdering::Greater;
        }
        match a[i].cmp(&b[i]) {
            CmpOrdering::Equal => continue,
            other => return other,
        }
    }
    a.len().cmp(&b.len())
}
