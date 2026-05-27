use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub struct LocalEntry { pub name: String, pub is_dir: bool, pub size_str: String }

pub struct UploadTransfer {
    pub name: String,
    pub progress: Arc<AtomicU64>,
    pub done: Arc<AtomicBool>,
    pub error: Arc<Mutex<Option<String>>>,
}

impl UploadTransfer {
    pub fn new(name: String) -> Self {
        Self { name, progress: Arc::new(AtomicU64::new(0)), done: Arc::new(AtomicBool::new(false)), error: Arc::new(Mutex::new(None)) }
    }
    pub fn progress_pct(&self) -> u64 { self.progress.load(Ordering::Relaxed) }
    pub fn is_done(&self) -> bool { self.done.load(Ordering::Relaxed) }
}

pub struct UploadState {
    pub path: PathBuf,
    pub entries: Vec<LocalEntry>,
    pub cursor: usize,
    pub scroll: usize,
    pub selected: Vec<usize>,
    pub transfers: Vec<UploadTransfer>,
    pub spinner: u64,
}

impl UploadState {
    pub fn new(base: &str) -> Self {
        let mut s = Self { path: PathBuf::from(base), entries: Vec::new(), cursor: 0, scroll: 0, selected: Vec::new(), transfers: Vec::new(), spinner: 0 };
        s.refresh(); s
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        if let Ok(rd) = std::fs::read_dir(&self.path) {
            for e in rd.flatten() {
                let Ok(m) = e.metadata() else { continue };
                let n = e.file_name().to_string_lossy().to_string();
                if n.starts_with('.') { continue; }
                let szs = if m.is_dir() { String::new() } else { let sz = m.len(); if sz < 1024 { format!("{sz}B") } else if sz < 1024*1024 { format!("{:.0}K", sz as f64/1024.0) } else { format!("{:.1}M", sz as f64/(1024.0*1024.0)) } };
                self.entries.push(LocalEntry { name: n, is_dir: m.is_dir(), size_str: szs });
            }
        }
        self.entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
        if self.cursor >= self.entries.len() { self.cursor = self.entries.len().saturating_sub(1); }
    }

    pub fn toggle_select(&mut self) {
        let idx = self.cursor;
        if let Some(p) = self.selected.iter().position(|&i| i == idx) { self.selected.remove(p); return; }
        if !self.entries.get(idx).is_some_and(|e| e.is_dir) { self.selected.push(idx); return; }
        if !self.selected.contains(&idx) { self.selected.push(idx); }
    }

    pub fn enter_dir(&mut self) {
        if self.entries.get(self.cursor).is_some_and(|e| e.is_dir) {
            self.path.push(&self.entries[self.cursor].name);
            self.cursor = 0; self.scroll = 0; self.refresh();
        }
    }

    pub fn go_up(&mut self) { if self.path.pop() { self.cursor = 0; self.scroll = 0; self.refresh(); } }

    pub fn upload_queue(&self) -> Vec<(String, PathBuf)> {
        let mut all = Vec::new();
        for &i in &self.selected {
            if let Some(e) = self.entries.get(i) {
                if e.is_dir { all.extend(collect_files(&self.path.join(&e.name), &e.name)); }
                else { all.push((e.name.clone(), self.path.join(&e.name))); }
            }
        }
        all
    }
}

pub fn collect_files(dir: &PathBuf, prefix: &str) -> Vec<(String, PathBuf)> {
    let mut files = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if e.file_name().to_string_lossy().starts_with('.') { continue; }
            let Ok(m) = e.metadata() else { continue; };
            let n = e.file_name().to_string_lossy().to_string();
            let full = e.path();
            if m.is_dir() { files.extend(collect_files(&full, &format!("{prefix}/{n}"))); }
            else { files.push((format!("{prefix}/{n}"), full)); }
        }
    }
    files
}
