use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub seuil_tickets_technicien: u32,
    pub seuil_anciennete_cloturer: u32,
    pub seuil_inactivite_cloturer: u32,
    pub seuil_anciennete_relancer: u32,
    pub seuil_inactivite_relancer: u32,
    pub seuil_couleur_vert: u32,
    pub seuil_couleur_jaune: u32,
    pub seuil_couleur_orange: u32,
    pub statuts_vivants: Vec<String>,
    pub statuts_termines: Vec<String>,
}

pub fn get_config_from_db(conn: &Connection) -> Result<AppConfig, rusqlite::Error> {
    let mut stmt = conn.prepare_cached("SELECT key, value FROM config")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut config = AppConfig {
        seuil_tickets_technicien: 20,
        seuil_anciennete_cloturer: 90,
        seuil_inactivite_cloturer: 60,
        seuil_anciennete_relancer: 30,
        seuil_inactivite_relancer: 14,
        seuil_couleur_vert: 10,
        seuil_couleur_jaune: 20,
        seuil_couleur_orange: 40,
        statuts_vivants: vec![
            "Nouveau".into(),
            "En cours (Attribué)".into(),
            "En cours (Planifié)".into(),
            "En attente".into(),
        ],
        statuts_termines: vec!["Clos".into(), "Résolu".into()],
    };

    for row in rows {
        let (key, value) = row?;
        match key.as_str() {
            "seuil_tickets_technicien" => {
                config.seuil_tickets_technicien = value.parse().unwrap_or(20)
            }
            "seuil_anciennete_cloturer" => {
                config.seuil_anciennete_cloturer = value.parse().unwrap_or(90)
            }
            "seuil_inactivite_cloturer" => {
                config.seuil_inactivite_cloturer = value.parse().unwrap_or(60)
            }
            "seuil_anciennete_relancer" => {
                config.seuil_anciennete_relancer = value.parse().unwrap_or(30)
            }
            "seuil_inactivite_relancer" => {
                config.seuil_inactivite_relancer = value.parse().unwrap_or(14)
            }
            "seuil_couleur_vert" => config.seuil_couleur_vert = value.parse().unwrap_or(10),
            "seuil_couleur_jaune" => config.seuil_couleur_jaune = value.parse().unwrap_or(20),
            "seuil_couleur_orange" => config.seuil_couleur_orange = value.parse().unwrap_or(40),
            "statuts_vivants" => {
                if let Ok(v) = serde_json::from_str(&value) {
                    config.statuts_vivants = v;
                }
            }
            "statuts_termines" => {
                if let Ok(v) = serde_json::from_str(&value) {
                    config.statuts_termines = v;
                }
            }
            _ => {}
        }
    }

    Ok(config)
}

pub fn update_config_in_db(conn: &Connection, config: &AppConfig) -> Result<(), rusqlite::Error> {
    let pairs: Vec<(&str, String)> = vec![
        (
            "seuil_tickets_technicien",
            config.seuil_tickets_technicien.to_string(),
        ),
        (
            "seuil_anciennete_cloturer",
            config.seuil_anciennete_cloturer.to_string(),
        ),
        (
            "seuil_inactivite_cloturer",
            config.seuil_inactivite_cloturer.to_string(),
        ),
        (
            "seuil_anciennete_relancer",
            config.seuil_anciennete_relancer.to_string(),
        ),
        (
            "seuil_inactivite_relancer",
            config.seuil_inactivite_relancer.to_string(),
        ),
        (
            "seuil_couleur_vert",
            config.seuil_couleur_vert.to_string(),
        ),
        (
            "seuil_couleur_jaune",
            config.seuil_couleur_jaune.to_string(),
        ),
        (
            "seuil_couleur_orange",
            config.seuil_couleur_orange.to_string(),
        ),
        (
            "statuts_vivants",
            serde_json::to_string(&config.statuts_vivants).unwrap_or_default(),
        ),
        (
            "statuts_termines",
            serde_json::to_string(&config.statuts_termines).unwrap_or_default(),
        ),
    ];

    let mut stmt = conn.prepare_cached(
        "INSERT OR REPLACE INTO config (key, value, updated_at) VALUES (?1, ?2, datetime('now'))",
    )?;

    for (key, value) in pairs {
        stmt.execute(rusqlite::params![key, value])?;
    }

    Ok(())
}
