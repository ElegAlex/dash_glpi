use crate::commands::bilan::{BilanRequest, BilanTemporel};
use crate::error::AppError;
use crate::export::{create_header_format, create_integer_format, create_number_format};
use rust_xlsxwriter::{Workbook, XlsxError};

fn xlsx_err(e: XlsxError) -> AppError {
    AppError::Custom(e.to_string())
}

/// Génère le rapport bilan d'activité Excel, 3 onglets (RG-049, US022):
/// - "Volume"     : tableau entrants/sortants/delta/stock cumulé par période
/// - "Tendance"   : KPI globaux + série temporelle pour graphique côté frontend
/// - "Ventilation": répartition par technicien/groupe (si disponible)
pub fn generate_bilan_report(
    bilan: &BilanTemporel,
    _request: &BilanRequest,
) -> Result<Vec<u8>, AppError> {
    let mut wb = Workbook::new();
    write_volume(&mut wb, bilan).map_err(xlsx_err)?;
    write_tendance(&mut wb, bilan).map_err(xlsx_err)?;
    write_ventilation(&mut wb, bilan).map_err(xlsx_err)?;
    wb.save_to_buffer().map_err(xlsx_err)
}

// ── Onglet 1 : Volume ────────────────────────────────────────────────────────

fn write_volume(wb: &mut Workbook, bilan: &BilanTemporel) -> Result<(), XlsxError> {
    let ws = wb.add_worksheet();
    ws.set_name("Volume")?;

    let hdr = create_header_format();
    let int = create_integer_format();

    let headers = ["Période", "Entrants", "Sortants", "Delta", "Stock cumulé"];
    for (col, h) in headers.iter().enumerate() {
        ws.write_with_format(0, col as u16, *h, &hdr)?;
    }

    for (i, p) in bilan.periodes.iter().enumerate() {
        let row = (i + 1) as u32;
        ws.write(row, 0, p.period_label.as_str())?;
        ws.write_with_format(row, 1, p.entrees as f64, &int)?;
        ws.write_with_format(row, 2, p.sorties as f64, &int)?;
        ws.write_with_format(row, 3, p.delta as f64, &int)?;
        if let Some(sc) = p.stock_cumule {
            ws.write_with_format(row, 4, sc as f64, &int)?;
        }
    }

    // Totaux
    if !bilan.periodes.is_empty() {
        let last_data_row = bilan.periodes.len() as u32;
        let total_row = last_data_row + 2;

        ws.write_with_format(total_row, 0, "TOTAL", &hdr)?;
        ws.write_with_format(total_row, 1, bilan.totaux.total_entrees as f64, &int)?;
        ws.write_with_format(total_row, 2, bilan.totaux.total_sorties as f64, &int)?;
        ws.write_with_format(total_row, 3, bilan.totaux.delta_global as f64, &int)?;

        // Freeze pane + auto-filter (RG-053)
        ws.set_freeze_panes(1, 0)?;
        ws.autofilter(0, 0, last_data_row, (headers.len() - 1) as u16)?;
    }

    ws.set_column_width(0, 18)?;
    for col in 1u16..=4 {
        ws.set_column_width(col, 14)?;
    }

    Ok(())
}

// ── Onglet 2 : Tendance ──────────────────────────────────────────────────────

fn write_tendance(wb: &mut Workbook, bilan: &BilanTemporel) -> Result<(), XlsxError> {
    let ws = wb.add_worksheet();
    ws.set_name("Tendance")?;

    let hdr = create_header_format();
    let int = create_integer_format();
    let num = create_number_format();

    // KPI globaux
    ws.write_with_format(0, 0, "Indicateur", &hdr)?;
    ws.write_with_format(0, 1, "Valeur", &hdr)?;

    let kpis: &[(&str, f64, bool)] = &[
        ("Total entrants", bilan.totaux.total_entrees as f64, true),
        ("Total sortants", bilan.totaux.total_sorties as f64, true),
        ("Delta global", bilan.totaux.delta_global as f64, true),
        ("Moy. entrants/période", bilan.totaux.moyenne_entrees_par_periode, false),
        ("Moy. sortants/période", bilan.totaux.moyenne_sorties_par_periode, false),
    ];

    for (i, (label, val, is_int)) in kpis.iter().enumerate() {
        let row = (i + 1) as u32;
        ws.write(row, 0, *label)?;
        if *is_int {
            ws.write_with_format(row, 1, *val, &int)?;
        } else {
            ws.write_with_format(row, 1, *val, &num)?;
        }
    }

    // Série temporelle (données pour graphique côté frontend)
    let series_row = (kpis.len() + 2) as u32;
    let series_headers = ["Période", "Entrants", "Sortants", "Delta"];
    for (col, h) in series_headers.iter().enumerate() {
        ws.write_with_format(series_row, col as u16, *h, &hdr)?;
    }

    for (i, p) in bilan.periodes.iter().enumerate() {
        let row = series_row + 1 + i as u32;
        ws.write(row, 0, p.period_label.as_str())?;
        ws.write_with_format(row, 1, p.entrees as f64, &int)?;
        ws.write_with_format(row, 2, p.sorties as f64, &int)?;
        ws.write_with_format(row, 3, p.delta as f64, &int)?;
    }

    ws.set_column_width(0, 24)?;
    ws.set_column_width(1, 18)?;

    Ok(())
}

