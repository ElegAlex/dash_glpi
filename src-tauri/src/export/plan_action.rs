use crate::commands::stock::{TechnicianStock, TicketSummary};
use crate::error::AppError;
use crate::export::{
    apply_rag_conditional_format, create_header_format, create_integer_format, create_number_format,
};
use rust_xlsxwriter::{Format, Workbook, XlsxError};

const SEUILS_RAG: (u32, u32, u32) = (10, 20, 40);
const SEUIL_ANCIENNETE_ROUGE: u32 = 90;

fn xlsx_err(e: XlsxError) -> AppError {
    AppError::Custom(e.to_string())
}

/// Génère le plan d'action individuel technicien, 3 onglets (RG-055, US021).
/// Retourne les bytes XLSX.
pub fn generate_plan_action(
    technician: &str,
    stats: &TechnicianStock,
    tickets: &[TicketSummary],
) -> Result<Vec<u8>, AppError> {
    let mut wb = Workbook::new();
    write_entretien(&mut wb, technician, stats).map_err(xlsx_err)?;
    write_detail_tickets(&mut wb, tickets).map_err(xlsx_err)?;
    write_checklist(&mut wb, tickets).map_err(xlsx_err)?;
    wb.save_to_buffer().map_err(xlsx_err)
}

// ── Onglet 1 : Entretien ─────────────────────────────────────────────────────

fn write_entretien(
    wb: &mut Workbook,
    technician: &str,
    s: &TechnicianStock,
) -> Result<(), XlsxError> {
    let ws = wb.add_worksheet();
    ws.set_name("Entretien")?;

    let hdr = create_header_format();
    let num = create_number_format();
    let int = create_integer_format();

    ws.write_with_format(0, 0, "Indicateur", &hdr)?;
    ws.write_with_format(0, 1, "Valeur", &hdr)?;

    // Row 1: technicien name
    ws.write(1, 0, "Technicien")?;
    ws.write(1, 1, technician)?;

    // Row 2-6: numeric KPIs
    let numeric_kpis: &[(&str, f64, bool)] = &[
        ("Stock total", s.total as f64, true),
        ("Incidents", s.incidents as f64, true),
        ("Demandes", s.demandes as f64, true),
        ("Inactifs 14j", s.inactifs_14j as f64, true),
        ("Âge moyen (j)", s.age_moyen_jours, false),
    ];
    for (i, (label, val, is_int)) in numeric_kpis.iter().enumerate() {
        let row = (i + 2) as u32;
        ws.write(row, 0, *label)?;
        if *is_int {
            ws.write_with_format(row, 1, *val, &int)?;
        } else {
            ws.write_with_format(row, 1, *val, &num)?;
        }
    }

    // Row 8: couleur seuil (texte)
    ws.write(8, 0, "Couleur seuil")?;
    ws.write(8, 1, s.couleur_seuil.as_str())?;

    // RAG sur la cellule Stock total (ligne 2, col 1)
    apply_rag_conditional_format(ws, 2, 1, 2, SEUILS_RAG)?;

    ws.set_column_width(0, 22)?;
    ws.set_column_width(1, 18)?;

    Ok(())
}

// ── Onglet 2 : Détail tickets ─────────────────────────────────────────────────

fn write_detail_tickets(wb: &mut Workbook, tickets: &[TicketSummary]) -> Result<(), XlsxError> {
    let ws = wb.add_worksheet();
    ws.set_name("Détail tickets")?;

    let hdr = create_header_format();
    let int = create_integer_format();

    let headers = [
        "ID",
        "Titre",
        "Statut",
        "Type",
        "Date ouverture",
        "Ancienneté (j)",
        "Inactivité (j)",
        "Nb suivis",
        "Action recommandée",
    ];
    for (col, h) in headers.iter().enumerate() {
        ws.write_with_format(0, col as u16, *h, &hdr)?;
    }

    // Trier par ancienneté décroissante (RG-054 — tickets >90j visibles en haut)
    let mut sorted: Vec<&TicketSummary> = tickets.iter().collect();
    sorted.sort_by(|a, b| {
        b.anciennete_jours
            .unwrap_or(0)
            .cmp(&a.anciennete_jours.unwrap_or(0))
    });

    for (i, t) in sorted.iter().enumerate() {
        let row = (i + 1) as u32;
        ws.write(row, 0, t.id as f64)?;
        ws.write(row, 1, t.titre.as_str())?;
        ws.write(row, 2, t.statut.as_str())?;
        ws.write(row, 3, t.type_ticket.as_str())?;
        ws.write(row, 4, t.date_ouverture.as_str())?;
        match t.anciennete_jours {
            Some(v) => {
                ws.write_with_format(row, 5, v as f64, &int)?;
            }
            None => {
                ws.write(row, 5, "")?;
            }
        }
        match t.inactivite_jours {
            Some(v) => {
                ws.write_with_format(row, 6, v as f64, &int)?;
            }
            None => {
                ws.write(row, 6, "")?;
            }
        }
        match t.nombre_suivis {
            Some(v) => {
                ws.write_with_format(row, 7, v as f64, &int)?;
            }
            None => {
                ws.write(row, 7, "")?;
            }
        }
        ws.write(
            row,
            8,
            t.action_recommandee.as_deref().unwrap_or(""),
        )?;
    }

    if !sorted.is_empty() {
        let last_row = sorted.len() as u32;

        // Freeze pane ligne 1 (RG-053)
        ws.set_freeze_panes(1, 0)?;

        // Auto-filtre
        ws.autofilter(0, 0, last_row, (headers.len() - 1) as u16)?;

        // Surbrillance rouge sur ancienneté > 90j (col 5) (RG-054)
        use rust_xlsxwriter::{ConditionalFormatCell, ConditionalFormatCellRule};
        let red = Format::new()
            .set_background_color("FFC7CE")
            .set_font_color("9C0006");
        ws.add_conditional_format(
            1,
            5,
            last_row,
            5,
            &ConditionalFormatCell::new()
                .set_rule(ConditionalFormatCellRule::GreaterThan(
                    SEUIL_ANCIENNETE_ROUGE as f64,
                ))
                .set_format(&red),
        )?;
    }

    ws.set_column_width(0, 8)?;
    ws.set_column_width(1, 45)?;
    ws.set_column_width(2, 14)?;
    ws.set_column_width(3, 12)?;
    ws.set_column_width(4, 14)?;
    ws.set_column_width(5, 14)?;
    ws.set_column_width(6, 14)?;
    ws.set_column_width(7, 10)?;
    ws.set_column_width(8, 25)?;

    Ok(())
}

