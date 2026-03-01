mod analyzer;
mod analytics;
mod commands;
mod config;
mod db;
mod error;
mod export;
mod nlp;
mod parser;
mod state;

use state::AppState;
use std::sync::Mutex;
use tauri::Manager;

pub fn run() {
    let app_state = AppState {
        db: Mutex::new(None),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(|app| {
            let app_handle = app.handle().clone();
            let db_path = app_handle
                .path()
                .app_data_dir()
                .expect("Impossible de résoudre app_data_dir")
                .join("glpi_dashboard.db");

            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let conn = db::setup::init_db(db_path.to_str().unwrap())?;

            let state: tauri::State<AppState> = app.state();
            *state.db.lock().unwrap() = Some(conn);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Import
            commands::import::import_csv,
            commands::import::get_import_history,
            commands::import::compare_imports,
            commands::import::get_timeline_data,
            commands::import::get_technician_timeline,
            // Stock
            commands::stock::get_stock_overview,
            commands::stock::get_stock_by_technician,
            commands::stock::get_stock_by_group,
            commands::stock::get_ticket_detail,
            commands::stock::get_technician_tickets,
            // Bilan
            commands::bilan::get_bilan_temporel,
            // Catégories
            commands::categories::get_categories_tree,
            // Data mining
            commands::mining::run_text_analysis,
            commands::mining::get_clusters,
            commands::mining::detect_anomalies,
            commands::mining::detect_duplicates,
            commands::mining::get_cooccurrence_network,
            commands::mining::get_cluster_detail,
            // Export
            commands::export::export_excel_stock,
            commands::export::export_excel_bilan,
            commands::export::export_excel_plan_action,
            commands::export::export_all_plans_zip,
            // Config
            commands::config::get_config,
            commands::config::update_config,
            // Search
            commands::search::search_tickets,
            // Analytics
            commands::analytics::predict_workload,
            // Dashboard KPI
            commands::dashboard::get_dashboard_kpi,
        ])
        .run(tauri::generate_context!())
        .expect("Erreur au lancement de l'application");
}

// ─── E2E Integration Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod e2e_tests {
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("db/sql/001_initial.sql"))
            .unwrap();
        conn
    }

    /// E2E: parse real ticket.csv → classify → insert → query stock overview + technicians + FTS
    #[test]
    fn test_e2e_import_and_query_pipeline() {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let fixture_path = format!("{}/../ticket.csv", manifest_dir);
        if !std::path::Path::new(&fixture_path).exists() {
            eprintln!("Skipping E2E test: ticket.csv not found at {}", fixture_path);
            return;
        }

        // 1. Parse CSV
        let parse_output =
            crate::parser::parse_csv(&fixture_path, |_, _| {}).expect("CSV parsing failed");

        assert!(
            parse_output.tickets.len() > 100,
            "Expected >100 tickets, got {}",
            parse_output.tickets.len()
        );

        // 2. Classify tickets
        let mut conn = setup_db();
        let config =
            crate::config::get_config_from_db(&conn).expect("Config loading failed");

        let mut tickets = parse_output.tickets;
        for ticket in &mut tickets {
            crate::analyzer::classifier::classify_ticket(ticket, &config);
        }

        let vivants_count = tickets.iter().filter(|t| t.est_vivant).count();
        let termines_count = tickets.len() - vivants_count;
        assert!(vivants_count > 0, "Should have vivant tickets");
        assert!(termines_count > 0, "Should have terminé tickets");

        // Verify classifier assigned actions to some vivant tickets
        let classified_count = tickets
            .iter()
            .filter(|t| t.est_vivant && t.action_recommandee.is_some())
            .count();
        assert!(
            classified_count > 0,
            "Classifier should assign actions to some vivant tickets"
        );

        // 3. Insert import record + tickets
        let total = tickets.len();
        conn.execute(
            "INSERT INTO imports (
                filename, total_rows, parsed_rows, skipped_rows,
                vivants_count, termines_count,
                detected_columns, unique_statuts, unique_types,
                is_active
             ) VALUES ('ticket.csv', ?1, ?2, ?3, ?4, ?5, '[]', '[]', '[]', 1)",
            rusqlite::params![
                total as i64,
                total as i64,
                parse_output.skipped_rows as i64,
                vivants_count as i64,
                termines_count as i64,
            ],
        )
        .expect("Import record insertion failed");
        let import_id = conn.last_insert_rowid();

        crate::db::insert::bulk_insert_tickets(&mut conn, import_id, &tickets)
            .expect("Bulk insert failed");

        // 4. Verify get_stock_overview
        let overview =
            crate::db::queries::get_stock_overview(&conn).expect("get_stock_overview failed");

        assert_eq!(overview.total_vivants, vivants_count);
        assert_eq!(overview.total_termines, termines_count);
        assert!(
            overview.total_vivants + overview.total_termines == total,
            "vivants + terminés should equal total"
        );
        assert!(overview.age_moyen_jours > 0.0, "Age moyen should be > 0");
        assert!(overview.age_median_jours > 0.0, "Age médian should be > 0");
        assert!(!overview.par_statut.is_empty(), "par_statut should not be empty");
        assert!(
            overview.par_type.incidents + overview.par_type.demandes == overview.total_vivants,
            "incidents + demandes should equal total_vivants"
        );
        // Age distribution should sum to total vivants
        let age_sum: usize = overview.par_anciennete.iter().map(|a| a.count).sum();
        assert_eq!(
            age_sum, overview.total_vivants,
            "Age distribution sum should equal total vivants"
        );

        // 5. Verify get_technicians_stock
        let techs = crate::db::queries::get_technicians_stock(&conn, None)
            .expect("get_technicians_stock failed");

        assert!(!techs.is_empty(), "Should have at least one technician");
        for tech in &techs {
            assert!(
                !tech.technicien.is_empty(),
                "Technician name should not be empty"
            );
            assert!(tech.total > 0, "Technician should have at least 1 ticket");
            // Verify RAG couleur is one of the valid values
            assert!(
                ["vert", "jaune", "orange", "rouge"].contains(&tech.couleur_seuil.as_str()),
                "couleur_seuil '{}' should be vert/jaune/orange/rouge",
                tech.couleur_seuil
            );
        }

        // Verify enrich_technician_stock works
        let mut techs_enriched = crate::db::queries::get_technicians_stock(&conn, None).unwrap();
        crate::analyzer::stock::enrich_technician_stock(&mut techs_enriched, &config);

        // 6. Verify get_category_tree_data
        let categories =
            crate::db::queries::get_category_tree_data(&conn).expect("get_category_tree_data failed");
        assert!(
            !categories.is_empty(),
            "Should have category/group data"
        );

        // 7. Verify FTS search
        let search_results = crate::db::queries::search_tickets_fts(&conn, "réseau", 10);
        // FTS search should not error (may return 0 results depending on data)
        assert!(search_results.is_ok(), "FTS search should not error");

        // 8. Verify get_ticket_detail for a known ticket
        let first_ticket_id = tickets[0].id as u64;
        let detail = crate::db::queries::get_ticket_detail(&conn, first_ticket_id)
            .expect("get_ticket_detail failed for first ticket");
        assert_eq!(detail.id, first_ticket_id);
        assert!(!detail.statut.is_empty());

        // 9. Verify get_import_history
        let history =
            crate::db::queries::get_import_history(&conn).expect("get_import_history failed");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].filename, "ticket.csv");
        assert!(history[0].is_active);
    }

    // ── Phase 2 E2E Tests ────────────────────────────────────────────────────

    /// Helper: setup DB, parse, classify, insert real tickets, return conn
    fn setup_with_real_data() -> Option<Connection> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let fixture_path = format!("{}/../ticket.csv", manifest_dir);
        if !std::path::Path::new(&fixture_path).exists() {
            eprintln!("Skipping E2E test: ticket.csv not found at {}", fixture_path);
            return None;
        }

        let parse_output =
            crate::parser::parse_csv(&fixture_path, |_, _| {}).expect("CSV parsing failed");

        let conn = setup_db();
        let config =
            crate::config::get_config_from_db(&conn).expect("Config loading failed");

        let mut tickets = parse_output.tickets;
        for ticket in &mut tickets {
            crate::analyzer::classifier::classify_ticket(ticket, &config);
        }

        let vivants_count = tickets.iter().filter(|t| t.est_vivant).count();
        let termines_count = tickets.len() - vivants_count;
        let total = tickets.len();

        conn.execute(
            "INSERT INTO imports (
                filename, total_rows, parsed_rows, skipped_rows,
                vivants_count, termines_count,
                detected_columns, unique_statuts, unique_types,
                is_active
             ) VALUES ('ticket.csv', ?1, ?2, ?3, ?4, ?5, '[]', '[]', '[]', 1)",
            rusqlite::params![
                total as i64,
                total as i64,
                parse_output.skipped_rows as i64,
                vivants_count as i64,
                termines_count as i64,
            ],
        )
        .expect("Import record insertion failed");
        let import_id = conn.last_insert_rowid();

        let mut conn = conn;
        crate::db::insert::bulk_insert_tickets(&mut conn, import_id, &tickets)
            .expect("Bulk insert failed");

        Some(conn)
    }

    /// E2E: get_bilan_temporel → entrants > 0, sortants > 0, stock cumulé cohérent
    #[test]
    fn test_e2e_bilan_temporel() {
        let conn = match setup_with_real_data() {
            Some(d) => d,
            None => return,
        };

        let state = crate::state::AppState {
            db: std::sync::Mutex::new(Some(conn)),
        };

        let request = crate::commands::bilan::BilanRequest {
            period: "month".to_string(),
            date_from: "2025-01-01".to_string(),
            date_to: "2026-12-31".to_string(),
            group_by: None,
        };

        let bilan = crate::commands::bilan::run_bilan_logic(&state, &request)
            .expect("run_bilan_logic failed");

        assert!(
            !bilan.periodes.is_empty(),
            "Should have at least one period"
        );

        assert!(
            bilan.totaux.total_entrees > 0,
            "Total entrants should be > 0, got {}",
            bilan.totaux.total_entrees
        );
        assert!(
            bilan.totaux.total_sorties > 0,
            "Total sortants should be > 0, got {}",
            bilan.totaux.total_sorties
        );

        // Verify stock_cumule coherence: stock_debut + Σentrants - Σsortants = last stock_cumule
        let from_str = "2025-01-01";
        let stock_debut = {
            let guard = state.db.lock().unwrap();
            let conn = guard.as_ref().unwrap();
            crate::db::queries::get_stock_at_date(conn, from_str)
                .expect("get_stock_at_date failed")
        };

        let mut expected_stock = stock_debut as i64;
        for p in &bilan.periodes {
            expected_stock = (expected_stock + p.entrees as i64 - p.sorties as i64).max(0);
            assert_eq!(
                p.stock_cumule,
                Some(expected_stock as usize),
                "Stock cumulé mismatch at period {}",
                p.period_key
            );
        }
    }

    /// E2E: generate_stock_report → bytes start with PK (0x50 0x4B)
    #[test]
    fn test_e2e_export_stock_report_pk() {
        let conn = match setup_with_real_data() {
            Some(d) => d,
            None => return,
        };

        let overview =
            crate::db::queries::get_stock_overview(&conn).expect("get_stock_overview failed");
        let technicians = crate::db::queries::get_technicians_stock(&conn, None)
            .expect("get_technicians_stock failed");
        let groups = crate::db::queries::get_groups_stock(&conn, None)
            .expect("get_groups_stock failed");

        let bytes =
            crate::export::stock_report::generate_stock_report(&overview, &technicians, &groups)
                .expect("generate_stock_report failed");

        assert!(bytes.len() > 4, "XLSX bytes should be non-trivial");
        assert_eq!(bytes[0], 0x50, "First byte should be 0x50 (P)");
        assert_eq!(bytes[1], 0x4B, "Second byte should be 0x4B (K)");
    }

    /// E2E: generate_plan_action for a technician → bytes start with PK
    #[test]
    fn test_e2e_export_plan_action_pk() {
        let conn = match setup_with_real_data() {
            Some(d) => d,
            None => return,
        };

        let technicians = crate::db::queries::get_technicians_stock(&conn, None)
            .expect("get_technicians_stock failed");

        // Pick first technician with tickets
        let tech = technicians
            .iter()
            .find(|t| t.total > 0)
            .expect("Should have at least one technician with tickets");

        let tickets =
            crate::db::queries::get_technician_tickets(&conn, &tech.technicien, None)
                .expect("get_technician_tickets failed");

        let bytes =
            crate::export::plan_action::generate_plan_action(&tech.technicien, tech, &tickets)
                .expect("generate_plan_action failed");

        assert!(bytes.len() > 4, "XLSX bytes should be non-trivial");
        assert_eq!(bytes[0], 0x50, "First byte should be 0x50 (P)");
        assert_eq!(bytes[1], 0x4B, "Second byte should be 0x4B (K)");
    }

    /// E2E: generate_bilan_report → bytes start with PK
    #[test]
    fn test_e2e_export_bilan_report_pk() {
        let conn = match setup_with_real_data() {
            Some(d) => d,
            None => return,
        };

        let state = crate::state::AppState {
            db: std::sync::Mutex::new(Some(conn)),
        };

        let request = crate::commands::bilan::BilanRequest {
            period: "month".to_string(),
            date_from: "2025-01-01".to_string(),
            date_to: "2026-12-31".to_string(),
            group_by: None,
        };

        let bilan = crate::commands::bilan::run_bilan_logic(&state, &request)
            .expect("run_bilan_logic failed");

        let bytes =
            crate::export::bilan_report::generate_bilan_report(&bilan, &request)
                .expect("generate_bilan_report failed");

        assert!(bytes.len() > 4, "XLSX bytes should be non-trivial");
        assert_eq!(bytes[0], 0x50, "First byte should be 0x50 (P)");
        assert_eq!(bytes[1], 0x4B, "Second byte should be 0x4B (K)");
    }

    // ── Dashboard KPI E2E Test ───────────────────────────────────────────────

    #[test]
    fn test_e2e_dashboard_kpi() {
        let conn = match setup_with_real_data() {
            Some(c) => c,
            None => return,
        };
        let kpi = crate::analyzer::dashboard::build_dashboard_kpi(&conn, 1, &None, &None, "month")
            .expect("build_dashboard_kpi failed");
        assert!(kpi.meta.total_tickets > 0);
        assert!(kpi.resolution.mttr_global_jours > 0.0);
        assert!(kpi.resolution.echantillon > 0);
        assert!(!kpi.volumes.par_mois.is_empty());
        assert!(kpi.taux_n1.objectif_itil == 75.0);
        assert!(
            kpi.taux_n1.n1_strict.pourcentage >= 0.0
                && kpi.taux_n1.n1_strict.pourcentage <= 100.0
        );
    }
}
