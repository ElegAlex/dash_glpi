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
    date_range_from     TEXT,
    date_range_to       TEXT,
    detected_columns    TEXT NOT NULL,
    unique_statuts      TEXT NOT NULL,
    unique_types        TEXT NOT NULL,
    parse_duration_ms   INTEGER,
    is_active           INTEGER NOT NULL DEFAULT 1
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
    id                      INTEGER NOT NULL,
    import_id               INTEGER NOT NULL REFERENCES imports(id) ON DELETE CASCADE,

    titre                   TEXT NOT NULL DEFAULT '',
    statut                  TEXT NOT NULL,
    type_ticket             TEXT NOT NULL DEFAULT '',
    priorite                INTEGER,
    urgence                 INTEGER,
    demandeur               TEXT NOT NULL DEFAULT '',
    date_ouverture          TEXT NOT NULL,
    derniere_modification   TEXT,
    nombre_suivis           INTEGER DEFAULT 0,
    suivis_description      TEXT NOT NULL DEFAULT '',
    solution                TEXT NOT NULL DEFAULT '',
    taches_description      TEXT NOT NULL DEFAULT '',
    intervention_fournisseur TEXT NOT NULL DEFAULT '',

    techniciens             TEXT NOT NULL DEFAULT '[]',
    groupes                 TEXT NOT NULL DEFAULT '[]',

    technicien_principal    TEXT,
    groupe_principal        TEXT,
    groupe_niveau1          TEXT,
    groupe_niveau2          TEXT,
    groupe_niveau3          TEXT,

    categorie               TEXT,
    categorie_niveau1       TEXT,
    categorie_niveau2       TEXT,

    est_vivant              INTEGER NOT NULL DEFAULT 0,
    anciennete_jours        INTEGER,
    inactivite_jours        INTEGER,
    date_cloture_approx     TEXT,

    action_recommandee      TEXT,
    motif_classification    TEXT,

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
    tokenize='unicode61 remove_diacritics 2'
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
-- ============================================================
CREATE TABLE IF NOT EXISTS keyword_dictionaries (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    category    TEXT NOT NULL,
    keyword     TEXT NOT NULL,
    is_regex    INTEGER NOT NULL DEFAULT 0,
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
-- ============================================================
CREATE TABLE IF NOT EXISTS analytics_cache (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    import_id       INTEGER NOT NULL REFERENCES imports(id) ON DELETE CASCADE,
    analysis_type   TEXT NOT NULL,
    parameters      TEXT NOT NULL,
    result          TEXT NOT NULL,
    computed_at     TEXT NOT NULL DEFAULT (datetime('now')),
    duration_ms     INTEGER,
    UNIQUE(import_id, analysis_type, parameters)
);

CREATE INDEX IF NOT EXISTS idx_cache_import ON analytics_cache(import_id);
