//! Interactive CLI mode with progress bars and real-time statistics

use crate::config::{ClassificationRule, Config, FileOperation, MonthFormat, ProcessingMode};
use crate::process::{FileResult, ProcessingStats, ProcessingStatus, Processor};

// Initialize i18n for this module
rust_i18n::i18n!("locales", fallback = "en");

/// Convenience macro for translation
macro_rules! t {
    ($key:expr) => {
        rust_i18n::t!($key)
    };
    ($key:expr, $($tt:tt)*) => {
        rust_i18n::t!($key, $($tt)*)
    };
}
use console::{style, Term};
use dialoguer::{Confirm, Input, Select};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Result of the interactive menu selection
pub enum InteractiveAction {
    /// Run with directly input parameters
    RunWithConfig(Config),
    /// User cancelled or chose to exit
    Exit,
}

/// Rolling log buffer to store recent log messages
pub struct RollingLogBuffer {
    messages: Vec<String>,
    max_size: usize,
}

impl RollingLogBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            messages: Vec::with_capacity(max_size),
            max_size,
        }
    }

    pub fn push(&mut self, message: String) {
        if self.messages.len() >= self.max_size {
            self.messages.remove(0);
        }
        self.messages.push(message);
    }

    pub fn get_messages(&self) -> &[String] {
        &self.messages
    }
}

/// Interactive configuration wizard
pub struct InteractiveWizard {
    #[allow(dead_code)]
    term: Term,
    exe_dir: PathBuf,
}

/// Result of interactive wizard with config and optional config name
pub struct InteractiveResult {
    pub config: Config,
    pub config_name: Option<String>,
}

impl InteractiveWizard {
    pub fn new() -> Self {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        Self {
            term: Term::stderr(),
            exe_dir,
        }
    }

    /// Display welcome banner
    pub fn show_banner(&self) {
        let banner = r#"
╔═══════════════════════════════════════════════════════════════╗
║                                                               ║
║   ██████╗  █████╗ ██╗     ██╗     ███████╗██████╗ ██╗   ██╗   ║
║  ██╔════╝ ██╔══██╗██║     ██║     ██╔════╝██╔══██╗╚██╗ ██╔╝   ║
║  ██║  ███╗███████║██║     ██║     █████╗  ██████╔╝ ╚████╔╝    ║
║  ██║   ██║██╔══██║██║     ██║     ██╔══╝  ██╔══██╗  ╚██╔╝     ║
║  ╚██████╔╝██║  ██║███████╗███████╗███████╗██║  ██║   ██║      ║
║   ╚═════╝ ╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝╚═╝  ╚═╝   ╚═╝      ║
║                            SORTER                             ║
║                                                               ║
║                  Photo & Video Organization                   ║
╚═══════════════════════════════════════════════════════════════╝
"#;
        println!("{}", style(banner).cyan());
    }

    /// Run the interactive main menu
    pub fn run(&self) -> anyhow::Result<Option<InteractiveResult>> {
        self.show_banner();

        println!("\n{}", style(t!("welcome_message").to_string()).green().bold());
        println!("{}\n", style(t!("select_option_prompt").to_string()).dim());

        loop {
            let menu_options = vec![
                t!("menu_option_run_direct").to_string(),
                t!("menu_option_run_config").to_string(),
                t!("menu_option_create_config").to_string(),
                t!("menu_option_exit").to_string(),
            ];

            let selection = Select::new()
                .with_prompt(t!("select_option").to_string())
                .items(&menu_options)
                .default(0)
                .interact()?;

            match selection {
                0 => {
                    // Run with directly input parameters
                    match self.collect_config_interactive()? {
                        Some(config) => return Ok(Some(InteractiveResult {
                            config,
                            config_name: None,
                        })),
                        None => continue, // User cancelled, show menu again
                    }
                }
                1 => {
                    // Run with selected configuration
                    match self.select_and_load_config()? {
                        Some((config, config_name)) => return Ok(Some(InteractiveResult {
                            config,
                            config_name: Some(config_name),
                        })),
                        None => continue, // User cancelled or no configs, show menu again
                    }
                }
                2 => {
                    // Create configuration
                    self.create_and_save_config()?;
                    // After creating config, show menu again
                    continue;
                }
                3 => {
                    // Exit program
                    println!("{}", style(t!("goodbye").to_string()).yellow());
                    return Ok(None);
                }
                _ => continue,
            }
        }
    }

