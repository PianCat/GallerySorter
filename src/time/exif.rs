//! EXIF time extraction for images

use crate::error::{Error, Result};
use chrono::NaiveDateTime;
use exif::{In, Reader, Tag};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tracing::trace;

/// EXIF tags to try for date extraction, in priority order
const DATE_TAGS: &[Tag] = &[
    Tag::DateTimeOriginal,    // When the original image was taken
    Tag::DateTimeDigitized,   // When the image was digitized
    Tag::DateTime,            // File modification date/time
];

/// Extract creation time from EXIF metadata
pub fn extract_exif_time(path: &Path) -> Result<NaiveDateTime> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let exif = Reader::new()
        .read_from_container(&mut reader)
        .map_err(|e| Error::ExifRead {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;

    // Try each date tag in priority order
    for tag in DATE_TAGS {
        if let Some(field) = exif.get_field(*tag, In::PRIMARY) {
            if let Some(datetime) = parse_exif_datetime(&field.display_value().to_string()) {
                trace!(?path, ?tag, "Found EXIF date");
                return Ok(datetime);
            }
        }
    }

    Err(Error::ExifRead {
        path: path.to_path_buf(),
        message: "No valid date tag found in EXIF data".to_string(),
    })
}

/// Parse EXIF datetime string format: "YYYY:MM:DD HH:MM:SS"
fn parse_exif_datetime(s: &str) -> Option<NaiveDateTime> {
    // EXIF format: "2024:01:15 14:30:00" or with quotes
    let s = s.trim().trim_matches('"');

    // Try standard EXIF format
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y:%m:%d %H:%M:%S") {
        return Some(dt);
    }

    // Try with subseconds
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y:%m:%d %H:%M:%S%.f") {
        return Some(dt);
    }

    // Try alternative formats
    let formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
    ];

    for format in formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, format) {
            return Some(dt);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_parse_exif_datetime() {
        // Standard EXIF format
        let dt = parse_exif_datetime("2024:01:15 14:30:00").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 14);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 0);

        // With quotes
        let dt = parse_exif_datetime("\"2024:01:15 14:30:00\"").unwrap();
        assert_eq!(dt.year(), 2024);

        // Alternative formats
        let dt = parse_exif_datetime("2024-01-15 14:30:00").unwrap();
        assert_eq!(dt.year(), 2024);

        // Invalid format
        assert!(parse_exif_datetime("invalid").is_none());
    }
}

