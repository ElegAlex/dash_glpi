/// Dashboard KPI ITSM — computes all key performance indicators for GLPI ticket analytics.
use std::collections::BTreeMap;
use std::time::Instant;

use rusqlite::Connection;
use serde::Serialize;

#[cfg(test)]
use rusqlite::params;

use super::stats::{ecart_type, moyenne, percentile};

// ─── Data Structures ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardKpi {
    pub meta: DashboardMeta,
    pub prise_en_charge: PriseEnChargeKpi,
    pub resolution: ResolutionKpi,
    pub taux_n1: TauxN1Kpi,
    pub volumes: VolumetrieKpi,
    pub typologie: TypologieKpi,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardMeta {
    pub total_tickets: i64,
    pub total_vivants: i64,
    pub total_termines: i64,
    pub plage_dates: (String, String),
    pub nb_techniciens_actifs: i64,
    pub nb_groupes: i64,
    pub has_categorie: bool,
    pub calcul_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriseEnChargeKpi {
    pub methode: String,
    pub confiance: String,
    pub delai_moyen_jours: Option<f64>,
    pub mediane_jours: Option<f64>,
    pub p90_jours: Option<f64>,
    pub distribution: Vec<TrancheDelai>,
    pub avertissement: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionKpi {
    pub mttr_global_jours: f64,
    pub mediane_jours: f64,
    pub p90_jours: f64,
    pub ecart_type_jours: f64,
    pub par_type: Vec<MttrParDimension>,
    pub par_priorite: Vec<MttrParDimension>,
    pub par_groupe: Vec<MttrParDimension>,
    pub par_technicien: Vec<MttrParDimension>,
    pub distribution_tranches: Vec<TrancheDelai>,
    pub trend_mensuel: Vec<MttrTrend>,
    pub echantillon: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TauxN1Kpi {
    pub total_termines: i64,
    pub n1_strict: TauxDetail,
    pub n1_elargi: TauxDetail,
    pub multi_niveaux: TauxDetail,
    pub sans_technicien: TauxDetail,
    pub par_groupe: Vec<TauxN1ParGroupe>,
    pub trend_mensuel: Vec<TauxN1Trend>,
    pub objectif_itil: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumetrieKpi {
    pub par_mois: Vec<VolumePeriode>,
    pub total_crees: i64,
    pub total_resolus: i64,
    pub ratio_sortie_entree: f64,
    pub moyenne_mensuelle_creation: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypologieKpi {
    pub par_type: Vec<VentilationItem>,
    pub par_priorite: Vec<VentilationItem>,
    pub par_groupe: Vec<VentilationItem>,
    pub par_categorie: Option<Vec<VentilationItem>>,
    pub categorie_disponible: bool,
}

// ─── Sub-types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrancheDelai {
    pub label: String,
    pub count: i64,
    pub pourcentage: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MttrParDimension {
    pub label: String,
    pub mttr_jours: f64,
    pub mediane_jours: f64,
    pub count: i64,
    pub pourcentage_total: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MttrTrend {
    pub periode: String,
    pub mttr_jours: f64,
    pub mediane_jours: f64,
    pub nb_resolus: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TauxDetail {
    pub count: i64,
    pub pourcentage: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TauxN1ParGroupe {
    pub groupe: String,
    pub total_resolus: i64,
    pub n1_strict_count: i64,
    pub n1_strict_pct: f64,
    pub n1_elargi_count: i64,
    pub n1_elargi_pct: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TauxN1Trend {
    pub periode: String,
    pub n1_strict_pct: f64,
    pub n1_elargi_pct: f64,
    pub total_resolus: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumePeriode {
    pub periode: String,
    pub crees: i64,
    pub resolus: i64,
    pub delta: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VentilationItem {
    pub label: String,
    pub total: i64,
    pub vivants: i64,
    pub termines: i64,
    pub pourcentage_total: f64,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn priority_label(p: i32) -> &'static str {
    match p {
        1 => "Tres haute",
        2 => "Haute",
        3 => "Moyenne",
        4 => "Basse",
        5 => "Tres basse",
        6 => "Majeure",
        _ => "Inconnue",
    }
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

fn pct(count: i64, total: i64) -> f64 {
    if total == 0 {
        0.0
    } else {
        round1(count as f64 / total as f64 * 100.0)
    }
}

/// Builds a WHERE clause fragment for optional date filtering.
/// Returns (clause_string, params_vec).
fn date_filter_clause(
    date_debut: &Option<String>,
    date_fin: &Option<String>,
) -> (String, Vec<String>) {
    let mut clauses = Vec::new();
    let mut params_vec = Vec::new();
    if let Some(ref d) = date_debut {
        clauses.push("date_ouverture >= ?".to_string());
        params_vec.push(d.clone());
    }
    if let Some(ref d) = date_fin {
        clauses.push("date_ouverture <= ?".to_string());
        params_vec.push(d.clone());
    }
    let clause = if clauses.is_empty() {
        String::new()
    } else {
        format!(" AND {}", clauses.join(" AND "))
    };
    (clause, params_vec)
}

/// Safe json_array_length expression that handles empty string, '[]', and NULL.
const SAFE_JSON_LEN: &str =
    "CASE WHEN techniciens = '' OR techniciens = '[]' OR techniciens IS NULL THEN 0 ELSE json_array_length(techniciens) END";

// ─── Builder Functions ───────────────────────────────────────────────────────

fn build_meta(
    conn: &Connection,
    import_id: i64,
    date_clause: &str,
    date_params: &[String],
) -> Result<DashboardMeta, rusqlite::Error> {
    // Total, vivants, termines
    let sql = format!(
        "SELECT
            COUNT(*) AS total,
            COALESCE(SUM(CASE WHEN est_vivant = 1 THEN 1 ELSE 0 END), 0) AS vivants,
            COALESCE(SUM(CASE WHEN est_vivant = 0 THEN 1 ELSE 0 END), 0) AS termines
         FROM tickets WHERE import_id = ?{}",
        date_clause
    );
    let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        all_params.push(Box::new(p.clone()));
    }
    let (total, vivants, termines) = conn.query_row(
        &sql,
        rusqlite::params_from_iter(all_params.iter().map(|b| b.as_ref())),
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        },
    )?;

    // Date range
    let sql_dates = format!(
        "SELECT
            COALESCE(MIN(date_ouverture), '') AS min_date,
            COALESCE(MAX(date_ouverture), '') AS max_date
         FROM tickets WHERE import_id = ?{}",
        date_clause
    );
    let mut all_params2: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        all_params2.push(Box::new(p.clone()));
    }
    let (min_date, max_date) = conn.query_row(
        &sql_dates,
        rusqlite::params_from_iter(all_params2.iter().map(|b| b.as_ref())),
        |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        },
    )?;

    // Count distinct techniciens & groupes
    let sql_tech = format!(
        "SELECT COUNT(DISTINCT technicien_principal)
         FROM tickets WHERE import_id = ? AND technicien_principal IS NOT NULL AND technicien_principal != ''{}",
        date_clause
    );
    let mut all_params3: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        all_params3.push(Box::new(p.clone()));
    }
    let nb_tech: i64 = conn.query_row(
        &sql_tech,
        rusqlite::params_from_iter(all_params3.iter().map(|b| b.as_ref())),
        |row| row.get(0),
    )?;

    let sql_grp = format!(
        "SELECT COUNT(DISTINCT groupe_principal)
         FROM tickets WHERE import_id = ? AND groupe_principal IS NOT NULL AND groupe_principal != ''{}",
        date_clause
    );
    let mut all_params4: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        all_params4.push(Box::new(p.clone()));
    }
    let nb_grp: i64 = conn.query_row(
        &sql_grp,
        rusqlite::params_from_iter(all_params4.iter().map(|b| b.as_ref())),
        |row| row.get(0),
    )?;

    // has_categorie
    let sql_cat = format!(
        "SELECT COUNT(*) FROM tickets WHERE import_id = ? AND categorie IS NOT NULL AND categorie != ''{}",
        date_clause
    );
    let mut all_params5: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        all_params5.push(Box::new(p.clone()));
    }
    let cat_count: i64 = conn.query_row(
        &sql_cat,
        rusqlite::params_from_iter(all_params5.iter().map(|b| b.as_ref())),
        |row| row.get(0),
    )?;

    Ok(DashboardMeta {
        total_tickets: total,
        total_vivants: vivants,
        total_termines: termines,
        plage_dates: (min_date, max_date),
        nb_techniciens_actifs: nb_tech,
        nb_groupes: nb_grp,
        has_categorie: cat_count > 0,
        calcul_duration_ms: 0, // filled later
    })
}

fn build_prise_en_charge(
    conn: &Connection,
    import_id: i64,
    date_clause: &str,
    date_params: &[String],
) -> Result<PriseEnChargeKpi, rusqlite::Error> {
    // Collect proxy delays for terminated tickets
    let sql = format!(
        "SELECT julianday(derniere_modification) - julianday(date_ouverture)
         FROM tickets
         WHERE import_id = ? AND est_vivant = 0
           AND derniere_modification IS NOT NULL AND date_ouverture IS NOT NULL{}",
        date_clause
    );
    let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        all_params.push(Box::new(p.clone()));
    }
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        rusqlite::params_from_iter(all_params.iter().map(|b| b.as_ref())),
        |row| row.get::<_, f64>(0),
    )?;

    let mut delays: Vec<f64> = Vec::new();
    for row in rows {
        let v = row?;
        if v >= 0.0 {
            delays.push(v);
        }
    }

    if delays.is_empty() {
        return Ok(PriseEnChargeKpi {
            methode: "proxy_derniere_modification".to_string(),
            confiance: "basse".to_string(),
            delai_moyen_jours: None,
            mediane_jours: None,
            p90_jours: None,
            distribution: build_pec_distribution(&[], 0),
            avertissement: Some(
                "Aucun ticket terminé avec dates valides pour calculer le délai de prise en charge."
                    .to_string(),
            ),
        });
    }

    let mean = round1(moyenne(&delays));
    let med = round1(percentile(&delays, 50.0));
    let p90 = round1(percentile(&delays, 90.0));

    // Distribution by tranches
    let distribution = build_pec_distribution(&delays, delays.len() as i64);

    Ok(PriseEnChargeKpi {
        methode: "proxy_derniere_modification".to_string(),
        confiance: "basse".to_string(),
        delai_moyen_jours: Some(mean),
        mediane_jours: Some(med),
        p90_jours: Some(p90),
        distribution,
        avertissement: Some(
            "Approximation : le délai est calculé entre date_ouverture et derniere_modification. \
             GLPI ne fournit pas de date de première prise en charge dans l'export CSV."
                .to_string(),
        ),
    })
}

fn build_pec_distribution(delays: &[f64], total: i64) -> Vec<TrancheDelai> {
    let mut lt1 = 0i64;
    let mut from1to3 = 0i64;
    let mut from3to7 = 0i64;
    let mut from7to15 = 0i64;
    let mut gt15 = 0i64;

    for &d in delays {
        if d < 1.0 {
            lt1 += 1;
        } else if d < 3.0 {
            from1to3 += 1;
        } else if d < 7.0 {
            from3to7 += 1;
        } else if d < 15.0 {
            from7to15 += 1;
        } else {
            gt15 += 1;
        }
    }

    vec![
        TrancheDelai { label: "< 1 jour".to_string(), count: lt1, pourcentage: pct(lt1, total) },
        TrancheDelai { label: "1-3 jours".to_string(), count: from1to3, pourcentage: pct(from1to3, total) },
        TrancheDelai { label: "3-7 jours".to_string(), count: from3to7, pourcentage: pct(from3to7, total) },
        TrancheDelai { label: "7-15 jours".to_string(), count: from7to15, pourcentage: pct(from7to15, total) },
        TrancheDelai { label: "> 15 jours".to_string(), count: gt15, pourcentage: pct(gt15, total) },
    ]
}

fn build_resolution(
    conn: &Connection,
    import_id: i64,
    date_clause: &str,
    date_params: &[String],
) -> Result<ResolutionKpi, rusqlite::Error> {
    // Collect resolution durations
    let sql = format!(
        "SELECT julianday(date_cloture_approx) - julianday(date_ouverture)
         FROM tickets
         WHERE import_id = ? AND est_vivant = 0
           AND date_cloture_approx IS NOT NULL AND date_ouverture IS NOT NULL{}",
        date_clause
    );
    let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        all_params.push(Box::new(p.clone()));
    }
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        rusqlite::params_from_iter(all_params.iter().map(|b| b.as_ref())),
        |row| row.get::<_, f64>(0),
    )?;

    let mut durations: Vec<f64> = Vec::new();
    for row in rows {
        let v = row?;
        if v >= 0.0 {
            durations.push(v);
        }
    }

    let echantillon = durations.len() as i64;
    let mttr_global = round1(moyenne(&durations));
    let mediane = round1(percentile(&durations, 50.0));
    let p90 = round1(percentile(&durations, 90.0));
    let et = round1(ecart_type(&durations));

    // MTTR by type
    let par_type = build_mttr_by_dimension(conn, import_id, "type_ticket", date_clause, date_params, echantillon, false)?;

    // MTTR by priority
    let par_priorite = build_mttr_by_dimension(conn, import_id, "priorite", date_clause, date_params, echantillon, true)?;

    // MTTR by groupe
    let par_groupe = build_mttr_by_dimension(conn, import_id, "groupe_principal", date_clause, date_params, echantillon, false)?;

    // MTTR by technicien
    let par_technicien = build_mttr_by_dimension(conn, import_id, "technicien_principal", date_clause, date_params, echantillon, false)?;

    // Distribution by tranches
    let distribution_tranches = build_pec_distribution(&durations, echantillon);

    // Monthly trend
    let trend_mensuel = build_resolution_trend(conn, import_id, date_clause, date_params)?;

    Ok(ResolutionKpi {
        mttr_global_jours: mttr_global,
        mediane_jours: mediane,
        p90_jours: p90,
        ecart_type_jours: et,
        par_type,
        par_priorite,
        par_groupe,
        par_technicien,
        distribution_tranches,
        trend_mensuel,
        echantillon,
    })
}

fn build_mttr_by_dimension(
    conn: &Connection,
    import_id: i64,
    column: &str,
    date_clause: &str,
    date_params: &[String],
    total_echantillon: i64,
    is_priority: bool,
) -> Result<Vec<MttrParDimension>, rusqlite::Error> {
    let sql = format!(
        "SELECT {col},
                AVG(julianday(date_cloture_approx) - julianday(date_ouverture)) AS avg_dur,
                COUNT(*) AS cnt
         FROM tickets
         WHERE import_id = ? AND est_vivant = 0
           AND date_cloture_approx IS NOT NULL AND date_ouverture IS NOT NULL
           AND {col} IS NOT NULL AND {col} != ''{date_clause}
         GROUP BY {col}
         ORDER BY cnt DESC",
        col = column,
        date_clause = date_clause,
    );
    let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        all_params.push(Box::new(p.clone()));
    }
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        rusqlite::params_from_iter(all_params.iter().map(|b| b.as_ref())),
        |row| {
            Ok((
                row.get::<_, rusqlite::types::Value>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        },
    )?;

    // Also need median per dimension — collect durations per group
    let sql_details = format!(
        "SELECT {col},
                julianday(date_cloture_approx) - julianday(date_ouverture) AS dur
         FROM tickets
         WHERE import_id = ? AND est_vivant = 0
           AND date_cloture_approx IS NOT NULL AND date_ouverture IS NOT NULL
           AND {col} IS NOT NULL AND {col} != ''{date_clause}
         ORDER BY {col}",
        col = column,
        date_clause = date_clause,
    );
    let mut all_params2: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        all_params2.push(Box::new(p.clone()));
    }
    let mut stmt2 = conn.prepare(&sql_details)?;
    let detail_rows = stmt2.query_map(
        rusqlite::params_from_iter(all_params2.iter().map(|b| b.as_ref())),
        |row| {
            Ok((
                row.get::<_, rusqlite::types::Value>(0)?,
                row.get::<_, f64>(1)?,
            ))
        },
    )?;

    // Group durations by dimension value for median computation
    let mut durations_by_key: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for row in detail_rows {
        let (val, dur) = row?;
        if dur < 0.0 {
            continue;
        }
        let key = value_to_label(&val, is_priority);
        durations_by_key.entry(key).or_default().push(dur);
    }

    let mut results = Vec::new();
    for row in rows {
        let (val, avg_dur, cnt) = row?;
        let label = value_to_label(&val, is_priority);
        let med = durations_by_key
            .get(&label)
            .map(|v| round1(percentile(v, 50.0)))
            .unwrap_or(0.0);
        results.push(MttrParDimension {
            label,
            mttr_jours: round1(avg_dur),
            mediane_jours: med,
            count: cnt,
            pourcentage_total: pct(cnt, total_echantillon),
        });
    }

    Ok(results)
}