    /// Get the Config directory path
    fn get_config_dir(&self) -> PathBuf {
        self.exe_dir.join("Config")
    }

    /// List available configuration files
    fn list_config_files(&self) -> Vec<PathBuf> {
        let config_dir = self.get_config_dir();
        if !config_dir.exists() {
            return Vec::new();
        }

        fs::read_dir(&config_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path().extension()
                            .map(|ext| ext == "toml")
                            .unwrap_or(false)
                    })
                    .map(|e| e.path())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Select and load a configuration file
    fn select_and_load_config(&self) -> anyhow::Result<Option<(Config, String)>> {
        let config_files = self.list_config_files();

        if config_files.is_empty() {
            println!("\n{} {}",
                style("!").yellow(),
                t!("no_configs_found")
            );
            println!("  {} {}\n",
                style("→").dim(),
                t!("use_option_3_hint")
            );
            return Ok(None);
        }

        // Build display names for configs
        let mut display_names: Vec<String> = config_files
            .iter()
            .map(|p| {
                p.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
                    .to_string()
            })
            .collect();
        display_names.push(t!("back_to_main_menu").to_string());

        println!("\n{} {}\n", style(">").cyan(), t!("available_configurations"));

        let selection = Select::new()
            .with_prompt(t!("select_configuration").to_string())
            .items(&display_names)
            .default(0)
            .interact()?;

        // Check if user selected "Back"
        if selection == config_files.len() {
            return Ok(None);
        }

        let selected_path = &config_files[selection];
        let config_name = selected_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        // Load the configuration
        println!("\n{} {} {}",
            style("📄").cyan(),
            t!("loading_configuration"),
            style(selected_path.display()).green()
        );

        let config = Config::load_from_file(selected_path)?;

        // Show summary
        self.show_config_summary(&config);

        // Confirm to proceed
        if !Confirm::new()
            .with_prompt(t!("proceed_with_config").to_string())
            .default(true)
            .interact()?
        {
            return Ok(None);
        }

        Ok(Some((config, config_name)))
    }

    /// Create and save a new configuration file
    fn create_and_save_config(&self) -> anyhow::Result<()> {
        println!("\n{}", style("━".repeat(60)).dim());
        println!("{}", style(t!("create_new_configuration").to_string()).cyan().bold());
        println!("{}\n", style("━".repeat(60)).dim());

        // Get configuration name
        let config_name: String = Input::new()
            .with_prompt(t!("enter_config_name").to_string())
            .validate_with(|input: &String| -> Result<(), String> {
                if input.trim().is_empty() {
                    Err(t!("config_name_empty_error").to_string())
                } else if input.contains('/') || input.contains('\\') || input.contains('.') {
                    Err(t!("config_name_invalid_chars_error").to_string())
                } else {
                    Ok(())
                }
            })
            .interact_text()?;

        // Check if config already exists
        let config_dir = self.get_config_dir();
        let config_path = config_dir.join(format!("{}.toml", config_name.trim()));

        if config_path.exists() {
            if !Confirm::new()
                .with_prompt(format!(
                    "'{}' {}",
                    config_name,
                    t!("config_exists_overwrite")
                ))
                .default(false)
                .interact()?
            {
                println!("{}", style(t!("configuration_cancelled").to_string()).yellow());
                return Ok(());
            }
        }

        // Collect configuration parameters
        let config = match self.collect_config_params()? {
            Some(config) => config,
            None => {
                println!("{}", style(t!("configuration_cancelled").to_string()).yellow());
                return Ok(());
            }
        };

        // Show summary
        self.show_config_summary(&config);

        // Confirm to save
        if !Confirm::new()
            .with_prompt(t!("save_configuration").to_string())
            .default(true)
            .interact()?
        {
            println!("{}", style(t!("config_not_saved").to_string()).yellow());
            return Ok(());
        }

        // Save the configuration
        config.save_to_file(&config_path)?;

        println!("\n{} {} {}",
            style("✓").green(),
            t!("config_saved_to"),
            style(config_path.display()).green()
        );
        println!("  {} {}\n",
            style("→").dim(),
            t!("use_option_2_hint")
        );

        Ok(())
    }

    /// Collect configuration interactively and confirm to run
    fn collect_config_interactive(&self) -> anyhow::Result<Option<Config>> {
        let config = match self.collect_config_params()? {
            Some(config) => config,
            None => return Ok(None),
        };

        // Show summary
        self.show_config_summary(&config);

        // Confirm to proceed
        if !Confirm::new()
            .with_prompt(t!("proceed_with_settings").to_string())
            .default(true)
            .interact()?
        {
            println!("{}", style(t!("operation_cancelled").to_string()).yellow());
            return Ok(None);
        }

        Ok(Some(config))
    }

