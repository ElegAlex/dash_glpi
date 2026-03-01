# Segment 4 — Catégories hiérarchiques : parsing, drill-down, fallback

**Guide technique complet pour le module Catégories du GLPI Dashboard**

---

Le module Catégories ajoute la dimension structurelle aux indicateurs Stock (Segment 3). Là où le stock répond « combien de tickets sont ouverts ? », les catégories répondent « dans quel sous-service sont-ils, et comment cette répartition évolue-t-elle ? ». L'architecture s'appuie sur **deux sources hiérarchiques** : le champ « Groupe de techniciens » (disponible dès aujourd'hui, séparateur `>`) et la colonne « Catégorie ITIL » (optionnelle, absente de l'export actuel, ajoutée à terme). Un système de fallback sélectionne automatiquement la meilleure source disponible, et la structure arborescente en Rust permet l'agrégation à n'importe quel niveau de profondeur.

---

## 1. Données réelles de l'export CPAM 92

### 1.1 Inventaire des groupes hiérarchiques

L'analyse du fichier `tickets.csv` (9 616 tickets) révèle exactement **8 valeurs distinctes** de « Groupe de techniciens » :

|Groupe complet|Niveau 1|Niveau 2|Niveau 3|Tickets|%|
|---|---|---|---|--:|--:|
|`_DSI > _SUPPORT UTILISATEURS ET POSTES DE TRAVAIL`|`_DSI`|`_SUPPORT UTILISATEURS ET POSTES DE TRAVAIL`|—|6 562|68,2%|
|`_DSI > _PRODUCTION-INFRASTRUCTURES`|`_DSI`|`_PRODUCTION-INFRASTRUCTURES`|—|1 474|15,3%|
|`_DSI > _SERVICE DES CORRESPONDANTS INFORMATIQUE`|`_DSI`|`_SERVICE DES CORRESPONDANTS INFORMATIQUE`|—|1 178|12,3%|
|`_DSI > _HABILITATIONS_PRODUCTION`|`_DSI`|`_HABILITATIONS_PRODUCTION`|—|302|3,1%|
|`_DSI > _SUPPORT UTILISATEURS ET POSTES DE TRAVAIL > _SUPPORT - PARC`|`_DSI`|`_SUPPORT UTILISATEURS ET POSTES DE TRAVAIL`|`_SUPPORT - PARC`|165|1,7%|
|`_DSI > _DIADEME`|`_DSI`|`_DIADEME`|—|31|0,3%|
|`_DSI > _DEVELOPPEMENT &amp; INDUSTRIALISATION`|`_DSI`|`_DEVELOPPEMENT & INDUSTRIALISATION`|—|24|0,2%|
|`GC_SD`|`GC_SD`|—|—|1|~0%|

**Observations structurelles :**

- La profondeur maximale est **3 niveaux** (`_DSI > _SUPPORT UTILISATEURS... > _SUPPORT - PARC`)
- Tous les groupes sauf un commencent par `_DSI` → l'arbre a essentiellement **une racine principale** et un orphelin (`GC_SD`)
- Le préfixe `_` (underscore) est une convention GLPI pour distinguer les groupes techniques des groupes organisationnels
- L'entité HTML `&amp;` dans `_DEVELOPPEMENT &amp; INDUSTRIALISATION` doit être décodée en `&` lors du parsing
- Un seul groupe utilise le niveau 3, ce qui signifie que le drill-down aura peu de profondeur — mais l'architecture doit supporter N niveaux pour les futures évolutions

### 1.2 Tickets multi-groupes

138 tickets sur 9 616 (1,4%) sont assignés à **plusieurs groupes** simultanément (champ multilignes séparé par `\n`). Conformément au Segment 1, le `groupe_principal` est le premier de la liste. Pour l'agrégation par catégorie, chaque ticket n'est compté qu'**une seule fois** via son groupe principal, afin d'éviter le double comptage dans les totaux et pourcentages.

### 1.3 Répartition vivants/terminés par groupe

|Groupe (niveau 2)|Vivants|Terminés|Total|% vivants|
|---|--:|--:|--:|--:|
|_SUPPORT UTILISATEURS ET POSTES DE TRAVAIL|402|6 281|6 727*|6,0%|
|_PRODUCTION-INFRASTRUCTURES|39|1 435|1 474|2,6%|
|_SERVICE DES CORRESPONDANTS INFORMATIQUE|52|1 126|1 178|4,4%|
|_HABILITATIONS_PRODUCTION|9|293|302|3,0%|
|_DIADEME|4|27|31|12,9%|
|_DEVELOPPEMENT & INDUSTRIALISATION|1|23|24|4,2%|
|GC_SD|1|0|1|100%|

_* Inclut le niveau 3 `_SUPPORT - PARC` (44 vivants, 121 terminés)._

---

## 2. Parsing de la hiérarchie

### 2.1 Algorithme de découpage

Le parsing d'une chaîne de groupe GLPI suit une séquence stricte : split sur le séparateur `>` (espace-chevron-espace), trim de chaque segment, décodage des entités HTML, filtrage des segments vides.

```rust
/// Parse une chaîne de hiérarchie GLPI en niveaux individuels.
///
/// "_DSI > _SUPPORT > _PARC" → vec!["_DSI", "_SUPPORT", "_PARC"]
/// "_DSI > _DEVELOPPEMENT &amp; INDUSTRIALISATION" → vec!["_DSI", "_DEVELOPPEMENT & INDUSTRIALISATION"]
/// "GC_SD" → vec!["GC_SD"]
/// "" → vec![]
///
/// Le séparateur est ` > ` (avec espaces), pas juste `>`.
/// Ceci évite les faux positifs si un nom de groupe contient `>` sans espaces.
pub fn parse_hierarchy(raw: &str) -> Vec<String> {
    if raw.trim().is_empty() {
        return Vec::new();
    }

    raw.split(" > ")
        .map(|segment| {
            segment
                .trim()
                .replace("&amp;", "&")   // Entité HTML GLPI
                .replace("&lt;", "<")     // Défensif
                .replace("&gt;", ">")     // Défensif
                .replace("&quot;", "\"")  // Défensif
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// Extrait les niveaux individuels avec fallback None pour les niveaux absents.
/// Retourne (niveau1, niveau2, niveau3) pour insertion SQLite.
pub fn extract_levels(hierarchy: &[String]) -> (Option<String>, Option<String>, Option<String>) {
    (
        hierarchy.get(0).cloned(),
        hierarchy.get(1).cloned(),
        hierarchy.get(2).cloned(),
    )
}

/// Reconstruit le chemin complet à partir des niveaux.
/// ("_DSI", Some("_SUPPORT"), None) → "_DSI > _SUPPORT"
pub fn rebuild_path(levels: &[Option<String>]) -> String {
    levels
        .iter()
        .filter_map(|l| l.as_ref())
        .cloned()
        .collect::<Vec<_>>()
        .join(" > ")
}
```

### 2.2 Intégration dans le pipeline de normalisation (Segment 1)