fn value_to_label(val: &rusqlite::types::Value, is_priority: bool) -> String {
    match val {
        rusqlite::types::Value::Integer(i) => {
            if is_priority {
                priority_label(*i as i32).to_string()
            } else {
                i.to_string()
            }
        }
        rusqlite::types::Value::Text(s) => s.clone(),
        rusqlite::types::Value::Real(f) => {
            if is_priority {
                priority_label(*f as i32).to_string()
            } else {
                format!("{:.1}", f)
            }
        }
        _ => "Inconnu".to_string(),
    }
}

fn build_resolution_trend(
    conn: &Connection,
    import_id: i64,
    date_clause: &str,
    date_params: &[String],
) -> Result<Vec<MttrTrend>, rusqlite::Error> {
    // Collect all durations with their month
    let sql = format!(
        "SELECT strftime('%Y-%m', date_cloture_approx) AS mois,
                julianday(date_cloture_approx) - julianday(date_ouverture) AS dur
         FROM tickets
         WHERE import_id = ? AND est_vivant = 0
           AND date_cloture_approx IS NOT NULL AND date_ouverture IS NOT NULL{}
         ORDER BY mois",
        date_clause
    );
    let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        all_params.push(Box::new(p.clone()));
    }
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        rusqlite::params_from_iter(all_params.iter().map(|b| b.as_ref())),
        |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        },
    )?;

    let mut by_month: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for row in rows {
        let (mois, dur) = row?;
        if dur >= 0.0 {
            by_month.entry(mois).or_default().push(dur);
        }
    }

    let mut trends = Vec::new();
    for (mois, durs) in &by_month {
        trends.push(MttrTrend {
            periode: mois.clone(),
            mttr_jours: round1(moyenne(durs)),
            mediane_jours: round1(percentile(durs, 50.0)),
            nb_resolus: durs.len() as i64,
        });
    }

    Ok(trends)
}

