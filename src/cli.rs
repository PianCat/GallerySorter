//! CLI argument parsing with clap

use crate::config::{ClassificationRule, Config, FileOperation, MonthFormat, ProcessingMode};
use clap::Parser;
use std::path::PathBuf;

/// Gallery Sorter - Professional photo and video organization tool
///
/// Organizes your photos and videos by creation date with intelligent
/// time extraction from EXIF data, video metadata, filenames, and
/// file system timestamps.
#[derive(Parser, Debug)]
#[command(name = "gallery-sorter")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Path to configuration file (TOML format)
    ///
    /// When specified, settings from the config file are used as defaults.
    /// CLI arguments will override config file settings.
    #[arg(short = 'C', long)]
    pub config: Option<PathBuf>,

    /// Input directories to scan for media files
    #[arg(short, long, num_args = 1..)]
    pub input: Option<Vec<PathBuf>>,

    /// Output directory for organized files
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Processing mode:
    /// - full: Process all files, overwrite existing (default)
    /// - supplement: Skip files that already exist in target
    /// - incremental: Only process files newer than newest in target
    #[arg(short = 'M', long, value_enum)]
    pub mode: Option<ProcessingMode>,

    /// Classification rule for organizing files
    #[arg(short, long, value_enum)]
    pub classify: Option<ClassificationRule>,

    /// Month format for year-month classification
    #[arg(short = 'm', long, value_enum)]
    pub month_format: Option<MonthFormat>,

    /// File operation mode
    #[arg(short = 'O', long, value_enum)]
    pub operation: Option<FileOperation>,

    /// Disable file deduplication
    #[arg(long)]
    pub no_deduplicate: bool,

    /// State file path for tracking processed files
    #[arg(long)]
    pub state_file: Option<PathBuf>,

    /// Number of threads for parallel processing (0 = auto)
    #[arg(short = 't', long)]
    pub threads: Option<usize>,

    /// Large file threshold in MB (files larger use sampled hashing)
    #[arg(long)]
    pub large_file_mb: Option<u64>,

    /// Dry run mode - show what would be done without doing it
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Output log format as JSON
    #[arg(long)]
    pub json_log: bool,
}

impl Cli {
    /// Get config file name (without extension) for log naming
    pub fn config_name(&self) -> Option<String> {
        self.config.as_ref().and_then(|p| {
            p.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
    }

    /// Merge CLI arguments with config from file
    /// CLI arguments take precedence over config file settings
    pub fn merge_with_config(&self, mut config: Config) -> Config {
        // Override with CLI arguments if provided
        if let Some(ref inputs) = self.input {
            config.input_dirs = inputs.clone();
        }
        if let Some(ref output) = self.output {
            config.output_dir = output.clone();
        }
        if let Some(mode) = self.mode {
            config.processing_mode = mode;
        }
        if let Some(classify) = self.classify {
            config.classification = classify;
        }
        if let Some(month_format) = self.month_format {
            config.month_format = month_format;
        }
        if let Some(operation) = self.operation {
            config.operation = operation;
        }
        if self.no_deduplicate {
            config.deduplicate = false;
        }
        if let Some(ref state_file) = self.state_file {
            config.state_file = Some(state_file.clone());
        }
        if let Some(threads) = self.threads {
            config.threads = threads;
        }
        if let Some(large_file_mb) = self.large_file_mb {
            config.large_file_threshold = large_file_mb * 1024 * 1024;
        }
        if self.dry_run {
            config.dry_run = true;
        }
        if self.verbose {
            config.verbose = true;
        }

        config
    }

    /// Convert CLI arguments to Config (when no config file is used)
    pub fn to_config(&self) -> Config {
        let mut config = Config::default();

        if let Some(ref inputs) = self.input {
            config.input_dirs = inputs.clone();
        }
        if let Some(ref output) = self.output {
            config.output_dir = output.clone();
        }
        if let Some(mode) = self.mode {
            config.processing_mode = mode;
        } else {
            config.processing_mode = ProcessingMode::Full;
        }
        if let Some(classify) = self.classify {
            config.classification = classify;
        } else {
            config.classification = ClassificationRule::YearMonth;
        }
        if let Some(month_format) = self.month_format {
            config.month_format = month_format;
        }
        if let Some(operation) = self.operation {
            config.operation = operation;
        }
        config.deduplicate = !self.no_deduplicate;
        config.state_file = self.state_file.clone();
        if let Some(threads) = self.threads {
            config.threads = threads;
        }
        if let Some(large_file_mb) = self.large_file_mb {
            config.large_file_threshold = large_file_mb * 1024 * 1024;
        }
        config.dry_run = self.dry_run;
        config.verbose = self.verbose;

        config
    }
}
