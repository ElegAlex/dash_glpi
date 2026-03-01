use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct GlpiTicketRaw {
    pub id: Option<String>,
    pub titre: Option<String>,
    pub statut: Option<String>,
    pub type_ticket: Option<String>,
    pub priorite: Option<String>,
    pub urgence: Option<String>,
    pub demandeur: Option<String>,
    pub date_ouverture: Option<String>,
    pub derniere_modification: Option<String>,
    pub nombre_suivis: Option<String>,
    pub suivis_description: Option<String>,
    pub solution: Option<String>,
    pub taches_description: Option<String>,
    pub intervention_fournisseur: Option<String>,
    pub technicien: Option<String>,
    pub groupe: Option<String>,
    pub categorie: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GlpiTicketNormalized {
    pub id: i64,
    pub titre: String,
    pub statut: String,
    pub type_ticket: String,
    pub priorite: Option<i32>,
    pub urgence: Option<i32>,
    pub demandeur: String,
    pub date_ouverture: String,
    pub derniere_modification: Option<String>,
    pub nombre_suivis: Option<i32>,
    pub suivis_description: String,
    pub solution: String,
    pub taches_description: String,
    pub intervention_fournisseur: String,
    pub techniciens: Vec<String>,
    pub groupes: Vec<String>,
    pub technicien_principal: Option<String>,
    pub groupe_principal: Option<String>,
    pub groupe_niveau1: Option<String>,
    pub groupe_niveau2: Option<String>,
    pub groupe_niveau3: Option<String>,
    pub categorie: Option<String>,
    pub categorie_niveau1: Option<String>,
    pub categorie_niveau2: Option<String>,
    pub est_vivant: bool,
    pub anciennete_jours: Option<i64>,
    pub inactivite_jours: Option<i64>,
    pub date_cloture_approx: Option<String>,
    pub action_recommandee: Option<String>,
    pub motif_classification: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvImportResult {
    pub import_id: i64,
    pub total_tickets: usize,
    pub vivants_count: usize,
    pub termines_count: usize,
    pub skipped_rows: usize,
    pub warnings: Vec<ParseWarning>,
    pub detected_columns: Vec<String>,
    pub missing_optional_columns: Vec<String>,
    pub unique_statuts: Vec<String>,
    pub parse_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseWarning {
    pub line: usize,
    pub message: String,
}