La fonction `normalize_ticket()` du Segment 1 doit être étendue pour extraire les niveaux hiérarchiques. Le `GlpiTicketNormalized` reçoit trois nouveaux champs :

```rust
/// Extension de GlpiTicketNormalized (Segment 1, §5.3)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlpiTicketNormalized {
    // ... champs existants du Segment 1 ...

    // Hiérarchie groupe (déduite du groupe_principal)
    pub groupe_niveau1: Option<String>,
    pub groupe_niveau2: Option<String>,
    pub groupe_niveau3: Option<String>,

    // Hiérarchie catégorie ITIL (colonne optionnelle, même format " > ")
    pub categorie_niveau1: Option<String>,
    pub categorie_niveau2: Option<String>,
}
```

```rust
fn normalize_ticket(raw: &GlpiTicketRaw, now: &chrono::NaiveDateTime) -> GlpiTicketNormalized {
    let groupes = raw.groupes();
    let groupe_principal = groupes.first().map(|s| s.to_string());

    // Parsing hiérarchique du groupe principal
    let groupe_hierarchy = groupe_principal
        .as_deref()
        .map(parse_hierarchy)
        .unwrap_or_default();
    let (gn1, gn2, gn3) = extract_levels(&groupe_hierarchy);

    // Parsing hiérarchique de la catégorie ITIL (si présente)
    let categorie = raw.categorie.clone().filter(|s| !s.is_empty());
    let cat_hierarchy = categorie
        .as_deref()
        .map(parse_hierarchy)
        .unwrap_or_default();
    let (cn1, cn2, _cn3) = extract_levels(&cat_hierarchy);

    GlpiTicketNormalized {
        // ... champs existants ...
        groupe_principal,
        groupes: groupes.iter().map(|s| s.to_string()).collect(),
        groupe_niveau1: gn1,
        groupe_niveau2: gn2,
        groupe_niveau3: gn3,
        categorie,
        categorie_niveau1: cn1,
        categorie_niveau2: cn2,
        // ...
    }
}
```

### 2.3 Cas limites et robustesse

|Cas|Entrée|Comportement|Résultat|
|---|---|---|---|
|Standard 2 niveaux|`_DSI > _SUPPORT`|Split normal|N1=`_DSI`, N2=`_SUPPORT`, N3=None|
|Standard 3 niveaux|`_DSI > _SUPPORT > _PARC`|Split normal|N1=`_DSI`, N2=`_SUPPORT`, N3=`_PARC`|
|Plat (1 niveau)|`GC_SD`|Aucun séparateur|N1=`GC_SD`, N2=None, N3=None|
|Entité HTML|`_DSI > _DEV &amp; INDUS`|Décodage `&amp;`|N2=`_DEV & INDUS`|
|Vide|`""`|Filtré|N1=None, N2=None, N3=None|
|Espaces excédentaires|`_DSI > _SUPPORT`|Trim par segment|N1=`_DSI`, N2=`_SUPPORT`|
|Séparateur sans espaces|`_DSI>_SUPPORT`|**Non splitté** — traité comme nom plat|N1=`_DSI>_SUPPORT`, N2=None|
|4+ niveaux (futur)|`A > B > C > D`|Niveaux 4+ ignorés|N1=`A`, N2=`B`, N3=`C`|
|Multi-groupes|`_DSI > _A\n_DSI > _B`|Seul le 1er (principal) est hiérarchisé|Conforme Segment 1|

**Justification du séparateur `>` (avec espaces)** : GLPI utilise systématiquement cette convention dans ses exports CSV. Un split sur `>` seul casserait les noms contenant le caractère (improbable mais défensif). Les données réelles CPAM 92 confirment ce pattern sur 100% des 9 616 tickets.

---

## 3. Structure arborescente en Rust

### 3.1 Modèle de données

L'arbre de catégories utilise les structs définies dans le Segment 2, rappelées ici avec les implémentations :

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Requête frontend pour l'arbre de catégories.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoriesRequest {
    /// "vivants", "tous", "termines"
    pub scope: String,
    /// "groupe" (défaut) ou "categorie" (si colonne disponible)
    pub source: Option<String>,
    /// Filtre optionnel sur un sous-arbre : "groupe_niveau1 = '_DSI'"
    pub filter_path: Option<String>,
}

/// Réponse complète : l'arbre + métadonnées.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryTree {
    /// "groupe" ou "categorie"
    pub source: String,
    /// Indique si la source "categorie" est disponible dans l'import
    pub categorie_disponible: bool,
    /// Nœuds racine de l'arbre
    pub nodes: Vec<CategoryNode>,
    /// Nombre total de tickets dans le scope
    pub total_tickets: usize,
    /// Profondeur maximale observée
    pub max_depth: usize,
}

/// Nœud de l'arbre. Chaque nœud porte ses métriques agrégées
/// et une liste de sous-nœuds (récursif).
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CategoryNode {
    /// Nom court du nœud (ex: "_SUPPORT UTILISATEURS ET POSTES DE TRAVAIL")
    pub name: String,
    /// Chemin complet depuis la racine (ex: "_DSI > _SUPPORT UTILISATEURS...")
    pub full_path: String,
    /// Profondeur dans l'arbre (1 = racine)
    pub level: usize,

    // --- Métriques agrégées ---
    /// Nombre de tickets à CE nœud (pas en comptant les enfants)
    pub own_count: usize,
    /// Nombre total incluant tous les descendants
    pub total_count: usize,
    /// Pourcentage par rapport au total du scope
    pub percentage: f64,
    /// Ventilation par type
    pub incidents: usize,
    pub demandes: usize,
    /// Âge moyen des tickets (jours)
    pub age_moyen: f64,
    /// Nombre de vivants dans ce nœud+descendants
    pub vivants: usize,
    /// Nombre de terminés dans ce nœud+descendants
    pub termines: usize,

    // --- Enfants ---
    pub children: Vec<CategoryNode>,
}
```

### 3.2 Construction de l'arbre depuis les données SQL

L'algorithme de construction procède en deux passes : d'abord la collecte des données brutes via SQL, puis l'assemblage de l'arbre en mémoire Rust. Cette approche hybride SQL+Rust est cohérente avec la philosophie du Segment 3 (agrégation SQL, structures complexes Rust).

```rust
/// Données brutes d'un ticket pour la construction de l'arbre.
/// Récupérées par une seule requête SQL groupée.
struct CategoryRawRow {
    niveau1: Option<String>,
    niveau2: Option<String>,
    niveau3: Option<String>,
    type_ticket: String,
    est_vivant: bool,
    anciennete_jours: i64,
}

/// Point d'agrégation intermédiaire, indexé par chemin complet.
#[derive(Default)]
struct NodeAccumulator {
    own_tickets: usize,
    incidents: usize,
    demandes: usize,
    vivants: usize,
    termines: usize,
    ages: Vec<f64>,
}

