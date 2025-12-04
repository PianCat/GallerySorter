//! Incremental processing state management
//!
//! Provides two mechanisms for incremental processing:
//! 1. Full state tracking (JSON) - tracks every processed file
//! 2. Watermark-based (TOML) - tracks only the newest file for fast comparison
//!
//! The watermark approach is more efficient for append-only workflows like
//! photo imports, as it only needs to compare timestamps rather than
//! computing metadata hashes for all files.

use crate::config::{ClassificationRule, MonthFormat};
use crate::error::{Error, Result};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use walkdir::WalkDir;

/// Record of a processed file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedFile {
    /// Original file path
    pub source_path: PathBuf,

    /// Destination file path
    pub dest_path: PathBuf,

    /// File content hash
    pub content_hash: u64,

    /// Metadata hash (size + mtime) for quick change detection
    pub metadata_hash: u64,

    /// Timestamp when the file was processed
    pub processed_at: chrono::DateTime<chrono::Utc>,
}

/// Processing state for incremental operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingState {
    /// Version for state file format compatibility
    version: u32,

    /// Map of source path to processed file record
    processed_files: HashMap<PathBuf, ProcessedFile>,

    /// Map of content hash to destination path (for deduplication)
    hash_to_dest: HashMap<u64, PathBuf>,

    /// Last run timestamp
    last_run: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for ProcessingState {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessingState {
    /// Current state file format version
    const VERSION: u32 = 1;

    /// Create a new empty state
    pub fn new() -> Self {
        Self {
            version: Self::VERSION,
            processed_files: HashMap::new(),
            hash_to_dest: HashMap::new(),
            last_run: None,
        }
    }

    /// Load state from file
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            debug!(?path, "State file does not exist, creating new state");
            return Ok(Self::new());
        }

        let file = File::open(path).map_err(|e| Error::StateFile(format!(
            "Failed to open state file: {}", e
        )))?;
        let reader = BufReader::new(file);

        let state: Self = serde_json::from_reader(reader).map_err(|e| Error::StateFile(format!(
            "Failed to parse state file: {}", e
        )))?;

        if state.version != Self::VERSION {
            warn!(
                state_version = state.version,
                current_version = Self::VERSION,
                "State file version mismatch, starting fresh"
            );
            return Ok(Self::new());
        }

        info!(
            files_tracked = state.processed_files.len(),
            "Loaded processing state"
        );

        Ok(state)
    }

    /// Save state to file
    pub fn save(&mut self, path: &Path) -> Result<()> {
        self.last_run = Some(chrono::Utc::now());

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write to a temporary file first, then rename for atomicity
        let temp_path = path.with_extension("tmp");

        let file = File::create(&temp_path).map_err(|e| Error::StateFile(format!(
            "Failed to create temp state file: {}", e
        )))?;
        let writer = BufWriter::new(file);

        serde_json::to_writer_pretty(writer, self).map_err(|e| Error::StateFile(format!(
            "Failed to write state file: {}", e
        )))?;

        // Atomic rename
        fs::rename(&temp_path, path).map_err(|e| Error::StateFile(format!(
            "Failed to rename temp state file: {}", e
        )))?;

        info!(
            files_tracked = self.processed_files.len(),
            "Saved processing state"
        );

        Ok(())
    }

    /// Check if a file needs processing
    ///
    /// Returns true if:
    /// - File has not been processed before
    /// - File's metadata hash has changed (modified since last processing)
    pub fn needs_processing(&self, path: &Path, metadata_hash: u64) -> bool {
        match self.processed_files.get(path) {
            Some(record) => record.metadata_hash != metadata_hash,
            None => true,
        }
    }

    /// Check if a content hash already exists (duplicate detection)
    pub fn has_content_hash(&self, content_hash: u64) -> Option<&PathBuf> {
        self.hash_to_dest.get(&content_hash)
    }

    /// Record a processed file
    pub fn record_processed(
        &mut self,
        source_path: PathBuf,
        dest_path: PathBuf,
        content_hash: u64,
        metadata_hash: u64,
    ) {
        let record = ProcessedFile {
            source_path: source_path.clone(),
            dest_path: dest_path.clone(),
            content_hash,
            metadata_hash,
            processed_at: chrono::Utc::now(),
        };

        self.processed_files.insert(source_path, record);
        self.hash_to_dest.insert(content_hash, dest_path);
    }

    /// Get the number of tracked files
    pub fn file_count(&self) -> usize {
        self.processed_files.len()
    }

    /// Get last run timestamp
    pub fn last_run(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.last_run
    }

    /// Clear all state
    pub fn clear(&mut self) {
        self.processed_files.clear();
        self.hash_to_dest.clear();
        self.last_run = None;
    }

    /// Remove entries for files that no longer exist at their source paths
    pub fn cleanup_missing(&mut self) {
        let missing: Vec<PathBuf> = self
            .processed_files
            .keys()
            .filter(|p| !p.exists())
            .cloned()
            .collect();

        for path in &missing {
            if let Some(record) = self.processed_files.remove(path) {
                self.hash_to_dest.remove(&record.content_hash);
            }
        }

        if !missing.is_empty() {
            info!(count = missing.len(), "Cleaned up missing file entries");
        }
    }
}

