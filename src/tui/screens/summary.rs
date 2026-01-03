//! 摘要屏幕渲染

use crate::tui::components::{render_hint, render_title_block, three_panel_layout};
use crate::tui::state::AppState;
use crate::tui::theme::theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph, Wrap},
};
use rust_i18n::t;

/// 渲染摘要屏幕
pub fn draw(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let [header, body, footer] = three_panel_layout(area);

    render_title_block(&t!("processing_complete"), frame, header);

    let processed = state
        .summary_state
        .stats
        .processed
        .load(std::sync::atomic::Ordering::Relaxed);
    let skipped = state
        .summary_state
        .stats
        .skipped
        .load(std::sync::atomic::Ordering::Relaxed);
    let duplicates = state
        .summary_state
        .stats
        .duplicates
        .load(std::sync::atomic::Ordering::Relaxed);
    let failed = state
        .summary_state
        .stats
        .failed
        .load(std::sync::atomic::Ordering::Relaxed);

    let mut stats_lines = vec![
        Line::from(t!("statistics")).style(theme().title()),
        Line::from(vec![
            Span::from(t!("stat_processed")),
            Span::raw(": "),
            Span::from(format!("{}", processed)).style(theme().success()),
        ]),
        Line::from(vec![
            Span::from(t!("stat_skipped")),
            Span::raw(": "),
            Span::from(format!("{}", skipped)).style(theme().warning()),
        ]),
        Line::from(vec![
            Span::from(t!("stat_duplicates")),
            Span::raw(": "),
            Span::from(format!("{}", duplicates))
                .style(Style::new().fg(theme().accent).bg(theme().bg)),
        ]),
        Line::from(vec![
            Span::from(t!("stat_failed")),
            Span::raw(": "),
            Span::from(format!("{}", failed)).style(theme().error()),
        ]),
    ];

    if state.summary_state.dry_run {
        stats_lines.push(Line::from(""));
        stats_lines.push(
            Line::from(t!("dry_run_notice")).style(theme().warning().add_modifier(Modifier::BOLD)),
        );
    }

    let failed_count = state
        .summary_state
        .results
        .iter()
        .filter(|r| r.status == crate::process::ProcessingStatus::Failed)
        .count();
    if failed_count > 0 {
        stats_lines.push(Line::from(""));
        stats_lines.push(
            Line::from(format!("{}: {}", t!("failed_files"), failed_count)).style(theme().error()),
        );
    }

    if let Some(ref log_path) = state.summary_state.log_path {
        stats_lines.push(Line::from(""));
        stats_lines.push(
            Line::from(vec![
                Span::from(t!("log_saved_to")),
                Span::raw(" "),
                Span::from(log_path.display().to_string())
                    .style(Style::new().fg(theme().accent).bg(theme().bg)),
            ])
            .style(theme().hint()),
        );
    }

    let stats_para = Paragraph::new(stats_lines)
        .block(Block::bordered().border_type(BorderType::Rounded))
        .style(theme().normal())
        .wrap(Wrap { trim: true });
    frame.render_widget(stats_para, body);

    render_hint(&t!("summary_hint"), frame, footer);
}
