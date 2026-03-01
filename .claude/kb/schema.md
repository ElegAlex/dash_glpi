# Schema SQLite — GLPI Dashboard

Source : Segment 2

---

## PRAGMAs (à exécuter à CHAQUE ouverture)

```sql
PRAGMA journal_mode = WAL;          -- Write-Ahead Logging
PRAGMA synchronous = NORMAL;        -- Bon compromis performance/sécurité avec WAL
PRAGMA cache_size = -64000;         -- 64 Mo de cache (négatif = kilo-octets)
PRAGMA foreign_keys = ON;           -- Activer les clés étrangères (désactivé par défaut !)
PRAGMA busy_timeout = 5000;         -- 5 secondes avant SQLITE_BUSY
PRAGMA temp_store = MEMORY;         -- Tables temporaires en RAM
PRAGMA mmap_size = 268435456;       -- 256 Mo de memory-mapped I/O
```

---

## TABLE : imports

```sql
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
```

---

## TABLE : tickets

```sql
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
```

### Index table tickets

```sql
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
```

---

## TABLE VIRTUELLE FTS5 : tickets_fts

```sql
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
```

---

## TABLE : config

```sql
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
```

---

## TABLE : keyword_dictionaries

```sql
CREATE TABLE IF NOT EXISTS keyword_dictionaries (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    category    TEXT NOT NULL,       -- 'resolution', 'relance', 'annulation', 'exclusion'
    keyword     TEXT NOT NULL,
    is_regex    INTEGER NOT NULL DEFAULT 0,  -- 0 = mot exact, 1 = regex
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(category, keyword)
);
```

---

## TABLE : analytics_cache

```sql
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

---

## Dépendances rusqlite

```toml
# BREAKING CHANGE 0.38: u64/usize ToSql/FromSql désactivé par défaut → fallible_uint obligatoire
# BREAKING CHANGE 0.38: statement cache optionnel → feature "cache" pour prepare_cached()
rusqlite = { version = "0.38", features = ["bundled", "fallible_uint", "cache"] }
```

- **bundled** : inclut SQLite 3.51.1, FTS5 activé par défaut
- **fallible_uint** : permet u64 si <= i64::MAX (IDs GLPI ~5M, largement OK)
- **cache** : active `prepare_cached()` pour les statements réutilisés

## Migrations

Système basé sur `PRAGMA user_version`. Fichiers SQL dans `src-tauri/src/db/sql/`:
- `001_initial.sql` : schéma initial complet (voir ci-dessus)
