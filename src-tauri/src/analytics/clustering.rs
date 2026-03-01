// K-Means sur TF-IDF, silhouette score, méthode Elbow

use kneed::knee_locator::{InterpMethod, KneeLocator, KneeLocatorParams, ValidCurve, ValidDirection};
use linfa::prelude::*;
use linfa_clustering::KMeans;
use ndarray::{Array1, Array2};

// ─────────────────────────────────────────────
// Structs publiques
// ─────────────────────────────────────────────

/// Résultat du clustering K-Means.
#[derive(Debug, Clone)]
pub struct ClusteringResult {
    pub k_optimal: usize,
    pub clusters: Vec<ClusterInfo>,
    pub silhouette_score: f64,
    /// (k, inertia) pour chaque K testé dans k_min..=k_max
    pub inertias: Vec<(usize, f64)>,
}

/// Informations sur un cluster.
#[derive(Debug, Clone)]
pub struct ClusterInfo {
    pub id: usize,
    /// Top 5 mots-clés concaténés par espace
    pub label: String,
    /// Top 5 mots-clés du centroïde
    pub top_keywords: Vec<String>,
    /// Indices des documents dans ce cluster
    pub doc_indices: Vec<usize>,
    pub size: usize,
}

// ─────────────────────────────────────────────
// Conversion sparse → dense
// ─────────────────────────────────────────────

/// Convertit une matrice CsMat (sparse CSR) en Array2 (dense ndarray).
pub fn sparse_to_dense(matrix: &sprs::CsMat<f64>) -> Array2<f64> {
    let rows = matrix.rows();
    let cols = matrix.cols();
    let mut dense = Array2::zeros((rows, cols));
    for (val, (row, col)) in matrix.iter() {
        dense[[row, col]] = *val;
    }
    dense
}

// ─────────────────────────────────────────────
// Helpers privés
// ─────────────────────────────────────────────

fn euclidean_dist(a: ndarray::ArrayView1<f64>, b: ndarray::ArrayView1<f64>) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// Calcule la somme des distances au carré entre chaque point et son centroïde assigné.
#[allow(dead_code)]
fn compute_inertia(
    data: &Array2<f64>,
    centroids: &Array2<f64>,
    labels: &[usize],
) -> f64 {
    data.rows()
        .into_iter()
        .zip(labels.iter())
        .map(|(row, &label)| {
            let centroid = centroids.row(label);
            let dist = euclidean_dist(row, centroid);
            dist * dist
        })
        .sum()
}

/// Calcule le score silhouette moyen sur toutes les observations.
///
/// Pour chaque point i :
/// - a(i) = distance moyenne aux autres points du même cluster
/// - b(i) = distance moyenne minimale aux points d'un autre cluster
/// - s(i) = (b - a) / max(a, b)  — 0 si cluster singleton
fn silhouette_score(data: &Array2<f64>, labels: &[usize]) -> f64 {
    let n = data.nrows();
    if n <= 1 {
        return 0.0;
    }

    let k = match labels.iter().max() {
        Some(&m) => m + 1,
        None => return 0.0,
    };

    if k <= 1 {
        // Un seul cluster : score silhouette indéfini → 0
        return 0.0;
    }

    let mut total = 0.0;

    for i in 0..n {
        let cluster_i = labels[i];

        // a(i) : distance moyenne aux autres points du même cluster
        let same_indices: Vec<usize> = (0..n)
            .filter(|&j| j != i && labels[j] == cluster_i)
            .collect();

        let a = if same_indices.is_empty() {
            0.0
        } else {
            let sum: f64 = same_indices
                .iter()
                .map(|&j| euclidean_dist(data.row(i), data.row(j)))
                .sum();
            sum / same_indices.len() as f64
        };

        // b(i) : distance moyenne minimale aux points d'un autre cluster
        let mut min_b = f64::INFINITY;
        for c in 0..k {
            if c == cluster_i {
                continue;
            }
            let other_indices: Vec<usize> =
                (0..n).filter(|&j| labels[j] == c).collect();
            if other_indices.is_empty() {
                continue;
            }
            let sum: f64 = other_indices
                .iter()
                .map(|&j| euclidean_dist(data.row(i), data.row(j)))
                .sum();
            let mean_dist = sum / other_indices.len() as f64;
            if mean_dist < min_b {
                min_b = mean_dist;
            }
        }

        let s = if min_b == f64::INFINITY || (a == 0.0 && min_b == 0.0) {
            0.0
        } else {
            (min_b - a) / a.max(min_b)
        };

        total += s;
    }

    total / n as f64
}

