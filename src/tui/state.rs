//! TUI state management module
//!
//! Contains all state structures used in TUI applications.

use crate::config::{ClassificationRule, Config, FileOperation, MonthFormat, ProcessingMode};
use crate::process::{FileResult, ProcessingStats};
use ratatui::widgets::ListState;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use unicode_width::UnicodeWidthStr;

/// Trait for enum types that represent selectable options.
/// Provides a unified interface for selection management.
pub trait EnumOption: Clone + Copy + Default + PartialEq + 'static {
    /// Get the index of this variant
    fn to_index(&self) -> usize;

    /// Create a variant from index (returns default if invalid)
    fn from_index(index: usize) -> Self;

    /// Get total number of variants
    fn count() -> usize;

    /// Get all available variants (supports platform filtering)
    fn variants() -> &'static [Self];
}

/// Generic selection state for enum-based options.
/// Provides a unified interface for selection management across different enum types.
#[derive(Debug, Clone, Copy)]
pub struct EnumSelection<E: EnumOption> {
    /// Current selected value
    selected: E,
    /// Phantom data to link to enum type
    _phantom: PhantomData<E>,
}

impl<E: EnumOption> EnumSelection<E> {
    /// Create a new selection with default value
    pub fn new() -> Self {
        Self {
            selected: E::default(),
            _phantom: PhantomData,
        }
    }

    /// Create with a specific selection
    pub fn with_selected(selected: E) -> Self {
        Self {
            selected,
            _phantom: PhantomData,
        }
    }

    /// Get current selected value
    pub fn selected(&self) -> E {
        self.selected
    }

    /// Get current selected index
    pub fn index(&self) -> usize {
        self.selected.to_index()
    }

    /// Set selected value by enum
    pub fn select(&mut self, value: E) {
        self.selected = value;
    }

    /// Set selected value by index
    pub fn select_by_index(&mut self, index: usize) {
        self.selected = E::from_index(index);
    }

    /// Get number of available options
    pub fn count(&self) -> usize {
        E::variants().len()
    }

    /// Select next option (with wrap-around)
    pub fn next(&mut self) {
        let count = self.count();
        let new_index = (self.selected.to_index() + 1) % count;
        self.selected = E::from_index(new_index);
    }

    /// Select previous option (with wrap-around)
    pub fn prev(&mut self) {
        let count = self.count();
        let new_index = if self.selected.to_index() == 0 {
            count - 1
        } else {
            self.selected.to_index() - 1
        };
        self.selected = E::from_index(new_index);
    }
}

impl<E: EnumOption> Default for EnumSelection<E> {
    fn default() -> Self {
        Self::new()
    }
}

/// Boolean selection (for yes/no options)
#[derive(Debug, Clone, Copy, Default)]
pub struct BoolSelection(bool);

impl BoolSelection {
    /// Create a new boolean selection
    pub fn new(value: bool) -> Self {
        Self(value)
    }

    /// Get the value
    pub fn value(&self) -> bool {
        self.0
    }

    /// Toggle the value
    pub fn toggle(&mut self) {
        self.0 = !self.0;
    }

    /// Get count (always 2 for yes/no)
    pub fn count(&self) -> usize {
        2
    }

    /// Get current index
    pub fn index(&self) -> usize {
        if self.0 { 1 } else { 0 }
    }

    /// Select by index
    pub fn select_by_index(&mut self, index: usize) {
        self.0 = index == 1;
    }

    /// Select next
    pub fn next(&mut self) {
        self.toggle();
    }

    /// Select previous
    pub fn prev(&mut self) {
        self.toggle();
    }
}

/// Screen enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    /// Main menu
    #[default]
    MainMenu,
    /// Configuration wizard
    ConfigWizard,
    /// Processing progress
    Progress,
    /// Result summary
    Summary,
    /// Exit confirmation
    Exit,
}

/// Menu items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    /// Run directly
    RunDirect,
    /// Run with configuration
    RunConfig,
    /// Create configuration
    CreateConfig,
    /// Exit
    Exit,
}

impl MenuItem {
    /// Get display label
    pub fn label(&self) -> String {
        match self {
            MenuItem::RunDirect => rust_i18n::t!("menu_option_run_direct").to_string(),
            MenuItem::RunConfig => rust_i18n::t!("menu_option_run_config").to_string(),
            MenuItem::CreateConfig => rust_i18n::t!("menu_option_create_config").to_string(),
            MenuItem::Exit => rust_i18n::t!("menu_option_exit").to_string(),
        }
    }
}

