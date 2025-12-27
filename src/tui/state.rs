//! TUI状态管理模块
//!
//! 包含TUI应用中使用的所有状态结构。

use crate::config::{ClassificationRule, Config, FileOperation, MonthFormat, ProcessingMode};
use crate::process::{FileResult, ProcessingStats};
use ratatui::widgets::ListState;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use unicode_width::UnicodeWidthStr;

/// 屏幕枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    /// 主菜单
    #[default]
    MainMenu,
    /// 配置向导
    ConfigWizard,
    /// 处理进度
    Progress,
    /// 结果摘要
    Summary,
    /// 退出确认
    Exit,
}

/// 菜单项
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    /// 直接输入运行
    RunDirect,
    /// 使用配置运行
    RunConfig,
    /// 创建配置
    CreateConfig,
    /// 退出
    Exit,
}

impl MenuItem {
    /// 获取显示标签
    pub fn label(&self) -> String {
        match self {
            MenuItem::RunDirect => rust_i18n::t!("menu_option_run_direct").to_string(),
            MenuItem::RunConfig => rust_i18n::t!("menu_option_run_config").to_string(),
            MenuItem::CreateConfig => rust_i18n::t!("menu_option_create_config").to_string(),
            MenuItem::Exit => rust_i18n::t!("menu_option_exit").to_string(),
        }
    }
}

/// 可选择项 trait - 消除 MenuState 和 SelectionState 的重复代码
pub trait Selectable {
    /// 获取选项总数
    fn count(&self) -> usize;
    /// 获取列表状态引用
    fn list_state(&self) -> &ListState;
    /// 获取列表状态可变引用
    fn list_state_mut(&mut self) -> &mut ListState;
    /// 选中下一项
    fn next(&mut self) {
        let count = self.count();
        if let Some(i) = self.list_state().selected() {
            self.list_state_mut().select(Some((i + 1) % count));
        }
    }
    /// 选中上一项
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
    /// 选中指定索引
    fn select(&mut self, index: usize) {
        let count = self.count();
        self.list_state_mut().select(Some(index % count));
    }
    /// 获取当前选中值（带默认值）
    fn selected_or_default(&self) -> usize {
        self.list_state().selected().unwrap_or(0)
    }
    /// 获取当前选中值（可选）
    fn selected(&self) -> Option<usize> {
        self.list_state().selected()
    }
}

/// 菜单状态
#[derive(Debug, Default)]
pub struct MenuState {
    /// 列表状态（用于List widget）
    pub list_state: ListState,
    /// 菜单项总数
    pub count: usize,
}

impl MenuState {
    /// 创建新菜单状态
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

    /// 获取当前选中项索引
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

/// 配置向导步骤
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConfigStep {
    /// 配置选择
    #[default]
    ConfigSelect,
    /// 确认是否修改配置
    ConfigConfirm,
    /// 配置名称
    ConfigName,
    /// 输入目录
    InputDir,
    /// 输出目录
    OutputDir,
    /// 排除目录
    ExcludeDir,
    /// 处理模式
    ProcessingMode,
    /// 分类规则
    Classification,
    /// 月份格式
    MonthFormat,
    /// 文件操作
    FileOperation,
    /// 去重
    Deduplication,
    /// 试运行
    DryRun,
    /// 按类型分类
    ClassifyByType,
    /// 摘要确认
    Summary,
    /// 确认执行
    ConfirmRun,
}

impl ConfigStep {
    /// 获取步骤标题
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
            ConfigStep::FileOperation => rust_i18n::t!("select_file_operation").to_string(),
            ConfigStep::Deduplication => rust_i18n::t!("enable_deduplication").to_string(),
            ConfigStep::DryRun => rust_i18n::t!("dry_run_mode").to_string(),
            ConfigStep::ClassifyByType => rust_i18n::t!("classify_by_file_type").to_string(),
            ConfigStep::Summary => rust_i18n::t!("configuration_summary").to_string(),
            ConfigStep::ConfirmRun => rust_i18n::t!("proceed_instent").to_string(),
        }
    }

    /// 获取选项数量
    pub fn option_count(&self) -> usize {
        match self {
            ConfigStep::ConfigSelect => 1, // 实际数量由 ConfigWizardState.option_count() 返回
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

    /// 获取选项列表
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
            ],
            ConfigStep::Deduplication | ConfigStep::DryRun | ConfigStep::ClassifyByType => vec![
                rust_i18n::t!("option_no").to_string(),
                rust_i18n::t!("option_yes").to_string(),
            ],
            ConfigStep::ConfigSelect => vec![rust_i18n::t!("no_configs_found").to_string()],
            ConfigStep::ConfigConfirm => vec![
                rust_i18n::t!("option_no").to_string(), // 默认：不修改，直接执行
                rust_i18n::t!("option_yes").to_string(),
            ],
            ConfigStep::ConfirmRun => vec![
                rust_i18n::t!("option_yes").to_string(),
                rust_i18n::t!("option_no").to_string(),
            ],
            _ => vec![],
        }
    }

    /// 获取下一步
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
                    ConfigStep::FileOperation
                }
            }
            ConfigStep::MonthFormat => ConfigStep::FileOperation,
            ConfigStep::FileOperation => ConfigStep::Deduplication,
            ConfigStep::Deduplication => ConfigStep::DryRun,
            ConfigStep::DryRun => ConfigStep::ClassifyByType,
            ConfigStep::ClassifyByType => ConfigStep::Summary,
            ConfigStep::Summary => ConfigStep::ConfirmRun,
            ConfigStep::ConfirmRun => ConfigStep::ConfirmRun,
        }
    }
}

