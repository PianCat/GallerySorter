//! Full processing state (JSON) tracking
//!
//! Tracks every processed file so incremental runs can detect files that
//! have already been handled or changed since the last run.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

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

        let file = File::open(path)
            .map_err(|e| Error::StateFile(format!("Failed to open state file: {}", e)))?;
        let reader = BufReader::new(file);

        let state: Self = serde_json::from_reader(reader)
            .map_err(|e| Error::StateFile(format!("Failed to parse state file: {}", e)))?;

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

        let file = File::create(&temp_path)
            .map_err(|e| Error::StateFile(format!("Failed to create temp state file: {}", e)))?;
        let writer = BufWriter::new(file);

        serde_json::to_writer_pretty(writer, self)
            .map_err(|e| Error::StateFile(format!("Failed to write state file: {}", e)))?;

        // Atomic rename
        fs::rename(&temp_path, path)
            .map_err(|e| Error::StateFile(format!("Failed to rename temp state file: {}", e)))?;

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

#[cfg(test)]
mod tests {
    use super::*;
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
}
