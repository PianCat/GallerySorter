//! UI 渲染与执行逻辑
//!
//! 负责渲染调度与处理执行过程。

use crate::tui::screens;
use crate::tui::state::{AppState, ProgressState, Screen, SummaryState};
use crate::tui::theme::theme;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::Style,
};

/// 在 TUI 内执行处理流程
pub fn run_processing(
    terminal: &mut DefaultTerminal,
    config: crate::config::Config,
    log_path: Option<std::path::PathBuf>,
) -> std::io::Result<SummaryState> {
    let mut processor = match crate::process::Processor::new(config.clone()) {
        Ok(p) => p,
        Err(_) => {
            let stats = crate::process::ProcessingStats::new();
            return Ok(SummaryState::new(
                stats,
                Vec::new(),
                config.dry_run,
                log_path,
            ));
        }
    };

    let total_files = processor.total_files_count().unwrap_or(0);
    let stats = processor.stats_arc();
    let (tx, rx) = std::sync::mpsc::channel::<Result<(), ()>>();

    let mut state = AppState {
        current_screen: Screen::Progress,
        progress_state: ProgressState::new(stats.clone(), total_files),
        ..Default::default()
    };

    render(terminal, &mut state)?;

    std::thread::sleep(std::time::Duration::from_millis(100));
    render(terminal, &mut state)?;

    let handle = std::thread::spawn(move || {
        let results = processor.run().unwrap_or_default();
        let final_stats = (*processor.stats()).clone();

        let _ = tx.send(Ok(()));

        (final_stats, results)
    });

    loop {
        if let Ok(Ok(())) = rx.recv_timeout(std::time::Duration::from_millis(50)) {
            break;
        }

        render(terminal, &mut state)?;
    }

    let (final_stats, results) = handle.join().unwrap();
    let summary_state = SummaryState::new(final_stats, results, config.dry_run, log_path);

    Ok(summary_state)
}

/// 设置背景色
fn set_background(area: Rect, buf: &mut Buffer) {
    let style = Style::new().bg(theme().bg);
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut(ratatui::layout::Position { x, y }) {
                cell.set_style(style);
            }
        }
    }
}

/// 渲染应用
pub fn render(terminal: &mut DefaultTerminal, state: &mut AppState) -> std::io::Result<()> {
    terminal.draw(|frame| draw(frame, frame.area(), state))?;
    Ok(())
}

fn draw(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let buf = frame.buffer_mut();
    set_background(area, buf);

    match state.current_screen {
        Screen::MainMenu => screens::main_menu::draw(frame, area, state),
        Screen::ConfigWizard => screens::config_wizard::draw(frame, area, state),
        Screen::Progress => screens::progress::draw(frame, area, state),
        Screen::Summary => screens::summary::draw(frame, area, state),
        Screen::Exit => screens::exit::draw(frame, area),
    }
}
