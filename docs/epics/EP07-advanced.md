# EP07 — Analytique avancée

## Description

Le module analytique avancé enrichit le dashboard avec des capacités de détection automatique : doublons entre tickets par similarité de chaînes (strsim), clustering sémantique K-Means sur la matrice TF-IDF (linfa-clustering), détection d'anomalies par Z-score sur les délais log-transformés, et prédiction de charge par décomposition saisonnière (augurs). Les résultats sont mis en cache dans la table `analytics_cache` SQLite et présentés dans des vues dédiées.

## Règles métier couvertes

| Règle | Description |
|-------|-------------|
| RG-057 | La détection de doublons utilise la similarité Jaro-Winkler (strsim v0.11) sur les titres |
| RG-058 | Seuil de doublon potentiel : similarité Jaro-Winkler > 0.85 ET même groupe technicien |
| RG-059 | Le clustering K-Means utilise linfa-clustering v0.8.1 sur la matrice TF-IDF convertie en dense |
| RG-060 | Le nombre de clusters K est déterminé par la méthode du coude (inertie pour K=2..10) |
| RG-061 | Les anomalies Z-score s'appliquent aux délais de résolution log-transformés (Z > 2.5 = anomalie) |
| RG-062 | La prédiction de charge utilise augurs v0.10.1 (MSTL + AutoETS) sur 90 jours d'historique minimum |
| RG-063 | Les résultats analytics sont mis en cache dans `analytics_cache` avec TTL de 24h |

## User stories

### US024 — Détection de doublons par similarité

**Module cible :** `src-tauri/src/analytics/` (ou `commands/mining.rs`), `src/pages/StockDashboard.tsx`

**GIVEN** le corpus de tickets est chargé en mémoire
**WHEN** la commande de détection de doublons est invoquée
**THEN** les paires de tickets avec un titre similaire (Jaro-Winkler > 0.85) ET appartenant au même groupe technicien sont identifiées et présentées dans un panneau "Doublons potentiels" avec les deux tickets côte à côte

**Critères de validation :**
- [ ] La similarité Jaro-Winkler est calculée via `strsim::jaro_winkler()` (RG-057)
- [ ] Seules les paires du même groupe sont signalées (réduction des faux positifs, RG-058)
- [ ] Les tickets identiques (même ID) ne sont pas comparés avec eux-mêmes
- [ ] La comparaison O(n²) est optimisée : on ne compare que les tickets vivants, pas les résolus
- [ ] L'utilisateur peut marquer une paire comme "pas un doublon" (exclusion persistée en base)

---

### US025 — Clustering sémantique des tickets

**Module cible :** `src-tauri/src/analytics/`, `src-tauri/src/commands/mining.rs`, `src/pages/StockDashboard.tsx`

**GIVEN** la matrice TF-IDF a été calculée (EP05) et le nombre optimal de clusters K est déterminé
**WHEN** l'utilisateur lance le clustering via l'interface
**THEN** les tickets sont regroupés en K clusters sémantiques, chaque cluster est labellisé par ses 5 mots-clés représentatifs, et une heatmap ECharts affiche la distribution des clusters par groupe de techniciens

**Critères de validation :**
- [ ] Le K optimal est déterminé automatiquement par la méthode du coude (K=2..10, RG-060)
- [ ] Le clustering K-Means utilise linfa-clustering v0.8.1 (RG-059)
- [ ] Chaque cluster a un label lisible composé de ses 5 mots-clés TF-IDF les plus représentatifs
- [ ] Le score de silhouette est calculé et affiché pour informer l'utilisateur de la qualité du clustering
- [ ] La heatmap ECharts affiche la répartition clusters × groupes

---

### US026 — Détection d'anomalies Z-score sur délais

**Module cible :** `src-tauri/src/analytics/`, `src-tauri/src/commands/mining.rs`

**GIVEN** le bilan temporel (EP04) est calculé avec les délais de résolution par ticket
**WHEN** la détection d'anomalies est invoquée
**THEN** les tickets avec un délai de résolution anormalement long (Z-score > 2.5 sur la distribution log-transformée des délais) sont identifiés et affichés dans une liste d'alertes avec leur Z-score et le délai effectif

**Critères de validation :**
- [ ] Le Z-score est calculé sur `log(délai + 1)` pour normaliser la distribution asymétrique (RG-061)
- [ ] Les tickets avec `délai = 0` (résolution immédiate) sont exclus de l'analyse
- [ ] Seuls les tickets résolus/clos sont inclus (délai calculable)
- [ ] Le seuil Z > 2.5 est configurable dans les paramètres (EP05, config.rs)
- [ ] Les anomalies sont exportables vers l'export bilan (EP06, US022)

---

### US027 — Prédiction de charge future

**Module cible :** `src-tauri/src/analytics/`, `src-tauri/src/commands/mining.rs`, `src/pages/BilanPage.tsx`

**GIVEN** au moins 90 jours d'historique de flux entrants sont disponibles dans le bilan temporel
**WHEN** l'utilisateur active la prédiction de charge
**THEN** une projection des tickets entrants pour les 30 prochains jours est affichée dans le graphique de bilan comme une zone grisée avec intervalle de confiance, calculée via augurs v0.10.1 (MSTL + AutoETS)

**Critères de validation :**
- [ ] La prédiction n'est disponible que si ≥ 90 jours d'historique existent (RG-062)
- [ ] La zone de prédiction est visuellement distincte des données réelles (zone grisée + intervalle de confiance)
- [ ] Le calcul de prédiction prend moins de 5 secondes pour 365 jours de données
- [ ] Les résultats sont mis en cache 24h dans `analytics_cache` (RG-063)
- [ ] Un avertissement s'affiche si la qualité de la prédiction est insuffisante (MAPE > 30%)

## Critères de succès de l'epic

- [ ] La détection de doublons identifie les vrais doublons dans les données CPAM 92 avec < 5% de faux positifs
- [ ] Le clustering produit des groupes sémantiquement cohérents (validé manuellement sur 50 tickets)
- [ ] La détection d'anomalies identifie les tickets pathologiques connus (délais > 6 mois)
- [ ] Les résultats analytics sont mis en cache et rechargés instantanément au redémarrage de l'app
