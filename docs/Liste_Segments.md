Voici le plan. Chaque segment est suffisamment ciblé pour que la recherche ne s'effondre pas sous son propre poids.

---

**Segment 1 — Parsing CSV GLPI en Rust : BOM, encodage, multilignes, dates françaises** Crates csv + serde + chrono + encoding_rs_io. Gestion des ID avec espaces, champs multilignes (\n dans les cellules), séparateur ;, dates DD-MM-YYYY HH:MM. Colonnes optionnelles (Catégorie absente aujourd'hui, présente demain). Détection dynamique des valeurs de Statut.

**Segment 2 — Architecture Tauri 2 : commandes, state, IPC, SQLite** Structure projet from scratch. Schéma SQLite complet (imports, tickets, config, cache). Commandes Tauri avec signatures. State management Rust (Mutex<Connection>). Progress reporting via Channel. PRAGMAs SQLite. Bulk insert performant. FTS5.

**Segment 3 — KPI Stock + Bilan temporel : calculs et agrégation** Distinction vivant/terminé. Cards par statut. Âge moyen/médian des vivants. Flux entrée/sortie par période (jour/semaine/mois). Delta stock. Ventilation technicien/groupe/type. Approximation date de clôture via Dernière modification. Statuts standard GLPI 9.5.

**Segment 4 — Catégories hiérarchiques : parsing, drill-down, fallback** Parser la hiérarchie "_DSI > _SUPPORT > _PARC". Niveaux 1/2/3. Prévoir colonne Catégorie ITIL optionnelle comme override. Structure arborescente en Rust. Agrégation par niveau. Évolution temporelle par catégorie.

**Segment 5 — NLP et text mining en Rust : pipeline complet** Charabia (tokenizer français), rust-stemmers (Snowball), stop-words français enrichis IT. TF-IDF avec linfa-preprocessing + sprs. Pipeline tokenize → stem → vectorize. Tantivy pour full-text search. Benchmarks performance sur 10K tickets.

**Segment 6 — Clustering, anomalies, prédictif** K-Means linfa-clustering : paramétrage, elbow method, silhouette score. Détection anomalies (z-score délais, distance centroïde). Prédiction charge (séries temporelles, saisonnalité). Candle pour embeddings français (faisabilité desktop). Classification automatique de tickets ITSM (état de l'art).

**Segment 7 — Frontend React : visualisations, tableaux, drill-down** ECharts vs Recharts vs Nivo (treemap, sunburst, heatmap, Sankey). TanStack Table v8 virtualisation. Date range picker (react-day-picker v9). Wordcloud. Patterns drill-down ECharts. Composants KPI cards. Thème clair pro.

**Segment 8 — Exports Excel + fonctionnalités avancées** rust_xlsxwriter : multi-onglets, conditional formatting, graphiques embarqués, freeze panes. Détection doublons (strsim). Alerting (tauri-plugin-notification). Suivi longitudinal (diff entre imports). Export PDF. Intégration future API GLPI REST.

---

8 segments, chacun ciblé sur un périmètre précis. On les lance un par un ou par paires. Dis-moi comment tu veux procéder — tout d'un coup séquentiellement, ou tu veux valider/ajuster le découpage d'abord ?