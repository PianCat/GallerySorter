//! Configuration types for the gallery sorter

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Processing mode for handling files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingMode {
    /// Full mode: Process all files, overwrite existing files in target
    Full,
    /// Supplement mode: Process all files, but skip if deduplicated file
    /// already exists in target folder
    Supplement,
    /// Incremental mode: Only process source files newer than the newest
    /// file in target folder (based on timestamps)
    /// (Default for regular use - efficient for ongoing photo imports)
    #[default]
    Incremental,
}

/// Classification rule for organizing files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ClassificationRule {
    /// No classification - all files go to output directory root
    #[default]
    None,
    /// Classify by year: output_directory/YYYY/
    Year,
    /// Classify by year and month: output_directory/YYYY/MM/
    YearMonth,
}

/// Month format for year-month classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum MonthFormat {
    /// Nested format: YYYY/MM/
    #[default]
    Nested,
    /// Combined format: YYYY-MM/
    Combined,
}

/// File type for classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// Photo files (jpg, png, etc.)
    Photos,
    /// Video files (mp4, mov, etc.)
    Videos,
    /// RAW image files (arw, cr2, etc.) - nested under Photos
    Raw,
}

impl FileType {
    /// Get the folder name for this file type
    pub fn folder_name(&self) -> &'static str {
        match self {
            FileType::Photos => "Photos",
            FileType::Videos => "Videos",
            FileType::Raw => "Raw",
        }
    }
}

/// File operation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum FileOperation {
    /// Copy files to destination
    #[default]
    Copy,
    /// Move files to destination
    Move,
    /// Create symbolic links (Unix-like systems only)
    Symlink,
    /// Create hard links
    Hardlink,
}

/// Configuration for the gallery sorter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Input directories to scan for media files
    pub input_dirs: Vec<PathBuf>,

    /// Output directory for organized files
    pub output_dir: PathBuf,

    /// Directories to exclude from scanning (can be absolute paths or folder names)
    #[serde(default)]
    pub exclude_dirs: Vec<PathBuf>,

    /// Processing mode
    pub processing_mode: ProcessingMode,

    /// Classification rule
    pub classification: ClassificationRule,

    /// Month format for year-month classification
    pub month_format: MonthFormat,

    /// Classify by file type (adds Photos/Videos subdirectory, RAW files nested under Photos/Raw)
    #[serde(default)]
    pub classify_by_type: bool,

    /// File operation mode
    pub operation: FileOperation,

    /// Enable file deduplication
    pub deduplicate: bool,

    /// State file path for incremental processing
    pub state_file: Option<PathBuf>,

    /// Number of threads for parallel processing (0 = auto)
    pub threads: usize,

    /// Large file threshold in bytes (files larger than this use sampled hashing)
    pub large_file_threshold: u64,

    /// Dry run mode - don't actually move/copy files
    pub dry_run: bool,

    /// Verbose output
    pub verbose: bool,

    /// Supported image extensions
    pub image_extensions: Vec<String>,

    /// Supported video extensions
    pub video_extensions: Vec<String>,

    /// Supported RAW extensions
    pub raw_extensions: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input_dirs: vec![],
            output_dir: PathBuf::from("output"),
            exclude_dirs: vec![],
            processing_mode: ProcessingMode::default(),
            classification: ClassificationRule::default(),
            month_format: MonthFormat::default(),
            classify_by_type: false,
            operation: FileOperation::default(),
            deduplicate: true,
            state_file: None,
            threads: 0, // Auto-detect
            large_file_threshold: 100 * 1024 * 1024, // 100MB
            dry_run: false,
            verbose: false,
            image_extensions: vec![
                "jpg".into(), "jpeg".into(), "png".into(), "gif".into(),
                "bmp".into(), "webp".into(), "heic".into(), "heif".into(),
                "avif".into(), "tiff".into(), "tif".into(),
            ],
            video_extensions: vec![
                "mp4".into(), "mov".into(), "avi".into(), "mkv".into(),
                "wmv".into(), "flv".into(), "m4v".into(), "3gp".into(),
            ],
            raw_extensions: vec![
                "raw".into(), "arw".into(), "cr2".into(), "cr3".into(),
                "nef".into(), "orf".into(), "rw2".into(), "dng".into(),
                "raf".into(), "srw".into(), "pef".into(),
            ],
        }
    }
}

impl Config {
    /// Check if a file extension is a supported image format
    pub fn is_image(&self, ext: &str) -> bool {
        let ext_lower = ext.to_lowercase();
        self.image_extensions.iter().any(|e| e == &ext_lower)
            || self.raw_extensions.iter().any(|e| e == &ext_lower)
    }

    /// Check if a file extension is a supported video format
    pub fn is_video(&self, ext: &str) -> bool {
        let ext_lower = ext.to_lowercase();
        self.video_extensions.iter().any(|e| e == &ext_lower)
    }

    /// Check if a file extension is a supported RAW format
    pub fn is_raw(&self, ext: &str) -> bool {
        let ext_lower = ext.to_lowercase();
        self.raw_extensions.iter().any(|e| e == &ext_lower)
    }

    /// Check if a file extension is a HEIF-based format (HEIC, HEIF, AVIF)
    /// These formats use the same container and EXIF extraction method
    pub fn is_heif_container(&self, ext: &str) -> bool {
        let ext_lower = ext.to_lowercase();
        ext_lower == "heic" || ext_lower == "heif" || ext_lower == "avif"
    }

    /// Check if a file extension is a HEIF format (legacy method)
    pub fn is_heif(&self, ext: &str) -> bool {
        let ext_lower = ext.to_lowercase();
        ext_lower == "heic" || ext_lower == "heif"
    }

