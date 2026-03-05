use serde::Serialize;
use std::path::Path;
use std::time::Instant;
use tauri::ipc::Channel;

use crate::state::{AppState, DbAccess};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum ImportEvent {
    #[serde(rename_all = "camelCase")]
    Progress {
        rows_parsed: usize,
        total_estimated: usize,
        phase: String,
    },
    #[serde(rename_all = "camelCase")]
    Complete {
        duration_ms: u64,
        total_tickets: usize,
        vivants: usize,
        termines: usize,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub import_id: i64,
    pub total_tickets: usize,
    pub vivants_count: usize,
    pub termines_count: usize,
    pub skipped_rows: usize,
    pub warnings: Vec<crate::parser::types::ParseWarning>,
    pub detected_columns: Vec<String>,
    pub missing_optional_columns: Vec<String>,
    pub unique_statuts: Vec<String>,
    pub parse_duration_ms: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRecord {
    pub id: i64,
    pub filename: String,
    pub import_date: String,
    pub total_rows: usize,
    pub vivants_count: usize,
    pub termines_count: usize,
    pub date_range_from: Option<String>,
    pub date_range_to: Option<String>,
    pub is_active: bool,
}

#[tauri::command]
pub async fn import_csv(
    state: tauri::State<'_, AppState>,
    path: String,
    merge: Option<bool>,
    on_progress: Channel<ImportEvent>,
) -> Result<ImportResult, String> {
    let start = Instant::now();
    let merge = merge.unwrap_or(false);

    // Extract filename from path
    let filename = Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&path)
        .to_string();

    // File size
    let file_size_bytes: i64 = std::fs::metadata(&path)
        .map(|m| m.len() as i64)
        .unwrap_or(0);

    // Duplicate check by filename
    let is_duplicate = state.db(|conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM imports WHERE filename = ?1",
            rusqlite::params![&filename],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    })?;

    // Parse CSV — progress callback sends ImportEvent::Progress every 500 rows
    let on_prog = on_progress.clone();
    let parse_output = crate::parser::pipeline::parse_file(&path, move |rows_parsed, _accepted| {
        let _ = on_prog.send(ImportEvent::Progress {
            rows_parsed,
            total_estimated: 0,
            phase: "Parsing".to_string(),
        });
    })
    .map_err(|e| e.to_string())?;

    // Load config for classification thresholds
    let config = state.db(|conn| crate::config::get_config_from_db(conn))?;

    // Classify each ticket (vivants only — terminés are left unchanged)
    let mut tickets = parse_output.tickets;
    for ticket in &mut tickets {
        crate::analyzer::classifier::classify_ticket(ticket, &config);
    }

    let total_tickets = tickets.len();
    let skipped_rows = parse_output.skipped_rows;
    let parse_duration_ms = parse_output.parse_duration_ms;

    // Persist: either merge into active import or create a new one
    let import_id = if merge {
        // Merge mode: insert tickets into existing active import
        let active_id = state
            .db(|conn| crate::db::queries::get_active_import_id(conn))
            .map_err(|_| "Aucun import actif pour la fusion. Importez d'abord un fichier.".to_string())?;

        state.db_mut(|conn| {
            crate::db::insert::bulk_insert_tickets(conn, active_id, &tickets)?;

            // Recalculate import metadata from the merged ticket set
            conn.execute(
                "UPDATE imports SET
                    parsed_rows = (SELECT COUNT(*) FROM tickets WHERE import_id = ?1),
                    vivants_count = (SELECT COUNT(*) FROM tickets WHERE import_id = ?1 AND est_vivant = 1),
                    termines_count = (SELECT COUNT(*) FROM tickets WHERE import_id = ?1 AND est_vivant = 0),
                    date_range_from = (SELECT MIN(date_ouverture) FROM tickets WHERE import_id = ?1),
                    date_range_to = (SELECT MAX(date_ouverture) FROM tickets WHERE import_id = ?1),
                    total_rows = (SELECT COUNT(*) FROM tickets WHERE import_id = ?1)
                WHERE id = ?1",
                rusqlite::params![active_id],
            )?;
            Ok(active_id)
        })?
    } else {
        // Normal mode: deactivate previous imports, create new import record
        let date_range_from = tickets
            .iter()
            .map(|t| t.date_ouverture.as_str())
            .min()
            .map(str::to_string);
        let date_range_to = tickets
            .iter()
            .map(|t| t.date_ouverture.as_str())
            .max()
            .map(str::to_string);

        let total_rows = parse_output.total_rows_processed;
        let vivants_count = tickets.iter().filter(|t| t.est_vivant).count();
        let termines_count = tickets.len() - vivants_count;

        state.db_mut(|conn| {
            conn.execute("UPDATE imports SET is_active = 0", [])?;

            let detected_json = serde_json::to_string(&parse_output.detected_columns)
                .unwrap_or_else(|_| "[]".to_string());
            let statuts_json = serde_json::to_string(&parse_output.unique_statuts)
                .unwrap_or_else(|_| "[]".to_string());
            let types_json = serde_json::to_string(&parse_output.unique_types)
                .unwrap_or_else(|_| "[]".to_string());

            conn.execute(
                "INSERT INTO imports (
                    filename, file_size_bytes, total_rows, parsed_rows, skipped_rows,
                    vivants_count, termines_count, date_range_from, date_range_to,
                    detected_columns, unique_statuts, unique_types, parse_duration_ms, is_active
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 1)",
                rusqlite::params![
                    &filename,
                    file_size_bytes,
                    total_rows as i64,
                    total_tickets as i64,
                    skipped_rows as i64,
                    vivants_count as i64,
                    termines_count as i64,
                    date_range_from.as_deref(),
                    date_range_to.as_deref(),
                    detected_json,
                    statuts_json,
                    types_json,
                    parse_duration_ms as i64,
                ],
            )?;
            let import_id = conn.last_insert_rowid();
            crate::db::insert::bulk_insert_tickets(conn, import_id, &tickets)?;
            Ok(import_id)
        })?
    };

    // Read final counts from DB (accounts for merge deduplication)
    let (vivants_count, termines_count) = state.db(|conn| {
        let v: i64 = conn.query_row(
            "SELECT vivants_count FROM imports WHERE id = ?1",
            rusqlite::params![import_id],
            |row| row.get(0),
        )?;
        let t: i64 = conn.query_row(
            "SELECT termines_count FROM imports WHERE id = ?1",
            rusqlite::params![import_id],
            |row| row.get(0),
        )?;
        Ok((v as usize, t as usize))
    })?;

    let duration_ms = start.elapsed().as_millis() as u64;

    // Notify frontend that import is complete
    let _ = on_progress.send(ImportEvent::Complete {
        duration_ms,
        total_tickets,
        vivants: vivants_count,
        termines: termines_count,
    });

    // Build warnings — prepend duplicate warning if the filename was already imported
    let mut warnings = parse_output.warnings;
    if is_duplicate {
        warnings.insert(
            0,
            crate::parser::types::ParseWarning {
                line: 0,
                message: format!("Fichier '{}' déjà importé (doublon potentiel)", filename),
            },
        );
    }
    if merge {
        warnings.insert(
            0,
            crate::parser::types::ParseWarning {
                line: 0,
                message: format!(
                    "Fusion réussie : {} tickets ajoutés/mis à jour dans l'import actif",
                    total_tickets
                ),
            },
        );
    }

    Ok(ImportResult {
        import_id,
        total_tickets: vivants_count + termines_count,
        vivants_count,
        termines_count,
        skipped_rows,
        warnings,
        detected_columns: parse_output.detected_columns,
        missing_optional_columns: parse_output.missing_optional_columns,
        unique_statuts: parse_output.unique_statuts,
        parse_duration_ms,
    })
}