/// 配置向导状态
#[derive(Debug, Default)]
pub struct ConfigWizardState {
    /// 当前步骤
    pub step: ConfigStep,
    /// 输入目录
    pub input_dirs: String,
    /// 输出目录
    pub output_dir: String,
    /// 排除目录
    pub exclude_dirs: String,
    /// 处理模式
    pub processing_mode: ProcessingMode,
    /// 分类规则
    pub classification: ClassificationRule,
    /// 月份格式
    pub month_format: MonthFormat,
    /// 文件操作
    pub operation: FileOperation,
    /// 去重
    pub deduplicate: bool,
    /// 试运行
    pub dry_run: bool,
    /// 按类型分类
    pub classify_by_type: bool,
    /// 配置名称
    pub config_name: String,
    /// 可用配置列表
    pub available_configs: Vec<PathBuf>,
    /// 选中的配置索引
    pub selected_config: Option<usize>,
    /// 验证错误信息
    pub error_message: Option<String>,
    /// 配置是否已保存
    pub config_saved: bool,
    /// 配置保存路径
    pub config_path: Option<PathBuf>,
    /// 是否跳过确认运行步骤（RunDirect 模式）
    pub skip_confirm_run: bool,
}

impl ConfigWizardState {
    /// 创建新状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 检查在 ConfigSelect 步骤是否可以确认选择
    /// 如果没有配置文件，返回 false（只能按 ESC 返回）
    pub fn can_confirm_config_select(&self) -> bool {
        if self.step == ConfigStep::ConfigSelect {
            !self.available_configs.is_empty()
        } else {
            true
        }
    }

    /// 重置布尔选项步骤的选中状态为默认值（否/0）
    /// 当进入 Deduplication、DryRun、ClassifyByType 步骤时调用
    pub fn reset_boolean_selection(&mut self) {
        match self.step {
            ConfigStep::Deduplication => {
                self.deduplicate = false;
                self.selected_config = Some(0);
            }
            ConfigStep::DryRun => {
                self.dry_run = false;
                self.selected_config = Some(0);
            }
            ConfigStep::ClassifyByType => {
                self.classify_by_type = false;
                self.selected_config = Some(0);
            }
            _ => {}
        }
    }