// ─────────────────────────────────────────────
// Fonction principale
// ─────────────────────────────────────────────

/// Exécute le clustering K-Means sur une matrice TF-IDF creuse.
///
/// # Arguments
/// * `matrix`       – Matrice docs × vocab au format CSR
/// * `vocabulary`   – Vocabulaire (index → mot)
/// * `k_min`        – Nombre minimal de clusters à tester (défaut : 2)
/// * `k_max`        – Nombre maximal de clusters à tester (défaut : 10)
/// * `n_iterations` – Nombre maximal d'itérations K-Means (défaut : 100)
pub fn run_kmeans(
    matrix: &sprs::CsMat<f64>,
    vocabulary: &[String],
    k_min: usize,
    k_max: usize,
    n_iterations: usize,
) -> Result<ClusteringResult, String> {
    // 1. Conversion sparse → dense
    let dense = sparse_to_dense(matrix);
    let n_docs = dense.nrows();

    // 2. Validation
    if n_docs == 0 || dense.ncols() == 0 {
        return Err("La matrice TF-IDF est vide".to_string());
    }
    if n_docs < k_min {
        return Err(format!(
            "Trop peu de documents ({}) pour k_min={} clusters",
            n_docs, k_min
        ));
    }

    // 3. Ajuster k_max au nombre de documents
    let k_max = k_max.min(n_docs);
    let k_min = k_min.min(k_max);

    // Créer le dataset linfa une seule fois
    let dataset = DatasetBase::from(dense.clone());

    // 4. Méthode du coude : itérer sur k_min..=k_max
    let mut inertias: Vec<(usize, f64)> = Vec::new();

    for k in k_min..=k_max {
        let model = KMeans::params(k)
            .max_n_iterations(n_iterations as u64)
            .tolerance(1e-4f64)
            .fit(&dataset)
            .map_err(|e| format!("K-Means (k={k}) erreur: {e}"))?;

        let inertia = model.inertia();
        inertias.push((k, inertia));
    }

    // 5. Trouver K optimal via kneed
    let k_optimal = find_optimal_k(&inertias, k_min, k_max);

    // 6. Re-exécuter K-Means avec K optimal
    let final_model = KMeans::params(k_optimal)
        .max_n_iterations(n_iterations as u64)
        .tolerance(1e-4f64)
        .fit(&dataset)
        .map_err(|e| format!("K-Means final (k={k_optimal}) erreur: {e}"))?;

    let centroids = final_model.centroids().clone();
    let labels_array: Array1<usize> = final_model.predict(&dense);
    let labels: Vec<usize> = labels_array.iter().copied().collect();

    // 7. Labelliser les clusters
    let clusters = build_cluster_infos(k_optimal, &labels, &centroids, vocabulary, n_docs);

    // 8. Calculer le score silhouette
    let silhouette = silhouette_score(&dense, &labels);

    Ok(ClusteringResult {
        k_optimal,
        clusters,
        silhouette_score: silhouette,
        inertias,
    })
}