/// Increment Metadata file name
const WATERMARK_FILENAME: &str = ".gallery_sorter_increment_metadata.toml";

/// Serde helper for serializing u64 as hex string (TOML doesn't support u64 > i64::MAX)
mod hex_u64 {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{:016x}", value))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        u64::from_str_radix(&s, 16).map_err(serde::de::Error::custom)
    }
}

/// Incremental watermark for efficient timestamp-based filtering
///
/// This tracks only the newest processed file, enabling quick filtering
/// of source files without computing hashes for every file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalWatermark {
    /// Version for format compatibility
    version: u32,

    /// Relative path to the newest file in output directory
    pub newest_file_path: PathBuf,

    /// Timestamp of the newest file (extracted creation time)
    pub newest_timestamp: NaiveDateTime,

    /// Content hash of the newest file (for verification, stored as hex string)
    #[serde(with = "hex_u64")]
    pub newest_hash: u64,

    /// Classification rule used when organizing
    pub classification: ClassificationRule,

    /// Month format (if year-month classification)
    pub month_format: MonthFormat,

    /// When this watermark was last updated
    pub last_updated: chrono::DateTime<chrono::Utc>,

    /// Total files processed in last run
    pub files_processed: usize,
}

impl IncrementalWatermark {
    /// Current watermark format version
    const VERSION: u32 = 1;

    /// Create a new watermark
    pub fn new(
        newest_file_path: PathBuf,
        newest_timestamp: NaiveDateTime,
        newest_hash: u64,
        classification: ClassificationRule,
        month_format: MonthFormat,
    ) -> Self {
        Self {
            version: Self::VERSION,
            newest_file_path,
            newest_timestamp,
            newest_hash,
            classification,
            month_format,
            last_updated: chrono::Utc::now(),
            files_processed: 0,
        }
    }

    /// Get the watermark file path for an output directory
    pub fn get_path(output_dir: &Path) -> PathBuf {
        output_dir.join(WATERMARK_FILENAME)
    }

    /// Load watermark from output directory
    pub fn load(output_dir: &Path) -> Result<Option<Self>> {
        let path = Self::get_path(output_dir);

        if !path.exists() {
            debug!(?path, "Watermark file does not exist");
            return Ok(None);
        }

        let content = fs::read_to_string(&path).map_err(|e| {
            Error::StateFile(format!("Failed to read watermark file: {}", e))
        })?;

        let watermark: Self = toml::from_str(&content).map_err(|e| {
            Error::StateFile(format!("Failed to parse watermark file: {}", e))
        })?;

        if watermark.version != Self::VERSION {
            warn!(
                watermark_version = watermark.version,
                current_version = Self::VERSION,
                "Watermark version mismatch, will rescan"
            );
            return Ok(None);
        }

        info!(
            newest_file = %watermark.newest_file_path.display(),
            newest_timestamp = %watermark.newest_timestamp,
            "Loaded incremental watermark"
        );

        Ok(Some(watermark))
    }

    /// Save watermark to output directory
    pub fn save(&self, output_dir: &Path) -> Result<()> {
        let path = Self::get_path(output_dir);

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self).map_err(|e| {
            Error::StateFile(format!("Failed to serialize watermark: {}", e))
        })?;

