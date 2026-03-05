use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

// -- French formats --
const FRENCH_DT_FMT: &str = "%d-%m-%Y %H:%M";
const FRENCH_DT_FMT_SLASH: &str = "%d/%m/%Y %H:%M";

// -- ISO 8601 formats --
const ISO_DT_FMT_T: &str = "%Y-%m-%dT%H:%M:%S";
const ISO_DT_FMT: &str = "%Y-%m-%d %H:%M:%S";
const ISO_DT_FMT_NO_SEC: &str = "%Y-%m-%d %H:%M";
const ISO_DATE_ONLY: &str = "%Y-%m-%d";

/// Excel epoch: December 30, 1899 (accounting for the Lotus 1-2-3 leap year bug).
const EXCEL_EPOCH: Option<NaiveDate> = NaiveDate::from_ymd_opt(1899, 12, 30);

/// Parse an Excel serial date number ("44914,39167" or "44914.39167" → NaiveDateTime).
/// The integer part = days since 1899-12-30, the fractional part = fraction of day.
/// Accepts comma (French) or dot (English) as decimal separator.
fn parse_excel_serial(s: &str) -> Option<NaiveDateTime> {
    let normalized = s.replace(',', ".");
    let serial: f64 = normalized.parse().ok()?;
    if serial < 1.0 || serial > 2_958_465.0 {
        return None;
    }
    let days = serial.trunc() as i64;
    let frac = serial.fract();
    let date = EXCEL_EPOCH? + chrono::Duration::days(days);
    let total_secs = (frac * 86400.0).round() as u32;
    let time = NaiveTime::from_num_seconds_from_midnight_opt(total_secs, 0)?;
    Some(NaiveDateTime::new(date, time))
}

/// Parse a datetime string into NaiveDateTime.
/// Supports (in priority order):
/// 1. French: `DD-MM-YYYY HH:MM`
/// 2. French: `DD/MM/YYYY HH:MM`
/// 3. ISO 8601: `YYYY-MM-DDTHH:MM:SS`
/// 4. ISO 8601: `YYYY-MM-DD HH:MM:SS`
/// 5. ISO 8601: `YYYY-MM-DD HH:MM`
/// 6. ISO date only: `YYYY-MM-DD` (time set to 00:00:00)
/// 7. Excel serial number: `44914,39167` or `44914.39167`
/// Returns None for empty or unparseable strings.
pub fn parse_french_datetime(s: &str) -> Option<NaiveDateTime> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }

    // French formats (most common in GLPI exports)
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, FRENCH_DT_FMT) {
        return Some(dt);
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, FRENCH_DT_FMT_SLASH) {
        return Some(dt);
    }

    // ISO 8601 formats
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, ISO_DT_FMT_T) {
        return Some(dt);
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, ISO_DT_FMT) {
        return Some(dt);
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, ISO_DT_FMT_NO_SEC) {
        return Some(dt);
    }

    // ISO date only → 00:00:00
    if let Ok(d) = NaiveDate::parse_from_str(trimmed, ISO_DATE_ONLY) {
        return d.and_hms_opt(0, 0, 0);
    }

    // Excel serial number (comma or dot decimal)
    parse_excel_serial(trimmed)
}

/// Parse an ID string that may contain non-breaking spaces ("5 732 943" → 5732943).
pub fn parse_spaced_i64(s: &str) -> Option<i64> {
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    cleaned.parse::<i64>().ok()
}

/// Parse an optional integer from a string ("" → None, "5" → Some(5)).
pub fn parse_opt_i32(s: &str) -> Option<i32> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<i32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- French formats --

    #[test]
    fn test_parse_french_datetime() {
        let dt = parse_french_datetime("05-01-2026 16:24").unwrap();
        assert_eq!(dt.format("%Y-%m-%dT%H:%M:%S").to_string(), "2026-01-05T16:24:00");
    }

    #[test]
    fn test_parse_french_datetime_slash() {
        let dt = parse_french_datetime("06/02/2026 16:18").unwrap();
        assert_eq!(dt.format("%Y-%m-%dT%H:%M:%S").to_string(), "2026-02-06T16:18:00");
    }

    #[test]
    fn test_parse_french_datetime_empty() {
        assert!(parse_french_datetime("").is_none());
        assert!(parse_french_datetime("   ").is_none());
    }

    // -- ISO 8601 formats --

    #[test]
    fn test_parse_iso_datetime_t() {
        let dt = parse_french_datetime("2026-01-05T16:24:00").unwrap();
        assert_eq!(dt.format("%d-%m-%Y %H:%M").to_string(), "05-01-2026 16:24");
    }

    #[test]
    fn test_parse_iso_datetime_space() {
        let dt = parse_french_datetime("2026-01-05 16:24:00").unwrap();
        assert_eq!(dt.format("%d-%m-%Y %H:%M").to_string(), "05-01-2026 16:24");
    }

    #[test]
    fn test_parse_iso_datetime_no_seconds() {
        let dt = parse_french_datetime("2026-01-05 16:24").unwrap();
        assert_eq!(dt.format("%d-%m-%Y %H:%M").to_string(), "05-01-2026 16:24");
    }

    #[test]
    fn test_parse_iso_date_only() {
        let dt = parse_french_datetime("2026-01-05").unwrap();
        assert_eq!(dt.format("%Y-%m-%dT%H:%M:%S").to_string(), "2026-01-05T00:00:00");
    }

    // -- Excel serial numbers --

    #[test]
    fn test_parse_excel_serial_comma() {
        // 44914,39167 → 2022-12-19 09:24
        let dt = parse_french_datetime("44914,39167").unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2022-12-19");
        assert_eq!(dt.format("%H:%M").to_string(), "09:24");
    }

    #[test]
    fn test_parse_excel_serial_dot() {
        // Same with dot decimal (English Excel)
        let dt = parse_french_datetime("44914.39167").unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2022-12-19");
        assert_eq!(dt.format("%H:%M").to_string(), "09:24");
    }

    #[test]
    fn test_parse_excel_serial_integer_only() {
        // 44927 → 2023-01-01 00:00:00
        let dt = parse_french_datetime("44927").unwrap();
        assert_eq!(dt.format("%Y-%m-%d %H:%M").to_string(), "2023-01-01 00:00");
    }

    #[test]
    fn test_parse_excel_serial_46080() {
        // 46080 → 2026-02-27 (not 2026-03-03)
        let dt = parse_french_datetime("46080,55278").unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2026-02-27");
    }

    // -- Utility parsers --

    #[test]
    fn test_parse_spaced_i64() {
        assert_eq!(parse_spaced_i64("5 732 943"), Some(5_732_943));
        assert_eq!(parse_spaced_i64("5\u{00A0}732\u{00A0}943"), Some(5_732_943));
        assert_eq!(parse_spaced_i64("123"), Some(123));
        assert_eq!(parse_spaced_i64("abc"), None);
    }

    #[test]
    fn test_parse_opt_i32() {
        assert_eq!(parse_opt_i32(""), None);
        assert_eq!(parse_opt_i32("   "), None);
        assert_eq!(parse_opt_i32("3"), Some(3));
        assert_eq!(parse_opt_i32("invalid"), None);
    }
}
