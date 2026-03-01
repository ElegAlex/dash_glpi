use chrono::NaiveDateTime;

const FRENCH_DT_FMT: &str = "%d-%m-%Y %H:%M";

/// Parse a French datetime string (DD-MM-YYYY HH:MM) into NaiveDateTime.
/// Returns None for empty or unparseable strings.
pub fn parse_french_datetime(s: &str) -> Option<NaiveDateTime> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    NaiveDateTime::parse_from_str(trimmed, FRENCH_DT_FMT).ok()
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

/// Serde-compatible deserializers for use with `#[serde(deserialize_with = "de::...")]`.
pub mod de {
    use chrono::NaiveDateTime;
    use serde::{self, Deserialize, Deserializer};

    const FRENCH_DT_FMT: &str = "%d-%m-%Y %H:%M";

    /// "05-01-2026 16:24" → NaiveDateTime (obligatoire)
    pub fn french_datetime<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDateTime::parse_from_str(s.trim(), FRENCH_DT_FMT)
            .map_err(serde::de::Error::custom)
    }

    /// "05-01-2026 16:24" → Some(NaiveDateTime), "" → None
    pub fn french_datetime_opt<'de, D>(deserializer: D) -> Result<Option<NaiveDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        NaiveDateTime::parse_from_str(trimmed, FRENCH_DT_FMT)
            .map(Some)
            .map_err(serde::de::Error::custom)
    }

    /// "5 732 943" → 5732943u64 (supprime tous les espaces et espaces insécables)
    pub fn spaced_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        cleaned.parse::<u64>().map_err(serde::de::Error::custom)
    }

    /// "" → None, "5" → Some(5)
    pub fn opt_u32_empty<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        trimmed
            .parse::<u32>()
            .map(Some)
            .map_err(serde::de::Error::custom)
    }

    /// "" → None, "3" → Some(3)
    pub fn opt_u8_empty<'de, D>(deserializer: D) -> Result<Option<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        trimmed
            .parse::<u8>()
            .map(Some)
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_french_datetime() {
        let dt = parse_french_datetime("05-01-2026 16:24").unwrap();
        assert_eq!(dt.format("%Y-%m-%dT%H:%M:%S").to_string(), "2026-01-05T16:24:00");
    }

    #[test]
    fn test_parse_french_datetime_empty() {
        assert!(parse_french_datetime("").is_none());
        assert!(parse_french_datetime("   ").is_none());
    }

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
