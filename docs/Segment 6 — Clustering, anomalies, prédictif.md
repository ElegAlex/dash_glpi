# Segment 6 — Clustering, anomalies, prédictif

**Le pipeline ML complet (clustering K-Means + détection d'anomalies + prédiction de charge) pour 10 000 tickets ITSM français s'exécute en moins de 2 secondes en Rust natif, avec linfa-clustering v0.8.1 pour le K-Means, des z-scores sur délais log-transformés pour les anomalies statistiques, et augurs v0.10.1 (Grafana Labs) pour la prédiction de charge par décomposition saisonnière MSTL + AutoETS.** L'écosystème Rust a franchi en 2025-2026 un seuil de maturité critique : linfa fournit un K-Means production avec inertie et silhouette intégrées, augurs porte Prophet en Rust pur avec optimiseur WASM embarqué, et candle v0.9.2 ou ort v2.0 permettent l'embedding de phrases françaises sur desktop en moins de 30 secondes. La contrainte principale reste la conversion creuse → dense de la matrice TF-IDF (~200 MB en f32 pour 10 000 × 5 000), gérable sur les postes modernes mais motivant une réduction de dimensionnalité préalable.

Ce segment spécifie l'architecture du module `analytics/` qui consomme la matrice TF-IDF produite par le Segment 5, les métriques temporelles du Segment 3, et les catégories hiérarchiques du Segment 4. Il s'intègre dans l'application desktop Tauri 2.10+ (Rust + React 19) sous Windows, avec mise en cache des résultats dans la table `analytics_cache` définie au Segment 2.

---

## 1. K-Means avec linfa-clustering : état du crate et paramétrage

### 1.1 État de linfa en février 2026

Le framework **linfa** est l'écosystème ML de référence en Rust, avec **4 500+ étoiles GitHub**, 51 contributeurs, et un développement actif jusqu'à novembre 2025 (nouveaux algorithmes LARS, méthodes d'ensemble). La version courante est **v0.8.1** pour le méta-crate et les sous-crates. Le rythme de développement est modéré mais constant — une discussion ouverte (issue #367) sur la gouvernance long terme indique une communauté qui s'organise pour la pérennité.

**linfa-clustering v0.8.1** fournit K-Means, DBSCAN, Gaussian Mixture Models (GMM), et OPTICS. Le K-Means est l'algorithme recommandé pour notre cas d'usage : les clusters de tickets ITSM sont typiquement convexes et de taille comparable, conditions idéales pour K-Means.

|Crate|Version|Algorithmes|Maintenance|Pertinence|
|---|---|---|---|---|
|**linfa-clustering**|0.8.1|K-Means, DBSCAN, GMM, OPTICS|✅ Actif|**HAUTE** — cœur du clustering|
|**linfa-reduction**|0.8.1|PCA, SVD tronquée|✅ Actif|**HAUTE** — réduction dimensionnalité|
|**linfa-svm**|0.8.1|SVM linéaire/RBF|✅ Actif|**HAUTE** — classification Phase 2|
|**linfa-nn**|0.8.1|KNN, ball-tree|✅ Actif|**MOYENNE** — graphe de voisinage|
|**petal-clustering**|0.13|DBSCAN, HDBSCAN, OPTICS|✅ Actif|**MOYENNE** — clusters sans k fixe|
|**kneed**|1.0.0|Algorithme Kneedle|⚠️ Récent|**MOYENNE** — détection du coude|
|**extended-isolation-forest**|0.2|EIF (Hariri 2018)|✅ Actif|**HAUTE** — anomalies multi-dim|
|**augurs**|0.10.1|MSTL, ETS, Prophet, saisonnalité|✅ Actif (Grafana)|**HAUTE** — prédiction charge|

### 1.2 Initialisation K-Means++

L'implémentation linfa expose quatre stratégies d'initialisation via l'enum `KMeansInit` :

- **`KMeansInit::KMeansPlusPlus`** (défaut, recommandé) — sélection pondérée par la distance aux centroïdes existants, convergence 5-10× plus rapide que l'initialisation aléatoire
- **`KMeansInit::KMeansPara`** — variante parallélisée de K-Means++ pour > 100 clusters (hors périmètre ici)
- **`KMeansInit::Random`** — initialisation uniforme, utile uniquement pour reproduire des résultats
- **`KMeansInit::Precomputed(centroids)`** — centroïdes initiaux fournis manuellement (warm start après un premier clustering)

Pour les tickets ITSM de la CPAM, **K-Means++ est le seul choix raisonnable**. L'initialisation aléatoire produit des résultats instables sur des données textuelles à haute dimensionnalité.

### 1.3 Configuration et API

```rust
use linfa::prelude::*;
use linfa_clustering::KMeans;
use ndarray::Array2;

/// Configuration K-Means optimisée pour les tickets ITSM.
/// 
/// - n_clusters : 5-15 pour les catégories ITSM typiques
/// - tolerance : 1e-4 sur le mouvement des centroïdes
/// - max_iterations : 300 (largement suffisant pour 10K points)
/// - n_runs : 10 exécutions avec meilleur résultat par inertie
pub fn cluster_tickets(
    tfidf_dense: &Array2<f32>,
    n_clusters: usize,
) -> Result<KMeansResult, linfa::Error> {
    let dataset = DatasetBase::from(tfidf_dense.clone());

    let model = KMeans::params_with_rng(n_clusters, rand::thread_rng())
        .tolerance(1e-4)
        .max_n_iterations(300)
        .n_runs(10)                    // 10 random restarts, garde le meilleur
        .init_method(KMeansInit::KMeansPlusPlus)
        .fit(&dataset)?;

    // Inertie (WCSS) directement disponible
    let inertia = model.inertia();

    // Prédictions : cluster assigné à chaque ticket
    let predictions = model.predict(&dataset);

    // Centroïdes : Array2<f32> de shape (n_clusters, n_features)
    let centroids = model.centroids().clone();

    Ok(KMeansResult {
        model,
        inertia,
        predictions: predictions.targets().to_vec(),
        centroids,
    })
}

/// Résultat du clustering K-Means.
#[derive(Debug)]
pub struct KMeansResult {
    pub model: KMeans<f32, ndarray::Ix2>,
    pub inertia: f64,
    pub predictions: Vec<usize>,    // Cluster ID par ticket (0..n_clusters)
    pub centroids: Array2<f32>,
}
```

### 1.4 Compatibilité avec les matrices creuses sprs

linfa-clustering opère **exclusivement sur `ndarray::Array2<F>`** encapsulé dans `DatasetBase`. Il ne consomme pas directement les `sprs::CsMat`. La conversion s'effectue via `CsMat::to_dense()`, qui retourne un `ndarray::Array2` — les deux crates sont compatibles sur **ndarray 0.16.x**.

**Coût mémoire de la conversion** : 10 000 × 5 000 × 4 octets = **200 MB en f32** (400 MB en f64). C'est faisable sur un poste CPAM avec 8+ Go de RAM, mais significatif. Trois stratégies de mitigation existent :

|Stratégie|Gain mémoire|Impact qualité|Complexité|
|---|---|---|---|
|**f32 au lieu de f64**|÷ 2 (200 MB → 100 MB)|Négligeable pour TF-IDF|Trivial|
|**Réduction dimensionnalité (SVD tronquée)**|÷ 15-50 (200 MB → 4-13 MB)|Améliore souvent les clusters|Modéré|
|**K-Means creux custom avec sprs**|~3 MB (sparse natif)|Identique|Élevé (~200 lignes)|

**La stratégie recommandée est f32 + SVD tronquée** : réduire les 5 000 dimensions à 100-300 via `linfa-reduction` avant le clustering. Cela résout simultanément le problème mémoire, accélère la convergence K-Means, rend le silhouette score tractable, et produit des clusters plus significatifs en éliminant le bruit dimensionnel.

```rust
use linfa_reduction::Pca;

/// Réduit la matrice TF-IDF dense à n_components dimensions via SVD tronquée.
/// 
/// Pour 10 000 × 5 000 → 10 000 × 200 :
/// - Mémoire : 200 MB → 8 MB
/// - Temps K-Means : ~500 ms → ~50 ms
/// - Silhouette : O(n² × 200) au lieu de O(n² × 5000)
pub fn reduce_dimensions(
    tfidf_dense: &Array2<f32>,
    n_components: usize,
) -> Result<Array2<f32>, linfa::Error> {
    let dataset = DatasetBase::from(tfidf_dense.clone());
    let embedding = Pca::params(n_components)
        .fit(&dataset)?;
    let reduced = embedding.predict(&dataset);
    Ok(reduced.records().clone())
}
```

---

## 2. Elbow method et silhouette score : sélection du k optimal

### 2.1 Elbow method avec calcul d'inertie

L'elbow method consiste à exécuter K-Means pour k=2..K_max, collecter l'inertie (Within-Cluster Sum of Squares — WCSS) à chaque k, et identifier le « coude » de la courbe — le point où l'ajout d'un cluster supplémentaire n'apporte plus de réduction significative de l'inertie.

Pour des tickets ITSM, **k=3..20 est la plage standard**, correspondant aux catégories typiques d'un helpdesk : matériel, logiciel, réseau, accès/habilitations, messagerie, impression, téléphonie, etc. Le k optimal se situe généralement entre **5 et 12** pour un corpus CPAM.

```rust
/// Résultat de l'analyse elbow pour un k donné.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElbowPoint {
    pub k: usize,
    pub inertia: f64,
    pub silhouette: Option<f64>,     // Calculé uniquement si demandé (coûteux)
    pub duration_ms: u64,
}

/// Exécute l'analyse elbow pour k_min..=k_max.
/// 
/// Pour k=3..20 sur 10 000 × 200 (après SVD) : ~5-15 secondes total.
/// Progress reporté via le channel Tauri.
pub fn elbow_analysis(
    data: &Array2<f32>,
    k_min: usize,
    k_max: usize,
    compute_silhouette: bool,
    progress: Option<&tauri::ipc::Channel<ImportProgress>>,
) -> Result<Vec<ElbowPoint>, String> {
    let mut results = Vec::with_capacity(k_max - k_min + 1);
    let total_steps = k_max - k_min + 1;

    for (i, k) in (k_min..=k_max).enumerate() {
        let start = std::time::Instant::now();

        let dataset = DatasetBase::from(data.clone());
        let model = KMeans::params_with_rng(k, rand::thread_rng())
            .tolerance(1e-4)
            .max_n_iterations(200)
            .n_runs(5)   // Moins de runs pour l'exploration (5 au lieu de 10)
            .init_method(KMeansInit::KMeansPlusPlus)
            .fit(&dataset)
            .map_err(|e| format!("K-Means k={k} échoué: {e}"))?;

        let inertia = model.inertia();

        // Silhouette score : coûteux, calculé sur un échantillon si demandé
        let silhouette = if compute_silhouette {
            Some(compute_silhouette_sampled(&dataset, &model, 2000)?)
        } else {
            None
        };

        let elapsed = start.elapsed();

        results.push(ElbowPoint {
            k,
            inertia,
            silhouette,
            duration_ms: elapsed.as_millis() as u64,
        });

        // Report progress
        if let Some(channel) = progress {
            let _ = channel.send(ImportProgress {
                step: format!("Elbow k={k}"),
                current: i + 1,
                total: total_steps,
            });
        }
    }

    Ok(results)
}
```

### 2.2 Détection automatique du coude : algorithme Kneedle

Le crate **`kneed` v1.0.0** (licence BSD-3-Clause) implémente l'algorithme Kneedle en Rust pur, portage du package Python `kneed`. Il accepte des vecteurs x/y et identifie le point de coude selon la direction de la courbe (décroissante) et sa forme (convexe).

```rust
use kneed::KneeLocator;

/// Détecte le k optimal via l'algorithme Kneedle sur la courbe d'inertie.
/// 
/// Retourne None si la courbe n'a pas de coude clair (rare mais possible
/// avec des données très homogènes ou très hétérogènes).
pub fn detect_elbow(elbow_points: &[ElbowPoint]) -> Option<usize> {
    let x: Vec<f64> = elbow_points.iter().map(|p| p.k as f64).collect();
    let y: Vec<f64> = elbow_points.iter().map(|p| p.inertia).collect();

    // Direction : decreasing (l'inertie diminue quand k augmente)
    // Courbe : convex (concavité vers le haut)
    let locator = KneeLocator::new(
        x.clone(),
        y.clone(),
        kneed::CurveDirection::Decreasing,
        kneed::CurveShape::Convex,
        1.0, // sensitivity (défaut)
    );

    locator.knee().map(|knee_x| knee_x as usize)
}
```

Avec seulement ~1 700 téléchargements totaux, le crate `kneed` est une commodité — l'algorithme est suffisamment simple pour être ré-implémenté en fallback (~30 lignes). L'idée centrale est de normaliser les courbes x et y dans [0, 1], calculer la différence entre la courbe et la ligne droite reliant les extrêmes, et trouver le maximum de cette différence.

**Fallback manuel** si `kneed` pose problème :

```rust
/// Détection du coude par différence maximale à la ligne droite.
/// Plus simple que Kneedle, fonctionne bien en pratique pour l'elbow K-Means.
pub fn detect_elbow_simple(ks: &[usize], inertias: &[f64]) -> Option<usize> {
    if ks.len() < 3 { return None; }
    let n = ks.len();

    // Normaliser dans [0, 1]
    let x_min = ks[0] as f64;
    let x_max = ks[n - 1] as f64;
    let y_min = inertias[n - 1];  // Plus petit (dernier k)
    let y_max = inertias[0];       // Plus grand (premier k)

    if (x_max - x_min).abs() < f64::EPSILON || (y_max - y_min).abs() < f64::EPSILON {
        return None;
    }

    let x_norm: Vec<f64> = ks.iter().map(|&k| (k as f64 - x_min) / (x_max - x_min)).collect();
    let y_norm: Vec<f64> = inertias.iter().map(|&y| (y - y_min) / (y_max - y_min)).collect();

    // Distance à la ligne droite (premier point → dernier point)
    // Ligne : y = 1 - x (après normalisation, décroissante)
    let mut max_dist = 0.0_f64;
    let mut best_idx = 0;

    for i in 0..n {
        let expected = 1.0 - x_norm[i];
        let diff = (y_norm[i] - expected).abs();
        if diff > max_dist {
            max_dist = diff;
            best_idx = i;
        }
    }

    Some(ks[best_idx])
}
```

### 2.3 Silhouette score

Le **silhouette score** mesure la cohérence des clusters : pour chaque point, il compare la distance moyenne aux points de son propre cluster (cohésion _a_) à la distance moyenne aux points du cluster le plus proche (séparation _b_). Le score par point est _(b - a) / max(a, b)_, et le score global est la moyenne sur tous les points.

**Complexité** : O(n² × d) — pour 10 000 points en 5 000 dimensions, c'est **prohibitif** (~250 milliards d'opérations). Deux solutions :

1. **Échantillonnage** : calculer le silhouette sur un sous-ensemble aléatoire de 1 000–2 000 points (recommandé)
2. **Réduction dimensionnelle préalable** : après SVD tronquée à 200 dimensions, le calcul sur 10 000 points prend ~2-5 secondes

linfa fournit le trait `SilhouetteScore` dans `linfa::metrics`, mais son implémentation est O(n²). L'approche recommandée est le calcul sur échantillon :

```rust
use linfa::metrics::SilhouetteScore;
use rand::seq::SliceRandom;

/// Calcule le silhouette score sur un échantillon aléatoire.
/// 
/// Pour 10 000 points × 200 dims avec sample_size=2000 : ~500 ms.
/// Le score est dans [-1, 1] :
///   > 0.5 = bonne structure de clusters
///   0.25–0.5 = structure faible mais exploitable
///   < 0.25 = pas de structure significative
pub fn compute_silhouette_sampled(
    dataset: &DatasetBase<Array2<f32>, ()>,
    model: &KMeans<f32, ndarray::Ix2>,
    sample_size: usize,
) -> Result<f64, String> {
    let n = dataset.records().nrows();
    let actual_sample = sample_size.min(n);

    if actual_sample == n {
        // Calcul complet si le dataset est petit
        let predictions = model.predict(dataset);
        return predictions.silhouette_score()
            .map_err(|e| format!("Silhouette échoué: {e}"));
    }

    // Échantillonnage aléatoire
    let mut rng = rand::thread_rng();
    let mut indices: Vec<usize> = (0..n).collect();
    indices.shuffle(&mut rng);
    indices.truncate(actual_sample);
    indices.sort_unstable();

    let sampled_records = dataset.records().select(ndarray::Axis(0), &indices);
    let sampled_dataset = DatasetBase::from(sampled_records);
    let predictions = model.predict(&sampled_dataset);
    
    predictions.silhouette_score()
        .map_err(|e| format!("Silhouette échantillonné échoué: {e}"))
}
```

### 2.4 Interprétation des clusters pour les tickets ITSM

Après le clustering, chaque cluster reçoit un **label automatique** généré à partir de ses top keywords TF-IDF. Pour chaque cluster _c_, on calcule le centroïde (déjà fourni par linfa), on retrouve les indices des dimensions avec les poids les plus élevés, et on mappe ces indices vers le vocabulaire TF-IDF du Segment 5.

```rust
use crate::nlp::tfidf::TfIdfResult;

/// Label auto-généré pour un cluster à partir de ses top keywords.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterLabel {
    pub cluster_id: usize,
    pub label: String,                    // "imprimante réseau pilote"
    pub top_keywords: Vec<KeywordWeight>,
    pub ticket_count: usize,
    pub ticket_ids: Vec<u64>,
    pub avg_resolution_days: Option<f64>,
    pub pct_incidents: f64,
    pub pct_demandes: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeywordWeight {
    pub word: String,
    pub weight: f32,
}

/// Génère les labels de clusters à partir des centroïdes et du vocabulaire TF-IDF.
///
/// Les top_n mots avec le poids le plus élevé dans chaque centroïde
/// forment le label du cluster. top_n=3 pour le label court,
/// top_n=10 pour la vue détaillée.
pub fn label_clusters(
    centroids: &Array2<f32>,
    vocabulary: &[String],      // Index → mot (depuis TfIdfResult)
    predictions: &[usize],
    ticket_ids: &[u64],
    top_n: usize,
) -> Vec<ClusterLabel> {
    let n_clusters = centroids.nrows();
    let mut labels = Vec::with_capacity(n_clusters);

    for c in 0..n_clusters {
        let centroid = centroids.row(c);

        // Top keywords par poids décroissant dans le centroïde
        let mut indexed_weights: Vec<(usize, f32)> = centroid
            .iter()
            .enumerate()
            .map(|(i, &w)| (i, w))
            .collect();
        indexed_weights.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let top_keywords: Vec<KeywordWeight> = indexed_weights.iter()
            .take(top_n)
            .filter(|(i, _)| *i < vocabulary.len())
            .map(|(i, w)| KeywordWeight {
                word: vocabulary[*i].clone(),
                weight: *w,
            })
            .collect();

        // Label = concaténation des top 3 mots
        let label = top_keywords.iter()
            .take(3)
            .map(|kw| kw.word.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        // Tickets dans ce cluster
        let cluster_ticket_ids: Vec<u64> = predictions.iter()
            .enumerate()
            .filter(|(_, &pred)| pred == c)
            .map(|(i, _)| ticket_ids[i])
            .collect();

        labels.push(ClusterLabel {
            cluster_id: c,
            label,
            top_keywords,
            ticket_count: cluster_ticket_ids.len(),
            ticket_ids: cluster_ticket_ids,
            avg_resolution_days: None,  // Rempli par la couche SQL
            pct_incidents: 0.0,
            pct_demandes: 0.0,
        });
    }

    labels
}
```

### 2.5 Enrichissement SQL des clusters

Les métriques métier (délai moyen, ratio incidents/demandes, répartition par groupe) sont calculées en SQL après le clustering, en stockant l'assignation cluster dans une table temporaire :

```sql
-- Création de la table temporaire d'assignation cluster
CREATE TEMP TABLE IF NOT EXISTS ticket_clusters (
    ticket_id INTEGER PRIMARY KEY,
    cluster_id INTEGER NOT NULL
);

-- Statistiques par cluster enrichies avec les données métier
SELECT
    tc.cluster_id,
    COUNT(*) AS ticket_count,
    SUM(CASE WHEN t.type_ticket = 'Incident' THEN 1 ELSE 0 END) AS incidents,
    SUM(CASE WHEN t.type_ticket = 'Demande' THEN 1 ELSE 0 END) AS demandes,
    ROUND(AVG(t.anciennete_jours), 1) AS age_moyen,
    ROUND(AVG(CASE WHEN t.est_vivant = 0 
        THEN julianday(t.date_cloture_approx) - julianday(t.date_ouverture) 
        ELSE NULL END), 1) AS delai_resolution_moyen,
    GROUP_CONCAT(DISTINCT t.groupe_niveau2) AS groupes_concernes,
    SUM(CASE WHEN t.est_vivant = 1 THEN 1 ELSE 0 END) AS vivants,
    SUM(CASE WHEN t.nombre_suivis = 0 THEN 1 ELSE 0 END) AS sans_suivi
FROM ticket_clusters tc
JOIN tickets t ON t.id = tc.ticket_id AND t.import_id = ?1
GROUP BY tc.cluster_id
ORDER BY ticket_count DESC;
```

### 2.6 Alternatives au K-Means : HDBSCAN pour la découverte exploratoire

Le K-Means impose un nombre fixe de clusters. Pour une **exploration initiale** — découvrir combien de groupes thématiques existent naturellement dans les tickets — **HDBSCAN** est plus adapté. Le crate **petal-clustering v0.13** fournit HDBSCAN avec détection de clusters à densité variable et identification automatique du bruit (tickets inclassables).

```rust
use petal_clustering::{HDbscan, Fit};

/// HDBSCAN pour découverte exploratoire du nombre naturel de clusters.
///
/// min_cluster_size = 50 : un cluster doit contenir au moins 50 tickets
/// pour être significatif dans un contexte ITSM CPAM.
/// Les points classés -1 sont du bruit (tickets inclassables).
pub fn discover_clusters(data: &Array2<f64>, min_cluster_size: usize) -> HdbscanResult {
    let mut hdbscan = HDbscan {
        min_cluster_size,
        min_samples: None,       // Défaut = min_cluster_size
        alpha: 1.0,
        metric: petal_clustering::Metric::Euclidean,
    };

    let labels = hdbscan.fit(data.view());
    let n_clusters = labels.iter().filter(|&&l| l >= 0).max().map_or(0, |&m| m as usize + 1);
    let n_noise = labels.iter().filter(|&&l| l < 0).count();

    HdbscanResult {
        labels,
        n_clusters,
        n_noise,
        noise_ratio: n_noise as f64 / data.nrows() as f64,
    }
}

#[derive(Debug)]
pub struct HdbscanResult {
    pub labels: Vec<i64>,
    pub n_clusters: usize,
    pub n_noise: usize,
    pub noise_ratio: f64,
}
```

Le workflow recommandé est : HDBSCAN en exploration → noter le nombre de clusters découverts → utiliser ce nombre comme k initial pour K-Means → affiner avec l'elbow method.

---

## 3. Détection d'anomalies : architecture à trois niveaux

La détection d'anomalies dans les tickets ITSM suit une **architecture à trois niveaux** : statistique (z-score, IQR) pour les cas évidents, cluster-based pour les anomalies sémantiques, et Isolation Forest pour les anomalies multi-dimensionnelles complexes. On s'attend à **1–5% de tickets anomaliques** dans un contexte ITSM typique, soit 100–500 sur les 10 000 tickets CPAM.

### 3.1 Niveau 1 — Anomalies statistiques : z-score sur délais log-transformés

Les délais de résolution des tickets sont **systématiquement log-normalement distribués** : toujours positifs, asymétriques à droite avec une queue lourde. Appliquer un z-score aux délais bruts produit des résultats trompeurs car les valeurs extrêmes gonflent la moyenne. L'approche correcte est de **log-transformer d'abord** (`ln(délai_heures + 1)`), puis de calculer le z-score sur les valeurs transformées.

```rust
/// Détection d'anomalies par z-score sur délais log-transformés.
///
/// Seuils ITSM recommandés :
///   |z| > 2.0 → "warning" (~4.6% des tickets)
///   |z| > 3.0 → "critical" (~0.3% des tickets)
///
/// Les z-scores négatifs (résolution anormalement rapide) peuvent indiquer
/// des tickets clos sans résolution réelle.
pub fn detect_delay_anomalies(
    tickets: &[(u64, f64)],  // (ticket_id, délai_jours)
    threshold_warning: f64,   // 2.0
    threshold_critical: f64,  // 3.0
) -> Vec<DelayAnomaly> {
    if tickets.is_empty() { return vec![]; }

    // Log-transformation (ln(x + 1) pour gérer les délais = 0)
    let log_delays: Vec<f64> = tickets.iter()
        .map(|(_, d)| (d + 1.0).ln())
        .collect();

    let mean = log_delays.iter().sum::<f64>() / log_delays.len() as f64;
    let std_dev = {
        let variance = log_delays.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / log_delays.len() as f64;
        variance.sqrt()
    };

    if std_dev < f64::EPSILON {
        return vec![]; // Tous les délais sont identiques
    }

    let mut anomalies = Vec::new();
    for (i, &log_d) in log_delays.iter().enumerate() {
        let z = (log_d - mean) / std_dev;
        let severity = if z.abs() > threshold_critical {
            AnomalySeverity::Critical
        } else if z.abs() > threshold_warning {
            AnomalySeverity::Warning
        } else {
            continue;
        };

        anomalies.push(DelayAnomaly {
            ticket_id: tickets[i].0,
            delay_days: tickets[i].1,
            z_score: z,
            severity,
            anomaly_direction: if z > 0.0 {
                AnomalyDirection::TropLent
            } else {
                AnomalyDirection::TropRapide
            },
        });
    }

    // Trier par sévérité décroissante
    anomalies.sort_by(|a, b| b.z_score.abs().partial_cmp(&a.z_score.abs()).unwrap());
    anomalies
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DelayAnomaly {
    pub ticket_id: u64,
    pub delay_days: f64,
    pub z_score: f64,
    pub severity: AnomalySeverity,
    pub anomaly_direction: AnomalyDirection,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum AnomalySeverity { Warning, Critical }

#[derive(Debug, Clone, serde::Serialize)]
pub enum AnomalyDirection { TropLent, TropRapide }
```

### 3.2 Z-score modifié (robuste aux outliers extrêmes)

Quand quelques tickets « fossiles » ouverts depuis des années corrompent la moyenne et l'écart-type, le **z-score modifié** utilisant la médiane et le MAD (Median Absolute Deviation) est plus robuste. Le seuil standard est **3.5** (Iglewicz & Hoaglin, 1993).

```rust
/// Z-score modifié basé sur la médiane et le MAD.
///
/// MAD = médiane(|xi - médiane(x)|)
/// z_modifié = 0.6745 × (xi - médiane) / MAD
///
/// Le facteur 0.6745 normalise pour que le MAD d'une distribution
/// normale soit égal à l'écart-type.
/// Seuil recommandé : |z_modifié| > 3.5
pub fn modified_z_scores(values: &[f64]) -> Option<Vec<f64>> {
    if values.len() < 3 { return None; }

    let median = crate::stats::mediane(values)?;
    let abs_deviations: Vec<f64> = values.iter()
        .map(|x| (x - median).abs())
        .collect();
    let mad = crate::stats::mediane(&abs_deviations)?;

    if mad < f64::EPSILON {
        // MAD = 0 signifie que > 50% des valeurs sont identiques
        // Toute valeur différente de la médiane est suspecte
        return Some(values.iter()
            .map(|x| if (x - median).abs() < f64::EPSILON { 0.0 } else { f64::INFINITY })
            .collect());
    }

    let scores: Vec<f64> = values.iter()
        .map(|x| 0.6745 * (x - median) / mad)
        .collect();
    Some(scores)
}
```

### 3.3 Méthode IQR en complément

La méthode IQR (Q1 − k×IQR, Q3 + k×IQR) est un complément robuste au z-score, particulièrement utile quand des problèmes de qualité de données existent (tickets laissés ouverts par erreur pendant des semaines). Utiliser **k=1.5 pour les outliers standard** et **k=3.0 pour les cas extrêmes**.

```rust
/// Détection d'outliers par méthode IQR.
///
/// k=1.5 → outliers standards (convention Tukey)
/// k=3.0 → outliers extrêmes
pub fn iqr_outliers(values: &[f64], k: f64) -> IqrResult {
    let q1 = crate::stats::percentile(values, 25.0).unwrap_or(0.0);
    let q3 = crate::stats::percentile(values, 75.0).unwrap_or(0.0);
    let iqr = q3 - q1;

    let lower_bound = q1 - k * iqr;
    let upper_bound = q3 + k * iqr;

    let outlier_indices: Vec<usize> = values.iter()
        .enumerate()
        .filter(|(_, &v)| v < lower_bound || v > upper_bound)
        .map(|(i, _)| i)
        .collect();

    IqrResult {
        q1, q3, iqr, lower_bound, upper_bound,
        n_outliers: outlier_indices.len(),
        outlier_indices,
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IqrResult {
    pub q1: f64,
    pub q3: f64,
    pub iqr: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub n_outliers: usize,
    pub outlier_indices: Vec<usize>,
}
```

### 3.4 Niveau 2 — Anomalies par distance au centroïde

Après le clustering K-Means, on calcule la distance euclidienne de chaque ticket à son centroïde assigné. Les tickets dans le **top 5% des distances intra-cluster** sont des « misfits » qui n'appartiennent bien à aucune catégorie. Cette approche capture les anomalies sémantiques que les méthodes statistiques sur caractéristiques isolées manquent — un ticket réseau classé sous « imprimante », par exemple.

```rust
/// Détection d'anomalies par distance au centroïde du cluster.
///
/// Deux stratégies de seuil :
///   - Percentile : top 5% par cluster (recommandé)
///   - Écart-type : distance > μ_d + 2σ_d au sein de chaque cluster
pub fn centroid_distance_anomalies(
    data: &Array2<f32>,
    predictions: &[usize],
    centroids: &Array2<f32>,
    percentile_threshold: f64,  // 95.0 pour le top 5%
) -> Vec<CentroidAnomaly> {
    let n_clusters = centroids.nrows();
    let mut anomalies = Vec::new();

    for c in 0..n_clusters {
        // Indices et distances des tickets dans ce cluster
        let cluster_data: Vec<(usize, f64)> = predictions.iter()
            .enumerate()
            .filter(|(_, &pred)| pred == c)
            .map(|(i, _)| {
                let point = data.row(i);
                let centroid = centroids.row(c);
                let dist = point.iter()
                    .zip(centroid.iter())
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f32>()
                    .sqrt() as f64;
                (i, dist)
            })
            .collect();

        if cluster_data.is_empty() { continue; }

        let distances: Vec<f64> = cluster_data.iter().map(|(_, d)| *d).collect();
        let threshold = crate::stats::percentile(&distances, percentile_threshold)
            .unwrap_or(f64::MAX);

        for (idx, dist) in &cluster_data {
            if *dist > threshold {
                anomalies.push(CentroidAnomaly {
                    ticket_index: *idx,
                    cluster_id: c,
                    distance: *dist,
                    threshold,
                    ratio: dist / threshold, // > 1.0 = anomalie
                });
            }
        }
    }

    anomalies.sort_by(|a, b| b.ratio.partial_cmp(&a.ratio).unwrap());
    anomalies
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CentroidAnomaly {
    pub ticket_index: usize,
    pub cluster_id: usize,
    pub distance: f64,
    pub threshold: f64,
    pub ratio: f64,
}
```

### 3.5 Niveau 3 — Extended Isolation Forest pour anomalies multi-dimensionnelles

Le crate **`extended-isolation-forest`** (par nmandery) implémente l'algorithme EIF amélioré (Hariri et al., 2018), qui utilise des coupes par hyperplans au lieu de coupes alignées sur les axes, produisant des scores d'anomalie plus fiables. L'API utilise des const-generics pour la dimensionnalité.

Pour la détection multi-dimensionnelle, on combine **4 caractéristiques z-normalisées** : délai de résolution (log-transformé), nombre de suivis, priorité (encodée numériquement), et distance au centroïde du cluster. Ces caractéristiques capturent des anomalies composites invisibles aux méthodes univariées — un ticket de priorité basse avec un nombre élevé de suivis et un délai anormalement court suggère un problème de classification initiale.

```rust
use extended_isolation_forest::{Forest, ForestOptions};

/// Dimensions de l'espace de caractéristiques pour l'Isolation Forest.
/// 4 features : log_délai, nb_suivis_norm, priorité_norm, distance_centroïde_norm
const N_FEATURES: usize = 4;

/// Prépare les données et entraîne un Extended Isolation Forest.
///
/// Entraînement : ~200-500 ms pour 10 000 tickets, 150 arbres.
/// Scoring : ~50-100 ms pour 10 000 tickets.
pub fn train_isolation_forest(
    features: &[[f64; N_FEATURES]],
) -> Result<Forest<f64, N_FEATURES>, String> {
    let options = ForestOptions {
        n_trees: 150,
        sample_size: 256,
        max_tree_depth: None,   // Auto (ceil(log2(256)) = 8)
        extension_level: 1,     // Hyperplans aléatoires (0 = axis-aligned classique)
    };

    Forest::from_slice(features, &options)
        .map_err(|e| format!("Erreur Isolation Forest: {e:?}"))
}

/// Score d'anomalie pour chaque ticket.
///
/// score > 0.5 → probable anomalie
/// score > 0.7 → anomalie forte
/// score < 0.5 → comportement normal
pub fn score_anomalies(
    forest: &Forest<f64, N_FEATURES>,
    features: &[[f64; N_FEATURES]],
    threshold: f64,  // 0.6 recommandé pour ITSM
) -> Vec<IsolationAnomaly> {
    let mut anomalies: Vec<IsolationAnomaly> = features.iter()
        .enumerate()
        .map(|(i, feat)| {
            let score = forest.score(feat);
            IsolationAnomaly {
                ticket_index: i,
                anomaly_score: score,
                is_anomaly: score > threshold,
                features: *feat,
            }
        })
        .filter(|a| a.is_anomaly)
        .collect();

    anomalies.sort_by(|a, b| b.anomaly_score.partial_cmp(&a.anomaly_score).unwrap());
    anomalies
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IsolationAnomaly {
    pub ticket_index: usize,
    pub anomaly_score: f64,
    pub is_anomaly: bool,
    pub features: [f64; N_FEATURES],
}
```

### 3.6 Préparation des caractéristiques pour l'Isolation Forest

```rust
/// Normalise les caractéristiques et les combine pour l'Isolation Forest.
///
/// Chaque feature est z-normalisée (μ=0, σ=1) pour que toutes les
/// dimensions aient le même poids dans la détection d'anomalies.
pub fn prepare_isolation_features(
    log_delays: &[f64],
    followup_counts: &[f64],
    priorities: &[f64],           // 1=très basse .. 6=majeure
    centroid_distances: &[f64],
) -> Vec<[f64; N_FEATURES]> {
    let n = log_delays.len();
    assert_eq!(n, followup_counts.len());
    assert_eq!(n, priorities.len());
    assert_eq!(n, centroid_distances.len());

    // Z-normalisation par feature
    let norm_delays = z_normalize(log_delays);
    let norm_followups = z_normalize(followup_counts);
    let norm_priorities = z_normalize(priorities);
    let norm_distances = z_normalize(centroid_distances);

    (0..n)
        .map(|i| [norm_delays[i], norm_followups[i], norm_priorities[i], norm_distances[i]])
        .collect()
}

fn z_normalize(values: &[f64]) -> Vec<f64> {
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let std = {
        let var = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64;
        var.sqrt()
    };
    if std < f64::EPSILON {
        return vec![0.0; values.len()];
    }
    values.iter().map(|x| (x - mean) / std).collect()
}
```

### 3.7 Orchestration de la détection d'anomalies

```rust
/// Résultat consolidé de la détection d'anomalies multi-niveaux.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnomalyReport {
    pub statistical_anomalies: Vec<DelayAnomaly>,
    pub centroid_anomalies: Vec<CentroidAnomaly>,
    pub isolation_anomalies: Vec<IsolationAnomaly>,
    pub combined_anomalies: Vec<CombinedAnomaly>,
    pub summary: AnomalySummary,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CombinedAnomaly {
    pub ticket_id: u64,
    pub titre: String,
    pub anomaly_types: Vec<String>,     // ["delai_anormal", "hors_cluster", "isolation"]
    pub max_severity: AnomalySeverity,
    pub description: String,             // Texte explicatif pour l'analyste
    pub recommended_action: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnomalySummary {
    pub total_tickets_analyzed: usize,
    pub total_anomalies: usize,
    pub pct_anomalies: f64,
    pub by_level: AnomalyCountByLevel,
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnomalyCountByLevel {
    pub statistical: usize,
    pub centroid: usize,
    pub isolation: usize,
    pub multi_level: usize,  // Détecté par ≥ 2 niveaux
}
```

Les tickets détectés par **deux ou trois niveaux simultanément** méritent une attention prioritaire — ils sont à la fois statistiquement aberrants, sémantiquement mal classés, et atypiques en profil multi-dimensionnel.

---

## 4. Prédiction de charge : séries temporelles avec augurs

### 4.1 Le crate augurs : toolkit complet de séries temporelles

**augurs v0.10.1** (Grafana Labs, septembre 2025) est le toolkit de séries temporelles le plus complet en Rust. Avec **552+ étoiles GitHub** et 255 releases, il fournit des sous-crates modulaires derrière des feature flags : décomposition MSTL, lissage exponentiel automatique (AutoETS), un **portage complet de Prophet en Rust**, détection de saisonnalité par périodogramme, détection de ruptures, et détection d'outliers.

|Sous-crate|Fonction|Pertinence|
|---|---|---|
|**augurs-mstl**|Décomposition saisonnière multi-périodes (MSTL)|**HAUTE** — semaine + mois|
|**augurs-ets**|AutoETS (sélection automatique du meilleur modèle ETS)|**HAUTE** — tendance|
|**augurs-seasons**|Détection saisonnalité par périodogramme|**HAUTE** — validation|
|**augurs-prophet**|Port complet de Facebook Prophet en Rust|**HAUTE** — jours fériés|
|**augurs-forecaster**|API unifiée de prédiction|**HAUTE** — orchestration|
|**augurs-changepoint**|Détection de ruptures dans la série|**MOYENNE** — monitoring|
|**augurs-outlier**|Détection d'outliers dans la série|**MOYENNE** — nettoyage|

### 4.2 MSTL + AutoETS : moteur de prédiction principal

L'approche recommandée combine **MSTL** (Multiple Seasonal-Trend decomposition using LOESS) pour la décomposition saisonnière avec **AutoETS** pour la prédiction de tendance. MSTL gère plusieurs périodes saisonnières simultanément — hebdomadaire (période=7) et mensuelle (période≈30) — puis alimente la tendance désaisonnalisée vers AutoETS, qui sélectionne automatiquement le meilleur modèle de lissage exponentiel non-saisonnier.

Cette approche **surpasse Holt-Winters standalone** dans la plupart des benchmarks et contourne la limitation que l'ETS d'augurs ne supporte pas encore directement les modèles saisonniers.

```rust
use augurs::mstl::MSTLModel;
use augurs::ets::AutoETS;
use augurs::prelude::*;

/// Prédiction du volume de tickets pour les N prochains jours.
///
/// Entrée : série temporelle de comptages journaliers de tickets.
/// Sortie : prédictions avec intervalle de confiance à 95%.
///
/// Saisonnalités modélisées :
///   - Hebdomadaire (période=7) : pics lundi, creux week-end
///   - Mensuelle (période=30) : cycles liés aux échéances de la CPAM
pub fn predict_ticket_volume(
    daily_counts: &[f64],
    forecast_horizon: usize,    // Typiquement 7 (semaine) ou 30 (mois)
    confidence_level: f64,      // 0.95 pour 95%
) -> Result<ForecastResult, String> {
    if daily_counts.len() < 60 {
        return Err(format!(
            "Historique insuffisant : {} jours (minimum 60 pour saisonnalité mensuelle)",
            daily_counts.len()
        ));
    }

    let periods = vec![7, 30]; // Saisonnalité hebdomadaire + mensuelle
    let trend_model = AutoETS::non_seasonal().into_trend_model();
    let mstl = MSTLModel::new(periods, trend_model);

    let fit = mstl.fit(daily_counts)
        .map_err(|e| format!("Erreur MSTL fit: {e}"))?;

    let forecast = fit.predict(forecast_horizon, confidence_level)
        .map_err(|e| format!("Erreur MSTL predict: {e}"))?;

    Ok(ForecastResult {
        point_forecasts: forecast.point.clone(),
        lower_bounds: forecast.intervals.as_ref()
            .map(|fi| fi.lower.clone())
            .unwrap_or_default(),
        upper_bounds: forecast.intervals.as_ref()
            .map(|fi| fi.upper.clone())
            .unwrap_or_default(),
        confidence_level,
        horizon: forecast_horizon,
    })
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastResult {
    pub point_forecasts: Vec<f64>,
    pub lower_bounds: Vec<f64>,
    pub upper_bounds: Vec<f64>,
    pub confidence_level: f64,
    pub horizon: usize,
}
```

### 4.3 Détection automatique de saisonnalité

Le sous-crate **augurs-seasons** fournit une détection de saisonnalité par périodogramme, confirmant si les patterns hebdomadaires et mensuels existent réellement dans les données avant d'ajuster le modèle de décomposition.

```rust
use augurs::seasons::Detector;

/// Détecte les périodes saisonnières significatives dans la série.
///
/// Retourne les périodes détectées (ex: [7, 30] pour hebdo + mensuel).
/// Si aucune saisonnalité n'est détectée, utiliser un simple AutoETS
/// sans décomposition MSTL.
pub fn detect_seasonality(daily_counts: &[f64]) -> Vec<usize> {
    let detector = Detector::default();
    let periods = detector.detect(daily_counts);
    
    // Filtrer les périodes pertinentes pour l'ITSM CPAM
    periods.into_iter()
        .filter(|&p| p >= 5 && p <= 365) // Ignorer les micro-périodes et > 1 an
        .collect()
}
```

### 4.4 Prophet en Rust pour la modélisation des jours fériés

**augurs-prophet** est un portage complet de Facebook Prophet en Rust, utilisant le backend **`wasmstan`** — l'optimiseur Stan compilé en WebAssembly, ne nécessitant aucun binaire externe. C'est idéal pour une application Tauri autonome.

Prophet excelle pour modéliser les **11 jours fériés français**, les « ponts » (notamment le jeudi de l'Ascension), et la période de congés d'août qui réduit typiquement le volume de tickets CPAM de **30–50%**. Le modèle accepte des calendriers de jours fériés personnalisés avec fenêtres d'effet.

```rust
use augurs::prophet::{Prophet, TrainingData, Holiday, Regressor};
use augurs::prophet::wasmstan::WasmStanOptimizer;

/// Prédiction avec Prophet incluant les jours fériés français.
///
/// Avantage par rapport à MSTL + AutoETS : modélisation explicite
/// des effets de calendrier (fériés, ponts, août).
/// Inconvénient : plus lent (2-10s vs ~100ms).
pub fn predict_with_prophet(
    timestamps: &[i64],         // Unix timestamps des dates
    daily_counts: &[f64],
    forecast_horizon: usize,
    holidays: Vec<Holiday>,
) -> Result<ForecastResult, String> {
    let optimizer = WasmStanOptimizer::new();
    
    let mut prophet = Prophet::new(Default::default(), optimizer);

    let training = TrainingData::new(
        timestamps.to_vec(),
        daily_counts.to_vec(),
    ).map_err(|e| format!("Erreur données Prophet: {e}"))?;

    // Ajouter les jours fériés
    for holiday in holidays {
        prophet.add_holiday(holiday);
    }

    prophet.fit(training, Default::default())
        .map_err(|e| format!("Erreur Prophet fit: {e}"))?;

    // Générer les dates futures
    let last_ts = *timestamps.last().unwrap();
    let day_secs = 86_400;
    let future_ts: Vec<i64> = (1..=forecast_horizon as i64)
        .map(|d| last_ts + d * day_secs)
        .collect();

    let predictions = prophet.predict(Some(future_ts))
        .map_err(|e| format!("Erreur Prophet predict: {e}"))?;

    Ok(ForecastResult {
        point_forecasts: predictions.yhat.point.clone(),
        lower_bounds: predictions.yhat.lower.unwrap_or_default(),
        upper_bounds: predictions.yhat.upper.unwrap_or_default(),
        confidence_level: 0.95,
        horizon: forecast_horizon,
    })
}

/// Calendrier des jours fériés français pour Prophet.
///
/// Les fériés à date fixe sont définis une fois.
/// Les fériés mobiles (Pâques, Ascension, Pentecôte) doivent être
/// précalculés année par année.
pub fn french_holidays_2025_2027() -> Vec<Holiday> {
    vec![
        // 2025
        holiday("jour_an", "2025-01-01", 0, 0),
        holiday("paques", "2025-04-21", -1, 1),        // Lundi de Pâques
        holiday("fete_travail", "2025-05-01", 0, 0),
        holiday("victoire_1945", "2025-05-08", 0, 0),
        holiday("ascension", "2025-05-29", -1, 1),      // + pont vendredi
        holiday("pentecote", "2025-06-09", 0, 0),       // Lundi de Pentecôte
        holiday("fete_nationale", "2025-07-14", 0, 0),
        holiday("assomption", "2025-08-15", 0, 0),
        holiday("toussaint", "2025-11-01", 0, 0),
        holiday("armistice", "2025-11-11", 0, 0),
        holiday("noel", "2025-12-25", 0, 0),
        // Vacances d'été CPAM (effet étalé)
        holiday("vacances_ete", "2025-08-01", -7, 7),   // Fenêtre 3 semaines
        // 2026
        holiday("jour_an", "2026-01-01", 0, 0),
        holiday("paques", "2026-04-06", -1, 1),
        holiday("fete_travail", "2026-05-01", 0, 0),
        holiday("victoire_1945", "2026-05-08", 0, 0),
        holiday("ascension", "2026-05-14", -1, 1),
        holiday("pentecote", "2026-05-25", 0, 0),
        holiday("fete_nationale", "2026-07-14", 0, 0),
        holiday("assomption", "2026-08-15", 0, 0),
        holiday("toussaint", "2026-11-01", 0, 0),
        holiday("armistice", "2026-11-11", 0, 0),
        holiday("noel", "2026-12-25", 0, 0),
        holiday("vacances_ete", "2026-08-01", -7, 7),
    ]
}

fn holiday(name: &str, date: &str, lower_window: i32, upper_window: i32) -> Holiday {
    Holiday::new(name.into())
        .with_dates(vec![date.into()])
        .with_lower_window(lower_window)
        .with_upper_window(upper_window)
}
```

### 4.5 Patterns de saisonnalité ITSM spécifiques à la CPAM

Les patterns de saisonnalité à modéliser pour un helpdesk CPAM incluent :

|Pattern|Période|Amplitude typique|Cause|
|---|---|---|---|
|**Pic du lundi**|Hebdomadaire (7j)|+20–40% vs mid-semaine|Accumulation week-end, démarrage de semaine|
|**Creux week-end**|Hebdomadaire (7j)|~0 tickets|Fermeture des bureaux CPAM|
|**Pic début de mois**|Mensuel (~30j)|+10–20%|Cycles de traitement des prestations|
|**Creux août**|Annuel|−30–50%|Congés été massifs secteur public|
|**Rentrée septembre**|Annuel|+20–30%|Retours de congés, nouveaux matériels|
|**Ponts**|Sporadique|−50–80%|Ascension, Toussaint, 11 novembre|
|**RTT**|Diffus|−5–10%|Réduction temps de travail 35h, imprévisible|

### 4.6 Construction de la série temporelle depuis SQLite

```sql
-- Comptage journalier de tickets créés (pour la prédiction de charge entrante)
SELECT
    DATE(date_ouverture) AS jour,
    COUNT(*) AS nb_tickets
FROM tickets
WHERE import_id = ?1
GROUP BY DATE(date_ouverture)
ORDER BY jour;

-- Comptage journalier de tickets résolus (pour la prédiction de capacité de sortie)
SELECT
    DATE(date_cloture_approx) AS jour,
    COUNT(*) AS nb_resolus
FROM tickets
WHERE import_id = ?1 AND est_vivant = 0 AND date_cloture_approx IS NOT NULL
GROUP BY DATE(date_cloture_approx)
ORDER BY jour;
```

**Important** : les jours sans tickets (week-ends, fériés) doivent être insérés avec un comptage de 0 pour que la série temporelle soit complète. Les trous dans la série perturbent la détection de saisonnalité.

```rust
use chrono::NaiveDate;
use std::collections::HashMap;

/// Complète une série temporelle journalière en insérant les jours manquants à 0.
///
/// Essentiel pour les modèles de saisonnalité qui supposent un pas de temps régulier.
pub fn fill_missing_days(
    counts: &HashMap<NaiveDate, f64>,
    start: NaiveDate,
    end: NaiveDate,
) -> Vec<(NaiveDate, f64)> {
    let mut result = Vec::new();
    let mut current = start;
    while current <= end {
        let count = counts.get(&current).copied().unwrap_or(0.0);
        result.push((current, count));
        current = current.succ_opt().unwrap_or(current);
    }
    result
}
```

### 4.7 Granularité des données et exigences minimales

Avec ~10 000 tickets sur une période estimée de 1 à 3 ans, l'agrégation **journalière** produit ~10–30 tickets par jour — suffisant pour la détection de saisonnalité hebdomadaire (période=7) et mensuelle (période≈30).

|Exigence|Minimum|Recommandé|
|---|---|---|
|Saisonnalité hebdomadaire|14 jours (2 cycles)|60+ jours|
|Saisonnalité mensuelle|60 jours (2 cycles)|180+ jours|
|Patterns annuels|365 jours|730+ jours (2 ans)|
|Prophet (robuste)|90 jours|365+ jours|

Si l'historique est trop court pour les patterns annuels, se limiter à MSTL + AutoETS avec saisonnalité hebdomadaire uniquement.

---

## 5. Embeddings français sur desktop : faisabilité et recommandations

### 5.1 Candle v0.9.2 : framework ML Rust de Hugging Face

**candle-core v0.9.2** (publié le 24 janvier 2026) est activement maintenu par Hugging Face avec **19 485+ étoiles GitHub** et des releases régulières. Il supporte les modèles d'architecture BERT via `candle-transformers`, et puisque **CamemBERT est architecturalement identique à RoBERTa** (un variant BERT), les modèles sentence-camembert se chargent via l'implémentation `BertModel` de candle. L'inférence CPU est le backend par défaut, avec accélération optionnelle MKL. Le crate `tokenizers` gère le tokenizer SentencePiece de CamemBERT nativement.

### 5.2 Comparaison des modèles d'embedding français

|Modèle|Paramètres|Dim. embedding|Taille disque|Score STS-FR|Temps 10K (ONNX)|
|---|---|---|---|---|---|
|paraphrase-multilingual-MiniLM-L12-v2|118M|384|~440 MB|Bon (multilingue)|**5–15s** ✅|
|sentence-camembert-base (Lajavaness)|110M|768|~440 MB|82.4 Pearson|**8–25s** ✅|
|sentence-camembert-large (dangvantuan)|336M|1024|~1.3 GB|85.9 Pearson|**25–60s** ⚠️|

Pour des descriptions courtes de tickets ITSM, **paraphrase-multilingual-MiniLM-L12-v2** offre le meilleur compromis vitesse/qualité : des embeddings de 384 dimensions gardent le calcul en aval rapide, et son entraînement multilingue couvre adéquatement le français pour du vocabulaire IT. Si la précision française est critique, le **Lajavaness/sentence-camembert-base** (version AugSBERT améliorée) offre une meilleure capture sémantique à 768 dimensions. Le modèle large dépasse probablement le budget de 30 secondes sur du matériel de bureau CPAM standard.

### 5.3 Comparaison des moteurs d'inférence pour Tauri

|Moteur|Approche|Perf CPU|Taille binaire|Dépendances externes|Recommandation|
|---|---|---|---|---|---|
|**ort v2.0.0-rc.11**|Binding ONNX Runtime 1.23|⭐⭐⭐ (3-5ms/phrase)|+20-40 MB|DLL par plateforme|**Meilleure perf, plus complexe**|
|**candle v0.9.2**|Tenseurs Rust pur, safetensors|⭐⭐ (6-10ms/phrase)|Minimal|Aucune|**Plus simple, recommandé pour Tauri**|
|**tract v0.22.0** (Sonos)|Inférence ONNX Rust pur|⭐⭐ (5-8ms/phrase)|Modéré|Aucune|**Bon compromis**|
|**fastembed-rs v5**|API haut niveau sur ort|⭐⭐⭐ (3-5ms/phrase)|+20-40 MB|DLL ort|**Clé en main, hérite des contraintes ort**|

**Recommandation pour Tauri CPAM** : **candle** pour la phase initiale (zéro dépendance externe, simplicité de déploiement), puis migration vers **ort** si les performances deviennent un goulot d'étranglement. Le gain ort (1.5–2× plus rapide) ne justifie la complexité de bundling de la DLL que si l'embedding est appelé fréquemment.

### 5.4 Pipeline d'embedding complet

```rust
// NOTE : Ce code illustre l'architecture conceptuelle.
// L'implémentation réelle dépendra du moteur choisi (candle/ort/tract).

/// Pipeline d'embedding pour tickets ITSM français.
///
/// Flux : tokenisation → inférence modèle → mean pooling → normalisation L2.
/// Pour 10 000 tickets courts (~50 tokens chacun) avec MiniLM-L12 :
///   - candle CPU : ~15-25 secondes
///   - ort CPU : ~5-15 secondes
pub struct EmbeddingPipeline {
    // tokenizer: tokenizers::Tokenizer,
    // model: BertModel / ort::Session / tract::SimplePlan,
    embedding_dim: usize,
}

impl EmbeddingPipeline {
    /// Embed un batch de textes et retourne une matrice [n, embedding_dim].
    /// Le batching (32-64 textes par batch) est critique pour la performance.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Array2<f32>, String> {
        // 1. Tokenisation : texts → token_ids + attention_mask
        //    (batch padding au max_length du batch)
        // 2. Inférence : token_ids → hidden_states [batch, seq_len, hidden_dim]
        // 3. Mean pooling : moyenne pondérée par attention_mask → [batch, hidden_dim]
        // 4. Normalisation L2 : chaque vecteur normalisé → similarité cosinus = dot product
        todo!("Implémentation spécifique au moteur choisi")
    }

    /// Similarité cosinus entre deux vecteurs d'embedding.
    /// Après normalisation L2, cos(a,b) = a·b (dot product simple).
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }
}
```

### 5.5 Verdict : faisabilité desktop

**Oui, l'embedding de 10 000 tickets ITSM français est réalisable sur desktop en moins de 30 secondes** avec le modèle MiniLM-L12 via ort, et en 15-25 secondes avec candle. Le modèle sentence-camembert-base reste dans le budget avec ort. Le modèle large (1.3 GB, 25-60 secondes) est à la limite et ne vaut le coût supplémentaire que si la qualité sémantique est insuffisante avec les modèles plus petits.

**Cependant**, pour la Phase 1 du GLPI Dashboard, les embeddings sont un **luxe fonctionnel, pas une nécessité**. Le pipeline TF-IDF du Segment 5 couvre 80% des besoins d'analyse textuelle. Les embeddings deviennent pertinents pour la classification zero-shot (§6.2), la détection de doublons sémantiques (Segment 8), et le clustering sémantique fin — des fonctionnalités de Phase 2.

---

## 6. Classification automatique de tickets ITSM : état de l'art

### 6.1 Panorama des approches (littérature 2023-2026)

La littérature positionne de façon consistante **TF-IDF + SVM à noyau linéaire** comme le meilleur baseline traditionnel pour la classification ITSM, atteignant **80–92% macro F1** sur des jeux de données avec 10–30 catégories bien définies. Une étude Springer 2024 rapporte 93,27% d'accuracy avec TF-IDF + deep learning (LSTM), tandis qu'une thèse RIT 2024 trouve SVM à 90% et XGBoost à 95% sur des enregistrements ServiceNow. Le module Predictive Intelligence de ServiceNow utilise TF-IDF en interne et recommande une précision >70% avec une couverture >80% comme seuil où le ML surpasse le routage humain.

**Caveat critique** de Zangari et al. (2023) : même le deep learning avancé ne dépasse **que difficilement 50% de F1** sur des données ITSM réelles avec du jargon spécialisé, des labels ambigus, et une structure incohérente. L'écart entre les benchmarks académiques propres et les données de production désordonnées est considérable. **La qualité des données et la cohérence des labels comptent infiniment plus que la sophistication du modèle.**

### 6.2 Stratégie progressive de classification

L'approche optimale pour une application desktop CPAM est une stratégie progressive en trois phases :

**Phase 1 — Zéro donnée labellisée** : Utiliser les embeddings pré-calculés avec **classification zero-shot** via similarité cosinus vers des vecteurs prototypes de catégorie (dérivés des noms et descriptions de catégories). Précision attendue : **55–70%** — utile pour le bootstrapping et la suggestion de catégories aux utilisateurs. Les embeddings multilingual MiniLM ou sentence-camembert-base fonctionnent ici.

**Phase 2 — Après 200–500 tickets labellisés** : Basculer vers **TF-IDF + SVM** via `linfa-svm`. Cela délivre **75–85% macro F1** avec une inférence sub-milliseconde, une taille de modèle minuscule, et une exécution entièrement Rust native. La matrice TF-IDF sprs existante alimente directement ce pipeline. Compléter avec la similarité d'embeddings comme vérification de confiance.

**Phase 3 — Après 1 000+ tickets labellisés** (optionnel) : Fine-tuner **CamemBERT-base** ou DistilCamemBERT en Python, exporter en ONNX via Optimum, et déployer via ort en Rust pour **80–92% macro F1** — un gain de +5–8% par rapport à TF-IDF + SVM au prix de 50–200 ms d'inférence et ~250 MB de taille modèle.

### 6.3 TF-IDF + SVM avec linfa : implémentation Phase 2

```rust
use linfa::prelude::*;
use linfa_svm::Svm;
use ndarray::Array2;

/// Entraîne un classifieur SVM linéaire pour les tickets ITSM.
///
/// Entrée : matrice TF-IDF dense (après SVD optionnelle) + labels de catégorie.
/// Le SVM linéaire est optimal pour les données textuelles à haute dimensionnalité :
///   - O(n × d) en entraînement (vs O(n² × d) pour noyau RBF)
///   - Inférence < 1ms par ticket
///   - Régularisation C contrôle le surapprentissage
///
/// Avec 500 tickets labellisés et ~10 catégories, l'entraînement prend ~100-500ms.
pub fn train_svm_classifier(
    features: &Array2<f32>,
    labels: &[usize],           // 0..n_categories
    category_names: &[String],
) -> Result<SvmClassifier, String> {
    // One-vs-Rest pour la classification multi-classes
    // linfa-svm utilise Platt scaling pour les probabilités
    let dataset = Dataset::new(features.clone(), labels.to_vec().into());

    let model = Svm::<f32, usize>::params()
        .linear_kernel()
        .fit(&dataset)
        .map_err(|e| format!("Erreur SVM: {e}"))?;

    Ok(SvmClassifier {
        model,
        category_names: category_names.to_vec(),
    })
}

pub struct SvmClassifier {
    model: linfa_svm::FittedSvm<f32, usize>,
    category_names: Vec<String>,
}

impl SvmClassifier {
    /// Prédit la catégorie d'un ticket à partir de son vecteur TF-IDF.
    /// Retourne le nom de la catégorie et le score de confiance.
    pub fn predict(&self, features: &Array2<f32>) -> Vec<PredictedCategory> {
        let predictions = self.model.predict(features);
        predictions.iter()
            .map(|&label| PredictedCategory {
                category: self.category_names.get(label)
                    .cloned()
                    .unwrap_or_else(|| format!("Catégorie {label}")),
                label,
                confidence: 0.0, // Score brut SVM si disponible
            })
            .collect()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PredictedCategory {
    pub category: String,
    pub label: usize,
    pub confidence: f64,
}
```

### 6.4 Apprentissage semi-supervisé et actif

Quand seule une fraction des tickets porte des labels (courant en ITSM), deux techniques réduisent l'effort de labellisation :

**Self-training** : entraîner TF-IDF + SVM sur les données labellisées, prédire les tickets non labellisés avec haute confiance (seuil > 0.9), ajouter ceux-ci aux données d'entraînement, et ré-entraîner. Itérer 3–5 fois. Gain typique : +5–10% de F1 par rapport à l'entraînement supervisé seul.

**Active learning** (échantillonnage par incertitude) : identifier les tickets où le SVM est le moins sûr (proches de la frontière de décision), les présenter à l'analyste CPAM pour labellisation manuelle, et ré-entraîner. 200–500 tickets stratégiquement sélectionnés peuvent égaler la performance de 2 000+ tickets aléatoirement labellisés.

### 6.5 TF-IDF + SVM suffit-il ?

**Oui, pour la grande majorité des usages pratiques.** Le gain marginal en F1 des transformers (+5–8%) coûte 100× en temps d'inférence et en complexité. Pour une application desktop Tauri ciblant les analystes CPAM, TF-IDF + SVM fournit une inférence sub-milliseconde, un modèle de quelques Mo, et une exécution Rust native complète sans dépendance externe. Le goulot d'étranglement principal en classification ITSM est la cohérence des labels et la qualité des données, pas l'architecture du modèle. Réserver la classification par transformers aux cas où les descriptions courtes et ambiguës de tickets bénéficient véritablement de la compréhension sémantique plutôt que du matching par mots-clés.

---

## 7. Architecture du module `analytics/`

### 7.1 Structure de fichiers

```
src-tauri/src/analytics/
├── mod.rs                 // Exports publics du module
├── clustering.rs          // K-Means, HDBSCAN, labels, elbow
├── dimensionality.rs      // PCA/SVD tronquée via linfa-reduction
├── anomalies.rs           // Z-score, IQR, centroid distance, Isolation Forest
├── forecasting.rs         // MSTL + AutoETS, Prophet, saisonnalité
├── classification.rs      // SVM, zero-shot (futur), active learning (futur)
└── types.rs               // Structs partagées (résultats, configs)
```

### 7.2 Commandes Tauri

```rust
// src-tauri/src/commands.rs — ajouts Segment 6

/// Lance l'analyse elbow pour déterminer le k optimal.
/// Retourne la courbe d'inertie et le k recommandé.
#[tauri::command]
async fn analyze_elbow(
    state: State<'_, AppState>,
    import_id: i64,
    k_min: usize,
    k_max: usize,
    compute_silhouette: bool,
    channel: tauri::ipc::Channel<ImportProgress>,
) -> Result<ElbowAnalysisResult, String> {
    // 1. Charger les vecteurs TF-IDF depuis le cache (Segment 5)
    // 2. Réduire les dimensions via SVD tronquée
    // 3. Exécuter K-Means pour chaque k
    // 4. Détecter le coude via Kneedle
    todo!()
}

/// Exécute le clustering K-Means avec le k choisi.
/// Retourne les clusters labellisés et enrichis de métriques métier.
#[tauri::command]
async fn cluster_tickets(
    state: State<'_, AppState>,
    import_id: i64,
    n_clusters: usize,
) -> Result<ClusterResult, String> {
    todo!()
}

/// Lance la détection d'anomalies multi-niveaux.
#[tauri::command]
async fn detect_anomalies(
    state: State<'_, AppState>,
    import_id: i64,
    config: AnomalyConfig,
) -> Result<AnomalyReport, String> {
    todo!()
}

/// Prédiction de charge pour les N prochains jours.
#[tauri::command]
async fn predict_workload(
    state: State<'_, AppState>,
    import_id: i64,
    horizon_days: usize,
    use_prophet: bool,
) -> Result<ForecastResult, String> {
    todo!()
}

/// Configuration de la détection d'anomalies.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnomalyConfig {
    pub z_threshold_warning: f64,    // Défaut: 2.0
    pub z_threshold_critical: f64,   // Défaut: 3.0
    pub centroid_percentile: f64,    // Défaut: 95.0
    pub isolation_threshold: f64,    // Défaut: 0.6
    pub use_modified_zscore: bool,   // Défaut: true
}
```

### 7.3 Cache des résultats analytiques

Les résultats de clustering et de détection d'anomalies sont coûteux à calculer (2-15 secondes) et stables entre deux imports. Le cache utilise la table `analytics_cache` définie au Segment 2 :

```rust
use rusqlite::Connection;
use serde_json;

/// Stocke un résultat analytique dans le cache SQLite.
pub fn cache_result<T: serde::Serialize>(
    conn: &Connection,
    import_id: i64,
    analysis_type: &str,     // "clustering", "anomalies", "forecast"
    parameters: &str,         // JSON des paramètres (k, seuils, horizon)
    result: &T,
    duration_ms: u64,
) -> Result<(), rusqlite::Error> {
    let result_json = serde_json::to_string(result)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    conn.execute(
        "INSERT OR REPLACE INTO analytics_cache 
         (import_id, analysis_type, parameters, result, duration_ms)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![import_id, analysis_type, parameters, result_json, duration_ms],
    )?;
    Ok(())
}

/// Récupère un résultat depuis le cache, si les paramètres correspondent.
pub fn get_cached_result<T: serde::de::DeserializeOwned>(
    conn: &Connection,
    import_id: i64,
    analysis_type: &str,
    parameters: &str,
) -> Result<Option<(T, u64)>, rusqlite::Error> {
    let mut stmt = conn.prepare_cached(
        "SELECT result, duration_ms FROM analytics_cache
         WHERE import_id = ?1 AND analysis_type = ?2 AND parameters = ?3"
    )?;

    let result = stmt.query_row(
        rusqlite::params![import_id, analysis_type, parameters],
        |row| {
            let json: String = row.get(0)?;
            let duration: u64 = row.get(1)?;
            Ok((json, duration))
        },
    );

    match result {
        Ok((json, duration)) => {
            let parsed: T = serde_json::from_str(&json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            Ok(Some((parsed, duration)))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}
```

---

## 8. Benchmarks et performance estimée

### 8.1 Temps d'exécution pour 10 000 tickets

|Opération|Temps estimé (single-thread)|Avec parallélisation|
|---|---|---|
|Conversion creuse → dense (f32)|~50 ms|—|
|SVD tronquée (5000 → 200 dims)|~200-500 ms|~100-200 ms (BLAS)|
|K-Means (k=8, 200 dims, 10 runs)|~300-800 ms|~100-300 ms (rayon)|
|Elbow k=3..20 (17 runs × 5 restarts)|~3-8 s|~1-3 s|
|Silhouette (2000 échantillons, 200 dims)|~500 ms|~200 ms|
|Z-score anomalies|< 1 ms|—|
|Distance centroïde (10K × 200)|~20-50 ms|—|
|Isolation Forest (150 arbres, 4 dims)|~200-500 ms|—|
|MSTL + AutoETS (365 jours)|~50-200 ms|—|
|Prophet + jours fériés (365 jours)|~2-10 s|—|
|**Pipeline complet (sans Prophet)**|**~5-12 s**|**~2-5 s**|

### 8.2 Empreinte mémoire

|Composant|Taille|
|---|---|
|Matrice TF-IDF dense f32 (10K × 5000)|~200 MB|
|Matrice réduite f32 (10K × 200)|~8 MB|
|Centroïdes (8 × 200 × f32)|~6 KB|
|Predictions (10K × usize)|~80 KB|
|Features Isolation Forest (10K × 4 × f64)|~320 KB|
|Forest (150 arbres)|~5-10 MB|
|Série temporelle (365 × f64)|~3 KB|
|**Total (avec SVD)**|**~220 MB peak, ~20 MB steady**|

Le pic mémoire de ~220 MB survient uniquement pendant la conversion dense + SVD. Après réduction, la matrice originale peut être libérée, ramenant l'empreinte stable à ~20 MB. Pour un poste CPAM avec 8 Go de RAM, c'est confortable.

---

## 9. Cargo.toml consolidé pour le Segment 6

```toml
[dependencies]
# --- Clustering & ML (Segment 6) ---
linfa = "0.8"
linfa-clustering = "0.8"                # K-Means, DBSCAN
linfa-reduction = "0.8"                 # PCA, SVD tronquée
linfa-svm = "0.8"                       # SVM linéaire (classification Phase 2)
linfa-nn = "0.8"                        # KNN (active learning)
ndarray = { version = "0.16", features = ["serde"] }

# --- Clustering alternatif ---
petal-clustering = "0.13"               # HDBSCAN (découverte exploratoire)

# --- Anomalies ---
extended-isolation-forest = "0.2"       # EIF multi-dimensionnel
statrs = "0.17"                         # Statistiques (distributions, tests)

# --- Séries temporelles ---
augurs = { version = "0.10", features = ["mstl", "ets", "seasons", "forecaster"] }
# augurs = { version = "0.10", features = ["prophet-wasmstan"] }  # Activer pour Prophet

# --- Elbow detection ---
kneed = "1.0"                           # Algorithme Kneedle

# --- Algèbre linéaire ---
nalgebra = "0.34"                       # Matrices, Mahalanobis

# --- Embeddings (Phase 2, optionnel) ---
# candle-core = { version = "0.9", features = [] }
# candle-nn = "0.9"
# candle-transformers = "0.9"
# tokenizers = "0.20"
# hf-hub = "0.3"

# --- Déjà présent (Segments 1-5) ---
sprs = "0.11"
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rand = "0.8"
log = "0.4"
```

---

## 10. Récapitulatif des décisions d'architecture

|Décision|Choix|Justification|
|---|---|---|
|Moteur de clustering|linfa-clustering K-Means|4 500+ étoiles, API stable, inertie intégrée|
|Initialisation K-Means|K-Means++ avec 10 runs|Convergence robuste pour données textuelles|
|Réduction dimensionnalité|SVD tronquée 5000 → 200|÷25 mémoire, +5× vitesse, meilleurs clusters|
|Type numérique|f32 au lieu de f64|÷2 mémoire, qualité identique pour TF-IDF|
|Détection du coude|Kneedle (kneed) + fallback custom|Automatique, reproductible|
|Silhouette score|Échantillonnage 2 000 points|O(n²) impraticable sur 10K complet|
|Anomalies niveau 1|Z-score modifié sur log-délais|Robuste aux outliers extrêmes CPAM|
|Anomalies niveau 2|Top 5% distance centroïde|Capture les tickets mal classés sémantiquement|
|Anomalies niveau 3|Extended Isolation Forest 4D|Anomalies multi-dimensionnelles composites|
|Prédiction principale|MSTL + AutoETS (augurs)|Saisonnalité multiple automatique, intervalles|
|Prédiction avancée|Prophet Rust (augurs-prophet)|Jours fériés français, ponts, août CPAM|
|Classification Phase 1|TF-IDF + SVM linéaire (linfa-svm)|80-85% F1, sub-ms, zéro dépendance|
|Embeddings|candle (Phase 2 optionnelle)|Pure Rust, déploiement simple Tauri|
|Cache analytique|SQLite analytics_cache (Segment 2)|Évite le recalcul entre les sessions|
|Exploration clusters|HDBSCAN (petal-clustering)|Découverte du nombre naturel de groupes|

---

_Ce segment fournit l'intégralité de la logique de clustering, détection d'anomalies et prédiction de charge pour le GLPI Dashboard. Il consomme la matrice TF-IDF du Segment 5, les métriques temporelles du Segment 3, et les catégories du Segment 4. Le schéma de cache du Segment 2 stocke les résultats coûteux. Le Segment 7 (Frontend React) fournira les visualisations correspondantes : scatter plot des clusters, heatmap des anomalies, courbes de prédiction avec intervalles de confiance._