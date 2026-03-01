# EP03 — Catégories hiérarchiques

## Description

Le module catégories ajoute la dimension structurelle aux indicateurs stock. Il parse les groupes hiérarchiques de techniciens (`_DSI > _SUPPORT > _PARC`) comme source principale, et la colonne "Catégorie" ITIL comme source secondaire optionnelle. Un système de fallback sélectionne automatiquement la meilleure source disponible. L'arborescence en Rust permet l'agrégation à n'importe quel niveau de profondeur, et le frontend expose un treemap ECharts avec drill-down et un sunburst interactif.

## Règles métier couvertes

| Règle | Description |
|-------|-------------|
| RG-026 | Le séparateur de niveau hiérarchique dans les groupes est ` > ` (espace-chevron-espace) |
| RG-027 | Le groupe `_DSI > _SUPPORT > _PARC` produit 3 niveaux : `_DSI`, `_SUPPORT`, `_PARC` |
| RG-028 | Fallback : si `Catégorie` est absente, utiliser `Groupe de techniciens` comme catégorie |
| RG-029 | Les tickets multi-groupes comptent pour le groupe principal (premier du champ multiligne) |
| RG-030 | Les compteurs de nœuds parents incluent récursivement tous leurs descendants |
| RG-031 | Un nœud sans enfants est une feuille ; un nœud avec enfants est agrégé |
| RG-032 | La profondeur maximale supportée est illimitée (structure arborescente récursive en Rust) |

## User stories

### US010 — Arbre catégories avec compteurs

**Module cible :** `src-tauri/src/analyzer/categories.rs`, `src-tauri/src/commands/categories.rs`, `src/pages/CategoriesPage.tsx`

**GIVEN** des tickets ont été importés avec des groupes hiérarchiques du type `_DSI > _SUPPORT UTILISATEURS ET POSTES DE TRAVAIL`
**WHEN** l'utilisateur navigue vers la vue Catégories
**THEN** un treemap ECharts affiche l'arborescence complète des groupes/catégories avec un compteur de tickets vivants pour chaque nœud, les nœuds parents agrégant récursivement leurs enfants

**Critères de validation :**
- [ ] Le nœud `_DSI` agrège les 8 groupes enfants (≈ 9 616 tickets pour CPAM 92)
- [ ] Le nœud `_DSI > _SUPPORT UTILISATEURS ET POSTES DE TRAVAIL` affiche ≈ 6 562 tickets
- [ ] Les tickets multi-groupes sont comptés une seule fois (groupe principal, RG-029)
- [ ] La somme des feuilles = total du nœud racine
- [ ] Le treemap ECharts utilise `leafDepth: 1` et `nodeClick: 'zoomToNode'`

---

### US011 — Drill-down par niveau hiérarchique

**Module cible :** `src/pages/CategoriesPage.tsx`, `src/components/DrillBreadcrumb.tsx`

**GIVEN** le treemap est affiché avec les nœuds de niveau 1 (_DSI, etc.)
**WHEN** l'utilisateur clique sur un nœud (ex : `_DSI > _SUPPORT UTILISATEURS ET POSTES DE TRAVAIL`)
**THEN** le treemap zoome sur les enfants du nœud sélectionné, un fil d'Ariane (`Racine > _DSI > _SUPPORT...`) permet de remonter à n'importe quel niveau, et un sunburst synchronisé reflète le même état de drill-down

**Critères de validation :**
- [ ] Le drill-down fonctionne sur au moins 3 niveaux (`_DSI > _SUPPORT > _PARC`)
- [ ] Cliquer sur "Racine" dans le fil d'Ariane ramène à la vue complète
- [ ] L'animation de transition ECharts (`UniversalTransition`) est fluide (< 300ms)
- [ ] Le sunburst et le treemap sont synchronisés sur le même état de drill
- [ ] Le drill-down sur un nœud feuille affiche la liste des tickets dans le tableau sous-jacent

---

### US012 — Fallback groupe → catégorie ITIL

**Module cible :** `src-tauri/src/analyzer/categories.rs`, `src-tauri/src/commands/categories.rs`

**GIVEN** le CSV importé n'a pas de colonne "Catégorie" (cas actuel CPAM 92)
**WHEN** le module catégories construit l'arborescence
**THEN** les groupes de techniciens (`Attribué à - Groupe de techniciens`) sont utilisés comme catégories avec leur hiérarchie naturelle (séparateur ` > `), et un indicateur visuel dans l'UI signale que la vue est basée sur les groupes, non sur les catégories ITIL

**Critères de validation :**
- [ ] Si la colonne `Catégorie` est absente, le fallback groupe s'active automatiquement (RG-028)
- [ ] Un badge "Source : Groupes de techniciens" apparaît dans la vue
- [ ] Quand la colonne `Catégorie` est présente (futur), la vue bascule automatiquement dessus
- [ ] Les données du fallback sont structurellement identiques aux données ITIL (même format `CategoryNode`)

## Critères de succès de l'epic

- [ ] L'arborescence complète des 8 groupes CPAM 92 est rendue correctement
- [ ] Le drill-down sur 3 niveaux fonctionne sans rechargement backend
- [ ] Le fallback groupe → catégorie est transparent pour l'utilisateur
- [ ] Les compteurs sont cohérents avec les KPIs du dashboard stock (même périmètre de filtres)
