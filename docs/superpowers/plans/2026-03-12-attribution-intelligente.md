# Attribution Intelligente — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an intelligent ticket assignment recommendation engine that suggests the top 3 best-fit technicians for each unassigned ticket, based on competence profiling (category + TF-IDF) and workload balancing.

**Architecture:** Two-phase approach — (1) build technician competence profiles from 6 months of resolved tickets using hybrid category distribution + TF-IDF centroid, cached in `analytics_cache`; (2) score unassigned tickets against profiles with a workload penalty factor. Two separate Tauri IPC commands. New `recommandation/` Rust module + `AttributionSection` React component in StockPage.

**Tech Stack:** Rust (sprs sparse matrices, serde_json for cache, existing NLP pipeline), React 19, TypeScript, Tailwind CSS 4, Tauri 2.10 IPC.

**Spec:** `docs/superpowers/specs/2026-03-12-attribution-intelligente-design.md`

---

## Chunk 1: Rust Types + Sparse Vector Utilities

### Task 1: Types module

**Files:**
- Create: `src-tauri/src/recommandation/types.rs`
- Create: `src-tauri/src/recommandation/mod.rs`
- Modify: `src-tauri/src/lib.rs:1-10` (add `mod recommandation`)

- [ ] **Step 1: Create the types file**

```rust
// src-tauri/src/recommandation/types.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TechnicianProfile {
    pub technicien: String,
    pub nb_tickets_reference: usize,
    pub cat_distribution: HashMap<String, f64>,
    pub centroide_tfidf: Vec<(usize, f64)>,
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
        self.score_minimum.unwrap_or(0.05)
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
```

- [ ] **Step 2: Create the module root**

```rust
// src-tauri/src/recommandation/mod.rs
pub mod types;
pub mod profiling;
pub mod scoring;
```

- [ ] **Step 3: Add module declaration to lib.rs**

In `src-tauri/src/lib.rs`, add `mod recommandation;` after the existing module declarations (line 10).

- [ ] **Step 4: Create stub files so it compiles**

Create empty stub files:
- `src-tauri/src/recommandation/profiling.rs` → `// TODO`
- `src-tauri/src/recommandation/scoring.rs` → `// TODO`

- [ ] **Step 5: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: Compiles with no errors.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/recommandation/ src-tauri/src/lib.rs
git commit -m "feat(recommandation): add types module and module structure"
```

---

### Task 2: Sparse vector utilities

**Files:**
- Modify: `src-tauri/src/recommandation/scoring.rs`
- Test: inline `#[cfg(test)]` in same file

- [ ] **Step 1: Write failing tests for cosine_similarity_sparse**

```rust
// src-tauri/src/recommandation/scoring.rs

/// Cosine similarity between two L2-normalized sparse vectors.
/// Since both are L2-normalized, cosine = dot product.
pub fn cosine_similarity_sparse(a: &[(usize, f64)], b: &[(usize, f64)]) -> f64 {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_identical_vectors() {
        let v = vec![(0, 0.5), (2, 0.5), (5, 0.7071)];
        let sim = cosine_similarity_sparse(&v, &v);
        // dot product of L2-normalized vector with itself ≈ 1.0
        // (0.5*0.5 + 0.5*0.5 + 0.7071*0.7071) = 0.25+0.25+0.5 = 1.0
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
        // overlap only at index 1: 0.8 * 0.6 = 0.48
        let sim = cosine_similarity_sparse(&a, &b);
        assert!((sim - 0.48).abs() < 1e-6, "got {sim}");
    }

    #[test]
    fn test_cosine_empty_vectors() {
        let sim = cosine_similarity_sparse(&[], &[]);
        assert!((sim - 0.0).abs() < 1e-6);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib recommandation::scoring::tests -- --nocapture`
Expected: FAIL with "not yet implemented"

- [ ] **Step 3: Implement cosine_similarity_sparse**

Replace the `todo!()` body:

```rust
pub fn cosine_similarity_sparse(a: &[(usize, f64)], b: &[(usize, f64)]) -> f64 {
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
    dot
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --lib recommandation::scoring::tests -- --nocapture`
Expected: 4 tests PASS

- [ ] **Step 5: Write failing tests for compute_centroid**

Add to scoring.rs, above the tests module:

```rust
use sprs::CsMat;

/// Compute the L2-normalized centroid of selected rows from a sparse matrix.
/// Returns a sparse vector as sorted Vec<(index, weight)>.
pub fn compute_centroid(matrix: &CsMat<f64>, row_indices: &[usize]) -> Vec<(usize, f64)> {
    todo!()
}
```

Add tests:

