# Segment 2 — Architecture Tauri 2 : commandes, state, IPC, SQLite

**Référence technique complète pour la refonte V2 du GLPI Dashboard**

---

## 1. Structure complète du projet

### 1.1 Initialisation

```bash
pnpm create tauri-app glpi-dashboard --template react-ts
cd glpi-dashboard
pnpm add react-router zustand recharts @tanstack/react-table lucide-react date-fns
pnpm add -D @types/node
```

### 1.2 Arborescence

```
glpi-dashboard/
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs                  # Point d'entrée Tauri 2 (PAS main.rs)
│   │   ├── state.rs                # AppState + trait DbAccess
│   │   ├── error.rs                # Types d'erreur avec thiserror
│   │   ├── commands/
│   │   │   ├── mod.rs
│   │   │   ├── import.rs           # import_csv
│   │   │   ├── stock.rs            # get_stock_overview, get_stock_by_technician, etc.
│   │   │   ├── bilan.rs            # get_bilan_temporel
│   │   │   ├── categories.rs       # get_categories_tree
│   │   │   ├── mining.rs           # run_text_analysis, get_clusters, detect_anomalies
│   │   │   ├── export.rs           # export_excel_stock, export_excel_bilan, etc.
│   │   │   └── config.rs           # get_config, update_config
│   │   ├── parser/
│   │   │   ├── mod.rs
│   │   │   ├── types.rs            # GlpiTicketRaw, GlpiTicketNormalized
│   │   │   ├── deserializers.rs    # Désérialiseurs custom Serde
│   │   │   ├── columns.rs          # ColumnMap, validation
│   │   │   └── pipeline.rs         # parse_csv() orchestrateur
│   │   ├── db/
│   │   │   ├── mod.rs
│   │   │   ├── setup.rs            # init_db(), PRAGMAs, migrations
│   │   │   ├── migrations.rs       # Schéma versionné
│   │   │   ├── insert.rs           # bulk_insert_tickets()
│   │   │   └── queries.rs          # Requêtes métier (stock, bilan, catégories)
│   │   ├── analyzer/
│   │   │   ├── mod.rs
│   │   │   ├── stock.rs            # Calculs KPI stock
│   │   │   ├── bilan.rs            # Calculs flux entrée/sortie
│   │   │   ├── classifier.rs       # Classification automatique des tickets
│   │   │   └── temporal.rs         # Agrégation jour/semaine/mois
│   │   ├── nlp/
│   │   │   ├── mod.rs
│   │   │   ├── preprocessing.rs    # Tokenization, stemming, stop words
│   │   │   ├── tfidf.rs            # Vectorisation TF-IDF
│   │   │   ├── clustering.rs       # K-Means
│   │   │   └── patterns.rs         # Détection patterns résolution/relance
│   │   └── export/
│   │       ├── mod.rs
│   │       ├── stock_report.rs     # Export Excel tableau de bord stock
│   │       ├── bilan_report.rs     # Export Excel bilan d'activité
│   │       └── plan_action.rs      # Export Excel plan d'action individuel
│   ├── capabilities/
│   │   └── default.json            # Permissions Tauri 2
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/
│   ├── main.tsx                    # Point d'entrée React
│   ├── App.tsx                     # Router principal
│   ├── hooks/
│   │   ├── useInvoke.ts            # Hook typé pour invoke()
│   │   └── useImport.ts            # Hook import CSV avec progress
│   ├── stores/
│   │   ├── appStore.ts             # Zustand : import actif, filtres globaux
│   │   └── settingsStore.ts        # Zustand : config utilisateur
│   ├── types/
│   │   ├── tickets.ts              # Miroir des structs Rust
│   │   ├── kpi.ts                  # Types KPI/analytics
│   │   └── config.ts               # Types configuration
│   ├── pages/
│   │   ├── ImportPage.tsx
│   │   ├── StockPage.tsx
│   │   ├── BilanPage.tsx
│   │   ├── CategoriesPage.tsx
│   │   ├── MiningPage.tsx
│   │   ├── ExportPage.tsx
│   │   └── SettingsPage.tsx
│   ├── components/
│   │   ├── layout/
│   │   │   ├── Sidebar.tsx
│   │   │   └── PageHeader.tsx
│   │   ├── stock/
│   │   │   ├── KpiCards.tsx
│   │   │   ├── TechnicianTable.tsx
│   │   │   ├── GroupTable.tsx
│   │   │   └── TicketDetailModal.tsx
│   │   ├── bilan/
│   │   │   ├── PeriodSelector.tsx
│   │   │   ├── FlowChart.tsx
│   │   │   └── BilanTechnicianTable.tsx
│   │   ├── categories/
│   │   │   ├── CategoryTreemap.tsx
│   │   │   └── CategoryDrilldown.tsx
│   │   ├── mining/
│   │   │   ├── WordCloud.tsx
│   │   │   ├── ClusterView.tsx
│   │   │   └── AnomalyList.tsx
│   │   └── shared/
│   │       ├── DataTable.tsx        # Wrapper TanStack Table
│   │       ├── DateRangePicker.tsx
│   │       └── ExportButton.tsx
│   └── lib/
│       └── utils.ts                # Helpers (formatDate, etc.)
├── index.html
├── package.json
├── tsconfig.json
├── vite.config.ts
└── postcss.config.js              # Tailwind CSS 4
```

### 1.3 Point d'entrée Tauri 2 : `lib.rs`

**IMPORTANT** : Tauri 2 utilise `lib.rs`, pas `main.rs`. Le `main.rs` ne fait que `glpi_dashboard_lib::run()`.

```rust
// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    glpi_dashboard_lib::run();
}
```

```rust
// src-tauri/src/lib.rs
mod commands;
mod db;
mod error;
mod state;
mod parser;
mod analyzer;
mod nlp;
mod export;

use state::AppState;
use std::sync::Mutex;

pub fn run() {
    let app_state = AppState {
        db: Mutex::new(None),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(|app| {
            // Initialiser la base SQLite au démarrage
            let app_handle = app.handle().clone();
            let db_path = app_handle
                .path()
                .app_data_dir()
                .expect("Impossible de résoudre app_data_dir")
                .join("glpi_dashboard.db");

            // Créer le dossier si nécessaire
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let conn = db::setup::init_db(db_path.to_str().unwrap())?;

            let state: tauri::State<AppState> = app.state();
            *state.db.lock().unwrap() = Some(conn);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Import
            commands::import::import_csv,
            // Stock
            commands::stock::get_stock_overview,
            commands::stock::get_stock_by_technician,
            commands::stock::get_stock_by_group,
            commands::stock::get_ticket_detail,
            commands::stock::get_technician_tickets,
            // Bilan
            commands::bilan::get_bilan_temporel,
            // Catégories
            commands::categories::get_categories_tree,
            // Data mining
            commands::mining::run_text_analysis,
            commands::mining::get_clusters,
            commands::mining::detect_anomalies,
            // Export
            commands::export::export_excel_stock,
            commands::export::export_excel_bilan,
            commands::export::export_excel_plan_action,
            // Config
            commands::config::get_config,
            commands::config::update_config,
            // Historique
            commands::import::get_import_history,
            commands::import::compare_imports,
        ])
        .run(tauri::generate_context!())
        .expect("Erreur au lancement de l'application");
}
```

