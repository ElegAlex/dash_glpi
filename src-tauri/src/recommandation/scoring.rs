use sprs::CsMat;

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
}
