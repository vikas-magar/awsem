use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::style::Stylize;
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Wrap};
use ratatui::Frame;
use crate::theme;

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
    pub fn error_msg(&self) -> Option<String> { self.error.lock().unwrap().take() }
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
        // Directory selected — add dir index; upload_queue expands it
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
                if e.is_dir {
                    all.extend(collect_files(&self.path.join(&e.name), &e.name));
                } else {
                    all.push((e.name.clone(), self.path.join(&e.name)));
                }
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

const SPINNER: &str = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";

pub fn render(frame: &mut Frame, area: Rect, state: &UploadState, bucket: &str, prefix: &str) {
    let block = Block::default().borders(Borders::ALL).border_style(theme::frame()).title(format!(" UPLOAD → s3://{bucket}/{prefix} "));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let mut y = inner.y;

    let p = state.path.to_string_lossy();
    frame.render_widget(Paragraph::new(Line::from(vec![Span::styled(" ", Style::default().fg(theme::FG)), Span::styled(&*p, Style::default().fg(theme::ACCENT))])).wrap(Wrap { trim: false }), Rect::new(inner.x, y, inner.width, 1));
    y += 1;
    frame.render_widget(Paragraph::new(Line::from(Span::styled("─".repeat(inner.width as usize), theme::muted()))), Rect::new(inner.x, y, inner.width, 1));
    y += 1;

    let half = inner.width as usize / 2;
    let list_w = half.saturating_sub(2);
    let xfer_w = inner.width as usize - list_w - 4;
    let list_area = Rect::new(inner.x + 1, y, list_w as u16, (inner.y + inner.height).saturating_sub(y + 2) as u16);
    let xfer_area = Rect::new(inner.x + list_w as u16 + 3, y, xfer_w as u16, list_area.height);

    let max_rows = (list_area.y + list_area.height).saturating_sub(list_area.y) as usize;
    let count = max_rows.min(state.entries.len().saturating_sub(state.scroll));
    for (ly, i) in (list_area.y..).zip((0..count).map(|i| state.scroll + i)) {
        let e = &state.entries[i];
        let sel = i == state.cursor;
        let picked = state.selected.contains(&i);
        let ptr = if sel { "▸" } else { " " };
        let icon = if e.is_dir { if picked { "📂" } else { "📁" } } else if picked { "▸" } else { " " };
        let st = if sel { theme::selected() } else { Style::default().fg(theme::FG) };
        frame.render_widget(Paragraph::new(Line::from(Span::styled(format!("{}{} {}  {}", ptr, icon, e.name, e.size_str), st))), Rect::new(list_area.x, ly, list_area.width, 1));
    }

    let max_ty = xfer_area.y + xfer_area.height;
    for (ty, t) in (xfer_area.y..).zip(state.transfers.iter()) {
        if ty >= max_ty { break; }
        let pct = t.progress_pct();
        let label = if t.is_done() {
            if t.error.lock().unwrap().is_some() { "✕ failed".into() } else { "✓".into() }
        } else {
            let chars: Vec<char> = SPINNER.chars().collect();
            let si = (state.spinner / 3) as usize % chars.len();
            format!("{} {:>3}%", chars[si], pct)
        };
        let st = if t.is_done() { if t.error.lock().unwrap().is_some() { theme::error() } else { theme::success() } } else { theme::warn() };
        frame.render_widget(Paragraph::new(Line::from(vec![Span::styled(&t.name, Style::default().fg(theme::FG)), Span::styled(format!(" {}", label), st)])), Rect::new(xfer_area.x, ty, xfer_area.width, 1));
        if !t.is_done() && pct > 0 {
            let bar_w = xfer_area.width.saturating_sub(t.name.len() as u16 + 8).min(20);
            frame.render_widget(Gauge::default().percent(pct as u16).style(theme::warn()).fg(theme::ACCENT).label(""), Rect::new(xfer_area.x + t.name.len() as u16 + 6, ty, bar_w, 1));
        }
    }

    let hint_y = inner.y + inner.height - 1;
    frame.render_widget(Paragraph::new(Line::from(Span::styled(" ↑↓ nav · space sel(dir=all) · → dir · ← parent · u↩ upload · Esc close ", theme::muted()))), Rect::new(inner.x, hint_y, inner.width, 1));
}