### 1.4 Cargo.toml complet

```toml
[package]
name = "glpi-dashboard"
version = "0.1.0"
edition = "2021"

[lib]
name = "glpi_dashboard_lib"
crate-type = ["lib", "cdylib", "staticlib"]

[build-dependencies]
tauri-build = { version = "2.5", features = [] }

[dependencies]
# Tauri core + plugins — versions vérifiées le 28/02/2026
tauri = { version = "2.10", features = [] }
tauri-plugin-dialog = "2.4"
tauri-plugin-fs = "2.4"
tauri-plugin-notification = "2.3"
tauri-plugin-shell = "2.2"

# Sérialisation
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# CSV parsing — 1.4.0 (oct 2025), gère BOM UTF-8 nativement
csv = "1.4"

# Dates
chrono = { version = "0.4", features = ["serde"] }

# SQLite — 0.38.0 bundle SQLite 3.51.1, FTS5 activé par défaut avec bundled
# BREAKING CHANGE 0.38: u64/usize ToSql/FromSql désactivé par défaut → fallible_uint obligatoire
# BREAKING CHANGE 0.38: statement cache optionnel → feature "cache" pour prepare_cached()
# fallible_uint permet u64 si <= i64::MAX (nos IDs GLPI ~5M, largement OK)
rusqlite = { version = "0.38", features = ["bundled", "fallible_uint", "cache"] }

# Erreurs
thiserror = "2"

# NLP / text mining
regex = "1"
rust-stemmers = "1.2"
unicode-normalization = "0.1"

# Excel export — 0.93 supporte charts embarqués, checkboxes, sparklines
rust_xlsxwriter = "0.93"

# Logging
log = "0.4"
env_logger = "0.11"

# Async
tokio = { version = "1", features = ["full"] }
```

### 1.5 Capabilities Tauri 2 (`capabilities/default.json`)

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "GLPI Dashboard capabilities",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:default",
    "dialog:allow-open",
    "dialog:allow-save",
    "fs:default",
    "fs:allow-read",
    "fs:allow-write",
    "notification:default",
    "notification:allow-notify",
    "shell:default",
    "shell:allow-open"
  ]
}
```

### 1.6 tauri.conf.json (Tauri 2 format)

```json
{
  "$schema": "https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-utils/schema.json",
  "productName": "GLPI Dashboard",
  "version": "0.1.0",
  "identifier": "fr.cpam92.glpi-dashboard",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "pnpm dev",
    "beforeBuildCommand": "pnpm build"
  },
  "app": {
    "windows": [
      {
        "title": "GLPI Dashboard — DSI CPAM 92",
        "width": 1440,
        "height": 900,
        "minWidth": 1024,
        "minHeight": 700,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": "default-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self' data:"
    }
  },
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "windows": {
      "nsis": {
        "installMode": "perMachine"
      }
    }
  }
}
```

---

## 2. State management Rust

### 2.1 AppState

```rust
// src-tauri/src/state.rs
use rusqlite::Connection;
use std::sync::Mutex;

/// State global de l'application.
/// Tauri wrappe automatiquement dans un Arc — NE PAS faire Arc<Mutex<T>>.
/// Utiliser Mutex<Option<Connection>> car la connexion est initialisée dans setup().
pub struct AppState {
    pub db: Mutex<Option<Connection>>,
}

/// Trait pour accéder facilement à la connexion depuis n'importe quel contexte.
/// Évite de répéter le pattern lock().unwrap().as_ref().unwrap() partout.
pub trait DbAccess {
    fn db<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>;

    fn db_mut<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut Connection) -> Result<T, rusqlite::Error>;
}

impl DbAccess for AppState {
    fn db<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    {
        let guard = self.db.lock().map_err(|e| format!("Mutex poisoned: {}", e))?;
        let conn = guard.as_ref().ok_or("Base de données non initialisée")?;
        f(conn).map_err(|e| format!("Erreur SQLite: {}", e))
    }

    fn db_mut<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut Connection) -> Result<T, rusqlite::Error>,
    {
        let mut guard = self.db.lock().map_err(|e| format!("Mutex poisoned: {}", e))?;
        let conn = guard.as_mut().ok_or("Base de données non initialisée")?;
        f(conn).map_err(|e| format!("Erreur SQLite: {}", e))
    }
}
```

### 2.2 Pourquoi Mutex et pas RwLock

`rusqlite::Connection` n'est PAS `Sync`. Impossible d'utiliser `RwLock<Connection>` car `RwLock::read()` requiert `T: Sync`. Avec SQLite en mode WAL, les lectures concurrentes ne sont pas un problème de performance car le Mutex est relâché très vite (requêtes < 1ms sur 10K lignes). Si on avait besoin de lectures concurrentes, il faudrait un pool de connexions (r2d2-rusqlite), mais c'est overkill ici.

### 2.3 Accès au state dans les commandes

```rust
use tauri::State;
use crate::state::{AppState, DbAccess};

#[tauri::command]
pub async fn get_stock_overview(
    state: State<'_, AppState>,
) -> Result<StockOverview, String> {
    state.db(|conn| {
        // conn est une &Connection, requêtes directes
        let total_vivants: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tickets WHERE est_vivant = 1 AND import_id = (SELECT MAX(id) FROM imports)",
            [],
            |row| row.get(0),
        )?;
        // ... etc
        Ok(StockOverview { /* ... */ })
    })
}
```

**Règle : toujours `async` pour les commandes.** Les commandes async s'exécutent sur le thread pool Tokio, donc ne bloquent jamais le thread principal (UI). Les commandes sync bloquent le thread principal — à éviter absolument.

### 2.4 Accès au state en dehors des commandes (setup, background tasks)

```rust
// Dans setup() ou un background thread
let app_handle = app.handle().clone();

tokio::spawn(async move {
    let state: tauri::State<AppState> = app_handle.state();
    // utiliser state.db(|conn| { ... })
});
```

---

## 3. Schéma SQLite complet

### 3.1 Initialisation avec PRAGMAs

```rust
// src-tauri/src/db/setup.rs
use rusqlite::Connection;

pub fn init_db(path: &str) -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open(path)?;

    // PRAGMAs critiques — à exécuter à CHAQUE ouverture de connexion
    conn.execute_batch("
        PRAGMA journal_mode = WAL;          -- Write-Ahead Logging : lectures non bloquées par écritures
        PRAGMA synchronous = NORMAL;        -- Bon compromis performance/sécurité avec WAL
        PRAGMA cache_size = -64000;         -- 64 Mo de cache (négatif = kilo-octets)
        PRAGMA foreign_keys = ON;           -- Activer les clés étrangères (désactivé par défaut !)
        PRAGMA busy_timeout = 5000;         -- 5 secondes avant SQLITE_BUSY
        PRAGMA temp_store = MEMORY;         -- Tables temporaires en RAM
        PRAGMA mmap_size = 268435456;       -- 256 Mo de memory-mapped I/O
    ")?;

    // Exécuter les migrations
    run_migrations(&conn)?;

    Ok(conn)
}
```

### 3.2 Système de migrations

```rust
// src-tauri/src/db/migrations.rs
use rusqlite::Connection;

