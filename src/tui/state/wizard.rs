//! 配置向导状态

use crate::config::{
    ClassificationRule, Config, EnumOption, FileOperation, MonthFormat, ProcessingMode,
};
use crate::tui::labels::{
    bool_label, classification_label, file_operation_label, month_format_label,
    processing_mode_label,
};
use crate::tui::state::input::InputState;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

/// 枚举选择状态
#[derive(Debug, Clone, Copy)]
pub struct EnumSelection<E: EnumOption> {
    selected: E,
    _phantom: PhantomData<E>,
}

impl<E: EnumOption> EnumSelection<E> {
    /// 创建选择状态
    pub fn new() -> Self {
        Self {
            selected: E::default(),
            _phantom: PhantomData,
        }
    }

    /// 使用指定选项创建
    pub fn with_selected(selected: E) -> Self {
        Self {
            selected,
            _phantom: PhantomData,
        }
    }

    /// 获取当前选项
    pub fn selected(&self) -> E {
        self.selected
    }

    /// 获取当前索引
    pub fn index(&self) -> usize {
        self.selected.to_index()
    }

    /// 选择指定选项
    pub fn select(&mut self, value: E) {
        self.selected = value;
    }

    /// 根据索引选择
    pub fn select_by_index(&mut self, index: usize) {
        self.selected = E::from_index(index);
    }

    /// 可选项数量
    pub fn count(&self) -> usize {
        E::variants().len()
    }

    /// 选择下一个
    pub fn next(&mut self) {
        let count = self.count();
        let new_index = (self.selected.to_index() + 1) % count;
        self.selected = E::from_index(new_index);
    }

    /// 选择上一个
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

/// 布尔值选择
#[derive(Debug, Clone, Copy, Default)]
pub struct BoolSelection(bool);

impl BoolSelection {
    /// 创建布尔选择
    pub fn new(value: bool) -> Self {
        Self(value)
    }

    /// 获取值
    pub fn value(&self) -> bool {
        self.0
    }

    /// 切换值
    pub fn toggle(&mut self) {
        self.0 = !self.0;
    }

    /// 可选项数量（固定 2）
    pub fn count(&self) -> usize {
        2
    }

    /// 当前索引
    pub fn index(&self) -> usize {
        if self.0 { 1 } else { 0 }
    }

    /// 根据索引设置
    pub fn select_by_index(&mut self, index: usize) {
        self.0 = index == 1;
    }

    /// 选择下一个
    pub fn next(&mut self) {
        self.toggle();
    }

    /// 选择上一个
    pub fn prev(&mut self) {
        self.toggle();
    }
}

/// 配置向导步骤
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConfigStep {
    /// 配置选择
    #[default]
    ConfigSelect,
    /// 配置名称
    ConfigName,
    /// 配置表单
    ConfigForm,
    /// 配置摘要
    Summary,
    /// 确认运行
    ConfirmRun,
}

impl ConfigStep {
    /// 获取标题
    pub fn title(&self) -> String {
        match self {
            ConfigStep::ConfigSelect => rust_i18n::t!("available_configurations").to_string(),
            ConfigStep::ConfigName => rust_i18n::t!("enter_config_name").to_string(),
            ConfigStep::ConfigForm => rust_i18n::t!("configuration_form").to_string(),
            ConfigStep::Summary => rust_i18n::t!("configuration_summary").to_string(),
            ConfigStep::ConfirmRun => rust_i18n::t!("proceed_instent").to_string(),
        }
    }

    /// 可选项数量
    pub fn option_count(&self) -> usize {
        match self {
            ConfigStep::ConfigSelect => 1,
            ConfigStep::ConfirmRun => 2,
            _ => 0,
        }
    }

    /// 获取选项列表
    pub fn options(&self) -> Vec<String> {
        match self {
            ConfigStep::ConfirmRun => vec![
                rust_i18n::t!("option_yes").to_string(),
                rust_i18n::t!("option_no").to_string(),
            ],
            _ => vec![],
        }
    }