/// Selectable trait - eliminates duplicate code between MenuState and SelectionState
pub trait Selectable {
    /// Get total number of options
    fn count(&self) -> usize;
    /// Get list state reference
    fn list_state(&self) -> &ListState;
    /// Get list state mutable reference
    fn list_state_mut(&mut self) -> &mut ListState;
    /// Select next item
    fn next(&mut self) {
        let count = self.count();
        if let Some(i) = self.list_state().selected() {
            self.list_state_mut().select(Some((i + 1) % count));
        }
    }
    /// Select previous item
    fn prev(&mut self) {
        let count = self.count();
        if let Some(i) = self.list_state().selected() {
            let prev = if i == 0 {
                count.saturating_sub(1)
            } else {
                i - 1
            };
            self.list_state_mut().select(Some(prev));
        }
    }
    /// Select specified index
    fn select(&mut self, index: usize) {
        let count = self.count();
        self.list_state_mut().select(Some(index % count));
    }
    /// Get current selected value (with default)
    fn selected_or_default(&self) -> usize {
        self.list_state().selected().unwrap_or(0)
    }
    /// Get current selected value (optional)
    fn selected(&self) -> Option<usize> {
        self.list_state().selected()
    }
}

/// Menu state
#[derive(Debug, Default)]
pub struct MenuState {
    /// List state (for List widget)
    pub list_state: ListState,
    /// Total number of menu items
    pub count: usize,
}

impl MenuState {
    /// Create new menu state
    pub fn with_count(count: usize) -> Self {
        Self {
            list_state: {
                let mut state = ListState::default();
                state.select(Some(0));
                state
            },
            count,
        }
    }

    /// Get currently selected item index
    pub fn selected(&self) -> usize {
        self.list_state.selected().unwrap_or(0)
    }
}

impl Selectable for MenuState {
    fn count(&self) -> usize {
        self.count
    }

    fn list_state(&self) -> &ListState {
        &self.list_state
    }

    fn list_state_mut(&mut self) -> &mut ListState {
        &mut self.list_state
    }
}

/// Configuration wizard steps
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConfigStep {
    /// Configuration selection
    #[default]
    ConfigSelect,
    /// Confirm whether to modify configuration
    ConfigConfirm,
    /// Configuration name
    ConfigName,
    /// Input directory
    InputDir,
    /// Output directory
    OutputDir,
    /// Exclude directories
    ExcludeDir,
    /// Processing mode
    ProcessingMode,
    /// Classification rule
    Classification,
    /// Month format
    MonthFormat,
    /// File operation
    FileOperation,
    /// Deduplication
    Deduplication,
    /// Dry run
    DryRun,
    /// Classify by type
    ClassifyByType,
    /// Summary
    Summary,
    /// Confirm run
    ConfirmRun,
}

impl ConfigStep {
    /// Get step title
    pub fn title(&self) -> String {
        match self {
            ConfigStep::ConfigSelect => rust_i18n::t!("available_configurations").to_string(),
            ConfigStep::ConfigConfirm => rust_i18n::t!("confirm_modify_config").to_string(),
            ConfigStep::ConfigName => rust_i18n::t!("enter_config_name").to_string(),
            ConfigStep::InputDir => rust_i18n::t!("enter_input_directory").to_string(),
            ConfigStep::OutputDir => rust_i18n::t!("enter_output_directory").to_string(),
            ConfigStep::ExcludeDir => rust_i18n::t!("enter_exclude_directories").to_string(),
            ConfigStep::ProcessingMode => rust_i18n::t!("select_processing_mode").to_string(),
            ConfigStep::Classification => rust_i18n::t!("select_classification_rule").to_string(),
            ConfigStep::MonthFormat => rust_i18n::t!("select_month_format").to_string(),
            ConfigStep::FileOperation => rust_i18n::t!("select_file_operation_mode").to_string(),
            ConfigStep::Deduplication => rust_i18n::t!("enable_deduplication").to_string(),
            ConfigStep::DryRun => rust_i18n::t!("dry_run_mode").to_string(),
            ConfigStep::ClassifyByType => rust_i18n::t!("classify_by_file_type").to_string(),
            ConfigStep::Summary => rust_i18n::t!("configuration_summary").to_string(),
            ConfigStep::ConfirmRun => rust_i18n::t!("proceed_instent").to_string(),
        }
    }

