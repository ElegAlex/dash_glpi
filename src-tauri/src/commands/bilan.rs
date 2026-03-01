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

    Ok(bilan)
}

#[tauri::command]
pub async fn get_bilan_temporel(
    state: tauri::State<'_, AppState>,
    request: BilanRequest,
) -> Result<BilanTemporel, String> {
    run_bilan_logic(&state, &request)
}
