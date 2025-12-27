//! EXIF time extraction for images

use crate::error::{Error, Result};
use crate::time::datetime::parse_exif;
use chrono::NaiveDateTime;
use exif::{In, Reader, Tag};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tracing::trace;

/// EXIF tags to try for date extraction, in priority order
const DATE_TAGS: &[Tag] = &[
    Tag::DateTimeOriginal,  // When the original image was taken
    Tag::DateTimeDigitized, // When the image was digitized
    Tag::DateTime,          // File modification date/time
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
        if let Some(field) = exif.get_field(*tag, In::PRIMARY)
            && let Some(datetime) = parse_exif(&field.display_value().to_string())
        {
            trace!(?path, ?tag, "Found EXIF date");
            return Ok(datetime);
        }
    }

    Err(Error::ExifRead {
        path: path.to_path_buf(),
        message: "No valid date tag found in EXIF data".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_parse_exif() {
        // Standard EXIF format
        let dt = parse_exif("2024:01:15 14:30:00").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 14);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 0);

        // With quotes
        let dt = parse_exif("\"2024:01:15 14:30:00\"").unwrap();
        assert_eq!(dt.year(), 2024);

        // Alternative formats
        let dt = parse_exif("2024-01-15 14:30:00").unwrap();
        assert_eq!(dt.year(), 2024);

        // Invalid format
        assert!(parse_exif("invalid").is_none());
    }
}
