//! UI渲染模块
//!
//! 包含所有屏幕的渲染逻辑，使用Ratatui最佳实践。

use crate::tui::state::{
    ConfigStep, ConfigWizardState, InputState, MenuItem, MenuState, ProgressState, Screen,
    SummaryState,
};
use crate::tui::theme::theme;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Cell, Gauge, List, ListItem, ListState, Paragraph, Row, Table},
};
use rust_i18n::t;

/// TUI运行结果
#[derive(Debug)]
pub struct TuiResult {
    /// 配置
    pub config: crate::config::Config,
    /// 配置名称
    pub config_name: Option<String>,
    /// 是否应在TUI内运行处理
    pub run_processing: bool,
}

/// 应用程序状态（包含UI状态）
#[derive(Debug)]
pub struct AppState {
    /// 当前屏幕
    pub current_screen: Screen,
    /// 菜单状态
    pub menu_state: MenuState,
    /// 配置向导状态
    pub config_wizard: ConfigWizardState,
    /// 进度状态
    pub progress_state: ProgressState,
    /// 摘要状态
    pub summary_state: SummaryState,
    /// 输入状态
    pub input_state: InputState,
    /// 是否应该退出
    pub should_exit: bool,
    /// TUI运行结果
    pub result: Option<TuiResult>,
    /// 选择列表状态（用于配置向导选择）
    pub select_state: ListState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_screen: Screen::MainMenu,
            menu_state: MenuState::with_count(4),
            config_wizard: ConfigWizardState::new(),
            progress_state: ProgressState::new(
                std::sync::Arc::new(crate::process::ProcessingStats::new()),
                0,
            ),
            summary_state: SummaryState::new(
                crate::process::ProcessingStats::new(),
                Vec::new(),
                false,
                None,
            ),
            input_state: InputState::new(),
            should_exit: false,
            result: None,
            select_state: {
                let mut state = ListState::default();
                state.select(Some(0));
                state
            },
        }
    }
}

/// 在TUI内运行处理
pub fn run_processing(
    terminal: &mut DefaultTerminal,
    config: crate::config::Config,
    log_path: Option<std::path::PathBuf>,
) -> std::io::Result<SummaryState> {
    // 创建处理器
    let mut processor = match crate::process::Processor::new(config.clone()) {
        Ok(p) => p,
        Err(_) => {
            // 返回包含错误的状态
            let stats = crate::process::ProcessingStats::new();
            return Ok(SummaryState::new(
                stats,
                Vec::new(),
                config.dry_run,
                log_path,
            ));
        }
    };

    // 先获取文件数量用于进度条
    let total_files = processor.total_files_count().unwrap_or(0);

    // 获取处理器的stats（用于共享）
    let stats = processor.stats_arc();

    // 使用通道传递当前处理的文件名和完成信号
    let (tx, rx) = std::sync::mpsc::channel::<Result<(), ()>>();

    // 初始化应用状态（用于渲染）
    let mut state = AppState {
        current_screen: Screen::Progress,
        progress_state: ProgressState::new(stats.clone(), total_files),
        ..Default::default()
    };

    // 初始渲染进度屏幕
    render(terminal, &mut state)?;

    // 短延时让用户看到进度屏幕
    std::thread::sleep(std::time::Duration::from_millis(100));
    render(terminal, &mut state)?;

    // 在后台线程中运行处理
    let handle = std::thread::spawn(move || {
        // 运行处理
        let results = processor.run().unwrap_or_default();
        let final_stats = (*processor.stats()).clone();

        // 发送完成信号
        let _ = tx.send(Ok(()));

        (final_stats, results)
    });

    // 定期刷新UI，同时检查是否完成
    loop {
        // 检查是否有完成信号
        if let Ok(Ok(())) = rx.recv_timeout(std::time::Duration::from_millis(50)) {
            // 处理完成
            break;
        }

        // 刷新UI - stats 是共享的，会自动反映最新进度
        render(terminal, &mut state)?;
    }

    // 等待线程完成并获取结果
    let (final_stats, results) = handle.join().unwrap();

    // 创建摘要状态
    let summary_state = SummaryState::new(final_stats, results, config.dry_run, log_path);

    Ok(summary_state)
}

