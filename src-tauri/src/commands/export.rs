use std::io::Write as _;
use std::time::Instant;

use serde::Serialize;

use crate::db::queries;
use crate::export::bilan_report;
use crate::export::plan_action;
use crate::export::stock_report;
use crate::state::{AppState, DbAccess};

use super::bilan::{run_bilan_logic, BilanRequest};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    pub size_bytes: u64,
    pub duration_ms: u64,
}

#[tauri::command]
pub async fn export_excel_stock(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<ExportResult, String> {
    let start = Instant::now();

    let overview = state.db(|conn| queries::get_stock_overview(conn))?;
    let technicians = state.db(|conn| queries::get_technicians_stock(conn, None))?;
    let groups = state.db(|conn| queries::get_groups_stock(conn, None))?;

    let bytes = stock_report::generate_stock_report(&overview, &technicians, &groups)
        .map_err(|e| e.to_string())?;

    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;

    Ok(ExportResult {
        path,
        size_bytes: bytes.len() as u64,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

#[tauri::command]
pub async fn export_excel_bilan(
    state: tauri::State<'_, AppState>,
    path: String,
    request: BilanRequest,
) -> Result<ExportResult, String> {
    let start = Instant::now();

    let bilan = run_bilan_logic(&state, &request)?;
    let bytes = bilan_report::generate_bilan_report(&bilan, &request)
        .map_err(|e| e.to_string())?;

    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;

    Ok(ExportResult {
        path,
        size_bytes: bytes.len() as u64,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

#[tauri::command]
pub async fn export_excel_plan_action(
    state: tauri::State<'_, AppState>,
    path: String,
    technician: String,
) -> Result<ExportResult, String> {
    let start = Instant::now();

    let technicians = state.db(|conn| queries::get_technicians_stock(conn, None))?;
    let stats = technicians
        .into_iter()
        .find(|t| t.technicien == technician)
        .ok_or_else(|| format!("Technicien introuvable: {}", technician))?;

    let tickets = state.db(|conn| queries::get_technician_tickets(conn, &technician, None))?;

    let bytes = plan_action::generate_plan_action(&technician, &stats, &tickets)
        .map_err(|e| e.to_string())?;

    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;

    Ok(ExportResult {
        path,
        size_bytes: bytes.len() as u64,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

#[tauri::command]
pub async fn export_all_plans_zip(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<ExportResult, String> {
    let start = Instant::now();

    let technicians = state.db(|conn| queries::get_technicians_stock(conn, None))?;

    let cursor = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(cursor);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for tech in technicians.iter().filter(|t| t.total > 0) {
        let tickets =
            state.db(|conn| queries::get_technician_tickets(conn, &tech.technicien, None))?;

        let bytes = plan_action::generate_plan_action(&tech.technicien, tech, &tickets)
            .map_err(|e| e.to_string())?;

        // Sanitize filename for ZIP entry
        let safe_name: String = tech
            .technicien
            .chars()
            .map(|c| {
                if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        let filename = format!("{}.xlsx", safe_name);

        zip.start_file(filename, options).map_err(|e| e.to_string())?;
        zip.write_all(&bytes).map_err(|e| e.to_string())?;
    }

    let cursor = zip.finish().map_err(|e| e.to_string())?;
    let buf = cursor.into_inner();

    std::fs::write(&path, &buf).map_err(|e| e.to_string())?;

    Ok(ExportResult {
        path,
        size_bytes: buf.len() as u64,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}