    /// Get number of options
    pub fn option_count(&self) -> usize {
        match self {
            ConfigStep::ConfigSelect => 1, // Actual count returned by ConfigWizardState.option_count()
            ConfigStep::ConfigConfirm => 2,
            ConfigStep::ProcessingMode => 3,
            ConfigStep::Classification => 3,
            ConfigStep::MonthFormat => 2,
            ConfigStep::FileOperation => 3,
            ConfigStep::Deduplication | ConfigStep::DryRun | ConfigStep::ClassifyByType => 2,
            ConfigStep::ConfirmRun => 2,
            _ => 0,
        }
    }

    /// Get option list
    pub fn options(&self) -> Vec<String> {
        match self {
            ConfigStep::ProcessingMode => vec![
                rust_i18n::t!("mode_full").to_string(),
                rust_i18n::t!("mode_supplement").to_string(),
                rust_i18n::t!("mode_incremental").to_string(),
            ],
            ConfigStep::Classification => vec![
                rust_i18n::t!("classify_none").to_string(),
                rust_i18n::t!("classify_year").to_string(),
                rust_i18n::t!("classify_year_month").to_string(),
            ],
            ConfigStep::MonthFormat => vec![
                rust_i18n::t!("month_format_nested").to_string(),
                rust_i18n::t!("month_format_combined").to_string(),
            ],
            ConfigStep::FileOperation => vec![
                rust_i18n::t!("operation_copy").to_string(),
                rust_i18n::t!("operation_move").to_string(),
                rust_i18n::t!("operation_hardlink").to_string(),
                #[cfg(unix)]
                rust_i18n::t!("operation_symlink").to_string(),
            ],
            ConfigStep::Deduplication | ConfigStep::DryRun | ConfigStep::ClassifyByType => vec![
                rust_i18n::t!("option_no").to_string(),
                rust_i18n::t!("option_yes").to_string(),
            ],
            ConfigStep::ConfigSelect => vec![rust_i18n::t!("no_configs_found").to_string()],
            ConfigStep::ConfigConfirm => vec![
                rust_i18n::t!("option_no").to_string(), // Default: no modification, run directly
                rust_i18n::t!("option_yes").to_string(),
            ],
            ConfigStep::ConfirmRun => vec![
                rust_i18n::t!("option_yes").to_string(),
                rust_i18n::t!("option_no").to_string(),
            ],
            _ => vec![],
        }
    }

    /// Get next step
    pub fn next(&self, classification: ClassificationRule) -> Self {
        match self {
            ConfigStep::ConfigSelect => ConfigStep::ConfigConfirm,
            ConfigStep::ConfigConfirm => ConfigStep::InputDir,
            ConfigStep::ConfigName => ConfigStep::InputDir,
            ConfigStep::InputDir => ConfigStep::OutputDir,
            ConfigStep::OutputDir => ConfigStep::ExcludeDir,
            ConfigStep::ExcludeDir => ConfigStep::ProcessingMode,
            ConfigStep::ProcessingMode => ConfigStep::Classification,
            ConfigStep::Classification => {
                if classification == ClassificationRule::YearMonth {
                    ConfigStep::MonthFormat
                } else {
                    ConfigStep::ClassifyByType
                }
            }
            ConfigStep::MonthFormat => ConfigStep::ClassifyByType,
            ConfigStep::ClassifyByType => ConfigStep::Deduplication,
            ConfigStep::Deduplication => ConfigStep::FileOperation,
            ConfigStep::FileOperation => ConfigStep::DryRun,
            ConfigStep::DryRun => ConfigStep::Summary,
            ConfigStep::Summary => ConfigStep::ConfirmRun,
            ConfigStep::ConfirmRun => ConfigStep::ConfirmRun,
        }
    }
}

/// Configuration wizard state
#[derive(Debug, Default)]
pub struct ConfigWizardState {
    /// Current step
    pub step: ConfigStep,
    /// Input directories
    pub input_dirs: String,
    /// Output directory
    pub output_dir: String,
    /// Exclude directories
    pub exclude_dirs: String,
    /// Processing mode selection
    pub processing_mode: EnumSelection<ProcessingMode>,
    /// Classification rule selection
    pub classification: EnumSelection<ClassificationRule>,
    /// Month format selection
    pub month_format: EnumSelection<MonthFormat>,
    /// File operation selection
    pub operation: EnumSelection<FileOperation>,
    /// Deduplication selection
    pub deduplicate: BoolSelection,
    /// Dry run selection
    pub dry_run: BoolSelection,
    /// Classify by type selection
    pub classify_by_type: BoolSelection,
    /// Configuration name
    pub config_name: String,
    /// Available configurations list
    pub available_configs: Vec<PathBuf>,
    /// Selected configuration index
    pub selected_config: Option<usize>,
    /// Validation error message
    pub error_message: Option<String>,
    /// Configuration has been saved
    pub config_saved: bool,
    /// Configuration save path
    pub config_path: Option<PathBuf>,
    /// Whether to skip confirm run step (RunDirect mode)
    pub skip_confirm_run: bool,
}