#[tauri::command]
pub async fn get_import_history(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ImportRecord>, String> {
    state.db(|conn| crate::db::queries::get_import_history(conn))
}

// ─── Suivi individuel technicien ─────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TechHistory {
    pub kpi: TechHistoryKpi,
    pub periodes: Vec<TechHistoryPeriod>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TechHistoryKpi {
    pub total_entrants: usize,
    pub total_sortants: usize,
    pub stock_actuel: usize,
    pub mttr_jours: Option<f64>,
    pub incidents: usize,
    pub demandes: usize,
    pub age_moyen_jours: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TechHistoryPeriod {
    pub period_key: String,
    pub entrants: usize,
    pub sortants: usize,
    pub stock_cumule: usize,
    pub mttr_jours: Option<f64>,
}

#[tauri::command]
pub async fn get_technician_history(
    state: tauri::State<'_, AppState>,
    technicien: String,
    granularity: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
) -> Result<TechHistory, String> {
    let gran = granularity.as_deref().unwrap_or("month");
    state.db(|conn| {
        crate::db::queries::get_technician_history(
            conn,
            &technicien,
            gran,
            date_from.as_deref(),
            date_to.as_deref(),
        )
    })
}

#[tauri::command]
pub async fn set_active_import(
    state: tauri::State<'_, AppState>,
    import_id: i64,
) -> Result<(), String> {
    state.db(|conn| {
        conn.execute(
            "UPDATE imports SET is_active = 1 WHERE id = ?1",
            rusqlite::params![import_id],
        )?;
        Ok(())
    })
}

#[tauri::command]
pub async fn delete_import(
    state: tauri::State<'_, AppState>,
    import_id: i64,
) -> Result<(), String> {
    state.db(|conn| {
        conn.execute("DELETE FROM imports WHERE id = ?1", rusqlite::params![import_id])?;
        Ok(())
    })
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::db::queries::get_import_history;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../db/sql/001_initial.sql"))
            .unwrap();
        conn
    }

    /// GIVEN an empty imports table
    /// WHEN get_import_history is called
    /// THEN it returns an empty vec (no error)
    #[test]
    fn test_import_history_empty() {
        let conn = setup_db();
        let history = get_import_history(&conn).unwrap();
        assert!(history.is_empty());
    }

    /// GIVEN an imports record
    /// WHEN get_import_history is called
    /// THEN it maps all fields correctly
    #[test]
    fn test_import_history_maps_fields() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO imports (
                filename, total_rows, parsed_rows, skipped_rows,
                vivants_count, termines_count,
                detected_columns, unique_statuts, unique_types,
                is_active
             ) VALUES ('test.csv', 100, 98, 2, 43, 55, '[]', '[]', '[]', 1)",
            [],
        )
        .unwrap();
        let history = get_import_history(&conn).unwrap();
        assert_eq!(history.len(), 1);
        let rec = &history[0];
        assert_eq!(rec.filename, "test.csv");
        assert_eq!(rec.total_rows, 100);
        assert_eq!(rec.vivants_count, 43);
        assert_eq!(rec.termines_count, 55);
        assert!(rec.is_active);
        assert!(rec.date_range_from.is_none());
    }

    /// GIVEN two imports (second more recent)
    /// WHEN get_import_history is called
    /// THEN results are ordered most-recent first
    #[test]
    fn test_import_history_ordered_desc() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO imports (
                filename, total_rows, parsed_rows, skipped_rows,
                vivants_count, termines_count,
                detected_columns, unique_statuts, unique_types,
                import_date, is_active
             ) VALUES ('a.csv', 10, 10, 0, 5, 5, '[]', '[]', '[]', '2026-02-01T10:00:00', 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO imports (
                filename, total_rows, parsed_rows, skipped_rows,
                vivants_count, termines_count,
                detected_columns, unique_statuts, unique_types,
                import_date, is_active
             ) VALUES ('b.csv', 20, 20, 0, 10, 10, '[]', '[]', '[]', '2026-03-01T10:00:00', 1)",
            [],
        )
        .unwrap();
        let history = get_import_history(&conn).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].filename, "b.csv");
        assert_eq!(history[1].filename, "a.csv");
    }

}
