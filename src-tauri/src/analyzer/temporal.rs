use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime};

/// Determines granularity automatically based on number of days in range.
/// < 30 days → "week", < 365 days → "month", else → "quarter"
pub fn auto_granularity(days: i64) -> String {
    if days <= 14 {
        "day".to_string()
    } else if days < 30 {
        "week".to_string()
    } else if days < 365 {
        "month".to_string()
    } else {
        "quarter".to_string()
    }
}

/// Generates (period_key, period_label, period_start, period_end) for each sub-period
/// between date_from and date_to with the given granularity.
/// Granularities: "week", "month", "quarter".
pub fn generate_period_keys(
    date_from: NaiveDateTime,
    date_to: NaiveDateTime,
    granularity: &str,
) -> Vec<(String, String, NaiveDateTime, NaiveDateTime)> {
    match granularity {
        "day" => generate_day_keys(date_from, date_to),
        "week" => generate_week_keys(date_from, date_to),
        "quarter" => generate_quarter_keys(date_from, date_to),
        _ => generate_month_keys(date_from, date_to),
    }
}

fn generate_day_keys(
    date_from: NaiveDateTime,
    date_to: NaiveDateTime,
) -> Vec<(String, String, NaiveDateTime, NaiveDateTime)> {
    let mut result = Vec::new();
    let mut current = date_from.date();
    let end = date_to.date();

    while current <= end {
        let period_key = current.format("%Y-%m-%d").to_string();
        let period_label = format!("{:02}/{:02}", current.day(), current.month());
        let start = current.and_hms_opt(0, 0, 0).unwrap();
        let end_dt = current.and_hms_opt(23, 59, 59).unwrap();
        result.push((period_key, period_label, start, end_dt));
        current += Duration::days(1);
    }

    result
}

fn generate_week_keys(
    date_from: NaiveDateTime,
    date_to: NaiveDateTime,
) -> Vec<(String, String, NaiveDateTime, NaiveDateTime)> {
    let mut result = Vec::new();
    let start_date = date_from.date();
    let end_date = date_to.date();

    // Find the Monday of the week containing date_from
    let days_from_monday = start_date.weekday().num_days_from_monday() as i64;
    let mut current_monday = start_date - Duration::days(days_from_monday);

    while current_monday <= end_date {
        let iw = current_monday.iso_week();
        let week_num = iw.week();
        let iso_year = iw.year();
        let period_key = format!("{:04}-W{:02}", iso_year, week_num);
        let period_label = format!("Sem. {}", week_num);
        let sunday = current_monday + Duration::days(6);

        let start = current_monday.and_hms_opt(0, 0, 0).unwrap();
        let end = sunday.and_hms_opt(23, 59, 59).unwrap();

        result.push((period_key, period_label, start, end));
        current_monday = current_monday + Duration::days(7);
    }

    result
}

fn generate_month_keys(
    date_from: NaiveDateTime,
    date_to: NaiveDateTime,
) -> Vec<(String, String, NaiveDateTime, NaiveDateTime)> {
    let mut result = Vec::new();
    let mut year = date_from.year();
    let mut month = date_from.month();
    let end_year = date_to.year();
    let end_month = date_to.month();

    loop {
        if year > end_year || (year == end_year && month > end_month) {
            break;
        }

        let period_key = format!("{:04}-{:02}", year, month);
        let period_label = format!("{} {}", french_month_name(month), year);

        let start = NaiveDate::from_ymd_opt(year, month, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let next_month_first = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
        };
        let last_day = next_month_first - Duration::days(1);
        let end = last_day.and_hms_opt(23, 59, 59).unwrap();

        result.push((period_key, period_label, start, end));

        if month == 12 {
            year += 1;
            month = 1;
        } else {
            month += 1;
        }
    }

    result
}

fn generate_quarter_keys(
    date_from: NaiveDateTime,
    date_to: NaiveDateTime,
) -> Vec<(String, String, NaiveDateTime, NaiveDateTime)> {
    let mut result = Vec::new();
    let mut year = date_from.year();
    let mut quarter = (date_from.month() - 1) / 3 + 1;
    let end_year = date_to.year();
    let end_quarter = (date_to.month() - 1) / 3 + 1;

    loop {
        if year > end_year || (year == end_year && quarter > end_quarter) {
            break;
        }

        let period_key = format!("{:04}-Q{}", year, quarter);
        let period_label = format!("T{} {}", quarter, year);

        let start_month = (quarter - 1) * 3 + 1;
        let end_month = quarter * 3;

        let start = NaiveDate::from_ymd_opt(year, start_month, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let next_start = if end_month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(year, end_month + 1, 1).unwrap()
        };
        let last_day = next_start - Duration::days(1);
        let end = last_day.and_hms_opt(23, 59, 59).unwrap();

        result.push((period_key, period_label, start, end));

        if quarter == 4 {
            year += 1;
            quarter = 1;
        } else {
            quarter += 1;
        }
    }

    result
}