impl ConfigWizardState {
    /// Create new state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if confirmation is allowed at ConfigSelect step
    /// Returns false if no config files exist (only ESC can be pressed)
    pub fn can_confirm_config_select(&self) -> bool {
        if self.step == ConfigStep::ConfigSelect {
            !self.available_configs.is_empty()
        } else {
            true
        }
    }

    /// Reset boolean option step selection to default (no/0)
    /// Called when entering Deduplication, DryRun, ClassifyByType steps
    pub fn reset_boolean_selection(&mut self) {
        match self.step {
            ConfigStep::Deduplication => {
                self.deduplicate.select_by_index(0);
                self.selected_config = Some(0);
            }
            ConfigStep::DryRun => {
                self.dry_run.select_by_index(0);
                self.selected_config = Some(0);
            }
            ConfigStep::ClassifyByType => {
                self.classify_by_type.select_by_index(0);
                self.selected_config = Some(0);
            }
            _ => {}
        }
    }

    /// Build Config from state
    pub fn build_config(&self) -> Config {
        let input_dirs: Vec<PathBuf> = self
            .input_dirs
            .split(';')
            .map(|s| PathBuf::from(s.trim()))
            .filter(|p| !p.as_os_str().is_empty())
            .collect();

        let exclude_dirs: Vec<PathBuf> = self
            .exclude_dirs
            .split(';')
            .map(|s| PathBuf::from(s.trim()))
            .filter(|p| !p.as_os_str().is_empty())
            .collect();

        Config {
            input_dirs,
            output_dir: PathBuf::from(&self.output_dir),
            exclude_dirs,
            processing_mode: self.processing_mode.selected(),
            classification: self.classification.selected(),
            month_format: self.month_format.selected(),
            classify_by_type: self.classify_by_type.value(),
            operation: self.operation.selected(),
            deduplicate: self.deduplicate.value(),
            dry_run: self.dry_run.value(),
            verbose: false,
            ..Default::default()
        }
    }

