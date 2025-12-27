//! TUI应用主模块
//!
//! 包含TUI应用的创建和运行逻辑。

use crate::config::Config;
use crate::process::ProcessingStats;
use crate::tui::event::{EventPoll, TuiEvent};
use crate::tui::state::{ConfigStep, ConfigWizardState, Screen, Selectable};
use crate::tui::ui::{AppState, TuiResult, render};
use ratatui::DefaultTerminal;
use std::path::PathBuf;

/// TUI应用
#[derive(Debug)]
pub struct TuiApp {
    /// 终端
    pub terminal: DefaultTerminal,
    /// 事件轮询器
    pub event_poll: EventPoll,
    /// 应用状态
    pub state: AppState,
    /// 日志路径
    log_path: Option<PathBuf>,
    /// 是否应运行处理
    should_run_processing: bool,
}

impl TuiApp {
    /// 创建新的TUI应用
    pub fn new() -> std::io::Result<Self> {
        let terminal = ratatui::init();
        let event_poll = EventPoll::default();
        let state = AppState::default();

        Ok(Self {
            terminal,
            event_poll,
            state,
            log_path: None,
            should_run_processing: false,
        })
    }

    /// 设置日志路径
    pub fn set_log_path(&mut self, path: PathBuf) {
        self.log_path = Some(path);
    }

    /// 运行应用
    pub fn run(&mut self) -> std::io::Result<Option<TuiResult>> {
        // 初始渲染
        render(&mut self.terminal, &mut self.state)?;

        // 主事件循环
        loop {
            // 检查是否需要运行处理
            if self.should_run_processing {
                let config = self.state.result.as_ref().map(|r| r.config.clone());
                if let Some(cfg) = config {
                    // 切换到进度屏幕
                    self.state.current_screen = Screen::Progress;
                    render(&mut self.terminal, &mut self.state)?;

                    // 运行处理
                    let summary_state =
                        crate::tui::run_processing(&mut self.terminal, cfg, self.log_path.clone())?;

                    // 设置摘要状态并切换到摘要屏幕
                    self.state.summary_state = summary_state;
                    self.state.current_screen = Screen::Summary;
                    self.should_run_processing = false;
                    render(&mut self.terminal, &mut self.state)?;
                }
            }

            match self.event_poll.next() {
                TuiEvent::Resize(_, _) => {
                    render(&mut self.terminal, &mut self.state)?;
                }
                TuiEvent::CtrlC => {
                    self.state.should_exit = true;
                    self.state.current_screen = Screen::Exit;
                    render(&mut self.terminal, &mut self.state)?;
                    break;
                }
                event => {
                    if self.handle_event(event)? {
                        // 如果是退出请求，检查是否在摘要屏幕
                        if self.state.current_screen == Screen::Summary {
                            // 返回主菜单而不是退出
                            self.state.current_screen = Screen::MainMenu;
                            self.state.menu_state = crate::tui::state::MenuState::with_count(4);
                            self.state.progress_state = crate::tui::ProgressState::new(
                                std::sync::Arc::new(ProcessingStats::new()),
                                0,
                            );
                            self.state.summary_state = crate::tui::SummaryState::new(
                                ProcessingStats::new(),
                                Vec::new(),
                                false,
                                None,
                            );
                            render(&mut self.terminal, &mut self.state)?;
                            continue;
                        }
                        break;
                    }
                    render(&mut self.terminal, &mut self.state)?;
                }
            }
        }

        ratatui::restore();
        Ok(self.state.result.take())
    }

    /// 处理事件
    fn handle_event(&mut self, event: TuiEvent) -> std::io::Result<bool> {
        match self.state.current_screen {
            Screen::MainMenu => self.handle_main_menu(event),
            Screen::ConfigWizard => self.handle_config_wizard(event),
            Screen::Progress => self.handle_progress(event),
            Screen::Summary => self.handle_summary(event),
            Screen::Exit => self.handle_exit(event),
        }
    }