```rust
    #[test]
    fn test_centroid_single_row() {
        // 1x3 matrix: [0.0, 0.6, 0.8]
        let mat = CsMat::new(
            (1, 3),
            vec![0, 2],       // indptr
            vec![1, 2],       // indices
            vec![0.6, 0.8],   // data
        );
        let centroid = compute_centroid(&mat, &[0]);
        // L2 norm = sqrt(0.36 + 0.64) = 1.0 → already normalized
        assert_eq!(centroid.len(), 2);
        assert!((centroid[0].1 - 0.6).abs() < 1e-6);
        assert!((centroid[1].1 - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_centroid_two_rows_averaged() {
        // 2x3 matrix:
        // row 0: [1.0, 0.0, 0.0]
        // row 1: [0.0, 1.0, 0.0]
        let mat = CsMat::new(
            (2, 3),
            vec![0, 1, 2],
            vec![0, 1],
            vec![1.0, 1.0],
        );
        let centroid = compute_centroid(&mat, &[0, 1]);
        // mean = [0.5, 0.5, 0.0], L2 = sqrt(0.5) ≈ 0.7071
        // normalized = [0.7071, 0.7071]
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
```

- [ ] **Step 6: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib recommandation::scoring::tests -- --nocapture`
Expected: FAIL with "not yet implemented"

- [ ] **Step 7: Implement compute_centroid**

Replace the `todo!()` body:

```rust
pub fn compute_centroid(matrix: &CsMat<f64>, row_indices: &[usize]) -> Vec<(usize, f64)> {
    if row_indices.is_empty() {
        return Vec::new();
    }

    let n = row_indices.len() as f64;
    let cols = matrix.cols();
    let mut sum = vec![0.0; cols];

    for &row_idx in row_indices {
        if let Some(row) = matrix.outer_view(row_idx) {
            for (col, &val) in row.iter() {
                sum[col] += val;
            }
        }
    }

    // Average
    for v in sum.iter_mut() {
        *v /= n;
    }

    // L2 normalize
    let norm: f64 = sum.iter().map(|v| v * v).sum::<f64>().sqrt();
    if norm < 1e-10 {
        return Vec::new();
    }

    // Collect non-zero entries as sparse vector
    sum.iter()
        .enumerate()
        .filter(|(_, &v)| v.abs() > 1e-10)
        .map(|(i, &v)| (i, v / norm))
        .collect()
}
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --lib recommandation::scoring::tests -- --nocapture`
Expected: 7 tests PASS

- [ ] **Step 9: Write failing test for project_to_vocabulary**

Add to scoring.rs:

```rust
/// Project a preprocessed document (stems) into an existing vocabulary space.
/// Returns an L2-normalized sparse TF-IDF vector.
pub fn project_to_vocabulary(
    stems: &[String],
    vocab: &std::collections::HashMap<String, usize>,
    idf_values: &[f64],
) -> Vec<(usize, f64)> {
    todo!()
}
```

Add test:

```rust
    #[test]
    fn test_project_to_vocabulary() {
        use std::collections::HashMap;
        let mut vocab = HashMap::new();
        vocab.insert("imprim".to_string(), 0);
        vocab.insert("reseau".to_string(), 1);
        vocab.insert("problem".to_string(), 2);
        let idf = vec![1.5, 2.0, 1.0]; // idf per term

        let stems = vec![
            "imprim".to_string(),
            "imprim".to_string(), // count=2
            "reseau".to_string(), // count=1
            "inconnu".to_string(), // not in vocab → ignored
        ];

        let result = project_to_vocabulary(&stems, &vocab, &idf);
        // tf("imprim") = 1 + ln(2) ≈ 1.693, tfidf = 1.693 * 1.5 = 2.540
        // tf("reseau") = 1 + ln(1) = 1.0, tfidf = 1.0 * 2.0 = 2.0
        // L2 norm = sqrt(2.540^2 + 2.0^2) = sqrt(6.4516 + 4.0) = sqrt(10.4516) ≈ 3.233
        // normalized: imprim = 2.540/3.233 ≈ 0.7856, reseau = 2.0/3.233 ≈ 0.6187
        assert_eq!(result.len(), 2);

        let imprim = result.iter().find(|(i, _)| *i == 0).unwrap().1;
        let reseau = result.iter().find(|(i, _)| *i == 1).unwrap().1;
        assert!((imprim - 0.7856).abs() < 0.01, "got {imprim}");
        assert!((reseau - 0.6187).abs() < 0.01, "got {reseau}");
    }
