use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::analyzer::bilan::{compute_bilan, compute_ventilation};
use crate::analyzer::temporal::{auto_granularity, generate_period_keys};
use crate::db::queries;
use crate::state::{AppState, DbAccess};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BilanRequest {
    pub period: String,
    pub date_from: String,
    pub date_to: String,
    pub group_by: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BilanTemporel {
    pub periodes: Vec<PeriodData>,
    pub totaux: BilanTotaux,
    pub ventilation: Option<Vec<BilanVentilation>>,
    pub resolution: Option<BilanResolution>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BilanResolution {
    pub tranches: Vec<ResolutionTranche>,
    pub total_resolus: usize,
    pub mttr_jours: f64,
    pub mediane_jours: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionTranche {
    pub label: String,
    pub count: usize,
    pub pourcentage: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeriodData {
    pub period_key: String,
    pub period_label: String,
    pub entrees: usize,
    pub sorties: usize,
    pub delta: i64,
    pub stock_cumule: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BilanTotaux {
    pub total_entrees: usize,
    pub total_sorties: usize,
    pub delta_global: i64,
    pub moyenne_entrees_par_periode: f64,
    pub moyenne_sorties_par_periode: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BilanVentilation {
    pub label: String,
    pub entrees: usize,
    pub sorties: usize,
    pub delta: i64,
}

fn parse_date_flexible(s: &str) -> Option<NaiveDateTime> {
    for fmt in &["%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%SZ"] {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(dt);
        }
    }
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
}

/// Shared bilan computation logic — usable from both the IPC command and the export command.
pub(crate) fn run_bilan_logic(state: &AppState, request: &BilanRequest) -> Result<BilanTemporel, String> {
    let date_from = parse_date_flexible(&request.date_from)
        .ok_or_else(|| format!("Date de début invalide: {}", request.date_from))?;
    let date_to = parse_date_flexible(&request.date_to)
        .ok_or_else(|| format!("Date de fin invalide: {}", request.date_to))?;

    let days = date_to.signed_duration_since(date_from).num_days();
    let granularity = if request.period == "auto" || request.period.is_empty() {
        auto_granularity(days)
    } else {
        request.period.clone()
    };

    let period_keys = generate_period_keys(date_from, date_to, &granularity);

    let from_str = date_from.format("%Y-%m-%d").to_string();
    let to_str = date_to.format("%Y-%m-%d").to_string();

    let stock_debut = state.db(|conn| queries::get_stock_at_date(conn, &from_str))?;

    let entrees = state.db(|conn| {
        queries::get_bilan_entrees_par_periode(conn, &from_str, &to_str, &granularity, None)
    })?;

    let sorties = state.db(|conn| {
        queries::get_bilan_sorties_par_periode(conn, &from_str, &to_str, &granularity, None)
    })?;

    let mut bilan = compute_bilan(&entrees, &sorties, &period_keys, stock_debut);

    if let Some(ref group_by) = request.group_by {
        let vent_data = if group_by == "technicien" {
            state.db(|conn| {
                queries::get_bilan_ventilation_par_technicien(conn, &from_str, &to_str)
            })?
        } else {
            state.db(|conn| {
                queries::get_bilan_ventilation_par_groupe(conn, &from_str, &to_str)
            })?
        };
        bilan.ventilation = Some(compute_ventilation(&vent_data));
    }

    // Resolution distribution
    let durations = state.db(|conn| {
        queries::get_resolution_durations(conn, &from_str, &to_str)
    })?;
    if !durations.is_empty() {
        bilan.resolution = Some(build_resolution_distribution(&durations));
    }

    Ok(bilan)
}

fn build_resolution_distribution(durations: &[f64]) -> BilanResolution {
    let total = durations.len();
    let pct = |n: usize| -> f64 {
        if total == 0 { 0.0 } else { (n as f64 / total as f64 * 1000.0).round() / 10.0 }
    };

    let mut lt24h = 0usize;
    let mut lt48h = 0usize;
    let mut lt7j = 0usize;
    let mut lt30j = 0usize;
    let mut ge30j = 0usize;

    for &d in durations {
        if d < 1.0 {
            lt24h += 1;
        } else if d < 2.0 {
            lt48h += 1;
        } else if d < 7.0 {
            lt7j += 1;
        } else if d < 30.0 {
            lt30j += 1;
        } else {
            ge30j += 1;
        }
    }

    let tranches = vec![
        ResolutionTranche { label: "< 24h".to_string(), count: lt24h, pourcentage: pct(lt24h) },
        ResolutionTranche { label: "24h - 48h".to_string(), count: lt48h, pourcentage: pct(lt48h) },
        ResolutionTranche { label: "2j - 7j".to_string(), count: lt7j, pourcentage: pct(lt7j) },
        ResolutionTranche { label: "7j - 30j".to_string(), count: lt30j, pourcentage: pct(lt30j) },
        ResolutionTranche { label: "> 30j".to_string(), count: ge30j, pourcentage: pct(ge30j) },
    ];

    let sum: f64 = durations.iter().sum();
    let mttr = if total > 0 { (sum / total as f64 * 10.0).round() / 10.0 } else { 0.0 };

    let mut sorted = durations.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mediane = if sorted.is_empty() {
        0.0
    } else if sorted.len() % 2 == 0 {
        let mid = sorted.len() / 2;
        ((sorted[mid - 1] + sorted[mid]) / 2.0 * 10.0).round() / 10.0
    } else {
        (sorted[sorted.len() / 2] * 10.0).round() / 10.0
    };

    BilanResolution {
        tranches,
        total_resolus: total,
        mttr_jours: mttr,
        mediane_jours: mediane,
    }
}

#[tauri::command]
pub async fn get_bilan_temporel(
    state: tauri::State<'_, AppState>,
    request: BilanRequest,
) -> Result<BilanTemporel, String> {
    run_bilan_logic(&state, &request)
}
