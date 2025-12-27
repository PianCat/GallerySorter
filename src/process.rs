//! Main file processor with Rayon parallel processing
//!
//! Handles the core logic of:
//! - Scanning input directories
//! - Extracting timestamps
//! - Computing hashes for deduplication
//! - Organizing files to output directory

use crate::config::{
    ClassificationRule, Config, FileOperation, FileType, MonthFormat, ProcessingMode,
};
use crate::error::{Error, Result};
use crate::hash::{compute_file_hash, compute_metadata_hash};
use crate::state::{IncrementalWatermark, ProcessingState};
use crate::time::{ExtractedTime, extract_time};
use chrono::{Datelike, NaiveDateTime};

use rayon::prelude::*;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tracing::{Level, debug, error, info, span, warn};
use walkdir::WalkDir;

/// Patterns that indicate a file is a copy/duplicate (lower priority)
static COPY_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

/// Initialize COPY_PATTERNS on first use
fn get_copy_patterns() -> &'static Vec<Regex> {
    COPY_PATTERNS.get_or_init(|| {
        vec![
            Regex::new(r" - 副本").unwrap(),
            Regex::new(r"_\d+$").unwrap(),
            Regex::new(r" \d+$").unwrap(),
            Regex::new(r"\(\d+\)$").unwrap(),
            Regex::new(r"(?i)[- _]copy").unwrap(),
            Regex::new(r"(?i)[- _]копия").unwrap(),
        ]
    })
}

/// Calculate filename priority score (lower = better/cleaner filename)
/// Primary factor: filename length (shorter = better, as originals don't have copy suffixes)
/// Secondary factor: presence of known copy indicators adds penalty
fn filename_priority_score(path: &Path) -> u32 {
    let filename = match path.file_stem().and_then(|s| s.to_str()) {
        Some(name) => name,
        None => return u32::MAX, // Invalid filename gets lowest priority
    };

    // Primary: filename length (shorter = better)
    let length_score = filename.len() as u32;

    // Secondary: penalty for copy indicators (to break ties)
    let mut copy_penalty = 0u32;
    for pattern in get_copy_patterns().iter() {
        if pattern.is_match(filename) {
            copy_penalty += 1000;
        }
    }

    length_score + copy_penalty
}

/// Result of processing a single file
#[derive(Debug, Clone)]
pub struct FileResult {
    /// Source file path
    pub source: PathBuf,
    /// Destination file path (if successful)
    pub destination: Option<PathBuf>,
    /// Extracted time information
    pub time_info: Option<ExtractedTime>,
    /// Processing status
    pub status: ProcessingStatus,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Status of file processing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingStatus {
    /// File was successfully processed
    Success,
    /// File was skipped (already processed)
    Skipped,
    /// File was skipped as duplicate
    Duplicate,
    /// Processing failed
    Failed,
    /// Dry run - would have processed
    DryRun,
}

/// Processing statistics
#[derive(Debug, Default)]
pub struct ProcessingStats {
    pub total_files: AtomicUsize,
    pub processed: AtomicUsize,
    pub skipped: AtomicUsize,
    pub duplicates: AtomicUsize,
    pub failed: AtomicUsize,
}

impl Clone for ProcessingStats {
    fn clone(&self) -> Self {
        Self {
            total_files: AtomicUsize::new(self.total_files.load(Ordering::Relaxed)),
            processed: AtomicUsize::new(self.processed.load(Ordering::Relaxed)),
            skipped: AtomicUsize::new(self.skipped.load(Ordering::Relaxed)),
            duplicates: AtomicUsize::new(self.duplicates.load(Ordering::Relaxed)),
            failed: AtomicUsize::new(self.failed.load(Ordering::Relaxed)),
        }
    }
}

impl ProcessingStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn summary(&self) -> String {
        format!(
            "Total: {}, Processed: {}, Skipped: {}, Duplicates: {}, Failed: {}",
            self.total_files.load(Ordering::Relaxed),
            self.processed.load(Ordering::Relaxed),
            self.skipped.load(Ordering::Relaxed),
            self.duplicates.load(Ordering::Relaxed),
            self.failed.load(Ordering::Relaxed)
        )
    }
}

/// Main processor for organizing media files
pub struct Processor {
    config: Config,
    state: ProcessingState,
    watermark: Option<IncrementalWatermark>,
    stats: Arc<ProcessingStats>,
}