    /// Collect configuration parameters from user input
    fn collect_config_params(&self) -> anyhow::Result<Option<Config>> {
        println!("\n{}", style("━".repeat(60)).dim());
        println!("{}", style(t!("configuration_parameters").to_string()).cyan().bold());
        println!("{}\n", style("━".repeat(60)).dim());

        // Get input directories
        let input_str: String = Input::new()
            .with_prompt(t!("enter_input_directory").to_string())
            .interact_text()?;

        let input_dirs: Vec<PathBuf> = input_str
            .split(';')
            .map(|s| PathBuf::from(s.trim()))
            .filter(|p| !p.as_os_str().is_empty())
            .collect();

        if input_dirs.is_empty() {
            println!("{} {}", style("!").yellow(), t!("no_input_dirs_specified"));
            return Ok(None);
        }

        // Validate input directories
        for dir in &input_dirs {
            if !dir.exists() {
                println!("{} {} {}",
                    style("!").yellow(),
                    t!("directory_not_exist"),
                    style(dir.display()).red()
                );
            }
        }

        // Get output directory
        let output_dir: String = Input::new()
            .with_prompt(t!("enter_output_directory").to_string())
            .default("output".to_string())
            .interact_text()?;

        // Get exclude directories
        let exclude_str: String = Input::new()
            .with_prompt(t!("enter_exclude_directories").to_string())
            .allow_empty(true)
            .interact_text()?;

        let exclude_dirs: Vec<PathBuf> = exclude_str
            .split(';')
            .map(|s| PathBuf::from(s.trim()))
            .filter(|p| !p.as_os_str().is_empty())
            .collect();

        // Select processing mode
        let mode_options = vec![
            t!("mode_full").to_string(),
            t!("mode_supplement").to_string(),
            t!("mode_incremental").to_string(),
        ];
        let mode_idx = Select::new()
            .with_prompt(t!("select_processing_mode").to_string())
            .items(&mode_options)
            .default(0)
            .interact()?;

        let processing_mode = match mode_idx {
            0 => ProcessingMode::Full,
            1 => ProcessingMode::Supplement,
            2 => ProcessingMode::Incremental,
            _ => ProcessingMode::Full,
        };

        // Select classification rule
        let classify_options = vec![
            t!("classify_year_month").to_string(),
            t!("classify_year").to_string(),
            t!("classify_none").to_string(),
        ];
        let classify_idx = Select::new()
            .with_prompt(t!("select_classification_rule").to_string())
            .items(&classify_options)
            .default(0)
            .interact()?;

        let classification = match classify_idx {
            0 => ClassificationRule::YearMonth,
            1 => ClassificationRule::Year,
            2 => ClassificationRule::None,
            _ => ClassificationRule::YearMonth,
        };

        // Select month format if year-month classification
        let month_format = if classification == ClassificationRule::YearMonth {
            let format_options = vec![
                t!("month_format_nested").to_string(),
                t!("month_format_combined").to_string(),
            ];
            let format_idx = Select::new()
                .with_prompt(t!("select_month_format").to_string())
                .items(&format_options)
                .default(0)
                .interact()?;

            match format_idx {
                0 => MonthFormat::Nested,
                1 => MonthFormat::Combined,
                _ => MonthFormat::Nested,
            }
        } else {
            MonthFormat::Nested
        };

        // Select file operation
        let operation_options = vec![
            t!("operation_copy").to_string(),
            t!("operation_move").to_string(),
            t!("operation_hardlink").to_string(),
        ];
        let operation_idx = Select::new()
            .with_prompt(t!("select_file_operation").to_string())
            .items(&operation_options)
            .default(0)
            .interact()?;

        let operation = match operation_idx {
            0 => FileOperation::Copy,
            1 => FileOperation::Move,
            2 => FileOperation::Hardlink,
            _ => FileOperation::Copy,
        };

        // Deduplication
        let deduplicate = Confirm::new()
            .with_prompt(t!("enable_deduplication").to_string())
            .default(true)
            .interact()?;

        // Dry run
        let dry_run = Confirm::new()
            .with_prompt(t!("dry_run_mode").to_string())
            .default(false)
            .interact()?;

        // Classify by file type
        let classify_by_type = Confirm::new()
            .with_prompt(t!("classify_by_file_type").to_string())
            .default(false)
            .interact()?;

        // Build config
        let config = Config {
            input_dirs,
            output_dir: PathBuf::from(output_dir),
            exclude_dirs,
            processing_mode,
            classification,
            month_format,
            classify_by_type,
            operation,
            deduplicate,
            dry_run,
            verbose: false,
            ..Default::default()
        };

        Ok(Some(config))
    }

