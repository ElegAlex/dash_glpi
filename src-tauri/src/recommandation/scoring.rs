use sprs::CsMat;
use std::collections::HashMap;
use super::types::{
    AssignmentRecommendation, CachedProfilingData, TechnicianSuggestion,
};

/// A ticket to be scored for assignment.
pub struct UnassignedTicket {
    pub id: i64,
    pub titre: String,
    pub categorie_niveau1: Option<String>,
    pub categorie_niveau2: Option<String>,
}

/// Stock count per technician (name → vivant ticket count).
pub type TechnicianStockMap = HashMap<String, usize>;

const POIDS_CATEGORIE: f64 = 0.4;
const POIDS_TFIDF: f64 = 0.6;

/// Cosine similarity between two L2-normalized sparse vectors.
/// Inputs are assumed L2-normalized; divides by norms for correctness
/// even with floating-point imprecision in near-unit vectors.
/// Uses merge-join on sorted indices.
pub fn cosine_similarity_sparse(a: &[(usize, f64)], b: &[(usize, f64)]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut i = 0;
    let mut j = 0;
    while i < a.len() && j < b.len() {
        match a[i].0.cmp(&b[j].0) {
            std::cmp::Ordering::Equal => {
                dot += a[i].1 * b[j].1;
                i += 1;
                j += 1;
            }
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
        }
    }
    let norm_a: f64 = a.iter().map(|(_, v)| v * v).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|(_, v)| v * v).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Compute the L2-normalized centroid of selected rows from a sparse matrix.
/// Returns a sparse vector as sorted Vec<(index, weight)>.
pub fn compute_centroid(matrix: &CsMat<f64>, row_indices: &[usize]) -> Vec<(usize, f64)> {
    if row_indices.is_empty() {
        return vec![];
    }

    // Accumulate sum across selected rows into a dense-ish map
    let mut acc: std::collections::HashMap<usize, f64> = std::collections::HashMap::new();
    for &row_idx in row_indices {
        if let Some(row) = matrix.outer_view(row_idx) {
            for (&col, &val) in row.indices().iter().zip(row.data().iter()) {
                *acc.entry(col).or_insert(0.0) += val;
            }
        }
    }

    // L2-normalize
    let norm: f64 = acc.values().map(|v| v * v).sum::<f64>().sqrt();
    if norm == 0.0 {
        return vec![];
    }

    let mut result: Vec<(usize, f64)> = acc.into_iter().map(|(k, v)| (k, v / norm)).collect();
    result.sort_unstable_by_key(|(k, _)| *k);
    result
}

/// Project preprocessed stems into an existing vocabulary.
/// For each stem found in vocab, compute sublinear TF (1 + ln(count)),
/// multiply by IDF, L2-normalize, return sorted sparse vector.
pub fn project_to_vocabulary(
    stems: &[String],
    vocab: &std::collections::HashMap<String, usize>,
    idf_values: &[f64],
) -> Vec<(usize, f64)> {
    // Count term frequencies
    let mut tf: std::collections::HashMap<usize, f64> = std::collections::HashMap::new();
    for stem in stems {
        if let Some(&idx) = vocab.get(stem) {
            *tf.entry(idx).or_insert(0.0) += 1.0;
        }
    }

    if tf.is_empty() {
        return vec![];
    }

    // Apply sublinear TF scaling and multiply by IDF
    let mut weighted: Vec<(usize, f64)> = tf
        .into_iter()
        .map(|(idx, count)| {
            let sublinear_tf = 1.0 + count.ln();
            let score = sublinear_tf * idf_values[idx];
            (idx, score)
        })
        .collect();

    // L2-normalize
    let norm: f64 = weighted.iter().map(|(_, v)| v * v).sum::<f64>().sqrt();
    if norm == 0.0 {
        return vec![];
    }
    for (_, v) in &mut weighted {
        *v /= norm;
    }

    weighted.sort_unstable_by_key(|(k, _)| *k);
    weighted
}