    /// Check if a file extension is AVIF format
    pub fn is_avif(&self, ext: &str) -> bool {
        ext.to_lowercase() == "avif"
    }

    /// Check if a file extension is supported
    pub fn is_supported(&self, ext: &str) -> bool {
        self.is_image(ext) || self.is_video(ext)
    }

    /// Get the file type for a given extension
    pub fn get_file_type(&self, ext: &str) -> Option<FileType> {
        let ext_lower = ext.to_lowercase();
        if self.raw_extensions.iter().any(|e| e == &ext_lower) {
            Some(FileType::Raw)
        } else if self.video_extensions.iter().any(|e| e == &ext_lower) {
            Some(FileType::Videos)
        } else if self.image_extensions.iter().any(|e| e == &ext_lower) {
            Some(FileType::Photos)
        } else {
            None
        }
    }

    /// Get state file path, using default if not specified
    pub fn get_state_file(&self) -> PathBuf {
        self.state_file
            .clone()
            .unwrap_or_else(|| self.output_dir.join(".gallery_sorter_state.json"))
    }

    /// Load configuration from a TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).map_err(|e| ConfigError::ReadError {
            path: path.to_path_buf(),
            source: e,
        })?;

        let config: Config = toml::from_str(&content).map_err(|e| ConfigError::ParseError {
            path: path.to_path_buf(),
            source: e,
        })?;

        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let path = path.as_ref();

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::WriteError {
                path: path.to_path_buf(),
                source: e,
            })?;
        }

        let content = toml::to_string_pretty(self).map_err(|e| ConfigError::SerializeError {
            source: e,
        })?;

        fs::write(path, content).map_err(|e| ConfigError::WriteError {
            path: path.to_path_buf(),
            source: e,
        })?;

        Ok(())
    }

    /// Generate a sample configuration file content
    pub fn sample_config() -> String {
        r#"# Gallery Sorter Configuration File
# This file uses TOML format (https://toml.io)

# Input directories to scan for media files
# Can specify multiple directories
input_dirs = [
    "D:/Photos",
    "D:/Videos",
]

# Output directory for organized files
output_dir = "D:/Sorted"

# Directories to exclude from scanning
# Can be absolute paths or folder names (will match any folder with that name)
# Examples:
#   - ".sync" - excludes any folder named ".sync"
#   - "D:/Photos/.thumbnails" - excludes this specific folder
exclude_dirs = [
    ".sync",
    ".thumbnails",
    "@eaDir",
]

# Processing mode: "full", "supplement", or "incremental"
# - full: Process all files, overwrite existing files in target
# - supplement: Skip files that already exist in target
# - incremental: Only process files newer than newest in target (default)
processing_mode = "incremental"

# Classification rule: "none", "year", or "year-month"
# - none: All files go to output directory root
# - year: Organize by year (YYYY/)
# - year-month: Organize by year and month (YYYY/MM/)
classification = "year-month"

# Month format: "nested" or "combined"
# - nested: YYYY/MM/
# - combined: YYYY-MM/
month_format = "nested"

# Classify by file type (adds Photos/Videos subdirectory, RAW files nested under Photos/Raw)
# - false: 2024/01/photo.jpg (default)
# - true: 2024/01/Photos/photo.jpg, 2024/01/Photos/Raw/photo.arw
classify_by_type = false

# File operation: "copy", "move", "symlink", or "hardlink"
operation = "copy"

# Enable file deduplication (skip duplicate files)
deduplicate = true

# Number of threads for parallel processing (0 = auto-detect)
threads = 0

# Large file threshold in bytes (files larger use sampled hashing)
# Default: 100MB = 104857600 bytes
large_file_threshold = 104857600

# Dry run mode - show what would be done without actually doing it
dry_run = false

# Verbose output - show detailed processing information
verbose = false

# Supported file extensions (customize as needed)
# EXIF extraction supported: jpg, jpeg, png, webp, heic, heif, avif, tiff, tif
image_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "heic", "heif", "avif", "tiff", "tif"]
video_extensions = ["mp4", "mov", "avi", "mkv", "wmv", "flv", "m4v", "3gp"]
raw_extensions = ["raw", "arw", "cr2", "cr3", "nef", "orf", "rw2", "dng", "raf", "srw", "pef"]
"#
        .to_string()
    }
}

/// Errors that can occur when loading or saving configuration
#[derive(Debug)]
pub enum ConfigError {
    /// Failed to read configuration file
    ReadError {
        path: PathBuf,
        source: std::io::Error,
    },
    /// Failed to parse configuration file
    ParseError {
        path: PathBuf,
        source: toml::de::Error,
    },
    /// Failed to write configuration file
    WriteError {
        path: PathBuf,
        source: std::io::Error,
    },
    /// Failed to serialize configuration
    SerializeError {
        source: toml::ser::Error,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::ReadError { path, source } => {
                write!(f, "Failed to read config file '{}': {}", path.display(), source)
            }
            ConfigError::ParseError { path, source } => {
                write!(f, "Failed to parse config file '{}': {}", path.display(), source)
            }
            ConfigError::WriteError { path, source } => {
                write!(f, "Failed to write config file '{}': {}", path.display(), source)
            }
            ConfigError::SerializeError { source } => {
                write!(f, "Failed to serialize config: {}", source)
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::ReadError { source, .. } => Some(source),
            ConfigError::ParseError { source, .. } => Some(source),
            ConfigError::WriteError { source, .. } => Some(source),
            ConfigError::SerializeError { source } => Some(source),
        }
    }
}
