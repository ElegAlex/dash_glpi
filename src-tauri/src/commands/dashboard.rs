use crate::analyzer::dashboard::{build_dashboard_kpi, DashboardKpi};
use crate::db::queries::get_active_import_id;
use crate::state::{AppState, DbAccess};

/// Returns the complete Dashboard KPI ITSM payload for the active import.
///
/// Optional `date_debut` and `date_fin` parameters (ISO format) filter tickets
/// by `date_ouverture`.
#[tauri::command]
pub async fn get_dashboard_kpi(
    state: tauri::State<'_, AppState>,
    date_debut: Option<String>,
    date_fin: Option<String>,
) -> Result<DashboardKpi, String> {
    state.db(|conn| {
        let import_id = get_active_import_id(conn)?;
        build_dashboard_kpi(conn, import_id, &date_debut, &date_fin)
    })
}