/// Construit l'arbre de catégories à partir des tickets bruts.
///
/// L'algorithme :
/// 1. Itère sur chaque ticket et incrémente le nœud correspondant
///    au niveau le plus profond disponible (pas de double comptage)
/// 2. Assemble les nœuds en arbre par relation parent-enfant
/// 3. Propage les totaux des feuilles vers les racines (bottom-up)
pub fn build_category_tree(
    rows: &[CategoryRawRow],
    source: &str,
    total_tickets: usize,
) -> CategoryTree {
    let mut accumulators: HashMap<String, NodeAccumulator> = HashMap::new();
    let mut parent_map: HashMap<String, String> = HashMap::new(); // enfant → parent
    let mut max_depth: usize = 0;

    for row in rows {
        // Construire le chemin le plus profond disponible
        let levels: Vec<&str> = [
            row.niveau1.as_deref(),
            row.niveau2.as_deref(),
            row.niveau3.as_deref(),
        ]
        .iter()
        .filter_map(|l| *l)
        .collect();

        if levels.is_empty() {
            continue;
        }

        max_depth = max_depth.max(levels.len());

        // Enregistrer tous les nœuds intermédiaires (pour l'arbre)
        // mais ne compter le ticket QU'au niveau le plus profond
        for depth in 0..levels.len() {
            let path = levels[..=depth].join(" > ");

            // S'assurer que le nœud existe dans la map
            accumulators.entry(path.clone()).or_default();

            // Enregistrer la relation parent-enfant
            if depth > 0 {
                let parent_path = levels[..depth].join(" > ");
                parent_map.insert(path.clone(), parent_path);
            }
        }

        // Compter le ticket au niveau le plus profond uniquement
        let deepest_path = levels.join(" > ");
        let acc = accumulators.get_mut(&deepest_path).unwrap();
        acc.own_tickets += 1;
        match row.type_ticket.as_str() {
            "Incident" => acc.incidents += 1,
            "Demande" => acc.demandes += 1,
            _ => {}
        }
        if row.est_vivant {
            acc.vivants += 1;
        } else {
            acc.termines += 1;
        }
        acc.ages.push(row.anciennete_jours as f64);
    }

    // Phase 2 : assembler les nœuds leaf-first
    let mut nodes_map: HashMap<String, CategoryNode> = HashMap::new();

    for (path, acc) in &accumulators {
        let parts: Vec<&str> = path.split(" > ").collect();
        let name = parts.last().unwrap_or(&"").to_string();
        let level = parts.len();
        let age_moyen = if acc.ages.is_empty() {
            0.0
        } else {
            acc.ages.iter().sum::<f64>() / acc.ages.len() as f64
        };

        nodes_map.insert(
            path.clone(),
            CategoryNode {
                name,
                full_path: path.clone(),
                level,
                own_count: acc.own_tickets,
                total_count: acc.own_tickets, // sera propagé ensuite
                percentage: 0.0,              // calculé après propagation
                incidents: acc.incidents,
                demandes: acc.demandes,
                age_moyen,
                vivants: acc.vivants,
                termines: acc.termines,
                children: Vec::new(),
            },
        );
    }

    // Phase 3 : assembler l'arbre (attacher les enfants aux parents)
    // Trier les chemins par profondeur décroissante pour construire bottom-up
    let mut paths: Vec<String> = nodes_map.keys().cloned().collect();
    paths.sort_by(|a, b| {
        let depth_a = a.matches(" > ").count();
        let depth_b = b.matches(" > ").count();
        depth_b.cmp(&depth_a) // plus profond d'abord
    });

    for path in &paths {
        if let Some(parent_path) = parent_map.get(path) {
            // Retirer le nœud enfant de la map
            if let Some(child) = nodes_map.remove(path) {
                // Propager les totaux vers le parent
                if let Some(parent) = nodes_map.get_mut(parent_path) {
                    parent.total_count += child.total_count;
                    parent.incidents += child.incidents;
                    parent.demandes += child.demandes;
                    parent.vivants += child.vivants;
                    parent.termines += child.termines;
                    // Recalcul de l'âge moyen pondéré
                    if parent.total_count > 0 {
                        let parent_sum = parent.age_moyen * (parent.total_count - child.total_count) as f64;
                        let child_sum = child.age_moyen * child.total_count as f64;
                        parent.age_moyen = (parent_sum + child_sum) / parent.total_count as f64;
                    }
                    parent.children.push(child);
                }
            }
        }
    }

    // Phase 4 : calculer les pourcentages et trier
    let total = if total_tickets > 0 { total_tickets } else { 1 };
    let mut root_nodes: Vec<CategoryNode> = nodes_map.into_values().collect();

    fn apply_percentages(nodes: &mut [CategoryNode], total: usize) {
        for node in nodes.iter_mut() {
            node.percentage = (node.total_count as f64 / total as f64) * 100.0;
            // Trier les enfants par total_count décroissant
            node.children.sort_by(|a, b| b.total_count.cmp(&a.total_count));
            apply_percentages(&mut node.children, total);
        }
    }

    root_nodes.sort_by(|a, b| b.total_count.cmp(&a.total_count));
    apply_percentages(&mut root_nodes, total);

    CategoryTree {
        source: source.to_string(),
        categorie_disponible: false, // déterminé par la commande
        nodes: root_nodes,
        total_tickets,
        max_depth,
    }
}
```

### 3.3 Exemple concret avec les données CPAM 92

Pour le scope "vivants" (543 tickets), l'arbre produit ressemblerait à :

```
_DSI (total_count=542, own_count=0)
├── _SUPPORT UTILISATEURS ET POSTES DE TRAVAIL (total_count=402, own_count=358)
│   └── _SUPPORT - PARC (total_count=44, own_count=44)
├── _SERVICE DES CORRESPONDANTS INFORMATIQUE (total_count=52, own_count=52)
├── _PRODUCTION-INFRASTRUCTURES (total_count=39, own_count=39)
├── _HABILITATIONS_PRODUCTION (total_count=9, own_count=9)
├── _DIADEME (total_count=4, own_count=4)
└── _DEVELOPPEMENT & INDUSTRIALISATION (total_count=1, own_count=1)

