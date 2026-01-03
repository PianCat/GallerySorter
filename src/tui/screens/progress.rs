//! 进度屏幕渲染

use crate::tui::components::render_hint;
use crate::tui::state::AppState;
use crate::tui::theme::theme;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, BorderType, Gauge, Paragraph, Wrap},
};
use rust_i18n::t;

/// 渲染进度屏幕
pub fn draw(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let [progress_area, stats_area, file_area, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .areas(area);

    let ratio = state.progress_state.progress_ratio();
    let progress_text = format!(
        "{}/{}",
        state.progress_state.processed(),
        state.progress_state.total_files
    );

    let progress = Gauge::default()
        .block(
            Block::bordered()
                .title(t!("processing_progress"))
                .border_type(BorderType::Rounded),
        )
        .gauge_style(theme().progress())
        .ratio(ratio)
        .label(progress_text);

    frame.render_widget(progress, progress_area);

    let stats = format!(
        "{}: {}  {}: {}  {}: {}  {}: {}",
        t!("stat_processed"),
        state.progress_state.processed(),
        t!("stat_skipped"),
        state.progress_state.skipped(),
        t!("stat_duplicates"),
        state.progress_state.duplicates(),
        t!("stat_failed"),
        state.progress_state.failed()
    );
    let stats_widget = Paragraph::new(stats)
        .style(theme().normal())
        .wrap(Wrap { trim: true });
    frame.render_widget(stats_widget, stats_area);

    if !state.progress_state.current_file.is_empty() {
        let current = format!(
            "{} {}",
            t!("current_file"),
            state.progress_state.current_file
        );
        let current_widget = Paragraph::new(current)
            .style(theme().hint())
            .wrap(Wrap { trim: true });
        frame.render_widget(current_widget, file_area);
    }

    render_hint(&t!("processing_hint"), frame, footer);
}
