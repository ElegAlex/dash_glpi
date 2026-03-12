# Attribution Intelligente des Tickets — Design Spec

## Contexte

L'application de pilotage GLPI ingère ~90 000 tickets historiques. Certains tickets vivants n'ont pas de technicien assigné (`technicien_principal IS NULL AND est_vivant = true`). Le superviseur doit pouvoir obtenir des **suggestions d'attribution** basées sur la compétence des techniciens et leur charge de travail actuelle.

## Décisions

| Paramètre | Décision |
|-----------|----------|
| Ticket non attribué | `technicien_principal IS NULL AND est_vivant = true` |
| Emplacement UI | Nouvelle section en bas de `/stock` |
| Matching | Hybride catégorie (40%) + TF-IDF titres (60%) |
| Déclenchement | On-demand avec cache `analytics_cache` |
| Output | Top 3 techniciens avec scores décomposés |
| Algorithme | Score hybride avec pénalité charge |
| Colonne temporelle | `date_resolution` avec fallback `date_cloture_approx` puis `derniere_modification` |

## Architecture

### Nouveaux fichiers Rust

- `src-tauri/src/recommandation/mod.rs` — module racine
- `src-tauri/src/recommandation/profiling.rs` — construction des profils technicien
- `src-tauri/src/recommandation/scoring.rs` — moteur de scoring et recommandation
- `src-tauri/src/recommandation/types.rs` — structs (tous avec `#[derive(Serialize, Deserialize, Clone)]` et `#[serde(rename_all = "camelCase")]`)
- `src-tauri/src/commands/recommandation.rs` — commandes Tauri IPC

### Nouveaux fichiers Frontend

- `src/components/stock/AttributionSection.tsx` — section UI
- `src/types/recommandation.ts` — types TypeScript miroir (camelCase)

### Fichiers modifiés

- `src-tauri/src/main.rs` — enregistrer les nouvelles commandes
- `src-tauri/src/lib.rs` — déclarer le module `recommandation`
- `src/pages/StockPage.tsx` — intégrer la section Attribution
- `src/types/index.ts` — ré-exporter les types recommandation

### Flux

Deux commandes IPC séparées :

1. **`build_technician_profiles`** (lourd, cacheable) : charge les tickets résolus des 6 derniers mois, construit un profil par technicien (distribution catégories + centroïde TF-IDF + vocabulaire + IDF), persiste dans `analytics_cache` comme JSON via serde_json.
2. **`get_assignment_recommendations`** (léger) : charge les profils et le vocabulaire depuis le cache, récupère les tickets non attribués vivants et le stock actuel par technicien, calcule le score hybride, retourne le top 3 par ticket.

### Stratégie de cache (`analytics_cache`)

C'est le **premier consommateur** de la table `analytics_cache`. L'implémentation crée le pattern de cache pour le projet.

- **Clé** : `(import_id = active_import_id, analysis_type = "technician_profiles", parameters = "{}")`
- **Valeur** : JSON sérialisé de `CachedProfilingData` (profils + vocabulaire + IDF values)
- **Invalidation** : le cache est lié à l'`import_id`. Un nouvel import crée un nouvel `import_id` → le cache précédent est ignoré (l'ancien reste en base mais n'est plus chargé). Le bouton "Analyser" force un recalcul et fait un `INSERT OR REPLACE`.
- **Requêtes SQL** :
  - Insert : `INSERT OR REPLACE INTO analytics_cache (import_id, analysis_type, parameters, result) VALUES (?1, 'technician_profiles', '{}', ?2)`
  - Select : `SELECT result FROM analytics_cache WHERE import_id = ?1 AND analysis_type = 'technician_profiles' AND parameters = '{}'`

## Algorithme

### Étape 1 : Profil technicien (on-demand, cacheable)

**Requête des tickets de référence :**
```sql
SELECT technicien_principal, titre, categorie_niveau1, categorie_niveau2
FROM tickets
WHERE import_id = :active_import_id
  AND est_vivant = 0  -- terminés
  AND technicien_principal IS NOT NULL
  AND COALESCE(date_resolution, date_cloture_approx, derniere_modification) >= :date_6_mois_ago
```

Pour chaque technicien ayant des tickets dans ce résultat :

**Composante catégorie :**
- Compter les occurrences de `categorie_niveau2 ?? categorie_niveau1 ?? "SANS_CATEGORIE"`
- Normaliser en fréquences relatives (somme = 1.0)

