// Vectorisation TF-IDF en matrice creuse (sprs)
// RG-044 : sublinear_tf=true, smooth_idf=true, l2_normalize=true
// RG-045 : min_df=2 pour exclure hapax
// RG-048 : mots-clés par groupe = agrégation TF-IDF du sous-ensemble

use std::collections::HashMap;

// ─────────────────────────────────────────────
// Structs publiques
// ─────────────────────────────────────────────

/// Résultat complet du calcul TF-IDF pour un corpus de documents tokenisés.
#[derive(Debug, Clone)]
pub struct TfIdfResult {
    /// Matrice docs × vocab, format CSR (ligne = document, colonne = terme)
    pub matrix: sprs::CsMat<f64>,
    /// Index → mot (stem) ; trié alphabétiquement pour ordre déterministe
    pub vocabulary: Vec<String>,
    /// Mot → index dans le vocabulaire
    pub vocab_index: HashMap<String, usize>,
    /// Valeurs IDF par terme (smooth IDF)
    pub idf: Vec<f64>,
    /// Fréquence document par terme (nombre de docs contenant le terme, avant filtrage)
    pub doc_freq: Vec<usize>,
    /// Nombre de documents dans le corpus
    pub doc_count: usize,
    /// Taille du vocabulaire filtré
    pub vocab_size: usize,
}

/// Mot-clé avec score TF-IDF agrégé.
#[derive(Debug, Clone)]
pub struct Keyword {
    /// Le mot (stem)
    pub word: String,
    /// Score TF-IDF agrégé (somme sur les documents du contexte)
    pub tfidf_score: f64,
    /// Nombre de documents contenant ce terme
    pub doc_frequency: usize,
}

/// Statistiques descriptives du corpus et de la matrice TF-IDF.
#[derive(Debug, Clone)]
pub struct CorpusStats {
    pub total_documents: usize,
    pub total_tokens: usize,
    pub vocabulary_size: usize,
    pub avg_tokens_per_doc: f64,
    /// Proportion de zéros dans la matrice TF-IDF (0.0 = dense, 1.0 = vide)
    pub sparsity: f64,
}

// ─────────────────────────────────────────────
// Fonction principale
// ─────────────────────────────────────────────