fn build_taux_n1(
    conn: &Connection,
    import_id: i64,
    date_clause: &str,
    date_params: &[String],
) -> Result<TauxN1Kpi, rusqlite::Error> {
    let safe_len = SAFE_JSON_LEN;

    // Total terminated tickets
    let sql_total = format!(
        "SELECT COUNT(*) FROM tickets WHERE import_id = ? AND est_vivant = 0{}",
        date_clause
    );
    let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        all_params.push(Box::new(p.clone()));
    }
    let total_termines: i64 = conn.query_row(
        &sql_total,
        rusqlite::params_from_iter(all_params.iter().map(|b| b.as_ref())),
        |row| row.get(0),
    )?;

    // N1 strict: single tech + nombre_suivis <= 1
    let sql_n1_strict = format!(
        "SELECT COUNT(*) FROM tickets
         WHERE import_id = ? AND est_vivant = 0
           AND ({safe_len}) <= 1
           AND technicien_principal IS NOT NULL AND technicien_principal != ''
           AND COALESCE(nombre_suivis, 0) <= 1{}",
        date_clause,
        safe_len = safe_len,
    );
    let mut p1: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        p1.push(Box::new(p.clone()));
    }
    let n1_strict_count: i64 = conn.query_row(
        &sql_n1_strict,
        rusqlite::params_from_iter(p1.iter().map(|b| b.as_ref())),
        |row| row.get(0),
    )?;

    // N1 elargi: single tech (no nombre_suivis constraint)
    let sql_n1_elargi = format!(
        "SELECT COUNT(*) FROM tickets
         WHERE import_id = ? AND est_vivant = 0
           AND ({safe_len}) <= 1
           AND technicien_principal IS NOT NULL AND technicien_principal != ''{}",
        date_clause,
        safe_len = safe_len,
    );
    let mut p2: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        p2.push(Box::new(p.clone()));
    }
    let n1_elargi_count: i64 = conn.query_row(
        &sql_n1_elargi,
        rusqlite::params_from_iter(p2.iter().map(|b| b.as_ref())),
        |row| row.get(0),
    )?;

    // Multi-niveaux: more than 1 technician
    let sql_multi = format!(
        "SELECT COUNT(*) FROM tickets
         WHERE import_id = ? AND est_vivant = 0
           AND ({safe_len}) > 1{}",
        date_clause,
        safe_len = safe_len,
    );
    let mut p3: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        p3.push(Box::new(p.clone()));
    }
    let multi_count: i64 = conn.query_row(
        &sql_multi,
        rusqlite::params_from_iter(p3.iter().map(|b| b.as_ref())),
        |row| row.get(0),
    )?;

    // Sans technicien
    let sql_sans = format!(
        "SELECT COUNT(*) FROM tickets
         WHERE import_id = ? AND est_vivant = 0
           AND (technicien_principal IS NULL OR technicien_principal = ''){}",
        date_clause,
    );
    let mut p4: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        p4.push(Box::new(p.clone()));
    }
    let sans_tech_count: i64 = conn.query_row(
        &sql_sans,
        rusqlite::params_from_iter(p4.iter().map(|b| b.as_ref())),
        |row| row.get(0),
    )?;

    // Par groupe
    let par_groupe = build_taux_n1_par_groupe(conn, import_id, date_clause, date_params)?;

    // Trend mensuel
    let trend_mensuel = build_taux_n1_trend(conn, import_id, date_clause, date_params)?;

    Ok(TauxN1Kpi {
        total_termines,
        n1_strict: TauxDetail {
            count: n1_strict_count,
            pourcentage: pct(n1_strict_count, total_termines),
        },
        n1_elargi: TauxDetail {
            count: n1_elargi_count,
            pourcentage: pct(n1_elargi_count, total_termines),
        },
        multi_niveaux: TauxDetail {
            count: multi_count,
            pourcentage: pct(multi_count, total_termines),
        },
        sans_technicien: TauxDetail {
            count: sans_tech_count,
            pourcentage: pct(sans_tech_count, total_termines),
        },
        par_groupe,
        trend_mensuel,
        objectif_itil: 75.0,
    })
}

