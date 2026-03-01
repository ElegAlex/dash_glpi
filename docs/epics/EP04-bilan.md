# EP04 — Bilan temporel

## Description

Le module bilan mesure les flux d'activité sur une période choisie : tickets créés (entrants), tickets résolus/clos (sortants), stock net en fin de période, et délais de résolution. Il complète le stock instantané (EP02) par une dimension temporelle, permettant de répondre à "combien de tickets ont été traités ce mois-ci ?" plutôt que "combien en avons-nous actuellement ?". Le moteur utilise des agrégations SQLite par période (semaine, mois, trimestre) et des calculs de délai en Rust.

## Règles métier couvertes

| Règle | Description |
|-------|-------------|
| RG-033 | Flux entrant = tickets dont `date_ouverture` est dans la période sélectionnée |
| RG-034 | Flux sortant = tickets dont la date de résolution/clôture est dans la période |
| RG-035 | Stock net = stock début de période + flux entrant - flux sortant |
| RG-036 | Délai de résolution = `date_resolution - date_ouverture` en jours ouvrés |
| RG-037 | La granularité temporelle est configurable : semaine ISO, mois calendaire, trimestre |
| RG-038 | Les périodes sans activité apparaissent avec valeur 0 (pas de trou dans la série) |
| RG-039 | La période maximale couverte est déterminée par les dates min/max des tickets importés |
| RG-040 | Le bilan peut être filtré par technicien, groupe ou type de ticket |

## User stories

### US013 — Bilan temporel flux entrée/sortie par période

**Module cible :** `src-tauri/src/analyzer/bilan.rs`, `src-tauri/src/commands/bilan.rs`, `src/pages/BilanPage.tsx`

**GIVEN** des tickets ont été importés couvrant plusieurs mois d'activité
**WHEN** l'utilisateur navigue vers la vue Bilan et sélectionne une période
**THEN** un tableau de bilan affiche pour chaque sous-période (semaine/mois/trimestre) : nombre de tickets créés, résolus/clos, stock net en fin de période, et délai moyen de résolution

**Critères de validation :**
- [ ] Les flux entrants comptent les tickets par `date_ouverture` (RG-033)
- [ ] Les flux sortants comptent les tickets par statut Résolu/Clos dans la période (RG-034)
- [ ] Les sous-périodes sans activité affichent 0 (RG-038)
- [ ] La somme des flux entrants sur toutes les sous-périodes = total tickets de la période
- [ ] Le stock net début + entrants - sortants = stock net fin (RG-035)

---

### US014 — Graphique de tendance flux temporels

**Module cible :** `src/pages/BilanPage.tsx`, `src/components/BilanChart.tsx`

**GIVEN** les données de bilan temporel sont calculées pour la période sélectionnée
**WHEN** la section graphique est rendue
**THEN** un graphique ECharts de type ligne/barre combiné affiche les flux entrants (barres bleues) et sortants (barres vertes) par sous-période, avec une ligne de tendance du stock net ; les tooltips affichent les valeurs exactes au survol

**Critères de validation :**
- [ ] Les barres entrant/sortant sont visuellement distinctes (palette CPAM)
- [ ] La ligne de stock net est en échelle secondaire si les ordres de grandeur diffèrent
- [ ] Le graphique supporte la granularité semaine/mois/trimestre sans rechargement backend
- [ ] Un DataZoom ECharts permet de zoomer sur une sous-période
- [ ] L'export du graphique en PNG est possible via le toolbox ECharts

---

### US015 — Sélection de période avec presets

**Module cible :** `src/components/shared/DateRangePicker.tsx`, `src/pages/BilanPage.tsx`

**GIVEN** l'utilisateur est sur la vue Bilan
**WHEN** il interagit avec le sélecteur de période
**THEN** il peut choisir parmi les presets (7 derniers jours, 30 derniers jours, dernier trimestre, plage personnalisée) ou saisir une plage manuelle via `react-day-picker` avec locale française, et tous les KPIs/graphiques se recalculent immédiatement

**Critères de validation :**
- [ ] Le calendrier affiche les labels en français (react-day-picker `locale={fr}`, RG-037)
- [ ] Le preset "Dernier trimestre" calcule correctement les dates de début et fin de trimestre
- [ ] La plage minimale sélectionnable est 1 jour
- [ ] La plage est limitée à la période couverte par les données importées (RG-039)
- [ ] La granularité s'adapte automatiquement : < 30j → semaines, < 365j → mois, sinon → trimestres

## Critères de succès de l'epic

- [ ] Les flux entrée/sortie sur les données CPAM 92 sont cohérents avec les exports GLPI natifs
- [ ] Le graphique s'affiche en moins de 200 ms pour une période d'un an
- [ ] La sélection de période et le changement de granularité ne déclenchent pas de rechargement de page
- [ ] La vue bilan peut être filtrée simultanément par période ET par technicien/groupe
