//! TUI 应用入口
//!
//! 提供 TUI 应用的创建与运行逻辑。

use crate::config::Config;
use crate::tui::event::{EventPoll, TuiEvent};
use crate::tui::state::{
    AppState, ConfigFormState, ConfigStep, ConfigWizardState, Screen, Selectable, TuiResult,
    reset_to_main_menu,
};
use crate::tui::ui::{render, run_processing};
use ratatui::DefaultTerminal;
use std::path::PathBuf;

/// TUI 应用
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
    /// 是否执行处理
    should_run_processing: bool,
}

impl TuiApp {
    /// 创建 TUI 应用
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
        render(&mut self.terminal, &mut self.state)?;

        loop {
            if self.should_run_processing {
                let config = self.state.result.as_ref().map(|r| r.config.clone());
                if let Some(cfg) = config {
                    self.state.current_screen = Screen::Progress;
                    render(&mut self.terminal, &mut self.state)?;

                    let summary_state =
                        run_processing(&mut self.terminal, cfg, self.log_path.clone())?;

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
                        if self.state.current_screen == Screen::Summary {
                            reset_to_main_menu(&mut self.state);
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

    fn handle_event(&mut self, event: TuiEvent) -> std::io::Result<bool> {
        match self.state.current_screen {
            Screen::MainMenu => self.handle_main_menu(event),
            Screen::ConfigWizard => self.handle_config_wizard(event),
            Screen::Progress => self.handle_progress(event),
            Screen::Summary => self.handle_summary(event),
            Screen::Exit => self.handle_exit(event),
        }
    }

    fn handle_main_menu(&mut self, event: TuiEvent) -> std::io::Result<bool> {
        match event {
            TuiEvent::Up | TuiEvent::Left => self.state.menu_state.prev(),
            TuiEvent::Down | TuiEvent::Right => self.state.menu_state.next(),
            TuiEvent::Enter => match self.state.menu_state.selected() {
                3 => return Ok(true),
                0 => {
                    self.state.current_screen = Screen::ConfigWizard;
                    self.state.config_wizard = ConfigWizardState::new();
                    self.state.config_wizard.step = ConfigStep::ConfigForm;
                    self.state.config_wizard.skip_confirm_run = true;
                    self.state.config_wizard.from_config_select = false;
                    self.state.config_wizard.need_modify_confirm = false;
                    self.state.config_wizard.form_state.selected_field = 0;
                }
                1 => {
                    self.state.current_screen = Screen::ConfigWizard;
                    self.state.config_wizard = ConfigWizardState::new();
                    self.state.config_wizard.step = ConfigStep::ConfigSelect;
                    self.state.config_wizard.skip_confirm_run = false;
                    self.state.config_wizard.from_config_select = true;
                    self.state.config_wizard.need_modify_confirm = true;
                    self.state.config_wizard.refresh_configs();
                }
                2 => {
                    self.state.current_screen = Screen::ConfigWizard;
                    self.state.config_wizard = ConfigWizardState::new();
                    self.state.config_wizard.step = ConfigStep::ConfigForm;
                    self.state.config_wizard.skip_confirm_run = false;
                    self.state.config_wizard.from_config_select = false;
                    self.state.config_wizard.need_modify_confirm = false;
                    self.state.config_wizard.form_state.selected_field = 0;
                }
                _ => {}
            },
            TuiEvent::Escape => return Ok(true),
            _ => {}
        }
        Ok(false)
    }

    fn handle_config_wizard(&mut self, event: TuiEvent) -> std::io::Result<bool> {
        let step = self.state.config_wizard.step.clone();

        match event {
            TuiEvent::Up => {
                if self.state.config_wizard.is_in_input_mode() {
                    self.state.config_wizard.input_move_to_start();
                } else {
                    match step {
                        ConfigStep::ConfigForm => self.state.config_wizard.navigate_form_prev(),
                        ConfigStep::ConfigSelect | ConfigStep::ConfirmRun => {
                            self.state.config_wizard.navigate_prev()
                        }
                        _ => {}
                    }
                }
            }
            TuiEvent::Down => {
                if self.state.config_wizard.is_in_input_mode() {
                    self.state.config_wizard.input_move_to_end();
                } else {
                    match step {
                        ConfigStep::ConfigForm => self.state.config_wizard.navigate_form_next(),
                        ConfigStep::ConfigSelect | ConfigStep::ConfirmRun => {
                            self.state.config_wizard.navigate_next()
                        }
                        _ => {}
                    }
                }
            }
            TuiEvent::Left => {
                if self.state.config_wizard.is_in_input_mode() {
                    self.state.config_wizard.input_move_left();
                } else {
                    match step {
                        ConfigStep::ConfigForm => {
                            if !self.state.config_wizard.is_next_selected() {
                                self.state.config_wizard.toggle_current_field_prev();
                            }
                        }
                        ConfigStep::ConfigSelect | ConfigStep::ConfirmRun => {
                            self.state.config_wizard.navigate_prev();
                        }
                        _ => {}
                    }
                }
            }
            TuiEvent::Right => {
                if self.state.config_wizard.is_in_input_mode() {
                    self.state.config_wizard.input_move_right();
                } else {
                    match step {
                        ConfigStep::ConfigForm => {
                            if !self.state.config_wizard.is_next_selected() {
                                self.state.config_wizard.toggle_current_field_next();
                            }
                        }
                        ConfigStep::ConfigSelect | ConfigStep::ConfirmRun => {
                            self.state.config_wizard.navigate_next();
                        }
                        _ => {}
                    }
                }
            }
            TuiEvent::Enter => match step {
                ConfigStep::ConfigSelect => {
                    if !self.state.config_wizard.can_confirm_config_select() {
                        return Ok(false);
                    }

                    self.state.config_wizard.ensure_selection();
                    let selected = self.state.config_wizard.selected_value();
                    let selected_path = self
                        .state
                        .config_wizard
                        .available_configs
                        .get(selected)
                        .cloned();

                    if let Some(path) = selected_path {
                        match Config::load_from_file(&path) {
                            Ok(config) => {
                                self.state.config_wizard.init_from_config(&config, &path);
                                self.state.config_wizard.config_saved = true;
                                self.state.config_wizard.config_path = Some(path);
                                self.state.config_wizard.error_message = None;
                                self.state.config_wizard.form_state = ConfigFormState::new();
                                self.state.config_wizard.need_modify_confirm = true;
                                self.state.config_wizard.step = ConfigStep::Summary;
                            }
                            Err(err) => {
                                self.state.config_wizard.error_message = Some(err.to_string());
                            }
                        }
                    }
                }
                ConfigStep::ConfirmRun => {
                    self.state.config_wizard.ensure_selection();
                    let selected = self.state.config_wizard.selected_value();

                    if self.state.config_wizard.is_select_config_flow() {
                        if selected == 0 {
                            self.state.config_wizard.need_modify_confirm = false;
                            self.state.config_wizard.form_state = ConfigFormState::new();
                            self.state.config_wizard.step = ConfigStep::ConfigForm;
                        } else {
                            let config = self.state.config_wizard.build_config();
                            self.state.result = Some(TuiResult {
                                config,
                                config_name: Some(self.state.config_wizard.config_name.clone()),
                                run_processing: true,
                            });
                            self.should_run_processing = true;
                            return Ok(false);
                        }
                    } else {
                        if self.state.config_wizard.config_name.is_empty()
                            || !self.state.config_wizard.config_saved
                        {
                            let _ = self.state.config_wizard.save_config();
                        }

                        if selected == 0 {
                            let config = self.state.config_wizard.build_config();
                            self.state.result = Some(TuiResult {
                                config,
                                config_name: Some(self.state.config_wizard.config_name.clone()),
                                run_processing: true,
                            });
                            self.should_run_processing = true;
                            return Ok(false);
                        } else {
                            reset_to_main_menu(&mut self.state);
                            return Ok(false);
                        }
                    }
                }
                ConfigStep::ConfigForm => {
                    if self.state.config_wizard.is_in_input_mode() {
                        self.state.config_wizard.exit_input_mode_apply();
                    } else if self.state.config_wizard.is_next_selected() {
                        if self.state.config_wizard.validate_form().is_ok() {
                            self.state.config_wizard.step = ConfigStep::Summary;
                        } else {
                            self.state.config_wizard.error_message =
                                self.state.config_wizard.validate_form().err();
                        }
                    } else {
                        if let Some(field) = self.state.config_wizard.selected_form_field() {
                            if field.is_input_field() {
                                self.state.config_wizard.enter_input_mode_for_field();
                            }
                        }
                    }
                }
                ConfigStep::Summary => {
                    if self.state.config_wizard.skip_confirm_run
                        || (self.state.config_wizard.is_select_config_flow()
                            && !self.state.config_wizard.need_modify_confirm)
                    {
                        let config = self.state.config_wizard.build_config();
                        let config_name = if self.state.config_wizard.skip_confirm_run {
                            None
                        } else {
                            Some(self.state.config_wizard.config_name.clone())
                        };
                        self.state.result = Some(TuiResult {
                            config,
                            config_name,
                            run_processing: true,
                        });
                        self.should_run_processing = true;
                        return Ok(false);
                    } else {
                        self.state.config_wizard.step = ConfigStep::ConfirmRun;
                        self.state.config_wizard.ensure_selection();
                    }
                }
                _ => {}
            },
            TuiEvent::Char(c) => {
                if self.state.config_wizard.is_in_input_mode() {
                    self.state.config_wizard.input_insert_char(c);
                }
            }
            TuiEvent::Backspace => {
                if self.state.config_wizard.is_in_input_mode() {
                    self.state.config_wizard.input_backspace();
                }
            }
            TuiEvent::Delete => {
                if self.state.config_wizard.is_in_input_mode() {
                    self.state.config_wizard.input_delete();
                }
            }
            TuiEvent::Home => {
                if self.state.config_wizard.is_in_input_mode() {
                    self.state.config_wizard.input_move_to_start();
                }
            }
            TuiEvent::End => {
                if self.state.config_wizard.is_in_input_mode() {
                    self.state.config_wizard.input_move_to_end();
                }
            }
            TuiEvent::Escape => {
                if self.state.config_wizard.is_in_input_mode() {
                    self.state.config_wizard.exit_input_mode_cancel();
                } else {
                    match step {
                        ConfigStep::ConfirmRun => {
                            self.state.config_wizard.step = ConfigStep::Summary;
                        }
                        ConfigStep::ConfigForm => {
                            if self.state.config_wizard.from_config_select {
                                self.state.config_wizard.step = ConfigStep::ConfigSelect;
                            } else {
                                reset_to_main_menu(&mut self.state);
                            }
                        }
                        ConfigStep::Summary => {
                            if self.state.config_wizard.is_select_config_flow()
                                && self.state.config_wizard.need_modify_confirm
                            {
                                self.state.config_wizard.step = ConfigStep::ConfigSelect;
                            } else {
                                self.state.config_wizard.step = ConfigStep::ConfigForm;
                            }
                        }
                        ConfigStep::ConfigSelect | ConfigStep::ConfigName => {
                            reset_to_main_menu(&mut self.state);
                        }
                        _ => {
                            reset_to_main_menu(&mut self.state);
                        }
                    }
                }
            }
            TuiEvent::Tab => {
                if !self.state.config_wizard.is_in_input_mode() {
                    match step {
                        ConfigStep::ConfigForm => self.state.config_wizard.navigate_form_next(),
                        ConfigStep::ConfigSelect | ConfigStep::ConfirmRun => {
                            self.state.config_wizard.navigate_next()
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_progress(&mut self, _event: TuiEvent) -> std::io::Result<bool> {
        Ok(false)
    }

    fn handle_summary(&mut self, event: TuiEvent) -> std::io::Result<bool> {
        match event {
            TuiEvent::Enter | TuiEvent::Escape => {
                reset_to_main_menu(&mut self.state);
            }
            _ => {}
        }
        Ok(false)
    }

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
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new().expect("Failed to initialize TUI")
    }
}