```

- [ ] **Step 10: Run test to verify it fails**

Run: `cd src-tauri && cargo test --lib recommandation::scoring::tests::test_project_to_vocabulary -- --nocapture`
Expected: FAIL

- [ ] **Step 11: Implement project_to_vocabulary**

```rust
pub fn project_to_vocabulary(
    stems: &[String],
    vocab: &std::collections::HashMap<String, usize>,
    idf_values: &[f64],
) -> Vec<(usize, f64)> {
    use std::collections::HashMap;

    // Count term frequencies
    let mut tf_counts: HashMap<usize, usize> = HashMap::new();
    for stem in stems {
        if let Some(&idx) = vocab.get(stem) {
            *tf_counts.entry(idx).or_insert(0) += 1;
        }
    }

    if tf_counts.is_empty() {
        return Vec::new();
    }

    // Sublinear TF × IDF
    let mut entries: Vec<(usize, f64)> = tf_counts
        .into_iter()
        .map(|(idx, count)| {
            let tf = 1.0 + (count as f64).ln();
            let tfidf = tf * idf_values[idx];
            (idx, tfidf)
        })
        .collect();

    // L2 normalize
    let norm: f64 = entries.iter().map(|(_, v)| v * v).sum::<f64>().sqrt();
    if norm < 1e-10 {
        return Vec::new();
    }
    for entry in entries.iter_mut() {
        entry.1 /= norm;
    }

    // Sort by index for consistent sparse vector operations
    entries.sort_by_key(|(i, _)| *i);
    entries
}
```

- [ ] **Step 12: Run tests to verify all pass**

Run: `cd src-tauri && cargo test --lib recommandation::scoring::tests -- --nocapture`
Expected: 8 tests PASS

- [ ] **Step 13: Commit**

```bash
git add src-tauri/src/recommandation/scoring.rs
git commit -m "feat(recommandation): add sparse vector utilities (cosine, centroid, project)"
```

---

## Chunk 2: Profiling Module

### Task 3: Profiling — build technician competence profiles

**Files:**
- Modify: `src-tauri/src/recommandation/profiling.rs`
- Reads from: `src-tauri/src/nlp/preprocessing.rs` (StopWordFilter, preprocess_text)
- Reads from: `src-tauri/src/nlp/tfidf.rs` (build_tfidf_matrix, TfIdfResult)
- Reads from: `src-tauri/src/recommandation/scoring.rs` (compute_centroid)
- Test: inline `#[cfg(test)]`

- [ ] **Step 1: Write the profiling function signature and test**

```rust
// src-tauri/src/recommandation/profiling.rs

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

/// Configuration constants for profiling.
const TFIDF_MIN_DF: usize = 2;

/// Build technician competence profiles from resolved tickets.
///
/// Takes pre-fetched ticket data (already filtered to 6 months, resolved only).
/// Returns cached profiling data with vocabulary for later scoring.
pub fn build_profiles(
    tickets: Vec<ProfilingTicket>,
    periode_from: &str,
    periode_to: &str,
) -> CachedProfilingData {
    todo!()
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

        // Imprimante: 2/3, Habilitations: 1/3
        assert!((profile.cat_distribution["Imprimante"] - 2.0 / 3.0).abs() < 1e-6);
        assert!((profile.cat_distribution["Habilitations"] - 1.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_build_profiles_uses_niveau2_over_niveau1() {
        let tickets = vec![
            make_ticket("A", "test ticket", Some("Matériel"), Some("Imprimante")),
        ];
        let result = build_profiles(tickets, "2025-09-12", "2026-03-12");
        let profile = &result.profiles[0];
        assert!(profile.cat_distribution.contains_key("Imprimante"));
        assert!(!profile.cat_distribution.contains_key("Matériel"));
    }

    #[test]
    fn test_build_profiles_fallback_to_niveau1() {
        let tickets = vec![
            make_ticket("A", "test ticket", Some("Matériel"), None),
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
            make_ticket("Dupont", "imprimante bloquée", Some("Matériel"), None),
            make_ticket("Dupont", "imprimante HP cassée", Some("Matériel"), None),
            make_ticket("Leroy", "accès SAP refusé", Some("Habilitations"), None),
        ];
        let result = build_profiles(tickets, "2025-09-12", "2026-03-12");
        assert_eq!(result.profiles.len(), 2);
        assert_eq!(result.nb_tickets_analysed, 3);
    }

    #[test]
    fn test_build_profiles_centroid_is_l2_normalized() {
        let tickets = vec![
            make_ticket("A", "imprimante réseau bloquée bâtiment", None, None),
            make_ticket("A", "imprimante HP réseau problème", None, None),
            make_ticket("A", "imprimante laser couleur panne", None, None),
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
            make_ticket("A", "imprimante réseau bloquée", None, None),
            make_ticket("B", "imprimante laser problème", None, None),
        ];
        let result = build_profiles(tickets, "2025-09-12", "2026-03-12");
        assert!(result.vocabulary_size > 0);
        assert!(!result.vocabulary.is_empty());
        assert!(!result.idf_values.is_empty());
        assert_eq!(result.vocabulary.len(), result.idf_values.len());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib recommandation::profiling::tests -- --nocapture`
