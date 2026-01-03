//! 退出确认渲染

use crate::tui::theme::theme;
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Paragraph, Wrap},
};
use rust_i18n::t;

/// 渲染退出确认
pub fn draw(frame: &mut Frame, area: Rect) {
    let confirm_text = Paragraph::new(t!("exit_confirm"))
        .style(theme().warning())
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(confirm_text, area);
}
