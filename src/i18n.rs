//! Internationalization (i18n) module
//!
//! Provides language detection and localized strings for the CLI interface.
//! Supports English and Chinese Simplified.
//! Note: Log messages remain in English for consistency.

use std::sync::OnceLock;

/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    ChineseSimplified,
}

/// Global language instance
static LANGUAGE: OnceLock<Language> = OnceLock::new();

/// Initialize and get the current language based on system locale
pub fn get_language() -> Language {
    *LANGUAGE.get_or_init(detect_language)
}

/// Detect system language from environment variables
fn detect_language() -> Language {
    // Check common locale environment variables
    let locale = std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .unwrap_or_default()
        .to_lowercase();

    // Check for Chinese locale indicators
    if locale.starts_with("zh")
        || locale.contains("chinese")
        || locale.contains("cn")
        || locale.contains("hans")
    {
        return Language::ChineseSimplified;
    }

    // Windows-specific detection
    #[cfg(windows)]
    {
        if let Some(lang) = detect_windows_language() {
            return lang;
        }
    }

    Language::English
}

/// Windows-specific language detection using GetUserDefaultUILanguage
#[cfg(windows)]
fn detect_windows_language() -> Option<Language> {
    use std::process::Command;

    // Try PowerShell to get UI culture
    let output = Command::new("powershell")
        .args(["-Command", "(Get-Culture).Name"])
        .output()
        .ok()?;

    let culture = String::from_utf8_lossy(&output.stdout).to_lowercase();

    if culture.starts_with("zh") {
        return Some(Language::ChineseSimplified);
    }

    None
}

/// Localized strings for the CLI interface
pub struct Strings;

