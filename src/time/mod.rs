//! Time extraction module
//!
//! This module provides functionality to extract creation timestamps from:
//! - EXIF metadata in images (JPEG, HEIF, RAW formats)
//! - Video metadata via FFprobe
//! - Filename patterns
//! - File system modification time

pub mod exif;
pub mod filename;
pub mod video;

use crate::config::Config;
use crate::error::Result;
use chrono::NaiveDateTime;
use std::fs;
use std::path::Path;
use tracing::{debug, warn};

/// Source of the extracted timestamp
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSource {
    /// Extracted from EXIF metadata
    Exif,
    /// Extracted from video metadata via FFprobe
    VideoMetadata,
    /// Parsed from filename
    Filename,
    /// From file system modification time
    FileSystem,
}

/// Result of timestamp extraction
#[derive(Debug, Clone)]
pub struct ExtractedTime {
    /// The extracted timestamp
    pub timestamp: NaiveDateTime,
    /// Source of the timestamp
    pub source: TimeSource,
}

/// Unified datetime parsing utilities
pub mod datetime {
    use chrono::{DateTime, NaiveDateTime};

    /// Common ISO 8601 formats for video metadata
    const ISO8601_FORMATS: &[&str] = &[
        "%Y-%m-%dT%H:%M:%S%.fZ",
        "%Y-%m-%dT%H:%M:%SZ",
        "%Y-%m-%dT%H:%M:%S%.f%:z",
        "%Y-%m-%dT%H:%M:%S%:z",
        "%Y-%m-%dT%H:%M:%S%.f%z",
        "%Y-%m-%dT%H:%M:%S%z",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
    ];

    /// Common date formats for various sources
    const DATE_FORMATS: &[&str] = &[
        "%Y:%m:%d %H:%M:%S",
        "%Y:%m:%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
    ];

    /// Parse datetime string with multiple format attempts
    ///
    /// Returns `Some(NaiveDateTime)` if parsing succeeds, `None` otherwise.
    /// Tries formats in order, returns on first success.
    pub fn parse_multi(s: &str, formats: &[&str]) -> Option<NaiveDateTime> {
        let s = s.trim();
        for format in formats {
            if let Ok(dt) = NaiveDateTime::parse_from_str(s, format) {
                return Some(dt);
            }
        }
        None
    }

    /// Parse datetime string with timezone (ISO 8601 variants)
    ///
    /// Handles formats with timezone info (Z, +08:00, etc.) and returns UTC.
    /// Falls back to naive datetime if no timezone present.
    /// Also handles RFC 3339 and date-only formats.
    pub fn parse_video_datetime(s: &str) -> Option<NaiveDateTime> {
        let s = s.trim();

        // Try RFC 3339 first (more flexible)
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Some(dt.naive_utc());
        }

        // Try parsing with timezone and convert to UTC
        for format in ISO8601_FORMATS {
            if let Ok(dt) = DateTime::parse_from_str(s, format) {
                return Some(dt.naive_utc());
            }
        }

        // Try as naive datetime (assumed UTC)
        if let Some(dt) = parse_multi(s, DATE_FORMATS) {
            return Some(dt);
        }

        // Try parsing just the date part if time is missing
        if let Ok(date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            return date.and_hms_opt(0, 0, 0);
        }

        None
    }

    /// Parse EXIF format datetime string
    pub fn parse_exif(s: &str) -> Option<NaiveDateTime> {
        let s = s.trim().trim_matches('"');
        parse_multi(s, DATE_FORMATS)
    }
}

/// Extract creation time from a media file using multiple strategies
///
/// The extraction follows this priority:
/// 1. EXIF metadata (for images)
/// 2. Video metadata via FFprobe (for videos)
/// 3. Filename parsing
/// 4. File system modification time
pub fn extract_time(path: &Path, config: &Config) -> Result<ExtractedTime> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    // Try EXIF for images
    if config.is_image(ext) {
        if let Ok(time) = exif::extract_exif_time(path) {
            debug!(?path, "Extracted time from EXIF");
            return Ok(ExtractedTime {
                timestamp: time,
                source: TimeSource::Exif,
            });
        }
        debug!(?path, "No EXIF time found, trying other methods");
    }

    // Try video metadata for videos
    if config.is_video(ext) {
        if let Ok(time) = video::extract_video_time(path) {
            debug!(?path, "Extracted time from video metadata");
            return Ok(ExtractedTime {
                timestamp: time,
                source: TimeSource::VideoMetadata,
            });
        }
        debug!(?path, "No video metadata time found, trying other methods");
    }

    // Try filename parsing
    if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
        if let Some(time) = filename::parse_filename_time(filename) {
            debug!(?path, "Extracted time from filename");
            return Ok(ExtractedTime {
                timestamp: time,
                source: TimeSource::Filename,
            });
        }
        debug!(?path, "No time found in filename, using file system time");
    }

    // Fall back to file system modification time
    let metadata = fs::metadata(path)?;
    let modified = metadata.modified()?;
    let datetime: chrono::DateTime<chrono::Utc> = modified.into();
    let naive = datetime.naive_utc();

    warn!(?path, "Using file system modification time as fallback");

    Ok(ExtractedTime {
        timestamp: naive,
        source: TimeSource::FileSystem,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_source_debug() {
        assert_eq!(format!("{:?}", TimeSource::Exif), "Exif");
        assert_eq!(format!("{:?}", TimeSource::VideoMetadata), "VideoMetadata");
        assert_eq!(format!("{:?}", TimeSource::Filename), "Filename");
        assert_eq!(format!("{:?}", TimeSource::FileSystem), "FileSystem");
    }
}
