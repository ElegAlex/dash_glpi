# Validation documentation — Wave 0
**Date :** 2026-03-01
**Validateur :** T4 Validator

---

## VERDICT GLOBAL : PASS ✓ (après corrections)

Verdict initial : FAIL. Corrections appliquées le 2026-03-01 par team-lead.
3 écarts corrigés : story-to-module mapping, cohérence RAG seuils, ipc-commands types manquants.

---

## Détail par point

### 1. CLAUDE.md — format et sections

**Résultat : OK**

- Lignes : 56 (< 80 ✓)
- Sections présentes : Stack ✓, Structure ✓, Conventions ✓, Commandes ✓, Agent Teams ✓, Références ✓
- Écart mineur : "linfa-clustering 0.8" en ligne 4 alors que stack.md spécifie "0.8.1" → cohérence interne à améliorer mais non bloquant.

---

### 2. Epics EP01–EP08 — ≥ 3 stories avec GIVEN/WHEN/THEN testables

**Résultat : OK**

| Epic | Stories | Format GWT | Critères chiffrés |
|------|---------|------------|-------------------|
| EP01 | 5 (US001–US005) | ✓ | ✓ |
| EP02 | 4 (US006–US009) | ✓ | ✓ |
| EP03 | 3 (US010–US012) | ✓ | ✓ |
| EP04 | 3 (US013–US015) | ✓ | ✓ |
| EP05 | 4 (US016–US019) | ✓ | ✓ |
| EP06 | 4 (US020–US023) | ✓ | ✓ |
| EP07 | 4 (US024–US027) | ✓ | ✓ |
| EP08 | 4 (US028–US031) | ✓ | ✓ |

Tous les GIVEN/WHEN/THEN sont concrets et testables unitairement.

---

### 3. Stories → modules docs/architecture/structure.md

**Résultat : ÉCART BLOQUANT**

Plusieurs stories citent des fichiers absents de structure.md. Toute implémentation Wave 1 en aveugle créerait des fichiers avec des noms divergents.

| Story | Module cité dans epic | Module réel (structure.md) |
|-------|-----------------------|---------------------------|
| US006, US007, US009 | `src/pages/StockDashboard.tsx` | `src/pages/StockPage.tsx` |
| US008 | `src/pages/TechnicianDetail.tsx` | **absent** de structure.md |
| US010 | `src/pages/CategoriesView.tsx` | `src/pages/CategoriesPage.tsx` |
| US013, US014, US022, US027 | `src/pages/BilanView.tsx` | `src/pages/BilanPage.tsx` |
| US028–US031 | `src/pages/TimelineView.tsx` | **absent** de structure.md |
| US006 | `src/components/KpiCard.tsx` | `src/components/stock/KpiCards.tsx` |
| US011 | `src/components/DrillBreadcrumb.tsx` | **absent** (structure.md liste `CategoryDrilldown.tsx`) |
| US014, US004 | `src/components/BilanChart.tsx` | **absent** de structure.md |
| US015 | `src/components/DateRangeWithPresets.tsx` | `src/components/shared/DateRangePicker.tsx` |
| US020, US023 | `src/components/ExportPanel.tsx` | **absent** (structure.md liste `ExportButton.tsx` dans shared/) |

**Corrections nécessaires :**
- Option A : Aligner les modules cibles des epics sur les noms de structure.md (rectifier les epics)
- Option B : Enrichir structure.md avec les pages/composants manquants (TechnicianDetail, TimelineView, DrillBreadcrumb, BilanChart, ExportPanel) — ce qui est probablement la bonne approche car ces modules sont légitimes

---

### 4. .claude/kb/business-rules.md — couverture règles métier

**Résultat : OK avec réserve**

| Règle requise | Présente | Détail |
|---------------|----------|--------|
| Classification vivant/terminé (RG-005) | ✓ | 6 statuts, tableau complet, champ `est_vivant` |
| Seuils RAG (RG-007) | ✓ | 4 tiers : vert/jaune/orange/rouge, valeurs 10/20/40 |
| Poids priorité (RG-006) | ✓ | Tableau 1–5, 6 libellés |
| Parsing hiérarchie (RG-011) | ✓ | Séparateur ` > `, 3 niveaux, HTML entities, exemples réels |
| Normalisation dates (RG-001, RG-014) | ✓ | `%d-%m-%Y %H:%M` → ISO 8601 `%Y-%m-%dT%H:%M:%S` |
| Détection zombies (RG-010) | ✓ | `nombre_suivis = 0` → action `'qualifier'` |