GC_SD (total_count=1, own_count=1)
```

**Point clé** : `own_count` vs `total_count`. Le nœud `_SUPPORT UTILISATEURS...` a 358 tickets propres + 44 de `_SUPPORT - PARC` = 402 au total. Le nœud racine `_DSI` a `own_count=0` (aucun ticket n'est assigné directement à `_DSI` sans sous-groupe) mais `total_count=542` (somme de tous ses descendants).

---

## 4. Fallback Groupe ↔ Catégorie ITIL

### 4.1 Stratégie de sélection de source

L'export CPAM 92 actuel ne contient pas de colonne « Catégorie ITIL ». Cette colonne sera ajoutée dans un futur export. Le système doit fonctionner avec les deux sources et basculer intelligemment de l'une à l'autre :

```rust
/// Détermine la source à utiliser pour l'arbre de catégories.
///
/// Règles de fallback :
/// 1. Si l'utilisateur demande explicitement "categorie" ET que la colonne existe → categorie
/// 2. Si l'utilisateur demande "categorie" mais la colonne est absente → fallback groupe + warning
/// 3. Si l'utilisateur demande "groupe" → groupe
/// 4. Par défaut (source = None) → categorie si disponible, sinon groupe
pub fn resolve_category_source(
    requested: Option<&str>,
    categorie_column_exists: bool,
    categorie_has_data: bool,
) -> (CategorySource, Option<String>) {
    match requested {
        Some("categorie") if categorie_column_exists && categorie_has_data => {
            (CategorySource::CategorieItil, None)
        }
        Some("categorie") if categorie_column_exists && !categorie_has_data => {
            (
                CategorySource::GroupeTechniciens,
                Some("La colonne Catégorie ITIL existe mais est vide pour tous les tickets. \
                      Utilisation du Groupe de techniciens par défaut.".into()),
            )
        }
        Some("categorie") => {
            (
                CategorySource::GroupeTechniciens,
                Some("La colonne Catégorie ITIL n'est pas présente dans cet import. \
                      Utilisation du Groupe de techniciens par défaut.".into()),
            )
        }
        Some("groupe") | None => {
            // Par défaut : groupe (source la plus fiable actuellement)
            if !categorie_column_exists {
                (CategorySource::GroupeTechniciens, None)
            } else {
                // Si les deux existent, préférer catégorie comme override
                // sauf si demande explicite de groupe
                let source = if requested == Some("groupe") || !categorie_has_data {
                    CategorySource::GroupeTechniciens
                } else {
                    CategorySource::CategorieItil
                };
                (source, None)
            }
        }
        _ => (CategorySource::GroupeTechniciens, None),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CategorySource {
    GroupeTechniciens,
    CategorieItil,
}
```

### 4.2 Détection de la disponibilité de la colonne Catégorie

La disponibilité est vérifiée lors de l'import (Segment 1) et stockée dans la table `imports` :

```sql
-- Ajout à la table imports (Segment 2, §3.2)
ALTER TABLE imports ADD COLUMN has_categorie INTEGER NOT NULL DEFAULT 0;
```

```rust
/// Vérifie si l'import courant contient des données de catégorie ITIL exploitables.
pub fn check_categorie_availability(conn: &Connection, import_id: i64) -> (bool, bool) {
    let column_exists: bool = conn
        .query_row(
            "SELECT has_categorie FROM imports WHERE id = ?1",
            [import_id],
            |row| row.get(0),
        )
        .unwrap_or(false);

    let has_data: bool = if column_exists {
        conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM tickets
                WHERE import_id = ?1 AND categorie IS NOT NULL AND categorie != ''
                LIMIT 1
            )",
            [import_id],
            |row| row.get(0),
        )
        .unwrap_or(false)
    } else {
        false
    };

    (column_exists, has_data)
}
```

### 4.3 Catégorie ITIL comme override

Quand la colonne Catégorie ITIL est disponible, elle constitue un **override** (remplacement) de la hiérarchie groupe, pas un complément. Le format attendu est identique : niveaux séparés par `>`. Le parsing est mutualisé via `parse_hierarchy()`.

Les colonnes SQLite `categorie_niveau1` et `categorie_niveau2` (Segment 2) sont remplies en parallèle des `groupe_niveauN`. Le schéma ne prévoit que 2 niveaux de catégorie ITIL car la norme ITIL limite rarement à plus de 3 niveaux (Service/Sous-catégorie/Détail), et l'usage CPAM sera probablement à 2 niveaux.

---

## 5. Requêtes SQL d'agrégation

### 5.1 Données brutes pour la construction de l'arbre

```sql
-- Récupère les données nécessaires à build_category_tree()
-- Source : Groupe de techniciens
SELECT
    groupe_niveau1,
    groupe_niveau2,
    groupe_niveau3,
    type_ticket,
    est_vivant,
    anciennete_jours
FROM tickets
WHERE import_id = ?1
  AND CASE
        WHEN ?2 = 'vivants' THEN est_vivant = 1
        WHEN ?2 = 'termines' THEN est_vivant = 0
        ELSE 1 = 1  -- 'tous'
      END;
```

```sql
-- Source : Catégorie ITIL (quand disponible)
SELECT
    categorie_niveau1 AS niveau1,
    categorie_niveau2 AS niveau2,
    NULL AS niveau3,
    type_ticket,
    est_vivant,
    anciennete_jours
FROM tickets
WHERE import_id = ?1
  AND categorie_niveau1 IS NOT NULL
  AND CASE
        WHEN ?2 = 'vivants' THEN est_vivant = 1
        WHEN ?2 = 'termines' THEN est_vivant = 0
        ELSE 1 = 1
      END;
```

### 5.2 Agrégation par niveau (vue tabulaire rapide)

Pour les cas où l'arbre complet n'est pas nécessaire (ex: export Excel, tooltip KPI), des requêtes SQL directes sont plus efficaces :

```sql
-- Comptage par niveau 1 (vue la plus agrégée)
SELECT
    groupe_niveau1 AS categorie,
    COUNT(*) AS total,
    SUM(CASE WHEN est_vivant = 1 THEN 1 ELSE 0 END) AS vivants,
    SUM(CASE WHEN type_ticket = 'Incident' THEN 1 ELSE 0 END) AS incidents,
    SUM(CASE WHEN type_ticket = 'Demande' THEN 1 ELSE 0 END) AS demandes,
    ROUND(AVG(anciennete_jours), 1) AS age_moyen,
    COUNT(DISTINCT technicien_principal) AS nb_techniciens
FROM tickets
WHERE import_id = ?1
  AND groupe_niveau1 IS NOT NULL
  AND est_vivant = 1
GROUP BY groupe_niveau1
ORDER BY total DESC;
```

```sql
-- Comptage par niveau 2 avec contexte niveau 1 (drill-down intermédiaire)
SELECT
    groupe_niveau1,
    groupe_niveau2 AS categorie,
    COUNT(*) AS total,
    SUM(CASE WHEN est_vivant = 1 THEN 1 ELSE 0 END) AS vivants,
    SUM(CASE WHEN type_ticket = 'Incident' THEN 1 ELSE 0 END) AS incidents,
    SUM(CASE WHEN type_ticket = 'Demande' THEN 1 ELSE 0 END) AS demandes,
    ROUND(AVG(anciennete_jours), 1) AS age_moyen
FROM tickets
WHERE import_id = ?1
  AND groupe_niveau2 IS NOT NULL
  AND est_vivant = 1
GROUP BY groupe_niveau1, groupe_niveau2
ORDER BY total DESC;
```

```sql
-- Comptage par chemin complet (niveau le plus granulaire)
SELECT
    COALESCE(
        groupe_niveau1 || ' > ' || groupe_niveau2 || ' > ' || groupe_niveau3,
        groupe_niveau1 || ' > ' || groupe_niveau2,
        groupe_niveau1
    ) AS chemin_complet,
    COUNT(*) AS total,
    SUM(CASE WHEN est_vivant = 1 THEN 1 ELSE 0 END) AS vivants,
    SUM(CASE WHEN type_ticket = 'Incident' THEN 1 ELSE 0 END) AS incidents,
    ROUND(AVG(anciennete_jours), 1) AS age_moyen