/// 设置全局背景（使用Style）
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

/// 渲染整个应用
pub fn render(terminal: &mut DefaultTerminal, state: &mut AppState) -> std::io::Result<()> {
    terminal.draw(|frame| draw(frame, frame.area(), state))?;
    Ok(())
}

/// 主渲染函数
fn draw(frame: &mut Frame, area: Rect, state: &mut AppState) {
    // 设置全局背景
    let buf = frame.buffer_mut();
    set_background(area, buf);

    match state.current_screen {
        Screen::MainMenu => draw_main_menu(frame, area, state),
        Screen::ConfigWizard => draw_config_wizard(frame, area, state),
        Screen::Progress => draw_progress(frame, area, state),
        Screen::Summary => draw_summary(frame, area, state),
        Screen::Exit => draw_exit_confirm(frame, area),
    }
}

/// 绘制主菜单
fn draw_main_menu(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // 标题
    let title = Line::from(" Gallery Sorter ")
        .centered()
        .style(theme().title());

    let title_block = Block::bordered()
        .title(title)
        .border_type(BorderType::Rounded)
        .border_style(theme().border())
        .style(theme().normal());

    frame.render_widget(title_block, header);

    // 菜单列表
    let items: Vec<ListItem> = MenuItem::iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == state.menu_state.selected() {
                theme().selected()
            } else {
                theme().normal()
            };
            ListItem::new(item.label()).style(style)
        })
        .collect();

    let menu_list = List::new(items)
        .block(
            Block::bordered()
                .title(t!("select_option"))
                .border_type(BorderType::Rounded)
                .border_style(theme().border()),
        )
        .highlight_style(theme().selected())
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(menu_list, body, &mut state.menu_state.list_state);

    // 底部提示
    let hint = Paragraph::new(t!("menu_hint"))
        .style(theme().hint())
        .alignment(Alignment::Center);
    frame.render_widget(hint, footer);
}

/// 绘制配置向导
fn draw_config_wizard(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // 步骤标题
    let step_title = Line::from(format!(" {} ", state.config_wizard.step.title()))
        .centered()
        .style(theme().title());

    let title_block = Block::bordered()
        .title(step_title)
        .border_type(BorderType::Rounded)
        .border_style(theme().border())
        .style(theme().normal());

    frame.render_widget(title_block, header);

    // 根据步骤类型渲染不同内容
    match &state.config_wizard.step {
        step if step.option_count() > 0 => {
            draw_selection_step(frame, body, state);
        }
        ConfigStep::InputDir
        | ConfigStep::OutputDir
        | ConfigStep::ExcludeDir
        | ConfigStep::ConfigName => {
            draw_input_step(frame, body, state);
        }
        ConfigStep::Summary => {
            draw_summary_step(frame, body, &state.config_wizard);
        }
        _ => {}
    }

    // 底部提示
    let hint_text = match state.config_wizard.step {
        ConfigStep::InputDir
        | ConfigStep::OutputDir
        | ConfigStep::ExcludeDir
        | ConfigStep::ConfigName => {
            t!("input_hint")
        }
        _ => t!("nav_hint"),
    };

    let hint = Paragraph::new(hint_text)
        .style(theme().hint())
        .alignment(Alignment::Center);
    frame.render_widget(hint, footer);
}