    /// 处理主菜单事件
    fn handle_main_menu(&mut self, event: TuiEvent) -> std::io::Result<bool> {
        match event {
            TuiEvent::Up | TuiEvent::Left => self.state.menu_state.prev(),
            TuiEvent::Down | TuiEvent::Right => self.state.menu_state.next(),
            TuiEvent::Enter => match self.state.menu_state.selected() {
                3 => return Ok(true), // Exit
                0 => {
                    // RunDirect
                    self.state.current_screen = Screen::ConfigWizard;
                    self.state.config_wizard = ConfigWizardState::new();
                    self.state.config_wizard.step = ConfigStep::InputDir;
                    self.state.config_wizard.skip_confirm_run = true; // 跳过确认运行步骤
                    self.state.input_state.clear();
                }
                1 => {
                    // RunConfig
                    self.state.current_screen = Screen::ConfigWizard;
                    self.state.config_wizard = ConfigWizardState::new();
                    self.state.config_wizard.step = ConfigStep::ConfigSelect;
                    self.state.config_wizard.skip_confirm_run = false; // 不跳过确认运行
                    self.state.config_wizard.refresh_configs();
                }
                2 => {
                    // CreateConfig
                    self.state.current_screen = Screen::ConfigWizard;
                    self.state.config_wizard = ConfigWizardState::new();
                    self.state.config_wizard.step = ConfigStep::ConfigName;
                    self.state.config_wizard.skip_confirm_run = false;
                    self.state.input_state.clear();
                }
                _ => {}
            },
            TuiEvent::Escape => return Ok(true),
            _ => {}
        }
        Ok(false)
    }

    /// 处理配置向导事件
    fn handle_config_wizard(&mut self, event: TuiEvent) -> std::io::Result<bool> {
        let step = self.state.config_wizard.step.clone();
        let options_count = self.state.config_wizard.option_count();

        match event {
            TuiEvent::Up | TuiEvent::Left => {
                if options_count > 0 {
                    self.state.select_state.select(None);
                    self.navigate_selection(-1);
                } else if options_count == 0
                    && step != ConfigStep::Summary
                    && step != ConfigStep::ConfirmRun
                {
                    self.state.input_state.move_cursor_left();
                }
            }
            TuiEvent::Down | TuiEvent::Right => {
                if options_count > 0 {
                    self.navigate_selection(1);
                } else if options_count == 0
                    && step != ConfigStep::Summary
                    && step != ConfigStep::ConfirmRun
                {
                    self.state.input_state.move_cursor_right();
                }
            }
            TuiEvent::Enter => {
                if step == ConfigStep::ConfirmRun {
                    // 保存配置并处理用户选择
                    let selected = self.state.config_wizard.selected_value();
                    if self.state.config_wizard.config_name.is_empty()
                        || !self.state.config_wizard.config_saved
                    {
                        // 保存配置
                        let _ = self.state.config_wizard.save_config();
                    }

                    if selected == 0 {
                        // 选择"是"：运行处理
                        let config = self.state.config_wizard.build_config();
                        self.state.result = Some(TuiResult {
                            config,
                            config_name: Some(self.state.config_wizard.config_name.clone()),
                            run_processing: true,
                        });
                        self.should_run_processing = true;
                        return Ok(false); // 继续事件循环，下一轮会处理
                    } else {
                        // 选择"否"：返回主菜单
                        self.state.current_screen = Screen::MainMenu;
                        self.state.menu_state = crate::tui::state::MenuState::with_count(4);
                        self.reset_wizard_state();
                        return Ok(false);
                    }
                } else if step == ConfigStep::ConfigSelect
                    && !self.state.config_wizard.can_confirm_config_select()
                {
                    // 没有配置文件时，忽略 Enter 键（只能按 ESC 返回）
                } else if step.option_count() > 0 {
                    self.confirm_selection();
                } else if step.option_count() == 0 && step != ConfigStep::Summary {
                    self.confirm_input();
                } else if step == ConfigStep::Summary {
                    self.finish_config();
                    // 不退出，继续事件循环以运行处理
                }
            }
            TuiEvent::Char(c) => {
                if step.option_count() == 0
                    && step != ConfigStep::Summary
                    && step != ConfigStep::ConfirmRun
                {
                    self.state.input_state.insert_char(c);
                }
            }
            TuiEvent::Backspace => {
                if step.option_count() == 0
                    && step != ConfigStep::Summary
                    && step != ConfigStep::ConfirmRun
                {
                    self.state.input_state.delete_before_cursor();
                }
            }
            TuiEvent::Delete => {
                if step.option_count() == 0
                    && step != ConfigStep::Summary
                    && step != ConfigStep::ConfirmRun
                {
                    self.state.input_state.delete_after_cursor();
                }
            }
            TuiEvent::Home => {
                if step.option_count() == 0
                    && step != ConfigStep::Summary
                    && step != ConfigStep::ConfirmRun
                {
                    self.state.input_state.move_cursor_to_start();
                }
            }
            TuiEvent::End => {
                if step.option_count() == 0
                    && step != ConfigStep::Summary
                    && step != ConfigStep::ConfirmRun
                {
                    self.state.input_state.move_cursor_to_end();
                }
            }
            TuiEvent::Tab => {
                if step.option_count() > 0 {
                    self.navigate_selection(1);
                }
            }
            TuiEvent::Escape => {
                self.state.current_screen = Screen::MainMenu;
                self.state.menu_state = crate::tui::state::MenuState::with_count(4);
                self.reset_wizard_state();
            }
            _ => {}
        }
        Ok(false)
    }

