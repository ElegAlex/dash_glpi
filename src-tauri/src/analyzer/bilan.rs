use std::collections::HashMap;

use chrono::NaiveDateTime;

use crate::commands::bilan::{BilanTemporel, BilanTotaux, BilanVentilation, PeriodData};

/// Assembles BilanTemporel from pre-aggregated entry/exit data and period keys.
///
/// - `entrees`: (period_key, count) for ticket creations
/// - `sorties`: (period_key, count) for ticket resolutions/closures
/// - `period_keys`: ordered list of (key, label, start, end) from temporal::generate_period_keys
/// - `stock_debut`: stock at the start of the first period
///
/// Periods with no data get entrees=0, sorties=0 (RG-038, no gaps in series).
/// stock_cumule is computed progressively: stock_debut + Σ(entrees - sorties).
pub fn compute_bilan(
    entrees: &[(String, usize)],
    sorties: &[(String, usize)],
    period_keys: &[(String, String, NaiveDateTime, NaiveDateTime)],
    stock_debut: usize,
) -> BilanTemporel {
    let entrees_map: HashMap<&str, usize> = entrees
        .iter()
        .map(|(k, v)| (k.as_str(), *v))
        .collect();
    let sorties_map: HashMap<&str, usize> = sorties
        .iter()
        .map(|(k, v)| (k.as_str(), *v))
        .collect();

    let mut periodes = Vec::with_capacity(period_keys.len());
    let mut stock_running = stock_debut as i64;

    for (period_key, period_label, _start, _end) in period_keys {
        let e = entrees_map.get(period_key.as_str()).copied().unwrap_or(0);
        let s = sorties_map.get(period_key.as_str()).copied().unwrap_or(0);
        let delta = e as i64 - s as i64;

        stock_running = (stock_running + delta).max(0);

        periodes.push(PeriodData {
            period_key: period_key.clone(),
            period_label: period_label.clone(),
            entrees: e,
            sorties: s,
            delta,
            stock_cumule: Some(stock_running as usize),
        });
    }

    let n = periodes.len();
    let total_entrees: usize = periodes.iter().map(|p| p.entrees).sum();
    let total_sorties: usize = periodes.iter().map(|p| p.sorties).sum();
    let delta_global = total_entrees as i64 - total_sorties as i64;

    let moyenne_entrees_par_periode = if n > 0 {
        total_entrees as f64 / n as f64
    } else {
        0.0
    };
    let moyenne_sorties_par_periode = if n > 0 {
        total_sorties as f64 / n as f64
    } else {
        0.0
    };

    BilanTemporel {
        periodes,
        totaux: BilanTotaux {
            total_entrees,
            total_sorties,
            delta_global,
            moyenne_entrees_par_periode,
            moyenne_sorties_par_periode,
        },
        ventilation: None,
    }
}

