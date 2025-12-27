//! Filename timestamp parsing

use chrono::NaiveDateTime;
use regex::Regex;
use std::sync::OnceLock;
use tracing::trace;

/// Pattern: YYYYMMDD_HHmmss or YYYYMMDD-HHmmss
static PATTERN_COMPACT: OnceLock<Regex> = OnceLock::new();

/// Pattern: YYYY-MM-DD_HH-mm-ss or similar with separators
static PATTERN_SEPARATED: OnceLock<Regex> = OnceLock::new();

/// Pattern: IMG_YYYYMMDD_HHmmss (common camera naming)
static PATTERN_IMG: OnceLock<Regex> = OnceLock::new();

/// Pattern: Unix timestamp (10 or 13 digits)
static PATTERN_UNIX: OnceLock<Regex> = OnceLock::new();

/// Pattern: YYYYMMDD only (date without time)
static PATTERN_DATE_ONLY: OnceLock<Regex> = OnceLock::new();

/// Pattern: Screenshot formats (common on various platforms)
static PATTERN_SCREENSHOT: OnceLock<Regex> = OnceLock::new();

/// Pattern: WhatsApp format (IMG-YYYYMMDD-WAxxxx)
static PATTERN_WHATSAPP: OnceLock<Regex> = OnceLock::new();

/// Get the compact pattern
fn get_pattern_compact() -> &'static Regex {
    PATTERN_COMPACT
        .get_or_init(|| Regex::new(r"(\d{4})(\d{2})(\d{2})[_\-](\d{2})(\d{2})(\d{2})").unwrap())
}

/// Get the separated pattern
fn get_pattern_separated() -> &'static Regex {
    PATTERN_SEPARATED.get_or_init(|| {
        Regex::new(r"(\d{4})[-_](\d{2})[-_](\d{2})[-_\s](\d{2})[-_](\d{2})[-_](\d{2})").unwrap()
    })
}

/// Get the IMG pattern
fn get_pattern_img() -> &'static Regex {
    PATTERN_IMG.get_or_init(|| Regex::new(r"(?:IMG|VID|DSC|DCIM|MOV|MVI|DJI|GOPR|GP)[-_]?(\d{4})(\d{2})(\d{2})[-_]?(\d{2})(\d{2})(\d{2})").unwrap())
}

/// Get the Unix pattern
fn get_pattern_unix() -> &'static Regex {
    PATTERN_UNIX.get_or_init(|| Regex::new(r"(\d{10}|\d{13})").unwrap())
}

/// Get the date-only pattern
fn get_pattern_date_only() -> &'static Regex {
    PATTERN_DATE_ONLY.get_or_init(|| Regex::new(r"(\d{4})(\d{2})(\d{2})").unwrap())
}

/// Get the screenshot pattern
fn get_pattern_screenshot() -> &'static Regex {
    PATTERN_SCREENSHOT.get_or_init(|| Regex::new(r"(?:Screenshot|Screen Shot|Capture|截图|截屏)[-_\s]*(\d{4})[-_]?(\d{2})[-_]?(\d{2})[-_\s]*(?:at[-_\s]*)?(\d{1,2})[-_\.]?(\d{2})[-_\.]?(\d{2})").unwrap())
}

/// Get the WhatsApp pattern
fn get_pattern_whatsapp() -> &'static Regex {
    PATTERN_WHATSAPP
        .get_or_init(|| Regex::new(r"(?:IMG|VID)[-_](\d{4})(\d{2})(\d{2})[-_]WA").unwrap())
}

/// Parse timestamp from filename using various patterns
pub fn parse_filename_time(filename: &str) -> Option<NaiveDateTime> {
    // Remove extension for cleaner parsing
    let name = filename
        .rsplit('.')
        .skip(1)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(".");
    let name = if name.is_empty() { filename } else { &name };

    // Try compact format first (most common)
    if let Some(dt) = try_pattern_compact(name) {
        trace!(filename, "Matched compact pattern");
        return Some(dt);
    }

    // Try IMG/VID prefix pattern
    if let Some(dt) = try_pattern_img(name) {
        trace!(filename, "Matched IMG pattern");
        return Some(dt);
    }

    // Try screenshot pattern
    if let Some(dt) = try_pattern_screenshot(name) {
        trace!(filename, "Matched screenshot pattern");
        return Some(dt);
    }

    // Try separated format
    if let Some(dt) = try_pattern_separated(name) {
        trace!(filename, "Matched separated pattern");
        return Some(dt);
    }

    // Try WhatsApp pattern
    if let Some(dt) = try_pattern_whatsapp(name) {
        trace!(filename, "Matched WhatsApp pattern");
        return Some(dt);
    }

    // Try Unix timestamp
    if let Some(dt) = try_pattern_unix(name) {
        trace!(filename, "Matched Unix timestamp pattern");
        return Some(dt);
    }

    // Try date only as last resort
    if let Some(dt) = try_pattern_date_only(name) {
        trace!(filename, "Matched date-only pattern");
        return Some(dt);
    }

    None
}

