//! Error types for the gallery sorter

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for gallery sorter operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for the gallery sorter
#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to read EXIF data from {path}: {message}")]
    ExifRead { path: PathBuf, message: String },

    #[error("Failed to parse timestamp from {source_info}: {message}")]
    TimestampParse { source_info: String, message: String },

    #[error("Failed to extract video metadata from {path}: {message}")]
    VideoMetadata { path: PathBuf, message: String },

    #[error("File hash computation failed for {path}: {message}")]
    HashComputation { path: PathBuf, message: String },

    #[error("State file error: {0}")]
    StateFile(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Duplicate file detected: {original} and {duplicate}")]
    DuplicateFile { original: PathBuf, duplicate: PathBuf },

    #[error("Unsupported file format: {path}")]
    UnsupportedFormat { path: PathBuf },

    #[error("FFprobe not found. Please install FFmpeg and ensure ffprobe is in PATH")]
    FfprobeNotFound,

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("Directory traversal error: {0}")]
    WalkDir(#[from] walkdir::Error),

    #[error("Chrono parse error: {0}")]
    ChronoParse(#[from] chrono::ParseError),
}