// ── Onglet 3 : Ventilation ───────────────────────────────────────────────────

fn write_ventilation(wb: &mut Workbook, bilan: &BilanTemporel) -> Result<(), XlsxError> {
    let ws = wb.add_worksheet();
    ws.set_name("Ventilation")?;

    let hdr = create_header_format();
    let int = create_integer_format();

    let headers = ["Technicien / Groupe", "Entrants", "Sortants", "Delta"];
    for (col, h) in headers.iter().enumerate() {
        ws.write_with_format(0, col as u16, *h, &hdr)?;
    }

    if let Some(ref vent) = bilan.ventilation {
        for (i, v) in vent.iter().enumerate() {
            let row = (i + 1) as u32;
            ws.write(row, 0, v.label.as_str())?;
            ws.write_with_format(row, 1, v.entrees as f64, &int)?;
            ws.write_with_format(row, 2, v.sorties as f64, &int)?;
            ws.write_with_format(row, 3, v.delta as f64, &int)?;
        }

        if !vent.is_empty() {
            let last_row = vent.len() as u32;
            ws.set_freeze_panes(1, 0)?;
            ws.autofilter(0, 0, last_row, (headers.len() - 1) as u16)?;
        }
    }

    ws.set_column_width(0, 28)?;
    for col in 1u16..=3 {
        ws.set_column_width(col, 14)?;
    }

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::bilan::{BilanTotaux, BilanVentilation, PeriodData};

    fn make_bilan(with_ventilation: bool) -> BilanTemporel {
        let periodes = vec![
            PeriodData {
                period_key: "2026-01".into(),
                period_label: "Janvier 2026".into(),
                entrees: 10,
                sorties: 5,
                delta: 5,
                stock_cumule: Some(105),
            },
            PeriodData {
                period_key: "2026-02".into(),
                period_label: "Février 2026".into(),
                entrees: 8,
                sorties: 12,
                delta: -4,
                stock_cumule: Some(101),
            },
        ];

        let ventilation = if with_ventilation {
            Some(vec![
                BilanVentilation {
                    label: "Alice".into(),
                    entrees: 10,
                    sorties: 8,
                    delta: 2,
                },
                BilanVentilation {
                    label: "Bob".into(),
                    entrees: 8,
                    sorties: 9,
                    delta: -1,
                },
            ])
        } else {
            None
        };

        BilanTemporel {
            periodes,
            totaux: BilanTotaux {
                total_entrees: 18,
                total_sorties: 17,
                delta_global: 1,
                moyenne_entrees_par_periode: 9.0,
                moyenne_sorties_par_periode: 8.5,
            },
            ventilation,
        }
    }

    fn make_request() -> BilanRequest {
        BilanRequest {
            period: "month".into(),
            date_from: "2026-01-01".into(),
            date_to: "2026-02-28".into(),
            group_by: None,
        }
    }

    #[test]
    fn test_generate_bilan_report_xlsx_signature() {
        let bilan = make_bilan(false);
        let request = make_request();
        let result = generate_bilan_report(&bilan, &request);
        assert!(result.is_ok(), "generate_bilan_report failed: {:?}", result.err());
        let bytes = result.unwrap();
        assert!(bytes.len() > 4, "XLSX too small");
        assert_eq!(bytes[0], 0x50, "Expected PK byte 0");
        assert_eq!(bytes[1], 0x4B, "Expected PK byte 1");
    }

    #[test]
    fn test_generate_bilan_report_with_ventilation() {
        let bilan = make_bilan(true);
        let request = make_request();
        let result = generate_bilan_report(&bilan, &request);
        assert!(result.is_ok(), "generate_bilan_report with ventilation failed: {:?}", result.err());
        let bytes = result.unwrap();
        assert_eq!(bytes[0], 0x50);
        assert_eq!(bytes[1], 0x4B);
    }

    #[test]
    fn test_generate_bilan_report_empty_periods() {
        let bilan = BilanTemporel {
            periodes: vec![],
            totaux: BilanTotaux {
                total_entrees: 0,
                total_sorties: 0,
                delta_global: 0,
                moyenne_entrees_par_periode: 0.0,
                moyenne_sorties_par_periode: 0.0,
            },
            ventilation: None,
        };
        let request = make_request();
        let result = generate_bilan_report(&bilan, &request);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert_eq!(bytes[0], 0x50);
        assert_eq!(bytes[1], 0x4B);
    }
}