    /// Display configuration summary
    fn show_config_summary(&self, config: &Config) {
        println!("\n{}", style("━".repeat(60)).dim());
        println!("{}", style(t!("configuration_summary").to_string()).cyan().bold());
        println!("{}", style("━".repeat(60)).dim());
        println!("  {} {:?}", style(t!("summary_input").to_string()).green(), config.input_dirs);
        println!("  {} {}", style(t!("summary_output").to_string()).green(), config.output_dir.display());
        if !config.exclude_dirs.is_empty() {
            println!("  {} {:?}", style(t!("summary_exclude_dirs").to_string()).green(), config.exclude_dirs);
        }
        println!("  {} {:?}", style(t!("summary_mode").to_string()).green(), config.processing_mode);
        println!("  {} {:?}", style(t!("summary_classify").to_string()).green(), config.classification);
        if config.classification == ClassificationRule::YearMonth {
            println!("  {} {:?}", style(t!("summary_month_format").to_string()).green(), config.month_format);
        }
        println!("  {} {}", style(t!("summary_classify_by_type").to_string()).green(), config.classify_by_type);
        println!("  {} {:?}", style(t!("summary_operation").to_string()).green(), config.operation);
        println!("  {} {}", style(t!("summary_deduplicate").to_string()).green(), config.deduplicate);
        println!("  {} {}", style(t!("summary_dry_run").to_string()).green(), config.dry_run);
        println!("{}\n", style("━".repeat(60)).dim());
    }
}

/// Progress display for processing
pub struct ProgressDisplay {
    #[allow(dead_code)]
    multi_progress: MultiProgress,
    main_progress: ProgressBar,
    stats_bar: ProgressBar,
    log_bar: ProgressBar,
    log_buffer: Arc<Mutex<RollingLogBuffer>>,
}

impl ProgressDisplay {
    pub fn new(total_files: u64) -> Self {
        let multi_progress = MultiProgress::new();

        // Main progress bar
        let main_style = ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
            .unwrap()
            .progress_chars("█▓▒░  ");

        let main_progress = multi_progress.add(ProgressBar::new(total_files));
        main_progress.set_style(main_style);
        main_progress.enable_steady_tick(Duration::from_millis(100));

        // Stats bar (shows real-time statistics)
        let stats_style = ProgressStyle::default_bar()
            .template("{msg}")
            .unwrap();

        let stats_bar = multi_progress.add(ProgressBar::new(0));
        stats_bar.set_style(stats_style);

        // Log bar (shows recent log messages)
        let log_style = ProgressStyle::default_bar()
            .template("{msg}")
            .unwrap();

        let log_bar = multi_progress.add(ProgressBar::new(0));
        log_bar.set_style(log_style);

        Self {
            multi_progress,
            main_progress,
            stats_bar,
            log_bar,
            log_buffer: Arc::new(Mutex::new(RollingLogBuffer::new(5))),
        }
    }

    /// Get the log buffer for external use
    pub fn log_buffer(&self) -> Arc<Mutex<RollingLogBuffer>> {
        Arc::clone(&self.log_buffer)
    }

    /// Update progress
    pub fn set_position(&self, pos: u64) {
        self.main_progress.set_position(pos);
    }

    /// Increment progress
    pub fn inc(&self, delta: u64) {
        self.main_progress.inc(delta);
    }

    /// Set current file being processed
    pub fn set_message(&self, msg: &str) {
        // Truncate long paths
        let display_msg = if msg.len() > 50 {
            format!("...{}", &msg[msg.len()-47..])
        } else {
            msg.to_string()
        };
        self.main_progress.set_message(display_msg);
    }

