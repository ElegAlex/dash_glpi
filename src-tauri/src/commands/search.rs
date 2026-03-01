use serde::Serialize;

use crate::db::queries;
use crate::state::{AppState, DbAccess};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketSearchResult {
    pub id: u64,
    pub titre: String,
    pub statut: String,
    pub technicien: Option<String>,
    pub titre_highlight: String,
    pub solution_highlight: Option<String>,
    pub rank: f64,
}

#[tauri::command]
pub async fn search_tickets(
    state: tauri::State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<TicketSearchResult>, String> {
    let limit = limit.unwrap_or(50);
    state.db(|conn| queries::search_tickets_fts(conn, &query, limit))
}