    /// 重置向导状态
    fn reset_wizard_state(&mut self) {
        self.state.config_wizard = ConfigWizardState::new();
        self.state.input_state.clear();
        self.state.result = None;
    }

    /// 处理进度事件
    fn handle_progress(&mut self, _event: TuiEvent) -> std::io::Result<bool> {
        Ok(false)
    }

    /// 处理摘要事件
    fn handle_summary(&mut self, event: TuiEvent) -> std::io::Result<bool> {
        match event {
            TuiEvent::Enter | TuiEvent::Escape => {
                // 返回主菜单
                self.state.current_screen = Screen::MainMenu;
                self.state.menu_state = crate::tui::state::MenuState::with_count(4);
                self.state.progress_state =
                    crate::tui::ProgressState::new(std::sync::Arc::new(ProcessingStats::new()), 0);
                self.state.summary_state =
                    crate::tui::SummaryState::new(ProcessingStats::new(), Vec::new(), false, None);
                // 重置向导状态
                self.reset_wizard_state();
            }
            _ => {}
        }
        Ok(false)
    }

    /// 处理退出确认
    fn handle_exit(&mut self, event: TuiEvent) -> std::io::Result<bool> {
        match event {
            TuiEvent::Char('y') | TuiEvent::Char('Y') => return Ok(true),
            TuiEvent::Char('n') | TuiEvent::Char('N') | TuiEvent::Escape => {
                self.state.current_screen = Screen::MainMenu;
                self.state.should_exit = false;
            }
            _ => {}
        }
        Ok(false)
    }

    /// 导航选择列表
    fn navigate_selection(&mut self, delta: i32) {
        let options_count = self.state.config_wizard.option_count();

        if options_count == 0 {
            return;
        }

        let current = self.state.config_wizard.selected_value();
        let new = if delta > 0 {
            (current + delta as usize) % options_count
        } else {
            (current as i32 + delta + options_count as i32) as usize % options_count
        };

        self.state.config_wizard.set_selected(new);
    }

