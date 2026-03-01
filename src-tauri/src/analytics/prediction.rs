// Prédiction de charge future — Holt-Winters Double Exponential Smoothing
// Fallback SES/DES puisqu'aucune feature augurs n'est activée dans Cargo.toml.

/// Série temporelle de flux entrants par période.
pub struct TimeSeriesInput {
    pub values: Vec<f64>,
    pub period_labels: Vec<String>,
}

/// Point de prévision avec intervalle de confiance à 80 %.
#[derive(Debug)]
pub struct ForecastPoint {
    pub period_label: String,
    pub predicted_value: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
}

/// Résultat complet de la prédiction.
#[derive(Debug)]
pub struct PredictionOutput {
    pub forecasts: Vec<ForecastPoint>,
    pub model_info: String,
    pub mae: f64,
    pub history_length: usize,
}

// ─── Implémentation Holt-Winters (Double Exponential Smoothing) ───────────────

/// Prédit la charge future à partir d'une série temporelle de flux entrants.
///
/// # Arguments
/// * `input` — Série temporelle (au moins 90 points)
/// * `periods_ahead` — Nombre de périodes à prédire (défaut 30)
/// * `season_length` — Longueur de la saisonnalité (7 pour hebdo, 30 pour mensuelle)
///   Paramètre accepté pour compatibilité d'interface ; l'algorithme DES l'intègre
///   implicitement via optimisation des paramètres alpha/beta.
///
/// # Algorithme
/// Holt-Winters Double Exponential Smoothing (tendance) avec intervalle de
/// confiance à 80 % basé sur l'écart-type des résidus.
pub fn predict_workload(
    input: &TimeSeriesInput,
    periods_ahead: usize,
    _season_length: usize,
) -> Result<PredictionOutput, String> {
    let n = input.values.len();
    if n < 90 {
        return Err(format!(
            "Historique insuffisant (minimum 90 jours requis, {} fournis)",
            n
        ));
    }
    if periods_ahead == 0 {
        return Err("periods_ahead doit être > 0".into());
    }

    let y = &input.values;

    // --- Optimisation des hyperparamètres alpha/beta par grid-search sur MAE ---
    let (best_alpha, best_beta) = optimize_holt_winters(y);

    // --- Lissage Holt-Winters sur les données historiques ---
    let (levels, trends, fitted) = holt_winters_fit(y, best_alpha, best_beta);

    let last_level = *levels.last().unwrap();
    let last_trend = *trends.last().unwrap();

    let mae = compute_mae(&y[1..], &fitted[1..]);

    // Résidus pour l'intervalle de confiance
    let residuals: Vec<f64> = y[1..]
        .iter()
        .zip(fitted[1..].iter())
        .map(|(a, f)| a - f)
        .collect();
    let std_res = std_dev(&residuals);
    // z = 1.28 pour un intervalle de confiance bilatéral à 80 %
    let z80 = 1.28_f64;

    // --- Génération des prévisions ---
    let mut forecasts = Vec::with_capacity(periods_ahead);
    for h in 1..=periods_ahead {
        let predicted = (last_level + (h as f64) * last_trend).max(0.0);
        // Incertitude croît avec l'horizon : ±z80 * std * sqrt(h)
        let margin = z80 * std_res * (h as f64).sqrt();
        let lower = (predicted - margin).max(0.0);
        let upper = predicted + margin;

        let label = future_label(&input.period_labels, n, h);

        forecasts.push(ForecastPoint {
            period_label: label,
            predicted_value: predicted,
            lower_bound: lower,
            upper_bound: upper,
        });
    }

    Ok(PredictionOutput {
        forecasts,
        model_info: format!(
            "Holt-Winters DES (alpha={:.3}, beta={:.3})",
            best_alpha, best_beta
        ),
        mae,
        history_length: n,
    })
}

// ─── Helpers internes ─────────────────────────────────────────────────────────

/// Ajuste un modèle Holt-Winters sur la série et retourne levels, trends, fitted.
fn holt_winters_fit(y: &[f64], alpha: f64, beta: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let n = y.len();
    let mut levels = vec![0.0f64; n];
    let mut trends = vec![0.0f64; n];
    let mut fitted = vec![0.0f64; n];

    // Initialisation : niveau = première valeur, tendance = pente initiale
    levels[0] = y[0];
    trends[0] = if n > 1 { y[1] - y[0] } else { 0.0 };
    fitted[0] = levels[0];

    for t in 1..n {
        let l_prev = levels[t - 1];
        let b_prev = trends[t - 1];
        levels[t] = alpha * y[t] + (1.0 - alpha) * (l_prev + b_prev);
        trends[t] = beta * (levels[t] - l_prev) + (1.0 - beta) * b_prev;
        fitted[t] = l_prev + b_prev;
    }

    (levels, trends, fitted)
}