**Réserve (non bloquant) :** EP02 utilise "Ambre" et 3 tiers (vert ≤ 10, ambre 11-30, rouge > 30) alors que la KB et le Segment 2 définissent 4 tiers (vert/jaune/orange/rouge). Voir point 3 et point de cohérence ci-dessous.

---

### 5. .claude/kb/schema.md — conformité Segment 2

**Résultat : OK**

Vérification ligne à ligne par grep du Segment 2 :

| Élément | schema.md | Segment 2 | Statut |
|---------|-----------|-----------|--------|
| Tables (6) | imports, tickets, tickets_fts, config, keyword_dictionaries, analytics_cache | Identique | ✓ |
| Index tickets (12) | idx_tickets_import, _vivant, _statut, _technicien, _groupe, _groupe_n1, _groupe_n2, _type, _date_ouv, _date_modif, _anciennete, _categorie | Identique (lignes 598–609) | ✓ |
| FTS5 | `tokenize='unicode61 remove_diacritics 2'`, content='tickets', content_rowid='rowid' | Identique (ligne 614) | ✓ |
| Trigger `trg_single_active_import` | AFTER UPDATE OF is_active ON imports | Identique (ligne 537) | ✓ |
| Triggers FTS5 `trg_tickets_ai`, `trg_tickets_ad` | ✓ | Identique (lignes 625, 630) | ✓ |
| PRAGMAs (7) | WAL, NORMAL, -64000, ON, 5000, MEMORY, 268435456 | Identique | ✓ |
| rusqlite features | bundled, fallible_uint, cache | Identique (ligne 248) | ✓ |
| Migrations via PRAGMA user_version | ✓ | ✓ | ✓ |

Total index : 12 (tickets) + 1 (analytics_cache) = 13 ≥ 12 ✓

---

### 6. .claude/kb/column-mapping.md — chemin CSV→Raw→Normalized→SQLite

**Résultat : OK**

- 17 colonnes CSV tracées (dont Catégorie optionnelle) ✓
- 4 étapes pour chaque : CSV header → GlpiTicketRaw (Serde) → GlpiTicketNormalized → tickets SQLite ✓
- Champs calculés documentés séparément (technicien_principal, groupe_principal, anciennete_jours, groupe_niveau1/2/3, est_vivant, date_cloture_approx) ✓
- Désérialiseurs custom documentés avec signatures ✓
- Colonnes obligatoires vs optionnelles (const REQUIRED/OPTIONAL) ✓
- CsvImportResult struct documentée ✓
- Configuration ReaderBuilder complète ✓

---

### 7. .claude/kb/ipc-commands.md — commandes Tauri avec types

**Résultat : ÉCART MINEUR**

Commandes présentes avec types I/O :

| Commande | Input typé | Output typé | Statut |
|----------|-----------|-------------|--------|
| import_csv | path: String, on_progress: Channel | ImportResult | ✓ |
| get_import_history | — | Vec<ImportRecord> | ✓ |
| compare_imports | import_id_a/b: i64 | ImportComparison | ✓ |
| get_stock_overview | — | StockOverview | ✓ |
| get_stock_by_technician | filters: Option<StockFilters> | Vec<TechnicianStock> | ✓ |
| get_stock_by_group | filters: Option<StockFilters> | Vec<GroupStock> | ✓ |
| get_ticket_detail | ticket_id: u64 | TicketDetail | ✓ |
| get_technician_tickets | technician: String, filters | Vec<TicketSummary> | ✓ |
| get_bilan_temporel | BilanRequest | BilanTemporel | ✓ |
| get_categories_tree | CategoriesRequest | CategoryTree | ✓ |
| run_text_analysis | TextAnalysisRequest | TextAnalysisResult | ✓ |
| **get_clusters** | `// paramètres K-Means` (commentaire) | ClusterResult | **ÉCART** |
| **detect_anomalies** | — | Vec<AnomalyAlert> | struct manquante |
| export_excel_stock | — | ExportResult | ✓ |
| export_excel_bilan | BilanRequest | ExportResult | ✓ |
| export_excel_plan_action | technician: String | ExportResult | ✓ |
| get_config | — | AppConfig | ✓ |
| update_config | AppConfig | () | ✓ |