/// Builds ventilation breakdown from (label, entrees, sorties) tuples.
pub fn compute_ventilation(data: &[(String, usize, usize)]) -> Vec<BilanVentilation> {
    data.iter()
        .map(|(label, entrees, sorties)| BilanVentilation {
            label: label.clone(),
            entrees: *entrees,
            sorties: *sorties,
            delta: *entrees as i64 - *sorties as i64,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::temporal::generate_period_keys;

    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    #[test]
    fn test_no_gaps_in_series_rg038() {
        let from = dt("2026-01-01 00:00:00");
        let to = dt("2026-03-31 23:59:59");
        let keys = generate_period_keys(from, to, "month");

        // Only January has data
        let entrees = vec![("2026-01".to_string(), 10usize)];
        let sorties = vec![("2026-01".to_string(), 5usize)];

        let bilan = compute_bilan(&entrees, &sorties, &keys, 0);

        assert_eq!(bilan.periodes.len(), 3);
        // February and March must appear with 0
        assert_eq!(bilan.periodes[1].entrees, 0);
        assert_eq!(bilan.periodes[1].sorties, 0);
        assert_eq!(bilan.periodes[2].entrees, 0);
        assert_eq!(bilan.periodes[2].sorties, 0);
    }

    #[test]
    fn test_stock_cumule_progressif() {
        let from = dt("2026-01-01 00:00:00");
        let to = dt("2026-03-31 23:59:59");
        let keys = generate_period_keys(from, to, "month");

        let entrees = vec![
            ("2026-01".to_string(), 10usize),
            ("2026-02".to_string(), 5usize),
            ("2026-03".to_string(), 8usize),
        ];
        let sorties = vec![
            ("2026-01".to_string(), 3usize),
            ("2026-02".to_string(), 7usize),
            ("2026-03".to_string(), 4usize),
        ];

        let bilan = compute_bilan(&entrees, &sorties, &keys, 100);

        // Jan: 100 + (10 - 3) = 107
        assert_eq!(bilan.periodes[0].stock_cumule, Some(107));
        // Feb: 107 + (5 - 7) = 105
        assert_eq!(bilan.periodes[1].stock_cumule, Some(105));
        // Mar: 105 + (8 - 4) = 109
        assert_eq!(bilan.periodes[2].stock_cumule, Some(109));
    }

    #[test]
    fn test_stock_cumule_clamped_at_zero() {
        let from = dt("2026-01-01 00:00:00");
        let to = dt("2026-01-31 23:59:59");
        let keys = generate_period_keys(from, to, "month");

        // More sorties than entrees + stock_debut → clamp at 0
        let entrees = vec![("2026-01".to_string(), 0usize)];
        let sorties = vec![("2026-01".to_string(), 50usize)];

        let bilan = compute_bilan(&entrees, &sorties, &keys, 10);

        assert_eq!(bilan.periodes[0].stock_cumule, Some(0));
    }

    #[test]
    fn test_totaux_correctness() {
        let from = dt("2026-01-01 00:00:00");
        let to = dt("2026-02-28 23:59:59");
        let keys = generate_period_keys(from, to, "month");

        let entrees = vec![
            ("2026-01".to_string(), 10usize),
            ("2026-02".to_string(), 6usize),
        ];
        let sorties = vec![
            ("2026-01".to_string(), 4usize),
            ("2026-02".to_string(), 8usize),
        ];

        let bilan = compute_bilan(&entrees, &sorties, &keys, 0);

        assert_eq!(bilan.totaux.total_entrees, 16);
        assert_eq!(bilan.totaux.total_sorties, 12);
        assert_eq!(bilan.totaux.delta_global, 4);
        assert!((bilan.totaux.moyenne_entrees_par_periode - 8.0).abs() < f64::EPSILON);
        assert!((bilan.totaux.moyenne_sorties_par_periode - 6.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_delta_per_period() {
        let from = dt("2026-01-01 00:00:00");
        let to = dt("2026-01-31 23:59:59");
        let keys = generate_period_keys(from, to, "month");

        let entrees = vec![("2026-01".to_string(), 20usize)];
        let sorties = vec![("2026-01".to_string(), 15usize)];

        let bilan = compute_bilan(&entrees, &sorties, &keys, 0);

        assert_eq!(bilan.periodes[0].delta, 5);
    }

    #[test]
    fn test_empty_periods() {
        let from = dt("2026-01-01 00:00:00");
        let to = dt("2026-03-31 23:59:59");
        let keys = generate_period_keys(from, to, "month");

        let bilan = compute_bilan(&[], &[], &keys, 50);

        assert_eq!(bilan.periodes.len(), 3);
        assert_eq!(bilan.totaux.total_entrees, 0);
        assert_eq!(bilan.totaux.total_sorties, 0);
        // All periods should show stock_debut (no change)
        for p in &bilan.periodes {
            assert_eq!(p.stock_cumule, Some(50));
        }
    }

    #[test]
    fn test_compute_ventilation() {
        let data = vec![
            ("Alice".to_string(), 10usize, 8usize),
            ("Bob".to_string(), 5usize, 7usize),
        ];
        let vent = compute_ventilation(&data);
        assert_eq!(vent.len(), 2);
        assert_eq!(vent[0].label, "Alice");
        assert_eq!(vent[0].entrees, 10);
        assert_eq!(vent[0].sorties, 8);
        assert_eq!(vent[0].delta, 2);
        assert_eq!(vent[1].delta, -2);
    }
}
