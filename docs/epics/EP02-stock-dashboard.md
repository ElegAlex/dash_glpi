# EP02 — Dashboard Stock

## Description

Le dashboard stock transforme les tickets importés en indicateurs de pilotage actionnables. Il affiche le stock vivant global (Nouveau, En cours, En attente, Planifié), la charge par technicien avec indicateur RAG 4-tiers (Vert/Jaune/Orange/Rouge), le détail par technicien avec drill-down vers la liste de tickets, et des filtres multi-dimensionnels par statut, type et groupe. Le moteur de calcul hybride SQL + Rust produit tous les KPIs en moins de 5 ms pour ≤ 50 000 tickets.

## Règles métier couvertes

| Règle | Description |
|-------|-------------|
| RG-015 | Statuts vivants : Nouveau, En cours (Attribué), En cours (Planifié), En attente |
| RG-016 | Statuts terminés : Résolu, Clos — exclus du stock vivant |
| RG-017 | L'ancienneté = `(date_export - date_ouverture).num_days()` |
| RG-018 | Seuil critique : ticket vivant avec ancienneté > 90 jours |
| RG-019 | Indicateur RAG technicien : Vert < 10, Jaune 10-20, Orange 20-40, Rouge > 40 (seuils configurables, cf. RG-007 KB) |
| RG-020 | L'âge moyen du stock = médiane des anciennetés (calculée en Rust sur Vec<f64>) |
| RG-021 | La distribution par tranches : < 7j, 7-30j, 30-90j, > 90j |
| RG-022 | Les tickets sans technicien assigné sont groupés sous "Non assigné" |
| RG-023 | La charge technicien inclut la décomposition Incidents vs Demandes |
| RG-024 | Le filtre par groupe filtre sur le groupe principal (premier du champ multiligne) |
| RG-025 | Les KPIs de l'overview sont calculés en SQLite (COUNT, GROUP BY), la médiane en Rust |

## User stories

### US006 — Vue stock globale avec KPIs

**Module cible :** `src-tauri/src/analyzer/stock.rs`, `src-tauri/src/commands/stock.rs`, `src/pages/StockPage.tsx`, `src/components/stock/KpiCards.tsx`

**GIVEN** des tickets ont été importés et stockés en SQLite
**WHEN** l'utilisateur navigue vers le dashboard stock
**THEN** quatre cartes KPI s'affichent : stock total vivant, âge moyen (médiane en jours), nombre de tickets > 90 jours, et répartition Incidents/Demandes ; chaque carte affiche un indicateur de couleur basé sur les seuils configurés

**Critères de validation :**
- [ ] Le stock total exclut les statuts Résolu et Clos (RG-016)
- [ ] L'âge moyen est la médiane (pas la moyenne) des anciennetés en jours (RG-020)
- [ ] Les tickets > 90 jours sont comptés correctement selon RG-018
- [ ] Tous les KPIs se calculent en moins de 5 ms
- [ ] Les cartes KPI affichent un sparkline si des données historiques sont disponibles

---

### US007 — Charge par technicien avec indicateur RAG

**Module cible :** `src-tauri/src/analyzer/stock.rs`, `src-tauri/src/commands/stock.rs`, `src/pages/StockPage.tsx`

**GIVEN** le dashboard stock est chargé avec des tickets assignés à plusieurs techniciens
**WHEN** la section "Charge par technicien" est rendue
**THEN** un tableau TanStack Table affiche pour chaque technicien : nom, stock total, décomposition par statut, nombre d'incidents, nombre de demandes, tickets > 90j, et un badge RAG 4-tiers (Vert/Jaune/Orange/Rouge) selon RG-019

**Critères de validation :**
- [ ] Badge Vert si stock < 10 (RG-019)
- [ ] Badge Jaune si stock entre 10 et 20 (RG-019)
- [ ] Badge Orange si stock entre 20 et 40 (RG-019)
- [ ] Badge Rouge si stock > 40 (RG-019)
- [ ] Les tickets sans technicien apparaissent sous "Non assigné" (RG-022)
- [ ] Le tableau est triable par toutes les colonnes
- [ ] Le tri par défaut est par stock total décroissant

---

### US008 — Détail technicien avec drill-down

**Module cible :** `src-tauri/src/commands/stock.rs`, `src/pages/TechnicianDetail.tsx`

**GIVEN** l'utilisateur clique sur le nom d'un technicien dans le tableau de charge
**WHEN** la page de détail se charge
**THEN** la liste complète des tickets vivants du technicien est affichée dans un TanStack Table virtualisé (60 FPS pour 10 000+ lignes), avec colonnes : ID, Titre, Statut, Type, Priorité, Groupe, Date d'ouverture, Ancienneté (jours), badge de criticité

**Critères de validation :**
- [ ] Seuls les tickets vivants du technicien sélectionné sont affichés (RG-015)
- [ ] La virtualisation maintient 60 FPS avec 10 000 lignes (react-virtual, `useFlushSync: false`)
- [ ] Les tickets > 90 jours sont mis en évidence visuellement (RG-018)
- [ ] Un bouton "Retour" permet de revenir au dashboard stock
- [ ] Le fil d'Ariane indique "Stock > Technicien : <nom>"

---

### US009 — Filtres par statut, type et groupe

**Module cible :** `src-tauri/src/commands/stock.rs`, `src/pages/StockPage.tsx`

**GIVEN** le dashboard stock affiche les KPIs et le tableau de charge
**WHEN** l'utilisateur sélectionne des filtres (statut, type, groupe) dans la barre de filtres
**THEN** tous les KPIs et le tableau de charge se recalculent immédiatement pour refléter uniquement les tickets correspondant aux filtres actifs

**Critères de validation :**
- [ ] Les filtres sont persistés dans le store Zustand pendant la session
- [ ] Le filtre "groupe" filtre sur le groupe principal (RG-024)
- [ ] La combinaison de filtres (AND logique) fonctionne correctement
- [ ] Un bouton "Réinitialiser" restaure l'état sans filtres
- [ ] Les valeurs disponibles dans chaque filtre sont extraites dynamiquement des données importées

## Critères de succès de l'epic

- [ ] Tous les KPIs se calculent en < 5 ms pour 10 000 tickets
- [ ] Le tableau de charge par technicien est correct (vérifié manuellement sur données CPAM 92)
- [ ] La navigation Stock → Détail technicien → Stock fonctionne sans rechargement de données
- [ ] Les filtres combinés produisent des résultats cohérents avec des requêtes SQL équivalentes
