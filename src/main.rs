//! Gallery Sorter - Professional photo and video organization tool
//!
//! A CLI tool for organizing media files based on creation time with
//! intelligent time extraction from EXIF, video metadata, filenames,
//! and file system timestamps.

use anyhow::Result;
use chrono::Local;
use clap::Parser;
use gallery_sorter::{Cli, Config, Processor, TuiApp, init_locale, should_run_interactive};
use std::path::{Path, PathBuf};
use tracing::{Level, error, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

// Initialize i18n for this binary
rust_i18n::i18n!("locales", fallback = "en");

// CLI Output Module
mod cli_output {
    //! CLI 输出美化模块
    //!
    //! 为命令行输出提供统一的颜色和格式样式。

    use crossterm::{
        ExecutableCommand,
        style::{Color, Print, Stylize, style},
    };
    use std::io::stdout;

    /// CLI 主题颜色
    pub struct CliTheme;

    impl CliTheme {
        /// 成功颜色（绿色）
        pub const SUCCESS: Color = Color::Green;
        /// 警告颜色（黄色）
        pub const WARNING: Color = Color::Yellow;
        /// 错误颜色（红色）
        pub const ERROR: Color = Color::Red;
        /// 提示颜色（暗灰色）
        pub const HINT: Color = Color::DarkGrey;
        /// 强调颜色（青色）
        pub const ACCENT: Color = Color::Cyan;
    }

    /// 打印分隔线
    pub fn print_separator() {
        let _ = stdout().execute(Print(&format!("{}\n", "─".repeat(60))));
    }

    /// 打印居中的标题
    pub fn print_title(title: &str) {
        let width = 60;
        let padding = (width - title.len()) / 2;
        let left_pad = " ".repeat(padding.saturating_sub(1));

        let _ = stdout().execute(Print(&format!(
            "{}{} {}{}\n",
            left_pad,
            "╔".bold().stylize(),
            title.bold().stylize(),
            "╗".bold().stylize(),
        )));
        let _ = stdout().execute(Print("\n"));
    }

    /// 打印警告消息
    pub fn print_warning(msg: &str) {
        let _ = stdout().execute(Print(style("⚠ ").with(CliTheme::WARNING).bold()));
        let _ = stdout().execute(Print(format!("{}\n", msg)));
    }

    /// 打印错误消息
    pub fn print_error(msg: &str) {
        let _ = stdout().execute(Print(style("✗ ").with(CliTheme::ERROR).bold()));
        let _ = stdout().execute(Print(format!("{}\n", msg)));
    }

    /// 打印提示消息
    pub fn print_hint(msg: &str) {
        let _ = stdout().execute(Print(style("→ ").with(CliTheme::HINT)));
        let _ = stdout().execute(Print(format!("{}\n", msg)));
    }

    /// 打印键值对
    pub fn print_key_value(key: &str, value: &str, value_color: Option<Color>) {
        let key_styled = style(key).with(CliTheme::HINT);
        let value_styled = match value_color {
            Some(color) => style(value).with(color),
            None => style(value).bold(),
        };
        let _ = stdout().execute(Print("  "));
        let _ = stdout().execute(Print(key_styled));
        let _ = stdout().execute(Print(": "));
        let _ = stdout().execute(Print(value_styled));
        let _ = stdout().execute(Print("\n"));
    }

    /// 打印统计项
    pub fn print_stat(key: &str, value: &str, color: Color) {
        let key_styled = style(key).with(CliTheme::HINT);
        let value_styled = style(value).with(color).bold();
        let _ = stdout().execute(Print("  "));
        let _ = stdout().execute(Print(key_styled));
        let _ = stdout().execute(Print(": "));
        let _ = stdout().execute(Print(value_styled));
        let _ = stdout().execute(Print("\n"));
    }

    /// 打印处理结果行
    pub fn print_result(status_icon: &str, status_color: Color, source: &str, dest_or_msg: &str) {
        let icon_styled = style(status_icon).with(status_color).bold();
        let source_styled = style(source).italic();
        let msg_styled = style(dest_or_msg).with(CliTheme::HINT);

        let _ = stdout().execute(Print("  "));
        let _ = stdout().execute(Print(icon_styled));
        let _ = stdout().execute(Print(" "));
        let _ = stdout().execute(Print(source_styled));
        let _ = stdout().execute(Print(" "));
        let _ = stdout().execute(Print(msg_styled));
        let _ = stdout().execute(Print("\n"));
    }

    /// 打印日志文件路径
    pub fn print_log_path(path: &str) {
        let _ = stdout().execute(Print("\n"));
        let _ = stdout().execute(Print(style("  📁 ").with(CliTheme::ACCENT)));
        let _ = stdout().execute(Print(style("日志文件: ").with(CliTheme::HINT)));
        let _ = stdout().execute(Print(format!("{}\n", path)));
    }

    /// 打印空行
    pub fn print_blank() {
        let _ = stdout().execute(Print("\n"));
    }
}

/// Convenience macro for translation
macro_rules! t {
    ($key:expr) => {
        rust_i18n::t!($key)
    };
    ($key:expr, $($tt:tt)*) => {
        rust_i18n::t!($key, $($tt)*)
    };
}

fn main() -> Result<()> {
    // Initialize locale based on system settings
    init_locale();

    // Check if we should run in interactive mode
    if should_run_interactive() {
        return run_interactive_mode();
    }

    // Standard CLI mode
    run_cli_mode()
}

/// Run in interactive mode with Ratatui TUI
fn run_interactive_mode() -> Result<()> {
    // Get executable directory first for log path
    let exe_dir = get_executable_dir()?;
    let log_dir = exe_dir.join("Log");
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let log_path = log_dir.join(format!("Interactive_{}.log", timestamp));

    // Setup file-only logging before TUI starts
    let _guard = setup_file_only_logging(&log_path)?;

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "Gallery Sorter starting in interactive mode"
    );

    let mut wizard = TuiApp::new()?;
    wizard.set_log_path(log_path.clone());

    // Run configuration wizard (processing happens within TUI)
    match wizard.run()? {
        Some(_) => {
            // Config was completed and processing ran within TUI
            info!(log_file = %log_path.display(), "Interactive session complete");
        }
        None => {
            // User cancelled
            info!("User cancelled interactive mode");
        }
    };

    // 日志路径已在 TUI 摘要屏幕显示

    Ok(())
}

