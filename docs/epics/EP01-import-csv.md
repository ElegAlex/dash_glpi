# EP01 — Import et parsing CSV

## Description

L'utilisateur importe des fichiers CSV exportés depuis GLPI 9.5+ dans l'application. Le parser valide la structure, gère les anomalies de format (BOM UTF-8, dates françaises, IDs avec espaces insécables, champs multilignes entre guillemets), et produit un rapport d'import détaillé. La progression est reportée en temps réel au frontend via le Channel API Tauri 2.

## Règles métier couvertes

| Règle | Description |
|-------|-------------|
| RG-001 | Le délimiteur CSV est le point-virgule `;` |
| RG-002 | L'encodage est UTF-8 avec ou sans BOM (strippé automatiquement par csv 1.4.0) |
| RG-003 | Les champs multilignes entre guillemets RFC 4180 sont parsés nativement |
| RG-004 | Le champ `ID` contient des espaces insécables : `"5 732 943"` → `5732943u64` |
| RG-005 | Les dates sont au format français `DD-MM-YYYY HH:MM` → `NaiveDateTime` |
| RG-006 | Les champs optionnels vides (`""`) produis `None` pour les types numériques |
| RG-007 | Le champ `Catégorie` est optionnel (absent du CSV actuel) — `#[serde(default)]` |
| RG-008 | Les colonnes obligatoires manquantes déclenchent une erreur fatale |
| RG-009 | Les lignes malformées sont ignorées (skip) sans bloquer l'import |
| RG-010 | Le champ `Attribué à - Technicien` peut contenir plusieurs valeurs séparées par `\n` |
| RG-011 | Le champ `Attribué à - Groupe de techniciens` peut contenir plusieurs groupes séparés par `\n` |
| RG-012 | La progression est reportée tous les 500 lignes via `Channel<ImportEvent>` |
| RG-013 | Un checksum SHA-256 identifie les réimportations du même fichier |
| RG-014 | Le résultat inclut les valeurs uniques de statuts, types et groupes détectés |

## User stories

### US001 — Sélection et validation d'un fichier CSV

**Module cible :** `src-tauri/src/commands/import.rs` + `tauri-plugin-dialog`

**GIVEN** l'utilisateur est sur la page d'import et clique sur "Importer un fichier CSV"
**WHEN** le dialogue natif s'ouvre et l'utilisateur sélectionne un fichier `.csv`
**THEN** l'application vérifie que le fichier est accessible (métadonnées lisibles), affiche le nom et la taille du fichier sélectionné dans l'interface, et active le bouton "Lancer l'import"

**Critères de validation :**
- [ ] Un fichier inexistant produit une erreur `"Fichier inaccessible"` affichée dans l'UI
- [ ] Un fichier vide (0 octet) produit une erreur `CsvImportError::EmptyFile`
- [ ] Annuler le dialogue ne provoque aucune erreur

---

### US002 — Parsing tolérant des lignes CSV

**Module cible :** `src-tauri/src/parser/pipeline.rs`, `src-tauri/src/parser/deserializers.rs`

**GIVEN** un fichier CSV GLPI valide avec des anomalies de format connues (BOM UTF-8, IDs avec espaces, champs multilignes, dates françaises, champs numériques vides)
**WHEN** la commande `import_csv` est invoquée
**THEN** le parser produit un `Vec<GlpiTicketNormalized>` complet sans erreur, les anomalies de format étant traitées silencieusement selon les règles RG-001 à RG-007

**Critères de validation :**
- [ ] `"5 732 943"` → `id: 5_732_943u64` (RG-004)
- [ ] `"05-01-2026 16:24"` → `date_ouverture: NaiveDateTime(2026-01-05T16:24:00)` (RG-005)
- [ ] Champ `priorite` vide `""` → `priorite: None` (RG-006)
- [ ] BOM UTF-8 (`\xEF\xBB\xBF`) en début de fichier → ignoré, parsing normal (RG-002)
- [ ] Champ `Catégorie` absent des en-têtes → `categorie: None` dans tous les tickets (RG-007)
- [ ] `"BLANQUART CHRISTOPHE\nMEY CHETHARITH"` → `techniciens: ["BLANQUART CHRISTOPHE", "MEY CHETHARITH"]` (RG-010)

---

### US003 — Normalisation des données parsées

**Module cible :** `src-tauri/src/parser/pipeline.rs` (fn `normalize_ticket`)

**GIVEN** un `GlpiTicketRaw` désérialisé depuis le CSV
**WHEN** la fonction `normalize_ticket()` est appliquée
**THEN** le `GlpiTicketNormalized` produit contient les dates en ISO 8601, les techniciens et groupes en `Vec<String>`, et les champs calculés `anciennete_jours` et `inactivite_jours` basés sur `chrono::Utc::now().naive_utc()`

**Critères de validation :**
- [ ] `date_ouverture` → `"2026-01-05T16:24:00"` (ISO 8601)
- [ ] `anciennete_jours` > 0 pour tout ticket ouvert dans le passé
- [ ] `inactivite_jours: None` si `derniere_modification` est vide
- [ ] `technicien_principal` = premier élément de `techniciens`
- [ ] `groupe_principal` = premier élément de `groupes`
- [ ] `categorie: None` si la valeur est une chaîne vide

---

### US004 — Progression temps réel et rapport d'import

**Module cible :** `src-tauri/src/commands/import.rs`, `src/pages/ImportPage.tsx`

**GIVEN** l'import d'un fichier CSV de 10 000 lignes est en cours
**WHEN** le parser traite les lignes par blocs de 500
**THEN** le frontend reçoit des événements `ImportEvent::Progress { bytes_read, rows_parsed, total_bytes }` via `Channel<ImportEvent>`, la barre de progression affiche le pourcentage (ratio `bytes_read / total_bytes`), et à la fin un `ImportEvent::Complete { duration_ms }` est émis

**Critères de validation :**
- [ ] Au moins 10 événements Progress pour un fichier de 10 000 lignes
- [ ] Le pourcentage n'atteint jamais 100% avant l'événement Complete
- [ ] Le rapport final affiche : total lignes traitées, lignes ignorées, durée en ms
- [ ] Les avertissements de parsing sont listés avec numéro de ligne et message

---

### US005 — Détection de réimportation par checksum

**Module cible :** `src-tauri/src/db/`, `src-tauri/src/commands/import.rs`

**GIVEN** l'utilisateur tente d'importer un fichier CSV déjà importé précédemment
**WHEN** le checksum SHA-256 du fichier est calculé et comparé aux imports enregistrés en base
**THEN** un avertissement est affiché proposant de continuer ou d'annuler l'import (pas d'erreur bloquante)

**Critères de validation :**
- [ ] Un fichier identique importé deux fois produit un avertissement `"Fichier déjà importé le <date>"`
- [ ] L'utilisateur peut choisir de forcer la réimportation
- [ ] Un fichier différent (même nom, contenu différent) n'est pas signalé comme doublon

## Critères de succès de l'epic

- [ ] Tous les tests unitaires des stories US001 à US005 passent
- [ ] Un fichier CSV GLPI réel (10 000 tickets) est importé en moins de 200 ms (insertion SQLite incluse)
- [ ] Les lignes malformées sont ignorées sans bloquer l'import (RG-009)
- [ ] Le rapport d'import affiche le total, les erreurs et les avertissements (RG-014)
- [ ] La progression est reportée en temps réel via Tauri Channel (RG-012)
