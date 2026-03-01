use crate::commands::stock::{AgeRangeCount, GroupStock, StockOverview, TechnicianStock};
use crate::error::AppError;
use crate::export::{
    apply_rag_conditional_format, create_header_format, create_integer_format,
    create_number_format, create_percent_format,
};
use rust_xlsxwriter::{Workbook, XlsxError};

const SEUILS_RAG: (u32, u32, u32) = (10, 20, 40);

fn xlsx_err(e: XlsxError) -> AppError {
    AppError::Custom(e.to_string())
}

/// Génère le rapport stock Excel 3 onglets (RG-049, US020).
/// Retourne les bytes XLSX via workbook.save_to_buffer().
pub fn generate_stock_report(
    overview: &StockOverview,
    technicians: &[TechnicianStock],
    groups: &[GroupStock],
) -> Result<Vec<u8>, AppError> {
    let mut wb = Workbook::new();
    write_vue_globale(&mut wb, overview).map_err(xlsx_err)?;
    write_technicians(&mut wb, technicians).map_err(xlsx_err)?;
    write_groups(&mut wb, groups).map_err(xlsx_err)?;
    wb.save_to_buffer().map_err(xlsx_err)
}

// ── Onglet 1 : Vue globale ────────────────────────────────────────────────────

fn write_vue_globale(wb: &mut Workbook, o: &StockOverview) -> Result<(), XlsxError> {
    let ws = wb.add_worksheet();
    ws.set_name("Vue globale")?;

    let hdr = create_header_format();
    let num = create_number_format();
    let pct = create_percent_format();

    // Section KPI
    ws.write_with_format(0, 0, "Indicateur", &hdr)?;
    ws.write_with_format(0, 1, "Valeur", &hdr)?;

    let kpis: &[(&str, f64)] = &[
        ("Total vivants", o.total_vivants as f64),
        ("Total terminés", o.total_termines as f64),
        ("Âge moyen (j)", o.age_moyen_jours),
        ("Âge médian (j)", o.age_median_jours),
        ("Incidents", o.par_type.incidents as f64),
        ("Demandes", o.par_type.demandes as f64),
        ("Inactifs 14j", o.inactifs_14j as f64),
        ("Inactifs 30j", o.inactifs_30j as f64),
    ];
    for (i, (label, val)) in kpis.iter().enumerate() {
        let row = (i + 1) as u32;
        ws.write(row, 0, *label)?;
        ws.write_with_format(row, 1, *val, &num)?;
    }

    // Section distribution par tranches d'âge
    let header_row = (kpis.len() + 2) as u32;
    ws.write_with_format(header_row, 0, "Tranche d'âge", &hdr)?;
    ws.write_with_format(header_row, 1, "Nb tickets", &hdr)?;
    ws.write_with_format(header_row, 2, "%", &hdr)?;
    write_age_ranges(ws, header_row + 1, &o.par_anciennete, &num, &pct)?;

    ws.set_column_width(0, 22)?;
    ws.set_column_width(1, 14)?;
    ws.set_column_width(2, 10)?;

    Ok(())
}

fn write_age_ranges(
    ws: &mut rust_xlsxwriter::Worksheet,
    start_row: u32,
    ranges: &[AgeRangeCount],
    num: &rust_xlsxwriter::Format,
    pct: &rust_xlsxwriter::Format,
) -> Result<(), XlsxError> {
    for (i, r) in ranges.iter().enumerate() {
        let row = start_row + i as u32;
        ws.write(row, 0, r.label.as_str())?;
        ws.write_with_format(row, 1, r.count as f64, num)?;
        ws.write_with_format(row, 2, r.percentage / 100.0, pct)?;
    }
    Ok(())
}

// ── Onglet 2 : Techniciens ───────────────────────────────────────────────────

fn write_technicians(wb: &mut Workbook, technicians: &[TechnicianStock]) -> Result<(), XlsxError> {
    let ws = wb.add_worksheet();
    ws.set_name("Techniciens")?;

    let hdr = create_header_format();
    let int = create_integer_format();
    let num = create_number_format();

    // En-têtes (RG-052)
    let headers = [
        "Technicien",
        "Stock",
        "En cours",
        "En attente",
        "Incidents",
        "Demandes",
        "Âge moyen (j)",
        "Inactifs 14j",
        "Couleur seuil",
    ];
    for (col, h) in headers.iter().enumerate() {
        ws.write_with_format(0, col as u16, *h, &hdr)?;
    }

    // Données
    for (i, t) in technicians.iter().enumerate() {
        let row = (i + 1) as u32;
        ws.write(row, 0, t.technicien.as_str())?;
        ws.write_with_format(row, 1, t.total as f64, &int)?;
        ws.write_with_format(row, 2, t.en_cours as f64, &int)?;
        ws.write_with_format(row, 3, t.en_attente as f64, &int)?;
        ws.write_with_format(row, 4, t.incidents as f64, &int)?;
        ws.write_with_format(row, 5, t.demandes as f64, &int)?;
        ws.write_with_format(row, 6, t.age_moyen_jours, &num)?;
        ws.write_with_format(row, 7, t.inactifs_14j as f64, &int)?;
        ws.write(row, 8, t.couleur_seuil.as_str())?;
    }

    if !technicians.is_empty() {
        let last_row = technicians.len() as u32;

        // Freeze pane ligne 1 (RG-053)
        ws.set_freeze_panes(1, 0)?;

        // Auto-filtre (RG-053)
        ws.autofilter(0, 0, last_row, (headers.len() - 1) as u16)?;

        // Formatage conditionnel RAG sur colonne "Stock" (col 1) (RG-054)
        apply_rag_conditional_format(ws, 1, 1, last_row, SEUILS_RAG)?;
    }

    // Largeurs colonnes
    ws.set_column_width(0, 28)?;
    ws.set_column_width(1, 10)?;
    for col in 2u16..=7 {
        ws.set_column_width(col, 14)?;
    }
    ws.set_column_width(8, 14)?;

    Ok(())
}