Expected: FAIL with "not yet implemented"

- [ ] **Step 3: Implement build_profiles**

Replace the `todo!()` body of `build_profiles`:

```rust
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
        by_tech
            .entry(ticket.technicien.clone())
            .or_default()
            .push(idx);
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

    // Build vocabulary map (stem → index)
    let vocabulary: HashMap<String, usize> = tfidf_result
        .vocabulary
        .iter()
        .enumerate()
        .map(|(i, term)| (term.clone(), i))
        .collect();

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

    // Sort profiles by technician name for deterministic output
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --lib recommandation::profiling::tests -- --nocapture`
Expected: 8 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/recommandation/profiling.rs
git commit -m "feat(recommandation): implement technician profiling (category + TF-IDF centroid)"
```

---

## Chunk 3: Scoring Module

### Task 4: Hybrid scoring engine

**Files:**
- Modify: `src-tauri/src/recommandation/scoring.rs` (add `score_tickets` function)
- Test: inline `#[cfg(test)]` in same file

- [ ] **Step 1: Write the scoring function signature and types**

Add to the top of `scoring.rs` (before the existing utility functions):

```rust
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

/// Scoring constants
const POIDS_CATEGORIE: f64 = 0.4;
const POIDS_TFIDF: f64 = 0.6;
const SCORE_MINIMUM_DEFAULT: f64 = 0.05;

/// Score unassigned tickets against technician profiles.
/// Returns recommendations sorted by ticket_id, each with top `limit` suggestions.
pub fn score_tickets(
    tickets: &[UnassignedTicket],
    profiling_data: &CachedProfilingData,
    stock_map: &TechnicianStockMap,
    seuil_tickets: f64,
    limit: usize,
    score_minimum: f64,
) -> Vec<AssignmentRecommendation> {
    todo!()
}
```

- [ ] **Step 2: Write failing tests**

Add tests to the existing `mod tests`:

```rust
    use super::*;
    use crate::recommandation::types::TechnicianProfile;

    fn make_profiling_data() -> CachedProfilingData {
        let mut vocab = HashMap::new();
        vocab.insert("imprim".to_string(), 0);
        vocab.insert("reseau".to_string(), 1);
        vocab.insert("sap".to_string(), 2);
        vocab.insert("acces".to_string(), 3);

        let idf = vec![1.5, 1.2, 2.0, 1.8];

        // Dupont: expert imprimante/réseau
        let mut cat_dupont = HashMap::new();
        cat_dupont.insert("Imprimante".to_string(), 0.8);
        cat_dupont.insert("Réseau".to_string(), 0.2);

        // Leroy: expert habilitations/SAP
        let mut cat_leroy = HashMap::new();
        cat_leroy.insert("Habilitations".to_string(), 0.9);
        cat_leroy.insert("Imprimante".to_string(), 0.1);

        CachedProfilingData {
            profiles: vec![
                TechnicianProfile {
                    technicien: "Dupont".to_string(),
                    nb_tickets_reference: 50,
                    cat_distribution: cat_dupont,
                    // centroid heavily weighted towards imprim/reseau
                    centroide_tfidf: vec![(0, 0.8), (1, 0.6)],
                },
                TechnicianProfile {
                    technicien: "Leroy".to_string(),
                    nb_tickets_reference: 30,
                    cat_distribution: cat_leroy,
                    // centroid weighted towards sap/acces
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
        // Dupont has way more stock
        let mut stock = HashMap::new();
        stock.insert("Dupont".to_string(), 60_usize);
        stock.insert("Leroy".to_string(), 2_usize);

        let results = score_tickets(&tickets, &data, &stock, 20.0, 3, 0.0);
        // Despite being more competent, Dupont's high stock should penalize him
        // Dupont charge factor: 1/(1+60/20) = 1/4 = 0.25
        // Leroy charge factor: 1/(1+2/20) = 1/1.1 ≈ 0.91
        // So Leroy may rank higher depending on competence gap
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
        // Without category, scoring is 100% TF-IDF
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
        // With a very high minimum, no suggestion should pass
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
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib recommandation::scoring::tests -- --nocapture`
Expected: New tests FAIL with "not yet implemented"

- [ ] **Step 4: Implement score_tickets**

Replace `todo!()` body:

