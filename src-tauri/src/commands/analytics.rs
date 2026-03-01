use serde::Serialize;
use tauri::State;

use crate::analytics::prediction;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastPointIpc {
    pub period: String,
    pub predicted_value: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PredictionResult {
    pub forecasts: Vec<ForecastPointIpc>,
    pub model_info: String,
    pub mae: f64,
    pub history_length: usize,
}

/// Prédit la charge future à partir du flux entrant quotidien de l'import actif.
///
/// Retourne une erreur si moins de 90 jours de données sont disponibles.
#[tauri::command]
pub async fn predict_workload(
    state: State<'_, AppState>,
    periods_ahead: usize,
) -> Result<PredictionResult, String> {
    // Charge la série temporelle depuis la DB
    let (values, labels) = {
        let guard = state
            .db
            .lock()
            .map_err(|e| format!("Lock error: {e}"))?;
        let conn = guard
            .as_ref()
            .ok_or("Base de données non initialisée")?;

        // Récupère l'import actif
        let import_id: i64 = conn
            .query_row(
                "SELECT id FROM imports WHERE is_active = 1 ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .map_err(|_| "Aucun import actif trouvé")?;

        // Flux entrants quotidiens groupés par date_ouverture
        let mut stmt = conn
            .prepare(
                "SELECT DATE(date_ouverture) AS jour, COUNT(*) AS nb \
                 FROM tickets \
                 WHERE import_id = ?1 \
                 GROUP BY jour \
                 ORDER BY jour ASC",
            )
            .map_err(|e| format!("Erreur préparation requête: {e}"))?;

        let rows: Vec<(String, f64)> = stmt
            .query_map([import_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            })
            .map_err(|e| format!("Erreur requête: {e}"))?
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Erreur lecture résultats: {e}"))?;

        let labels: Vec<String> = rows.iter().map(|(d, _)| d.clone()).collect();
        let values: Vec<f64> = rows.iter().map(|(_, n)| *n).collect();
        (values, labels)
    };

    if values.len() < 90 {
        return Err(format!(
            "Historique insuffisant (minimum 90 jours requis, {} disponibles)",
            values.len()
        ));
    }

    let periods = if periods_ahead == 0 { 30 } else { periods_ahead };

    let input = prediction::TimeSeriesInput { values, period_labels: labels };
    let output = prediction::predict_workload(&input, periods, 7)?;

    let forecasts = output
        .forecasts
        .into_iter()
        .map(|fp| ForecastPointIpc {
            period: fp.period_label,
            predicted_value: fp.predicted_value,
            lower_bound: fp.lower_bound,
            upper_bound: fp.upper_bound,
        })
        .collect();

    Ok(PredictionResult {
        forecasts,
        model_info: output.model_info,
        mae: output.mae,
        history_length: output.history_length,
    })
}