**Corrections nécessaires :**
- `get_clusters` : définir la struct `ClusterRequest` avec les paramètres K-Means (k_min, k_max, n_iterations)
- `detect_anomalies` : définir la struct `AnomalyAlert` avec ses champs (ticket_id, type, z_score, valeur)

---

### 8. docs/architecture/stack.md — versions exactes

**Résultat : OK**

Toutes les crates Rust ont des versions précises (ex : linfa-clustering 0.8.1, augurs 0.10.1, kneed 1.0). Tous les packages frontend ont des versions semver précises. Les sections Dev/Test et Futures sont présentes. Les PRAGMAs SQLite et la configuration Cargo.toml lib section sont documentés.

---

### 9. docs/architecture/structure.md — arborescence complète avec rôles

**Résultat : OK (mais insuffisant face aux écarts du point 3)**

L'arborescence est complète pour les modules listés : tous les fichiers Rust par module (parser/, db/, analyzer/, nlp/, analytics/, export/, commands/), toutes les pages et composants frontend, les hooks, stores, types. Le graphe de dépendances inter-modules et le mapping teammates sont présents.

**Insuffisance :** Les pages `TechnicianDetail.tsx` et `TimelineView.tsx`, et les composants `DrillBreadcrumb.tsx`, `BilanChart.tsx`, `ExportPanel.tsx` sont référencés par les epics mais absents de structure.md. Cela crée l'écart constaté au point 3.

---

## Écarts bloquants — Corrections requises avant Wave 1

### CORRECTION 1 (Critique) — Aligner structure.md avec les modules référencés dans les epics

Ajouter dans structure.md les entrées manquantes :

**Pages manquantes :**
```
| TechnicianDetail.tsx | `/stock/:technicien` | Liste tickets + plan d'action d'un technicien |
| TimelineView.tsx     | `/timeline`          | Suivi longitudinal multi-imports              |
```

**Composants manquants :**
```
stock/     : BilanChart.tsx    → graphique flux entrants/sortants (partagé bilan)
categories/: DrillBreadcrumb.tsx → fil d'Ariane pour le drill-down
shared/    : ExportPanel.tsx   → panneau d'export (choix type + options)
            BilanChart.tsx     → graphique tendance flux temporels
```
OU : rectifier les epics pour pointer vers les noms corrects (StockPage, CategoriesPage, BilanPage, DateRangePicker, KpiCards, CategoryDrilldown, ExportButton).

### CORRECTION 2 (Modérée) — Harmoniser RAG seuils entre EP02 et KB

EP02 (RG-019) dit : "Vert ≤ 10, Ambre 11-30, Rouge > 30" (3 tiers, terme "Ambre").
KB business-rules.md (RG-007) dit : 4 tiers (vert/jaune/orange/rouge, seuils 10/20/40).

La KB et le Segment 2 font référence. Rectifier EP02 RG-019 pour aligner sur les 4 tiers de la KB.

### CORRECTION 3 (Mineure) — Compléter ipc-commands.md

- `get_clusters` : définir struct `ClusterRequest { k_min: usize, k_max: usize, n_iterations: usize }`
- `detect_anomalies` : définir struct `AnomalyAlert { ticket_id: u64, anomaly_type: String, z_score: f64, value_days: f64 }`

---

## Récapitulatif

| Point | Statut | Sévérité |
|-------|--------|----------|
| 1. CLAUDE.md | OK | — |
| 2. Epics 3+ stories GWT | OK | — |
| 3. Stories → modules structure.md | **ÉCART** | 🔴 Bloquant |
| 4. business-rules.md couverture | OK (réserve) | 🟡 Mineur |
| 5. schema.md vs Segment 2 | OK | — |
| 6. column-mapping.md | OK | — |
| 7. ipc-commands.md types | **ÉCART** | 🟡 Mineur |
| 8. stack.md versions | OK | — |
| 9. structure.md arborescence | OK (incomplet) | 🔴 Bloquant via pt 3 |

**Action requise avant lancement Wave 1 (task #5) :** appliquer les corrections 1 et 2 (correction 3 peut être faite en parallèle).
