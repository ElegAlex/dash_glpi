use serde::{Deserialize, Serialize};

use crate::analyzer::stock::enrich_technician_stock;
use crate::config::get_config_from_db;
use crate::db::queries;
use crate::state::{AppState, DbAccess};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockOverview {
    pub total_vivants: usize,
    pub total_termines: usize,
    pub par_statut: Vec<StatutCount>,
    pub age_moyen_jours: f64,
    pub age_median_jours: f64,
    pub par_type: TypeBreakdown,
    pub par_anciennete: Vec<AgeRangeCount>,
    pub inactifs_14j: usize,
    pub inactifs_30j: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatutCount {
    pub statut: String,
    pub count: usize,
    pub est_vivant: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeBreakdown {
    pub incidents: usize,
    pub demandes: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgeRangeCount {
    pub label: String,
    pub threshold_days: usize,
    pub count: usize,
    pub percentage: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockFilters {
    pub statut: Option<String>,
    pub type_ticket: Option<String>,
    pub groupe: Option<String>,
    pub min_anciennete: Option<i64>,
    pub max_anciennete: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TechnicianStock {
    pub technicien: String,
    pub total: usize,
    pub en_cours: usize,
    pub en_attente: usize,
    pub planifie: usize,
    pub nouveau: usize,
    pub incidents: usize,
    pub demandes: usize,
    pub age_moyen_jours: f64,
    pub inactifs_14j: usize,
    pub ecart_seuil: i64,
    pub couleur_seuil: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupStock {
    pub groupe: String,
    pub groupe_niveau1: String,
    pub groupe_niveau2: Option<String>,
    pub total: usize,
    pub en_cours: usize,
    pub en_attente: usize,
    pub incidents: usize,
    pub demandes: usize,
    pub nb_techniciens: usize,
    pub age_moyen_jours: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketSummary {
    pub id: u64,
    pub titre: String,
    pub statut: String,
    pub type_ticket: String,
    pub technicien_principal: Option<String>,
    pub groupe_principal: Option<String>,
    pub date_ouverture: String,
    pub derniere_modification: Option<String>,
    pub anciennete_jours: Option<i64>,
    pub inactivite_jours: Option<i64>,
    pub nombre_suivis: Option<u32>,
    pub action_recommandee: Option<String>,
    pub motif_classification: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketDetail {
    pub id: u64,
    pub titre: String,
    pub statut: String,
    pub type_ticket: String,
    pub priorite: Option<u8>,
    pub urgence: Option<u8>,
    pub demandeur: String,
    pub techniciens: Vec<String>,
    pub groupes: Vec<String>,
    pub date_ouverture: String,
    pub derniere_modification: Option<String>,
    pub nombre_suivis: Option<u32>,
    pub suivis_description: String,
    pub solution: String,
    pub taches_description: String,
    pub anciennete_jours: Option<i64>,
    pub inactivite_jours: Option<i64>,
    pub action_recommandee: Option<String>,
    pub motif_classification: Option<String>,
    pub categorie: Option<String>,
}

#[tauri::command]
pub async fn get_stock_overview(
    state: tauri::State<'_, AppState>,
) -> Result<StockOverview, String> {
    state.db(|conn| queries::get_stock_overview(conn))
}

#[tauri::command]
pub async fn get_stock_by_technician(
    state: tauri::State<'_, AppState>,
    filters: Option<StockFilters>,
) -> Result<Vec<TechnicianStock>, String> {
    state.db(|conn| {
        let config = get_config_from_db(conn)?;
        let mut techs = queries::get_technicians_stock(conn, filters.as_ref())?;
        enrich_technician_stock(&mut techs, &config);
        Ok(techs)
    })
}

#[tauri::command]
pub async fn get_stock_by_group(
    state: tauri::State<'_, AppState>,
    filters: Option<StockFilters>,
) -> Result<Vec<GroupStock>, String> {
    state.db(|conn| queries::get_groups_stock(conn, filters.as_ref()))
}

#[tauri::command]
pub async fn get_ticket_detail(
    state: tauri::State<'_, AppState>,
    ticket_id: u64,
) -> Result<TicketDetail, String> {
    state.db(|conn| queries::get_ticket_detail(conn, ticket_id))
}

#[tauri::command]
pub async fn get_technician_tickets(
    state: tauri::State<'_, AppState>,
    technician: String,
    filters: Option<StockFilters>,
) -> Result<Vec<TicketSummary>, String> {
    state.db(|conn| queries::get_technician_tickets(conn, &technician, filters.as_ref()))
}