    /// 从状态构建Config
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
            processing_mode: self.processing_mode,
            classification: self.classification,
            month_format: self.month_format,
            classify_by_type: self.classify_by_type,
            operation: self.operation,
            deduplicate: self.deduplicate,
            dry_run: self.dry_run,
            verbose: false,
            ..Default::default()
        }
    }

    /// 验证当前步骤
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

    /// 保存配置文件
    pub fn save_config(&mut self) -> Result<PathBuf, String> {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        let config_dir = exe_dir.join("Config");

        // 创建配置目录
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

    /// 刷新可用配置列表
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
        // 如果有配置文件，默认选中第一个
        if self.available_configs.is_empty() {
            self.selected_config = None;
        } else if self.selected_config.is_none() {
            self.selected_config = Some(0);
        }
    }

    /// 获取当前选中值
    pub fn selected_value(&self) -> usize {
        match self.step {
            ConfigStep::ProcessingMode => self.processing_mode as usize,
            ConfigStep::Classification => self.classification as usize,
            ConfigStep::MonthFormat => self.month_format as usize,
            ConfigStep::FileOperation => self.operation as usize,
            ConfigStep::Deduplication
            | ConfigStep::DryRun
            | ConfigStep::ClassifyByType
            | ConfigStep::ConfirmRun
            | ConfigStep::ConfigConfirm => self.selected_config.unwrap_or(0),
            ConfigStep::ConfigSelect => self.selected_config.unwrap_or(0),
            _ => 0,
        }
    }

    /// 获取当前步骤的选项数量（考虑可用配置）
    pub fn option_count(&self) -> usize {
        match self.step {
            ConfigStep::ConfigSelect => self.available_configs.len().max(1),
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

    /// 设置选中值
    pub fn set_selected(&mut self, index: usize) {
        match self.step {
            ConfigStep::ProcessingMode => {
                self.processing_mode = match index {
                    0 => ProcessingMode::Full,
                    1 => ProcessingMode::Supplement,
                    2 => ProcessingMode::Incremental,
                    _ => ProcessingMode::Full,
                };
            }
            ConfigStep::Classification => {
                self.classification = match index {
                    0 => ClassificationRule::None,
                    1 => ClassificationRule::Year,
                    2 => ClassificationRule::YearMonth,
                    _ => ClassificationRule::None,
                };
            }
            ConfigStep::MonthFormat => {
                self.month_format = match index {
                    0 => MonthFormat::Nested,
                    1 => MonthFormat::Combined,
                    _ => MonthFormat::Nested,
                };
            }
            ConfigStep::FileOperation => {
                self.operation = match index {
                    0 => FileOperation::Copy,
                    1 => FileOperation::Move,
                    2 => FileOperation::Hardlink,
                    _ => FileOperation::Copy,
                };
            }
            ConfigStep::Deduplication => {
                self.deduplicate = index == 1;
                self.selected_config = Some(index);
            }
            ConfigStep::DryRun => {
                self.dry_run = index == 1;
                self.selected_config = Some(index);
            }
            ConfigStep::ConfirmRun => {
                self.selected_config = Some(index);
            }
            ConfigStep::ClassifyByType => {
                self.classify_by_type = index == 1;
                self.selected_config = Some(index);
            }
            ConfigStep::ConfigSelect => {
                self.selected_config = Some(index);
            }
            ConfigStep::ConfigConfirm => {
                self.selected_config = Some(index);
            }
            _ => {}
        }
    }
}

/// 输入状态
#[derive(Debug, Default)]
pub struct InputState {
    /// 输入缓冲区
    pub buffer: String,
    /// 光标位置
    pub cursor: usize,
}

impl InputState {
    /// 创建新输入状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 使用初始值创建
    pub fn with_value(value: &str) -> Self {
        Self {
            buffer: value.to_string(),
            cursor: value.len(),
        }
    }

    /// 清空输入
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
    }

    /// 插入字符
    pub fn insert_char(&mut self, c: char) {
        self.buffer.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// 删除光标前字符
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

    /// 删除光标后字符
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

    /// 光标左移
    pub fn move_cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= self.buffer[..self.cursor]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
        }
    }

    /// 光标右移
    pub fn move_cursor_right(&mut self) {
        if self.cursor < self.buffer.len() {
            self.cursor += self.buffer[self.cursor..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
        }
    }

    /// 光标移到开头
    pub fn move_cursor_to_start(&mut self) {
        self.cursor = 0;
    }

    /// 光标移到末尾
    pub fn move_cursor_to_end(&mut self) {
        self.cursor = self.buffer.len();
    }

    /// 获取可视光标位置
    pub fn visual_cursor_position(&self) -> usize {
        self.buffer[..self.cursor].width()
    }

    /// 获取当前值
    pub fn value(&self) -> &str {
        &self.buffer
    }
}

/// 进度状态
#[derive(Debug)]
pub struct ProgressState {
    /// 处理统计
    pub stats: Arc<ProcessingStats>,
    /// 总文件数
    pub total_files: usize,
    /// 当前处理文件
    pub current_file: String,
}

impl ProgressState {
    /// 创建新进度状态
    pub fn new(stats: Arc<ProcessingStats>, total_files: usize) -> Self {
        Self {
            stats,
            total_files,
            current_file: String::new(),
        }
    }

    /// 设置当前处理文件
    pub fn set_current_file(&mut self, file: &str) {
        self.current_file = if file.len() > 50 {
            format!("...{}", &file[file.len() - 47..])
        } else {
            file.to_string()
        };
    }

    /// 获取进度比例
    pub fn progress_ratio(&self) -> f64 {
        let processed = self.stats.processed.load(Ordering::Relaxed);
        let total = self.total_files;
        if total == 0 {
            0.0
        } else {
            processed as f64 / total as f64
        }
    }

    /// 已处理数量
    pub fn processed(&self) -> usize {
        self.stats.processed.load(Ordering::Relaxed)
    }

    /// 跳过数量
    pub fn skipped(&self) -> usize {
        self.stats.skipped.load(Ordering::Relaxed)
    }

    /// 重复数量
    pub fn duplicates(&self) -> usize {
        self.stats.duplicates.load(Ordering::Relaxed)
    }

    /// 失败数量
    pub fn failed(&self) -> usize {
        self.stats.failed.load(Ordering::Relaxed)
    }
}

/// 摘要状态
#[derive(Debug)]
pub struct SummaryState {
    /// 处理统计
    pub stats: ProcessingStats,
    /// 处理结果
    pub results: Vec<FileResult>,
    /// 试运行模式
    pub dry_run: bool,
    /// 日志路径
    pub log_path: Option<PathBuf>,
}

impl SummaryState {
    /// 创建新摘要状态
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

/// 列表选择状态
#[derive(Debug, Default)]
pub struct SelectionState {
    /// ListState for ratatui
    pub list_state: ListState,
    /// 选项总数
    pub count: usize,
}

impl SelectionState {
    /// 创建新选择状态
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

    /// 获取当前选中索引
    pub fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }

    /// 设置选中索引
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