/// 绘制选择步骤
fn draw_selection_step(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let step = &state.config_wizard.step;
    let selected_idx = state.config_wizard.selected_value();

    // 更新选择状态
    state.select_state.select(Some(selected_idx));

    // 如果是配置选择步骤，显示实际配置文件
    if matches!(step, ConfigStep::ConfigSelect) {
        let configs = &state.config_wizard.available_configs;
        let items: Vec<ListItem> = if configs.is_empty() {
            vec![ListItem::new(rust_i18n::t!("no_configs_found")).style(theme().hint())]
        } else {
            configs
                .iter()
                .enumerate()
                .map(|(i, config_path)| {
                    let config_name = config_path
                        .file_stem()
                        .map(|os| os.to_string_lossy().to_string())
                        .unwrap_or_else(|| config_path.display().to_string());
                    let style = if i == selected_idx {
                        theme().selected()
                    } else {
                        theme().normal()
                    };
                    ListItem::new(config_name).style(style)
                })
                .collect()
        };

        let list = List::new(items)
            .block(
                Block::bordered()
                    .title(t!("available_configurations"))
                    .border_type(BorderType::Rounded),
            )
            .highlight_style(theme().selected())
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut state.select_state);
    } else {
        // 其他选择步骤使用预定义选项
        let options = step.options();
        let items: Vec<ListItem> = options
            .into_iter()
            .enumerate()
            .map(|(i, text)| {
                let style = if i == selected_idx {
                    theme().selected()
                } else {
                    theme().normal()
                };
                ListItem::new(text).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::bordered().border_type(BorderType::Rounded))
            .highlight_style(theme().selected())
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut state.select_state);
    }
}

/// 绘制输入步骤
fn draw_input_step(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let [prompt_area, input_area, error_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .areas(area);

    // 如果输入框为空，从wizard状态初始化
    if state.input_state.buffer.is_empty() {
        let value = match state.config_wizard.step {
            ConfigStep::ConfigName => state.config_wizard.config_name.clone(),
            ConfigStep::InputDir => state.config_wizard.input_dirs.clone(),
            ConfigStep::OutputDir => state.config_wizard.output_dir.clone(),
            ConfigStep::ExcludeDir => state.config_wizard.exclude_dirs.clone(),
            _ => String::new(),
        };
        if !value.is_empty() {
            state.input_state.buffer = value;
            state.input_state.cursor = state.input_state.buffer.len();
        }
    }

    let prompt = match state.config_wizard.step {
        ConfigStep::ConfigName => rust_i18n::t!("enter_config_name").to_string(),
        ConfigStep::InputDir => rust_i18n::t!("enter_input_directory").to_string(),
        ConfigStep::OutputDir => rust_i18n::t!("enter_output_directory").to_string(),
        ConfigStep::ExcludeDir => rust_i18n::t!("enter_exclude_directories").to_string(),
        _ => String::new(),
    };

    // 提示文本
    let prompt_para = Paragraph::new(prompt)
        .style(theme().hint())
        .alignment(Alignment::Left);
    frame.render_widget(prompt_para, prompt_area);

    // 输入框
    let input_block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme().border());

    let input_para = Paragraph::new(state.input_state.buffer.as_str())
        .block(input_block)
        .style(theme().normal());

    frame.render_widget(input_para, input_area);

    // 设置光标位置
    let cursor_x = input_area.x + 1 + state.input_state.visual_cursor_position() as u16;
    let cursor_y = input_area.y + 1;
    if cursor_x < input_area.x + input_area.width - 1 {
        frame.set_cursor_position(ratatui::layout::Position {
            x: cursor_x,
            y: cursor_y,
        });
    }

    // 错误信息
    if let Some(ref error) = state.config_wizard.error_message {
        let error_line = Line::from(error.as_str()).style(theme().error());
        frame.render_widget(error_line, error_area);
    }
}

