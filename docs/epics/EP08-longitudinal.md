# EP08 — Suivi longitudinal

## Description

Le module longitudinal permet de comparer des imports CSV successifs pour mesurer l'évolution du stock dans le temps. Chaque import crée un snapshot horodaté. Le moteur de diff identifie les tickets apparus, résolus, et modifiés entre deux imports. La vue TimelineView affiche l'évolution du stock et la productivité des techniciens sur toute la période couverte par les imports.

## Règles métier couvertes

| Règle | Description |
|-------|-------------|
| RG-064 | Chaque import crée un snapshot horodaté identifié par `(checksum, import_datetime)` |
| RG-065 | Le diff entre deux imports identifie : tickets nouveaux, tickets résolus/clos, tickets modifiés |
| RG-066 | Un ticket est "nouveau" si son ID n'existait pas dans le snapshot précédent |
| RG-067 | Un ticket est "résolu/clos" si son statut est passé de vivant à terminé entre deux imports |
| RG-068 | Un ticket est "modifié" si au moins un de ses champs a changé (statut, technicien, groupe) |
| RG-069 | L'historique du stock est calculé comme série temporelle : stock_vivant(t) pour chaque snapshot t |
| RG-070 | La vue longitudinale peut être filtrée par technicien pour suivre l'évolution de sa charge |

## User stories

### US028 — Snapshots d'import horodatés

**Module cible :** `src-tauri/src/commands/import.rs`, `src-tauri/src/db/setup.rs`

**GIVEN** l'utilisateur importe un fichier CSV
**WHEN** l'import se termine avec succès
**THEN** un snapshot est créé en base avec l'horodatage de l'import, le checksum du fichier, le nombre total de tickets vivants, et le lien vers les données de l'import ; la liste des imports précédents est accessible dans l'interface

**Critères de validation :**
- [ ] Chaque import produit exactement un snapshot en table `imports` (RG-064)
- [ ] Le snapshot inclut : `id`, `import_datetime`, `checksum`, `filename`, `total_vivant`, `total_tickets`
- [ ] La liste des snapshots est accessible depuis la page d'import avec date, fichier, et totaux
- [ ] Un re-import du même fichier (US005 d'EP01) ne crée pas de snapshot dupliqué
- [ ] Les snapshots sont persistés entre les redémarrages de l'application

---

### US029 — Diff entre deux imports

**Module cible :** `src-tauri/src/commands/import.rs`, `src/pages/TimelineView.tsx`

**GIVEN** au moins deux snapshots d'import existent
**WHEN** l'utilisateur sélectionne deux imports à comparer (avant/après)
**THEN** un rapport de diff est affiché avec 3 sections : tickets nouveaux (IDs apparus), tickets résolus/clos (IDs devenus terminés), tickets modifiés (IDs avec changements de champs), et un résumé numérique de chaque catégorie

**Critères de validation :**
- [ ] Les tickets "nouveaux" = IDs présents dans B mais absents de A (RG-066)
- [ ] Les tickets "résolus/clos" = IDs vivants dans A, terminés dans B (RG-067)
- [ ] Les tickets "modifiés" incluent le détail des champs changés (avant/après) (RG-068)
- [ ] Le diff est calculé en Rust en < 500 ms pour 10 000 tickets × 2 imports
- [ ] Le rapport de diff est exportable en XLSX (onglet supplémentaire de US022)

---

### US030 — Suivi de l'évolution du stock global

**Module cible :** `src-tauri/src/commands/import.rs`, `src/pages/TimelineView.tsx`

**GIVEN** plusieurs snapshots d'import existent sur une période de plusieurs semaines
**WHEN** l'utilisateur navigue vers la vue TimelineView
**THEN** un graphique ECharts de type ligne affiche le stock vivant total pour chaque snapshot dans l'ordre chronologique, avec les dates d'import en abscisse et le nombre de tickets vivants en ordonnée

**Critères de validation :**
- [ ] Chaque point de la courbe correspond à un snapshot horodaté (RG-069)
- [ ] Les points sont affichés dans l'ordre chronologique des imports
- [ ] Survoler un point affiche la date d'import, le fichier source, et le stock vivant
- [ ] Le graphique supporte au moins 52 snapshots (un par semaine pendant un an) sans dégradation de performance
- [ ] Un DataZoom ECharts permet de zoomer sur une sous-période

---

### US031 — Historique d'évolution par technicien

**Module cible :** `src-tauri/src/commands/import.rs`, `src/pages/TimelineView.tsx`, `src/pages/TechnicianDetail.tsx`

**GIVEN** plusieurs snapshots d'import incluent les données d'un même technicien
**WHEN** l'utilisateur sélectionne un technicien dans la vue longitudinale
**THEN** une courbe d'évolution de la charge du technicien est affichée (stock vivant par snapshot), ainsi qu'un tableau des changements notables (nouveaux tickets, tickets résolus entre chaque import)

**Critères de validation :**
- [ ] La courbe du technicien utilise les mêmes snapshots que la courbe globale (RG-070)
- [ ] Un technicien sans tickets dans un snapshot apparaît avec la valeur 0 (pas de trou)
- [ ] La page technicien (US008 d'EP02) a un onglet "Historique" qui affiche cette vue
- [ ] La sélection du technicien dans la liste met à jour instantanément le graphique (< 100 ms)
- [ ] Les techniciens disparus (plus de tickets depuis un import) restent visibles avec valeur 0

## Critères de succès de l'epic

- [ ] Le diff entre deux imports de 10 000 tickets s'exécute en < 500 ms
- [ ] La vue longitudinale avec 12 snapshots mensuels s'affiche en < 200 ms
- [ ] Les courbes d'évolution sont cohérentes avec les totaux affichés dans le dashboard stock
- [ ] L'historique est conservé entre les sessions (persisté en SQLite, pas en mémoire)