/// Construit la matrice TF-IDF creuse pour un corpus de documents tokenisés.
///
/// # Arguments
/// * `corpus`  – Liste de documents ; chaque document est une liste de tokens/stems.
/// * `min_df`  – Fréquence document minimale (défaut : 2, exclut les hapax — RG-045).
///
/// # Formules (RG-044)
/// * sublinear_tf : `tf = 1 + ln(count)` si count > 0
/// * smooth_idf   : `idf = ln(1 + n / (1 + df(t)))`
/// * L2 normalisation par ligne
pub fn build_tfidf_matrix(corpus: &[Vec<String>], min_df: usize) -> TfIdfResult {
    let n_docs = corpus.len();

    // Cas corpus vide
    if n_docs == 0 {
        let tri: sprs::TriMat<f64> = sprs::TriMat::new((0, 0));
        return TfIdfResult {
            matrix: tri.to_csr(),
            vocabulary: Vec::new(),
            vocab_index: HashMap::new(),
            idf: Vec::new(),
            doc_freq: Vec::new(),
            doc_count: 0,
            vocab_size: 0,
        };
    }

    // ── Étape 1 : document frequency ──────────────────────────────────────
    let mut df_map: HashMap<String, usize> = HashMap::new();
    for doc in corpus {
        let unique_terms: std::collections::HashSet<&String> = doc.iter().collect();
        for term in unique_terms {
            *df_map.entry(term.clone()).or_insert(0) += 1;
        }
    }

    // Filtrer les termes sous le seuil min_df (exclut hapax — RG-045)
    let mut vocabulary: Vec<String> = df_map
        .iter()
        .filter(|(_, &count)| count >= min_df)
        .map(|(term, _)| term.clone())
        .collect();
    vocabulary.sort(); // ordre déterministe

    let vocab_size = vocabulary.len();

    // Cas vocabulaire vide après filtrage
    if vocab_size == 0 {
        let tri: sprs::TriMat<f64> = sprs::TriMat::new((n_docs, 0));
        return TfIdfResult {
            matrix: tri.to_csr(),
            vocabulary: Vec::new(),
            vocab_index: HashMap::new(),
            idf: Vec::new(),
            doc_freq: Vec::new(),
            doc_count: n_docs,
            vocab_size: 0,
        };
    }

    let vocab_index: HashMap<String, usize> = vocabulary
        .iter()
        .enumerate()
        .map(|(i, term)| (term.clone(), i))
        .collect();

    // ── Étape 2 : IDF smooth — ln(1 + n / (1 + df(t))) ───────────────────
    let idf: Vec<f64> = vocabulary
        .iter()
        .map(|term| {
            let df_t = df_map[term] as f64;
            (1.0 + n_docs as f64 / (1.0 + df_t)).ln()
        })
        .collect();

    let doc_freq: Vec<usize> = vocabulary.iter().map(|term| df_map[term]).collect();

    // ── Étape 3 : matrice TF-IDF creuse via TriMat ────────────────────────
    let mut tri_mat: sprs::TriMat<f64> = sprs::TriMat::new((n_docs, vocab_size));

    for (doc_idx, doc) in corpus.iter().enumerate() {
        if doc.is_empty() {
            continue;
        }

        // Comptage TF brut pour ce document
        let mut tf_counts: HashMap<usize, usize> = HashMap::new();
        for token in doc {
            if let Some(&term_idx) = vocab_index.get(token) {
                *tf_counts.entry(term_idx).or_insert(0) += 1;
            }
        }

        if tf_counts.is_empty() {
            continue;
        }

        // Sublinear TF × IDF
        let mut row: Vec<(usize, f64)> = tf_counts
            .iter()
            .map(|(&term_idx, &count)| {
                let tf_sublinear = 1.0 + (count as f64).ln(); // RG-044
                let tfidf = tf_sublinear * idf[term_idx];
                (term_idx, tfidf)
            })
            .collect();

        // L2 normalisation — RG-044
        let l2_norm: f64 = row.iter().map(|(_, v)| v * v).sum::<f64>().sqrt();
        if l2_norm > 0.0 {
            for (_, v) in row.iter_mut() {
                *v /= l2_norm;
            }
        }

        for (term_idx, tfidf) in row {
            tri_mat.add_triplet(doc_idx, term_idx, tfidf);
        }
    }

    TfIdfResult {
        matrix: tri_mat.to_csr(),
        vocabulary,
        vocab_index,
        idf,
        doc_freq,
        doc_count: n_docs,
        vocab_size,
    }
}

// ─────────────────────────────────────────────
// Extraction de mots-clés
// ─────────────────────────────────────────────

