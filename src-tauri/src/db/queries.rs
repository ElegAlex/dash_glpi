use rusqlite::{params_from_iter, types::Value, Connection};

use crate::commands::import::{ImportComparison, ImportRecord, TechTimelinePoint, TechnicianDelta, TimelinePoint};
use crate::commands::search::TicketSearchResult;
use crate::commands::stock::{
    AgeRangeCount, GroupStock, StatutCount, StockFilters, StockOverview, TicketDetail, TicketSummary,
    TechnicianStock, TypeBreakdown,
};

// ─── Helpers privés ───────────────────────────────────────────────────────────

fn get_active_import_id(conn: &Connection) -> Result<i64, rusqlite::Error> {
    conn.query_row(
        "SELECT id FROM imports WHERE is_active = 1 ORDER BY id DESC LIMIT 1",
        [],
        |row| row.get(0),
    )
}

fn get_seuil_tickets(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT CAST(value AS INTEGER) FROM config WHERE key = 'seuil_tickets_technicien'",
        [],
        |row| row.get::<_, i64>(0),
    )
    .unwrap_or(20)
}

fn couleur_charge(nb: usize, seuil: i64) -> String {
    if seuil == 0 {
        return "rouge".to_string();
    }
    let ratio = nb as f64 / seuil as f64;
    if ratio <= 0.5 {
        "vert".to_string()
    } else if ratio <= 1.0 {
        "jaune".to_string()
    } else if ratio <= 2.0 {
        "orange".to_string()
    } else {
        "rouge".to_string()
    }
}

fn mediane(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

/// Ajoute dynamiquement des conditions WHERE selon les filtres fournis.
/// `params` est alimenté au fur et à mesure ; le ?N correspondant est calculé
/// d'après `params.len()` après push (SQLite params sont 1-indexés).
fn apply_filters(sql: &mut String, params: &mut Vec<Value>, filters: &StockFilters) {
    if let Some(statut) = &filters.statut {
        params.push(Value::Text(statut.clone()));
        sql.push_str(&format!(" AND statut = ?{}", params.len()));
    }
    if let Some(type_ticket) = &filters.type_ticket {
        params.push(Value::Text(type_ticket.clone()));
        sql.push_str(&format!(" AND type_ticket = ?{}", params.len()));
    }
    if let Some(groupe) = &filters.groupe {
        params.push(Value::Text(groupe.clone()));
        sql.push_str(&format!(" AND groupe_niveau1 = ?{}", params.len()));
    }
    if let Some(min) = &filters.min_anciennete {
        params.push(Value::Integer(*min));
        sql.push_str(&format!(" AND anciennete_jours >= ?{}", params.len()));
    }
    if let Some(max) = &filters.max_anciennete {
        params.push(Value::Integer(*max));
        sql.push_str(&format!(" AND anciennete_jours <= ?{}", params.len()));
    }
}

// ─── Fonctions de requête publiques ───────────────────────────────────────────

/// Vue d'ensemble du stock : totaux, statuts, types, distribution d'âge, inactifs.
pub fn get_stock_overview(conn: &Connection) -> Result<StockOverview, rusqlite::Error> {
    let import_id = get_active_import_id(conn)?;

    // 1. Comptages par statut
    let mut stmt = conn.prepare_cached(
        "SELECT statut, COUNT(*) AS cnt, est_vivant
         FROM tickets
         WHERE import_id = ?1
         GROUP BY statut
         ORDER BY cnt DESC",
    )?;
    let statut_rows: Vec<(String, i64, bool)> = stmt
        .query_map(rusqlite::params![import_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)? != 0,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let total_vivants: usize = statut_rows
        .iter()
        .filter(|(_, _, v)| *v)
        .map(|(_, c, _)| *c as usize)
        .sum();
    let total_termines: usize = statut_rows
        .iter()
        .filter(|(_, _, v)| !*v)
        .map(|(_, c, _)| *c as usize)
        .sum();

    let par_statut: Vec<StatutCount> = statut_rows
        .into_iter()
        .map(|(s, c, v)| StatutCount {
            statut: s,
            count: c as usize,
            est_vivant: v,
        })
        .collect();

    // 2. Âges des vivants (pour moyenne et médiane Rust)
    let mut stmt = conn.prepare_cached(
        "SELECT anciennete_jours
         FROM tickets
         WHERE import_id = ?1 AND est_vivant = 1 AND anciennete_jours IS NOT NULL",
    )?;
    let ages: Vec<f64> = stmt
        .query_map(rusqlite::params![import_id], |row| {
            row.get::<_, i64>(0)
        })?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|v| v as f64)
        .collect();

    let age_moyen_jours = if ages.is_empty() {
        0.0
    } else {
        (ages.iter().sum::<f64>() / ages.len() as f64 * 10.0).round() / 10.0
    };
    let age_median_jours = mediane(&ages);

    // 3. Par type (incidents / demandes vivants) — un seul scan
    let (incidents, demandes) = conn.query_row(
        "SELECT
            SUM(CASE WHEN type_ticket = 'Incident' THEN 1 ELSE 0 END),
            SUM(CASE WHEN type_ticket = 'Demande'  THEN 1 ELSE 0 END)
         FROM tickets
         WHERE import_id = ?1 AND est_vivant = 1",
        rusqlite::params![import_id],
        |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?.unwrap_or(0) as usize,
                row.get::<_, Option<i64>>(1)?.unwrap_or(0) as usize,
            ))
        },
    )?;

    // 4. Distribution par tranches d'ancienneté — un seul scan
    let (lt7, range7_30, range30_90, ge90) = conn.query_row(
        "SELECT
            SUM(CASE WHEN anciennete_jours < 7  THEN 1 ELSE 0 END),
            SUM(CASE WHEN anciennete_jours >= 7  AND anciennete_jours < 30 THEN 1 ELSE 0 END),
            SUM(CASE WHEN anciennete_jours >= 30 AND anciennete_jours < 90 THEN 1 ELSE 0 END),
            SUM(CASE WHEN anciennete_jours >= 90 THEN 1 ELSE 0 END)
         FROM tickets
         WHERE import_id = ?1 AND est_vivant = 1",
        rusqlite::params![import_id],
        |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?.unwrap_or(0) as usize,
                row.get::<_, Option<i64>>(1)?.unwrap_or(0) as usize,
                row.get::<_, Option<i64>>(2)?.unwrap_or(0) as usize,
                row.get::<_, Option<i64>>(3)?.unwrap_or(0) as usize,
            ))
        },
    )?;

    let total_v = total_vivants.max(1) as f64;
    let par_anciennete = vec![
        AgeRangeCount {
            label: "< 7j".to_string(),
            threshold_days: 0,
            count: lt7,
            percentage: (lt7 as f64 / total_v * 1000.0).round() / 10.0,
        },
        AgeRangeCount {
            label: "7-30j".to_string(),
            threshold_days: 7,
            count: range7_30,
            percentage: (range7_30 as f64 / total_v * 1000.0).round() / 10.0,
        },
        AgeRangeCount {
            label: "30-90j".to_string(),
            threshold_days: 30,
            count: range30_90,
            percentage: (range30_90 as f64 / total_v * 1000.0).round() / 10.0,
        },
        AgeRangeCount {
            label: "> 90j".to_string(),
            threshold_days: 90,
            count: ge90,
            percentage: (ge90 as f64 / total_v * 1000.0).round() / 10.0,
        },
    ];

    // 5. Inactifs — un seul scan
    let (inactifs_14j, inactifs_30j) = conn.query_row(
        "SELECT
            SUM(CASE WHEN inactivite_jours >= 14 THEN 1 ELSE 0 END),
            SUM(CASE WHEN inactivite_jours >= 30 THEN 1 ELSE 0 END)
         FROM tickets
         WHERE import_id = ?1 AND est_vivant = 1",
        rusqlite::params![import_id],
        |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?.unwrap_or(0) as usize,
                row.get::<_, Option<i64>>(1)?.unwrap_or(0) as usize,
            ))
        },
    )?;

    Ok(StockOverview {
        total_vivants,
        total_termines,
        par_statut,
        age_moyen_jours,
        age_median_jours,
        par_type: TypeBreakdown { incidents, demandes },
        par_anciennete,
        inactifs_14j,
        inactifs_30j,
    })
}