fn french_month_name(month: u32) -> &'static str {
    match month {
        1 => "Janvier",
        2 => "Février",
        3 => "Mars",
        4 => "Avril",
        5 => "Mai",
        6 => "Juin",
        7 => "Juillet",
        8 => "Août",
        9 => "Septembre",
        10 => "Octobre",
        11 => "Novembre",
        12 => "Décembre",
        _ => "Inconnu",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    #[test]
    fn test_auto_granularity_day() {
        assert_eq!(auto_granularity(1), "day");
        assert_eq!(auto_granularity(7), "day");
        assert_eq!(auto_granularity(14), "day");
    }

    #[test]
    fn test_auto_granularity_week() {
        assert_eq!(auto_granularity(15), "week");
        assert_eq!(auto_granularity(29), "week");
    }

    #[test]
    fn test_auto_granularity_month() {
        assert_eq!(auto_granularity(30), "month");
        assert_eq!(auto_granularity(100), "month");
        assert_eq!(auto_granularity(364), "month");
    }

    #[test]
    fn test_auto_granularity_quarter() {
        assert_eq!(auto_granularity(365), "quarter");
        assert_eq!(auto_granularity(1000), "quarter");
    }

    #[test]
    fn test_generate_month_keys_three_months() {
        let from = dt("2026-01-01 00:00:00");
        let to = dt("2026-03-31 23:59:59");
        let keys = generate_period_keys(from, to, "month");
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].0, "2026-01");
        assert_eq!(keys[0].1, "Janvier 2026");
        assert_eq!(keys[1].0, "2026-02");
        assert_eq!(keys[1].1, "Février 2026");
        assert_eq!(keys[2].0, "2026-03");
        assert_eq!(keys[2].1, "Mars 2026");
    }

    #[test]
    fn test_generate_month_keys_year_boundary() {
        let from = dt("2025-11-01 00:00:00");
        let to = dt("2026-02-28 23:59:59");
        let keys = generate_period_keys(from, to, "month");
        assert_eq!(keys.len(), 4);
        assert_eq!(keys[0].0, "2025-11");
        assert_eq!(keys[3].0, "2026-02");
    }

    #[test]
    fn test_generate_quarter_keys_full_year() {
        let from = dt("2026-01-01 00:00:00");
        let to = dt("2026-12-31 23:59:59");
        let keys = generate_period_keys(from, to, "quarter");
        assert_eq!(keys.len(), 4);
        assert_eq!(keys[0].0, "2026-Q1");
        assert_eq!(keys[0].1, "T1 2026");
        assert_eq!(keys[1].0, "2026-Q2");
        assert_eq!(keys[1].1, "T2 2026");
        assert_eq!(keys[3].0, "2026-Q4");
        assert_eq!(keys[3].1, "T4 2026");
    }

    #[test]
    fn test_generate_quarter_keys_cross_year() {
        let from = dt("2025-10-01 00:00:00");
        let to = dt("2026-03-31 23:59:59");
        let keys = generate_period_keys(from, to, "quarter");
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].0, "2025-Q4");
        assert_eq!(keys[1].0, "2026-Q1");
    }

    #[test]
    fn test_generate_week_keys_two_weeks() {
        // 2026-W02: Mon 2026-01-05 → Sun 2026-01-11
        // 2026-W03: Mon 2026-01-12 → Sun 2026-01-18
        let from = dt("2026-01-05 00:00:00");
        let to = dt("2026-01-18 23:59:59");
        let keys = generate_period_keys(from, to, "week");
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].0, "2026-W02");
        assert_eq!(keys[0].1, "Sem. 2");
        assert_eq!(keys[1].0, "2026-W03");
        assert_eq!(keys[1].1, "Sem. 3");
    }

    #[test]
    fn test_generate_week_keys_starts_mid_week() {
        // Start Wednesday, should still include the full week from Monday
        let from = dt("2026-01-07 00:00:00"); // Wednesday of W02
        let to = dt("2026-01-11 23:59:59");   // Sunday of W02
        let keys = generate_period_keys(from, to, "week");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].0, "2026-W02");
    }

    #[test]
    fn test_period_start_end_bounds_month() {
        let from = dt("2026-01-01 00:00:00");
        let to = dt("2026-01-31 23:59:59");
        let keys = generate_period_keys(from, to, "month");
        assert_eq!(keys.len(), 1);
        // Start = Jan 1, end = Jan 31
        assert_eq!(keys[0].2, dt("2026-01-01 00:00:00"));
        assert_eq!(keys[0].3, dt("2026-01-31 23:59:59"));
    }
}
