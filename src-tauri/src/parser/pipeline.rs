use std::collections::HashSet;
use std::io::Read;
use std::time::Instant;

use chrono::Utc;

use crate::error::AppError;
use crate::parser::columns::{validate_columns, ColumnMap};
use crate::parser::deserializers::{parse_french_datetime, parse_opt_i32, parse_spaced_i64};
use crate::parser::types::{GlpiTicketNormalized, GlpiTicketRaw, ParseWarning};

/// Statuts indiquant un ticket encore actif (vivant).
const VIVANTS: &[&str] = &[
    "Nouveau",
    "En cours (Attribué)",
    "En cours (Planifié)",
    "En attente",
];

/// Output of `parse_csv` — carries normalized tickets and import metadata.
/// Used by `commands::import` to persist tickets in SQLite and build `CsvImportResult`.
#[derive(Debug)]
pub struct ParseOutput {
    pub tickets: Vec<GlpiTicketNormalized>,
    pub warnings: Vec<ParseWarning>,
    pub total_rows_processed: usize,
    pub skipped_rows: usize,
    pub detected_columns: Vec<String>,
    pub missing_optional_columns: Vec<String>,
    pub unique_statuts: Vec<String>,
    pub unique_types: Vec<String>,
    pub unique_groupes: Vec<String>,
    pub parse_duration_ms: u64,
}

/// Parse a GLPI CSV file from `path`.
/// `progress_cb(rows_processed, tickets_accepted)` is called every 500 rows.
pub fn parse_csv(
    path: &str,
    progress_cb: impl Fn(usize, usize),
) -> Result<ParseOutput, AppError> {
    let file = std::fs::File::open(path)?;
    parse_csv_reader(std::io::BufReader::new(file), progress_cb)
}

/// Core parsing logic — accepts any `Read` source, useful for tests.
pub fn parse_csv_reader<R: Read>(
    reader: R,
    progress_cb: impl Fn(usize, usize),
) -> Result<ParseOutput, AppError> {
    let start = Instant::now();

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(true)
        .flexible(true)
        .trim(csv::Trim::Headers)
        .double_quote(true)
        .quoting(true)
        .from_reader(reader);

    // Phase 1: validate columns
    let headers = rdr.headers()?.clone();
    if headers.is_empty() {
        return Err(AppError::EmptyFile);
    }
    let col_map = ColumnMap::from_headers(&headers);
    let col_validation = validate_columns(&col_map)?;

    // Phase 2: parse and normalise records
    let now = Utc::now().naive_utc();
    let mut tickets: Vec<GlpiTicketNormalized> = Vec::with_capacity(10_000);
    let mut warnings: Vec<ParseWarning> = Vec::new();
    let mut skipped = 0usize;
    let mut row_idx = 0usize;

    let mut unique_statuts: HashSet<String> = HashSet::new();
    let mut unique_types: HashSet<String> = HashSet::new();
    let mut unique_groupes: HashSet<String> = HashSet::new();

    for result in rdr.records() {
        row_idx += 1;
        if row_idx % 500 == 0 {
            progress_cb(row_idx, tickets.len());
        }

        match result {
            Ok(record) => {
                let raw = record_to_raw(&col_map, &record);
                match normalize_ticket(&raw, &now) {
                    Ok(normalized) => {
                        unique_statuts.insert(normalized.statut.clone());
                        unique_types.insert(normalized.type_ticket.clone());
                        for g in &normalized.groupes {
                            unique_groupes.insert(g.clone());
                        }
                        tickets.push(normalized);
                    }
                    Err(msg) => {
                        warnings.push(ParseWarning {
                            line: row_idx + 1, // +1 for the header row
                            message: msg,
                        });
                        skipped += 1;
                    }
                }
            }
            Err(err) => {
                warnings.push(ParseWarning {
                    line: row_idx + 1,
                    message: err.to_string(),
                });
                skipped += 1;
            }
        }
    }

    if row_idx == 0 {
        return Err(AppError::EmptyFile);
    }

    let mut unique_statuts: Vec<String> = unique_statuts.into_iter().collect();
    unique_statuts.sort();
    let mut unique_types: Vec<String> = unique_types.into_iter().collect();
    unique_types.sort();
    let mut unique_groupes: Vec<String> = unique_groupes.into_iter().collect();
    unique_groupes.sort();

    Ok(ParseOutput {
        tickets,
        warnings,
        total_rows_processed: row_idx,
        skipped_rows: skipped,
        detected_columns: col_validation.present,
        missing_optional_columns: col_validation.missing_optional,
        unique_statuts,
        unique_types,
        unique_groupes,
        parse_duration_ms: start.elapsed().as_millis() as u64,
    })
}

