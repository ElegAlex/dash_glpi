# Structure du projet — Arborescence et rôles

## Vue d'ensemble

```
glpi-dashboard/
├── src-tauri/                    # Backend Rust (Tauri 2)
│   ├── src/
│   │   ├── main.rs               # Bootstrap → glpi_dashboard_lib::run()
│   │   ├── lib.rs                # Entry point Tauri 2, enregistre plugins + commandes
│   │   ├── state.rs              # AppState (Mutex<Option<Connection>>) + trait DbAccess
│   │   ├── error.rs              # Enum d'erreurs avec thiserror, Serialize pour IPC
│   │   ├── commands/             # Handlers IPC #[tauri::command]
│   │   ├── parser/               # Parsing CSV GLPI
│   │   ├── db/                   # SQLite : setup, migrations, queries
│   │   ├── analyzer/             # Calculs KPI (stock, bilan, classification)
│   │   ├── nlp/                  # NLP : tokenisation, TF-IDF, stemming
│   │   ├── export/               # Génération Excel XLSX
│   │   └── analytics/            # ML : clustering, anomalies, prédiction
│   ├── capabilities/
│   │   └── default.json          # Permissions Tauri 2 (dialog, fs, notification, shell)
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                          # Frontend React + TypeScript
│   ├── main.tsx                  # Point d'entrée React
│   ├── App.tsx                   # Router principal (React Router)
│   ├── pages/                    # Pages/routes
│   ├── components/               # Composants UI organisés par domaine
│   ├── hooks/                    # Custom hooks (invoke, ECharts, import)
│   ├── stores/                   # Zustand stores
│   ├── types/                    # Types TypeScript miroir des structs Rust
│   └── lib/                      # Utilitaires (formatDate, etc.)
├── index.html
├── package.json
├── tsconfig.json
├── vite.config.ts
└── CLAUDE.md
```

## Backend — Modules Rust détaillés

### commands/ — Handlers IPC
Chaque fichier expose des fonctions `#[tauri::command] pub async fn`.
Toutes les commandes sont `async` (thread pool Tokio, jamais le thread UI).

| Fichier | Commandes | Dépend de |
|---------|-----------|-----------|
| mod.rs | Re-exports | — |
| import.rs | `import_csv`, `get_import_history`, `compare_imports` | parser/, db/ |
| stock.rs | `get_stock_overview`, `get_stock_by_technician`, `get_stock_by_group`, `get_ticket_detail`, `get_technician_tickets` | db/, analyzer/ |
| bilan.rs | `get_bilan_temporel` | db/, analyzer/ |
| categories.rs | `get_categories_tree` | db/, analyzer/ |
| mining.rs | `run_text_analysis`, `get_clusters`, `detect_anomalies` | nlp/, analytics/, db/ |
| export.rs | `export_excel_stock`, `export_excel_bilan`, `export_excel_plan_action` | export/, db/ |
| config.rs | `get_config`, `update_config` | db/ |

### parser/ — Parsing CSV GLPI
Pipeline : lecture → désérialisation Serde → normalisation → CsvImportResult.

| Fichier | Rôle | Dépend de |
|---------|------|-----------|
| mod.rs | Re-exports publics | — |
| types.rs | `GlpiTicketRaw`, `GlpiTicketNormalized`, `CsvImportResult`, `ParseWarning` | chrono, serde |
| deserializers.rs | Module `de::` — dates françaises, IDs spacés, nombres optionnels | chrono |
| columns.rs | `ColumnMap`, validation colonnes obligatoires/optionnelles | — |
| pipeline.rs | `parse_csv()` — orchestrateur avec progress callback | types, deserializers, columns |

### db/ — SQLite
Gestion base de données : migrations versionnées, insertions bulk, requêtes métier.

