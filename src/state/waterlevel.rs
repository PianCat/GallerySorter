//! Water level (TOML) incremental state
//!
//! Tracks only the newest processed file, enabling quick filtering of
//! source files without computing metadata hashes for every file.

use crate::config::{ClassificationRule, Config, MonthFormat};
use crate::error::{Error, Result};
use chrono::{Datelike, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use walkdir::WalkDir;

/// Increment Metadata file name
const WATER_LEVEL_FILENAME: &str = ".gallery_sorter_increment_metadata.toml";

/// 重建水位线时的时间来源优先级：元数据 > 文件名 > 文件系统时间。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum WaterLevelTimePriority {
    FileSystem,
    Filename,
    Metadata,
}

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

/// Incremental water level for efficient timestamp-based filtering
///
/// This tracks only the newest processed file, enabling quick filtering
/// of source files without computing hashes for every file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalWaterLevel {
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

    /// When this water level was last updated
    pub last_updated: chrono::DateTime<chrono::Utc>,

    /// Total files processed in last run
    pub files_processed: usize,
}

impl IncrementalWaterLevel {
    /// Current water level format version
    const VERSION: u32 = 1;

    /// Create a new water level
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

    /// Get the water level file path for an output directory
    pub fn get_path(output_dir: &Path) -> PathBuf {
        output_dir.join(WATER_LEVEL_FILENAME)
    }

    /// Load water level from output directory
    pub fn load(output_dir: &Path) -> Result<Option<Self>> {
        let path = Self::get_path(output_dir);

        if !path.exists() {
            debug!(?path, "Water level file does not exist");
            return Ok(None);
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| Error::StateFile(format!("Failed to read water level file: {}", e)))?;

        let water_level: Self = match toml::from_str(&content) {
            Ok(water_level) => water_level,
            Err(e) => {
                warn!(
                    path = %path.display(),
                    error = %e,
                    "Water level file is invalid, rescanning output directory"
                );
                return Ok(None);
            }
        };

        if water_level.version != Self::VERSION {
            warn!(
                water_level_version = water_level.version,
                current_version = Self::VERSION,
                "Water level version mismatch, will rescan"
            );
            return Ok(None);
        }

        info!(
            newest_file = %water_level.newest_file_path.display(),
            newest_timestamp = %water_level.newest_timestamp,
            "Loaded incremental water level"
        );

        Ok(Some(water_level))
    }

    /// Save water level to output directory
    pub fn save(&self, output_dir: &Path) -> Result<()> {
        let path = Self::get_path(output_dir);

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::StateFile(format!("Failed to serialize water level: {}", e)))?;

