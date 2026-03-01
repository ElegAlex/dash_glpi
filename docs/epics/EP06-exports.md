# EP06 — Exports Excel

## Description

Le module exports produit les livrables principaux de l'application : fichiers XLSX multi-onglets professionnels via rust_xlsxwriter 0.93+. Les quatre types d'exports couvrent le tableau de bord stock, les plans d'action individuels par technicien, le bilan d'activité, et un ZIP de tous les plans d'action. Chaque export inclut des formatages conditionnels, des filtres automatiques, des panneaux figés et une mise en page prête à imprimer.

## Règles métier couvertes

| Règle | Description |
|-------|-------------|
| RG-049 | Le format XLSX est généré via rust_xlsxwriter 0.93+ (features: serde, chrono, zlib) |
| RG-050 | Les dates sont exportées au format Excel natif `dd/mm/yyyy` (locale FR automatique) |
| RG-051 | Les nombres décimaux utilisent le format `#,##0.00` (locale FR affiche `,` pour décimale) |
| RG-052 | Les en-têtes ont fond bleu `2C5F8A`, texte blanc, gras, bordure fine |
| RG-053 | La première ligne est figée (`set_freeze_panes(1, 0)`) et un auto-filtre est activé |
| RG-054 | Le formatage conditionnel de la colonne stock suit le RAG : Vert ≤ 10, Ambre 11-30, Rouge > 30 |
| RG-055 | L'export individuel par technicien est un fichier XLSX séparé nommé `plan_action_<nom>.xlsx` |
| RG-056 | L'export ZIP regroupe tous les plans d'action individuels en un seul fichier `plans_action_<date>.zip` |

## User stories

### US020 — Export stock Excel multi-onglets

**Module cible :** `src-tauri/src/export/`, `src-tauri/src/commands/export.rs`, `src/components/ExportPanel.tsx`

**GIVEN** le dashboard stock est affiché avec les données du dernier import
**WHEN** l'utilisateur clique sur "Exporter le stock" dans l'ExportPanel
**THEN** un fichier XLSX est généré avec 3 onglets : "Vue globale" (KPIs + distribution par tranches), "Techniciens" (tableau de charge complet), "Groupes" (agrégation par groupe), et enregistré à l'emplacement choisi via le dialogue natif

**Critères de validation :**
- [ ] L'onglet "Techniciens" utilise la sérialisation Serde (`TechnicianRow`) (RG-049)
- [ ] La colonne "Stock total" a un formatage conditionnel RAG natif XLSX (RG-054)
- [ ] Les en-têtes sont figées et un auto-filtre est activé (RG-053)
- [ ] Le fichier s'ouvre correctement dans Excel 2016+ et LibreOffice Calc 7+
- [ ] La génération prend moins de 2 secondes pour 100 techniciens

---

### US021 — Export plan d'action par technicien

**Module cible :** `src-tauri/src/export/`, `src-tauri/src/commands/export.rs`, `src/pages/TechnicianDetail.tsx`

**GIVEN** l'utilisateur consulte le détail d'un technicien
**WHEN** il clique sur "Exporter le plan d'action"
**THEN** un fichier XLSX nommé `plan_action_<nom_technicien>_<date>.xlsx` est généré avec 3 onglets : "Entretien" (résumé avec KPIs du technicien, mots-clés NLP), "Détail tickets" (liste complète avec ancienneté et criticité), "Checklist" (actions recommandées auto-générées)

**Critères de validation :**
- [ ] Le nom du fichier sanitise les caractères spéciaux du nom du technicien (espaces → `_`) (RG-055)
- [ ] L'onglet "Détail tickets" liste tous les tickets vivants du technicien, triés par ancienneté décroissante
- [ ] Les tickets > 90 jours ont une mise en surbrillance rouge dans l'onglet "Détail tickets" (RG-054)
- [ ] L'onglet "Entretien" affiche les 10 mots-clés NLP du technicien si disponibles (EP05)
- [ ] Les dates dans le fichier sont au format `dd/mm/yyyy` lisibles en locale française (RG-050)

---

### US022 — Export bilan d'activité

**Module cible :** `src-tauri/src/export/`, `src-tauri/src/commands/export.rs`, `src/pages/BilanPage.tsx`

**GIVEN** la vue bilan affiche des données pour une période sélectionnée
**WHEN** l'utilisateur clique sur "Exporter le bilan"
**THEN** un fichier XLSX est généré avec 3 onglets : "Volume" (flux entrants/sortants par sous-période), "Délais" (délais de résolution par sous-période et type), "Comparatif techniciens" (productivité comparée)

**Critères de validation :**
- [ ] L'onglet "Volume" inclut les mêmes données que le graphique affiché à l'écran
- [ ] La période et la granularité sélectionnées dans l'UI sont reflétées dans l'export
- [ ] L'onglet "Délais" distingue Incidents et Demandes (RG-023 d'EP02)
- [ ] Le formatage des nombres suit RG-051 (virgule décimale en locale FR)
- [ ] Le nom du fichier inclut la période : `bilan_<date_debut>_<date_fin>.xlsx`

---

### US023 — Export ZIP tous les plans d'action

**Module cible :** `src-tauri/src/commands/export.rs`, `src/components/ExportPanel.tsx`

**GIVEN** plusieurs techniciens ont des tickets vivants dans le dashboard stock
**WHEN** l'utilisateur clique sur "Exporter tous les plans d'action (ZIP)"
**THEN** un fichier ZIP contenant un XLSX par technicien est généré, nommé `plans_action_<YYYY-MM-DD>.zip`, et enregistré à l'emplacement choisi via le dialogue natif

**Critères de validation :**
- [ ] Le ZIP contient autant de XLSX que de techniciens avec des tickets vivants (RG-056)
- [ ] Chaque XLSX est identique à celui produit par US021 pour le technicien correspondant
- [ ] La progression de génération est reportée au frontend (un événement par XLSX généré)
- [ ] Un technicien sans ticket vivant n'est pas inclus dans le ZIP
- [ ] Le ZIP s'ouvre correctement sur Windows avec l'explorateur de fichiers natif

## Critères de succès de l'epic

- [ ] Tous les exports XLSX s'ouvrent sans erreur dans Excel 2016+ et LibreOffice Calc 7+
- [ ] Le formatage conditionnel RAG est dynamiquement mis à jour si l'utilisateur modifie les valeurs dans Excel
- [ ] L'export ZIP de 30 techniciens prend moins de 10 secondes
- [ ] Les noms de fichiers générés sont valides sur Windows (caractères spéciaux sanitisés)