fn build_taux_n1_par_groupe(
    conn: &Connection,
    import_id: i64,
    date_clause: &str,
    date_params: &[String],
) -> Result<Vec<TauxN1ParGroupe>, rusqlite::Error> {
    let safe_len = SAFE_JSON_LEN;
    let sql = format!(
        "SELECT
            groupe_principal,
            COUNT(*) AS total_resolus,
            SUM(CASE WHEN ({safe_len}) <= 1
                      AND technicien_principal IS NOT NULL AND technicien_principal != ''
                      AND COALESCE(nombre_suivis, 0) <= 1 THEN 1 ELSE 0 END) AS n1_strict,
            SUM(CASE WHEN ({safe_len}) <= 1
                      AND technicien_principal IS NOT NULL AND technicien_principal != '' THEN 1 ELSE 0 END) AS n1_elargi
         FROM tickets
         WHERE import_id = ? AND est_vivant = 0
           AND groupe_principal IS NOT NULL AND groupe_principal != ''{date_clause}
         GROUP BY groupe_principal
         ORDER BY total_resolus DESC",
        safe_len = safe_len,
        date_clause = date_clause,
    );
    let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        all_params.push(Box::new(p.clone()));
    }
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        rusqlite::params_from_iter(all_params.iter().map(|b| b.as_ref())),
        |row| {
            let groupe: String = row.get(0)?;
            let total_resolus: i64 = row.get(1)?;
            let n1_strict_count: i64 = row.get(2)?;
            let n1_elargi_count: i64 = row.get(3)?;
            Ok(TauxN1ParGroupe {
                groupe,
                total_resolus,
                n1_strict_count,
                n1_strict_pct: pct(n1_strict_count, total_resolus),
                n1_elargi_count,
                n1_elargi_pct: pct(n1_elargi_count, total_resolus),
            })
        },
    )?;

    rows.collect()
}

