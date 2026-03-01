use rusqlite::Connection;

use crate::parser::types::GlpiTicketNormalized;

pub fn bulk_insert_tickets(
    conn: &mut Connection,
    import_id: i64,
    tickets: &[GlpiTicketNormalized],
) -> Result<usize, rusqlite::Error> {
    let tx = conn.transaction()?;

    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR REPLACE INTO tickets (
                id, import_id, titre, statut, type_ticket, priorite, priorite_label, urgence,
                demandeur, date_ouverture, derniere_modification, nombre_suivis,
                suivis_description, solution, taches_description, intervention_fournisseur,
                techniciens, groupes,
                technicien_principal, groupe_principal,
                groupe_niveau1, groupe_niveau2, groupe_niveau3,
                categorie, categorie_niveau1, categorie_niveau2,
                est_vivant, anciennete_jours, inactivite_jours, date_cloture_approx,
                action_recommandee, motif_classification
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16,
                ?17, ?18,
                ?19, ?20,
                ?21, ?22, ?23,
                ?24, ?25, ?26,
                ?27, ?28, ?29, ?30,
                ?31, ?32
            )",
        )?;

        for t in tickets {
            stmt.execute(rusqlite::params![
                t.id,
                import_id,
                t.titre,
                t.statut,
                t.type_ticket,
                t.priorite,
                t.priorite_label,
                t.urgence,
                t.demandeur,
                t.date_ouverture,
                t.derniere_modification,
                t.nombre_suivis,
                t.suivis_description,
                t.solution,
                t.taches_description,
                t.intervention_fournisseur,
                serde_json::to_string(&t.techniciens).unwrap_or_default(),
                serde_json::to_string(&t.groupes).unwrap_or_default(),
                t.technicien_principal,
                t.groupe_principal,
                t.groupe_niveau1,
                t.groupe_niveau2,
                t.groupe_niveau3,
                t.categorie,
                t.categorie_niveau1,
                t.categorie_niveau2,
                t.est_vivant as i32,
                t.anciennete_jours,
                t.inactivite_jours,
                t.date_cloture_approx,
                t.action_recommandee,
                t.motif_classification,
            ])?;
        }
    }

    tx.commit()?;
    Ok(tickets.len())
}