fn record_to_raw(col_map: &ColumnMap, record: &csv::StringRecord) -> GlpiTicketRaw {
    GlpiTicketRaw {
        id: col_map.get(record, "ID").map(str::to_string),
        titre: col_map.get(record, "Titre").map(str::to_string),
        statut: col_map.get(record, "Statut").map(str::to_string),
        type_ticket: col_map.get(record, "Type").map(str::to_string),
        priorite: col_map.get(record, "Priorité").map(str::to_string),
        urgence: col_map.get(record, "Urgence").map(str::to_string),
        demandeur: col_map
            .get(record, "Demandeur - Demandeur")
            .map(str::to_string),
        date_ouverture: col_map
            .get(record, "Date d'ouverture")
            .map(str::to_string),
        derniere_modification: col_map
            .get(record, "Dernière modification")
            .map(str::to_string),
        nombre_suivis: col_map
            .get(record, "Suivis - Nombre de suivis")
            .map(str::to_string),
        suivis_description: col_map
            .get(record, "Suivis - Description")
            .map(str::to_string),
        solution: col_map
            .get(record, "Solution - Solution")
            .map(str::to_string),
        taches_description: col_map
            .get(record, "Tâches - Description")
            .map(str::to_string),
        intervention_fournisseur: col_map
            .get(
                record,
                "Plugins - Intervention fourniseur : Intervention",
            )
            .map(str::to_string),
        technicien: col_map
            .get(record, "Attribué à - Technicien")
            .map(str::to_string),
        groupe: col_map
            .get(record, "Attribué à - Groupe de techniciens")
            .map(str::to_string),
        categorie: col_map.get(record, "Catégorie").map(str::to_string),
    }
}

