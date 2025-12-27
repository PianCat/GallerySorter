//! Video metadata extraction via FFprobe

use crate::error::{Error, Result};
use crate::time::datetime::parse_video_datetime;
use crate::time::filename::parse_filename_time;
use chrono::{Local, NaiveDateTime};
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;
use tracing::{debug, trace, warn};

/// Metadata keys to try for creation date
const CREATION_DATE_KEYS: &[&str] = &[
    "creation_time",
    "com.apple.quicktime.creationdate",
    "date",
    "date_recorded",
];

/// Cached FFprobe availability check
static FFPROBE_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Check if ffprobe is available (cached)
fn is_ffprobe_available() -> bool {
    *FFPROBE_AVAILABLE.get_or_init(|| Command::new("ffprobe").arg("-version").output().is_ok())
}

/// Extract creation time from video metadata using FFprobe
///
/// Video metadata typically stores creation time in UTC. This function:
/// 1. Extracts the UTC timestamp from video metadata
/// 2. Attempts to parse the filename for a local timestamp
/// 3. If filename has a valid timestamp, calculates timezone offset and applies correction
/// 4. If no valid filename timestamp, converts UTC to local system timezone
pub fn extract_video_time(path: &Path) -> Result<NaiveDateTime> {
    // Check if ffprobe is available (cached)
    if !is_ffprobe_available() {
        return Err(Error::FfprobeNotFound);
    }

    // Run ffprobe to get metadata
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .map_err(|e| Error::VideoMetadata {
            path: path.to_path_buf(),
            message: format!("Failed to execute ffprobe: {}", e),
        })?;

    if !output.status.success() {
        return Err(Error::VideoMetadata {
            path: path.to_path_buf(),
            message: format!(
                "FFprobe failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        });
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    trace!(?path, "FFprobe output: {}", json_str);

    // Parse JSON output
    let json: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| Error::VideoMetadata {
            path: path.to_path_buf(),
            message: format!("Failed to parse FFprobe JSON: {}", e),
        })?;

    // Try to find creation time in format tags
    let mut utc_time: Option<NaiveDateTime> = None;

    if let Some(format) = json.get("format")
        && let Some(tags) = format.get("tags")
    {
        for key in CREATION_DATE_KEYS {
            // Try both lowercase and original case
            for tag_key in [*key, &key.to_uppercase()] {
                if let Some(value) = tags.get(tag_key).and_then(|v| v.as_str())
                    && let Some(dt) = parse_video_datetime(value)
                {
                    debug!(?path, key = tag_key, "Found video creation time");
                    utc_time = Some(dt);
                    break;
                }
            }
            if utc_time.is_some() {
                break;
            }
        }
    }

    // Try to find in stream tags if not found in format
    if utc_time.is_none()
        && let Some(streams) = json.get("streams").and_then(|s| s.as_array())
    {
        'outer: for stream in streams {
            if let Some(tags) = stream.get("tags") {
                for key in CREATION_DATE_KEYS {
                    for tag_key in [*key, &key.to_uppercase()] {
                        if let Some(value) = tags.get(tag_key).and_then(|v| v.as_str())
                            && let Some(dt) = parse_video_datetime(value)
                        {
                            debug!(?path, key = tag_key, "Found video creation time in stream");
                            utc_time = Some(dt);
                            break 'outer;
                        }
                    }
                }
            }
        }
    }

    let utc_time = utc_time.ok_or_else(|| Error::VideoMetadata {
        path: path.to_path_buf(),
        message: "No creation time found in video metadata".to_string(),
    })?;

    // Apply timezone correction
    let corrected_time = apply_timezone_correction(path, utc_time);
    Ok(corrected_time)
}

/// Apply timezone correction to UTC video timestamp
///
/// Strategy:
/// 1. Try to parse filename for a local timestamp
/// 2. If filename has valid timestamp, calculate timezone offset from difference
/// 3. If no valid filename timestamp, use local system timezone
fn apply_timezone_correction(path: &Path, utc_time: NaiveDateTime) -> NaiveDateTime {
    // Try to get filename timestamp
    if let Some(filename) = path.file_name().and_then(|f| f.to_str())
        && let Some(filename_time) = parse_filename_time(filename)
    {
        // Calculate the difference between filename time and UTC time
        // filename_time is assumed to be local time
        // utc_time is UTC
        let diff_seconds = (filename_time - utc_time).num_seconds();

        // Timezone offset should be in reasonable range (-12h to +14h)
        // and typically in hour increments
        if diff_seconds.abs() <= 14 * 3600 && diff_seconds.abs() >= 0 {
            // Round to nearest 15-minute increment (some timezones have 30/45 min offsets)
            let offset_seconds = (diff_seconds / 900) * 900;

            if offset_seconds != 0 {
                debug!(
                    ?path,
                    utc_time = %utc_time,
                    filename_time = %filename_time,
                    offset_hours = offset_seconds / 3600,
                    "Calculated timezone offset from filename"
                );

                // Apply the offset to UTC time
                return utc_time + chrono::Duration::seconds(offset_seconds);
            }
        } else {
            // Difference is too large, might be wrong filename or different date
            warn!(
                ?path,
                utc_time = %utc_time,
                filename_time = %filename_time,
                diff_hours = diff_seconds / 3600,
                "Filename timestamp differs too much from metadata, using system timezone"
            );
        }
    }

    // Fall back to system local timezone
    // Convert UTC naive datetime to local timezone
    let local_offset = Local::now().offset().local_minus_utc();
    let local_time = utc_time + chrono::Duration::seconds(local_offset as i64);

    debug!(
        ?path,
        utc_time = %utc_time,
        local_time = %local_time,
        offset_hours = local_offset / 3600,
        "Applied system timezone offset"
    );

    local_time
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_parse_video_datetime() {
        // ISO 8601 with Z
        let dt = parse_video_datetime("2024-01-15T14:30:00Z").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);

        // With milliseconds
        let dt = parse_video_datetime("2024-01-15T14:30:00.123Z").unwrap();
        assert_eq!(dt.year(), 2024);

        // With timezone offset - should convert to UTC
        let dt = parse_video_datetime("2024-01-15T14:30:00+08:00").unwrap();
        assert_eq!(dt.year(), 2024);
        // 14:30 +08:00 = 06:30 UTC
        assert_eq!(dt.hour(), 6);
        assert_eq!(dt.minute(), 30);

        // Without timezone (assumed UTC)
        let dt = parse_video_datetime("2024-01-15T14:30:00").unwrap();
        assert_eq!(dt.year(), 2024);

        // Space separator
        let dt = parse_video_datetime("2024-01-15 14:30:00").unwrap();
        assert_eq!(dt.year(), 2024);

        // Invalid format
        assert!(parse_video_datetime("invalid").is_none());
    }
}