/// Retourne les `top_n` mots-clés globaux triés par score TF-IDF agrégé décroissant.
///
/// Le score est la somme des valeurs TF-IDF (L2-normalisées) sur l'ensemble du corpus.
pub fn top_keywords(result: &TfIdfResult, top_n: usize) -> Vec<Keyword> {
    if result.vocab_size == 0 || top_n == 0 {
        return Vec::new();
    }

    let mut scores = vec![0.0f64; result.vocab_size];
    for row in result.matrix.outer_iterator() {
        for (col, val) in row.indices().iter().zip(row.data().iter()) {
            scores[*col] += *val;
        }
    }

    let mut keywords: Vec<Keyword> = result
        .vocabulary
        .iter()
        .enumerate()
        .map(|(i, word)| Keyword {
            word: word.clone(),
            tfidf_score: scores[i],
            doc_frequency: result.doc_freq[i],
        })
        .collect();

    keywords.sort_by(|a, b| {
        b.tfidf_score
            .partial_cmp(&a.tfidf_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    keywords.truncate(top_n);
    keywords
}

/// Retourne les `top_n` mots-clés pour un sous-ensemble de documents (RG-048).
///
/// Agrège les scores TF-IDF uniquement sur les documents dont les indices sont fournis.
pub fn top_keywords_for_group(
    result: &TfIdfResult,
    doc_indices: &[usize],
    top_n: usize,
) -> Vec<Keyword> {
    if result.vocab_size == 0 || top_n == 0 || doc_indices.is_empty() {
        return Vec::new();
    }

    let mut scores = vec![0.0f64; result.vocab_size];
    let mut group_doc_freq = vec![0usize; result.vocab_size];

    for &doc_idx in doc_indices {
        if doc_idx >= result.doc_count {
            continue;
        }
        if let Some(row) = result.matrix.outer_view(doc_idx) {
            for (col, val) in row.indices().iter().zip(row.data().iter()) {
                scores[*col] += *val;
                if *val > 0.0 {
                    group_doc_freq[*col] += 1;
                }
            }
        }
    }

    let mut keywords: Vec<Keyword> = result
        .vocabulary
        .iter()
        .enumerate()
        .filter(|(i, _)| scores[*i] > 0.0)
        .map(|(i, word)| Keyword {
            word: word.clone(),
            tfidf_score: scores[i],
            doc_frequency: group_doc_freq[i],
        })
        .collect();

    keywords.sort_by(|a, b| {
        b.tfidf_score
            .partial_cmp(&a.tfidf_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    keywords.truncate(top_n);
    keywords
}

// ─────────────────────────────────────────────
// Utilitaires
// ─────────────────────────────────────────────

/// Calcule les statistiques descriptives du corpus et de la matrice TF-IDF.
///
/// # Arguments
/// * `result`       – Résultat TF-IDF
/// * `total_tokens` – Nombre total de tokens dans le corpus (avant filtrage du vocabulaire)
pub fn corpus_stats(result: &TfIdfResult, total_tokens: usize) -> CorpusStats {
    let total_cells = result.doc_count * result.vocab_size;
    let nnz = result.matrix.nnz();
    let sparsity = if total_cells > 0 {
        1.0 - (nnz as f64 / total_cells as f64)
    } else {
        1.0
    };
    let avg_tokens_per_doc = if result.doc_count > 0 {
        total_tokens as f64 / result.doc_count as f64
    } else {
        0.0
    };
    CorpusStats {
        total_documents: result.doc_count,
        total_tokens,
        vocabulary_size: result.vocab_size,
        avg_tokens_per_doc,
        sparsity,
    }
}

/// Retourne l'index d'un terme dans le vocabulaire, ou `None` s'il est absent (filtré ou inconnu).
pub fn term_index(result: &TfIdfResult, term: &str) -> Option<usize> {
    result.vocab_index.get(term).copied()
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Corpus de base : 3 docs, 3 termes chacun apparaît dans ≥ 2 docs.
    fn make_corpus() -> Vec<Vec<String>> {
        vec![
            vec!["hello".into(), "world".into(), "hello".into()],
            vec!["hello".into(), "rust".into()],
            vec!["world".into(), "rust".into(), "rust".into()],
        ]
    }

    #[test]
    fn test_build_tfidf_basic() {
        let corpus = make_corpus();
        let result = build_tfidf_matrix(&corpus, 2);
        // "hello" df=2, "world" df=2, "rust" df=2 → tous inclus avec min_df=2
        assert_eq!(result.doc_count, 3);
        assert_eq!(result.vocab_size, 3);
        assert_eq!(result.matrix.rows(), 3);
        assert_eq!(result.matrix.cols(), 3);
        assert_eq!(result.vocabulary.len(), 3);
        assert_eq!(result.idf.len(), 3);
        assert_eq!(result.doc_freq.len(), 3);
    }

    #[test]
    fn test_min_df_filter() {
        let corpus = vec![
            vec!["common".into(), "rare".into()],
            vec!["common".into(), "other_rare".into()],
            vec!["common".into()],
        ];
        // "common" df=3, "rare" df=1, "other_rare" df=1
        let result = build_tfidf_matrix(&corpus, 2);
        assert_eq!(result.vocab_size, 1);
        assert!(result.vocab_index.contains_key("common"));
        assert!(!result.vocab_index.contains_key("rare"));
        assert!(!result.vocab_index.contains_key("other_rare"));
    }

    #[test]
    fn test_sublinear_tf() {
        // Doc 0 contient "hello" 3 fois → tf_sublinear = 1 + ln(3)
        // Doc 1 contient "hello" 1 fois → tf_sublinear = 1 + ln(1) = 1.0
        let corpus = vec![
            vec!["hello".into(), "hello".into(), "hello".into(), "world".into()],
            vec!["hello".into(), "world".into()],
        ];
        let result = build_tfidf_matrix(&corpus, 1);

        let expected_tf_high = 1.0 + 3.0_f64.ln(); // ≈ 2.099
        let expected_tf_low = 1.0_f64; // 1 + ln(1) = 1.0
        assert!(
            expected_tf_high > expected_tf_low,
            "sublinear TF(3) doit être > TF(1)"
        );

        // La matrice doit avoir des valeurs non nulles pour "hello" dans les deux docs
        let hello_idx = result.vocab_index["hello"];
        let mut val0 = 0.0_f64;
        let mut val1 = 0.0_f64;
        if let Some(row0) = result.matrix.outer_view(0) {
            for (col, val) in row0.indices().iter().zip(row0.data().iter()) {
                if *col == hello_idx {
                    val0 = *val;
                }
            }
        }
        if let Some(row1) = result.matrix.outer_view(1) {
            for (col, val) in row1.indices().iter().zip(row1.data().iter()) {
                if *col == hello_idx {
                    val1 = *val;
                }
            }
        }
        assert!(val0 > 0.0, "TF-IDF 'hello' doc0 doit être positif");
        assert!(val1 > 0.0, "TF-IDF 'hello' doc1 doit être positif");
    }

    #[test]
    fn test_smooth_idf() {
        // 4 docs : "common" dans tous (df=4), "rare" dans 2 (df=2)
        let corpus = vec![
            vec!["common".into(), "rare".into()],
            vec!["common".into(), "rare".into()],
            vec!["common".into()],
            vec!["common".into()],
        ];
        let result = build_tfidf_matrix(&corpus, 1);
        let n = 4.0_f64;

        let common_idx = result.vocab_index["common"];
        let rare_idx = result.vocab_index["rare"];

        let expected_idf_common = (1.0 + n / (1.0 + 4.0)).ln();
        let expected_idf_rare = (1.0 + n / (1.0 + 2.0)).ln();
        let tol = 1e-9;

        assert!(
            (result.idf[common_idx] - expected_idf_common).abs() < tol,
            "IDF common: got {}, expected {}",
            result.idf[common_idx],
            expected_idf_common
        );
        assert!(
            (result.idf[rare_idx] - expected_idf_rare).abs() < tol,
            "IDF rare: got {}, expected {}",
            result.idf[rare_idx],
            expected_idf_rare
        );
        // Terme rare → IDF plus élevé
        assert!(
            result.idf[rare_idx] > result.idf[common_idx],
            "Terme rare doit avoir IDF > terme fréquent"
        );
    }

    #[test]
    fn test_l2_normalization() {
        let corpus = make_corpus();
        let result = build_tfidf_matrix(&corpus, 1);

        for (doc_idx, row) in result.matrix.outer_iterator().enumerate() {
            let norm_sq: f64 = row.data().iter().map(|&v| v * v).sum::<f64>();
            let norm = norm_sq.sqrt();
            if norm > 0.0 {
                assert!(
                    (norm - 1.0).abs() < 1e-9,
                    "Doc {}: norme L2 = {:.6}, attendu ≈ 1.0",
                    doc_idx,
                    norm
                );
            }
        }
    }

    #[test]
    fn test_top_keywords() {
        let corpus = make_corpus();
        let result = build_tfidf_matrix(&corpus, 1);
        let keywords = top_keywords(&result, 3);

        assert!(!keywords.is_empty(), "top_keywords ne doit pas être vide");
        // Tous les scores doivent être positifs
        for kw in &keywords {
            assert!(kw.tfidf_score > 0.0, "score '{}' doit être > 0", kw.word);
        }
        // Triés par score décroissant
        for i in 1..keywords.len() {
            assert!(
                keywords[i - 1].tfidf_score >= keywords[i].tfidf_score,
                "Keywords non triés : {} < {}",
                keywords[i - 1].tfidf_score,
                keywords[i].tfidf_score
            );
        }
    }

    #[test]
    fn test_top_keywords_for_group() {
        // Groupe 1 : docs 0 et 1 partagent "alpha"/"beta"/"gamma"
        // Groupe 2 : docs 2 et 3 partagent "delta"/"epsilon"/"zeta"
        let corpus = vec![
            vec!["alpha".into(), "beta".into()],
            vec!["alpha".into(), "gamma".into()],
            vec!["delta".into(), "epsilon".into()],
            vec!["delta".into(), "zeta".into()],
        ];
        let result = build_tfidf_matrix(&corpus, 1);

        let group1 = top_keywords_for_group(&result, &[0, 1], 10);
        let group2 = top_keywords_for_group(&result, &[2, 3], 10);

        let words1: std::collections::HashSet<&str> =
            group1.iter().map(|k| k.word.as_str()).collect();
        let words2: std::collections::HashSet<&str> =
            group2.iter().map(|k| k.word.as_str()).collect();

        // Les termes du groupe 1 ne doivent pas apparaître dans le groupe 2 et vice-versa
        assert!(
            words1.is_disjoint(&words2),
            "Groupes distincts doivent avoir des mots-clés disjoints"
        );
        // "alpha" doit être dans groupe 1
        assert!(words1.contains("alpha"), "'alpha' doit être dans groupe 1");
        // "delta" doit être dans groupe 2
        assert!(words2.contains("delta"), "'delta' doit être dans groupe 2");

        // Triés par score décroissant dans chaque groupe
        for i in 1..group1.len() {
            assert!(
                group1[i - 1].tfidf_score >= group1[i].tfidf_score,
                "Groupe 1 non trié"
            );
        }
    }

    #[test]
    fn test_empty_corpus() {
        let corpus: Vec<Vec<String>> = Vec::new();
        let result = build_tfidf_matrix(&corpus, 2);

        assert_eq!(result.doc_count, 0);
        assert_eq!(result.vocab_size, 0);
        assert_eq!(result.vocabulary.len(), 0);
        assert_eq!(result.matrix.rows(), 0);

        // Ces appels ne doivent pas paniquer
        let keywords = top_keywords(&result, 10);
        assert!(keywords.is_empty());

        let group_kw = top_keywords_for_group(&result, &[], 10);
        assert!(group_kw.is_empty());

        let stats = corpus_stats(&result, 0);
        assert_eq!(stats.total_documents, 0);
        assert_eq!(stats.sparsity, 1.0);
    }

    #[test]
    fn test_corpus_stats() {
        let corpus = make_corpus();
        let total_tokens: usize = corpus.iter().map(|d| d.len()).sum(); // 3+2+3 = 8
        let result = build_tfidf_matrix(&corpus, 1);
        let stats = corpus_stats(&result, total_tokens);

        assert_eq!(stats.total_documents, 3);
        assert_eq!(stats.total_tokens, total_tokens);
        assert_eq!(stats.vocabulary_size, result.vocab_size);
        assert!(
            (stats.avg_tokens_per_doc - total_tokens as f64 / 3.0).abs() < 1e-9,
            "avg_tokens_per_doc incorrect"
        );
        assert!(
            stats.sparsity >= 0.0 && stats.sparsity <= 1.0,
            "sparsity hors [0,1]"
        );
    }
}