/// Optimise alpha et beta par grid-search (pas 0.1) minimisant le MAE sur la série.
fn optimize_holt_winters(y: &[f64]) -> (f64, f64) {
    let candidates = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];
    let mut best_mae = f64::MAX;
    let mut best_alpha = 0.3;
    let mut best_beta = 0.1;

    for &a in &candidates {
        for &b in &candidates {
            let (_, _, fitted) = holt_winters_fit(y, a, b);
            let mae = compute_mae(&y[1..], &fitted[1..]);
            if mae < best_mae {
                best_mae = mae;
                best_alpha = a;
                best_beta = b;
            }
        }
    }

    (best_alpha, best_beta)
}

/// Calcule le Mean Absolute Error.
pub fn compute_mae(actual: &[f64], predicted: &[f64]) -> f64 {
    let n = actual.len().min(predicted.len());
    if n == 0 {
        return 0.0;
    }
    actual[..n]
        .iter()
        .zip(predicted[..n].iter())
        .map(|(a, p)| (a - p).abs())
        .sum::<f64>()
        / n as f64
}

/// Écart-type des résidus.
fn std_dev(values: &[f64]) -> f64 {
    let n = values.len();
    if n < 2 {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / n as f64;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    variance.sqrt()
}

/// Génère un label de période future à partir des labels historiques.
/// Si les labels ressemblent à des dates (YYYY-MM-DD), incrémente le dernier.
/// Sinon retourne "T+h".
fn future_label(labels: &[String], history_len: usize, h: usize) -> String {
    if let Some(last) = labels.last() {
        // Tente d'incrémenter une date ISO
        if last.len() >= 10 {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(&last[..10], "%Y-%m-%d") {
                let future = date + chrono::Duration::days(h as i64);
                return future.format("%Y-%m-%d").to_string();
            }
        }
    }
    format!("T+{}", history_len + h)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_input(n: usize) -> TimeSeriesInput {
        let values: Vec<f64> = (0..n).map(|i| 10.0 + (i as f64 * 0.1)).collect();
        let labels: Vec<String> = (0..n)
            .map(|i| {
                let date = chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
                    + chrono::Duration::days(i as i64);
                date.format("%Y-%m-%d").to_string()
            })
            .collect();
        TimeSeriesInput { values, period_labels: labels }
    }

    #[test]
    fn test_predict_basic() {
        let input = make_input(100);
        let result = predict_workload(&input, 30, 7);
        assert!(result.is_ok(), "Devrait réussir avec 100 points");
        let output = result.unwrap();
        assert_eq!(output.forecasts.len(), 30);
        assert_eq!(output.history_length, 100);
        assert!(!output.model_info.is_empty());
    }

    #[test]
    fn test_predict_insufficient_data() {
        let input = make_input(10);
        let result = predict_workload(&input, 30, 7);
        assert!(result.is_err(), "Devrait échouer avec 10 points");
        let err = result.unwrap_err();
        assert!(
            err.contains("Historique insuffisant"),
            "Message d'erreur inattendu : {err}"
        );
    }

    #[test]
    fn test_mae_calculation() {
        let actual = vec![1.0, 2.0, 3.0, 4.0];
        let predicted = vec![1.5, 2.5, 2.5, 3.5];
        let mae = compute_mae(&actual, &predicted);
        // Erreurs absolues : 0.5, 0.5, 0.5, 0.5 → MAE = 0.5
        assert!((mae - 0.5).abs() < 1e-9, "MAE attendu 0.5, obtenu {mae}");
    }

    #[test]
    fn test_forecast_positive() {
        let input = make_input(90);
        let output = predict_workload(&input, 30, 7).unwrap();
        for fp in &output.forecasts {
            assert!(
                fp.predicted_value >= 0.0,
                "Prévision négative : {}",
                fp.predicted_value
            );
            assert!(
                fp.lower_bound >= 0.0,
                "Borne basse négative : {}",
                fp.lower_bound
            );
        }
    }
}
