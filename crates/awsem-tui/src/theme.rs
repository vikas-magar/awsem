use ratatui::style::{Color, Style};

pub const ACCENT: Color = Color::Rgb(74, 158, 255);
pub const FG: Color = Color::Rgb(192, 192, 192);
pub const FRAME: Color = ACCENT;
pub const PANEL_BORDER: Color = Color::Rgb(58, 58, 58);
pub const HEADER_BG: Color = Color::Rgb(26, 26, 26);
pub const HEADER_FG: Color = Color::Rgb(255, 255, 255);
pub const SIDEBAR_BG: Color = Color::Rgb(13, 13, 13);
pub const SELECTED: Color = ACCENT;
pub const SELECTED_BG: Color = Color::Rgb(22, 34, 50);
pub const SUCCESS: Color = Color::Rgb(74, 154, 106);
pub const WARN: Color = Color::Rgb(154, 138, 74);
pub const ERROR: Color = Color::Rgb(154, 74, 74);
pub const MUTED: Color = Color::Rgb(106, 106, 106);
pub const LABEL: Color = Color::Rgb(138, 138, 138);

pub fn frame() -> Style { Style::default().fg(FRAME) }
pub fn header() -> Style { Style::default().fg(HEADER_FG).bg(HEADER_BG) }
pub fn sidebar() -> Style { Style::default().fg(FG).bg(SIDEBAR_BG) }
pub fn sidebar_active() -> Style { Style::default().fg(SELECTED).bg(SIDEBAR_BG) }
pub fn accent() -> Style { Style::default().fg(ACCENT) }
pub fn panel_border() -> Style { Style::default().fg(PANEL_BORDER) }
pub fn selected() -> Style { Style::default().fg(SELECTED).bg(SELECTED_BG) }
pub fn muted() -> Style { Style::default().fg(MUTED) }
pub fn success() -> Style { Style::default().fg(SUCCESS) }
pub fn warn() -> Style { Style::default().fg(WARN) }
pub fn error() -> Style { Style::default().fg(ERROR) }
