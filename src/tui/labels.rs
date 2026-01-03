//! 本地化标签映射
//!
//! 用于将枚举/布尔值映射为稳定的翻译键，避免依赖 Debug 输出。

use crate::config::{ClassificationRule, FileOperation, MonthFormat, ProcessingMode};
use rust_i18n::t;
use std::borrow::Cow;

/// 处理模式标签
pub fn processing_mode_label(mode: ProcessingMode) -> Cow<'static, str> {
    match mode {
        ProcessingMode::Full => t!("mode_full"),
        ProcessingMode::Supplement => t!("mode_supplement"),
        ProcessingMode::Incremental => t!("mode_incremental"),
    }
}

/// 分类规则标签
pub fn classification_label(rule: ClassificationRule) -> Cow<'static, str> {
    match rule {
        ClassificationRule::None => t!("classify_none"),
        ClassificationRule::Year => t!("classify_year"),
        ClassificationRule::YearMonth => t!("classify_year_month"),
    }
}

/// 月份格式标签
pub fn month_format_label(format: MonthFormat) -> Cow<'static, str> {
    match format {
        MonthFormat::Nested => t!("month_format_nested"),
        MonthFormat::Combined => t!("month_format_combined"),
    }
}

/// 文件操作标签
pub fn file_operation_label(operation: FileOperation) -> Cow<'static, str> {
    match operation {
        FileOperation::Copy => t!("operation_copy"),
        FileOperation::Move => t!("operation_move"),
        FileOperation::Symlink => t!("operation_symlink"),
        FileOperation::Hardlink => t!("operation_hardlink"),
    }
}

/// 布尔值标签
pub fn bool_label(value: bool) -> Cow<'static, str> {
    if value {
        t!("option_yes")
    } else {
        t!("option_no")
    }
}
