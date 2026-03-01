use std::collections::HashMap;

/// Ticket avec délai de résolution pour l'analyse d'anomalies
#[derive(Debug, Clone)]
pub struct TicketDelay {
    pub ticket_id: u64,
    pub titre: String,
    pub technicien: Option<String>,
    pub groupe: Option<String>,
    pub delay_days: f64,
}

/// Anomalie détectée
#[derive(Debug, Clone)]
pub struct AnomalyResult {
    pub ticket_id: u64,
    pub titre: String,
    pub anomaly_type: String,
    pub severity: String,
    pub z_score: f64,
    pub delay_days: f64,
    pub technicien: Option<String>,
    pub groupe: Option<String>,
    pub description: String,
}

/// Ticket pour la détection de doublons
#[derive(Debug, Clone)]
pub struct TicketForDuplicates {
    pub ticket_id: u64,
    pub titre: String,
    pub groupe: Option<String>,
}

/// Paire de doublons potentiels
#[derive(Debug, Clone)]
pub struct DuplicatePair {
    pub ticket_a_id: u64,
    pub ticket_a_titre: String,
    pub ticket_b_id: u64,
    pub ticket_b_titre: String,
    pub similarity: f64,
    pub groupe: String,
}

/// Statistiques descriptives (mean, std) d'une slice
fn mean_std(values: &[f64]) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0);
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    (mean, variance.sqrt())
}

