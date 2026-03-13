use std::collections::{HashMap, HashSet};
use crate::nlp::preprocessing::{StopWordFilter, preprocess_text};
use crate::nlp::tfidf::build_tfidf_matrix;
use super::scoring::compute_centroid_weighted;
use super::types::{CachedProfilingData, TechnicianProfile};

/// Raw ticket data extracted from DB for profiling.
pub struct ProfilingTicket {
    pub technicien: String,
    pub titre: String,
    pub categorie_niveau1: Option<String>,
    pub categorie_niveau2: Option<String>,
    pub description: String,
    pub solution: String,
    pub date_resolution: Option<String>,
    pub groupe: Option<String>,
}

const TFIDF_MIN_DF: usize = 2;

/// Half-life in days for recency decay (30 days = tickets from 30 days ago
/// have half the weight of today's tickets).
const RECENCY_HALF_LIFE_DAYS: f64 = 30.0;

/// Generate bigrams from a list of stems: ["a","b","c"] → ["a_b","b_c"]
pub fn extract_bigrams(stems: &[String]) -> Vec<String> {
    if stems.len() < 2 {
        return vec![];
    }
    stems.windows(2).map(|w| format!("{}_{}", w[0], w[1])).collect()
}

/// Compute exponential decay weight from a date string relative to today.
fn recency_weight(date_str: &str, today: &chrono::NaiveDate) -> f64 {
    let parsed = chrono::NaiveDate::parse_from_str(
        &date_str[..10.min(date_str.len())],
        "%Y-%m-%d",
    );
    match parsed {
        Ok(d) => {
            let days_ago = (*today - d).num_days().max(0) as f64;
            (-(days_ago * 0.693) / RECENCY_HALF_LIFE_DAYS).exp()
        }
        Err(_) => 0.5, // fallback: half weight for unparseable dates
    }
}