    /// Validate current step
    pub fn validate(&self, step: &ConfigStep) -> Result<(), String> {
        match step {
            ConfigStep::InputDir => {
                if self.input_dirs.trim().is_empty() {
                    Err(rust_i18n::t!("no_input_dirs_specified").to_string())
                } else {
                    Ok(())
                }
            }
            ConfigStep::OutputDir => {
                if self.output_dir.trim().is_empty() {
                    Err(rust_i18n::t!("enter_output_directory").to_string())
                } else {
                    Ok(())
                }
            }
            ConfigStep::ConfigName => {
                let name = self.config_name.trim();
                if name.is_empty() {
                    Err(rust_i18n::t!("config_name_empty_error").to_string())
                } else if name.contains('/') || name.contains('\\') || name.contains('.') {
                    Err(rust_i18n::t!("config_name_invalid_chars_error").to_string())
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }

    /// Save configuration file
    pub fn save_config(&mut self) -> Result<PathBuf, String> {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        let config_dir = exe_dir.join("Config");

        // Create config directory
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;

        let config_path = config_dir.join(&self.config_name).with_extension("toml");
        let config = self.build_config();

        toml::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))
            .and_then(|content| {
                std::fs::write(&config_path, content)
                    .map_err(|e| format!("Failed to write config file: {}", e))
            })?;

        self.config_saved = true;
        self.config_path = Some(config_path.clone());

        Ok(config_path)
    }

    /// Refresh available configurations list
    pub fn refresh_configs(&mut self) {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));
        let config_dir = exe_dir.join("Config");
        self.available_configs = if !config_dir.exists() {
            Vec::new()
        } else {
            std::fs::read_dir(&config_dir)
                .ok()
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            e.path()
                                .extension()
                                .map(|ext| ext == std::ffi::OsStr::new("toml"))
                                .unwrap_or(false)
                        })
                        .map(|e| e.path())
                        .collect()
                })
                .unwrap_or_default()
        };
        // If config files exist, select the first one by default
        if self.available_configs.is_empty() {
            self.selected_config = None;
        } else if self.selected_config.is_none() {
            self.selected_config = Some(0);
        }
    }

    /// Get current selected value
    pub fn selected_value(&self) -> usize {
        match self.step {
            ConfigStep::ProcessingMode => self.processing_mode.index(),
            ConfigStep::Classification => self.classification.index(),
            ConfigStep::MonthFormat => self.month_format.index(),
            ConfigStep::FileOperation => self.operation.index(),
            ConfigStep::Deduplication => self.deduplicate.index(),
            ConfigStep::DryRun => self.dry_run.index(),
            ConfigStep::ClassifyByType => self.classify_by_type.index(),
            ConfigStep::ConfirmRun | ConfigStep::ConfigConfirm | ConfigStep::ConfigSelect => {
                self.selected_config.unwrap_or(0)
            }
            _ => 0,
        }
    }

    /// Get number of options for current step (considering available configs)
    pub fn option_count(&self) -> usize {
        match self.step {
            ConfigStep::ConfigSelect => self.available_configs.len().max(1),
            ConfigStep::ConfigConfirm => 2,
            ConfigStep::ProcessingMode => self.processing_mode.count(),
            ConfigStep::Classification => self.classification.count(),
            ConfigStep::MonthFormat => self.month_format.count(),
            ConfigStep::FileOperation => self.operation.count(),
            ConfigStep::Deduplication => self.deduplicate.count(),
            ConfigStep::DryRun => self.dry_run.count(),
            ConfigStep::ClassifyByType => self.classify_by_type.count(),
            ConfigStep::ConfirmRun => 2,
            _ => 0,
        }
    }

    /// Set selected value
    pub fn set_selected(&mut self, index: usize) {
        match self.step {
            ConfigStep::ProcessingMode => self.processing_mode.select_by_index(index),
            ConfigStep::Classification => self.classification.select_by_index(index),
            ConfigStep::MonthFormat => self.month_format.select_by_index(index),
            ConfigStep::FileOperation => self.operation.select_by_index(index),
            ConfigStep::Deduplication => self.deduplicate.select_by_index(index),
            ConfigStep::DryRun => self.dry_run.select_by_index(index),
            ConfigStep::ClassifyByType => self.classify_by_type.select_by_index(index),
            ConfigStep::ConfirmRun | ConfigStep::ConfigConfirm | ConfigStep::ConfigSelect => {
                self.selected_config = Some(index);
            }
            _ => {}
        }
    }

    /// Navigate to next option
    pub fn navigate_next(&mut self) {
        match self.step {
            ConfigStep::ProcessingMode => self.processing_mode.next(),
            ConfigStep::Classification => self.classification.next(),
            ConfigStep::MonthFormat => self.month_format.next(),
            ConfigStep::FileOperation => self.operation.next(),
            ConfigStep::Deduplication => self.deduplicate.next(),
            ConfigStep::DryRun => self.dry_run.next(),
            ConfigStep::ClassifyByType => self.classify_by_type.next(),
            ConfigStep::ConfirmRun | ConfigStep::ConfigConfirm | ConfigStep::ConfigSelect => {
                if let Some(idx) = self.selected_config {
                    let count = self.option_count();
                    self.selected_config = Some((idx + 1) % count);
                }
            }
            _ => {}
        }
    }

    /// Navigate to previous option
    pub fn navigate_prev(&mut self) {
        match self.step {
            ConfigStep::ProcessingMode => self.processing_mode.prev(),
            ConfigStep::Classification => self.classification.prev(),
            ConfigStep::MonthFormat => self.month_format.prev(),
            ConfigStep::FileOperation => self.operation.prev(),
            ConfigStep::Deduplication => self.deduplicate.prev(),
            ConfigStep::DryRun => self.dry_run.prev(),
            ConfigStep::ClassifyByType => self.classify_by_type.prev(),
            ConfigStep::ConfirmRun | ConfigStep::ConfigConfirm | ConfigStep::ConfigSelect => {
                if let Some(idx) = self.selected_config {
                    let count = self.option_count();
                    self.selected_config = Some(if idx == 0 { count - 1 } else { idx - 1 });
                }
            }
            _ => {}
        }
    }
}

/// Input state
#[derive(Debug, Default)]
pub struct InputState {
    /// Input buffer
    pub buffer: String,
    /// Cursor position
    pub cursor: usize,
}

