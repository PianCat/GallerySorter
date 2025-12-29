//! Gallery Sorter - A professional CLI tool for photo album organization
//!
//! This library provides functionality for organizing photos and videos
//! based on their creation time with support for:
//! - EXIF metadata extraction for images
//! - FFprobe-based metadata extraction for videos
//! - Filename timestamp parsing
//! - xxHash-based file deduplication
//! - Parallel processing with Rayon
//! - Incremental processing
//! - Interactive CLI mode with progress display

// Initialize i18n with locale files
rust_i18n::i18n!("locales", fallback = "en");

pub mod cli;
pub mod config;
pub mod error;
pub mod hash;
pub mod i18n;
pub mod os;
pub mod process;
pub mod state;
pub mod time;
pub mod tui;

pub use cli::Cli;
pub use config::{ClassificationRule, Config, ConfigError, FileOperation, FileType, MonthFormat, ProcessingMode};
pub use error::{Error, Result};
pub use i18n::init_locale;
pub use process::Processor;
pub use state::{IncrementalWatermark, ProcessingState};
pub use tui::{TuiApp, TuiResult, display_summary, should_run_interactive};