/// Détecte les anomalies de délai par Z-score sur distribution log-transformée.
///
/// # Arguments
/// * `tickets` — Liste de tickets avec leurs délais de résolution
/// * `z_threshold` — Seuil Z-score (défaut 2.5, RG-061)
///
/// # Algorithme
/// 1. Filtrer les tickets avec delay_days > 0 (exclure résolutions immédiates)
/// 2. Log-transformer : `log_delay = ln(delay_days + 1)`
/// 3. Calculer mean et std de la distribution log-transformée
/// 4. Pour chaque ticket : `z = (log_delay - mean) / std`
/// 5. Marquer comme anomalie si Z > z_threshold
pub fn detect_zscore_anomalies(
    tickets: &[TicketDelay],
    z_threshold: f64,
) -> Vec<AnomalyResult> {
    let valid: Vec<&TicketDelay> = tickets.iter().filter(|t| t.delay_days > 0.0).collect();

    if valid.is_empty() {
        return vec![];
    }

    let log_delays: Vec<f64> = valid.iter().map(|t| (t.delay_days + 1.0).ln()).collect();
    let (log_mean, log_std) = mean_std(&log_delays);

    if log_std < 1e-10 {
        return vec![];
    }

    let original_delays: Vec<f64> = valid.iter().map(|t| t.delay_days).collect();
    let (mean_delay, std_delay) = mean_std(&original_delays);

    let mut results: Vec<AnomalyResult> = valid
        .iter()
        .zip(log_delays.iter())
        .filter_map(|(ticket, &log_delay)| {
            let z = (log_delay - log_mean) / log_std;
            if z > z_threshold {
                let severity = if z > 3.5 {
                    "high".to_string()
                } else {
                    "medium".to_string()
                };
                let description = format!(
                    "Délai de {} jours (Z-score: {:.2}, attendu: {:.0}-{:.0} jours)",
                    ticket.delay_days,
                    z,
                    mean_delay,
                    mean_delay + std_delay,
                );
                Some(AnomalyResult {
                    ticket_id: ticket.ticket_id,
                    titre: ticket.titre.clone(),
                    anomaly_type: "delai_anormal".to_string(),
                    severity,
                    z_score: z,
                    delay_days: ticket.delay_days,
                    technicien: ticket.technicien.clone(),
                    groupe: ticket.groupe.clone(),
                    description,
                })
            } else {
                None
            }
        })
        .collect();

    results.sort_by(|a, b| {
        b.z_score
            .partial_cmp(&a.z_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

/// Détecte les paires de tickets potentiellement dupliqués.
///
/// # Arguments
/// * `tickets` — Liste de tickets vivants
/// * `similarity_threshold` — Seuil Jaro-Winkler (défaut 0.85, RG-058)
///
/// # Algorithme
/// 1. Grouper les tickets par groupe (tickets sans groupe → groupe "Inconnu")
/// 2. Pour chaque groupe, comparer toutes les paires (i, j) avec i < j
/// 3. Calculer la similarité Jaro-Winkler sur les titres en lowercase
/// 4. Si similarité > threshold → ajouter à la liste des doublons
/// 5. Ne pas comparer un ticket avec lui-même (même ID)
pub fn find_duplicates(
    tickets: &[TicketForDuplicates],
    similarity_threshold: f64,
) -> Vec<DuplicatePair> {
    let mut groups: HashMap<String, Vec<&TicketForDuplicates>> = HashMap::new();
    for ticket in tickets {
        let groupe = ticket
            .groupe
            .clone()
            .unwrap_or_else(|| "Inconnu".to_string());
        groups.entry(groupe).or_default().push(ticket);
    }

    let mut results: Vec<DuplicatePair> = Vec::new();

    for (groupe, group_tickets) in &groups {
        let n = group_tickets.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let a = group_tickets[i];
                let b = group_tickets[j];
                if a.ticket_id == b.ticket_id {
                    continue;
                }
                let similarity = strsim::jaro_winkler(
                    &a.titre.to_lowercase(),
                    &b.titre.to_lowercase(),
                );
                if similarity > similarity_threshold {
                    results.push(DuplicatePair {
                        ticket_a_id: a.ticket_id,
                        ticket_a_titre: a.titre.clone(),
                        ticket_b_id: b.ticket_id,
                        ticket_b_titre: b.titre.clone(),
                        similarity,
                        groupe: groupe.clone(),
                    });
                }
            }
        }
    }

    results.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_delay(id: u64, titre: &str, days: f64) -> TicketDelay {
        TicketDelay {
            ticket_id: id,
            titre: titre.to_string(),
            technicien: None,
            groupe: None,
            delay_days: days,
        }
    }

    fn make_dup(id: u64, titre: &str, groupe: Option<&str>) -> TicketForDuplicates {
        TicketForDuplicates {
            ticket_id: id,
            titre: titre.to_string(),
            groupe: groupe.map(str::to_string),
        }
    }

    #[test]
    fn test_zscore_basic() {
        let tickets = vec![
            make_delay(1, "Ticket normal A", 8.0),
            make_delay(2, "Ticket normal B", 10.0),
            make_delay(3, "Ticket normal C", 12.0),
            make_delay(4, "Ticket normal D", 9.0),
            make_delay(5, "Ticket normal E", 11.0),
            make_delay(6, "Ticket normal F", 7.0),
            make_delay(7, "Ticket normal G", 10.0),
            make_delay(8, "Ticket outlier", 500.0),
        ];
        let anomalies = detect_zscore_anomalies(&tickets, 2.5);
        assert!(!anomalies.is_empty(), "L'outlier à 500 jours doit être détecté");
        assert_eq!(
            anomalies[0].ticket_id, 8,
            "L'anomalie principale doit être le ticket 8"
        );
        assert!(anomalies[0].z_score > 2.5);
    }

    #[test]
    fn test_zscore_excludes_zero_delay() {
        let tickets = vec![
            make_delay(1, "Résolution immédiate", 0.0),
            make_delay(2, "Autre immédiat", 0.0),
        ];
        let anomalies = detect_zscore_anomalies(&tickets, 2.5);
        assert!(
            anomalies.is_empty(),
            "Les tickets à délai 0 ne doivent pas provoquer d'anomalie"
        );
    }

    #[test]
    fn test_zscore_empty_input() {
        let anomalies = detect_zscore_anomalies(&[], 2.5);
        assert!(anomalies.is_empty());
    }

    #[test]
    fn test_zscore_uniform_delays() {
        let tickets = vec![
            make_delay(1, "Ticket A", 5.0),
            make_delay(2, "Ticket B", 5.0),
            make_delay(3, "Ticket C", 5.0),
        ];
        let anomalies = detect_zscore_anomalies(&tickets, 2.5);
        assert!(
            anomalies.is_empty(),
            "Des délais uniformes ne doivent produire aucune anomalie (std ≈ 0)"
        );
    }

    #[test]
    fn test_duplicates_basic() {
        let tickets = vec![
            make_dup(1, "Imprimante bureau 3 en panne", Some("Support")),
            make_dup(2, "Imprimante bureau 3 ne fonctionne plus", Some("Support")),
            make_dup(3, "Problème réseau totalement différent", Some("Support")),
        ];
        let pairs = find_duplicates(&tickets, 0.85);
        // Titres très similaires dans le même groupe doivent être détectés
        let found = pairs.iter().any(|p| {
            (p.ticket_a_id == 1 && p.ticket_b_id == 2)
                || (p.ticket_a_id == 2 && p.ticket_b_id == 1)
        });
        assert!(
            found,
            "Les tickets aux titres très similaires dans le même groupe doivent être détectés (score={:.3})",
            pairs.first().map(|p| p.similarity).unwrap_or(0.0)
        );
    }

    #[test]
    fn test_duplicates_different_groups() {
        let tickets = vec![
            make_dup(1, "Imprimante bureau 3 en panne", Some("Support")),
            make_dup(2, "Imprimante bureau 3 en panne", Some("Réseau")),
        ];
        let pairs = find_duplicates(&tickets, 0.85);
        assert!(
            pairs.is_empty(),
            "Des tickets dans des groupes différents ne doivent pas être détectés comme doublons"
        );
    }

    #[test]
    fn test_duplicates_threshold() {
        // Titres similaires mais suffisamment différents pour ne pas passer 0.95
        let tickets = vec![
            make_dup(1, "Imprimante bureau 3 en panne", Some("Support")),
            make_dup(2, "Imprimante bureau 3 ne fonctionne plus", Some("Support")),
        ];
        let pairs_strict = find_duplicates(&tickets, 0.95);
        let pairs_lenient = find_duplicates(&tickets, 0.85);

        // Avec un seuil très strict, ces titres modérément similaires ne doivent pas passer
        // Avec un seuil lenient, au moins un des deux jeux trouve quelque chose
        let score = strsim::jaro_winkler(
            "imprimante bureau 3 en panne",
            "imprimante bureau 3 ne fonctionne plus",
        );
        if score <= 0.95 {
            assert!(
                pairs_strict.is_empty(),
                "Avec seuil 0.95, ces titres (score={:.3}) ne doivent pas être détectés",
                score
            );
        }
        // Vérifie la cohérence : un seuil plus bas trouve au moins autant de paires
        assert!(pairs_lenient.len() >= pairs_strict.len());
    }
}
