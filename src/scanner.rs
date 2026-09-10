use eframe::egui;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use walkdir::WalkDir;

/// 扫描结果中的单个文件
#[derive(Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
}

/// 后台扫描状态：工作线程通过共享内存写入结果
pub struct ScanState {
    entries: Arc<Mutex<Vec<FileEntry>>>,
    done: Arc<AtomicBool>,
}

impl ScanState {
    /// 是否已完成（完成后可通过 take_result 取走全部结果）
    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::Relaxed)
    }

    /// 当前已扫描到的文件数（用于进度提示）
    pub fn current_len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    /// 取走扫描结果（仅在 is_done 为 true 后调用）
    pub fn take_result(&self) -> Vec<FileEntry> {
        std::mem::take(&mut *self.entries.lock().unwrap())
    }
}

/// 启动后台线程递归扫描目录，返回可轮询的状态句柄。
/// 扫描在独立线程中进行，不阻塞 UI；完成后通过 ctx 请求重绘。
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
