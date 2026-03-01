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
    #[serde(rename_all = "camelCase")]
    Warning { line: usize, message: String },
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportComparison {
    pub import_a: ImportRecord,
    pub import_b: ImportRecord,
    pub delta_total: i64,
    pub delta_vivants: i64,
    pub nouveaux_tickets: Vec<u64>,
    pub disparus_tickets: Vec<u64>,
    pub delta_par_technicien: Vec<TechnicianDelta>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TechnicianDelta {
    pub technicien: String,
    pub count_a: usize,
    pub count_b: usize,
    pub delta: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelinePoint {
    pub import_id: i64,
    pub filename: String,
    pub import_date: String,
    pub vivants_count: usize,
    pub termines_count: usize,
    pub total_rows: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TechTimelinePoint {
    pub import_id: i64,
    pub import_date: String,
    pub ticket_count: usize,
    pub avg_age: f64,
}

#[tauri::command]
pub async fn import_csv(
    state: tauri::State<'_, AppState>,
    path: String,
    on_progress: Channel<ImportEvent>,
) -> Result<ImportResult, String> {
    let start = Instant::now();

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
    let parse_output = crate::parser::parse_csv(&path, move |rows_parsed, _accepted| {
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

    let vivants_count = tickets.iter().filter(|t| t.est_vivant).count();
    let termines_count = tickets.len() - vivants_count;
    let total_tickets = tickets.len();
    let skipped_rows = parse_output.skipped_rows;
    let total_rows = parse_output.total_rows_processed;
    let parse_duration_ms = parse_output.parse_duration_ms;

    // Date range derived from ticket opening dates
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

    // Persist: deactivate previous imports, insert new import record, bulk-insert tickets
    let import_id = state.db_mut(|conn| {
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

    Ok(ImportResult {
        import_id,
        total_tickets,
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

#[tauri::command]
pub async fn compare_imports(
    state: tauri::State<'_, AppState>,
    import_id_a: i64,
    import_id_b: i64,
) -> Result<ImportComparison, String> {
    state.db(|conn| crate::db::queries::compare_imports_logic(conn, import_id_a, import_id_b))
}

#[tauri::command]
pub async fn get_timeline_data(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<TimelinePoint>, String> {
    state.db(|conn| crate::db::queries::get_timeline_data(conn))
}

#[tauri::command]
pub async fn get_technician_timeline(
    state: tauri::State<'_, AppState>,
    technicien: String,
) -> Result<Vec<TechTimelinePoint>, String> {
    state.db(|conn| crate::db::queries::get_technician_timeline(conn, &technicien))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::db::queries::{
        compare_imports_logic, get_import_history, get_technician_timeline, get_timeline_data,
    };

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

    fn insert_import(conn: &Connection, filename: &str, date: &str, total: i64, vivants: i64, termines: i64, active: i64) -> i64 {
        conn.execute(
            "INSERT INTO imports (
                filename, total_rows, parsed_rows, skipped_rows,
                vivants_count, termines_count,
                detected_columns, unique_statuts, unique_types,
                import_date, is_active
             ) VALUES (?1, ?2, ?2, 0, ?3, ?4, '[]', '[]', '[]', ?5, ?6)",
            rusqlite::params![filename, total, vivants, termines, date, active],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn insert_ticket(conn: &Connection, ticket_id: u64, import_id: i64, est_vivant: i64, tech: &str) {
        conn.execute(
            "INSERT INTO tickets (id, import_id, titre, statut, type_ticket, date_ouverture, est_vivant, technicien_principal)
             VALUES (?1, ?2, 'T', 'En cours (Attribué)', 'Incident', '2026-01-01', ?3, ?4)",
            rusqlite::params![ticket_id, import_id, est_vivant, tech],
        )
        .unwrap();
    }

    /// GIVEN 2 imports A and B with overlapping tickets
    /// WHEN compare_imports_logic is called
    /// THEN delta_total, delta_vivants correct, nouveaux/disparus correct, tech deltas correct
    #[test]
    fn test_compare_imports_basic() {
        let conn = setup_db();
        let id_a = insert_import(&conn, "a.csv", "2026-01-01T10:00:00", 3, 2, 1, 0);
        let id_b = insert_import(&conn, "b.csv", "2026-02-01T10:00:00", 4, 3, 1, 1);

        // A: tickets 1, 2, 3 — all vivant, Dupont
        for tid in [1u64, 2, 3] {
            insert_ticket(&conn, tid, id_a, 1, "Dupont");
        }
        // B: tickets 1, 2, 4 — all vivant, Dupont; 1 extra (4) is nouveaux, 3 is disparu
        for tid in [1u64, 2, 4] {
            insert_ticket(&conn, tid, id_b, 1, "Dupont");
        }

        let cmp = compare_imports_logic(&conn, id_a, id_b).unwrap();

        assert_eq!(cmp.delta_total, 1);  // 4 - 3
        assert_eq!(cmp.delta_vivants, 1); // 3 - 2
        assert_eq!(cmp.nouveaux_tickets, vec![4u64]);
        assert_eq!(cmp.disparus_tickets, vec![3u64]);
        assert_eq!(cmp.delta_par_technicien.len(), 1);
        let delta = &cmp.delta_par_technicien[0];
        assert_eq!(delta.technicien, "Dupont");
        assert_eq!(delta.count_a, 3);
        assert_eq!(delta.count_b, 3);
        assert_eq!(delta.delta, 0);
    }

    /// GIVEN 2 imports with completely different tickets
    /// WHEN compare_imports_logic is called
    /// THEN all tickets from A are disparus, all from B are nouveaux
    #[test]
    fn test_compare_imports_no_overlap() {
        let conn = setup_db();
        let id_a = insert_import(&conn, "a.csv", "2026-01-01T10:00:00", 2, 2, 0, 0);
        let id_b = insert_import(&conn, "b.csv", "2026-02-01T10:00:00", 2, 2, 0, 1);

        insert_ticket(&conn, 10, id_a, 1, "Martin");
        insert_ticket(&conn, 11, id_a, 1, "Martin");
        insert_ticket(&conn, 20, id_b, 1, "Durand");
        insert_ticket(&conn, 21, id_b, 1, "Durand");

        let cmp = compare_imports_logic(&conn, id_a, id_b).unwrap();

        let mut nouveaux = cmp.nouveaux_tickets.clone();
        nouveaux.sort_unstable();
        let mut disparus = cmp.disparus_tickets.clone();
        disparus.sort_unstable();

        assert_eq!(nouveaux, vec![20u64, 21]);
        assert_eq!(disparus, vec![10u64, 11]);
        assert_eq!(cmp.delta_par_technicien.iter().find(|d| d.technicien == "Martin").map(|d| d.delta), Some(-2));
        assert_eq!(cmp.delta_par_technicien.iter().find(|d| d.technicien == "Durand").map(|d| d.delta), Some(2));
    }

    /// GIVEN 3 imports inserted out of order
    /// WHEN get_timeline_data is called
    /// THEN results are returned ASC by import_date
    #[test]
    fn test_timeline_data() {
        let conn = setup_db();
        insert_import(&conn, "c.csv", "2026-03-01T10:00:00", 30, 20, 10, 1);
        insert_import(&conn, "a.csv", "2026-01-01T10:00:00", 10, 5, 5, 0);
        insert_import(&conn, "b.csv", "2026-02-01T10:00:00", 20, 15, 5, 0);

        let timeline = get_timeline_data(&conn).unwrap();
        assert_eq!(timeline.len(), 3);
        assert_eq!(timeline[0].filename, "a.csv");
        assert_eq!(timeline[1].filename, "b.csv");
        assert_eq!(timeline[2].filename, "c.csv");
        assert_eq!(timeline[2].vivants_count, 20);
        assert_eq!(timeline[2].total_rows, 30);
    }

    /// GIVEN tickets for a tech spread across 2 imports
    /// WHEN get_technician_timeline is called
    /// THEN returns 2 points with correct ticket_count
    #[test]
    fn test_technician_timeline() {
        let conn = setup_db();
        let id_a = insert_import(&conn, "a.csv", "2026-01-01T10:00:00", 3, 3, 0, 0);
        let id_b = insert_import(&conn, "b.csv", "2026-02-01T10:00:00", 5, 5, 0, 1);

        // 3 vivant tickets for "Dupont" in A
        for tid in [1u64, 2, 3] {
            conn.execute(
                "INSERT INTO tickets (id, import_id, titre, statut, type_ticket, date_ouverture, est_vivant, technicien_principal, anciennete_jours)
                 VALUES (?1, ?2, 'T', 'En cours (Attribué)', 'Incident', '2026-01-01', 1, 'Dupont', 10)",
                rusqlite::params![tid, id_a],
            ).unwrap();
        }
        // 5 vivant tickets for "Dupont" in B
        for tid in [1u64, 2, 3, 4, 5] {
            conn.execute(
                "INSERT INTO tickets (id, import_id, titre, statut, type_ticket, date_ouverture, est_vivant, technicien_principal, anciennete_jours)
                 VALUES (?1, ?2, 'T', 'En cours (Attribué)', 'Incident', '2026-01-01', 1, 'Dupont', 20)",
                rusqlite::params![tid, id_b],
            ).unwrap();
        }
        // 2 tickets for another tech in B (should not appear)
        insert_ticket(&conn, 10, id_b, 1, "Martin");
        insert_ticket(&conn, 11, id_b, 1, "Martin");

        let timeline = get_technician_timeline(&conn, "Dupont").unwrap();
        assert_eq!(timeline.len(), 2);
        assert_eq!(timeline[0].ticket_count, 3); // A
        assert_eq!(timeline[1].ticket_count, 5); // B
        assert!((timeline[0].avg_age - 10.0).abs() < 0.01);
        assert!((timeline[1].avg_age - 20.0).abs() < 0.01);
    }
}