fn build_taux_n1_trend(
    conn: &Connection,
    import_id: i64,
    date_clause: &str,
    date_params: &[String],
) -> Result<Vec<TauxN1Trend>, rusqlite::Error> {
    let safe_len = SAFE_JSON_LEN;
    let sql = format!(
        "SELECT
            strftime('%Y-%m', date_cloture_approx) AS mois,
            COUNT(*) AS total_resolus,
            SUM(CASE WHEN ({safe_len}) <= 1
                      AND technicien_principal IS NOT NULL AND technicien_principal != ''
                      AND COALESCE(nombre_suivis, 0) <= 1 THEN 1 ELSE 0 END) AS n1_strict,
            SUM(CASE WHEN ({safe_len}) <= 1
                      AND technicien_principal IS NOT NULL AND technicien_principal != '' THEN 1 ELSE 0 END) AS n1_elargi
         FROM tickets
         WHERE import_id = ? AND est_vivant = 0
           AND date_cloture_approx IS NOT NULL{date_clause}
         GROUP BY mois
         ORDER BY mois",
        safe_len = safe_len,
        date_clause = date_clause,
    );
    let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        all_params.push(Box::new(p.clone()));
    }
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        rusqlite::params_from_iter(all_params.iter().map(|b| b.as_ref())),
        |row| {
            let mois: String = row.get(0)?;
            let total_resolus: i64 = row.get(1)?;
            let n1_strict_count: i64 = row.get(2)?;
            let n1_elargi_count: i64 = row.get(3)?;
            Ok(TauxN1Trend {
                periode: mois,
                n1_strict_pct: pct(n1_strict_count, total_resolus),
                n1_elargi_pct: pct(n1_elargi_count, total_resolus),
                total_resolus,
            })
        },
    )?;

    rows.collect()
}

fn build_volumetrie(
    conn: &Connection,
    import_id: i64,
    date_clause: &str,
    date_params: &[String],
) -> Result<VolumetrieKpi, rusqlite::Error> {
    // Created by month
    let sql_crees = format!(
        "SELECT strftime('%Y-%m', date_ouverture) AS mois, COUNT(*) AS cnt
         FROM tickets
         WHERE import_id = ? AND date_ouverture IS NOT NULL{}
         GROUP BY mois
         ORDER BY mois",
        date_clause
    );
    let mut p1: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        p1.push(Box::new(p.clone()));
    }
    let mut stmt = conn.prepare(&sql_crees)?;
    let created_rows = stmt.query_map(
        rusqlite::params_from_iter(p1.iter().map(|b| b.as_ref())),
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
    )?;

    let mut volume_map: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    let mut total_crees: i64 = 0;
    for row in created_rows {
        let (mois, cnt) = row?;
        total_crees += cnt;
        volume_map.entry(mois).or_insert((0, 0)).0 = cnt;
    }

    // Resolved by month
    let sql_resolus = format!(
        "SELECT strftime('%Y-%m', date_cloture_approx) AS mois, COUNT(*) AS cnt
         FROM tickets
         WHERE import_id = ? AND est_vivant = 0 AND date_cloture_approx IS NOT NULL{}
         GROUP BY mois
         ORDER BY mois",
        date_clause
    );
    let mut p2: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        p2.push(Box::new(p.clone()));
    }
    let mut stmt2 = conn.prepare(&sql_resolus)?;
    let resolved_rows = stmt2.query_map(
        rusqlite::params_from_iter(p2.iter().map(|b| b.as_ref())),
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
    )?;

    let mut total_resolus: i64 = 0;
    for row in resolved_rows {
        let (mois, cnt) = row?;
        total_resolus += cnt;
        volume_map.entry(mois).or_insert((0, 0)).1 = cnt;
    }

    let par_mois: Vec<VolumePeriode> = volume_map
        .iter()
        .map(|(mois, (crees, resolus))| VolumePeriode {
            periode: mois.clone(),
            crees: *crees,
            resolus: *resolus,
            delta: *crees - *resolus,
        })
        .collect();

    let nb_mois = if par_mois.is_empty() { 1 } else { par_mois.len() };
    let ratio = if total_crees == 0 {
        0.0
    } else {
        round1(total_resolus as f64 / total_crees as f64)
    };

    Ok(VolumetrieKpi {
        par_mois,
        total_crees,
        total_resolus,
        ratio_sortie_entree: ratio,
        moyenne_mensuelle_creation: round1(total_crees as f64 / nb_mois as f64),
    })
}

fn build_typologie(
    conn: &Connection,
    import_id: i64,
    has_categorie: bool,
    date_clause: &str,
    date_params: &[String],
) -> Result<TypologieKpi, rusqlite::Error> {
    let par_type = build_ventilation(conn, import_id, "type_ticket", None, date_clause, date_params)?;
    let par_priorite = build_ventilation_priorite(conn, import_id, date_clause, date_params)?;
    let par_groupe = build_ventilation(conn, import_id, "groupe_principal", Some(10), date_clause, date_params)?;

    let par_categorie = if has_categorie {
        Some(build_ventilation(conn, import_id, "categorie", None, date_clause, date_params)?)
    } else {
        None
    };

    Ok(TypologieKpi {
        par_type,
        par_priorite,
        par_groupe,
        par_categorie,
        categorie_disponible: has_categorie,
    })
}