        // Write atomically via temp file
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, &content)
            .map_err(|e| Error::StateFile(format!("Failed to write water level file: {}", e)))?;

        fs::rename(&temp_path, &path)
            .map_err(|e| Error::StateFile(format!("Failed to rename water level file: {}", e)))?;

        info!(
            newest_file = %self.newest_file_path.display(),
            newest_timestamp = %self.newest_timestamp,
            "Saved incremental water level"
        );

        Ok(())
    }

    /// Check if a source file's timestamp is newer than the water level
    ///
    /// Returns true if the file should be processed (is newer than water level)
    pub fn is_newer(&self, timestamp: &NaiveDateTime) -> bool {
        *timestamp > self.newest_timestamp
    }

    /// Scan output directory to find the newest file based on directory structure
    ///
    /// This is used when the water level file doesn't exist but we need to
    /// determine the cutoff timestamp by analyzing existing files.
    pub fn scan_output_directory(output_dir: &Path, config: &Config) -> Result<Option<Self>> {
        if !output_dir.exists() {
            debug!(?output_dir, "Output directory does not exist");
            return Ok(None);
        }

        info!(
            ?output_dir,
            "Scanning output directory to find newest folder"
        );

        // 第一遍：锁定最新的分类文件夹。
        let newest_folder = if config.classification == ClassificationRule::None {
            (0, 0)
        } else {
            let mut newest_folder: Option<(i32, u32)> = None;
            for entry in WalkDir::new(output_dir)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if !path.is_file() || !Self::is_visible_output_file(path, config) {
                    continue;
                }
                if let Some(folder) = Self::output_folder_key(path, output_dir, config)
                    && newest_folder.is_none_or(|current| folder > current)
                {
                    newest_folder = Some(folder);
                }
            }
            newest_folder.unwrap_or((0, 0))
        };

        // 第二遍：只扫描最新文件夹，按可信度选择真正最新的文件。
        let mut newest: Option<(WaterLevelTimePriority, NaiveDateTime, PathBuf)> = None;

        for entry in WalkDir::new(output_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if !Self::is_visible_output_file(path, config) {
                continue;
            }

            if config.classification != ClassificationRule::None {
                let Some(folder) = Self::output_folder_key(path, output_dir, config) else {
                    continue;
                };
                if folder != newest_folder {
                    continue;
                }
            }

            let Some((priority, timestamp)) = Self::extract_water_level_candidate(path, config)
            else {
                continue;
            };
            let candidate = (priority, timestamp, path.to_path_buf());

            match &newest {
                Some((newest_priority, newest_ts, newest_path)) => {
                    let is_newer = candidate.0 > *newest_priority
                        || (candidate.0 == *newest_priority && candidate.1 > *newest_ts)
                        || (candidate.0 == *newest_priority
                            && candidate.1 == *newest_ts
                            && candidate.2 > *newest_path);
                    if is_newer {
                        newest = Some(candidate);
                    }
                }
                None => newest = Some(candidate),
            }
        }

        match newest {
            Some((_, timestamp, path)) => {
                // Compute hash for verification
                let hash = crate::hash::compute_file_hash(&path, 100 * 1024 * 1024).unwrap_or(0);

                let relative_path = path.strip_prefix(output_dir).unwrap_or(&path).to_path_buf();

                info!(
                    newest_file = %relative_path.display(),
                    timestamp = %timestamp,
                    "Found newest file in output directory"
                );

                Ok(Some(Self::new(
                    relative_path,
                    timestamp,
                    hash,
                    config.classification,
                    config.month_format,
                )))
            }
            None => {
                debug!("No files found in output directory");
                Ok(None)
            }
        }
    }

    /// 判断是否为可见且受支持的输出媒体文件。
    fn is_visible_output_file(path: &Path, config: &Config) -> bool {
        if !path.is_file() {
            return false;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && name.starts_with('.')
        {
            return false;
        }

        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            return false;
        };
        config.is_image(ext) || config.is_video(ext) || config.is_raw(ext)
    }

    /// 按分类从目录结构取得文件夹优先级标识。
    fn output_folder_key(path: &Path, output_dir: &Path, config: &Config) -> Option<(i32, u32)> {
        if config.classification == ClassificationRule::None {
            return Some((0, 0));
        }

        let folder_timestamp = Self::extract_timestamp_from_path(
            path,
            output_dir,
            config.classification,
            config.month_format,
        )?;
        Some((folder_timestamp.year(), folder_timestamp.month()))
    }

    /// 从输出文件提取时间戳：EXIF/FFprobe 元数据 > 文件名 > 本地修改时间。
    ///
    /// 明显晚于当前时间（超过 1 天）的异常值会被忽略，避免异常文件把水位线推远。
    fn extract_water_level_candidate(
        path: &Path,
        config: &Config,
    ) -> Option<(WaterLevelTimePriority, NaiveDateTime)> {
        let now = chrono::Local::now().naive_local();

        if let Ok(metadata) = crate::time::extract_metadata_time(path, config) {
            if !Self::is_future_timestamp(metadata.timestamp, now) {
                return Some((WaterLevelTimePriority::Metadata, metadata.timestamp));
            }
            warn!(
                ?path,
                timestamp = %metadata.timestamp,
                "Water level metadata timestamp is in the future, ignoring"
            );
        }

        if let Some(filename) = path.file_name().and_then(|name| name.to_str())
            && let Some(timestamp) = crate::time::filename::parse_filename_time(filename)
        {
            if !Self::is_future_timestamp(timestamp, now) {
                return Some((WaterLevelTimePriority::Filename, timestamp));
            }
            warn!(
                ?path,
                timestamp = %timestamp,
                "Water level filename timestamp is in the future, ignoring"
            );
        }

        let modified = fs::metadata(path).ok()?.modified().ok()?;
        let datetime: chrono::DateTime<chrono::Local> = modified.into();
        let timestamp = datetime.naive_local();

        if Self::is_future_timestamp(timestamp, now) {
            warn!(
                ?path,
                timestamp = %timestamp,
                "Water level file system timestamp is in the future, ignoring"
            );
            return None;
        }

        Some((WaterLevelTimePriority::FileSystem, timestamp))
    }

    fn is_future_timestamp(timestamp: NaiveDateTime, now: NaiveDateTime) -> bool {
        timestamp > now + chrono::Duration::days(1)
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

    /// Update water level with a new file if it's newer than the current one
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
    use std::time::{Duration, SystemTime};
    use tempfile::tempdir;

    #[test]
    fn test_water_level_load_returns_none_when_newest_file_missing() {
        let dir = tempdir().unwrap();
        let water_level_path = IncrementalWaterLevel::get_path(dir.path());
        fs::write(&water_level_path, "version = 1\n").unwrap();

        let loaded = IncrementalWaterLevel::load(dir.path()).unwrap();

        assert!(loaded.is_none());
    }

    #[test]
    fn test_water_level_new() {
        let timestamp =
            NaiveDateTime::parse_from_str("2024-06-15 14:30:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let wl = IncrementalWaterLevel::new(
            PathBuf::from("2024/06/photo.jpg"),
            timestamp,
            12345,
            ClassificationRule::YearMonth,
            MonthFormat::Nested,
        );

        assert_eq!(wl.newest_timestamp, timestamp);
        assert_eq!(wl.newest_hash, 12345);
        assert_eq!(wl.classification, ClassificationRule::YearMonth);
    }

    #[test]
    fn test_water_level_is_newer() {
        let timestamp =
            NaiveDateTime::parse_from_str("2024-06-15 14:30:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let wl = IncrementalWaterLevel::new(
            PathBuf::from("photo.jpg"),
            timestamp,
            12345,
            ClassificationRule::None,
            MonthFormat::Nested,
        );

        // Older timestamp - should not be newer
        let older =
            NaiveDateTime::parse_from_str("2024-05-01 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        assert!(!wl.is_newer(&older));

        // Same timestamp - should not be newer
        assert!(!wl.is_newer(&timestamp));

        // Newer timestamp - should be newer
        let newer =
            NaiveDateTime::parse_from_str("2024-07-01 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        assert!(wl.is_newer(&newer));
    }

    #[test]
    fn test_water_level_update_if_newer() {
        let timestamp1 =
            NaiveDateTime::parse_from_str("2024-06-15 14:30:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let mut wl = IncrementalWaterLevel::new(
            PathBuf::from("2024/06/photo1.jpg"),
            timestamp1,
            12345,
            ClassificationRule::YearMonth,
            MonthFormat::Nested,
        );

        // Try to update with older timestamp - should not change
        let older =
            NaiveDateTime::parse_from_str("2024-05-01 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        wl.update_if_newer(PathBuf::from("2024/05/old.jpg"), older, 99999);
        assert_eq!(wl.newest_timestamp, timestamp1);
        assert_eq!(wl.newest_hash, 12345);

        // Update with newer timestamp - should change
        let newer =
            NaiveDateTime::parse_from_str("2024-07-20 18:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        wl.update_if_newer(PathBuf::from("2024/07/new.jpg"), newer, 67890);
        assert_eq!(wl.newest_timestamp, newer);
        assert_eq!(wl.newest_hash, 67890);
        assert_eq!(wl.newest_file_path, PathBuf::from("2024/07/new.jpg"));
    }

    #[test]
    fn test_water_level_save_and_load() {
        let dir = tempdir().unwrap();
        let output_dir = dir.path();

        let timestamp =
            NaiveDateTime::parse_from_str("2024-06-15 14:30:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let wl = IncrementalWaterLevel::new(
            PathBuf::from("2024/06/photo.jpg"),
            timestamp,
            12345,
            ClassificationRule::YearMonth,
            MonthFormat::Nested,
        );

        // Save water level
        wl.save(output_dir).unwrap();

        // Check file exists
        let wl_path = IncrementalWaterLevel::get_path(output_dir);
        assert!(wl_path.exists());

        // Load water level
        let loaded = IncrementalWaterLevel::load(output_dir).unwrap();
        assert!(loaded.is_some());

        let loaded = loaded.unwrap();
        assert_eq!(loaded.newest_timestamp, timestamp);
        assert_eq!(loaded.newest_hash, 12345);
        assert_eq!(loaded.classification, ClassificationRule::YearMonth);
        assert_eq!(loaded.month_format, MonthFormat::Nested);
    }

    #[test]
    fn test_water_level_extract_timestamp_nested() {
        let output_dir = PathBuf::from("/output");

        // Test nested format: /output/2024/06/photo.jpg
        let file_path = PathBuf::from("/output/2024/06/photo.jpg");
        let ts = IncrementalWaterLevel::extract_timestamp_from_path(
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
    fn test_water_level_extract_timestamp_combined() {
        let output_dir = PathBuf::from("/output");

        // Test combined format: /output/2024-06/photo.jpg
        let file_path = PathBuf::from("/output/2024-06/photo.jpg");
        let ts = IncrementalWaterLevel::extract_timestamp_from_path(
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
    fn test_water_level_scan_uses_newest_month_and_newest_file_timestamp() {
        let dir = tempdir().unwrap();
        let output_dir = dir.path();
        let july = output_dir.join("2026/07/Photos");
        let august = output_dir.join("2026/08/Photos");
        fs::create_dir_all(&july).unwrap();
        fs::create_dir_all(&august).unwrap();

        fs::write(july.join("IMG_20260731_235959.jpg"), "older-folder").unwrap();
        fs::write(august.join("IMG_20260801_000000.jpg"), "newer-folder-start").unwrap();
        fs::write(august.join("IMG_20260824_120000.jpg"), "newest-file").unwrap();

        let config = Config {
            classification: ClassificationRule::YearMonth,
            month_format: MonthFormat::Nested,
            image_extensions: vec!["jpg".to_string()],
            ..Default::default()
        };
        let water_level = IncrementalWaterLevel::scan_output_directory(output_dir, &config)
            .unwrap()
            .unwrap();

        assert_eq!(
            water_level.newest_file_path,
            PathBuf::from("2026")
                .join("08")
                .join("Photos")
                .join("IMG_20260824_120000.jpg")
        );
        assert_eq!(
            water_level.newest_timestamp,
            NaiveDateTime::parse_from_str("2026-08-24 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap()
        );
    }

    #[test]
    fn test_water_level_scan_prefers_filename_over_newer_mtime() {
        let dir = tempdir().unwrap();
        let output_dir = dir.path();
        let photos = output_dir.join("2025/02/Photos");
        fs::create_dir_all(&photos).unwrap();

        let earlier_name = photos.join("IMG_20250201_000001.jpg");
        let later_name = photos.join("IMG_20250202_000001.jpg");
        fs::write(&earlier_name, "earlier-filename").unwrap();
        fs::write(&later_name, "later-filename").unwrap();
        let newer_mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(1_738_500_000);
        filetime::set_file_mtime(
            &earlier_name,
            filetime::FileTime::from_system_time(newer_mtime),
        )
        .unwrap();

        let config = Config {
            classification: ClassificationRule::YearMonth,
            month_format: MonthFormat::Nested,
            image_extensions: vec!["jpg".to_string()],
            ..Default::default()
        };
        let water_level = IncrementalWaterLevel::scan_output_directory(output_dir, &config)
            .unwrap()
            .unwrap();

        assert_eq!(
            water_level.newest_file_path,
            PathBuf::from("2025")
                .join("02")
                .join("Photos")
                .join("IMG_20250202_000001.jpg")
        );
        assert_eq!(
            water_level.newest_timestamp,
            NaiveDateTime::parse_from_str("2025-02-02 00:00:01", "%Y-%m-%d %H:%M:%S").unwrap()
        );
    }

    #[test]
    fn test_water_level_extract_timestamp_year_only() {
        let output_dir = PathBuf::from("/output");

        // Test year only format: /output/2024/photo.jpg
        let file_path = PathBuf::from("/output/2024/photo.jpg");
        let ts = IncrementalWaterLevel::extract_timestamp_from_path(
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
