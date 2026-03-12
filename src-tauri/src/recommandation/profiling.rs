use std::collections::HashMap;
use crate::nlp::preprocessing::{StopWordFilter, preprocess_text};
use crate::nlp::tfidf::build_tfidf_matrix;
use super::scoring::compute_centroid;
use super::types::{CachedProfilingData, TechnicianProfile};

/// Raw ticket data extracted from DB for profiling.
pub struct ProfilingTicket {
    pub technicien: String,
    pub titre: String,
    pub categorie_niveau1: Option<String>,
    pub categorie_niveau2: Option<String>,
}

const TFIDF_MIN_DF: usize = 2;

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

    // Group tickets by technician
    let mut by_tech: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, ticket) in tickets.iter().enumerate() {
        by_tech.entry(ticket.technicien.clone()).or_default().push(idx);
    }

    // Build category distributions
    let mut cat_distributions: HashMap<String, HashMap<String, f64>> = HashMap::new();
    for (tech, indices) in &by_tech {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for &idx in indices {
            let t = &tickets[idx];
            let cat = t.categorie_niveau2.as_deref()
                .or(t.categorie_niveau1.as_deref())
                .unwrap_or("SANS_CATEGORIE");
            *counts.entry(cat.to_string()).or_insert(0) += 1;
        }
        let total = indices.len() as f64;
        let dist: HashMap<String, f64> = counts
            .into_iter()
            .map(|(k, v)| (k, v as f64 / total))
            .collect();
        cat_distributions.insert(tech.clone(), dist);
    }

    // Preprocess all titles for TF-IDF
    let filter = StopWordFilter::new();
    let corpus: Vec<Vec<String>> = tickets
        .iter()
        .map(|t| preprocess_text(&t.titre, &filter))
        .collect();

    // Build global TF-IDF matrix
    let tfidf_result = build_tfidf_matrix(&corpus, TFIDF_MIN_DF);

    // Build vocabulary map (stem → index) — use vocab_index from TfIdfResult
    let vocabulary: HashMap<String, usize> = tfidf_result.vocab_index.clone();
    let idf_values = tfidf_result.idf.clone();

    // Build profiles with centroids
    let mut profiles: Vec<TechnicianProfile> = Vec::new();
    for (tech, indices) in &by_tech {
        let centroid = compute_centroid(&tfidf_result.matrix, indices);
        profiles.push(TechnicianProfile {
            technicien: tech.clone(),
            nb_tickets_reference: indices.len(),
            cat_distribution: cat_distributions.remove(tech).unwrap_or_default(),
            centroide_tfidf: centroid,
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

        assert!((profile.cat_distribution["Imprimante"] - 2.0 / 3.0).abs() < 1e-6);
        assert!((profile.cat_distribution["Habilitations"] - 1.0 / 3.0).abs() < 1e-6);
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
}