/// 绘制摘要步骤
fn draw_summary_step(frame: &mut Frame, area: Rect, wizard: &ConfigWizardState) {
    let config = wizard.build_config();

    // 获取月份格式的中文显示
    let month_format_text = match config.month_format {
        crate::config::MonthFormat::Nested => t!("month_format_nested"),
        crate::config::MonthFormat::Combined => t!("month_format_combined"),
    };

    let rows = vec![
        Row::new(vec![
            Cell::from(t!("summary_input")),
            Cell::from(format!("{:?}", config.input_dirs)),
        ]),
        Row::new(vec![
            Cell::from(t!("summary_output")),
            Cell::from(config.output_dir.display().to_string()),
        ]),
        Row::new(vec![
            Cell::from(t!("summary_mode")),
            Cell::from(format!("{:?}", config.processing_mode)),
        ]),
        Row::new(vec![
            Cell::from(t!("summary_month_format")),
            Cell::from(month_format_text),
        ]),
        Row::new(vec![
            Cell::from(t!("summary_operation")),
            Cell::from(format!("{:?}", config.operation)),
        ]),
        Row::new(vec![
            Cell::from(t!("summary_deduplicate")),
            Cell::from(config.deduplicate.to_string()),
        ]),
        Row::new(vec![
            Cell::from(t!("summary_dry_run")),
            Cell::from(config.dry_run.to_string()),
        ]),
        Row::new(vec![
            Cell::from(t!("summary_classify_by_type")),
            Cell::from(config.classify_by_type.to_string()),
        ]),
    ];

    let table = Table::new(rows, [Constraint::Length(15), Constraint::Fill(1)])
        .block(
            Block::bordered()
                .title(t!("configuration_summary"))
                .border_type(BorderType::Rounded),
        )
        .column_spacing(2)
        .style(theme().normal());

    frame.render_widget(table, area);
}

/// 绘制进度屏幕
fn draw_progress(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let [progress_area, stats_area, file_area, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // 进度条
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

    // 统计信息
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
    let stats_line = Line::from(stats).style(theme().normal());
    frame.render_widget(stats_line, stats_area);

    // 当前文件
    if !state.progress_state.current_file.is_empty() {
        let current = Line::from(format!(
            "{} {}",
            t!("current_file"),
            state.progress_state.current_file
        ))
        .style(theme().hint());
        frame.render_widget(current, file_area);
    }

    // 底部提示
    let hint = Paragraph::new(t!("processing_hint"))
        .style(theme().hint())
        .alignment(Alignment::Center);
    frame.render_widget(hint, footer);
}

/// 绘制摘要屏幕
fn draw_summary(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // 完成标题
    let title = Line::from(format!(" {} ", t!("processing_complete")))
        .centered()
        .style(theme().title());

    let title_block = Block::bordered()
        .title(title)
        .border_type(BorderType::Rounded)
        .style(theme().normal());

    frame.render_widget(title_block, header);

    // 获取统计值
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

    // 使用强调色创建统计行
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

    // 如果是试运行模式，添加提示
    if state.summary_state.dry_run {
        stats_lines.push(Line::from(""));
        stats_lines.push(
            Line::from(t!("dry_run_notice")).style(theme().warning().add_modifier(Modifier::BOLD)),
        );
    }

    // 如果有失败的文件，显示失败数量
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

    // 显示日志保存路径
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
        .style(theme().normal());
    frame.render_widget(stats_para, body);

    // 底部提示
    let hint = Paragraph::new(t!("summary_hint"))
        .style(theme().hint())
        .alignment(Alignment::Center);
    frame.render_widget(hint, footer);
}

/// 绘制退出确认
fn draw_exit_confirm(frame: &mut Frame, area: Rect) {
    let confirm_text = Line::from(t!("exit_confirm"))
        .centered()
        .style(theme().warning());
    frame.render_widget(confirm_text, area);
}

// 为MenuItem实现iter
impl MenuItem {
    fn iter() -> MenuItemIter {
        MenuItemIter { index: 0 }
    }
}

struct MenuItemIter {
    index: usize,
}

impl Iterator for MenuItemIter {
    type Item = MenuItem;

    fn next(&mut self) -> Option<Self::Item> {
        let item = match self.index {
            0 => Some(MenuItem::RunDirect),
            1 => Some(MenuItem::RunConfig),
            2 => Some(MenuItem::CreateConfig),
            3 => Some(MenuItem::Exit),
            _ => None,
        };
        self.index += 1;
        item
    }
}