fn build_ventilation(
    conn: &Connection,
    import_id: i64,
    column: &str,
    limit: Option<usize>,
    date_clause: &str,
    date_params: &[String],
) -> Result<Vec<VentilationItem>, rusqlite::Error> {
    let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();
    let sql = format!(
        "SELECT
            COALESCE({col}, 'Non renseigné') AS label,
            COUNT(*) AS total,
            SUM(CASE WHEN est_vivant = 1 THEN 1 ELSE 0 END) AS vivants,
            SUM(CASE WHEN est_vivant = 0 THEN 1 ELSE 0 END) AS termines
         FROM tickets
         WHERE import_id = ?{date_clause}
         GROUP BY label
         ORDER BY total DESC{limit_clause}",
        col = column,
        date_clause = date_clause,
        limit_clause = limit_clause,
    );

    // Get grand total for percentage calculation
    let sql_total = format!(
        "SELECT COUNT(*) FROM tickets WHERE import_id = ?{}",
        date_clause
    );
    let mut pt: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        pt.push(Box::new(p.clone()));
    }
    let grand_total: i64 = conn.query_row(
        &sql_total,
        rusqlite::params_from_iter(pt.iter().map(|b| b.as_ref())),
        |row| row.get(0),
    )?;

    let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        all_params.push(Box::new(p.clone()));
    }
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        rusqlite::params_from_iter(all_params.iter().map(|b| b.as_ref())),
        |row| {
            let label: String = row.get(0)?;
            let total: i64 = row.get(1)?;
            let vivants: i64 = row.get(2)?;
            let termines: i64 = row.get(3)?;
            Ok(VentilationItem {
                label,
                total,
                vivants,
                termines,
                pourcentage_total: pct(total, grand_total),
            })
        },
    )?;

    rows.collect()
}

fn build_ventilation_priorite(
    conn: &Connection,
    import_id: i64,
    date_clause: &str,
    date_params: &[String],
) -> Result<Vec<VentilationItem>, rusqlite::Error> {
    let sql = format!(
        "SELECT
            priorite,
            COUNT(*) AS total,
            SUM(CASE WHEN est_vivant = 1 THEN 1 ELSE 0 END) AS vivants,
            SUM(CASE WHEN est_vivant = 0 THEN 1 ELSE 0 END) AS termines
         FROM tickets
         WHERE import_id = ?{date_clause}
         GROUP BY priorite
         ORDER BY priorite",
        date_clause = date_clause,
    );

    let sql_total = format!(
        "SELECT COUNT(*) FROM tickets WHERE import_id = ?{}",
        date_clause
    );
    let mut pt: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        pt.push(Box::new(p.clone()));
    }
    let grand_total: i64 = conn.query_row(
        &sql_total,
        rusqlite::params_from_iter(pt.iter().map(|b| b.as_ref())),
        |row| row.get(0),
    )?;

    let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(import_id)];
    for p in date_params {
        all_params.push(Box::new(p.clone()));
    }
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        rusqlite::params_from_iter(all_params.iter().map(|b| b.as_ref())),
        |row| {
            let prio: Option<i32> = row.get(0)?;
            let total: i64 = row.get(1)?;
            let vivants: i64 = row.get(2)?;
            let termines: i64 = row.get(3)?;
            let label = match prio {
                Some(p) => priority_label(p).to_string(),
                None => "Non renseigné".to_string(),
            };
            Ok(VentilationItem {
                label,
                total,
                vivants,
                termines,
                pourcentage_total: pct(total, grand_total),
            })
        },
    )?;

    rows.collect()
}

// ─── Main Entry Point ────────────────────────────────────────────────────────