| Fichier | Rôle | Dépend de |
|---------|------|-----------|
| mod.rs | Re-exports | — |
| setup.rs | `init_db()`, PRAGMAs, appel migrations | migrations |
| migrations.rs | Système versionné via PRAGMA user_version | sql/*.sql |
| insert.rs | `bulk_insert_tickets()` — insertion transactionnelle | parser/types |
| queries.rs | Requêtes métier SQL (stock, bilan, catégories, recherche FTS5) | — |

### analyzer/ — Calculs KPI
Logique métier pure — transforme données SQL en indicateurs.

| Fichier | Rôle | Dépend de |
|---------|------|-----------|
| mod.rs | Re-exports | — |
| stock.rs | KPI stock : total vivants, âge moyen/médian, tranches d'âge, charge pondérée | db/queries |
| bilan.rs | Flux entrée/sortie, taux résolution, comparaison périodes | db/queries |
| classifier.rs | Classification automatique tickets (statut vivant/terminé, priorité → poids) | — |
| temporal.rs | Agrégation jour/semaine/mois pour graphiques temporels | chrono |

### nlp/ — Text mining
Pipeline NLP français : tokenisation → stop words → stemming → TF-IDF.

| Fichier | Rôle | Dépend de |
|---------|------|-----------|
| mod.rs | Re-exports | — |
| preprocessing.rs | Tokenisation (charabia), stop words FR + ITSM, filtrage | charabia |
| tfidf.rs | Vectorisation TF-IDF en matrice creuse (sprs) | sprs, preprocessing |
| clustering.rs | Wrapper K-Means (linfa) sur vecteurs TF-IDF | linfa, ndarray |
| patterns.rs | Détection patterns résolution/relance dans textes de suivis | regex |

### export/ — Génération Excel
Exports XLSX multi-onglets professionnels via rust_xlsxwriter.

| Fichier | Rôle | Dépend de |
|---------|------|-----------|
| mod.rs | Re-exports + `ExportConfig` (formats partagés) | rust_xlsxwriter |
| stock_report.rs | Export tableau de bord stock (3 onglets) | analyzer/stock |
| bilan_report.rs | Export bilan d'activité (3 onglets) | analyzer/bilan |
| plan_action.rs | Export plan d'action individuel technicien | analyzer/stock, db/ |

### analytics/ — ML avancé
Clustering, détection d'anomalies, prédiction de charge.

| Fichier | Rôle | Dépend de |
|---------|------|-----------|
| mod.rs | Re-exports | — |
| clustering.rs | K-Means sur TF-IDF, silhouette score, méthode Elbow | linfa-clustering, kneed |
| anomalies.rs | Z-scores sur délais log-transformés, outliers statistiques | — |
| prediction.rs | Prédiction charge future (augurs : MSTL + AutoETS) | augurs |

## Frontend — Structure React

### pages/
Chaque page = une route React Router.

| Fichier | Route | Rôle |
|---------|-------|------|
| ImportPage.tsx | `/import` | Import CSV avec barre de progression (Channel API) |
| StockPage.tsx | `/stock` | Dashboard principal : KPI cards + tables techniciens/groupes |
| TechnicianDetail.tsx | `/stock/:technicien` | Liste tickets + plan d'action d'un technicien (drill-down depuis StockPage) |
| BilanPage.tsx | `/bilan` | Bilan temporel : flux entrée/sortie, période sélectionnable |
| CategoriesPage.tsx | `/categories` | Treemap + sunburst avec drill-down |
| MiningPage.tsx | `/mining` | Word cloud, clusters, anomalies |
| ExportPage.tsx | `/export` | Panneau d'export Excel (choix du type + options) |
| TimelineView.tsx | `/timeline` | Suivi longitudinal multi-imports : évolution stock + diff snapshots |
| SettingsPage.tsx | `/settings` | Configuration utilisateur |

### components/
Organisés par domaine métier.

| Dossier | Composants | Dépend de |
|---------|------------|-----------|
| layout/ | `Sidebar.tsx`, `PageHeader.tsx` | React Router |
| stock/ | `KpiCards.tsx`, `TechnicianTable.tsx`, `GroupTable.tsx`, `TicketDetailModal.tsx` | hooks/, stores/ |
| bilan/ | `PeriodSelector.tsx`, `FlowChart.tsx`, `BilanChart.tsx`, `BilanTechnicianTable.tsx` | hooks/, stores/ |
| categories/ | `CategoryTreemap.tsx`, `CategoryDrilldown.tsx`, `DrillBreadcrumb.tsx` | hooks/useECharts |
| mining/ | `WordCloud.tsx`, `ClusterView.tsx`, `AnomalyList.tsx` | @visx/wordcloud, hooks/ |
| shared/ | `DataTable.tsx`, `DateRangePicker.tsx`, `ExportButton.tsx`, `ExportPanel.tsx` | @tanstack/react-table, react-day-picker |

### hooks/
| Fichier | Rôle |
|---------|------|
| useInvoke.ts | Hook typé pour `invoke()` Tauri avec loading/error state |
| useImport.ts | Hook import CSV avec Channel API pour progress |
| useECharts.ts | Custom wrapper ECharts (~50 lignes) : init, resize, events, dispose |

### stores/ (Zustand)
| Fichier | State | Rôle |
|---------|-------|------|
| appStore.ts | dateRange, filters, currentImport | Filtres globaux dashboard |
| settingsStore.ts | config utilisateur | Préférences persistées en SQLite |

### types/
Miroir TypeScript des structs Rust exposées via IPC.

| Fichier | Types |
|---------|-------|
| tickets.ts | `GLPITicket`, `CsvImportResult`, `ParseWarning` |
| kpi.ts | `StockOverview`, `TechnicianStats`, `BilanTemporel`, `CategoryNode` |
| config.ts | `AppConfig`, `ImportHistory` |

## Graphe de dépendances inter-modules (Backend)

```
commands/ ──→ parser/     (import.rs)
          ──→ db/         (tous)
          ──→ analyzer/   (stock, bilan, categories)
          ──→ nlp/        (mining)
          ──→ analytics/  (mining)
          ──→ export/     (export)

parser/   ──→ (aucune dépendance interne)

db/       ──→ parser/types (pour insert.rs)

analyzer/ ──→ db/queries

nlp/      ──→ (aucune dépendance interne)

analytics/──→ nlp/tfidf (pour matrice TF-IDF)

export/   ──→ analyzer/  (données KPI)
          ──→ db/        (données brutes)

state.rs  ←── commands/ (via tauri::State)
lib.rs    ←── tous les modules (registration)
error.rs  ←── tous les modules
```

## Mapping modules → teammates

Chaque module est un périmètre exclusif pour un teammate.

| Module | Périmètre fichiers | Phase |
|--------|-------------------|-------|
| parser/ | src-tauri/src/parser/*.rs | 1 |
| db/ | src-tauri/src/db/*.rs | 1 |
| analyzer/ | src-tauri/src/analyzer/*.rs | 1 |
| commands/ (import+stock) | src-tauri/src/commands/{import,stock}.rs | 1 |
| nlp/ | src-tauri/src/nlp/*.rs | 2 |
| export/ | src-tauri/src/export/*.rs | 2 |
| analytics/ | src-tauri/src/analytics/*.rs | 3 |
| Frontend pages | src/pages/*.tsx | 1-3 |
| Frontend components | src/components/**/*.tsx | 1-3 |
| Shared (Wave 0 only) | lib.rs, state.rs, error.rs, db/migrations, types/ | 0 |