/// Chaque migration a un numéro de version et du SQL.
/// On utilise PRAGMA user_version pour tracker la version actuelle.
struct Migration {
    version: u32,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("sql/001_initial.sql"),
    },
    // Futures migrations :
    // Migration { version: 2, sql: include_str!("sql/002_add_categories.sql") },
];

pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    let current_version: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    for migration in MIGRATIONS {
        if migration.version > current_version {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(migration.sql)?;
            tx.pragma_update(None, "user_version", migration.version)?;
            tx.commit()?;
            log::info!("Migration {} appliquée", migration.version);
        }
    }

    Ok(())
}
```

### 3.3 Schéma SQL initial complet

```sql
-- src-tauri/src/db/sql/001_initial.sql

-- ============================================================
-- TABLE : imports
-- Historique de chaque import CSV. Permet le suivi longitudinal.
-- ============================================================
CREATE TABLE IF NOT EXISTS imports (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    filename            TEXT NOT NULL,
    import_date         TEXT NOT NULL DEFAULT (datetime('now')),
    file_size_bytes     INTEGER,
    total_rows          INTEGER NOT NULL,
    parsed_rows         INTEGER NOT NULL,
    skipped_rows        INTEGER NOT NULL DEFAULT 0,
    vivants_count       INTEGER NOT NULL,
    termines_count      INTEGER NOT NULL,
    date_range_from     TEXT,                -- Plus ancienne date d'ouverture
    date_range_to       TEXT,                -- Plus récente date d'ouverture
    detected_columns    TEXT NOT NULL,        -- JSON array des colonnes détectées
    unique_statuts      TEXT NOT NULL,        -- JSON array des statuts uniques
    unique_types        TEXT NOT NULL,        -- JSON array des types uniques
    parse_duration_ms   INTEGER,
    is_active           INTEGER NOT NULL DEFAULT 1  -- Import actuellement sélectionné
);

-- Un seul import actif à la fois
CREATE TRIGGER IF NOT EXISTS trg_single_active_import
    AFTER UPDATE OF is_active ON imports
    WHEN NEW.is_active = 1
BEGIN
    UPDATE imports SET is_active = 0 WHERE id != NEW.id AND is_active = 1;
END;

-- ============================================================
-- TABLE : tickets
-- Tous les tickets importés, avec colonnes calculées.
-- ============================================================
CREATE TABLE IF NOT EXISTS tickets (
    -- Clé primaire composite : un même ticket peut exister dans plusieurs imports
    id                      INTEGER NOT NULL,
    import_id               INTEGER NOT NULL REFERENCES imports(id) ON DELETE CASCADE,

    -- Champs bruts du CSV
    titre                   TEXT NOT NULL DEFAULT '',
    statut                  TEXT NOT NULL,
    type_ticket             TEXT NOT NULL DEFAULT '',          -- Incident / Demande
    priorite                INTEGER,
    urgence                 INTEGER,
    demandeur               TEXT NOT NULL DEFAULT '',
    date_ouverture          TEXT NOT NULL,                     -- ISO 8601
    derniere_modification   TEXT,                              -- ISO 8601
    nombre_suivis           INTEGER DEFAULT 0,
    suivis_description      TEXT NOT NULL DEFAULT '',
    solution                TEXT NOT NULL DEFAULT '',
    taches_description      TEXT NOT NULL DEFAULT '',
    intervention_fournisseur TEXT NOT NULL DEFAULT '',

    -- Champs multi-valeurs (stockés en JSON array)
    techniciens             TEXT NOT NULL DEFAULT '[]',        -- JSON array
    groupes                 TEXT NOT NULL DEFAULT '[]',        -- JSON array

    -- Champs dénormalisés pour les requêtes rapides
    technicien_principal    TEXT,                              -- Premier technicien
    groupe_principal        TEXT,                              -- Premier groupe complet
    groupe_niveau1          TEXT,                              -- "_DSI"
    groupe_niveau2          TEXT,                              -- "_SUPPORT UTILISATEURS..."
    groupe_niveau3          TEXT,                              -- "_SUPPORT - PARC"

    -- Colonne optionnelle (future)
    categorie               TEXT,                              -- Catégorie ITIL si présente
    categorie_niveau1       TEXT,
    categorie_niveau2       TEXT,

    -- Colonnes calculées
    est_vivant              INTEGER NOT NULL DEFAULT 0,        -- 0 = Clos/Résolu, 1 = vivant
    anciennete_jours        INTEGER,                           -- Jours depuis ouverture
    inactivite_jours        INTEGER,                           -- Jours depuis dernière modif
    date_cloture_approx     TEXT,                              -- = dernière modif si terminé

    -- Classification automatique
    action_recommandee      TEXT,                              -- 'cloturer', 'relancer', 'qualifier', 'escalader'
    motif_classification    TEXT,                              -- Explication de la classification

    PRIMARY KEY (id, import_id)
);

-- Index pour les requêtes fréquentes
CREATE INDEX IF NOT EXISTS idx_tickets_import          ON tickets(import_id);
CREATE INDEX IF NOT EXISTS idx_tickets_vivant          ON tickets(import_id, est_vivant);
CREATE INDEX IF NOT EXISTS idx_tickets_statut          ON tickets(import_id, statut);
CREATE INDEX IF NOT EXISTS idx_tickets_technicien      ON tickets(import_id, technicien_principal);
CREATE INDEX IF NOT EXISTS idx_tickets_groupe          ON tickets(import_id, groupe_principal);
CREATE INDEX IF NOT EXISTS idx_tickets_groupe_n1       ON tickets(import_id, groupe_niveau1);
CREATE INDEX IF NOT EXISTS idx_tickets_groupe_n2       ON tickets(import_id, groupe_niveau2);
CREATE INDEX IF NOT EXISTS idx_tickets_type            ON tickets(import_id, type_ticket);
CREATE INDEX IF NOT EXISTS idx_tickets_date_ouv        ON tickets(import_id, date_ouverture);
CREATE INDEX IF NOT EXISTS idx_tickets_date_modif      ON tickets(import_id, derniere_modification);
CREATE INDEX IF NOT EXISTS idx_tickets_anciennete      ON tickets(import_id, est_vivant, anciennete_jours);
CREATE INDEX IF NOT EXISTS idx_tickets_categorie       ON tickets(import_id, categorie_niveau1, categorie_niveau2);

-- ============================================================
-- TABLE VIRTUELLE FTS5 : recherche full-text dans les tickets
-- ============================================================
CREATE VIRTUAL TABLE IF NOT EXISTS tickets_fts USING fts5(
    titre,
    suivis_description,
    solution,
    taches_description,
    content='tickets',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'  -- Normalise les accents français
);

