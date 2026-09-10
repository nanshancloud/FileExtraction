use eframe::egui;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use walkdir::WalkDir;

/// A single file in the scan results
#[derive(Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
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

    thread::spawn(move || {
        for entry in WalkDir::new(&dir)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            if exts.contains(&ext.to_lowercase()) {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                let fe = FileEntry {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: path.to_string_lossy().to_string(),
                    size,
                };
                e.lock().unwrap().push(fe);
            }
        }
        d.store(true, Ordering::Relaxed);
        ctx.request_repaint();
    });

    ScanState { entries, done }
}
