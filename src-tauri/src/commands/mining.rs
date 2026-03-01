use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::analytics::anomalies::{
    detect_zscore_anomalies, find_duplicates, TicketDelay, TicketForDuplicates,
};
use crate::analytics::clustering::run_kmeans;
use crate::nlp::preprocessing::{preprocess_corpus, StopWordFilter};
use crate::nlp::tfidf::{build_tfidf_matrix, corpus_stats, top_keywords, top_keywords_for_group};
use crate::state::AppState;

// ── Structs IPC ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextAnalysisRequest {
    pub corpus: String,
    pub scope: String,
    pub group_by: Option<String>,
    pub top_n: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextAnalysisResult {
    pub keywords: Vec<KeywordFrequency>,
    pub by_group: Option<Vec<GroupKeywords>>,
    pub corpus_stats: CorpusStats,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeywordFrequency {
    pub word: String,
    pub count: usize,
    pub tfidf_score: f64,
    pub doc_frequency: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupKeywords {
    pub group_name: String,
    pub keywords: Vec<KeywordFrequency>,
    pub ticket_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorpusStats {
    pub total_documents: usize,
    pub total_tokens: usize,
    pub vocabulary_size: usize,
    pub avg_tokens_per_doc: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterResult {
    pub clusters: Vec<Cluster>,
    pub silhouette_score: f64,
    pub total_tickets: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cluster {
    pub id: usize,
    pub label: String,
    pub top_keywords: Vec<String>,
    pub ticket_count: usize,
    pub ticket_ids: Vec<u64>,
    pub avg_resolution_days: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnomalyAlert {
    pub ticket_id: u64,
    pub titre: String,
    pub anomaly_type: String,
    pub severity: String,
    pub description: String,
    pub metric_value: f64,
    pub expected_range: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicatePairIpc {
    pub ticket_a_id: u64,
    pub ticket_a_titre: String,
    pub ticket_b_id: u64,
    pub ticket_b_titre: String,
    pub similarity: f64,
    pub groupe: String,
}

// ── Helper ────────────────────────────────────────────────────────────────────

fn get_active_import(conn: &rusqlite::Connection) -> Result<i64, String> {
    conn.query_row(
        "SELECT id FROM imports WHERE is_active = 1 ORDER BY id DESC LIMIT 1",
        [],
        |row| row.get(0),
    )
    .map_err(|e| format!("Aucun import actif: {e}"))
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn run_text_analysis(
    state: tauri::State<'_, AppState>,
    request: TextAnalysisRequest,
) -> Result<TextAnalysisResult, String> {
    // ── 1. Load all data from DB (sync, before spawn_blocking) ────────────────
    let need_groups = request.scope == "group" && request.group_by.is_some();
    let top_n = request.top_n.unwrap_or(20);

    let (texts, group_map, technician_names): (
        Vec<String>,
        Option<HashMap<String, Vec<usize>>>,
        Vec<String>,
    ) = {
        let guard = state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
        let conn = guard.as_ref().ok_or("Base de données non initialisée")?;

        let import_id = get_active_import(conn)?;

        // Load texts (and optionally group labels)
        let (texts, group_map) = if need_groups {
            let mut stmt = conn
                .prepare(
                    "SELECT titre, groupe_principal \
                     FROM tickets WHERE import_id = ?1 AND est_vivant = 1",
                )
                .map_err(|e| format!("SQL prepare: {e}"))?;

            let mut texts: Vec<String> = Vec::new();
            let mut groups: HashMap<String, Vec<usize>> = HashMap::new();

            let rows = stmt
                .query_map([import_id], |row: &rusqlite::Row<'_>| {
                    let titre: String = row.get(0)?;
                    let groupe: Option<String> = row.get(1)?;
                    Ok((titre, groupe))
                })
                .map_err(|e| format!("SQL query: {e}"))?;

            for row in rows {
                let (titre, groupe) = row.map_err(|e| format!("SQL row: {e}"))?;
                let doc_idx = texts.len();
                texts.push(titre);
                let group_key = groupe.unwrap_or_else(|| "Inconnu".to_string());
                groups.entry(group_key).or_default().push(doc_idx);
            }

            (texts, Some(groups))
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT titre FROM tickets WHERE import_id = ?1 AND est_vivant = 1",
                )
                .map_err(|e| format!("SQL prepare: {e}"))?;

            let texts: Vec<String> = stmt
                .query_map([import_id], |row: &rusqlite::Row<'_>| row.get(0))
                .map_err(|e| format!("SQL query: {e}"))?
                .collect::<Result<Vec<String>, _>>()
                .map_err(|e| format!("SQL collect: {e}"))?;

            (texts, None)
        };

        // Load technician names for dynamic stop words
        let mut tech_stmt = conn
            .prepare(
                "SELECT DISTINCT technicien_principal FROM tickets \
                 WHERE import_id = ?1 AND technicien_principal IS NOT NULL",
            )
            .map_err(|e| format!("SQL prepare techniciens: {e}"))?;

        let technician_names: Vec<String> = tech_stmt
            .query_map([import_id], |row: &rusqlite::Row<'_>| row.get(0))
            .map_err(|e| format!("SQL query techniciens: {e}"))?
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| format!("SQL collect techniciens: {e}"))?;

        (texts, group_map, technician_names)
    };

    // ── 2. Heavy NLP work in spawn_blocking ───────────────────────────────────
    let result =
        tokio::task::spawn_blocking(move || -> Result<TextAnalysisResult, String> {
            // Build stop-word filter with technician names
            let mut filter = StopWordFilter::new();
            filter.add_technician_names(&technician_names);

            // Preprocess corpus
            let tokenized = preprocess_corpus(&texts, &filter);

            // Count total tokens for stats
            let total_tokens: usize = tokenized.iter().map(|d| d.len()).sum();

            // Build TF-IDF matrix (min_df = 2 — RG-045)
            let tfidf = build_tfidf_matrix(&tokenized, 2);

            // Global keywords
            let global_kw = top_keywords(&tfidf, top_n);
            let keywords: Vec<KeywordFrequency> = global_kw
                .into_iter()
                .map(|kw| KeywordFrequency {
                    word: kw.word,
                    count: kw.doc_frequency,
                    tfidf_score: kw.tfidf_score,
                    doc_frequency: kw.doc_frequency,
                })
                .collect();

            // Per-group keywords
            let by_group: Option<Vec<GroupKeywords>> =
                group_map.map(|groups: HashMap<String, Vec<usize>>| {
                    let mut result: Vec<GroupKeywords> = groups
                        .iter()
                        .map(|(group_name, doc_indices)| {
                            let group_kw =
                                top_keywords_for_group(&tfidf, doc_indices, top_n);
                            let kws: Vec<KeywordFrequency> = group_kw
                                .into_iter()
                                .map(|kw| KeywordFrequency {
                                    word: kw.word,
                                    count: kw.doc_frequency,
                                    tfidf_score: kw.tfidf_score,
                                    doc_frequency: kw.doc_frequency,
                                })
                                .collect();
                            GroupKeywords {
                                group_name: group_name.clone(),
                                keywords: kws,
                                ticket_count: doc_indices.len(),
                            }
                        })
                        .collect();
                    result.sort_by(|a, b| b.ticket_count.cmp(&a.ticket_count));
                    result
                });

            // Corpus stats
            let stats = corpus_stats(&tfidf, total_tokens);
            let cs = CorpusStats {
                total_documents: stats.total_documents,
                total_tokens: stats.total_tokens,
                vocabulary_size: stats.vocabulary_size,
                avg_tokens_per_doc: stats.avg_tokens_per_doc,
            };

            Ok(TextAnalysisResult { keywords, by_group, corpus_stats: cs })
        })
        .await
        .map_err(|e| format!("spawn_blocking error: {e}"))??;

    Ok(result)
}

#[tauri::command]
pub async fn get_clusters(
    state: tauri::State<'_, AppState>,
    corpus: String,
    n_clusters: usize,
) -> Result<ClusterResult, String> {
    let _ = corpus; // corpus loaded from DB

    let (texts, ticket_ids, technician_names) = {
        let guard = state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
        let conn = guard.as_ref().ok_or("Base de données non initialisée")?;
        let import_id = get_active_import(conn)?;

        let mut stmt = conn
            .prepare(
                "SELECT id_ticket, titre FROM tickets WHERE import_id = ?1 AND est_vivant = 1",
            )
            .map_err(|e| format!("SQL prepare: {e}"))?;

        let mut texts: Vec<String> = Vec::new();
        let mut ticket_ids: Vec<u64> = Vec::new();

        let rows = stmt
            .query_map([import_id], |row| {
                let id: i64 = row.get(0)?;
                let titre: String = row.get(1)?;
                Ok((id, titre))
            })
            .map_err(|e| format!("SQL query: {e}"))?;

        for row in rows {
            let (id, titre) = row.map_err(|e| format!("SQL row: {e}"))?;
            ticket_ids.push(id as u64);
            texts.push(titre);
        }

        let mut tech_stmt = conn
            .prepare(
                "SELECT DISTINCT technicien_principal FROM tickets \
                 WHERE import_id = ?1 AND technicien_principal IS NOT NULL",
            )
            .map_err(|e| format!("SQL prepare techniciens: {e}"))?;

        let technician_names: Vec<String> = tech_stmt
            .query_map([import_id], |row| row.get(0))
            .map_err(|e| format!("SQL query techniciens: {e}"))?
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| format!("SQL collect techniciens: {e}"))?;

        (texts, ticket_ids, technician_names)
    };

    let total_tickets = ticket_ids.len();

    let result = tokio::task::spawn_blocking(move || -> Result<ClusterResult, String> {
        let mut filter = StopWordFilter::new();
        filter.add_technician_names(&technician_names);

        let tokenized = preprocess_corpus(&texts, &filter);
        let tfidf = build_tfidf_matrix(&tokenized, 2);

        let (k_min, k_max) = if n_clusters > 0 {
            (n_clusters, n_clusters)
        } else {
            (2, 10)
        };

        let clustering = run_kmeans(&tfidf.matrix, &tfidf.vocabulary, k_min, k_max, 100)?;

        let clusters: Vec<Cluster> = clustering
            .clusters
            .iter()
            .map(|ci| {
                let cluster_ticket_ids: Vec<u64> = ci
                    .doc_indices
                    .iter()
                    .filter_map(|&idx| ticket_ids.get(idx).copied())
                    .collect();
                Cluster {
                    id: ci.id,
                    label: ci.label.clone(),
                    top_keywords: ci.top_keywords.clone(),
                    ticket_count: ci.size,
                    ticket_ids: cluster_ticket_ids,
                    avg_resolution_days: None,
                }
            })
            .collect();

        Ok(ClusterResult { clusters, silhouette_score: clustering.silhouette_score, total_tickets })
    })
    .await
    .map_err(|e| format!("spawn_blocking error: {e}"))??;

    Ok(result)
}

#[tauri::command]
pub async fn detect_anomalies(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AnomalyAlert>, String> {
    let ticket_delays = {
        let guard = state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
        let conn = guard.as_ref().ok_or("Base de données non initialisée")?;
        let import_id = get_active_import(conn)?;

        let mut stmt = conn
            .prepare(
                "SELECT id_ticket, titre, technicien_principal, groupe_principal, anciennete_jours \
                 FROM tickets WHERE import_id = ?1 AND est_vivant = 0 \
                 AND anciennete_jours IS NOT NULL AND anciennete_jours > 0",
            )
            .map_err(|e| format!("SQL prepare: {e}"))?;

        let rows = stmt
            .query_map([import_id], |row| {
                let id: i64 = row.get(0)?;
                let titre: String = row.get(1)?;
                let technicien: Option<String> = row.get(2)?;
                let groupe: Option<String> = row.get(3)?;
                let anciennete: i64 = row.get(4)?;
                Ok((id, titre, technicien, groupe, anciennete))
            })
            .map_err(|e| format!("SQL query: {e}"))?;

        let mut delays: Vec<TicketDelay> = Vec::new();
        for row in rows {
            let (id, titre, technicien, groupe, anciennete) =
                row.map_err(|e| format!("SQL row: {e}"))?;
            delays.push(TicketDelay {
                ticket_id: id as u64,
                titre,
                technicien,
                groupe,
                delay_days: anciennete as f64,
            });
        }
        delays
    };

    let anomalies = detect_zscore_anomalies(&ticket_delays, 2.5);

    let alerts: Vec<AnomalyAlert> = anomalies
        .into_iter()
        .map(|a| AnomalyAlert {
            ticket_id: a.ticket_id,
            titre: a.titre,
            anomaly_type: a.anomaly_type,
            severity: a.severity,
            description: a.description,
            metric_value: a.delay_days,
            expected_range: format!("Z-score: {:.2}", a.z_score),
        })
        .collect();

    Ok(alerts)
}

#[tauri::command]
pub async fn detect_duplicates(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<DuplicatePairIpc>, String> {
    let tickets = {
        let guard = state.db.lock().map_err(|e| format!("Lock error: {e}"))?;
        let conn = guard.as_ref().ok_or("Base de données non initialisée")?;
        let import_id = get_active_import(conn)?;

        let mut stmt = conn
            .prepare(
                "SELECT id_ticket, titre, groupe_principal \
                 FROM tickets WHERE import_id = ?1 AND est_vivant = 1",
            )
            .map_err(|e| format!("SQL prepare: {e}"))?;

        let rows = stmt
            .query_map([import_id], |row| {
                let id: i64 = row.get(0)?;
                let titre: String = row.get(1)?;
                let groupe: Option<String> = row.get(2)?;
                Ok((id, titre, groupe))
            })
            .map_err(|e| format!("SQL query: {e}"))?;

        let mut tickets: Vec<TicketForDuplicates> = Vec::new();
        for row in rows {
            let (id, titre, groupe) = row.map_err(|e| format!("SQL row: {e}"))?;
            tickets.push(TicketForDuplicates { ticket_id: id as u64, titre, groupe });
        }
        tickets
    };

    let pairs = find_duplicates(&tickets, 0.85);

    let result: Vec<DuplicatePairIpc> = pairs
        .into_iter()
        .map(|p| DuplicatePairIpc {
            ticket_a_id: p.ticket_a_id,
            ticket_a_titre: p.ticket_a_titre,
            ticket_b_id: p.ticket_b_id,
            ticket_b_titre: p.ticket_b_titre,
            similarity: p.similarity,
            groupe: p.groupe,
        })
        .collect();

    Ok(result)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::nlp::preprocessing::{preprocess_corpus, StopWordFilter};
    use crate::nlp::tfidf::{build_tfidf_matrix, top_keywords};

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB");
        conn.execute_batch(
            "CREATE TABLE imports (id INTEGER PRIMARY KEY, is_active INTEGER NOT NULL);
             INSERT INTO imports VALUES (1, 1);
             CREATE TABLE tickets (
                 rowid INTEGER PRIMARY KEY,
                 import_id INTEGER NOT NULL,
                 id_ticket INTEGER NOT NULL,
                 titre TEXT NOT NULL,
                 statut TEXT NOT NULL,
                 est_vivant INTEGER NOT NULL DEFAULT 1,
                 technicien_principal TEXT,
                 groupe_principal TEXT
             );",
        )
        .expect("setup DDL");
        conn
    }

    fn insert_ticket(
        conn: &Connection,
        titre: &str,
        groupe: Option<&str>,
        tech: Option<&str>,
    ) {
        conn.execute(
            "INSERT INTO tickets \
             (import_id, id_ticket, titre, statut, est_vivant, technicien_principal, groupe_principal) \
             VALUES (1, 1, ?1, 'Ouvert', 1, ?2, ?3)",
            rusqlite::params![titre, tech, groupe],
        )
        .expect("insert ticket");
    }

    #[test]
    fn test_keywords_non_empty_from_corpus() {
        let texts = vec![
            "imprimante réseau bureau bloquée".to_string(),
            "connexion vpn imprimante lente".to_string(),
            "imprimante papier bureau".to_string(),
        ];
        let filter = StopWordFilter::new();
        let tokenized = preprocess_corpus(&texts, &filter);
        let tfidf = build_tfidf_matrix(&tokenized, 1);
        let kws = top_keywords(&tfidf, 10);
        assert!(!kws.is_empty(), "keywords should not be empty");
        assert!(kws.iter().all(|k| !k.word.is_empty()), "no empty keyword words");
    }

    #[test]
    fn test_corpus_stats_total_documents() {
        let texts: Vec<String> = (0..5)
            .map(|i| format!("ticket problème réseau bureau {i}"))
            .collect();
        let filter = StopWordFilter::new();
        let tokenized = preprocess_corpus(&texts, &filter);
        let tfidf = build_tfidf_matrix(&tokenized, 1);
        assert_eq!(tfidf.doc_count, 5, "total_documents should equal input count");
    }

    #[test]
    fn test_db_query_texts_and_keywords() {
        let conn = setup_db();
        insert_ticket(&conn, "imprimante réseau bloquée", Some("DSI"), Some("Jean Dupont"));
        insert_ticket(&conn, "connexion vpn lente réseau", Some("DSI"), Some("Marie Martin"));
        insert_ticket(&conn, "imprimante papier bureau bloquée", Some("Support"), None);

        let texts: Vec<String> = conn
            .prepare("SELECT titre FROM tickets WHERE import_id = 1 AND est_vivant = 1")
            .unwrap()
            .query_map([], |row: &rusqlite::Row<'_>| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let tech_names: Vec<String> = conn
            .prepare(
                "SELECT DISTINCT technicien_principal FROM tickets \
                 WHERE import_id = 1 AND technicien_principal IS NOT NULL",
            )
            .unwrap()
            .query_map([], |row: &rusqlite::Row<'_>| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(texts.len(), 3);
        assert_eq!(tech_names.len(), 2);

        let mut filter = StopWordFilter::new();
        filter.add_technician_names(&tech_names);
        let tokenized = preprocess_corpus(&texts, &filter);
        let tfidf = build_tfidf_matrix(&tokenized, 1);
        let kws = top_keywords(&tfidf, 10);

        assert!(!kws.is_empty(), "keywords should not be empty for 3 tickets");
        assert_eq!(tfidf.doc_count, 3, "corpus_stats.total_documents == 3");

        let kw_words: Vec<&str> = kws.iter().map(|k| k.word.as_str()).collect();
        assert!(
            !kw_words.contains(&"jean"),
            "'jean' (technician name) should not be in keywords"
        );
        assert!(
            !kw_words.contains(&"dupont"),
            "'dupont' should not be in keywords"
        );
    }
}