        // Write atomically via temp file
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, &content).map_err(|e| {
            Error::StateFile(format!("Failed to write watermark file: {}", e))
        })?;

        fs::rename(&temp_path, &path).map_err(|e| {
            Error::StateFile(format!("Failed to rename watermark file: {}", e))
        })?;

        info!(
            newest_file = %self.newest_file_path.display(),
            newest_timestamp = %self.newest_timestamp,
            "Saved incremental watermark"
        );

        Ok(())
    }

    /// Check if a source file's timestamp is newer than the watermark
    ///
    /// Returns true if the file should be processed (is newer than watermark)
    pub fn is_newer(&self, timestamp: &NaiveDateTime) -> bool {
        *timestamp > self.newest_timestamp
    }

    /// Scan output directory to find the newest file based on directory structure
    ///
    /// This is used when the watermark file doesn't exist but we need to
    /// determine the cutoff timestamp by analyzing existing files.
    pub fn scan_output_directory(
        output_dir: &Path,
        classification: ClassificationRule,
        month_format: MonthFormat,
        supported_extensions: &[String],
    ) -> Result<Option<Self>> {
        if !output_dir.exists() {
            debug!(?output_dir, "Output directory does not exist");
            return Ok(None);
        }

        info!(?output_dir, "Scanning output directory to find newest file");

        let mut newest: Option<(PathBuf, NaiveDateTime)> = None;

        for entry in WalkDir::new(output_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip non-files
            if !path.is_file() {
                continue;
            }

            // Skip hidden files and the watermark file itself
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') {
                    continue;
                }
            }

            // Check if supported extension
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase());

            if let Some(ext) = ext {
                if !supported_extensions.iter().any(|e| e == &ext) {
                    continue;
                }
            } else {
                continue;
            }

            // Try to extract timestamp from directory structure
            if let Some(timestamp) = Self::extract_timestamp_from_path(
                path,
                output_dir,
                classification,
                month_format,
            ) {
                match &newest {
                    Some((_, newest_ts)) if timestamp > *newest_ts => {
                        newest = Some((path.to_path_buf(), timestamp));
                    }
                    None => {
                        newest = Some((path.to_path_buf(), timestamp));
                    }
                    _ => {}
                }
            }
        }

        match newest {
            Some((path, timestamp)) => {
                // Compute hash for verification
                let hash = crate::hash::compute_file_hash(&path, 100 * 1024 * 1024)
                    .unwrap_or(0);

                let relative_path = path
                    .strip_prefix(output_dir)
                    .unwrap_or(&path)
                    .to_path_buf();

                info!(
                    newest_file = %relative_path.display(),
                    timestamp = %timestamp,
                    "Found newest file in output directory"
                );

                Ok(Some(Self::new(
                    relative_path,
                    timestamp,
                    hash,
                    classification,
                    month_format,
                )))
            }
            None => {
                debug!("No files found in output directory");
                Ok(None)
            }
        }
    }

    /// Extract timestamp from file path based on directory structure
    ///
    /// For YearMonth classification with Nested format: output/2024/06/file.jpg -> 2024-06-01
    /// For YearMonth classification with Combined format: output/2024-06/file.jpg -> 2024-06-01
    /// For Year classification: output/2024/file.jpg -> 2024-01-01
    fn extract_timestamp_from_path(
        file_path: &Path,
        output_dir: &Path,
        classification: ClassificationRule,
        month_format: MonthFormat,
    ) -> Option<NaiveDateTime> {
        let relative = file_path.strip_prefix(output_dir).ok()?;
        let components: Vec<_> = relative.components().collect();

        match classification {
            ClassificationRule::None => {
                // No directory structure, use file modification time
                fs::metadata(file_path)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        let dt: chrono::DateTime<chrono::Utc> = t.into();
                        dt.naive_utc()
                    })
            }
            ClassificationRule::Year => {
                // Expect: YYYY/filename
                if components.len() >= 2 {
                    let year_str = components[0].as_os_str().to_str()?;
                    let year: i32 = year_str.parse().ok()?;
                    NaiveDateTime::parse_from_str(
                        &format!("{}-01-01 00:00:00", year),
                        "%Y-%m-%d %H:%M:%S",
                    )
                    .ok()
                } else {
                    None
                }
            }
            ClassificationRule::YearMonth => {
                match month_format {
                    MonthFormat::Nested => {
                        // Expect: YYYY/MM/filename
                        if components.len() >= 3 {
                            let year_str = components[0].as_os_str().to_str()?;
                            let month_str = components[1].as_os_str().to_str()?;
                            let year: i32 = year_str.parse().ok()?;
                            let month: u32 = month_str.parse().ok()?;

                            if !(1..=12).contains(&month) {
                                return None;
                            }

                            NaiveDateTime::parse_from_str(
                                &format!("{}-{:02}-01 00:00:00", year, month),
                                "%Y-%m-%d %H:%M:%S",
                            )
                            .ok()
                        } else {
                            None
                        }
                    }
                    MonthFormat::Combined => {
                        // Expect: YYYY-MM/filename
                        if components.len() >= 2 {
                            let dir_name = components[0].as_os_str().to_str()?;
                            let parts: Vec<_> = dir_name.split('-').collect();
                            if parts.len() == 2 {
                                let year: i32 = parts[0].parse().ok()?;
                                let month: u32 = parts[1].parse().ok()?;

                                if !(1..=12).contains(&month) {
                                    return None;
                                }

                                NaiveDateTime::parse_from_str(
                                    &format!("{}-{:02}-01 00:00:00", year, month),
                                    "%Y-%m-%d %H:%M:%S",
                                )
                                .ok()
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                }
            }
        }
    }

    /// Update watermark with a new file if it's newer than the current one
    pub fn update_if_newer(&mut self, file_path: PathBuf, timestamp: NaiveDateTime, hash: u64) {
        if timestamp > self.newest_timestamp {
            self.newest_file_path = file_path;
            self.newest_timestamp = timestamp;
            self.newest_hash = hash;
            self.last_updated = chrono::Utc::now();
        }
    }

    /// Set the files processed count
    pub fn set_files_processed(&mut self, count: usize) {
        self.files_processed = count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;
    use tempfile::tempdir;

    #[test]
    fn test_new_state() {
        let state = ProcessingState::new();
        assert_eq!(state.file_count(), 0);
        assert!(state.last_run().is_none());
    }

    #[test]
    fn test_record_and_query() {
        let mut state = ProcessingState::new();

        let source = PathBuf::from("/source/file.jpg");
        let dest = PathBuf::from("/dest/2024/01/file.jpg");
        let content_hash = 12345u64;
        let metadata_hash = 67890u64;

        assert!(state.needs_processing(&source, metadata_hash));
        assert!(state.has_content_hash(content_hash).is_none());

        state.record_processed(source.clone(), dest.clone(), content_hash, metadata_hash);

        assert!(!state.needs_processing(&source, metadata_hash));
        assert!(state.needs_processing(&source, 99999)); // Different metadata hash
        assert_eq!(state.has_content_hash(content_hash), Some(&dest));
        assert_eq!(state.file_count(), 1);
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("state.json");

        let mut state = ProcessingState::new();
        let source = PathBuf::from("/source/file.jpg");
        let dest = PathBuf::from("/dest/2024/01/file.jpg");
        state.record_processed(source.clone(), dest.clone(), 12345, 67890);

        state.save(&state_path).unwrap();

        let loaded = ProcessingState::load(&state_path).unwrap();
        assert_eq!(loaded.file_count(), 1);
        assert!(!loaded.needs_processing(&source, 67890));
    }

    #[test]
    fn test_watermark_new() {
        let timestamp = NaiveDateTime::parse_from_str("2024-06-15 14:30:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let wm = IncrementalWatermark::new(
            PathBuf::from("2024/06/photo.jpg"),
            timestamp,
            12345,
            ClassificationRule::YearMonth,
            MonthFormat::Nested,
        );

        assert_eq!(wm.newest_timestamp, timestamp);
        assert_eq!(wm.newest_hash, 12345);
        assert_eq!(wm.classification, ClassificationRule::YearMonth);
    }

    #[test]
    fn test_watermark_is_newer() {
        let timestamp = NaiveDateTime::parse_from_str("2024-06-15 14:30:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let wm = IncrementalWatermark::new(
            PathBuf::from("photo.jpg"),
            timestamp,
            12345,
            ClassificationRule::None,
            MonthFormat::Nested,
        );

        // Older timestamp - should not be newer
        let older = NaiveDateTime::parse_from_str("2024-05-01 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        assert!(!wm.is_newer(&older));

        // Same timestamp - should not be newer
        assert!(!wm.is_newer(&timestamp));

        // Newer timestamp - should be newer
        let newer = NaiveDateTime::parse_from_str("2024-07-01 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        assert!(wm.is_newer(&newer));
    }

    #[test]
    fn test_watermark_update_if_newer() {
        let timestamp1 = NaiveDateTime::parse_from_str("2024-06-15 14:30:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let mut wm = IncrementalWatermark::new(
            PathBuf::from("2024/06/photo1.jpg"),
            timestamp1,
            12345,
            ClassificationRule::YearMonth,
            MonthFormat::Nested,
        );

        // Try to update with older timestamp - should not change
        let older = NaiveDateTime::parse_from_str("2024-05-01 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        wm.update_if_newer(PathBuf::from("2024/05/old.jpg"), older, 99999);
        assert_eq!(wm.newest_timestamp, timestamp1);
        assert_eq!(wm.newest_hash, 12345);

        // Update with newer timestamp - should change
        let newer = NaiveDateTime::parse_from_str("2024-07-20 18:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        wm.update_if_newer(PathBuf::from("2024/07/new.jpg"), newer, 67890);
        assert_eq!(wm.newest_timestamp, newer);
        assert_eq!(wm.newest_hash, 67890);
        assert_eq!(wm.newest_file_path, PathBuf::from("2024/07/new.jpg"));
    }

    #[test]
    fn test_watermark_save_and_load() {
        let dir = tempdir().unwrap();
        let output_dir = dir.path();

        let timestamp = NaiveDateTime::parse_from_str("2024-06-15 14:30:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let wm = IncrementalWatermark::new(
            PathBuf::from("2024/06/photo.jpg"),
            timestamp,
            12345,
            ClassificationRule::YearMonth,
            MonthFormat::Nested,
        );

        // Save watermark
        wm.save(output_dir).unwrap();

        // Check file exists
        let wm_path = IncrementalWatermark::get_path(output_dir);
        assert!(wm_path.exists());

        // Load watermark
        let loaded = IncrementalWatermark::load(output_dir).unwrap();
        assert!(loaded.is_some());

        let loaded = loaded.unwrap();
        assert_eq!(loaded.newest_timestamp, timestamp);
        assert_eq!(loaded.newest_hash, 12345);
        assert_eq!(loaded.classification, ClassificationRule::YearMonth);
        assert_eq!(loaded.month_format, MonthFormat::Nested);
    }

    #[test]
    fn test_watermark_extract_timestamp_nested() {
        let output_dir = PathBuf::from("/output");

        // Test nested format: /output/2024/06/photo.jpg
        let file_path = PathBuf::from("/output/2024/06/photo.jpg");
        let ts = IncrementalWatermark::extract_timestamp_from_path(
            &file_path,
            &output_dir,
            ClassificationRule::YearMonth,
            MonthFormat::Nested,
        );

        assert!(ts.is_some());
        let ts = ts.unwrap();
        assert_eq!(ts.year(), 2024);
        assert_eq!(ts.month(), 6);
    }

    #[test]
    fn test_watermark_extract_timestamp_combined() {
        let output_dir = PathBuf::from("/output");

        // Test combined format: /output/2024-06/photo.jpg
        let file_path = PathBuf::from("/output/2024-06/photo.jpg");
        let ts = IncrementalWatermark::extract_timestamp_from_path(
            &file_path,
            &output_dir,
            ClassificationRule::YearMonth,
            MonthFormat::Combined,
        );

        assert!(ts.is_some());
        let ts = ts.unwrap();
        assert_eq!(ts.year(), 2024);
        assert_eq!(ts.month(), 6);
    }

    #[test]
    fn test_watermark_extract_timestamp_year_only() {
        let output_dir = PathBuf::from("/output");

        // Test year only format: /output/2024/photo.jpg
        let file_path = PathBuf::from("/output/2024/photo.jpg");
        let ts = IncrementalWatermark::extract_timestamp_from_path(
            &file_path,
            &output_dir,
            ClassificationRule::Year,
            MonthFormat::Nested,
        );

        assert!(ts.is_some());
        let ts = ts.unwrap();
        assert_eq!(ts.year(), 2024);
        assert_eq!(ts.month(), 1); // Defaults to January
    }
}