impl Strings {
    pub fn welcome_message() -> &'static str {
        match get_language() {
            Language::English => "Welcome to Gallery Sorter Interactive Mode!",
            Language::ChineseSimplified => "欢迎使用 Gallery Sorter 交互模式！",
        }
    }

    pub fn select_option_prompt() -> &'static str {
        match get_language() {
            Language::English => "Please select an option to continue.",
            Language::ChineseSimplified => "请选择一个选项以继续。",
        }
    }

    pub fn menu_option_run_direct() -> &'static str {
        match get_language() {
            Language::English => "[1] Run with directly input parameters",
            Language::ChineseSimplified => "[1] 直接输入参数运行",
        }
    }

    pub fn menu_option_run_config() -> &'static str {
        match get_language() {
            Language::English => "[2] Run with selected configuration",
            Language::ChineseSimplified => "[2] 使用已有配置运行",
        }
    }

    pub fn menu_option_create_config() -> &'static str {
        match get_language() {
            Language::English => "[3] Create configuration",
            Language::ChineseSimplified => "[3] 创建配置文件",
        }
    }

    pub fn menu_option_exit() -> &'static str {
        match get_language() {
            Language::English => "[4] Exit program",
            Language::ChineseSimplified => "[4] 退出程序",
        }
    }

    pub fn select_option() -> &'static str {
        match get_language() {
            Language::English => "Select an option",
            Language::ChineseSimplified => "选择选项",
        }
    }

    pub fn goodbye() -> &'static str {
        match get_language() {
            Language::English => "Goodbye!",
            Language::ChineseSimplified => "再见！",
        }
    }

    pub fn no_configs_found() -> &'static str {
        match get_language() {
            Language::English => "No configuration files found in Config directory.",
            Language::ChineseSimplified => "Config 目录中未找到配置文件。",
        }
    }

    pub fn use_option_3_hint() -> &'static str {
        match get_language() {
            Language::English => "Use option 3 to create a configuration first.",
            Language::ChineseSimplified => "请先使用选项 3 创建配置文件。",
        }
    }

    pub fn available_configurations() -> &'static str {
        match get_language() {
            Language::English => "Available Configurations:",
            Language::ChineseSimplified => "可用配置：",
        }
    }

    pub fn select_configuration() -> &'static str {
        match get_language() {
            Language::English => "Select a configuration",
            Language::ChineseSimplified => "选择配置",
        }
    }

    pub fn back_to_main_menu() -> &'static str {
        match get_language() {
            Language::English => "← Back to main menu",
            Language::ChineseSimplified => "← 返回主菜单",
        }
    }

    pub fn loading_configuration() -> &'static str {
        match get_language() {
            Language::English => "Loading configuration:",
            Language::ChineseSimplified => "正在加载配置：",
        }
    }

    pub fn proceed_with_config() -> &'static str {
        match get_language() {
            Language::English => "Proceed with this configuration?",
            Language::ChineseSimplified => "使用此配置继续？",
        }
    }

    pub fn create_new_configuration() -> &'static str {
        match get_language() {
            Language::English => "Create New Configuration",
            Language::ChineseSimplified => "创建新配置",
        }
    }

    pub fn enter_config_name() -> &'static str {
        match get_language() {
            Language::English => "Enter configuration name (without .toml extension)",
            Language::ChineseSimplified => "输入配置名称（不含 .toml 扩展名）",
        }
    }

    pub fn config_name_empty_error() -> &'static str {
        match get_language() {
            Language::English => "Configuration name cannot be empty",
            Language::ChineseSimplified => "配置名称不能为空",
        }
    }

    pub fn config_name_invalid_chars_error() -> &'static str {
        match get_language() {
            Language::English => "Configuration name cannot contain /, \\, or .",
            Language::ChineseSimplified => "配置名称不能包含 /、\\ 或 .",
        }
    }

    pub fn config_exists_overwrite() -> &'static str {
        match get_language() {
            Language::English => "already exists. Overwrite?",
            Language::ChineseSimplified => "已存在。是否覆盖？",
        }
    }

    pub fn configuration_cancelled() -> &'static str {
        match get_language() {
            Language::English => "Configuration creation cancelled.",
            Language::ChineseSimplified => "已取消创建配置。",
        }
    }

    pub fn save_configuration() -> &'static str {
        match get_language() {
            Language::English => "Save this configuration?",
            Language::ChineseSimplified => "保存此配置？",
        }
    }

    pub fn config_not_saved() -> &'static str {
        match get_language() {
            Language::English => "Configuration not saved.",
            Language::ChineseSimplified => "配置未保存。",
        }
    }

    pub fn config_saved_to() -> &'static str {
        match get_language() {
            Language::English => "Configuration saved to:",
            Language::ChineseSimplified => "配置已保存至：",
        }
    }

    pub fn use_option_2_hint() -> &'static str {
        match get_language() {
            Language::English => "You can now use option 2 to run with this configuration.",
            Language::ChineseSimplified => "您现在可以使用选项 2 来运行此配置。",
        }
    }

    pub fn configuration_parameters() -> &'static str {
        match get_language() {
            Language::English => "Configuration Parameters",
            Language::ChineseSimplified => "配置参数",
        }
    }

    pub fn enter_input_directory() -> &'static str {
        match get_language() {
            Language::English => "Enter input directory (or multiple separated by ;)",
            Language::ChineseSimplified => "输入源目录（多个目录用 ; 分隔）",
        }
    }

    pub fn no_input_dirs_specified() -> &'static str {
        match get_language() {
            Language::English => "No input directories specified.",
            Language::ChineseSimplified => "未指定输入目录。",
        }
    }

    pub fn directory_not_exist() -> &'static str {
        match get_language() {
            Language::English => "Directory does not exist:",
            Language::ChineseSimplified => "目录不存在：",
        }
    }

    pub fn enter_output_directory() -> &'static str {
        match get_language() {
            Language::English => "Enter output directory",
            Language::ChineseSimplified => "输入目标目录",
        }
    }

    pub fn enter_exclude_directories() -> &'static str {
        match get_language() {
            Language::English => "Enter directories to exclude (folder names or paths, separated by ;, leave empty to skip)",
            Language::ChineseSimplified => "输入要排除的目录（文件夹名或路径，用 ; 分隔，留空跳过）",
        }
    }

    pub fn summary_exclude_dirs() -> &'static str {
        match get_language() {
            Language::English => "Exclude:",
            Language::ChineseSimplified => "排除：",
        }
    }

    pub fn select_processing_mode() -> &'static str {
        match get_language() {
            Language::English => "Select processing mode",
            Language::ChineseSimplified => "选择处理模式",
        }
    }

    pub fn mode_full() -> &'static str {
        match get_language() {
            Language::English => "Full - Process all files, overwrite existing",
            Language::ChineseSimplified => "完整模式 - 处理所有文件，覆盖已存在的文件",
        }
    }

    pub fn mode_supplement() -> &'static str {
        match get_language() {
            Language::English => "Supplement - Skip files that already exist in target",
            Language::ChineseSimplified => "补充模式 - 跳过目标目录中已存在的文件",
        }
    }

    pub fn mode_incremental() -> &'static str {
        match get_language() {
            Language::English => "Incremental - Only process new/modified files",
            Language::ChineseSimplified => "增量模式 - 仅处理新增/修改的文件",
        }
    }

    pub fn select_classification_rule() -> &'static str {
        match get_language() {
            Language::English => "Select classification rule",
            Language::ChineseSimplified => "选择分类规则",
        }
    }

    pub fn classify_year_month() -> &'static str {
        match get_language() {
            Language::English => "Year-Month - Organize by YYYY/MM/",
            Language::ChineseSimplified => "年-月 - 按 YYYY/MM/ 整理",
        }
    }

    pub fn classify_year() -> &'static str {
        match get_language() {
            Language::English => "Year - Organize by YYYY/",
            Language::ChineseSimplified => "年份 - 按 YYYY/ 整理",
        }
    }

    pub fn classify_none() -> &'static str {
        match get_language() {
            Language::English => "None - All files in root",
            Language::ChineseSimplified => "无分类 - 所有文件放在根目录",
        }
    }

    pub fn select_month_format() -> &'static str {
        match get_language() {
            Language::English => "Select month format",
            Language::ChineseSimplified => "选择月份格式",
        }
    }

    pub fn month_format_nested() -> &'static str {
        match get_language() {
            Language::English => "Nested - YYYY/MM/",
            Language::ChineseSimplified => "嵌套格式 - YYYY/MM/",
        }
    }

    pub fn month_format_combined() -> &'static str {
        match get_language() {
            Language::English => "Combined - YYYY-MM/",
            Language::ChineseSimplified => "组合格式 - YYYY-MM/",
        }
    }

    pub fn select_file_operation() -> &'static str {
        match get_language() {
            Language::English => "Select file operation",
            Language::ChineseSimplified => "选择文件操作",
        }
    }

    pub fn operation_copy() -> &'static str {
        match get_language() {
            Language::English => "Copy - Copy files to destination",
            Language::ChineseSimplified => "复制 - 复制文件到目标目录",
        }
    }

    pub fn operation_move() -> &'static str {
        match get_language() {
            Language::English => "Move - Move files to destination",
            Language::ChineseSimplified => "移动 - 移动文件到目标目录",
        }
    }

    pub fn operation_hardlink() -> &'static str {
        match get_language() {
            Language::English => "Hardlink - Create hard links",
            Language::ChineseSimplified => "硬链接 - 创建硬链接",
        }
    }

    pub fn enable_deduplication() -> &'static str {
        match get_language() {
            Language::English => "Enable file deduplication?",
            Language::ChineseSimplified => "启用文件去重？",
        }
    }

    pub fn dry_run_mode() -> &'static str {
        match get_language() {
            Language::English => "Dry run mode (preview without making changes)?",
            Language::ChineseSimplified => "试运行模式（预览而不实际修改）？",
        }
    }

    pub fn classify_by_file_type() -> &'static str {
        match get_language() {
            Language::English => "Classify by file type (Photos/Videos/Raw)?",
            Language::ChineseSimplified => "按文件类型分类（Photos/Videos/Raw）？",
        }
    }

    pub fn proceed_with_settings() -> &'static str {
        match get_language() {
            Language::English => "Proceed with these settings?",
            Language::ChineseSimplified => "使用这些设置继续？",
        }
    }

    pub fn operation_cancelled() -> &'static str {
        match get_language() {
            Language::English => "Operation cancelled.",
            Language::ChineseSimplified => "操作已取消。",
        }
    }

    pub fn configuration_summary() -> &'static str {
        match get_language() {
            Language::English => "Configuration Summary:",
            Language::ChineseSimplified => "配置摘要：",
        }
    }

    pub fn summary_input() -> &'static str {
        match get_language() {
            Language::English => "Input:",
            Language::ChineseSimplified => "输入：",
        }
    }

    pub fn summary_output() -> &'static str {
        match get_language() {
            Language::English => "Output:",
            Language::ChineseSimplified => "输出：",
        }
    }

    pub fn summary_mode() -> &'static str {
        match get_language() {
            Language::English => "Mode:",
            Language::ChineseSimplified => "模式：",
        }
    }

    pub fn summary_classify() -> &'static str {
        match get_language() {
            Language::English => "Classify:",
            Language::ChineseSimplified => "分类：",
        }
    }

    pub fn summary_month_format() -> &'static str {
        match get_language() {
            Language::English => "Month Format:",
            Language::ChineseSimplified => "月份格式：",
        }
    }

    pub fn summary_operation() -> &'static str {
        match get_language() {
            Language::English => "Operation:",
            Language::ChineseSimplified => "操作：",
        }
    }

    pub fn summary_deduplicate() -> &'static str {
        match get_language() {
            Language::English => "Deduplicate:",
            Language::ChineseSimplified => "去重：",
        }
    }

    pub fn summary_dry_run() -> &'static str {
        match get_language() {
            Language::English => "Dry Run:",
            Language::ChineseSimplified => "试运行：",
        }
    }

    pub fn summary_classify_by_type() -> &'static str {
        match get_language() {
            Language::English => "File Type:",
            Language::ChineseSimplified => "类型分类：",
        }
    }

    pub fn starting_processing() -> &'static str {
        match get_language() {
            Language::English => "Starting processing...",
            Language::ChineseSimplified => "开始处理...",
        }
    }

    pub fn scanning_directories() -> &'static str {
        match get_language() {
            Language::English => "Scanning input directories...",
            Language::ChineseSimplified => "正在扫描输入目录...",
        }
    }

    pub fn log_saved_to() -> &'static str {
        match get_language() {
            Language::English => "Log saved to:",
            Language::ChineseSimplified => "日志已保存至：",
        }
    }

    pub fn processing_complete() -> &'static str {
        match get_language() {
            Language::English => "PROCESSING COMPLETE",
            Language::ChineseSimplified => "处理完成",
        }
    }

    pub fn statistics() -> &'static str {
        match get_language() {
            Language::English => "Statistics:",
            Language::ChineseSimplified => "统计：",
        }
    }

    pub fn stat_total_files() -> &'static str {
        match get_language() {
            Language::English => "Total files:",
            Language::ChineseSimplified => "文件总数：",
        }
    }

    pub fn stat_processed() -> &'static str {
        match get_language() {
            Language::English => "Processed:",
            Language::ChineseSimplified => "已处理：",
        }
    }

    pub fn stat_skipped() -> &'static str {
        match get_language() {
            Language::English => "Skipped:",
            Language::ChineseSimplified => "已跳过：",
        }
    }

    pub fn stat_duplicates() -> &'static str {
        match get_language() {
            Language::English => "Duplicates:",
            Language::ChineseSimplified => "重复文件：",
        }
    }

    pub fn stat_failed() -> &'static str {
        match get_language() {
            Language::English => "Failed:",
            Language::ChineseSimplified => "失败：",
        }
    }

    pub fn failed_files() -> &'static str {
        match get_language() {
            Language::English => "Failed Files:",
            Language::ChineseSimplified => "失败文件：",
        }
    }

    pub fn and_n_more(n: usize) -> String {
        match get_language() {
            Language::English => format!("... and {} more...", n),
            Language::ChineseSimplified => format!("... 还有 {} 个...", n),
        }
    }

    pub fn dry_run_notice() -> &'static str {
        match get_language() {
            Language::English => "DRY RUN - No files were actually modified",
            Language::ChineseSimplified => "试运行 - 没有实际修改任何文件",
        }
    }

    pub fn cli_processing_complete() -> &'static str {
        match get_language() {
            Language::English => "Processing Complete!",
            Language::ChineseSimplified => "处理完成！",
        }
    }

    pub fn cli_detailed_results() -> &'static str {
        match get_language() {
            Language::English => "Detailed Results:",
            Language::ChineseSimplified => "详细结果：",
        }
    }

    pub fn cli_failed_files() -> &'static str {
        match get_language() {
            Language::English => "Failed files:",
            Language::ChineseSimplified => "失败的文件：",
        }
    }

    pub fn cli_dry_run_notice() -> &'static str {
        match get_language() {
            Language::English => "[DRY RUN] No files were actually modified.",
            Language::ChineseSimplified => "[试运行] 没有实际修改任何文件。",
        }
    }

    pub fn cli_no_input_dirs_error() -> &'static str {
        match get_language() {
            Language::English => "No input directories specified. Use -i/--input or specify in config file.",
            Language::ChineseSimplified => "未指定输入目录。请使用 -i/--input 参数或在配置文件中指定。",
        }
    }

    pub fn cli_input_dir_not_exist() -> &'static str {
        match get_language() {
            Language::English => "Warning: Input directory does not exist:",
            Language::ChineseSimplified => "警告：输入目录不存在：",
        }
    }

    pub fn cli_output_inside_input_error() -> &'static str {
        match get_language() {
            Language::English => "Output directory cannot be inside input directory:",
            Language::ChineseSimplified => "输出目录不能在输入目录内：",
        }
    }

    pub fn cli_is_inside() -> &'static str {
        match get_language() {
            Language::English => "is inside",
            Language::ChineseSimplified => "在",
        }
    }

    pub fn status_ok() -> &'static str {
        match get_language() {
            Language::English => "[OK]",
            Language::ChineseSimplified => "[成功]",
        }
    }

    pub fn status_skip() -> &'static str {
        match get_language() {
            Language::English => "[SKIP]",
            Language::ChineseSimplified => "[跳过]",
        }
    }

    pub fn status_dup() -> &'static str {
        match get_language() {
            Language::English => "[DUP]",
            Language::ChineseSimplified => "[重复]",
        }
    }

    pub fn status_fail() -> &'static str {
        match get_language() {
            Language::English => "[FAIL]",
            Language::ChineseSimplified => "[失败]",
        }
    }

    pub fn status_dry() -> &'static str {
        match get_language() {
            Language::English => "[DRY]",
            Language::ChineseSimplified => "[试运行]",
        }
    }

    pub fn already_processed() -> &'static str {
        match get_language() {
            Language::English => "(already processed)",
            Language::ChineseSimplified => "（已处理）",
        }
    }

    pub fn duplicate_of() -> &'static str {
        match get_language() {
            Language::English => "(duplicate of",
            Language::ChineseSimplified => "（与此重复：",
        }
    }

    pub fn unknown_error() -> &'static str {
        match get_language() {
            Language::English => "Unknown error",
            Language::ChineseSimplified => "未知错误",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        // This test just ensures the function doesn't panic
        let _lang = detect_language();
    }

    #[test]
    fn test_strings_exist() {
        // Ensure all string functions return non-empty strings
        assert!(!Strings::welcome_message().is_empty());
        assert!(!Strings::goodbye().is_empty());
        assert!(!Strings::processing_complete().is_empty());
    }
}