```rust
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
            // Preprocess and project title into vocabulary
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
                    // TF-IDF score
                    let score_tfidf =
                        cosine_similarity_sparse(&vec_ticket, &profile.centroide_tfidf);

                    // Category score
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

                    // Charge penalty
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
```

- [ ] **Step 5: Run tests to verify all pass**

Run: `cd src-tauri && cargo test --lib recommandation::scoring::tests -- --nocapture`
Expected: 15 tests PASS (8 utility + 7 scoring)

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/recommandation/scoring.rs
git commit -m "feat(recommandation): implement hybrid scoring engine (category + TF-IDF + charge)"
```

---

## Chunk 4: Tauri IPC Commands + DB Queries

### Task 5: DB query for profiling tickets and cache operations

**Files:**
- Modify: `src-tauri/src/db/queries.rs` (add query functions)
- Test: inline `#[cfg(test)]`

- [ ] **Step 0: Make `get_seuil_tickets` visible to commands**

In `src-tauri/src/db/queries.rs`, line 20, change `fn get_seuil_tickets` to `pub(crate) fn get_seuil_tickets`. This function is currently module-private but needs to be accessible from `commands/recommandation.rs`.

- [ ] **Step 1: Add query functions**

Add at the end of `src-tauri/src/db/queries.rs`:

```rust
/// Fetch resolved tickets from the last N months for profiling.
pub fn get_profiling_tickets(
    conn: &Connection,
    import_id: i64,
    months_back: i64,
) -> Result<Vec<(String, String, Option<String>, Option<String>)>, rusqlite::Error> {
    let date_cutoff = chrono::Utc::now()
        .naive_utc()
        .date()
        .checked_sub_months(chrono::Months::new(months_back as u32))
        .unwrap_or(chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap())
        .format("%Y-%m-%d")
        .to_string();

    let mut stmt = conn.prepare(
        "SELECT technicien_principal, titre, categorie_niveau1, categorie_niveau2
         FROM tickets
         WHERE import_id = ?1
           AND est_vivant = 0
           AND technicien_principal IS NOT NULL
           AND COALESCE(date_resolution, date_cloture_approx, derniere_modification) >= ?2"
    )?;

    let rows = stmt.query_map(rusqlite::params![import_id, date_cutoff], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;

    rows.collect()
}

/// Fetch unassigned vivant tickets, ordered by ancienneté DESC, limited.
pub fn get_unassigned_tickets_for_attribution(
    conn: &Connection,
    import_id: i64,
    limit: usize,
) -> Result<Vec<(i64, String, Option<String>, Option<String>)>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, titre, categorie_niveau1, categorie_niveau2
         FROM tickets
         WHERE import_id = ?1
           AND est_vivant = 1
           AND (technicien_principal IS NULL OR technicien_principal = '')
         ORDER BY anciennete_jours DESC
         LIMIT ?2"
    )?;

    let rows = stmt.query_map(rusqlite::params![import_id, limit], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;

    rows.collect()
}

/// Get vivant ticket count per technician.
pub fn get_technician_stock_counts(
    conn: &Connection,
    import_id: i64,
) -> Result<std::collections::HashMap<String, usize>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT technicien_principal, COUNT(*) as cnt
         FROM tickets
         WHERE import_id = ?1
           AND est_vivant = 1
           AND technicien_principal IS NOT NULL
           AND technicien_principal != ''
         GROUP BY technicien_principal"
    )?;

    let mut map = std::collections::HashMap::new();
    let rows = stmt.query_map(rusqlite::params![import_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
    })?;

    for row in rows {
        let (tech, count) = row?;
        map.insert(tech, count);
    }

    Ok(map)
}

/// Read cached profiling data from analytics_cache.
pub fn get_cached_profiling(
    conn: &Connection,
    import_id: i64,
) -> Result<Option<String>, rusqlite::Error> {
    let result = conn.query_row(
        "SELECT result FROM analytics_cache
         WHERE import_id = ?1 AND analysis_type = 'technician_profiles' AND parameters = '{}'",
        rusqlite::params![import_id],
        |row| row.get::<_, String>(0),
    );

    match result {
        Ok(json) => Ok(Some(json)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Write profiling data to analytics_cache.
pub fn save_cached_profiling(
    conn: &Connection,
    import_id: i64,
    json_result: &str,
    duration_ms: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO analytics_cache (import_id, analysis_type, parameters, result, duration_ms)
         VALUES (?1, 'technician_profiles', '{}', ?2, ?3)",
        rusqlite::params![import_id, json_result, duration_ms],
    )?;
    Ok(())
}
```