pub fn build_profiles(
    tickets: Vec<ProfilingTicket>,
    periode_from: &str,
    periode_to: &str,
) -> CachedProfilingData {
    if tickets.is_empty() {
        return CachedProfilingData {
            profiles: Vec::new(),
            vocabulary: HashMap::new(),
            idf_values: Vec::new(),
            vocabulary_size: 0,
            nb_tickets_analysed: 0,
            periode_from: periode_from.to_string(),
            periode_to: periode_to.to_string(),
        };
    }

    let nb_tickets = tickets.len();
    let today = chrono::Utc::now().naive_utc().date();

    // Group tickets by technician
    let mut by_tech: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, ticket) in tickets.iter().enumerate() {
        by_tech.entry(ticket.technicien.clone()).or_default().push(idx);
    }

    // Build category distributions (weighted by recency)
    let mut cat_distributions: HashMap<String, HashMap<String, f64>> = HashMap::new();
    for (tech, indices) in &by_tech {
        let mut weighted_counts: HashMap<String, f64> = HashMap::new();
        let mut total_weight = 0.0;
        for &idx in indices {
            let t = &tickets[idx];
            let cat = t.categorie_niveau2.as_deref()
                .or(t.categorie_niveau1.as_deref())
                .unwrap_or("SANS_CATEGORIE");
            let w = t.date_resolution.as_deref()
                .map(|d| recency_weight(d, &today))
                .unwrap_or(0.5);
            *weighted_counts.entry(cat.to_string()).or_insert(0.0) += w;
            total_weight += w;
        }
        if total_weight > 0.0 {
            let dist: HashMap<String, f64> = weighted_counts
                .into_iter()
                .map(|(k, v)| (k, v / total_weight))
                .collect();
            cat_distributions.insert(tech.clone(), dist);
        }
    }

    // Build technician → groups mapping
    let mut tech_groupes: HashMap<String, HashSet<String>> = HashMap::new();
    for ticket in &tickets {
        if let Some(ref g) = ticket.groupe {
            if !g.is_empty() {
                tech_groupes
                    .entry(ticket.technicien.clone())
                    .or_default()
                    .insert(g.clone());
            }
        }
    }

    // Preprocess titles + descriptions + solution for TF-IDF, with bigrams
    let filter = StopWordFilter::new();
    let corpus: Vec<Vec<String>> = tickets
        .iter()
        .map(|t| {
            let mut stems = preprocess_text(&t.titre, &filter);
            if !t.description.is_empty() {
                stems.extend(preprocess_text(&t.description, &filter));
            }
            if !t.solution.is_empty() {
                stems.extend(preprocess_text(&t.solution, &filter));
            }
            let bigrams = extract_bigrams(&stems);
            stems.extend(bigrams);
            stems
        })
        .collect();

    // Compute per-ticket recency weights for centroid
    let weights: Vec<f64> = tickets
        .iter()
        .map(|t| {
            t.date_resolution.as_deref()
                .map(|d| recency_weight(d, &today))
                .unwrap_or(0.5)
        })
        .collect();

    // Build global TF-IDF matrix
    let tfidf_result = build_tfidf_matrix(&corpus, TFIDF_MIN_DF);

    let vocabulary: HashMap<String, usize> = tfidf_result.vocab_index.clone();
    let idf_values = tfidf_result.idf.clone();

    // Build profiles with weighted centroids
    let mut profiles: Vec<TechnicianProfile> = Vec::new();
    for (tech, indices) in &by_tech {
        let row_weights: Vec<f64> = indices.iter().map(|&i| weights[i]).collect();
        let centroid = compute_centroid_weighted(&tfidf_result.matrix, indices, &row_weights);
        let groupes: Vec<String> = tech_groupes
            .remove(tech)
            .map(|s| s.into_iter().collect())
            .unwrap_or_default();
        profiles.push(TechnicianProfile {
            technicien: tech.clone(),
            nb_tickets_reference: indices.len(),
            cat_distribution: cat_distributions.remove(tech).unwrap_or_default(),
            centroide_tfidf: centroid,
            groupes,
        });
    }

    profiles.sort_by(|a, b| a.technicien.cmp(&b.technicien));

    CachedProfilingData {
        profiles,
        vocabulary_size: vocabulary.len(),
        vocabulary,
        idf_values,
        nb_tickets_analysed: nb_tickets,
        periode_from: periode_from.to_string(),
        periode_to: periode_to.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ticket(tech: &str, titre: &str, cat1: Option<&str>, cat2: Option<&str>) -> ProfilingTicket {
        ProfilingTicket {
            technicien: tech.to_string(),
            titre: titre.to_string(),
            categorie_niveau1: cat1.map(|s| s.to_string()),
            categorie_niveau2: cat2.map(|s| s.to_string()),
            description: String::new(),
            solution: String::new(),
            date_resolution: None,
            groupe: None,
        }
    }

    #[test]
    fn test_build_profiles_empty() {
        let result = build_profiles(vec![], "2025-09-12", "2026-03-12");
        assert_eq!(result.profiles.len(), 0);
        assert_eq!(result.nb_tickets_analysed, 0);
    }

    #[test]
    fn test_build_profiles_category_distribution_sums_to_one() {
        let tickets = vec![
            make_ticket("Dupont", "imprimante réseau bloquée", Some("Matériel"), Some("Imprimante")),
            make_ticket("Dupont", "imprimante HP en panne", Some("Matériel"), Some("Imprimante")),
            make_ticket("Dupont", "accès SAP refusé", Some("Habilitations"), None),
        ];
        let result = build_profiles(tickets, "2025-09-12", "2026-03-12");
        assert_eq!(result.profiles.len(), 1);

        let profile = &result.profiles[0];
        assert_eq!(profile.technicien, "Dupont");
        assert_eq!(profile.nb_tickets_reference, 3);

        let sum: f64 = profile.cat_distribution.values().sum();
        assert!((sum - 1.0).abs() < 1e-6, "sum={sum}");
    }

    #[test]
    fn test_build_profiles_uses_niveau2_over_niveau1() {
        let tickets = vec![
            make_ticket("A", "test ticket imprimante", Some("Matériel"), Some("Imprimante")),
        ];
        let result = build_profiles(tickets, "2025-09-12", "2026-03-12");
        let profile = &result.profiles[0];
        assert!(profile.cat_distribution.contains_key("Imprimante"));
        assert!(!profile.cat_distribution.contains_key("Matériel"));
    }

    #[test]
    fn test_build_profiles_fallback_to_niveau1() {
        let tickets = vec![
            make_ticket("A", "test ticket matériel", Some("Matériel"), None),
        ];
        let result = build_profiles(tickets, "2025-09-12", "2026-03-12");
        let profile = &result.profiles[0];
        assert!(profile.cat_distribution.contains_key("Matériel"));
    }

    #[test]
    fn test_build_profiles_no_category_uses_sans_categorie() {
        let tickets = vec![
            make_ticket("A", "test ticket problème réseau", None, None),
        ];
        let result = build_profiles(tickets, "2025-09-12", "2026-03-12");
        let profile = &result.profiles[0];
        assert!(profile.cat_distribution.contains_key("SANS_CATEGORIE"));
    }

    #[test]
    fn test_build_profiles_multiple_technicians() {
        let tickets = vec![
            make_ticket("Dupont", "imprimante bloquée réseau", Some("Matériel"), None),
            make_ticket("Dupont", "imprimante HP cassée panne", Some("Matériel"), None),
            make_ticket("Leroy", "accès SAP refusé compte", Some("Habilitations"), None),
        ];
        let result = build_profiles(tickets, "2025-09-12", "2026-03-12");
        assert_eq!(result.profiles.len(), 2);
        assert_eq!(result.nb_tickets_analysed, 3);
    }

    #[test]
    fn test_build_profiles_centroid_is_l2_normalized() {
        let tickets = vec![
            make_ticket("A", "imprimante réseau bloquée bâtiment", None, None),
            make_ticket("A", "imprimante HP réseau problème connexion", None, None),
            make_ticket("A", "imprimante laser couleur panne totale", None, None),
        ];
        let result = build_profiles(tickets, "2025-09-12", "2026-03-12");
        let profile = &result.profiles[0];

        if !profile.centroide_tfidf.is_empty() {
            let norm: f64 = profile.centroide_tfidf.iter().map(|(_, v)| v * v).sum::<f64>().sqrt();
            assert!((norm - 1.0).abs() < 1e-4, "centroid L2 norm = {norm}, expected 1.0");
        }
    }

    #[test]
    fn test_build_profiles_vocabulary_stored() {
        let tickets = vec![
            make_ticket("A", "imprimante réseau bloquée connexion", None, None),
            make_ticket("B", "imprimante laser problème impression", None, None),
        ];
        let result = build_profiles(tickets, "2025-09-12", "2026-03-12");
        assert!(result.vocabulary_size > 0);
        assert!(!result.vocabulary.is_empty());
        assert!(!result.idf_values.is_empty());
    }

    #[test]
    fn test_extract_bigrams() {
        let stems = vec!["imprim".to_string(), "reseau".to_string(), "problem".to_string()];
        let bigrams = extract_bigrams(&stems);
        assert_eq!(bigrams, vec!["imprim_reseau", "reseau_problem"]);
    }

    #[test]
    fn test_extract_bigrams_single() {
        let stems = vec!["imprim".to_string()];
        assert!(extract_bigrams(&stems).is_empty());
    }

    #[test]
    fn test_recency_weight_today_is_one() {
        let today = chrono::Utc::now().naive_utc().date();
        let date_str = today.format("%Y-%m-%d").to_string();
        let w = recency_weight(&date_str, &today);
        assert!((w - 1.0).abs() < 0.01, "w={w}");
    }

    #[test]
    fn test_recency_weight_30_days_is_half() {
        let today = chrono::Utc::now().naive_utc().date();
        let thirty_days_ago = today - chrono::Duration::days(30);
        let date_str = thirty_days_ago.format("%Y-%m-%d").to_string();
        let w = recency_weight(&date_str, &today);
        assert!((w - 0.5).abs() < 0.05, "w={w}");
    }

    #[test]
    fn test_build_profiles_stores_groupes() {
        let mut t1 = make_ticket("A", "imprimante réseau bloquée", None, None);
        t1.groupe = Some("_DSI > _SUPPORT".to_string());
        let mut t2 = make_ticket("A", "imprimante laser panne", None, None);
        t2.groupe = Some("_DSI > _PRODUCTION".to_string());
        let result = build_profiles(vec![t1, t2], "2025-09-12", "2026-03-12");
        let profile = &result.profiles[0];
        assert_eq!(profile.groupes.len(), 2);
    }
}