-- Triggers pour maintenir la FTS synchronisée
CREATE TRIGGER IF NOT EXISTS trg_tickets_ai AFTER INSERT ON tickets BEGIN
    INSERT INTO tickets_fts(rowid, titre, suivis_description, solution, taches_description)
    VALUES (NEW.rowid, NEW.titre, NEW.suivis_description, NEW.solution, NEW.taches_description);
END;

CREATE TRIGGER IF NOT EXISTS trg_tickets_ad AFTER DELETE ON tickets BEGIN
    INSERT INTO tickets_fts(tickets_fts, rowid, titre, suivis_description, solution, taches_description)
    VALUES ('delete', OLD.rowid, OLD.titre, OLD.suivis_description, OLD.solution, OLD.taches_description);
END;

-- ============================================================
-- TABLE : config
-- Configuration utilisateur (clé/valeur), persistée entre sessions.
-- ============================================================
CREATE TABLE IF NOT EXISTS config (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Valeurs par défaut
INSERT OR IGNORE INTO config (key, value) VALUES
    ('seuil_tickets_technicien', '20'),
    ('seuil_anciennete_cloturer', '90'),
    ('seuil_inactivite_cloturer', '60'),
    ('seuil_anciennete_relancer', '30'),
    ('seuil_inactivite_relancer', '14'),
    ('seuil_couleur_vert', '10'),
    ('seuil_couleur_jaune', '20'),
    ('seuil_couleur_orange', '40'),
    ('statuts_vivants', '["Nouveau","En cours (Attribué)","En cours (Planifié)","En attente"]'),
    ('statuts_termines', '["Clos","Résolu"]');

-- ============================================================
-- TABLE : keyword_dictionaries
-- Dictionnaires de mots-clés pour l'analyse textuelle.
-- L'utilisateur peut ajouter/modifier depuis l'interface.
-- ============================================================
CREATE TABLE IF NOT EXISTS keyword_dictionaries (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    category    TEXT NOT NULL,       -- 'resolution', 'relance', 'annulation', 'exclusion'
    keyword     TEXT NOT NULL,
    is_regex    INTEGER NOT NULL DEFAULT 0,  -- 0 = mot exact, 1 = regex
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(category, keyword)
);

-- Dictionnaire par défaut : résolutions implicites
INSERT OR IGNORE INTO keyword_dictionaries (category, keyword) VALUES
    ('resolution', 'résolu'),
    ('resolution', 'réglé'),
    ('resolution', 'terminé'),
    ('resolution', 'effectué'),
    ('resolution', 'remplacé'),
    ('resolution', 'installé'),
    ('resolution', 'livré'),
    ('resolution', 'configuré'),
    ('resolution', 'fonctionnel'),
    ('resolution', 'c''est bon'),
    ('resolution', 're-fonctionne'),
    ('resolution', 'refonctionne'),
    ('resolution', 'opérationnel'),
    ('resolution', 'corrigé'),
    ('resolution', 'mis à jour'),
    ('resolution', 'déployé'),
    ('resolution', 'activé'),
    ('resolution', 'débloqu');

-- Dictionnaire par défaut : relances
INSERT OR IGNORE INTO keyword_dictionaries (category, keyword) VALUES
    ('relance', 'toujours d''actualité'),
    ('relance', 'impossibilité de vous joindre'),
    ('relance', 'sans nouvelles'),
    ('relance', 'relance'),
    ('relance', 'en attente de retour'),
    ('relance', 'merci de confirmer');

-- Dictionnaire par défaut : annulations/doublons
INSERT OR IGNORE INTO keyword_dictionaries (category, keyword) VALUES
    ('annulation', 'annulé'),
    ('annulation', 'doublon'),
    ('annulation', 'obsolète'),
    ('annulation', 'plus d''actualité'),
    ('annulation', 'ne plus traiter');

-- Dictionnaire par défaut : exclusions (faux positifs à ignorer)
INSERT OR IGNORE INTO keyword_dictionaries (category, keyword) VALUES
    ('exclusion', 'pièce jointe liée lors de la création'),
    ('exclusion', 'ticket créé automatiquement'),
    ('exclusion', 'mail collecteur');

-- ============================================================
-- TABLE : analytics_cache
-- Cache des résultats d'analyse lourds (text mining, clusters).
-- Invalidé quand l'import actif change.
-- ============================================================
CREATE TABLE IF NOT EXISTS analytics_cache (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    import_id       INTEGER NOT NULL REFERENCES imports(id) ON DELETE CASCADE,
    analysis_type   TEXT NOT NULL,       -- 'tfidf', 'clusters', 'anomalies', 'keywords'
    parameters      TEXT NOT NULL,       -- JSON des paramètres utilisés
    result          TEXT NOT NULL,       -- JSON du résultat
    computed_at     TEXT NOT NULL DEFAULT (datetime('now')),
    duration_ms     INTEGER,
    UNIQUE(import_id, analysis_type, parameters)
);

CREATE INDEX IF NOT EXISTS idx_cache_import ON analytics_cache(import_id);
```

### 3.4 Bulk insert performant

```rust
// src-tauri/src/db/insert.rs
use rusqlite::Connection;
use crate::parser::types::GlpiTicketNormalized;

/// Insère tous les tickets d'un import en une seule transaction.
/// 10 000 lignes avec 20+ colonnes : ~200-400ms avec WAL + prepare_cached.
pub fn bulk_insert_tickets(
    conn: &mut Connection,
    import_id: i64,
    tickets: &[GlpiTicketNormalized],
) -> Result<usize, rusqlite::Error> {
    let tx = conn.transaction()?;

    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR REPLACE INTO tickets (
                id, import_id, titre, statut, type_ticket, priorite, urgence,
                demandeur, date_ouverture, derniere_modification, nombre_suivis,
                suivis_description, solution, taches_description, intervention_fournisseur,
                techniciens, groupes,
                technicien_principal, groupe_principal,
                groupe_niveau1, groupe_niveau2, groupe_niveau3,
                categorie, categorie_niveau1, categorie_niveau2,
                est_vivant, anciennete_jours, inactivite_jours, date_cloture_approx,
                action_recommandee, motif_classification
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                ?8, ?9, ?10, ?11,
                ?12, ?13, ?14, ?15,
                ?16, ?17,
                ?18, ?19,
                ?20, ?21, ?22,
                ?23, ?24, ?25,
                ?26, ?27, ?28, ?29,
                ?30, ?31
            )"
        )?;

        for t in tickets {
            stmt.execute(rusqlite::params![
                t.id,
                import_id,
                t.titre,
                t.statut,
                t.type_ticket,
                t.priorite,
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
```

**Pourquoi `prepare_cached()` ?** Le statement préparé est compilé une seule fois et réutilisé pour chaque itération. Sans cache, chaque insert recompile le SQL → ~10x plus lent.

**Pourquoi une seule transaction ?** Sans transaction explicite, SQLite crée une transaction implicite par INSERT → un `fsync()` par ligne → 10 000 lignes ≈ 300+ secondes. Avec une seule transaction : un seul `fsync()` à la fin → < 500ms.

### 3.5 Recherche full-text avec FTS5

```rust
// Exemple de requête FTS5
pub fn search_tickets(
    conn: &Connection,
    import_id: i64,
    query: &str,
    limit: usize,
) -> Result<Vec<TicketSearchResult>, rusqlite::Error> {
    let mut stmt = conn.prepare_cached(
        "SELECT t.id, t.titre, t.statut, t.technicien_principal,
                snippet(tickets_fts, 0, '<b>', '</b>', '...', 32) as titre_highlight,
                snippet(tickets_fts, 2, '<b>', '</b>', '...', 64) as solution_highlight,
                rank
         FROM tickets_fts
         JOIN tickets t ON t.rowid = tickets_fts.rowid
         WHERE tickets_fts MATCH ?1
           AND t.import_id = ?2
         ORDER BY rank
         LIMIT ?3"
    )?;

    let rows = stmt.query_map(
        rusqlite::params![query, import_id, limit as i64],
        |row| {
            Ok(TicketSearchResult {
                id: row.get(0)?,
                titre: row.get(1)?,
                statut: row.get(2)?,
                technicien: row.get(3)?,
                titre_highlight: row.get(4)?,
                solution_highlight: row.get(5)?,
                rank: row.get(6)?,
            })
        },
    )?;

    rows.collect()
}
```

Le tokenizer `unicode61 remove_diacritics 2` normalise automatiquement les accents : une recherche "resolu" trouvera "résolu". L'option `2` signifie "remove diacritics for ASCII-equivalent characters".

---

## 4. Commandes Tauri : signatures complètes et types

### 4.1 Types d'erreur

```rust
// src-tauri/src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Erreur d'entrée/sortie: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erreur CSV: {0}")]
    Csv(#[from] csv::Error),

    #[error("Erreur SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Erreur de sérialisation: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Colonnes obligatoires manquantes: {}", .0.join(", "))]
    MissingColumns(Vec<String>),

    #[error("Fichier vide ou sans données")]
    EmptyFile,

    #[error("Import introuvable: {0}")]
    ImportNotFound(i64),

    #[error("{0}")]
    Custom(String),
}

// Les commandes Tauri requièrent Result<T, String> ou un type impl Serialize.
// On implémente Serialize sur AppError pour pouvoir le retourner directement.
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
```

### 4.2 Types de données partagés (structs Serialize)

```rust
// Types utilisés par les commandes — dans les modules respectifs
// Tous doivent implémenter Serialize pour le retour IPC
// et Deserialize pour les arguments entrants.

use serde::{Deserialize, Serialize};

// ---- IMPORT ----

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum ImportEvent {
    #[serde(rename_all = "camelCase")]
    Progress {
        rows_parsed: usize,
        total_estimated: usize,
        phase: String,          // "parsing", "normalizing", "inserting", "indexing"
    },
    #[serde(rename_all = "camelCase")]
    Complete {
        duration_ms: u64,
        total_tickets: usize,
        vivants: usize,
        termines: usize,
    },
    #[serde(rename_all = "camelCase")]
    Warning {
        line: usize,
        message: String,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub import_id: i64,
    pub total_tickets: usize,
    pub vivants_count: usize,
    pub termines_count: usize,
    pub skipped_rows: usize,
    pub warnings: Vec<ParseWarning>,
    pub detected_columns: Vec<String>,
    pub missing_optional_columns: Vec<String>,
    pub unique_statuts: Vec<String>,
    pub parse_duration_ms: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseWarning {
    pub line: usize,
    pub message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRecord {
    pub id: i64,
    pub filename: String,
    pub import_date: String,
    pub total_rows: usize,
    pub vivants_count: usize,
    pub termines_count: usize,
    pub date_range_from: Option<String>,
    pub date_range_to: Option<String>,
    pub is_active: bool,
}

// ---- STOCK ----

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockOverview {
    pub total_vivants: usize,
    pub total_termines: usize,
    pub par_statut: Vec<StatutCount>,         // Nombre par statut distinct
    pub age_moyen_jours: f64,                 // Moyenne des vivants uniquement
    pub age_median_jours: f64,
    pub par_type: TypeBreakdown,
    pub par_anciennete: Vec<AgeRangeCount>,   // >30j, >60j, >90j, >180j, >365j
    pub inactifs_14j: usize,
    pub inactifs_30j: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatutCount {
    pub statut: String,
    pub count: usize,
    pub est_vivant: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeBreakdown {
    pub incidents: usize,
    pub demandes: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgeRangeCount {
    pub label: String,          // ">30j", ">60j", etc.
    pub threshold_days: usize,
    pub count: usize,
    pub percentage: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockFilters {
    pub statut: Option<String>,
    pub type_ticket: Option<String>,
    pub groupe: Option<String>,
    pub min_anciennete: Option<i64>,
    pub max_anciennete: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TechnicianStock {
    pub technicien: String,
    pub total: usize,
    pub en_cours: usize,
    pub en_attente: usize,
    pub planifie: usize,
    pub nouveau: usize,
    pub incidents: usize,
    pub demandes: usize,
    pub age_moyen_jours: f64,
    pub inactifs_14j: usize,
    pub ecart_seuil: i64,          // total - seuil (négatif = sous le seuil)
    pub couleur_seuil: String,     // "vert", "jaune", "orange", "rouge"
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupStock {
    pub groupe: String,
    pub groupe_niveau1: String,
    pub groupe_niveau2: Option<String>,
    pub total: usize,
    pub en_cours: usize,
    pub en_attente: usize,
    pub incidents: usize,
    pub demandes: usize,
    pub nb_techniciens: usize,
    pub age_moyen_jours: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketSummary {
    pub id: u64,
    pub titre: String,
    pub statut: String,
    pub type_ticket: String,
    pub technicien_principal: Option<String>,
    pub groupe_principal: Option<String>,
    pub date_ouverture: String,
    pub derniere_modification: Option<String>,
    pub anciennete_jours: Option<i64>,
    pub inactivite_jours: Option<i64>,
    pub nombre_suivis: Option<u32>,
    pub action_recommandee: Option<String>,
    pub motif_classification: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketDetail {
    pub id: u64,
    pub titre: String,
    pub statut: String,
    pub type_ticket: String,
    pub priorite: Option<u8>,
    pub urgence: Option<u8>,
    pub demandeur: String,
    pub techniciens: Vec<String>,
    pub groupes: Vec<String>,
    pub date_ouverture: String,
    pub derniere_modification: Option<String>,
    pub nombre_suivis: Option<u32>,
    pub suivis_description: String,
    pub solution: String,
    pub taches_description: String,
    pub anciennete_jours: Option<i64>,
    pub inactivite_jours: Option<i64>,
    pub action_recommandee: Option<String>,
    pub motif_classification: Option<String>,
    pub categorie: Option<String>,
}

// ---- BILAN TEMPOREL ----

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BilanRequest {
    pub period: String,              // "day", "week", "month"
    pub date_from: String,           // ISO 8601
    pub date_to: String,             // ISO 8601
    pub group_by: Option<String>,    // "technicien", "groupe", "type"
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BilanTemporel {
    pub periodes: Vec<PeriodData>,
    pub totaux: BilanTotaux,
    pub ventilation: Option<Vec<BilanVentilation>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeriodData {
    pub period_key: String,          // "2025-01-06", "2025-S02", "2025-01"
    pub period_label: String,        // "06/01/2025", "Sem. 2", "Janvier 2025"
    pub entrees: usize,              // Tickets créés sur la période
    pub sorties: usize,              // Tickets clos/résolus sur la période
    pub delta: i64,                  // entrees - sorties
    pub stock_cumule: Option<usize>, // Stock estimé à la fin de la période
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BilanTotaux {
    pub total_entrees: usize,
    pub total_sorties: usize,
    pub delta_global: i64,
    pub moyenne_entrees_par_periode: f64,
    pub moyenne_sorties_par_periode: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BilanVentilation {
    pub label: String,               // Nom du technicien/groupe/type
    pub entrees: usize,
    pub sorties: usize,
    pub delta: i64,
}

// ---- CATÉGORIES ----

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoriesRequest {
    pub scope: String,               // "vivants", "tous", "termines"
    pub source: Option<String>,      // "groupe" (défaut) ou "categorie" (si disponible)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryTree {
    pub source: String,              // "groupe" ou "categorie"
    pub nodes: Vec<CategoryNode>,
    pub total_tickets: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryNode {
    pub name: String,
    pub full_path: String,
    pub level: usize,
    pub count: usize,
    pub percentage: f64,
    pub incidents: usize,
    pub demandes: usize,
    pub age_moyen: f64,
    pub children: Vec<CategoryNode>,
}

// ---- DATA MINING ----

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextAnalysisRequest {
    pub corpus: String,              // "titres", "suivis", "solutions", "all"
    pub scope: String,               // "vivants", "tous", "termines"
    pub group_by: Option<String>,    // "groupe", "technicien", "categorie"
    pub top_n: Option<usize>,        // Nombre de mots-clés à retourner (défaut 50)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextAnalysisResult {
    pub keywords: Vec<KeywordFrequency>,
    pub by_group: Option<Vec<GroupKeywords>>,
    pub corpus_stats: CorpusStats,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeywordFrequency {
    pub word: String,
    pub count: usize,
    pub tfidf_score: f64,
    pub doc_frequency: usize,        // Dans combien de tickets le mot apparaît
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupKeywords {
    pub group_name: String,
    pub keywords: Vec<KeywordFrequency>,
    pub ticket_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorpusStats {
    pub total_documents: usize,
    pub total_tokens: usize,
    pub vocabulary_size: usize,
    pub avg_tokens_per_doc: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterResult {
    pub clusters: Vec<Cluster>,
    pub silhouette_score: f64,
    pub total_tickets: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cluster {
    pub id: usize,
    pub label: String,               // Généré à partir des top keywords
    pub top_keywords: Vec<String>,
    pub ticket_count: usize,
    pub ticket_ids: Vec<u64>,
    pub avg_resolution_days: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnomalyAlert {
    pub ticket_id: u64,
    pub titre: String,
    pub anomaly_type: String,        // "delai_anormal", "categorie_inhabituelle", "dormant"
    pub severity: String,            // "info", "warning", "critical"
    pub description: String,
    pub metric_value: f64,
    pub expected_range: String,
}

// ---- EXPORT ----

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    pub size_bytes: u64,
    pub duration_ms: u64,
}

// ---- CONFIG ----

#[derive(Serialize, Deserialize)]
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

// ---- COMPARAISON IMPORTS ----

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportComparison {
    pub import_a: ImportRecord,
    pub import_b: ImportRecord,
    pub delta_total: i64,
    pub delta_vivants: i64,
    pub nouveaux_tickets: Vec<u64>,       // IDs apparus dans B mais pas dans A
    pub disparus_tickets: Vec<u64>,       // IDs dans A mais pas dans B
    pub delta_par_technicien: Vec<TechnicianDelta>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TechnicianDelta {
    pub technicien: String,
    pub count_a: usize,
    pub count_b: usize,
    pub delta: i64,
}
```

### 4.3 Signatures des commandes

```rust
// src-tauri/src/commands/import.rs
use tauri::ipc::Channel;

#[tauri::command]
pub async fn import_csv(
    state: tauri::State<'_, AppState>,
    path: String,
    on_progress: Channel<ImportEvent>,
) -> Result<ImportResult, String> { /* ... */ }

#[tauri::command]
pub async fn get_import_history(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ImportRecord>, String> { /* ... */ }

#[tauri::command]
pub async fn compare_imports(
    state: tauri::State<'_, AppState>,
    import_id_a: i64,
    import_id_b: i64,
) -> Result<ImportComparison, String> { /* ... */ }
```

```rust
// src-tauri/src/commands/stock.rs

#[tauri::command]
pub async fn get_stock_overview(
    state: tauri::State<'_, AppState>,
) -> Result<StockOverview, String> { /* ... */ }

#[tauri::command]
pub async fn get_stock_by_technician(
    state: tauri::State<'_, AppState>,
    filters: Option<StockFilters>,
) -> Result<Vec<TechnicianStock>, String> { /* ... */ }

#[tauri::command]
pub async fn get_stock_by_group(
    state: tauri::State<'_, AppState>,
    filters: Option<StockFilters>,
) -> Result<Vec<GroupStock>, String> { /* ... */ }

#[tauri::command]
pub async fn get_ticket_detail(
    state: tauri::State<'_, AppState>,
    ticket_id: u64,
) -> Result<TicketDetail, String> { /* ... */ }

#[tauri::command]
pub async fn get_technician_tickets(
    state: tauri::State<'_, AppState>,
    technician: String,
    filters: Option<StockFilters>,
) -> Result<Vec<TicketSummary>, String> { /* ... */ }
```

```rust
// src-tauri/src/commands/bilan.rs

#[tauri::command]
pub async fn get_bilan_temporel(
    state: tauri::State<'_, AppState>,
    request: BilanRequest,
) -> Result<BilanTemporel, String> { /* ... */ }
```

```rust
// src-tauri/src/commands/categories.rs

#[tauri::command]
pub async fn get_categories_tree(
    state: tauri::State<'_, AppState>,
    request: CategoriesRequest,
) -> Result<CategoryTree, String> { /* ... */ }
```

```rust
// src-tauri/src/commands/mining.rs

#[tauri::command]
pub async fn run_text_analysis(
    state: tauri::State<'_, AppState>,
    request: TextAnalysisRequest,
) -> Result<TextAnalysisResult, String> { /* ... */ }

#[tauri::command]
pub async fn get_clusters(
    state: tauri::State<'_, AppState>,
    corpus: String,        // "titres", "suivis", "solutions"
    n_clusters: usize,
) -> Result<ClusterResult, String> { /* ... */ }

#[tauri::command]
pub async fn detect_anomalies(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AnomalyAlert>, String> { /* ... */ }
```

```rust
// src-tauri/src/commands/export.rs

#[tauri::command]
pub async fn export_excel_stock(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<ExportResult, String> { /* ... */ }

#[tauri::command]
pub async fn export_excel_bilan(
    state: tauri::State<'_, AppState>,
    path: String,
    request: BilanRequest,
) -> Result<ExportResult, String> { /* ... */ }

#[tauri::command]
pub async fn export_excel_plan_action(
    state: tauri::State<'_, AppState>,
    path: String,
    technician: String,
) -> Result<ExportResult, String> { /* ... */ }
```

```rust
// src-tauri/src/commands/config.rs

#[tauri::command]
pub async fn get_config(
    state: tauri::State<'_, AppState>,
) -> Result<AppConfig, String> { /* ... */ }

#[tauri::command]
pub async fn update_config(
    state: tauri::State<'_, AppState>,
    config: AppConfig,
) -> Result<(), String> { /* ... */ }
```

---

## 5. Frontend TypeScript : types miroirs et hooks

### 5.1 Types TypeScript (miroir exact des structs Rust)

```typescript
// src/types/tickets.ts

export interface ImportEvent {
  event: 'progress' | 'complete' | 'warning';
  data: ProgressData | CompleteData | WarningData;
}

export interface ProgressData {
  rowsParsed: number;
  totalEstimated: number;
  phase: 'parsing' | 'normalizing' | 'inserting' | 'indexing';
}

export interface CompleteData {
  durationMs: number;
  totalTickets: number;
  vivants: number;
  termines: number;
}

export interface WarningData {
  line: number;
  message: string;
}

export interface ImportResult {
  importId: number;
  totalTickets: number;
  vivantsCount: number;
  terminesCount: number;
  skippedRows: number;
  warnings: ParseWarning[];
  detectedColumns: string[];
  missingOptionalColumns: string[];
  uniqueStatuts: string[];
  parseDurationMs: number;
}

export interface ParseWarning {
  line: number;
  message: string;
}

export interface ImportRecord {
  id: number;
  filename: string;
  importDate: string;
  totalRows: number;
  vivantsCount: number;
  terminesCount: number;
  dateRangeFrom: string | null;
  dateRangeTo: string | null;
  isActive: boolean;
}

export interface StockOverview {
  totalVivants: number;
  totalTermines: number;
  parStatut: StatutCount[];
  ageMoyenJours: number;
  ageMedianJours: number;
  parType: TypeBreakdown;
  parAnciennete: AgeRangeCount[];
  inactifs14j: number;
  inactifs30j: number;
}

export interface StatutCount {
  statut: string;
  count: number;
  estVivant: boolean;
}

export interface TypeBreakdown {
  incidents: number;
  demandes: number;
}

export interface AgeRangeCount {
  label: string;
  thresholdDays: number;
  count: number;
  percentage: number;
}

export interface TechnicianStock {
  technicien: string;
  total: number;
  enCours: number;
  enAttente: number;
  planifie: number;
  nouveau: number;
  incidents: number;
  demandes: number;
  ageMoyenJours: number;
  inactifs14j: number;
  ecartSeuil: number;
  couleurSeuil: 'vert' | 'jaune' | 'orange' | 'rouge';
}

export interface GroupStock {
  groupe: string;
  groupeNiveau1: string;
  groupeNiveau2: string | null;
  total: number;
  enCours: number;
  enAttente: number;
  incidents: number;
  demandes: number;
  nbTechniciens: number;
  ageMoyenJours: number;
}

export interface TicketSummary {
  id: number;
  titre: string;
  statut: string;
  typeTicket: string;
  technicienPrincipal: string | null;
  groupePrincipal: string | null;
  dateOuverture: string;
  derniereModification: string | null;
  ancienneteJours: number | null;
  inactiviteJours: number | null;
  nombreSuivis: number | null;
  actionRecommandee: string | null;
  motifClassification: string | null;
}

export interface TicketDetail extends TicketSummary {
  priorite: number | null;
  urgence: number | null;
  demandeur: string;
  techniciens: string[];
  groupes: string[];
  suivisDescription: string;
  solution: string;
  tachesDescription: string;
  categorie: string | null;
}

// src/types/kpi.ts

export interface BilanRequest {
  period: 'day' | 'week' | 'month';
  dateFrom: string;
  dateTo: string;
  groupBy?: 'technicien' | 'groupe' | 'type';
}

export interface BilanTemporel {
  periodes: PeriodData[];
  totaux: BilanTotaux;
  ventilation: BilanVentilation[] | null;
}

export interface PeriodData {
  periodKey: string;
  periodLabel: string;
  entrees: number;
  sorties: number;
  delta: number;
  stockCumule: number | null;
}

export interface BilanTotaux {
  totalEntrees: number;
  totalSorties: number;
  deltaGlobal: number;
  moyenneEntreesParPeriode: number;
  moyenneSortiesParPeriode: number;
}

export interface BilanVentilation {
  label: string;
  entrees: number;
  sorties: number;
  delta: number;
}

export interface CategoryTree {
  source: 'groupe' | 'categorie';
  nodes: CategoryNode[];
  totalTickets: number;
}

export interface CategoryNode {
  name: string;
  fullPath: string;
  level: number;
  count: number;
  percentage: number;
  incidents: number;
  demandes: number;
  ageMoyen: number;
  children: CategoryNode[];
}

export interface TextAnalysisResult {
  keywords: KeywordFrequency[];
  byGroup: GroupKeywords[] | null;
  corpusStats: CorpusStats;
}

export interface KeywordFrequency {
  word: string;
  count: number;
  tfidfScore: number;
  docFrequency: number;
}

export interface GroupKeywords {
  groupName: string;
  keywords: KeywordFrequency[];
  ticketCount: number;
}

export interface CorpusStats {
  totalDocuments: number;
  totalTokens: number;
  vocabularySize: number;
  avgTokensPerDoc: number;
}

export interface ClusterResult {
  clusters: Cluster[];
  silhouetteScore: number;
  totalTickets: number;
}

export interface Cluster {
  id: number;
  label: string;
  topKeywords: string[];
  ticketCount: number;
  ticketIds: number[];
  avgResolutionDays: number | null;
}

export interface AnomalyAlert {
  ticketId: number;
  titre: string;
  anomalyType: 'delai_anormal' | 'categorie_inhabituelle' | 'dormant';
  severity: 'info' | 'warning' | 'critical';
  description: string;
  metricValue: number;
  expectedRange: string;
}

export interface ExportResult {
  path: string;
  sizeBytes: number;
  durationMs: number;
}

// src/types/config.ts

export interface AppConfig {
  seuilTicketsTechnicien: number;
  seuilAncienneteCloturer: number;
  seuilInactiviteCloturer: number;
  seuilAncienneteRelancer: number;
  seuilInactiviteRelancer: number;
  seuilCouleurVert: number;
  seuilCouleurJaune: number;
  seuilCouleurOrange: number;
  statutsVivants: string[];
  statutsTermines: string[];
}

export interface ImportComparison {
  importA: ImportRecord;
  importB: ImportRecord;
  deltaTotal: number;
  deltaVivants: number;
  nouveauxTickets: number[];
  disparusTickets: number[];
  deltaParTechnicien: TechnicianDelta[];
}

export interface TechnicianDelta {
  technicien: string;
  countA: number;
  countB: number;
  delta: number;
}
```

### 5.2 Hook useInvoke typé

```typescript
// src/hooks/useInvoke.ts
import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface UseInvokeResult<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
  execute: (...args: unknown[]) => Promise<T>;
  reset: () => void;
}

export function useInvoke<T>(command: string): UseInvokeResult<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const execute = useCallback(async (args?: Record<string, unknown>): Promise<T> => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<T>(command, args);
      setData(result);
      return result;
    } catch (err) {
      const message = typeof err === 'string' ? err : String(err);
      setError(message);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [command]);

  const reset = useCallback(() => {
    setData(null);
    setError(null);
    setLoading(false);
  }, []);

  return { data, loading, error, execute, reset };
}
```

### 5.3 Hook useImport avec Channel progress

```typescript
// src/hooks/useImport.ts
import { useState } from 'react';
import { invoke, Channel } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { ImportResult, ImportEvent, ProgressData } from '../types/tickets';

interface ImportProgress {
  phase: string;
  rowsParsed: number;
  totalEstimated: number;
  percentage: number;
}

export function useImport() {
  const [importing, setImporting] = useState(false);
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [result, setResult] = useState<ImportResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function startImport(): Promise<ImportResult | null> {
    // Ouvrir le sélecteur de fichier
    const path = await open({
      multiple: false,
      filters: [{ name: 'CSV', extensions: ['csv'] }],
    });
    if (!path) return null;

    setImporting(true);
    setError(null);
    setProgress({ phase: 'parsing', rowsParsed: 0, totalEstimated: 0, percentage: 0 });

    try {
      // Créer le channel pour le progress
      const onProgress = new Channel<ImportEvent>();
      onProgress.onmessage = (msg) => {
        if (msg.event === 'progress') {
          const data = msg.data as ProgressData;
          const pct = data.totalEstimated > 0
            ? Math.min((data.rowsParsed / data.totalEstimated) * 100, 99)
            : 0;
          setProgress({
            phase: data.phase,
            rowsParsed: data.rowsParsed,
            totalEstimated: data.totalEstimated,
            percentage: pct,
          });
        }
      };

      const importResult = await invoke<ImportResult>('import_csv', {
        path: path as string,
        onProgress,
      });

      setResult(importResult);
      setProgress({ phase: 'complete', rowsParsed: importResult.totalTickets, totalEstimated: importResult.totalTickets, percentage: 100 });
      return importResult;
    } catch (err) {
      const message = typeof err === 'string' ? err : String(err);
      setError(message);
      return null;
    } finally {
      setImporting(false);
    }
  }

  return { startImport, importing, progress, result, error };
}
```

### 5.4 Routing principal

```typescript
// src/App.tsx
import { BrowserRouter, Routes, Route, Navigate } from 'react-router';
import { Sidebar } from './components/layout/Sidebar';
import { ImportPage } from './pages/ImportPage';
import { StockPage } from './pages/StockPage';
import { BilanPage } from './pages/BilanPage';
import { CategoriesPage } from './pages/CategoriesPage';
import { MiningPage } from './pages/MiningPage';
import { ExportPage } from './pages/ExportPage';
import { SettingsPage } from './pages/SettingsPage';

export default function App() {
  return (
    <BrowserRouter>
      <div className="flex h-screen bg-gray-50">
        <Sidebar />
        <main className="flex-1 overflow-auto">
          <Routes>
            <Route path="/" element={<Navigate to="/import" replace />} />
            <Route path="/import" element={<ImportPage />} />
            <Route path="/stock" element={<StockPage />} />
            <Route path="/bilan" element={<BilanPage />} />
            <Route path="/categories" element={<CategoriesPage />} />
            <Route path="/mining" element={<MiningPage />} />
            <Route path="/export" element={<ExportPage />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  );
}
```

**IMPORTANT** : `import { BrowserRouter } from 'react-router'` — PAS `react-router-dom` (convention du projet, cf. CLAUDE.md).

### 5.5 Zustand store principal

```typescript
// src/stores/appStore.ts
import { create } from 'zustand';
import type { ImportResult, StockOverview } from '../types/tickets';

interface AppStore {
  // Import actif
  activeImportId: number | null;
  lastImportResult: ImportResult | null;
  setActiveImport: (id: number, result?: ImportResult) => void;

  // Cache des données chargées
  stockOverview: StockOverview | null;
  setStockOverview: (overview: StockOverview) => void;

  // Filtres globaux
  globalScope: 'vivants' | 'tous' | 'termines';
  setGlobalScope: (scope: 'vivants' | 'tous' | 'termines') => void;

  // Reset
  clearAll: () => void;
}

export const useAppStore = create<AppStore>((set) => ({
  activeImportId: null,
  lastImportResult: null,
  setActiveImport: (id, result) => set({
    activeImportId: id,
    lastImportResult: result ?? null,
    stockOverview: null, // Invalidate cache
  }),

  stockOverview: null,
  setStockOverview: (overview) => set({ stockOverview: overview }),

  globalScope: 'vivants',
  setGlobalScope: (scope) => set({ globalScope: scope }),

  clearAll: () => set({
    activeImportId: null,
    lastImportResult: null,
    stockOverview: null,
  }),
}));
```

---

## 6. Récapitulatif des décisions d'architecture

|Décision|Choix|Justification|
|---|---|---|
|State Rust|`Mutex<Option<Connection>>`|Connection non Sync, Tauri wrappe dans Arc auto|
|Migrations|`PRAGMA user_version` + SQL files|Simple, pas de dépendance, versionné|
|Bulk insert|Transaction unique + `prepare_cached`|10K lignes en <500ms|
|FTS|FTS5 `unicode61 remove_diacritics 2`|Recherche française sans accents|
|FTS sync|Triggers AFTER INSERT/DELETE|Automatique, pas d'oubli|
|Progress|Tauri Channel API|Typé, ordonné, plus performant qu'events|
|Commandes|Toujours `async`|Ne bloque jamais le thread principal|
|Erreurs|`thiserror` + impl Serialize|Erreurs explicites, sérialisables pour IPC|
|Frontend state|Zustand|Léger, simple, pas de boilerplate|
|Routing|react-router (pas -dom)|Convention projet CLAUDE.md|
|Scope filtre|"vivants" / "tous" / "termines"|Applicable à toutes les vues|
|Colonnes futures|`#[serde(default)]` + ColumnMap|Absorbe l'ajout de "Catégorie" sans modif|