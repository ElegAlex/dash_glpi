# Stack technique — Versions exactes

## Backend Rust (Cargo.toml)

### Core Tauri
| Crate | Version | Rôle |
|-------|---------|------|
| tauri | 2.10 | Framework desktop |
| tauri-build | 2.5 | Build dependencies |
| tauri-plugin-dialog | 2.4 | Dialogues fichier (open/save) |
| tauri-plugin-fs | 2.4 | Accès filesystem |
| tauri-plugin-notification | 2.3 | Notifications desktop natives |
| tauri-plugin-shell | 2.2 | Ouverture liens externes |

### Sérialisation & Parsing
| Crate | Version | Rôle |
|-------|---------|------|
| serde | 1.0 (features: derive) | Sérialisation/désérialisation |
| serde_json | 1.0 | JSON |
| csv | 1.4 | Parsing CSV (BOM UTF-8 natif, point-virgule, multilignes) |
| chrono | 0.4 (features: serde) | Dates — format français `%d-%m-%Y %H:%M` |

### Base de données
| Crate | Version | Features | Rôle |
|-------|---------|----------|------|
| rusqlite | 0.38 | bundled, fallible_uint, cache | SQLite 3.51.1 embarqué, FTS5 inclus |

### NLP / Text mining
| Crate | Version | Rôle |
|-------|---------|------|
| charabia | 0.9 (default-features = false) | Tokenisation française (Meilisearch) |
| rust-stemmers | 1.2 | Stemming Snowball français |
| unicode-normalization | 0.1 | Normalisation NFKD accents |
| regex | 1 | Expressions régulières |
| sprs | 0.11 | Matrices creuses TF-IDF |

### Analytics / ML
| Crate | Version | Rôle |
|-------|---------|------|
| linfa | 0.8.1 | Meta-crate ML (prelude, traits) |
| linfa-clustering | 0.8.1 | K-Means, DBSCAN |
| linfa-reduction | 0.8.1 | PCA, SVD tronquée |
| ndarray | 0.16 | Tableaux N-dimensionnels |
| kneed | 1.0 | Détection du coude (méthode Elbow) |
| augurs | 0.10.1 | Prédiction de charge (MSTL, ETS, Prophet) |

### Export
| Crate | Version | Features | Rôle |
|-------|---------|----------|------|
| rust_xlsxwriter | 0.93 | serde, chrono, zlib, ryu | Export Excel XLSX multi-onglets |
| strsim | 0.11 | — | Détection doublons (Jaro-Winkler, Sørensen-Dice) |

### Utilitaires
| Crate | Version | Rôle |
|-------|---------|------|
| thiserror | 2 | Macros d'erreur typées |
| log | 0.4 | Logging façade |
| env_logger | 0.11 | Implémentation logger |
| tokio | 1 (features: full) | Runtime async |

### Dev / Test
| Crate | Version | Rôle |
|-------|---------|------|
| criterion | 0.5 (features: html_reports) | Benchmarks |

### Futures (non inclus au scaffolding)
| Crate | Version | Rôle | Phase |
|-------|---------|------|-------|
| typst-as-lib | 0.4 | Compilation Typst → PDF | Phase 5 |
| typst-pdf | 0.14 | Rendu PDF | Phase 5 |
| extended-isolation-forest | 0.2 | Anomalies multi-dimensionnelles | Phase 6 |
| reqwest | 0.12 (features: json, rustls-tls) | Client HTTP API GLPI | Futur |

## Frontend (package.json)

### Dependencies
| Package | Version | Rôle |
|---------|---------|------|
| react | ^18 | UI framework |
| react-dom | ^18 | Rendu DOM |
| react-router | ^7 | Routage SPA |
| zustand | ^5 | State management |
| echarts | ^6.0.0 | Graphiques (treemap, sunburst, heatmap, Sankey) |
| @tanstack/react-table | ^8.21.3 | Data table avec tri, filtres, groupement |
| @tanstack/react-virtual | ^3.13.19 | Scrolling virtuel (50K+ lignes) |
| react-day-picker | ^9.14.0 | Sélecteur de plages de dates (locale fr native) |
| @visx/wordcloud | ^3.12.0 | Nuage de mots TF-IDF |
| @visx/text | ^3 | Rendu texte SVG |
| @visx/scale | ^3 | Échelles logarithmiques |
| lucide-react | latest | Icônes |
| date-fns | ^4 | Manipulation de dates |
| @tauri-apps/api | ^2 | API Tauri (invoke, Channel) |
| @tauri-apps/plugin-dialog | ^2 | Plugin dialog côté JS |
| @tauri-apps/plugin-notification | ^2 | Plugin notification côté JS |

### DevDependencies
| Package | Version | Rôle |
|---------|---------|------|
| typescript | ^5 | Typage |
| vite | ^6 | Bundler |
| @vitejs/plugin-react | latest | Plugin React pour Vite |
| tailwindcss | ^4.2 | CSS utility-first (config via @theme) |
| @types/node | latest | Types Node.js |
| @tauri-apps/cli | ^2 | CLI Tauri |

## Configuration clé

### Cargo.toml — Lib section
```toml
[lib]
name = "glpi_dashboard_lib"
crate-type = ["lib", "cdylib", "staticlib"]
```

### Tauri identifier
```
fr.cpam92.glpi-dashboard
```

### SQLite PRAGMAs (appliqués à chaque ouverture)
```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = -64000;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
PRAGMA temp_store = MEMORY;
PRAGMA mmap_size = 268435456;
```
