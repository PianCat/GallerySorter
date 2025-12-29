//! TUI application main module
//!
//! Contains TUI application creation and running logic.

use crate::config::Config;
use crate::process::ProcessingStats;
use crate::tui::event::{EventPoll, TuiEvent};
use crate::tui::state::{ConfigStep, ConfigWizardState, Screen, Selectable};
use crate::tui::ui::{AppState, TuiResult, render};
use ratatui::DefaultTerminal;
use std::path::PathBuf;

/// TUI application
#[derive(Debug)]
pub struct TuiApp {
    /// Terminal
    pub terminal: DefaultTerminal,
    /// Event poller
    pub event_poll: EventPoll,
    /// Application state
    pub state: AppState,
    /// Log path
    log_path: Option<PathBuf>,
    /// Whether to run processing
    should_run_processing: bool,
}

impl TuiApp {
    /// Create new TUI application
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

    /// Set log path
    pub fn set_log_path(&mut self, path: PathBuf) {
        self.log_path = Some(path);
    }

    /// Run application
    pub fn run(&mut self) -> std::io::Result<Option<TuiResult>> {
        // Initial render
        render(&mut self.terminal, &mut self.state)?;

        // Main event loop
        loop {
            // Check if processing should run
            if self.should_run_processing {
                let config = self.state.result.as_ref().map(|r| r.config.clone());
                if let Some(cfg) = config {
                    // Switch to progress screen
                    self.state.current_screen = Screen::Progress;
                    render(&mut self.terminal, &mut self.state)?;

                    // Run processing
                    let summary_state =
                        crate::tui::run_processing(&mut self.terminal, cfg, self.log_path.clone())?;

                    // Set summary state and switch to summary screen
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
                        // If exit request, check if on summary screen
                        if self.state.current_screen == Screen::Summary {
                            // Return to main menu instead of exiting
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

    /// Handle event
    fn handle_event(&mut self, event: TuiEvent) -> std::io::Result<bool> {
        match self.state.current_screen {
            Screen::MainMenu => self.handle_main_menu(event),
            Screen::ConfigWizard => self.handle_config_wizard(event),
            Screen::Progress => self.handle_progress(event),
            Screen::Summary => self.handle_summary(event),
            Screen::Exit => self.handle_exit(event),
        }
    }

    /// Handle main menu event
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
                    self.state.config_wizard.skip_confirm_run = true; // Skip confirm run step
                    self.state.input_state.clear();
                }
                1 => {
                    // RunConfig
                    self.state.current_screen = Screen::ConfigWizard;
                    self.state.config_wizard = ConfigWizardState::new();
                    self.state.config_wizard.step = ConfigStep::ConfigSelect;
                    self.state.config_wizard.skip_confirm_run = false; // Don't skip confirm run
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

    /// Handle config wizard event
    fn handle_config_wizard(&mut self, event: TuiEvent) -> std::io::Result<bool> {
        let step = self.state.config_wizard.step.clone();
        let options_count = self.state.config_wizard.option_count();

        match event {
            TuiEvent::Up | TuiEvent::Left => {
                if options_count > 0 {
                    self.state.config_wizard.navigate_prev();
                } else if options_count == 0
                    && step != ConfigStep::Summary
                    && step != ConfigStep::ConfirmRun
                {
                    self.state.input_state.move_cursor_left();
                }
            }
            TuiEvent::Down | TuiEvent::Right => {
                if options_count > 0 {
                    self.state.config_wizard.navigate_next();
                } else if options_count == 0
                    && step != ConfigStep::Summary
                    && step != ConfigStep::ConfirmRun
                {
                    self.state.input_state.move_cursor_right();
                }
            }
            TuiEvent::Enter => {
                if step == ConfigStep::ConfirmRun {
                    // Save config and handle user selection
                    let selected = self.state.config_wizard.selected_value();
                    if self.state.config_wizard.config_name.is_empty()
                        || !self.state.config_wizard.config_saved
                    {
                        // Save config
                        let _ = self.state.config_wizard.save_config();
                    }

                    if selected == 0 {
                        // Select "yes": run processing
                        let config = self.state.config_wizard.build_config();
                        self.state.result = Some(TuiResult {
                            config,
                            config_name: Some(self.state.config_wizard.config_name.clone()),
                            run_processing: true,
                        });
                        self.should_run_processing = true;
                        return Ok(false); // Continue event loop, processing will happen next round
                    } else {
                        // Select "no": return to main menu
                        self.state.current_screen = Screen::MainMenu;
                        self.state.menu_state = crate::tui::state::MenuState::with_count(4);
                        self.reset_wizard_state();
                        return Ok(false);
                    }
                } else if step == ConfigStep::ConfigSelect
                    && !self.state.config_wizard.can_confirm_config_select()
                {
                    // When no config files exist, ignore Enter key (only ESC can be pressed)
                } else if step.option_count() > 0 {
                    self.confirm_selection();
                } else if step.option_count() == 0 && step != ConfigStep::Summary {
                    self.confirm_input();
                } else if step == ConfigStep::Summary {
                    self.finish_config();
                    // Don't exit, continue event loop to run processing
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
                    self.state.config_wizard.navigate_next();
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

    /// Reset wizard state
    fn reset_wizard_state(&mut self) {
        self.state.config_wizard = ConfigWizardState::new();
        self.state.input_state.clear();
        self.state.result = None;
    }

    /// Handle progress event
    fn handle_progress(&mut self, _event: TuiEvent) -> std::io::Result<bool> {
        Ok(false)
    }

    /// Handle summary event
    fn handle_summary(&mut self, event: TuiEvent) -> std::io::Result<bool> {
        match event {
            TuiEvent::Enter | TuiEvent::Escape => {
                // Return to main menu
                self.state.current_screen = Screen::MainMenu;
                self.state.menu_state = crate::tui::state::MenuState::with_count(4);
                self.state.progress_state =
                    crate::tui::ProgressState::new(std::sync::Arc::new(ProcessingStats::new()), 0);
                self.state.summary_state =
                    crate::tui::SummaryState::new(ProcessingStats::new(), Vec::new(), false, None);
                // Reset wizard state
                self.reset_wizard_state();
            }
            _ => {}
        }
        Ok(false)
    }

    /// Handle exit confirmation
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

    /// Confirm selection
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
                            self.state.config_wizard.processing_mode.select(config.processing_mode);
                            self.state.config_wizard.classification.select(config.classification);
                            self.state.config_wizard.month_format.select(config.month_format);
                            self.state.config_wizard.operation.select(config.operation);
                            self.state.config_wizard.deduplicate.select_by_index(if config.deduplicate { 1 } else { 0 });
                            self.state.config_wizard.dry_run.select_by_index(if config.dry_run { 1 } else { 0 });
                            self.state.config_wizard.classify_by_type.select_by_index(if config.classify_by_type { 1 } else { 0 });
                        }
                    }
                }
                // After selecting config, enter confirm whether to modify config step
                self.state.config_wizard.step = ConfigStep::ConfigConfirm;
                // Reset selection to default value "no" (don't modify)
                self.state.config_wizard.set_selected(0);
            }
            ConfigStep::ConfigConfirm => {
                if selected == 0 {
                    // Select "no": run processing directly
                    let config = self.state.config_wizard.build_config();
                    self.state.result = Some(TuiResult {
                        config,
                        config_name: Some(self.state.config_wizard.config_name.clone()),
                        run_processing: true,
                    });
                    self.should_run_processing = true;
                } else {
                    // Select "yes": enter input directory step to modify
                    self.state.config_wizard.step = ConfigStep::InputDir;
                    self.state.input_state.clear();
                }
            }
            ConfigStep::Classification => {
                self.state.config_wizard.set_selected(selected);
                self.state.config_wizard.step = step.next(self.state.config_wizard.classification.selected());
                // Reset boolean option state for next step
                self.state.config_wizard.reset_boolean_selection();
            }
            _ => {
                self.state.config_wizard.set_selected(selected);
                self.state.config_wizard.step = step.next(self.state.config_wizard.classification.selected());
                // Reset boolean option state for next step
                self.state.config_wizard.reset_boolean_selection();
            }
        }
    }

    /// Confirm input
    fn confirm_input(&mut self) {
        let step = self.state.config_wizard.step.clone();
        let value = self.state.input_state.value().to_string();

        match step {
            ConfigStep::InputDir => {
                self.state.config_wizard.input_dirs = value;
                self.state.config_wizard.step = step.next(self.state.config_wizard.classification.selected());
                self.state.input_state.clear();
                // Reset boolean option state for next step
                self.state.config_wizard.reset_boolean_selection();
            }
            ConfigStep::OutputDir => {
                self.state.config_wizard.output_dir = value;
                self.state.config_wizard.step = step.next(self.state.config_wizard.classification.selected());
                self.state.input_state.clear();
                // Reset boolean option state for next step
                self.state.config_wizard.reset_boolean_selection();
            }
            ConfigStep::ExcludeDir => {
                self.state.config_wizard.exclude_dirs = value;
                self.state.config_wizard.step = step.next(self.state.config_wizard.classification.selected());
                self.state.input_state.clear();
                // Reset boolean option state for next step
                self.state.config_wizard.reset_boolean_selection();
            }
            ConfigStep::ConfigName => {
                self.state.config_wizard.config_name = value.clone();
                if self.state.config_wizard.validate(&step).is_ok() {
                    self.state.config_wizard.step =
                        step.next(self.state.config_wizard.classification.selected());
                    self.state.input_state.clear();
                    // Reset boolean option state for next step
                    self.state.config_wizard.reset_boolean_selection();
                } else {
                    self.state.config_wizard.error_message =
                        self.state.config_wizard.validate(&step).err();
                }
            }
            _ => {}
        }
    }

    /// Finish configuration (advance to ConfirmRun step, unless skip_confirm_run is true)
    fn finish_config(&mut self) {
        if self.state.config_wizard.skip_confirm_run {
            // Run processing directly, skip confirmation step
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