impl InputState {
    /// Create new input state
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with initial value
    pub fn with_value(value: &str) -> Self {
        Self {
            buffer: value.to_string(),
            cursor: value.len(),
        }
    }

    /// Clear input
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
    }

    /// Insert character
    pub fn insert_char(&mut self, c: char) {
        self.buffer.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Delete character before cursor
    pub fn delete_before_cursor(&mut self) {
        if self.cursor > 0 {
            let prev_char_len = self.buffer[..self.cursor]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            self.cursor -= prev_char_len;
            self.buffer.remove(self.cursor);
        }
    }

    /// Delete character after cursor
    pub fn delete_after_cursor(&mut self) {
        if self.cursor < self.buffer.len() {
            let next_char_len = self.buffer[self.cursor..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            self.buffer.drain(self.cursor..self.cursor + next_char_len);
        }
    }

    /// Move cursor left
    pub fn move_cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= self.buffer[..self.cursor]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
        }
    }

    /// Move cursor right
    pub fn move_cursor_right(&mut self) {
        if self.cursor < self.buffer.len() {
            self.cursor += self.buffer[self.cursor..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
        }
    }

    /// Move cursor to start
    pub fn move_cursor_to_start(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end
    pub fn move_cursor_to_end(&mut self) {
        self.cursor = self.buffer.len();
    }

    /// Get visual cursor position
    pub fn visual_cursor_position(&self) -> usize {
        self.buffer[..self.cursor].width()
    }

    /// Get current value
    pub fn value(&self) -> &str {
        &self.buffer
    }
}

/// Progress state
#[derive(Debug)]
pub struct ProgressState {
    /// Processing statistics
    pub stats: Arc<ProcessingStats>,
    /// Total file count
    pub total_files: usize,
    /// Current processing file
    pub current_file: String,
}

impl ProgressState {
    /// Create new progress state
    pub fn new(stats: Arc<ProcessingStats>, total_files: usize) -> Self {
        Self {
            stats,
            total_files,
            current_file: String::new(),
        }
    }

    /// Set current processing file
    pub fn set_current_file(&mut self, file: &str) {
        self.current_file = if file.len() > 50 {
            format!("...{}", &file[file.len() - 47..])
        } else {
            file.to_string()
        };
    }

    /// Get progress ratio
    pub fn progress_ratio(&self) -> f64 {
        let processed = self.stats.processed.load(Ordering::Relaxed);
        let total = self.total_files;
        if total == 0 {
            0.0
        } else {
            processed as f64 / total as f64
        }
    }

    /// Processed count
    pub fn processed(&self) -> usize {
        self.stats.processed.load(Ordering::Relaxed)
    }

    /// Skipped count
    pub fn skipped(&self) -> usize {
        self.stats.skipped.load(Ordering::Relaxed)
    }

    /// Duplicates count
    pub fn duplicates(&self) -> usize {
        self.stats.duplicates.load(Ordering::Relaxed)
    }

    /// Failed count
    pub fn failed(&self) -> usize {
        self.stats.failed.load(Ordering::Relaxed)
    }
}

/// Summary state
#[derive(Debug)]
pub struct SummaryState {
    /// Processing statistics
    pub stats: ProcessingStats,
    /// Processing results
    pub results: Vec<FileResult>,
    /// Dry run mode
    pub dry_run: bool,
    /// Log path
    pub log_path: Option<PathBuf>,
}

impl SummaryState {
    /// Create new summary state
    pub fn new(
        stats: ProcessingStats,
        results: Vec<FileResult>,
        dry_run: bool,
        log_path: Option<PathBuf>,
    ) -> Self {
        Self {
            stats,
            results,
            dry_run,
            log_path,
        }
    }
}

/// List selection state
#[derive(Debug, Default)]
pub struct SelectionState {
    /// ListState for ratatui
    pub list_state: ListState,
    /// Total number of options
    pub count: usize,
}

impl SelectionState {
    /// Create new selection state
    pub fn with_count(count: usize) -> Self {
        Self {
            list_state: {
                let mut state = ListState::default();
                state.select(Some(0));
                state
            },
            count,
        }
    }

    /// Get currently selected index
    pub fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }

    /// Set selected index
    pub fn select(&mut self, index: usize) {
        self.list_state.select(Some(index % self.count));
    }
}

impl Selectable for SelectionState {
    fn count(&self) -> usize {
        self.count
    }

    fn list_state(&self) -> &ListState {
        &self.list_state
    }

    fn list_state_mut(&mut self) -> &mut ListState {
        &mut self.list_state
    }
}