    /// 获取下一步
    pub fn next(&self, _classification: ClassificationRule) -> Self {
        match self {
            ConfigStep::ConfigSelect => ConfigStep::ConfigForm,
            ConfigStep::ConfigName => ConfigStep::ConfigForm,
            ConfigStep::ConfigForm => ConfigStep::Summary,
            ConfigStep::Summary => ConfigStep::ConfirmRun,
            ConfigStep::ConfirmRun => ConfigStep::ConfirmRun,
        }
    }
}

/// 表单字段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    /// 配置名称
    ConfigName,
    /// 输入目录
    InputDirs,
    /// 输出目录
    OutputDir,
    /// 排除目录
    ExcludeDirs,
    /// 处理模式
    ProcessingMode,
    /// 分类规则
    Classification,
    /// 月份格式
    MonthFormat,
    /// 按类型分类
    ClassifyByType,
    /// 文件操作
    FileOperation,
    /// 去重
    Deduplication,
    /// 试运行
    DryRun,
}

impl FormField {
    /// 字段数量
    pub fn count() -> usize {
        11
    }

    /// 获取全部字段
    pub fn all() -> &'static [FormField] {
        &[
            FormField::ConfigName,
            FormField::InputDirs,
            FormField::OutputDir,
            FormField::ExcludeDirs,
            FormField::ProcessingMode,
            FormField::Classification,
            FormField::MonthFormat,
            FormField::ClassifyByType,
            FormField::FileOperation,
            FormField::Deduplication,
            FormField::DryRun,
        ]
    }

    /// 是否输入字段
    pub fn is_input_field(&self) -> bool {
        matches!(
            self,
            FormField::ConfigName
                | FormField::InputDirs
                | FormField::OutputDir
                | FormField::ExcludeDirs
        )
    }

    /// 是否选项字段
    pub fn is_option_field(&self) -> bool {
        !self.is_input_field()
    }

    /// 字段标签
    pub fn label(&self) -> String {
        match self {
            FormField::ConfigName => rust_i18n::t!("field_config_name").to_string(),
            FormField::InputDirs => rust_i18n::t!("field_input_dirs").to_string(),
            FormField::OutputDir => rust_i18n::t!("field_output_dir").to_string(),
            FormField::ExcludeDirs => rust_i18n::t!("field_exclude_dirs").to_string(),
            FormField::ProcessingMode => rust_i18n::t!("field_processing_mode").to_string(),
            FormField::Classification => rust_i18n::t!("field_classification").to_string(),
            FormField::MonthFormat => rust_i18n::t!("field_month_format").to_string(),
            FormField::ClassifyByType => rust_i18n::t!("field_classify_by_type").to_string(),
            FormField::FileOperation => rust_i18n::t!("field_file_operation").to_string(),
            FormField::Deduplication => rust_i18n::t!("field_deduplication").to_string(),
            FormField::DryRun => rust_i18n::t!("field_dry_run").to_string(),
        }
    }

    /// 获取显示值
    pub fn get_value_string(&self, state: &ConfigWizardState) -> String {
        match self {
            FormField::ConfigName => state.config_name.clone(),
            FormField::InputDirs => state.input_dirs.clone(),
            FormField::OutputDir => state.output_dir.clone(),
            FormField::ExcludeDirs => state.exclude_dirs.clone(),
            FormField::ProcessingMode => {
                processing_mode_label(state.processing_mode.selected()).to_string()
            }
            FormField::Classification => {
                classification_label(state.classification.selected()).to_string()
            }
            FormField::MonthFormat => month_format_label(state.month_format.selected()).to_string(),
            FormField::ClassifyByType => bool_label(state.classify_by_type.value()).to_string(),
            FormField::FileOperation => {
                file_operation_label(state.operation.selected()).to_string()
            }
            FormField::Deduplication => bool_label(state.deduplicate.value()).to_string(),
            FormField::DryRun => bool_label(state.dry_run.value()).to_string(),
        }
    }

    /// 是否可见
    pub fn is_visible(&self, state: &ConfigWizardState) -> bool {
        match self {
            FormField::ConfigName => !state.skip_confirm_run,
            FormField::MonthFormat => {
                state.classification.selected() == ClassificationRule::YearMonth
            }
            _ => true,
        }
    }
}

/// 表单状态
#[derive(Debug, Default)]
pub struct ConfigFormState {
    /// 当前选中字段索引
    pub selected_field: usize,
    /// 是否处于输入模式
    pub in_input_mode: bool,
    /// 输入状态
    pub input: InputState,
    /// 表单滚动偏移
    pub scroll_offset: usize,
}

impl ConfigFormState {
    /// 创建表单状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取当前字段
    pub fn selected(&self) -> FormField {
        FormField::all()
            .get(self.selected_field)
            .copied()
            .unwrap_or(FormField::ConfigName)
    }