**Composante TF-IDF :**
- Collecter tous les titres de tous les techniciens ensemble
- Preprocesser via le pipeline NLP existant (Charabia + stop-words 4 couches + Snowball)
- Calculer la matrice TF-IDF globale via `build_tfidf_matrix()` (sublinear_tf, smooth_idf, L2-normalisé)
- **Conserver le vocabulaire (`HashMap<String, usize>`) et les IDF values (`Vec<f64>`)** pour la phase de scoring
- Pour chaque technicien, extraire les lignes de la matrice correspondant à ses tickets, calculer le centroïde (moyenne des vecteurs sparse), L2-normaliser

**Utilitaires à implémenter :**
- `compute_centroid(matrix: &CsMat<f64>, row_indices: &[usize]) -> Vec<(usize, f64)>` — moyenne des lignes sparse, L2-normalisé
- `cosine_similarity_sparse(a: &[(usize, f64)], b: &[(usize, f64)]) -> f64` — dot product de deux vecteurs sparse L2-normalisés

### Étape 2 : Scoring d'un ticket non attribué

**Limite** : les 100 premiers tickets non attribués (triés par ancienneté DESC) pour éviter un temps de calcul excessif.

Pour chaque ticket X (`technicien_principal IS NULL AND est_vivant = true`) :

```
// Projeter le titre dans le vocabulaire existant (du profiling)
tokens_X = preprocess(X.titre)
vecteur_X = sparse_vector_from_vocab(tokens_X, vocabulary, idf_values)
L2_normalize(vecteur_X)

cat_X = X.categorie_niveau2 ?? X.categorie_niveau1

POUR chaque technicien T ayant un profil:
    // Score TF-IDF (cosine similarity = dot product car L2-normalisés)
    score_tfidf = cosine_similarity_sparse(vecteur_X, centroide_T)

    // Score catégorie
    SI cat_X existe ET cat_X != "SANS_CATEGORIE":
        score_cat = profil_T.cat_distribution[cat_X] ?? 0.0
        poids_cat = 0.4
        poids_tfidf = 0.6
    SINON:
        score_cat = 0.0
        poids_cat = 0.0
        poids_tfidf = 1.0

    // Score compétence
    score_competence = poids_cat * score_cat + poids_tfidf * score_tfidf

    // Pénalité charge (seuil lu depuis config table)
    stock_T = tickets vivants actuels du technicien T
    facteur_charge = 1.0 / (1.0 + stock_T / seuil_tickets_technicien)

    // Score final
    score_final = score_competence * facteur_charge

trier par score_final DESC → retourner top 3 (si score_final >= score_minimum)
```

**Projection d'un nouveau document dans le vocabulaire existant :**
Pour chaque stem dans `tokens_X`, chercher dans `vocabulary[stem]` → obtenir l'index. Calculer `sublinear_tf = 1 + ln(count)`. Multiplier par `idf_values[index]`. Le vecteur résultant est sparse. L2-normaliser.

### Paramètres

| Paramètre | Défaut | Source | Rôle |
|-----------|--------|--------|------|
| `poids_categorie` | 0.4 | constante v1 | Poids du signal catégorie |
| `poids_tfidf` | 0.6 | constante v1 | Poids du signal textuel |
| `seuil_tickets_technicien` | 20 | config table | Réutilisé pour la pénalité charge |
| `periode_profil_mois` | 6 | constante v1 | Fenêtre glissante pour les profils |
| `score_minimum` | 0.05 | constante v1 | Seuil d'exclusion des suggestions |
| `max_unassigned_tickets` | 100 | constante v1 | Limite de tickets à scorer |

## Structures de données

### Rust

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TechnicianProfile {
    pub technicien: String,
    pub nb_tickets_reference: usize,
    pub cat_distribution: HashMap<String, f64>,
    pub centroide_tfidf: Vec<(usize, f64)>,  // sparse: (term_index, weight)
}

/// Données complètes cachées dans analytics_cache (JSON)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedProfilingData {
    pub profiles: Vec<TechnicianProfile>,
    pub vocabulary: HashMap<String, usize>,   // stem → index
    pub idf_values: Vec<f64>,                  // idf par index de vocabulaire
    pub vocabulary_size: usize,
    pub nb_tickets_analysed: usize,
    pub periode_from: String,                  // ISO date
    pub periode_to: String,
}

