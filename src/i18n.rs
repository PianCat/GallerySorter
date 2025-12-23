//! Internationalization (i18n) module
//!
//! Provides language detection and localized strings for the CLI interface.
//! Supports English and Chinese Simplified.
//! Note: Log messages remain in English for consistency.

use rust_i18n::set_locale;
use sys_locale::get_locale;

/// Initialize the locale based on system settings
/// Returns the detected locale code
pub fn init_locale() -> String {
    let locale = detect_locale();
    set_locale(&locale);
    locale
}

/// Detect the system locale and map to supported languages
/// Supports: en (English), zh-CN (Simplified Chinese)
fn detect_locale() -> String {
    let system_locale = get_locale().unwrap_or_else(|| "en".to_string());

    // Check if the locale starts with "zh" for Chinese
    if system_locale.starts_with("zh") {
        "zh-CN".to_string()
    } else {
        // Default to English for all other languages
        "en".to_string()
    }
}

/// Get the current locale
pub fn current_locale() -> String {
    rust_i18n::locale().to_string()
}

/// Check if current locale is Chinese
pub fn is_chinese() -> bool {
    rust_i18n::locale().starts_with("zh")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_detection() {
        let locale = detect_locale();
        assert!(locale == "en" || locale == "zh-CN");
    }

    #[test]
    fn test_init_locale() {
        let locale = init_locale();
        assert!(!locale.is_empty());
    }
}
