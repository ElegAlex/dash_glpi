use crate::commands::stock::{AgeRangeCount, TechnicianStock};
use crate::config::AppConfig;

/// Médiane d'un slice de f64. Retourne 0.0 si vide.
pub fn compute_median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

/// Distribue des anciennetés en 4 tranches : <7j, 7-30j, 30-90j, >90j.
pub fn compute_age_distribution(anciennetes: &[i64]) -> Vec<AgeRangeCount> {
    let total = anciennetes.len();

    let mut lt7: usize = 0;
    let mut from7to30: usize = 0;
    let mut from30to90: usize = 0;
    let mut gt90: usize = 0;

    for &a in anciennetes {
        if a < 7 {
            lt7 += 1;
        } else if a < 30 {
            from7to30 += 1;
        } else if a < 90 {
            from30to90 += 1;
        } else {
            gt90 += 1;
        }
    }

    let pct = |count: usize| -> f64 {
        if total == 0 {
            0.0
        } else {
            count as f64 / total as f64 * 100.0
        }
    };

    vec![
        AgeRangeCount {
            label: "< 7j".to_string(),
            threshold_days: 7,
            count: lt7,
            percentage: pct(lt7),
        },
        AgeRangeCount {
            label: "7-30j".to_string(),
            threshold_days: 30,
            count: from7to30,
            percentage: pct(from7to30),
        },
        AgeRangeCount {
            label: "30-90j".to_string(),
            threshold_days: 90,
            count: from30to90,
            percentage: pct(from30to90),
        },
        AgeRangeCount {
            label: "> 90j".to_string(),
            threshold_days: usize::MAX,
            count: gt90,
            percentage: pct(gt90),
        },
    ]
}

/// Retourne la couleur RAG selon le nombre de tickets et les seuils de la config.
/// Vert  : stock < seuil_couleur_vert
/// Jaune : seuil_couleur_vert  <= stock < seuil_couleur_jaune
/// Orange: seuil_couleur_jaune <= stock < seuil_couleur_orange
/// Rouge : stock >= seuil_couleur_orange
pub fn compute_couleur_seuil(stock: u32, config: &AppConfig) -> String {
    if stock < config.seuil_couleur_vert {
        "vert".to_string()
    } else if stock < config.seuil_couleur_jaune {
        "jaune".to_string()
    } else if stock < config.seuil_couleur_orange {
        "orange".to_string()
    } else {
        "rouge".to_string()
    }
}