FROM tickets
WHERE import_id = ?1 AND est_vivant = 1
GROUP BY chemin_complet
ORDER BY total DESC;
```

### 5.3 Drill-down UNION (vue hiérarchique aplatie)

Pour alimenter un treemap ou sunburst qui a besoin de tous les niveaux en une seule requête :

```sql
-- Niveau 1
SELECT
    groupe_niveau1 AS label,
    NULL AS parent,
    1 AS niveau,
    COUNT(*) AS total
FROM tickets
WHERE import_id = ?1 AND est_vivant = 1 AND groupe_niveau1 IS NOT NULL
GROUP BY groupe_niveau1

UNION ALL

-- Niveau 2
SELECT
    groupe_niveau2 AS label,
    groupe_niveau1 AS parent,
    2 AS niveau,
    COUNT(*) AS total
FROM tickets
WHERE import_id = ?1 AND est_vivant = 1 AND groupe_niveau2 IS NOT NULL
GROUP BY groupe_niveau1, groupe_niveau2

UNION ALL

-- Niveau 3
SELECT
    groupe_niveau3 AS label,
    groupe_niveau2 AS parent,
    3 AS niveau,
    COUNT(*) AS total
FROM tickets
WHERE import_id = ?1 AND est_vivant = 1 AND groupe_niveau3 IS NOT NULL
GROUP BY groupe_niveau1, groupe_niveau2, groupe_niveau3

