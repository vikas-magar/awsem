use ratatui::style::{Color, Style};

/// 256-color xterm palette — works on Terminal.app, iTerm2, Alacritty, kitty, etc.
pub const ACCENT: Color = Color::Indexed(39);
pub const FG: Color = Color::Indexed(252);
pub const FRAME: Color = ACCENT;
pub const PANEL_BORDER: Color = Color::Indexed(239);
pub const HEADER_BG: Color = Color::Indexed(235);
pub const HEADER_FG: Color = Color::Indexed(15);
pub const SIDEBAR_BG: Color = Color::Indexed(233);
pub const SELECTED: Color = ACCENT;
pub const SELECTED_BG: Color = Color::Indexed(18);
pub const SUCCESS: Color = Color::Indexed(77);
pub const WARN: Color = Color::Indexed(214);
pub const ERROR: Color = Color::Indexed(196);
pub const MUTED: Color = Color::Indexed(244);
pub const LABEL: Color = Color::Indexed(249);
pub const INFO: Color = Color::Indexed(44);
pub const PURPLE: Color = Color::Indexed(135);
pub const DIM: Color = Color::Indexed(240);

pub fn frame() -> Style { Style::default().fg(FRAME) }
pub fn header() -> Style { Style::default().fg(HEADER_FG).bg(HEADER_BG) }
pub fn sidebar() -> Style { Style::default().fg(FG).bg(SIDEBAR_BG) }
pub fn sidebar_active() -> Style { Style::default().fg(ACCENT).bg(SIDEBAR_BG) }
pub fn panel_border() -> Style { Style::default().fg(PANEL_BORDER) }
pub fn selected() -> Style { Style::default().fg(SELECTED).bg(SELECTED_BG) }
pub fn muted() -> Style { Style::default().fg(MUTED) }
pub fn warn() -> Style { Style::default().fg(WARN) }
pub fn error() -> Style { Style::default().fg(ERROR) }
pub fn info() -> Style { Style::default().fg(INFO) }
pub fn dim() -> Style { Style::default().fg(DIM) }