/// Trouve le K optimal via la méthode du coude (kneed).
/// En cas d'échec, utilise k_min + (k_max - k_min) / 2 comme fallback.
fn find_optimal_k(inertias: &[(usize, f64)], k_min: usize, k_max: usize) -> usize {
    if inertias.len() <= 1 {
        return inertias.first().map(|(k, _)| *k).unwrap_or(k_min);
    }

    let x: Vec<f64> = inertias.iter().map(|(k, _)| *k as f64).collect();
    let y: Vec<f64> = inertias.iter().map(|(_, inertia)| *inertia).collect();

    let params = KneeLocatorParams::new(
        ValidCurve::Convex,
        ValidDirection::Decreasing,
        InterpMethod::Interp1d,
    );

    let knee_opt = KneeLocator::new(x, y, 1.0, params)
        .ok()
        .and_then(|locator| locator.knee);

    if let Some(knee_x) = knee_opt {
        let k = knee_x.round() as usize;
        if k >= k_min && k <= k_max {
            return k;
        }
    }

    // Fallback : heuristique simple
    k_min + (k_max - k_min) / 2
}

/// Construit les infos de chaque cluster à partir des assignations et des centroïdes.
fn build_cluster_infos(
    k: usize,
    labels: &[usize],
    centroids: &Array2<f64>,
    vocabulary: &[String],
    n_docs: usize,
) -> Vec<ClusterInfo> {
    let mut clusters: Vec<ClusterInfo> = (0..k)
        .map(|id| ClusterInfo {
            id,
            label: String::new(),
            top_keywords: Vec::new(),
            doc_indices: Vec::new(),
            size: 0,
        })
        .collect();

    // Assigner les documents aux clusters
    for doc_idx in 0..n_docs {
        if doc_idx < labels.len() {
            let cluster_id = labels[doc_idx];
            if cluster_id < k {
                clusters[cluster_id].doc_indices.push(doc_idx);
            }
        }
    }

    // Calculer taille et top keywords depuis les centroïdes
    let vocab_size = vocabulary.len();
    for cluster in clusters.iter_mut() {
        cluster.size = cluster.doc_indices.len();

        if vocab_size == 0 || cluster.id >= centroids.nrows() {
            continue;
        }

        let centroid_row = centroids.row(cluster.id);

        // Trier les termes par poids décroissant dans le centroïde
        let mut term_weights: Vec<(usize, f64)> = centroid_row
            .iter()
            .enumerate()
            .filter(|(_, &w)| w > 0.0)
            .map(|(idx, &w)| (idx, w))
            .collect();

        term_weights.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });

        let top5: Vec<String> = term_weights
            .iter()
            .take(5)
            .filter_map(|(idx, _)| vocabulary.get(*idx).cloned())
            .collect();

        cluster.label = top5.join(" ");
        cluster.top_keywords = top5;
    }

    clusters
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Crée une petite matrice CSR sparse pour les tests.
    fn make_sparse(data: Vec<Vec<f64>>) -> sprs::CsMat<f64> {
        let rows = data.len();
        let cols = data.first().map(|r| r.len()).unwrap_or(0);
        let mut tri: sprs::TriMat<f64> = sprs::TriMat::new((rows, cols));
        for (r, row) in data.iter().enumerate() {
            for (c, &val) in row.iter().enumerate() {
                if val != 0.0 {
                    tri.add_triplet(r, c, val);
                }
            }
        }
        tri.to_csr()
    }

    #[test]
    fn test_sparse_to_dense() {
        // Matrice 3×3 avec quelques valeurs non-nulles
        let sparse = make_sparse(vec![
            vec![1.0, 0.0, 0.5],
            vec![0.0, 2.0, 0.0],
            vec![0.3, 0.0, 0.7],
        ]);
        let dense = sparse_to_dense(&sparse);

        assert_eq!(dense.shape(), &[3, 3]);
        assert!((dense[[0, 0]] - 1.0).abs() < 1e-10);
        assert!((dense[[0, 1]] - 0.0).abs() < 1e-10);
        assert!((dense[[0, 2]] - 0.5).abs() < 1e-10);
        assert!((dense[[1, 1]] - 2.0).abs() < 1e-10);
        assert!((dense[[2, 0]] - 0.3).abs() < 1e-10);
        assert!((dense[[2, 2]] - 0.7).abs() < 1e-10);
    }

    #[test]
    fn test_run_kmeans_basic() {
        // 6 documents clairement séparables en 2 groupes distincts dans l'espace à 4 features
        // Groupe A : features 0-1 élevées, features 2-3 nulles
        // Groupe B : features 0-1 nulles, features 2-3 élevées
        let data = make_sparse(vec![
            vec![1.0, 0.9, 0.0, 0.0],
            vec![0.9, 1.0, 0.0, 0.0],
            vec![1.0, 0.8, 0.05, 0.0],
            vec![0.0, 0.0, 1.0, 0.9],
            vec![0.0, 0.0, 0.9, 1.0],
            vec![0.0, 0.05, 0.8, 1.0],
        ]);
        let vocab: Vec<String> = vec!["a".into(), "b".into(), "c".into(), "d".into()];

        let result = run_kmeans(&data, &vocab, 2, 5, 100);
        assert!(result.is_ok(), "run_kmeans a échoué: {:?}", result.err());

        let result = result.unwrap();
        // K optimal doit être ≤ 3 pour des groupes clairement séparés
        assert!(
            result.k_optimal <= 3,
            "k_optimal = {} devrait être ≤ 3",
            result.k_optimal
        );
        assert_eq!(result.clusters.len(), result.k_optimal);
        assert!(!result.inertias.is_empty());
    }

    #[test]
    fn test_silhouette_perfect() {
        // 2 clusters parfaitement séparés → silhouette > 0.5
        let data_vec: Vec<Vec<f64>> = vec![
            vec![10.0, 0.0],
            vec![10.0, 0.1],
            vec![9.9, 0.0],
            vec![0.0, 10.0],
            vec![0.1, 10.0],
            vec![0.0, 9.9],
        ];
        let dense: Array2<f64> = Array2::from_shape_vec(
            (6, 2),
            data_vec.into_iter().flatten().collect(),
        )
        .unwrap();
        let labels = vec![0, 0, 0, 1, 1, 1];

        let score = silhouette_score(&dense, &labels);
        assert!(
            score > 0.5,
            "Silhouette sur clusters parfaits doit être > 0.5, obtenu: {}",
            score
        );
    }

    #[test]
    fn test_cluster_labels() {
        let data = make_sparse(vec![
            vec![1.0, 0.0, 0.0, 0.0],
            vec![0.9, 0.0, 0.0, 0.0],
            vec![0.0, 0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.9, 0.0],
        ]);
        let vocab: Vec<String> = vec!["alpha".into(), "beta".into(), "gamma".into(), "delta".into()];

        let result = run_kmeans(&data, &vocab, 2, 2, 100);
        assert!(result.is_ok(), "Erreur: {:?}", result.err());
        let result = result.unwrap();

        for cluster in &result.clusters {
            assert!(
                !cluster.top_keywords.is_empty(),
                "Cluster {} doit avoir des top_keywords non vides",
                cluster.id
            );
            assert!(
                !cluster.label.is_empty(),
                "Cluster {} doit avoir un label non vide",
                cluster.id
            );
        }
    }

    #[test]
    fn test_too_few_docs() {
        // 1 document < k_min=2 → erreur propre
        let data = make_sparse(vec![vec![1.0, 0.5]]);
        let vocab: Vec<String> = vec!["a".into(), "b".into()];

        let result = run_kmeans(&data, &vocab, 2, 5, 100);
        assert!(result.is_err(), "Doit retourner une erreur pour k_min > n_docs");
    }

    #[test]
    fn test_single_cluster() {
        // k_min=1, k_max=1 → un seul cluster
        let data = make_sparse(vec![
            vec![1.0, 0.5],
            vec![0.8, 0.6],
            vec![0.9, 0.4],
        ]);
        let vocab: Vec<String> = vec!["a".into(), "b".into()];

        let result = run_kmeans(&data, &vocab, 1, 1, 100);
        assert!(result.is_ok(), "Erreur: {:?}", result.err());
        let result = result.unwrap();

        assert_eq!(result.k_optimal, 1);
        assert_eq!(result.clusters.len(), 1);
        assert_eq!(result.clusters[0].size, 3);
        // Silhouette avec 1 cluster = 0
        assert_eq!(result.silhouette_score, 0.0);
    }
}