/// Run in standard CLI mode
fn run_cli_mode() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Get the executable directory for Config and Log directories
    let exe_dir = get_executable_dir()?;

    // Determine log file path based on config file or timestamp
    let log_path = get_log_path(&exe_dir, &cli);

    // Setup logging
    let _guard = setup_logging(&cli, &log_path)?;

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "Gallery Sorter starting"
    );

    // Load configuration
    let config = load_config(&cli, &exe_dir)?;

    // Log configuration
    if cli.verbose {
        info!(?config, "Configuration loaded");
    }

    // Log to file location
    info!(log_file = %log_path.display(), "Log file location");

    // Validate configuration
    validate_config(&config)?;

    // Create and run processor
    let mut processor = Processor::new(config)?;

    match processor.run() {
        Ok(results) => {
            use cli_output::*;

            // Store translations to avoid temporary value issues
            let stat_processed = t!("stat_processed");
            let stat_skipped = t!("stat_skipped");
            let stat_duplicates = t!("stat_duplicates");
            let stat_failed = t!("stat_failed");

            // Print summary header
            print_separator();
            print_title(&t!("cli_processing_complete"));
            print_separator();

            // Get stats
            let stats = processor.stats();
            let processed = stats.processed.load(std::sync::atomic::Ordering::Relaxed);
            let skipped = stats.skipped.load(std::sync::atomic::Ordering::Relaxed);
            let duplicates = stats.duplicates.load(std::sync::atomic::Ordering::Relaxed);
            let failed_count = stats.failed.load(std::sync::atomic::Ordering::Relaxed);

            // Print stats with colors
            print_blank();
            print_stat(&stat_processed, &processed.to_string(), CliTheme::SUCCESS);
            print_stat(&stat_skipped, &skipped.to_string(), CliTheme::WARNING);
            print_stat(&stat_duplicates, &duplicates.to_string(), CliTheme::ACCENT);
            print_stat(&stat_failed, &failed_count.to_string(), CliTheme::ERROR);
            print_blank();

            // Store translations for results
            let already_processed = t!("already_processed");
            let duplicate_of = t!("duplicate_of");
            let unknown_error = t!("unknown_error");

            // Print detailed results if verbose
            if cli.verbose {
                print_separator();
                print_hint(&t!("cli_detailed_results"));
                print_blank();

                for result in &results {
                    match result.status {
                        gallery_sorter::process::ProcessingStatus::Success => {
                            let dest = result
                                .destination
                                .as_ref()
                                .map(|p| p.display().to_string())
                                .unwrap_or_default();
                            print_result(
                                "✓",
                                CliTheme::SUCCESS,
                                &result.source.display().to_string(),
                                &format!("→ {}", dest),
                            );
                        }
                        gallery_sorter::process::ProcessingStatus::Skipped => {
                            print_result(
                                "⊘",
                                CliTheme::WARNING,
                                &result.source.display().to_string(),
                                &already_processed,
                            );
                        }
                        gallery_sorter::process::ProcessingStatus::Duplicate => {
                            let dest = result
                                .destination
                                .as_ref()
                                .map(|p| p.display().to_string())
                                .unwrap_or_default();
                            print_result(
                                "≡",
                                CliTheme::ACCENT,
                                &result.source.display().to_string(),
                                &format!("{}: {}", duplicate_of, dest),
                            );
                        }
                        gallery_sorter::process::ProcessingStatus::Failed => {
                            let error_msg = result.error.as_deref().unwrap_or(&unknown_error);
                            print_result(
                                "✗",
                                CliTheme::ERROR,
                                &result.source.display().to_string(),
                                error_msg,
                            );
                        }
                        gallery_sorter::process::ProcessingStatus::DryRun => {
                            let dest = result
                                .destination
                                .as_ref()
                                .map(|p| p.display().to_string())
                                .unwrap_or_default();
                            print_result(
                                "~",
                                CliTheme::ACCENT,
                                &result.source.display().to_string(),
                                &format!("→ {}", dest),
                            );
                        }
                    }
                }
            }

            // Report failed files summary
            let failed_items: Vec<_> = results
                .iter()
                .filter(|r| r.status == gallery_sorter::process::ProcessingStatus::Failed)
                .collect();

            if !failed_items.is_empty() {
                print_separator();
                print_error(&format!(
                    "{} {} {}",
                    t!("cli_failed_files"),
                    failed_items.len(),
                    t!("files")
                ));
                print_blank();
                for result in &failed_items {
                    let error_msg = result.error.as_deref().unwrap_or(&unknown_error);
                    print_key_value(
                        &result.source.display().to_string(),
                        error_msg,
                        Some(CliTheme::ERROR),
                    );
                }
            }

            if cli.dry_run {
                print_separator();
                print_warning(&t!("cli_dry_run_notice"));
            }

            // Print log file path
            print_separator();
            print_log_path(&log_path.display().to_string());

            info!(log_file = %log_path.display(), "Processing complete. Log saved to");

            Ok(())
        }
        Err(e) => {
            error!(error = %e, "Processing failed");
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

/// Get the directory where the executable is located
fn get_executable_dir() -> Result<PathBuf> {
    let exe_path = std::env::current_exe()?;
    Ok(exe_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".")))
}

