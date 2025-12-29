//! Theme module
//!
//! Provides unified theme definition, using Stylize trait for concise style settings.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;

/// Theme color configuration
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// Background color (dark theme)
    pub bg: Color,
    /// Foreground color (white)
    pub fg: Color,
    /// Accent color (cyan)
    pub accent: Color,
    /// Selected item background color
    pub selected_bg: Color,
    /// Selected item foreground color
    pub selected_fg: Color,
    /// Success color (green)
    pub success: Color,
    /// Warning color (yellow)
    pub warning: Color,
    /// Error color (red)
    pub error: Color,
    /// Hint/secondary text color (gray)
    pub hint: Color,
    /// Border color
    pub border: Color,
    /// Progress bar color
    pub progress: Color,
    /// Title color
    pub title: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Black,
            fg: Color::White,
            accent: Color::Cyan,
            selected_bg: Color::Cyan,
            selected_fg: Color::Black,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            hint: Color::Gray,
            border: Color::Cyan,
            progress: Color::Cyan,
            title: Color::Cyan,
        }
    }
}

impl Theme {
    /// Create new theme
    pub fn new() -> Self {
        Self::default()
    }

    /// Normal text style
    pub fn normal(&self) -> Style {
        Style::new().fg(self.fg).bg(self.bg)
    }

    /// Title style - using Stylize trait
    pub fn title(&self) -> Style {
        Style::new()
            .fg(self.title)
            .bg(self.bg)
            .add_modifier(Modifier::BOLD)
    }

    /// Selected item style
    pub fn selected(&self) -> Style {
        Style::new()
            .fg(self.selected_fg)
            .bg(self.selected_bg)
            .add_modifier(Modifier::BOLD)
    }

    /// Border style
    pub fn border(&self) -> Style {
        Style::new().fg(self.border).bg(self.bg)
    }

    /// Hint text style
    pub fn hint(&self) -> Style {
        Style::new().fg(self.hint).bg(self.bg)
    }

    /// Success style
    pub fn success(&self) -> Style {
        Style::new().fg(self.success).bg(self.bg)
    }

    /// Warning style
    pub fn warning(&self) -> Style {
        Style::new().fg(self.warning).bg(self.bg)
    }

    /// Error style
    pub fn error(&self) -> Style {
        Style::new().fg(self.error).bg(self.bg)
    }

    /// Progress bar style
    pub fn progress(&self) -> Style {
        Style::new().fg(self.progress).bg(self.bg)
    }

    /// Create centered title line
    pub fn centered_title(&self, text: String) -> Line<'static> {
        Line::from(text).centered().style(self.title())
    }

    /// Create styled line
    pub fn styled_line<'a>(&self, text: String, style: Style) -> Line<'a> {
        Line::from(text).style(style)
    }
}

/// Global theme instance
pub static THEME: Theme = Theme {
    bg: Color::Black,
    fg: Color::White,
    accent: Color::Cyan,
    selected_bg: Color::Cyan,
    selected_fg: Color::Black,
    success: Color::Green,
    warning: Color::Yellow,
    error: Color::Red,
    hint: Color::Gray,
    border: Color::Cyan,
    progress: Color::Cyan,
    title: Color::Cyan,
};

/// Get global theme reference
pub fn theme() -> &'static Theme {
    &THEME
}
