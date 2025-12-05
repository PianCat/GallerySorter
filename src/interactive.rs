//! Interactive CLI mode with progress bars and real-time statistics

use crate::config::{ClassificationRule, Config, FileOperation, MonthFormat, ProcessingMode};
use crate::i18n::Strings;
use crate::process::{FileResult, ProcessingStats, ProcessingStatus, Processor};
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
║                        SORTER v0.1.0                          ║
║                                                               ║
║                  Photo & Video Organization                   ║
╚═══════════════════════════════════════════════════════════════╝
"#;
        println!("{}", style(banner).cyan());
    }

    /// Run the interactive main menu
    pub fn run(&self) -> anyhow::Result<Option<InteractiveResult>> {
        self.show_banner();

        println!("\n{}", style(Strings::welcome_message()).green().bold());
        println!("{}\n", style(Strings::select_option_prompt()).dim());

        loop {
            let menu_options = vec![
                Strings::menu_option_run_direct(),
                Strings::menu_option_run_config(),
                Strings::menu_option_create_config(),
                Strings::menu_option_exit(),
            ];

            let selection = Select::new()
                .with_prompt(Strings::select_option())
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
                    println!("{}", style(Strings::goodbye()).yellow());
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
                style("⚠").yellow(),
                Strings::no_configs_found()
            );
            println!("  {} {}\n",
                style("→").dim(),
                Strings::use_option_3_hint()
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
        display_names.push(Strings::back_to_main_menu().to_string());

        println!("\n{} {}\n", style("📁").cyan(), Strings::available_configurations());

        let selection = Select::new()
            .with_prompt(Strings::select_configuration())
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
            Strings::loading_configuration(),
            style(selected_path.display()).green()
        );

        let config = Config::load_from_file(selected_path)?;

        // Show summary
        self.show_config_summary(&config);

        // Confirm to proceed
        if !Confirm::new()
            .with_prompt(Strings::proceed_with_config())
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
        println!("{}", style(Strings::create_new_configuration()).cyan().bold());
        println!("{}\n", style("━".repeat(60)).dim());

        // Get configuration name
        let config_name: String = Input::new()
            .with_prompt(Strings::enter_config_name())
            .validate_with(|input: &String| -> Result<(), &str> {
                if input.trim().is_empty() {
                    Err(Strings::config_name_empty_error())
                } else if input.contains('/') || input.contains('\\') || input.contains('.') {
                    Err(Strings::config_name_invalid_chars_error())
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
                    Strings::config_exists_overwrite()
                ))
                .default(false)
                .interact()?
            {
                println!("{}", style(Strings::configuration_cancelled()).yellow());
                return Ok(());
            }
        }

        // Collect configuration parameters
        let config = match self.collect_config_params()? {
            Some(config) => config,
            None => {
                println!("{}", style(Strings::configuration_cancelled()).yellow());
                return Ok(());
            }
        };

        // Show summary
        self.show_config_summary(&config);

        // Confirm to save
        if !Confirm::new()
            .with_prompt(Strings::save_configuration())
            .default(true)
            .interact()?
        {
            println!("{}", style(Strings::config_not_saved()).yellow());
            return Ok(());
        }

        // Save the configuration
        config.save_to_file(&config_path)?;

        println!("\n{} {} {}",
            style("✓").green(),
            Strings::config_saved_to(),
            style(config_path.display()).green()
        );
        println!("  {} {}\n",
            style("→").dim(),
            Strings::use_option_2_hint()
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
            .with_prompt(Strings::proceed_with_settings())
            .default(true)
            .interact()?
        {
            println!("{}", style(Strings::operation_cancelled()).yellow());
            return Ok(None);
        }

        Ok(Some(config))
    }

    /// Collect configuration parameters from user input
    fn collect_config_params(&self) -> anyhow::Result<Option<Config>> {
        println!("\n{}", style("━".repeat(60)).dim());
        println!("{}", style(Strings::configuration_parameters()).cyan().bold());
        println!("{}\n", style("━".repeat(60)).dim());

        // Get input directories
        let input_str: String = Input::new()
            .with_prompt(Strings::enter_input_directory())
            .interact_text()?;

        let input_dirs: Vec<PathBuf> = input_str
            .split(';')
            .map(|s| PathBuf::from(s.trim()))
            .filter(|p| !p.as_os_str().is_empty())
            .collect();

        if input_dirs.is_empty() {
            println!("{} {}", style("⚠").yellow(), Strings::no_input_dirs_specified());
            return Ok(None);
        }

        // Validate input directories
        for dir in &input_dirs {
            if !dir.exists() {
                println!("{} {} {}",
                    style("⚠").yellow(),
                    Strings::directory_not_exist(),
                    style(dir.display()).red()
                );
            }
        }

        // Get output directory
        let output_dir: String = Input::new()
            .with_prompt(Strings::enter_output_directory())
            .default("output".to_string())
            .interact_text()?;

        // Select processing mode
        let mode_options = vec![
            Strings::mode_full(),
            Strings::mode_supplement(),
            Strings::mode_incremental(),
        ];
        let mode_idx = Select::new()
            .with_prompt(Strings::select_processing_mode())
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
            Strings::classify_year_month(),
            Strings::classify_year(),
            Strings::classify_none(),
        ];
        let classify_idx = Select::new()
            .with_prompt(Strings::select_classification_rule())
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
                Strings::month_format_nested(),
                Strings::month_format_combined(),
            ];
            let format_idx = Select::new()
                .with_prompt(Strings::select_month_format())
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
            Strings::operation_copy(),
            Strings::operation_move(),
            Strings::operation_hardlink(),
        ];
        let operation_idx = Select::new()
            .with_prompt(Strings::select_file_operation())
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
            .with_prompt(Strings::enable_deduplication())
            .default(true)
            .interact()?;

        // Dry run
        let dry_run = Confirm::new()
            .with_prompt(Strings::dry_run_mode())
            .default(false)
            .interact()?;

        // Build config
        let config = Config {
            input_dirs,
            output_dir: PathBuf::from(output_dir),
            processing_mode,
            classification,
            month_format,
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
        println!("{}", style(Strings::configuration_summary()).cyan().bold());
        println!("{}", style("━".repeat(60)).dim());
        println!("  {} {:?}", style(Strings::summary_input()).green(), config.input_dirs);
        println!("  {} {}", style(Strings::summary_output()).green(), config.output_dir.display());
        println!("  {} {:?}", style(Strings::summary_mode()).green(), config.processing_mode);
        println!("  {} {:?}", style(Strings::summary_classify()).green(), config.classification);
        if config.classification == ClassificationRule::YearMonth {
            println!("  {} {:?}", style(Strings::summary_month_format()).green(), config.month_format);
        }
        println!("  {} {:?}", style(Strings::summary_operation()).green(), config.operation);
        println!("  {} {}", style(Strings::summary_deduplicate()).green(), config.deduplicate);
        println!("  {} {}", style(Strings::summary_dry_run()).green(), config.dry_run);
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
    println!("\n{} {}", style("📁").cyan(), Strings::scanning_directories());

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
    println!("{:^60}", style(Strings::processing_complete()).cyan().bold());
    println!("{}", style("═".repeat(60)).cyan());

    let total = stats.total_files.load(Ordering::Relaxed);
    let processed = stats.processed.load(Ordering::Relaxed);
    let skipped = stats.skipped.load(Ordering::Relaxed);
    let duplicates = stats.duplicates.load(Ordering::Relaxed);
    let failed = stats.failed.load(Ordering::Relaxed);

    println!("\n  {} {}",
        style("📊").cyan(),
        style(Strings::statistics()).bold()
    );
    println!("  {}", style("─".repeat(40)).dim());
    println!("    {} {}     {}", style("📁").dim(), Strings::stat_total_files(), style(total).bold());
    println!("    {} {}       {}", style("✓").green(), Strings::stat_processed(), style(processed).green().bold());
    println!("    {} {}         {}", style("○").cyan(), Strings::stat_skipped(), style(skipped).cyan().bold());
    println!("    {} {}      {}", style("◎").yellow(), Strings::stat_duplicates(), style(duplicates).yellow().bold());
    println!("    {} {}          {}", style("✗").red(), Strings::stat_failed(), style(failed).red().bold());

    // Show failed files if any
    let failed_files: Vec<_> = results.iter()
        .filter(|r| r.status == ProcessingStatus::Failed)
        .collect();

    if !failed_files.is_empty() {
        println!("\n  {} {}",
            style("⚠").yellow(),
            style(Strings::failed_files()).yellow().bold()
        );
        println!("  {}", style("─".repeat(40)).dim());
        for (i, result) in failed_files.iter().take(5).enumerate() {
            println!("    {}. {} - {}",
                i + 1,
                style(result.source.file_name().unwrap_or_default().to_string_lossy()).red(),
                style(result.error.as_deref().unwrap_or(Strings::unknown_error())).dim()
            );
        }
        if failed_files.len() > 5 {
            println!("    {}", style(Strings::and_n_more(failed_files.len() - 5)).dim());
        }
    }

    if dry_run {
        println!("\n  {}", style(format!("🔍 {}", Strings::dry_run_notice())).yellow().bold());
    }

    println!("\n{}", style("═".repeat(60)).cyan());
}

/// Check if running in interactive mode (no arguments provided)
pub fn should_run_interactive() -> bool {
    let args: Vec<String> = std::env::args().collect();
    // Only the program name, no other arguments
    args.len() == 1
}
