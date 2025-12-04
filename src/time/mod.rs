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

/// Extract creation time from a media file using multiple strategies
///
/// The extraction follows this priority:
/// 1. EXIF metadata (for images)
/// 2. Video metadata via FFprobe (for videos)
/// 3. Filename parsing
/// 4. File system modification time
pub fn extract_time(path: &Path, config: &Config) -> Result<ExtractedTime> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

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