fn try_pattern_compact(s: &str) -> Option<NaiveDateTime> {
    let caps = get_pattern_compact().captures(s)?;
    build_datetime(
        caps.get(1)?.as_str(),
        caps.get(2)?.as_str(),
        caps.get(3)?.as_str(),
        caps.get(4)?.as_str(),
        caps.get(5)?.as_str(),
        caps.get(6)?.as_str(),
    )
}

fn try_pattern_img(s: &str) -> Option<NaiveDateTime> {
    let caps = get_pattern_img().captures(s)?;
    build_datetime(
        caps.get(1)?.as_str(),
        caps.get(2)?.as_str(),
        caps.get(3)?.as_str(),
        caps.get(4)?.as_str(),
        caps.get(5)?.as_str(),
        caps.get(6)?.as_str(),
    )
}

fn try_pattern_screenshot(s: &str) -> Option<NaiveDateTime> {
    let caps = get_pattern_screenshot().captures(s)?;
    build_datetime(
        caps.get(1)?.as_str(),
        caps.get(2)?.as_str(),
        caps.get(3)?.as_str(),
        caps.get(4)?.as_str(),
        caps.get(5)?.as_str(),
        caps.get(6)?.as_str(),
    )
}

fn try_pattern_separated(s: &str) -> Option<NaiveDateTime> {
    let caps = get_pattern_separated().captures(s)?;
    build_datetime(
        caps.get(1)?.as_str(),
        caps.get(2)?.as_str(),
        caps.get(3)?.as_str(),
        caps.get(4)?.as_str(),
        caps.get(5)?.as_str(),
        caps.get(6)?.as_str(),
    )
}

fn try_pattern_whatsapp(s: &str) -> Option<NaiveDateTime> {
    let caps = get_pattern_whatsapp().captures(s)?;
    build_datetime(
        caps.get(1)?.as_str(),
        caps.get(2)?.as_str(),
        caps.get(3)?.as_str(),
        "00",
        "00",
        "00",
    )
}

fn try_pattern_unix(s: &str) -> Option<NaiveDateTime> {
    let caps = get_pattern_unix().captures(s)?;
    let timestamp_str = caps.get(1)?.as_str();
    let timestamp: i64 = timestamp_str.parse().ok()?;

    // Handle millisecond timestamps
    let timestamp = if timestamp_str.len() == 13 {
        timestamp / 1000
    } else {
        timestamp
    };

    // Validate reasonable timestamp range (1990-2100)
    if !(631152000..=4102444800).contains(&timestamp) {
        return None;
    }

    chrono::DateTime::from_timestamp(timestamp, 0).map(|dt| dt.naive_utc())
}

fn try_pattern_date_only(s: &str) -> Option<NaiveDateTime> {
    let caps = get_pattern_date_only().captures(s)?;
    build_datetime(
        caps.get(1)?.as_str(),
        caps.get(2)?.as_str(),
        caps.get(3)?.as_str(),
        "00",
        "00",
        "00",
    )
}

fn build_datetime(
    year: &str,
    month: &str,
    day: &str,
    hour: &str,
    minute: &str,
    second: &str,
) -> Option<NaiveDateTime> {
    let year: i32 = year.parse().ok()?;
    let month: u32 = month.parse().ok()?;
    let day: u32 = day.parse().ok()?;
    let hour: u32 = hour.parse().ok()?;
    let minute: u32 = minute.parse().ok()?;
    let second: u32 = second.parse().ok()?;

    // Validate ranges
    if !(1990..=2100).contains(&year) {
        return None;
    }
    if !(1..=12).contains(&month) {
        return None;
    }
    if !(1..=31).contains(&day) {
        return None;
    }
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }

    chrono::NaiveDate::from_ymd_opt(year, month, day)?.and_hms_opt(hour, minute, second)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_compact_format() {
        let dt = parse_filename_time("20240115_143000.jpg").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 14);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 0);

        let dt = parse_filename_time("20240115-143000.jpg").unwrap();
        assert_eq!(dt.year(), 2024);
    }

    #[test]
    fn test_img_format() {
        let dt = parse_filename_time("IMG_20240115_143000.jpg").unwrap();
        assert_eq!(dt.year(), 2024);

        let dt = parse_filename_time("VID_20240115_143000.mp4").unwrap();
        assert_eq!(dt.year(), 2024);

        let dt = parse_filename_time("DSC_20240115_143000.jpg").unwrap();
        assert_eq!(dt.year(), 2024);
    }

    #[test]
    fn test_unix_timestamp() {
        // 2024-01-15 14:30:00 UTC
        let dt = parse_filename_time("photo_1705329000.jpg").unwrap();
        assert_eq!(dt.year(), 2024);

        // Millisecond timestamp
        let dt = parse_filename_time("photo_1705329000000.jpg").unwrap();
        assert_eq!(dt.year(), 2024);
    }

    #[test]
    fn test_whatsapp_format() {
        let dt = parse_filename_time("IMG-20240115-WA0001.jpg").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_separated_format() {
        let dt = parse_filename_time("2024-01-15_14-30-00.jpg").unwrap();
        assert_eq!(dt.year(), 2024);
    }

    #[test]
    fn test_invalid_formats() {
        assert!(parse_filename_time("random_file.jpg").is_none());
        assert!(parse_filename_time("photo.jpg").is_none());
        assert!(parse_filename_time("19800101_000000.jpg").is_none()); // Too old
    }
}