    /// 选择下一个字段
    pub fn next_field(&mut self, visible_count: usize) {
        self.selected_field = (self.selected_field + 1) % visible_count;
        self.auto_scroll();
    }

    /// 选择上一个字段
    pub fn prev_field(&mut self, visible_count: usize) {
        self.selected_field = if self.selected_field == 0 {
            visible_count.saturating_sub(1)
        } else {
            self.selected_field - 1
        };
        self.auto_scroll();
    }

    fn auto_scroll(&mut self) {
        use crate::tui::theme::config::FORM_VISIBLE_ROWS;
        if self.selected_field >= self.scroll_offset + FORM_VISIBLE_ROWS {
            self.scroll_offset = self.selected_field.saturating_sub(FORM_VISIBLE_ROWS - 1);
        } else if self.selected_field < self.scroll_offset {
            self.scroll_offset = self.selected_field;
        }
    }

    /// 进入输入模式
    pub fn enter_input_mode(&mut self, value: &str) {
        self.in_input_mode = true;
        self.input = InputState::with_value(value);
    }

    /// 退出输入模式
    pub fn exit_input_mode(&mut self) {
        self.in_input_mode = false;
        self.input.clear();
    }

    /// 获取输入值
    pub fn input_value(&self) -> &str {
        self.input.value()
    }

    /// 获取光标位置
    pub fn input_cursor(&self) -> usize {
        self.input.cursor_position()
    }

    /// 清空输入
    pub fn clear_input(&mut self) {
        self.input.clear();
    }

    /// 可见字段数量
    pub fn visible_fields_count(&self, state: &ConfigWizardState) -> usize {
        FormField::all()
            .iter()
            .filter(|f| f.is_visible(state))
            .count()
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
    pub processing_mode: EnumSelection<ProcessingMode>,
    /// 分类规则
    pub classification: EnumSelection<ClassificationRule>,
    /// 月份格式
    pub month_format: EnumSelection<MonthFormat>,
    /// 文件操作
    pub operation: EnumSelection<FileOperation>,
    /// 去重
    pub deduplicate: BoolSelection,
    /// 试运行
    pub dry_run: BoolSelection,
    /// 按类型分类
    pub classify_by_type: BoolSelection,
    /// 配置名称
    pub config_name: String,
    /// 可用配置列表
    pub available_configs: Vec<PathBuf>,
    /// 选中配置索引
    pub selected_config: Option<usize>,
    /// 校验错误
    pub error_message: Option<String>,
    /// 是否已保存配置
    pub config_saved: bool,
    /// 保存路径
    pub config_path: Option<PathBuf>,
    /// 是否跳过确认
    pub skip_confirm_run: bool,
    /// 是否来自配置选择
    pub from_config_select: bool,
    /// 是否需要提示修改确认
    pub need_modify_confirm: bool,
    /// 表单状态
    pub form_state: ConfigFormState,
}

impl ConfigWizardState {
    /// 创建向导状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 是否处于创建配置流程
    pub fn is_create_config_flow(&self) -> bool {
        !self.skip_confirm_run && !self.from_config_select
    }

    /// 是否处于配置选择流程
    pub fn is_select_config_flow(&self) -> bool {
        self.from_config_select
    }

    /// 配置选择是否允许确认
    pub fn can_confirm_config_select(&self) -> bool {
        if self.step == ConfigStep::ConfigSelect {
            !self.available_configs.is_empty()
        } else {
            true
        }
    }

    /// 从配置初始化表单
    pub fn init_from_config(&mut self, config: &Config, config_path: &Path) {
        self.input_dirs = config
            .input_dirs
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("; ");
        self.output_dir = config.output_dir.display().to_string();
        self.exclude_dirs = config
            .exclude_dirs
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("; ");
        self.processing_mode.select(config.processing_mode);
        self.classification.select(config.classification);
        self.month_format.select(config.month_format);
        self.operation.select(config.operation);
        self.deduplicate
            .select_by_index(if config.deduplicate { 1 } else { 0 });
        self.dry_run
            .select_by_index(if config.dry_run { 1 } else { 0 });
        self.classify_by_type
            .select_by_index(if config.classify_by_type { 1 } else { 0 });
        self.config_name = config_path
            .file_stem()
            .map(|os| os.to_string_lossy().to_string())
            .unwrap_or_default();
    }

    /// 构建配置
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

