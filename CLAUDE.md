# PILOTAGE GLPI — Dashboard d'analyse de tickets

## Stack
- Backend : Tauri 2.x (Rust) — rusqlite 0.38 (bundled), csv 1.4, chrono 0.4, rust_xlsxwriter 0.93, charabia 0.9, linfa-clustering 0.8, thiserror 2
- Frontend : React 18 + TypeScript + Tailwind CSS 4 + ECharts 6 + TanStack Table v8 + Zustand + React Router + react-day-picker v9
- Build : Vite, cargo tauri

## Structure
src-tauri/src/
  lib.rs          — entry point Tauri 2
  state.rs        — AppState (Mutex<Option<Connection>>)
  error.rs        — types d'erreur thiserror
  config.rs       — gestion config SQLite
  db/             — migrations, helpers, queries
  parser/         — CSV parsing, normalization
  analyzer/       — stock, bilan, categories, classifier
  nlp/            — tokenizer, TF-IDF, stemming, stop words
  export/         — Excel workbooks (stock, bilan, plan action)
  analytics/      — clustering, anomalies, prédiction
  commands/       — IPC handlers (import, stock, categories, bilan, export, mining, config)
src/
  App.tsx, main.tsx
  pages/          — ImportPage, StockPage, BilanPage, CategoriesPage, MiningPage, ExportPage, SettingsPage
  components/     — layout/, stock/, bilan/, categories/, mining/, shared/
  hooks/          — useInvoke, useImport, useECharts
  stores/         — Zustand (appStore, settingsStore)
  types/          — miroir structs Rust (tickets, kpi, config)
  lib/            — utils.ts

## Conventions
- Rust : snake_case, erreurs via thiserror, #[tauri::command] async pour IPC, AppState via tauri::State
- TypeScript : camelCase, types stricts, invoke() typé
- SQL : migrations versionnées via PRAGMA user_version, WAL mode
- Pas de unwrap() en production, Result<T, E> partout côté Rust
- Serde rename_all = "camelCase" sur les structs exposées au frontend
- Serde rename pour mapper colonnes CSV françaises vers champs Rust

## Commandes
- Dev : cargo tauri dev
- Build : cargo tauri build
- Test backend : cd src-tauri && cargo test
- Test frontend : npm test
- Lint : cargo clippy && npm run lint

## Agent Teams
- Sonnet pour les workers, Opus pour tâches critiques (sécu, archi, scaffolding)
- 1 module = 1 dossier = 1 teammate = périmètre fichiers exclusif
- Fichiers partagés (db/migrations, state.rs, types/, lib.rs) → Wave 0 uniquement
- Ne jamais modifier un fichier hors de son périmètre assigné
- Chaque critère GIVEN/WHEN/THEN = 1 test

## Références
- Specs techniques : docs/Segment 1-8
- Epics et stories : docs/epics/
- Architecture détaillée : docs/architecture/ (stack.md, structure.md)
- Knowledge base : .claude/kb/