// ── Onglet 3 : Groupes ───────────────────────────────────────────────────────

fn write_groups(wb: &mut Workbook, groups: &[GroupStock]) -> Result<(), XlsxError> {
    let ws = wb.add_worksheet();
    ws.set_name("Groupes")?;

    let hdr = create_header_format();
    let int = create_integer_format();
    let num = create_number_format();

    let headers = [
        "Groupe",
        "Niveau 1",
        "Niveau 2",
        "Stock",
        "En cours",
        "En attente",
        "Incidents",
        "Demandes",
        "Techniciens",
        "Âge moyen (j)",
    ];
    for (col, h) in headers.iter().enumerate() {
        ws.write_with_format(0, col as u16, *h, &hdr)?;
    }

    for (i, g) in groups.iter().enumerate() {
        let row = (i + 1) as u32;
        ws.write(row, 0, g.groupe.as_str())?;
        ws.write(row, 1, g.groupe_niveau1.as_str())?;
        ws.write(
            row,
            2,
            g.groupe_niveau2.as_deref().unwrap_or(""),
        )?;
        ws.write_with_format(row, 3, g.total as f64, &int)?;
        ws.write_with_format(row, 4, g.en_cours as f64, &int)?;
        ws.write_with_format(row, 5, g.en_attente as f64, &int)?;
        ws.write_with_format(row, 6, g.incidents as f64, &int)?;
        ws.write_with_format(row, 7, g.demandes as f64, &int)?;
        ws.write_with_format(row, 8, g.nb_techniciens as f64, &int)?;
        ws.write_with_format(row, 9, g.age_moyen_jours, &num)?;
    }

    if !groups.is_empty() {
        let last_row = groups.len() as u32;
        ws.set_freeze_panes(1, 0)?;
        ws.autofilter(0, 0, last_row, (headers.len() - 1) as u16)?;
    }

    ws.set_column_width(0, 30)?;
    ws.set_column_width(1, 18)?;
    ws.set_column_width(2, 18)?;
    for col in 3u16..=9 {
        ws.set_column_width(col, 14)?;
    }

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::stock::{AgeRangeCount, StatutCount, TypeBreakdown};

    fn make_overview() -> StockOverview {
        StockOverview {
            total_vivants: 100,
            total_termines: 50,
            par_statut: vec![
                StatutCount {
                    statut: "En cours".into(),
                    count: 60,
                    est_vivant: true,
                },
                StatutCount {
                    statut: "Résolu".into(),
                    count: 50,
                    est_vivant: false,
                },
            ],
            age_moyen_jours: 45.5,
            age_median_jours: 30.0,
            par_type: TypeBreakdown {
                incidents: 60,
                demandes: 40,
            },
            par_anciennete: vec![
                AgeRangeCount {
                    label: "0-30j".into(),
                    threshold_days: 30,
                    count: 40,
                    percentage: 40.0,
                },
                AgeRangeCount {
                    label: "31-90j".into(),
                    threshold_days: 90,
                    count: 40,
                    percentage: 40.0,
                },
                AgeRangeCount {
                    label: ">90j".into(),
                    threshold_days: 9999,
                    count: 20,
                    percentage: 20.0,
                },
            ],
            inactifs_14j: 15,
            inactifs_30j: 8,
        }
    }

    fn make_technician() -> TechnicianStock {
        TechnicianStock {
            technicien: "Alice Martin".into(),
            total: 25,
            en_cours: 15,
            en_attente: 10,
            planifie: 0,
            nouveau: 0,
            incidents: 15,
            demandes: 10,
            age_moyen_jours: 42.3,
            inactifs_14j: 3,
            ecart_seuil: 5,
            couleur_seuil: "orange".into(),
        }
    }

    fn make_group() -> GroupStock {
        GroupStock {
            groupe: "DSI > Support".into(),
            groupe_niveau1: "DSI".into(),
            groupe_niveau2: Some("Support".into()),
            total: 80,
            en_cours: 50,
            en_attente: 30,
            incidents: 50,
            demandes: 30,
            nb_techniciens: 5,
            age_moyen_jours: 38.0,
        }
    }

    #[test]
    fn test_generate_stock_report_xlsx_signature() {
        let overview = make_overview();
        let technicians = vec![make_technician()];
        let groups = vec![make_group()];
        let result = generate_stock_report(&overview, &technicians, &groups);
        assert!(result.is_ok(), "generate_stock_report failed: {:?}", result.err());
        let bytes = result.unwrap();
        assert!(bytes.len() > 4, "XLSX too small");
        // ZIP magic bytes PK (0x50 0x4B)
        assert_eq!(bytes[0], 0x50, "Expected PK signature byte 0");
        assert_eq!(bytes[1], 0x4B, "Expected PK signature byte 1");
    }

    #[test]
    fn test_generate_stock_report_empty_slices() {
        let overview = make_overview();
        let result = generate_stock_report(&overview, &[], &[]);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert_eq!(bytes[0], 0x50);
        assert_eq!(bytes[1], 0x4B);
    }
}