/// Determine the log file path based on config file or timestamp
fn get_log_path(exe_dir: &Path, cli: &Cli) -> PathBuf {
    let log_dir = exe_dir.join("Log");
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");

    if let Some(config_name) = cli.config_name() {
        let config_log_dir = log_dir.join(&config_name);
        let log_filename = format!("{}_{}.log", config_name, timestamp);
        config_log_dir.join(log_filename)
    } else {
        let log_filename = format!("CLIRun_{}.log", timestamp);
        log_dir.join(log_filename)
    }
}

/// Resolve config path - supports shorthand syntax
fn resolve_config_path(exe_dir: &Path, config_path: &Path) -> PathBuf {
    if config_path.exists() {
        return config_path.to_path_buf();
    }

    let with_extension = if config_path.extension().is_none() {
        config_path.with_extension("toml")
    } else {
        config_path.to_path_buf()
    };

    if with_extension.exists() {
        return with_extension;
    }

    let config_dir = exe_dir.join("Config");
    let filename = config_path.file_name().unwrap_or(config_path.as_os_str());

    let mut in_config_dir = config_dir.join(filename);
    if in_config_dir.extension().is_none() {
        in_config_dir = in_config_dir.with_extension("toml");
    }

    if in_config_dir.exists() {
        return in_config_dir;
    }

    config_path.to_path_buf()
}

/// Load configuration from file or CLI arguments
fn load_config(cli: &Cli, exe_dir: &Path) -> Result<Config> {
    let config = if let Some(ref config_path) = cli.config {
        let resolved_path = resolve_config_path(exe_dir, config_path);
        info!(config_file = %resolved_path.display(), "Loading configuration from file");
        let file_config = Config::load_from_file(&resolved_path)?;
        cli.merge_with_config(file_config)
    } else {
        cli.to_config()
    };

    if config.input_dirs.is_empty() {
        anyhow::bail!("{}", t!("cli_no_input_dirs_error"));
    }

    Ok(config)
}

/// Setup logging for CLI mode (file + console)
fn setup_logging(cli: &Cli, log_path: &Path) -> Result<Option<WorkerGuard>> {
    let level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    let env_filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();

    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log_path)?;

    let (non_blocking, guard) = tracing_appender::non_blocking(file);

    let subscriber = tracing_subscriber::registry().with(env_filter);

    if cli.json_log {
        subscriber
            .with(
                fmt::layer()
                    .json()
                    .with_ansi(false)
                    .with_writer(non_blocking),
            )
            .with(fmt::layer().with_writer(std::io::stderr))
            .init();
    } else {
        subscriber
            .with(fmt::layer().with_ansi(false).with_writer(non_blocking))
            .with(fmt::layer().with_writer(std::io::stderr))
            .init();
    }

    Ok(Some(guard))
}

/// Setup logging for interactive mode (file only, no console)
fn setup_file_only_logging(log_path: &Path) -> Result<Option<WorkerGuard>> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .from_env_lossy();

    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log_path)?;

    let (non_blocking, guard) = tracing_appender::non_blocking(file);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_ansi(false).with_writer(non_blocking))
        .init();

    Ok(Some(guard))
}

/// Validate configuration before processing
fn validate_config(config: &gallery_sorter::Config) -> Result<()> {
    for input_dir in &config.input_dirs {
        if !input_dir.exists() {
            eprintln!("{} {}", t!("cli_input_dir_not_exist"), input_dir.display());
        }
    }

    for input_dir in &config.input_dirs {
        if config.output_dir.starts_with(input_dir) {
            anyhow::bail!(
                "{} {} {} {}",
                t!("cli_output_inside_input_error"),
                config.output_dir.display(),
                t!("cli_is_inside"),
                input_dir.display()
            );
        }
    }

    Ok(())
}