    /// Update statistics display
    pub fn update_stats(&self, stats: &ProcessingStats) {
        let processed = stats.processed.load(Ordering::Relaxed);
        let skipped = stats.skipped.load(Ordering::Relaxed);
        let duplicates = stats.duplicates.load(Ordering::Relaxed);
        let failed = stats.failed.load(Ordering::Relaxed);

        let stats_msg = format!(
            "  {} {} | {} {} | {} {} | {} {}",
            style("✓").green(),
            style(format!("Processed: {}", processed)).green(),
            style("○").cyan(),
            style(format!("Skipped: {}", skipped)).cyan(),
            style("◎").yellow(),
            style(format!("Duplicates: {}", duplicates)).yellow(),
            style("✗").red(),
            style(format!("Failed: {}", failed)).red(),
        );

        self.stats_bar.set_message(stats_msg);
    }

    /// Add a log message
    pub fn add_log(&self, message: String) {
        let mut buffer = self.log_buffer.lock().unwrap();
        buffer.push(message);

        // Update log display
        let messages = buffer.get_messages();
        let log_display = messages.iter()
            .map(|m| format!("  {}", style(m).dim()))
            .collect::<Vec<_>>()
            .join("\n");

        drop(buffer);
        self.log_bar.set_message(log_display);
    }

    /// Finish progress display
    pub fn finish(&self) {
        self.main_progress.finish_with_message("Complete!");
    }

    /// Finish with error
    pub fn finish_with_error(&self, msg: &str) {
        self.main_progress.finish_with_message(format!("{} {}", style("Error:").red(), msg));
    }
}

/// Run processing with progress display
pub fn run_with_progress(
    config: Config,
    _log_path: &PathBuf,
) -> anyhow::Result<(Vec<FileResult>, ProcessingStats)> {
    // Create processor
    let mut processor = Processor::new(config)?;

    // Show scanning message
    println!("\n{} {}", style(">").cyan(), t!("scanning_directories"));

    // Get results with progress
    let results = processor.run()?;
    let stats = processor.stats().clone();

    Ok((results, stats))
}

/// Display final summary with beautiful formatting
pub fn display_summary(stats: &ProcessingStats, results: &[FileResult], dry_run: bool) {
    let term = Term::stderr();
    let _ = term.clear_last_lines(0);

    println!("\n{}", style("═".repeat(60)).cyan());
    println!("{:^60}", style(t!("processing_complete").to_string()).cyan().bold());
    println!("{}", style("═".repeat(60)).cyan());

    let total = stats.total_files.load(Ordering::Relaxed);
    let processed = stats.processed.load(Ordering::Relaxed);
    let skipped = stats.skipped.load(Ordering::Relaxed);
    let duplicates = stats.duplicates.load(Ordering::Relaxed);
    let failed = stats.failed.load(Ordering::Relaxed);

    println!("\n  {} {}",
        style("#").cyan(),
        style(t!("statistics").to_string()).bold()
    );
    println!("  {}", style("─".repeat(40)).dim());
    println!("    {} {}     {}", style(">").dim(), t!("stat_total_files"), style(total).bold());
    println!("    {} {}       {}", style("✓").green(), t!("stat_processed"), style(processed).green().bold());
    println!("    {} {}         {}", style("○").cyan(), t!("stat_skipped"), style(skipped).cyan().bold());
    println!("    {} {}      {}", style("◎").yellow(), t!("stat_duplicates"), style(duplicates).yellow().bold());
    println!("    {} {}          {}", style("✗").red(), t!("stat_failed"), style(failed).red().bold());

    // Show failed files if any
    let failed_files: Vec<_> = results.iter()
        .filter(|r| r.status == ProcessingStatus::Failed)
        .collect();

    if !failed_files.is_empty() {
        println!("\n  {} {}",
            style("!").yellow(),
            style(t!("failed_files").to_string()).yellow().bold()
        );
        println!("  {}", style("─".repeat(40)).dim());
        for (i, result) in failed_files.iter().take(5).enumerate() {
            println!("    {}. {} - {}",
                i + 1,
                style(result.source.file_name().unwrap_or_default().to_string_lossy()).red(),
                style(result.error.as_deref().unwrap_or(&t!("unknown_error"))).dim()
            );
        }
        if failed_files.len() > 5 {
            println!("    {}", style(t!("and_n_more", n = failed_files.len() - 5).to_string()).dim());
        }
    }

    if dry_run {
        println!("\n  {}", style(format!("* {}", t!("dry_run_notice"))).yellow().bold());
    }

    println!("\n{}", style("═".repeat(60)).cyan());
}

/// Check if running in interactive mode (no arguments provided)
pub fn should_run_interactive() -> bool {
    let args: Vec<String> = std::env::args().collect();
    // Only the program name, no other arguments
    args.len() == 1
}