    /// 确认选择
    fn confirm_selection(&mut self) {
        let step = self.state.config_wizard.step.clone();
        let selected = self.state.config_wizard.selected_value();

        match step {
            ConfigStep::ConfigSelect => {
                if let Some(idx) = self.state.config_wizard.selected_config {
                    let configs = &self.state.config_wizard.available_configs;
                    if idx < configs.len() {
                        let config_path = &configs[idx];
                        if let Ok(config) = Config::load_from_file(config_path) {
                            self.state.config_wizard.config_name = config_path
                                .file_stem()
                                .map(|os| os.to_string_lossy().to_string())
                                .unwrap_or_default();
                            self.state.config_wizard.input_dirs = config
                                .input_dirs
                                .iter()
                                .map(|p| p.display().to_string())
                                .collect::<Vec<_>>()
                                .join("; ");
                            self.state.config_wizard.output_dir =
                                config.output_dir.display().to_string();
                            self.state.config_wizard.exclude_dirs = config
                                .exclude_dirs
                                .iter()
                                .map(|p| p.display().to_string())
                                .collect::<Vec<_>>()
                                .join("; ");
                            self.state.config_wizard.processing_mode = config.processing_mode;
                            self.state.config_wizard.classification = config.classification;
                            self.state.config_wizard.month_format = config.month_format;
                            self.state.config_wizard.operation = config.operation;
                            self.state.config_wizard.deduplicate = config.deduplicate;
                            self.state.config_wizard.dry_run = config.dry_run;
                            self.state.config_wizard.classify_by_type = config.classify_by_type;
                        }
                    }
                }
                // 选择配置后，进入确认是否修改配置的步骤
                self.state.config_wizard.step = ConfigStep::ConfigConfirm;
                // 重置选中状态为默认值 "否"（不修改）
                self.state.config_wizard.set_selected(0);
            }
            ConfigStep::ConfigConfirm => {
                if selected == 0 {
                    // 选择"否"：直接运行处理
                    let config = self.state.config_wizard.build_config();
                    self.state.result = Some(TuiResult {
                        config,
                        config_name: Some(self.state.config_wizard.config_name.clone()),
                        run_processing: true,
                    });
                    self.should_run_processing = true;
                } else {
                    // 选择"是"：进入输入目录步骤进行修改
                    self.state.config_wizard.step = ConfigStep::InputDir;
                    self.state.input_state.clear();
                }
            }
            ConfigStep::Classification => {
                self.state.config_wizard.set_selected(selected);
                self.state.config_wizard.step = step.next(self.state.config_wizard.classification);
                // 重置下一步的布尔选项状态
                self.state.config_wizard.reset_boolean_selection();
            }
            _ => {
                self.state.config_wizard.set_selected(selected);
                self.state.config_wizard.step = step.next(self.state.config_wizard.classification);
                // 重置下一步的布尔选项状态
                self.state.config_wizard.reset_boolean_selection();
            }
        }
    }

    /// 确认输入
    fn confirm_input(&mut self) {
        let step = self.state.config_wizard.step.clone();
        let value = self.state.input_state.value().to_string();

        match step {
            ConfigStep::InputDir => {
                self.state.config_wizard.input_dirs = value;
                self.state.config_wizard.step = step.next(self.state.config_wizard.classification);
                self.state.input_state.clear();
                // 重置下一步的布尔选项状态
                self.state.config_wizard.reset_boolean_selection();
            }
            ConfigStep::OutputDir => {
                self.state.config_wizard.output_dir = value;
                self.state.config_wizard.step = step.next(self.state.config_wizard.classification);
                self.state.input_state.clear();
                // 重置下一步的布尔选项状态
                self.state.config_wizard.reset_boolean_selection();
            }
            ConfigStep::ExcludeDir => {
                self.state.config_wizard.exclude_dirs = value;
                self.state.config_wizard.step = step.next(self.state.config_wizard.classification);
                self.state.input_state.clear();
                // 重置下一步的布尔选项状态
                self.state.config_wizard.reset_boolean_selection();
            }
            ConfigStep::ConfigName => {
                self.state.config_wizard.config_name = value.clone();
                if self.state.config_wizard.validate(&step).is_ok() {
                    self.state.config_wizard.step =
                        step.next(self.state.config_wizard.classification);
                    self.state.input_state.clear();
                    // 重置下一步的布尔选项状态
                    self.state.config_wizard.reset_boolean_selection();
                } else {
                    self.state.config_wizard.error_message =
                        self.state.config_wizard.validate(&step).err();
                }
            }
            _ => {}
        }
    }

    /// 完成配置（推进到 ConfirmRun 步骤，除非 skip_confirm_run 为 true）
    fn finish_config(&mut self) {
        if self.state.config_wizard.skip_confirm_run {
            // 直接运行处理，跳过确认步骤
            let config = self.state.config_wizard.build_config();
            self.state.result = Some(TuiResult {
                config,
                config_name: Some(self.state.config_wizard.config_name.clone()),
                run_processing: true,
            });
            self.should_run_processing = true;
        } else {
            self.state.config_wizard.step = ConfigStep::ConfirmRun;
        }
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new().expect("Failed to initialize TUI")
    }
}
