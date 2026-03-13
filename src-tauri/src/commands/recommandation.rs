use tauri::State;
use crate::state::AppState;
use crate::db::queries::{
    get_active_import_id, get_profiling_tickets, get_unassigned_tickets_for_attribution,
    get_technician_stock_counts, get_cached_profiling, save_cached_profiling,
    get_seuil_tickets, get_unassigned_ticket_stats,
};
use crate::recommandation::profiling::{build_profiles, ProfilingTicket};
use crate::recommandation::scoring::{score_tickets, UnassignedTicket};
use crate::recommandation::types::{
    AssignmentRecommendation, CachedProfilingData, ProfilingResult, RecommendationRequest,
    UnassignedTicketStats,
};

const PERIODE_PROFIL_MOIS: i64 = 3;
const MAX_UNASSIGNED_TICKETS: usize = 500;

#[tauri::command]
pub async fn build_technician_profiles(
    state: State<'_, AppState>,
) -> Result<ProfilingResult, String> {
    let start = std::time::Instant::now();

    // Phase 1: Read data from DB (hold lock briefly)
    let (import_id, raw_tickets) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let conn = db.as_ref().ok_or("Base de données non initialisée")?;
        let import_id = get_active_import_id(conn).map_err(|e| e.to_string())?;
        let rows = get_profiling_tickets(conn, import_id, PERIODE_PROFIL_MOIS)
            .map_err(|e| e.to_string())?;
        (import_id, rows)
    };
    // Lock released here

    // Convert to profiling tickets
    let tickets: Vec<ProfilingTicket> = raw_tickets
        .into_iter()
        .map(|(tech, titre, cat1, cat2, desc, sol, date_res, groupe)| ProfilingTicket {
            technicien: tech,
            titre,
            categorie_niveau1: cat1,
            categorie_niveau2: cat2,
            description: desc,
            solution: sol,
            date_resolution: date_res,
            groupe,
        })
        .collect();

    // Phase 2: Heavy computation (no lock held)
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let six_months_ago = chrono::Utc::now()
        .naive_utc()
        .date()
        .checked_sub_months(chrono::Months::new(PERIODE_PROFIL_MOIS as u32))
        .unwrap_or(chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap())
        .format("%Y-%m-%d")
        .to_string();

    let cached_data = build_profiles(tickets, &six_months_ago, &today);
    let result = ProfilingResult::from(&cached_data);

    // Phase 3: Write cache (hold lock briefly)
    let json = serde_json::to_string(&cached_data)
        .map_err(|e| format!("Erreur sérialisation JSON: {e}"))?;
    let duration_ms = start.elapsed().as_millis() as i64;

    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let conn = db.as_ref().ok_or("Base de données non initialisée")?;
        save_cached_profiling(conn, import_id, &json, duration_ms)
            .map_err(|e| e.to_string())?;
    }

    Ok(result)
}

#[tauri::command]
pub async fn get_assignment_recommendations(
    state: State<'_, AppState>,
    request: RecommendationRequest,
) -> Result<Vec<AssignmentRecommendation>, String> {
    // Phase 1: Read all needed data from DB (hold lock briefly)
    let (profiling_data, tickets, stock_map, seuil) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let conn = db.as_ref().ok_or("Base de données non initialisée")?;

        let import_id = get_active_import_id(conn).map_err(|e| e.to_string())?;

        let json = get_cached_profiling(conn, import_id)
            .map_err(|e| e.to_string())?
            .ok_or("Profils non calculés. Cliquez sur 'Analyser' d'abord.")?;

        let profiling_data: CachedProfilingData = serde_json::from_str(&json)
            .map_err(|e| format!("Erreur désérialisation cache: {e}"))?;

        let raw_tickets = get_unassigned_tickets_for_attribution(conn, import_id, MAX_UNASSIGNED_TICKETS)
            .map_err(|e| e.to_string())?;

        let tickets: Vec<UnassignedTicket> = raw_tickets
            .into_iter()
            .map(|(id, titre, cat1, cat2, desc, groupe)| UnassignedTicket {
                id,
                titre,
                categorie_niveau1: cat1,
                categorie_niveau2: cat2,
                description: desc,
                groupe,
            })
            .collect();

        let stock_map = get_technician_stock_counts(conn, import_id)
            .map_err(|e| e.to_string())?;

        let seuil = get_seuil_tickets(conn) as f64;

        (profiling_data, tickets, stock_map, seuil)
    };
    // Lock released here

    // Phase 2: Score (no DB access needed)
    let results = score_tickets(
        &tickets,
        &profiling_data,
        &stock_map,
        seuil,
        request.limit(),
        request.min_score(),
    );

    Ok(results)
}

#[tauri::command]
pub async fn get_unassigned_ticket_stats_cmd(
    state: State<'_, AppState>,
) -> Result<UnassignedTicketStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.as_ref().ok_or("Base de données non initialisée")?;
    let import_id = get_active_import_id(conn).map_err(|e| e.to_string())?;
    let (count, age_moyen_jours) =
        get_unassigned_ticket_stats(conn, import_id).map_err(|e| e.to_string())?;
    Ok(UnassignedTicketStats {
        count,
        age_moyen_jours,
    })
}
