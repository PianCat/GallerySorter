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

pub mod cli;
pub mod config;
pub mod error;
pub mod hash;
pub mod i18n;
pub mod interactive;
pub mod process;
pub mod state;
pub mod time;

pub use cli::Cli;
pub use config::{ClassificationRule, Config, ConfigError, MonthFormat, ProcessingMode};
pub use error::{Error, Result};
pub use interactive::{InteractiveAction, InteractiveWizard, ProgressDisplay, should_run_interactive, display_summary};
pub use process::Processor;
pub use state::{IncrementalWatermark, ProcessingState};