/// Builds the complete Dashboard KPI structure from the database.
///
/// # Arguments
/// * `conn` - SQLite connection reference
/// * `import_id` - Active import ID
/// * `date_debut` - Optional start date filter (ISO format)
/// * `date_fin` - Optional end date filter (ISO format)
pub fn build_dashboard_kpi(
    conn: &Connection,
    import_id: i64,
    date_debut: &Option<String>,
    date_fin: &Option<String>,
) -> Result<DashboardKpi, rusqlite::Error> {
    let start = Instant::now();

    let (date_clause, date_params) = date_filter_clause(date_debut, date_fin);

    let mut meta = build_meta(conn, import_id, &date_clause, &date_params)?;
    let prise_en_charge = build_prise_en_charge(conn, import_id, &date_clause, &date_params)?;
    let resolution = build_resolution(conn, import_id, &date_clause, &date_params)?;
    let taux_n1 = build_taux_n1(conn, import_id, &date_clause, &date_params)?;
    let volumes = build_volumetrie(conn, import_id, &date_clause, &date_params)?;
    let typologie = build_typologie(conn, import_id, meta.has_categorie, &date_clause, &date_params)?;

    meta.calcul_duration_ms = start.elapsed().as_millis() as u64;

    Ok(DashboardKpi {
        meta,
        prise_en_charge,
        resolution,
        taux_n1,
        volumes,
        typologie,
    })
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../db/sql/001_initial.sql"))
            .unwrap();

        // Insert test import
        conn.execute(
            "INSERT INTO imports (id, filename, total_rows, parsed_rows, skipped_rows,
                vivants_count, termines_count, detected_columns, unique_statuts, unique_types, is_active)
             VALUES (1, 'test.csv', 10, 10, 0, 3, 7, '[]', '[]', '[]', 1)",
            [],
        )
        .unwrap();

        // Insert test tickets
        // 3 vivants, 7 termines
        let tickets = vec![
            // (id, statut, type_ticket, priorite, date_ouverture, derniere_modification, techniciens, technicien_principal, groupe_principal, est_vivant, anciennete_jours, date_cloture_approx, nombre_suivis, categorie)
            (1, "En cours", "Incident", 3, "2025-01-10T10:00:00", "2025-01-15T10:00:00", r#"["Alice"]"#, "Alice", "Support N1", 1, 50, None::<&str>, 0, Some("Réseau")),
            (2, "En cours", "Demande", 4, "2025-02-01T08:00:00", "2025-02-10T08:00:00", r#"["Bob"]"#, "Bob", "Support N2", 1, 30, None, 2, Some("Logiciel")),
            (3, "En attente", "Incident", 2, "2025-03-01T09:00:00", "2025-03-05T09:00:00", r#"["Alice","Bob"]"#, "Alice", "Support N1", 1, 10, None, 1, None),

            // Terminated tickets
            (4, "Résolu", "Incident", 3, "2025-01-05T10:00:00", "2025-01-12T10:00:00", r#"["Alice"]"#, "Alice", "Support N1", 0, 0, Some("2025-01-12T10:00:00"), 1, Some("Réseau")),
            (5, "Clos", "Demande", 4, "2025-01-20T08:00:00", "2025-01-22T08:00:00", r#"["Bob"]"#, "Bob", "Support N2", 0, 0, Some("2025-01-22T08:00:00"), 0, Some("Logiciel")),
            (6, "Résolu", "Incident", 3, "2025-02-10T09:00:00", "2025-02-20T09:00:00", r#"["Alice","Charlie"]"#, "Alice", "Support N1", 0, 0, Some("2025-02-20T09:00:00"), 3, Some("Réseau")),
            (7, "Clos", "Incident", 2, "2025-02-15T10:00:00", "2025-02-16T10:00:00", r#"["Charlie"]"#, "Charlie", "Support N1", 0, 0, Some("2025-02-16T10:00:00"), 0, None),
            (8, "Résolu", "Demande", 4, "2025-03-01T08:00:00", "2025-03-10T08:00:00", r#"["Bob"]"#, "Bob", "Support N2", 0, 0, Some("2025-03-10T08:00:00"), 1, Some("Logiciel")),
            (9, "Clos", "Incident", 3, "2025-03-05T10:00:00", "2025-03-08T10:00:00", r#"[]"#, "", "", 0, 0, Some("2025-03-08T10:00:00"), 0, None),
            (10, "Clos", "Demande", 5, "2025-03-10T09:00:00", "2025-03-15T09:00:00", "[]", "", "Support N1", 0, 0, Some("2025-03-15T09:00:00"), 0, Some("Réseau")),
        ];

        for t in &tickets {
            conn.execute(
                "INSERT INTO tickets (id, import_id, statut, type_ticket, priorite, date_ouverture,
                    derniere_modification, techniciens, technicien_principal, groupe_principal,
                    est_vivant, anciennete_jours, date_cloture_approx, nombre_suivis, categorie)
                 VALUES (?1, 1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![t.0, t.1, t.2, t.3, t.4, t.5, t.6, t.7, t.8, t.9, t.10, t.11, t.12, t.13],
            )
            .unwrap();
        }

        conn
    }

    #[test]
    fn test_build_meta() {
        let conn = setup_test_db();
        let kpi = build_dashboard_kpi(&conn, 1, &None, &None).unwrap();

        assert_eq!(kpi.meta.total_tickets, 10);
        assert_eq!(kpi.meta.total_vivants, 3);
        assert_eq!(kpi.meta.total_termines, 7);
        assert_eq!(kpi.meta.nb_techniciens_actifs, 3); // Alice, Bob, Charlie
        assert!(kpi.meta.nb_groupes >= 2); // Support N1, Support N2
        assert!(kpi.meta.has_categorie);
        assert!(!kpi.meta.plage_dates.0.is_empty());
        assert!(!kpi.meta.plage_dates.1.is_empty());
    }

    #[test]
    fn test_resolution_mttr() {
        let conn = setup_test_db();
        let kpi = build_dashboard_kpi(&conn, 1, &None, &None).unwrap();

        // We have 7 terminated tickets with known durations
        assert!(kpi.resolution.echantillon > 0);
        assert!(kpi.resolution.mttr_global_jours > 0.0);
        assert!(kpi.resolution.mediane_jours > 0.0);
        assert!(kpi.resolution.p90_jours >= kpi.resolution.mediane_jours);
        assert!(kpi.resolution.ecart_type_jours >= 0.0);

        // Check dimensions are populated
        assert!(!kpi.resolution.par_type.is_empty());
        assert!(!kpi.resolution.par_priorite.is_empty());
        // par_groupe may be empty if some tickets have empty groupe_principal
        assert!(!kpi.resolution.distribution_tranches.is_empty());
        assert!(!kpi.resolution.trend_mensuel.is_empty());

        // Verify distribution sums to echantillon
        let dist_sum: i64 = kpi.resolution.distribution_tranches.iter().map(|t| t.count).sum();
        assert_eq!(dist_sum, kpi.resolution.echantillon);
    }

    #[test]
    fn test_taux_n1_mono() {
        // Create a DB where all terminated tickets are mono-tech
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../db/sql/001_initial.sql")).unwrap();
        conn.execute(
            "INSERT INTO imports (id, filename, total_rows, parsed_rows, skipped_rows,
                vivants_count, termines_count, detected_columns, unique_statuts, unique_types, is_active)
             VALUES (1, 'test.csv', 3, 3, 0, 0, 3, '[]', '[]', '[]', 1)",
            [],
        ).unwrap();

        for i in 1..=3 {
            conn.execute(
                "INSERT INTO tickets (id, import_id, statut, type_ticket, priorite, date_ouverture,
                    derniere_modification, techniciens, technicien_principal, groupe_principal,
                    est_vivant, anciennete_jours, date_cloture_approx, nombre_suivis)
                 VALUES (?1, 1, 'Résolu', 'Incident', 3, '2025-01-01T10:00:00',
                    '2025-01-05T10:00:00', ?2, 'Alice', 'Support N1',
                    0, 0, '2025-01-05T10:00:00', 0)",
                params![i, r#"["Alice"]"#],
            ).unwrap();
        }

        let kpi = build_dashboard_kpi(&conn, 1, &None, &None).unwrap();
        assert_eq!(kpi.taux_n1.total_termines, 3);
        assert_eq!(kpi.taux_n1.n1_strict.count, 3);
        assert!((kpi.taux_n1.n1_strict.pourcentage - 100.0).abs() < 0.1);
        assert_eq!(kpi.taux_n1.n1_elargi.count, 3);
        assert_eq!(kpi.taux_n1.multi_niveaux.count, 0);
        assert_eq!(kpi.taux_n1.sans_technicien.count, 0);
    }

    #[test]
    fn test_taux_n1_mixed() {
        let conn = setup_test_db();
        let kpi = build_dashboard_kpi(&conn, 1, &None, &None).unwrap();

        assert_eq!(kpi.taux_n1.total_termines, 7);
        assert_eq!(kpi.taux_n1.objectif_itil, 75.0);

        // Ticket 4: Alice, 1 suivi → N1 strict? nombre_suivis=1 ≤1, single tech → YES
        // Ticket 5: Bob, 0 suivis → YES strict
        // Ticket 6: Alice+Charlie, multi → NO
        // Ticket 7: Charlie, 0 suivis → YES strict
        // Ticket 8: Bob, 1 suivi → YES strict
        // Ticket 9: no tech → sans_technicien
        // Ticket 10: no tech → sans_technicien

        // N1 strict: tickets 4, 5, 7, 8 = 4
        assert_eq!(kpi.taux_n1.n1_strict.count, 4);

        // N1 elargi (mono tech, any suivis): tickets 4, 5, 7, 8 = 4
        assert_eq!(kpi.taux_n1.n1_elargi.count, 4);

        // Multi: ticket 6 = 1
        assert_eq!(kpi.taux_n1.multi_niveaux.count, 1);

        // Sans tech: tickets 9, 10 = 2
        assert_eq!(kpi.taux_n1.sans_technicien.count, 2);

        // Sanity: strict + multi + sans_tech should be ≤ total (elargi can overlap with strict)
        let sum = kpi.taux_n1.n1_elargi.count + kpi.taux_n1.multi_niveaux.count + kpi.taux_n1.sans_technicien.count;
        assert_eq!(sum, kpi.taux_n1.total_termines);

        // Percentages
        assert!(kpi.taux_n1.n1_strict.pourcentage > 0.0);
        assert!(kpi.taux_n1.n1_strict.pourcentage <= 100.0);
    }

    #[test]
    fn test_volumetrie() {
        let conn = setup_test_db();
        let kpi = build_dashboard_kpi(&conn, 1, &None, &None).unwrap();

        assert!(!kpi.volumes.par_mois.is_empty());
        assert_eq!(kpi.volumes.total_crees, 10);
        assert!(kpi.volumes.total_resolus > 0);
        assert!(kpi.volumes.ratio_sortie_entree > 0.0);
        assert!(kpi.volumes.moyenne_mensuelle_creation > 0.0);

        // Verify monthly sums
        let sum_crees: i64 = kpi.volumes.par_mois.iter().map(|v| v.crees).sum();
        let sum_resolus: i64 = kpi.volumes.par_mois.iter().map(|v| v.resolus).sum();
        assert_eq!(sum_crees, kpi.volumes.total_crees);
        assert_eq!(sum_resolus, kpi.volumes.total_resolus);
    }

    #[test]
    fn test_empty_dataset() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../db/sql/001_initial.sql")).unwrap();
        conn.execute(
            "INSERT INTO imports (id, filename, total_rows, parsed_rows, skipped_rows,
                vivants_count, termines_count, detected_columns, unique_statuts, unique_types, is_active)
             VALUES (1, 'empty.csv', 0, 0, 0, 0, 0, '[]', '[]', '[]', 1)",
            [],
        ).unwrap();

        let kpi = build_dashboard_kpi(&conn, 1, &None, &None).unwrap();

        assert_eq!(kpi.meta.total_tickets, 0);
        assert_eq!(kpi.meta.total_vivants, 0);
        assert_eq!(kpi.meta.total_termines, 0);
        assert_eq!(kpi.resolution.mttr_global_jours, 0.0);
        assert_eq!(kpi.resolution.echantillon, 0);
        assert_eq!(kpi.taux_n1.total_termines, 0);
        assert_eq!(kpi.taux_n1.objectif_itil, 75.0);
        assert_eq!(kpi.volumes.total_crees, 0);
        assert_eq!(kpi.volumes.total_resolus, 0);
        assert!(kpi.volumes.par_mois.is_empty());
    }

    #[test]
    fn test_date_filtering() {
        let conn = setup_test_db();

        // Filter to only January 2025
        let kpi = build_dashboard_kpi(
            &conn,
            1,
            &Some("2025-01-01".to_string()),
            &Some("2025-01-31T23:59:59".to_string()),
        )
        .unwrap();

        // January tickets: IDs 1, 4, 5 → 3 tickets
        assert_eq!(kpi.meta.total_tickets, 3);
        assert_eq!(kpi.meta.total_vivants, 1); // ticket 1
        assert_eq!(kpi.meta.total_termines, 2); // tickets 4, 5

        // Resolution should only count terminated tickets in January
        assert_eq!(kpi.resolution.echantillon, 2);
    }

    #[test]
    fn test_typologie() {
        let conn = setup_test_db();
        let kpi = build_dashboard_kpi(&conn, 1, &None, &None).unwrap();

        // par_type should have Incident and Demande
        assert!(!kpi.typologie.par_type.is_empty());
        let type_labels: Vec<&str> = kpi.typologie.par_type.iter().map(|v| v.label.as_str()).collect();
        assert!(type_labels.contains(&"Incident"));
        assert!(type_labels.contains(&"Demande"));

        // par_priorite should have entries
        assert!(!kpi.typologie.par_priorite.is_empty());

        // categorie should be available
        assert!(kpi.typologie.categorie_disponible);
        assert!(kpi.typologie.par_categorie.is_some());

        // Verify totals sum correctly
        let type_total: i64 = kpi.typologie.par_type.iter().map(|v| v.total).sum();
        assert_eq!(type_total, 10);
    }

    #[test]
    fn test_prise_en_charge() {
        let conn = setup_test_db();
        let kpi = build_dashboard_kpi(&conn, 1, &None, &None).unwrap();

        assert_eq!(kpi.prise_en_charge.methode, "proxy_derniere_modification");
        assert_eq!(kpi.prise_en_charge.confiance, "basse");
        assert!(kpi.prise_en_charge.delai_moyen_jours.is_some());
        assert!(kpi.prise_en_charge.mediane_jours.is_some());
        assert!(kpi.prise_en_charge.p90_jours.is_some());
        assert!(kpi.prise_en_charge.avertissement.is_some());
        assert_eq!(kpi.prise_en_charge.distribution.len(), 5);

        // Distribution should sum to total terminated tickets with valid dates
        let dist_sum: i64 = kpi.prise_en_charge.distribution.iter().map(|t| t.count).sum();
        assert!(dist_sum > 0);
    }
}
