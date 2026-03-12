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

## Architecture

### Nouveaux fichiers Rust

- `src-tauri/src/recommandation/mod.rs` — module racine
- `src-tauri/src/recommandation/profiling.rs` — construction des profils technicien
- `src-tauri/src/recommandation/scoring.rs` — moteur de scoring et recommandation
- `src-tauri/src/recommandation/types.rs` — structs
- `src-tauri/src/commands/recommandation.rs` — commandes Tauri IPC

### Nouveaux fichiers Frontend

- `src/components/stock/AttributionSection.tsx` — section UI
- `src/types/recommandation.ts` — types TypeScript miroir

### Fichiers modifiés

- `src-tauri/src/main.rs` — enregistrer les nouvelles commandes
- `src-tauri/src/lib.rs` — déclarer le module `recommandation`
- `src/pages/StockPage.tsx` — intégrer la section Attribution

### Flux

Deux commandes IPC séparées :

1. **`build_technician_profiles`** (lourd, cacheable) : charge les tickets résolus des 6 derniers mois, construit un profil par technicien (distribution catégories + centroïde TF-IDF), persiste dans `analytics_cache`.
2. **`get_assignment_recommendations`** (léger) : charge les profils depuis le cache, récupère les tickets non attribués vivants et le stock actuel par technicien, calcule le score hybride, retourne le top 3 par ticket.

## Algorithme

### Étape 1 : Profil technicien (on-demand, cacheable)

Pour chaque technicien ayant des tickets résolus dans les 6 derniers mois :

**Composante catégorie :**
- Compter les occurrences de `categorie_niveau2 ?? categorie_niveau1 ?? "SANS_CATEGORIE"`
- Normaliser en fréquences relatives (somme = 1.0)

**Composante TF-IDF :**
- Collecter tous les titres des tickets du technicien
- Preprocesser via le pipeline NLP existant (Charabia + stop-words 4 couches + Snowball)
- Calculer les vecteurs TF-IDF (sublinear_tf, smooth_idf, L2-normalisé)
- Moyenner les vecteurs en un centroïde L2-normalisé (représentation sparse)

### Étape 2 : Scoring d'un ticket non attribué

Pour chaque ticket X (`technicien_principal IS NULL AND est_vivant = true`) :

```
vecteur_X = tfidf(preprocess(X.titre))
cat_X = X.categorie_niveau2 ?? X.categorie_niveau1

POUR chaque technicien T ayant un profil:
    // Score TF-IDF
    score_tfidf = cosine(vecteur_X, centroide_T)

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

    // Pénalité charge
    stock_T = tickets vivants actuels du technicien T
    facteur_charge = 1.0 / (1.0 + stock_T / seuil_tickets_technicien)

    // Score final
    score_final = score_competence * facteur_charge

trier par score_final DESC → retourner top 3 (si score_final >= score_minimum)
```

### Paramètres

| Paramètre | Défaut | Rôle |
|-----------|--------|------|
| `poids_categorie` | 0.4 | Poids du signal catégorie |
| `poids_tfidf` | 0.6 | Poids du signal textuel |
| `seuil_tickets_technicien` | 20 | Réutilisé pour la pénalité charge |
| `periode_profil_mois` | 6 | Fenêtre glissante pour les profils |
| `score_minimum` | 0.05 | Seuil d'exclusion des suggestions |

## Structures de données

### Rust

```rust
pub struct TechnicianProfile {
    pub technicien: String,
    pub nb_tickets_reference: usize,
    pub cat_distribution: HashMap<String, f64>,
    pub centroide_tfidf: Vec<(usize, f64)>,  // sparse
}

pub struct AssignmentRecommendation {
    pub ticket_id: i64,
    pub ticket_titre: String,
    pub ticket_categorie: Option<String>,
    pub suggestions: Vec<TechnicianSuggestion>,
}

pub struct TechnicianSuggestion {
    pub technicien: String,
    pub score_final: f64,
    pub score_competence: f64,
    pub score_categorie: f64,
    pub score_tfidf: f64,
    pub stock_actuel: usize,
    pub facteur_charge: f64,
}

pub struct ProfilingResult {
    pub profiles: Vec<TechnicianProfile>,
    pub vocabulary_size: usize,
    pub nb_tickets_analysed: usize,
    pub periode_from: String,
    pub periode_to: String,
}

pub struct RecommendationRequest {
    pub limit_per_ticket: Option<usize>,  // défaut 3
    pub score_minimum: Option<f64>,       // défaut 0.05
}
```

### TypeScript

```typescript
interface TechnicianSuggestion {
    technicien: string;
    score_final: number;
    score_competence: number;
    score_categorie: number;
    score_tfidf: number;
    stock_actuel: number;
    facteur_charge: number;
}

interface AssignmentRecommendation {
    ticket_id: number;
    ticket_titre: string;
    ticket_categorie: string | null;
    suggestions: TechnicianSuggestion[];
}

interface ProfilingResult {
    profiles_count: number;
    vocabulary_size: number;
    nb_tickets_analysed: number;
    periode_from: string;
    periode_to: string;
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
- **État vide** : message + icône check si aucun ticket non attribué
- **Pas de pagination** en v1 (quelques dizaines de tickets attendus)

### Design system

Respect intégral de CLAUDE.md : cards rounded-2xl, shadow-1/shadow-2 hover, DM Sans pour les valeurs, Source Sans 3 pour le texte, pas de bordures, couleurs sémantiques.

## Tests

### Unitaires Rust

1. **Profiling** : distribution catégories somme à 1.0, fenêtre 6 mois respectée, technicien sans tickets → pas de profil
2. **Scoring** : score catégorie exact, fallback TF-IDF quand catégorie absente, facteur charge correct (stock=0→1.0, stock=seuil→0.5), tri DESC, seuil minimum respecté
3. **Intégration** : pipeline complet sur jeu synthétique (10 techniciens, 500 tickets), cache écriture/relecture

### Backtesting

Sur les tickets résolus des 3 derniers mois : masquer `technicien_principal`, lancer le moteur, comparer.

- **Top-1 accuracy** : technicien réel en position 1 (cible > 40%)
- **Top-3 accuracy** : technicien réel dans le top 3 (cible > 70%)
- **Distribution de charge** : écart-type des stocks proposés < écart-type réel

### Métriques de suivi

- Nombre de tickets non attribués
- Score moyen du top 1
- Couverture : % de tickets avec au moins 1 suggestion > 0.3