impl Processor {
    /// Create a new processor with the given configuration
    pub fn new(config: Config) -> Result<Self> {
        // Configure Rayon thread pool
        if config.threads > 0 {
            rayon::ThreadPoolBuilder::new()
                .num_threads(config.threads)
                .build_global()
                .ok(); // Ignore if already initialized
        }

        // Load existing state for incremental processing
        let state = if config.processing_mode == ProcessingMode::Incremental {
            ProcessingState::load(&config.get_state_file())?
        } else {
            ProcessingState::new()
        };

        // Helper to collect all supported extensions
        let get_extensions = || -> Vec<String> {
            config
                .image_extensions
                .iter()
                .chain(config.video_extensions.iter())
                .chain(config.raw_extensions.iter())
                .cloned()
                .collect()
        };

        // Load or create watermark for incremental mode
        let watermark = if config.processing_mode == ProcessingMode::Incremental {
            // Try to load existing watermark
            match IncrementalWatermark::load(&config.output_dir)? {
                Some(wm) => {
                    // Check if classification settings match
                    if wm.classification != config.classification
                        || wm.month_format != config.month_format
                    {
                        warn!(
                            "Watermark classification settings don't match current config, rescanning"
                        );
                        IncrementalWatermark::scan_output_directory(
                            &config.output_dir,
                            config.classification,
                            config.month_format,
                            &get_extensions(),
                        )?
                    } else {
                        // Verify the newest file still exists
                        let newest_file_path = config.output_dir.join(&wm.newest_file_path);
                        if !newest_file_path.exists() {
                            warn!(
                                newest_file = %wm.newest_file_path.display(),
                                "Watermark references non-existent file, rescanning output directory"
                            );
                            IncrementalWatermark::scan_output_directory(
                                &config.output_dir,
                                config.classification,
                                config.month_format,
                                &get_extensions(),
                            )?
                        } else {
                            Some(wm)
                        }
                    }
                }
                None => {
                    // No watermark file, scan directory to find newest file
                    IncrementalWatermark::scan_output_directory(
                        &config.output_dir,
                        config.classification,
                        config.month_format,
                        &get_extensions(),
                    )?
                }
            }
        } else {
            None
        };

        Ok(Self {
            config,
            state,
            watermark,
            stats: Arc::new(ProcessingStats::new()),
        })
    }

    /// Get the total number of files that would be processed
    /// This can be called before run() to get the file count for progress tracking
    pub fn total_files_count(&self) -> Result<usize> {
        let files = self.collect_files()?;
        Ok(files.len())
    }

