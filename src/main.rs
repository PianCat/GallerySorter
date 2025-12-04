//! Gallery Sorter - Professional photo and video organization tool
//!
//! A CLI tool for organizing media files based on creation time with
//! intelligent time extraction from EXIF, video metadata, filenames,
//! and file system timestamps.

use anyhow::Result;
use chrono::Local;
use clap::Parser;
use console::style;
use gallery_sorter::{
    display_summary, should_run_interactive, Cli, Config, InteractiveWizard, Processor,
};
use gallery_sorter::i18n::Strings;
use std::path::PathBuf;
use tracing::{error, info, Level};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn main() -> Result<()> {
    // Check if we should run in interactive mode
    if should_run_interactive() {
        return run_interactive_mode();
    }

    // Standard CLI mode
    run_cli_mode()
}

/// Run in interactive mode with wizard and progress display
fn run_interactive_mode() -> Result<()> {
    let wizard = InteractiveWizard::new();

    // Run configuration wizard
    let config = match wizard.run()? {
        Some(config) => config,
        None => return Ok(()), // User cancelled
    };

    // Get the executable directory for Log directory
    let exe_dir = get_executable_dir()?;
    let log_dir = exe_dir.join("Log");
    std::fs::create_dir_all(&log_dir)?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let log_path = log_dir.join(format!("Interactive_{}.log", timestamp));

    // Setup file-only logging (no console output for interactive mode)
    let _guard = setup_file_only_logging(&log_path)?;

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "Gallery Sorter starting in interactive mode"
    );
    info!(?config, "Configuration loaded");

    // Validate configuration
    validate_config(&config)?;

    let dry_run = config.dry_run;

    // Create processor
    let mut processor = Processor::new(config)?;

    // Run with progress display
    println!("\n{} {}\n", style("🚀").cyan(), Strings::starting_processing());

    match processor.run() {
        Ok(results) => {
            let stats = processor.stats();

            // Display beautiful summary
            display_summary(stats, &results, dry_run);

            info!(log_file = %log_path.display(), "Processing complete");
            println!("\n  {} {} {}\n",
                style("📝").dim(),
                Strings::log_saved_to(),
                style(log_path.display()).dim()
            );

            Ok(())
        }
        Err(e) => {
            error!(error = %e, "Processing failed");
            eprintln!("\n{} Error: {}\n", style("✗").red(), style(&e).red());
            std::process::exit(1);
        }
    }
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
            // Print summary
            println!("\n{}", "=".repeat(60));
            println!("{}", Strings::cli_processing_complete());
            println!("{}", "=".repeat(60));
            println!("{}", processor.stats().summary());

            // Print detailed results if verbose
            if cli.verbose {
                println!("\n{}:", Strings::cli_detailed_results());
                for result in &results {
                    match result.status {
                        gallery_sorter::process::ProcessingStatus::Success => {
                            println!(
                                "  {} {} -> {}",
                                Strings::status_ok(),
                                result.source.display(),
                                result.destination.as_ref().map(|p| p.display().to_string()).unwrap_or_default()
                            );
                        }
                        gallery_sorter::process::ProcessingStatus::Skipped => {
                            println!("  {} {} {}", Strings::status_skip(), result.source.display(), Strings::already_processed());
                        }
                        gallery_sorter::process::ProcessingStatus::Duplicate => {
                            println!(
                                "  {} {} {} {})",
                                Strings::status_dup(),
                                result.source.display(),
                                Strings::duplicate_of(),
                                result.destination.as_ref().map(|p| p.display().to_string()).unwrap_or_default()
                            );
                        }
                        gallery_sorter::process::ProcessingStatus::Failed => {
                            println!(
                                "  {} {} - {}",
                                Strings::status_fail(),
                                result.source.display(),
                                result.error.as_deref().unwrap_or(Strings::unknown_error())
                            );
                        }
                        gallery_sorter::process::ProcessingStatus::DryRun => {
                            println!(
                                "  {} {} -> {}",
                                Strings::status_dry(),
                                result.source.display(),
                                result.destination.as_ref().map(|p| p.display().to_string()).unwrap_or_default()
                            );
                        }
                    }
                }
            }

            // Report failed files
            let failed: Vec<_> = results
                .iter()
                .filter(|r| r.status == gallery_sorter::process::ProcessingStatus::Failed)
                .collect();

            if !failed.is_empty() {
                eprintln!("\n{}:", Strings::cli_failed_files());
                for result in &failed {
                    eprintln!(
                        "  {} - {}",
                        result.source.display(),
                        result.error.as_deref().unwrap_or(Strings::unknown_error())
                    );
                }
            }

            if cli.dry_run {
                println!("\n{}", Strings::cli_dry_run_notice());
            }

            info!(log_file = %log_path.display(), "Processing complete. Log saved to");
            println!("\n{} {}", Strings::log_saved_to(), log_path.display());

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
fn get_log_path(exe_dir: &PathBuf, cli: &Cli) -> PathBuf {
    let log_dir = exe_dir.join("Log");

    let log_filename = if let Some(config_name) = cli.config_name() {
        // Use config file name for log file
        format!("{}.log", config_name)
    } else {
        // Use timestamp for CLI run
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        format!("CLIRun_{}.log", timestamp)
    };

    log_dir.join(log_filename)
}