- [ ] **Step 2: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: Compiles (may need `use chrono` and `use rusqlite::params` imports — add them if missing at top of file).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/db/queries.rs
git commit -m "feat(recommandation): add DB queries for profiling, unassigned tickets, and cache"
```

---

### Task 6: Tauri IPC commands

**Files:**
- Create: `src-tauri/src/commands/recommandation.rs`
- Modify: `src-tauri/src/commands/mod.rs` (add `pub mod recommandation;`)
- Modify: `src-tauri/src/lib.rs` (register commands in invoke_handler)

- [ ] **Step 1: Create the commands file**

```rust
// src-tauri/src/commands/recommandation.rs

use tauri::State;
use crate::state::AppState;
use crate::db::queries::{
    get_active_import_id, get_profiling_tickets, get_unassigned_tickets_for_attribution,
    get_technician_stock_counts, get_cached_profiling, save_cached_profiling,
    get_seuil_tickets,
};
use crate::recommandation::profiling::{build_profiles, ProfilingTicket};
use crate::recommandation::scoring::{score_tickets, UnassignedTicket};
use crate::recommandation::types::{
    AssignmentRecommendation, CachedProfilingData, ProfilingResult, RecommendationRequest,
};

const PERIODE_PROFIL_MOIS: i64 = 6;
const MAX_UNASSIGNED_TICKETS: usize = 100;

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
        .map(|(tech, titre, cat1, cat2)| ProfilingTicket {
            technicien: tech,
            titre,
            categorie_niveau1: cat1,
            categorie_niveau2: cat2,
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

        // Load cached profiles
        let json = get_cached_profiling(conn, import_id)
            .map_err(|e| e.to_string())?
            .ok_or("Profils non calculés. Cliquez sur 'Analyser' d'abord.")?;

        let profiling_data: CachedProfilingData = serde_json::from_str(&json)
            .map_err(|e| format!("Erreur désérialisation cache: {e}"))?;

        // Load unassigned tickets
        let raw_tickets = get_unassigned_tickets_for_attribution(conn, import_id, MAX_UNASSIGNED_TICKETS)
            .map_err(|e| e.to_string())?;

        let tickets: Vec<UnassignedTicket> = raw_tickets
            .into_iter()
            .map(|(id, titre, cat1, cat2)| UnassignedTicket {
                id,
                titre,
                categorie_niveau1: cat1,
                categorie_niveau2: cat2,
            })
            .collect();

        // Load current stock per technician
        let stock_map = get_technician_stock_counts(conn, import_id)
            .map_err(|e| e.to_string())?;

        // Load seuil from config
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
```

- [ ] **Step 2: Register module in commands/mod.rs**

Add `pub mod recommandation;` in `src-tauri/src/commands/mod.rs` after the existing modules.

- [ ] **Step 3: Register commands in lib.rs invoke_handler**

In `src-tauri/src/lib.rs`, inside the `tauri::generate_handler![]` macro (around line 117), add:
```rust
commands::recommandation::build_technician_profiles,
commands::recommandation::get_assignment_recommendations,
```

- [ ] **Step 4: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: Compiles. Fix any import issues (e.g., `get_seuil_tickets` visibility — it may need `pub` or `pub(crate)`).

- [ ] **Step 5: Run all existing tests to ensure no regressions**

Run: `cd src-tauri && cargo test`
Expected: All existing tests PASS + new recommandation tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/recommandation.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(recommandation): add Tauri IPC commands (build_profiles + get_recommendations)"
```

---

## Chunk 5: Frontend

### Task 7: TypeScript types

**Files:**
- Create: `src/types/recommandation.ts`
- Modify: `src/types/index.ts`

- [ ] **Step 1: Create TypeScript types**

```typescript
// src/types/recommandation.ts

export interface TechnicianSuggestion {
    technicien: string;
    scoreFinal: number;
    scoreCompetence: number;
    scoreCategorie: number;
    scoreTfidf: number;
    stockActuel: number;
    facteurCharge: number;
}

export interface AssignmentRecommendation {
    ticketId: number;
    ticketTitre: string;
    ticketCategorie: string | null;
    suggestions: TechnicianSuggestion[];
}

export interface ProfilingResult {
    profilesCount: number;
    vocabularySize: number;
    nbTicketsAnalysed: number;
    periodeFrom: string;
    periodeTo: string;
}

export interface RecommendationRequest {
    limitPerTicket?: number;
    scoreMinimum?: number;
}
```

- [ ] **Step 2: Add export to types/index.ts**

Add this line at the end of `src/types/index.ts`:
```typescript
export type * from './recommandation';
```

- [ ] **Step 3: Verify TypeScript compilation**

Run: `pnpm tsc --noEmit`
Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add src/types/recommandation.ts src/types/index.ts
git commit -m "feat(recommandation): add TypeScript types for attribution"
```

---

### Task 8: AttributionSection component

**Files:**
- Create: `src/components/stock/AttributionSection.tsx`
- Modify: `src/pages/StockPage.tsx`

- [ ] **Step 1: Create the component**

```tsx
// src/components/stock/AttributionSection.tsx

import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  ProfilingResult,
  AssignmentRecommendation,
  TechnicianSuggestion,
} from "../../types/recommandation";

function ScoreBar({ score }: { score: number }) {
  const pct = Math.round(score * 100);
  const color =
    score > 0.6
      ? "bg-emerald-500"
      : score > 0.3
        ? "bg-amber-500"
        : "bg-red-500";

  return (
    <div className="flex items-center gap-2">
      <div className="w-16 h-1.5 bg-slate-100 rounded-full overflow-hidden">
        <div
          className={`h-full rounded-full ${color}`}
          style={{ width: `${pct}%` }}
        />
      </div>
      <span className="font-[DM_Sans] font-semibold tabular-nums text-sm">
        {score.toFixed(2)}
      </span>
    </div>
  );
}

function SuggestionTable({
  suggestions,
}: {
  suggestions: TechnicianSuggestion[];
}) {
  if (suggestions.length === 0) {
    return (
      <p className="text-sm text-slate-400 italic font-[Source_Sans_3]">
        Aucune suggestion (scores trop faibles)
      </p>
    );
  }

  return (
    <table className="w-full text-sm">
      <thead>
        <tr className="text-left">
          <th className="pb-2 text-xs font-semibold uppercase tracking-wider text-slate-400">
            #
          </th>
          <th className="pb-2 text-xs font-semibold uppercase tracking-wider text-slate-400">
            Technicien
          </th>
          <th className="pb-2 text-xs font-semibold uppercase tracking-wider text-slate-400">
            Comp.
          </th>
          <th className="pb-2 text-xs font-semibold uppercase tracking-wider text-slate-400">
            Stock
          </th>
          <th className="pb-2 text-xs font-semibold uppercase tracking-wider text-slate-400">
            Score final
          </th>
        </tr>
      </thead>
      <tbody>
        {suggestions.map((s, idx) => (
          <tr
            key={s.technicien}
            className="hover:bg-[#0C419A]/[0.04] transition-colors"
          >
            <td className="py-1.5 font-[DM_Sans] font-semibold text-slate-400">
              {idx + 1}
            </td>
            <td className="py-1.5 font-[Source_Sans_3] font-semibold text-slate-800">
              {s.technicien}
            </td>
            <td className="py-1.5 font-[DM_Sans] font-semibold tabular-nums">
              {s.scoreCompetence.toFixed(2)}
            </td>
            <td className="py-1.5 font-[DM_Sans] font-semibold tabular-nums">
              {s.stockActuel}
            </td>
            <td className="py-1.5">
              <ScoreBar score={s.scoreFinal} />
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

export function AttributionSection() {
  const [loading, setLoading] = useState(false);
  const [profilingResult, setProfilingResult] =
    useState<ProfilingResult | null>(null);
  const [recommendations, setRecommendations] = useState<
    AssignmentRecommendation[]
  >([]);
  const [error, setError] = useState<string | null>(null);

  async function handleAnalyze() {
    setLoading(true);
    setError(null);
    try {
      const profiling = await invoke<ProfilingResult>(
        "build_technician_profiles"
      );
      setProfilingResult(profiling);

      if (profiling.profilesCount === 0) {
        setRecommendations([]);
        setLoading(false);
        return;
      }

      const recs = await invoke<AssignmentRecommendation[]>(
        "get_assignment_recommendations",
        { request: {} }
      );
      setRecommendations(recs);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold font-[DM_Sans] text-slate-800">
            Attribution intelligente
          </h2>
          <p className="text-sm text-slate-400 font-[Source_Sans_3]">
            Suggestions d'attribution basées sur les compétences et la charge
          </p>
        </div>
        <button
          onClick={handleAnalyze}
          disabled={loading}
          className="px-5 py-2.5 bg-[#0C419A] text-white font-[DM_Sans] font-semibold
                     rounded-xl hover:bg-[#082A66] transition-colors disabled:opacity-50
                     shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]
                     hover:shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)]"
        >
          {loading ? (
            <span className="flex items-center gap-2">
              <svg
                className="animate-spin h-4 w-4"
                viewBox="0 0 24 24"
                fill="none"
              >
                <circle
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  strokeWidth="4"
                  className="opacity-25"
                />
                <path
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
                  className="opacity-75"
                />
              </svg>
              Analyse en cours...
            </span>
          ) : (
            "Analyser"
          )}
        </button>
      </div>

      {/* Error */}
      {error && (
        <div className="bg-red-50 text-red-700 rounded-2xl p-4 font-[Source_Sans_3]">
          {error}
        </div>
      )}

      {/* Profiling status banner */}
      {profilingResult && (
        <div className="bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] p-4">
          <div className="flex items-center gap-3 text-sm font-[Source_Sans_3]">
            <span className="w-2 h-2 rounded-full bg-emerald-500" />
            <span className="text-slate-600">
              Profils calculés :{" "}
              <span className="font-semibold text-slate-800">
                {profilingResult.profilesCount} techniciens
              </span>
              ,{" "}
              <span className="font-semibold text-slate-800">
                {profilingResult.nbTicketsAnalysed} tickets
              </span>{" "}
              analysés
            </span>
            <span className="text-slate-400">
              Période : {profilingResult.periodeFrom} &rarr;{" "}
              {profilingResult.periodeTo}
            </span>
          </div>
        </div>
      )}

      {/* No profiles state */}
      {profilingResult && profilingResult.profilesCount === 0 && (
        <div className="bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] p-8 text-center">
          <div className="text-amber-500 text-3xl mb-2">&#9888;</div>
          <p className="text-slate-600 font-[Source_Sans_3]">
            Aucun ticket résolu trouvé dans les 6 derniers mois.
            <br />
            Impossible de construire des profils de compétence.
          </p>
        </div>
      )}

      {/* No unassigned tickets */}
      {profilingResult &&
        profilingResult.profilesCount > 0 &&
        recommendations.length === 0 &&
        !loading && (
          <div className="bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)] p-8 text-center">
            <div className="text-emerald-500 text-3xl mb-2">&#10003;</div>
            <p className="text-slate-600 font-[Source_Sans_3]">
              Aucun ticket non attribué — tous les tickets vivants ont un
              technicien assigné.
            </p>
          </div>
        )}

      {/* Recommendation cards */}
      {recommendations.length > 0 && (
        <div className="space-y-4">
          {recommendations.map((rec) => (
            <div
              key={rec.ticketId}
              className="bg-white rounded-2xl shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]
                         hover:shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)]
                         transition-shadow duration-200 p-6"
            >
              <div className="mb-3">
                <div className="flex items-baseline gap-2">
                  <span className="text-xs font-[DM_Sans] font-semibold text-slate-400">
                    #{rec.ticketId}
                  </span>
                  <h3 className="font-[DM_Sans] font-semibold text-slate-800">
                    {rec.ticketTitre}
                  </h3>
                </div>
                {rec.ticketCategorie && (
                  <span className="text-xs text-slate-400 font-[Source_Sans_3]">
                    {rec.ticketCategorie}
                  </span>
                )}
              </div>
              <SuggestionTable suggestions={rec.suggestions} />
            </div>
          ))}
          <p className="text-xs text-slate-400 font-[Source_Sans_3] text-center">
            {recommendations.length} ticket
            {recommendations.length > 1 ? "s" : ""} non attribué
            {recommendations.length > 1 ? "s" : ""}
          </p>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Integrate into StockPage**

In `src/pages/StockPage.tsx`:

1. Add import at the top:
```typescript
import { AttributionSection } from "../components/stock/AttributionSection";
```

2. Add the section after the technician table section (around line 438), before the unassigned drawer:
```tsx
{/* Attribution intelligente */}
<AttributionSection />
```

- [ ] **Step 3: Verify TypeScript compilation**

Run: `pnpm tsc --noEmit`
Expected: No errors.

- [ ] **Step 4: Verify frontend builds**

Run: `pnpm build`
Expected: Build succeeds.

- [ ] **Step 5: Commit**

```bash
git add src/components/stock/AttributionSection.tsx src/pages/StockPage.tsx
git commit -m "feat(recommandation): add AttributionSection UI component in StockPage"
```

---

## Chunk 6: Full Integration Test

### Task 9: End-to-end verification

- [ ] **Step 1: Run all Rust tests**

Run: `cd src-tauri && cargo test`
Expected: All tests PASS including new recommandation tests.

- [ ] **Step 2: Run TypeScript check**

Run: `pnpm tsc --noEmit`
Expected: No errors.

- [ ] **Step 3: Run Cargo clippy**

Run: `cd src-tauri && cargo clippy -- -D warnings`
Expected: No warnings.

- [ ] **Step 4: Test full app in dev mode**

Run: `cargo tauri dev`
Expected: App launches. Navigate to `/stock`, scroll down, see "Attribution intelligente" section with "Analyser" button. Click it. If data is loaded, profiling runs and recommendations appear.

- [ ] **Step 5: Final commit if any fixes were needed**

```bash
git add -A
git commit -m "fix(recommandation): address integration issues"
```