    /// Run the processing pipeline
    pub fn run(&mut self) -> Result<Vec<FileResult>> {
        let _span = span!(Level::INFO, "processor_run").entered();

        // Collect all files to process
        info!("Scanning input directories...");
        let files = self.collect_files()?;
        info!(count = files.len(), "Found media files");

        if files.is_empty() {
            info!("No files to process");
            return Ok(Vec::new());
        }

        // Update stats
        self.stats.total_files.store(files.len(), Ordering::Relaxed);

        // Create output directory
        if !self.config.dry_run {
            fs::create_dir_all(&self.config.output_dir)?;
        }

        let config = Arc::new(self.config.clone());

        // Incremental mode: Filter files by timestamp using watermark
        // This is done BEFORE computing hashes to minimize disk I/O
        let (files, skipped_by_watermark) = if config.processing_mode == ProcessingMode::Incremental
        {
            if let Some(ref watermark) = self.watermark {
                info!(
                    watermark_timestamp = %watermark.newest_timestamp,
                    "Filtering files by watermark timestamp (only processing newer files)"
                );

                let mut newer_files = Vec::new();
                let mut skipped_count = 0usize;

                for file_path in files {
                    // Extract timestamp for comparison
                    match extract_time(&file_path, &config) {
                        Ok(time_info) => {
                            if watermark.is_newer(&time_info.timestamp) {
                                newer_files.push(file_path);
                            } else {
                                debug!(?file_path, "Skipping file older than watermark");
                                skipped_count += 1;
                            }
                        }
                        Err(_) => {
                            // Can't determine timestamp, include for processing
                            newer_files.push(file_path);
                        }
                    }
                }

                info!(
                    total = newer_files.len() + skipped_count,
                    newer = newer_files.len(),
                    skipped = skipped_count,
                    "Filtered files by watermark timestamp"
                );

                (newer_files, skipped_count)
            } else {
                // No watermark (first run or empty output), process all files
                info!("No watermark found - processing all files (first run behavior)");
                (files, 0)
            }
        } else {
            (files, 0)
        };

        // Update skipped count
        self.stats
            .skipped
            .fetch_add(skipped_by_watermark, Ordering::Relaxed);

        if files.is_empty() {
            info!("No new files to process (all files are older than watermark)");
            return Ok(Vec::new());
        }

        // Phase 1: Compute hashes for all files in parallel to determine duplicates
        info!("Computing file hashes for deduplication...");
        let file_hashes: Vec<(PathBuf, Option<u64>)> = if config.deduplicate {
            files
                .par_iter()
                .map(|path| {
                    let hash = compute_file_hash(path, config.large_file_threshold).ok();
                    (path.clone(), hash)
                })
                .collect()
        } else {
            files.iter().map(|p| (p.clone(), None)).collect()
        };

        // Phase 2: Select best file for each unique hash (cleanest filename wins)
        // Group files by hash
        let mut hash_groups: HashMap<u64, Vec<PathBuf>> = HashMap::new();
        let mut no_hash_files: Vec<PathBuf> = Vec::new();

        for (path, hash) in &file_hashes {
            if let Some(h) = hash {
                hash_groups.entry(*h).or_default().push(path.clone());
            } else {
                no_hash_files.push(path.clone());
            }
        }

        // Select the best file from each group (files are already sorted by priority)
        let mut files_to_process: HashSet<PathBuf> = HashSet::new();
        let mut hash_to_best_file: HashMap<u64, PathBuf> = HashMap::new();

        for (hash, mut group) in hash_groups {
            // Sort by priority score (lowest = best)
            group.sort_by_cached_key(|p| filename_priority_score(p));
            let best = group.remove(0);
            hash_to_best_file.insert(hash, best.clone());
            files_to_process.insert(best);
        }

        // All files without hash should be processed
        for path in no_hash_files {
            files_to_process.insert(path);
        }

        let duplicate_count = files.len() - files_to_process.len();
        info!(
            "Selected {} files to process ({} duplicates will be skipped)",
            files_to_process.len(),
            duplicate_count
        );

        // For Supplement mode: scan target directory for existing file hashes
        let existing_hashes: HashSet<u64> = if config.processing_mode == ProcessingMode::Supplement
        {
            info!("Scanning target directory for existing files...");
            self.scan_target_hashes()?
        } else {
            HashSet::new()
        };

        if config.processing_mode == ProcessingMode::Supplement && !existing_hashes.is_empty() {
            info!(
                "Found {} existing files in target directory",
                existing_hashes.len()
            );
        }

        // Phase 3: Process files
        info!("Processing files...");

        // Wrap state in Arc<Mutex> for shared access
        let state = Arc::new(Mutex::new(std::mem::take(&mut self.state)));
        let stats = Arc::new(ProcessingStats::new());
        stats.total_files.store(
            self.stats.total_files.load(Ordering::Relaxed),
            Ordering::Relaxed,
        );

        // Map to track hash -> destination for duplicate reporting
        let hash_to_dest: Arc<Mutex<HashMap<u64, PathBuf>>> = Arc::new(Mutex::new(HashMap::new()));

        // Convert file_hashes to a lookup map
        let file_hash_map: HashMap<PathBuf, Option<u64>> = file_hashes.into_iter().collect();
        let hash_to_best_file = Arc::new(hash_to_best_file);
        let existing_hashes = Arc::new(existing_hashes);

        // Process all files, marking duplicates appropriately
        let results: Vec<FileResult> = files
            .par_iter()
            .map(|file_path| {
                let _file_span = span!(Level::DEBUG, "process_file", ?file_path).entered();

                // Check if this is a duplicate that should be skipped
                if !files_to_process.contains(file_path) {
                    // Find the hash for this file to get the kept file's destination
                    if let Some(Some(hash)) = file_hash_map.get(file_path) {
                        // Get destination from already-processed best file, or report the best file path
                        let dest = {
                            let dest_map = hash_to_dest.lock().unwrap();
                            dest_map.get(hash).cloned()
                        }
                        .or_else(|| hash_to_best_file.get(hash).cloned());

                        debug!(
                            ?file_path,
                            ?dest,
                            "Skipping duplicate file (inferior filename)"
                        );
                        stats.duplicates.fetch_add(1, Ordering::Relaxed);
                        return FileResult {
                            source: file_path.clone(),
                            destination: dest,
                            time_info: None,
                            status: ProcessingStatus::Duplicate,
                            error: None,
                        };
                    }
                }

                process_single_file(
                    file_path,
                    &config,
                    &state,
                    &stats,
                    &hash_to_dest,
                    &file_hash_map,
                    &existing_hashes,
                )
            })
            .collect();

        // Restore state from Arc<Mutex>
        self.state = Arc::try_unwrap(state)
            .expect("All references should be dropped")
            .into_inner()
            .unwrap();

        // Update stats (add to existing counts to preserve watermark-filtered files count)
        self.stats
            .processed
            .fetch_add(stats.processed.load(Ordering::Relaxed), Ordering::Relaxed);
        self.stats
            .skipped
            .fetch_add(stats.skipped.load(Ordering::Relaxed), Ordering::Relaxed);
        self.stats
            .duplicates
            .fetch_add(stats.duplicates.load(Ordering::Relaxed), Ordering::Relaxed);
        self.stats
            .failed
            .fetch_add(stats.failed.load(Ordering::Relaxed), Ordering::Relaxed);

        // Save state if incremental processing is enabled
        if self.config.processing_mode == ProcessingMode::Incremental && !self.config.dry_run {
            self.state.save(&self.config.get_state_file())?;

            // Update watermark with newest processed file
            self.update_watermark(&results)?;
        }

        // Log summary
        info!("{}", self.stats.summary());

        Ok(results)
    }