/// Score unassigned tickets against technician profiles.
pub fn score_tickets(
    tickets: &[UnassignedTicket],
    profiling_data: &CachedProfilingData,
    stock_map: &TechnicianStockMap,
    seuil_tickets: f64,
    limit: usize,
    score_minimum: f64,
) -> Vec<AssignmentRecommendation> {
    use crate::nlp::preprocessing::{StopWordFilter, preprocess_text};

    if tickets.is_empty() || profiling_data.profiles.is_empty() {
        return tickets
            .iter()
            .map(|t| AssignmentRecommendation {
                ticket_id: t.id,
                ticket_titre: t.titre.clone(),
                ticket_categorie: t.categorie_niveau2.clone().or(t.categorie_niveau1.clone()),
                suggestions: Vec::new(),
            })
            .collect();
    }

    let filter = StopWordFilter::new();

    tickets
        .iter()
        .map(|ticket| {
            let stems = preprocess_text(&ticket.titre, &filter);
            let vec_ticket = project_to_vocabulary(
                &stems,
                &profiling_data.vocabulary,
                &profiling_data.idf_values,
            );

            let cat_ticket = ticket
                .categorie_niveau2
                .as_deref()
                .or(ticket.categorie_niveau1.as_deref());

            let has_category = cat_ticket.is_some()
                && cat_ticket != Some("SANS_CATEGORIE");

            let mut suggestions: Vec<TechnicianSuggestion> = profiling_data
                .profiles
                .iter()
                .map(|profile| {
                    let score_tfidf =
                        cosine_similarity_sparse(&vec_ticket, &profile.centroide_tfidf);

                    let (score_cat, w_cat, w_tfidf) = if has_category {
                        let cat = cat_ticket.unwrap();
                        let sc = profile
                            .cat_distribution
                            .get(cat)
                            .copied()
                            .unwrap_or(0.0);
                        (sc, POIDS_CATEGORIE, POIDS_TFIDF)
                    } else {
                        (0.0, 0.0, 1.0)
                    };

                    let score_competence = w_cat * score_cat + w_tfidf * score_tfidf;

                    let stock = stock_map
                        .get(&profile.technicien)
                        .copied()
                        .unwrap_or(0);
                    let facteur_charge =
                        1.0 / (1.0 + stock as f64 / seuil_tickets);

                    let score_final = score_competence * facteur_charge;

                    TechnicianSuggestion {
                        technicien: profile.technicien.clone(),
                        score_final,
                        score_competence,
                        score_categorie: score_cat,
                        score_tfidf,
                        stock_actuel: stock,
                        facteur_charge,
                    }
                })
                .filter(|s| s.score_final >= score_minimum)
                .collect();

            suggestions.sort_by(|a, b| {
                b.score_final
                    .partial_cmp(&a.score_final)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            suggestions.truncate(limit);

            AssignmentRecommendation {
                ticket_id: ticket.id,
                ticket_titre: ticket.titre.clone(),
                ticket_categorie: cat_ticket.map(|s| s.to_string()),
                suggestions,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_identical_vectors() {
        let v = vec![(0, 0.5), (2, 0.5), (5, 0.7071)];
        let sim = cosine_similarity_sparse(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6, "got {sim}");
    }

    #[test]
    fn test_cosine_orthogonal_vectors() {
        let a = vec![(0, 1.0)];
        let b = vec![(1, 1.0)];
        let sim = cosine_similarity_sparse(&a, &b);
        assert!((sim - 0.0).abs() < 1e-6, "got {sim}");
    }

    #[test]
    fn test_cosine_partial_overlap() {
        let a = vec![(0, 0.6), (1, 0.8)];
        let b = vec![(1, 0.6), (2, 0.8)];
        let sim = cosine_similarity_sparse(&a, &b);
        assert!((sim - 0.48).abs() < 1e-6, "got {sim}");
    }

    #[test]
    fn test_cosine_empty_vectors() {
        let sim = cosine_similarity_sparse(&[], &[]);
        assert!((sim - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_centroid_single_row() {
        let mat = CsMat::new(
            (1, 3),
            vec![0, 2],
            vec![1, 2],
            vec![0.6, 0.8],
        );
        let centroid = compute_centroid(&mat, &[0]);
        assert_eq!(centroid.len(), 2);
        assert!((centroid[0].1 - 0.6).abs() < 1e-6);
        assert!((centroid[1].1 - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_centroid_two_rows_averaged() {
        let mat = CsMat::new(
            (2, 3),
            vec![0, 1, 2],
            vec![0, 1],
            vec![1.0, 1.0],
        );
        let centroid = compute_centroid(&mat, &[0, 1]);
        assert_eq!(centroid.len(), 2);
        let expected = 1.0 / (2.0_f64).sqrt();
        assert!((centroid[0].1 - expected).abs() < 1e-4, "got {}", centroid[0].1);
        assert!((centroid[1].1 - expected).abs() < 1e-4);
    }

    #[test]
    fn test_centroid_empty_indices() {
        let mat = CsMat::new((1, 3), vec![0, 1], vec![0], vec![1.0]);
        let centroid = compute_centroid(&mat, &[]);
        assert!(centroid.is_empty());
    }

    #[test]
    fn test_project_to_vocabulary() {
        use std::collections::HashMap;
        let mut vocab = HashMap::new();
        vocab.insert("imprim".to_string(), 0);
        vocab.insert("reseau".to_string(), 1);
        vocab.insert("problem".to_string(), 2);
        let idf = vec![1.5, 2.0, 1.0];

        let stems = vec![
            "imprim".to_string(),
            "imprim".to_string(),
            "reseau".to_string(),
            "inconnu".to_string(),
        ];

        let result = project_to_vocabulary(&stems, &vocab, &idf);
        assert_eq!(result.len(), 2);

        let imprim = result.iter().find(|(i, _)| *i == 0).unwrap().1;
        let reseau = result.iter().find(|(i, _)| *i == 1).unwrap().1;
        assert!((imprim - 0.7856).abs() < 0.01, "got {imprim}");
        assert!((reseau - 0.6187).abs() < 0.01, "got {reseau}");
    }

    use crate::recommandation::types::TechnicianProfile;

    fn make_profiling_data() -> CachedProfilingData {
        let mut vocab = HashMap::new();
        vocab.insert("imprim".to_string(), 0);
        vocab.insert("reseau".to_string(), 1);
        vocab.insert("sap".to_string(), 2);
        vocab.insert("acces".to_string(), 3);

        let idf = vec![1.5, 1.2, 2.0, 1.8];

        let mut cat_dupont = HashMap::new();
        cat_dupont.insert("Imprimante".to_string(), 0.8);
        cat_dupont.insert("Réseau".to_string(), 0.2);

        let mut cat_leroy = HashMap::new();
        cat_leroy.insert("Habilitations".to_string(), 0.9);
        cat_leroy.insert("Imprimante".to_string(), 0.1);

        CachedProfilingData {
            profiles: vec![
                TechnicianProfile {
                    technicien: "Dupont".to_string(),
                    nb_tickets_reference: 50,
                    cat_distribution: cat_dupont,
                    centroide_tfidf: vec![(0, 0.8), (1, 0.6)],
                },
                TechnicianProfile {
                    technicien: "Leroy".to_string(),
                    nb_tickets_reference: 30,
                    cat_distribution: cat_leroy,
                    centroide_tfidf: vec![(2, 0.7), (3, 0.7)],
                },
            ],
            vocabulary: vocab,
            idf_values: idf,
            vocabulary_size: 4,
            nb_tickets_analysed: 80,
            periode_from: "2025-09-12".to_string(),
            periode_to: "2026-03-12".to_string(),
        }
    }

    #[test]
    fn test_score_tickets_imprimante_favors_dupont() {
        let data = make_profiling_data();
        let tickets = vec![UnassignedTicket {
            id: 1,
            titre: "imprimante réseau bloquée".to_string(),
            categorie_niveau1: Some("Matériel".to_string()),
            categorie_niveau2: Some("Imprimante".to_string()),
        }];
        let mut stock = HashMap::new();
        stock.insert("Dupont".to_string(), 10_usize);
        stock.insert("Leroy".to_string(), 10_usize);

        let results = score_tickets(&tickets, &data, &stock, 20.0, 3, 0.0);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].suggestions[0].technicien, "Dupont");
        assert!(results[0].suggestions[0].score_final > results[0].suggestions[1].score_final);
    }

    #[test]
    fn test_score_tickets_sap_favors_leroy() {
        let data = make_profiling_data();
        let tickets = vec![UnassignedTicket {
            id: 2,
            titre: "accès SAP refusé nouvel agent".to_string(),
            categorie_niveau1: Some("Habilitations".to_string()),
            categorie_niveau2: None,
        }];
        let mut stock = HashMap::new();
        stock.insert("Dupont".to_string(), 10_usize);
        stock.insert("Leroy".to_string(), 10_usize);

        let results = score_tickets(&tickets, &data, &stock, 20.0, 3, 0.0);
        assert_eq!(results[0].suggestions[0].technicien, "Leroy");
    }

    #[test]
    fn test_score_tickets_high_stock_penalizes() {
        let data = make_profiling_data();
        let tickets = vec![UnassignedTicket {
            id: 3,
            titre: "imprimante réseau bloquée".to_string(),
            categorie_niveau1: Some("Matériel".to_string()),
            categorie_niveau2: Some("Imprimante".to_string()),
        }];
        let mut stock = HashMap::new();
        stock.insert("Dupont".to_string(), 60_usize);
        stock.insert("Leroy".to_string(), 2_usize);

        let results = score_tickets(&tickets, &data, &stock, 20.0, 3, 0.0);
        let dupont = results[0].suggestions.iter().find(|s| s.technicien == "Dupont").unwrap();
        let leroy = results[0].suggestions.iter().find(|s| s.technicien == "Leroy").unwrap();
        assert!(dupont.facteur_charge < leroy.facteur_charge);
    }

    #[test]
    fn test_score_tickets_no_category_full_tfidf() {
        let data = make_profiling_data();
        let tickets = vec![UnassignedTicket {
            id: 4,
            titre: "imprimante réseau bloquée".to_string(),
            categorie_niveau1: None,
            categorie_niveau2: None,
        }];
        let mut stock = HashMap::new();
        stock.insert("Dupont".to_string(), 10_usize);
        stock.insert("Leroy".to_string(), 10_usize);

        let results = score_tickets(&tickets, &data, &stock, 20.0, 3, 0.0);
        let dupont = results[0].suggestions.iter().find(|s| s.technicien == "Dupont").unwrap();
        assert!((dupont.score_categorie - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_score_tickets_respects_limit() {
        let data = make_profiling_data();
        let tickets = vec![UnassignedTicket {
            id: 5,
            titre: "test".to_string(),
            categorie_niveau1: None,
            categorie_niveau2: None,
        }];
        let stock = HashMap::new();
        let results = score_tickets(&tickets, &data, &stock, 20.0, 1, 0.0);
        assert!(results[0].suggestions.len() <= 1);
    }

    #[test]
    fn test_score_tickets_filters_below_minimum() {
        let data = make_profiling_data();
        let tickets = vec![UnassignedTicket {
            id: 6,
            titre: "quelque chose de totalement inconnu zzzzz".to_string(),
            categorie_niveau1: Some("CatégorieInexistante".to_string()),
            categorie_niveau2: None,
        }];
        let stock = HashMap::new();
        let results = score_tickets(&tickets, &data, &stock, 20.0, 3, 0.99);
        assert!(results[0].suggestions.is_empty());
    }

    #[test]
    fn test_score_tickets_empty_tickets() {
        let data = make_profiling_data();
        let stock = HashMap::new();
        let results = score_tickets(&[], &data, &stock, 20.0, 3, 0.05);
        assert!(results.is_empty());
    }
}