/// Résultat renvoyé au frontend (sans les données volumineuses)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilingResult {
    pub profiles_count: usize,
    pub vocabulary_size: usize,
    pub nb_tickets_analysed: usize,
    pub periode_from: String,
    pub periode_to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentRecommendation {
    pub ticket_id: i64,
    pub ticket_titre: String,
    pub ticket_categorie: Option<String>,
    pub suggestions: Vec<TechnicianSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TechnicianSuggestion {
    pub technicien: String,
    pub score_final: f64,
    pub score_competence: f64,
    pub score_categorie: f64,
    pub score_tfidf: f64,
    pub stock_actuel: usize,
    pub facteur_charge: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationRequest {
    pub limit_per_ticket: Option<usize>,  // défaut 3
    pub score_minimum: Option<f64>,       // défaut 0.05
}
```

### TypeScript

```typescript
export interface TechnicianSuggestion {
    technicien: string;
    scoreFinal: number;
    scoreCompetence: number;
    scoreCategorie: number;
    scoreTfidf: number;
    stockActuel: number;
    facteurCharge: number;
}

export interface AssignmentRecommendation {
    ticketId: number;
    ticketTitre: string;
    ticketCategorie: string | null;
    suggestions: TechnicianSuggestion[];
}

export interface ProfilingResult {
    profilesCount: number;
    vocabularySize: number;
    nbTicketsAnalysed: number;
    periodeFrom: string;
    periodeTo: string;
}

export interface RecommendationRequest {
    limitPerTicket?: number;
    scoreMinimum?: number;
}
```

### Commandes IPC

```rust
#[tauri::command]
pub async fn build_technician_profiles(
    state: State<'_, AppState>,
) -> Result<ProfilingResult, String>

#[tauri::command]
pub async fn get_assignment_recommendations(
    state: State<'_, AppState>,
    request: RecommendationRequest,
) -> Result<Vec<AssignmentRecommendation>, String>
```

## Gestion du Mutex

Le `AppState` utilise un `Mutex<Option<Connection>>` (connexion unique). Pour éviter de bloquer l'UI pendant le profiling :

1. Acquérir le lock, exécuter la requête SQL, **copier les données en mémoire**, relâcher le lock
2. Effectuer le calcul TF-IDF + centroïdes **hors du lock** (CPU-bound, pas de DB)
3. Ré-acquérir le lock, écrire le cache dans `analytics_cache`, relâcher

La commande Tauri est `async` donc le calcul CPU ne bloque pas le thread principal.

## Interface utilisateur

### Intégration

Section ajoutée en bas de `StockPage.tsx`, après les tableaux techniciens/groupes existants.

### Layout

- **Header** : titre "Attribution intelligente" + bouton "Analyser" (primary)
- **Bandeau d'état** : après calcul, affiche le nombre de profils, tickets analysés, période
- **Cards par ticket** : une card `bg-white rounded-2xl shadow-[shadow-1]` par ticket non attribué contenant :
  - Titre et catégorie du ticket
  - Tableau top 3 : colonnes Rang, Technicien, Compétence, Stock, Score final
  - Barre de progression colorée sur le score final (vert > 0.6, jaune > 0.3, rouge sinon)
- **État vide — pas de tickets** : message "Aucun ticket non attribué" + icône check
- **État vide — pas de profils** : message "Aucun ticket résolu trouvé dans les 6 derniers mois. Impossible de construire des profils de compétence." + icône warning
- **Pas de pagination** en v1 (limité à 100 tickets non attribués max)

### Design system

Respect intégral de CLAUDE.md : cards rounded-2xl, shadow-1/shadow-2 hover, DM Sans pour les valeurs, Source Sans 3 pour le texte, pas de bordures, couleurs sémantiques.

## Tests

### Unitaires Rust

1. **Profiling** : distribution catégories somme à 1.0, fenêtre 6 mois respectée, technicien sans tickets → pas de profil, vocabulaire et IDF values correctement extraits
2. **Scoring** : score catégorie exact, fallback TF-IDF quand catégorie absente, facteur charge correct (stock=0→1.0, stock=seuil→0.5), tri DESC, seuil minimum respecté, projection dans vocabulaire existant correcte
3. **Utilitaires** : `compute_centroid` produit un vecteur L2-normalisé, `cosine_similarity_sparse` renvoie 1.0 pour vecteurs identiques et 0.0 pour orthogonaux
4. **Cache** : sérialisation/désérialisation JSON de `CachedProfilingData` roundtrip, invalidation par import_id
5. **Intégration** : pipeline complet sur jeu synthétique (10 techniciens, 500 tickets)

### Backtesting

Sur les tickets résolus des 3 derniers mois : masquer `technicien_principal`, lancer le moteur, comparer.

- **Top-1 accuracy** : technicien réel en position 1 (cible > 40%)
- **Top-3 accuracy** : technicien réel dans le top 3 (cible > 70%)
- **Distribution de charge** : écart-type des stocks proposés < écart-type réel

### Métriques de suivi

- Nombre de tickets non attribués
- Score moyen du top 1
- Couverture : % de tickets avec au moins 1 suggestion > 0.3
