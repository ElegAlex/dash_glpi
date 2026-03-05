use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::analyzer::temporal::{auto_granularity, generate_period_keys};
use crate::db::queries;
use crate::state::{AppState, DbAccess};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelaisRequest {
    pub date_from: String,
    pub date_to: String,
    pub granularity: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DelaisKpi {
    pub taux_24h: f64,
    pub taux_48h: f64,
    pub mttr_jours: f64,
    pub mediane_jours: f64,
    pub total_resolus: usize,
    pub trend: Vec<DelaisTrend>,
    pub distribution: Vec<TrancheDelai>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DelaisTrend {
    pub period_key: String,
    pub period_label: String,
    pub pct_24h: f64,
    pub pct_48h: f64,
    pub total_resolus: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrancheDelai {
    pub label: String,
    pub count: usize,
    pub pourcentage: f64,
}

fn parse_date_flexible(s: &str) -> Option<chrono::NaiveDateTime> {
    for fmt in &["%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%SZ"] {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, fmt) {
            return Some(dt);
        }
    }
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
}

fn pct(n: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        (n as f64 / total as f64 * 1000.0).round() / 10.0
    }
}

/// Period expression for SQL bucketing (replicates dashboard.rs logic).
fn period_expr(granularity: &str, date_col: &str) -> String {
    match granularity {
        "day" => format!("strftime('%Y-%m-%d', {date_col})"),
        "week" => format!("strftime('%Y-W%W', {date_col})"),
        "quarter" => format!(
            "(strftime('%Y', {date_col}) || '-Q' || ((CAST(strftime('%m', {date_col}) AS INTEGER) - 1) / 3 + 1))"
        ),
        "year" => format!("strftime('%Y', {date_col})"),
        _ => format!("strftime('%Y-%m', {date_col})"),
    }
}

#[tauri::command]
pub async fn get_delais_kpi(
    state: tauri::State<'_, AppState>,
    request: DelaisRequest,
) -> Result<DelaisKpi, String> {
    let date_from = parse_date_flexible(&request.date_from)
        .ok_or_else(|| format!("Date de début invalide: {}", request.date_from))?;
    let date_to = parse_date_flexible(&request.date_to)
        .ok_or_else(|| format!("Date de fin invalide: {}", request.date_to))?;

    let days = date_to.signed_duration_since(date_from).num_days();
    let granularity = match &request.granularity {
        Some(g) if !g.is_empty() && g != "auto" => g.clone(),
        _ => auto_granularity(days),
    };

    let from_str = date_from.format("%Y-%m-%d").to_string();
    let to_str = date_to.format("%Y-%m-%d").to_string();

    // Get all resolution durations for the period
    let durations = state.db(|conn| queries::get_resolution_durations(conn, &from_str, &to_str))?;

    // Global KPIs
    let total_resolus = durations.len();
    let lt24h_global = durations.iter().filter(|&&d| d >= 0.0 && d < 1.0).count();
    let lt48h_global = durations.iter().filter(|&&d| d >= 0.0 && d < 2.0).count();
    let taux_24h = pct(lt24h_global, total_resolus);
    let taux_48h = pct(lt48h_global, total_resolus);

    let positive: Vec<f64> = durations.iter().copied().filter(|&d| d >= 0.0).collect();
    let sum: f64 = positive.iter().sum();
    let mttr_jours = if positive.is_empty() {
        0.0
    } else {
        (sum / positive.len() as f64 * 10.0).round() / 10.0
    };

    let mediane_jours = if positive.is_empty() {
        0.0
    } else {
        let mut sorted = positive.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            ((sorted[mid - 1] + sorted[mid]) / 2.0 * 10.0).round() / 10.0
        } else {
            (sorted[mid] * 10.0).round() / 10.0
        }
    };

    // Distribution by tranches
    let mut lt24 = 0usize;
    let mut lt48 = 0usize;
    let mut lt7j = 0usize;
    let mut lt30j = 0usize;
    let mut ge30j = 0usize;
    for &d in &positive {
        if d < 1.0 {
            lt24 += 1;
        } else if d < 2.0 {
            lt48 += 1;
        } else if d < 7.0 {
            lt7j += 1;
        } else if d < 30.0 {
            lt30j += 1;
        } else {
            ge30j += 1;
        }
    }
    let distribution = vec![
        TrancheDelai { label: "< 24h".to_string(), count: lt24, pourcentage: pct(lt24, total_resolus) },
        TrancheDelai { label: "24h - 48h".to_string(), count: lt48, pourcentage: pct(lt48, total_resolus) },
        TrancheDelai { label: "2j - 7j".to_string(), count: lt7j, pourcentage: pct(lt7j, total_resolus) },
        TrancheDelai { label: "7j - 30j".to_string(), count: lt30j, pourcentage: pct(lt30j, total_resolus) },
        TrancheDelai { label: "> 30j".to_string(), count: ge30j, pourcentage: pct(ge30j, total_resolus) },
    ];

    // Trend by period: query individual durations with their period key
    let period_keys = generate_period_keys(date_from, date_to, &granularity);
    let trend = state.db(|conn| {
        let import_id = queries::get_active_import_id(conn)?;
        let pe = period_expr(&granularity, "date_cloture_approx");
        let sql = format!(
            "SELECT {pe} AS periode,
                    julianday(date_cloture_approx) - julianday(date_ouverture) AS dur
             FROM tickets
             WHERE import_id = ? AND est_vivant = 0
               AND date_cloture_approx IS NOT NULL AND date_cloture_approx != ''
               AND date_ouverture IS NOT NULL
               AND date_cloture_approx >= ? AND date_cloture_approx < date(?, '+1 day')
             ORDER BY periode"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(
            rusqlite::params![import_id, &from_str, &to_str],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)),
        )?;

        // Group by period
        let mut by_period: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
        for row in rows {
            let (p, dur) = row?;
            if dur >= 0.0 {
                let entry = by_period.entry(p).or_insert((0, 0, 0));
                entry.0 += 1; // total
                if dur < 1.0 {
                    entry.1 += 1; // <24h
                    entry.2 += 1; // <48h
                } else if dur < 2.0 {
                    entry.2 += 1; // <48h
                }
            }
        }

        // Build trend with proper labels from period_keys
        let label_map: std::collections::HashMap<&str, &str> = period_keys
            .iter()
            .map(|(key, label, _, _)| (key.as_str(), label.as_str()))
            .collect();

        let mut trend_vec = Vec::new();
        for (key, label, _, _) in &period_keys {
            let (total, lt24, lt48) = by_period.get(key.as_str()).copied().unwrap_or((0, 0, 0));
            trend_vec.push(DelaisTrend {
                period_key: key.clone(),
                period_label: label_map.get(key.as_str()).unwrap_or(&key.as_str()).to_string(),
                pct_24h: pct(lt24, total),
                pct_48h: pct(lt48, total),
                total_resolus: total,
            });
        }
        Ok(trend_vec)
    })?;

    Ok(DelaisKpi {
        taux_24h,
        taux_48h,
        mttr_jours,
        mediane_jours,
        total_resolus,
        trend,
        distribution,
    })
}