/// Calcule ecart_seuil et couleur_seuil pour chaque technicien.
pub fn enrich_technician_stock(techs: &mut Vec<TechnicianStock>, config: &AppConfig) {
    for tech in techs.iter_mut() {
        tech.ecart_seuil = tech.total as i64 - config.seuil_tickets_technicien as i64;
        tech.couleur_seuil = compute_couleur_seuil(tech.total as u32, config);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::commands::stock::TechnicianStock;

    fn default_config() -> AppConfig {
        AppConfig {
            seuil_tickets_technicien: 20,
            seuil_anciennete_cloturer: 90,
            seuil_inactivite_cloturer: 60,
            seuil_anciennete_relancer: 30,
            seuil_inactivite_relancer: 14,
            seuil_couleur_vert: 10,
            seuil_couleur_jaune: 20,
            seuil_couleur_orange: 40,
            statuts_vivants: vec![],
            statuts_termines: vec![],
        }
    }

    // --- compute_median ---

    #[test]
    fn test_median_vide() {
        assert_eq!(compute_median(&[]), 0.0);
    }

    #[test]
    fn test_median_impair() {
        assert_eq!(compute_median(&[3.0, 1.0, 2.0]), 2.0);
    }

    #[test]
    fn test_median_pair() {
        assert_eq!(compute_median(&[4.0, 1.0, 3.0, 2.0]), 2.5);
    }

    #[test]
    fn test_median_un_element() {
        assert_eq!(compute_median(&[42.0]), 42.0);
    }

    // --- compute_age_distribution ---

    #[test]
    fn test_age_distribution_vide() {
        let dist = compute_age_distribution(&[]);
        assert_eq!(dist.len(), 4);
        for range in &dist {
            assert_eq!(range.count, 0);
            assert_eq!(range.percentage, 0.0);
        }
    }

    #[test]
    fn test_age_distribution_repartition() {
        let ages: Vec<i64> = vec![1, 5, 10, 25, 35, 60, 100, 200];
        let dist = compute_age_distribution(&ages);

        assert_eq!(dist[0].label, "< 7j");
        assert_eq!(dist[0].count, 2); // 1, 5

        assert_eq!(dist[1].label, "7-30j");
        assert_eq!(dist[1].count, 2); // 10, 25

        assert_eq!(dist[2].label, "30-90j");
        assert_eq!(dist[2].count, 2); // 35, 60

        assert_eq!(dist[3].label, "> 90j");
        assert_eq!(dist[3].count, 2); // 100, 200

        // Percentages doivent totaliser ~100%
        let total_pct: f64 = dist.iter().map(|r| r.percentage).sum();
        assert!((total_pct - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_age_distribution_bornes() {
        // 7 doit aller dans 7-30j (>= 7), 30 dans 30-90j (>= 30), 90 dans > 90j (>= 90)
        let dist = compute_age_distribution(&[7, 30, 90]);
        assert_eq!(dist[0].count, 0); // < 7j : aucun
        assert_eq!(dist[1].count, 1); // 7-30j : 7
        assert_eq!(dist[2].count, 1); // 30-90j : 30
        assert_eq!(dist[3].count, 1); // > 90j : 90
    }

    // --- compute_couleur_seuil ---

    #[test]
    fn test_couleur_vert() {
        let config = default_config(); // seuil_vert=10
        assert_eq!(compute_couleur_seuil(0, &config), "vert");
        assert_eq!(compute_couleur_seuil(9, &config), "vert");
    }

    #[test]
    fn test_couleur_jaune() {
        let config = default_config(); // seuil_vert=10, seuil_jaune=20
        assert_eq!(compute_couleur_seuil(10, &config), "jaune");
        assert_eq!(compute_couleur_seuil(19, &config), "jaune");
    }

    #[test]
    fn test_couleur_orange() {
        let config = default_config(); // seuil_jaune=20, seuil_orange=40
        assert_eq!(compute_couleur_seuil(20, &config), "orange");
        assert_eq!(compute_couleur_seuil(39, &config), "orange");
    }

    #[test]
    fn test_couleur_rouge() {
        let config = default_config(); // seuil_orange=40
        assert_eq!(compute_couleur_seuil(40, &config), "rouge");
        assert_eq!(compute_couleur_seuil(100, &config), "rouge");
    }

    // --- enrich_technician_stock ---

    fn make_tech(nom: &str, total: usize) -> TechnicianStock {
        TechnicianStock {
            technicien: nom.to_string(),
            total,
            en_cours: 0,
            en_attente: 0,
            planifie: 0,
            nouveau: 0,
            incidents: 0,
            demandes: 0,
            age_moyen_jours: 0.0,
            inactifs_14j: 0,
            ecart_seuil: 0,
            couleur_seuil: String::new(),
        }
    }

    #[test]
    fn test_enrich_technician_stock() {
        let config = default_config(); // seuil=20, vert<10, jaune<20, orange<40
        let mut techs = vec![
            make_tech("Alice", 5),  // vert, écart = -15
            make_tech("Bob", 15),   // jaune, écart = -5
            make_tech("Charlie", 25), // orange, écart = +5
            make_tech("Dana", 50),  // rouge, écart = +30
        ];
        enrich_technician_stock(&mut techs, &config);

        assert_eq!(techs[0].ecart_seuil, -15);
        assert_eq!(techs[0].couleur_seuil, "vert");

        assert_eq!(techs[1].ecart_seuil, -5);
        assert_eq!(techs[1].couleur_seuil, "jaune");

        assert_eq!(techs[2].ecart_seuil, 5);
        assert_eq!(techs[2].couleur_seuil, "orange");

        assert_eq!(techs[3].ecart_seuil, 30);
        assert_eq!(techs[3].couleur_seuil, "rouge");
    }
}