    /// Collect all media files from input directories
    /// Files are sorted by filename priority score (cleanest filenames first)
    /// to ensure proper duplicate retention strategy
    fn collect_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for input_dir in &self.config.input_dirs {
            if !input_dir.exists() {
                warn!(?input_dir, "Input directory does not exist, skipping");
                continue;
            }

            for entry in WalkDir::new(input_dir)
                .follow_links(true)
                .into_iter()
                .filter_entry(|e| !self.is_excluded_dir(e.path()))
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.is_file()
                    && let Some(ext) = path.extension().and_then(|e| e.to_str())
                    && self.config.is_supported(ext)
                {
                    files.push(path.to_path_buf());
                }
            }
        }

        // Sort files by priority score (lowest score = cleanest filename = processed first)
        // This ensures that when duplicates are detected, the cleanest filename is kept
        files.sort_by_cached_key(|path| filename_priority_score(path));

        debug!(
            "Sorted {} files by filename priority (cleanest first)",
            files.len()
        );

        Ok(files)
    }

    /// Check if a path should be excluded based on exclude_dirs configuration
    fn is_excluded_dir(&self, path: &Path) -> bool {
        if self.config.exclude_dirs.is_empty() {
            return false;
        }

        for exclude in &self.config.exclude_dirs {
            // Check if it's an absolute path match
            if exclude.is_absolute() {
                if path.starts_with(exclude) {
                    debug!(?path, ?exclude, "Excluding directory (absolute path match)");
                    return true;
                }
            } else {
                // Check if any component of the path matches the exclude pattern
                if let Some(exclude_name) = exclude.file_name() {
                    for component in path.components() {
                        if let std::path::Component::Normal(name) = component
                            && name == exclude_name
                        {
                            debug!(?path, ?exclude, "Excluding directory (folder name match)");
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Scan target directory for existing file hashes (for Supplement mode)
    fn scan_target_hashes(&self) -> Result<HashSet<u64>> {
        let mut hashes = HashSet::new();

        if !self.config.output_dir.exists() {
            return Ok(hashes);
        }

        let files: Vec<PathBuf> = WalkDir::new(&self.config.output_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| self.config.is_supported(ext))
                    .unwrap_or(false)
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        // Compute hashes in parallel
        let computed_hashes: Vec<Option<u64>> = files
            .par_iter()
            .map(|path| compute_file_hash(path, self.config.large_file_threshold).ok())
            .collect();

        for hash in computed_hashes.into_iter().flatten() {
            hashes.insert(hash);
        }

        Ok(hashes)
    }

    /// Update watermark with the newest successfully processed file
    fn update_watermark(&mut self, results: &[FileResult]) -> Result<()> {
        // Find the newest successfully processed file
        let mut newest: Option<(PathBuf, NaiveDateTime, u64)> = None;

        for result in results {
            // Only consider successfully processed files
            if result.status != ProcessingStatus::Success
                && result.status != ProcessingStatus::DryRun
            {
                continue;
            }

            if let (Some(time_info), Some(dest)) = (&result.time_info, &result.destination) {
                let is_newer = match &newest {
                    Some((_, ts, _)) => time_info.timestamp > *ts,
                    None => true,
                };

                if is_newer {
                    // Get relative path
                    let relative_path = dest
                        .strip_prefix(&self.config.output_dir)
                        .unwrap_or(dest)
                        .to_path_buf();

                    // Compute hash if needed
                    let hash =
                        compute_file_hash(dest, self.config.large_file_threshold).unwrap_or(0);

                    newest = Some((relative_path, time_info.timestamp, hash));
                }
            }
        }

        if let Some((path, timestamp, hash)) = newest {
            // Update or create watermark
            match &mut self.watermark {
                Some(wm) => {
                    wm.update_if_newer(path, timestamp, hash);
                    wm.set_files_processed(self.stats.processed.load(Ordering::Relaxed));
                }
                None => {
                    let mut wm = IncrementalWatermark::new(
                        path,
                        timestamp,
                        hash,
                        self.config.classification,
                        self.config.month_format,
                    );
                    wm.set_files_processed(self.stats.processed.load(Ordering::Relaxed));
                    self.watermark = Some(wm);
                }
            }

            // Save watermark to disk
            if let Some(ref wm) = self.watermark {
                wm.save(&self.config.output_dir)?;
            }
        }

        Ok(())
    }

    /// Get processing statistics reference
    pub fn stats(&self) -> &ProcessingStats {
        &self.stats
    }

    /// Get a clone of the internal stats Arc for shared access
    pub fn stats_arc(&self) -> Arc<ProcessingStats> {
        self.stats.clone()
    }
}

/// Process a single file (standalone function for parallel processing)
fn process_single_file(
    path: &Path,
    config: &Arc<Config>,
    state: &Arc<Mutex<ProcessingState>>,
    stats: &Arc<ProcessingStats>,
    hash_to_dest: &Arc<Mutex<HashMap<u64, PathBuf>>>,
    file_hash_map: &HashMap<PathBuf, Option<u64>>,
    existing_hashes: &Arc<HashSet<u64>>,
) -> FileResult {
    // Get content hash from pre-computed map (needed for Supplement mode check)
    let content_hash = file_hash_map.get(&path.to_path_buf()).and_then(|h| *h);

    // Supplement mode: skip if file hash already exists in target directory
    if config.processing_mode == ProcessingMode::Supplement
        && let Some(hash) = content_hash
        && existing_hashes.contains(&hash)
    {
        debug!(
            ?path,
            "File already exists in target (Supplement mode), skipping"
        );
        stats.skipped.fetch_add(1, Ordering::Relaxed);
        return FileResult {
            source: path.to_path_buf(),
            destination: None,
            time_info: None,
            status: ProcessingStatus::Skipped,
            error: None,
        };
    }

    // Check if file needs processing (incremental mode)
    if config.processing_mode == ProcessingMode::Incremental {
        match compute_metadata_hash(path) {
            Ok(metadata_hash) => {
                let state_guard = state.lock().unwrap();
                if !state_guard.needs_processing(path, metadata_hash) {
                    debug!(?path, "File already processed, skipping");
                    stats.skipped.fetch_add(1, Ordering::Relaxed);
                    return FileResult {
                        source: path.to_path_buf(),
                        destination: None,
                        time_info: None,
                        status: ProcessingStatus::Skipped,
                        error: None,
                    };
                }
            }
            Err(e) => {
                warn!(?path, error = %e, "Failed to compute metadata hash");
            }
        }
    }

    // Extract time information
    let time_info = match extract_time(path, config) {
        Ok(info) => info,
        Err(e) => {
            error!(?path, error = %e, "Failed to extract time");
            stats.failed.fetch_add(1, Ordering::Relaxed);
            return FileResult {
                source: path.to_path_buf(),
                destination: None,
                time_info: None,
                status: ProcessingStatus::Failed,
                error: Some(e.to_string()),
            };
        }
    };

    // Check for duplicates in persisted state (for incremental processing)
    if let Some(hash) = content_hash
        && config.processing_mode == ProcessingMode::Incremental
    {
        let state_guard = state.lock().unwrap();
        if let Some(existing) = state_guard.has_content_hash(hash) {
            debug!(?path, ?existing, "Duplicate file detected (from state)");
            stats.duplicates.fetch_add(1, Ordering::Relaxed);
            return FileResult {
                source: path.to_path_buf(),
                destination: Some(existing.clone()),
                time_info: Some(time_info),
                status: ProcessingStatus::Duplicate,
                error: None,
            };
        }
    }

    // Build base destination path (without conflict resolution)
    let base_dest_path = match build_base_destination_path(path, &time_info.timestamp, config) {
        Ok(p) => p,
        Err(e) => {
            error!(?path, error = %e, "Failed to build destination path");
            stats.failed.fetch_add(1, Ordering::Relaxed);
            return FileResult {
                source: path.to_path_buf(),
                destination: None,
                time_info: Some(time_info),
                status: ProcessingStatus::Failed,
                error: Some(e.to_string()),
            };
        }
    };

    // Check if destination already exists with the same content
    // Behavior depends on processing mode:
    // - Full mode: overwrite (use base path, don't add suffix)
    // - Supplement/Incremental mode: skip if same content already exists
    let dest_path = if base_dest_path.exists() {
        if let Some(source_hash) = content_hash {
            if let Ok(dest_hash) = compute_file_hash(&base_dest_path, config.large_file_threshold) {
                if source_hash == dest_hash {
                    match config.processing_mode {
                        ProcessingMode::Full => {
                            // Full mode: file is identical, still "process" it
                            // (actually just skip the copy but count as processed for user expectation)
                            debug!(
                                ?path,
                                ?base_dest_path,
                                "File already exists with identical content (Full mode - counting as processed)"
                            );
                            stats.processed.fetch_add(1, Ordering::Relaxed);
                            return FileResult {
                                source: path.to_path_buf(),
                                destination: Some(base_dest_path),
                                time_info: Some(time_info),
                                status: ProcessingStatus::Success,
                                error: None,
                            };
                        }
                        ProcessingMode::Supplement | ProcessingMode::Incremental => {
                            // Supplement/Incremental: skip file with same content
                            debug!(
                                ?path,
                                ?base_dest_path,
                                "Skipping file - destination already exists with identical content"
                            );
                            stats.skipped.fetch_add(1, Ordering::Relaxed);
                            return FileResult {
                                source: path.to_path_buf(),
                                destination: Some(base_dest_path),
                                time_info: Some(time_info),
                                status: ProcessingStatus::Skipped,
                                error: None,
                            };
                        }
                    }
                } else {
                    // Different content - behavior by mode
                    match config.processing_mode {
                        ProcessingMode::Full => {
                            // Full mode: overwrite existing file (use base path)
                            debug!(
                                ?path,
                                ?base_dest_path,
                                "Overwriting existing file (Full mode)"
                            );
                            base_dest_path
                        }
                        ProcessingMode::Supplement | ProcessingMode::Incremental => {
                            // Add suffix to avoid overwriting
                            match resolve_filename_conflict(base_dest_path) {
                                Ok(p) => p,
                                Err(e) => {
                                    error!(?path, error = %e, "Failed to resolve filename conflict");
                                    stats.failed.fetch_add(1, Ordering::Relaxed);
                                    return FileResult {
                                        source: path.to_path_buf(),
                                        destination: None,
                                        time_info: Some(time_info),
                                        status: ProcessingStatus::Failed,
                                        error: Some(e.to_string()),
                                    };
                                }
                            }
                        }
                    }
                }
            } else {
                // Couldn't compute dest hash, resolve conflict normally
                match config.processing_mode {
                    ProcessingMode::Full => base_dest_path,
                    _ => match resolve_filename_conflict(base_dest_path) {
                        Ok(p) => p,
                        Err(e) => {
                            error!(?path, error = %e, "Failed to resolve filename conflict");
                            stats.failed.fetch_add(1, Ordering::Relaxed);
                            return FileResult {
                                source: path.to_path_buf(),
                                destination: None,
                                time_info: Some(time_info),
                                status: ProcessingStatus::Failed,
                                error: Some(e.to_string()),
                            };
                        }
                    },
                }
            }
        } else {
            // No source hash available, resolve conflict normally
            match config.processing_mode {
                ProcessingMode::Full => base_dest_path,
                _ => match resolve_filename_conflict(base_dest_path) {
                    Ok(p) => p,
                    Err(e) => {
                        error!(?path, error = %e, "Failed to resolve filename conflict");
                        stats.failed.fetch_add(1, Ordering::Relaxed);
                        return FileResult {
                            source: path.to_path_buf(),
                            destination: None,
                            time_info: Some(time_info),
                            status: ProcessingStatus::Failed,
                            error: Some(e.to_string()),
                        };
                    }
                },
            }
        }
    } else {
        // Destination doesn't exist, use base path
        base_dest_path
    };

    // Handle dry run
    if config.dry_run {
        info!(
            source = ?path,
            destination = ?dest_path,
            time_source = ?time_info.source,
            "Would process file"
        );

        // Record destination for duplicate reporting
        if let Some(hash) = content_hash {
            let mut dest_map = hash_to_dest.lock().unwrap();
            dest_map.insert(hash, dest_path.clone());
        }

        stats.processed.fetch_add(1, Ordering::Relaxed);
        return FileResult {
            source: path.to_path_buf(),
            destination: Some(dest_path),
            time_info: Some(time_info),
            status: ProcessingStatus::DryRun,
            error: None,
        };
    }

    // Perform the file operation
    if let Err(e) = perform_file_operation(path, &dest_path, config) {
        error!(?path, ?dest_path, error = %e, "Failed to process file");
        stats.failed.fetch_add(1, Ordering::Relaxed);
        return FileResult {
            source: path.to_path_buf(),
            destination: Some(dest_path),
            time_info: Some(time_info),
            status: ProcessingStatus::Failed,
            error: Some(e.to_string()),
        };
    }

    // Record destination for duplicate reporting
    if let Some(hash) = content_hash {
        let mut dest_map = hash_to_dest.lock().unwrap();
        dest_map.insert(hash, dest_path.clone());
    }

    // Update state
    if config.processing_mode == ProcessingMode::Incremental
        && let (Ok(metadata_hash), Some(content_hash)) = (compute_metadata_hash(path), content_hash)
    {
        let mut state_guard = state.lock().unwrap();
        state_guard.record_processed(
            path.to_path_buf(),
            dest_path.clone(),
            content_hash,
            metadata_hash,
        );
    }

    info!(
        source = ?path,
        destination = ?dest_path,
        time_source = ?time_info.source,
        timestamp = %time_info.timestamp,
        "Processed file"
    );
    stats.processed.fetch_add(1, Ordering::Relaxed);

    FileResult {
        source: path.to_path_buf(),
        destination: Some(dest_path),
        time_info: Some(time_info),
        status: ProcessingStatus::Success,
        error: None,
    }
}

/// Build the base destination path based on classification rules (without conflict resolution)
fn build_base_destination_path(
    source: &Path,
    timestamp: &NaiveDateTime,
    config: &Config,
) -> Result<PathBuf> {
    let filename = source
        .file_name()
        .ok_or_else(|| Error::Config("Invalid source filename".into()))?;

    let mut dest = config.output_dir.clone();

    // Time-based classification
    match config.classification {
        ClassificationRule::None => {
            // Files go directly to output directory
        }
        ClassificationRule::Year => {
            dest.push(format!("{}", timestamp.year()));
        }
        ClassificationRule::YearMonth => match config.month_format {
            MonthFormat::Nested => {
                dest.push(format!("{}", timestamp.year()));
                dest.push(format!("{:02}", timestamp.month()));
            }
            MonthFormat::Combined => {
                dest.push(format!("{}-{:02}", timestamp.year(), timestamp.month()));
            }
        },
    }

    // File type classification (after time classification)
    if config.classify_by_type
        && let Some(ext) = source.extension().and_then(|e| e.to_str())
        && let Some(file_type) = config.get_file_type(ext)
    {
        match file_type {
            FileType::Photos => {
                dest.push(file_type.folder_name());
            }
            FileType::Raw => {
                // RAW files are nested under Photos/Raw
                dest.push(FileType::Photos.folder_name());
                dest.push(file_type.folder_name());
            }
            FileType::Videos => {
                dest.push(file_type.folder_name());
            }
        }
    }

    dest.push(filename);
    Ok(dest)
}

/// Resolve filename conflicts by adding a numeric suffix
fn resolve_filename_conflict(mut path: PathBuf) -> Result<PathBuf> {
    if !path.exists() {
        return Ok(path);
    }

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| Error::Config("Invalid filename".into()))?
        .to_string();

    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();

    let parent = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();

    for i in 1..10000 {
        let new_name = format!("{}_{}{}", stem, i, extension);
        path = parent.join(new_name);
        if !path.exists() {
            return Ok(path);
        }
    }

    Err(Error::Config("Could not resolve filename conflict".into()))
}

/// Perform the actual file operation (copy, move, symlink, hardlink)
fn perform_file_operation(source: &Path, dest: &Path, config: &Config) -> Result<()> {
    // Create parent directory
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    match config.operation {
        FileOperation::Copy => {
            copy_file(source, dest)?;
        }
        FileOperation::Move => {
            // Try rename first (faster for same filesystem)
            if fs::rename(source, dest).is_err() {
                // Fall back to copy + delete for cross-filesystem moves
                copy_file(source, dest)?;
                fs::remove_file(source)?;
            }
        }
        FileOperation::Symlink => {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(source, dest)?;
            }
            #[cfg(windows)]
            {
                // Windows symlinks require special permissions
                std::os::windows::fs::symlink_file(source, dest)?;
            }
        }
        FileOperation::Hardlink => {
            fs::hard_link(source, dest)?;
        }
    }

    // Preserve modification time
    if let Ok(metadata) = fs::metadata(source)
        && let Ok(mtime) = metadata.modified()
    {
        let _ = filetime::set_file_mtime(dest, filetime::FileTime::from_system_time(mtime));
    }

    Ok(())
}

/// Copy file with buffered I/O for efficiency
fn copy_file(source: &Path, dest: &Path) -> Result<()> {
    let src_file = File::open(source)?;
    let dest_file = File::create(dest)?;

    let mut reader = BufReader::with_capacity(256 * 1024, src_file);
    let mut writer = BufWriter::with_capacity(256 * 1024, dest_file);

    let mut buffer = vec![0u8; 256 * 1024];
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        writer.write_all(&buffer[..bytes_read])?;
    }

    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processing_stats() {
        let stats = ProcessingStats::new();
        stats.processed.fetch_add(5, Ordering::Relaxed);
        stats.skipped.fetch_add(2, Ordering::Relaxed);
        stats.duplicates.fetch_add(1, Ordering::Relaxed);
        stats.failed.fetch_add(1, Ordering::Relaxed);

        let summary = stats.summary();
        assert!(summary.contains("Processed: 5"));
        assert!(summary.contains("Skipped: 2"));
        assert!(summary.contains("Duplicates: 1"));
        assert!(summary.contains("Failed: 1"));
    }

    #[test]
    fn test_filename_priority_score() {
        // Clean filenames (shorter) should have lower scores than copies (longer)
        let clean = Path::new("IMG_20251006_180519.jpg");
        let copy_cn = Path::new("IMG_20251006_180519 - 副本.jpg");
        let copy_suffix1 = Path::new("IMG_20251006_180519_1.jpg");
        let copy_suffix2 = Path::new("IMG_20251006_180527_2.jpg");
        let copy_space = Path::new("IMG_20251007_151359 1.jpg");
        let copy_paren = Path::new("IMG_20251006_180519(1).jpg");

        let score_clean = filename_priority_score(clean);
        let score_copy_cn = filename_priority_score(copy_cn);
        let score_suffix1 = filename_priority_score(copy_suffix1);
        let score_suffix2 = filename_priority_score(copy_suffix2);
        let score_space = filename_priority_score(copy_space);
        let score_paren = filename_priority_score(copy_paren);

        // Clean filename (shortest) should have the lowest score
        assert!(
            score_clean < score_copy_cn,
            "Clean ({}) < Chinese copy ({})",
            score_clean,
            score_copy_cn
        );
        assert!(
            score_clean < score_suffix1,
            "Clean ({}) < _1 suffix ({})",
            score_clean,
            score_suffix1
        );
        assert!(
            score_clean < score_suffix2,
            "Clean ({}) < _2 suffix ({})",
            score_clean,
            score_suffix2
        );
        assert!(
            score_clean < score_space,
            "Clean ({}) < space suffix ({})",
            score_clean,
            score_space
        );
        assert!(
            score_clean < score_paren,
            "Clean ({}) < parentheses ({})",
            score_clean,
            score_paren
        );
    }

    #[test]
    fn test_filename_priority_sorting() {
        let mut files = vec![
            PathBuf::from("IMG_20251006_180519 - 副本.jpg"),
            PathBuf::from("IMG_20251006_180519.jpg"),
            PathBuf::from("IMG_20251006_180527_2.jpg"),
            PathBuf::from("IMG_20251006_180527.jpg"),
            PathBuf::from("IMG_20251006_180527_1.jpg"),
        ];

        files.sort_by_cached_key(|path| filename_priority_score(path));

        // Clean filenames should come first
        assert_eq!(
            files[0].file_name().unwrap().to_str().unwrap(),
            "IMG_20251006_180519.jpg"
        );
        assert_eq!(
            files[1].file_name().unwrap().to_str().unwrap(),
            "IMG_20251006_180527.jpg"
        );
    }
}