// ── Onglet 3 : Checklist ─────────────────────────────────────────────────────

fn write_checklist(wb: &mut Workbook, tickets: &[TicketSummary]) -> Result<(), XlsxError> {
    let ws = wb.add_worksheet();
    ws.set_name("Checklist")?;

    let hdr = create_header_format();
    let int = create_integer_format();

    // En-têtes
    ws.write_with_format(0, 0, "Action recommandée", &hdr)?;
    ws.write_with_format(0, 1, "Nb tickets", &hdr)?;
    ws.write_with_format(0, 2, "IDs tickets", &hdr)?;

    // Grouper par action_recommandee (ordre défini)
    let action_order = ["qualifier", "clôturer", "relancer", "suivre"];
    let mut row = 1u32;

    for action in &action_order {
        let matching: Vec<&TicketSummary> = tickets
            .iter()
            .filter(|t| {
                t.action_recommandee
                    .as_deref()
                    .map(|a| a.to_lowercase().contains(action))
                    .unwrap_or(false)
            })
            .collect();

        if matching.is_empty() {
            continue;
        }

        let ids: Vec<String> = matching.iter().map(|t| t.id.to_string()).collect();
        let ids_str = ids.join(", ");

        ws.write(row, 0, *action)?;
        ws.write_with_format(row, 1, matching.len() as f64, &int)?;
        ws.write(row, 2, ids_str.as_str())?;
        row += 1;
    }

    // Tickets sans classification
    let unclassified: Vec<&TicketSummary> = tickets
        .iter()
        .filter(|t| t.action_recommandee.is_none())
        .collect();
    if !unclassified.is_empty() {
        let ids: Vec<String> = unclassified.iter().map(|t| t.id.to_string()).collect();
        ws.write(row, 0, "Non classé")?;
        ws.write_with_format(row, 1, unclassified.len() as f64, &int)?;
        ws.write(row, 2, ids.join(", ").as_str())?;
    }

    ws.set_freeze_panes(1, 0)?;
    ws.set_column_width(0, 22)?;
    ws.set_column_width(1, 12)?;
    ws.set_column_width(2, 60)?;

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stats() -> TechnicianStock {
        TechnicianStock {
            technicien: "Bob Dupont".into(),
            total: 12,
            en_cours: 8,
            en_attente: 4,
            planifie: 0,
            nouveau: 0,
            incidents: 7,
            demandes: 5,
            age_moyen_jours: 55.2,
            inactifs_14j: 2,
            ecart_seuil: 2,
            couleur_seuil: "jaune".into(),
        }
    }

    fn make_ticket(id: u64, anciennete: i64, action: Option<&str>) -> TicketSummary {
        TicketSummary {
            id,
            titre: format!("Ticket {id}"),
            statut: "En cours".into(),
            type_ticket: "Incident".into(),
            technicien_principal: Some("Bob Dupont".into()),
            groupe_principal: Some("DSI".into()),
            date_ouverture: "2025-10-01".into(),
            derniere_modification: None,
            anciennete_jours: Some(anciennete),
            inactivite_jours: Some(anciennete / 2),
            nombre_suivis: Some(3),
            action_recommandee: action.map(String::from),
            motif_classification: None,
        }
    }

    #[test]
    fn test_generate_plan_action_xlsx_signature() {
        let stats = make_stats();
        let tickets = vec![
            make_ticket(101, 95, Some("clôturer")),
            make_ticket(102, 45, Some("relancer")),
            make_ticket(103, 15, None),
        ];
        let result = generate_plan_action("Bob Dupont", &stats, &tickets);
        assert!(
            result.is_ok(),
            "generate_plan_action failed: {:?}",
            result.err()
        );
        let bytes = result.unwrap();
        assert!(bytes.len() > 4, "XLSX too small");
        assert_eq!(bytes[0], 0x50, "Expected PK byte 0");
        assert_eq!(bytes[1], 0x4B, "Expected PK byte 1");
    }

    #[test]
    fn test_generate_plan_action_empty_tickets() {
        let stats = make_stats();
        let result = generate_plan_action("Bob Dupont", &stats, &[]);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert_eq!(bytes[0], 0x50);
        assert_eq!(bytes[1], 0x4B);
    }
}