/// Resolve config path - supports shorthand syntax
/// e.g., "Config1" -> "Config/Config1.toml"
fn resolve_config_path(exe_dir: &PathBuf, config_path: &PathBuf) -> PathBuf {
    // If the path exists as-is, use it directly
    if config_path.exists() {
        return config_path.clone();
    }

    // Try adding .toml extension if not present
    let with_extension = if config_path.extension().is_none() {
        config_path.with_extension("toml")
    } else {
        config_path.clone()
    };

    if with_extension.exists() {
        return with_extension;
    }

    // Try in the Config directory relative to executable
    let config_dir = exe_dir.join("Config");
    let filename = config_path
        .file_name()
        .unwrap_or(config_path.as_os_str());

    // Try with .toml extension
    let mut in_config_dir = config_dir.join(filename);
    if in_config_dir.extension().is_none() {
        in_config_dir = in_config_dir.with_extension("toml");
    }

    if in_config_dir.exists() {
        return in_config_dir;
    }

    // Return original path (will fail with proper error message later)
    config_path.clone()
}

/// Load configuration from file or CLI arguments
fn load_config(cli: &Cli, exe_dir: &PathBuf) -> Result<Config> {
    let config = if let Some(ref config_path) = cli.config {
        // Resolve config path (support shorthand syntax)
        let resolved_path = resolve_config_path(exe_dir, config_path);
        // Load from config file
        info!(config_file = %resolved_path.display(), "Loading configuration from file");
        let file_config = Config::load_from_file(&resolved_path)?;
        // Merge with CLI arguments (CLI takes precedence)
        cli.merge_with_config(file_config)
    } else {
        // Use CLI arguments only
        cli.to_config()
    };

    // Validate that we have input directories
    if config.input_dirs.is_empty() {
        anyhow::bail!(Strings::cli_no_input_dirs_error());
    }

    Ok(config)
}

/// Setup logging for CLI mode (file + console)
fn setup_logging(cli: &Cli, log_path: &PathBuf) -> Result<Option<WorkerGuard>> {
    let level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    let env_filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();

    // Create Log directory if needed
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Open log file
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log_path)?;

    let (non_blocking, guard) = tracing_appender::non_blocking(file);

    let subscriber = tracing_subscriber::registry().with(env_filter);

    if cli.json_log {
        subscriber
            .with(fmt::layer().json().with_ansi(false).with_writer(non_blocking))
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
fn setup_file_only_logging(log_path: &PathBuf) -> Result<Option<WorkerGuard>> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .from_env_lossy();

    // Create Log directory if needed
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Open log file
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
    // Check input directories exist
    for input_dir in &config.input_dirs {
        if !input_dir.exists() {
            eprintln!("{} {}", Strings::cli_input_dir_not_exist(), input_dir.display());
        }
    }

    // Check output directory is not inside input directories
    for input_dir in &config.input_dirs {
        if config.output_dir.starts_with(input_dir) {
            anyhow::bail!(
                "{} {} {} {}",
                Strings::cli_output_inside_input_error(),
                config.output_dir.display(),
                Strings::cli_is_inside(),
                input_dir.display()
            );
        }
    }

    Ok(())
}