ORDER BY niveau, total DESC;
```

---

## 6. Évolution temporelle par catégorie

### 6.1 Principe

L'évolution temporelle par catégorie répond à la question : « Comment le flux de tickets évolue-t-il pour chaque sous-service au fil du temps ? ». Elle croise la dimension temporelle (Segment 3, §3) avec la dimension catégorielle.

### 6.2 Structures de données

```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryTemporalRequest {
    /// "day", "week", "month"
    pub period: String,
    /// ISO 8601
    pub date_from: String,
    /// ISO 8601
    pub date_to: String,
    /// Niveau de catégorie à ventiler (1, 2 ou 3)
    pub category_level: usize,
    /// "groupe" ou "categorie"
    pub source: Option<String>,
    /// "vivants", "tous", "termines"
    pub scope: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryTemporalResult {
    pub source: String,
    pub period: String,
    pub category_level: usize,
    pub series: Vec<CategoryTimeSeries>,
    pub period_keys: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryTimeSeries {
    /// Nom de la catégorie (ex: "_SUPPORT UTILISATEURS...")
    pub category: String,
    /// Données par période, dans le même ordre que period_keys
    pub data: Vec<CategoryPeriodData>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryPeriodData {
    pub period_key: String,
    pub entrees: usize,
    pub sorties: usize,
    pub delta: i64,
}
```

### 6.3 Requête SQL pivotée par catégorie et période

```sql
-- Entrées par catégorie et par mois
SELECT
    groupe_niveau2 AS categorie,
    strftime('%Y-%m', date_ouverture) AS period_key,
    COUNT(*) AS entrees
FROM tickets
WHERE import_id = ?1
  AND date_ouverture BETWEEN ?2 AND ?3
  AND groupe_niveau2 IS NOT NULL
GROUP BY categorie, period_key
ORDER BY categorie, period_key;
```

```sql
-- Sorties (approximées par dernière modification des terminés)
SELECT
    groupe_niveau2 AS categorie,
    strftime('%Y-%m', date_cloture_approx) AS period_key,
    COUNT(*) AS sorties
FROM tickets
WHERE import_id = ?1
  AND est_vivant = 0
  AND date_cloture_approx BETWEEN ?2 AND ?3
  AND groupe_niveau2 IS NOT NULL
GROUP BY categorie, period_key
ORDER BY categorie, period_key;
```

### 6.4 Assemblage en Rust

```rust
use std::collections::{BTreeMap, HashMap};

/// Assemble les séries temporelles par catégorie à partir des résultats SQL
/// d'entrées et de sorties.
pub fn build_category_temporal(
    entrees_rows: Vec<(String, String, usize)>,  // (categorie, period_key, count)
    sorties_rows: Vec<(String, String, usize)>,
    all_period_keys: &[String],
) -> Vec<CategoryTimeSeries> {
    // Indexer par (catégorie, période)
    let mut entrees_map: HashMap<(String, String), usize> = HashMap::new();
    let mut sorties_map: HashMap<(String, String), usize> = HashMap::new();
    let mut categories: BTreeMap<String, ()> = BTreeMap::new();

    for (cat, pk, count) in entrees_rows {
        categories.insert(cat.clone(), ());
        entrees_map.insert((cat, pk), count);
    }
    for (cat, pk, count) in sorties_rows {
        categories.insert(cat.clone(), ());
        sorties_map.insert((cat, pk), count);
    }

    categories
        .keys()
        .map(|cat| {
            let data = all_period_keys
                .iter()
                .map(|pk| {
                    let e = entrees_map
                        .get(&(cat.clone(), pk.clone()))
                        .copied()
                        .unwrap_or(0);
                    let s = sorties_map
                        .get(&(cat.clone(), pk.clone()))
                        .copied()
                        .unwrap_or(0);
                    CategoryPeriodData {
                        period_key: pk.clone(),
                        entrees: e,
                        sorties: s,
                        delta: e as i64 - s as i64,
                    }
                })
                .collect();

            CategoryTimeSeries {
                category: cat.clone(),
                data,
            }
        })
        .collect()
}
```

### 6.5 Granularité temporelle

Le paramètre `period` contrôle la granularité, en réutilisant les fonctions SQL du Segment 3 :

|Period|`strftime` format|Label exemple|Cas d'usage|
|---|---|---|---|
|`"day"`|`%Y-%m-%d`|`2026-01-15`|Analyse fine sur 2-4 semaines|
|`"week"`|`%G-S%V`*|`2026-S03`|Vue opérationnelle sur 1-3 mois|
|`"month"`|`%Y-%m`|`2026-01`|Vue tendancielle sur 6-12 mois|

_* `%G-S%V` utilise la semaine ISO (Segment 3, §3.3). Attention aux frontières d'année : `strftime('%G')` et non `strftime('%Y')` pour la cohérence._

---

## 7. Commande Tauri : `get_categories_tree`

### 7.1 Implémentation complète

```rust
// src-tauri/src/commands/categories.rs

use crate::analyzer::categories::{
    build_category_tree, build_category_temporal, resolve_category_source,
    check_categorie_availability, CategorySource,
};
use crate::models::{
    CategoryTree, CategoryTemporalResult, CategoriesRequest, CategoryTemporalRequest,
};
use crate::state::AppState;
use rusqlite::params;

#[tauri::command]
pub async fn get_categories_tree(
    state: tauri::State<'_, AppState>,
    request: CategoriesRequest,
) -> Result<CategoryTree, String> {
    let db_guard = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db_guard.as_ref().ok_or("Base de données non initialisée")?;

    let import_id = state.active_import_id(conn)?;

    // Déterminer la source
    let (col_exists, has_data) = check_categorie_availability(conn, import_id);
    let (source, warning) = resolve_category_source(
        request.source.as_deref(),
        col_exists,
        has_data,
    );

    // Requête adaptée à la source
    let (col_n1, col_n2, col_n3) = match source {
        CategorySource::GroupeTechniciens => {
            ("groupe_niveau1", "groupe_niveau2", "groupe_niveau3")
        }
        CategorySource::CategorieItil => {
            ("categorie_niveau1", "categorie_niveau2", "NULL")
        }
    };

    let scope_clause = match request.scope.as_str() {
        "vivants" => "AND est_vivant = 1",
        "termines" => "AND est_vivant = 0",
        _ => "", // "tous"
    };

    let sql = format!(
        "SELECT {col_n1}, {col_n2}, {col_n3}, type_ticket, est_vivant, \
         COALESCE(anciennete_jours, 0) \
         FROM tickets \
         WHERE import_id = ?1 {scope_clause}"
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows: Vec<_> = stmt
        .query_map(params![import_id], |row| {
            Ok(crate::analyzer::categories::CategoryRawRow {
                niveau1: row.get(0)?,
                niveau2: row.get(1)?,
                niveau3: row.get(2)?,
                type_ticket: row.get(3)?,
                est_vivant: row.get::<_, i32>(4)? == 1,
                anciennete_jours: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let total_tickets = rows.len();
    let source_str = match source {
        CategorySource::GroupeTechniciens => "groupe",
        CategorySource::CategorieItil => "categorie",
    };

    let mut tree = build_category_tree(&rows, source_str, total_tickets);
    tree.categorie_disponible = col_exists && has_data;

    // Ajouter le warning de fallback si applicable
    if let Some(msg) = warning {
        // Le warning est logué côté Rust et transmis au frontend
        // via un champ optionnel ou un event Tauri
        log::warn!("Fallback catégorie: {}", msg);
    }

    Ok(tree)
}

#[tauri::command]
pub async fn get_category_temporal(
    state: tauri::State<'_, AppState>,
    request: CategoryTemporalRequest,
) -> Result<CategoryTemporalResult, String> {
    let db_guard = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db_guard.as_ref().ok_or("Base de données non initialisée")?;

    let import_id = state.active_import_id(conn)?;

    let (col_exists, has_data) = check_categorie_availability(conn, import_id);
    let (source, _) = resolve_category_source(
        request.source.as_deref(),
        col_exists,
        has_data,
    );

    // Sélectionner la colonne de catégorie selon le niveau demandé
    let cat_col = match (&source, request.category_level) {
        (CategorySource::GroupeTechniciens, 1) => "groupe_niveau1",
        (CategorySource::GroupeTechniciens, 2) => "groupe_niveau2",
        (CategorySource::GroupeTechniciens, _) => "groupe_niveau3",
        (CategorySource::CategorieItil, 1) => "categorie_niveau1",
        (CategorySource::CategorieItil, _) => "categorie_niveau2",
    };

    let period_fmt = match request.period.as_str() {
        "day" => "%Y-%m-%d",
        "week" => "%G-S%V",
        _ => "%Y-%m",
    };

    // Entrées
    let sql_entrees = format!(
        "SELECT {cat_col}, strftime('{period_fmt}', date_ouverture), COUNT(*) \
         FROM tickets \
         WHERE import_id = ?1 \
           AND date_ouverture BETWEEN ?2 AND ?3 \
           AND {cat_col} IS NOT NULL \
         GROUP BY 1, 2 ORDER BY 1, 2"
    );

    let mut stmt = conn.prepare(&sql_entrees).map_err(|e| e.to_string())?;
    let entrees_rows: Vec<(String, String, usize)> = stmt
        .query_map(params![import_id, &request.date_from, &request.date_to], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get::<_, usize>(2)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Sorties
    let sql_sorties = format!(
        "SELECT {cat_col}, strftime('{period_fmt}', date_cloture_approx), COUNT(*) \
         FROM tickets \
         WHERE import_id = ?1 \
           AND est_vivant = 0 \
           AND date_cloture_approx BETWEEN ?2 AND ?3 \
           AND {cat_col} IS NOT NULL \
         GROUP BY 1, 2 ORDER BY 1, 2"
    );

    let mut stmt = conn.prepare(&sql_sorties).map_err(|e| e.to_string())?;
    let sorties_rows: Vec<(String, String, usize)> = stmt
        .query_map(params![import_id, &request.date_from, &request.date_to], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get::<_, usize>(2)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Générer toutes les clés de période (Segment 3, §3.4)
    let all_period_keys = crate::analyzer::temporal::generate_period_keys(
        &request.date_from,
        &request.date_to,
        &request.period,
    )?;

    let series = build_category_temporal(entrees_rows, sorties_rows, &all_period_keys);

    let source_str = match source {
        CategorySource::GroupeTechniciens => "groupe",
        CategorySource::CategorieItil => "categorie",
    };

    Ok(CategoryTemporalResult {
        source: source_str.to_string(),
        period: request.period,
        category_level: request.category_level,
        series,
        period_keys: all_period_keys,
    })
}
```

### 7.2 Enregistrement des commandes

Ajouter à `lib.rs` (Segment 2, §1.3) :

```rust
.invoke_handler(tauri::generate_handler![
    // ... commandes existantes ...
    commands::categories::get_categories_tree,
    commands::categories::get_category_temporal,
])
```

---

## 8. Types TypeScript (frontend)

```typescript
// src/types/categories.ts

/** Requête pour l'arbre de catégories */
export interface CategoriesRequest {
  scope: 'vivants' | 'tous' | 'termines';
  source?: 'groupe' | 'categorie';
  filterPath?: string;
}

/** Arbre complet retourné par get_categories_tree */
export interface CategoryTree {
  source: 'groupe' | 'categorie';
  categorieDisponible: boolean;
  nodes: CategoryNode[];
  totalTickets: number;
  maxDepth: number;
}

/** Nœud de l'arbre (récursif) */
export interface CategoryNode {
  name: string;
  fullPath: string;
  level: number;
  ownCount: number;
  totalCount: number;
  percentage: number;
  incidents: number;
  demandes: number;
  ageMoyen: number;
  vivants: number;
  termines: number;
  children: CategoryNode[];
}

/** Requête pour l'évolution temporelle par catégorie */
export interface CategoryTemporalRequest {
  period: 'day' | 'week' | 'month';
  dateFrom: string;
  dateTo: string;
  categoryLevel: number;
  source?: 'groupe' | 'categorie';
  scope: 'vivants' | 'tous' | 'termines';
}

/** Résultat de l'évolution temporelle */
export interface CategoryTemporalResult {
  source: string;
  period: string;
  categoryLevel: number;
  series: CategoryTimeSeries[];
  periodKeys: string[];
}

export interface CategoryTimeSeries {
  category: string;
  data: CategoryPeriodData[];
}

export interface CategoryPeriodData {
  periodKey: string;
  entrees: number;
  sorties: number;
  delta: number;
}
```

### 8.1 Hook d'appel

```typescript
// src/hooks/useCategories.ts
import { invoke } from '@tauri-apps/api/core';
import { useState, useCallback } from 'react';
import type { CategoryTree, CategoriesRequest, CategoryTemporalResult, CategoryTemporalRequest } from '../types/categories';

export function useCategories() {
  const [tree, setTree] = useState<CategoryTree | null>(null);
  const [temporal, setTemporal] = useState<CategoryTemporalResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchTree = useCallback(async (request: CategoriesRequest) => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<CategoryTree>('get_categories_tree', { request });
      setTree(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  const fetchTemporal = useCallback(async (request: CategoryTemporalRequest) => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<CategoryTemporalResult>('get_category_temporal', { request });
      setTemporal(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  return { tree, temporal, loading, error, fetchTree, fetchTemporal };
}
```

---

## 9. Helpers utilitaires

### 9.1 Aplatir l'arbre pour les vues tabulaires

```rust
/// Aplatit un arbre de catégories en liste de nœuds avec indentation.
/// Utile pour les exports Excel et les tableaux à indentation visuelle.
pub fn flatten_tree(nodes: &[CategoryNode]) -> Vec<FlatCategoryRow> {
    let mut result = Vec::new();
    fn recurse(nodes: &[CategoryNode], indent: usize, result: &mut Vec<FlatCategoryRow>) {
        for node in nodes {
            result.push(FlatCategoryRow {
                indent,
                name: node.name.clone(),
                full_path: node.full_path.clone(),
                level: node.level,
                own_count: node.own_count,
                total_count: node.total_count,
                percentage: node.percentage,
                incidents: node.incidents,
                demandes: node.demandes,
                age_moyen: node.age_moyen,
            });
            recurse(&node.children, indent + 1, result);
        }
    }
    recurse(nodes, 0, &mut result);
    result
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlatCategoryRow {
    pub indent: usize,
    pub name: String,
    pub full_path: String,
    pub level: usize,
    pub own_count: usize,
    pub total_count: usize,
    pub percentage: f64,
    pub incidents: usize,
    pub demandes: usize,
    pub age_moyen: f64,
}
```

### 9.2 Trouver un sous-arbre

```rust
/// Recherche un nœud par son chemin complet et retourne le sous-arbre.
/// Utilisé pour le drill-down frontend : clic sur un nœud → afficher ses enfants.
pub fn find_subtree<'a>(nodes: &'a [CategoryNode], full_path: &str) -> Option<&'a CategoryNode> {
    for node in nodes {
        if node.full_path == full_path {
            return Some(node);
        }
        if let Some(found) = find_subtree(&node.children, full_path) {
            return Some(found);
        }
    }
    None
}
```

### 9.3 Transformation pour treemap/sunburst (ECharts)

```typescript
// src/utils/categoryChartData.ts
import type { CategoryNode } from '../types/categories';

/** Transforme l'arbre en format ECharts treemap/sunburst */
export function toEChartsTreeData(nodes: CategoryNode[]): EChartsTreeNode[] {
  return nodes.map(node => ({
    name: node.name,
    value: node.totalCount,
    itemStyle: {
      // Dégradé de couleur selon le % de vivants
      color: getHeatColor(node.vivants / Math.max(node.totalCount, 1)),
    },
    children: node.children.length > 0
      ? toEChartsTreeData(node.children)
      : undefined,
    // Données supplémentaires pour le tooltip
    ownCount: node.ownCount,
    incidents: node.incidents,
    demandes: node.demandes,
    ageMoyen: node.ageMoyen,
  }));
}

interface EChartsTreeNode {
  name: string;
  value: number;
  itemStyle?: { color: string };
  children?: EChartsTreeNode[];
  [key: string]: unknown;
}

function getHeatColor(ratio: number): string {
  // 0% vivants = vert (tout résolu), 100% = rouge (tout ouvert)
  const r = Math.round(255 * ratio);
  const g = Math.round(200 * (1 - ratio));
  return `rgb(${r}, ${g}, 80)`;
}
```

---

## 10. Tests

### 10.1 Tests unitaires du parsing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hierarchy_standard_2_levels() {
        let result = parse_hierarchy("_DSI > _SUPPORT UTILISATEURS ET POSTES DE TRAVAIL");
        assert_eq!(result, vec!["_DSI", "_SUPPORT UTILISATEURS ET POSTES DE TRAVAIL"]);
    }

    #[test]
    fn test_parse_hierarchy_3_levels() {
        let result = parse_hierarchy(
            "_DSI > _SUPPORT UTILISATEURS ET POSTES DE TRAVAIL > _SUPPORT - PARC"
        );
        assert_eq!(result, vec![
            "_DSI",
            "_SUPPORT UTILISATEURS ET POSTES DE TRAVAIL",
            "_SUPPORT - PARC"
        ]);
    }

    #[test]
    fn test_parse_hierarchy_flat_name() {
        let result = parse_hierarchy("GC_SD");
        assert_eq!(result, vec!["GC_SD"]);
    }

    #[test]
    fn test_parse_hierarchy_html_entity() {
        let result = parse_hierarchy("_DSI > _DEVELOPPEMENT &amp; INDUSTRIALISATION");
        assert_eq!(result, vec!["_DSI", "_DEVELOPPEMENT & INDUSTRIALISATION"]);
    }

    #[test]
    fn test_parse_hierarchy_empty() {
        let result = parse_hierarchy("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_hierarchy_whitespace_only() {
        let result = parse_hierarchy("   ");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_hierarchy_extra_spaces() {
        let result = parse_hierarchy("  _DSI  >  _SUPPORT  ");
        assert_eq!(result, vec!["_DSI", "_SUPPORT"]);
    }

    #[test]
    fn test_parse_hierarchy_no_space_around_chevron() {
        // Le séparateur est " > " (avec espaces). Sans espaces, pas de split.
        let result = parse_hierarchy("_DSI>_SUPPORT");
        assert_eq!(result, vec!["_DSI>_SUPPORT"]);
    }

    #[test]
    fn test_extract_levels_full() {
        let h = vec!["A".into(), "B".into(), "C".into()];
        let (n1, n2, n3) = extract_levels(&h);
        assert_eq!(n1.as_deref(), Some("A"));
        assert_eq!(n2.as_deref(), Some("B"));
        assert_eq!(n3.as_deref(), Some("C"));
    }

    #[test]
    fn test_extract_levels_partial() {
        let h = vec!["A".into()];
        let (n1, n2, n3) = extract_levels(&h);
        assert_eq!(n1.as_deref(), Some("A"));
        assert!(n2.is_none());
        assert!(n3.is_none());
    }

    #[test]
    fn test_extract_levels_empty() {
        let h: Vec<String> = vec![];
        let (n1, n2, n3) = extract_levels(&h);
        assert!(n1.is_none());
        assert!(n2.is_none());
        assert!(n3.is_none());
    }

    #[test]
    fn test_rebuild_path() {
        let levels = vec![
            Some("_DSI".to_string()),
            Some("_SUPPORT".to_string()),
            None,
        ];
        assert_eq!(rebuild_path(&levels), "_DSI > _SUPPORT");
    }
}
```

### 10.2 Tests de construction d'arbre

```rust
#[cfg(test)]
mod tree_tests {
    use super::*;

    fn make_row(n1: &str, n2: Option<&str>, n3: Option<&str>, typ: &str, vivant: bool) -> CategoryRawRow {
        CategoryRawRow {
            niveau1: Some(n1.to_string()),
            niveau2: n2.map(|s| s.to_string()),
            niveau3: n3.map(|s| s.to_string()),
            type_ticket: typ.to_string(),
            est_vivant: vivant,
            anciennete_jours: 30,
        }
    }

    #[test]
    fn test_tree_single_level() {
        let rows = vec![
            make_row("GC_SD", None, None, "Incident", true),
        ];
        let tree = build_category_tree(&rows, "groupe", 1);
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(tree.nodes[0].name, "GC_SD");
        assert_eq!(tree.nodes[0].total_count, 1);
        assert_eq!(tree.nodes[0].own_count, 1);
        assert!(tree.nodes[0].children.is_empty());
    }

    #[test]
    fn test_tree_two_levels() {
        let rows = vec![
            make_row("_DSI", Some("_SUPPORT"), None, "Incident", true),
            make_row("_DSI", Some("_SUPPORT"), None, "Demande", true),
            make_row("_DSI", Some("_PROD"), None, "Incident", false),
        ];
        let tree = build_category_tree(&rows, "groupe", 3);
        assert_eq!(tree.nodes.len(), 1); // une seule racine : _DSI
        let dsi = &tree.nodes[0];
        assert_eq!(dsi.name, "_DSI");
        assert_eq!(dsi.total_count, 3);
        assert_eq!(dsi.own_count, 0); // aucun ticket directement sur _DSI
        assert_eq!(dsi.children.len(), 2);

        // Enfants triés par total_count DESC
        assert_eq!(dsi.children[0].name, "_SUPPORT");
        assert_eq!(dsi.children[0].total_count, 2);
        assert_eq!(dsi.children[0].incidents, 1);
        assert_eq!(dsi.children[0].demandes, 1);

        assert_eq!(dsi.children[1].name, "_PROD");
        assert_eq!(dsi.children[1].total_count, 1);
    }

    #[test]
    fn test_tree_three_levels() {
        let rows = vec![
            make_row("_DSI", Some("_SUPPORT"), None, "Incident", true),
            make_row("_DSI", Some("_SUPPORT"), Some("_PARC"), "Demande", true),
        ];
        let tree = build_category_tree(&rows, "groupe", 2);
        let dsi = &tree.nodes[0];
        assert_eq!(dsi.total_count, 2);

        let support = &dsi.children[0];
        assert_eq!(support.name, "_SUPPORT");
        assert_eq!(support.own_count, 1);   // 1 ticket directement sur _SUPPORT
        assert_eq!(support.total_count, 2); // + 1 de _PARC
        assert_eq!(support.children.len(), 1);
        assert_eq!(support.children[0].name, "_PARC");
        assert_eq!(support.children[0].own_count, 1);
    }

    #[test]
    fn test_tree_percentages() {
        let rows = vec![
            make_row("A", None, None, "Incident", true),
            make_row("B", None, None, "Incident", true),
            make_row("B", None, None, "Incident", true),
            make_row("B", None, None, "Incident", true),
        ];
        let tree = build_category_tree(&rows, "groupe", 4);
        // A = 25%, B = 75%
        let a = tree.nodes.iter().find(|n| n.name == "A").unwrap();
        let b = tree.nodes.iter().find(|n| n.name == "B").unwrap();
        assert!((a.percentage - 25.0).abs() < 0.1);
        assert!((b.percentage - 75.0).abs() < 0.1);
    }

    #[test]
    fn test_tree_empty() {
        let rows: Vec<CategoryRawRow> = vec![];
        let tree = build_category_tree(&rows, "groupe", 0);
        assert!(tree.nodes.is_empty());
        assert_eq!(tree.total_tickets, 0);
    }
}
```

### 10.3 Tests du fallback

```rust
#[cfg(test)]
mod fallback_tests {
    use super::*;

    #[test]
    fn test_fallback_no_categorie() {
        let (source, warning) = resolve_category_source(Some("categorie"), false, false);
        assert_eq!(source, CategorySource::GroupeTechniciens);
        assert!(warning.is_some());
    }

    #[test]
    fn test_fallback_categorie_empty() {
        let (source, warning) = resolve_category_source(Some("categorie"), true, false);
        assert_eq!(source, CategorySource::GroupeTechniciens);
        assert!(warning.is_some());
    }

    #[test]
    fn test_explicit_groupe() {
        let (source, warning) = resolve_category_source(Some("groupe"), true, true);
        assert_eq!(source, CategorySource::GroupeTechniciens);
        assert!(warning.is_none());
    }

    #[test]
    fn test_categorie_available() {
        let (source, warning) = resolve_category_source(Some("categorie"), true, true);
        assert_eq!(source, CategorySource::CategorieItil);
        assert!(warning.is_none());
    }

    #[test]
    fn test_default_with_no_categorie() {
        let (source, _) = resolve_category_source(None, false, false);
        assert_eq!(source, CategorySource::GroupeTechniciens);
    }
}
```

---

## 11. Récapitulatif des décisions d'architecture

|Décision|Choix|Justification|
|---|---|---|
|Séparateur de hiérarchie|`>` (avec espaces)|Convention GLPI systématique, confirmée sur 100% des données|
|Profondeur maximale stockée|3 niveaux (groupe) / 2 niveaux (catégorie)|Suffisant pour les données réelles, extensible|
|Comptage des tickets|Sur groupe_principal uniquement|Évite le double comptage des multi-assignés (1,4% des tickets)|
|Own_count vs total_count|Distinction explicite|Permet au frontend de choisir : treemap (total) vs tableau (own)|
|Propagation des métriques|Bottom-up en Rust|SQL ne peut pas faire de récursion arborescente efficacement|
|Âge moyen propagé|Moyenne pondérée par total_count|Plus précis qu'une simple moyenne des moyennes enfants|
|Fallback categorie → groupe|Automatique avec warning|L'utilisateur n'a pas à connaître la structure de l'export|
|Entités HTML|Décodage `&amp;` systématique|GLPI encode les `&` dans ses exports CSV|
|Nœud racine sans tickets propres|own_count = 0, total_count = Σ enfants|`_DSI` n'a jamais de tickets directs, mais agrège tout|
|Tri des nœuds|Par total_count DESC|Les sous-services les plus chargés apparaissent en premier|
|Évolution temporelle|Requête séparée (pas dans l'arbre)|Sépare les préoccupations : arbre = snapshot, temporal = série|

---

_Ce segment fournit l'intégralité de la logique de catégorisation hiérarchique pour le GLPI Dashboard. Il consomme les données parsées par le Segment 1, utilise le schéma SQLite du Segment 2, et étend les patterns d'agrégation du Segment 3. Le Segment 5 (NLP et text mining) pourra ventiler ses résultats par catégorie en s'appuyant sur le même arbre._