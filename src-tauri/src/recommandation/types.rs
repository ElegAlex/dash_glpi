use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TechnicianProfile {
    pub technicien: String,
    pub nb_tickets_reference: usize,
    pub cat_distribution: HashMap<String, f64>,
    pub centroide_tfidf: Vec<(usize, f64)>,
    pub groupes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedProfilingData {
    pub profiles: Vec<TechnicianProfile>,
    pub vocabulary: HashMap<String, usize>,
    pub idf_values: Vec<f64>,
    pub vocabulary_size: usize,
    pub nb_tickets_analysed: usize,
    pub periode_from: String,
    pub periode_to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilingResult {
    pub profiles_count: usize,
    pub vocabulary_size: usize,
    pub nb_tickets_analysed: usize,
    pub periode_from: String,
    pub periode_to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnassignedTicketStats {
    pub count: usize,
    pub age_moyen_jours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentRecommendation {
    pub ticket_id: i64,
    pub ticket_titre: String,
    pub ticket_categorie: Option<String>,
    pub suggestions: Vec<TechnicianSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TechnicianSuggestion {
    pub technicien: String,
    pub score_final: f64,
    pub score_competence: f64,
    pub score_categorie: f64,
    pub score_tfidf: f64,
    pub stock_actuel: usize,
    pub facteur_charge: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationRequest {
    pub limit_per_ticket: Option<usize>,
    pub score_minimum: Option<f64>,
}

impl RecommendationRequest {
    pub fn limit(&self) -> usize {
        self.limit_per_ticket.unwrap_or(3)
    }
    pub fn min_score(&self) -> f64 {
        self.score_minimum.unwrap_or(0.01)
    }
}

impl From<&CachedProfilingData> for ProfilingResult {
    fn from(data: &CachedProfilingData) -> Self {
        Self {
            profiles_count: data.profiles.len(),
            vocabulary_size: data.vocabulary_size,
            nb_tickets_analysed: data.nb_tickets_analysed,
            periode_from: data.periode_from.clone(),
            periode_to: data.periode_to.clone(),
        }
    }
}
