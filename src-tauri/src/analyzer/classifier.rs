use crate::config::AppConfig;
use crate::parser::types::GlpiTicketNormalized;

/// Classifie un ticket vivant et affecte action_recommandee + motif_classification.
/// Ordre de priorité : Zombie > Ancien > Inactif > Normal.
/// Les tickets terminés (est_vivant = false) ne sont pas modifiés.
pub fn classify_ticket(ticket: &mut GlpiTicketNormalized, config: &AppConfig) {
    if !ticket.est_vivant {
        return;
    }

    // Zombie : vivant + 0 suivis (None ou 0)
    if ticket.nombre_suivis.unwrap_or(0) == 0 {
        ticket.action_recommandee = Some("qualifier".to_string());
        ticket.motif_classification = Some("Ticket sans suivi".to_string());
        return;
    }

    // Ancien : ancienneté dépasse le seuil de clôture
    if let Some(anciennete) = ticket.anciennete_jours {
        if anciennete > config.seuil_anciennete_cloturer as i64 {
            ticket.action_recommandee = Some("clôturer".to_string());
            ticket.motif_classification =
                Some(format!("Ancienneté > {}j", config.seuil_anciennete_cloturer));
            return;
        }
    }

    // Inactif : inactivité dépasse le seuil de relance
    if let Some(inactivite) = ticket.inactivite_jours {
        if inactivite > config.seuil_inactivite_relancer as i64 {
            ticket.action_recommandee = Some("relancer".to_string());
            ticket.motif_classification = Some(format!("Inactif depuis {}j", inactivite));
            return;
        }
    }

    // En cours normal
    ticket.action_recommandee = Some("suivre".to_string());
    ticket.motif_classification = Some("En cours normal".to_string());
}

/// Retourne le poids de pondération pour une priorité GLPI (libellé français).
/// Inclut "Majeure" (non standard GLPI vanilla, présent dans l'export CPAM 92).
pub fn poids_priorite(priorite: &str) -> u8 {
    match priorite.trim() {
        "Très haute" => 5,
        "Haute" | "Majeure" => 4,
        "Moyenne" => 3,
        "Basse" => 2,
        "Très basse" => 1,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::parser::types::GlpiTicketNormalized;

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

    fn base_ticket() -> GlpiTicketNormalized {
        GlpiTicketNormalized {
            id: 1,
            titre: "Test".to_string(),
            statut: "En cours (Attribué)".to_string(),
            type_ticket: "Incident".to_string(),
            priorite: None,
            urgence: None,
            demandeur: String::new(),
            date_ouverture: String::new(),
            derniere_modification: None,
            nombre_suivis: Some(1),
            suivis_description: String::new(),
            solution: String::new(),
            taches_description: String::new(),
            intervention_fournisseur: String::new(),
            techniciens: vec![],
            groupes: vec![],
            technicien_principal: None,
            groupe_principal: None,
            groupe_niveau1: None,
            groupe_niveau2: None,
            groupe_niveau3: None,
            categorie: None,
            categorie_niveau1: None,
            categorie_niveau2: None,
            est_vivant: true,
            anciennete_jours: Some(5),
            inactivite_jours: Some(3),
            date_cloture_approx: None,
            action_recommandee: None,
            motif_classification: None,
        }
    }

    #[test]
    fn test_classify_zombie_aucun_suivi() {
        let config = default_config();
        let mut ticket = base_ticket();
        ticket.nombre_suivis = None;
        classify_ticket(&mut ticket, &config);
        assert_eq!(ticket.action_recommandee.as_deref(), Some("qualifier"));
        assert_eq!(
            ticket.motif_classification.as_deref(),
            Some("Ticket sans suivi")
        );
    }

    #[test]
    fn test_classify_zombie_zero_suivi() {
        let config = default_config();
        let mut ticket = base_ticket();
        ticket.nombre_suivis = Some(0);
        classify_ticket(&mut ticket, &config);
        assert_eq!(ticket.action_recommandee.as_deref(), Some("qualifier"));
    }

    #[test]
    fn test_classify_ancien() {
        let config = default_config();
        let mut ticket = base_ticket();
        ticket.nombre_suivis = Some(3);
        ticket.anciennete_jours = Some(91); // > 90j
        ticket.inactivite_jours = Some(5);
        classify_ticket(&mut ticket, &config);
        assert_eq!(ticket.action_recommandee.as_deref(), Some("clôturer"));
        assert_eq!(
            ticket.motif_classification.as_deref(),
            Some("Ancienneté > 90j")
        );
    }

    #[test]
    fn test_classify_inactif() {
        let config = default_config();
        let mut ticket = base_ticket();
        ticket.nombre_suivis = Some(2);
        ticket.anciennete_jours = Some(20); // < 90j
        ticket.inactivite_jours = Some(15); // > 14j
        classify_ticket(&mut ticket, &config);
        assert_eq!(ticket.action_recommandee.as_deref(), Some("relancer"));
        assert_eq!(
            ticket.motif_classification.as_deref(),
            Some("Inactif depuis 15j")
        );
    }

    #[test]
    fn test_classify_normal() {
        let config = default_config();
        let mut ticket = base_ticket();
        ticket.nombre_suivis = Some(2);
        ticket.anciennete_jours = Some(10);
        ticket.inactivite_jours = Some(3);
        classify_ticket(&mut ticket, &config);
        assert_eq!(ticket.action_recommandee.as_deref(), Some("suivre"));
        assert_eq!(
            ticket.motif_classification.as_deref(),
            Some("En cours normal")
        );
    }

    #[test]
    fn test_classify_termine_non_modifie() {
        let config = default_config();
        let mut ticket = base_ticket();
        ticket.est_vivant = false;
        ticket.nombre_suivis = None; // serait zombie si vivant
        classify_ticket(&mut ticket, &config);
        assert!(ticket.action_recommandee.is_none());
        assert!(ticket.motif_classification.is_none());
    }

    #[test]
    fn test_classify_zombie_prioritaire_sur_ancien() {
        let config = default_config();
        let mut ticket = base_ticket();
        ticket.nombre_suivis = Some(0);
        ticket.anciennete_jours = Some(200); // serait aussi "ancien"
        classify_ticket(&mut ticket, &config);
        assert_eq!(ticket.action_recommandee.as_deref(), Some("qualifier")); // zombie gagne
    }

    #[test]
    fn test_poids_priorite() {
        assert_eq!(poids_priorite("Très haute"), 5);
        assert_eq!(poids_priorite("Haute"), 4);
        assert_eq!(poids_priorite("Majeure"), 4);
        assert_eq!(poids_priorite("Moyenne"), 3);
        assert_eq!(poids_priorite("Basse"), 2);
        assert_eq!(poids_priorite("Très basse"), 1);
        assert_eq!(poids_priorite("Inconnue"), 1);
    }
}