fn normalize_ticket(
    raw: &GlpiTicketRaw,
    now: &chrono::NaiveDateTime,
) -> Result<GlpiTicketNormalized, String> {
    // ID (required)
    let id_str = raw.id.as_deref().unwrap_or("").trim().to_string();
    let id =
        parse_spaced_i64(&id_str).ok_or_else(|| format!("ID invalide: {:?}", id_str))?;

    // Date d'ouverture (required)
    let ouverture_str = raw.date_ouverture.as_deref().unwrap_or("");
    let ouverture_dt = parse_french_datetime(ouverture_str)
        .ok_or_else(|| format!("Date d'ouverture invalide: {:?}", ouverture_str))?;
    let date_ouverture = ouverture_dt.format("%Y-%m-%dT%H:%M:%S").to_string();

    // Statut (required)
    let statut = raw.statut.as_deref().unwrap_or("").trim().to_string();
    if statut.is_empty() {
        return Err("Statut manquant".to_string());
    }

    // Dernière modification (optional)
    let derniere_dt = raw
        .derniere_modification
        .as_deref()
        .and_then(parse_french_datetime);
    let derniere_modification =
        derniere_dt.map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string());

    // Computed fields
    let est_vivant = VIVANTS.contains(&statut.as_str());
    let anciennete_jours = Some((*now - ouverture_dt).num_days());
    let inactivite_jours = derniere_dt.map(|dt| (*now - dt).num_days());
    let date_cloture_approx = if !est_vivant {
        derniere_modification.clone()
    } else {
        None
    };

    // Techniciens (multiligne)
    let techniciens: Vec<String> = raw
        .technicien
        .as_deref()
        .unwrap_or("")
        .split('\n')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let technicien_principal = techniciens.first().cloned();

    // Groupes (multiligne)
    let groupes: Vec<String> = raw
        .groupe
        .as_deref()
        .unwrap_or("")
        .split('\n')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let groupe_principal = groupes.first().cloned();

    // Groupe niveaux (split " > ")
    let g_parts: Vec<&str> = groupe_principal
        .as_deref()
        .unwrap_or("")
        .split(" > ")
        .collect();
    let groupe_niveau1 = g_parts
        .first()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let groupe_niveau2 = g_parts
        .get(1)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let groupe_niveau3 = g_parts
        .get(2)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // Catégorie
    let categorie = raw
        .categorie
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());

    let c_parts: Vec<&str> = categorie.as_deref().unwrap_or("").split(" > ").collect();
    let categorie_niveau1 = c_parts
        .first()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let categorie_niveau2 = c_parts
        .get(1)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    Ok(GlpiTicketNormalized {
        id,
        titre: raw.titre.as_deref().unwrap_or("").trim().to_string(),
        statut,
        type_ticket: raw.type_ticket.as_deref().unwrap_or("").trim().to_string(),
        priorite: raw.priorite.as_deref().and_then(parse_opt_i32),
        priorite_label: raw.priorite.as_deref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        urgence: raw.urgence.as_deref().and_then(parse_opt_i32),
        demandeur: raw.demandeur.as_deref().unwrap_or("").trim().to_string(),
        date_ouverture,
        derniere_modification,
        nombre_suivis: raw.nombre_suivis.as_deref().and_then(parse_opt_i32),
        suivis_description: raw
            .suivis_description
            .as_deref()
            .unwrap_or("")
            .to_string(),
        solution: raw.solution.as_deref().unwrap_or("").to_string(),
        taches_description: raw
            .taches_description
            .as_deref()
            .unwrap_or("")
            .to_string(),
        intervention_fournisseur: raw
            .intervention_fournisseur
            .as_deref()
            .unwrap_or("")
            .to_string(),
        techniciens,
        groupes,
        technicien_principal,
        groupe_principal,
        groupe_niveau1,
        groupe_niveau2,
        groupe_niveau3,
        categorie,
        categorie_niveau1,
        categorie_niveau2,
        est_vivant,
        anciennete_jours,
        inactivite_jours,
        date_cloture_approx,
        action_recommandee: None,
        motif_classification: None,
    })
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal required headers for inline test CSV.
    const HDR: &str = concat!(
        "ID;Titre;Attribué à - Groupe de techniciens;Statut;",
        "Attribué à - Technicien;Demandeur - Demandeur;Date d'ouverture;",
        "Type;Suivis - Description;Suivis - Nombre de suivis;",
        "Plugins - Intervention fourniseur : Intervention;Solution - Solution;",
        "Priorité;Tâches - Description;Urgence;Dernière modification"
    );

    fn parse(csv: &str) -> ParseOutput {
        parse_csv_reader(csv.as_bytes(), |_, _| {}).unwrap()
    }

    fn parse_err(csv: &str) -> AppError {
        parse_csv_reader(csv.as_bytes(), |_, _| {}).unwrap_err()
    }

    // ── US002 / RG-004 : IDs with spaces ────────────────────────────────────

    #[test]
    fn test_spaced_id() {
        let csv = format!(
            "{HDR}\n5 732 943;Titre;G;Nouveau;T;D;05-01-2026 16:24;Incident;;3;;Sol;3;;4;06-01-2026 09:00"
        );
        let out = parse(&csv);
        assert_eq!(out.tickets.len(), 1);
        assert_eq!(out.tickets[0].id, 5_732_943);
    }

    // ── US002 / RG-005 : French dates ────────────────────────────────────────

    #[test]
    fn test_french_date_parsing() {
        let csv = format!(
            "{HDR}\n1;T;G;Nouveau;T;D;05-01-2026 16:24;Incident;;0;;;;3;;4;"
        );
        let out = parse(&csv);
        assert_eq!(out.tickets[0].date_ouverture, "2026-01-05T16:24:00");
    }

    // ── US002 / RG-002 : BOM UTF-8 ───────────────────────────────────────────

    #[test]
    fn test_bom_utf8() {
        // prepend UTF-8 BOM (\xEF\xBB\xBF)
        let csv = format!("\u{FEFF}{HDR}\n1;T;G;Nouveau;T;D;01-01-2026 08:00;Incident;;0;;;;3;;4;");
        let out = parse(&csv);
        assert_eq!(out.tickets.len(), 1, "BOM doit être ignoré");
    }

    // ── US002 / RG-003 + RG-010/011 : multiline fields ───────────────────────

    #[test]
    fn test_multiline_techniciens_groupes() {
        let csv = format!(
            "{HDR}\n\"100\";Test;\"_DSI > _SUPPORT\n_DSI > _PRODUCTION\";En cours (Attribué);\"BLANQUART CHRISTOPHE\nMEY CHETHARITH\";DEM;05-01-2026 10:00;Demande;;0;;;;3;;4;"
        );
        let out = parse(&csv);
        assert_eq!(out.tickets.len(), 1);
        let t = &out.tickets[0];
        assert_eq!(t.techniciens.len(), 2);
        assert_eq!(t.techniciens[0], "BLANQUART CHRISTOPHE");
        assert_eq!(t.techniciens[1], "MEY CHETHARITH");
        assert_eq!(t.groupes.len(), 2);
        assert_eq!(t.technicien_principal, Some("BLANQUART CHRISTOPHE".into()));
        assert_eq!(t.groupe_principal, Some("_DSI > _SUPPORT".into()));
    }

    // ── US002 / RG-007 : optional Catégorie column absent ────────────────────

    #[test]
    fn test_categorie_column_absent() {
        let csv = format!("{HDR}\n1;T;G;Nouveau;T;D;01-01-2026 08:00;Inc;;0;;;;3;;4;");
        let out = parse(&csv);
        assert!(out.tickets[0].categorie.is_none());
        assert!(
            out.missing_optional_columns
                .contains(&"Catégorie".to_string()),
            "Catégorie doit figurer dans missing_optional_columns"
        );
    }

    // ── US002 / RG-006 : empty numeric fields → None ─────────────────────────

    #[test]
    fn test_empty_numerics_become_none() {
        let csv = format!("{HDR}\n2;T;G;Nouveau;T;D;01-01-2026 08:00;Inc;;;;;;;;");
        let out = parse(&csv);
        let t = &out.tickets[0];
        assert!(t.priorite.is_none());
        assert!(t.urgence.is_none());
        assert!(t.nombre_suivis.is_none());
        assert!(t.inactivite_jours.is_none());
    }

    // ── US003 : normalisation ─────────────────────────────────────────────────

    #[test]
    fn test_anciennete_positive() {
        // date in 2024 — definitely in the past
        let csv = format!(
            "{HDR}\n1;T;G;Nouveau;T;D;01-01-2024 08:00;Inc;;0;;;;3;;4;"
        );
        let out = parse(&csv);
        assert!(out.tickets[0].anciennete_jours.unwrap() > 0);
    }

    #[test]
    fn test_inactivite_none_when_no_derniere_modif() {
        let csv = format!("{HDR}\n1;T;G;Nouveau;T;D;01-01-2026 08:00;Inc;;0;;;;3;;4;");
        let out = parse(&csv);
        assert!(out.tickets[0].inactivite_jours.is_none());
    }

    #[test]
    fn test_est_vivant_true() {
        for statut in &["Nouveau", "En cours (Attribué)", "En cours (Planifié)", "En attente"] {
            let csv = format!("{HDR}\n1;T;G;{statut};T;D;01-01-2026 08:00;Inc;;0;;;;3;;4;");
            let out = parse(&csv);
            assert!(out.tickets[0].est_vivant, "statut {statut} should be vivant");
        }
    }

    #[test]
    fn test_est_vivant_false() {
        // 16 fields: ID;Titre;Groupe;Statut;Tech;Dem;DateOuv;Type;SuivisDesc;NbSuivis;Int;Sol;Prio;Taches;Urgence;DernièreModif
        let csv = format!("{HDR}\n1;T;G;Résolu;T;D;01-01-2026 08:00;Inc;;0;;;;;;01-06-2026 10:00");
        let out = parse(&csv);
        assert!(!out.tickets[0].est_vivant);
        assert!(out.tickets[0].date_cloture_approx.is_some());
    }

    #[test]
    fn test_groupe_niveaux() {
        let csv = format!("{HDR}\n1;T;_DSI > _SUPPORT > _N2;Nouveau;T;D;01-01-2026 08:00;Inc;;0;;;;3;;4;");
        let out = parse(&csv);
        let t = &out.tickets[0];
        assert_eq!(t.groupe_niveau1.as_deref(), Some("_DSI"));
        assert_eq!(t.groupe_niveau2.as_deref(), Some("_SUPPORT"));
        assert_eq!(t.groupe_niveau3.as_deref(), Some("_N2"));
    }

    #[test]
    fn test_categorie_empty_string_becomes_none() {
        let hdr_with_cat = format!("{HDR};Catégorie");
        let csv = format!("{hdr_with_cat}\n1;T;G;Nouveau;T;D;01-01-2026 08:00;Inc;;0;;;;3;;4;;");
        let out = parse(&csv);
        assert!(out.tickets[0].categorie.is_none());
    }

    // ── RG-009 : malformed lines are skipped ─────────────────────────────────

    #[test]
    fn test_malformed_lines_skip() {
        // Line 2 has invalid ID and invalid date → should be skipped
        let csv = format!(
            "{HDR}\n\
             1;OK;G;Nouveau;T;D;01-01-2026 08:00;Inc;;0;;;;3;;4;\n\
             INVALID;Bad;G;X;T;D;not-a-date;Inc;;0;;;;3;;4;\n\
             2;Also OK;G;Résolu;T;D;02-01-2026 09:00;Inc;;1;;;;3;;4;01-02-2026 10:00"
        );
        let out = parse(&csv);
        assert_eq!(out.tickets.len(), 2, "2 valid tickets expected");
        assert_eq!(out.skipped_rows, 1);
        assert_eq!(out.warnings.len(), 1);
    }

    // ── RG-008 : missing required columns → error ─────────────────────────────

    #[test]
    fn test_missing_required_column_error() {
        let csv = "Titre;Statut\nFoo;Nouveau";
        match parse_err(csv) {
            AppError::MissingColumns(cols) => {
                assert!(cols.contains(&"ID".to_string()));
            }
            e => panic!("Expected MissingColumns, got {:?}", e),
        }
    }

    // ── Empty file ────────────────────────────────────────────────────────────

    #[test]
    fn test_empty_file_error() {
        match parse_err("") {
            AppError::EmptyFile | AppError::MissingColumns(_) | AppError::Csv(_) => {}
            e => panic!("Expected EmptyFile or related error, got {:?}", e),
        }
    }

    // ── Fixture: real ticket.csv (first ~50 records via actual file) ──────────

    #[test]
    fn test_real_ticket_csv_fixture() {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let fixture_path = format!("{}/../ticket.csv", manifest_dir);
        if !std::path::Path::new(&fixture_path).exists() {
            eprintln!("Skipping fixture test: {} not found", fixture_path);
            return;
        }
        let out = parse_csv(&fixture_path, |_, _| {}).unwrap();
        assert!(out.tickets.len() > 0, "Doit parser au moins un ticket");
        assert!(
            out.skipped_rows * 100 < out.total_rows_processed,
            "Taux d'erreur > 1% sur ticket.csv réel"
        );
        // All tickets have a valid date
        for t in &out.tickets {
            assert!(!t.date_ouverture.is_empty());
            assert!(t.id > 0);
        }
    }
}
