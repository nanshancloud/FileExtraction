use crate::exporter::{dest_flat, dest_with_structure};
use crate::scanner::FileEntry;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Summary of a move run (the UI layer turns it into a status bar message)
pub struct MoveResult {
    /// Absolute paths of the files that reached the destination
    pub moved: Vec<String>,
    pub total: usize,
    /// Absolute paths that could not be moved
    pub failed: Vec<String>,
}

/// Move the scanned files (or the ones the user ticked) into `out_dir`.
/// The destination layout follows the same rules as the export: flat or
/// mirroring the directory structure of the scan root, with automatic renaming
/// on name conflicts.
pub fn move_files(
    entries: &[FileEntry],
    scan_dir: &str,
    out_dir: &Path,
    keep_structure: bool,
) -> MoveResult {
    let scan_root = PathBuf::from(scan_dir);
    let mut used_names: HashSet<String> = HashSet::new();
    let mut moved = Vec::new();
    let mut failed = Vec::new();

    let _ = std::fs::create_dir_all(out_dir);

    for e in entries {
        let src = PathBuf::from(&e.path);
        let file_name = src.file_name().map(|n| n.to_string_lossy().to_string());
        let Some(file_name) = file_name else {
            failed.push(e.path.clone());
            continue;
        };

        let natural = natural_dest(&src, &scan_root, out_dir, keep_structure);
        // A file that is already at its destination stays where it is: moving it
        // would only leave a renamed copy behind
        if same_path(&natural, &src) {
            moved.push(e.path.clone());
            continue;
        }
        let dest = if keep_structure {
            dest_with_structure(&src, &scan_root, out_dir, &file_name)
        } else {
            dest_flat(&src, out_dir, &file_name, &mut used_names)
        };

        match move_one(&src, &dest) {
            Ok(()) => moved.push(e.path.clone()),
            Err(_) => failed.push(e.path.clone()),
        }
    }

    MoveResult {
        moved,
        total: entries.len(),
        failed,
    }
}

/// Where a file would end up before name conflicts are resolved: below the
/// destination with the layout of the scan root, or flat with its own name
fn natural_dest(src: &Path, scan_root: &Path, out_dir: &Path, keep_structure: bool) -> PathBuf {
    let name = src.file_name().unwrap_or_default();
    if keep_structure {
        let rel = src
            .strip_prefix(scan_root)
            .map(|r| r.to_path_buf())
            .unwrap_or_else(|_| PathBuf::from(name));
        out_dir.join(rel)
    } else {
        out_dir.join(name)
    }
}

/// Whether two paths point at the same file (both are compared as they are
/// given and, when they exist, through their canonical form)
fn same_path(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// Move a single file. `rename` keeps the file identity but it only works
/// inside one volume, so across volumes the file is copied first and deleted
/// afterwards.
fn move_one(src: &Path, dest: &Path) -> std::io::Result<()> {
    // Already at the destination: nothing to do
    if src == dest {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::rename(src, dest).is_ok() {
        return Ok(());
    }
    // Another volume (or a destination rename cannot express): copy, then delete
    match std::fs::copy(src, dest) {
        Ok(_) => std::fs::remove_file(src),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
        let p = dir.join(name);
        fs::write(&p, body).unwrap();
        p
    }

    fn entry(path: &Path) -> FileEntry {
        FileEntry {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            path: path.to_string_lossy().to_string(),
            rel_path: String::new(),
            size: 0,
            modified: 0,
        }
    }

    /// Build "src/c.txt" and "src/docs/a.txt" plus an empty "out" directory
    fn fixture(tag: &str) -> (PathBuf, Vec<FileEntry>, PathBuf) {
        let base = std::env::temp_dir().join(format!("fe_move_test_{tag}"));
        let _ = fs::remove_dir_all(&base);
        let src = base.join("src");
        let sub = src.join("docs");
        let out = base.join("out");
        fs::create_dir_all(&sub).unwrap();
        fs::create_dir_all(&out).unwrap();
        let a = write(&sub, "a.txt", "a");
        let b = write(&sub, "b.txt", "b");
        let c = write(&src, "c.txt", "c");
        let entries = vec![entry(&a), entry(&b), entry(&c)];
        (src, entries, out)
    }

    #[test]
    fn flat_move_puts_every_file_in_one_directory() {
        let (src, entries, out) = fixture("flat");
        let a = src.join("docs").join("a.txt");

        let r = move_files(&entries, &src.to_string_lossy(), &out, false);
        assert_eq!(r.moved.len(), 3);
        assert!(r.failed.is_empty());
        assert!(out.join("a.txt").exists());
        assert!(out.join("b.txt").exists());
        assert!(out.join("c.txt").exists());
        assert!(!a.exists());

        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    #[test]
    fn structured_move_keeps_the_directory_layout() {
        let (src, entries, out) = fixture("structure");

        let r = move_files(&entries, &src.to_string_lossy(), &out, true);
        assert_eq!(r.moved.len(), 3);
        assert!(out.join("docs").join("a.txt").exists());
        assert!(out.join("docs").join("b.txt").exists());
        assert!(out.join("c.txt").exists());

        let _ = fs::remove_dir_all(src.parent().unwrap());
    }

    #[test]
    fn moving_onto_the_source_directory_leaves_the_files_alone() {
        let (src, entries, _) = fixture("same");
        let a = src.join("docs").join("a.txt");

        let r = move_files(&entries, &src.to_string_lossy(), &src, true);
        assert_eq!(r.moved.len(), 3);
        assert!(a.exists());

        let _ = fs::remove_dir_all(src.parent().unwrap());
    }
}