    /// 校验表单
    pub fn validate_form(&self) -> Result<(), String> {
        let mut errors = Vec::new();

        if !self.skip_confirm_run {
            if self.config_name.trim().is_empty() {
                errors.push(rust_i18n::t!("config_name_empty_error").to_string());
            } else if self.config_name.contains('/')
                || self.config_name.contains('\\')
                || self.config_name.contains('.')
            {
                errors.push(rust_i18n::t!("config_name_invalid_chars_error").to_string());
            }
        }

        if self.input_dirs.trim().is_empty() {
            errors.push(rust_i18n::t!("no_input_dirs_specified").to_string());
        }

        if self.output_dir.trim().is_empty() {
            errors.push(rust_i18n::t!("output_dir_empty_error").to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }

    /// 保存配置文件
    pub fn save_config(&mut self) -> Result<PathBuf, String> {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        let config_dir = exe_dir.join("Config");

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

    /// 刷新配置列表
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

        if self.available_configs.is_empty() {
            self.selected_config = None;
        } else {
            let max_index = self.available_configs.len().saturating_sub(1);
            let index = self.selected_config.unwrap_or(0).min(max_index);
            self.selected_config = Some(index);
        }
    }

    /// 获取当前选中索引
    pub fn selected_value(&self) -> usize {
        match self.step {
            ConfigStep::ConfirmRun | ConfigStep::ConfigSelect => self.selected_config.unwrap_or(0),
            _ => 0,
        }
    }

    /// 获取当前步骤选项数量
    pub fn option_count(&self) -> usize {
        match self.step {
            ConfigStep::ConfigSelect => self.available_configs.len().max(1),
            ConfigStep::ConfirmRun => 2,
            _ => 0,
        }
    }

    /// 设置选中索引
    pub fn set_selected(&mut self, index: usize) {
        match self.step {
            ConfigStep::ConfirmRun | ConfigStep::ConfigSelect => {
                self.selected_config = Some(index);
            }
            _ => {}
        }
    }

    /// 初始化/校准选择索引
    pub fn ensure_selection(&mut self) {
        match self.step {
            ConfigStep::ConfirmRun => {
                let index = match self.selected_config {
                    Some(index) if index <= 1 => index,
                    _ => 0,
                };
                self.selected_config = Some(index);
            }
            ConfigStep::ConfigSelect => {
                if self.available_configs.is_empty() {
                    self.selected_config = None;
                } else {
                    let max_index = self.available_configs.len() - 1;
                    let index = self.selected_config.unwrap_or(0).min(max_index);
                    self.selected_config = Some(index);
                }
            }
            _ => {}
        }
    }

    /// 选择下一个选项
    pub fn navigate_next(&mut self) {
        match self.step {
            ConfigStep::ConfirmRun | ConfigStep::ConfigSelect => {
                self.ensure_selection();
                if let Some(idx) = self.selected_config {
                    let count = self.option_count();
                    self.selected_config = Some((idx + 1) % count);
                }
            }
            _ => {}
        }
    }

    /// 选择上一个选项
    pub fn navigate_prev(&mut self) {
        match self.step {
            ConfigStep::ConfirmRun | ConfigStep::ConfigSelect => {
                self.ensure_selection();
                if let Some(idx) = self.selected_config {
                    let count = self.option_count();
                    self.selected_config = Some(if idx == 0 { count - 1 } else { idx - 1 });
                }
            }
            _ => {}
        }
    }

    /// 表单选择下一个字段
    pub fn navigate_form_next(&mut self) {
        let visible_count = self.form_state.visible_fields_count(self) + 1;
        if visible_count > 0 {
            self.form_state.next_field(visible_count);
        }
    }

    /// 表单选择上一个字段
    pub fn navigate_form_prev(&mut self) {
        let visible_count = self.form_state.visible_fields_count(self) + 1;
        if visible_count > 0 {
            self.form_state.prev_field(visible_count);
        }
    }

    /// 是否选中“下一步”
    pub fn is_next_selected(&self) -> bool {
        let visible_count = self.form_state.visible_fields_count(self);
        self.form_state.selected_field >= visible_count
    }

    /// 切换当前字段到下一个选项
    pub fn toggle_current_field_next(&mut self) {
        let visible_fields = self.get_visible_fields();
        let field_opt = visible_fields.get(self.form_state.selected_field).copied();

        match field_opt {
            Some(FormField::ProcessingMode) => self.processing_mode.next(),
            Some(FormField::Classification) => self.classification.next(),
            Some(FormField::MonthFormat) => self.month_format.next(),
            Some(FormField::FileOperation) => self.operation.next(),
            Some(FormField::Deduplication) => self.deduplicate.next(),
            Some(FormField::DryRun) => self.dry_run.next(),
            Some(FormField::ClassifyByType) => self.classify_by_type.next(),
            _ => {}
        }
    }

    /// 切换当前字段到上一个选项
    pub fn toggle_current_field_prev(&mut self) {
        let visible_fields = self.get_visible_fields();
        let field_opt = visible_fields.get(self.form_state.selected_field).copied();

        match field_opt {
            Some(FormField::ProcessingMode) => self.processing_mode.prev(),
            Some(FormField::Classification) => self.classification.prev(),
            Some(FormField::MonthFormat) => self.month_format.prev(),
            Some(FormField::FileOperation) => self.operation.prev(),
            Some(FormField::Deduplication) => self.deduplicate.prev(),
            Some(FormField::DryRun) => self.dry_run.prev(),
            Some(FormField::ClassifyByType) => self.classify_by_type.prev(),
            _ => {}
        }
    }

    /// 更新输入字段值
    pub fn update_field_from_input(&mut self, value: String) {
        let visible_fields = self.get_visible_fields();
        let field_opt = visible_fields.get(self.form_state.selected_field).copied();

        match field_opt {
            Some(FormField::ConfigName) => self.config_name = value,
            Some(FormField::InputDirs) => self.input_dirs = value,
            Some(FormField::OutputDir) => self.output_dir = value,
            Some(FormField::ExcludeDirs) => self.exclude_dirs = value,
            _ => {}
        }
    }

    /// 进入输入模式
    pub fn enter_input_mode_for_field(&mut self) {
        let visible_fields = self.get_visible_fields();
        let field_opt = visible_fields.get(self.form_state.selected_field).copied();

        let value = match field_opt {
            Some(FormField::ConfigName) => self.config_name.clone(),
            Some(FormField::InputDirs) => self.input_dirs.clone(),
            Some(FormField::OutputDir) => self.output_dir.clone(),
            Some(FormField::ExcludeDirs) => self.exclude_dirs.clone(),
            _ => return,
        };
        self.form_state.enter_input_mode(&value);
    }

    /// 获取输入内容
    pub fn input_buffer(&self) -> &str {
        self.form_state.input_value()
    }

    /// 获取光标位置
    pub fn input_cursor(&self) -> usize {
        self.form_state.input_cursor()
    }

    /// 设置输入缓冲区
    pub fn set_input_buffer(&mut self, buffer: String, cursor: usize) {
        self.form_state.input.set_buffer(buffer, cursor);
    }

    /// 退出输入模式并应用
    pub fn exit_input_mode_apply(&mut self) {
        let value = self.form_state.input_value().to_string();
        self.update_field_from_input(value);
        self.form_state.exit_input_mode();
    }

    /// 退出输入模式（不保存）
    pub fn exit_input_mode_cancel(&mut self) {
        self.form_state.exit_input_mode();
    }

    /// 获取可见字段
    pub fn get_visible_fields(&self) -> Vec<FormField> {
        FormField::all()
            .iter()
            .filter(|f| f.is_visible(self))
            .copied()
            .collect()
    }

    /// 获取当前可见字段
    pub fn selected_form_field(&self) -> Option<FormField> {
        let visible_fields = self.get_visible_fields();
        visible_fields.get(self.form_state.selected_field).copied()
    }

    /// 是否输入模式
    pub fn is_in_input_mode(&self) -> bool {
        self.form_state.in_input_mode
    }

    /// 输入插入字符
    pub fn input_insert_char(&mut self, c: char) {
        self.form_state.input.insert_char(c);
    }

    /// 输入退格
    pub fn input_backspace(&mut self) {
        self.form_state.input.delete_before_cursor();
    }

    /// 输入删除
    pub fn input_delete(&mut self) {
        self.form_state.input.delete_after_cursor();
    }

    /// 输入光标左移
    pub fn input_move_left(&mut self) {
        self.form_state.input.move_cursor_left();
    }

    /// 输入光标右移
    pub fn input_move_right(&mut self) {
        self.form_state.input.move_cursor_right();
    }

    /// 输入光标到行首
    pub fn input_move_to_start(&mut self) {
        self.form_state.input.move_cursor_to_start();
    }

    /// 输入光标到行尾
    pub fn input_move_to_end(&mut self) {
        self.form_state.input.move_cursor_to_end();
    }
}