/// Charge par technicien, avec filtres optionnels.
pub fn get_technicians_stock(
    conn: &Connection,
    filters: Option<&StockFilters>,
) -> Result<Vec<TechnicianStock>, rusqlite::Error> {
    let import_id = get_active_import_id(conn)?;
    let seuil = get_seuil_tickets(conn);

    let mut sql = "\
        SELECT technicien_principal,
               COUNT(*) AS total,
               SUM(CASE WHEN statut = 'En cours (Attribué)' THEN 1 ELSE 0 END) AS en_cours,
               SUM(CASE WHEN statut = 'En attente'          THEN 1 ELSE 0 END) AS en_attente,
               SUM(CASE WHEN statut = 'En cours (Planifié)' THEN 1 ELSE 0 END) AS planifie,
               SUM(CASE WHEN statut = 'Nouveau'             THEN 1 ELSE 0 END) AS nouveau,
               SUM(CASE WHEN type_ticket = 'Incident'       THEN 1 ELSE 0 END) AS incidents,
               SUM(CASE WHEN type_ticket = 'Demande'        THEN 1 ELSE 0 END) AS demandes,
               COALESCE(AVG(CAST(anciennete_jours AS REAL)), 0.0) AS age_moyen,
               SUM(CASE WHEN inactivite_jours >= 14 THEN 1 ELSE 0 END) AS inactifs_14j
        FROM tickets
        WHERE import_id = ?1 AND est_vivant = 1
          AND technicien_principal IS NOT NULL AND technicien_principal != ''"
        .to_string();

    let mut params: Vec<Value> = vec![Value::Integer(import_id)];
    if let Some(f) = filters {
        apply_filters(&mut sql, &mut params, f);
    }
    sql.push_str(" GROUP BY technicien_principal ORDER BY total DESC");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params_from_iter(params), |row| {
            let tech: String = row.get(0)?;
            let total = row.get::<_, i64>(1)? as usize;
            let en_cours = row.get::<_, i64>(2)? as usize;
            let en_attente = row.get::<_, i64>(3)? as usize;
            let planifie = row.get::<_, i64>(4)? as usize;
            let nouveau = row.get::<_, i64>(5)? as usize;
            let incidents = row.get::<_, i64>(6)? as usize;
            let demandes = row.get::<_, i64>(7)? as usize;
            let age_moyen: f64 = row.get(8)?;
            let inactifs_14j = row.get::<_, i64>(9)? as usize;
            Ok((
                tech, total, en_cours, en_attente, planifie, nouveau,
                incidents, demandes, age_moyen, inactifs_14j,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows
        .into_iter()
        .map(
            |(tech, total, en_cours, en_attente, planifie, nouveau,
              incidents, demandes, age_moyen, inactifs_14j)| {
                TechnicianStock {
                    technicien: tech,
                    total,
                    en_cours,
                    en_attente,
                    planifie,
                    nouveau,
                    incidents,
                    demandes,
                    age_moyen_jours: (age_moyen * 10.0).round() / 10.0,
                    inactifs_14j,
                    ecart_seuil: total as i64 - seuil,
                    couleur_seuil: couleur_charge(total, seuil),
                }
            },
        )
        .collect())
}

/// Stock par groupe, avec filtres optionnels.
pub fn get_groups_stock(
    conn: &Connection,
    filters: Option<&StockFilters>,
) -> Result<Vec<GroupStock>, rusqlite::Error> {
    let import_id = get_active_import_id(conn)?;

    let mut sql = "\
        SELECT groupe_principal, groupe_niveau1, groupe_niveau2,
               COUNT(*) AS total,
               SUM(CASE WHEN statut = 'En cours (Attribué)' THEN 1 ELSE 0 END) AS en_cours,
               SUM(CASE WHEN statut = 'En attente'          THEN 1 ELSE 0 END) AS en_attente,
               SUM(CASE WHEN type_ticket = 'Incident'       THEN 1 ELSE 0 END) AS incidents,
               SUM(CASE WHEN type_ticket = 'Demande'        THEN 1 ELSE 0 END) AS demandes,
               COUNT(DISTINCT technicien_principal) AS nb_techniciens,
               COALESCE(AVG(CAST(anciennete_jours AS REAL)), 0.0) AS age_moyen
        FROM tickets
        WHERE import_id = ?1 AND est_vivant = 1
          AND groupe_principal IS NOT NULL AND groupe_principal != ''"
        .to_string();

    let mut params: Vec<Value> = vec![Value::Integer(import_id)];
    if let Some(f) = filters {
        apply_filters(&mut sql, &mut params, f);
    }
    sql.push_str(
        " GROUP BY groupe_principal, groupe_niveau1, groupe_niveau2 ORDER BY total DESC",
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params_from_iter(params), |row| {
            let age_moyen: f64 = row.get(9)?;
            Ok(GroupStock {
                groupe: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                groupe_niveau1: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                groupe_niveau2: row.get(2)?,
                total: row.get::<_, i64>(3)? as usize,
                en_cours: row.get::<_, i64>(4)? as usize,
                en_attente: row.get::<_, i64>(5)? as usize,
                incidents: row.get::<_, i64>(6)? as usize,
                demandes: row.get::<_, i64>(7)? as usize,
                nb_techniciens: row.get::<_, i64>(8)? as usize,
                age_moyen_jours: (age_moyen * 10.0).round() / 10.0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

/// Détail complet d'un ticket (dans l'import actif).
pub fn get_ticket_detail(
    conn: &Connection,
    ticket_id: u64,
) -> Result<TicketDetail, rusqlite::Error> {
    let import_id = get_active_import_id(conn)?;
    conn.query_row(
        "SELECT id, titre, statut, type_ticket, priorite, urgence, demandeur,
                techniciens, groupes, date_ouverture, derniere_modification, nombre_suivis,
                suivis_description, solution, taches_description,
                anciennete_jours, inactivite_jours, action_recommandee, motif_classification,
                categorie
         FROM tickets
         WHERE id = ?1 AND import_id = ?2",
        rusqlite::params![ticket_id, import_id],
        |row| {
            let techniciens_json: String = row.get(7)?;
            let groupes_json: String = row.get(8)?;
            let techniciens: Vec<String> =
                serde_json::from_str(&techniciens_json).unwrap_or_default();
            let groupes: Vec<String> =
                serde_json::from_str(&groupes_json).unwrap_or_default();
            Ok(TicketDetail {
                id: row.get::<_, u64>(0)?,
                titre: row.get(1)?,
                statut: row.get(2)?,
                type_ticket: row.get(3)?,
                priorite: row.get::<_, Option<i64>>(4)?.map(|v| v as u8),
                urgence: row.get::<_, Option<i64>>(5)?.map(|v| v as u8),
                demandeur: row.get(6)?,
                techniciens,
                groupes,
                date_ouverture: row.get(9)?,
                derniere_modification: row.get(10)?,
                nombre_suivis: row.get::<_, Option<i64>>(11)?.map(|v| v as u32),
                suivis_description: row
                    .get::<_, Option<String>>(12)?
                    .unwrap_or_default(),
                solution: row.get::<_, Option<String>>(13)?.unwrap_or_default(),
                taches_description: row
                    .get::<_, Option<String>>(14)?
                    .unwrap_or_default(),
                anciennete_jours: row.get(15)?,
                inactivite_jours: row.get(16)?,
                action_recommandee: row.get(17)?,
                motif_classification: row.get(18)?,
                categorie: row.get(19)?,
            })
        },
    )
}

/// Liste des tickets vivants d'un technicien, avec filtres optionnels.
pub fn get_technician_tickets(
    conn: &Connection,
    technician: &str,
    filters: Option<&StockFilters>,
) -> Result<Vec<TicketSummary>, rusqlite::Error> {
    let import_id = get_active_import_id(conn)?;

    let mut sql = "\
        SELECT id, titre, statut, type_ticket, technicien_principal, groupe_principal,
               date_ouverture, derniere_modification, anciennete_jours, inactivite_jours,
               nombre_suivis, action_recommandee, motif_classification
        FROM tickets
        WHERE import_id = ?1 AND est_vivant = 1 AND technicien_principal = ?2"
        .to_string();

    let mut params: Vec<Value> = vec![
        Value::Integer(import_id),
        Value::Text(technician.to_string()),
    ];
    if let Some(f) = filters {
        apply_filters(&mut sql, &mut params, f);
    }
    sql.push_str(" ORDER BY COALESCE(anciennete_jours, 0) DESC");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params_from_iter(params), |row| {
            Ok(TicketSummary {
                id: row.get::<_, u64>(0)?,
                titre: row.get(1)?,
                statut: row.get(2)?,
                type_ticket: row.get(3)?,
                technicien_principal: row.get(4)?,
                groupe_principal: row.get(5)?,
                date_ouverture: row.get(6)?,
                derniere_modification: row.get(7)?,
                anciennete_jours: row.get(8)?,
                inactivite_jours: row.get(9)?,
                nombre_suivis: row.get::<_, Option<i64>>(10)?.map(|v| v as u32),
                action_recommandee: row.get(11)?,
                motif_classification: row.get(12)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

/// Historique des imports, du plus récent au plus ancien.
pub fn get_import_history(conn: &Connection) -> Result<Vec<ImportRecord>, rusqlite::Error> {
    let mut stmt = conn.prepare_cached(
        "SELECT id, filename, import_date, total_rows, vivants_count, termines_count,
                date_range_from, date_range_to, is_active
         FROM imports
         ORDER BY import_date DESC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ImportRecord {
                id: row.get(0)?,
                filename: row.get(1)?,
                import_date: row.get(2)?,
                total_rows: row.get::<_, i64>(3)? as usize,
                vivants_count: row.get::<_, i64>(4)? as usize,
                termines_count: row.get::<_, i64>(5)? as usize,
                date_range_from: row.get(6)?,
                date_range_to: row.get(7)?,
                is_active: row.get::<_, i64>(8)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Logique de comparaison de deux imports : deltas, nouveaux, disparus, par technicien.
pub fn compare_imports_logic(
    conn: &Connection,
    import_id_a: i64,
    import_id_b: i64,
) -> Result<ImportComparison, rusqlite::Error> {
    let load_record = |id: i64| -> Result<ImportRecord, rusqlite::Error> {
        conn.query_row(
            "SELECT id, filename, import_date, total_rows, vivants_count, termines_count,
                    date_range_from, date_range_to, is_active
             FROM imports WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                Ok(ImportRecord {
                    id: row.get(0)?,
                    filename: row.get(1)?,
                    import_date: row.get(2)?,
                    total_rows: row.get::<_, i64>(3)? as usize,
                    vivants_count: row.get::<_, i64>(4)? as usize,
                    termines_count: row.get::<_, i64>(5)? as usize,
                    date_range_from: row.get(6)?,
                    date_range_to: row.get(7)?,
                    is_active: row.get::<_, i64>(8)? != 0,
                })
            },
        )
    };

    let import_a = load_record(import_id_a)?;
    let import_b = load_record(import_id_b)?;

    let ticket_ids = |import_id: i64| -> Result<std::collections::HashSet<u64>, rusqlite::Error> {
        let mut stmt = conn.prepare("SELECT id FROM tickets WHERE import_id = ?1")?;
        let ids = stmt
            .query_map(rusqlite::params![import_id], |row| row.get::<_, u64>(0))?
            .collect::<Result<_, _>>()?;
        Ok(ids)
    };

    let ids_a = ticket_ids(import_id_a)?;
    let ids_b = ticket_ids(import_id_b)?;

    let delta_total = import_b.total_rows as i64 - import_a.total_rows as i64;
    let delta_vivants = import_b.vivants_count as i64 - import_a.vivants_count as i64;

    let mut nouveaux_tickets: Vec<u64> = ids_b.difference(&ids_a).copied().collect();
    nouveaux_tickets.sort_unstable();
    let mut disparus_tickets: Vec<u64> = ids_a.difference(&ids_b).copied().collect();
    disparus_tickets.sort_unstable();

    let tech_counts = |import_id: i64| -> Result<std::collections::HashMap<String, usize>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT technicien_principal, COUNT(*) FROM tickets
             WHERE import_id = ?1 AND est_vivant = 1
               AND technicien_principal IS NOT NULL AND technicien_principal != ''
             GROUP BY technicien_principal",
        )?;
        let pairs = stmt
            .query_map(rusqlite::params![import_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(pairs.into_iter().collect())
    };

    let tech_a = tech_counts(import_id_a)?;
    let tech_b = tech_counts(import_id_b)?;

    let mut all_techs: std::collections::HashSet<String> = std::collections::HashSet::new();
    all_techs.extend(tech_a.keys().cloned());
    all_techs.extend(tech_b.keys().cloned());

    let mut delta_par_technicien: Vec<TechnicianDelta> = all_techs
        .into_iter()
        .map(|tech| {
            let count_a = tech_a.get(&tech).copied().unwrap_or(0);
            let count_b = tech_b.get(&tech).copied().unwrap_or(0);
            TechnicianDelta {
                technicien: tech,
                count_a,
                count_b,
                delta: count_b as i64 - count_a as i64,
            }
        })
        .collect();
    delta_par_technicien.sort_by(|a, b| b.delta.unsigned_abs().cmp(&a.delta.unsigned_abs()));

    Ok(ImportComparison {
        import_a,
        import_b,
        delta_total,
        delta_vivants,
        nouveaux_tickets,
        disparus_tickets,
        delta_par_technicien,
    })
}

/// Timeline de tous les imports, du plus ancien au plus récent.
pub fn get_timeline_data(conn: &Connection) -> Result<Vec<TimelinePoint>, rusqlite::Error> {
    let mut stmt = conn.prepare_cached(
        "SELECT id, filename, import_date, vivants_count, termines_count, total_rows
         FROM imports ORDER BY import_date ASC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(TimelinePoint {
                import_id: row.get(0)?,
                filename: row.get(1)?,
                import_date: row.get(2)?,
                vivants_count: row.get::<_, i64>(3)? as usize,
                termines_count: row.get::<_, i64>(4)? as usize,
                total_rows: row.get::<_, i64>(5)? as usize,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Timeline d'un technicien : ticket vivant count + âge moyen par import (ASC).
pub fn get_technician_timeline(
    conn: &Connection,
    technicien: &str,
) -> Result<Vec<TechTimelinePoint>, rusqlite::Error> {
    let mut stmt = conn.prepare_cached(
        "SELECT i.id, i.import_date, COUNT(*) AS ticket_count, AVG(t.anciennete_jours) AS avg_age
         FROM tickets t
         JOIN imports i ON t.import_id = i.id
         WHERE t.technicien_principal = ?1 AND t.est_vivant = 1
         GROUP BY i.id
         ORDER BY i.import_date ASC",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![technicien], |row| {
            Ok(TechTimelinePoint {
                import_id: row.get(0)?,
                import_date: row.get(1)?,
                ticket_count: row.get::<_, i64>(2)? as usize,
                avg_age: row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Données brutes pour construire l'arbre catégories/groupes.
/// Retourne (groupe_principal, total_vivants, incidents, demandes).
pub fn get_category_tree_data(
    conn: &Connection,
) -> Result<Vec<(String, i64, i64, i64)>, rusqlite::Error> {
    let import_id = get_active_import_id(conn)?;
    let mut stmt = conn.prepare_cached(
        "SELECT groupe_principal,
                COUNT(*) AS total,
                SUM(CASE WHEN type_ticket = 'Incident' THEN 1 ELSE 0 END) AS incidents,
                SUM(CASE WHEN type_ticket = 'Demande'  THEN 1 ELSE 0 END) AS demandes
         FROM tickets
         WHERE import_id = ?1 AND est_vivant = 1 AND groupe_principal IS NOT NULL
         GROUP BY groupe_principal
         ORDER BY total DESC",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![import_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Recherche FTS5 full-text dans les tickets de l'import actif.
/// Retourne jusqu'à `limit` résultats triés par pertinence décroissante.
pub fn search_tickets_fts(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<TicketSearchResult>, rusqlite::Error> {
    let import_id = get_active_import_id(conn)?;
    let mut stmt = conn.prepare(
        "SELECT t.id, t.titre, t.statut, t.technicien_principal,
                highlight(tickets_fts, 0, '<mark>', '</mark>'),
                snippet(tickets_fts, 2, '<mark>', '</mark>', '...', 32),
                tickets_fts.rank
         FROM tickets_fts
         JOIN tickets t ON t.rowid = tickets_fts.rowid
         WHERE tickets_fts MATCH ?1
           AND t.import_id = ?2
         ORDER BY tickets_fts.rank
         LIMIT ?3",
    )?;
    let rows = stmt
        .query_map(
            rusqlite::params![query, import_id, limit as i64],
            |row| {
                let rank: f64 = row.get(6)?;
                Ok(TicketSearchResult {
                    id: row.get::<_, u64>(0)?,
                    titre: row.get(1)?,
                    statut: row.get(2)?,
                    technicien: row.get(3)?,
                    titre_highlight: row
                        .get::<_, Option<String>>(4)?
                        .unwrap_or_default(),
                    solution_highlight: row.get(5)?,
                    rank: -rank, // FTS5 rank est négatif ; on inverse pour que plus grand = meilleur
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

// ─── Fonctions Bilan temporel ────────────────────────────────────────────────

/// Construit l'expression SQLite pour la granularité de période.
fn periode_expr(granularity: &str, col: &str) -> String {
    match granularity {
        "day" => format!("strftime('%Y-%m-%d', {col})"),
        "week" => format!("strftime('%Y-W%W', {col})"),
        "quarter" => format!(
            "CASE \
             WHEN strftime('%m', {col}) BETWEEN '01' AND '03' THEN strftime('%Y', {col}) || '-Q1' \
             WHEN strftime('%m', {col}) BETWEEN '04' AND '06' THEN strftime('%Y', {col}) || '-Q2' \
             WHEN strftime('%m', {col}) BETWEEN '07' AND '09' THEN strftime('%Y', {col}) || '-Q3' \
             ELSE strftime('%Y', {col}) || '-Q4' \
             END"
        ),
        _ => format!("strftime('%Y-%m', {col})"), // default: month
    }
}

/// Flux entrants : tickets dont `date_ouverture` est dans [date_from, date_to],
/// groupés par période (week / month / quarter). Filtres optionnels applicables.
pub fn get_bilan_entrees_par_periode(
    conn: &Connection,
    date_from: &str,
    date_to: &str,
    granularity: &str,
    filters: Option<&StockFilters>,
) -> Result<Vec<(String, usize)>, rusqlite::Error> {
    let import_id = get_active_import_id(conn)?;
    let periode = periode_expr(granularity, "date_ouverture");

    let mut sql = format!(
        "SELECT {periode} AS periode, COUNT(*) AS n \
         FROM tickets \
         WHERE import_id = ?1 \
           AND date_ouverture >= ?2 \
           AND date_ouverture < date(?3, '+1 day')"
    );

    let mut params: Vec<Value> = vec![
        Value::Integer(import_id),
        Value::Text(date_from.to_string()),
        Value::Text(date_to.to_string()),
    ];

    if let Some(f) = filters {
        apply_filters(&mut sql, &mut params, f);
    }
    sql.push_str(" GROUP BY periode ORDER BY periode");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params_from_iter(params), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Flux sortants : tickets terminés (statut IN ('Résolu','Clos')) dont
/// `derniere_modification` est dans [date_from, date_to], groupés par période.
pub fn get_bilan_sorties_par_periode(
    conn: &Connection,
    date_from: &str,
    date_to: &str,
    granularity: &str,
    filters: Option<&StockFilters>,
) -> Result<Vec<(String, usize)>, rusqlite::Error> {
    let import_id = get_active_import_id(conn)?;
    let periode = periode_expr(granularity, "derniere_modification");

    let mut sql = format!(
        "SELECT {periode} AS periode, COUNT(*) AS n \
         FROM tickets \
         WHERE import_id = ?1 \
           AND statut IN ('Résolu', 'Clos') \
           AND derniere_modification >= ?2 \
           AND derniere_modification < date(?3, '+1 day')"
    );

    let mut params: Vec<Value> = vec![
        Value::Integer(import_id),
        Value::Text(date_from.to_string()),
        Value::Text(date_to.to_string()),
    ];

    if let Some(f) = filters {
        apply_filters(&mut sql, &mut params, f);
    }
    sql.push_str(" GROUP BY periode ORDER BY periode");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params_from_iter(params), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Stock à une date donnée (approximation) :
/// COUNT tickets de l'import actif dont `date_ouverture` ≤ date ET `est_vivant` = 1.
pub fn get_stock_at_date(conn: &Connection, date: &str) -> Result<usize, rusqlite::Error> {
    let import_id = get_active_import_id(conn)?;
    conn.query_row(
        "SELECT COUNT(*) FROM tickets \
         WHERE import_id = ?1 \
           AND date_ouverture < date(?2, '+1 day') \
           AND est_vivant = 1",
        rusqlite::params![import_id, date],
        |row| row.get::<_, i64>(0).map(|v| v as usize),
    )
}

/// Ventilation par technicien sur la période : (technicien, entrants, sortants).
/// Entrants = tickets dont `date_ouverture` dans la période.
/// Sortants = tickets Résolu/Clos dont `derniere_modification` dans la période.
pub fn get_bilan_ventilation_par_technicien(
    conn: &Connection,
    date_from: &str,
    date_to: &str,
) -> Result<Vec<(String, usize, usize)>, rusqlite::Error> {
    let import_id = get_active_import_id(conn)?;
    let mut stmt = conn.prepare(
        "SELECT technicien, SUM(entrants), SUM(sortants) \
         FROM ( \
             SELECT technicien_principal AS technicien, 1 AS entrants, 0 AS sortants \
             FROM tickets \
             WHERE import_id = ?1 \
               AND date_ouverture >= ?2 \
               AND date_ouverture < date(?3, '+1 day') \
               AND technicien_principal IS NOT NULL AND technicien_principal != '' \
             UNION ALL \
             SELECT technicien_principal AS technicien, 0 AS entrants, 1 AS sortants \
             FROM tickets \
             WHERE import_id = ?1 \
               AND statut IN ('Résolu', 'Clos') \
               AND derniere_modification >= ?2 \
               AND derniere_modification < date(?3, '+1 day') \
               AND technicien_principal IS NOT NULL AND technicien_principal != '' \
         ) \
         GROUP BY technicien \
         ORDER BY (SUM(entrants) + SUM(sortants)) DESC",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![import_id, date_from, date_to], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)? as usize,
                row.get::<_, i64>(2)? as usize,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Ventilation par groupe sur la période : (groupe, entrants, sortants).
/// Entrants = tickets dont `date_ouverture` dans la période.
/// Sortants = tickets Résolu/Clos dont `derniere_modification` dans la période.
pub fn get_bilan_ventilation_par_groupe(
    conn: &Connection,
    date_from: &str,
    date_to: &str,
) -> Result<Vec<(String, usize, usize)>, rusqlite::Error> {
    let import_id = get_active_import_id(conn)?;
    let mut stmt = conn.prepare(
        "SELECT groupe, SUM(entrants), SUM(sortants) \
         FROM ( \
             SELECT groupe_principal AS groupe, 1 AS entrants, 0 AS sortants \
             FROM tickets \
             WHERE import_id = ?1 \
               AND date_ouverture >= ?2 \
               AND date_ouverture < date(?3, '+1 day') \
               AND groupe_principal IS NOT NULL AND groupe_principal != '' \
             UNION ALL \
             SELECT groupe_principal AS groupe, 0 AS entrants, 1 AS sortants \
             FROM tickets \
             WHERE import_id = ?1 \
               AND statut IN ('Résolu', 'Clos') \
               AND derniere_modification >= ?2 \
               AND derniere_modification < date(?3, '+1 day') \
               AND groupe_principal IS NOT NULL AND groupe_principal != '' \
         ) \
         GROUP BY groupe \
         ORDER BY (SUM(entrants) + SUM(sortants)) DESC",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![import_id, date_from, date_to], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)? as usize,
                row.get::<_, i64>(2)? as usize,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup() -> (Connection, i64) {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("sql/001_initial.sql")).unwrap();

        conn.execute(
            "INSERT INTO imports (
                filename, total_rows, parsed_rows, skipped_rows,
                vivants_count, termines_count,
                detected_columns, unique_statuts, unique_types,
                is_active
             ) VALUES ('test.csv', 5, 5, 0, 3, 2, '[]', '[]', '[]', 1)",
            [],
        )
        .unwrap();
        let import_id = conn.last_insert_rowid();

        // Ticket 1 : vivant, Incident, Dupont, _DSI > _SUPPORT, 5j, 2j inactivité
        conn.execute(
            "INSERT INTO tickets (
                id, import_id, titre, statut, type_ticket, demandeur, date_ouverture,
                est_vivant, technicien_principal, groupe_principal, groupe_niveau1, groupe_niveau2,
                anciennete_jours, inactivite_jours, nombre_suivis
             ) VALUES (1, ?1, 'Ticket 1', 'En cours (Attribué)', 'Incident',
                       'user1', '2026-02-24', 1, 'Dupont',
                       '_DSI > _SUPPORT', '_DSI', '_SUPPORT', 5, 2, 1)",
            rusqlite::params![import_id],
        )
        .unwrap();

        // Ticket 2 : vivant, Demande, Dupont, _DSI > _SUPPORT, 15j, 20j inactivité
        conn.execute(
            "INSERT INTO tickets (
                id, import_id, titre, statut, type_ticket, demandeur, date_ouverture,
                est_vivant, technicien_principal, groupe_principal, groupe_niveau1, groupe_niveau2,
                anciennete_jours, inactivite_jours, nombre_suivis
             ) VALUES (2, ?1, 'Ticket 2', 'En attente', 'Demande',
                       'user2', '2026-02-14', 1, 'Dupont',
                       '_DSI > _SUPPORT', '_DSI', '_SUPPORT', 15, 20, 0)",
            rusqlite::params![import_id],
        )
        .unwrap();

        // Ticket 3 : vivant, Incident, Martin, _DSI > _INFRA, 100j, 50j inactivité
        conn.execute(
            "INSERT INTO tickets (
                id, import_id, titre, statut, type_ticket, demandeur, date_ouverture,
                est_vivant, technicien_principal, groupe_principal, groupe_niveau1, groupe_niveau2,
                anciennete_jours, inactivite_jours, nombre_suivis
             ) VALUES (3, ?1, 'Ticket 3 reseau probleme', 'Nouveau', 'Incident',
                       'user3', '2025-11-22', 1, 'Martin',
                       '_DSI > _INFRA', '_DSI', '_INFRA', 100, 50, 2)",
            rusqlite::params![import_id],
        )
        .unwrap();

        // Ticket 4 : terminé (Clos), Dupont
        conn.execute(
            "INSERT INTO tickets (
                id, import_id, titre, statut, type_ticket, demandeur, date_ouverture,
                est_vivant, technicien_principal, groupe_principal, groupe_niveau1,
                anciennete_jours, inactivite_jours, nombre_suivis
             ) VALUES (4, ?1, 'Ticket 4 clos solution', 'Clos', 'Incident',
                       'user1', '2026-01-01', 0, 'Dupont',
                       '_DSI > _SUPPORT', '_DSI', 60, 60, 3)",
            rusqlite::params![import_id],
        )
        .unwrap();

        // Ticket 5 : terminé (Résolu), Martin
        conn.execute(
            "INSERT INTO tickets (
                id, import_id, titre, statut, type_ticket, demandeur, date_ouverture,
                est_vivant, technicien_principal, groupe_principal, groupe_niveau1,
                anciennete_jours, inactivite_jours, nombre_suivis
             ) VALUES (5, ?1, 'Ticket 5', 'Résolu', 'Demande',
                       'user2', '2026-01-15', 0, 'Martin',
                       '_DSI > _INFRA', '_DSI', 45, 45, 1)",
            rusqlite::params![import_id],
        )
        .unwrap();

        (conn, import_id)
    }

    #[test]
    fn test_import_history_returns_record() {
        let (conn, _) = setup();
        let history = get_import_history(&conn).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].filename, "test.csv");
        assert!(history[0].is_active);
        assert_eq!(history[0].total_rows, 5);
        assert_eq!(history[0].vivants_count, 3);
        assert_eq!(history[0].termines_count, 2);
    }

    #[test]
    fn test_import_history_ordered_desc() {
        let (conn, _) = setup();
        // Deuxième import plus récent
        conn.execute(
            "INSERT INTO imports (
                filename, total_rows, parsed_rows, skipped_rows,
                vivants_count, termines_count,
                detected_columns, unique_statuts, unique_types,
                import_date, is_active
             ) VALUES ('test2.csv', 10, 10, 0, 5, 5, '[]', '[]', '[]',
                       '2026-03-01T10:00:00', 0)",
            [],
        )
        .unwrap();
        let history = get_import_history(&conn).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].filename, "test2.csv");
    }

    #[test]
    fn test_stock_overview_totals() {
        let (conn, _) = setup();
        let ov = get_stock_overview(&conn).unwrap();
        assert_eq!(ov.total_vivants, 3);
        assert_eq!(ov.total_termines, 2);
    }

    #[test]
    fn test_stock_overview_par_type() {
        let (conn, _) = setup();
        let ov = get_stock_overview(&conn).unwrap();
        // Vivants : ticket 1 (Incident), ticket 2 (Demande), ticket 3 (Incident)
        assert_eq!(ov.par_type.incidents, 2);
        assert_eq!(ov.par_type.demandes, 1);
    }

    #[test]
    fn test_stock_overview_age_distribution() {
        let (conn, _) = setup();
        let ov = get_stock_overview(&conn).unwrap();
        // Âges des vivants : 5j, 15j, 100j
        assert_eq!(ov.par_anciennete[0].count, 1); // < 7j  → ticket 1 (5j)
        assert_eq!(ov.par_anciennete[1].count, 1); // 7-30j → ticket 2 (15j)
        assert_eq!(ov.par_anciennete[2].count, 0); // 30-90j → aucun
        assert_eq!(ov.par_anciennete[3].count, 1); // > 90j → ticket 3 (100j)
    }

    #[test]
    fn test_stock_overview_inactifs() {
        let (conn, _) = setup();
        let ov = get_stock_overview(&conn).unwrap();
        // inactivite >= 14j : ticket 2 (20j), ticket 3 (50j) → 2
        assert_eq!(ov.inactifs_14j, 2);
        // inactivite >= 30j : ticket 3 (50j) → 1
        assert_eq!(ov.inactifs_30j, 1);
    }

    #[test]
    fn test_stock_overview_age_moyen() {
        let (conn, _) = setup();
        let ov = get_stock_overview(&conn).unwrap();
        // (5 + 15 + 100) / 3 = 40.0
        assert!((ov.age_moyen_jours - 40.0).abs() < 0.5);
    }

    #[test]
    fn test_technicians_stock_grouping() {
        let (conn, _) = setup();
        let techs = get_technicians_stock(&conn, None).unwrap();
        assert_eq!(techs.len(), 2);
        // Trié par total DESC : Dupont (2), Martin (1)
        assert_eq!(techs[0].technicien, "Dupont");
        assert_eq!(techs[0].total, 2);
        assert_eq!(techs[0].en_cours, 1); // ticket 1 : En cours (Attribué)
        assert_eq!(techs[0].en_attente, 1); // ticket 2 : En attente
        assert_eq!(techs[0].incidents, 1);
        assert_eq!(techs[0].demandes, 1);
        assert_eq!(techs[1].technicien, "Martin");
        assert_eq!(techs[1].total, 1);
        assert_eq!(techs[1].nouveau, 1); // ticket 3 : Nouveau
    }

    #[test]
    fn test_technicians_stock_inactifs() {
        let (conn, _) = setup();
        let techs = get_technicians_stock(&conn, None).unwrap();
        // Dupont inactifs_14j : ticket 2 (20j) → 1
        assert_eq!(techs[0].inactifs_14j, 1);
        // Martin inactifs_14j : ticket 3 (50j) → 1
        assert_eq!(techs[1].inactifs_14j, 1);
    }

    #[test]
    fn test_technicians_stock_couleur_seuil() {
        let (conn, _) = setup();
        let techs = get_technicians_stock(&conn, None).unwrap();
        // Seuil par défaut = 20 ; Dupont 2/20 = 0.1 → ratio <= 0.5 → "vert"
        assert_eq!(techs[0].couleur_seuil, "vert");
        assert_eq!(techs[0].ecart_seuil, 2 - 20);
    }

    #[test]
    fn test_technicians_stock_with_filter() {
        let (conn, _) = setup();
        let filters = StockFilters {
            statut: Some("En cours (Attribué)".to_string()),
            type_ticket: None,
            groupe: None,
            min_anciennete: None,
            max_anciennete: None,
        };
        let techs = get_technicians_stock(&conn, Some(&filters)).unwrap();
        // Seul ticket 1 (Dupont, En cours Attribué) passe le filtre
        assert_eq!(techs.len(), 1);
        assert_eq!(techs[0].technicien, "Dupont");
        assert_eq!(techs[0].total, 1);
    }

    #[test]
    fn test_groups_stock() {
        let (conn, _) = setup();
        let groups = get_groups_stock(&conn, None).unwrap();
        assert_eq!(groups.len(), 2);
        // _DSI > _SUPPORT : 2 tickets vivants
        assert_eq!(groups[0].total, 2);
        assert_eq!(groups[0].groupe_niveau1, "_DSI");
        assert_eq!(groups[0].groupe_niveau2, Some("_SUPPORT".to_string()));
    }

    #[test]
    fn test_ticket_detail() {
        let (conn, _) = setup();
        let detail = get_ticket_detail(&conn, 1).unwrap();
        assert_eq!(detail.id, 1);
        assert_eq!(detail.titre, "Ticket 1");
        assert_eq!(detail.statut, "En cours (Attribué)");
        assert_eq!(detail.type_ticket, "Incident");
        assert_eq!(detail.demandeur, "user1");
        assert_eq!(detail.anciennete_jours, Some(5));
    }

    #[test]
    fn test_ticket_detail_not_found() {
        let (conn, _) = setup();
        let result = get_ticket_detail(&conn, 999);
        assert!(result.is_err());
    }

    #[test]
    fn test_technician_tickets_ordered_by_age() {
        let (conn, _) = setup();
        let tickets = get_technician_tickets(&conn, "Dupont", None).unwrap();
        assert_eq!(tickets.len(), 2);
        // Ordonné par ancienneté DESC : ticket 2 (15j) avant ticket 1 (5j)
        assert_eq!(tickets[0].id, 2);
        assert_eq!(tickets[1].id, 1);
    }

    #[test]
    fn test_technician_tickets_with_filter() {
        let (conn, _) = setup();
        let filters = StockFilters {
            statut: None,
            type_ticket: Some("Demande".to_string()),
            groupe: None,
            min_anciennete: None,
            max_anciennete: None,
        };
        let tickets = get_technician_tickets(&conn, "Dupont", Some(&filters)).unwrap();
        // Seul ticket 2 (Demande) pour Dupont
        assert_eq!(tickets.len(), 1);
        assert_eq!(tickets[0].id, 2);
    }

    #[test]
    fn test_category_tree_data() {
        let (conn, _) = setup();
        let data = get_category_tree_data(&conn).unwrap();
        assert_eq!(data.len(), 2);
        // _DSI > _SUPPORT : 2 tickets vivants (Incident + Demande)
        assert_eq!(data[0].1, 2);
        assert_eq!(data[0].2, 1); // incidents
        assert_eq!(data[0].3, 1); // demandes
        // _DSI > _INFRA : 1 ticket vivant
        assert_eq!(data[1].1, 1);
    }

    #[test]
    fn test_no_active_import_returns_error() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("sql/001_initial.sql")).unwrap();
        assert!(get_stock_overview(&conn).is_err());
        assert!(get_import_history(&conn).unwrap().is_empty());
    }

    #[test]
    fn test_search_tickets_fts() {
        let (conn, _) = setup();
        // "reseau" doit matcher ticket 3 ("Ticket 3 reseau probleme")
        let results = search_tickets_fts(&conn, "reseau", 10).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, 3);
        assert!(results[0].rank > 0.0);
    }

    #[test]
    fn test_search_tickets_fts_no_match() {
        let (conn, _) = setup();
        let results = search_tickets_fts(&conn, "xyzzy", 10).unwrap();
        assert!(results.is_empty());
    }

    // ─── Test couleur_charge ────────────────────────────────────────────────

    #[test]
    fn test_couleur_charge_thresholds() {
        assert_eq!(couleur_charge(10, 20), "vert");   // 0.5 → vert
        assert_eq!(couleur_charge(11, 20), "jaune");  // 0.55 → jaune
        assert_eq!(couleur_charge(20, 20), "jaune");  // 1.0 → jaune
        assert_eq!(couleur_charge(21, 20), "orange"); // 1.05 → orange
        assert_eq!(couleur_charge(40, 20), "orange"); // 2.0 → orange
        assert_eq!(couleur_charge(41, 20), "rouge");  // > 2.0 → rouge
        assert_eq!(couleur_charge(5, 0), "rouge");    // seuil 0 → rouge
    }
}

// ─── Tests Bilan temporel ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests_bilan {
    use super::*;
    use rusqlite::Connection;

    /// Setup dédié aux tests bilan : tickets avec `derniere_modification` explicite,
    /// répartis sur plusieurs mois pour tester les agrégations temporelles.
    ///
    /// Import actif id=1, tickets :
    /// | id | date_ouverture      | statut               | derniere_modification | tech    | groupe          | type     |
    /// |----|---------------------|----------------------|-----------------------|---------|-----------------|----------|
    /// | 10 | 2026-01-05T09:00:00 | Nouveau (vivant)     | 2026-01-05T09:00:00   | Dupont  | _DSI > _SUPPORT | Incident |
    /// | 11 | 2026-01-15T08:00:00 | Clos (terminé)       | 2026-01-25T10:00:00   | Dupont  | _DSI > _SUPPORT | Incident |
    /// | 12 | 2026-02-03T10:00:00 | En cours (vivant)    | 2026-02-03T10:00:00   | Martin  | _DSI > _INFRA   | Demande  |
    /// | 13 | 2026-02-10T11:00:00 | Résolu (terminé)     | 2026-02-20T14:00:00   | Martin  | _DSI > _INFRA   | Incident |
    /// | 14 | 2025-12-20T09:00:00 | Clos (terminé)       | 2026-02-01T09:00:00   | Dupont  | _DSI > _SUPPORT | Demande  |
    /// | 15 | 2026-03-05T08:00:00 | Nouveau (vivant)     | 2026-03-05T08:00:00   | Martin  | _DSI > _INFRA   | Incident |
    fn setup_bilan() -> (Connection, i64) {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("sql/001_initial.sql")).unwrap();

        conn.execute(
            "INSERT INTO imports (
                filename, total_rows, parsed_rows, skipped_rows,
                vivants_count, termines_count,
                detected_columns, unique_statuts, unique_types,
                is_active
             ) VALUES ('bilan.csv', 6, 6, 0, 3, 3, '[]', '[]', '[]', 1)",
            [],
        )
        .unwrap();
        let import_id = conn.last_insert_rowid();

        let tickets: &[(&str, &str, &str, &str, &str, i64, &str)] = &[
            ("Nouveau",            "2026-01-05T09:00:00", "2026-01-05T09:00:00", "Dupont", "_DSI > _SUPPORT", 1, "Incident"),
            ("Clos",               "2026-01-15T08:00:00", "2026-01-25T10:00:00", "Dupont", "_DSI > _SUPPORT", 0, "Incident"),
            ("En cours (Attribué)","2026-02-03T10:00:00", "2026-02-03T10:00:00", "Martin", "_DSI > _INFRA",   1, "Demande"),
            ("Résolu",             "2026-02-10T11:00:00", "2026-02-20T14:00:00", "Martin", "_DSI > _INFRA",   0, "Incident"),
            ("Clos",               "2025-12-20T09:00:00", "2026-02-01T09:00:00", "Dupont", "_DSI > _SUPPORT", 0, "Demande"),
            ("Nouveau",            "2026-03-05T08:00:00", "2026-03-05T08:00:00", "Martin", "_DSI > _INFRA",   1, "Incident"),
        ];

        for (i, (statut, date_ouv, date_mod, tech, groupe, vivant, type_t)) in
            tickets.iter().enumerate()
        {
            let ticket_id = (i + 10) as i64;
            conn.execute(
                "INSERT INTO tickets (
                    id, import_id, titre, statut, type_ticket, demandeur,
                    date_ouverture, derniere_modification, est_vivant,
                    technicien_principal, groupe_principal, groupe_niveau1,
                    anciennete_jours, inactivite_jours, nombre_suivis
                 ) VALUES (?1, ?2, 'T', ?3, ?4, 'u', ?5, ?6, ?7, ?8, ?9, '_DSI', 10, 0, 0)",
                rusqlite::params![ticket_id, import_id, statut, type_t, date_ouv, date_mod,
                                  vivant, tech, groupe],
            )
            .unwrap();
        }

        (conn, import_id)
    }

    // ─── get_bilan_entrees_par_periode ───────────────────────────────────────

    #[test]
    fn test_entrees_mois_periode_complete() {
        // Période : tout janvier + février 2026
        // T10 (jan), T11 (jan), T12 (fev), T13 (fev) → 4 entrants
        // T14 (dec 2025) et T15 (mars 2026) sont hors période
        let (conn, _) = setup_bilan();
        let rows = get_bilan_entrees_par_periode(&conn, "2026-01-01", "2026-02-28", "month", None)
            .unwrap();
        assert_eq!(rows.len(), 2);
        let jan = rows.iter().find(|(p, _)| p == "2026-01").unwrap();
        let feb = rows.iter().find(|(p, _)| p == "2026-02").unwrap();
        assert_eq!(jan.1, 2); // T10, T11
        assert_eq!(feb.1, 2); // T12, T13
    }

    #[test]
    fn test_entrees_mois_periode_etroite() {
        // Seul janvier → T14 exclus (dec 2025), T12/T13/T15 exclus (fev+)
        let (conn, _) = setup_bilan();
        let rows = get_bilan_entrees_par_periode(&conn, "2026-01-01", "2026-01-31", "month", None)
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "2026-01");
        assert_eq!(rows[0].1, 2);
    }

    #[test]
    fn test_entrees_quarter() {
        // Jan + Fev = Q1 → 4 entrants dans un seul bucket
        let (conn, _) = setup_bilan();
        let rows = get_bilan_entrees_par_periode(&conn, "2026-01-01", "2026-02-28", "quarter", None)
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "2026-Q1");
        assert_eq!(rows[0].1, 4);
    }

    #[test]
    fn test_entrees_week() {
        // Chaque ticket tombe dans une semaine distincte
        // T10 (2026-01-05 = lun, W01), T11 (2026-01-15 = jeu, W02)
        // T12 (2026-02-03 = mar, W05), T13 (2026-02-10 = mar, W06)
        let (conn, _) = setup_bilan();
        let rows = get_bilan_entrees_par_periode(&conn, "2026-01-01", "2026-02-28", "week", None)
            .unwrap();
        assert_eq!(rows.len(), 4);
        // Vérifie l'ordre croissant des semaines
        assert!(rows[0].0 < rows[1].0);
        assert!(rows[1].0 < rows[2].0);
        assert!(rows[2].0 < rows[3].0);
        // Chaque semaine a 1 ticket
        for (_, count) in &rows {
            assert_eq!(*count, 1);
        }
    }

    #[test]
    fn test_entrees_avec_filtre_type() {
        // Filtre Incident : T10 (jan), T11 (jan), T13 (fev) → jan=2, fev=1
        // T12 (Demande) est exclu
        let (conn, _) = setup_bilan();
        let f = StockFilters {
            statut: None,
            type_ticket: Some("Incident".to_string()),
            groupe: None,
            min_anciennete: None,
            max_anciennete: None,
        };
        let rows = get_bilan_entrees_par_periode(&conn, "2026-01-01", "2026-02-28", "month", Some(&f))
            .unwrap();
        assert_eq!(rows.len(), 2);
        let jan = rows.iter().find(|(p, _)| p == "2026-01").unwrap();
        let feb = rows.iter().find(|(p, _)| p == "2026-02").unwrap();
        assert_eq!(jan.1, 2); // T10, T11
        assert_eq!(feb.1, 1); // T13 seulement (T12 est Demande)
    }

    #[test]
    fn test_entrees_periode_vide_retourne_rien() {
        // Aucun ticket entre le 1er et 3 jan (T10 est le 5 jan)
        let (conn, _) = setup_bilan();
        let rows = get_bilan_entrees_par_periode(&conn, "2026-01-01", "2026-01-04", "month", None)
            .unwrap();
        assert!(rows.is_empty());
    }

    // ─── get_bilan_sorties_par_periode ───────────────────────────────────────

    #[test]
    fn test_sorties_mois_periode_complete() {
        // Période : jan + fev 2026
        // Sorties : T11 (Clos, derniere_mod=2026-01-25) → jan
        //           T13 (Résolu, derniere_mod=2026-02-20) → fev
        //           T14 (Clos, derniere_mod=2026-02-01) → fev
        let (conn, _) = setup_bilan();
        let rows = get_bilan_sorties_par_periode(&conn, "2026-01-01", "2026-02-28", "month", None)
            .unwrap();
        assert_eq!(rows.len(), 2);
        let jan = rows.iter().find(|(p, _)| p == "2026-01").unwrap();
        let feb = rows.iter().find(|(p, _)| p == "2026-02").unwrap();
        assert_eq!(jan.1, 1); // T11
        assert_eq!(feb.1, 2); // T13 + T14
    }

    #[test]
    fn test_sorties_seul_janvier() {
        // Seul janvier → 1 sortant (T11, 2026-01-25)
        // T14 a derniere_mod en fev → exclu, T13 en fev → exclu
        let (conn, _) = setup_bilan();
        let rows = get_bilan_sorties_par_periode(&conn, "2026-01-01", "2026-01-31", "month", None)
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "2026-01");
        assert_eq!(rows[0].1, 1);
    }

    #[test]
    fn test_sorties_quarter() {
        // Jan + Fev Q1 → 3 sortants dans un seul bucket
        let (conn, _) = setup_bilan();
        let rows = get_bilan_sorties_par_periode(&conn, "2026-01-01", "2026-02-28", "quarter", None)
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "2026-Q1");
        assert_eq!(rows[0].1, 3);
    }

    #[test]
    fn test_sorties_periode_sans_sortant_retourne_rien() {
        // Aucun ticket clos avant le 20 jan
        let (conn, _) = setup_bilan();
        let rows = get_bilan_sorties_par_periode(&conn, "2026-01-01", "2026-01-19", "month", None)
            .unwrap();
        assert!(rows.is_empty());
    }

    // ─── get_stock_at_date ───────────────────────────────────────────────────

    #[test]
    fn test_stock_at_date_avant_toute_ouverture() {
        // Aucun ticket ouvert avant le 2025-12-01 (T14 est le 2025-12-20)
        let (conn, _) = setup_bilan();
        let stock = get_stock_at_date(&conn, "2025-12-01").unwrap();
        assert_eq!(stock, 0);
    }

    #[test]
    fn test_stock_at_date_inclut_seulement_vivants() {
        // Au 2026-01-31 : tickets ouverts avant cette date : T10 (jan 5, vivant), T11 (jan 15, mort), T14 (dec 20, mort)
        // Seul T10 est vivant → stock = 1
        let (conn, _) = setup_bilan();
        let stock = get_stock_at_date(&conn, "2026-01-31").unwrap();
        assert_eq!(stock, 1);
    }

    #[test]
    fn test_stock_at_date_fin_fevrier() {
        // Au 2026-02-28 : T10 (vivant, jan 5), T12 (vivant, fev 3), T15 non (mars)
        // T11 (mort), T13 (mort), T14 (mort)
        // vivants ouverts <= 2026-02-28 : T10 + T12 → 2
        let (conn, _) = setup_bilan();
        let stock = get_stock_at_date(&conn, "2026-02-28").unwrap();
        assert_eq!(stock, 2);
    }

    #[test]
    fn test_stock_at_date_inclut_date_exacte() {
        // T10 est ouvert le 2026-01-05T09:00:00, date "2026-01-05" doit l'inclure
        let (conn, _) = setup_bilan();
        let stock = get_stock_at_date(&conn, "2026-01-05").unwrap();
        assert_eq!(stock, 1);
    }

    // ─── get_bilan_ventilation_par_technicien ────────────────────────────────

    #[test]
    fn test_ventilation_technicien_periode_complete() {
        // Période 2026-01-01 / 2026-02-28
        // Entrants : T10 (Dupont), T11 (Dupont), T12 (Martin), T13 (Martin) → Dup=2, Mar=2
        // Sortants : T11 (Dupont, jan 25), T13 (Martin, fev 20), T14 (Dupont, fev 1) → Dup=2, Mar=1
        let (conn, _) = setup_bilan();
        let rows = get_bilan_ventilation_par_technicien(&conn, "2026-01-01", "2026-02-28").unwrap();

        assert_eq!(rows.len(), 2);
        let dupont = rows.iter().find(|(t, _, _)| t == "Dupont").unwrap();
        let martin = rows.iter().find(|(t, _, _)| t == "Martin").unwrap();

        assert_eq!(dupont.1, 2); // entrants
        assert_eq!(dupont.2, 2); // sortants
        assert_eq!(martin.1, 2); // entrants
        assert_eq!(martin.2, 1); // sortants

        // Dupont total=4, Martin total=3 → Dupont en premier
        assert_eq!(rows[0].0, "Dupont");
    }

    #[test]
    fn test_ventilation_technicien_seul_janvier() {
        // Période jan uniquement
        // Entrants : T10 (Dupont), T11 (Dupont) → Dupont=2
        // Sortants : T11 (Dupont, jan 25) → Dupont=1
        // Martin n'a aucune activité en jan → absent
        let (conn, _) = setup_bilan();
        let rows = get_bilan_ventilation_par_technicien(&conn, "2026-01-01", "2026-01-31").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "Dupont");
        assert_eq!(rows[0].1, 2);
        assert_eq!(rows[0].2, 1);
    }

    // ─── get_bilan_ventilation_par_groupe ────────────────────────────────────

    #[test]
    fn test_ventilation_groupe_periode_complete() {
        // Entrants : T10 (_SUPPORT), T11 (_SUPPORT), T12 (_INFRA), T13 (_INFRA)
        // Sortants : T11 (_SUPPORT), T13 (_INFRA), T14 (_SUPPORT)
        // _SUPPORT : entrants=2, sortants=2, total=4
        // _INFRA   : entrants=2, sortants=1, total=3
        let (conn, _) = setup_bilan();
        let rows = get_bilan_ventilation_par_groupe(&conn, "2026-01-01", "2026-02-28").unwrap();
        assert_eq!(rows.len(), 2);
        let support = rows.iter().find(|(g, _, _)| g == "_DSI > _SUPPORT").unwrap();
        let infra = rows.iter().find(|(g, _, _)| g == "_DSI > _INFRA").unwrap();

        assert_eq!(support.1, 2);
        assert_eq!(support.2, 2);
        assert_eq!(infra.1, 2);
        assert_eq!(infra.2, 1);

        // _SUPPORT total=4 > _INFRA total=3 → _SUPPORT en premier
        assert_eq!(rows[0].0, "_DSI > _SUPPORT");
    }

    #[test]
    fn test_ventilation_groupe_hors_periode_retourne_vide() {
        // Période avant toute activité
        let (conn, _) = setup_bilan();
        let rows = get_bilan_ventilation_par_groupe(&conn, "2024-01-01", "2024-12-31").unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn test_bilan_sans_import_actif_retourne_erreur() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("sql/001_initial.sql")).unwrap();
        assert!(get_bilan_entrees_par_periode(&conn, "2026-01-01", "2026-01-31", "month", None).is_err());
        assert!(get_bilan_sorties_par_periode(&conn, "2026-01-01", "2026-01-31", "month", None).is_err());
        assert!(get_stock_at_date(&conn, "2026-01-01").is_err());
        assert!(get_bilan_ventilation_par_technicien(&conn, "2026-01-01", "2026-01-31").is_err());
        assert!(get_bilan_ventilation_par_groupe(&conn, "2026-01-01", "2026-01-31").is_err());
    }
}
