//! Common UI components module
//!
//! Provides reusable UI components to avoid code duplication.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Block, BorderType, Paragraph, Wrap},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::theme::theme;

/// Three-panel layout: header, body, footer
pub fn three_panel_layout(area: Rect) -> [Rect; 3] {
    Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .areas(area)
}

/// Render a centered title block with border
pub fn render_title_block(title: &str, frame: &mut ratatui::prelude::Frame, area: Rect) {
    let title_line = Line::from(format!(" {} ", title))
        .centered()
        .style(theme().title());

    let block = Block::bordered()
        .title(title_line)
        .border_type(BorderType::Rounded)
        .border_style(theme().border())
        .style(theme().normal());

    frame.render_widget(block, area);
}

/// Render bottom hint text
pub fn render_hint(hint: &str, frame: &mut ratatui::prelude::Frame, area: Rect) {
    let hint_widget = Paragraph::new(hint)
        .style(theme().hint())
        .alignment(ratatui::prelude::Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(hint_widget, area);
}

/// 按显示宽度截断文本，避免 UTF-8 边界问题
pub fn truncate_value(value: &str, max_width: usize) -> String {
    if value.width() <= max_width {
        return value.to_string();
    }

    let target_width = max_width.saturating_sub(3);
    let mut current_width = 0;
    let mut output = String::new();

    for ch in value.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width + ch_width > target_width {
            break;
        }
        output.push(ch);
        current_width += ch_width;
    }

    output.push_str("...");
    output
}

/// 按显示宽度换行
pub fn wrap_lines(value: &str, max_width: usize) -> Vec<Line<'static>> {
    if max_width == 0 {
        return vec![Line::from(String::new())];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;

    for ch in value.chars() {
        if ch == '\n' {
            lines.push(Line::from(current));
            current = String::new();
            current_width = 0;
            continue;
        }

        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width + ch_width > max_width && !current.is_empty() {
            lines.push(Line::from(current));
            current = String::new();
            current_width = 0;
        }

        current.push(ch);
        current_width += ch_width;
    }

    if !current.is_empty() || lines.is_empty() {
        lines.push(Line::from(current));
    }

    lines
}
