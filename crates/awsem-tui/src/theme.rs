use ratatui::style::{Color, Modifier, Style};

pub const ACCENT: Color = Color::Cyan;
pub const SELECTED: Color = Color::Green;
pub const SUCCESS: Color = Color::Green;
pub const WARN: Color = Color::Yellow;
pub const ERROR: Color = Color::Red;
pub const MUTED: Color = Color::DarkGray;
pub const TEXT: Color = Color::White;

pub fn selected() -> Style { Style::default().fg(SELECTED).add_modifier(Modifier::BOLD) }
pub fn accent() -> Style { Style::default().fg(ACCENT) }
pub fn muted() -> Style { Style::default().fg(MUTED) }
pub fn success() -> Style { Style::default().fg(SUCCESS) }
pub fn warn() -> Style { Style::default().fg(WARN) }


pub fn status_style(s: &str) -> Style {
    if s.contains("RUNNING") || s.contains("COMPLETED") || s.contains("CONFIRMED") || s.contains("Enabled") { success() }
    else if s.contains("FAILED") || s.contains("TERMINATED") || s.contains("ERROR") { Style::default().fg(ERROR) }
    else { Style::default().fg(WARN) }
}

pub fn fmt_size(bytes: i64) -> String {
    if bytes < 1024 { format!("{bytes}B") }
    else if bytes < 1024 * 1024 { format!("{:.1}K", bytes as f64 / 1024.0) }
    else if bytes < 1024 * 1024 * 1024 { format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0)) }
    else { format!("{:.1}G", bytes as f64 / (1024.0 * 1024.0 * 1024.0)) }
}
